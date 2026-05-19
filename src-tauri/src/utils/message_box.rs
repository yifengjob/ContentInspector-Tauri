#![allow(dead_code)]
/// 消息对话框模块（UI增强）
/// 
/// 提供统一的消息对话框API，支持多种类型和按钮配置。
/// 
/// 功能：
/// - 信息提示对话框
/// - 警告对话框
/// - 错误对话框
/// - 确认对话框（是/否）
/// - 自定义按钮文本

use tauri::{AppHandle};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// 消息对话框类型
#[derive(Debug, Clone)]
pub enum MessageBoxType {
    /// 信息提示
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 确认（是/否）
    Confirm,
    /// 自定义
    Custom {
        yes_text: String,
        no_text: String,
    },
}

/// 消息对话框配置
#[derive(Debug, Clone)]
pub struct MessageBoxConfig {
    /// 标题
    pub title: String,
    /// 消息内容
    pub message: String,
    /// 对话框类型
    pub box_type: MessageBoxType,
}

impl MessageBoxConfig {
    /// 创建信息对话框配置
    pub fn info(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            box_type: MessageBoxType::Info,
        }
    }

    /// 创建警告对话框配置
    pub fn warning(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            box_type: MessageBoxType::Warning,
        }
    }

    /// 创建错误对话框配置
    pub fn error(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            box_type: MessageBoxType::Error,
        }
    }

    /// 创建确认对话框配置
    pub fn confirm(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            box_type: MessageBoxType::Confirm,
        }
    }

    /// 创建自定义按钮对话框配置
    pub fn custom(title: &str, message: &str, yes_text: &str, no_text: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            box_type: MessageBoxType::Custom {
                yes_text: yes_text.to_string(),
                no_text: no_text.to_string(),
            },
        }
    }
}

/// 显示消息对话框
/// 
/// # 参数
/// - `app_handle`: Tauri应用句柄
/// - `config`: 对话框配置
/// 
/// # 返回
/// - `true`: 用户点击了"是"/"确定"/"OK"
/// - `false`: 用户点击了"否"/"取消"/关闭对话框
/// 
/// # 示例
/// ```rust
/// let config = MessageBoxConfig::confirm("确认删除", "确定要删除这个文件吗？");
/// let result = show_message_box(&app_handle, config);
/// if result {
///     // 执行删除操作
/// }
/// ```
pub fn show_message_box(app_handle: &AppHandle, config: MessageBoxConfig) -> bool {
    // 转换对话框类型
    let (kind, buttons) = match &config.box_type {
        MessageBoxType::Info => (MessageDialogKind::Info, MessageDialogButtons::Ok),
        MessageBoxType::Warning => (MessageDialogKind::Warning, MessageDialogButtons::Ok),
        MessageBoxType::Error => (MessageDialogKind::Error, MessageDialogButtons::Ok),
        MessageBoxType::Confirm => (MessageDialogKind::Warning, MessageDialogButtons::YesNo),
        MessageBoxType::Custom { yes_text: _, no_text: _ } => {
            // 自定义按钮文本（Tauri dialog插件目前不支持自定义按钮文本）
            // 使用YesNo作为替代
            log::warn!("自定义按钮文本暂不支持，使用默认Yes/No");
            (MessageDialogKind::Warning, MessageDialogButtons::YesNo)
        }
    };

    // 显示对话框并等待用户响应
    app_handle
        .dialog()
        .message(&config.message)
        .title(&config.title)
        .kind(kind)
        .buttons(buttons)
        .blocking_show()
}

/// 异步显示消息对话框
/// 
/// 与`show_message_box`类似，但不会阻塞当前线程
/// 
/// # 参数
/// - `app_handle`: Tauri应用句柄
/// - `config`: 对话框配置
/// - `callback`: 用户响应回调函数
pub async fn show_message_box_async<F>(
    app_handle: &AppHandle,
    config: MessageBoxConfig,
    callback: F,
) where
    F: FnOnce(bool) + Send + 'static,
{
    let kind = match &config.box_type {
        MessageBoxType::Info | MessageBoxType::Warning => MessageDialogKind::Info,
        MessageBoxType::Error => MessageDialogKind::Error,
        MessageBoxType::Confirm | MessageBoxType::Custom { .. } => MessageDialogKind::Warning,
    };

    let buttons = match &config.box_type {
        MessageBoxType::Info | MessageBoxType::Warning | MessageBoxType::Error => {
            MessageDialogButtons::Ok
        }
        MessageBoxType::Confirm | MessageBoxType::Custom { .. } => MessageDialogButtons::YesNo,
    };

    app_handle
        .dialog()
        .message(&config.message)
        .title(config.title)
        .kind(kind)
        .buttons(buttons)
        .show(callback);
}

/// 便捷函数：显示简单的信息提示
pub fn show_info(app_handle: &AppHandle, title: &str, message: &str) {
    let config = MessageBoxConfig::info(title, message);
    show_message_box(app_handle, config);
}

/// 便捷函数：显示简单的警告
pub fn show_warning(app_handle: &AppHandle, title: &str, message: &str) {
    let config = MessageBoxConfig::warning(title, message);
    show_message_box(app_handle, config);
}

/// 便捷函数：显示简单的错误
pub fn show_error(app_handle: &AppHandle, title: &str, message: &str) {
    let config = MessageBoxConfig::error(title, message);
    show_message_box(app_handle, config);
}

/// 便捷函数：显示确认对话框
pub fn show_confirm(app_handle: &AppHandle, title: &str, message: &str) -> bool {
    let config = MessageBoxConfig::confirm(title, message);
    show_message_box(app_handle, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_box_config_creation() {
        // 测试各种配置的创建
        let info_config = MessageBoxConfig::info("测试", "这是信息");
        assert_eq!(info_config.title, "测试");
        assert_eq!(info_config.message, "这是信息");
        assert!(matches!(info_config.box_type, MessageBoxType::Info));

        let warning_config = MessageBoxConfig::warning("警告", "这是警告");
        assert!(matches!(warning_config.box_type, MessageBoxType::Warning));

        let error_config = MessageBoxConfig::error("错误", "这是错误");
        assert!(matches!(error_config.box_type, MessageBoxType::Error));

        let confirm_config = MessageBoxConfig::confirm("确认", "是否继续？");
        assert!(matches!(confirm_config.box_type, MessageBoxType::Confirm));

        let custom_config = MessageBoxConfig::custom("自定义", "请选择", "确定", "取消");
        assert!(matches!(custom_config.box_type, MessageBoxType::Custom { .. }));
    }

    #[test]
    fn test_message_box_type_variants() {
        // 测试所有对话框类型变体
        let types = vec![
            MessageBoxType::Info,
            MessageBoxType::Warning,
            MessageBoxType::Error,
            MessageBoxType::Confirm,
            MessageBoxType::Custom {
                yes_text: "是".to_string(),
                no_text: "否".to_string(),
            },
        ];

        assert_eq!(types.len(), 5);
    }
}
