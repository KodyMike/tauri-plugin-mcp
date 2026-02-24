import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerListWindowsTool(server: McpServer) {
  server.tool(
    "list_windows",
    "Enumerates all open windows and webviews with metadata: label, title, URL, visibility, focus, size, position, scale factor, and monitor. Use this to discover available windows before targeting them with other tools.",
    {},
    {
      title: "List Windows",
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    async () => {
      try {
        logCommandParams('list_windows', {});

        const result = await socketClient.sendCommand('list_windows', {});

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'list_windows failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('list_windows error:', error);
        return createErrorResponse(`Failed to list windows: ${(error as Error).message}`);
      }
    },
  );
}
