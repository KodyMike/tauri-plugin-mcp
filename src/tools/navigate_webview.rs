use serde::Deserialize;
use serde_json::Value;
use std::sync::mpsc;
use tauri::{AppHandle, Emitter, Listener, Runtime};

use crate::desktop::{get_emit_target, get_webview_for_eval};
use crate::socket_server::SocketResponse;

#[derive(Debug, Deserialize)]
struct NavigatePayload {
    window_label: Option<String>,
    action: String,
    url: Option<String>,
}

/// Handler for navigate_webview — URL navigation, reload, back/forward
pub async fn handle_navigate_webview<R: Runtime>(
    app: &AppHandle<R>,
    payload: Value,
) -> Result<SocketResponse, crate::error::Error> {
    let parsed: NavigatePayload = serde_json::from_value(payload).map_err(|e| {
        crate::error::Error::Anyhow(format!("Invalid payload for navigate_webview: {}", e))
    })?;

    let window_label = parsed.window_label.unwrap_or_else(|| "main".to_string());
    let webview = get_webview_for_eval(app, &window_label).ok_or_else(|| {
        crate::error::Error::Anyhow(format!("Webview not found: {}", window_label))
    })?;

    match parsed.action.as_str() {
        "navigate" => {
            let url = parsed.url.ok_or_else(|| {
                crate::error::Error::Anyhow("'url' is required for navigate action".to_string())
            })?;
            let parsed_url: tauri::Url = url.parse().map_err(|e| {
                crate::error::Error::Anyhow(format!("Invalid URL '{}': {}", url, e))
            })?;
            webview.navigate(parsed_url).map_err(|e| {
                crate::error::Error::Anyhow(format!("Failed to navigate: {}", e))
            })?;
            Ok(SocketResponse {
                success: true,
                data: Some(serde_json::json!({"action": "navigate", "url": url})),
                error: None,
                id: None,
            })
        }
        "reload" => {
            webview.eval("location.reload()").map_err(|e| {
                crate::error::Error::Anyhow(format!("Failed to reload: {}", e))
            })?;
            Ok(SocketResponse {
                success: true,
                data: Some(serde_json::json!({"action": "reload"})),
                error: None,
                id: None,
            })
        }
        "get_url" => {
            let url = webview.url().map(|u| u.to_string()).unwrap_or_default();
            Ok(SocketResponse {
                success: true,
                data: Some(serde_json::json!({"url": url})),
                error: None,
                id: None,
            })
        }
        "back" | "forward" => {
            let emit_target = get_emit_target(app, &window_label);
            let (tx, rx) = mpsc::channel();

            app.once("navigate-webview-response", move |event| {
                let _ = tx.send(event.payload().to_string());
            });

            let js_payload = serde_json::json!({
                "action": parsed.action,
            });

            app.emit_to(&emit_target, "navigate-webview", js_payload)
                .map_err(|e| {
                    crate::error::Error::Anyhow(format!(
                        "Failed to emit navigate-webview event: {}",
                        e
                    ))
                })?;

            match rx.recv_timeout(std::time::Duration::from_secs(5)) {
                Ok(result) => Ok(crate::tools::webview::parse_js_response(&result)),
                Err(e) => Ok(SocketResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Timeout waiting for navigation: {}", e)),
                    id: None,
                }),
            }
        }
        _ => Ok(SocketResponse {
            success: false,
            data: None,
            error: Some(format!(
                "Unknown action '{}'. Valid actions: navigate, reload, get_url, back, forward",
                parsed.action
            )),
            id: None,
        }),
    }
}
