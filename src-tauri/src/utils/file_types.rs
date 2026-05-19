#![allow(dead_code)]
/// 智能文件类型路由系统
/// 
/// 提供基于注册表的文件类型管理，支持：
/// - 文件处理器类型分类（流式文本、流式Office、预解析等）
/// - 扩展名到处理器的快速查找
/// - 文件类型的元数据（描述、图标、优先级等）
/// 
/// # 设计目标
/// - 与Electron版对齐的文件类型管理能力
/// - 易于扩展新的文件格式
/// - 支持不同处理策略（流式 vs 预解析）

use std::collections::HashMap;

/// 文件处理器类型枚举
/// 
/// 区分不同的处理策略：
/// - StreamingText: 真正流式逐行/逐块读取（txt/log/csv等）
/// - StreamingOffice: 真正流式逐段落/逐幻灯片提取（docx/pptx/odt等）
/// - PreExtracted: 需要预提取后分块处理（pdf/doc/ppt等）
/// - BinaryScan: 仅二进制扫描，不支持预览
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FileProcessorType {
    /// 流式文本文件（直接逐行读取）
    StreamingText,
    
    /// 流式Office文档（逐段落/逐幻灯片提取）
    StreamingOffice,
    
    /// 预提取文本（需要先完整解析，然后分块处理）
    PreExtracted,
    
    /// 仅二进制扫描（不支持预览和文本提取）
    BinaryScan,
}

impl FileProcessorType {
    /// 是否支持流式处理
    pub fn is_streaming(&self) -> bool {
        matches!(self, FileProcessorType::StreamingText | FileProcessorType::StreamingOffice)
    }
    
    /// 是否支持预览
    pub fn supports_preview(&self) -> bool {
        !matches!(self, FileProcessorType::BinaryScan)
    }
}

/// 文件类型配置
/// 
/// 包含文件类型的完整元数据
#[derive(Debug, Clone)]
pub struct FileTypeConfig {
    /// 文件处理器类型
    pub processor_type: FileProcessorType,
    
    /// 文件类型描述（用于UI显示）
    pub description: &'static str,
    
    /// 文件图标名称（前端使用）
    pub icon: &'static str,
    
    /// 是否默认启用
    pub enabled_by_default: bool,
    
    /// 处理优先级（数值越小优先级越高）
    pub priority: u8,
}

impl FileTypeConfig {
    /// 创建新的文件类型配置
    pub const fn new(
        processor_type: FileProcessorType,
        description: &'static str,
        icon: &'static str,
        enabled_by_default: bool,
        priority: u8,
    ) -> Self {
        Self {
            processor_type,
            description,
            icon,
            enabled_by_default,
            priority,
        }
    }
}

/// 文件类型注册表
/// 
/// 维护扩展名到文件类型配置的映射
pub struct FileTypeRegistry {
    /// 扩展名 -> 文件类型配置的映射
    map: HashMap<String, FileTypeConfig>,
}

impl FileTypeRegistry {
    /// 创建新的注册表并初始化所有已知文件类型
    pub fn new() -> Self {
        let mut registry = Self {
            map: HashMap::new(),
        };
        
        // 注册所有文件类型
        registry.register_all();
        
        registry
    }
    
    /// 注册所有已知文件类型
    fn register_all(&mut self) {
        // ==================== 流式文本文件 ====================
        
        // 纯文本和日志
        self.register("txt", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "纯文本文件",
            "file-text",
            true,
            1,
        ));
        
