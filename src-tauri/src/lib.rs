mod accessibility;
mod icon_cache;
mod path_resolver;
mod title_parser;

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri::PhysicalPosition;

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
    pub tray_position: Option<PhysicalPosition<i32>>,
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
    let dev_apps = accessibility::DEV_APPS;
    let mut all_projects = Vec::new();

    for dev_app in dev_apps {
        let pids = accessibility::find_pids_for_bundle_id(dev_app.bundle_id);

        for pid in pids {
            let titles = accessibility::get_window_titles(pid);
            let icon = icon_cache::get_icon_data_uri(dev_app.bundle_id);

            for (ax_index, title) in titles.iter().enumerate() {
                let project_name =
                    match title_parser::parse_project_name(title, dev_app.title_suffix) {
                        Some(name) => name,
                        None => continue,
                    };

                let full_path =
                    path_resolver::resolve_path(&project_name, dev_app.storage_path);

                // ax_index = position in raw AXWindows array, used by focus_window
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

    // Update state
    if let Some(state) = app.try_state::<Mutex<AppState>>() {
        let mut state = state.lock().unwrap();
        state.projects = all_projects;
    }

    // Notify frontend
    let _ = app.emit("projects-updated", ());
}

static POPOVER_VISIBLE: AtomicBool = AtomicBool::new(false);

// --- Popover management ---

/// Position window centered below the tray icon using our own saved position.
fn position_near_tray(app: &AppHandle, window: &tauri::WebviewWindow) {
    let tray_pos = app
        .try_state::<Mutex<AppState>>()
        .and_then(|s| s.lock().ok()?.tray_position);

    if let Some(pos) = tray_pos {
        let win_width = window
            .outer_size()
            .map(|s| s.width as i32)
            .unwrap_or(336);
        let x = pos.x - win_width / 2;
        let _ = window.set_position(PhysicalPosition::new(x, pos.y));
    } else {
        let _ = window.set_position(PhysicalPosition::new(800, 30));
    }
}

fn toggle_popover(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("popover") {
        if window.is_visible().unwrap_or(false) {
            POPOVER_VISIBLE.store(false, Ordering::Relaxed);
            let _ = window.hide();
        } else {
            refresh_projects(app);
            position_near_tray(app, &window);
            let _ = window.show();
            let _ = window.set_focus();
            POPOVER_VISIBLE.store(true, Ordering::Relaxed);
        }
    } else {
        refresh_projects(app);

        let window = WebviewWindowBuilder::new(app, "popover", WebviewUrl::default())
            .title("Mux")
            .inner_size(336.0, 416.0) // 320+16, 400+16 for 8px body padding
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build();

        if let Ok(window) = window {
            apply_macos_vibrancy(&window);
            position_near_tray(app, &window);
            let _ = window.show();
            let _ = window.set_focus();
            POPOVER_VISIBLE.store(true, Ordering::Relaxed);
        }
    }
}

// --- App setup ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } = event
                {
                    // Save tray icon position for window placement
                    let pos: PhysicalPosition<i32> = rect.position.to_physical(1.0);
                    let size = rect.size.to_physical::<i32>(1.0);
                    if let Some(state) = app_handle.try_state::<Mutex<AppState>>() {
                        if let Ok(mut s) = state.lock() {
                            s.tray_position = Some(PhysicalPosition::new(
                                pos.x as i32 + size.width / 2,
                                pos.y as i32 + size.height,
                            ));
                        }
                    }
                    toggle_popover(app_handle.app_handle());
                }
            });

            // Reconciliation poll — only refreshes when popover is visible
            let poll_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                if POPOVER_VISIBLE.load(Ordering::Relaxed) {
                    refresh_projects(poll_handle.app_handle());
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide popover when it loses focus (but not immediately on creation)
            if window.label() == "popover" {
                match event {
                    tauri::WindowEvent::Focused(true) => {
                        POPOVER_VISIBLE.store(true, Ordering::Relaxed);
                    }
                    tauri::WindowEvent::Focused(false) => {
                        POPOVER_VISIBLE.store(false, Ordering::Relaxed);
                        let w = window.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(150));
                            if !w.is_focused().unwrap_or(true) {
                                let _ = w.hide();
                            } else {
                                POPOVER_VISIBLE.store(true, Ordering::Relaxed);
                            }
                        });
                    }
                    tauri::WindowEvent::Destroyed => {
                        POPOVER_VISIBLE.store(false, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Mux");
}

/// Apply native macOS vibrancy + transparency + rounded corners.
#[cfg(target_os = "macos")]
fn apply_macos_vibrancy(window: &tauri::WebviewWindow) {
    let ns_win = match window.ns_window() {
        Ok(w) => w as *mut std::ffi::c_void,
        Err(_) => return,
    };
    use std::ffi::c_void;

    extern "C" {
        fn objc_getClass(name: *const libc::c_char) -> *mut c_void;
        fn sel_registerName(name: *const libc::c_char) -> *mut c_void;
        fn objc_msgSend(receiver: *mut c_void, sel: *mut c_void, ...) -> *mut c_void;
    }

    unsafe {
        // 1. window.backgroundColor = NSColor.clearColor
        let nscolor_cls = objc_getClass(b"NSColor\0".as_ptr() as *const libc::c_char);
        let clear_sel = sel_registerName(b"clearColor\0".as_ptr() as *const libc::c_char);
        let clear_color = objc_msgSend(nscolor_cls, clear_sel);
        let set_bg_sel = sel_registerName(b"setBackgroundColor:\0".as_ptr() as *const libc::c_char);
        objc_msgSend(ns_win, set_bg_sel, clear_color);

        // 2. window.isOpaque = false (BOOL = i8 on macOS)
        let set_opaque_sel = sel_registerName(b"setOpaque:\0".as_ptr() as *const libc::c_char);
        let send_bool: extern "C" fn(*mut c_void, *mut c_void, i8) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const c_void);
        send_bool(ns_win, set_opaque_sel, 0);

        // 3. Get contentView
        let content_view_sel = sel_registerName(b"contentView\0".as_ptr() as *const libc::c_char);
        let content_view = objc_msgSend(ns_win, content_view_sel);

        // 4. Create NSVisualEffectView with NSZeroRect (autoresizing will fill)
        let ve_cls = objc_getClass(b"NSVisualEffectView\0".as_ptr() as *const libc::c_char);
        let alloc_sel = sel_registerName(b"alloc\0".as_ptr() as *const libc::c_char);
        let ve = objc_msgSend(ve_cls, alloc_sel);

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct NSRect { x: f64, y: f64, w: f64, h: f64 }

        // Inset 8px on all sides to match CSS body padding (window is 336x416, content is 320x400)
        let inset_rect = NSRect { x: 8.0, y: 8.0, w: 320.0, h: 400.0 };
        let init_frame_sel = sel_registerName(b"initWithFrame:\0".as_ptr() as *const libc::c_char);
        let init_ve: extern "C" fn(*mut c_void, *mut c_void, NSRect) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const c_void);
        let ve = init_ve(ve, init_frame_sel, inset_rect);

        // Configure vibrancy
        objc_msgSend(ve, sel_registerName(b"setMaterial:\0".as_ptr() as *const libc::c_char), 3u64); // popover
        objc_msgSend(ve, sel_registerName(b"setBlendingMode:\0".as_ptr() as *const libc::c_char), 0u64); // behindWindow
        objc_msgSend(ve, sel_registerName(b"setState:\0".as_ptr() as *const libc::c_char), 1u64); // active
        // NO autoresizing — fixed frame matching CSS container

        // Corner radius + clip on the vibrancy view
        send_bool(ve, sel_registerName(b"setWantsLayer:\0".as_ptr() as *const libc::c_char), 1);
        let layer = objc_msgSend(ve, sel_registerName(b"layer\0".as_ptr() as *const libc::c_char));
        if !layer.is_null() {
            let set_f64: extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            set_f64(layer, sel_registerName(b"setCornerRadius:\0".as_ptr() as *const libc::c_char), 12.0);
            send_bool(layer, sel_registerName(b"setMasksToBounds:\0".as_ptr() as *const libc::c_char), 1);
        }

        // 10. Clip the contentView itself (this is what actually clips the webview)
        send_bool(content_view, sel_registerName(b"setWantsLayer:\0".as_ptr() as *const libc::c_char), 1);
        let cv_layer = objc_msgSend(content_view, sel_registerName(b"layer\0".as_ptr() as *const libc::c_char));
        if !cv_layer.is_null() {
            let set_f64: extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            set_f64(cv_layer, sel_registerName(b"setCornerRadius:\0".as_ptr() as *const libc::c_char), 12.0);
            send_bool(cv_layer, sel_registerName(b"setMasksToBounds:\0".as_ptr() as *const libc::c_char), 1);
        }
        // Window shadow
        send_bool(ns_win, sel_registerName(b"setHasShadow:\0".as_ptr() as *const libc::c_char), 1);

        // Add visual effect view behind web content
        let add_sub_sel = sel_registerName(b"addSubview:positioned:relativeTo:\0".as_ptr() as *const libc::c_char);
        let add_positioned: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i64, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const c_void);
        add_positioned(content_view, add_sub_sel, ve, -1i64, std::ptr::null_mut());
    }
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
