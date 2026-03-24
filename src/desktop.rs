use crate::error::Error;
use crate::models::*;
use crate::native_input::{self, TextParams};
use crate::shared::ScreenshotParams;
use crate::socket_server::SocketServer;
use crate::tools::mouse_movement;
use crate::{PluginConfig, Result};
use log::info;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime, plugin::PluginApi};

// ----- Webview Fallback Config -----

/// Stores the configured fallback webview label, managed as Tauri state.
pub struct WebviewFallbackConfig {
    pub label: Option<String>,
}

// ----- Window/Webview Resolution Helpers -----

/// Represents either a WebviewWindow or a separate Window handle
pub enum WindowHandle<R: Runtime> {
    WebviewWindow(tauri::WebviewWindow<R>),
    Window(tauri::Window<R>),
}

impl<R: Runtime> WindowHandle<R> {
    pub fn minimize(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.minimize(),
            WindowHandle::Window(w) => w.minimize(),
        }
    }

    pub fn maximize(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.maximize(),
            WindowHandle::Window(w) => w.maximize(),
        }
    }

    pub fn unmaximize(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.unmaximize(),
            WindowHandle::Window(w) => w.unmaximize(),
        }
    }

    pub fn close(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.close(),
            WindowHandle::Window(w) => w.close(),
        }
    }

    pub fn show(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.show(),
            WindowHandle::Window(w) => w.show(),
        }
    }

    pub fn hide(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.hide(),
            WindowHandle::Window(w) => w.hide(),
        }
    }

    pub fn set_focus(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.set_focus(),
            WindowHandle::Window(w) => w.set_focus(),
        }
    }

    pub fn set_position(
        &self,
        pos: tauri::LogicalPosition<f64>,
    ) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.set_position(pos),
            WindowHandle::Window(w) => w.set_position(pos),
        }
    }

    pub fn set_size(&self, size: tauri::LogicalSize<f64>) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.set_size(size),
            WindowHandle::Window(w) => w.set_size(size),
        }
    }

    pub fn center(&self) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.center(),
            WindowHandle::Window(w) => w.center(),
        }
    }

    pub fn set_fullscreen(&self, fullscreen: bool) -> std::result::Result<(), tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.set_fullscreen(fullscreen),
            WindowHandle::Window(w) => w.set_fullscreen(fullscreen),
        }
    }

    pub fn is_fullscreen(&self) -> std::result::Result<bool, tauri::Error> {
        match self {
            WindowHandle::WebviewWindow(w) => w.is_fullscreen(),
            WindowHandle::Window(w) => w.is_fullscreen(),
        }
    }
}

/// Get a window handle by label, supporting both WebviewWindow and Window architectures.
/// First tries exact label match, then falls back to matching by window title.
pub fn get_window_handle<R: Runtime>(app: &AppHandle<R>, label: &str) -> Option<WindowHandle<R>> {
    // First try WebviewWindow by exact label (combined window+webview)
    if let Some(ww) = app.get_webview_window(label) {
        return Some(WindowHandle::WebviewWindow(ww));
    }
    // Try separate Window by exact label (multi-webview architecture)
    if let Some(w) = app.get_window(label) {
        return Some(WindowHandle::Window(w));
    }
    // Fall back to matching by window title (case-insensitive)
    let label_lower = label.to_lowercase();
    for ww in app.webview_windows().values() {
        if let Ok(title) = ww.title() {
            if title.to_lowercase() == label_lower || title.to_lowercase().contains(&label_lower) {
                return Some(WindowHandle::WebviewWindow(ww.clone()));
            }
        }
    }
    for w in app.windows().values() {
        if let Ok(title) = w.title() {
            if title.to_lowercase() == label_lower || title.to_lowercase().contains(&label_lower) {
                return Some(WindowHandle::Window(w.clone()));
            }
        }
    }
    None
}

