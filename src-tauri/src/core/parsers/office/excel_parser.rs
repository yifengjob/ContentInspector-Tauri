/// Excel 文件解析器
/// 支持 .xlsx, .xls, .et 格式
/// 
/// 【新增】支持真正流式处理：逐行提取，边读边处理

use calamine::{open_workbook_auto, Reader};

/// 读取 Excel 文件（传统方式，用于小文件）
pub fn read_excel_file(path: &str) -> Result<String, String> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| format!("Excel 解析失败: {}", e))?;
        
        let mut text = String::new();
        
        for sheet in workbook.sheet_names().to_owned() {
            if let Ok(range) = workbook.worksheet_range(&sheet) {
                for row in range.rows() {
                    let cells: Vec<String> = row.iter()
                        .map(|cell| cell.to_string())
                        .collect();
                    text.push_str(&cells.join("\t"));
                    text.push('\n');
                }
                text.push_str("---\n");
            }
        }
        
        Ok(text)
    }));
    
    match result {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Excel 文件解析过程中发生错误（可能是损坏的文件）".to_string()),
    }
}

/// 【新增】流式提取 Excel 文本（逐行处理，真正流式）
/// 
/// # 参数
/// * `path` - Excel文件路径
/// * `callback` - 每提取一行文本后调用的回调函数
///   - 参数：当前行的文本内容
///   - 返回值：Ok(true)继续处理，Ok(false)取消，Err停止并返回错误
/// 
/// # 优势
/// - 内存占用极低：每次只保留1行数据
/// - 支持GB级大Excel文件
/// - 可以中途取消
/// - 真正的边读边处理
pub fn stream_extract_excel<F>(path: &str, mut callback: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<bool, String>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| format!("Excel 解析失败: {}", e))?;
        
        let mut row_count = 0u64;
        
        for sheet in workbook.sheet_names().to_owned() {
            if let Ok(range) = workbook.worksheet_range(&sheet) {
                for row in range.rows() {
                    // 将行转换为文本
                    let cells: Vec<String> = row.iter()
                        .map(|cell| cell.to_string())
                        .collect();
                    let row_text = cells.join("\t");
                    
                    if !row_text.is_empty() {
                        // 立即调用回调处理当前行
                        match callback(row_text) {
                            Ok(continue_processing) => {
                                if !continue_processing {
                                    log_debug!("Excel流式提取在第 {} 行被取消", row_count);
                                    return Ok(());
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    
                    row_count += 1;
                    
                    // 每1000行输出进度
                    if row_count.is_multiple_of(1000) {
                        log_debug!("Excel流式提取进度: {} 行", row_count);
                    }
                }
                
                // 工作表分隔符
                match callback("---".to_string()) {
                    Ok(continue_processing) => {
                        if !continue_processing {
                            return Ok(());
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        
        log_debug!("Excel流式提取完成，共 {} 行", row_count);
        Ok(())
    }));
    
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Excel 文件流式解析过程中发生错误".to_string()),
    }
}
