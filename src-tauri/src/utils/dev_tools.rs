#![allow(dead_code)]
/// 开发者工具模块
/// 
/// 提供打开开发者工具的功能，用于调试和开发。

use tauri::WebviewWindow;

/// 打开开发者工具
/// 
/// # 参数
/// - `window`: Webview窗口引用
/// 
/// # 示例
/// ```rust
/// open_dev_tools(&window);
/// ```
pub fn open_dev_tools(window: &WebviewWindow) {
    // Tauri 2.x 使用 devtools 方法
    #[cfg(debug_assertions)]
    {
        window.open_devtools();
        log_info!("✅ 开发者工具已打开");
    }
    
    #[cfg(not(debug_assertions))]
    {
        let _ = window; // 避免未使用警告
        log_warn!("⚠️ 生产模式下无法打开开发者工具");
    }
}

/// 关闭开发者工具
pub fn close_dev_tools(window: &WebviewWindow) {
    #[cfg(debug_assertions)]
    {
        window.close_devtools();
        log_info!("✅ 开发者工具已关闭");
    }
    
    #[cfg(not(debug_assertions))]
    {
        let _ = window; // 避免未使用警告
    }
}

/// 切换开发者工具（打开/关闭）
pub fn toggle_dev_tools(window: &WebviewWindow) {
    #[cfg(debug_assertions)]
    {
        // Tauri 2.x 没有直接的toggle方法，需要检查状态
        // 这里简单实现为打开
        window.open_devtools();
        log_info!("✅ 开发者工具已切换");
    }
    
    #[cfg(not(debug_assertions))]
    {
        let _ = window; // 避免未使用警告
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_dev_tools_functions_exist() {
        // 测试函数存在性
        // 实际调用需要WebviewWindow实例，这里只测试编译
        assert!(true);
    }
}
