//! System Tray module for Tauri v2
//!
//! This module provides system tray functionality including
//! icon management, context menu, and event handling.

use crate::NotificationManager;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    webview::WebviewWindowBuilder,
    App, AppHandle, Manager, WebviewUrl,
};
use tracing::{debug, info, warn};

mod menu_ids {
    pub const STATUS: &str = "status";
    pub const SETTINGS: &str = "settings";
    pub const EXPORT: &str = "export";
    pub const QUIT: &str = "quit";
}

pub fn init_tray(app: &mut App) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    info!("Initializing system tray...");

    let status_item =
        MenuItem::with_id(app, menu_ids::STATUS, "Status: Idle", false, None::<&str>)?;

    let settings_item = MenuItem::with_id(
        app,
        menu_ids::SETTINGS,
        "通知設定...",
        true,
        None::<&str>,
    )?;

    let export_item = MenuItem::with_id(
        app,
        menu_ids::EXPORT,
        "設定エクスポート...",
        true,
        None::<&str>,
    )?;

    let quit_item = MenuItem::with_id(app, menu_ids::QUIT, "終了", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&status_item)
        .separator()
        .item(&settings_item)
        .item(&export_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Claude Code Notify")
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_event)
        .build(app)?;

    info!("System tray initialized successfully");
    Ok(tray)
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    debug!("Menu event: {:?}", event.id());

    match event.id().as_ref() {
        menu_ids::SETTINGS => {
            open_settings_window(app);
        }
        menu_ids::EXPORT => {
            open_export_window(app);
        }
        menu_ids::QUIT => {
            info!("Quit requested from tray menu");
            app.exit(0);
        }
        _ => {}
    }
}

/// Open the settings window
fn open_settings_window(app: &AppHandle) {
    // Check if settings window already exists
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    // Create new settings window
    info!("Opening settings window...");
    match WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("通知設定 - Claude Code Notify")
        .inner_size(450.0, 500.0)
        .resizable(false)
        .center()
        .build()
    {
        Ok(window) => {
            let _ = window.show();
            let _ = window.set_focus();
        }
        Err(e) => {
            warn!("Failed to create settings window: {}", e);
        }
    }
}

/// Open the export configuration window
fn open_export_window(app: &AppHandle) {
    // Check if export window already exists
    if let Some(window) = app.get_webview_window("export") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    // Create new export window
    info!("Opening export configuration window...");
    match WebviewWindowBuilder::new(app, "export", WebviewUrl::App("export.html".into()))
        .title("設定エクスポート - Claude Code Notify")
        .inner_size(500.0, 550.0)
        .resizable(false)
        .center()
        .build()
    {
        Ok(window) => {
            let _ = window.show();
            let _ = window.set_focus();
        }
        Err(e) => {
            warn!("Failed to create export window: {}", e);
        }
    }
}

fn handle_tray_event(tray: &TrayIcon, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => {
            debug!("Tray icon left clicked");
            let app = tray.app_handle();

            // Reset notification state when tray is clicked
            if let Some(notification_manager) = app.try_state::<Arc<NotificationManager>>() {
                notification_manager.reset(app);
                info!("Notification state reset on tray click");
            }

            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => {
            debug!("Tray icon double clicked");
            let app = tray.app_handle();

            // Reset notification state when tray is double-clicked
            if let Some(notification_manager) = app.try_state::<Arc<NotificationManager>>() {
                notification_manager.reset(app);
                info!("Notification state reset on tray double-click");
            }

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
pub fn update_tooltip(tray: &TrayIcon, tooltip: &str) -> Result<(), tauri::Error> {
    tray.set_tooltip(Some(tooltip))
}

#[allow(dead_code)]
pub fn update_status(_app: &AppHandle, status: &str) {
    info!("Status updated: {}", status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_ids() {
        assert_eq!(menu_ids::EXPORT, "export");
        assert_eq!(menu_ids::QUIT, "quit");
    }
}
