//! MQTT Client module using rumqttc
//!
//! This module provides an async MQTT client for subscribing to
//! Claude Code notifications and publishing status updates.

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// MQTT Topics for Claude Code notifications
pub mod topics {
    pub const ALL: &str = "claude-code/#";
    pub const TASK_COMPLETE: &str = "claude-code/task/complete";
    pub const ERROR: &str = "claude-code/error";
    pub const STATUS: &str = "claude-code/status";
    /// Stop event from Claude Code hooks
    pub const EVENTS_STOP: &str = "claude-code/events/stop";
    /// Permission request event from Claude Code hooks (approval requests)
    pub const EVENTS_PERMISSION_REQUEST: &str = "claude-code/events/permission-request";
    /// Notification event from Claude Code hooks (elicitation dialogs, etc.)
    pub const EVENTS_NOTIFICATION: &str = "claude-code/events/notification";
    /// Status updates from Claude Code statusline (prefix for session-specific topics)
    pub const STATUS_PREFIX: &str = "claude-code/status/";
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ClientError {
    #[error("Connection error: {0}")]
    Connection(#[from] rumqttc::ClientError),

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,
}

/// Message received from MQTT broker
#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

impl MqttMessage {
    pub fn payload_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }
}

/// Start MQTT client and return a receiver for incoming messages
pub fn start_mqtt_client(client_id: &str) -> (AsyncClient, mpsc::Receiver<MqttMessage>) {
    let mut options = MqttOptions::new(client_id, "127.0.0.1", 1883);
    options.set_keep_alive(Duration::from_secs(30));
    options.set_clean_session(true);

    let (client, eventloop) = AsyncClient::new(options, 100);
    let (tx, rx) = mpsc::channel(100);

    let client_clone = client.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async move {
            run_event_loop(client_clone, eventloop, tx).await;
        });
    });

    (client, rx)
}

async fn run_event_loop(
    client: AsyncClient,
    mut eventloop: EventLoop,
    tx: mpsc::Sender<MqttMessage>,
) {
    // Subscribe to topics after connection
    let mut subscribed = false;

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                info!("Connected to MQTT broker");
                if !subscribed {
                    info!("Subscribing to topic: {}", topics::ALL);
                    // Use QoS 0 (AtMostOnce) to prevent duplicate notifications
                    if let Err(e) = client.subscribe(topics::ALL, QoS::AtMostOnce).await {
                        error!("Failed to subscribe: {:?}", e);
                    }
                }
            }
            Ok(Event::Incoming(Packet::SubAck(_))) => {
                info!("Subscription confirmed");
                subscribed = true;
            }
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                let msg = MqttMessage {
                    topic: publish.topic.clone(),
                    payload: publish.payload.to_vec(),
                };
                debug!("Received message on topic: {}", msg.topic);

                if tx.send(msg).await.is_err() {
                    warn!("Message receiver dropped, stopping event loop");
                    break;
                }
            }
            Ok(_) => {}
            Err(e) => {
                error!("MQTT event loop error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topics() {
        assert_eq!(topics::ALL, "claude-code/#");
        assert_eq!(topics::TASK_COMPLETE, "claude-code/task/complete");
    }
}
