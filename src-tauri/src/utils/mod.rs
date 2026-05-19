/// 工具模块

pub mod error_utils;
pub mod scanner_helpers;
pub mod config;
pub mod file_types;  // 智能文件类型路由系统
pub mod concurrency;
pub mod environment;
pub mod system_dirs;
#[macro_use]
pub mod logger;  // 日志模块（包含宏定义）
pub mod path_security;
pub mod power_manager;
pub mod message_box;
pub mod cache_cleanup;
pub mod dev_tools;
pub mod excel_export;
pub mod windows_multi_drive;
pub mod environment_check;
pub mod expression_parser;  // 【新增】自定义表达式解析器
