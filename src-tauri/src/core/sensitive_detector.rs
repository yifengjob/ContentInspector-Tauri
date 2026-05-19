use regex::Regex;
use std::collections::HashMap;
use lazy_static::lazy_static;
use chrono::Datelike;

lazy_static! {
    /// 内置敏感数据规则
    static ref BUILTIN_RULES: Vec<SensitiveRuleDef> = vec![
        SensitiveRuleDef {
            id: "person_id",
            name: "身份证号",
            // 18位身份证：前17位数字，最后1位数字或X
            // 注意：Rust regex不支持look-around，需要在代码中过滤前后数字
            pattern: r"\d{17}[\dXx]",
            enabled_by_default: true,
            has_validation: true,  // 【新增】标记需要额外验证
        },
        SensitiveRuleDef {
            id: "phone",
            name: "手机号",
            // 中国大陆手机号：1开头，第二位3-9，共11位
            // 注意：Rust regex不支持look-around，需要在代码中过滤
            pattern: r"1[3-9]\d{9}",
            enabled_by_default: true,
            has_validation: false,
        },
        SensitiveRuleDef {
            id: "email",
            name: "电子邮箱",
            // 标准邮箱格式：用户名@域名.顶级域名
            pattern: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
            enabled_by_default: true,
            has_validation: false,
        },
        SensitiveRuleDef {
            id: "bank_card",
            name: "银行卡号",
            // 银行卡号：以特定卡BIN开头，16-19位
            // 常见卡BIN：
            // - 银联借记卡：62开头
            // - Visa：4开头
            // - MasterCard：51-55或2开头
            // - 银联信用卡：62、60等
            pattern: r"(?:62|60|4|5[1-5]|2[2-7])\d{14,18}",
            enabled_by_default: true,
            has_validation: true,  // 【新增】需要Luhn校验
        },
        SensitiveRuleDef {
            id: "name",
            name: "中文姓名",
            // 2-4个连续汉字（易误报，默认关闭）
            pattern: r"[\u4e00-\u9fa5]{2,4}",
            enabled_by_default: false,
            has_validation: false,
        },
        SensitiveRuleDef {
            id: "address",
            name: "地址",
            // 极其严格的地址匹配：必须是真实的中国行政区划格式
            // 核心要求：必须包含“XX路/街/道”或“XX号”等明确地址标识
            // 模式1: XX省XX市XX区XX路XX号
            // 模式2: XX市XX区XX路XX号
            // 模式3: XX市XX县XX镇
            pattern: r"(?:[\u4e00-\u9fa5]{2,4}(?:省|自治区))?[\u4e00-\u9fa5]{2,4}(?:市|自治州|地区|盟)(?:[\u4e00-\u9fa5]{2,4}(?:区|县|市|旗))?(?:[\u4e00-\u9fa5]{2,10}(?:路|街|道|巷|胡同|里|弄|桥|广场|镇|乡))(?:[\d]+(?:号|栋|楼|单元|室|房)?)?",
            enabled_by_default: true,
            has_validation: false,
        },
        SensitiveRuleDef {
            id: "ip_address",
            name: "IPv4地址",
            // IPv4地址：每段0-255，用点分隔
            pattern: r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b",
            enabled_by_default: true,
            has_validation: true,  // 【新增】需要格式验证
        },
        SensitiveRuleDef {
            id: "password",
            name: "密码关键词",
            // 匹配 password/pwd/passwd/密码 后面跟着 := 和值的模式
            pattern: r"(?i)(?:password|pwd|passwd|密码)\s*[:=]\s*\S+",
            enabled_by_default: true,
            has_validation: false,
        },
        // 【新增】统一社会信用代码
        SensitiveRuleDef {
            id: "unified_social_credit_code",
            name: "统一社会信用代码",
            // 18位：第1位登记管理部门代码，第2位机构类别代码，第3-8位登记管理机关行政区划码，
            // 第9-17位主体标识码，第18位校验码
            pattern: r"[0-9A-HJ-NPQRTUWXY]{2}\d{6}[0-9A-HJ-NPQRTUWXY]{10}",
            enabled_by_default: true,
            has_validation: true,
        },
        // 【新增】车牌号
        SensitiveRuleDef {
            id: "license_plate",
            name: "车牌号",
            // 普通车牌：省份简称 + 字母 + 5位数字/字母
            // 新能源车牌：省份简称 + 字母 + 6位数字/字母
            pattern: r"[京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤青藏川宁琼使领][A-Z][A-HJ-NP-Z0-9]{5,6}",
            enabled_by_default: true,
            has_validation: false,
        },
        // 【新增】护照号码
        SensitiveRuleDef {
            id: "passport",
            name: "护照号码",
            // 中国护照：G/E/D/S/P/H开头 + 8位数字
            pattern: r"(?:[GEDSPH]\d{8}|E\d{8})",
            enabled_by_default: true,
            has_validation: false,
        },
        // 【新增】军官证
        SensitiveRuleDef {
            id: "military_id",
            name: "军官证",
            // 军/兵/士/文/职/广/后/北/沈/兰/济/南/广/成/空/海/武/公/消/警/边/防/通/装/技/参/政/后/装/纪/法/检/审/供/卫/训/科/院/校/所/站/队/团/营/连/排/班 + 字 + 8位数字
            pattern: r"(?:[军兵士文职广后北沈兰济南广成空海武公消警边防通装技参政后装纪检法检审供卫训科院校所站队团营连排班]字\d{8})",
            enabled_by_default: false,
            has_validation: false,
        },
        // 【新增】港澳通行证
        SensitiveRuleDef {
            id: "hk_macau_permit",
            name: "港澳通行证",
            // H/M + 10位数字
            pattern: r"(?:[HM]\d{10})",
            enabled_by_default: true,
            has_validation: false,
        },
        // 【新增】台湾通行证
        SensitiveRuleDef {
            id: "taiwan_permit",
            name: "台湾通行证",
            // T + 8位数字
            pattern: r"(?:T\d{8})",
            enabled_by_default: true,
            has_validation: false,
        },
    ];
    
    /// 【优化】缓存编译后的正则表达式，避免重复编译
    static ref COMPILED_REGEXES: HashMap<&'static str, Regex> = {
        let mut map = HashMap::new();
        for rule in BUILTIN_RULES.iter() {
            if let Ok(regex) = Regex::new(rule.pattern) {
                map.insert(rule.id, regex);
            }
        }
        map
    };
}

