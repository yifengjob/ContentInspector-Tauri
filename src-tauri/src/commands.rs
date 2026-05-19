use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{State, AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::models::*;
use crate::core::scanner::{run_scan_safe, ScanEvent};
use crate::processing::preview;
use crate::core::sensitive_detector::get_builtin_rules;
use crate::utils::config;

/// 应用状态（管理全局配置）
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
}

impl AppState {
    pub fn new() -> Self {
        // 尝试加载配置，如果失败则使用默认值
        let app_config = config::load_app_config().unwrap_or_default();
        Self {
            config: Arc::new(Mutex::new(app_config)),
        }
    }
}

/// 扫描状态
pub struct ScanState {
    pub is_scanning: Arc<Mutex<bool>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub logs: Arc<Mutex<Vec<String>>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            is_scanning: Arc::new(Mutex::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// 获取目录树
#[tauri::command]
pub fn get_directory_tree(path: String, show_hidden: bool) -> Result<Vec<DirectoryNode>, String> {
    let path_obj = Path::new(&path);
    
    if !path_obj.exists() {
        return Err("路径不存在".to_string());
    }
    
    let mut nodes = Vec::new();
    
    // 读取目录内容
    if let Ok(entries) = std::fs::read_dir(path_obj) {
        for entry in entries.filter_map(|e| e.ok()) {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_path = entry.path().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let is_hidden = file_name.starts_with('.');
            
            if !show_hidden && is_hidden {
                continue;
            }
            
            // 检查是否有子目录（用于懒加载）
            // 【注意】与Electron版本保持一致，不考虑show_hidden，只检查目录是否为空
            let has_children = if is_dir {
                match std::fs::read_dir(&entry.path()) {
                    Ok(rd) => {
                        let count = rd.count();
                        if file_name == "Applications" || file_name == "Library" || file_name == "Users" {
                            log_info!("目录 {} 有 {} 个子项", file_path, count);
                        }
                        count > 0
                    }
                    Err(e) => {
                        if file_name == "Applications" || file_name == "Library" || file_name == "Users" {
                            log_warn!("无法读取目录 {}: {}", file_path, e);
                        }
                        false
                    }
                }
            } else {
                false
            };
            
            nodes.push(DirectoryNode {
                path: file_path,
                name: file_name,
                is_dir,
                is_hidden,
                has_children,
                children: None, // 懒加载，不立即加载子节点
            });
        }
    }
    
    // 按名称排序，目录在前
    nodes.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });
    
    Ok(nodes)
}

