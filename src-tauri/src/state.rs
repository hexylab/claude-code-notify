//! Session State Management module
//!
//! Manages the state of Claude Code sessions, including
//! tracking active sessions, their status, and aggregated metrics.
//! Also handles session ID to display name mapping.

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Default timeout for session cleanup (5 minutes)
const SESSION_TIMEOUT_SECS: u64 = 300;

/// Status payload from Claude Code statusline
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusPayload {
    pub session_id: String,
    pub cwd: String,
    pub status: SessionStatus,
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Session status details from statusline
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SessionStatus {
    /// Current state (e.g., "idle", "working", "waiting")
    #[serde(default)]
    pub state: Option<String>,
    /// Context usage percentage (0-100)
    #[serde(default)]
    pub context_percent: Option<f64>,
    /// Total cost in USD
    #[serde(default)]
    pub cost_usd: Option<f64>,
    /// Lines added
    #[serde(default)]
    pub lines_added: Option<i64>,
    /// Lines removed
    #[serde(default)]
    pub lines_removed: Option<i64>,
}

/// Internal session data with metadata
#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub cwd: String,
    pub status: SessionStatus,
    pub last_updated: Instant,
}

impl SessionData {
    pub fn new(payload: StatusPayload) -> Self {
        Self {
            session_id: payload.session_id,
            cwd: payload.cwd,
            status: payload.status,
            last_updated: Instant::now(),
        }
    }

    pub fn update(&mut self, payload: StatusPayload) {
        self.cwd = payload.cwd;
        self.status = payload.status;
        self.last_updated = Instant::now();
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_updated.elapsed() > timeout
    }
}

/// Aggregated metrics across all sessions
#[derive(Debug, Clone, Default, Serialize)]
pub struct AggregatedMetrics {
    pub active_sessions: usize,
    pub total_cost_usd: f64,
    pub average_context_percent: f64,
    pub total_lines_added: i64,
    pub total_lines_removed: i64,
}

/// Session state manager
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    timeout: Duration,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            timeout: Duration::from_secs(SESSION_TIMEOUT_SECS),
        }
    }

    /// Update session with new status payload
    pub fn update_session(&self, payload: StatusPayload) {
        let session_id = payload.session_id.clone();
        let mut sessions = self.sessions.write().expect("Failed to acquire write lock");

        if let Some(session) = sessions.get_mut(&session_id) {
            debug!("Updating existing session: {}", session_id);
            session.update(payload);
        } else {
            info!("New session registered: {}", session_id);
            sessions.insert(session_id.clone(), SessionData::new(payload));
        }
    }

    /// Remove expired sessions
    pub fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().expect("Failed to acquire write lock");
        let before_count = sessions.len();

        sessions.retain(|id, session| {
            let expired = session.is_expired(self.timeout);
            if expired {
                info!("Session expired and removed: {}", id);
            }
            !expired
        });

        let removed = before_count - sessions.len();
        if removed > 0 {
            debug!("Cleaned up {} expired sessions", removed);
        }
        removed
    }

    /// Get aggregated metrics across all sessions
    pub fn get_metrics(&self) -> AggregatedMetrics {
        let sessions = self.sessions.read().expect("Failed to acquire read lock");

        let active_sessions = sessions.len();
        if active_sessions == 0 {
            return AggregatedMetrics::default();
        }

        let mut total_cost = 0.0;
        let mut total_context = 0.0;
        let mut context_count = 0;
        let mut total_added = 0i64;
        let mut total_removed = 0i64;

        for session in sessions.values() {
            if let Some(cost) = session.status.cost_usd {
                total_cost += cost;
            }
            if let Some(context) = session.status.context_percent {
                total_context += context;
                context_count += 1;
            }
            if let Some(added) = session.status.lines_added {
                total_added += added;
            }
            if let Some(removed) = session.status.lines_removed {
                total_removed += removed;
            }
        }

        let avg_context = if context_count > 0 {
            total_context / context_count as f64
        } else {
            0.0
        };

        AggregatedMetrics {
            active_sessions,
            total_cost_usd: total_cost,
            average_context_percent: avg_context,
            total_lines_added: total_added,
            total_lines_removed: total_removed,
        }
    }

    /// Generate tooltip text for tray icon
    pub fn generate_tooltip(&self) -> String {
        let metrics = self.get_metrics();

        if metrics.active_sessions == 0 {
            return "Claude Code Notify\nNo active sessions".to_string();
        }

        format!(
            "Claude Code Notify\n\
             Sessions: {}\n\
             Cost: ${:.2}\n\
             Context: {:.0}%",
            metrics.active_sessions, metrics.total_cost_usd, metrics.average_context_percent
        )
    }

    /// Get list of all active sessions
    pub fn get_sessions(&self) -> Vec<SessionData> {
        let sessions = self.sessions.read().expect("Failed to acquire read lock");
        sessions.values().cloned().collect()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.read().expect("Failed to acquire read lock");
        sessions.len()
    }
}

