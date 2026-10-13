use crate::ai::{self, ExtractedItem};
use crate::data::{self, PlannerItem};
use crate::email;
use crate::email_config::{self, EmailConfig};
use crate::rule_parser;
use serde::{Deserialize, Serialize};

/// Result of a single scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub success: bool,
    pub emails_scanned: usize,
    pub items_created: usize,
    pub errors: Vec<String>,
    pub created_items: Vec<ScanCreatedItem>,
    pub scan_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCreatedItem {
    pub email_subject: String,
    pub item_text: String,
    pub item_date: String,
}

fn scan_days_back(_config: &EmailConfig) -> u32 {
    // Keep failed first-scan messages within the next incremental search window.
    30
}

/// Execute a full email scan: fetch → analyze (rules + AI) → create items
pub fn execute_scan() -> Result<ScanResult, String> {
    let config = email_config::read_email_config();

    if !config.enabled {
        return Err("邮件扫描未启用".to_string());
    }
    if config.email_address.is_empty() || config.auth_code.is_empty() {
        return Err("请先配置邮箱地址和授权码".to_string());
    }

    let scan_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut result = ScanResult {
        success: true,
        emails_scanned: 0,
        items_created: 0,
        errors: Vec::new(),
        created_items: Vec::new(),
        scan_time: scan_time.clone(),
    };

    let days_back = scan_days_back(&config);

    let emails = match email::fetch_recent_emails(
        &config.email_address,
        &config.auth_code,
        &config.processed_uids,
        days_back,
    ) {
        Ok(emails) => emails,
        Err(e) => {
            result.success = false;
            result.errors.push(format!("邮件获取失败: {}", e));
            return Ok(result);
        }
    };

    result.emails_scanned = emails.len();

    if emails.is_empty() {
        // Update last scan time even if no new emails
        update_scan_state(&config, &[], &scan_time)?;
        return Ok(result);
    }

    let new_processed_uids = process_emails(
        &config,
        &emails,
        &mut result,
        |mail| ai::analyze_email(mail, &config.gemini_api_key, &config.gemini_model),
        create_planner_item,
    );
    update_scan_state(&config, &new_processed_uids, &scan_time)?;
    Ok(result)
}

fn process_emails(
    config: &EmailConfig,
    emails: &[email::ParsedEmail],
    result: &mut ScanResult,
    mut analyze: impl FnMut(&email::ParsedEmail) -> Result<Vec<ExtractedItem>, String>,
    mut create: impl FnMut(&ExtractedItem, &email::ParsedEmail) -> Result<PlannerItem, String>,
) -> Vec<u32> {
    // Dual-engine processing: Local rule extraction first, fallback/supplement with Gemini AI
    let mut new_processed_uids: Vec<u32> = Vec::new();
    let mut gemini_network_offline = false;

    for parsed_email in emails {
        // Keyword pre-filtering
        if !is_candidate_email(&parsed_email.subject, &parsed_email.body) {
            new_processed_uids.push(parsed_email.uid);
            continue;
        }

        let mut items_to_add: Vec<ExtractedItem> = Vec::new();
        let mut processed = false;

        // Engine 1: Fast local rule-based extractor (100% offline, highly accurate on standard formats)
        let rule_items = rule_parser::parse_email_items_by_rules(parsed_email);
        if !rule_items.is_empty() {
            processed = true;
            items_to_add.extend(rule_items);
        } else if !config.gemini_api_key.is_empty() && !gemini_network_offline {
            // Engine 2: Gemini AI fallback for complex unstructured emails
            match analyze(parsed_email) {
                Ok(items) => {
                    processed = true;
                    items_to_add.extend(items);
                }
                Err(e) => {
                    if e.contains("timed out")
                        || e.contains("error sending request")
                        || e.contains("dns")
                        || e.contains("connection")
                    {
                        gemini_network_offline = true;
                        result
                            .errors
                            .push("AI 服务暂时无法连接；未解析的邮件将在下次扫描重试".to_string());
                    } else {
                        result
                            .errors
                            .push("AI 辅助解析失败；该邮件将在下次扫描重试".into());
                    }
                }
            }
        } else if config.gemini_api_key.is_empty() {
            result
                .errors
                .push("本地规则未能解析一封候选邮件，且未配置 AI 解析；已保留待重试".into());
        }

        // Create tasks for extracted items
        for extracted in &items_to_add {
            if crate::task::event_type(&extracted.r#type) == "宣讲会" {
                continue;
            }
            match create(extracted, parsed_email) {
                Ok(created) => {
                    result.items_created += 1;
                    result.created_items.push(ScanCreatedItem {
                        email_subject: parsed_email.subject.clone(),
                        item_text: created.text.clone(),
                        item_date: created.created_date.clone(),
                    });
                }
                Err(e) => {
                    if !e.contains("已存在") {
                        processed = false;
                        result
                            .errors
                            .push(format!("创建日程失败 ({}): {}", parsed_email.subject, e));
                    }
                }
            }
        }

        if processed {
            new_processed_uids.push(parsed_email.uid);
        }
    }

    result.success = result.errors.is_empty();
    new_processed_uids
}

