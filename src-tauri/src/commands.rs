use crate::config::{self, WindowConfig};
use crate::data::{self, PlannerItem};
use crate::email_config;
use crate::email_scanner;
use crate::seu_fairs;

fn view_items(items: &[PlannerItem]) -> Vec<PlannerItem> {
    let mut result: Vec<_> = email_scanner::display_items(items)
        .into_iter()
        .filter(|item| !seu_fairs::is_email_talk(item))
        .collect();
    result.extend(seu_fairs::cached_items());
    result
}

#[derive(serde::Serialize)]
pub struct FairStatus {
    updated_at: String,
    count: usize,
}

#[tauri::command]
pub fn get_seu_fair_status() -> Result<FairStatus, String> {
    let snapshot = seu_fairs::cached_snapshot()?;
    Ok(FairStatus {
        updated_at: snapshot.updated_at,
        count: seu_fairs::cached_items().len(),
    })
}

#[tauri::command]
pub async fn refresh_seu_fairs() -> Result<FairStatus, String> {
    let snapshot = tauri::async_runtime::spawn_blocking(seu_fairs::refresh)
        .await
        .map_err(|e| e.to_string())??;
    Ok(FairStatus {
        updated_at: snapshot.updated_at,
        count: snapshot.items.len(),
    })
}

#[tauri::command]
pub fn get_items() -> Result<Vec<PlannerItem>, String> {
    let (mut data, _was_corrupt) = data::read_data();
    data::perform_rollover(&mut data);
    let _ = data::write_data(&data);
    let today = data::today();
    let official_today = seu_fairs::beijing_today(chrono::Utc::now());
    let today_items: Vec<PlannerItem> = view_items(&data.items)
        .into_iter()
        .filter(|item| seu_fairs::is_today_item(item, &today, &official_today))
        .collect();
    Ok(today_items)
}

#[tauri::command]
pub fn add_item(text: String, date: Option<String>) -> Result<PlannerItem, String> {
    if text.trim().is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    let (mut data, _) = data::read_data();
    data::perform_rollover(&mut data);
    let created_date = date.unwrap_or_else(data::today);
    let sort_order = data.items.len() as i64;
    let item = PlannerItem {
        id: data::generate_id(),
        text: text.trim().to_string(),
        done: false,
        created_date,
        carried_over: false,
        sort_order,
        source: "manual".to_string(),
        email_uid: None,
        schedule: None,
    };
    data.items.push(item.clone());
    data::write_data(&data)?;
    Ok(item)
}

#[tauri::command]
pub fn update_item(id: String, text: String) -> Result<(), String> {
    seu_fairs::ensure_mutable(&id)?;
    let (mut data, _) = data::read_data();
    let linked = email_scanner::linked_item_ids(&data.items, &id);
    let item = data
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or("Item not found")?;
    if text.trim().is_empty() {
        // Empty text means delete
        data.items.retain(|i| !linked.contains(&i.id));
    } else {
        item.text = text.trim().to_string();
    }
    data::write_data(&data)
}

#[tauri::command]
pub fn delete_item(id: String) -> Result<(), String> {
    seu_fairs::ensure_mutable(&id)?;
    let (mut data, _) = data::read_data();
    let linked = email_scanner::linked_item_ids(&data.items, &id);
    data.items.retain(|i| !linked.contains(&i.id));
    data::write_data(&data)
}

#[tauri::command]
pub fn toggle_item(id: String) -> Result<PlannerItem, String> {
    seu_fairs::ensure_mutable(&id)?;
    let (mut data, _) = data::read_data();
    let linked = email_scanner::linked_item_ids(&data.items, &id);
    let item = data
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or("Item not found")?;
    item.done = !item.done;
    let updated = item.clone();
    for item in &mut data.items {
        if linked.contains(&item.id) {
            item.done = updated.done;
        }
    }
    data::write_data(&data)?;
    Ok(updated)
}

#[tauri::command]
pub fn check_day_rollover() -> Result<bool, String> {
    let (mut data, _) = data::read_data();
    let changed = data::perform_rollover(&mut data);
    if changed {
        data::write_data(&data)?;
    }
    Ok(changed)
}

#[tauri::command]
pub fn get_window_config() -> Result<WindowConfig, String> {
    Ok(config::read_config())
}