// =============================================================================
// Session Name Manager
// =============================================================================

/// Human names in Katakana for session identification (150 names)
const SESSION_NAMES: &[&str] = &[
    // A
    "アリス", "アンナ", "アヤ", "アキ", "アオイ",
    "アレックス", "アンディ", "アーロン", "アダム", "エイミー",
    // B
    "ベン", "ボブ", "ブレイク", "ベラ", "ブルーノ",
    // C
    "チャーリー", "クロエ", "カール", "キャシー", "クリス",
    // D
    "ダン", "ダイアナ", "デイビッド", "ドリー", "ディラン",
    // E
    "エマ", "エミリー", "イーサン", "エリック", "エヴァ",
    // F
    "フィン", "フローラ", "フランク", "フェリックス", "フィオナ",
    // G
    "ジョージ", "グレース", "ガブリエル", "ジーナ", "ゴードン",
    // H
    "ハナ", "ヒロ", "ヘンリー", "ホリー", "ハルカ",
    // I
    "アイビー", "イアン", "イザベル", "イヴ", "イサム",
    // J
    "ジャック", "ジェーン", "ジェイク", "ジュリア", "ジョン",
    // K
    "ケイト", "カイ", "ケン", "キム", "カレン",
    // L
    "ルナ", "リオ", "レオ", "リリー", "ルーカス",
    // M
    "マヤ", "ミア", "マックス", "マイク", "モリー",
    // N
    "ノア", "ニナ", "ニック", "ナオミ", "ネイト",
    // O
    "オリバー", "オリビア", "オスカー", "オーウェン", "オパール",
    // P
    "ポール", "ペニー", "ピーター", "パム", "パトリック",
    // Q
    "クイン",
    // R
    "レイ", "ローズ", "ライアン", "レベッカ", "レオナ",
    // S
    "サラ", "ソフィア", "サム", "スカイ", "ショーン",
    "シオン", "セナ", "ソラ", "サクラ", "シュン",
    // T
    "トム", "ティナ", "タイラー", "テス", "トビー",
    // U
    "ウナ", "ウーゴ",
    // V
    "ヴィクター", "ヴィオラ", "ヴィンス", "ヴェラ", "ヴァル",
    // W
    "ウィル", "ウェンディ", "ワイアット", "ウィロー", "ウェイド",
    // X
    "ザビエル", "シアラ",
    // Y
    "ユキ", "ユウ", "ユナ", "ヨシ", "ユリ",
    // Z
    "ザック", "ゾーイ", "ゼン", "ザラ", "ジオ",
    // Additional names
    "アンジェラ", "ブライアン", "キャロル", "デレク", "エレナ",
    "フレッド", "グロリア", "ハロルド", "アイリス", "ジェシカ",
    "ケビン", "ローラ", "マーク", "ナンシー", "オスカル",
    "パメラ", "ロジャー", "ステラ", "テリー", "ウルスラ",
    "ビンセント", "ワンダ", "ザンダー", "イヴォンヌ", "ザカリー",
    "リナ", "タケシ", "ミサキ", "ケンジ", "アスカ",
    "リョウ", "マリコ", "ユウタ", "エリカ", "ダイキ",
];

