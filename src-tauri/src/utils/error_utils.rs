/// 错误处理模块
/// 
/// 提供统一的错误类型定义，简化错误处理逻辑

use thiserror::Error;

/// 扫描器错误类型
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ScannerError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("文件解析错误: {message}")]
    ParseError { 
        message: String,
        file_path: String,
    },
    
    #[error("敏感数据检测错误: {0}")]
    DetectionError(String),
    
    #[error("配置错误: {0}")]
    ConfigError(String),
    
    #[error("权限错误: 无法访问文件 {file_path} - {message}")]
    PermissionError {
        file_path: String,
        message: String,
    },
    
    #[error("删除失败: {file_path} - {message}")]
    DeleteError {
        file_path: String,
        message: String,
    },
    
    #[error("路径安全检查失败: {0}")]
    PathSecurityError(String),
    
    #[error("超时错误: {message}")]
    TimeoutError {
        message: String,
        timeout_secs: u64,
    },
    
    #[error("内存不足: {0}")]
    OutOfMemory(String),
    
    #[error("取消操作")]
    Cancelled,
}

/// 结果类型别名
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, ScannerError>;

/// 创建权限错误的便捷函数
#[allow(dead_code)]
pub fn permission_error(file_path: &str, message: &str) -> ScannerError {
    ScannerError::PermissionError {
        file_path: file_path.to_string(),
        message: message.to_string(),
    }
}

/// 创建删除错误的便捷函数
#[allow(dead_code)]
pub fn delete_error(file_path: &str, message: &str) -> ScannerError {
    ScannerError::DeleteError {
        file_path: file_path.to_string(),
        message: message.to_string(),
    }
}

/// 创建解析错误的便捷函数
#[allow(dead_code)]
pub fn parse_error(file_path: &str, message: &str) -> ScannerError {
    ScannerError::ParseError {
        file_path: file_path.to_string(),
        message: message.to_string(),
    }
}

/// 创建超时错误的便捷函数
#[allow(dead_code)]
pub fn timeout_error(message: &str, timeout_secs: u64) -> ScannerError {
    ScannerError::TimeoutError {
        message: message.to_string(),
        timeout_secs,
    }
}

/// 将ScannerError转换为友好的用户消息
#[allow(dead_code)]
pub fn to_user_message(error: &ScannerError) -> String {
    match error {
        ScannerError::PermissionError { file_path, message } => {
            format!("权限不足，无法访问文件:\n{}\n\n原因: {}", file_path, message)
        }
        ScannerError::DeleteError { file_path, message } => {
            format!("删除文件失败:\n{}\n\n原因: {}", file_path, message)
        }
        ScannerError::ParseError { file_path, message } => {
            format!("文件解析失败:\n{}\n\n原因: {}", file_path, message)
        }
        ScannerError::TimeoutError { message, timeout_secs } => {
            format!("操作超时 ({}秒): {}", timeout_secs, message)
        }
        ScannerError::PathSecurityError(msg) => {
            format!("路径安全检查失败: {}", msg)
        }
        ScannerError::OutOfMemory(msg) => {
            format!("内存不足: {}\n\n建议: 降低并发数或关闭其他应用", msg)
        }
        ScannerError::Cancelled => {
            "操作已取消".to_string()
        }
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_error() {
        let err = permission_error("/test/file.txt", "拒绝访问");
        assert!(matches!(err, ScannerError::PermissionError { .. }));
        
        let msg = to_user_message(&err);
        assert!(msg.contains("权限不足"));
        assert!(msg.contains("/test/file.txt"));
    }

    #[test]
    fn test_delete_error() {
        let err = delete_error("/test/file.txt", "文件被占用");
        assert!(matches!(err, ScannerError::DeleteError { .. }));
        
        let msg = to_user_message(&err);
        assert!(msg.contains("删除文件失败"));
    }

    #[test]
    fn test_timeout_error() {
        let err = timeout_error("文件解析", 60);
        assert!(matches!(err, ScannerError::TimeoutError { timeout_secs: 60, .. }));
        
        let msg = to_user_message(&err);
        assert!(msg.contains("60"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "文件不存在");
        let scanner_err: ScannerError = ScannerError::Io(io_err);
        
        assert!(matches!(scanner_err, ScannerError::Io(_)));
        assert!(scanner_err.to_string().contains("IO错误"));
    }
}
