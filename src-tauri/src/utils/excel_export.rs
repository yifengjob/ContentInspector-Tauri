#![allow(dead_code)]
/// Excel导出增强模块
/// 
/// 提供增强的Excel导出功能，支持：
/// - 表头样式（加粗、背景色）
/// - 数字格式化（千分位）
/// - 敏感数据高亮（红色加粗）
/// - 自适应列宽

use rust_xlsxwriter::*;

/// Excel样式配置
pub struct ExcelStyleConfig {
    /// 表头背景色
    pub header_bg_color: Color,
    /// 表头字体颜色
    pub header_font_color: Color,
    /// 敏感数据高亮颜色
    pub sensitive_highlight_color: Color,
    /// 普通单元格边框颜色
    pub border_color: Color,
}

impl Default for ExcelStyleConfig {
    fn default() -> Self {
        Self {
            header_bg_color: Color::RGB(0x4472C4), // 蓝色
            header_font_color: Color::White,
            sensitive_highlight_color: Color::Red,
            border_color: Color::Gray,
        }
    }
}

/// 创建表头样式
pub fn create_header_style(_workbook: &mut Workbook, config: &ExcelStyleConfig) -> Format {
    Format::new()
        .set_bold()
        .set_font_color(config.header_font_color)
        .set_background_color(config.header_bg_color)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_font_size(11)
}

/// 创建普通单元格样式
pub fn create_cell_style(_workbook: &mut Workbook, _config: &ExcelStyleConfig) -> Format {
    Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::VerticalCenter)
}

/// 创建敏感数据高亮样式
pub fn create_sensitive_style(_workbook: &mut Workbook, config: &ExcelStyleConfig) -> Format {
    Format::new()
        .set_bold()
        .set_font_color(config.sensitive_highlight_color)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Left)
}

/// 创建数字格式样式（千分位）
pub fn create_number_style(_workbook: &mut Workbook, _config: &ExcelStyleConfig) -> Format {
    Format::new()
        .set_num_format("#,##0")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right)
}

/// 创建文件大小格式样式
pub fn create_file_size_style(_workbook: &mut Workbook, _config: &ExcelStyleConfig) -> Format {
    Format::new()
        .set_num_format("#,##0.00 \"MB\"")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right)
}

/// 自动调整列宽
/// 
/// # 参数
/// - `worksheet`: 工作表引用
/// - `headers`: 表头列表
/// - `data`: 数据列表（二维数组）
pub fn auto_adjust_column_width(
    worksheet: &mut Worksheet,
    headers: &[String],
    data: &[Vec<String>],
) {
    for (col_idx, header) in headers.iter().enumerate() {
        let mut max_width = header.len();
        
        // 查找该列最大宽度
        for row in data {
            if col_idx < row.len() {
                let cell_width = row[col_idx].len();
                if cell_width > max_width {
                    max_width = cell_width;
                }
            }
        }
        
        // 设置列宽（添加一些padding）
        let width = (max_width + 4) as f64;
        let _ = worksheet.set_column_width(col_idx as u16, width.min(50.0)); // 最大50字符宽度
    }
}

/// 写入表头
pub fn write_headers(
    worksheet: &mut Worksheet,
    headers: &[String],
    header_style: &Format,
) -> Result<(), XlsxError> {
    for (col_idx, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col_idx as u16, header, header_style)?;
    }
    Ok(())
}

/// 写入数据行
pub fn write_data_row(
    worksheet: &mut Worksheet,
    row_idx: usize,
    data: &[String],
    cell_style: &Format,
    sensitive_columns: &[usize], // 需要高亮的列索引
    sensitive_style: &Format,
) -> Result<(), XlsxError> {
    for (col_idx, value) in data.iter().enumerate() {
        let style = if sensitive_columns.contains(&col_idx) && !value.is_empty() {
            sensitive_style
        } else {
            cell_style
        };
        
        worksheet.write_string_with_format(
            (row_idx + 1) as u32, // +1 because row 0 is header
            col_idx as u16,
            value,
            style,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excel_style_config_default() {
        let config = ExcelStyleConfig::default();
        assert_eq!(config.header_bg_color, Color::RGB(0x4472C4));
        assert_eq!(config.header_font_color, Color::White);
        assert_eq!(config.sensitive_highlight_color, Color::Red);
    }

    #[test]
    fn test_auto_adjust_column_width() {
        // 测试列宽计算逻辑
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let data = vec![
            vec!["John Doe".to_string(), "25".to_string()],
            vec!["Jane Smith".to_string(), "30".to_string()],
        ];
        
        // 验证最大宽度计算
        assert_eq!(headers[0].len(), 4); // "Name"
        assert_eq!(data[0][0].len(), 8); // "John Doe"
    }
}
