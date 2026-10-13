//! Official public schedules are cached separately from personal tasks.
use crate::{data::PlannerItem, task::Schedule};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

const ORIGIN: &str = "https://career.seu.edu.cn";
static REFRESH_LOCK: Mutex<()> = Mutex::new(());
static CACHE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub updated_at: String,
    pub items: Vec<PlannerItem>,
}

pub fn beijing_today(utc: chrono::DateTime<chrono::Utc>) -> String {
    utc.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d")
        .to_string()
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn datetime(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .or_else(|| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").ok())
}

fn convert_fair(value: &Value, updated_at: &str) -> Result<PlannerItem, String> {
    let id = string(value, "id");
    let title = string(value, "title");
    let start = datetime(&string(value, "startTime")).ok_or("官网活动开始时间格式无效")?;
    if id.is_empty() || title.is_empty() {
        return Err("官网活动缺少ID或标题".into());
    }
    let end = datetime(&string(value, "endTime")).filter(|end| *end >= start);
    let campus = string(value, "campusLabel");
    let campus = if campus.is_empty() {
        string(value, "campus")
    } else {
        campus
    };
    let place = value
        .pointer("/field/name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let place = if place.is_empty() {
        string(value, "place")
    } else {
        place.to_string()
    };
    let location = if place.contains(&campus) {
        place
    } else {
        format!("{campus} {place}").trim().to_string()
    };
    let mut fallback = reqwest::Url::parse(&format!("{ORIGIN}/f/recruitmentFair/show")).unwrap();
    fallback
        .query_pairs_mut()
        .append_pair("recruitmentFairId", &id);
    let link = reqwest::Url::parse(ORIGIN)
        .unwrap()
        .join(&string(value, "url"))
        .ok()
        .filter(|url| {
            url.scheme() == "https"
                && url.host_str() == Some("career.seu.edu.cn")
                && !url.path().is_empty()
                && url.path() != "/"
        })
        .unwrap_or(fallback)
        .to_string();
    let company = value
        .pointer("/corporationinfo/name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let date = start.format("%Y-%m-%d").to_string();
    Ok(PlannerItem {
        id: format!("seu:{id}"),
        text: format!("[宣讲会] {title}"),
        done: false,
        created_date: date.clone(),
        carried_over: false,
        sort_order: 0,
        source: "seu".into(),
        email_uid: None,
        schedule: Some(Schedule {
            event_type: "宣讲会".into(),
            company_raw: company.clone(),
            company,
            date,
            time: start.format("%H:%M").to_string(),
            end_date: end
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            end_time: end
                .map(|d| d.format("%H:%M").to_string())
                .unwrap_or_default(),
            time_kind: if end.is_some() { "window" } else { "start" }.into(),
            timezone: "UTC+08:00".into(),
            location,
            link,
            source_subject: title,
            source_date: updated_at.into(),
            time_evidence: format!(
                "{} — {}",
                string(value, "startTime"),
                string(value, "endTime")
            ),
            ..Default::default()
        }),
    })
}

pub fn collect_snapshot<F>(mut fetch: F, today: &str) -> Result<Snapshot, String>
where
    F: FnMut(usize) -> Result<Value, String>,
{
    let updated_at = chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let mut result = Snapshot {
        updated_at,
        items: vec![],
    };
    let mut ids = HashSet::new();
    let mut pages = 1;
    let mut number = 1;
    while number <= pages {
        let response = fetch(number)?;
        if response.get("state").and_then(Value::as_i64) != Some(1) {
            return Err(format!("官网第{number}页查询失败"));
        }
        let object = response.get("object").ok_or("官网响应缺少数据")?;
        let returned = object
            .get("pageNo")
            .and_then(Value::as_u64)
            .ok_or("官网响应缺少页码")? as usize;
        let total = object
            .get("totalPage")
            .and_then(Value::as_u64)
            .ok_or("官网响应缺少分页总数")? as usize;
        if returned != number || total > 100 || (number > 1 && total != pages) {
            return Err("官网分页发生变化，请重新刷新".into());
        }
        pages = total.max(1);
        let list = object
            .get("list")
            .and_then(Value::as_array)
            .ok_or("官网响应缺少活动列表")?;
        for fair in list {
            let mut item = convert_fair(fair, &result.updated_at)?;
            if item.created_date.as_str() >= today && ids.insert(item.id.clone()) {
                item.sort_order = result.items.len() as i64;
                result.items.push(item);
            }
        }
        number += 1;
    }
    result.items.sort_by_key(|item| {
        (
            item.created_date.clone(),
            item.schedule.as_ref().unwrap().time.clone(),
            item.id.clone(),
        )
    });
    Ok(result)
}

pub fn is_email_talk(item: &PlannerItem) -> bool {
    item.source == "email"
        && (item
            .schedule
            .as_ref()
            .is_some_and(|s| s.event_type == "宣讲会")
            || (item.schedule.is_none()
                && (item.text.starts_with("📧 [宣讲会]") || item.text.starts_with("[宣讲会]"))))
}

pub fn is_today_item(item: &PlannerItem, personal_day: &str, official_day: &str) -> bool {
    if item.source == "seu" {
        return item.created_date == official_day;
    }
    if item.source == "email" && item.schedule.as_ref().is_some_and(|s| {
        crate::task::valid_date(&s.date) && s.date.as_str() > personal_day
            && (s.time_kind != "deadline" || item.done)
    }) {
        return false;
    }
    item.created_date == personal_day
        || item.carried_over
        || item.schedule.as_ref().is_some_and(|s| s.date.is_empty())
        || (item.source == "email"
            && !item.done
            && item.schedule.as_ref().is_some_and(|s| {
                (s.time_kind == "deadline"
                    && crate::task::valid_date(&s.date)
                    && s.date.as_str() >= personal_day)
                    || (s.time_kind == "window"
                        && crate::task::valid_date(&s.date)
                        && crate::task::valid_date(&s.end_date)
                        && s.date.as_str() <= personal_day
                        && personal_day <= s.end_date.as_str())
            }))
}

pub fn ensure_mutable(id: &str) -> Result<(), String> {
    if id.starts_with("seu:") {
        Err("官网日程为只读，请在官网查看最新信息".into())
    } else {
        Ok(())
    }
}

fn cache_path() -> std::path::PathBuf {
    crate::data::data_dir().join("seu_fairs.json")
}

pub fn read_snapshot(path: &Path) -> Result<Snapshot, String> {
    if !path.exists() {
        let backup = path.with_extension("json.bak");
        if backup.exists() { return read_snapshot_file(&backup); }
        return Ok(Snapshot::default());
    }
    read_snapshot_file(path).or_else(|error| {
        let backup = path.with_extension("json.bak");
        if backup.exists() { read_snapshot_file(&backup) } else { Err(error) }
    })
}

fn read_snapshot_file(path: &Path) -> Result<Snapshot, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("官网缓存读取失败：{e}"))?)
        .map_err(|e| format!("官网缓存格式异常：{e}"))
}

