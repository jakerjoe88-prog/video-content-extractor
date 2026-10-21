// 系统托盘功能

use tauri::{
    tray::{TrayIcon, TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent},
    menu::{Menu, MenuItem, PredefinedMenuItem},
    Manager, AppHandle, Emitter,
};
use std::sync::atomic::{AtomicBool, Ordering};

static TRAY_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// 初始化系统托盘
pub fn init_tray(app: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    if TRAY_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err("托盘已初始化".into());
    }
    
    let app_handle = app.clone();
    
    // 创建托盘菜单
    let menu = create_tray_menu(app)?;
    
    // 创建托盘图标
    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("视频内容提取器")
        .on_tray_icon_event(move |tray, event| {
            handle_tray_event(&app_handle, tray, event);
        })
        .build(app)?;
    
    Ok(tray)
}

/// 创建托盘菜单
fn create_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let task_item = MenuItem::with_id(app, "tasks", "任务队列", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    
    let menu = Menu::with_items(app, &[
        &show_item,
        &task_item,
        &separator,
        &quit_item,
    ])?;
    
    Ok(menu)
}

/// 处理托盘事件
fn handle_tray_event(app: &AppHandle, _tray: &TrayIcon, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => {
            // 左键点击显示主窗口
            show_main_window(app);
        }
        TrayIconEvent::Click { button: MouseButton::Right, .. } => {
            // 右键点击显示菜单（自动处理）
        }
        _ => {}
    }
}

/// 显示主窗口
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
    }
}

/// 隐藏主窗口到托盘
pub fn hide_to_tray(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// 更新托盘提示文本
pub fn update_tray_tooltip(app: &AppHandle, tooltip: &str) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// 发送系统通知
pub fn send_notification(app: &AppHandle, title: &str, body: &str) {
    let _ = app.emit("notification", serde_json::json!({
        "title": title,
        "body": body,
    }));
}

/// 任务进度通知
pub fn notify_task_progress(app: &AppHandle, task_name: &str, progress: f32) {
    let tooltip = format!("正在处理: {} ({:.0}%)", task_name, progress * 100.0);
    update_tray_tooltip(app, &tooltip);
}

/// 任务完成通知
pub fn notify_task_completed(app: &AppHandle, task_name: &str) {
    update_tray_tooltip(app, "视频内容提取器");
    send_notification(app, "任务完成", &format!("{} 已完成", task_name));
}

/// 任务失败通知
pub fn notify_task_failed(app: &AppHandle, task_name: &str, error: &str) {
    update_tray_tooltip(app, "视频内容提取器");
    send_notification(app, "任务失败", &format!("{}: {}", task_name, error));
}
