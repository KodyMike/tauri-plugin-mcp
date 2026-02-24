import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerManageDevtoolsTool(server: McpServer) {
  server.tool(
    "manage_devtools",
    "Controls the browser DevTools for a webview window. Open, close, or check if DevTools are currently open. Only available in debug builds or with the 'devtools' feature enabled.",
    {
      action: z.enum(["open", "close", "is_open"]).describe("The devtools action to perform."),
      window_label: z.string().default("main").describe("The window to target. Defaults to 'main'."),
    },
    {
      title: "Manage DevTools",
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    async ({ action, window_label }) => {
      try {
        const payload = { action, window_label };
        logCommandParams('manage_devtools', payload);

        const result = await socketClient.sendCommand('manage_devtools', payload);

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'manage_devtools failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('manage_devtools error:', error);
        return createErrorResponse(`Failed to manage devtools: ${(error as Error).message}`);
      }
    },
  );
}
