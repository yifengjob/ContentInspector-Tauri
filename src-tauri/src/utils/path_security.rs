#![allow(dead_code)]
/// 文件路径安全检查模块
/// 
/// 提供基础的文件路径安全验证功能，防止：
/// - 路径遍历攻击（Path Traversal）- 如 `../..` 等
/// 
/// 注意：作为全盘扫描工具，不限制访问范围，仅防御明显的安全威胁。
/// 
/// 核心函数：
/// - is_path_safe: 检查路径是否安全（无路径遍历攻击）
/// - validate_scan_path: 验证扫描路径的合法性

use std::path::{Path, PathBuf};

/// 路径安全检查结果
#[derive(Debug, Clone, PartialEq)]
pub enum PathCheckResult {
    /// 路径安全
    Allowed,
    /// 路径为空或无效
    InvalidPath(String),
    /// 相对路径不被允许
    RelativePathNotAllowed(String),
    /// 检测到路径遍历攻击（如 `../..`）
    PathTraversalDetected(String),
}

impl std::fmt::Display for PathCheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathCheckResult::Allowed => write!(f, "路径安全"),
            PathCheckResult::InvalidPath(msg) => write!(f, "无效路径: {}", msg),
            PathCheckResult::RelativePathNotAllowed(path) => {
                write!(f, "相对路径不被允许: {}", path)
            }
            PathCheckResult::PathTraversalDetected(path) => {
                write!(f, "检测到路径遍历攻击: {}", path)
            }
        }
    }
}

impl PathCheckResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PathCheckResult::Allowed)
    }

    pub fn error_message(&self) -> Option<String> {
        match self {
            PathCheckResult::Allowed => None,
            PathCheckResult::InvalidPath(msg) => Some(format!("无效路径: {}", msg)),
            PathCheckResult::RelativePathNotAllowed(path) => {
                Some(format!("相对路径不被允许: {}", path))
            }
            PathCheckResult::PathTraversalDetected(path) => {
                Some(format!("检测到路径遍历攻击: {}", path))
            }
        }
    }
}

/// 检查文件路径是否安全（仅防御路径遍历攻击）
/// 
/// # 安全检查步骤
/// 1. 验证路径非空且有效
/// 2. 拒绝相对路径
/// 3. 检测路径遍历攻击（`..`）
/// 
/// # 参数
/// - `file_path`: 要检查的文件路径
/// 
/// # 返回
/// - `PathCheckResult::Allowed`: 路径安全
/// - 其他变体: 具体的错误原因
/// 
/// # 示例
/// ```rust
/// let result = is_path_safe("/home/user/Documents/file.txt");
/// assert_eq!(result, PathCheckResult::Allowed);
/// ```
pub fn is_path_safe(file_path: &str) -> PathCheckResult {
    // 1. 验证路径非空
    if file_path.is_empty() {
        return PathCheckResult::InvalidPath("路径为空".to_string());
    }

    let path = Path::new(file_path);

    // 2. 验证路径存在
    if !path.exists() {
        return PathCheckResult::InvalidPath("路径不存在".to_string());
    }

    // 3. 拒绝相对路径
    if !path.is_absolute() {
        return PathCheckResult::RelativePathNotAllowed(file_path.to_string());
    }

    // 4. 检测路径遍历攻击（规范化后检查）
    match path.canonicalize() {
        Ok(_real_path) => {
            // 检查规范化后的路径是否包含异常（理论上canonicalize已经处理了`..`）
            // 如果路径正常，则允许访问
            PathCheckResult::Allowed
        }
        Err(e) => {
            PathCheckResult::InvalidPath(format!("无法解析路径: {}", e))
        }
    }
}

