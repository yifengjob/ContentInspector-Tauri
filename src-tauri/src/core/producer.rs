/// 扫描生产者模块 - 负责目录遍历和文件任务生成

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::path::Path;
use tokio::sync::mpsc;

use crate::models::ScanConfig;
use crate::core::scanner::{ScanEvent, FileTask};
use crate::utils::config;

/// 通用生产者核心逻辑（支持自定义任务分发）
pub async fn producer_walk_directories_core<F>(
    config: &ScanConfig,
    cancel_flag: &Arc<AtomicBool>,
    event_tx: &mpsc::Sender<ScanEvent>,
    total_count: Arc<AtomicU64>,
    mut task_handler: F,
) where
    F: FnMut(FileTask),
{
    use walkdir::WalkDir;
    
    let mut local_count = 0u64;
    let mut last_progress_count = 0u64;
    
    for root_path in &config.selected_paths {
        // 【安全】检查取消标志
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = event_tx.send(ScanEvent::Log("扫描已取消".to_string())).await;
            return;
        }
        
        let path = Path::new(root_path);
        if !path.exists() || !path.is_dir() {
            continue;
        }
        
        // 遍历目录
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                !cancel_flag.load(Ordering::Relaxed) && should_include_directory(e, config)
            })
        {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            let file_path = match entry.path().to_str() {
                Some(p) => p.to_string(),
                None => continue,
            };
            
            // 检查扩展名
            if !should_include_extension(&file_path, &config.selected_extensions) {
                continue;
            }
            
            // 检查文件大小
            if !should_include_file_by_size(&file_path, &entry, config) {
                continue;
            }
            
            // 获取文件元数据
            let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let modified_time = entry.path()
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t: std::time::SystemTime| {
                    let datetime: chrono::DateTime<chrono::Local> = t.into();
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| "未知".to_string());
            
            // 创建文件任务
            let task = FileTask {
                file_path,
                file_size,
                modified_time,
            };
            
            // 调用自定义处理器
            task_handler(task);
            
            local_count += 1;
            total_count.fetch_add(1, Ordering::Relaxed); // 更新全局总数
            
            // 【新增】每 100 个文件发送一次进度更新
            if local_count - last_progress_count >= 100 {
                let current_total = total_count.load(Ordering::Relaxed);
                let _ = event_tx.send(ScanEvent::Progress {
                    current_file: format!("正在遍历... ({})", current_total),
                    scanned_count: 0, // 生产者不处理文件
                    total_count: current_total,
                    filtered_count: None, // 生产者阶段不统计过滤和跳过
                    skipped_count: None,
                }).await;
                last_progress_count = local_count;
            }
        }
    }
    
    log_info!("生产者完成，共找到 {} 个待扫描文件", local_count);
    
    // 【新增】发送最终总数
    let final_total = total_count.load(Ordering::Relaxed);
    let _ = event_tx.send(ScanEvent::Progress {
        current_file: "遍历完成".to_string(),
        scanned_count: 0,
        total_count: final_total,
        filtered_count: None, // 生产者阶段不统计过滤和跳过
        skipped_count: None,
    }).await;
}

/// 生产者：遍历目录并将文件任务发送到队列
#[allow(dead_code)]
pub async fn producer_walk_directories(
    config: &ScanConfig,
    task_tx: &mpsc::Sender<FileTask>,
    cancel_flag: &Arc<AtomicBool>,
    event_tx: &mpsc::Sender<ScanEvent>,
    total_count: Arc<AtomicU64>,
) {
    // 复用核心逻辑，将任务发送到channel
    producer_walk_directories_core(
        config,
        cancel_flag,
        event_tx,
        total_count,
        |task| {
            let _ = task_tx.try_send(task); // 非阻塞发送
        },
    ).await;
}

/// 检查是否应该包含目录
pub fn should_include_directory(entry: &walkdir::DirEntry, config: &ScanConfig) -> bool {
    let name = entry.file_name().to_string_lossy();
    let path = entry.path().to_string_lossy();
    
    // 1. 检查全局忽略列表
    if config.ignore_dir_names.contains(&name.to_string()) {
        log_debug!("过滤目录（名称匹配）: {}", path);
        return false;
    }
    
    // 2. 检查系统目录
    for system_dir in &config.system_dirs {
        if path.starts_with(system_dir) {
            log_debug!("过滤目录（系统目录）: {} (匹配: {})", path, system_dir);
            return false;
        }
    }
    
    true
}

/// 检查是否应该包含该扩展名的文件
pub fn should_include_extension(file_path: &str, selected_extensions: &[String]) -> bool {
    if selected_extensions.contains(&"*".to_string()) {
        // 【修复】选择"*"时，只包含支持扫描的文件类型（排除图片、视频等纯二进制文件）
        return crate::utils::file_types::supports_scanning(file_path);
    }
    
    if let Some(ext) = Path::new(file_path).extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        
        // 检查扩展名是否在选中列表中
        if !selected_extensions.contains(&ext_lower) {
            return false;
        }
        
        // 【关键修复】即使扩展名匹配，也要检查是否支持扫描
        // BinaryScan类型的文件（如图片）不应该加入队列
        crate::utils::file_types::supports_scanning(file_path)
    } else {
        false
    }
}

/// 检查文件大小是否在限制范围内
pub fn should_include_file_by_size(
    file_path: &str,
    entry: &walkdir::DirEntry,
    config: &ScanConfig,
) -> bool {
    if let Ok(metadata) = entry.metadata() {
        let file_size = metadata.len();
        let max_size = if file_path.to_lowercase().ends_with(".pdf") {
            config.max_pdf_size_mb * config::BYTES_TO_MB
        } else {
            config.max_file_size_mb * config::BYTES_TO_MB
        };
        
        file_size <= max_size
    } else {
        true
    }
}
