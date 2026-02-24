import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerManageWebviewStateTool(server: McpServer) {
  server.tool(
    "manage_webview_state",
    "Manages webview configuration and state. Clear all browsing data, set the background color, get webview bounds (position + size), or toggle auto-resize behavior.",
    {
      action: z.enum(["clear_browsing_data", "set_background_color", "get_bounds", "set_auto_resize"]).describe("The webview state action to perform."),
      window_label: z.string().default("main").describe("The window to target. Defaults to 'main'."),
      r: z.number().int().min(0).max(255).optional().describe("Red color component (0-255). For 'set_background_color'."),
      g: z.number().int().min(0).max(255).optional().describe("Green color component (0-255). For 'set_background_color'."),
      b: z.number().int().min(0).max(255).optional().describe("Blue color component (0-255). For 'set_background_color'."),
      a: z.number().int().min(0).max(255).optional().describe("Alpha component (0-255). For 'set_background_color'. 255 = fully opaque."),
      enabled: z.boolean().optional().describe("Whether to enable auto-resize. For 'set_auto_resize'."),
    },
    {
      title: "Manage Webview State",
      readOnlyHint: false,
      destructiveHint: true,
      idempotentHint: false,
      openWorldHint: false,
    },
    async ({ action, window_label, r, g, b, a, enabled }) => {
      try {
        const payload: Record<string, any> = { action, window_label };
        if (r !== undefined) payload.r = r;
        if (g !== undefined) payload.g = g;
        if (b !== undefined) payload.b = b;
        if (a !== undefined) payload.a = a;
        if (enabled !== undefined) payload.enabled = enabled;
        logCommandParams('manage_webview_state', payload);

        const result = await socketClient.sendCommand('manage_webview_state', payload);

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'manage_webview_state failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('manage_webview_state error:', error);
        return createErrorResponse(`Failed to manage webview state: ${(error as Error).message}`);
      }
    },
  );
}
