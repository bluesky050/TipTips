use crate::email::ParsedEmail;
use crate::email_config::deobfuscate;
use serde::{Deserialize, Serialize};

/// Represents an action item extracted from an email by the AI
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtractedItem {
    /// "面试" | "笔试" | "测评" | "其他"
    #[serde(default)]
    pub r#type: String,
    /// Short description, e.g. "字节跳动-后端开发-二面"
    #[serde(default)]
    pub title: String,
    /// Date in YYYY-MM-DD format
    #[serde(default)]
    pub date: String,
    /// Time in HH:MM format (optional)
    #[serde(default)]
    pub time: String,
    /// Company name
    #[serde(default)]
    pub company: String,
    /// Key details (interview link, location, notes)
    #[serde(default)]
    pub details: String,
    pub role: String,
    pub round: String,
    pub location: String,
    pub link: String,
    /// Verbatim line(s) containing this event's time. Used by shared normalization.
    pub time_evidence: String,
    /// Rules retain the exact activity block; AI cannot inject this internal field.
    #[serde(skip)]
    pub context: String,
}

/// Call Gemini API to analyze an email and extract action items
pub fn analyze_email(
    email: &ParsedEmail,
    api_key_obfuscated: &str,
    model: &str,
) -> Result<Vec<ExtractedItem>, String> {
    let api_key = deobfuscate(api_key_obfuscated);

    let prompt = format!(
        r#"你是一个邮件分析助手。请分析以下邮件，判断是否包含需要用户完成的待办事项。
待办事项类型包括：面试邀请、笔试邀请、在线测评、简历投递截止、offer签约截止、宣讲会等。

如果识别到待办事项，请以 JSON 数组格式返回：
[{{
  "type": "面试" 或 "笔试" 或 "测评" 或 "宣讲会" 或 "投递" 或 "签约" 或 "其他",
  "title": "事项简短描述，如：字节跳动-后端开发-二面",
  "date": "YYYY-MM-DD 格式的日期",
  "time": "HH:MM 格式的时间（如有，否则留空）",
  "company": "公司名称",
  "role": "岗位，没有则留空",
  "round": "面试轮次，没有则留空",
  "location": "地点或会议号，没有则留空",
  "link": "活动链接，没有则留空",
  "time_evidence": "直接复制该事项时间所在的原文完整行，不能改写；缺失则留空",
  "details": "准备材料、时长、注意事项等备注，不重复公司、类型、地点和链接"
}}]

注意：
1. 日期必须是 YYYY-MM-DD 格式。相对日期（如明天、下周一）以邮件发送日期为基准，不能使用扫描当天。没有明确日期则留空，仍保留该事项。
2. 如果没有识别到任何待办事项，返回空数组 []
3. 只返回 JSON 数组，不要包含其他文字说明。
4. 广告、推销、退订通知、系统通知等不算待办事项。
5. 同一邮件的不同活动分别返回；岗位、轮次和时间不允许猜测。

--- 邮件内容 ---
发件人: {from}
主题: {subject}
日期: {date}
正文:
{body}"#,
        from = email.from,
        subject = email.subject,
        date = email.date,
        body = email.body,
    );

    // Build Gemini API request
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "temperature": 0.1
        }
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .map_err(|e| format!("Gemini API request failed: {}", e.without_url()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Gemini API error ({}): {}", status, body));
    }

    let resp_json: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    // Extract the text content safely from Gemini's response structure
    let text = resp_json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| "AI 响应缺少事项内容".to_string())?;

    let items = parse_ai_items(text)?;

    // Dates are validated by shared normalization; missing dates are actionable too.
    let valid_items: Vec<ExtractedItem> = items
        .into_iter()
        .filter(|item| !item.title.trim().is_empty() || !item.r#type.trim().is_empty())
        .collect();

    Ok(valid_items)
}

fn parse_ai_items(text: &str) -> Result<Vec<ExtractedItem>, String> {
    if let Ok(items) = serde_json::from_str::<Vec<ExtractedItem>>(text) {
        return Ok(items);
    }
    if let (Some(start), Some(end)) = (text.find('['), text.rfind(']')) {
        if start < end {
            return serde_json::from_str(&text[start..=end])
                .map_err(|_| "AI 返回的事项格式无效".into());
        }
    }
    Err("AI 返回的事项格式无效".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_ai_output_is_not_a_successful_empty_analysis() {
        assert!(parse_ai_items("not JSON").is_err());
        assert!(parse_ai_items("[{broken}]").is_err());
        assert!(parse_ai_items("[]").unwrap().is_empty());
        assert_eq!(
            parse_ai_items("```json\n[{\"type\":\"测评\"}]\n```")
                .unwrap()
                .len(),
            1
        );
    }
}