/// Get a webview for JS execution and DOM access.
/// Supports both architectures:
/// - WebviewWindow: returns the webview directly
/// - Multi-webview: falls back to the configured `default_webview_label`
/// Also falls back to matching by window title if exact label fails.
pub fn get_webview_for_eval<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
) -> Option<tauri::Webview<R>> {
    // First try WebviewWindow with exact label (returns its inner webview)
    if let Some(ww) = app.get_webview_window(label) {
        return Some(ww.as_ref().clone());
    }
    // Multi-webview architecture: use the configured fallback webview label
    if let Some(config) = app.try_state::<WebviewFallbackConfig>() {
        if let Some(fallback) = &config.label {
            if let Some(wv) = app.get_webview(fallback) {
                return Some(wv);
            }
        }
    }
    // Try direct webview lookup
    if let Some(wv) = app.get_webview(label) {
        return Some(wv);
    }
    // Fall back to matching by window title (case-insensitive)
    let label_lower = label.to_lowercase();
    for ww in app.webview_windows().values() {
        if let Ok(title) = ww.title() {
            if title.to_lowercase() == label_lower || title.to_lowercase().contains(&label_lower) {
                return Some(ww.as_ref().clone());
            }
        }
    }
    None
}

/// Get the emit target label for multi-webview architecture.
/// If the window label doesn't match a WebviewWindow, falls back to the
/// configured `default_webview_label` from `PluginConfig`.
pub fn get_emit_target<R: Runtime>(app: &AppHandle<R>, window_label: &str) -> String {
    if app.get_webview_window(window_label).is_none() {
        if let Some(config) = app.try_state::<WebviewFallbackConfig>() {
            if let Some(fallback) = &config.label {
                if app.get_webview(fallback).is_some() {
                    return fallback.to_string();
                }
            }
        }
    }
    window_label.to_string()
}

// ----- Screenshot Utilities -----

/// Helper structure to hold window for screenshot functions.
/// Supports both WebviewWindow and Window architectures.
pub struct ScreenshotContext<R: Runtime> {
    pub window_handle: WindowHandle<R>,
}

/// Create a success response with data
pub fn create_success_response(data_url: String) -> ScreenshotResponse {
    ScreenshotResponse {
        data: Some(data_url),
        success: true,
        error: None,
        file_path: None,
    }
}

/// Create an error response
pub fn create_error_response(error_msg: String) -> ScreenshotResponse {
    ScreenshotResponse {
        data: None,
        success: false,
        error: Some(error_msg),
        file_path: None,
    }
}

// ----- TauriMcp Implementation -----

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
    config: &PluginConfig,
) -> crate::Result<TauriMcp<R>> {
    // Store webview fallback config as managed state for resolution helpers
    app.manage(WebviewFallbackConfig {
        label: config.default_webview_label.clone(),
    });

    // Register virtual cursor state for native input injection
    app.manage(crate::native_input::state::VirtualCursorState::new());

    let socket_server = if config.start_socket_server {
        let mut server = SocketServer::new(
            app.clone(),
            config.socket_type.clone(),
            config.auth_token.clone(),
        );
        server.start()?;
        Some(Arc::new(Mutex::new(server)))
    } else {
        None
    };

    Ok(TauriMcp {
        app: app.clone(),
        socket_server,
        application_name: config.application_name.clone(),
    })
}

/// Access to the tauri-mcp APIs.
pub struct TauriMcp<R: Runtime> {
    app: AppHandle<R>,
    socket_server: Option<Arc<Mutex<SocketServer<R>>>>,
    application_name: String,
}

