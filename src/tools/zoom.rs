use serde::Deserialize;
use serde_json::Value;
use std::sync::mpsc;
use tauri::{AppHandle, Emitter, Listener, Runtime};

use crate::desktop::{get_emit_target, get_webview_for_eval};
use crate::socket_server::SocketResponse;

#[derive(Debug, Deserialize)]
struct ZoomPayload {
    window_label: Option<String>,
    action: String,
    scale: Option<f64>,
}

/// Handler for manage_zoom — get/set webview zoom level
pub async fn handle_manage_zoom<R: Runtime>(
    app: &AppHandle<R>,
    payload: Value,
) -> Result<SocketResponse, crate::error::Error> {
    let parsed: ZoomPayload = serde_json::from_value(payload).map_err(|e| {
        crate::error::Error::Anyhow(format!("Invalid payload for manage_zoom: {}", e))
    })?;

    let window_label = parsed.window_label.unwrap_or_else(|| "main".to_string());
    let webview = get_webview_for_eval(app, &window_label).ok_or_else(|| {
        crate::error::Error::Anyhow(format!("Webview not found: {}", window_label))
    })?;

    match parsed.action.as_str() {
        "set" => {
            let scale = parsed.scale.ok_or_else(|| {
                crate::error::Error::Anyhow("'scale' is required for set action".to_string())
            })?;
            webview.set_zoom(scale).map_err(|e| {
                crate::error::Error::Anyhow(format!("Failed to set zoom: {}", e))
            })?;
            Ok(SocketResponse {
                success: true,
                data: Some(serde_json::json!({"action": "set", "scale": scale})),
                error: None,
                id: None,
            })
        }
        "get" => {
            let emit_target = get_emit_target(app, &window_label);
            let (tx, rx) = mpsc::channel();

            app.once("manage-zoom-response", move |event| {
                let _ = tx.send(event.payload().to_string());
            });

            app.emit_to(
                &emit_target,
                "manage-zoom",
                serde_json::json!({"action": "get"}),
            )
            .map_err(|e| {
                crate::error::Error::Anyhow(format!("Failed to emit manage-zoom event: {}", e))
            })?;

            match rx.recv_timeout(std::time::Duration::from_secs(5)) {
                Ok(result) => Ok(crate::tools::webview::parse_js_response(&result)),
                Err(e) => Ok(SocketResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Timeout waiting for zoom level: {}", e)),
                    id: None,
                }),
            }
        }
        _ => Ok(SocketResponse {
            success: false,
            data: None,
            error: Some(format!(
                "Unknown action '{}'. Valid actions: set, get",
                parsed.action
            )),
            id: None,
        }),
    }
}
