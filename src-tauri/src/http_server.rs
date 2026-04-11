//! 導入用 HTTP サーバ
//!
//! Tauri アプリ内蔵の軽量 HTTP サーバ。WSL / Linux / macOS / Windows の
//! クライアント側が `curl http://<host>:1884/install.sh | bash` のワン
//! ライナーでインストーラを取得できるようにする。併せて `mqtt-publish`
//! バイナリも同じポートで配信する。
//!
//! ランタイム経路（MQTT publish/subscribe）はこのサーバを通らない。
//! このサーバは「導入」専用。

use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::templates;

pub const HTTP_PORT: u16 = 1884;
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
struct HttpState {
    mqtt_port: u16,
    app: AppHandle,
}

/// HTTP サーバを別スレッドで起動する
pub fn start(app: AppHandle, mqtt_port: u16) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                warn!("Failed to create HTTP server runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            let state = HttpState { mqtt_port, app };
            let router = Router::new()
                .route("/health", get(health_handler))
                .route("/install.sh", get(install_sh_handler))
                .route("/install.ps1", get(install_ps1_handler))
                .route("/uninstall.sh", get(uninstall_sh_handler))
                .route("/uninstall.ps1", get(uninstall_ps1_handler))
                .route("/bin/:name", get(serve_binary))
                .with_state(state);

            let addr = SocketAddr::from(([0, 0, 0, 0], HTTP_PORT));
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    info!("HTTP setup server listening on {}", addr);
                    if let Err(e) = axum::serve(listener, router).await {
                        warn!("HTTP server error: {}", e);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to bind HTTP server on {}: {} (is another instance running?)",
                        addr, e
                    );
                }
            }
        });
    });
}

async fn health_handler() -> Response<Body> {
    let body = json!({
        "status": "ok",
        "version": APP_VERSION,
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Host ヘッダからホスト名部分のみを取り出す
fn host_from_headers(headers: &HeaderMap) -> String {
    headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // IPv6 ([::1]:port) と IPv4/hostname (host:port) の両方に対応
            if let Some(stripped) = s.strip_prefix('[') {
                stripped.split(']').next().map(String::from)
            } else {
                Some(s.split(':').next().unwrap_or(s).to_string())
            }
        })
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

async fn install_sh_handler(State(state): State<HttpState>, headers: HeaderMap) -> Response<Body> {
    let host = host_from_headers(&headers);
    let body = templates::INSTALL_SH_HTTP
        .replace("__HOST__", &host)
        .replace("__PORT__", &state.mqtt_port.to_string())
        .replace("__HTTP_PORT__", &HTTP_PORT.to_string());
    text_response(body)
}

async fn install_ps1_handler(State(state): State<HttpState>, headers: HeaderMap) -> Response<Body> {
    let host = host_from_headers(&headers);
    let body = templates::INSTALL_PS1_HTTP
        .replace("__HOST__", &host)
        .replace("__PORT__", &state.mqtt_port.to_string())
        .replace("__HTTP_PORT__", &HTTP_PORT.to_string());
    text_response(body)
}

async fn uninstall_sh_handler() -> Response<Body> {
    text_response(templates::UNINSTALL_SH_HTTP.to_string())
}

async fn uninstall_ps1_handler() -> Response<Body> {
    text_response(templates::UNINSTALL_PS1_HTTP.to_string())
}

fn text_response(body: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

/// 許可されたバイナリ名のみを受け付ける
fn is_allowed_binary(name: &str) -> bool {
    matches!(
        name,
        "mqtt-publish-linux-x64"
            | "mqtt-publish-macos-arm64"
            | "mqtt-publish-macos-x64"
            | "mqtt-publish.exe"
    )
}

async fn serve_binary(
    State(state): State<HttpState>,
    AxumPath(name): AxumPath<String>,
) -> Response<Body> {
    if !is_allowed_binary(&name) {
        return not_found(format!(
            "Unknown binary: {}\nSupported: mqtt-publish-linux-x64, mqtt-publish-macos-arm64, mqtt-publish-macos-x64, mqtt-publish.exe\n",
            name
        ));
    }

    match load_binary(&state.app, &name) {
        Some(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", name),
            )
            .body(Body::from(bytes))
            .unwrap(),
        None => not_found(format!(
            "Binary not available on this server: {}\nThis build of Claude Code Notify does not bundle {}.\n",
            name, name
        )),
    }
}

fn not_found(body: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

/// バイナリを取得する
///
/// 優先順位:
///   1. Tauri のリソースディレクトリ `binaries/<name>` （リリースバンドル時）
///   2. 開発ビルド時のワークスペース `target/release` と `target/debug`
///   3. `src-tauri/binaries/<name>`（手動配置用）
fn load_binary(app: &AppHandle, name: &str) -> Option<Vec<u8>> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let path = resource_dir.join("binaries").join(name);
        if let Ok(data) = std::fs::read(&path) {
            return Some(data);
        }
    }

    let manifest = env!("CARGO_MANIFEST_DIR");
    let dev_file = if name == "mqtt-publish.exe" {
        "mqtt-publish.exe"
    } else {
        "mqtt-publish"
    };
    let candidates = [
        format!("{}/../target/release/{}", manifest, dev_file),
        format!("{}/../target/debug/{}", manifest, dev_file),
        format!("{}/binaries/{}", manifest, name),
    ];
    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_host_from_headers_hostname_with_port() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("example.com:1884"));
        assert_eq!(host_from_headers(&headers), "example.com");
    }

    #[test]
    fn test_host_from_headers_ipv4() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("192.168.1.100:1884"));
        assert_eq!(host_from_headers(&headers), "192.168.1.100");
    }

    #[test]
    fn test_host_from_headers_ipv6() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("[::1]:1884"));
        assert_eq!(host_from_headers(&headers), "::1");
    }

    #[test]
    fn test_host_from_headers_missing_defaults_to_loopback() {
        let headers = HeaderMap::new();
        assert_eq!(host_from_headers(&headers), "127.0.0.1");
    }

    #[test]
    fn test_is_allowed_binary() {
        assert!(is_allowed_binary("mqtt-publish-linux-x64"));
        assert!(is_allowed_binary("mqtt-publish-macos-arm64"));
        assert!(is_allowed_binary("mqtt-publish.exe"));
        assert!(!is_allowed_binary("../etc/passwd"));
        assert!(!is_allowed_binary("malicious"));
    }
}
