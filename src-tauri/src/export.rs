//! Configuration Export module
//!
//! Generates setup files for Claude Code integration.

use crate::templates;
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};
use thiserror::Error;
use zip::{write::SimpleFileOptions, ZipWriter};

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("Failed to detect IP address: {0}")]
    IpDetection(String),
    #[error("Failed to create ZIP: {0}")]
    ZipCreation(String),
}

/// MQTT client types supported for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientType {
    MosquittoPub,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub host: String,
    pub port: u16,
    pub client_type: ClientType,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 1883,
            client_type: ClientType::MosquittoPub,
        }
    }
}

/// Detect local IP address
pub fn detect_local_ip() -> Result<String, ExportError> {
    local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| ExportError::IpDetection(e.to_string()))
}

/// Generate export ZIP file in memory
pub fn generate_export_zip(config: &ExportConfig) -> Result<Vec<u8>, ExportError> {
    let mut buffer = Cursor::new(Vec::new());

    {
        let mut zip = ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // on-stop.sh
        let on_stop = templates::ON_STOP_SH
            .replace("__HOST__", &config.host)
            .replace("__PORT__", &config.port.to_string());

        zip.start_file("on-stop.sh", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(on_stop.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        // on-permission-request.sh
        let on_permission_request = templates::ON_PERMISSION_REQUEST_SH
            .replace("__HOST__", &config.host)
            .replace("__PORT__", &config.port.to_string());

        zip.start_file("on-permission-request.sh", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(on_permission_request.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        // on-notification.sh
        let on_notification = templates::ON_NOTIFICATION_SH
            .replace("__HOST__", &config.host)
            .replace("__PORT__", &config.port.to_string());

        zip.start_file("on-notification.sh", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(on_notification.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        // statusline.sh (optional, for users who want real-time status)
        let statusline = templates::STATUSLINE_SH
            .replace("__HOST__", &config.host)
            .replace("__PORT__", &config.port.to_string());

        zip.start_file("statusline.sh", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(statusline.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        // install.sh - Automated installer
        let installer = templates::INSTALL_SH
            .replace("__HOST__", &config.host)
            .replace("__PORT__", &config.port.to_string());

        zip.start_file("install.sh", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(installer.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        // hooks-settings-snippet.json (for manual setup reference)
        let settings = templates::CLAUDE_SETTINGS_SNIPPET;
        zip.start_file("hooks-settings-snippet.json", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(settings.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        // README.txt
        let readme = templates::README_TEMPLATE
            .replace("__HOST__", &config.host)
            .replace("__PORT__", &config.port.to_string());

        zip.start_file("README.txt", options)
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
        zip.write_all(readme.as_bytes())
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;

        zip.finish()
            .map_err(|e| ExportError::ZipCreation(e.to_string()))?;
    }

    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_zip() {
        let config = ExportConfig {
            host: "192.168.1.100".to_string(),
            port: 1883,
            client_type: ClientType::MosquittoPub,
        };

        let result = generate_export_zip(&config);
        assert!(result.is_ok());

        let zip_data = result.unwrap();
        assert!(!zip_data.is_empty());
    }
}