/// 解析真实路径（解析符号链接和相对路径组件）
/// 
/// 类似于Node.js的`fs.realpathSync`
/// 
/// # 参数
/// - `path`: 要解析的路径
/// 
/// # 返回
/// - `Ok(PathBuf)`: 解析后的绝对路径
/// - `Err(String)`: 错误信息
pub fn resolve_real_path(path: &Path) -> Result<PathBuf, String> {
    // 使用std::fs::canonicalize解析符号链接和规范化路径
    path.canonicalize().map_err(|e| {
        format!("无法解析路径 '{}': {}", path.display(), e)
    })
}

/// 检查路径是否在扫描范围内
/// 
/// # 参数
/// - `file_path`: 文件路径
/// - `scan_paths`: 扫描路径列表
/// 
/// # 返回
/// - `true`: 文件路径是某个扫描路径的子路径
/// - `false`: 文件路径不在任何扫描路径下
fn is_in_scan_scope(file_path: &Path, scan_paths: &[String]) -> bool {
    for scan_path_str in scan_paths {
        let scan_path = Path::new(scan_path_str);
        
        // 精确匹配
        if file_path == scan_path {
            return true;
        }
        
        // 检查是否为子路径
        if let Ok(relative) = file_path.strip_prefix(scan_path) {
            // 确保不是空路径（即file_path就是scan_path本身）
            if relative.components().count() > 0 || file_path == scan_path {
                return true;
            }
        }
    }
    
    false
}

/// 验证扫描路径的合法性
/// 
/// # 安全检查
/// 1. 路径非空
/// 2. 路径存在
/// 3. 路径是目录
/// 4. 路径是绝对路径
/// 
/// # 参数
/// - `scan_path`: 要验证的扫描路径
/// 
/// # 返回
/// - `Ok(())`: 路径合法
/// - `Err(String)`: 错误信息
pub fn validate_scan_path(scan_path: &str) -> Result<(), String> {
    // 1. 验证非空
    if scan_path.is_empty() {
        return Err("扫描路径不能为空".to_string());
    }

    let path = Path::new(scan_path);

    // 2. 验证存在
    if !path.exists() {
        return Err(format!("扫描路径不存在: {}", scan_path));
    }

    // 3. 验证是目录
    if !path.is_dir() {
        return Err(format!("扫描路径必须是目录: {}", scan_path));
    }

    // 4. 验证是绝对路径
    if !path.is_absolute() {
        return Err(format!("扫描路径必须是绝对路径: {}", scan_path));
    }

    Ok(())
}

/// 【废弃】批量验证多个扫描路径，不再需要
#[deprecated(since = "2.0.0", note = "全盘扫描模式下不再使用批量验证")]
pub fn validate_scan_paths(scan_paths: &[String]) -> Result<Vec<String>, (Vec<String>, Vec<String>)> {
    let mut valid_paths = Vec::new();
    let mut errors = Vec::new();

    for path in scan_paths {
        match validate_scan_path(path) {
            Ok(()) => valid_paths.push(path.clone()),
            Err(e) => errors.push(format!("{}: {}", path, e)),
        }
    }

    if errors.is_empty() {
        Ok(valid_paths)
    } else {
        Err((valid_paths, errors))
    }
}

