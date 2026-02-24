import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerGetAppInfoTool(server: McpServer) {
  server.tool(
    "get_app_info",
    "Returns consolidated environment data: app name/version, OS/arch, all window labels with URLs/sizes, monitor info, and app directories. Use this as a first step to understand the application state.",
    {},
    {
      title: "Get Application Info",
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    async () => {
      try {
        logCommandParams('get_app_info', {});

        const result = await socketClient.sendCommand('get_app_info', {});

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'get_app_info failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('get_app_info error:', error);
        return createErrorResponse(`Failed to get app info: ${(error as Error).message}`);
      }
    },
  );
}
