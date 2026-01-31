//! MQTT Broker module using rumqttd
//!
//! This module provides an embedded MQTT broker for local communication
//! between Claude Code instances and the notification system.

use rumqttd::{Broker, Config};
use std::thread;
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum BrokerError {
    #[error("Failed to load configuration: {0}")]
    ConfigLoad(String),

    #[error("Failed to start broker: {0}")]
    StartError(String),
}

/// MQTT Broker wrapper for embedded usage
pub struct MqttBroker {
    config: Config,
    handle: Option<thread::JoinHandle<()>>,
}

impl MqttBroker {
    /// Create a new MQTT broker with default embedded configuration
    pub fn with_default_config() -> Result<Self, BrokerError> {
        let toml_config = include_str!("../config/rumqttd.toml");

        let config: Config = toml::from_str(toml_config)
            .map_err(|e| BrokerError::ConfigLoad(e.to_string()))?;

        Ok(Self {
            config,
            handle: None,
        })
    }

    /// Start the broker in a background thread
    pub fn start(&mut self) -> Result<(), BrokerError> {
        info!("Starting MQTT broker on port 1883...");

        let config = self.config.clone();

        let handle = thread::spawn(move || {
            let mut broker = Broker::new(config);
            if let Err(e) = broker.start() {
                error!("Broker error: {:?}", e);
            }
        });

        self.handle = Some(handle);
        info!("MQTT broker started successfully");

        Ok(())
    }

    /// Check if the broker is running
    pub fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for MqttBroker {
    fn drop(&mut self) {
        info!("Shutting down MQTT broker...");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let result = MqttBroker::with_default_config();
        assert!(result.is_ok());
    }
}
