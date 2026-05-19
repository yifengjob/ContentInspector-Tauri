#![allow(dead_code)]
/// 自定义敏感词逻辑表达式解析器
///
/// 支持运算符：
/// - ! : 非（NOT）
/// - & : 与（AND）
/// - | : 或（OR）
/// - () : 分组
///
/// 优先级：( ) > ! > & > |
///
/// 示例：
/// - "密码" → 匹配包含"密码"的文本
/// - "!密码" → 匹配不包含"密码"的文本
/// - "密码 & 身份证" → 匹配同时包含"密码"和"身份证"的文本
/// - "密码 | 身份证" → 匹配包含"密码"或"身份证"的文本
/// - "!密码 & (身份证 | 银行卡)" → 复杂逻辑组合

/// 表达式验证结果
#[derive(Debug, Clone)]
pub struct ExpressionValidationResult {
    pub valid: bool,
    pub error: Option<String>,
}

/// 表达式评估结果
#[derive(Debug, Clone)]
pub struct ExpressionEvaluationResult {
    pub matched: bool,
    pub error: Option<String>,
}

/// Token 类型
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Keyword(String),
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
}

/// 词法分析：将表达式字符串分解为 token 数组
fn tokenize(expression: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    
    for ch in expression.chars() {
        match ch {
            ' ' => {
                if !current_token.is_empty() {
                    tokens.push(Token::Keyword(current_token.clone()));
                    current_token.clear();
                }
            }
            '(' => {
                if !current_token.is_empty() {
                    tokens.push(Token::Keyword(current_token.clone()));
                    current_token.clear();
                }
                tokens.push(Token::LeftParen);
            }
            ')' => {
                if !current_token.is_empty() {
                    tokens.push(Token::Keyword(current_token.clone()));
                    current_token.clear();
                }
                tokens.push(Token::RightParen);
            }
            '!' => {
                if !current_token.is_empty() {
                    tokens.push(Token::Keyword(current_token.clone()));
                    current_token.clear();
                }
                tokens.push(Token::Not);
            }
            '&' => {
                if !current_token.is_empty() {
                    tokens.push(Token::Keyword(current_token.clone()));
                    current_token.clear();
                }
                tokens.push(Token::And);
            }
            '|' => {
                if !current_token.is_empty() {
                    tokens.push(Token::Keyword(current_token.clone()));
                    current_token.clear();
                }
                tokens.push(Token::Or);
            }
            _ => {
                current_token.push(ch);
            }
        }
    }
    
    if !current_token.is_empty() {
        tokens.push(Token::Keyword(current_token));
    }
    
    Ok(tokens)
}

