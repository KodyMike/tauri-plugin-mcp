import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerNavigateWebviewTool(server: McpServer) {
  server.tool(
    "navigate_webview",
    "Controls webview navigation: navigate to a URL, reload the page, get the current URL, or go back/forward in history. Subsumes navigate_back with additional URL navigation and reload capabilities.",
    {
      action: z.enum(["navigate", "reload", "get_url", "back", "forward"]).describe("The navigation action to perform."),
      window_label: z.string().default("main").describe("The window to target. Defaults to 'main'."),
      url: z.string().optional().describe("The URL to navigate to. Required for 'navigate' action."),
    },
    {
      title: "Navigate Webview",
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: false,
      openWorldHint: true,
    },
    async ({ action, window_label, url }) => {
      try {
        const payload: Record<string, any> = { action, window_label };
        if (url) payload.url = url;
        logCommandParams('navigate_webview', payload);

        const result = await socketClient.sendCommand('navigate_webview', payload);

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'navigate_webview failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('navigate_webview error:', error);
        return createErrorResponse(`Failed to navigate webview: ${(error as Error).message}`);
      }
    },
  );
}
