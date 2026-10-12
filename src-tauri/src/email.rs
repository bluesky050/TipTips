use crate::email_config::deobfuscate;
use mailparse::parse_mail;

/// Represents a parsed email with the fields we care about
#[derive(Debug, Clone)]
pub struct ParsedEmail {
    pub uid: u32,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub body: String,
}

/// Connect to QQ Mail via IMAP and fetch recent emails
pub fn fetch_recent_emails(
    email_address: &str,
    auth_code_obfuscated: &str,
    processed_uids: &[u32],
    days_back: u32,
) -> Result<Vec<ParsedEmail>, String> {
    let auth_code = deobfuscate(auth_code_obfuscated);

    // Connect to QQ Mail IMAP server
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS error: {}", e))?;

    let client = imap::connect(("imap.qq.com", 993), "imap.qq.com", &tls)
        .map_err(|e| format!("IMAP connection failed: {}", e))?;

    let mut session = client
        .login(email_address, &auth_code)
        .map_err(|e| format!("IMAP login failed: {}", e.0))?;

    // Select INBOX
    session
        .select("INBOX")
        .map_err(|e| format!("Failed to select INBOX: {}", e))?;

    // Search for recent emails (last N days)
    let since_date = chrono::Local::now()
        .checked_sub_signed(chrono::Duration::days(days_back as i64))
        .unwrap_or_else(chrono::Local::now)
        .format("%d-%b-%Y")
        .to_string();

    let search_query = format!("SINCE {}", since_date);
    let uids = session
        .uid_search(&search_query)
        .map_err(|e| format!("IMAP search failed: {}", e))?;

    // Filter out already-processed UIDs
    let mut new_uids: Vec<u32> = uids
        .iter()
        .filter(|uid| !processed_uids.contains(uid))
        .copied()
        .collect();

    // Sort newest first and limit to max 60 emails per scan
    new_uids.sort_by(|a, b| b.cmp(a));
    new_uids.truncate(60);

    if new_uids.is_empty() {
        let _ = session.logout();
        return Ok(Vec::new());
    }

    // Fetch emails in small batches (5 at a time)
    let mut emails = Vec::new();
    let batch_size = 5;
    for chunk in new_uids.chunks(batch_size) {
        let uid_set: String = chunk
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let messages = session
            .uid_fetch(&uid_set, "(UID RFC822)")
            .map_err(|e| format!("IMAP fetch failed: {}", e))?;

        for message in messages.iter() {
            let uid = message.uid.unwrap_or(0);
            if uid == 0 {
                continue;
            }

            let raw_bytes = match message.body() {
                Some(bytes) => bytes,
                None => continue,
            };

            let (from, subject, date, body) = parse_full_email(raw_bytes);

            // Safely truncate body to max 2500 characters by Unicode char count
            let body_truncated = safe_truncate_chars(&body, 2500);

            emails.push(ParsedEmail {
                uid,
                from,
                subject,
                date,
                body: body_truncated,
            });
        }
    }

    let _ = session.logout();
    Ok(emails)
}

/// Test IMAP connection without fetching emails
pub fn test_connection(email_address: &str, auth_code: &str) -> Result<String, String> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS error: {}", e))?;

    let client = imap::connect(("imap.qq.com", 993), "imap.qq.com", &tls)
        .map_err(|e| format!("连接 QQ 邮箱 IMAP 服务器失败: {}", e))?;

    let mut session = client
        .login(email_address, auth_code)
        .map_err(|e| format!("登录失败，请检查邮箱地址和授权码: {}", e.0))?;

    let mailbox = session
        .select("INBOX")
        .map_err(|e| format!("无法打开收件箱: {}", e))?;

    let msg_count = mailbox.exists;
    let _ = session.logout();

    Ok(format!("连接成功！收件箱共有 {} 封邮件", msg_count))
}

/// Parse full RFC822 raw email into (from, subject, date, body)
pub fn parse_full_email(raw_bytes: &[u8]) -> (String, String, String, String) {
    if let Ok(parsed) = parse_mail(raw_bytes) {
        let mut from = String::new();
        let mut subject = String::new();
        let mut date = String::new();

        for header in parsed.headers.iter() {
            let key = header.get_key().to_lowercase();
            match key.as_str() {
                "from" => from = header.get_value(),
                "subject" => subject = header.get_value(),
                "date" => date = header.get_value(),
                _ => {}
            }
        }

        let body = extract_text_from_parsed(&parsed);
        (from, subject, date, body)
    } else {
        (String::new(), String::new(), String::new(), String::new())
    }
}

