// 系统托盘相关命令

use tauri::AppHandle;
use crate::core::tray;

/// 显示主窗口
#[tauri::command]
pub fn show_window(app: AppHandle) {
    tray::show_main_window(&app);
}

/// 隐藏窗口到托盘
#[tauri::command]
pub fn hide_to_tray(app: AppHandle) {
    tray::hide_to_tray(&app);
}

/// 发送系统通知
#[tauri::command]
pub fn send_notification(app: AppHandle, title: String, body: String) {
    tray::send_notification(&app, &title, &body);
}

/// 更新托盘提示
#[tauri::command]
pub fn update_tray_tooltip(app: AppHandle, tooltip: String) {
    tray::update_tray_tooltip(&app, &tooltip);
}