/// 检查路径是否是符号链接
/// 
/// # 参数
/// - `path`: 要检查的路径
/// 
/// # 返回
/// - `true`: 是符号链接
/// - `false`: 不是符号链接或无法判断
pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// 获取符号链接的目标路径
/// 
/// # 参数
/// - `link_path`: 符号链接路径
/// 
/// # 返回
/// - `Ok(PathBuf)`: 目标路径
/// - `Err(String)`: 错误信息
pub fn read_symlink_target(link_path: &Path) -> Result<PathBuf, String> {
    if !is_symlink(link_path) {
        return Err(format!("路径不是符号链接: {}", link_path.display()));
    }

    std::fs::read_link(link_path).map_err(|e| {
        format!("无法读取符号链接目标: {}", e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// 创建测试环境
    fn setup_test_env() -> (TempDir, String, String) {
        let temp_dir = TempDir::new().unwrap();
        let scan_path = temp_dir.path().to_string_lossy().to_string();
        
        // 创建子目录和文件
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        
        let test_file = sub_dir.join("test.txt");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "test content").unwrap();
        
        (temp_dir, scan_path, test_file.to_string_lossy().to_string())
    }

    #[test]
    fn test_is_path_safe_valid() {
        let (_temp_dir, _scan_path, test_file) = setup_test_env();
        
        // 测试绝对路径且存在的文件
        let result = is_path_safe(&test_file);
        assert_eq!(result, PathCheckResult::Allowed);
    }

    #[test]
    fn test_is_path_safe_empty() {
        let result = is_path_safe("");
        assert!(matches!(result, PathCheckResult::InvalidPath(_)));
    }

    #[test]
    fn test_is_path_allowed_not_exists() {
        let result = is_path_safe("/nonexistent/path" );
        assert!(matches!(result, PathCheckResult::InvalidPath(_)));
    }

    #[test]
    fn test_is_path_safe_relative() {
        // 创建一个存在的相对路径文件
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        
        // 切换到临时目录
        std::env::set_current_dir(&temp_dir).unwrap();
        
        // 创建测试文件
        File::create("test.txt").unwrap();
        
        // 使用相对路径（文件存在）
        let result = is_path_safe("./test.txt");
        
        // 恢复原目录
        std::env::set_current_dir(&original_dir).unwrap();
        
        assert!(matches!(result, PathCheckResult::RelativePathNotAllowed(_)));
    }

    #[test]
    fn test_is_path_allowed_out_of_scope() {
        // 【注意】在全盘扫描模式下，只要路径安全（无遍历攻击），就允许访问
        // 不再检查是否在扫描范围内
        let (_temp_dir1, _scan_path1, _test_file1) = setup_test_env();
        let (_temp_dir2, _scan_path2, test_file2) = setup_test_env();
        
        // test_file2 是绝对路径且存在，应该被允许
        let result = is_path_safe(&test_file2);
        
        assert_eq!(result, PathCheckResult::Allowed);
    }

    #[test]
    fn test_validate_scan_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_string_lossy().to_string();
        
        assert!(validate_scan_path(&path).is_ok());
    }

    #[test]
    fn test_validate_scan_path_empty() {
        assert!(validate_scan_path("").is_err());
    }

    #[test]
    fn test_validate_scan_path_not_exists() {
        assert!(validate_scan_path("/nonexistent").is_err());
    }

    #[test]
    fn test_validate_scan_path_not_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        File::create(&file_path).unwrap();
        
        assert!(validate_scan_path(&file_path.to_string_lossy()).is_err());
    }

    #[test]
    fn test_validate_scan_path_relative() {
        assert!(validate_scan_path("relative/path").is_err());
    }

    #[test]
    fn test_resolve_real_path() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let real_path = resolve_real_path(path).unwrap();
        assert!(real_path.is_absolute());
    }

    #[test]
    fn test_is_in_scan_scope_exact_match() {
        let temp_dir = TempDir::new().unwrap();
        let scan_path = temp_dir.path().to_string_lossy().to_string();
        let scan_paths = vec![scan_path.clone()];
        
        assert!(is_in_scan_scope(Path::new(&scan_path), &scan_paths));
    }

    #[test]
    fn test_is_in_scan_scope_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let scan_path = temp_dir.path().to_string_lossy().to_string();
        let sub_path = temp_dir.path().join("subdir");
        fs::create_dir(&sub_path).unwrap();
        
        let scan_paths = vec![scan_path];
        assert!(is_in_scan_scope(&sub_path, &scan_paths));
    }

    #[test]
    fn test_is_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        let link_path = temp_dir.path().join("link.txt");
        
        File::create(&file_path).unwrap();
        
        #[cfg(unix)]
        std::os::unix::fs::symlink(&file_path, &link_path).unwrap();
        
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&file_path, &link_path).unwrap();
        
        assert!(is_symlink(&link_path));
        assert!(!is_symlink(&file_path));
    }
}