/// 验证 token 序列的基本合法性
fn validate_tokens(tokens: &[Token]) -> ExpressionValidationResult {
    if tokens.is_empty() {
        return ExpressionValidationResult { valid: true, error: None };
    }
    
    // 检查第一个 token 不能是二元运算符
    if matches!(tokens[0], Token::And | Token::Or) {
        return ExpressionValidationResult {
            valid: false,
            error: Some("表达式不能以运算符开头".to_string()),
        };
    }
    
    // 检查最后一个 token 不能是运算符或左括号
    if matches!(tokens.last().unwrap(), Token::Not | Token::And | Token::Or | Token::LeftParen) {
        return ExpressionValidationResult {
            valid: false,
            error: Some("表达式结尾缺少操作数".to_string()),
        };
    }
    
    // 检查连续的运算符
    for i in 0..tokens.len() - 1 {
        let current = &tokens[i];
        let next = &tokens[i + 1];
        
        // ! 后面必须是关键词或左括号
        if *current == Token::Not && matches!(next, Token::And | Token::Or | Token::RightParen) {
            return ExpressionValidationResult {
                valid: false,
                error: Some("'!' 后面缺少操作数".to_string()),
            };
        }
        
        // & 和 | 后面必须是关键词、! 或左括号
        if matches!(current, Token::And | Token::Or) 
            && matches!(next, Token::And | Token::Or | Token::RightParen) {
            return ExpressionValidationResult {
                valid: false,
                error: Some("运算符后面缺少操作数".to_string()),
            };
        }
        
        // 关键词或右括号后面不能直接跟关键词或左括号（缺少运算符）
        if (matches!(current, Token::Keyword(_)) || *current == Token::RightParen)
            && (matches!(next, Token::Keyword(_)) || *next == Token::LeftParen) {
            return ExpressionValidationResult {
                valid: false,
                error: Some("缺少运算符".to_string()),
            };
        }
    }
    
    // 检查括号匹配
    let mut bracket_count = 0;
    for token in tokens {
        match token {
            Token::LeftParen => bracket_count += 1,
            Token::RightParen => {
                bracket_count -= 1;
                if bracket_count < 0 {
                    return ExpressionValidationResult {
                        valid: false,
                        error: Some("右括号多余".to_string()),
                    };
                }
            }
            _ => {}
        }
    }
    
    if bracket_count > 0 {
        return ExpressionValidationResult {
            valid: false,
            error: Some("左括号未闭合".to_string()),
        };
    }
    
    ExpressionValidationResult { valid: true, error: None }
}

/// 验证表达式语法是否正确
pub fn validate_expression(expression: &str) -> ExpressionValidationResult {
    // 空表达式视为有效（表示不启用）
    if expression.trim().is_empty() {
        return ExpressionValidationResult { valid: true, error: None };
    }
    
    match tokenize(expression) {
        Ok(tokens) => validate_tokens(&tokens),
        Err(e) => ExpressionValidationResult {
            valid: false,
            error: Some(format!("词法分析失败: {}", e)),
        },
    }
}

/// 评估表达式是否匹配文本
pub fn evaluate_expression(expression: &str, text: &str) -> ExpressionEvaluationResult {
    // 空表达式视为不匹配
    if expression.trim().is_empty() {
        return ExpressionEvaluationResult {
            matched: false,
            error: None,
        };
    }
    
    // 首先验证表达式
    let validation = validate_expression(expression);
    if !validation.valid {
        return ExpressionEvaluationResult {
            matched: false,
            error: validation.error,
        };
    }
    
    // 词法分析
    let tokens = match tokenize(expression) {
        Ok(t) => t,
        Err(e) => {
            return ExpressionEvaluationResult {
                matched: false,
                error: Some(e),
            };
        }
    };
    
    // 递归下降解析并评估
    let mut pos = 0;
    match parse_or(&tokens, &mut pos, text) {
        Ok(result) => ExpressionEvaluationResult {
            matched: result,
            error: None,
        },
        Err(e) => ExpressionEvaluationResult {
            matched: false,
            error: Some(e),
        },
    }
}

/// 解析 OR 表达式（最低优先级）
fn parse_or(tokens: &[Token], pos: &mut usize, text: &str) -> Result<bool, String> {
    let mut left = parse_and(tokens, pos, text)?;
    
    while *pos < tokens.len() && tokens[*pos] == Token::Or {
        *pos += 1; // 跳过 OR
        let right = parse_and(tokens, pos, text)?;
        left = left || right;
    }
    
    Ok(left)
}

/// 解析 AND 表达式
fn parse_and(tokens: &[Token], pos: &mut usize, text: &str) -> Result<bool, String> {
    let mut left = parse_not(tokens, pos, text)?;
    
    while *pos < tokens.len() && tokens[*pos] == Token::And {
        *pos += 1; // 跳过 AND
        let right = parse_not(tokens, pos, text)?;
        left = left && right;
    }
    
    Ok(left)
}

