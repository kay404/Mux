mod accessibility;
mod icon_cache;
mod path_resolver;
mod title_parser;

use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_positioner::{Position, WindowExt};

// --- Shared state ---

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub app_name: String,
    pub bundle_id: String,
    pub project_name: String,
    pub full_path: Option<String>,
    pub window_index: usize,
    pub pid: i32,
    pub icon_data_uri: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub projects: Vec<ProjectInfo>,
}

// --- Tauri commands ---

#[tauri::command]
fn get_projects(state: tauri::State<'_, Mutex<AppState>>) -> Vec<ProjectInfo> {
    let state = state.lock().unwrap();
    state.projects.clone()
}

#[tauri::command]
fn focus_window(pid: i32, window_index: usize) -> Result<(), String> {
    accessibility::focus_window(pid, window_index).map_err(|e| e.to_string())
}

#[tauri::command]
fn check_accessibility_permission() -> bool {
    accessibility::is_trusted()
}

#[tauri::command]
fn request_accessibility_permission() -> bool {
    accessibility::request_permission()
}

// --- Refresh logic ---

fn refresh_projects(app: &AppHandle) {
    let trusted = accessibility::is_trusted();
    eprintln!("[Mux] refresh_projects: AX trusted = {}", trusted);

    let dev_apps = accessibility::DEV_APPS;
    let mut all_projects = Vec::new();

    for dev_app in dev_apps {
        let pids = accessibility::find_pids_for_bundle_id(dev_app.bundle_id);
        eprintln!(
            "[Mux] {} ({}): found {} PIDs: {:?}",
            dev_app.name,
            dev_app.bundle_id,
            pids.len(),
            pids
        );

        for pid in pids {
            let titles = accessibility::get_window_titles(pid);
            eprintln!(
                "[Mux]   PID {}: {} window titles: {:?}",
                pid,
                titles.len(),
                titles
            );
            let icon = icon_cache::get_icon_data_uri(dev_app.bundle_id);

            for (ax_index, title) in titles.iter().enumerate() {
                let project_name =
                    match title_parser::parse_project_name(title, dev_app.title_suffix) {
                        Some(name) => name,
                        None => {
                            eprintln!("[Mux]   Could not parse project from: \"{}\"", title);
                            continue;
                        }
                    };

                let full_path =
                    path_resolver::resolve_path(&project_name, dev_app.storage_path);

                eprintln!(
                    "[Mux]   -> {} (ax_index={}) | path: {:?}",
                    project_name, ax_index, full_path
                );

                // Use ax_index (position in the raw AXWindows array), NOT a
                // filtered counter, so focus_window targets the correct window.
                all_projects.push(ProjectInfo {
                    app_name: dev_app.name.to_string(),
                    bundle_id: dev_app.bundle_id.to_string(),
                    project_name,
                    full_path,
                    window_index: ax_index,
                    pid,
                    icon_data_uri: icon.clone(),
                });
            }
        }
    }

    eprintln!("[Mux] Total projects detected: {}", all_projects.len());

    // Update state
    if let Some(state) = app.try_state::<Mutex<AppState>>() {
        let mut state = state.lock().unwrap();
        state.projects = all_projects;
    }

    // Notify frontend
    let _ = app.emit("projects-updated", ());
}

// --- Popover management ---

fn toggle_popover(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("popover") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            // Refresh projects before showing
            refresh_projects(app);
            let _ = window.move_window(Position::TrayBottomCenter);
            let _ = window.show();
            let _ = window.set_focus();
        }
    } else {
        // Create the popover window
        refresh_projects(app);
        let window = WebviewWindowBuilder::new(app, "popover", WebviewUrl::default())
            .title("Mux")
            .inner_size(320.0, 400.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build();

        if let Ok(window) = window {
            let _ = window.move_window(Position::TrayBottomCenter);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

// --- App setup ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(Mutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            get_projects,
            focus_window,
            check_accessibility_permission,
            request_accessibility_permission,
        ])
        .setup(|app| {
            // Set activation policy to Accessory (hide from dock)
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            // Create tray icon with "{ }" text
            // Use a simple 22x22 PNG for the tray icon
            let tray_icon = create_tray_icon(app.handle())?;

            // Listen for tray events
            let app_handle = app.handle().clone();
            tray_icon.on_tray_icon_event(move |_tray, event| {
                tauri_plugin_positioner::on_tray_event(
                    app_handle.app_handle(),
                    &event,
                );
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    toggle_popover(app_handle.app_handle());
                }
            });

            // Start reconciliation poll (5 second interval)
            let poll_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                refresh_projects(poll_handle.app_handle());
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide popover when it loses focus (but not immediately on creation)
            if window.label() == "popover" {
                match event {
                    tauri::WindowEvent::Focused(false) => {
                        // Small delay to avoid hiding on initial focus dance
                        let w = window.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            if !w.is_focused().unwrap_or(true) {
                                let _ = w.hide();
                            }
                        });
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Mux");
}

fn create_tray_icon(
    app: &AppHandle,
) -> Result<tauri::tray::TrayIcon, Box<dyn std::error::Error>> {
    let icon = Image::from_bytes(include_bytes!("../icons/barIcon@2x.png"))?;

    let quit = MenuItem::with_id(app, "quit", "Quit Mux", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Mux")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .build(app)?;

    Ok(tray)
}