struct SensitiveRuleDef {
    id: &'static str,
    name: &'static str,
    pattern: &'static str,
    enabled_by_default: bool,
    #[allow(dead_code)]
    has_validation: bool,  // 【新增】是否需要额外验证
}

/// 获取所有内置规则
pub fn get_builtin_rules() -> Vec<(String, String, bool)> {
    BUILTIN_RULES.iter()
        .map(|rule| (rule.id.to_string(), rule.name.to_string(), rule.enabled_by_default))
        .collect()
}

/// 【优化】Luhn 算法校验银行卡号
fn luhn_check(card_number: &str) -> bool {
    // 【优化】先检查卡 BIN（快速失败）
    // 银联借记卡：62开头
    // 银联信用卡：62、60开头
    // Visa：4开头
    // MasterCard：51-55或2开头
    let bytes = card_number.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    
    let has_valid_bin = match (bytes[0], bytes.get(1)) {
        (b'6', Some(b'2')) | (b'6', Some(b'0')) => true,  // 银联
        (b'4', _) => true,  // Visa
        (b'5', Some(b'1'..=b'5')) => true,  // MasterCard 51-55
        (b'2', Some(b'2'..=b'7')) => true,  // MasterCard 2系列
        _ => false,
    };
    
    if !has_valid_bin {
        return false;  // 快速失败
    }
    
    // Luhn算法校验
    let mut sum = 0;
    let mut double = false;
    
    for c in card_number.chars().rev() {
        if let Some(digit) = c.to_digit(10) {
            let mut d = digit;
            if double {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            sum += d;
            double = !double;
        } else {
            return false; // 包含非数字字符
        }
    }
    
    sum % 10 == 0
}

/// 校验中国身份证号（完整版）
fn validate_person_id(id: &str) -> bool {
    // 必须是18位
    if id.len() != 18 {
        return false;
    }
    
    // 前17位必须是数字，最后一位可以是数字或X/x
    let bytes = id.as_bytes();
    for &byte in bytes.iter().take(17) {
        if !byte.is_ascii_digit() {
            return false;
        }
    }
    if !bytes[17].is_ascii_digit() && bytes[17] != b'X' && bytes[17] != b'x' {
        return false;
    }
    
    // 提取出生日期（第7-14位）
    let year_str = &id[6..10];
    let month_str = &id[10..12];
    let day_str = &id[12..14];
    
    let year = match year_str.parse::<u32>() {
        Ok(y) => y,
        Err(_) => return false,
    };
    let month = match month_str.parse::<u32>() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let day = match day_str.parse::<u32>() {
        Ok(d) => d,
        Err(_) => return false,
    };
    
    // 校验年份：1900至今
    let current_year = chrono::Local::now().year() as u32;
    if year < 1900 || year > current_year {
        return false;
    }
    
    // 校验月份：1-12
    if !(1..=12).contains(&month) {
        return false;
    }
    
    // 校验日期：根据月份和闰年判断
    let is_leap_year = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year { 29 } else { 28 },
        _ => return false,
    };
    
    if day < 1 || day > days_in_month {
        return false;
    }
    
    // 校验码验证（ISO 7064:1983.MOD 11-2）
    let weights = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    let check_codes = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];
    
    let mut sum = 0;
    for (i, &byte) in bytes.iter().take(17).enumerate() {
        let digit = byte - b'0';
        sum += (digit as u32) * weights[i];
    }
    
    let mod_result = (sum % 11) as usize;
    let expected_check_code = check_codes[mod_result];
    
    // 最后一位可能是 X 或 x，统一转为大写比较
    let actual_check_code = bytes[17].to_ascii_uppercase();
    
    actual_check_code == expected_check_code as u8
}

