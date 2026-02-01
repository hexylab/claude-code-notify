//! mqtt-publish - Lightweight MQTT publish tool for Claude Code Notify
//!
//! Usage:
//!   mqtt-publish -h <host> -p <port> -t <topic> -m <message>
//!   mqtt-publish -h <host> -p <port> -t <topic> --stdin
//!
//! Example:
//!   mqtt-publish -h 192.168.1.100 -p 1883 -t "claude-code/events/stop" -m '{"event":"stop"}'

use clap::Parser;
use rumqttc::{Client, MqttOptions, QoS};
use std::io::{self, Read};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "mqtt-publish")]
#[command(version)]
#[command(about = "Publish MQTT messages for Claude Code Notify")]
struct Args {
    /// MQTT broker host
    #[arg(short = 'h', long, default_value = "127.0.0.1")]
    host: String,

    /// MQTT broker port
    #[arg(short = 'p', long, default_value_t = 1883)]
    port: u16,

    /// MQTT topic
    #[arg(short = 't', long)]
    topic: String,

    /// Message payload (mutually exclusive with --stdin)
    #[arg(short = 'm', long, conflicts_with = "stdin")]
    message: Option<String>,

    /// Read message from stdin
    #[arg(long)]
    stdin: bool,

    /// Retain message on broker
    #[arg(short = 'r', long, default_value_t = false)]
    retain: bool,

    /// Connection timeout in seconds
    #[arg(long, default_value_t = 5)]
    timeout: u64,
}

fn main() {
    let args = Args::parse();

    // Get message content
    let payload = if args.stdin {
        let mut buffer = Vec::new();
        if let Err(e) = io::stdin().read_to_end(&mut buffer) {
            eprintln!("Failed to read from stdin: {}", e);
            std::process::exit(1);
        }

        // Convert to UTF-8, stripping BOM if present (PowerShell adds UTF-8 BOM)
        let s = if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
            // UTF-8 BOM detected, skip it
            String::from_utf8_lossy(&buffer[3..]).to_string()
        } else if buffer.starts_with(&[0xFF, 0xFE]) {
            // UTF-16 LE BOM
            let utf16: Vec<u16> = buffer[2..].chunks(2)
                .filter_map(|c| if c.len() == 2 { Some(u16::from_le_bytes([c[0], c[1]])) } else { None })
                .collect();
            String::from_utf16_lossy(&utf16)
        } else {
            String::from_utf8_lossy(&buffer).to_string()
        };

        s.trim_end().to_string()
    } else if let Some(msg) = args.message {
        msg
    } else {
        eprintln!("Error: Either --message or --stdin must be provided");
        std::process::exit(1);
    };

    // Use channel to communicate between threads
    let (tx, rx) = mpsc::channel();
    let timeout_secs = args.timeout;
    let host = args.host.clone();
    let port = args.port;
    let topic = args.topic.clone();
    let retain = args.retain;

    // Spawn worker thread for MQTT operations
    thread::spawn(move || {
        let result = publish_message(&host, port, &topic, retain, &payload);
        let _ = tx.send(result);
    });

    // Wait for result with timeout
    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(())) => {
            // Success
        }
        Ok(Err(e)) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            eprintln!("Connection timeout: could not connect to broker within {} seconds", timeout_secs);
            std::process::exit(1);
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            eprintln!("Internal error: worker thread terminated unexpectedly");
            std::process::exit(1);
        }
    }
}

fn publish_message(host: &str, port: u16, topic: &str, retain: bool, payload: &str) -> Result<(), String> {
    // Create MQTT client with unique client ID
    let client_id = format!("mqtt-publish-{}", std::process::id());
    let mut options = MqttOptions::new(client_id, host, port);
    options.set_keep_alive(Duration::from_secs(5));

    let (client, mut connection) = Client::new(options, 10);

    // Publish message (QoS 0 = fire and forget, no need to wait for ack)
    client
        .publish(topic, QoS::AtMostOnce, retain, payload.as_bytes())
        .map_err(|e| format!("Failed to publish: {}", e))?;

    // Wait for publish to complete or connection error
    for notification in connection.iter() {
        match notification {
            Ok(rumqttc::Event::Outgoing(rumqttc::Outgoing::Publish(_))) => {
                // Message sent successfully
                break;
            }
            Ok(rumqttc::Event::Outgoing(rumqttc::Outgoing::Disconnect)) => {
                break;
            }
            Err(e) => {
                return Err(format!("Connection error: {}", e));
            }
            _ => {
                // Continue waiting for publish confirmation
            }
        }
    }

    // Graceful disconnect
    let _ = client.disconnect();

    Ok(())
}
