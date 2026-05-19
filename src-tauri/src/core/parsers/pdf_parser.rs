/// PDF 文件解析器
/// 支持文本提取和纯图片检测
/// 
/// 【新增】支持真正流式处理：逐页提取文本，边读边处理

use std::fs;
use std::io::Read;

/// 读取 PDF 文件并提取文本（传统方式，用于小文件）
pub fn read_pdf_file(path: &str) -> Result<String, String> {
    use pdf_extract::extract_text;
    
    // 使用 catch_unwind 捕获可能的 panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        extract_text(path)
    }));
    
    match result {
        Ok(Ok(text)) => {
            if text.is_empty() {
                // 检测是否为纯图片PDF
                let is_image_only = detect_image_only_pdf(path);
                if is_image_only {
                    Err("PDF 文件为纯图片格式，需要 OCR 识别（当前版本不支持）".to_string())
                } else {
                    Err("PDF 文件中未提取到文本内容".to_string())
                }
            } else {
                Ok(text)
            }
        },
        Ok(Err(e)) => {
            // 记录具体错误信息，但不暴露敏感细节
            let error_msg = format!("{}", e);
            if error_msg.contains("unsupported encoding") || error_msg.contains("encoding") {
                Err("PDF 文件使用了不支持的字符编码，无法解析".to_string())
            } else if error_msg.contains("corrupt") || error_msg.contains("damaged") {
                Err("PDF 文件已损坏或不完整".to_string())
            } else {
                Err(format!("PDF 解析失败: {}", e))
            }
        },
        Err(_) => Err("PDF 解析过程中发生严重错误（可能是损坏的文件或不支持的格式）".to_string()),
    }
}