/// 【优化】校验 IP 地址（增加前导零检查）
fn validate_ip_address(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    
    for part in parts {
        // 检查是否为空
        if part.is_empty() {
            return false;
        }
        
        // 【优化】检查前导零（除了"0"本身）
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        
        // 解析数字并检查范围
        match part.parse::<u32>() {
            Ok(num) if num <= 255 => continue,
            _ => return false,
        }
    }
    
    true
}

/// 【新增】校验统一社会信用代码
fn validate_unified_social_credit_code(code: &str) -> bool {
    // 必须是18位
    if code.len() != 18 {
        return false;
    }
    
    // 字符集检查：0-9, A-H, J-N, P-R, T-U, W-Y (排除I, O, S, V, Z)
    let valid_chars = "0123456789ABCDEFGHJKLMNPQRTUWXY";
    for c in code.chars() {
        if !valid_chars.contains(c) {
            return false;
        }
    }
    
    // 校验码计算（MOD 31-1算法）
    let weights = [1, 3, 9, 27, 19, 26, 16, 17, 20, 29, 25, 13, 8, 24, 10, 30, 28];
    let char_values = "0123456789ABCDEFGHJKLMNPQRTUWXY";
    
    let mut sum = 0u64;
    for (i, c) in code.chars().take(17).enumerate() {
        if let Some(pos) = char_values.find(c) {
            sum += (pos as u64) * (weights[i] as u64);
        } else {
            return false;
        }
    }
    
    let mod_result = 31 - (sum % 31) as usize;
    let expected_check_char = if mod_result == 31 {
        '0'
    } else {
        char_values.chars().nth(mod_result).unwrap_or('0')
    };
    
    code.chars().nth(17) == Some(expected_check_char)
}