/// Recursively extract text from parsed MIME parts
fn extract_text_from_parsed(parsed: &mailparse::ParsedMail) -> String {
    // If this is a multipart message, recurse into parts
    if !parsed.subparts.is_empty() {
        // Prefer text/plain over text/html
        let mut text_plain = String::new();
        let mut text_html = String::new();

        for part in &parsed.subparts {
            let content_type = part.ctype.mimetype.to_lowercase();
            if content_type == "text/plain" && text_plain.is_empty() {
                if let Ok(body) = part.get_body() {
                    text_plain = body;
                }
            } else if content_type == "text/html" && text_html.is_empty() {
                if let Ok(body) = part.get_body() {
                    text_html = body;
                }
            } else if content_type.starts_with("multipart/") {
                let nested = extract_text_from_parsed(part);
                if !nested.is_empty() && text_plain.is_empty() {
                    text_plain = nested;
                }
            }
        }

        if !text_plain.is_empty() {
            return text_plain;
        }
        if !text_html.is_empty() {
            return strip_html_tags(&text_html);
        }
        return String::new();
    }

    // Single part
    let content_type = parsed.ctype.mimetype.to_lowercase();
    if content_type == "text/plain" {
        parsed.get_body().unwrap_or_default()
    } else if content_type == "text/html" {
        let html = parsed.get_body().unwrap_or_default();
        strip_html_tags(&html)
    } else {
        String::new()
    }
}

/// Simple HTML tag stripping
fn strip_html_tags(html: &str) -> String {
    // Block elements carry paragraph boundaries needed by event and note extraction.
    let blocks = regex::Regex::new(r"(?i)</?(?:p|div|tr|li|h[1-6]|br)\b[^>]*>").unwrap();
    let html = blocks.replace_all(html, "\u{2028}");
    let decoded = html_escape::decode_html_entities(&html);
    let mut result = String::with_capacity(decoded.len());
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in decoded.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ if !in_tag => {
                if ch == '\u{2028}' {
                    if !result.ends_with('\n') && !result.trim().is_empty() {
                        result.push('\n');
                    }
                    last_was_space = true;
                } else if ch.is_whitespace() {
                    if !last_was_space {
                        result.push(' ');
                        last_was_space = true;
                    }
                } else {
                    result.push(ch);
                    last_was_space = false;
                }
            }
            _ => {}
        }
    }

    result.trim().to_string()
}

/// Safely truncate string by character count (not byte offset) to prevent UTF-8 boundary panics
pub fn safe_truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_paragraphs_remain_separate() {
        let text = strip_html_tags(
            "<p>面向2027届毕业生</p><p>面试时间：2026-10-15 14:00</p><br>注意：携带证件",
        );
        assert_eq!(text.lines().count(), 3);
    }

    #[test]
    fn html_date_and_time_on_adjacent_lines_form_one_window() {
        let body = strip_html_tags("<p>面试时间：2026-10-15</p><p>14:00-15:00</p>");
        let mail = ParsedEmail {
            uid: 1,
            from: "hr@example.com".into(),
            subject: "【百度】面试邀请".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body,
        };
        let items = crate::rule_parser::parse_email_items_by_rules(&mail);
        let schedule = crate::task::normalize(&items[0], &mail);
        assert_eq!(schedule.time, "14:00");
        assert_eq!(schedule.end_time, "15:00");
    }

    #[test]
    fn test_safe_truncate_chars_chinese() {
        let text = "这是一段非常非常长的中文邮件正文测试内容，测试截断是否会造成边界崩溃。";
        let truncated = safe_truncate_chars(text, 5);
        assert_eq!(truncated, "这是一段非");
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "<div><p>你好，请于 <b>2026-10-15</b> 参加面试。</p></div>";
        let text = strip_html_tags(html);
        assert!(text.contains("你好，请于 2026-10-15 参加面试。"));
    }

    #[test]
    fn test_parse_full_email_plain() {
        let raw = b"From: hr@bytedance.com\r\nSubject: Interview Invitation\r\nDate: Sat, 10 Oct 2026 10:00:00 +0800\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nPlease join the meeting.";
        let (from, subject, date, body) = parse_full_email(raw);
        assert_eq!(from, "hr@bytedance.com");
        assert_eq!(subject, "Interview Invitation");
        assert_eq!(date, "Sat, 10 Oct 2026 10:00:00 +0800");
        assert_eq!(body.trim(), "Please join the meeting.");
    }
}
