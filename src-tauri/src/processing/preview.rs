/// 预览模块
/// 
/// 处理文件预览相关功能，包括传统预览和流式预览

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use lazy_static::lazy_static;

use crate::core::file_parser::extract_text_from_file;
use crate::core::sensitive_detector::{get_highlights, get_builtin_rules};
use crate::models::{PreviewResult, HighlightRange};
use crate::utils::config;

lazy_static! {
    /// 预览任务取消标志（只保留最新的）
    pub static ref LATEST_PREVIEW_CANCEL_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
}

/// 传统预览文件（一次性返回）
pub async fn preview_file(path: String, max_bytes: Option<usize>) -> Result<PreviewResult, String> {
    let max_bytes = max_bytes.unwrap_or(config::DEFAULT_PREVIEW_MAX_BYTES);
    
    log_debug!("开始预览任务");
    
    // 创建取消标志
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut latest_flag = LATEST_PREVIEW_CANCEL_FLAG.lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        *latest_flag = Some(cancel_flag.clone());
    }
    
    // 在后台线程执行
    let path_clone = path.clone();
    let cancel_flag_clone = cancel_flag.clone();
    let result = tokio::task::spawn_blocking(move || {
        if cancel_flag_clone.load(Ordering::Relaxed) {
            return Err("任务已取消".to_string());
        }
        extract_text_from_file(&path_clone)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| format!("文件读取失败: {}", e))?;
    
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("任务已取消".to_string());
    }
    
    let (text, unsupported_preview) = result;
    
    if unsupported_preview {
        return Ok(PreviewResult {
            content: "该文件类型不支持内容预览".to_string(),
            highlights: vec![],
        });
    }
    
    // 限制预览大小
    let truncated = if text.len() > max_bytes {
        let mut byte_idx = max_bytes;
        while byte_idx > 0 && !text.is_char_boundary(byte_idx) {
            byte_idx -= 1;
        }
        &text[..byte_idx]
    } else {
        &text
    };
    
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("任务已取消".to_string());
    }
    
    // 获取高亮
    let highlights = get_enabled_sensitive_types();
    let highlights_raw = get_highlights(truncated, &highlights);
    let highlights = highlights_raw.into_iter()
        .map(|(start, end, type_id, type_name)| HighlightRange {
            start,
            end,
            type_id,
            type_name,
        })
        .collect();
    
    Ok(PreviewResult {
        content: truncated.to_string(),
        highlights,
    })
}

/// 取消预览
pub fn cancel_preview() -> Result<bool, String> {
    let latest_flag = LATEST_PREVIEW_CANCEL_FLAG.lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    
    if let Some(flag) = latest_flag.as_ref() {
        flag.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 流式预览文件（分块返回）
pub async fn preview_file_stream(
    path: String,
    app: AppHandle,
) -> Result<(), String> {
    use crate::utils::file_types::{get_processor_type, FileProcessorType};
    use std::path::Path;
    
    log_info!("🔍 开始流式预览任务: {}", path);
    
    // 【修复】提取文件扩展名，而不是传入完整路径
    let ext = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    let processor_type = get_processor_type(ext);
    log_info!("📄 文件处理器类型: {:?} (扩展名: {})", processor_type, ext);
    
    if processor_type == FileProcessorType::BinaryScan {
        log_warn!("⚠️ 二进制文件不支持预览");
        send_binary_unsupported(&app)?;
        return Ok(());
    }
    
    // 创建取消标志
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut latest_flag = LATEST_PREVIEW_CANCEL_FLAG.lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        *latest_flag = Some(cancel_flag.clone());
    }
    
    let cancel_flag_clone = cancel_flag.clone();
    let path_clone = path.clone();
    let app_clone = app.clone();
    
    // 在后台执行
    tokio::spawn(async move {
        log_info!("🚀 启动后台预览任务");
        let result = if processor_type == FileProcessorType::StreamingText {
            log_info!("📝 使用文本流式读取");
            stream_read_text_file(&path_clone, &app_clone, &cancel_flag_clone).await
        } else {
            log_info!("📄 使用解析器流式读取");
            stream_read_parsed_file(&path_clone, &app_clone, &cancel_flag_clone).await
        };
        
        if let Err(e) = result {
            log_error!("❌ 流式预览失败: {}", e);
            let _ = app_clone.emit("preview-error", e);
        } else {
            log_info!("✅ 流式预览成功完成");
        }
    });
    
    log_info!("✅ preview_file_stream 函数返回");
    Ok(())
}

/// 发送二进制文件不支持预览的消息
fn send_binary_unsupported(app: &AppHandle) -> Result<(), String> {
    let json_data = create_unsupported_json();
    app.emit("preview-chunk", &json_data)
        .map_err(|e| format!("发送事件失败: {}", e))?;
    Ok(())
}

/// 获取启用的敏感类型
fn get_enabled_sensitive_types() -> Vec<String> {
    let rules = get_builtin_rules();
    rules.iter()
        .filter(|(_, _, enabled)| *enabled)
        .map(|(id, _, _)| id.clone())
        .collect()
}

/// 创建高亮JSON数组
fn create_highlights_json(content: &str) -> Vec<serde_json::Value> {
    let enabled_types = get_enabled_sensitive_types();
    let highlights_raw = get_highlights(content, &enabled_types);
    highlights_raw.into_iter()
        .map(|(start, end, type_id, type_name)| {
            serde_json::json!({
                "start": start,
                "end": end,
                "typeId": type_id,      // type_id → typeId
                "typeName": type_name,  // type_name → typeName
            })
        })
        .collect()
}

/// 创建不支持预览的JSON数据
fn create_unsupported_json() -> serde_json::Value {
    serde_json::json!({
        "chunkIndex": 0,
        "lines": ["该文件类型不支持内容预览"],
        "highlights": [],
        "startLine": 0,
        "totalLines": 1,
        "isLast": true,
    })
}

/// 流式读取文本文件
async fn stream_read_text_file(
    path: &str,
    app: &AppHandle,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), String> {
    use tokio::fs::File;
    use tokio::io::{BufReader, AsyncBufReadExt};
    
    log_info!("📖 打开文件: {}", path);
    let file = File::open(path)
        .await
        .map_err(|e| format!("无法打开文件: {}", e))?;
    
    let mut reader = BufReader::new(file);
    let mut buffer = String::new();
    let mut chunk_index = 0;
    let mut start_line = 0;
    const CHUNK_SIZE: usize = 1000;
    
    log_info!("📝 开始逐行读取...");
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            log_warn!("⚠️ 任务已取消");
            return Err("任务已取消".to_string());
        }
        
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)
            .await
            .map_err(|e| format!("读取失败: {}", e))?;
        
        if bytes_read == 0 {
            log_info!("📄 文件读取完成，剩余buffer大小: {}", buffer.len());
            if !buffer.is_empty() {
                log_info!("📤 发送最后一个数据块, chunk_index: {}", chunk_index);
                send_preview_chunk(app, chunk_index, &buffer, start_line, true)?;
            }
            break;
        }
        
        buffer.push_str(&line);
        
        let line_count = buffer.lines().count();
        if line_count >= CHUNK_SIZE {
            log_info!("📤 发送数据块, chunk_index: {}, lines: {}", chunk_index, line_count);
            send_preview_chunk(app, chunk_index, &buffer, start_line, false)?;
            start_line += line_count;
            chunk_index += 1;
            buffer.clear();
        }
    }
    
    log_info!("✅ 文本文件流式读取完成");
    Ok(())
}