/// 开始扫描
#[tauri::command]
pub async fn scan_start(
    config: ScanConfig,
    app: AppHandle,
    state: State<'_, ScanState>,
) -> Result<(), String> {
    let mut is_scanning = state.is_scanning.lock().map_err(|e| e.to_string())?;
    if *is_scanning {
        log_warn!("扫描正在进行中，拒绝新的扫描请求");
        return Err("扫描正在进行中".to_string());
    }
    *is_scanning = true;
    drop(is_scanning);
    
    log_info!("开始新的扫描任务");
    
    // 重置取消标志
    state.cancel_flag.store(false, Ordering::Relaxed);
    state.logs.lock().map_err(|e| e.to_string())?.clear();
    
    let cancel_flag = state.cancel_flag.clone();
    
    // 创建事件通道（增加缓冲区，避免阻塞）
    let (tx, mut rx) = mpsc::channel::<ScanEvent>(config::EVENT_CHANNEL_BUFFER_SIZE);
    
    // 启动扫描任务
    let app_clone_for_error = app.clone();
    tokio::spawn(async move {
        if let Err(e) = run_scan_safe(config, tx, cancel_flag).await {
            log_error!("扫描任务出错: {}", e);
            let _ = app_clone_for_error.emit("scan-error", e);
        }
    });
    
    // 处理事件
    let app_clone = app.clone();
    let logs_clone = state.logs.clone();
    let is_scanning_clone = state.is_scanning.clone();
    
    tokio::spawn(async move {
        let mut received_finished = false;
        
        // 设置超时，防止永远等待
        let timeout_duration = std::time::Duration::from_secs(config::SCAN_TIMEOUT_SECS);
        let start_time = std::time::Instant::now();
        
        // 【优化】日志节流：记录上次发送时间
        let mut last_log_time = std::time::Instant::now();
        let log_throttle = std::time::Duration::from_millis(config::LOG_THROTTLE_MS);
        
        loop {
            // 检查超时
            if start_time.elapsed() > timeout_duration {
                log_error!("扫描超时");
                if let Ok(mut is_scanning) = is_scanning_clone.lock() {
                    *is_scanning = false;
                }
                let _ = app_clone.emit("scan-error", "扫描超时");
                break;
            }
            
            tokio::select! {
                Some(event) = rx.recv() => {
                    
                    match event {
                        ScanEvent::Progress { current_file, scanned_count, total_count, filtered_count, skipped_count } => {
                            let mut progress_data = serde_json::json!({
                                "currentFile": current_file,
                                "scannedCount": scanned_count,
                                "totalCount": total_count,
                            });
                            // 【新增】只有当filtered_count和skipped_count有值时才添加
                            if let Some(fc) = filtered_count {
                                progress_data["filteredCount"] = serde_json::json!(fc);
                            }
                            if let Some(sc) = skipped_count {
                                progress_data["skippedCount"] = serde_json::json!(sc);
                            }
                            let _ = app_clone.emit("scan-progress", progress_data);
                        }
                        ScanEvent::BatchResult(items) => {
                            // 【优化】批量结果，一次性发送整个数组
                            let _ = app_clone.emit("scan-batch-result", items);
                        }
                        ScanEvent::Log(msg) => {
                            // 【优化】日志节流，但允许连续日志快速通过（初始阶段）
                            let now = std::time::Instant::now();
                            let time_since_last = now.duration_since(last_log_time);
                            
                            // 如果是刚开始扫描（3秒内），或者距离上次日志超过 100ms，则发送
                            let is_initial_phase = start_time.elapsed() < std::time::Duration::from_secs(config::INITIAL_LOG_PHASE_SECS);
                            if is_initial_phase || time_since_last >= log_throttle {
                                let _ = app_clone.emit("scan-log", msg.clone());
                                last_log_time = now;
                            }
                            
                            // 【优化】同步添加日志到内存，避免丢失
                            // 使用 try_lock 避免阻塞，如果锁被占用则跳过
                            if let Ok(mut l) = logs_clone.lock() {
                                l.push(msg);
                                // 限制日志数量，防止内存泄漏
                                let len = l.len();
                                if len > config::MAX_LOG_ENTRIES {
                                    l.drain(0..len - config::MAX_LOG_ENTRIES);
                                }
                            }
                        }
                        ScanEvent::Finished => {
                            log_info!("扫描完成，重置状态");
                            received_finished = true;
                            let _ = app_clone.emit("scan-finished", ());
                            if let Ok(mut is_scanning) = is_scanning_clone.lock() {
                                *is_scanning = false;
                            }
                            break;
                        }
                    }
                }
                else => {
                    // 通道关闭，扫描异常结束
                    log_warn!("扫描通道关闭，强制重置状态");
                    if let Ok(mut is_scanning) = is_scanning_clone.lock() {
                        *is_scanning = false;
                    }
                    break;
                }
            }
        }
        
        if !received_finished {
            log_warn!("扫描未正常结束，已强制重置状态");
        }
    });
    
    Ok(())
}

/// 取消扫描
#[tauri::command]
pub fn scan_cancel(state: State<'_, ScanState>) -> Result<bool, String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    Ok(true)
}

/// 预览文件
#[tauri::command]
pub async fn preview_file(path: String, max_bytes: Option<usize>) -> Result<PreviewResult, String> {
    preview::preview_file(path, max_bytes).await
}

/// 取消预览
#[tauri::command]
pub fn cancel_preview() -> Result<bool, String> {
    preview::cancel_preview()
}

/// 流式预览文件
#[tauri::command]
pub async fn preview_file_stream(
    path: String,
    app: AppHandle,
) -> Result<(), String> {
    preview::preview_file_stream(path, app).await
}

