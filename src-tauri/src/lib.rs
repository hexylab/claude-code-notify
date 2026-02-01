//! Claude Code Notification System
//!
//! A Tauri v2 application that provides desktop notifications
//! for Claude Code task completions via MQTT.

mod audio;
mod broker;
mod client;
mod export;
mod notification_state;
mod settings;
mod state;
mod taskbar;
mod templates;
mod tray;
mod tray_flash;

use broker::MqttBroker;
use client::{topics, MqttMessage};
use notification_state::NotificationState;
use serde::{Deserialize, Serialize};
use settings::NotificationSettings;
use state::{SessionManager, SessionNameManager, StatusPayload};
use std::sync::{Arc, RwLock};
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Payload structure for stop events from Claude Code
#[derive(Debug, Deserialize)]
struct StopEventPayload {
    #[allow(dead_code)]
    event: String,
    cwd: String,
    /// Session identifier (hostname-ppid format)
    session_id: Option<String>,
    /// Legacy: Human-readable session name (deprecated, use session_id instead)
    #[allow(dead_code)]
    session_name: Option<String>,
    #[allow(dead_code)]
    timestamp: Option<String>,
}

/// Payload structure for permission request events from Claude Code
#[derive(Debug, Deserialize)]
struct PermissionRequestPayload {
    #[allow(dead_code)]
    event: String,
    cwd: String,
    /// Session identifier (hostname-ppid format)
    session_id: Option<String>,
    /// Legacy: Human-readable session name (deprecated, use session_id instead)
    #[allow(dead_code)]
    session_name: Option<String>,
    content: PermissionRequestContent,
    #[allow(dead_code)]
    timestamp: Option<String>,
}

/// Content of a permission request (tool name, input, etc.)
#[derive(Debug, Deserialize)]
struct PermissionRequestContent {
    tool_name: Option<String>,
    tool_input: Option<serde_json::Value>,
    /// Fallback raw content when JSON parsing fails in the hook script
    raw: Option<String>,
}

/// Payload structure for notification events from Claude Code
#[derive(Debug, Deserialize)]
struct NotificationEventPayload {
    #[allow(dead_code)]
    event: String,
    cwd: String,
    /// Session identifier (hostname-ppid format)
    session_id: Option<String>,
    /// Legacy: Human-readable session name (deprecated, use session_id instead)
    #[allow(dead_code)]
    session_name: Option<String>,
    content: NotificationContent,
    #[allow(dead_code)]
    timestamp: Option<String>,
}

/// Content of a notification (elicitation dialogs, etc.)
#[derive(Debug, Deserialize)]
struct NotificationContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    notification_type: Option<String>,
    title: Option<String>,
    message: Option<String>,
    #[allow(dead_code)]
    question: Option<String>,
    /// Fallback raw content when JSON parsing fails in the hook script
    raw: Option<String>,
}

fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

pub struct AppState {
    pub broker: Option<MqttBroker>,
    pub session_manager: Arc<SessionManager>,
    pub session_name_manager: Arc<SessionNameManager>,
}

/// 通知を一元管理するマネージャー
/// 設定に基づいて、音声・タスクバー・トレイアイコン・Toast通知を制御する
pub struct NotificationManager {
    settings: Arc<RwLock<NotificationSettings>>,
    state: NotificationState,
    tray_flasher: tray_flash::TrayFlasher,
}

// NotificationManager を Send + Sync にするため、HWND を保持しない
unsafe impl Send for NotificationManager {}
unsafe impl Sync for NotificationManager {}

impl NotificationManager {
    /// 新しい NotificationManager を作成
    pub fn new(app: &tauri::AppHandle) -> Self {
        let settings = settings::load_settings(app);

        Self {
            settings: Arc::new(RwLock::new(settings)),
            state: NotificationState::new(),
            tray_flasher: tray_flash::TrayFlasher::new(),
        }
    }