        self.register("log", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "日志文件",
            "file-log",
            true,
            1,
        ));
        
        // 编程语言源代码
        for ext in &["js", "ts", "py", "java", "c", "cpp", "go", "rs", "php", "rb", "swift"] {
            self.register(ext, FileTypeConfig::new(
                FileProcessorType::StreamingText,
                "源代码文件",
                "file-code",
                true,
                1,
            ));
        }
        
        // Web前端文件
        self.register("html", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "HTML文件",
            "file-html",
            true,
            1,
        ));
        
        self.register("htm", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "HTML文件",
            "file-html",
            true,
            1,
        ));
        
        // 配置文件
        for ext in &["ini", "conf", "cfg", "env", "properties", "toml"] {
            self.register(ext, FileTypeConfig::new(
                FileProcessorType::StreamingText,
                "配置文件",
                "file-config",
                true,
                1,
            ));
        }
        
        // 数据格式
        self.register("csv", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "CSV文件",
            "file-csv",
            true,
            1,
        ));
        
        self.register("json", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "JSON文件",
            "file-json",
            true,
            1,
        ));
        
        self.register("xml", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "XML文件",
            "file-xml",
            true,
            1,
        ));
        
        self.register("yaml", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "YAML文件",
            "file-yaml",
            true,
            1,
        ));
        
        self.register("yml", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "YAML文件",
            "file-yaml",
            true,
            1,
        ));
        
        // Markdown
        self.register("md", FileTypeConfig::new(
            FileProcessorType::StreamingText,
            "Markdown文件",
            "file-markdown",
            true,
            1,
        ));
        
        // 脚本文件
        for ext in &["sh", "cmd", "bat"] {
            self.register(ext, FileTypeConfig::new(
                FileProcessorType::StreamingText,
                "脚本文件",
                "file-script",
                true,
                1,
            ));
        }
        
        // ==================== PDF文件 ====================
        
        self.register("pdf", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "PDF文档",
            "file-pdf",
            true,
            2,
        ));
        
        // ==================== Office文档（流式）====================
        
        // Word文档（OOXML格式 - 真正流式）
        self.register("docx", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "Word文档",
            "file-word",
            true,
            2,
        ));
        
        // PowerPoint演示（OOXML格式 - 真正流式）
        self.register("pptx", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "PowerPoint演示",
            "file-powerpoint",
            true,
            2,
        ));
        
        // OpenDocument Text（真正流式）
        self.register("odt", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "OpenDocument文本",
            "file-document",
            true,
            2,
        ));
        
        // OpenDocument Spreadsheet（真正流式）
        self.register("ods", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "OpenDocument表格",
            "file-spreadsheet",
            true,
            2,
        ));
        
        // OpenDocument Presentation（真正流式）
        self.register("odp", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "OpenDocument演示",
            "file-presentation",
            true,
            2,
        ));
        
        // RTF富文本（真正流式）
        self.register("rtf", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "RTF富文本文档",
            "file-rich-text",
            true,
            2,
        ));
        
        // ==================== Office文档（预提取）====================
        
        // Excel表格（真正流式逐行）
        self.register("xlsx", FileTypeConfig::new(
            FileProcessorType::StreamingOffice,
            "Excel表格",
            "file-excel",
            true,
            2,
        ));
        
        // Word旧版二进制（分块流式）
        self.register("doc", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "Word文档（旧版）",
            "file-word",
            true,
            3,
        ));
        
        // WPS文字
        self.register("wps", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "WPS文字",
            "file-word",
            true,
            3,
        ));
        
        // PowerPoint旧版二进制（分块流式）
        self.register("ppt", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "PowerPoint演示（旧版）",
            "file-powerpoint",
            true,
            3,
        ));
        
        // WPS演示
        self.register("dps", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "WPS演示",
            "file-powerpoint",
            true,
            3,
        ));
        
        // Excel旧版（预提取）
        self.register("xls", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "Excel表格（旧版）",
            "file-excel",
            true,
            3,
        ));
        
        // WPS表格
        self.register("et", FileTypeConfig::new(
            FileProcessorType::PreExtracted,
            "WPS表格",
            "file-excel",
            true,
            3,
        ));
        
        // // ==================== 二进制文件（不支持预览）====================
        //
        // // 压缩文件
        // for ext in &["zip", "rar", "7z", "tar", "gz"] {
        //     self.register(ext, FileTypeConfig::new(
        //         FileProcessorType::BinaryScan,
        //         "压缩文件",
        //         "file-archive",
        //         false,
        //         4,
        //     ));
        // }
    }
    
    /// 注册单个文件类型
    fn register(&mut self, ext: &str, config: FileTypeConfig) {
        self.map.insert(ext.to_lowercase(), config);
    }
    
    /// 根据扩展名获取文件类型配置
    pub fn get_by_extension(&self, ext: &str) -> Option<&FileTypeConfig> {
        self.map.get(&ext.to_lowercase())
    }
    
    /// 根据扩展名获取处理器类型
    pub fn get_processor_type(&self, ext: &str) -> FileProcessorType {
        self.get_by_extension(ext)
            .map(|config| config.processor_type)
            .unwrap_or(FileProcessorType::BinaryScan)
    }
    
    /// 检查扩展名是否支持预览
    pub fn supports_preview(&self, ext: &str) -> bool {
        self.get_by_extension(ext)
            .map(|config| config.processor_type.supports_preview())
            .unwrap_or(false)
    }
    
    /// 检查扩展名是否支持流式处理
    pub fn is_streaming(&self, ext: &str) -> bool {
        self.get_by_extension(ext)
            .map(|config| config.processor_type.is_streaming())
            .unwrap_or(false)
    }
    
    /// 获取所有已注册的扩展名
    pub fn registered_extensions(&self) -> Vec<&str> {
        self.map.keys().map(|k| k.as_str()).collect()
    }
    
    /// 获取注册的文件类型数量
    pub fn len(&self) -> usize {
        self.map.len()
    }
    
    /// 检查注册表是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ==================== 全局单例注册表 ====================

