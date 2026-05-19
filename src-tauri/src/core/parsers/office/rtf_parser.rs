/// RTF 富文本格式解析器
/// 支持 .rtf 格式（Rich Text Format）
/// 
/// 【核心策略】使用 rtf-parser 库进行高质量解析
/// 【前端流式】按段落分块发送，实现前端流式体验

use rtf_parser::RtfDocument;

/// 读取 RTF 文件（使用 rtf-parser 库）
pub fn read_rtf_file(path: &str) -> Result<String, String> {
    let doc = RtfDocument::from_filepath(path)
        .map_err(|e| format!("RTF 解析失败: {}", e))?;
    
    let text = doc.get_text();
    
    if text.trim().is_empty() {
        return Err("RTF 文件中未提取到文本内容".to_string());
    }
    
    Ok(text)
}

/// 【新增】流式提取 RTF 文本（前端流式：按段落分块回调）
/// 
/// # 参数
/// * `path` - RTF文件路径
/// * `_chunk_size` - 保留参数（未使用，因为rtf-parser一次性解析）
/// * `callback` - 每提取一个段落后调用的回调函数
///   - 参数：当前段落的文本内容
///   - 返回值：Ok(true)继续处理，Ok(false)取消，Err停止并返回错误
/// 
/// # 说明
/// rtf-parser 库内部是一次性解析整个文档，但我们按段落分块发送给前端，
/// 实现“前端流式”体验。
pub fn stream_extract_rtf<F>(path: &str, _chunk_size: usize, mut callback: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<bool, String>,
{
    let doc = RtfDocument::from_filepath(path)
        .map_err(|e| format!("RTF 解析失败: {}", e))?;
    
    let text = doc.get_text();
    
    // 按段落分割并逐个回调
    let paragraphs: Vec<&str> = text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    
    let total = paragraphs.len();
    for (idx, para) in paragraphs.iter().enumerate() {
        match callback(para.to_string()) {
            Ok(continue_processing) => {
                if !continue_processing {
                    log::debug!("RTF流式提取在第 {}/{} 段被取消", idx + 1, total);
                    return Ok(());
                }
            }
            Err(e) => return Err(e),
        }
    }
    
    log::debug!("RTF流式提取完成，共 {} 段", total);
    Ok(())
}