fn save_snapshot(path: &Path, snapshot: &Snapshot) -> Result<(), String> {
    let _guard = CACHE_LOCK.lock().map_err(|_| "官网缓存锁异常")?;
    let bytes = serde_json::to_vec_pretty(snapshot).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let staging = path.with_extension("json.tmp");
    fs::write(&staging, bytes).map_err(|e| format!("官网缓存写入失败：{e}"))?;
    replace_cache(&staging, path).map_err(|e| format!("官网缓存更新失败：{e}"))
}

#[cfg(not(windows))]
fn replace_cache(staging: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(staging, path)
}

#[cfg(windows)]
fn replace_cache(staging: &Path, path: &Path) -> std::io::Result<()> {
    match fs::rename(staging, path) {
        Ok(()) => Ok(()),
        // Some managed Windows file systems reject rename with ERROR_INVALID_HANDLE.
        // Preserve the previous complete snapshot before the compatible copy commit.
        Err(error) if error.raw_os_error() == Some(6) => copy_cache(staging, path),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn copy_cache(staging: &Path, path: &Path) -> std::io::Result<()> {
    let backup = path.with_extension("json.bak");
    let previous = read_snapshot_file(path).ok();
    if let Some(previous) = previous {
        fs::write(&backup, serde_json::to_vec_pretty(&previous).map_err(std::io::Error::other)?)?;
    }
    let had_previous = read_snapshot_file(&backup).is_ok();
    if let Err(error) = fs::copy(staging, path) {
        if had_previous { fs::copy(&backup, path)?; }
        return Err(error);
    }
    Ok(())
}

pub fn cached_snapshot() -> Result<Snapshot, String> {
    let _guard = CACHE_LOCK.lock().map_err(|_| "官网缓存锁异常")?;
    read_snapshot(&cache_path())
}

pub fn cached_items() -> Vec<PlannerItem> {
    let today = beijing_today(chrono::Utc::now());
    snapshot_items(cached_snapshot().unwrap_or_default(), &today)
}

fn snapshot_items(snapshot: Snapshot, today: &str) -> Vec<PlannerItem> {
    snapshot
        .items
        .into_iter()
        .filter(|item| item.created_date.as_str() >= today)
        .collect()
}

fn refresh_with<F, S>(fetch: F, today: &str, save: S) -> Result<Snapshot, String>
where
    F: FnMut(usize) -> Result<Value, String>,
    S: FnOnce(&Snapshot) -> Result<(), String>,
{
    let snapshot = collect_snapshot(fetch, today)?;
    save(&snapshot)?;
    Ok(snapshot)
}

pub fn refresh() -> Result<Snapshot, String> {
    if std::env::var("TIPTIPS_OFFLINE").as_deref() == Ok("1") {
        return Err("离线启动检查：未请求官网".into());
    }
    let _guard = REFRESH_LOCK.lock().map_err(|_| "官网同步锁异常")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("TipTips/0.1 official-calendar")
        .build()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    refresh_with(
        |page| {
            if started.elapsed() > Duration::from_secs(60) {
                return Err("官网同步超时，请稍后刷新".into());
            }
            client
                .post(format!("{ORIGIN}/f/recruitmentFair/ajax_frontRecruitfair"))
                .form(&[
                    ("pageNo", page.to_string()),
                    ("pageSize", "100".to_string()),
                ])
                .send()
                .and_then(|res| res.error_for_status())
                .and_then(|res| res.json::<Value>())
                .map_err(|e| format!("官网连接失败：{e}"))
        },
        &beijing_today(chrono::Utc::now()),
        |snapshot| save_snapshot(&cache_path(), snapshot),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[ignore = "manual public website integration check; writes official cache only"]
    fn live_public_sync() {
        let result = refresh();
        assert!(result.is_ok(), "{result:?}");
        let snapshot = result.unwrap();
        println!("official events: {}", snapshot.items.len());
    }

    #[test]
    fn cache_commit_supports_first_write_and_replacement() {
        let directory = std::env::temp_dir().join(format!("TipTips-fair-cache-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap()));
        let path = directory.join("seu_fairs.json");
        let first = Snapshot { updated_at: "first".into(), items: vec![] };
        save_snapshot(&path, &first).unwrap();
        assert_eq!(read_snapshot(&path).unwrap(), first);
        let second = Snapshot { updated_at: "second".into(), items: vec![] };
        save_snapshot(&path, &second).unwrap();
        assert_eq!(read_snapshot(&path).unwrap(), second);
    }

    #[test]
    #[cfg(windows)]
    fn compatible_cache_commit_preserves_recovery_snapshot() {
        let directory = std::env::temp_dir().join(format!("TipTips-fair-recovery-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap()));
        let path = directory.join("seu_fairs.json");
        let first = Snapshot { updated_at: "first".into(), items: vec![] };
        save_snapshot(&path, &first).unwrap();
        let staging = path.with_extension("json.tmp");
        let second = Snapshot { updated_at: "second".into(), items: vec![] };
        fs::write(&staging, serde_json::to_vec(&second).unwrap()).unwrap();
        copy_cache(&staging, &path).unwrap();
        assert_eq!(read_snapshot(&path).unwrap(), second);
        assert_eq!(read_snapshot_file(&path.with_extension("json.bak")).unwrap(), first);
        // A interrupted compatible commit still has the previous complete snapshot.
        fs::write(&path, b"{interrupted").unwrap();
        assert_eq!(read_snapshot(&path).unwrap(), first);
        // A second failed commit must not replace a valid backup with the corrupt main file.
        assert!(copy_cache(&directory.join("missing.tmp"), &path).is_err());
        assert_eq!(read_snapshot(&path).unwrap(), first);
        assert_eq!(read_snapshot_file(&path.with_extension("json.bak")).unwrap(), first);
    }

    fn fair(id: &str, start: &str) -> Value {
        json!({"id":id,"title":"示例企业校园宣讲会","startTime":start,"endTime":"2026-10-12 16:00:00","campus":"九龙湖校区","field":{"name":"教四-101"},"corporationinfo":{"name":"示例企业"},"url":format!("/f/recruitmentFair/show?recruitmentFairId={id}")})
    }
    fn page(list: Vec<Value>, number: usize, total: usize) -> Value {
        json!({"state":1,"object":{"pageNo":number,"totalPage":total,"list":list}})
    }
    #[test]
    fn beijing_midnight_includes_today() {
        let utc = chrono::DateTime::parse_from_rfc3339("2026-10-10T16:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(beijing_today(utc), "2026-10-11");
    }
    #[test]
    fn all_pages_include_today_future_and_distinct_ids() {
        let mut calls = vec![];
        let snapshot = collect_snapshot(
            |n| {
                calls.push(n);
                Ok(if n == 1 {
                    page(
                        vec![
                            fair("pinned-old", "2026-09-21 14:00:00"),
                            fair("today", "2026-10-11 09:00:00"),
                        ],
                        1,
                        2,
                    )
                } else {
                    page(
                        vec![
                            fair("today", "2026-10-11 09:00:00"),
                            fair("future", "2026-11-01 14:00:00"),
                        ],
                        2,
                        2,
                    )
                })
            },
            "2026-10-11",
        )
        .unwrap();
        assert_eq!(calls, vec![1, 2]);
        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.items[0].source, "seu");
        assert_eq!(snapshot.items[0].created_date, "2026-10-11");
        assert_eq!(
            snapshot.items[0].schedule.as_ref().unwrap().location,
            "九龙湖校区 教四-101"
        );
        assert!(snapshot.items[0]
            .schedule
            .as_ref()
            .unwrap()
            .link
            .starts_with("https://career.seu.edu.cn/"));
        assert!(!snapshot.items[0].carried_over);
    }
    #[test]
    fn partial_or_invalid_responses_cannot_be_successful_empty_snapshots() {
        assert!(collect_snapshot(
            |n| if n == 1 {
                Ok(page(vec![fair("one", "2026-10-11 14:00:00")], 1, 2))
            } else {
                Err("offline".into())
            },
            "2026-10-11"
        )
        .is_err());
        assert!(collect_snapshot(|_| Ok(json!({"state":0})), "2026-10-11").is_err());
        assert!(collect_snapshot(
            |_| Ok(page(vec![fair("bad", "2026-02-30 14:00:00")], 1, 1)),
            "2026-10-11"
        )
        .is_err());
    }
    #[test]
    fn email_talk_filter_keeps_manual_and_other_email_events() {
        let mut item = PlannerItem {
            id: "old".into(),
            text: "📧 [宣讲会] 示例企业".into(),
            done: false,
            created_date: "2026-10-11".into(),
            carried_over: false,
            sort_order: 0,
            source: "email".into(),
            email_uid: Some(1),
            schedule: None,
        };
        assert!(is_email_talk(&item));
        item.text = "我改过的标题".into();
        item.schedule = Some(Schedule {
            event_type: "宣讲会".into(),
            ..Default::default()
        });
        assert!(is_email_talk(&item));
        item.source = "manual".into();
        assert!(!is_email_talk(&item));
        item.source = "email".into();
        item.schedule.as_mut().unwrap().event_type = "面试".into();
        assert!(!is_email_talk(&item));
    }

    #[test]
    fn failed_refresh_preserves_cache_and_success_replaces_snapshot() {
        let mut cached = collect_snapshot(
            |_| Ok(page(vec![fair("existing", "2026-10-12 14:00:00")], 1, 1)),
            "2026-10-11",
        )
        .unwrap();
        let previous = cached.clone();
        let failed = refresh_with(
            |n| {
                if n == 1 {
                    Ok(page(vec![fair("new", "2026-10-13 14:00:00")], 1, 2))
                } else {
                    Err("offline".into())
                }
            },
            "2026-10-11",
            |s| {
                cached = s.clone();
                Ok(())
            },
        );
        assert!(failed.is_err());
        assert_eq!(cached, previous);
        refresh_with(
            |_| Ok(page(vec![], 1, 1)),
            "2026-10-11",
            |s| {
                cached = s.clone();
                Ok(())
            },
        )
        .unwrap();
        assert!(cached.items.is_empty());
        assert!(ensure_mutable("seu:one").is_err());
        assert!(ensure_mutable("personal").is_ok());
        assert!(snapshot_items(previous, "2026-10-13").is_empty());
    }

    #[test]
    fn pending_email_deadlines_are_visible_before_due_day() {
        let mut item = PlannerItem {
            id: "synthetic".into(),
            text: "测评".into(),
            done: false,
            created_date: "2026-10-18".into(),
            carried_over: false,
            sort_order: 0,
            source: "email".into(),
            email_uid: Some(1),
            schedule: Some(Schedule {
                date: "2026-10-18".into(),
                time_kind: "deadline".into(),
                ..Default::default()
            }),
        };
        assert!(is_today_item(&item, "2026-10-13", "2026-10-13"));
        item.done = true;
        assert!(!is_today_item(&item, "2026-10-13", "2026-10-13"));
        item.done = false;
        item.schedule.as_mut().unwrap().time_kind = "start".into();
        assert!(!is_today_item(&item, "2026-10-13", "2026-10-13"));
        item.carried_over = true;
        assert!(!is_today_item(&item, "2026-10-13", "2026-10-13"));
        item.carried_over = false;
        item.schedule.as_mut().unwrap().time_kind = "window".into();
        item.schedule.as_mut().unwrap().date = "2026-10-14".into();
        item.schedule.as_mut().unwrap().end_date = "2026-10-18".into();
        assert!(!is_today_item(&item, "2026-10-13", "2026-10-13"));
        assert!(is_today_item(&item, "2026-10-15", "2026-10-15"));
    }

    #[test]
    fn official_today_is_independent_of_personal_local_day() {
        let snapshot = collect_snapshot(
            |_| Ok(page(vec![fair("today", "2026-10-11 14:00:00")], 1, 1)),
            "2026-10-11",
        )
        .unwrap();
        let mut item = snapshot.items[0].clone();
        assert!(is_today_item(&item, "2026-10-10", "2026-10-11"));
        assert!(!is_today_item(&item, "2026-10-11", "2026-10-12"));
        item.source = "manual".into();
        assert!(is_today_item(&item, "2026-10-11", "2026-10-12"));
    }
}
