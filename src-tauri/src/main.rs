#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 工具模块（包含日志宏，必须最先声明）
#[macro_use]
mod utils;

// 数据模型
mod models;

// 核心业务逻辑
mod core;

// 数据处理
mod processing;

// 其他模块
mod commands;

use crate::utils::config;
use crate::utils::environment::check_environment;
use commands::*;
use tauri::Manager;

fn main() {
    // 设置全局 panic hook，捕获所有未处理的 panic
    // 这对于防止 pdf-extract 等第三方库的 panic 导致程序崩溃非常重要
    std::panic::set_hook(Box::new(|info| {
        // 只记录错误信息，不打印 panic 详情
        // 这样可以避免控制台输出大量技术细节，影响用户体验
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            log_error!("⚠️ 内部错误（已自动处理）: {}", s);
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            log_error!("⚠️ 内部错误（已自动处理）: {}", s);
        } else {
            log_error!("⚠️ 发生未知内部错误（已自动处理）");
        }

        // 注意：不调用 default_panic，完全抑制 panic 输出
        // 因为我们的 catch_unwind 已经处理了这些错误
        // 用户只会看到友好的错误提示，不会看到技术细节
    }));

    // 初始化日志系统（使用StructuredLogger）
    // 配置来自 config.rs 中的常量
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join(crate::utils::config::APP_IDENTIFIER)
        .join("logs");
    
    // 初始化全局logger的文件输出
    use crate::utils::logger::GENERAL_LOGGER;
    if let Err(e) = GENERAL_LOGGER.init_log_file(&log_dir) {
        eprintln!("警告: 无法初始化日志文件: {}", e);
    }
    
    // 同时初始化env_logger作为后备（用于第三方库的日志）
    // 根据config中的级别设置过滤器
    use crate::utils::config::LogLevel;
    let filter_str = match config::LOG_FILE_LEVEL {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    };
    
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(format!("{},lopdf=error,pdf_extract=error", filter_str)),
    )
    .init();

    // 检查系统环境
    let env_check = check_environment();

    if !env_check.is_ready {
        // 有严重问题，显示错误信息并退出
        eprintln!("\n❌ 系统环境检查失败！\n");
        eprintln!("操作系统: {}", env_check.os_version);
        eprintln!("\n发现以下问题：\n");

        for (i, issue) in env_check.issues.iter().enumerate() {
            let severity_icon = match issue.severity {
                utils::environment::IssueSeverity::Critical => "🔴",
                utils::environment::IssueSeverity::Warning => "🟡",
            };

            eprintln!("{}. {} {}", i + 1, severity_icon, issue.title);
            eprintln!("   {}", issue.description);
            eprintln!("   解决方案: {}", issue.solution);

            if let Some(url) = &issue.download_url {
                eprintln!("   下载地址: {}", url);
            }
            eprintln!();
        }

        eprintln!("\n请解决上述问题后重新启动应用程序。\n");

        // 在 Windows 上显示图形化对话框
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let message = format!(
                "系统环境检查失败！\n\n{}\n\n请查看控制台获取详细信息和下载链接。",
                env_check
                    .issues
                    .first()
                    .map(|i| i.title.as_str())
                    .unwrap_or("未知错误")
            );

            let _ = Command::new("cmd")
                .args(&["/c", "start", "cmd", "/k", "echo", &message])
                .spawn();
        }

        std::process::exit(1);
    }

    // 如果有警告，记录但不阻止启动
    if !env_check.issues.is_empty() {
        log_warn!("系统环境存在以下警告:");
        for issue in &env_check.issues {
            log_warn!("- {}: {}", issue.title, issue.description);
        }
    }

    log_info!("系统环境检查通过: {}", env_check.os_version);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .manage(ScanState::new())
        .manage(AppState::new())  // 【新增】管理应用状态
        .invoke_handler(tauri::generate_handler![
            get_directory_tree,
            scan_start,
            scan_cancel,
            preview_file,
            preview_file_stream, // 【新增】流式预览
            cancel_preview,
            read_file_as_blob,   // 【新增】读取文件为二进制
            open_file,
            open_file_location,
            delete_file,
            export_report,
            get_logs,
            get_sensitive_rules,
            save_config,
            load_config,
            check_system_environment,
            get_recommended_concurrency, // 【新增】获取推荐并发数
            show_message_box,            // 【新增】消息对话框
            clear_cache,                 // 【新增】清理缓存
            open_dev_tools,              // 【新增】打开开发者工具
            validate_search_expression,  // 【新增】验证搜索表达式
            get_search_expression,       // 【新增】获取搜索表达式
            set_search_expression,       // 【新增】设置搜索表达式
        ])
        .setup(|app| {
            // 动态计算窗口大小
            if let Some(window) = app.get_webview_window("main") {
                // 获取主监视器
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let size = monitor.size();
                    let scale_factor = monitor.scale_factor();

                    // monitor.size() 返回物理像素，需要除以 scale_factor 得到逻辑像素
                    // 然后取 80% 作为窗口大小
                    let logical_width = size.width as f64 / scale_factor;
                    let logical_height = size.height as f64 / scale_factor;

                    let width = (logical_width * config::WINDOW_TARGET_RATIO) as u32;
                    let height = (logical_height * config::WINDOW_TARGET_RATIO) as u32;

                    // 确保最小尺寸（逻辑像素）
                    let width = width.max(config::WINDOW_MIN_WIDTH);
                    let height = height.max(config::WINDOW_MIN_HEIGHT);

                    log_info!(
                        "屏幕物理尺寸: {}x{}, 逻辑尺寸: {:.0}x{:.0}, 缩放比例: {}, 窗口尺寸: {}x{}",
                        size.width,
                        size.height,
                        logical_width,
                        logical_height,
                        scale_factor,
                        width,
                        height
                    );

                    // 设置窗口大小（使用逻辑像素）
                    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                        width: width as f64,
                        height: height as f64,
                    }));

                    // 延迟一小段时间再居中，确保窗口大小已生效
                    let window_clone = window.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            config::WINDOW_CENTER_DELAY_MS,
                        ))
                        .await;
                        let _ = window_clone.center();
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
