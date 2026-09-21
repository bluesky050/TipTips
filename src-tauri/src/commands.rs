use crate::config::{self, WindowConfig};
use crate::data::{self, PlannerItem};

#[tauri::command]
pub fn get_items() -> Result<Vec<PlannerItem>, String> {
    let (mut data, _was_corrupt) = data::read_data();
    data::perform_rollover(&mut data);
    let _ = data::write_data(&data);
    let today = data::today();
    let today_items: Vec<PlannerItem> = data
        .items
        .into_iter()
        .filter(|item| item.created_date == today)
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
    };
    data.items.push(item.clone());
    data::write_data(&data)?;
    Ok(item)
}

#[tauri::command]
pub fn update_item(id: String, text: String) -> Result<(), String> {
    let (mut data, _) = data::read_data();
    let item = data
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or("Item not found")?;
    if text.trim().is_empty() {
        // Empty text means delete
        data.items.retain(|i| i.id != id);
    } else {
        item.text = text.trim().to_string();
    }
    data::write_data(&data)
}

#[tauri::command]
pub fn delete_item(id: String) -> Result<(), String> {
    let (mut data, _) = data::read_data();
    data.items.retain(|i| i.id != id);
    data::write_data(&data)
}

#[tauri::command]
pub fn toggle_item(id: String) -> Result<PlannerItem, String> {
    let (mut data, _) = data::read_data();
    let item = data
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or("Item not found")?;
    item.done = !item.done;
    let updated = item.clone();
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
    let filtered: Vec<PlannerItem> = data
        .items
        .into_iter()
        .filter(|item| item.created_date >= start_date && item.created_date <= end_date)
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
