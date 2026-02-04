//! 通知設定の管理モジュール
//!
//! tauri-plugin-store を使用して設定を永続化する

use serde::{Deserialize, Serialize};
use tauri_plugin_store::StoreExt;
use tracing::{error, info};

/// 通知設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// 通知音を鳴らすか
    pub sound_enabled: bool,
    /// タスクバー点滅を有効にするか
    pub taskbar_flash_enabled: bool,
    /// タスクバーにバッジ（未確認数）を表示するか
    pub taskbar_badge_enabled: bool,
    /// Windows Toast通知を表示するか
    pub toast_notification_enabled: bool,
    /// トレイアイコン点滅を有効にするか
    #[serde(default = "default_true")]
    pub tray_flash_enabled: bool,
    /// 音量（0.0 - 1.0）
    pub sound_volume: f32,
}

fn default_true() -> bool {
    true
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            taskbar_flash_enabled: true,
            taskbar_badge_enabled: true,
            toast_notification_enabled: true,
            tray_flash_enabled: true,
            sound_volume: 0.8,
        }
    }
}

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "notification";

/// 設定を読み込む
pub fn load_settings(app: &tauri::AppHandle) -> NotificationSettings {
    match app.store(SETTINGS_FILE) {
        Ok(store) => {
            match store.get(SETTINGS_KEY) {
                Some(value) => {
                    match serde_json::from_value(value.clone()) {
                        Ok(settings) => {
                            info!("Settings loaded successfully");
                            settings
                        }
                        Err(e) => {
                            error!("Failed to deserialize settings: {}", e);
                            NotificationSettings::default()
                        }
                    }
                }
                None => {
                    info!("No settings found, using defaults");
                    NotificationSettings::default()
                }
            }
        }
        Err(e) => {
            error!("Failed to open settings store: {}", e);
            NotificationSettings::default()
        }
    }
}

/// 設定を保存する
pub fn save_settings(app: &tauri::AppHandle, settings: &NotificationSettings) -> Result<(), String> {
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    store.set(SETTINGS_KEY, value);
    store.save().map_err(|e| e.to_string())?;
    info!("Settings saved successfully");
    Ok(())
}

/// Tauriコマンド: 設定を取得
#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> NotificationSettings {
    load_settings(&app)
}

// save_settings_command は lib.rs に移動
// NotificationManager のメモリ内設定も同時に更新するため

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = NotificationSettings::default();
        assert!(settings.sound_enabled);
        assert!(settings.taskbar_flash_enabled);
        assert!(settings.taskbar_badge_enabled);
        assert!(settings.toast_notification_enabled);
        assert!(settings.tray_flash_enabled);
        assert!((settings.sound_volume - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = NotificationSettings {
            sound_enabled: false,
            taskbar_flash_enabled: true,
            taskbar_badge_enabled: false,
            toast_notification_enabled: true,
            tray_flash_enabled: false,
            sound_volume: 0.5,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: NotificationSettings = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.sound_enabled);
        assert!(deserialized.taskbar_flash_enabled);
        assert!(!deserialized.taskbar_badge_enabled);
        assert!(deserialized.toast_notification_enabled);
        assert!(!deserialized.tray_flash_enabled);
        assert!((deserialized.sound_volume - 0.5).abs() < 0.01);
    }
}
