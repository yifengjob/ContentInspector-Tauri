#![allow(dead_code)]
/// 结构化日志系统（Electron版对齐）
/// 
/// 提供结构化的日志记录功能，支持：
/// - 多级别日志（DEBUG/INFO/WARN/ERROR）
/// - 日志文件输出（自动轮转、30天保留）
/// - 前端IPC通信（实时发送到UI）
/// - 内存环形缓冲区（最多1000条）
/// - 日志抑制（过滤PDF警告等噪音）
/// - 占位符格式化：logger.info("用户{}登录", username)

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Local;
use tauri::{AppHandle, Emitter};

use crate::utils::scanner_helpers::RingBuffer;

// 使用config中定义的LogLevel
pub use crate::utils::config::LogLevel;

/// 日志配置
pub struct LogConfig {
    /// 日志上下文（模块名）
    pub context: String,
    /// 是否写入文件
    pub enable_file: bool,
    /// 是否发送到前端
    pub enable_frontend: bool,
    /// 是否保存到内存
    pub enable_memory: bool,
    /// 写入文件的最低级别
    pub file_level: LogLevel,
    /// 发送到前端的最低级别
    pub frontend_level: LogLevel,
}

impl LogConfig {
    pub fn new(context: &str) -> Self {
        use crate::utils::config;
        Self {
            context: context.to_string(),
            enable_file: config::LOG_ENABLE_FILE,
            enable_frontend: config::LOG_ENABLE_FRONTEND,
            enable_memory: false,
            file_level: config::LOG_FILE_LEVEL,
            frontend_level: config::LOG_FRONTEND_LEVEL,
        }
    }

    pub fn with_file(mut self, enable: bool) -> Self {
        self.enable_file = enable;
        self
    }

    pub fn with_frontend(mut self, enable: bool) -> Self {
        self.enable_frontend = enable;
        self
    }

    pub fn with_memory(mut self, enable: bool) -> Self {
        self.enable_memory = enable;
        self
    }

    pub fn with_file_level(mut self, level: LogLevel) -> Self {
        self.file_level = level;
        self
    }

    pub fn with_frontend_level(mut self, level: LogLevel) -> Self {
        self.frontend_level = level;
        self
    }
}

/// 日志记录器
pub struct StructuredLogger {
    config: LogConfig,
    /// 内存缓冲区（环形队列）
    memory_buffer: Option<Arc<Mutex<RingBuffer<String>>>>,
    /// 上次更新时间（用于节流）
    last_update_time: Arc<Mutex<Instant>>,
    /// 日志文件路径（使用Mutex支持内部可变性）
    log_file_path: Arc<Mutex<Option<PathBuf>>>,
    /// 是否已初始化文件
    initialized: Arc<Mutex<bool>>,
}

