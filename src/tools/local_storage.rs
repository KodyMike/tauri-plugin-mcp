use serde_json::Value;
use tauri::{AppHandle, Runtime};

use crate::desktop::get_emit_target;
use crate::error::Error;
use crate::models::LocalStorageRequest;
use crate::socket_server::SocketResponse;
use crate::tools::webview::emit_and_wait;

pub async fn handle_get_local_storage<R: Runtime>(
    app: &AppHandle<R>,
    payload: Value,
) -> Result<SocketResponse, Error> {
    let params: LocalStorageRequest = serde_json::from_value(payload)
        .map_err(|e| Error::Anyhow(format!("Invalid payload for localStorage: {}", e)))?;

    // Validate input parameters
    match params.action.as_str() {
        "get" => {}
        "remove" => {
            if params.key.is_none() {
                return Ok(SocketResponse {
                    success: false,
                    data: None,
                    error: Some("Key is required for remove operations".to_string()),
                    id: None,
                });
            }
        }
        "set" => {
            if params.key.is_none() || params.value.is_none() {
                return Ok(SocketResponse {
                    success: false,
                    data: None,
                    error: Some("Both key and value are required for set operation".to_string()),
                    id: None,
                });
            }
        }
        "clear" | "keys" => {}
        _ => {
            return Ok(SocketResponse {
                success: false,
                data: None,
                error: Some(format!(
                    "Unsupported localStorage action: {}",
                    params.action
                )),
                id: None,
            });
        }
    };

    let window_label = params
        .window_label
        .clone()
        .unwrap_or_else(|| "main".to_string());
    let _webview = crate::desktop::get_webview_for_eval(app, &window_label)
        .ok_or_else(|| Error::Anyhow(format!("Webview not found: {}", window_label)))?;

    let emit_target = get_emit_target(app, &window_label);

    let payload_value = serde_json::to_value(&params)
        .map_err(|e| Error::Anyhow(format!("Failed to serialize params: {}", e)))?;

    // Use emit_and_wait with correlation ID (fixes previous emit-before-listen race)
    match emit_and_wait(
        app,
        &emit_target,
        "get-local-storage",
        "get-local-storage-response",
        payload_value,
        std::time::Duration::from_secs(5),
    )
    .await
    {
        Ok(result_string) => {
            let response: Value = serde_json::from_str(&result_string).map_err(|e| {
                Error::Anyhow(format!("Failed to parse localStorage response: {}", e))
            })?;

            if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
                return Ok(SocketResponse {
                    success: false,
                    data: None,
                    error: Some(error.to_string()),
                    id: None,
                });
            }

            let data = response.get("data").cloned().unwrap_or(Value::Null);
            Ok(SocketResponse {
                success: true,
                data: Some(
                    serde_json::to_value(data).map_err(|e| {
                        Error::Anyhow(format!("Failed to serialize response: {}", e))
                    })?,
                ),
                error: None,
                id: None,
            })
        }
        Err(e) => Ok(SocketResponse {
            success: false,
            data: None,
            error: Some(format!("Timeout waiting for localStorage response: {}", e)),
            id: None,
        }),
    }
}
