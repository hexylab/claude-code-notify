//! mqtt-publish - Claude Code Notify 用軽量 MQTT 送信ツール
//!
//! 使い方:
//!   mqtt-publish --event stop -h <host> -p <port> [--detach]
//!   mqtt-publish -h <host> -p <port> -t <topic> -m <message>
//!   mqtt-publish -h <host> -p <port> -t <topic> --stdin
//!
//! `--event` モードでは Claude Code のフックが渡す JSON を stdin から読み、
//! 適切な envelope を作成してトピックに publish する。`--detach` が指定さ
//! れた場合、親プロセスは子プロセスを spawn して即座に exit 0 する。
//! これにより Tauri アプリ未起動時でも Claude Code がブロックされない。

use clap::{Parser, ValueEnum};
use rumqttc::{Client, MqttOptions, QoS};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::process::{Command as ProcCommand, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// 子プロセスであることを示す環境変数
const CHILD_ENV: &str = "__MQTT_PUBLISH_CHILD";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum EventKind {
    Stop,
    #[value(name = "permission-request")]
    PermissionRequest,
    Notification,
    Statusline,
}

#[derive(Parser, Debug)]
#[command(name = "mqtt-publish")]
#[command(version)]
#[command(about = "Publish MQTT messages for Claude Code Notify")]
struct Args {
    /// MQTT ブローカーのホスト
    #[arg(short = 'h', long, default_value = "127.0.0.1")]
    host: String,

    /// MQTT ブローカーのポート
    #[arg(short = 'p', long, default_value_t = 1883)]
    port: u16,

    /// MQTT トピック（legacy モード、--event 使用時は不要）
    #[arg(short = 't', long)]
    topic: Option<String>,

    /// メッセージ本文（--stdin と排他）
    #[arg(short = 'm', long, conflicts_with = "stdin")]
    message: Option<String>,

    /// stdin から本文を読む
    #[arg(long)]
    stdin: bool,

    /// retain フラグ
    #[arg(short = 'r', long, default_value_t = false)]
    retain: bool,

    /// Claude Code フックのイベント種別
    #[arg(long, value_enum)]
    event: Option<EventKind>,

    /// 親プロセスを即 exit 0 し、publish をバックグラウンドで行う
    #[arg(long, default_value_t = false)]
    detach: bool,

    /// 接続タイムアウト（秒）
    #[arg(long, default_value_t = 1)]
    timeout: u64,
}

fn main() {
    let is_child = std::env::var(CHILD_ENV).is_ok();
    let args = Args::parse();

    if let Some(event) = args.event {
        run_event_mode(&args, event, is_child);
        return;
    }

    run_legacy_mode(&args, is_child);
}

fn run_event_mode(args: &Args, event: EventKind, is_child: bool) {
    let input = read_stdin_string().unwrap_or_default();

    let (topic, payload, statusline_out) = match build_event_payload(event, &input) {
        Ok(v) => v,
        Err(e) => {
            debug_log(format!("build_event_payload failed: {}", e));
            std::process::exit(0);
        }
    };

    // statusline の場合は先にテキストを出力（publish 失敗でも表示を保証）
    if let Some(out) = statusline_out {
        print!("{}", out);
        let _ = io::stdout().flush();
    }

    if args.detach && !is_child {
        if let Err(e) =
            spawn_detached_publisher(&args.host, args.port, &topic, args.retain, &payload)
        {
            debug_log(format!("spawn detached failed: {}", e));
        }
        std::process::exit(0);
    }

    if let Err(e) = publish_with_timeout(
        &args.host,
        args.port,
        &topic,
        args.retain,
        &payload,
        args.timeout,
    ) {
        debug_log(format!("publish failed: {}", e));
    }
    std::process::exit(0);
}

fn run_legacy_mode(args: &Args, is_child: bool) {
    let topic = match args.topic.clone() {
        Some(t) => t,
        None => {
            eprintln!("Error: --topic or --event is required");
            std::process::exit(1);
        }
    };

    let payload = if args.stdin {
        match read_stdin_string() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(m) = args.message.clone() {
        m
    } else {
        eprintln!("Error: --message or --stdin is required");
        std::process::exit(1);
    };

    if args.detach && !is_child {
        if let Err(e) =
            spawn_detached_publisher(&args.host, args.port, &topic, args.retain, &payload)
        {
            debug_log(format!("spawn detached failed: {}", e));
        }
        std::process::exit(0);
    }

    match publish_with_timeout(
        &args.host,
        args.port,
        &topic,
        args.retain,
        &payload,
        args.timeout,
    ) {
        Ok(()) => {}
        Err(e) => {
            if is_child {
                debug_log(format!("child publish failed: {}", e));
                std::process::exit(0);
            }
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn read_stdin_string() -> Result<String, String> {
    let mut buffer = Vec::new();
    io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read stdin: {}", e))?;

    let s = if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&buffer[3..]).to_string()
    } else if buffer.starts_with(&[0xFF, 0xFE]) {
        let utf16: Vec<u16> = buffer[2..]
            .chunks(2)
            .filter_map(|c| {
                if c.len() == 2 {
                    Some(u16::from_le_bytes([c[0], c[1]]))
                } else {
                    None
                }
            })
            .collect();
        String::from_utf16_lossy(&utf16)
    } else {
        String::from_utf8_lossy(&buffer).to_string()
    };

    Ok(s.trim_end().to_string())
}

/// `--event` モードの envelope を構築する
fn build_event_payload(
    event: EventKind,
    input: &str,
) -> Result<(String, String, Option<String>), String> {
    let input_json: Value = if input.is_empty() {
        json!({})
    } else {
        serde_json::from_str(input).unwrap_or_else(|_| json!({ "raw": input }))
    };

    let session_id = input_json
        .get("session_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| format!("{}-unknown", fallback_hostname()));

    let cwd = input_json
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(String::from))
                .unwrap_or_default()
        });

    let timestamp = chrono::Utc::now().to_rfc3339();

    match event {
        EventKind::Stop => {
            let payload = json!({
                "event": "stop",
                "cwd": cwd,
                "session_id": session_id,
                "timestamp": timestamp,
            });
            Ok((
                "claude-code/events/stop".to_string(),
                payload.to_string(),
                None,
            ))
        }
        EventKind::PermissionRequest => {
            let payload = json!({
                "event": "permission-request",
                "cwd": cwd,
                "session_id": session_id,
                "content": input_json,
                "timestamp": timestamp,
            });
            Ok((
                "claude-code/events/permission-request".to_string(),
                payload.to_string(),
                None,
            ))
        }
        EventKind::Notification => {
            let payload = json!({
                "event": "notification",
                "cwd": cwd,
                "session_id": session_id,
                "content": input_json,
                "timestamp": timestamp,
            });
            Ok((
                "claude-code/events/notification".to_string(),
                payload.to_string(),
                None,
            ))
        }
        EventKind::Statusline => {
            let model = input_json
                .pointer("/model/display_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Claude")
                .to_string();
            let cost = input_json
                .pointer("/cost/total_cost_usd")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let context_pct = input_json
                .pointer("/context_window/used_percentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let lines_added = input_json
                .pointer("/cost/total_lines_added")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let lines_removed = input_json
                .pointer("/cost/total_lines_removed")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let topic = format!("claude-code/status/{}", session_id);
            let payload = json!({
                "session_id": session_id,
                "cwd": cwd,
                "status": {
                    "state": "active",
                    "context_percent": context_pct,
                    "cost_usd": cost,
                    "lines_added": lines_added,
                    "lines_removed": lines_removed,
                },
                "timestamp": timestamp,
            });
            let line = format!(
                "[{}] ${:.4} | Ctx: {:.0}% | +{}/-{}",
                model, cost, context_pct, lines_added, lines_removed
            );
            Ok((topic, payload.to_string(), Some(line)))
        }
    }
}

fn fallback_hostname() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.is_empty() {
            return h;
        }
    }
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        if !h.is_empty() {
            return h;
        }
    }
    "host".to_string()
}

