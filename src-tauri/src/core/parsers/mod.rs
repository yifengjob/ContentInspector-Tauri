/// 文件解析器模块
/// 
/// 每个解析器专注于一种文档格式的解析，遵循单一职责原则。
/// 便于后续新增解析器和维护现有解析器。

pub mod text_parser;
pub mod pdf_parser;
pub mod office;  // Office文档解析器集合（进一步拆分）

pub use office::{
    read_doc_file,
    read_docx_pptx_simple,
    read_excel_file,
    read_odp_file,
    read_ods_file,
    read_odt_file,
    read_ppt_file,
    read_rtf_file,
};
pub use pdf_parser::read_pdf_file;
// 重新导出常用函数，保持向后兼容
pub use text_parser::read_text_file;
