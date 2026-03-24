import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { socketClient } from "./client.js";
import { createErrorResponse, createSuccessResponse, logCommandParams } from "./response-helpers.js";

export function registerExecuteJsTool(server: McpServer) {
  server.tool(
    "execute_js",
    "Executes JavaScript in a webview and returns the last expression's value. Supports single expressions ('document.title'), multi-statement code ('var x = 5; x.toString()' → '5'), Promises (auto-awaited), and top-level await ('const r = await fetch(url); r.status'). Use window.__TAURI__ for Tauri API access. Caution: can modify page state.",
    {
      code: z.string().describe("JavaScript code to execute. Single expressions return their value directly (e.g. 'document.title'). Multi-statement code returns the last expression's value (e.g. 'const el = document.querySelector(\"h1\"); el.textContent'). Promises are auto-awaited. Top-level await is supported (e.g. 'const r = await fetch(\"/api\"); await r.json()')."),
      window_label: z.string().default("main").describe("The identifier (e.g., visible title or internal label) of the application window where the JavaScript code will be executed. Defaults to 'main' if not specified."),
      timeout_ms: z.number().int().positive().optional().describe("The maximum time in milliseconds to allow for the JavaScript execution. If the script exceeds this timeout, its execution will be terminated, and an error may be returned."),
    },
    {
      title: "Execute JavaScript Code in Specified Application Window",
      readOnlyHint: false,
      destructiveHint: true,
      idempotentHint: false,
      openWorldHint: false,
    },
    async ({ code, window_label, timeout_ms }) => {
      try {
        // Validate required parameters
        if (!code || code.trim() === '') {
          return createErrorResponse("The code parameter is required and cannot be empty");
        }
        
        const params = { code, window_label, timeout_ms };
        logCommandParams('execute_js', params);
        
        // Use default window label if not provided
        const effectiveWindowLabel = window_label || 'main';
        
        const result = await socketClient.sendCommand('execute_js', {
          code,
          window_label: effectiveWindowLabel,
          timeout_ms
        });

        // Extract result and type from the response
        const resultValue = result?.result;
        const resultType: string = result?.type || 'unknown';

        let text: string;
        if (resultType === 'undefined' || resultType === 'null') {
          text = `[${resultType}]`;
        } else if (resultType === 'error' && typeof resultValue === 'object' && resultValue !== null) {
          text = `Error: ${resultValue.message || JSON.stringify(resultValue)}`;
        } else if (typeof resultValue === 'string') {
          text = resultValue;
        } else {
          text = JSON.stringify(resultValue, null, 2);
        }

        // Add type annotation for non-obvious types
        if (resultType !== 'string' && resultType !== 'undefined' && resultType !== 'null') {
          text = `[type: ${resultType}]\n${text}`;
        }

        return createSuccessResponse(text);
      } catch (error) {
        console.error('JS execution error:', error);
        return createErrorResponse(`Failed to execute JavaScript: ${(error as Error).message}`);
      }
    },
  );
} 