#[tauri::command]
pub fn save_window_config(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    view_mode: Option<String>,
    opacity: Option<f64>,
) -> Result<(), String> {
    let existing = config::read_config();
    let cfg = WindowConfig {
        x,
        y,
        width,
        height,
        view_mode: view_mode.unwrap_or(existing.view_mode),
        opacity: opacity.unwrap_or(existing.opacity),
    };
    config::write_config(&cfg)
}

#[tauri::command]
pub fn was_data_corrupt() -> Result<bool, String> {
    let (_, was_corrupt) = data::read_data();
    Ok(was_corrupt)
}

#[tauri::command]
pub fn get_items_by_date_range(
    start_date: String,
    end_date: String,
) -> Result<Vec<PlannerItem>, String> {
    let (mut data, _) = data::read_data();
    data::perform_rollover(&mut data);
    let filtered: Vec<PlannerItem> = view_items(&data.items)
        .into_iter()
        .filter(|item| {
            item.created_date >= start_date
                && item.created_date <= end_date
                && !item.schedule.as_ref().is_some_and(|s| s.date.is_empty())
        })
        .collect();
    Ok(filtered)
}

#[tauri::command]
pub fn reorder_items(ordered_ids: Vec<String>) -> Result<(), String> {
    let (mut data, _) = data::read_data();
    for (i, id) in ordered_ids.iter().enumerate() {
        if let Some(item) = data.items.iter_mut().find(|it| &it.id == id) {
            item.sort_order = i as i64;
        }
    }
    data.items.sort_by_key(|it| it.sort_order);
    data::write_data(&data)
}

// ========== Email Scanning Commands ==========

#[tauri::command]
pub fn save_email_config(
    email_address: String,
    auth_code: Option<String>,
    gemini_api_key: Option<String>,
    scan_interval_minutes: Option<u64>,
    enabled: Option<bool>,
    gemini_model: Option<String>,
) -> Result<(), String> {
    let mut config = email_config::read_email_config();

    config.email_address = email_address;

    if let Some(code) = auth_code {
        if !code.is_empty() {
            config.auth_code = email_config::obfuscate(&code);
        }
    }

    if let Some(key) = gemini_api_key {
        if !key.is_empty() {
            config.gemini_api_key = email_config::obfuscate(&key);
        }
    }

    if let Some(interval) = scan_interval_minutes {
        config.scan_interval_minutes = interval;
    }

    if let Some(en) = enabled {
        config.enabled = en;
    }

    if let Some(model) = gemini_model {
        config.gemini_model = model;
    }

    email_config::write_email_config(&config)
}

#[tauri::command]
pub fn get_email_config() -> Result<serde_json::Value, String> {
    let config = email_config::read_email_config();
    Ok(email_config::sanitize_config(&config))
}

#[tauri::command]
pub fn test_email_connection(
    email_address: String,
    auth_code: Option<String>,
) -> Result<String, String> {
    let code_to_use = match auth_code {
        Some(code) if !code.trim().is_empty() => code,
        _ => {
            let config = email_config::read_email_config();
            if config.auth_code.is_empty() {
                return Err("授权码为空，请输入授权码".to_string());
            }
            email_config::deobfuscate(&config.auth_code)
        }
    };

    crate::email::test_connection(&email_address, &code_to_use)
}

#[tauri::command]
pub fn scan_emails_now() -> Result<email_scanner::ScanResult, String> {
    email_scanner::execute_scan()
}

#[tauri::command]
pub fn set_email_scan_enabled(enabled: bool) -> Result<(), String> {
    let mut config = email_config::read_email_config();
    config.enabled = enabled;
    email_config::write_email_config(&config)
}

#[tauri::command]
pub fn reset_email_scan_history() -> Result<(), String> {
    let mut config = email_config::read_email_config();
    config.processed_uids.clear();
    config.last_scan_time.clear();
    email_config::write_email_config(&config)
}

#[tauri::command]
pub fn minimize_window(window: tauri::WebviewWindow) -> Result<(), String> {
    // In Windows, frameless always-on-top windows can remain painted unless topmost is cleared on minimize
    let _ = window.set_always_on_top(false);
    window
        .minimize()
        .map_err(|e| format!("Failed to minimize: {}", e))
}
