use crate::ai::ExtractedItem;
use crate::email::ParsedEmail;
use regex::Regex;

/// Rule-based parser for recruitment/interview emails.
/// Extracts company, event type, date, time, and links using pattern matching and regex.
/// Works 100% offline without needing any external AI API.
pub fn parse_email_by_rules(email: &ParsedEmail) -> Option<ExtractedItem> {
    let subject = &email.subject;
    let body = &email.body;

    // 1. Identify event type
    let event_type = detect_event_type(subject, body)?;

    // 2. Extract company name
    let company = extract_company(subject, body, &email.from);

    // 3. Extract event date and time
    let timing =
        crate::task::extract_timing(&format!("{}\n{}", subject, body), &email.date, &event_type);
    let (date, time) = (timing.date, timing.time);

    // 4. Extract key details (links, platform, notes, location)
    let details = extract_details(body);

    // 5. Build title
    let title = if !company.is_empty() {
        format!("{}-{}", company, event_type)
    } else {
        format!("校招{}", event_type)
    };

    Some(ExtractedItem {
        r#type: event_type,
        title,
        date,
        time,
        company,
        details,
        role: crate::task::labeled_value(
            &format!("{}\n{}", subject, body),
            "(?:应聘岗位|面试岗位|岗位|职位)",
        ),
        round: Regex::new(
            r"(?:第[一二三四五六七八九\d]+轮|[一二三四五六七八九]面|初试|复试|终面|初面)",
        )
        .unwrap()
        .find(&format!("{}\n{}", subject, body))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default(),
        time_evidence: timing.evidence,
        context: body.clone(),
        ..Default::default()
    })
}

/// Preserve activity paragraphs, including activities whose time is still unknown.
pub fn event_blocks(email: &ParsedEmail) -> Vec<(String, String)> {
    let mut headers: Vec<(usize, String, &str, bool)> = Vec::new();
    let mut offset = 0;
    for line in email.body.split_inclusive('\n') {
        let is_heading = [
            "时间",
            "日期",
            "安排",
            "另行",
            "待定",
            "邀请",
            "通知",
            "截止",
            "开始",
            "之前",
            "有效期",
        ]
        .iter()
        .any(|k| line.contains(k))
            && !["时长", "注意", "链接", "材料"]
                .iter()
                .any(|k| line.contains(k));
        if is_heading {
            if let Some(kind) = detect_event_type(line, "") {
                let definite = !crate::task::extract_timing(line, &email.date, &kind)
                    .date
                    .is_empty()
                    || line.contains("另行")
                    || line.contains("待定");
                if let Some(previous) = headers.last_mut() {
                    if previous.1 == kind && !previous.3 {
                        // The invitation introduction belongs to its following schedule.
                        previous.2 = line;
                        previous.3 = definite;
                    } else if definite || previous.1 != kind {
                        headers.push((offset, kind, line, definite));
                    }
                } else {
                    headers.push((offset, kind, line, definite));
                }
            }
        }
        offset += line.len();
    }
    let paired_window = headers.len() == 2
        && headers[0].1 == headers[1].1
        && headers
            .iter()
            .any(|h| h.2.contains("开始") || h.2.contains("开考"))
        && headers
            .iter()
            .any(|h| h.2.contains("截止") || h.2.contains("之前"));
    if headers.len() < 2 || paired_window {
        let kind = headers
            .first()
            .map(|h| h.1.clone())
            .or_else(|| detect_event_type(&email.subject, &email.body))
            .unwrap_or_else(|| "其他".into());
        let subject_kind = detect_event_type(&email.subject, "");
        let context = if subject_kind.as_ref() == Some(&kind) {
            format!("{}\n{}", email.subject, email.body)
        } else {
            email.body.clone()
        };
        return vec![(kind, context)];
    }
    headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let start = if i == 0 { 0 } else { h.0 };
            let end = headers.get(i + 1).map_or(email.body.len(), |next| next.0);
            (h.1.clone(), email.body[start..end].trim().to_string())
        })
        .collect()
}

