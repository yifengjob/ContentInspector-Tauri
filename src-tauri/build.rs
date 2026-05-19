use std::fs;
use std::path::Path;

fn main() {
    tauri_build::build();
    
    // 【优化】从 tauri.conf.json 读取 identifier 并生成编译时常量
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let config_path = Path::new(&manifest_dir).join("tauri.conf.json");
    
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)
            .expect("无法读取 tauri.conf.json");
        
        // 简单解析 JSON 获取 identifier
        if let Some(identifier) = extract_json_string(&config_content, "identifier") {
            println!("cargo:rustc-env=TAURI_APP_IDENTIFIER={}", identifier);
        } else {
            // 降级方案：使用默认值
            println!("cargo:rustc-env=TAURI_APP_IDENTIFIER=com.content.inspector");
        }
    } else {
        // 配置文件不存在，使用默认值
        println!("cargo:rustc-env=TAURI_APP_IDENTIFIER=com.content.inspector");
    }
}

/// 简单提取 JSON 字符串值（避免引入 serde 依赖）
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let search_key = format!("\"{}\":", key);
    if let Some(start_pos) = json.find(&search_key) {
        let after_key = &json[start_pos + search_key.len()..];
        // 跳过空白字符
        let trimmed = after_key.trim_start();
        if trimmed.starts_with('"') {
            // 找到结束引号
            if let Some(end_pos) = trimmed[1..].find('"') {
                return Some(trimmed[1..end_pos + 1].to_string());
            }
        }
    }
    None
}