/// Session name manager - maps session IDs to human-readable names
///
/// This manager assigns random names from a pool of 150 Katakana names
/// to session IDs. Names are assigned on first encounter and persisted for the session lifetime.
/// When all names are in use, the oldest session's name is recycled.
#[derive(Debug, Clone)]
pub struct SessionNameManager {
    /// Map from session_id to display name
    names: Arc<RwLock<HashMap<String, String>>>,
    /// Set of currently used names (to avoid duplicates)
    used_names: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl Default for SessionNameManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionNameManager {
    pub fn new() -> Self {
        Self {
            names: Arc::new(RwLock::new(HashMap::new())),
            used_names: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Get or create a display name for a session ID
    ///
    /// If the session ID already has a name, returns it.
    /// Otherwise, generates a random unique name and stores it.
    pub fn get_or_create_name(&self, session_id: &str) -> String {
        // Check if name already exists
        {
            let names = self.names.read().expect("Failed to acquire read lock");
            if let Some(name) = names.get(session_id) {
                return name.clone();
            }
        }

        // Generate a new unique name
        let new_name = self.generate_unique_name();

        // Store the new name
        {
            let mut names = self.names.write().expect("Failed to acquire write lock");
            let mut used = self.used_names.write().expect("Failed to acquire write lock");

            // Double-check in case another thread added it
            if let Some(name) = names.get(session_id) {
                return name.clone();
            }

            names.insert(session_id.to_string(), new_name.clone());
            used.insert(new_name.clone());
            info!("Assigned name '{}' to session '{}'", new_name, session_id);
        }

        new_name
    }

    /// Generate a unique random name that hasn't been used yet
    fn generate_unique_name(&self) -> String {
        let used = self.used_names.read().expect("Failed to acquire read lock");
        let mut rng = rand::rng();

        // Shuffle names and find an unused one
        let mut names: Vec<&str> = SESSION_NAMES.to_vec();
        names.shuffle(&mut rng);

        for name in &names {
            if !used.contains(*name) {
                return name.to_string();
            }
        }

        // All names are in use - this should rarely happen with 150 names
        // Return a random name anyway (will be duplicate but functional)
        names.first().copied().unwrap_or("アリス").to_string()
    }

    /// Remove a session and free up its name for reuse
    #[allow(dead_code)]
    pub fn remove_session(&self, session_id: &str) {
        let mut names = self.names.write().expect("Failed to acquire write lock");
        let mut used = self.used_names.write().expect("Failed to acquire write lock");

        if let Some(name) = names.remove(session_id) {
            used.remove(&name);
            info!("Removed session '{}' (was '{}')", session_id, name);
        }
    }

    /// Get the number of active sessions
    #[allow(dead_code)]
    pub fn session_count(&self) -> usize {
        let names = self.names.read().expect("Failed to acquire read lock");
        names.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_payload(session_id: &str) -> StatusPayload {
        StatusPayload {
            session_id: session_id.to_string(),
            cwd: "/test/path".to_string(),
            status: SessionStatus {
                state: Some("working".to_string()),
                context_percent: Some(45.5),
                cost_usd: Some(0.05),
                lines_added: Some(100),
                lines_removed: Some(20),
            },
            timestamp: None,
        }
    }

    #[test]
    fn test_session_manager_new_session() {
        let manager = SessionManager::new();
        let payload = create_test_payload("session-1");

        manager.update_session(payload);

        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_manager_update_session() {
        let manager = SessionManager::new();
        let payload1 = create_test_payload("session-1");
        manager.update_session(payload1);

        let mut payload2 = create_test_payload("session-1");
        payload2.status.cost_usd = Some(0.10);
        manager.update_session(payload2);

        assert_eq!(manager.session_count(), 1);
        let metrics = manager.get_metrics();
        assert!((metrics.total_cost_usd - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_aggregated_metrics() {
        let manager = SessionManager::new();

        let mut payload1 = create_test_payload("session-1");
        payload1.status.cost_usd = Some(0.05);
        payload1.status.context_percent = Some(40.0);
        manager.update_session(payload1);

        let mut payload2 = create_test_payload("session-2");
        payload2.status.cost_usd = Some(0.10);
        payload2.status.context_percent = Some(60.0);
        manager.update_session(payload2);

        let metrics = manager.get_metrics();
        assert_eq!(metrics.active_sessions, 2);
        assert!((metrics.total_cost_usd - 0.15).abs() < 0.001);
        assert!((metrics.average_context_percent - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_tooltip_generation() {
        let manager = SessionManager::new();

        // Empty state
        let tooltip = manager.generate_tooltip();
        assert!(tooltip.contains("No active sessions"));

        // With session
        let payload = create_test_payload("session-1");
        manager.update_session(payload);

        let tooltip = manager.generate_tooltip();
        assert!(tooltip.contains("Sessions: 1"));
        assert!(tooltip.contains("$0.05"));
    }

    // SessionNameManager tests

    #[test]
    fn test_session_name_manager_assigns_name() {
        let manager = SessionNameManager::new();
        let name = manager.get_or_create_name("wsl-12345");

        // Name should be a single Katakana name from the list
        assert!(!name.is_empty());
        assert!(SESSION_NAMES.contains(&name.as_str()));
    }

    #[test]
    fn test_session_name_manager_returns_same_name() {
        let manager = SessionNameManager::new();
        let name1 = manager.get_or_create_name("session-abc");
        let name2 = manager.get_or_create_name("session-abc");

        assert_eq!(name1, name2);
    }

    #[test]
    fn test_session_name_manager_unique_names() {
        let manager = SessionNameManager::new();
        let name1 = manager.get_or_create_name("session-1");
        let name2 = manager.get_or_create_name("session-2");
        let name3 = manager.get_or_create_name("session-3");

        // All names should be different
        assert_ne!(name1, name2);
        assert_ne!(name2, name3);
        assert_ne!(name1, name3);
    }

    #[test]
    fn test_session_name_manager_remove_session() {
        let manager = SessionNameManager::new();
        let _name = manager.get_or_create_name("session-to-remove");

        assert_eq!(manager.session_count(), 1);
        manager.remove_session("session-to-remove");
        assert_eq!(manager.session_count(), 0);
    }
}