/// 解析 NOT 表达式
fn parse_not(tokens: &[Token], pos: &mut usize, text: &str) -> Result<bool, String> {
    if *pos < tokens.len() && tokens[*pos] == Token::Not {
        *pos += 1; // 跳过 NOT
        let value = parse_primary(tokens, pos, text)?;
        Ok(!value)
    } else {
        parse_primary(tokens, pos, text)
    }
}

/// 解析基本表达式（关键词或括号）
fn parse_primary(tokens: &[Token], pos: &mut usize, text: &str) -> Result<bool, String> {
    if *pos >= tokens.len() {
        return Err("表达式不完整".to_string());
    }
    
    match &tokens[*pos] {
        Token::LeftParen => {
            *pos += 1; // 跳过左括号
            let result = parse_or(tokens, pos, text)?;
            if *pos >= tokens.len() || tokens[*pos] != Token::RightParen {
                return Err("缺少右括号".to_string());
            }
            *pos += 1; // 跳过右括号
            Ok(result)
        }
        Token::Keyword(keyword) => {
            *pos += 1;
            // 检查文本是否包含关键词（不区分大小写）
            Ok(text.to_lowercase().contains(&keyword.to_lowercase()))
        }
        _ => Err(format!("意外的 token: {:?}", tokens[*pos])),
    }
}

/// 批量评估表达式对多个文本片段的匹配情况
/// 返回每个片段是否匹配
pub fn evaluate_expression_batch(
    expression: &str,
    texts: &[&str],
) -> Vec<ExpressionEvaluationResult> {
    texts
        .iter()
        .map(|text| evaluate_expression(expression, text))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_valid_expressions() {
        assert!(validate_expression("密码").valid);
        assert!(validate_expression("密码 & 身份证").valid);
        assert!(validate_expression("密码 | 身份证").valid);
        assert!(validate_expression("!密码").valid);
        assert!(validate_expression("密码 & (身份证 | 银行卡)").valid);
    }
    
    #[test]
    fn test_validate_invalid_expressions() {
        assert!(!validate_expression("& 密码").valid);
        assert!(!validate_expression("密码 &").valid);
        assert!(!validate_expression("(密码").valid);
        assert!(!validate_expression("密码)").valid);
    }
    
    #[test]
    fn test_evaluate_simple_keyword() {
        let result = evaluate_expression("密码", "这是一个密码测试");
        assert!(result.matched);
        
        let result = evaluate_expression("密码", "这是普通文本");
        assert!(!result.matched);
    }
    
    #[test]
    fn test_evaluate_and() {
        let result = evaluate_expression("密码 & 身份证", "密码和身份证号都在这里");
        assert!(result.matched);
        
        let result = evaluate_expression("密码 & 身份证", "只有密保没有身份");
        assert!(!result.matched);
    }
    
    #[test]
    fn test_evaluate_or() {
        let result = evaluate_expression("密码 | 身份证", "只有密码");
        assert!(result.matched);
        
        let result = evaluate_expression("密码 | 身份证", "只有身份证");
        assert!(result.matched);
        
        let result = evaluate_expression("密码 | 身份证", "什么都没有");
        assert!(!result.matched);
    }
    
    #[test]
    fn test_evaluate_not() {
        let result = evaluate_expression("!密码", "这里没有敏感信息");
        assert!(result.matched);
        
        let result = evaluate_expression("!密码", "这里有密码");
        assert!(!result.matched);
    }
    
    #[test]
    fn test_evaluate_complex() {
        // 没有"密码"，有"身份证" -> true
        let result = evaluate_expression(
            "!密码 & (身份证 | 银行卡)",
            "有身份证号但没有口令"
        );
        assert!(result.matched);
        
        // 有"密码" -> false（因为 !密码 为假）
        let result = evaluate_expression(
            "!密码 & (身份证 | 银行卡)",
            "有密码和身份证"
        );
        assert!(!result.matched);
    }
    
    #[test]
    fn test_case_insensitive() {
        let result = evaluate_expression("Password", "这里是password测试");
        assert!(result.matched);
    }
}