    /// 設定を更新
    pub fn update_settings(&self, new_settings: NotificationSettings) {
        if let Ok(mut settings) = self.settings.write() {
            *settings = new_settings;
        }
    }

    /// 現在の設定を取得
    pub fn get_settings(&self) -> NotificationSettings {
        self.settings.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// 通知を発火（すべての通知チャネルを統合管理）
    pub fn notify(&self, app: &tauri::AppHandle, title: &str, body: &str) {
        let settings = self.get_settings();

        // 1. Toast通知
        if settings.toast_notification_enabled {
            match app.notification().builder().title(title).body(body).show() {
                Ok(_) => info!("Toast notification sent"),
                Err(e) => error!("Failed to show toast notification: {}", e),
            }
        }

        // 2. 通知音
        if settings.sound_enabled {
            audio::play_notification_sound(settings.sound_volume);
        }

        // 3. 未確認カウント増加
        let count = self.state.increment();

        // 4. ウィンドウの表示状態を確認
        let window_visible = app
            .get_webview_window("main")
            .map(|w| w.is_visible().unwrap_or(false))
            .unwrap_or(false);

        // 5. タスクバー機能（Windows専用、ウィンドウが表示されている場合）
        #[cfg(windows)]
        if window_visible {
            if let Some(window) = app.get_webview_window("main") {
                if let Some(hwnd) = taskbar::get_hwnd(&window) {
                    // タスクバー点滅
                    if settings.taskbar_flash_enabled {
                        taskbar::flash_taskbar(hwnd, 3);
                    }

                    // バッジ更新
                    if settings.taskbar_badge_enabled {
                        if let Err(e) = taskbar::set_overlay_badge(hwnd, count) {
                            error!("Failed to set overlay badge: {}", e);
                        }
                    }
                }
            }
        }

        // 6. トレイアイコン点滅（ウィンドウが非表示の場合）
        if !window_visible && settings.tray_flash_enabled {
            self.tray_flasher.start_flash(app);
        }
    }

    /// 通知状態をリセット（ウィンドウがフォーカスを得た時など）
    pub fn reset(&self, app: &tauri::AppHandle) {
        self.state.reset();

        // トレイアイコン点滅を停止
        self.tray_flasher.stop_flash(app);

        #[cfg(windows)]
        if let Some(window) = app.get_webview_window("main") {
            if let Some(hwnd) = taskbar::get_hwnd(&window) {
                if let Err(e) = taskbar::clear_overlay_badge(hwnd) {
                    error!("Failed to clear overlay badge: {}", e);
                }
                taskbar::stop_flash(hwnd);
            }
        }
    }

    /// 未確認カウントを取得
    #[allow(dead_code)]
    pub fn get_unread_count(&self) -> u32 {
        self.state.get()
    }
}

#[tauri::command]
fn get_broker_status(state: tauri::State<'_, std::sync::Mutex<AppState>>) -> bool {
    state
        .lock()
        .map(|s| s.broker.as_ref().map(|b| b.is_running()).unwrap_or(false))
        .unwrap_or(false)
}

#[tauri::command]
fn detect_ip() -> Result<String, String> {
    export::detect_local_ip().map_err(|e| e.to_string())
}

#[tauri::command]
fn generate_config_zip(host: String, port: u16) -> Result<Vec<u8>, String> {
    let config = export::ExportConfig {
        host,
        port,
        client_type: export::ClientType::MosquittoPub,
    };
    export::generate_export_zip(&config).map_err(|e| e.to_string())
}

/// Export options for platform-specific configuration export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub host: String,
    pub port: u16,
    pub platform: String, // "linux_wsl" or "windows"
}