/// 读取文件内容为二进制数据（用于预览）
#[tauri::command]
pub async fn read_file_as_blob(path: String) -> Result<Vec<u8>, String> {
    use crate::utils::path_security::is_path_safe;
    
    log_info!("[read_file_as_blob] 收到请求，路径: {}", path);
    
    // 【简化】仅检查路径安全性（防路径遍历攻击），允许访问所有绝对路径的文件
    let check_result = is_path_safe(&path);
    if !check_result.is_allowed() {
        log_warn!("[read_file_as_blob] 路径安全检查失败: {:?}", check_result);
        return Err(format!("不允许访问该路径: {:?}", check_result));
    }
    
    log_info!("[read_file_as_blob] 路径安全检查通过，开始读取文件...");
    
    // 读取文件内容
    match tokio::fs::read(&path).await {
        Ok(data) => {
            log_info!("[read_file_as_blob] 成功读取文件，大小: {} bytes", data.len());
            Ok(data)
        }
        Err(e) => {
            log_error!("[read_file_as_blob] 读取文件失败: {}", e);
            Err(format!("读取文件失败: {}", e))
        }
    }
}

/// 打开文件
#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    // 【修复】对于已扫描的文件，直接允许打开，无需再次检查路径
    // 因为能出现在结果列表中，说明已经通过了扫描时的路径安全检查
    open::that(&path).map_err(|e| format!("无法打开文件: {}", e))
}

