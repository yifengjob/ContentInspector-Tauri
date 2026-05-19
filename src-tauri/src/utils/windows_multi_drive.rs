#![allow(dead_code)]
/// Windows多磁盘系统目录配置模块
/// 
/// 提供Windows多磁盘环境下的系统目录忽略配置。
/// 
/// 功能：
/// - 检测所有可用磁盘驱动器
/// - 为每个驱动器生成系统目录列表
/// - 支持ignoreOtherDrivesSystemDirs选项

#[cfg(windows)]
use winreg::RegKey;

/// 获取Windows所有驱动器列表
#[cfg(windows)]
pub fn get_windows_drives() -> Vec<String> {
    let mut drives = Vec::new();
    
    // 通过注册表获取所有驱动器
    if let Ok(hklm) = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE) {
        if let Ok(system_disk) = hklm.open_subkey("SYSTEM\\MountedDevices") {
            // 这里简化实现，实际应该解析MountedDevices
            // 通常C盘是系统盘
            drives.push("C:".to_string());
        }
    }
    
    // 备选方案：检查常见驱动器
    for drive_letter in 'C'..='Z' {
        let drive = format!("{}:", drive_letter);
        if PathBuf::from(&drive).exists() {
            drives.push(drive);
        }
    }
    
    drives
}

/// 为指定驱动器生成系统目录列表
pub fn generate_system_dirs_for_drive(drive: &str) -> Vec<String> {
    vec![
        format!("{}\\Windows", drive),
        format!("{}\\WinNT", drive),
        format!("{}\\Program Files", drive),
        format!("{}\\Program Files (x86)", drive),
        format!("{}\\ProgramData", drive),
        format!("{}\\Recovery", drive),
        format!("{}\\$Recycle.Bin", drive),
        format!("{}\\System Volume Information", drive),
    ]
}

/// 获取所有驱动器的系统目录（当ignoreOtherDrivesSystemDirs为true时）
pub fn get_all_drives_system_dirs(ignore_other_drives: bool) -> Vec<String> {
    if !ignore_other_drives {
        // 只返回C盘系统目录
        return crate::utils::config::WINDOWS_SYSTEM_DIRS_C_DRIVE
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
    
    #[cfg(windows)]
    {
        let drives = get_windows_drives();
        let mut all_dirs = Vec::new();
        
        for drive in drives {
            all_dirs.extend(generate_system_dirs_for_drive(&drive));
        }
        
        all_dirs
    }
    
    #[cfg(not(windows))]
    {
        // 非Windows平台，返回空列表
        log::warn!("ignoreOtherDrivesSystemDirs仅在Windows平台有效");
        Vec::new()
    }
}

/// 验证ignoreOtherDrivesSystemDirs配置
pub fn validate_ignore_other_drives_config(enabled: bool) -> Result<Vec<String>, String> {
    if !enabled {
        return Ok(vec![]);
    }
    
    #[cfg(windows)]
    {
        let dirs = get_all_drives_system_dirs(true);
        if dirs.is_empty() {
            Err("未检测到任何驱动器".to_string())
        } else {
            Ok(dirs)
        }
    }
    
    #[cfg(not(windows))]
    {
        Err("此功能仅支持Windows平台".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_get_windows_drives() {
        let drives = get_windows_drives();
        assert!(!drives.is_empty());
        assert!(drives.iter().any(|d| d == "C:"));
    }

    #[test]
    fn test_generate_system_dirs_for_drive() {
        let dirs = generate_system_dirs_for_drive("D:");
        assert!(!dirs.is_empty());
        assert!(dirs.contains(&"D:\\Windows".to_string()));
        assert!(dirs.contains(&"D:\\Program Files".to_string()));
    }

    #[test]
    #[cfg(not(windows))]
    fn test_non_windows_platform() {
        let dirs = get_all_drives_system_dirs(true);
        assert!(dirs.is_empty());
    }
}