impl ScanResult {
    pub fn status_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "status": if self.success { "success" } else { "error" },
            "message": if self.success {
                format!("扫描完成: {} 封邮件, 新增 {} 条日程", self.emails_scanned, self.items_created)
            } else {
                format!("扫描未全部完成: 已扫描 {} 封邮件, 新增 {} 条日程；{}", self.emails_scanned, self.items_created, self.errors.join("；"))
            },
            "time": self.scan_time,
        })
    }
}

/// Normalize before persistence; event comparison does not depend on display text.
fn create_planner_item(
    extracted: &ExtractedItem,
    email: &email::ParsedEmail,
) -> Result<PlannerItem, String> {
    let (mut planner_data, _) = data::read_data();
    let item = build_planner_item(extracted, email, &planner_data.items)?;
    planner_data.items.push(item.clone());
    data::write_data(&planner_data)?;
    Ok(item)
}

pub fn linked_item_ids(items: &[PlannerItem], id: &str) -> Vec<String> {
    let Some(target) = items.iter().find(|i| i.id == id) else {
        return vec![id.to_string()];
    };
    let group = representative(items, target);
    items
        .iter()
        .filter(|item| representative(items, item).id == group.id)
        .map(|i| i.id.clone())
        .collect()
}

fn representative<'a>(items: &'a [PlannerItem], item: &'a PlannerItem) -> &'a PlannerItem {
    let base = legacy_pair(items, item).unwrap_or(item);
    let Some(_) = &base.schedule else {
        return base;
    };
    // Every known member must agree; an incomplete reminder cannot bridge conflicts.
    let mut groups: Vec<Vec<&PlannerItem>> = Vec::new();
    for other in items {
        let Some(schedule) = &other.schedule else {
            continue;
        };
        let matched = groups.iter().position(|group| {
            group.iter().all(|leader| {
                leader.done == other.done
                    && schedule.event_type != "其他"
                    && crate::task::same_event(
                        leader.schedule.as_ref().unwrap(),
                        schedule,
                        leader.email_uid == other.email_uid,
                    )
            })
        });
        if other.id == base.id {
            return matched.map_or(other, |index| groups[index][0]);
        }
        if let Some(index) = matched {
            groups[index].push(other);
        } else {
            groups.push(vec![other]);
        }
    }
    base
}

pub fn display_items(items: &[PlannerItem]) -> Vec<PlannerItem> {
    let representatives: Vec<_> = items
        .iter()
        .map(|item| representative(items, item))
        .collect();
    items
        .iter()
        .enumerate()
        .filter(|(index, item)| representatives[*index].id == item.id)
        .map(|(_, item)| {
            let mut visible = item.clone();
            if visible.source == "email" {
                if let Some(schedule) = &mut visible.schedule {
                    if schedule.company.is_empty() {
                        let company = rule_parser::extract_company(&schedule.source_subject, "", "");
                        if !company.is_empty() {
                            let generated_title = crate::task::title(schedule, "");
                            schedule.company = company.clone();
                            schedule.company_raw = company;
                            if visible.text == generated_title {
                                visible.text = crate::task::title(schedule, "");
                            }
                        }
                    }
                    if !schedule.date.is_empty() && !schedule.time_evidence.is_empty() {
                        let timing = crate::task::extract_timing(
                            &schedule.time_evidence,
                            &schedule.source_date,
                            &schedule.event_type,
                        );
                        if timing.date != schedule.date {
                            schedule.date = timing.date;
                            schedule.time = timing.time;
                            schedule.end_date = timing.end_date;
                            schedule.end_time = timing.end_time;
                            schedule.time_kind = timing.kind;
                            schedule.needs_confirmation |= timing.uncertain || schedule.date.is_empty() || schedule.time_kind == "unknown";
                            if !schedule.date.is_empty() {
                                visible.created_date = schedule.date.clone();
                            } else if let Ok(received) =
                                chrono::DateTime::parse_from_rfc2822(&schedule.source_date)
                            {
                                visible.created_date = received.format("%Y-%m-%d").to_string();
                            }
                        }
                    }
                }
            }
            for (index, old) in items.iter().enumerate() {
                if old.id != item.id && representatives[index].id == item.id {
                    if let Some(s) = &mut visible.schedule {
                        let original_notes = old
                            .schedule
                            .as_ref()
                            .map_or("", |schedule| schedule.notes.as_str());
                        s.notes = format!(
                            "{}\n关联记录（保留原文）：{}\n{}",
                            s.notes, old.text, original_notes
                        )
                        .trim()
                        .to_string();
                    }
                }
            }
            visible
        })
        .collect()
}