/// 打开文件所在目录
#[tauri::command]
pub fn open_file_location(path: String) -> Result<(), String> {
    // 【修复】对于已扫描的文件，直接允许打开目录，无需再次检查路径
    
    // 在不同平台上打开目录
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 explorer /select 选中文件
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("无法打开目录: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS: 使用 open -R 选中文件
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("无法打开目录: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::path::Path;
        
        // Linux: 使用 xdg-open 打开目录
        let path_obj = Path::new(&path);
        let parent = path_obj.parent()
            .ok_or_else(|| "无法获取文件所在目录".to_string())?;
        open::that(parent).map_err(|e| format!("无法打开目录: {}", e))?;
    }
    
    Ok(())
}

/// 删除文件（根据配置决定移入回收站或永久删除）
#[tauri::command]
pub fn delete_file(path: String) -> Result<(), String> {
    use crate::utils::path_security::is_path_safe;
    
    // 【简化】仅检查路径安全性（防路径遍历攻击），允许访问所有已扫描的文件
    let check_result = is_path_safe(&path);
    if !check_result.is_allowed() {
        return Err(format!("不允许访问该路径: {:?}", check_result));
    }
    
    let config = load_config().map_err(|e| format!("加载配置失败: {}", e))?;
    
    if config.delete_to_trash {
        // 移入回收站
        trash::delete(&path).map_err(|e| format!("删除失败: {}", e))
    } else {
        // 永久删除
        std::fs::remove_file(&path).map_err(|e| format!("删除失败: {}", e))
    }
}

/// 导出报告
#[tauri::command]
pub fn export_report(
    results: Vec<ScanResultItem>,
    format: String,
    save_path: String,
) -> Result<String, String> {
    // 【安全】验证导出路径安全性（允许写入用户选择的任何位置）
    let path = Path::new(&save_path);
    if !path.is_absolute() {
        return Err("导出路径必须是绝对路径".to_string());
    }
    
    match format.as_str() {
        "csv" => export_csv(&results, &save_path),
        "json" => export_json(&results, &save_path),
        "xlsx" => export_xlsx(&results, &save_path),
        _ => Err("不支持的格式".to_string()),
    }
}

fn export_csv(results: &[ScanResultItem], path: &str) -> Result<String, String> {
    use std::io::Write;
    
    // 【安全】CSV字段转义函数，防止公式注入
    fn escape_csv_field(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') || field.starts_with('=') || field.starts_with('+') || field.starts_with('-') || field.starts_with('@') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }
    
    let mut file = std::fs::File::create(path)
        .map_err(|e| format!("无法创建文件: {}", e))?;
    
    // 写入 CSV 头
    writeln!(file, "文件路径,文件大小,修改时间,身份证数,手机号数,邮箱数,银行卡数,地址数,IP地址数,密码数,总计").ok();
    
    for item in results {
        let person_id = item.counts.get("person_id").unwrap_or(&0);
        let phone = item.counts.get("phone").unwrap_or(&0);
        let email = item.counts.get("email").unwrap_or(&0);
        let bank_card = item.counts.get("bank_card").unwrap_or(&0);
        let address = item.counts.get("address").unwrap_or(&0);
        let ip_address = item.counts.get("ip_address").unwrap_or(&0);
        let password = item.counts.get("password").unwrap_or(&0);
        
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{},{}",
            escape_csv_field(&item.file_path),
            item.file_size,
            escape_csv_field(&item.modified_time),
            person_id,
            phone,
            email,
            bank_card,
            address,
            ip_address,
            password,
            item.total
        ).ok();
    }
    
    Ok(path.to_string())
}

fn export_json(results: &[ScanResultItem], path: &str) -> Result<String, String> {
    let json = serde_json::to_string_pretty(results)
        .map_err(|e| format!("JSON 序列化失败: {}", e))?;
    
    std::fs::write(path, json)
        .map_err(|e| format!("写入文件失败: {}", e))?;
    
    Ok(path.to_string())
}

fn export_xlsx(results: &[ScanResultItem], path: &str) -> Result<String, String> {
    use crate::utils::excel_export::{
        ExcelStyleConfig, create_header_style, create_cell_style, 
        create_sensitive_style, write_headers, write_data_row, auto_adjust_column_width
    };
    use rust_xlsxwriter::*;
    
    // 创建工作簿
    let mut workbook = Workbook::new();
    
    // 【关键】先创建所有样式，再获取worksheet（避免借用冲突）
    let config = ExcelStyleConfig::default();
    let header_style = create_header_style(&mut workbook, &config);
    let cell_style = create_cell_style(&mut workbook, &config);
    let sensitive_style = create_sensitive_style(&mut workbook, &config);
    
    // 现在才获取worksheet
    let worksheet = workbook.add_worksheet();
    
    // 准备表头和数据
    let headers = vec![
        "文件路径".to_string(),
        "文件大小 (字节)".to_string(),
        "修改时间".to_string(),
        "身份证".to_string(),
        "手机号".to_string(),
        "邮箱".to_string(),
        "银行卡".to_string(),
        "地址".to_string(),
        "IP地址".to_string(),
        "密码".to_string(),
        "总计".to_string(),
    ];
    
    // 写入表头
    write_headers(worksheet, &headers, &header_style)
        .map_err(|e| format!("写入表头失败: {}", e))?;
    
    // 准备数据行
    let sensitive_columns = vec![3, 4, 5, 6, 7, 8, 9, 10]; // 敏感数据列索引
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    
    for item in results {
        let person_id = item.counts.get("person_id").unwrap_or(&0).to_string();
        let phone = item.counts.get("phone").unwrap_or(&0).to_string();
        let email = item.counts.get("email").unwrap_or(&0).to_string();
        let bank_card = item.counts.get("bank_card").unwrap_or(&0).to_string();
        let address = item.counts.get("address").unwrap_or(&0).to_string();
        let ip_address = item.counts.get("ip_address").unwrap_or(&0).to_string();
        let password = item.counts.get("password").unwrap_or(&0).to_string();
        let total = item.total.to_string();
        
        let row = vec![
            item.file_path.clone(),
            item.file_size.to_string(),
            item.modified_time.clone(),
            person_id,
            phone,
            email,
            bank_card,
            address,
            ip_address,
            password,
            total,
        ];
        
        data_rows.push(row);
    }
    
    // 写入数据行
    for (row_idx, row_data) in data_rows.iter().enumerate() {
        write_data_row(
            worksheet,
            row_idx,
            row_data,
            &cell_style,
            &sensitive_columns,
            &sensitive_style,
        ).map_err(|e| format!("写入数据失败: {}", e))?;
    }
    
    // 自动调整列宽
    auto_adjust_column_width(worksheet, &headers, &data_rows);
    
    // 保存文件
    workbook.save(path)
        .map_err(|e| format!("保存 Excel 文件失败: {}", e))?;
    
    Ok(path.to_string())
}

/// 获取日志
#[tauri::command]
pub fn get_logs(state: State<'_, ScanState>) -> Result<Vec<String>, String> {
    let logs = state.logs.lock().map_err(|e| e.to_string())?;
    Ok(logs.clone())
}

/// 获取内置敏感规则
#[tauri::command]
pub fn get_sensitive_rules() -> Result<Vec<(String, String, bool)>, String> {
    Ok(get_builtin_rules())
}

/// 保存配置
#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path()?;
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化失败: {}", e))?;
    
    std::fs::write(&config_path, json)
        .map_err(|e| format!("写入配置失败: {}", e))?;
    
    Ok(())
}

/// 加载配置
#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    let config_path = get_config_path()?;
    
    if !Path::new(&config_path).exists() {
        // 【新增】首次运行时，使用当前平台的系统目录
        return Ok(AppConfig {
            system_dirs: crate::utils::system_dirs::generate_system_dirs(false),
            ..Default::default()
        });
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置失败: {}", e))?;
    
    let mut config: AppConfig = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置失败: {}", e))?;
    
    // 配置迁移：如果 system_dirs 为空，使用当前平台的默认值
    if config.system_dirs.is_empty() {
        config.system_dirs = crate::utils::system_dirs::generate_system_dirs(false);
    }
    
    Ok(config)
}