use std::sync::OnceLock;

/// 全局文件类型注册表（懒加载单例）
static FILE_TYPE_REGISTRY: OnceLock<FileTypeRegistry> = OnceLock::new();

/// 获取全局文件类型注册表
pub fn get_registry() -> &'static FileTypeRegistry {
    FILE_TYPE_REGISTRY.get_or_init(FileTypeRegistry::new)
}

/// 根据扩展名获取文件类型配置（便捷函数）
pub fn get_file_type_config(ext: &str) -> Option<&'static FileTypeConfig> {
    get_registry().get_by_extension(ext)
}

/// 根据扩展名获取处理器类型（便捷函数）
pub fn get_processor_type(ext: &str) -> FileProcessorType {
    get_registry().get_processor_type(ext)
}

/// 检查扩展名是否支持预览（便捷函数）
pub fn supports_preview(ext: &str) -> bool {
    get_registry().supports_preview(ext)
}

/// 检查扩展名是否支持流式处理（便捷函数）
pub fn is_streaming(ext: &str) -> bool {
    get_registry().is_streaming(ext)
}

/// 【新增】检查文件是否支持扫描
/// 
/// 判断标准：
/// 1. 文件扩展名在已注册的文件类型列表中
/// 2. 文件处理器类型不是BinaryScan（即有文本解析器）
/// 
/// 支持扫描的文件类型：
/// - StreamingText: txt, log, csv, json, 代码文件等
/// - StreamingOffice: docx, xlsx, pptx, odt等
/// - PreExtracted: pdf, doc, xls, ppt等
/// 
/// 不支持扫描的文件类型：
/// - BinaryScan: jpg, png, zip, rar等（纯二进制文件）
/// - 未注册的扩展名：xyz等未知格式
pub fn supports_scanning(file_path: &str) -> bool {
    use std::path::Path;
    
    // 提取扩展名
    if let Some(ext) = Path::new(file_path).extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        
        // 检查是否在注册表中
        if let Some(config) = get_registry().get_by_extension(&ext_lower) {
            // 检查处理器类型是否为BinaryScan
            !matches!(config.processor_type, FileProcessorType::BinaryScan)
        } else {
            false
        }
    } else {
        false
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry_initialization() {
        let registry = get_registry();
        assert!(!registry.is_empty());
        assert!(registry.len() > 30); // 应该注册了至少30种文件类型
    }
    
    #[test]
    fn test_text_file_detection() {
        assert_eq!(get_processor_type("txt"), FileProcessorType::StreamingText);
        assert_eq!(get_processor_type("log"), FileProcessorType::StreamingText);
        assert_eq!(get_processor_type("py"), FileProcessorType::StreamingText);
        assert_eq!(get_processor_type("js"), FileProcessorType::StreamingText);
    }
    
    #[test]
    fn test_pdf_detection() {
        assert_eq!(get_processor_type("pdf"), FileProcessorType::PreExtracted);
    }
    
    #[test]
    fn test_office_streaming_detection() {
        assert_eq!(get_processor_type("docx"), FileProcessorType::StreamingOffice);
        assert_eq!(get_processor_type("pptx"), FileProcessorType::StreamingOffice);
        assert_eq!(get_processor_type("odt"), FileProcessorType::StreamingOffice);
        assert_eq!(get_processor_type("xlsx"), FileProcessorType::StreamingOffice);
    }
    
    #[test]
    fn test_office_preextracted_detection() {
        assert_eq!(get_processor_type("doc"), FileProcessorType::PreExtracted);
        assert_eq!(get_processor_type("ppt"), FileProcessorType::PreExtracted);
        assert_eq!(get_processor_type("xls"), FileProcessorType::PreExtracted);
    }
    
    #[test]
    fn test_binary_scan_detection() {
        assert_eq!(get_processor_type("zip"), FileProcessorType::BinaryScan);
        assert_eq!(get_processor_type("rar"), FileProcessorType::BinaryScan);
        assert_eq!(get_processor_type("7z"), FileProcessorType::BinaryScan);
    }
    
    #[test]
    fn test_unknown_extension() {
        assert_eq!(get_processor_type("xyz"), FileProcessorType::BinaryScan);
        assert!(!supports_preview("xyz"));
    }
    
    #[test]
    fn test_case_insensitive() {
        assert_eq!(get_processor_type("TXT"), FileProcessorType::StreamingText);
        assert_eq!(get_processor_type("Pdf"), FileProcessorType::PreExtracted);
        assert_eq!(get_processor_type("DOCX"), FileProcessorType::StreamingOffice);
    }
    
    #[test]
    fn test_streaming_check() {
        assert!(is_streaming("txt"));
        assert!(is_streaming("docx"));
        assert!(is_streaming("xlsx"));
        assert!(!is_streaming("pdf"));
        assert!(!is_streaming("doc"));
    }
    
    #[test]
    fn test_preview_support() {
        assert!(supports_preview("txt"));
        assert!(supports_preview("pdf"));
        assert!(supports_preview("docx"));
        assert!(!supports_preview("zip"));
        assert!(!supports_preview("rar"));
    }
    
    #[test]
    fn test_file_type_config() {
        let config = get_file_type_config("txt").unwrap();
        assert_eq!(config.description, "纯文本文件");
        assert_eq!(config.icon, "file-text");
        assert!(config.enabled_by_default);
        assert_eq!(config.priority, 1);
    }
    
    #[test]
    fn test_supports_scanning() {
        // 支持扫描的文件类型
        assert!(supports_scanning("file.txt"));
        assert!(supports_scanning("document.pdf"));
        assert!(supports_scanning("report.docx"));
        assert!(supports_scanning("data.xlsx"));
        assert!(supports_scanning("script.py"));
        
        // 不支持扫描的文件类型（BinaryScan）
        assert!(!supports_scanning("photo.jpg"));
        assert!(!supports_scanning("image.png"));
        assert!(!supports_scanning("archive.zip"));
        assert!(!supports_scanning("backup.rar"));
        
        // 未注册的扩展名
        assert!(!supports_scanning("unknown.xyz"));
        assert!(!supports_scanning("file.abc"));
    }
}
