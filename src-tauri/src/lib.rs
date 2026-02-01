//! Claude Code Notification System
//!
//! A Tauri v2 application that provides desktop notifications
//! for Claude Code task completions via MQTT.

mod broker;
mod client;
mod export;
mod state;
mod templates;
mod tray;

use broker::MqttBroker;
use client::{topics, MqttMessage};
use serde::{Deserialize, Serialize};
use state::{SessionManager, StatusPayload};
use std::sync::Arc;
use tauri_plugin_notification::NotificationExt;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Payload structure for stop events from Claude Code
#[derive(Debug, Deserialize)]
struct StopEventPayload {
    #[allow(dead_code)]
    event: String,
    cwd: String,
    /// Human-readable session name (e.g., "Alice", "Bob")
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
    /// Human-readable session name (e.g., "Alice", "Bob")
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
    /// Human-readable session name (e.g., "Alice", "Bob")
    session_name: Option<String>,
    content: NotificationContent,
    #[allow(dead_code)]
    timestamp: Option<String>,
}

/// Content of a notification (elicitation dialogs, etc.)
#[derive(Debug, Deserialize)]
struct NotificationContent {
    #[serde(rename = "type")]
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

fn start_message_handler(app_handle: tauri::AppHandle, session_manager: Arc<SessionManager>) {
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
                handle_mqtt_message(&app_handle, &session_manager, msg);
            }
            warn!("MQTT message receiver closed");
        });
    });
}

