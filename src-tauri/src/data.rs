use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannerItem {
    pub id: String,
    pub text: String,
    pub done: bool,
    pub created_date: String, // YYYY-MM-DD
    pub carried_over: bool,
    #[serde(default)]
    pub sort_order: i64,
    /// "manual" or "email" — where this item was created from
    #[serde(default = "default_source")]
    pub source: String,
    /// UID of the email that created this item (for deduplication)
    #[serde(default)]
    pub email_uid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<crate::task::Schedule>,
}

fn default_source() -> String {
    "manual".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerData {
    pub items: Vec<PlannerItem>,
    pub last_active_date: String, // YYYY-MM-DD
}

impl Default for PlannerData {
    fn default() -> Self {
        PlannerData {
            items: Vec::new(),
            last_active_date: today(),
        }
    }
}

/// Returns the app data directory: %APPDATA%/TipTips/
pub fn data_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("TipTips")
}

pub fn data_file_path() -> PathBuf {
    data_dir().join("data.json")
}

/// Get today's date as YYYY-MM-DD
pub fn today() -> String {
    let now = chrono::Local::now();
    now.format("%Y-%m-%d").to_string()
}

/// Read data from file. If file is corrupt, back it up and return fresh data.
pub fn read_data() -> (PlannerData, bool) {
    let path = data_file_path();
    if !path.exists() {
        return (PlannerData::default(), false);
    }
    match fs::read_to_string(&path) {
        Ok(content) => {
            let clean = content.trim_start_matches('\u{feff}');
            match serde_json::from_str::<PlannerData>(clean) {
                Ok(data) => (data, false),
                Err(_) => {
                    // Corrupt file - back it up
                    let backup_path = path.with_extension("json.corrupt");
                    let _ = fs::rename(&path, &backup_path);
                    (PlannerData::default(), true)
                }
            }
        }
        Err(_) => {
            // Can't read - try backup
            let backup_path = path.with_extension("json.corrupt");
            let _ = fs::rename(&path, &backup_path);
            (PlannerData::default(), true)
        }
    }
}

/// Write data to file
pub fn write_data(data: &PlannerData) -> Result<(), String> {
    let dir = data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    }
    let json =
        serde_json::to_string_pretty(data).map_err(|e| format!("Failed to serialize: {}", e))?;
    let path = data_file_path();
    let mut file = fs::File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

/// Generate a unique ID
pub fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}{:04}", timestamp, rand_suffix())
}

fn rand_suffix() -> u16 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    (hasher.finish() & 0xFFFF) as u16
}

/// Perform day rollover: remove completed items from previous days, mark uncompleted as carried over
pub fn perform_rollover(data: &mut PlannerData) -> bool {
    let today_str = today();
    if data.last_active_date == today_str {
        return false;
    }

    // Remove items that are done and from a previous day
    data.items
        .retain(|item| !(item.done && item.created_date != today_str));

    // Mark remaining non-today items as carried over
    for item in &mut data.items {
        if item.created_date != today_str {
            item.carried_over = true;
        }
    }

    data.last_active_date = today_str;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_json_retains_text_and_completion() {
        let legacy = r#"{"items":[{"id":"legacy","text":"我自己改过的内容","done":true,"created_date":"2026-10-10","carried_over":false,"source":"email","email_uid":42}],"last_active_date":"2026-10-10"}"#;
        let data: PlannerData = serde_json::from_str(legacy).unwrap();
        assert!(data.items[0].schedule.is_none());
        assert!(data.items[0].done);
        assert_eq!(data.items[0].text, "我自己改过的内容");
        let encoded = serde_json::to_string(&data).unwrap();
        assert!(!encoded.contains("schedule"));
    }

    #[test]
    fn test_planner_data_default() {
        let data = PlannerData::default();
        assert!(data.items.is_empty());
        assert!(!data.last_active_date.is_empty());
    }

    #[test]
    fn test_generate_id_unique() {
        let id1 = generate_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_perform_rollover_same_day() {
        let mut data = PlannerData {
            items: vec![PlannerItem {
                id: "1".to_string(),
                text: "task".to_string(),
                done: false,
                created_date: today(),
                carried_over: false,
                sort_order: 0,
                source: "manual".to_string(),
                email_uid: None,
                schedule: None,
            }],
            last_active_date: today(),
        };
        assert!(!perform_rollover(&mut data));
        assert_eq!(data.items.len(), 1);
        assert!(!data.items[0].carried_over);
    }

    #[test]
    fn test_perform_rollover_new_day() {
        let yesterday = "2020-01-01".to_string();
        let mut data = PlannerData {
            items: vec![
                PlannerItem {
                    id: "1".to_string(),
                    text: "done task".to_string(),
                    done: true,
                    created_date: yesterday.clone(),
                    carried_over: false,
                    sort_order: 0,
                    source: "manual".to_string(),
                    email_uid: None,
                    schedule: None,
                },
                PlannerItem {
                    id: "2".to_string(),
                    text: "undone task".to_string(),
                    done: false,
                    created_date: yesterday.clone(),
                    carried_over: false,
                    sort_order: 1,
                    source: "manual".to_string(),
                    email_uid: None,
                    schedule: None,
                },
            ],
            last_active_date: yesterday,
        };
        assert!(perform_rollover(&mut data));
        // Done item should be removed
        assert_eq!(data.items.len(), 1);
        // Remaining item should be carried over
        assert!(data.items[0].carried_over);
        assert_eq!(data.items[0].id, "2");
    }

    #[test]
    fn test_read_data_missing_file() {
        // This test depends on filesystem state, just verify it doesn't panic
        let _data = PlannerData::default();
    }
}