/// 检测文本中的敏感数据
/// 
/// # 参数
/// * `text` - 待检测文本
/// * `enabled_types` - 启用的敏感数据类型ID列表
/// * `enable_builtin_rules` - 是否启用内置规则
/// * `search_expression` - 自定义搜索表达式（可选）
/// 
/// # 返回
/// HashMap<规则ID, 匹配次数>
pub fn detect_sensitive_data(
    text: &str,
    enabled_types: &[String],
    enable_builtin_rules: bool,
    search_expression: Option<&str>,
) -> (HashMap<String, u32>, Option<u32>) {
    let mut counts = HashMap::new();
    let mut expression_matched: Option<u32> = None;
    
    // 【新增】如果禁用内置规则，只使用表达式搜索
    if !enable_builtin_rules {
        if let Some(expr) = search_expression
            && !expr.trim().is_empty() {
                expression_matched = evaluate_search_expression(expr, text);
            }
        return (counts, expression_matched);
    }
    
    // 执行内置规则检测
    for rule in BUILTIN_RULES.iter() {
        if !enabled_types.contains(&rule.id.to_string()) {
            continue;
        }
        
        // 【优化】使用缓存的正则表达式，避免重复编译
        if let Some(regex) = COMPILED_REGEXES.get(rule.id) {
            let match_count = regex.find_iter(text)
                .take(1000)  // 【优化】限制最大匹配数，防止灾难性回溯
                .filter(|mat| {
                    let matched_str = mat.as_str();
                    
                    // 【新增】根据规则类型进行额外验证
                    match rule.id {
                        // 需要前后非数字检查的类型
                        "phone" | "bank_card" | "person_id" => {
                            let start = mat.start();
                            let end = mat.end();
                            
                            // 【优化】使用字节访问，比 chars() 快
                            let prev_is_digit = start > 0 && text.as_bytes()[start - 1].is_ascii_digit();
                            let next_is_digit = end < text.len() && text.as_bytes()[end].is_ascii_digit();
                            
                            // 如果前后有数字，不是有效匹配
                            if prev_is_digit || next_is_digit {
                                return false;
                            }
                            
                            // 特定类型的额外验证
                            match rule.id {
                                "bank_card" => luhn_check(matched_str),
                                "person_id" => validate_person_id(matched_str),
                                _ => true,
                            }
                        }
                        // IP地址验证
                        "ip_address" => validate_ip_address(matched_str),
                        // 统一社会信用代码验证
                        "unified_social_credit_code" => validate_unified_social_credit_code(matched_str),
                        // 其他类型无需额外验证
                        _ => true,
                    }
                })
                .count() as u32;
            
            if match_count > 0 {
                counts.insert(rule.id.to_string(), match_count);
            }
        }
    }
    
    // 【新增】如果提供了表达式，也进行评估
    if let Some(expr) = search_expression
        && !expr.trim().is_empty() {
            expression_matched = evaluate_search_expression(expr, text);
        }
    
    (counts, expression_matched)
}

/// 评估搜索表达式
fn evaluate_search_expression(expr: &str, text: &str) -> Option<u32> {
    use crate::utils::expression_parser::evaluate_expression;
    let result = evaluate_expression(expr, text);
    if result.matched {
        Some(1)
    } else {
        Some(0)
    }
}

/// 获取高亮区间
pub fn get_highlights(text: &str, enabled_types: &[String]) -> Vec<(usize, usize, String, String)> {
    let mut highlights = Vec::new();
    
    for rule in BUILTIN_RULES.iter() {
        if !enabled_types.contains(&rule.id.to_string()) {
            continue;
        }
        
        // 【优化】使用缓存的正则表达式
        if let Some(regex) = COMPILED_REGEXES.get(rule.id) {
            for mat in regex.find_iter(text).take(1000) {  // 【优化】限制匹配数
                let matched_str = mat.as_str();
                
                // 【新增】根据规则类型进行额外验证
                let is_valid = match rule.id {
                    "phone" | "bank_card" | "person_id" => {
                        let start = mat.start();
                        let end = mat.end();
                        
                        // 【优化】使用字节访问，比 chars() 快
                        let prev_is_digit = start > 0 && text.as_bytes()[start - 1].is_ascii_digit();
                        let next_is_digit = end < text.len() && text.as_bytes()[end].is_ascii_digit();
                        
                        // 如果前后有数字，跳过
                        if prev_is_digit || next_is_digit {
                            false
                        } else {
                            // 特定类型的额外验证
                            match rule.id {
                                "bank_card" => luhn_check(matched_str),
                                "person_id" => validate_person_id(matched_str),
                                _ => true,
                            }
                        }
                    }
                    "ip_address" => validate_ip_address(matched_str),
                    "unified_social_credit_code" => validate_unified_social_credit_code(matched_str),
                    _ => true,
                };
                
                if !is_valid {
                    continue;
                }
                
                // 将字节索引转换为字符索引
                let char_start = text[..mat.start()].chars().count();
                let char_end = char_start + mat.as_str().chars().count();
                
                highlights.push((
                    char_start,
                    char_end,
                    rule.id.to_string(),
                    rule.name.to_string(),
                ));
            }
        }
    }
    
    // 按起始位置排序
    highlights.sort_by_key(|h| h.0);
    highlights
}