fn handle_mqtt_message(
    app: &tauri::AppHandle,
    session_manager: &Arc<SessionManager>,
    msg: MqttMessage,
) {
    info!("Received MQTT message on topic: {}", msg.topic);

    match msg.topic.as_str() {
        topics::EVENTS_STOP => {
            if let Some(payload_str) = msg.payload_str() {
                match serde_json::from_str::<StopEventPayload>(payload_str) {
                    Ok(payload) => {
                        info!("Stop event received for: {}", payload.cwd);
                        show_stop_notification(app, &payload);
                    }
                    Err(e) => {
                        warn!("Failed to parse stop event payload: {}", e);
                        // Show notification with raw payload as fallback
                        show_simple_notification(app, "✅ タスク完了", payload_str);
                    }
                }
            }
        }
        topics::EVENTS_PERMISSION_REQUEST => {
            if let Some(payload_str) = msg.payload_str() {
                match serde_json::from_str::<PermissionRequestPayload>(payload_str) {
                    Ok(payload) => {
                        info!("Permission request received for: {}", payload.cwd);
                        show_permission_request_notification(app, &payload);
                    }
                    Err(e) => {
                        warn!("Failed to parse permission request payload: {}", e);
                        show_simple_notification(app, "⚠️ 承認依頼", payload_str);
                    }
                }
            }
        }
        topics::EVENTS_NOTIFICATION => {
            if let Some(payload_str) = msg.payload_str() {
                match serde_json::from_str::<NotificationEventPayload>(payload_str) {
                    Ok(payload) => {
                        info!("Notification event received for: {}", payload.cwd);
                        show_notification_event(app, &payload);
                    }
                    Err(e) => {
                        warn!("Failed to parse notification event payload: {}", e);
                        show_simple_notification(app, "💬 通知", payload_str);
                    }
                }
            }
        }
        topics::TASK_COMPLETE => {
            if let Some(payload) = msg.payload_str() {
                info!("Task completed: {}", payload);
                show_simple_notification(app, "✅ タスク完了", payload);
            }
        }
        topics::ERROR => {
            if let Some(payload) = msg.payload_str() {
                warn!("Error notification: {}", payload);
                show_simple_notification(app, "❌ エラー", payload);
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

/// Show notification for stop event
fn show_stop_notification(app: &tauri::AppHandle, payload: &StopEventPayload) {
    let project = extract_project_name(&payload.cwd);

    // Format title with optional session name
    let title = match &payload.session_name {
        Some(name) => format!("✅ タスク完了 - {} [{}]", project, name),
        None => format!("✅ タスク完了 - {}", project),
    };
    let body = payload.cwd.clone();

    info!("Attempting to show notification: {} - {}", title, body);

    match app.notification().builder().title(&title).body(&body).show() {
        Ok(_) => {
            info!("Notification sent successfully");
        }
        Err(e) => {
            error!("Failed to show notification: {}", e);
        }
    }
}

/// Show notification for permission request (approval needed) or AskUserQuestion
fn show_permission_request_notification(app: &tauri::AppHandle, payload: &PermissionRequestPayload) {
    let project = extract_project_name(&payload.cwd);

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
        show_ask_user_question_notification(app, payload, project);
    } else {
        // Show as a permission request notification
        show_tool_permission_notification(app, payload, project);
    }
}

/// Show notification for AskUserQuestion (Claude is asking a question)
fn show_ask_user_question_notification(app: &tauri::AppHandle, payload: &PermissionRequestPayload, project: &str) {
    // Format title with optional session name
    let title = match &payload.session_name {
        Some(name) => format!("❓ 質問 - {} [{}]", project, name),
        None => format!("❓ 質問 - {}", project),
    };

    // Try to extract the question text
    let body = extract_question_text(&payload.content)
        .unwrap_or_else(|| "Claude から質問が来ています".to_string());

    info!("Attempting to show AskUserQuestion notification: {} - {}", title, body);

    match app.notification().builder().title(&title).body(&body).show() {
        Ok(_) => {
            info!("Notification sent successfully");
        }
        Err(e) => {
            error!("Failed to show notification: {}", e);
        }
    }
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
fn show_tool_permission_notification(app: &tauri::AppHandle, payload: &PermissionRequestPayload, project: &str) {
    // Format title with optional session name
    let title = match &payload.session_name {
        Some(name) => format!("⚠️ 承認依頼 - {} [{}]", project, name),
        None => format!("⚠️ 承認依頼 - {}", project),
    };

    // Try to extract useful info from content
    let body = if let Some(tool_name) = &payload.content.tool_name {
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
            format!("承認が必要: {}", truncated)
        }
    } else {
        "ツールの実行許可が必要です".to_string()
    };

    info!("Attempting to show notification: {} - {}", title, body);

    match app.notification().builder().title(&title).body(&body).show() {
        Ok(_) => {
            info!("Notification sent successfully");
        }
        Err(e) => {
            error!("Failed to show notification: {}", e);
        }
    }
}

/// Show simple notification with title and body
fn show_simple_notification(app: &tauri::AppHandle, title: &str, body: &str) {
    info!("Attempting to show notification: {} - {}", title, body);
    match app.notification().builder().title(title).body(body).show() {
        Ok(_) => {
            info!("Notification sent successfully");
        }
        Err(e) => {
            error!("Failed to show notification: {}", e);
        }
    }
}

/// Show notification for elicitation dialogs (user input requests)
fn show_notification_event(app: &tauri::AppHandle, payload: &NotificationEventPayload) {
    let project = extract_project_name(&payload.cwd);

    // Format title with optional session name
    let title = match &payload.session_name {
        Some(name) => format!("💬 入力が必要です - {} [{}]", project, name),
        None => format!("💬 入力が必要です - {}", project),
    };

    // Try to extract message from content
    let message = if let Some(msg) = payload.content.message.as_deref() {
        msg.to_string()
    } else if let Some(title) = payload.content.title.as_deref() {
        title.to_string()
    } else if let Some(raw) = &payload.content.raw {
        // Fallback: try to parse raw JSON
        if let Ok(raw_json) = serde_json::from_str::<serde_json::Value>(raw) {
            raw_json.get("message")
                .or_else(|| raw_json.get("title"))
                .or_else(|| raw_json.get("question"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Claude が入力を待っています".to_string())
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
        "Claude が入力を待っています".to_string()
    };

    info!("Attempting to show notification: {} - {}", title, message);

    match app.notification().builder().title(&title).body(&message).show() {
        Ok(_) => {
            info!("Notification sent successfully");
        }
        Err(e) => {
            error!("Failed to show notification: {}", e);
        }
    }
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
    let app_state = std::sync::Mutex::new(AppState {
        broker: Some(broker),
        session_manager: session_manager.clone(),
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
        .manage(app_state)
        .setup(move |app| {
            info!("Setting up Tauri application...");

            let _tray = tray::init_tray(app)?;

            let app_handle = app.handle().clone();
            start_message_handler(app_handle, session_manager.clone());

            info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_broker_status,
            detect_ip,
            generate_config_zip,
            generate_config_zip_v2
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent the window from closing, hide it instead
                api.prevent_close();
                if let Err(e) = window.hide() {
                    error!("Failed to hide window: {}", e);
                } else {
                    info!("Window hidden to system tray");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
