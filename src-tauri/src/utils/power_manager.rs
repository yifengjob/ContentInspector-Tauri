#![allow(dead_code)]
/// 电源管理模块（防止锁屏/休眠）
/// 
/// 在扫描过程中阻止系统进入睡眠或锁屏状态，确保扫描不被中断。
/// 
/// 功能：
/// - 启动扫描时阻止系统休眠
/// - 扫描完成或取消时恢复系统正常休眠
/// - 跨平台支持（Windows/macOS/Linux）

use nosleep::{NoSleep, NoSleepType};
use std::sync::{Arc, Mutex};

/// 电源管理器
pub struct PowerManager {
    /// NoSleep实例（Option用于控制生命周期）
    nosleep: Option<Arc<Mutex<NoSleep>>>,
    /// 是否已激活
    is_active: bool,
}

impl PowerManager {
    /// 创建新的电源管理器
    pub fn new() -> Self {
        Self {
            nosleep: None,
            is_active: false,
        }
    }

    /// 启动电源阻止（防止系统休眠）
    /// 
    /// # 参数
    /// - `prevent_display_sleep`: 是否阻止屏幕关闭
    /// - `prevent_system_sleep`: 是否阻止系统休眠
    /// 
    /// # 返回
    /// - `Ok(())`: 成功启动
    /// - `Err(String)`: 错误信息
    /// 
    /// # 示例
    /// ```rust
    /// let mut pm = PowerManager::new();
    /// pm.start(true, false)?; // 只阻止屏幕关闭，允许系统休眠
    /// ```
    pub fn start(&mut self, prevent_display_sleep: bool, prevent_system_sleep: bool) -> Result<(), String> {
        if self.is_active {
            log::warn!("电源管理器已激活，无需重复启动");
            return Ok(());
        }

        // 确定阻止类型
        let sleep_type = if prevent_display_sleep && prevent_system_sleep {
            NoSleepType::PreventUserIdleSystemSleep
        } else if prevent_display_sleep {
            NoSleepType::PreventUserIdleDisplaySleep
        } else {
            // 如果只阻止系统休眠，使用PreventUserIdleSystemSleep
            NoSleepType::PreventUserIdleSystemSleep
        };

        // 创建NoSleep实例
        match NoSleep::new() {
            Ok(nosleep) => {
                let nosleep = Arc::new(Mutex::new(nosleep));
                
                // 启动阻止
                if let Err(e) = nosleep.lock().unwrap().start(sleep_type) {
                    return Err(format!("启动电源阻止失败: {}", e));
                }

                self.nosleep = Some(nosleep);
                self.is_active = true;

                let display_msg = if prevent_display_sleep { "锁屏" } else { "" };
                let system_msg = if prevent_system_sleep { "和休眠" } else { "" };
                log::info!("✅ 电源阻止已启动 - 防止系统{}{}", display_msg, system_msg);

                Ok(())
            }
            Err(e) => {
                Err(format!("创建NoSleep实例失败: {}", e))
            }
        }
    }

    /// 停止电源阻止（恢复系统正常休眠）
    /// 
    /// # 返回
    /// - `Ok(())`: 成功停止
    /// - `Err(String)`: 错误信息
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_active {
            log::warn!("电源管理器未激活，无需停止");
            return Ok(());
        }

        if let Some(nosleep) = self.nosleep.take() {
            // 停止阻止（drop会自动释放）
            drop(nosleep);
            
            self.is_active = false;
            
            log::info!("🔓 电源阻止已停止 - 系统恢复正常休眠");
            
            Ok(())
        } else {
            Err("NoSleep实例不存在".to_string())
        }
    }

    /// 检查电源阻止是否已激活
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// 获取当前状态描述
    pub fn status(&self) -> String {
        if self.is_active {
            "电源阻止已激活 - 系统将不会进入休眠或锁屏".to_string()
        } else {
            "电源阻止未激活 - 系统按正常设置休眠".to_string()
        }
    }
}

impl Drop for PowerManager {
    fn drop(&mut self) {
        // 确保在销毁时停止电源阻止
        if self.is_active {
            let _ = self.stop();
        }
    }
}

/// 全局电源管理器（可选，用于跨模块共享）
pub struct GlobalPowerManager {
    manager: Arc<Mutex<PowerManager>>,
}

impl GlobalPowerManager {
    /// 创建全局电源管理器
    pub fn new() -> Self {
        Self {
            manager: Arc::new(Mutex::new(PowerManager::new())),
        }
    }

    /// 启动电源阻止
    pub fn start(&self, prevent_display_sleep: bool, prevent_system_sleep: bool) -> Result<(), String> {
        let mut manager = self.manager.lock().unwrap();
        manager.start(prevent_display_sleep, prevent_system_sleep)
    }

    /// 停止电源阻止
    pub fn stop(&self) -> Result<(), String> {
        let mut manager = self.manager.lock().unwrap();
        manager.stop()
    }

    /// 检查是否已激活
    pub fn is_active(&self) -> bool {
        let manager = self.manager.lock().unwrap();
        manager.is_active()
    }
}

impl Clone for GlobalPowerManager {
    fn clone(&self) -> Self {
        Self {
            manager: self.manager.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_manager_creation() {
        let pm = PowerManager::new();
        assert!(!pm.is_active());
        assert_eq!(pm.status(), "电源阻止未激活 - 系统按正常设置休眠");
    }

    #[test]
    fn test_power_manager_start_stop() {
        let mut pm = PowerManager::new();
        
        // 启动电源阻止（仅阻止屏幕关闭）
        let result = pm.start(true, false);
        
        // 注意：在某些测试环境中可能无法真正启动NoSleep
        // 所以这里只检查返回值是否为Result类型
        match result {
            Ok(()) => {
                assert!(pm.is_active());
                // 停止
                let stop_result = pm.stop();
                assert!(stop_result.is_ok());
                assert!(!pm.is_active());
            }
            Err(_) => {
                // 如果启动失败（例如在无头环境中），也是可接受的
                assert!(!pm.is_active());
            }
        }
    }

    #[test]
    fn test_power_manager_double_start() {
        let mut pm = PowerManager::new();
        
        // 第一次启动
        let _ = pm.start(true, false);
        
        // 第二次启动应该返回Ok但警告
        let result = pm.start(true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_power_manager_stop_without_start() {
        let mut pm = PowerManager::new();
        
        // 未启动时停止应该返回Ok但警告
        let result = pm.stop();
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_power_manager() {
        let gpm = GlobalPowerManager::new();
        assert!(!gpm.is_active());
        
        // 克隆后仍然指向同一个管理器
        let gpm_clone = gpm.clone();
        assert!(!gpm_clone.is_active());
    }
}