#[cfg(test)]
#[allow(clippy::useless_vec)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_phone() {
        let text = "我的手机号是13812345678";
        let (counts, _) = detect_sensitive_data(text, &vec!["phone".to_string()], true, None);
        assert_eq!(counts.get("phone"), Some(&1));
    }
    
    #[test]
    fn test_detect_phone_with_punctuation() {
        // 测试带标点符号的手机号
        let text = "联系电话：0731—89801881；15364026015；";
        let (counts, _) = detect_sensitive_data(text, &vec!["phone".to_string()], true, None);
        assert_eq!(counts.get("phone"), Some(&1), "应该匹配到15364026015");
    }
    
    #[test]
    fn test_validate_person_id() {
        // 测试有效的身份证号（校验码正确）
        // 110101199001011237: 北京东城，1990-01-01，校验码7
        assert!(validate_person_id("110101199001011237"), "应该通过校验");
        
        // 测试无效的身份证号
        assert!(!validate_person_id("110101199001011234"), "校验码错误");
        assert!(!validate_person_id("11010119900101123"), "位数不足");
        assert!(!validate_person_id("110101199013011234"), "月份无效");
        assert!(!validate_person_id("110101199002301234"), "日期无效");
        assert!(!validate_person_id("110101189901011234"), "年份太早");
    }
    
    #[test]
    fn test_luhn_check() {
        // 有效的银行卡号（通过 Luhn 校验）
        assert!(luhn_check("4532015112830366"));
        assert!(luhn_check("6011111111111117"));
        
        // 无效的银行卡号
        assert!(!luhn_check("1234567890123456"));
        assert!(!luhn_check("1111111111111111"));  // 全1无法通过校验
    }
    
    #[test]
    fn test_ip_address_validation() {
        // 有效的 IP
        let text = "IP: 192.168.1.1 and 10.0.0.1";
        let (counts, _) = detect_sensitive_data(text, &vec!["ip_address".to_string()], true, None);
        assert_eq!(counts.get("ip_address"), Some(&2));
        
        // 无效的 IP（超过 255）
        let text_invalid = "Invalid: 999.999.999.999";
        let (counts_invalid, _) = detect_sensitive_data(text_invalid, &vec!["ip_address".to_string()], true, None);
        assert_eq!(counts_invalid.get("ip_address"), None);
    }
    
    #[test]
    fn test_highlights_char_index() {
        // 测试高亮区间使用字符索引而不是字节索引
        let text = "手机号13812345678测试";
        let highlights = get_highlights(text, &vec!["phone".to_string()]);
        
        assert_eq!(highlights.len(), 1);
        // "手机号" 是3个字符，所以手机号应该从索引3开始
        assert_eq!(highlights[0].0, 3, "起始位置应该是字符索引3");
        assert_eq!(highlights[0].1, 14, "结束位置应该是字符索引14 (3+11)");
    }
    
    #[test]
    fn test_address_strict_matching() {
        // 测试地址严格匹配
        
        // ✅ 应该匹配：完整地址结构（有省）
        let valid_addresses_with_province = vec![
            "湖南省长沙市岳麓区麓山南路100号",
            "广东省深圳市南山区科技园路",
            "浙江省杭州市西湖区文三路",
        ];
        
        for addr in valid_addresses_with_province {
            let (counts, _) = detect_sensitive_data(addr, &["address".to_string()], true, None);
            assert!(counts.contains_key("address") && counts["address"] > 0, 
                "应该匹配地址: {}", addr);
        }
        
        // ✅ 应该匹配：无省的地址结构
        let valid_addresses_without_province = vec![
            "北京市海淀区中关村大街27号",
            "成都市武侯区人民南路",
            "武汉市江汉区解放大道",
            "南京市鼓楼区中山路",
        ];
        
        for addr in valid_addresses_without_province {
            let (counts, _) = detect_sensitive_data(addr, &["address".to_string()], true, None);
            assert!(counts.contains_key("address") && counts["address"] > 0, 
                "应该匹配地址（无省）: {}", addr);
        }
        
        // ❌ 不应该匹配：不完整的地址片段或误报
        let invalid_addresses = vec![
            "市区道路",           // 缺少具体地名
            "区域划分",           // 不是地址
            "市场管理部门",       // "市场"不是行政区划
            "市民小区",           // 缺少市级行政区
            "社区服务中心",       // 缺少省市结构
            "以届时市场最优惠价格并不得比本合同成交价高出",  // "市场"是词语一部分
            "但任何由于市场变化或一方自身的经营所造成的事件不应视为不可抗力",  // "市场"是词语
            "包括市区与郊区",     // "市区"不是行政区划
        ];
        
        for addr in invalid_addresses {
            let (counts, _) = detect_sensitive_data(addr, &["address".to_string()], true, None);
            assert!(!counts.contains_key("address") || counts["address"] == 0,
                "不应该匹配地址: {}", addr);
        }
    }
}