#[tauri::command]
fn generate_config_zip_v2(options: ExportOptions) -> Result<Vec<u8>, String> {
    let platform = match options.platform.as_str() {
        "windows" => export::ExportPlatform::Windows,
        _ => export::ExportPlatform::LinuxWsl,
    };

    let config = export::ExportConfig {
        host: options.host,
        port: options.port,
        client_type: export::ClientType::MosquittoPub,
    };

    // For Windows export, try to include the mqtt-publish.exe binary
    let mqtt_publish_exe = if platform == export::ExportPlatform::Windows {
        // Try to read from the workspace target directory
        let exe_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/release/mqtt-publish.exe");
        std::fs::read(exe_path).ok()
    } else {
        None
    };

    export::generate_export_zip_for_platform(
        &config,
        platform,
        mqtt_publish_exe.as_deref(),
    )
    .map_err(|e| e.to_string())
}

fn start_message_handler(
    app_handle: tauri::AppHandle,
    session_manager: Arc<SessionManager>,
    session_name_manager: Arc<SessionNameManager>,
    notification_manager: Arc<NotificationManager>,
) {
    // Wait for broker to start
    std::thread::sleep(std::time::Duration::from_secs(1));

    let (_client, mut rx) = client::start_mqtt_client("claude-code-notify-client");

    info!("MQTT client started, listening for notifications...");

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async move {
            while let Some(msg) = rx.recv().await {
                handle_mqtt_message(&app_handle, &session_manager, &session_name_manager, &notification_manager, msg);
            }
            warn!("MQTT message receiver closed");
        });
    });
}

fn handle_mqtt_message(
    app: &tauri::AppHandle,
    session_manager: &Arc<SessionManager>,
    session_name_manager: &Arc<SessionNameManager>,
    notification_manager: &Arc<NotificationManager>,
    msg: MqttMessage,
) {
    info!("Received MQTT message on topic: {}", msg.topic);

    match msg.topic.as_str() {
        topics::EVENTS_STOP => {
            if let Some(payload_str) = msg.payload_str() {
                match serde_json::from_str::<StopEventPayload>(payload_str) {
                    Ok(payload) => {
                        info!("Stop event received for: {}", payload.cwd);
                        show_stop_notification(app, session_name_manager, notification_manager, &payload);
                    }
                    Err(e) => {
                        warn!("Failed to parse stop event payload: {}", e);
                        // Show notification with raw payload as fallback
                        show_simple_notification(app, notification_manager, "✅ タスク完了", payload_str);
                    }
                }
            }
        }
        topics::EVENTS_PERMISSION_REQUEST => {
            if let Some(payload_str) = msg.payload_str() {
                match serde_json::from_str::<PermissionRequestPayload>(payload_str) {
                    Ok(payload) => {
                        info!("Permission request received for: {}", payload.cwd);
                        show_permission_request_notification(app, session_name_manager, notification_manager, &payload);
                    }
                    Err(e) => {
                        warn!("Failed to parse permission request payload: {}", e);
                        show_simple_notification(app, notification_manager, "⚠️ 承認依頼", payload_str);
                    }
                }
            }
        }
        topics::EVENTS_NOTIFICATION => {
            if let Some(payload_str) = msg.payload_str() {
                match serde_json::from_str::<NotificationEventPayload>(payload_str) {
                    Ok(payload) => {
                        info!("Notification event received for: {}", payload.cwd);
                        show_notification_event(app, session_name_manager, notification_manager, &payload);
                    }
                    Err(e) => {
                        warn!("Failed to parse notification event payload: {}", e);
                        show_simple_notification(app, notification_manager, "💬 通知", payload_str);
                    }
                }
            }
        }
        topics::TASK_COMPLETE => {
            if let Some(payload) = msg.payload_str() {
                info!("Task completed: {}", payload);
                show_simple_notification(app, notification_manager, "✅ タスク完了", payload);
            }
        }
        topics::ERROR => {
            if let Some(payload) = msg.payload_str() {
                warn!("Error notification: {}", payload);
                show_simple_notification(app, notification_manager, "❌ エラー", payload);
            }
        }
        topic if topic.starts_with(topics::STATUS_PREFIX) => {
            if let Some(payload_str) = msg.payload_str() {
                info!("Status update on {}: {}", topic, payload_str);
                match serde_json::from_str::<StatusPayload>(payload_str) {
                    Ok(payload) => {
                        session_manager.update_session(payload);
                        // Cleanup expired sessions periodically
                        session_manager.cleanup_expired();
                        // Update tray tooltip
                        update_tray_tooltip(app, session_manager);
                    }
                    Err(e) => {
                        warn!("Failed to parse status payload: {}", e);
                    }
                }
            }
        }
        topics::STATUS => {
            if let Some(payload) = msg.payload_str() {
                info!("Status update: {}", payload);
            }
        }
        _ => {
            if let Some(payload) = msg.payload_str() {
                info!("Message: {}", payload);
            }
        }
    }
}

