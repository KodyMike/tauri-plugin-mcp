# Tauri MCP Debug Setup

Allows AI agents (Claude Code) to inspect and interact with the running Raymalian app via the tauri-plugin-mcp MCP server.

## Architecture

```
Claude Code (AI agent)
    | MCP protocol (stdio)
tauri-mcp-server (~/.local/bin/tauri-mcp-server)
    | IPC Unix socket (/run/user/<uid>/tauri-mcp.sock)
tauri-plugin-mcp (Rust, embedded in app)
    | Tauri events + webview eval
Guest JS (imported in main.ts)
    | DOM APIs
Raymalian App
```

## Components

### 1. Rust Plugin (Cargo dependency)

In `raymalian-tauri/src-tauri/Cargo.toml`:

```toml
tauri = { version = "2", features = ["image-png", "unstable"] }
tauri-plugin-mcp = { path = "../../../tauri-plugin-mcp", optional = true }

[features]
default = []
mcp-debug = ["tauri-plugin-mcp"]
```

The `unstable` tauri feature is required by the plugin.

### 2. Plugin Registration (lib.rs)

Registered on the builder chain (not in setup) so `on_page_load` hooks fire before the webview loads:

```rust
let mut builder = tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_dialog::init());
#[cfg(feature = "mcp-debug")]
{
    builder = builder.plugin(tauri_plugin_mcp::init_with_config(
        tauri_plugin_mcp::PluginConfig::new("Raymalian".to_string()),
    ));
}
builder.setup(|app| { /* ... */ })
```

Key: use `PluginConfig::new()` not `PluginConfig::default()` — the default has `start_socket_server: false`.

### 3. Guest JS (frontend)

The plugin's guest JS registers Tauri event listeners in the webview. Without it, all webview tools (execute_js, query_page map/state, click, etc.) time out.

```bash
# Build the plugin's guest JS (one-time, or after plugin source changes)
cd /path/to/tauri-plugin-mcp && npm install && npm run build
```

In `raymalian-tauri/ui/`:
```bash
npm install --save /path/to/tauri-plugin-mcp
```

In `raymalian-tauri/ui/src/main.ts`:
```typescript
import { setupPluginListeners } from "tauri-plugin-mcp";
setupPluginListeners();
```

The import must call `setupPluginListeners()` — bare `import "tauri-plugin-mcp"` does nothing.

### 4. MCP Server (Claude Code side)

Binary at `~/.local/bin/tauri-mcp-server`. Configured in Claude Code's MCP settings to connect via the Unix socket.

Socket path: `$XDG_RUNTIME_DIR/tauri-mcp.sock` (typically `/run/user/1000/tauri-mcp.sock`).

## Running

```bash
# Normal (no MCP):
cargo tauri dev

# With MCP debug:
cargo tauri dev --features mcp-debug
```

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `Socket address already in use` | Stale socket from crashed process | `rm /run/user/1000/tauri-mcp.sock` |
| `app_info` works but `execute_js` times out | Guest JS not loaded | Ensure `setupPluginListeners()` is called in main.ts |
| `ENOENT /run/user/1000/tauri-mcp.sock` | App not running with mcp-debug | Start with `--features mcp-debug` |
| `PluginConfig::default()` doesn't start server | `start_socket_server` defaults to false | Use `PluginConfig::new("name")` instead |
| Screenshots fail on Linux | xcap can't find window | Known limitation; use DOM queries instead |

## What NOT to commit

The tauri-plugin-mcp npm dependency and import are local dev-only. Do not commit:
- `package.json` changes adding `tauri-plugin-mcp`
- The `setupPluginListeners()` import in main.ts
- The Cargo path dependency (uses local path)

These changes live only in your local working tree.
