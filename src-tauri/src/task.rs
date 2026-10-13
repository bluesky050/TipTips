//! Shared normalization for both extractors. No filesystem or network access.
use crate::{ai::ExtractedItem, email::ParsedEmail};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub event_type: String,
    pub company: String,
    pub company_raw: String,
    pub role: String,
    pub round: String,
    pub date: String,
    pub time: String,
    pub end_date: String,
    pub end_time: String,
    /// start | deadline | window | unknown
    pub time_kind: String,
    pub timezone: String,
    pub location: String,
    pub link: String,
    pub notes: String,
    pub source_subject: String,
    pub source_date: String,
    pub time_evidence: String,
    pub needs_confirmation: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Timing {
    pub date: String,
    pub time: String,
    pub end_date: String,
    pub end_time: String,
    pub kind: String,
    pub evidence: String,
    pub uncertain: bool,
}

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn event_type(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "面试" | "interview" => "面试",
        "笔试" | "机试" | "test" => "笔试",
        "测评" | "assessment" => "测评",
        "宣讲会" | "宣讲" => "宣讲会",
        "投递" | "简历投递" => "投递",
        "签约" | "offer" | "录用" => "签约",
        _ => "其他",
    }
    .into()
}

fn company_name(value: &str) -> String {
    match value.trim() {
        "中兴" => "中兴通讯".into(),
        "携程" => "携程集团".into(),
        "字节" => "字节跳动".into(),
        _ => clean(value),
    }
}

pub fn valid_date(value: &str) -> bool {
    value.len() == 10
        && NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == value)
}

fn email_datetime(value: &str) -> Option<NaiveDateTime> {
    chrono::DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|d| d.naive_local())
}

fn dates_in_line(line: &str, base: Option<NaiveDateTime>) -> Vec<String> {
    let re = Regex::new(r"(?:(\d{4})[-年/.])?(\d{1,2})[-月/.](\d{1,2})(?:日)?").unwrap();
    re.captures_iter(line)
        .filter_map(|c| {
            let matched = c.get(0)?;
            let before = &line[..matched.start()];
            let after = &line[matched.end()..];
            if before.chars().last().is_some_and(|ch| ch.is_ascii_digit())
                || after.chars().next().is_some_and(|ch| ch.is_ascii_digit())
                || before
                    .chars()
                    .last()
                    .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '.')
                || Regex::new(r"(?:图|表|第|版本|章节)\s*$")
                    .unwrap()
                    .is_match(before)
                || Regex::new(r"^(?:\.\d|节|章|题|步)")
                    .unwrap()
                    .is_match(after)
                || Regex::new(r"^\s*(?:个?工作日|个?小时|分钟|天|个?月|年)")
                    .unwrap()
                    .is_match(after)
            {
                return None;
            }
            let explicit_year = c.get(1).and_then(|m| m.as_str().parse::<i32>().ok());
            let mut year = explicit_year.or_else(|| base.map(|d| d.year()))?;
            let month = c[2].parse().ok()?;
            let day = c[3].parse().ok()?;
            // A December mail referring to January means the next calendar year.
            if explicit_year.is_none() && base.is_some_and(|b| b.month() == 12 && month == 1) {
                year += 1;
            }
            NaiveDate::from_ymd_opt(year, month, day).map(|d| d.format("%Y-%m-%d").to_string())
        })
        .collect()
}

fn times_in_line(line: &str) -> Vec<String> {
    let without_zone = Regex::new(r"(?:UTC|GMT)\s*[+-]\d{1,2}:?\d{2}")
        .unwrap()
        .replace_all(line, "");
    let line = without_zone.as_ref();
    let re = Regex::new(r"(\d+)(?:(:|：)(\d*)(?::(\d+))?|点(\d*)分?)").unwrap();
    re.captures_iter(line)
        .map(|c| {
            let mut hour: u32 = c[1].parse().unwrap_or(99);
            let minute_text = c.get(3).or_else(|| c.get(5)).map_or("", |m| m.as_str());
            let is_colon = c.get(2).is_some();
            if c[1].len() > 2
                || (is_colon && minute_text.len() != 2)
                || (!is_colon && !minute_text.is_empty() && minute_text.len() != 2)
                || c.get(4).is_some_and(|m| {
                    m.as_str().len() != 2 || m.as_str().parse::<u32>().unwrap_or(99) >= 60
                })
            {
                return String::new();
            }
            let minute = if minute_text.is_empty() {
                0
            } else {
                minute_text.parse().unwrap_or(99)
            };
            let prefix = line[..c.get(0).unwrap().start()].trim_end();
            if (prefix.ends_with("下午") || prefix.ends_with("晚上")) && hour < 12 {
                hour += 12;
            }
            if prefix.ends_with("上午") && hour == 12 {
                hour = 0;
            }
            NaiveTime::from_hms_opt(hour, minute, 0)
                .map(|t| t.format("%H:%M").to_string())
                .unwrap_or_default()
        })
        .collect()
}

