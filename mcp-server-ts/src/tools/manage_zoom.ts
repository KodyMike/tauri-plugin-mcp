import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerManageZoomTool(server: McpServer) {
  server.tool(
    "manage_zoom",
    "Controls the zoom level of a webview. Set a specific zoom scale (1.0 = 100%, 0.5 = 50%, 2.0 = 200%) or get the current zoom level.",
    {
      action: z.enum(["set", "get"]).describe("The zoom action: 'set' changes zoom level, 'get' returns current zoom."),
      window_label: z.string().default("main").describe("The window to target. Defaults to 'main'."),
      scale: z.number().min(0.1).max(5.0).optional().describe("Zoom scale factor. 1.0 = 100%. Required for 'set' action."),
    },
    {
      title: "Manage Zoom",
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    async ({ action, window_label, scale }) => {
      try {
        const payload: Record<string, any> = { action, window_label };
        if (scale !== undefined) payload.scale = scale;
        logCommandParams('manage_zoom', payload);

        const result = await socketClient.sendCommand('manage_zoom', payload);

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'manage_zoom failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('manage_zoom error:', error);
        return createErrorResponse(`Failed to manage zoom: ${(error as Error).message}`);
      }
    },
  );
}
