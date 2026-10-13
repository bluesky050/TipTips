mod ai;
mod commands;
mod config;
mod data;
mod email;
mod email_config;
mod email_scanner;
mod rule_parser;
mod task;
mod seu_fairs;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.set_always_on_top(true);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_items,
            commands::add_item,
            commands::update_item,
            commands::delete_item,
            commands::toggle_item,
            commands::check_day_rollover,
            commands::get_window_config,
            commands::save_window_config,
            commands::was_data_corrupt,
            commands::get_items_by_date_range,
            commands::reorder_items,
            commands::get_seu_fair_status,
            commands::refresh_seu_fairs,
            // Email scanning commands
            commands::save_email_config,
            commands::get_email_config,
            commands::test_email_connection,
            commands::scan_emails_now,
            commands::set_email_scan_enabled,
            commands::reset_email_scan_history,
            commands::minimize_window,
        ])
        .setup(|app| {
            // Restore window position from config, with monitor bounds validation
            let cfg = config::read_config();
            if let Some(window) = app.get_webview_window("main") {
                let monitors = window.available_monitors().unwrap_or_default();
                let is_on_any_monitor = monitors.iter().any(|m| {
                    let pos = m.position();
                    let size = m.size();
                    let x = cfg.x as i32;
                    let y = cfg.y as i32;
                    x >= pos.x
                        && x < (pos.x + size.width as i32)
                        && y >= pos.y
                        && y < (pos.y + size.height as i32)
                });

                let (final_x, final_y, final_w, final_h) = if is_on_any_monitor
                    && cfg.width >= 280.0
                    && cfg.height >= 300.0
                {
                    (cfg.x as i32, cfg.y as i32, cfg.width as u32, cfg.height as u32)
                } else {
                    (100, 100, 360, 500)
                };

                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition {
                        x: final_x,
                        y: final_y,
                    },
                ));
                let _ = window.set_size(tauri::Size::Physical(
                    tauri::PhysicalSize {
                        width: final_w,
                        height: final_h,
                    },
                ));
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.set_always_on_top(true);
            }

            // Build tray menu
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Create tray icon
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("TipTips");

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _tray = tray_builder
                .on_tray_icon_event(|tray, event| {
                    match event {
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        }
                        | tauri::tray::TrayIconEvent::DoubleClick {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.set_always_on_top(true);
                            }
                        }
                        _ => {}
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.set_always_on_top(true);
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Start background email scanning thread
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Wait 30 seconds after startup before first check
                std::thread::sleep(std::time::Duration::from_secs(30));

                loop {
                    let email_cfg = email_config::read_email_config();

                    if email_scanner::should_scan_now(&email_cfg) {
                        match email_scanner::execute_scan() {
                            Ok(result) => {
                                if result.items_created > 0 {
                                    // Notify frontend to refresh data
                                    let _ = app_handle.emit("email-scan-complete", &result);
                                }
                                let _ = app_handle.emit("email-scan-status", result.status_payload());
                            }
                            Err(e) => {
                                let _ = app_handle.emit("email-scan-status", serde_json::json!({
                                    "status": "error",
                                    "message": format!("扫描失败: {}", e),
                                }));
                            }
                        }
                    }

                    // Check every 5 minutes if it's time to scan
                    std::thread::sleep(std::time::Duration::from_secs(300));
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Restore always on top when window gains focus or is restored
            if let WindowEvent::Focused(true) = event {
                let _ = window.set_always_on_top(true);
            }
            // Prevent window close - hide instead
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