impl StructuredLogger {
    /// 创建新的日志记录器
    pub fn new(config: LogConfig) -> Self {
        let memory_buffer = if config.enable_memory {
            Some(Arc::new(Mutex::new(RingBuffer::new(1000))))
        } else {
            None
        };

        Self {
            config,
            memory_buffer,
            last_update_time: Arc::new(Mutex::new(Instant::now())),
            log_file_path: Arc::new(Mutex::new(None)),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// 初始化日志文件（线程安全）
    pub fn init_log_file(&self, log_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // 检查是否已初始化
        {
            let mut init_flag = self.initialized.lock().unwrap();
            if *init_flag {
                return Ok(()); // 已经初始化过
            }
            *init_flag = true;
        }

        // 创建日志目录
        if !log_dir.exists() {
            fs::create_dir_all(log_dir)?;
        }

        // 清理旧日志文件
        cleanup_old_logs(log_dir)?;

        // 生成日志文件名（使用北京时间）
        let now = Local::now();
        let time_str = now.format("%Y-%m-%dT%H-%M-%S");
        let log_file = log_dir.join(format!("app-{}.log", time_str));

        // 创建日志文件
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        // 设置文件路径
        let mut path_guard = self.log_file_path.lock().unwrap();
        *path_guard = Some(log_file.clone());

        Ok(())
    }

    /// DEBUG级别日志
    pub fn debug(&self, app_handle: Option<&AppHandle>, template: &str, args: &[&dyn std::fmt::Display]) {
        self.log(LogLevel::Debug, app_handle, template, args);
    }

    /// INFO级别日志
    pub fn info(&self, app_handle: Option<&AppHandle>, template: &str, args: &[&dyn std::fmt::Display]) {
        self.log(LogLevel::Info, app_handle, template, args);
    }

    /// WARN级别日志
    pub fn warn(&self, app_handle: Option<&AppHandle>, template: &str, args: &[&dyn std::fmt::Display]) {
        self.log(LogLevel::Warn, app_handle, template, args);
    }

    /// ERROR级别日志
    pub fn error(&self, app_handle: Option<&AppHandle>, template: &str, args: &[&dyn std::fmt::Display]) {
        self.log(LogLevel::Error, app_handle, template, args);
    }

    /// 通用日志方法（支持占位符和级别过滤）
    fn log(&self, level: LogLevel, app_handle: Option<&AppHandle>, template: &str, args: &[&dyn std::fmt::Display]) {
        // 检查级别是否需要处理
        let should_write_to_file = self.config.enable_file && level >= self.config.file_level;
        let should_send_to_frontend = self.config.enable_frontend && level >= self.config.frontend_level;
        
        // 如果都不需要，直接返回
        if !should_write_to_file && !should_send_to_frontend && !self.config.enable_memory {
            return;
        }

        // 格式化消息
        let message = format_log_message(template, args);

        // 检查是否需要抑制
        if should_suppress_log(&message) {
            return;
        }

        // 生成完整日志条目
        let timestamp = get_beijing_timestamp();
        let formatted_msg = format!("[{}] [{}] [{}] {}", timestamp, level, self.config.context, message);

        // 写入文件
        if should_write_to_file {
            let path_guard = self.log_file_path.lock().unwrap();
            if let Some(ref log_path) = *path_guard {
                write_to_file(log_path, &formatted_msg).ok();
            }
        }

        // 发送到前端（通过IPC）
        if should_send_to_frontend
            && let Some(handle) = app_handle {
                send_to_frontend(handle, &formatted_msg);
            }

        // 保存到内存
        if self.config.enable_memory
            && let Some(ref buffer) = self.memory_buffer {
                let mut buf = buffer.lock().unwrap();
                buf.push(formatted_msg.clone());

                // 限制更新频率（1秒）
                let now = Instant::now();
                let mut last_time = self.last_update_time.lock().unwrap();
                if now.duration_since(*last_time) >= Duration::from_secs(1) {
                    // TODO: 可以在此处触发UI更新
                    *last_time = now;
                }
            }

        // 同时输出到控制台（使用log crate）
        match level {
            LogLevel::Debug => log::debug!("{}", formatted_msg),
            LogLevel::Info => log::info!("{}", formatted_msg),
            LogLevel::Warn => log::warn!("{}", formatted_msg),
            LogLevel::Error => log::error!("{}", formatted_msg),
        }
    }

    /// 获取内存中的日志列表
    pub fn get_memory_logs(&self) -> Vec<String> {
        if let Some(ref buffer) = self.memory_buffer {
            let buf = buffer.lock().unwrap();
            buf.to_vec()
        } else {
            vec![]
        }
    }
}

/// 格式化日志消息（支持占位符）
/// 
/// 支持两种模式：
/// 1. 占位符模式："用户{}登录", ["admin"] → "用户admin登录"
/// 2. 拼接模式："日志:", ["message"] → "日志: message"
fn format_log_message(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    if args.is_empty() {
        return template.to_string();
    }

    // 检查是否有占位符
    let placeholder_count = template.matches("{}").count();

    if placeholder_count > 0 {
        // 使用占位符模式
        let mut result = String::new();
        let mut arg_index = 0;
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'}') {
                chars.next(); // 消耗 '}'
                if arg_index < args.len() {
                    result.push_str(&format!("{}", args[arg_index]));
                    arg_index += 1;
                } else {
                    result.push_str("{}");
                }
            } else {
                result.push(ch);
            }
        }

        // 如果还有多余参数，追加到末尾
        if arg_index < args.len() {
            for i in arg_index..args.len() {
                result.push(' ');
                result.push_str(&format!("{}", args[i]));
            }
        }

        result
    } else {
        // 无占位符，使用空格连接
        let mut result = template.to_string();
        for arg in args {
            result.push(' ');
            result.push_str(&format!("{}", arg));
        }
        result
    }
}

/// 获取北京时间的时间戳字符串
fn get_beijing_timestamp() -> String {
    let now = Local::now();
    now.format("%H:%M:%S").to_string()
}

