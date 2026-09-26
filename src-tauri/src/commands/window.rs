use tauri::Window;

#[tauri::command]
pub fn minimize_window(window: Window) { window.minimize().unwrap(); }

#[tauri::command]
pub fn maximize_window(window: Window) { window.maximize().unwrap(); }

#[tauri::command]
pub fn unmaximize_window(window: Window) { window.unmaximize().unwrap(); }

#[tauri::command]
pub fn is_maximized(window: Window) -> bool { window.is_maximized().unwrap() }

#[tauri::command]
pub fn close_window(window: Window) { window.close().unwrap(); }