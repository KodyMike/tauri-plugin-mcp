import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerManageEventsTool(server: McpServer) {
  server.tool(
    "manage_events",
    "Interacts with the Tauri event system. Emit events to the app or specific targets, listen to a named event for a duration, or sniff events from any target for a named event. Powerful for debugging and triggering app behavior.",
    {
      action: z.enum(["emit", "emit_to", "listen", "sniff"]).describe("The event action: emit broadcasts to all, emit_to targets a specific window, listen captures events of a specific name, sniff captures from any target for a named event."),
      event: z.string().optional().describe("The event name. Required for all actions."),
      target: z.string().optional().describe("The target window/webview label. Required for 'emit_to'."),
      payload: z.any().optional().describe("The event payload (any JSON value). Used with emit/emit_to."),
      duration_ms: z.number().int().min(100).max(30000).optional().describe("How long to listen/sniff in milliseconds. Default: 1000, max: 30000."),
    },
    {
      title: "Manage Events",
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: false,
      openWorldHint: false,
    },
    async ({ action, event, target, payload, duration_ms }) => {
      try {
        const params: Record<string, any> = { action };
        if (event) params.event = event;
        if (target) params.target = target;
        if (payload !== undefined) params.payload = payload;
        if (duration_ms) params.duration_ms = duration_ms;
        logCommandParams('manage_events', params);

        const result = await socketClient.sendCommand('manage_events', params);

        if (!result || typeof result !== 'object') {
          return createErrorResponse('Failed to get a valid response');
        }

        if ('success' in result && !result.success) {
          return createErrorResponse(result.error as string || 'manage_events failed');
        }

        const data = result.data ?? result;
        return createSuccessResponse(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      } catch (error) {
        console.error('manage_events error:', error);
        return createErrorResponse(`Failed to manage events: ${(error as Error).message}`);
      }
    },
  );
}