/// 检查是否需要抑制日志
fn should_suppress_log(message: &str) -> bool {
    const SUPPRESS_PATTERNS: &[&str] = &[
        // pdfjs-dist 的字体警告
        "Warning: TT: undefined function",
        "Warning: TT: invalid offset",
        "Warning: Indexing all PDF objects",
        "Warning: Ran out of space in font private use area",
        "Warning: TT: undefined subroutine",
        "Warning: TT: invalid glyph index",
        "Warning: Required \"glyf\" table is not found",
        "Warning: fetchStandardFontData: failed to fetch file",
        "Warning: loadFont - translateFont failed:",
        
        // canvas 模块缺失警告
        "Cannot polyfill `Path2D`",
        "Cannot find module",
        "canvas.node",
    ];

    SUPPRESS_PATTERNS.iter().any(|pattern| message.contains(pattern))
}

/// 写入日志到文件
fn write_to_file(log_path: &Path, message: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    
    writeln!(file, "{}", message)?;
    file.flush()?;
    
    Ok(())
}

/// 发送日志到前端（通过IPC）
fn send_to_frontend(app_handle: &AppHandle, message: &str) {
    // 使用tauri的事件系统发送日志
    let _ = app_handle.emit("scan-log", message);
}

/// 清理旧日志文件（保留30天）
fn cleanup_old_logs(log_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !log_dir.exists() {
        return Ok(());
    }

    let retention_days = 30;
    let entries = fs::read_dir(log_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // 只处理日志文件
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if !file_name.starts_with("app-") || !file_name.ends_with(".log") {
                continue;
            }

            // 检查文件修改时间
            if let Ok(metadata) = fs::metadata(&path)
                && let Ok(modified) = metadata.modified()
                    && let Ok(elapsed) = modified.elapsed()
                        && elapsed.as_secs() > (retention_days as u64 * 24 * 60 * 60) {
                            fs::remove_file(&path)?;
                            log::info!("已删除旧日志文件: {}", file_name);
                        }
        }
    }

    Ok(())
}

/// 便捷函数：创建默认日志记录器
pub fn create_logger(context: &str) -> StructuredLogger {
    StructuredLogger::new(LogConfig::new(context))
}

lazy_static::lazy_static! {
    pub static ref GENERAL_LOGGER: StructuredLogger = create_logger("General");
    pub static ref FILE_LOGGER: StructuredLogger = create_logger("File");
    pub static ref MAIN_LOGGER: StructuredLogger = create_logger("Main");
    pub static ref WORKER_LOGGER: StructuredLogger = create_logger("Worker");
    pub static ref SCANNER_LOGGER: StructuredLogger = create_logger("Scanner");
}

/// 获取默认的全局logger（用于没有特定上下文的日志）
pub fn get_global_logger() -> &'static StructuredLogger {
    &GENERAL_LOGGER
}

// ==================== 全局日志宏定义 ====================
// 【优化】这些宏提供便捷的日志记录接口，支持任意类型的参数
// 使用format!进行格式化，兼容Display和Debug格式

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::utils::logger::get_global_logger().debug(None, &format!($($arg)*), &[])
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::utils::logger::get_global_logger().info(None, &format!($($arg)*), &[])
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::utils::logger::get_global_logger().warn(None, &format!($($arg)*), &[])
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::utils::logger::get_global_logger().error(None, &format!($($arg)*), &[])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_log_message_placeholders() {
        // 占位符模式
        assert_eq!(
            format_log_message("用户{}登录", &[&"admin"]),
            "用户admin登录"
        );

        assert_eq!(
            format_log_message("用户{}年龄{}", &[&"张三", &25]),
            "用户张三年龄25"
        );
    }

    #[test]
    fn test_format_log_message_concat() {
        // 拼接模式
        assert_eq!(
            format_log_message("日志:", &[&"message"]),
            "日志: message"
        );
    }

    #[test]
    fn test_format_log_message_no_args() {
        assert_eq!(
            format_log_message("简单消息", &[]),
            "简单消息"
        );
    }

    #[test]
    fn test_should_suppress_log() {
        // 应该被抑制的日志
        assert!(should_suppress_log("Warning: TT: undefined function: 32"));
        assert!(should_suppress_log("Cannot polyfill `Path2D`"));

        // 不应该被抑制的日志
        assert!(!should_suppress_log("正常日志消息"));
        assert!(!should_suppress_log("扫描完成"));
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }
}