impl<R: Runtime> TauriMcp<R> {
    pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
        Ok(PingResponse {
            value: payload.value,
        })
    }

    // Take screenshot - this feature depends on Tauri's window capabilities.
    // On Linux, falls back to webview-based JS capture if xcap fails.
    pub async fn take_screenshot_async(
        &self,
        payload: ScreenshotRequest,
    ) -> crate::Result<ScreenshotResponse> {
        let window_label = payload.window_label.clone();

        // Get window handle - supports both WebviewWindow and Window architectures
        let window_handle = get_window_handle(&self.app, &window_label)
            .ok_or_else(|| Error::WindowNotFound(window_label.clone()))?;

        // Create shared parameters struct from the request
        let params = ScreenshotParams {
            window_label: Some(window_label.clone()),
            quality: payload.quality,
            max_width: payload.max_width,
            max_size_mb: payload.max_size_mb,
            application_name: Some(self.application_name.clone()),
            output_dir: payload.output_dir,
            save_to_disk: payload.save_to_disk,
            thumbnail: payload.thumbnail,
        };

        // Create a context with the window handle for platform implementation
        let window_context = ScreenshotContext { window_handle };

        info!("[TAURI_MCP] Taking screenshot with default parameters");

        // Use platform-specific implementation to capture the window
        let result = crate::platform::current::take_screenshot(params.clone(), window_context).await;

        // On Linux, if xcap failed, fall back to JS-based webview capture
        #[cfg(target_os = "linux")]
        if let Err(ref _e) = result {
            info!("[TAURI_MCP] xcap screenshot failed, trying JS-based webview capture fallback");
            if let Some(fallback) = self.take_screenshot_via_js(&window_label, &params).await {
                return Ok(fallback);
            }
        }

        result
    }

    /// Fallback: capture screenshot via JavaScript in the webview.
    /// Uses canvas toDataURL to capture the visible page content.
    #[cfg(target_os = "linux")]
    async fn take_screenshot_via_js(
        &self,
        window_label: &str,
        params: &ScreenshotParams,
    ) -> Option<ScreenshotResponse> {
        use crate::tools::webview::emit_and_wait;
        use base64::Engine;

        let emit_target = get_emit_target(&self.app, window_label);
        let quality = params.quality.unwrap_or(70);
        let max_width = params.max_width.unwrap_or(1400);

        // JS code that captures the visible viewport as a JPEG data URL
        let js_code = format!(
            r#"(async () => {{
                const w = document.documentElement.scrollWidth;
                const h = document.documentElement.scrollHeight;
                const vw = window.innerWidth;
                const vh = window.innerHeight;
                const canvas = document.createElement('canvas');
                const scale = Math.min(1, {max_width} / vw);
                canvas.width = Math.round(vw * scale);
                canvas.height = Math.round(vh * scale);
                const ctx = canvas.getContext('2d');
                ctx.scale(scale, scale);
                // Render all visible elements by cloning DOM into SVG foreignObject
                const html = document.documentElement.outerHTML;
                const blob = new Blob([`
                    <svg xmlns="http://www.w3.org/2000/svg" width="${{vw}}" height="${{vh}}">
                        <foreignObject width="100%" height="100%">
                            ${{new XMLSerializer().serializeToString(document.documentElement)}}
                        </foreignObject>
                    </svg>
                `], {{type: 'image/svg+xml'}});
                const url = URL.createObjectURL(blob);
                const img = new Image();
                await new Promise((resolve, reject) => {{
                    img.onload = resolve;
                    img.onerror = reject;
                    img.src = url;
                }});
                ctx.drawImage(img, 0, 0);
                URL.revokeObjectURL(url);
                return canvas.toDataURL('image/jpeg', {quality_f});
            }})()"#,
            max_width = max_width,
            quality_f = quality as f64 / 100.0
        );

        let result = emit_and_wait(
            &self.app,
            &emit_target,
            "execute-js",
            "execute-js-response",
            serde_json::json!(js_code),
            Duration::from_secs(15),
        )
        .await;

        match result {
            Ok(response_str) => {
                let response: serde_json::Value =
                    serde_json::from_str(&response_str).ok()?;
                let data_url = response.get("result")?.as_str()?;

                if !data_url.starts_with("data:image/") {
                    info!("[TAURI_MCP] JS fallback returned non-image data");
                    return None;
                }

                info!("[TAURI_MCP] JS-based screenshot captured successfully");

                let save_to_disk = params.save_to_disk.unwrap_or(false);
                let thumbnail = params.thumbnail.unwrap_or(false);

                if save_to_disk {
                    // Extract base64 and save to file
                    let b64 = data_url.split(',').nth(1)?;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .ok()?;

                    let output_dir = params.output_dir.clone().unwrap_or_else(|| {
                        std::env::temp_dir()
                            .join("tauri-mcp-screenshots")
                            .to_string_lossy()
                            .to_string()
                    });
                    std::fs::create_dir_all(&output_dir).ok()?;
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let file_path = format!("{}/screenshot_{}.jpg", output_dir, timestamp);
                    std::fs::write(&file_path, &bytes).ok()?;

                    if thumbnail {
                        // Return both file path and thumbnail inline
                        Some(ScreenshotResponse {
                            data: Some(data_url.to_string()),
                            success: true,
                            error: None,
                            file_path: Some(file_path),
                        })
                    } else {
                        Some(ScreenshotResponse {
                            data: None,
                            success: true,
                            error: None,
                            file_path: Some(file_path),
                        })
                    }
                } else {
                    Some(ScreenshotResponse {
                        data: Some(data_url.to_string()),
                        success: true,
                        error: None,
                        file_path: None,
                    })
                }
            }
            Err(e) => {
                info!("[TAURI_MCP] JS-based screenshot fallback also failed: {}", e);
                None
            }
        }
    }

    // Add async method to perform window operations
    pub async fn manage_window_async(
        &self,
        params: WindowManagerRequest,
    ) -> Result<WindowManagerResponse> {
        let window_label = params.window_label.unwrap_or_else(|| "main".to_string());

        // Get the window by label - supports both WebviewWindow and Window architectures
        let window = get_window_handle(&self.app, &window_label).ok_or_else(|| {
            Error::WindowOperationFailed(format!("Window not found: {}", window_label))
        })?;

        // Execute the requested operation using WindowHandle methods
        match params.operation.as_str() {
            "minimize" => {
                window.minimize()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "maximize" => {
                window.maximize()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "unmaximize" => {
                window.unmaximize()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "close" => {
                window.close()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "show" => {
                window.show()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "hide" => {
                window.hide()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "setPosition" => {
                if let (Some(x), Some(y)) = (params.x, params.y) {
                    window.set_position(tauri::LogicalPosition::new(x as f64, y as f64))?;
                    Ok(WindowManagerResponse {
                        success: true,
                        error: None,
                    })
                } else {
                    Err(Error::WindowOperationFailed(
                        "setPosition requires x and y coordinates".to_string(),
                    ))
                }
            }
            "setSize" => {
                if let (Some(width), Some(height)) = (params.width, params.height) {
                    window.set_size(tauri::LogicalSize::new(width as f64, height as f64))?;
                    Ok(WindowManagerResponse {
                        success: true,
                        error: None,
                    })
                } else {
                    Err(Error::WindowOperationFailed(
                        "setSize requires width and height parameters".to_string(),
                    ))
                }
            }
            "center" => {
                window.center()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "toggleFullscreen" => {
                let is_fullscreen = window.is_fullscreen()?;
                window.set_fullscreen(!is_fullscreen)?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            "focus" => {
                window.set_focus()?;
                Ok(WindowManagerResponse {
                    success: true,
                    error: None,
                })
            }
            _ => Err(Error::WindowOperationFailed(format!(
                "Unknown window operation: {}",
                params.operation
            ))),
        }
    }

    // Text input simulation via native event injection (no Accessibility permissions needed)
    pub async fn simulate_text_input_async(
        &self,
        params: TextInputRequest,
    ) -> crate::Result<TextInputResponse> {
        let text = params.text;
        let delay_ms = params.delay_ms.unwrap_or(20);
        let initial_delay_ms = params.initial_delay_ms.unwrap_or(500);
        let window_label = params.window_label.as_deref().unwrap_or("main");

        // Resolve the webview for native event injection
        let webview = get_webview_for_eval(&self.app, window_label)
            .ok_or_else(|| Error::Anyhow(format!("Webview not found: {}", window_label)))?;

        // Initial delay before typing
        if initial_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(initial_delay_ms)).await;
        }

        let start_time = Instant::now();

        let text_params = TextParams {
            text: text.clone(),
            delay_ms,
        };

        let result = native_input::backend::inject_text(&webview, &text_params)
            .map_err(|e| Error::Anyhow(format!("Native text injection failed: {}", e)))?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(TextInputResponse {
            chars_typed: result.chars_typed,
            duration_ms,
        })
    }

    // Mouse movement simulation
    pub async fn simulate_mouse_movement_async(
        &self,
        params: MouseMovementRequest,
    ) -> crate::Result<MouseMovementResponse> {
        mouse_movement::simulate_mouse_movement_async(&self.app, params).await
    }
}

impl<R: Runtime> Drop for TauriMcp<R> {
    fn drop(&mut self) {
        if let Some(server) = &self.socket_server {
            if let Ok(mut server) = server.lock() {
                let _ = server.stop();
            }
        }
    }
}