/// 子プロセスとして自分自身を起動し、publish をそちらに任せる。
/// 親プロセスは本関数から返った直後に exit 0 する想定。
fn spawn_detached_publisher(
    host: &str,
    port: u16,
    topic: &str,
    retain: bool,
    payload: &str,
) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let mut cmd = ProcCommand::new(exe);
    cmd.env(CHILD_ENV, "1")
        .arg("-h")
        .arg(host)
        .arg("-p")
        .arg(port.to_string())
        .arg("-t")
        .arg(topic)
        .arg("--stdin")
        .arg("--timeout")
        .arg("1");

    if retain {
        cmd.arg("-r");
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Windows: DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP で親から切り離す
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload.as_bytes());
        // stdin を drop して子に EOF を通知
    }
    // child を drop する = wait しない。子は独立して走る。
    Ok(())
}

fn publish_with_timeout(
    host: &str,
    port: u16,
    topic: &str,
    retain: bool,
    payload: &str,
    timeout_secs: u64,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let host = host.to_string();
    let topic = topic.to_string();
    let payload = payload.to_string();

    thread::spawn(move || {
        let result = publish_message(&host, port, &topic, retain, &payload);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("connection timeout after {}s", timeout_secs))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("worker thread disconnected".to_string()),
    }
}

fn publish_message(
    host: &str,
    port: u16,
    topic: &str,
    retain: bool,
    payload: &str,
) -> Result<(), String> {
    let client_id = format!("mqtt-publish-{}", std::process::id());
    let mut options = MqttOptions::new(client_id, host, port);
    options.set_keep_alive(Duration::from_secs(5));

    let (client, mut connection) = Client::new(options, 10);

    client
        .publish(topic, QoS::AtMostOnce, retain, payload.as_bytes())
        .map_err(|e| format!("Failed to publish: {}", e))?;

    for notification in connection.iter() {
        match notification {
            Ok(rumqttc::Event::Outgoing(rumqttc::Outgoing::Publish(_))) => {
                break;
            }
            Ok(rumqttc::Event::Outgoing(rumqttc::Outgoing::Disconnect)) => {
                break;
            }
            Err(e) => {
                return Err(format!("Connection error: {}", e));
            }
            _ => {}
        }
    }

    let _ = client.disconnect();
    Ok(())
}