/// Parse only an actual time-bearing line. Ordinary durations never imply deadlines.
pub fn extract_timing(text: &str, email_date: &str, kind: &str) -> Timing {
    let base = email_datetime(email_date);
    let mut candidates = Vec::new();
    let duration = Regex::new(r"(\d{1,3})\s*(?:个)?(小时|天)(?:内|之前|前)?").unwrap();
    let weekday = Regex::new(r"下周([一二三四五六日天])").unwrap();
    let time_only = Regex::new(r"^\s*(?:(?:时间[:：]?|上午|下午|晚上)\s*)?\d{1,2}[:：]\d{2}(?:\s*[-至到~～]\s*\d{1,2}[:：]\d{2})?\s*[。；.]?\s*$").unwrap();
    let mut lines = text.lines().peekable();
    let mut logical_lines = Vec::new();
    while let Some(line) = lines.next() {
        let mut joined = line.to_string();
        if !dates_in_line(line, base).is_empty()
            && times_in_line(line).is_empty()
            && lines.peek().is_some_and(|next| time_only.is_match(next))
        {
            joined.push('\n');
            joined.push_str(lines.next().unwrap());
        }
        logical_lines.push(joined);
    }
    for raw_line in &logical_lines {
        let line = raw_line.trim();
        if line.contains("届毕业生") || line.contains("面向") || line.contains("学历") {
            continue;
        }
        let mut dates = dates_in_line(line, base);
        let mut times = times_in_line(line);
        let relative_word = if line.contains("后天") {
            Some(2)
        } else if line.contains("明天") {
            Some(1)
        } else if line.contains("今天") {
            Some(0)
        } else {
            None
        };
        if dates.is_empty() {
            if let (Some(days), Some(b)) = (relative_word, base) {
                if let Some(d) = b.checked_add_signed(chrono::Duration::days(days)) {
                    dates.push(d.format("%Y-%m-%d").to_string());
                }
            } else if let (Some(c), Some(b)) = (weekday.captures(line), base) {
                let index = "一二三四五六日"
                    .chars()
                    .position(|v| v.to_string() == c[1])
                    .unwrap_or(6) as i64;
                let days = 7 - b.weekday().num_days_from_monday() as i64 + index;
                if let Some(d) = b.checked_add_signed(chrono::Duration::days(days)) {
                    dates.push(d.format("%Y-%m-%d").to_string());
                }
            }
        }
        let deadline = [
            "截止",
            "之前",
            "前完成",
            "前提交",
            "有效期",
            "内完成",
            "内作答",
        ]
        .iter()
        .any(|s| line.contains(s));
        if dates.is_empty() && deadline && !line.contains("时长") && !line.contains("预计") {
            if let (Some(c), Some(b)) = (duration.captures(line), base) {
                // No event-relative calculation: only explicit mail/receipt validity.
                let anchored = ["有效期", "收到", "收件", "本邮件", "此邮件", "发送"]
                    .iter()
                    .any(|s| line.contains(s));
                if anchored {
                    let n: i64 = c[1].parse().unwrap_or(0);
                    let delta = if &c[2] == "小时" {
                        chrono::Duration::hours(n)
                    } else {
                        chrono::Duration::days(n)
                    };
                    if let Some(d) = b.checked_add_signed(delta) {
                        dates.push(d.format("%Y-%m-%d").to_string());
                        times = vec![d.format("%H:%M").to_string()];
                    }
                }
            }
        }
        if dates.is_empty() {
            continue;
        }
        let range = ["至", "到", "~", "～", "—", "–"]
            .iter()
            .any(|s| line.contains(s))
            || (line.contains("开始") && line.contains("截止"))
            || (times.len() == 2 && line.contains('-'));
        let mut timing = Timing {
            date: dates[0].clone(),
            time: times.first().cloned().unwrap_or_default(),
            kind: if deadline {
                "deadline"
            } else if ["面试", "笔试", "宣讲会"].contains(&kind)
                || line.contains("开始")
                || line.contains("开考")
            {
                "start"
            } else {
                "unknown"
            }
            .into(),
            evidence: line.into(),
            uncertain: times.iter().any(|t| t.is_empty()),
            ..Default::default()
        };
        if range && (dates.len() == 2 || times.len() == 2) {
            timing.kind = "window".into();
            timing.end_date = dates.get(1).unwrap_or(&dates[0]).clone();
            timing.end_time = times.get(1).cloned().unwrap_or_default();
            if timing.end_date < timing.date
                || (timing.end_date == timing.date
                    && !timing.end_time.is_empty()
                    && timing.end_time < timing.time)
            {
                timing.uncertain = true;
                timing.end_date.clear();
                timing.end_time.clear();
            }
        } else if dates.len() > 1 || times.len() > 1 {
            timing.uncertain = true;
        }
        // Prefer labeled event dates over unrelated dates in the subject/body.
        let labeled = [
            "时间",
            "开考",
            "开始",
            "截止",
            "之前",
            "请于",
            "有效期",
            "安排",
            "明天",
            "后天",
            "下周",
        ]
        .iter()
        .any(|s| line.contains(s));
        candidates.push((labeled, timing));
    }
    let labeled = candidates.iter().any(|c| c.0);
    let relevant: Vec<_> = candidates
        .into_iter()
        .filter(|c| !labeled || c.0)
        .map(|c| c.1)
        .collect();
    let Some(mut result) = relevant.first().cloned() else {
        return Timing {
            kind: "unknown".into(),
            uncertain: true,
            ..Default::default()
        };
    };
    if relevant.len() == 2 {
        let start = relevant.iter().find(|t| {
            t.kind == "start" && (t.evidence.contains("开始") || t.evidence.contains("开考"))
        });
        let end = relevant.iter().find(|t| t.kind == "deadline");
        if let (Some(start), Some(end)) = (start, end) {
            if end.date > start.date || (end.date == start.date && end.time >= start.time) {
                return Timing {
                    kind: "window".into(),
                    end_date: end.date.clone(),
                    end_time: end.time.clone(),
                    evidence: relevant
                        .iter()
                        .map(|t| t.evidence.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    uncertain: start.uncertain || end.uncertain,
                    ..start.clone()
                };
            }
        }
    }
    if relevant
        .iter()
        .skip(1)
        .any(|t| t.date != result.date || t.time != result.time || t.kind != result.kind)
    {
        // Multiple competing dates need human confirmation, never silently pick one.
        result.uncertain = true;
        result.date.clear();
        result.time.clear();
        result.end_date.clear();
        result.end_time.clear();
        result.kind = "unknown".into();
        result.evidence = relevant
            .iter()
            .map(|t| t.evidence.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }
    result
}

pub fn labeled_value(text: &str, label: &str) -> String {
    Regex::new(&format!(r"{}[:：]\s*([^\n，。;；|]{{1,60}})", label))
        .unwrap()
        .captures(text)
        .map(|c| clean(&c[1]))
        .unwrap_or_default()
}

pub fn normalize(extracted: &ExtractedItem, email: &ParsedEmail) -> Schedule {
    let raw = format!("{}\n{}", email.subject, email.body);
    let event_type = event_type(&extracted.r#type);
    let blocks = crate::rule_parser::event_blocks(email);
    let evidence = extracted.time_evidence.trim();
    // Every field is scoped to one activity, including undated activities.
    let matching: Vec<_> = blocks
        .iter()
        .filter(|(kind, _)| event_type == *kind)
        .collect();
    let context = if !extracted.context.is_empty()
        && raw.contains(&extracted.context)
        && (evidence.is_empty() || extracted.context.contains(evidence))
    {
        extracted.context.as_str()
    } else if !evidence.is_empty() {
        matching
            .iter()
            .find(|(_, block)| block.contains(evidence))
            .map(|(_, block)| block.as_str())
            .unwrap_or("")
    } else if matching.len() == 1 {
        matching[0].1.as_str()
    } else {
        ""
    };
    let grounded = !evidence.is_empty() && context.contains(evidence);
    let timing = extract_timing(
        if grounded { evidence } else { context },
        &email.date,
        &event_type,
    );
    let role = if !extracted.role.trim().is_empty() && context.contains(extracted.role.trim()) {
        clean(&extracted.role)
    } else {
        labeled_value(context, "(?:应聘岗位|面试岗位|岗位|职位)")
    };
    let round = if event_type != "面试" {
        String::new()
    } else if !extracted.round.trim().is_empty() && context.contains(extracted.round.trim()) {
        clean(&extracted.round)
    } else {
        Regex::new(r"(?:第[一二三四五六七八九\d]+轮|[一二三四五六七八九]面|初试|复试|终面|初面)")
            .unwrap()
            .find(context)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default()
    };
    let urls = Regex::new(r#"https?://[^\s<>，。、“”"'()\[\]]+"#).unwrap();
    let link = if !extracted.link.is_empty()
        && urls.is_match(&extracted.link)
        && context.contains(&extracted.link)
    {
        extracted.link.clone()
    } else {
        urls.find_iter(context)
            .find(|m| {
                [
                    "exam",
                    "meeting",
                    "zoom",
                    "feishu",
                    "nowcoder",
                    "mokahr",
                    "bsurl",
                    "interview",
                    "wjx",
                ]
                .iter()
                .any(|s| m.as_str().contains(s))
            })
            .map(|m| m.as_str().to_string())
            .unwrap_or_default()
    };
    let location = if !extracted.location.is_empty() && context.contains(&extracted.location) {
        clean(&extracted.location)
    } else {
        labeled_value(context, "(?:地点|教室|校区|腾讯会议|会议ID|会议号)")
    };
    let mut notes = clean(&extracted.details);
    if !link.is_empty() {
        notes = notes.replace(&link, "");
    }
    if !location.is_empty() {
        notes = notes.replace(&location, "");
    }
    notes = notes
        .replace("链接:", "")
        .replace("链接：", "")
        .replace("地点：", "")
        .replace("地点:", "");
    let notes = notes
        .trim_matches(|c: char| c.is_whitespace() || c == '|')
        .to_string();
    let company_raw = clean(&extracted.company);
    let timezone = if raw.contains("北京时间") || raw.contains("UTC+08:00") {
        "UTC+08:00".into()
    } else {
        chrono::DateTime::parse_from_rfc2822(&email.date)
            .ok()
            .map(|d| format!("UTC{}", d.format("%:z")))
            .unwrap_or_default()
    };
    Schedule {
        event_type,
        company: company_name(&company_raw),
        company_raw,
        role,
        round,
        date: timing.date.clone(),
        time: timing.time,
        end_date: timing.end_date,
        end_time: timing.end_time,
        time_kind: timing.kind.clone(),
        timezone,
        location,
        link,
        notes,
        source_subject: email.subject.clone(),
        source_date: email.date.clone(),
        time_evidence: timing.evidence,
        needs_confirmation: timing.uncertain
            || timing.date.is_empty()
            || timing.kind == "unknown"
            || extracted.company.trim().is_empty(),
    }
}

pub fn title(schedule: &Schedule, fallback: &str) -> String {
    let company = if schedule.company.is_empty() {
        "公司待确认"
    } else {
        &schedule.company
    };
    let mut pieces = vec![company.to_string()];
    for value in [&schedule.role, &schedule.round] {
        if !value.is_empty() {
            pieces.push(value.clone());
        }
    }
    if schedule.event_type == "其他" && !fallback.trim().is_empty() {
        pieces.push(clean(fallback));
    }
    format!("[{}] {}", schedule.event_type, pieces.join(" · "))
}

/// Only sufficiently identified events may be merged across separate emails.
pub fn same_event(a: &Schedule, b: &Schedule, same_email: bool) -> bool {
    let identity_matches = a.company == b.company
        && a.event_type == b.event_type
        && a.role == b.role
        && a.round == b.round
        && a.date == b.date
        && a.time == b.time
        && a.end_date == b.end_date
        && a.end_time == b.end_time
        && a.time_kind == b.time_kind
        && a.timezone == b.timezone;
    if !identity_matches {
        return false;
    }
    if same_email {
        return ((!a.needs_confirmation && valid_date(&a.date) && !a.time.is_empty())
            || (a.notes == b.notes && a.time_evidence == b.time_evidence))
            && (a.location.is_empty() || b.location.is_empty() || a.location == b.location)
            && (a.link.is_empty() || b.link.is_empty() || a.link == b.link);
    }
    if (!a.location.is_empty() && !b.location.is_empty() && a.location != b.location)
        || (!a.link.is_empty() && !b.link.is_empty() && a.link != b.link)
    {
        return false;
    }
    let exact_notice = !a.time_evidence.is_empty()
        && a.time_evidence == b.time_evidence
        && a.source_subject == b.source_subject
        && a.notes == b.notes;
    (exact_notice || (!a.needs_confirmation && !b.needs_confirmation))
        && !a.company.is_empty()
        && valid_date(&a.date)
        && (!a.time.is_empty() || !a.role.is_empty() || !a.round.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(body: &str) -> ParsedEmail {
        ParsedEmail {
            uid: 7,
            from: "hr@example.com".into(),
            subject: "【中兴通讯】面试邀请".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body: body.into(),
        }
    }

    fn item() -> ExtractedItem {
        ExtractedItem {
            r#type: "面试".into(),
            company: "中兴通讯".into(),
            title: "自由标题".into(),
            ..Default::default()
        }
    }

    #[test]
    fn both_engines_share_title_and_time() {
        let email = mail("面试时间：2026-10-15 14:00\n岗位：后端开发\n轮次：二面");
        let a = normalize(&item(), &email);
        let mut ai = item();
        ai.title = "中兴通讯-后端开发-二面".into();
        ai.role = "后端开发".into();
        ai.round = "二面".into();
        let b = normalize(&ai, &email);
        assert_eq!(a, b);
        assert_eq!(title(&a, "ignored"), "[面试] 中兴通讯 · 后端开发 · 二面");
    }

    #[test]
    fn instruction_numbers_are_not_dates() {
        for text in [
            "测评时间约30分钟，请参照图1-1完成操作",
            "安装指南见第1-1节",
            "浏览器版本1.1.0",
        ] {
            assert!(
                extract_timing(text, "Tue, 13 Oct 2026 10:00:00 +0800", "测评")
                    .date
                    .is_empty(),
                "{text}"
            );
        }
        assert_eq!(
            extract_timing(
                "测评截止时间：10-18",
                "Tue, 13 Oct 2026 10:00:00 +0800",
                "测评"
            )
            .date,
            "2026-10-18"
        );
    }

    #[test]
    fn working_day_range_is_not_a_calendar_date() {
        let timing = extract_timing(
            "笔试时间将在1-2个工作日发送",
            "Sat, 10 Oct 2026 10:00:00 +0800",
            "笔试",
        );
        assert!(timing.date.is_empty());
    }

    #[test]
    fn same_email_notes_changes_do_not_create_another_dated_event() {
        let a = normalize(&item(), &mail("面试时间：2026-10-15 14:00"));
        let mut b = a.clone();
        b.notes = "提前10分钟".into();
        assert!(same_event(&a, &b, true));
        let mut uncertain_a = a.clone();
        uncertain_a.time.clear();
        uncertain_a.needs_confirmation = true;
        let mut uncertain_b = uncertain_a.clone();
        uncertain_b.notes = "下午场主管交流".into();
        assert!(!same_event(&uncertain_a, &uncertain_b, true));
        let mut repeated = a.clone();
        repeated.needs_confirmation = true;
        let mut same = repeated.clone();
        same.source_subject = repeated.source_subject.clone();
        assert!(same_event(&repeated, &same, false));
    }

    #[test]
    fn missing_time_is_retained_for_confirmation() {
        let schedule = normalize(&item(), &mail("面试时间另行通知，预计时长2小时"));
        assert!(schedule.date.is_empty());
        assert!(schedule.needs_confirmation);
        assert_eq!(schedule.time_kind, "unknown");
    }

    #[test]
    fn invalid_date_and_time_are_not_trusted() {
        assert!(!valid_date("2026-02-30"));
        let mut extracted = item();
        extracted.date = "2026-10-12".into();
        extracted.time = "25:99".into();
        let schedule = normalize(&extracted, &mail("面试时间：2026-02-30 25:99"));
        assert!(schedule.date.is_empty());
        assert!(schedule.time.is_empty());
        assert!(schedule.needs_confirmation);
    }

    #[test]
    fn deadlines_and_same_day_ranges_are_distinct() {
        let deadline = normalize(&item(), &mail("请在2026-10-15 18:00之前完成面试"));
        assert_eq!(deadline.time_kind, "deadline");
        assert_eq!(deadline.time, "18:00");
        let window = normalize(&item(), &mail("面试时间：2026-10-15 14:00-15:00"));
        assert_eq!(window.time_kind, "window");
        assert_eq!(window.end_time, "15:00");
        assert_eq!(window.end_date, "2026-10-15");
        assert!(!window.needs_confirmation);
    }

    #[test]
    fn next_week_is_relative_to_the_mail() {
        let schedule = normalize(&item(), &mail("面试时间：下周一 09:30"));
        assert_eq!(schedule.date, "2026-10-12");
        assert_eq!(schedule.time, "09:30");
    }

    #[test]
    fn unknown_mail_date_does_not_use_today() {
        let mut email = mail("面试时间：明天 14:00");
        email.date = "invalid".into();
        assert!(normalize(&item(), &email).date.is_empty());
    }

    #[test]
    fn different_sessions_are_not_duplicates() {
        let a = normalize(&item(), &mail("面试时间：2026-10-15 14:00"));
        let b = normalize(&item(), &mail("面试时间：2026-10-15 16:00"));
        assert!(!same_event(&a, &b, true));
        assert!(!same_event(&a, &b, false));
        assert!(same_event(&a, &a, false));
        let mut different_round = a.clone();
        different_round.round = "二面".into();
        assert!(!same_event(&a, &different_round, false));
    }

    #[test]
    fn unknown_events_are_not_merged_across_emails() {
        let a = normalize(&item(), &mail("面试时间另行通知"));
        assert!(!same_event(&a, &a, false));
        assert!(same_event(&a, &a, true));
    }

    #[test]
    fn ungrounded_ai_evidence_cannot_supply_a_date() {
        let mut extracted = item();
        extracted.date = "2026-10-15".into();
        extracted.time_evidence = "面试时间：2026-10-15 14:00".into();
        assert!(normalize(&extracted, &mail("面试日期另行通知"))
            .date
            .is_empty());
    }

    #[test]
    fn afternoon_with_spaces_is_not_morning() {
        let s = normalize(&item(), &mail("面试时间：2026-10-15 下午 2:30"));
        assert_eq!(s.time, "14:30");
    }

    #[test]
    fn separate_start_and_deadline_lines_form_a_window() {
        let s = normalize(
            &item(),
            &mail("面试开始时间：2026-10-15 09:00\n面试截止时间：2026-10-16 18:00"),
        );
        assert_eq!(s.time_kind, "window");
        assert_eq!(s.date, "2026-10-15");
        assert_eq!(s.end_date, "2026-10-16");
        assert!(!s.needs_confirmation);
    }

    #[test]
    fn notes_are_not_only_links_and_locations() {
        let rule_item = crate::rule_parser::parse_email_by_rules(&mail(
            "面试时间：2026-10-15 14:00\n注意：提前10分钟进入\n预计时长：60分钟",
        ))
        .unwrap();
        let s = normalize(
            &rule_item,
            &mail("面试时间：2026-10-15 14:00\n注意：提前10分钟进入\n预计时长：60分钟"),
        );
        assert!(s.notes.contains("提前10分钟进入"));
        assert!(s.notes.contains("60分钟"));
    }

    #[test]
    fn undated_interview_does_not_inherit_test_time() {
        let email = mail("笔试时间：2026-10-15 10:00\n面试时间另行通知");
        let s = normalize(&item(), &email);
        assert!(s.date.is_empty());
        assert!(s.needs_confirmation);
    }

    #[test]
    fn malformed_times_are_not_truncated() {
        for clock in ["14:3", "14:300", "114:30", "14:99", "25:00"] {
            let s = normalize(&item(), &mail(&format!("面试时间：2026-10-15 {}", clock)));
            assert!(s.time.is_empty(), "{} became {}", clock, s.time);
            assert!(s.needs_confirmation);
        }
    }

    #[test]
    fn reminder_can_omit_link_and_location() {
        let a = normalize(
            &item(),
            &mail("面试时间：2026-10-15 14:00\n地点：会议室A\n链接：https://meeting.example.com/a"),
        );
        let b = normalize(&item(), &mail("面试时间：2026-10-15 14:00"));
        assert!(same_event(&a, &b, false));
        let c = normalize(
            &item(),
            &mail("面试时间：2026-10-15 14:00\n地点：会议室B\n链接：https://meeting.example.com/b"),
        );
        assert!(!same_event(&a, &c, false));
    }
}