/// Extract project name from path
fn extract_project_name(cwd: &str) -> &str {
    std::path::Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(cwd)
}

/// Resolve session name from session_id using SessionNameManager
fn resolve_session_name(session_name_manager: &SessionNameManager, session_id: Option<&str>) -> Option<String> {
    session_id.map(|id| session_name_manager.get_or_create_name(id))
}

/// Show notification for stop event
fn show_stop_notification(
    app: &tauri::AppHandle,
    session_name_manager: &SessionNameManager,
    notification_manager: &NotificationManager,
    payload: &StopEventPayload,
) {
    let project = extract_project_name(&payload.cwd);

    // Resolve session name from session_id (SMS-style: sender name as title)
    let session_name = resolve_session_name(session_name_manager, payload.session_id.as_deref());
    let title = session_name.unwrap_or_else(|| "Claude Code".to_string());

    // SMS-style body: event type + project name
    let body = format!("✅ タスクが完了しました\n📁 {}", project);

    info!("Attempting to show notification: {} - {}", title, body);

    // Use NotificationManager for unified notification handling
    notification_manager.notify(app, &title, &body);
}

/// Show notification for permission request (approval needed) or AskUserQuestion
fn show_permission_request_notification(
    app: &tauri::AppHandle,
    session_name_manager: &SessionNameManager,
    notification_manager: &NotificationManager,
    payload: &PermissionRequestPayload,
) {
    let project = extract_project_name(&payload.cwd);

    // Resolve session name from session_id
    let session_name = resolve_session_name(session_name_manager, payload.session_id.as_deref());

    // Check if this is an AskUserQuestion (question from Claude, not a permission request)
    let is_ask_user_question = payload.content.tool_name.as_deref() == Some("AskUserQuestion")
        || payload.content.raw.as_ref().map_or(false, |raw| {
            serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|v| v.get("tool_name").and_then(|t| t.as_str()).map(|s| s == "AskUserQuestion"))
                .unwrap_or(false)
        });

    if is_ask_user_question {
        // Show as a question notification
        show_ask_user_question_notification(app, notification_manager, payload, project, session_name.as_deref());
    } else {
        // Show as a permission request notification
        show_tool_permission_notification(app, notification_manager, payload, project, session_name.as_deref());
    }
}

/// Show notification for AskUserQuestion (Claude is asking a question)
fn show_ask_user_question_notification(
    app: &tauri::AppHandle,
    notification_manager: &NotificationManager,
    payload: &PermissionRequestPayload,
    project: &str,
    session_name: Option<&str>,
) {
    // SMS-style: sender name as title
    let title = session_name.unwrap_or("Claude Code").to_string();

    // Try to extract the question text
    let question_text = extract_question_text(&payload.content)
        .unwrap_or_else(|| "質問が来ています".to_string());

    // SMS-style body: event type + question
    let body = format!("❓ 質問があります\n{}\n📁 {}", question_text, project);

    info!("Attempting to show AskUserQuestion notification: {} - {}", title, body);

    // Use NotificationManager for unified notification handling
    notification_manager.notify(app, &title, &body);
}

