/// 文件解析统一入口
/// 
/// 根据文件扩展名路由到对应的解析器模块。
/// 本模块负责调度和错误处理，具体解析逻辑在各子模块中实现。
/// 
/// 【新增】支持流式处理模式，降低大文件内存占用

use std::path::Path;
use crate::utils::config;

// 导入解析器模块
use crate::core::parsers;

// 【新增】导入流式处理器
use crate::processing::file_stream_processor::{FileStreamProcessor, StreamProcessorConfig};

/// 根据扩展名路由到Office文件的对应解析器
fn read_office_file(path: &str, ext: &str) -> Result<String, String> {
    match ext {
        "xlsx" | "xls" | "et" => parsers::read_excel_file(path),
        "docx" | "pptx" => parsers::read_docx_pptx_simple(path),
        "doc" | "wps" => parsers::read_doc_file(path),
        "ppt" => parsers::read_ppt_file(path),
        "odt" => parsers::read_odt_file(path),
        "ods" => parsers::read_ods_file(path),
        "odp" => parsers::read_odp_file(path),
        "rtf" => parsers::read_rtf_file(path),
        _ => Err(format!("不支持的 Office 格式: {}", ext)),
    }
}

/// 从文件中提取文本的统一接口
/// 
/// # 参数
/// * `path` - 文件路径
/// 
/// # 返回
/// * `Ok((text, unsupported_preview))` - 文本内容和是否不支持预览
/// * `Err(message)` - 错误信息
/// 
/// # 示例
/// ```no_run
/// let (text, _) = extract_text_from_file("document.pdf").unwrap();
/// ```
pub fn extract_text_from_file(path: &str) -> Result<(String, bool), String> {
    let path_obj = Path::new(path);
    let ext = path_obj.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // 检查是否为不支持预览的文件类型
    if config::UNSUPPORTED_PREVIEW_EXTENSIONS.contains(&ext.as_str()) {
        return Ok(("".to_string(), true));
    }
    
    // 根据文件处理器类型路由
    let handler = match config::FileHandler::from_extension(&ext) {
        Some(h) => h,
        None => return Err(format!("不支持的文件格式: {}", ext)),
    };
    
    // 调用对应的解析器
    let text = match handler {
        config::FileHandler::Text => parsers::read_text_file(path)?,
        config::FileHandler::Pdf => parsers::read_pdf_file(path)?,
        config::FileHandler::Office => read_office_file(path, &ext)?,
    };
    
    Ok((text, false))
}

/// 【新增】流式处理文件（用于大文件扫描）
/// 
/// # 参数
/// * `path` - 文件路径
/// * `enabled_types` - 启用的敏感数据类型
/// 
/// # 返回
/// * `Ok(ProcessStats)` - 处理统计信息
/// * `Err(message)` - 错误信息
/// 
/// # 优势
/// - 内存峰值控制在 ~5MB（与文件大小无关）
/// - 支持GB级大文件处理
/// - 滑动窗口重叠策略，防止漏检跨边界敏感词
/// - PDF文件采用真正流式逐页提取
pub async fn extract_text_streaming(
    path: &str,
    enabled_types: &[String],
) -> Result<crate::processing::file_stream_processor::ProcessStats, String> {
    let path_obj = Path::new(path);
    let ext = path_obj.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // 检查是否为不支持预览的文件类型
    if config::UNSUPPORTED_PREVIEW_EXTENSIONS.contains(&ext.as_str()) {
        return Err("不支持的文件类型".to_string());
    }
    
    // 根据文件处理器类型路由
    let handler = match config::FileHandler::from_extension(&ext) {
        Some(h) => h,
        None => return Err(format!("不支持的文件格式: {}", ext)),
    };
    
    let mut processor = FileStreamProcessor::new();
    let stream_config = StreamProcessorConfig {
        enabled_types: enabled_types.to_vec(),
        preview_mode: false,
        enable_builtin_rules: true, // 默认启用
        search_expression: None,
    };
    
    // 对于需要预解析的文件类型，选择处理方式
    match handler {
        config::FileHandler::Text => {
            // 文本文件：直接流式读取（真正流式）
            processor.process_file(path, &stream_config, None)
                .await
                .map_err(|e| format!("流式处理失败: {}", e))
        }
        config::FileHandler::Pdf => {
            // PDF文件：真正流式逐页提取
            processor.process_pdf_streaming(path, &stream_config)
                .map_err(|e| format!("PDF流式处理失败: {}", e))
        }
        config::FileHandler::Office => {
            // Office文件：根据扩展名选择流式处理方式
            match ext.as_str() {
                "xlsx" | "xls" | "et" => {
                    // Excel：真正流式逐行提取
                    processor.process_excel_streaming(path, &stream_config)
                        .map_err(|e| format!("Excel流式处理失败: {}", e))
                }
                "docx" | "pptx" => {
                    // Word/PowerPoint (OOXML)：真正流式逐段落/逐幻灯片提取
                    processor.process_office_streaming(path, &stream_config)
                        .map_err(|e| format!("Office文件流式处理失败: {}", e))
                }
                "doc" | "wps" => {
                    // Word (旧版二进制)：分块流式处理
                    processor.process_doc_streaming(path, &stream_config)
                        .map_err(|e| format!("DOC文件流式处理失败: {}", e))
                }
                "ppt" => {
                    // PowerPoint (旧版二进制)：分块流式处理
                    processor.process_ppt_streaming(path, &stream_config)
                        .map_err(|e| format!("PPT文件流式处理失败: {}", e))
                }
                "odt" => {
                    // OpenDocument Text：真正流式逐段落提取
                    processor.process_odt_streaming(path, &stream_config)
                        .map_err(|e| format!("ODT文件流式处理失败: {}", e))
                }
                "ods" => {
                    // OpenDocument Spreadsheet：真正流式逐行提取
                    processor.process_ods_streaming(path, &stream_config)
                        .map_err(|e| format!("ODS文件流式处理失败: {}", e))
                }
                "odp" => {
                    // OpenDocument Presentation：真正流式逐幻灯片提取
                    processor.process_odp_streaming(path, &stream_config)
                        .map_err(|e| format!("ODP文件流式处理失败: {}", e))
                }
                "rtf" => {
                    // RTF富文本格式：真正流式逐段落提取
                    processor.process_rtf_streaming(path, &stream_config)
                        .map_err(|e| format!("RTF文件流式处理失败: {}", e))
                }
                _ => {
                    // 其他Office格式：回退到传统方式
                    let text = read_office_file(path, &ext)?;
                    processor.process_file(path, &stream_config, Some(text))
                        .await
                        .map_err(|e| format!("Office文件处理失败: {}", e))
                }
            }
        }
    }
}