/// 获取配置文件路径
/// 【优化】统一使用 config.rs 中的逻辑，从 tauri.conf.json 的 identifier 读取
fn get_config_path() -> Result<String, String> {
    // 直接调用 config 模块的统一函数
    let path = config::get_config_file_path();
    Ok(path.to_string_lossy().to_string())
}

/// 检查系统环境
#[tauri::command]
pub fn check_system_environment() -> Result<crate::utils::environment::EnvironmentCheck, String> {
    Ok(crate::utils::environment::check_environment())
}

/// 获取推荐的并发数（根据 CPU 和内存智能计算）
#[tauri::command]
pub fn get_recommended_concurrency() -> Result<serde_json::Value, String> {
    use crate::utils::concurrency::calculate_recommended_concurrency;
    
    let info = calculate_recommended_concurrency();
    
    Ok(serde_json::json!({
        "recommended": info.actual_concurrency,
        "max_allowed": info.max_allowed_concurrency,
        "cpu_count": info.cpu_count,
        "free_memory_gb": format!("{:.1}", info.free_memory_gb)
    }))
}

// ==================== 新增命令：暴露工具模块功能 ====================

/// 显示消息对话框
#[tauri::command]
pub fn show_message_box(
    app: AppHandle,
    title: String,
    message: String,
    box_type: String,  // info/warning/error/confirm
) -> Result<bool, String> {
    use crate::utils::message_box::{MessageBoxConfig, show_message_box as show_mb};
    
    let config = match box_type.as_str() {
        "info" => MessageBoxConfig::info(&title, &message),
        "warning" => MessageBoxConfig::warning(&title, &message),
        "error" => MessageBoxConfig::error(&title, &message),
        "confirm" => MessageBoxConfig::confirm(&title, &message),
        _ => return Err("不支持的对话框类型".to_string()),
    };
    
    Ok(show_mb(&app, config))
}

/// 清理缓存
#[tauri::command]
pub fn clear_cache(
    app: AppHandle,
    clean_logs: bool,
    clean_temp: bool,
    log_retention_days: Option<u64>,
) -> Result<serde_json::Value, String> {
    use crate::utils::cache_cleanup::clear_cache as clear;
    
    let retention = log_retention_days.unwrap_or(30);
    let result = clear(&app, clean_logs, clean_temp, retention)?;
    
    // 使用 cache_cleanup 模块的 format_bytes 函数
    use crate::utils::cache_cleanup::format_bytes;
    
    Ok(serde_json::json!({
        "success": true,
        "directories_cleaned": result.directories_cleaned,
        "files_cleaned": result.files_cleaned,
        "space_freed_bytes": result.space_freed_bytes,
        "space_freed_formatted": format_bytes(result.space_freed_bytes),
        "details": result.details
    }))
}

/// 打开开发者工具（仅debug模式）
#[tauri::command]
pub fn open_dev_tools(app: AppHandle) -> Result<(), String> {
    // Tauri 2.x API
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(debug_assertions)]
        {
            window.open_devtools();
            log_info!("✅ 开发者工具已打开");
            Ok(())
        }
        
        #[cfg(not(debug_assertions))]
        {
            let _ = window; // 避免未使用警告
            Err("生产模式下不支持打开开发者工具".to_string())
        }
    } else {
        Err("主窗口不存在".to_string())
    }
}

// ==================== 【新增】自定义表达式搜索相关命令 ====================

/// 验证搜索表达式语法
#[tauri::command]
pub fn validate_search_expression(expression: String) -> Result<serde_json::Value, String> {
    use crate::utils::expression_parser::validate_expression;
    
    let result = validate_expression(&expression);
    
    Ok(serde_json::json!({
        "valid": result.valid,
        "error": result.error
    }))
}

/// 获取当前搜索表达式
#[tauri::command]
pub fn get_search_expression(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.search_expression.clone())
}

/// 设置搜索表达式
#[tauri::command]
pub fn set_search_expression(
    expression: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.search_expression = expression;
    // 保存配置到文件
    config::save_app_config(&config)?;
    Ok(())
}