/// Extract question text from AskUserQuestion content
fn extract_question_text(content: &PermissionRequestContent) -> Option<String> {
    // Try to get from tool_input.questions[0].question
    if let Some(input) = &content.tool_input {
        if let Some(questions) = input.get("questions").and_then(|q| q.as_array()) {
            if let Some(first_question) = questions.first() {
                if let Some(question_text) = first_question.get("question").and_then(|q| q.as_str()) {
                    return Some(question_text.to_string());
                }
            }
        }
    }

    // Try to parse from raw JSON
    if let Some(raw) = &content.raw {
        if let Ok(raw_json) = serde_json::from_str::<serde_json::Value>(raw) {
            // Try tool_input.questions[0].question path
            if let Some(questions) = raw_json.get("tool_input")
                .and_then(|ti| ti.get("questions"))
                .and_then(|q| q.as_array())
            {
                if let Some(first_question) = questions.first() {
                    if let Some(question_text) = first_question.get("question").and_then(|q| q.as_str()) {
                        return Some(question_text.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Show notification for tool permission request (approval needed)
fn show_tool_permission_notification(
    app: &tauri::AppHandle,
    notification_manager: &NotificationManager,
    payload: &PermissionRequestPayload,
    project: &str,
    session_name: Option<&str>,
) {
    // SMS-style: sender name as title
    let title = session_name.unwrap_or("Claude Code").to_string();

    // Try to extract useful info from content
    let tool_info = if let Some(tool_name) = &payload.content.tool_name {
        // Standard format with tool_name
        if let Some(input) = &payload.content.tool_input {
            if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                format!("{}: {}", tool_name, command)
            } else {
                format!("{} の実行許可が必要です", tool_name)
            }
        } else {
            format!("{} の実行許可が必要です", tool_name)
        }
    } else if let Some(raw) = &payload.content.raw {
        // Fallback: try to parse raw JSON from Claude Code
        if let Ok(raw_json) = serde_json::from_str::<serde_json::Value>(raw) {
            // Try to extract tool info from raw JSON
            let tool = raw_json.get("tool_name")
                .or_else(|| raw_json.get("tool"))
                .and_then(|v| v.as_str());

            let command = raw_json.get("tool_input")
                .or_else(|| raw_json.get("input"))
                .and_then(|v| v.get("command"))
                .and_then(|v| v.as_str());

            match (tool, command) {
                (Some(t), Some(c)) => format!("{}: {}", t, c),
                (Some(t), None) => format!("{} の実行許可が必要です", t),
                (None, Some(c)) => format!("コマンド: {}", c),
                (None, None) => "ツールの実行許可が必要です".to_string(),
            }
        } else {
            // Raw is not valid JSON, show truncated version
            let truncated = if raw.len() > 100 {
                format!("{}...", &raw[..100])
            } else {
                raw.clone()
            };
            truncated
        }
    } else {
        "ツールの実行許可が必要です".to_string()
    };

    // SMS-style body: event type + tool info + project
    let body = format!("⚠️ 承認が必要です\n{}\n📁 {}", tool_info, project);

    info!("Attempting to show notification: {} - {}", title, body);

    // Use NotificationManager for unified notification handling
    notification_manager.notify(app, &title, &body);
}

/// Show simple notification with title and body
fn show_simple_notification(app: &tauri::AppHandle, notification_manager: &NotificationManager, title: &str, body: &str) {
    info!("Attempting to show notification: {} - {}", title, body);
    // Use NotificationManager for unified notification handling
    notification_manager.notify(app, title, body);
}

/// Show notification for elicitation dialogs (user input requests)
fn show_notification_event(
    app: &tauri::AppHandle,
    session_name_manager: &SessionNameManager,
    notification_manager: &NotificationManager,
    payload: &NotificationEventPayload,
) {
    let project = extract_project_name(&payload.cwd);

    // Resolve session name from session_id (SMS-style: sender name as title)
    let session_name = resolve_session_name(session_name_manager, payload.session_id.as_deref());
    let title = session_name.unwrap_or_else(|| "Claude Code".to_string());

    // Try to extract message from content
    let message = if let Some(msg) = payload.content.message.as_deref() {
        msg.to_string()
    } else if let Some(content_title) = payload.content.title.as_deref() {
        content_title.to_string()
    } else if let Some(raw) = &payload.content.raw {
        // Fallback: try to parse raw JSON
        if let Ok(raw_json) = serde_json::from_str::<serde_json::Value>(raw) {
            raw_json.get("message")
                .or_else(|| raw_json.get("title"))
                .or_else(|| raw_json.get("question"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "入力を待っています".to_string())
        } else {
            // Raw is not valid JSON
            let truncated = if raw.len() > 100 {
                format!("{}...", &raw[..100])
            } else {
                raw.clone()
            };
            truncated
        }
    } else {
        "入力を待っています".to_string()
    };

    // SMS-style body: event type + message + project
    let body = format!("💬 入力が必要です\n{}\n📁 {}", message, project);

    info!("Attempting to show notification: {} - {}", title, body);

    // Use NotificationManager for unified notification handling
    notification_manager.notify(app, &title, &body);
}

/// Update tray icon tooltip with session metrics
fn update_tray_tooltip(app: &tauri::AppHandle, session_manager: &Arc<SessionManager>) {
    let tooltip = session_manager.generate_tooltip();

    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Err(e) = tray.set_tooltip(Some(&tooltip)) {
            warn!("Failed to update tray tooltip: {}", e);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    info!("Starting Claude Code Notify...");

    // Initialize audio system
    if let Err(e) = audio::init_audio() {
        error!("Failed to initialize audio system: {}", e);
    }

    // Initialize taskbar system (Windows only)
    if let Err(e) = taskbar::init_taskbar() {
        error!("Failed to initialize taskbar system: {}", e);
    }

    let mut broker = match MqttBroker::with_default_config() {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to create MQTT broker: {:?}", e);
            return;
        }
    };

    if let Err(e) = broker.start() {
        error!("Failed to start MQTT broker: {:?}", e);
        return;
    }

    let session_manager = Arc::new(SessionManager::new());
    let session_name_manager = Arc::new(SessionNameManager::new());
    let app_state = std::sync::Mutex::new(AppState {
        broker: Some(broker),
        session_manager: session_manager.clone(),
        session_name_manager: session_name_manager.clone(),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance is launched, show notification
            info!("Second instance detected, application is already running");
            if let Err(e) = app
                .notification()
                .builder()
                .title("Claude Code Notify")
                .body("アプリケーションは既に起動しています")
                .show()
            {
                warn!("Failed to show duplicate instance notification: {}", e);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(app_state)
        .setup(move |app| {
            info!("Setting up Tauri application...");

            let _tray = tray::init_tray(app)?;

            // Create NotificationManager
            let notification_manager = Arc::new(NotificationManager::new(app.handle()));

            // Store NotificationManager in app state for access from window events
            app.manage(notification_manager.clone());

            let app_handle = app.handle().clone();
            start_message_handler(app_handle, session_manager.clone(), session_name_manager.clone(), notification_manager);

            info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_broker_status,
            detect_ip,
            generate_config_zip,
            generate_config_zip_v2,
            settings::get_settings,
            settings::save_settings_command,
            audio::play_test_sound
        ])
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::Focused(true) => {
                    // Reset notification state when window gains focus
                    let app_handle = window.app_handle();
                    if let Some(notification_manager) = app_handle.try_state::<Arc<NotificationManager>>() {
                        notification_manager.reset(app_handle);
                        info!("Notification state reset on window focus");
                    }
                }
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // Prevent the window from closing, hide it instead
                    api.prevent_close();
                    if let Err(e) = window.hide() {
                        error!("Failed to hide window: {}", e);
                    } else {
                        info!("Window hidden to system tray");
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