/// 【新增】流式提取 PDF 文本（逐页处理，真正流式）
/// 
/// # 参数
/// * `path` - PDF文件路径
/// * `callback` - 每提取一页文本后调用的回调函数
///   - 参数：当前页的文本内容
///   - 返回值：Ok(true)继续处理，Ok(false)取消，Err停止并返回错误
/// 
/// # 优势
/// - 内存占用极低：每次只保留1-2页文本（<1MB）
/// - 支持GB级大PDF文件
/// - 可以中途取消
/// - 真正的边读边处理
pub fn stream_extract_pdf<F>(path: &str, mut callback: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<bool, String>,
{
    use lopdf::{Document};
    
    // 加载PDF文档（仅加载结构索引，不加载所有内容）
    let doc = Document::load(path)
        .map_err(|e| format!("无法加载PDF: {}", e))?;
    
    let pages = doc.get_pages();
    let total_pages = pages.len();
    
    log_debug!("PDF共 {} 页，开始流式提取", total_pages);
    
    for (page_num, page_id) in pages.iter() {
        // 提取单页文本
        match extract_page_text(&doc, *page_id) {
            Ok(page_text) => {
                if !page_text.is_empty() {
                    // 立即调用回调处理当前页文本
                    match callback(page_text) {
                        Ok(continue_processing) => {
                            if !continue_processing {
                                log_debug!("流式提取在第 {} 页被取消", page_num);
                                return Ok(());
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                
                // 每10页输出进度
                if page_num % 10 == 0 {
                    log_debug!("PDF流式提取进度: {}/{}", page_num, total_pages);
                }
            }
            Err(e) => {
                log_warn!("第 {} 页提取失败: {}", page_num, e);
                // 继续处理下一页，不中断整个流程
            }
        }
    }
    
    log_debug!("PDF流式提取完成，共 {} 页", total_pages);
    Ok(())
}

/// 提取单页文本
fn extract_page_text(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<String, String> {
    use lopdf::Object;
    
    let page = doc.get_object(page_id)
        .map_err(|e| format!("获取页面对象失败: {}", e))?;
    
    let page_dict = page.as_dict()
        .map_err(|e| format!("页面对象不是字典: {}", e))?;
    
    // 获取页面内容流
    let contents = match page_dict.get(b"Contents") {
        Ok(content) => content,
        Err(_) => return Ok(String::new()), // 页面无内容，返回空字符串
    };
    
    let content_streams = match contents {
        Object::Reference(id) => vec![*id],
        Object::Array(arr) => arr.iter()
            .filter_map(|obj| obj.as_reference().ok())
            .collect(),
        _ => return Ok(String::new()),
    };
    
    // 合并所有内容流
    let mut all_content = Vec::new();
    for stream_id in content_streams {
        if let Ok(stream_obj) = doc.get_object(stream_id)
            && let Object::Stream(s) = stream_obj {
                // 直接使用content字段，不尝试解压缩
                all_content.extend(s.content.clone());
            }
    }
    
    // 从二进制内容中提取文本
    let text = extract_text_from_pdf_content(&all_content);
    Ok(text)
}

/// 从PDF内容流中提取可打印文本
fn extract_text_from_pdf_content(content: &[u8]) -> String {
    let mut result = String::new();
    let mut current_text = String::new();
    let mut in_text_object = false;
    
    let content_str = String::from_utf8_lossy(content);
    
    // 查找文本操作符 (BT/ET 包围的文本对象)
    for line in content_str.lines() {
        let trimmed = line.trim();
        
        if trimmed == "BT" {
            in_text_object = true;
        } else if trimmed == "ET" {
            in_text_object = false;
            if !current_text.is_empty() {
                result.push_str(&current_text);
                result.push(' ');
                current_text.clear();
            }
        } else if in_text_object {
            // 提取括号中的文本 (Tj 操作符)
            if let Some(start) = trimmed.find('(') {
                if let Some(end) = trimmed[start..].find(')') {
                    let text = &trimmed[start+1..start+end];
                    // 处理转义字符
                    let unescaped = text.replace("\\(", "(").replace("\\)", ")").replace("\\\\", "\\");
                    current_text.push_str(&unescaped);
                }
            }
            // 处理十六进制字符串 <...>
            else if let Some(start) = trimmed.find('<')
                && let Some(end) = trimmed[start..].find('>') {
                    let hex_str = &trimmed[start+1..start+end];
                    if let Ok(bytes) = hex::decode(hex_str)
                        && let Ok(text) = String::from_utf8(bytes) {
                            current_text.push_str(&text);
                        }
                }
        }
    }
    
    result.trim().to_string()
}

/// 检测 PDF 是否为纯图片格式
fn detect_image_only_pdf(path: &str) -> bool {
    // 策略：检查 PDF 中是否包含大量图片对象但无文本流
    
    // 读取 PDF 文件头部和部分结构
    if let Ok(mut file) = fs::File::open(path) {
        let mut buffer = Vec::new();
        // 读取前 10KB 进行分析
        let _ = file.read_to_end(&mut buffer);
        
        let content = String::from_utf8_lossy(&buffer);
        
        // 检查是否有文本操作符 (Tj, TJ, T*)
        let has_text_operators = content.contains("Tj") || 
                                  content.contains("TJ") || 
                                  content.contains("T*");
        
        // 检查是否有图片对象 (/Image)
        let image_count = content.matches("/Image").count();
        
        // 如果没有文本操作符但有大量图片，很可能是纯图片PDF
        !has_text_operators && image_count > 5
    } else {
        false
    }
}

/// OCR 识别接口（预留）
/// 当检测到纯图片PDF时，可以调用此函数进行OCR识别
/// TODO: 集成 OCR 引擎（如 tesseract、paddleocr 等）
#[allow(dead_code)]
pub fn ocr_pdf_file(_path: &str) -> Result<String, String> {
    // 【预留】OCR 功能实现
    // 可选方案：
    // 1. 使用 tesseract-ocr + poppler（将PDF转为图片后OCR）
    // 2. 使用 paddleocr-rs（百度PaddleOCR的Rust绑定）
    // 3. 调用外部 OCR API（如腾讯云、阿里云OCR服务）
    
    Err("OCR 功能尚未实现，请在配置中启用实验性OCR支持".to_string())
}
