//! 通知履歴管理モジュール
//!
//! 通知イベントの履歴を管理し、永続化する。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// 通知イベントの種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationEventType {
    Stop,
    PermissionRequest,
    Notification,
}

/// 通知履歴エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistoryEntry {
    pub id: u64,
    pub event_type: NotificationEventType,
    pub session_name: String,
    pub session_id: String,
    pub cwd: Option<String>,
    pub content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub read: bool,
}

/// 通知履歴マネージャー
pub struct NotificationHistoryManager {
    entries: RwLock<Vec<NotificationHistoryEntry>>,
    next_id: RwLock<u64>,
    max_entries: usize,
}

impl Default for NotificationHistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationHistoryManager {
    /// 新しい履歴マネージャーを作成
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            next_id: RwLock::new(1),
            max_entries: 100,
        }
    }

    /// 履歴をロード
    pub fn load(&self, app: &AppHandle) -> Result<(), String> {
        let store = app
            .store("notification_history.json")
            .map_err(|e| format!("Failed to open store: {}", e))?;

        if let Some(entries_value) = store.get("entries") {
            let entries: Vec<NotificationHistoryEntry> =
                serde_json::from_value(entries_value.clone())
                    .map_err(|e| format!("Failed to parse entries: {}", e))?;

            let max_id = entries.iter().map(|e| e.id).max().unwrap_or(0);

            *self.entries.write().unwrap() = entries;
            *self.next_id.write().unwrap() = max_id + 1;
        }

        Ok(())
    }

    /// 履歴を保存
    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let store = app
            .store("notification_history.json")
            .map_err(|e| format!("Failed to open store: {}", e))?;

        let entries = self.entries.read().unwrap();
        let entries_value = serde_json::to_value(&*entries)
            .map_err(|e| format!("Failed to serialize entries: {}", e))?;

        store.set("entries", entries_value);
        store
            .save()
            .map_err(|e| format!("Failed to save store: {}", e))?;

        Ok(())
    }

    /// 新しいエントリを追加
    pub fn add_entry(
        &self,
        app: &AppHandle,
        event_type: NotificationEventType,
        session_name: String,
        session_id: String,
        cwd: Option<String>,
        content: Option<String>,
    ) -> Result<u64, String> {
        let id = {
            let mut next_id = self.next_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let entry = NotificationHistoryEntry {
            id,
            event_type,
            session_name,
            session_id,
            cwd,
            content,
            timestamp: Utc::now(),
            read: false,
        };

        {
            let mut entries = self.entries.write().unwrap();
            // 先頭に追加（新しいものが上）
            entries.insert(0, entry);

            // 最大件数を超えたら古いものを削除
            if entries.len() > self.max_entries {
                entries.truncate(self.max_entries);
            }
        }

        // 永続化
        self.save(app)?;

        Ok(id)
    }

    /// 履歴を取得（フィルター付き）
    pub fn get_entries(&self, filter_session: Option<&str>) -> Vec<NotificationHistoryEntry> {
        let entries = self.entries.read().unwrap();

        match filter_session {
            Some(session) => entries
                .iter()
                .filter(|e| e.session_name == session)
                .cloned()
                .collect(),
            None => entries.clone(),
        }
    }

    /// 特定のエントリを既読にする
    pub fn mark_as_read(&self, app: &AppHandle, id: u64) -> Result<(), String> {
        {
            let mut entries = self.entries.write().unwrap();
            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                entry.read = true;
            }
        }
        self.save(app)
    }

    /// すべてのエントリを既読にする
    pub fn mark_all_as_read(&self, app: &AppHandle) -> Result<(), String> {
        {
            let mut entries = self.entries.write().unwrap();
            for entry in entries.iter_mut() {
                entry.read = true;
            }
        }
        self.save(app)
    }

    /// 履歴をクリア
    pub fn clear(&self, app: &AppHandle) -> Result<(), String> {
        {
            let mut entries = self.entries.write().unwrap();
            entries.clear();
        }
        self.save(app)
    }

    /// 未読件数を取得
    pub fn get_unread_count(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.iter().filter(|e| !e.read).count()
    }
}
