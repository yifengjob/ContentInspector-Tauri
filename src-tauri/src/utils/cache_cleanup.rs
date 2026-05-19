/// 应用数据清理模块
/// 
/// 【优化】Tauri使用系统WebView，缓存由系统自动管理。
/// 此模块专注于清理应用自身产生的数据：
/// - 旧日志文件（最有价值）
/// - 扫描结果缓存
/// - 临时文件

use std::fs;
use std::path::{Path};
use tauri::AppHandle;

/// 缓存清理结果
#[derive(Debug, Clone)]
pub struct CacheCleanupResult {
    /// 清理的缓存目录数量
    pub directories_cleaned: usize,
    /// 清理的文件数量
    pub files_cleaned: usize,
    /// 释放的空间（字节）
    pub space_freed_bytes: u64,
    /// 清理详情
    pub details: Vec<String>,
}

impl CacheCleanupResult {
    pub fn new() -> Self {
        Self {
            directories_cleaned: 0,
            files_cleaned: 0,
            space_freed_bytes: 0,
            details: Vec::new(),
        }
    }

    pub fn add_detail(&mut self, detail: String) {
        self.details.push(detail);
    }

    pub fn format_summary(&self) -> String {
        format!(
            "缓存清理完成:\n- 清理目录: {} 个\n- 删除文件: {} 个\n- 释放空间: {}",
            self.directories_cleaned,
            self.files_cleaned,
            format_bytes(self.space_freed_bytes)
        )
    }
}

/// 格式化字节数为可读字符串
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 清理应用数据
/// 
/// 【优化】移除WebView缓存清理（由系统管理），专注于应用数据
/// 
/// # 参数
/// - `_app_handle`: Tauri应用句柄（保留以保持API兼容）
/// - `clean_logs`: 是否清理旧日志文件
/// - `clean_temp`: 是否清理临时文件和扫描缓存
/// - `log_retention_days`: 日志保留天数（默认30天）
/// 
/// # 返回
/// - `Ok(CacheCleanupResult)`: 清理结果
/// - `Err(String)`: 错误信息
pub fn clear_cache(
    _app_handle: &AppHandle,
    clean_logs: bool,
    clean_temp: bool,
    log_retention_days: u64,
) -> Result<CacheCleanupResult, String> {
    let mut result = CacheCleanupResult::new();

    // 获取应用数据目录
    // 【严格模式】使用 config 模块中定义的 APP_IDENTIFIER 常量
    let app_data_dir = match dirs::data_local_dir() {
        Some(dir) => dir.join(crate::utils::config::APP_IDENTIFIER),
        None => {
            return Err("无法获取应用数据目录".to_string());
        }
    };

    // 1. 清理旧日志文件（最有价值）
    if clean_logs
        && let Err(e) = clean_old_logs(&app_data_dir, log_retention_days, &mut result) {
            log_warn!("清理旧日志失败: {}", e);
            result.add_detail(format!("⚠️ 日志清理失败: {}", e));
        }

    // 2. 清理临时文件和扫描缓存
    if clean_temp
        && let Err(e) = clean_temp_files(&app_data_dir, &mut result) {
            log_warn!("清理临时文件失败: {}", e);
            result.add_detail(format!("⚠️ 临时文件清理失败: {}", e));
        }

    log_info!("{}", result.format_summary());

    Ok(result)
}

/// 清理旧日志文件
fn clean_old_logs(
    app_data_dir: &Path,
    log_retention_days: u64,
    result: &mut CacheCleanupResult,
) -> Result<(), String> {
    let log_dir = app_data_dir.join("logs");
    
    if !log_dir.exists() {
        log_debug!("日志目录不存在，跳过清理");
        return Ok(());
    }

    let entries = fs::read_dir(&log_dir).map_err(|e| {
        format!("读取日志目录失败: {}", e)
    })?;

    let retention_secs = log_retention_days * 24 * 60 * 60;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        // 只处理.log文件
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext != "log" {
                continue;
            }
        } else {
            continue;
        }

        // 检查文件修改时间
        if let Ok(metadata) = fs::metadata(&path)
            && let Ok(modified) = metadata.modified()
                && let Ok(elapsed) = modified.elapsed()
                    && elapsed.as_secs() > retention_secs {
                        let file_size = metadata.len();
                        fs::remove_file(&path).map_err(|e| {
                            format!("删除日志文件 {:?} 失败: {}", path, e)
                        })?;
                        
                        result.files_cleaned += 1;
                        result.space_freed_bytes += file_size;
                        result.add_detail(format!(
                            "✅ 删除旧日志: {} ({}天前, 释放 {})",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            elapsed.as_secs() / (24 * 60 * 60),
                            format_bytes(file_size)
                        ));
                        
                        log_debug!("已删除旧日志文件: {:?}", path);
                    }
    }

    Ok(())
}

/// 清理临时文件和扫描缓存
fn clean_temp_files(app_data_dir: &Path, result: &mut CacheCleanupResult) -> Result<(), String> {
    // 清理临时目录
    let temp_dir = app_data_dir.join("temp");
    
    if temp_dir.exists() {
        clean_directory(&temp_dir, "临时文件", result)?;
    } else {
        log_debug!("临时目录不存在，跳过清理");
    }

    // 清理扫描结果缓存（如果有）
    let scan_cache_dir = app_data_dir.join("scan_cache");
    if scan_cache_dir.exists() {
        clean_directory(&scan_cache_dir, "扫描缓存", result)?;
    } else {
        log_debug!("扫描缓存目录不存在，跳过清理");
    }

    Ok(())
}

/// 通用目录清理函数
fn clean_directory(
    dir: &Path,
    dir_name: &str,
    result: &mut CacheCleanupResult,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| {
        format!("读取{}目录失败: {}", dir_name, e)
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        let file_size = if path.is_dir() {
            get_dir_size(&path)?
        } else {
            fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        };

        fs::remove_dir_all(&path).or_else(|_| fs::remove_file(&path)).map_err(|e| {
            format!("删除{}/{} {:?} 失败: {}", dir_name, path.file_name().unwrap_or_default().to_string_lossy(), path, e)
        })?;
        
        result.files_cleaned += 1;
        result.space_freed_bytes += file_size;
        result.add_detail(format!(
            "✅ 清理{}: {} (释放 {})",
            dir_name,
            path.file_name().unwrap_or_default().to_string_lossy(),
            format_bytes(file_size)
        ));
        
        log_debug!("已清理{}/{}: {:?}", dir_name, path.file_name().unwrap_or_default().to_string_lossy(), path);
    }

    Ok(())
}

/// 计算目录大小
fn get_dir_size(dir: &Path) -> Result<u64, String> {
    let mut total_size = 0u64;

    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| {
            format!("读取目录 {:?} 失败: {}", dir, e)
        })? {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                total_size += get_dir_size(&path)?;
            } else {
                total_size += fs::metadata(&path)
                    .map(|m| m.len())
                    .unwrap_or(0);
            }
        }
    }

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(100), "100 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_cache_cleanup_result() {
        let mut result = CacheCleanupResult::new();
        result.add_detail("测试详情".to_string());
        
        assert_eq!(result.directories_cleaned, 0);
        assert_eq!(result.files_cleaned, 0);
        assert_eq!(result.space_freed_bytes, 0);
        assert_eq!(result.details.len(), 1);
    }

    #[test]
    fn test_get_dir_size() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        
        let size = get_dir_size(temp_dir.path()).unwrap();
        assert!(size > 0);
    }
}
