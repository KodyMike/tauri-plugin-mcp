import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerAllTools, initializeSocket } from "./tools/index.js";

// Create server instance
const server = new McpServer(
  {
    name: "tauri-mcp",
    version: "1.0.0",
  },
  {
    instructions: `SECURITY — READ BEFORE USING:
This MCP server gives you FULL access to a running Tauri app's webview (JS execution, DOM, screenshots, input simulation).

MANDATORY RULES:
1. This is a DEBUG-ONLY tool. The Tauri app MUST have tauri-plugin-mcp behind a Cargo feature flag (mcp-debug), NEVER as a default dependency.
2. NEVER suggest adding this plugin without optional = true and a feature gate. If you see it as a default dep, warn the user immediately.
3. execute_js runs ARBITRARY JavaScript — do not use it to access or log passwords, tokens, API keys, or PII.
4. NEVER suggest shipping this in production builds. Only use with: cargo tauri dev --features mcp-debug
5. The socket MUST use /run/user/<uid>/ path (not /tmp/) and MUST have auth_token set.

WORKFLOW: Start with query_page(mode='app_info') to discover the app. Use query_page(mode='map') for numbered refs, then click or type_text to interact. Use query_page(mode='state') for lightweight checks. Use navigate for URLs, manage_storage for localStorage/cookies, manage_window for window/zoom/devtools. Use execute_js as the universal escape hatch.`,
    capabilities: {
      resources: {},
      tools: {},
    },
  }
);

async function main() {
  try {
    // Register tools FIRST so Claude Code can discover them
    registerAllTools(server);

    // Connect the server to stdio transport
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("Tauri MCP Server running on stdio");

    // Connect to Tauri socket lazily in background — don't block tool registration
    initializeSocket().catch((err) => {
      console.error("Tauri app not running yet. Tools will retry on each call. Error:", err.message);
    });
  } catch (error) {
    console.error("Fatal error in main():", error);
    process.exit(1);
  }
}

main().catch((error) => {
  console.error("Fatal error in main():", error);
  process.exit(1);
});