pub fn parse_email_items_by_rules(email: &ParsedEmail) -> Vec<ExtractedItem> {
    if !crate::email_scanner::is_candidate_email(&email.subject, &email.body) {
        return vec![];
    }
    let blocks = event_blocks(email);
    let common_company = extract_company(&email.subject, "", "");
    blocks
        .into_iter()
        .filter_map(|(kind, block)| {
            if kind == "其他" {
                return None;
            }
            let company = extract_company("", &block, "");
            let company = if company.is_empty() {
                &common_company
            } else {
                &company
            };
            let scoped = ParsedEmail {
                subject: format!("{}{}", company, kind),
                body: block,
                ..email.clone()
            };
            parse_email_by_rules(&scoped)
        })
        .collect()
}

/// Detect whether the email is an Interview, Test, Assessment, Talk, or Offer
fn detect_event_type(subject: &str, body: &str) -> Option<String> {
    // 1. Prioritize SUBJECT keywords (subject states the primary purpose)
    let s = subject.to_lowercase();
    if s.contains("笔试") || s.contains("机试") || s.contains("专业考试") || s.contains("在线测试")
    {
        return Some("笔试".to_string());
    }
    if s.contains("测评")
        || s.contains("人才测评")
        || s.contains("性格测试")
        || s.contains("素质测试")
        || s.contains("assessment")
    {
        return Some("测评".to_string());
    }
    if s.contains("宣讲会")
        || s.contains("空中宣讲")
        || s.contains("专场宣讲")
        || s.contains("宣讲")
        || s.contains("双选会")
        || s.contains("交流会")
    {
        return Some("宣讲会".to_string());
    }
    if (s.contains("面试")
        || s.contains("面谈")
        || s.contains("复试")
        || s.contains("初试")
        || s.contains("终面")
        || s.contains("直面")
        || s.contains("interview")
        || ["一面", "二面", "三面", "初面"]
            .iter()
            .any(|k| s.contains(k)))
        && !s.contains("抢面试")
        && !s.contains("锁定面试")
        && !s.contains("赢取面试")
        && !s.contains("面试先机")
    {
        return Some("面试".to_string());
    }
    if (s.contains("录用通知")
        || s.contains("offer发放")
        || s.contains("offer邀请")
        || (s.contains("签约通知") || s.contains("签约截止"))
        || s.contains("拟录用")
        || s.contains("录用意向"))
        && !s.contains("投递")
        && !s.contains("推荐")
        && !s.contains("拿offer")
    {
        return Some("签约".to_string());
    }

    if s.contains("投递截止") || s.contains("申请截止") {
        return Some("投递".to_string());
    }

    // 2. Fall back to body keywords if subject doesn't specify
    let b = body.to_lowercase();
    if b.contains("笔试")
        || b.contains("机试")
        || b.contains("考试开始时间")
        || b.contains("在线作答")
        || b.contains("线上考核")
    {
        Some("笔试".to_string())
    } else if b.contains("测评")
        || b.contains("素质测评")
        || b.contains("综合测评")
        || b.contains("人才测评")
        || b.contains("测评作答")
    {
        Some("测评".to_string())
    } else if b.contains("宣讲会") || b.contains("空中宣讲") || b.contains("专场宣讲") {
        Some("宣讲会".to_string())
    } else if b.contains("面试")
        || b.contains("面谈")
        || b.contains("复试")
        || b.contains("初试")
        || b.contains("终面")
        || b.contains("直面")
    {
        Some("面试".to_string())
    } else if b.contains("录用") || b.contains("offer") || b.contains("签约") {
        Some("签约".to_string())
    } else if b.contains("投递截止") || b.contains("申请截止") {
        Some("投递".into())
    } else {
        None
    }
}

