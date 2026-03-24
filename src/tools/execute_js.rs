use serde_json::Value;
use tauri::{AppHandle, Runtime};

use crate::desktop::get_emit_target;
use crate::error::Error;
use crate::socket_server::SocketResponse;
use crate::tools::webview::emit_and_wait;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecuteJsRequest {
    window_label: Option<String>,
    code: String,
    timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExecuteJsResponse {
    result: Value,
    #[serde(rename = "type")]
    result_type: String,
}

pub async fn handle_execute_js<R: Runtime>(
    app: &AppHandle<R>,
    payload: Value,
) -> Result<SocketResponse, Error> {
    let request: ExecuteJsRequest = serde_json::from_value(payload)
        .map_err(|e| Error::Anyhow(format!("Invalid payload for executeJs: {}", e)))?;

    let window_label = request
        .window_label
        .clone()
        .unwrap_or_else(|| "main".to_string());

    let _webview = crate::desktop::get_webview_for_eval(app, &window_label)
        .ok_or_else(|| Error::Anyhow(format!("Webview not found: {}", window_label)))?;

    let timeout_ms = request.timeout_ms.unwrap_or(30000);
    let emit_target = get_emit_target(app, &window_label);

    // Use emit_and_wait with correlation ID (fixes previous emit-before-listen race)
    match emit_and_wait(
        app,
        &emit_target,
        "execute-js",
        "execute-js-response",
        serde_json::json!(request.code),
        std::time::Duration::from_millis(timeout_ms),
    )
    .await
    {
        Ok(result_string) => {
            let response: Value = serde_json::from_str(&result_string)
                .map_err(|e| Error::Anyhow(format!("Failed to parse JS response: {}", e)))?;

            if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
                return Ok(SocketResponse {
                    success: false,
                    data: None,
                    error: Some(error.to_string()),
                    id: None,
                });
            }

            let result_str = response
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("[Result could not be stringified]");

            let result_type = response
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();

            let is_json = response
                .get("isJson")
                .and_then(|j| j.as_bool())
                .unwrap_or(false);

            // If JS side marked it as JSON, parse it to avoid double-encoding
            let result_value = if is_json {
                serde_json::from_str(result_str)
                    .unwrap_or_else(|_| Value::String(result_str.to_string()))
            } else {
                Value::String(result_str.to_string())
            };

            let data = serde_json::to_value(ExecuteJsResponse {
                result: result_value,
                result_type,
            })
            .map_err(|e| Error::Anyhow(format!("Failed to serialize response: {}", e)))?;

            Ok(SocketResponse {
                success: true,
                data: Some(data),
                error: None,
                id: None,
            })
        }
        Err(e) => Ok(SocketResponse {
            success: false,
            data: None,
            error: Some(format!("Timeout waiting for JS execution: {}", e)),
            id: None,
        }),
    }
}