fn legacy_pair<'a>(items: &'a [PlannerItem], old: &PlannerItem) -> Option<&'a PlannerItem> {
    let kind = legacy_kind(old)?;
    let old_count = items
        .iter()
        .filter(|i| i.email_uid == old.email_uid && legacy_kind(i) == Some(kind))
        .count();
    let matches: Vec<_> = items
        .iter()
        .filter(|new| {
            new.email_uid == old.email_uid
                && new.schedule.as_ref().is_some_and(|s| s.event_type == kind)
        })
        .collect();
    if old_count == 1
        && !matches.is_empty()
        && matches.iter().all(|new| {
            new.done == old.done
                && crate::task::same_event(
                    new.schedule.as_ref().unwrap(),
                    matches[0].schedule.as_ref().unwrap(),
                    new.email_uid == matches[0].email_uid,
                )
        })
    {
        Some(matches[0])
    } else {
        None
    }
}

fn legacy_kind(item: &PlannerItem) -> Option<&str> {
    if item.source != "email" || item.schedule.is_some() || item.email_uid.is_none() {
        return None;
    }
    item.text
        .strip_prefix("📧 [")?
        .split_once(']')
        .map(|(kind, _)| kind)
}

fn build_planner_item(
    extracted: &ExtractedItem,
    email: &email::ParsedEmail,
    existing: &[PlannerItem],
) -> Result<PlannerItem, String> {
    let schedule = crate::task::normalize(extracted, email);
    if schedule.event_type == "宣讲会" {
        return Err("宣讲会以东南大学就业网为准".into());
    }
    let text = crate::task::title(&schedule, &extracted.title);
    if existing.iter().any(|item| {
        if let Some(previous) = &item.schedule {
            crate::task::same_event(previous, &schedule, item.email_uid == Some(email.uid))
                && (schedule.event_type != "其他" || item.text == text)
        } else {
            item.email_uid == Some(email.uid)
                && legacy_kind(item) == Some(schedule.event_type.as_str())
                && rule_parser::parse_email_items_by_rules(email)
                    .iter()
                    .filter(|i| i.r#type == schedule.event_type)
                    .count()
                    == 1
        }
    }) {
        return Err("该事项已存在，已跳过重复添加".into());
    }
    Ok(PlannerItem {
        id: data::generate_id(),
        text,
        done: false,
        // created_date remains the calendar bucket for compatibility. Unknown events
        // use today's inbox bucket but their schedule.date stays empty.
        created_date: if schedule.date.is_empty() {
            data::today()
        } else {
            schedule.date.clone()
        },
        carried_over: false,
        sort_order: existing.len() as i64,
        source: "email".into(),
        email_uid: Some(email.uid),
        schedule: Some(schedule),
    })
}

fn merge_processed_uids(previous: &[u32], added: &[u32]) -> Vec<u32> {
    let mut merged = previous.to_vec();
    merged.extend_from_slice(added);
    merged.sort_unstable();
    merged.dedup();
    merged
}

/// Update the scan state in email config
fn update_scan_state(
    config: &EmailConfig,
    new_uids: &[u32],
    scan_time: &str,
) -> Result<(), String> {
    let mut updated_config = config.clone();
    updated_config.last_scan_time = scan_time.to_string();

    updated_config.processed_uids = merge_processed_uids(&config.processed_uids, new_uids);

    email_config::write_email_config(&updated_config)
}

/// Check if it's time to scan based on the configured interval
pub fn should_scan_now(config: &EmailConfig) -> bool {
    if !config.enabled || config.email_address.is_empty() {
        return false;
    }

    if config.last_scan_time.is_empty() {
        return true;
    }

    // Parse last scan time
    let last_scan =
        chrono::NaiveDateTime::parse_from_str(&config.last_scan_time, "%Y-%m-%d %H:%M:%S");

    match last_scan {
        Ok(last) => {
            let now = chrono::Local::now().naive_local();
            let elapsed = now.signed_duration_since(last);
            elapsed.num_minutes() >= config.scan_interval_minutes as i64
        }
        Err(_) => true, // If we can't parse the time, scan anyway
    }
}

/// Check if an email likely contains interview, test, or schedule items
pub(crate) fn is_candidate_email(subject: &str, body: &str) -> bool {
    let lower_subject = subject.to_lowercase();

    // Check if subject explicitly specifies an action event
    let has_action_keyword = lower_subject.contains("面试")
        || lower_subject.contains("笔试")
        || lower_subject.contains("测评")
        || lower_subject.contains("机试")
        || lower_subject.contains("宣讲")
        || lower_subject.contains("考核")
        || lower_subject.contains("考试")
        || lower_subject.contains("直面")
        || lower_subject.contains("interview")
        || lower_subject.contains("assessment")
        || lower_subject.contains("投递截止")
        || lower_subject.contains("申请截止")
        || lower_subject.contains("签约");

    // 1. Ignore obvious non-action emails (unless they contain an explicit action event)
    let ignore_patterns = [
        "退信",
        "系统通知",
        "订阅",
        "账单",
        "安全提醒",
        "登录提醒",
        "验证码",
        "mail delivery failed",
        "undelivered",
        "推广",
        "投递成功",
        "投递确认",
        "职位申请成功",
        "申请已收到",
        "感谢您的应聘",
        "感谢您投递",
        "感谢您的投递",
        "感谢您的关注",
        "简历投递成功",
        "简历已成功投递",
    ];
    for pattern in &ignore_patterns {
        if lower_subject.contains(pattern) && !has_action_keyword {
            return false;
        }
    }

    // Ignore promotional, announcement, and survey emails
    let promo_patterns = [
        "即将截止",
        "正式启动",
        "招聘公告",
        "简章",
        "满意度调查",
        "求职问卷",
        "诚邀投递",
        "锁定面试机会",
        "宣讲会排期",
        "启动｜",
        "全面启动",
        "抢面试先机",
        "拿offer",
        "赢offer",
        "邀您投递",
        "岗位推荐",
    ];
    for pattern in &promo_patterns {
        if lower_subject.contains(pattern)
            && !lower_subject.contains("邀请")
            && !lower_subject.contains("通知")
            && !lower_subject.contains("提醒")
            && !has_action_keyword
        {
            return false;
        }
    }

    // 2. High-priority subject keywords
    let subject_keywords = [
        "面试",
        "笔试",
        "测评",
        "机试",
        "面谈",
        "复试",
        "初试",
        "终面",
        "宣讲",
        "沟通会",
        "录用",
        "offer",
        "签约",
        "interview",
        "assessment",
        "test",
        "线上测试",
        "在线作答",
        "准考证",
        "考前",
        "笔试通知",
        "测评通知",
        "面试通知",
    ];
    for kw in &subject_keywords {
        if lower_subject.contains(kw) {
            return true;
        }
    }

    // 3. High-priority body keywords
    let body_keywords = [
        "面试邀请",
        "笔试邀请",
        "测评邀请",
        "面试通知",
        "笔试通知",
        "测评通知",
        "请参加",
        "在线作答",
        "面试时间",
        "笔试时间",
        "测评时间",
        "宣讲时间",
        "宣讲会时间",
        "投递截止",
        "申请截止",
        "截止时间",
        "腾讯会议",
        "飞书会议",
        "zoom",
        "准考证",
        "考前准备",
        "考试开始时间",
        "作答链接",
        "测试链接",
        "72小时",
        "48小时",
        "24小时",
        "有效期",
    ];
    for kw in &body_keywords {
        if body.contains(kw) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_company_records_gain_subject_name_without_overwriting_user_titles() {
        for company in ["华润电力", "九方智投"] {
            let mut mail = synthetic_mail("测评时间待确认");
            mail.subject = format!("{company}测评通知");
            let mut original = build_planner_item(&ExtractedItem { r#type: "测评".into(), ..Default::default() }, &mail, &[]).unwrap();
            original.schedule.as_mut().unwrap().company.clear();
            original.schedule.as_mut().unwrap().company_raw.clear();
            original.text = "[测评] 公司待确认".into();
            let visible = display_items(&[original.clone()]);
            assert_eq!(visible[0].text, format!("[测评] {company}"));
            assert!(original.schedule.as_ref().unwrap().company.is_empty());
            original.text = "我编辑的事项".into();
            assert_eq!(display_items(&[original])[0].text, "我编辑的事项");
        }
    }
    #[test]
    fn processed_mail_over_500_does_not_reappear_in_next_scan() {
        let added: Vec<u32> = (1..=600).rev().collect();
        let saved = super::merge_processed_uids(&[600], &added);
        assert!(added.iter().all(|uid| saved.contains(uid)));
        assert_eq!(saved.iter().filter(|uid| **uid == 600).count(), 1);
    }

    #[test]
    fn retry_search_window_does_not_shrink_after_first_scan() {
        let mut config = EmailConfig::default();
        assert_eq!(super::scan_days_back(&config), 30);
        config.last_scan_time = "2026-10-13 10:00:00".into();
        assert_eq!(super::scan_days_back(&config), 30);
    }

    use super::*;

    fn empty_result() -> ScanResult {
        ScanResult {
            success: true,
            emails_scanned: 0,
            items_created: 0,
            errors: vec![],
            created_items: vec![],
            scan_time: "2026-10-13 10:00:00".into(),
        }
    }

    fn synthetic_mail(body: &str) -> email::ParsedEmail {
        email::ParsedEmail {
            uid: 1,
            from: "hr@example.com".into(),
            subject: "测评通知".into(),
            date: "Tue, 13 Oct 2026 10:00:00 +0800".into(),
            body: body.into(),
        }
    }

    #[test]
    fn failed_analysis_is_retried_while_successful_empty_result_is_processed() {
        let config = EmailConfig {
            gemini_api_key: "synthetic".into(),
            ..Default::default()
        };
        let mut mail = synthetic_mail("请留意后续邮件");
        mail.subject = "test notification".into();
        let mut result = empty_result();
        let uids = process_emails(
            &config,
            &[mail.clone()],
            &mut result,
            |_| Err("connection unavailable".into()),
            |_, _| panic!("No extracted items"),
        );
        assert!(uids.is_empty());
        assert!(!result.success);
        let mut retry = empty_result();
        let uids = process_emails(
            &config,
            &[mail],
            &mut retry,
            |_| Ok(vec![]),
            |_, _| panic!("No extracted items"),
        );
        assert_eq!(uids, vec![1]);
        assert!(retry.success);
    }

    #[test]
    fn failed_creation_retries_but_duplicates_are_processed() {
        let mail = synthetic_mail("测评截止时间：2026-10-18 18:00");
        let config = EmailConfig::default();
        let mut result = empty_result();
        let uids = process_emails(
            &config,
            &[mail.clone()],
            &mut result,
            |_| panic!("Rules suffice"),
            |_, _| Err("synthetic write failed".into()),
        );
        assert!(uids.is_empty());
        assert!(!result.success);
        let mut retry = empty_result();
        let uids = process_emails(
            &config,
            &[mail],
            &mut retry,
            |_| panic!("Rules suffice"),
            |_, _| Err("该事项已存在".into()),
        );
        assert_eq!(uids, vec![1]);
        assert!(retry.success);
    }

    #[test]
    fn skipped_ai_after_network_failure_remains_retryable() {
        let config = EmailConfig {
            gemini_api_key: "synthetic".into(),
            ..Default::default()
        };
        let mut mail = synthetic_mail("请留意后续邮件");
        mail.subject = "test notification".into();
        let mut other = mail.clone();
        other.uid = 2;
        let mut calls = 0;
        let mut result = empty_result();
        let uids = process_emails(
            &config,
            &[mail, other],
            &mut result,
            |_| {
                calls += 1;
                Err("connection unavailable".into())
            },
            |_, _| panic!("No extracted items"),
        );
        assert_eq!(calls, 1);
        assert!(uids.is_empty());
    }

    #[test]
    fn background_failure_and_partial_failure_are_reported() {
        let mut result = empty_result();
        result.success = false;
        result.errors.push("synthetic failure".into());
        assert_eq!(result.status_payload()["status"], "error");
        result.items_created = 1;
        assert_eq!(result.status_payload()["status"], "error");
        assert!(result.status_payload()["message"]
            .as_str()
            .unwrap()
            .contains("1"));
    }

    #[test]
    fn stored_false_date_is_corrected_only_for_display() {
        let original = PlannerItem {
            id: "synthetic".into(),
            text: "测评".into(),
            done: false,
            created_date: "2026-01-01".into(),
            carried_over: false,
            sort_order: 0,
            source: "email".into(),
            email_uid: Some(1),
            schedule: Some(crate::task::Schedule {
                date: "2026-01-01".into(),
                event_type: "测评".into(),
                time_kind: "unknown".into(),
                source_date: "Tue, 13 Oct 2026 10:00:00 +0800".into(),
                time_evidence: "测评时间约30分钟，请参照图1-1完成操作".into(),
                ..Default::default()
            }),
        };
        let visible = display_items(&[original.clone()]);
        assert!(visible[0].schedule.as_ref().unwrap().date.is_empty());
        assert!(visible[0].schedule.as_ref().unwrap().needs_confirmation);
        assert_eq!(visible[0].created_date, "2026-10-13");
        assert_eq!(original.created_date, "2026-01-01");
    }

    #[test]
    fn stored_false_date_yields_to_valid_deadline_in_same_evidence() {
        let mut original = build_planner_item(
            &ExtractedItem { r#type: "测评".into(), ..Default::default() },
            &synthetic_mail("测评截止时间：2026-10-18 18:00"), &[],
        ).unwrap();
        let schedule = original.schedule.as_mut().unwrap();
        schedule.date = "2026-01-01".into();
        schedule.time_evidence = "测评截止时间：请参照图1-1完成操作，在10-18前完成".into();
        original.created_date = "2026-01-01".into();
        let visible = display_items(&[original]);
        assert_eq!(visible[0].created_date, "2026-10-18");
        assert_eq!(visible[0].schedule.as_ref().unwrap().date, "2026-10-18");
    }

    #[test]
    fn multiple_items_from_one_email_and_reminders_are_handled() {
        let email = email::ParsedEmail {
            uid: 100,
            from: "hr@example.com".into(),
            subject: "【百度】笔试和面试安排".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body: "笔试时间：2026-10-15 10:00\n面试时间：2026-10-16 14:00".into(),
        };
        let extracted = rule_parser::parse_email_items_by_rules(&email);
        let first = build_planner_item(&extracted[0], &email, &[]).unwrap();
        let second = build_planner_item(&extracted[1], &email, &[first.clone()]).unwrap();
        assert_ne!(first.text, second.text);
        assert!(build_planner_item(&extracted[0], &email, &[first.clone()]).is_err());
        let mut reminder = email.clone();
        reminder.uid = 101;
        assert!(build_planner_item(&extracted[0], &reminder, &[first.clone()]).is_err());
        let mut edited = first;
        edited.text = "我改过的标题".into();
        assert!(build_planner_item(&extracted[0], &reminder, &[edited]).is_err());
    }

    #[test]
    fn test_candidate_email_filtering() {
        assert!(!is_candidate_email(
            "示例公司2027校园招聘",
            "招聘流程：简历投递、测评、统一笔试、面试、录用"
        ));
        assert!(is_candidate_email("【携程集团】在线专业笔试邀请函", ""));
        assert!(is_candidate_email(
            "来自长鑫科技集团股份有限公司的AI面试邀请",
            ""
        ));
        assert!(is_candidate_email("中兴通讯校园招聘测评通知", ""));
        assert!(!is_candidate_email("退信：无法投递", ""));
        assert!(!is_candidate_email(
            "【网易招聘】简历投递成功，请确认职位信息",
            ""
        ));
    }

    #[test]
    fn email_talks_are_not_created_but_other_events_in_same_mail_are_kept() {
        let mail = email::ParsedEmail {
            uid: 100,
            from: "hr@example.com".into(),
            subject: "【百度】宣讲和面试安排".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body: "宣讲时间：2026-10-12 10:00\n面试时间：2026-10-13 14:00".into(),
        };
        let extracted = rule_parser::parse_email_items_by_rules(&mail);
        assert_eq!(extracted.len(), 2);
        assert!(build_planner_item(&extracted[0], &mail, &[]).is_err());
        assert!(build_planner_item(&extracted[1], &mail, &[]).is_ok());
    }

    #[test]
    fn legacy_duplicates_are_blocked_and_collapsed_without_deleting_records() {
        let mail = email::ParsedEmail {
            uid: 100,
            from: "hr@example.com".into(),
            subject: "【百度】笔试邀请".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body: "笔试时间：2026-10-15 10:00".into(),
        };
        let extracted = rule_parser::parse_email_items_by_rules(&mail);
        let new = build_planner_item(&extracted[0], &mail, &[]).unwrap();
        let mut old = new.clone();
        old.id = "legacy".into();
        old.schedule = None;
        old.text = "📧 [笔试] 百度-在线笔试 10:00".into();
        assert!(build_planner_item(&extracted[0], &mail, &[old.clone()]).is_err());
        let records = vec![old.clone(), new.clone()];
        assert_eq!(display_items(&records).len(), 1);
        assert_eq!(records.len(), 2);
        assert_eq!(linked_item_ids(&records, &new.id).len(), 2);
        assert!(display_items(&records)[0]
            .schedule
            .as_ref()
            .unwrap()
            .notes
            .contains(&old.text));
        let mut repeated = new.clone();
        repeated.id = "reminder".into();
        repeated.email_uid = Some(101);
        let duplicates = vec![old.clone(), new.clone(), repeated];
        assert_eq!(display_items(&duplicates).len(), 1);
        assert_eq!(linked_item_ids(&duplicates, &new.id).len(), 3);
        let mut edited = duplicates.clone();
        edited[1].text = "我修改的标题".into();
        assert_eq!(display_items(&edited).len(), 1);
        assert_eq!(display_items(&edited)[0].text, "我修改的标题");
        old.done = true;
        assert_eq!(display_items(&[old, new]).len(), 2);
    }

    #[test]
    fn ambiguous_legacy_pairs_remain_visible() {
        let mail = email::ParsedEmail {
            uid: 110,
            from: "hr@example.com".into(),
            subject: "【百度】笔试和面试安排".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body: "笔试时间：2026-10-15 10:00\n面试时间：2026-10-16 14:00".into(),
        };
        let extracted = rule_parser::parse_email_items_by_rules(&mail);
        let new = build_planner_item(&extracted[0], &mail, &[]).unwrap();
        let mut old = new.clone();
        old.id = "old".into();
        old.schedule = None;
        old.text = "📧 [面试] 百度-面试".into();
        assert_eq!(display_items(&[old.clone(), new.clone()]).len(), 2);
        let mut second = new.clone();
        second.id = "second".into();
        second.schedule.as_mut().unwrap().time = "14:00".into();
        old.text = "📧 [笔试] 百度-笔试".into();
        assert_eq!(display_items(&[old, new, second]).len(), 3);
    }

    #[test]
    fn missing_links_do_not_bridge_two_distinct_events() {
        let mail = email::ParsedEmail {
            uid: 100,
            from: "hr@example.com".into(),
            subject: "【百度】笔试邀请".into(),
            date: "Sat, 10 Oct 2026 10:00:00 +0800".into(),
            body: "笔试时间：2026-10-15 10:00".into(),
        };
        let extracted = rule_parser::parse_email_items_by_rules(&mail);
        let mut a = build_planner_item(&extracted[0], &mail, &[]).unwrap();
        a.schedule.as_mut().unwrap().link = "https://exam.example.com/a".into();
        let mut b = a.clone();
        b.id = "b".into();
        b.email_uid = Some(101);
        b.schedule.as_mut().unwrap().link.clear();
        let mut c = a.clone();
        c.id = "c".into();
        c.email_uid = Some(102);
        c.schedule.as_mut().unwrap().link = "https://exam.example.com/c".into();
        assert_eq!(display_items(&[a.clone(), b.clone(), c.clone()]).len(), 2);
        assert_eq!(display_items(&[b, a, c]).len(), 2);
    }
}