fn debug_log(msg: String) {
    if std::env::var("RUST_LOG").is_ok() {
        eprintln!("mqtt-publish: {}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_payload_shape() {
        let input = r#"{"session_id":"myhost-123","cwd":"/tmp/foo"}"#;
        let (topic, payload, extra) = build_event_payload(EventKind::Stop, input).unwrap();
        assert_eq!(topic, "claude-code/events/stop");
        assert!(extra.is_none());
        let v: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["event"], "stop");
        assert_eq!(v["session_id"], "myhost-123");
        assert_eq!(v["cwd"], "/tmp/foo");
    }

    #[test]
    fn test_permission_request_wraps_content() {
        let input = r#"{"session_id":"h-1","cwd":"/x","tool_name":"Bash"}"#;
        let (topic, payload, _) = build_event_payload(EventKind::PermissionRequest, input).unwrap();
        assert_eq!(topic, "claude-code/events/permission-request");
        let v: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["content"]["tool_name"], "Bash");
    }

    #[test]
    fn test_statusline_outputs_text_and_topic_per_session() {
        let input = r#"{"session_id":"abc","cwd":"/x","model":{"display_name":"Sonnet"},"cost":{"total_cost_usd":0.12,"total_lines_added":5,"total_lines_removed":2},"context_window":{"used_percentage":42.0}}"#;
        let (topic, payload, extra) = build_event_payload(EventKind::Statusline, input).unwrap();
        assert_eq!(topic, "claude-code/status/abc");
        assert!(extra.as_deref().unwrap().contains("[Sonnet]"));
        assert!(extra.as_deref().unwrap().contains("+5/-2"));
        let v: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["status"]["cost_usd"], 0.12);
    }

    #[test]
    fn test_fallback_when_session_id_missing() {
        let input = r#"{"cwd":"/x"}"#;
        let (_, payload, _) = build_event_payload(EventKind::Stop, input).unwrap();
        let v: Value = serde_json::from_str(&payload).unwrap();
        let sid = v["session_id"].as_str().unwrap();
        assert!(sid.ends_with("-unknown"));
    }
}