/// Extract company name from subject or sender
fn extract_company(subject: &str, body: &str, from: &str) -> String {
    // 1. Check known high-frequency campus recruitment companies
    let known = [
        "长鑫科技",
        "长鑫存储",
        "长鑫",
        "携程集团",
        "携程",
        "中兴通讯",
        "中兴",
        "百度",
        "腾讯",
        "阿里巴巴",
        "阿里",
        "字节跳动",
        "字节",
        "美团",
        "快手",
        "小鹏汽车",
        "小鹏",
        "华为",
        "网易",
        "宁德时代",
        "作业帮",
        "科锐国际",
        "科锐",
        "宁银消金",
        "龙蟠科技",
        "白云电气",
        "通用技术",
        "吉利汽车",
        "吉利",
        "小米",
        "大疆",
        "比亚迪",
        "蔚来",
        "理想汽车",
        "理想",
        "商汤",
        "旷视",
        "米哈游",
        "完美世界",
        "深信服",
        "海康威视",
        "联想",
        "经纬恒润",
        "远景能源",
        "思源电气",
    ];
    for comp in &known {
        if subject.contains(comp) || body.contains(comp) {
            return comp.to_string();
        }
    }

    // 2. Try extract from 【Company】 or [Company] in subject
    let re_bracket = Regex::new(r"[【\[]([\u4e00-\u9fa5A-Za-z0-9_]{2,15})[】\]]").unwrap();
    for caps in re_bracket.captures_iter(subject) {
        if let Some(m) = caps.get(1) {
            let name = m.as_str();
            if !name.contains("推荐")
                && !name.contains("智联")
                && !name.contains("校招")
                && !name.contains("通知")
                && !name.contains("招聘")
                && !name.contains("宣讲")
                && !name.contains("求职")
            {
                return name.to_string();
            }
        }
    }

    // 3. Try "来自xxx(公司/集团)的..."
    let re_from = Regex::new(
        r"来自([\u4e00-\u9fa5A-Za-z0-9]{2,8}?)(?:集团|有限|股份|公司|科技|网络|技术|证券|银行)",
    )
    .unwrap();
    if let Some(caps) = re_from.captures(subject) {
        if let Some(m) = caps.get(1) {
            let name = m.as_str();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }

    // 4. Look for common company suffixes in subject
    let re_corp = Regex::new(r"([\u4e00-\u9fa5A-Za-z0-9]{2,8})(?:集团|科技|网络|电气|游戏|通讯|汽车|健康|控股|证券|银行)").unwrap();
    if let Some(caps) = re_corp.captures(subject) {
        if let Some(m) = caps.get(0) {
            let name = m.as_str();
            if !name.contains("智联") && !name.contains("推荐") && !name.contains("专场") {
                return name.to_string();
            }
        }
    }

    // A sender domain is not evidence of the employing company.
    let _ = from;
    String::new()
}

/// Extract links, meeting info, or location from email body
fn extract_details(body: &str) -> String {
    let notes = body
        .split(['\n', '。', '！'])
        .filter(|s| {
            ["注意", "准备", "材料", "时长", "提前", "携带"]
                .iter()
                .any(|k| s.contains(k))
        })
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" | ");
    let mut link = String::new();
    let re_url = Regex::new(r"https?://[^\s<>，。、“”\(\)\[\]]+").unwrap();
    for cap in re_url.find_iter(body) {
        let url = cap.as_str();
        if url.contains("nowcoder")
            || url.contains("mokahr")
            || url.contains("bsurl")
            || url.contains("wjx")
            || url.contains("meeting")
            || url.contains("zoom")
            || url.contains("feishu")
            || url.contains("exam")
            || url.contains("hire")
            || url.contains("career")
            || url.contains("interview")
        {
            link = format!("链接: {}", url);
            break;
        }
    }

    // Check for meeting id or classroom/location
    let re_location =
        Regex::new(r"(?:地点|教室|校区|腾讯会议|会议ID|会议号)[:：]\s*([^\r\n，。]{2,40})")
            .unwrap();
    if let Some(caps) = re_location.captures(body) {
        if let Some(m) = caps.get(0) {
            let loc = m.as_str().trim();
            if !link.is_empty() {
                return format!("{} | {} | {}", loc, link, notes);
            } else {
                return format!("{} | {}", loc, notes);
            }
        }
    }

    format!("{} | {}", link, notes)
        .trim_matches(|c: char| c.is_whitespace() || c == '|')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(body: &str) -> ParsedEmail {
        ParsedEmail {
            uid: 42,
            from: "hr@example.com".into(),
            subject: "【百度】面试邀请".into(),
            date: "Thu, 31 Dec 2026 10:00:00 +0800".into(),
            body: body.into(),
        }
    }

    #[test]
    fn one_email_can_have_two_events() {
        let email = sample("笔试时间：2027-01-01 10:00\n面试时间：2027-01-02 14:00");
        let items = parse_email_items_by_rules(&email);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].r#type, "笔试");
        assert_eq!(items[1].r#type, "面试");
    }

    #[test]
    fn flattened_notes_do_not_copy_entire_brochure() {
        let notes = extract_details(
            "公司介绍：我们是一家科技公司。注意：提前10分钟进入。招聘流程：简历筛选、笔试、面试。",
        );
        assert!(notes.contains("提前10分钟"));
        assert!(!notes.contains("公司介绍"));
        assert!(!notes.contains("招聘流程"));
        let checklist = extract_details("请携带以下材料：身份证；学生证；成绩单。");
        assert!(checklist.contains("成绩单"));
    }

    #[test]
    fn one_window_is_not_split_into_two_events() {
        let email = sample("面试开始时间：2027-01-01 10:00\n面试截止时间：2027-01-02 14:00");
        assert_eq!(parse_email_items_by_rules(&email).len(), 1);
    }

    #[test]
    fn undated_second_event_is_preserved() {
        let items =
            parse_email_items_by_rules(&sample("笔试时间：2027-01-01 10:00\n面试时间另行通知"));
        assert_eq!(items.len(), 2);
        assert!(items[1].date.is_empty());
    }

    #[test]
    fn activity_fields_are_isolated_to_their_block() {
        let email = sample("笔试时间：2027-01-01 10:00\n岗位：后端开发\n地点：教室A\n链接：https://exam.example.com/a\n面试时间：2027-01-02 14:00\n岗位：前端开发\n地点：会议室B\n链接：https://meeting.example.com/b");
        let items = parse_email_items_by_rules(&email);
        let second = crate::task::normalize(&items[1], &email);
        assert_eq!(second.role, "前端开发");
        assert_eq!(second.location, "会议室B");
        assert_eq!(second.link, "https://meeting.example.com/b");
        let first = crate::task::normalize(&items[0], &email);
        assert_eq!(first.role, "后端开发");
        assert_eq!(first.location, "教室A");
    }

    #[test]
    fn invitation_intro_does_not_create_an_extra_event() {
        let email = sample("现邀请您参加在线笔试。\n笔试时间：2027-01-01 19:00\n考试链接：https://exam.nowcoder.com/test/baidu");
        let items = parse_email_items_by_rules(&email);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].date, "2027-01-01");
    }

    #[test]
    fn subject_only_schedule_and_round_are_preserved() {
        let mut email = sample("请提前十分钟进入会议。");
        email.subject = "【百度】面试岗位：后端开发；二面时间：2027-01-01 14:00".into();
        let items = parse_email_items_by_rules(&email);
        assert_eq!(items.len(), 1);
        let s = crate::task::normalize(&items[0], &email);
        assert_eq!(s.date, "2027-01-01");
        assert_eq!(s.time, "14:00");
        assert_eq!(s.role, "后端开发");
        assert_eq!(s.round, "二面");
    }

    #[test]
    fn missing_date_is_not_invented() {
        let item = parse_email_by_rules(&sample("您已进入面试环节，具体时间另行通知")).unwrap();
        assert!(item.date.is_empty(), "缺少时间不能自动填邮件日期加3天");
    }

    #[test]
    fn tomorrow_uses_email_date() {
        let item = parse_email_by_rules(&sample("面试时间：明天 14:00")).unwrap();
        assert_eq!(item.date, "2027-01-01");
        assert_eq!(item.time, "14:00");
    }

    #[test]
    fn duration_is_not_a_deadline() {
        let item = parse_email_by_rules(&sample("面试时长2小时，日期另行通知")).unwrap();
        assert!(item.date.is_empty());
    }

    #[test]
    fn missing_year_uses_email_year_and_rolls_over() {
        let item = parse_email_by_rules(&sample("面试时间：1月2日 09:00")).unwrap();
        assert_eq!(item.date, "2027-01-02");
    }

    #[test]
    fn test_parse_ctrip_email() {
        let email = ParsedEmail {
            uid: 2057,
            from: "hr@trip.com".to_string(),
            subject: "【携程集团】在线专业笔试邀请函".to_string(),
            date: "Sat, 10 Oct 2026 17:42:29 +0800".to_string(),
            body: "考试开始时间： (北京时间,UTC+08:00) 2026-10-13 19:00:00 考试链接： https://exam.nowcoder.com/cts/123".to_string(),
        };

        let item = parse_email_by_rules(&email).expect("Should parse Ctrip email");
        assert_eq!(item.r#type, "笔试");
        assert_eq!(item.company, "携程集团");
        assert_eq!(item.date, "2026-10-13");
        assert_eq!(item.time, "19:00");
        assert!(item.details.contains("nowcoder.com"));
    }

    #[test]
    fn test_parse_changxin_email() {
        let email = ParsedEmail {
            uid: 2041,
            from: "hr@cxmt.com".to_string(),
            subject: "来自长鑫科技集团股份有限公司的AI面试邀请".to_string(),
            date: "Sat, 10 Oct 2026 01:40:36 +0800".to_string(),
            body: "你已进入【长鑫存储校园招聘-AI初试】环节，请充分准备后在 2026-10-17 23:59 之前完成面试，链接：https://bsurl.cn/v2/abc".to_string(),
        };

        let item = parse_email_by_rules(&email).expect("Should parse Changxin email");
        assert_eq!(item.r#type, "面试");
        assert_eq!(item.company, "长鑫科技");
        assert_eq!(item.date, "2026-10-17");
        assert_eq!(item.time, "23:59");
    }

    #[test]
    fn test_parse_zte_email() {
        let email = ParsedEmail {
            uid: 2058,
            from: "hr@zte.com.cn".to_string(),
            subject: "中兴通讯校园招聘测评通知".to_string(),
            date: "Sat, 10 Oct 2026 17:48:12 +0800".to_string(),
            body: "您已进入人才测评阶段，点击下方链接可开始测评， https://app.mokahr.com/su/abc 注意：链接有效期为72小时".to_string(),
        };

        let item = parse_email_by_rules(&email).expect("Should parse ZTE email");
        assert_eq!(item.r#type, "测评");
        assert_eq!(item.company, "中兴通讯");
        assert_eq!(item.date, "2026-10-13"); // 10-10 + 72h
    }

    #[test]
    fn test_parse_longpan_email() {
        let email = ParsedEmail {
            uid: 2051,
            from: "campus@zhaopinmail.com".to_string(),
            subject: "尊敬的孙同学【智联推荐】龙蟠科技2027届校园招聘-东南大学宣讲会".to_string(),
            date: "Sat, 10 Oct 2026 12:00:00 +0800".to_string(),
            body: "宣讲会时间：2026年10月15日 14:30\n地点：东南大学九龙湖校区 教三-101".to_string(),
        };

        let item = parse_email_by_rules(&email).expect("Should parse Longpan email");
        assert_eq!(item.r#type, "宣讲会");
        assert_eq!(item.company, "龙蟠科技");
        assert_eq!(item.date, "2026-10-15");
        assert_eq!(item.time, "14:30");
        assert!(item.details.contains("地点：东南大学九龙湖校区"));
    }

    #[test]
    fn test_parse_baidu_email() {
        let email = ParsedEmail {
            uid: 2055,
            from: "campus@baidu.com".to_string(),
            subject: "【百度】邀请你参加2027届校招在线笔试".to_string(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".to_string(),
            body: "现邀请您参加在线笔试。\n笔试时间：2026-10-18 19:00\n考试链接：https://exam.nowcoder.com/test/baidu".to_string(),
        };

        let item = parse_email_by_rules(&email).expect("Should parse Baidu email");
        assert_eq!(item.r#type, "笔试");
        assert_eq!(item.company, "百度");
        assert_eq!(item.date, "2026-10-18");
        assert_eq!(item.time, "19:00");
    }
}
