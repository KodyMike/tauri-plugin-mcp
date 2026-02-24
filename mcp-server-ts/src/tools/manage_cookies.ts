import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerManageCookiesTool(server: McpServer) {
  server.tool(
    "manage_cookies",
    "Manages cookies for a webview. Get all cookies, get cookies for a specific URL, or clear all browsing data (nuclear option — clears cookies, cache, and local storage).",
    {
      action: z.enum(["get_all", "get_for_url", "clear_all"]).describe("The cookie action: get_all returns all cookies, get_for_url returns cookies for a URL, clear_all wipes all browsing data."),
      window_label: z.string().default("main").describe("The window to target. Defaults to 'main'."),
      url: z.string().optional().describe("URL to get cookies for. Required for 'get_for_url'."),
    },
    {
      title: "Manage Cookies",
      readOnlyHint: false,
      destructiveHint: true,
      idempotentHint: false,
      openWorldHint: false,
    },
    async ({ action, window_label, url }) => {
      try {
        const payload: Record<string, any> = { action, window_label };
        if (url) payload.url = url;
        logCommandParams('manage_cookies', payload);

        const result = await socketClient.sendCommand('manage_cookies', payload);

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'manage_cookies failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('manage_cookies error:', error);
        return createErrorResponse(`Failed to manage cookies: ${(error as Error).message}`);
      }
    },
  );
}