/// 流式读取需要解析的文件
async fn stream_read_parsed_file(
    path: &str,
    app: &AppHandle,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), String> {
    let path_owned = path.to_string();
    let (text, unsupported) = tokio::task::spawn_blocking(move || {
        extract_text_from_file(&path_owned)
    })
    .await
    .map_err(|e| format!("解析失败: {}", e))?
    .map_err(|e| format!("文件读取失败: {}", e))?;
    
    if unsupported {
        let json_data = create_unsupported_json();
        app.emit("preview-chunk", &json_data).ok();
        return Ok(());
    }
    
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();
    const CHUNK_SIZE: usize = 1000;
    
    let mut chunk_index = 0;
    let mut start = 0;
    
    while start < total_lines {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("任务已取消".to_string());
        }
        
        let end = (start + CHUNK_SIZE).min(total_lines);
        let chunk_lines: Vec<String> = lines[start..end].iter().map(|s| s.to_string()).collect();
        let is_last = end >= total_lines;
        
        // 【修复】对每个chunk计算高亮，而不是硬编码空数组
        let chunk_text = chunk_lines.join("\n");
        let highlights = create_highlights_json(&chunk_text);
        
        let json_data = serde_json::json!({
            "chunkIndex": chunk_index,
            "lines": chunk_lines,
            "highlights": highlights,
            "startLine": start,
            "totalLines": total_lines,
            "isLast": is_last,
        });
        
        app.emit("preview-chunk", &json_data).ok();
        
        start = end;
        chunk_index += 1;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    Ok(())
}

/// 发送预览数据块
fn send_preview_chunk(
    app: &AppHandle,
    chunk_index: usize,
    content: &str,
    start_line: usize,
    is_last: bool,
) -> Result<(), String> {
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    
    // 使用统一的高亮函数
    let highlights = create_highlights_json(content);
    
    // 【关键修复】使用 camelCase 字段名，与前端保持一致
    let json_data = serde_json::json!({
        "chunkIndex": chunk_index,      // chunk_index → chunkIndex
        "lines": lines,
        "highlights": highlights,
        "startLine": start_line,        // start_line → startLine
        "totalLines": null,             // total_lines → totalLines
        "isLast": is_last,              // is_last → isLast
    });
    
    log_info!("📤 准备发送 preview-chunk 事件, chunkIndex: {}, lines: {}", chunk_index, lines.len());
    match app.emit("preview-chunk", &json_data) {
        Ok(_) => {
            log_info!("✅ preview-chunk 事件发送成功");
            Ok(())
        }
        Err(e) => {
            log_error!("❌ preview-chunk 事件发送失败: {}", e);
            Err(format!("发送事件失败: {}", e))
        }
    }
}
