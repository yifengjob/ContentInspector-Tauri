/// Office 文档解析器子模块
/// 
/// 按格式类型进一步拆分，每个子模块专注一类文档格式。

pub mod excel_parser;        // Excel (.xlsx, .xls, .et)
pub mod msoffice_parser;     // Microsoft Office (.docx, .pptx, .doc, .ppt, .wps)
pub mod opendocument_parser; // OpenDocument (.odt, .ods, .odp)
pub mod rtf_parser;          // RTF (.rtf)

// 重新导出所有解析函数
pub use excel_parser::read_excel_file;
pub use msoffice_parser::{read_docx_pptx_simple, read_doc_file, read_ppt_file};
pub use opendocument_parser::{read_odt_file, read_ods_file, read_odp_file};
pub use rtf_parser::read_rtf_file;
