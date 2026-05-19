/// OpenDocument 格式解析器
/// 支持 .odt, .ods, .odp 格式（LibreOffice/OpenOffice）
/// 
/// 【核心策略】统一使用 litchi 库进行高质量文本提取

use std::fs;
use std::io::Read;
use super::msoffice_parser::{strip_xml_tags, extract_with_litchi};

/// 读取 ODT 文件（OpenDocument Text - 类似 Word）
pub fn read_odt_file(path: &str) -> Result<String, String> {
    extract_with_litchi(path)
}

/// 读取 ODS 文件（OpenDocument Spreadsheet - 类似 Excel）
pub fn read_ods_file(path: &str) -> Result<String, String> {
    extract_with_litchi(path)
}

/// 读取 ODP 文件（OpenDocument Presentation - 类似 PowerPoint）
pub fn read_odp_file(path: &str) -> Result<String, String> {
    extract_with_litchi(path)
}

/// 从 ODS XML 内容中提取表格数据
#[allow(dead_code)]
fn extract_ods_content(xml_content: &str) -> String {
    let mut result = String::new();
    let mut in_cell = false;
    let mut current_cell = String::new();
    let mut cell_count = 0;
    
    for line in xml_content.lines() {
        let trimmed = line.trim();
        
        if trimmed.contains("<text:p>") {
            let text = strip_xml_tags(trimmed);
            if !text.trim().is_empty() {
                result.push_str(&text);
                result.push('\n');
            }
        }
        
        if trimmed.contains("<table:table-cell>") {
            in_cell = true;
            current_cell.clear();
        }
        
        if in_cell {
            current_cell.push_str(line);
            current_cell.push('\n');
        }
        
        if trimmed.contains("</table:table-cell>") {
            in_cell = false;
            let cell_text = strip_xml_tags(&current_cell);
            if !cell_text.trim().is_empty() {
                result.push_str(cell_text.trim());
                result.push('\t');
                cell_count += 1;
            }
        }
        
        if trimmed.contains("</table:table-row>") && cell_count > 0 {
            result.push('\n');
            cell_count = 0;
        }
    }
    
    result
}

// ==================== 流式提取函数 ====================

/// 【新增】流式提取 ODT 文本（逐段落处理，真正流式）
pub fn stream_extract_odt<F>(path: &str, mut callback: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<bool, String>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let file = fs::File::open(path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("ZIP 解析失败: {}", e))?;
        
        let mut file = archive.by_name("content.xml")
            .map_err(|e| format!("无法读取 content.xml: {}", e))?;
        
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("读取 XML 内容失败: {}", e))?;
        
        // 按 <text:p> 标签分割段落
        let mut paragraph_count = 0u64;
        let mut remaining = content.as_str();
        
        while let Some(start) = remaining.find("<text:p") {
            if let Some(end) = remaining[start..].find("</text:p>") {
                let paragraph_xml = &remaining[start..start + end + 11];
                let paragraph_text = strip_xml_tags(paragraph_xml);
                
                if !paragraph_text.trim().is_empty() {
                    match callback(paragraph_text.trim().to_string()) {
                        Ok(continue_processing) => {
                            if !continue_processing {
                                log_debug!("ODT流式提取在第 {} 段被取消", paragraph_count);
                                return Ok(());
                            }
                        }
                        Err(e) => return Err(e),
                    }
                    
                    paragraph_count += 1;
                    
                    if paragraph_count.is_multiple_of(100) {
                        log_debug!("ODT流式提取进度: {} 段", paragraph_count);
                    }
                }
                
                remaining = &remaining[start + end + 11..];
            } else {
                break;
            }
        }
        
        log_debug!("ODT流式提取完成，共 {} 段", paragraph_count);
        Ok(())
    }));
    
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("ODT 文件流式解析过程中发生错误".to_string()),
    }
}

/// 【新增】流式提取 ODS 文本（逐行处理，真正流式）
pub fn stream_extract_ods<F>(path: &str, mut callback: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<bool, String>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let file = fs::File::open(path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("ZIP 解析失败: {}", e))?;
        
        let mut file = archive.by_name("content.xml")
            .map_err(|e| format!("无法读取 content.xml: {}", e))?;
        
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("读取 XML 内容失败: {}", e))?;
        
        // 按行提取
        let mut row_count = 0u64;
        let mut in_row = false;
        let mut current_row = String::new();
        let mut cell_count = 0;
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.contains("<table:table-row>") {
                in_row = true;
                current_row.clear();
                cell_count = 0;
            }
            
            if in_row && trimmed.contains("<table:table-cell>") {
                // 提取单元格内容
                if let Some(cell_start) = trimmed.find("<text:p>")
                    && let Some(cell_end) = trimmed[cell_start..].find("</text:p>") {
                        let cell_text = strip_xml_tags(&trimmed[cell_start..cell_start + cell_end + 9]);
                        if !cell_text.trim().is_empty() {
                            if cell_count > 0 {
                                current_row.push('\t');
                            }
                            current_row.push_str(cell_text.trim());
                            cell_count += 1;
                        }
                    }
            }
            
            if trimmed.contains("</table:table-row>") && in_row {
                in_row = false;
                
                if !current_row.is_empty() {
                    match callback(current_row.clone()) {
                        Ok(continue_processing) => {
                            if !continue_processing {
                                log_debug!("ODS流式提取在第 {} 行被取消", row_count);
                                return Ok(());
                            }
                        }
                        Err(e) => return Err(e),
                    }
                    
                    row_count += 1;
                    
                    if row_count.is_multiple_of(1000) {
                        log_debug!("ODS流式提取进度: {} 行", row_count);
                    }
                }
            }
        }
        
        log_debug!("ODS流式提取完成，共 {} 行", row_count);
        Ok(())
    }));
    
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("ODS 文件流式解析过程中发生错误".to_string()),
    }
}

/// 【新增】流式提取 ODP 文本（逐幻灯片处理，真正流式）
pub fn stream_extract_odp<F>(path: &str, mut callback: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<bool, String>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let file = fs::File::open(path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("ZIP 解析失败: {}", e))?;
        
        let mut file = archive.by_name("content.xml")
            .map_err(|e| format!("无法读取 content.xml: {}", e))?;
        
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("读取 XML 内容失败: {}", e))?;
        
        // 按 <draw:page> 标签分割幻灯片
        let mut slide_count = 0u64;
        let mut remaining = content.as_str();
        
        while let Some(start) = remaining.find("<draw:page") {
            if let Some(end) = remaining[start..].find("</draw:page>") {
                let slide_xml = &remaining[start..start + end + 12];
                let slide_text = strip_xml_tags(slide_xml);
                
                if !slide_text.trim().is_empty() {
                    match callback(slide_text.trim().to_string()) {
                        Ok(continue_processing) => {
                            if !continue_processing {
                                log_debug!("ODP流式提取在第 {} 张幻灯片被取消", slide_count);
                                return Ok(());
                            }
                        }
                        Err(e) => return Err(e),
                    }
                    
                    slide_count += 1;
                    
                    log_debug!("ODP流式提取进度: {} 张幻灯片", slide_count);
                }
                
                remaining = &remaining[start + end + 12..];
            } else {
                break;
            }
        }
        
        log_debug!("ODP流式提取完成，共 {} 张幻灯片", slide_count);
        Ok(())
    }));
    
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("ODP 文件流式解析过程中发生错误".to_string()),
    }
}
