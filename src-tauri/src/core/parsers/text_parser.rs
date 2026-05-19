/// 文本文件解析器
/// 支持 UTF-8 和 GBK 编码自动检测

use std::fs;
use encoding_rs::GBK;

/// 读取文本文件内容，自动检测编码
pub fn read_text_file(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("无法读取文件: {}", e))?;
    
    // 尝试 UTF-8 解码
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        return Ok(text);
    }
    
    // 回退到 GBK
    let (text, _, had_errors) = GBK.decode(&bytes);
    if had_errors {
        return Err("文件编码无法识别".to_string());
    }
    
    Ok(text.into_owned())
}
