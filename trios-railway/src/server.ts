/**
 * SR-02 — Streamable HTTP /mcp endpoint for the Browser Tools Server.
 *
 * Mounts a JSON-RPC 2.0 MCP server on the existing Express app so that
 * Perplexity Custom Connector (and any other MCP HTTP client) can use the
 * 14 browser tools through the same Tailscale Funnel URL with Basic Auth.
 *
 * Each tool delegates to the existing internal HTTP route on
 * http://127.0.0.1:<PORT>/... (the routes are already registered by
 * browser-connector.ts), so we don't duplicate any business logic.
 *
 * Stateless mode: a fresh McpServer + StreamableHTTPServerTransport pair
 * is constructed per request. This matches Perplexity's behaviour of
 * issuing initialize / tools/list / tools/call as independent POSTs.
 */

import type { Express, Request, Response } from "express";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";

type GetPort = () => number;

interface InternalAuth {
  username: string;
  password: string;
  enabled: boolean;
}

// Tool name + description + corresponding internal REST route (method, path).
// "compose" tools have no single backing route — they return canned guidance,
// matching the behaviour of the stdio mcp-server.ts.
type RouteSpec =
  | { kind: "fetch"; method: "GET" | "POST"; path: string; body?: unknown }
  | { kind: "compose"; text: string };

const NEXT_JS_AUDIT_TEXT = `runNextJSAudit composes SEO, accessibility and best-practices Lighthouse audits.
For each iteration of changes, re-invoke this tool and re-run the underlying
audits (runSEOAudit, runAccessibilityAudit, runBestPracticesAudit) to confirm
issues have been resolved. When no further issues remain, report the page as
optimised for SEO, accessibility, and best practices.`;

const DEBUGGER_MODE_TEXT = `runDebuggerMode is a composite workflow:
1. Call getConsoleErrors and getNetworkErrors.
2. If errors are present, call getConsoleLogs and getNetworkLogs to triage.
3. Call getSelectedElement to inspect the user-selected DOM node.
4. Optionally call takeScreenshot for a visual snapshot.
Reason about root causes only after gathering at least the first two
artifacts. Do not stop on the first error — collect them all first.`;

const AUDIT_MODE_TEXT = `runAuditMode is a composite workflow:
1. Run runAccessibilityAudit, runPerformanceAudit, runSEOAudit, and
   runBestPracticesAudit in sequence.
2. Aggregate the findings into a single mission-style report.
3. Call takeScreenshot to attach a visual snapshot of the audited page.
4. If runtime issues surface, hand off to runDebuggerMode.`;

interface ToolSpec {
  name: string;
  description: string;
  spec: RouteSpec;
}

const TOOLS: ToolSpec[] = [
  {
    name: "getConsoleLogs",
    description: "Check our browser logs",
    spec: { kind: "fetch", method: "GET", path: "/console-logs" },
  },
  {
    name: "getConsoleErrors",
    description: "Check our browsers console errors",
    spec: { kind: "fetch", method: "GET", path: "/console-errors" },
  },
  {
    name: "getNetworkErrors",
    description: "Check our network ERROR logs",
    spec: { kind: "fetch", method: "GET", path: "/network-errors" },
  },
  {
    name: "getNetworkLogs",
    description: "Check ALL our network logs",
    spec: { kind: "fetch", method: "GET", path: "/network-success" },
  },
  {
    name: "takeScreenshot",
    description: "Take a screenshot of the current browser tab",
    spec: { kind: "fetch", method: "POST", path: "/capture-screenshot" },
  },
  {
    name: "getSelectedElement",
    description: "Get the selected element from the browser",
    spec: { kind: "fetch", method: "GET", path: "/selected-element" },
  },
  {
    name: "wipeLogs",
    description: "Wipe all browser logs from memory",
    spec: { kind: "fetch", method: "POST", path: "/wipelogs" },
  },
  {
    name: "runAccessibilityAudit",
    description: "Run an accessibility audit on the current page",
    spec: {
      kind: "fetch",
      method: "POST",
      path: "/accessibility-audit",
      body: { category: "accessibility", source: "mcp_tool" },
    },
  },
  {
    name: "runPerformanceAudit",
    description: "Run a performance audit on the current page",
    spec: {
      kind: "fetch",
      method: "POST",
      path: "/performance-audit",
      body: { category: "performance", source: "mcp_tool" },
    },
  },
  {
    name: "runSEOAudit",
    description: "Run an SEO audit on the current page",
    spec: {
      kind: "fetch",
      method: "POST",
      path: "/seo-audit",
      body: { category: "seo", source: "mcp_tool" },
    },
  },
  {
    name: "runBestPracticesAudit",
    description: "Run a best-practices audit on the current page",
    spec: {
      kind: "fetch",
      method: "POST",
      path: "/best-practices-audit",
      body: { category: "best-practices", source: "mcp_tool" },
    },
  },
  {
    name: "runNextJSAudit",
    description: "Composite NextJS SEO/a11y/best-practices audit guidance",
    spec: { kind: "compose", text: NEXT_JS_AUDIT_TEXT },
  },
  {
    name: "runDebuggerMode",
    description: "Composite debugger workflow combining log + element + screenshot tools",
    spec: { kind: "compose", text: DEBUGGER_MODE_TEXT },
  },
  {
    name: "runAuditMode",
    description: "Composite audit workflow combining the four Lighthouse tools and a screenshot",
    spec: { kind: "compose", text: AUDIT_MODE_TEXT },
  },
];

function buildAuthHeader(auth: InternalAuth): string | null {
  if (!auth.enabled) return null;
  const token = Buffer.from(`${auth.username}:${auth.password}`, "utf8").toString("base64");
  return `Basic ${token}`;
}

async function callInternalRoute(
  route: { method: "GET" | "POST"; path: string; body?: unknown },
  port: number,
  auth: InternalAuth
): Promise<{ ok: boolean; status: number; text: string; json: unknown }> {
  const url = `http://127.0.0.1:${port}${route.path}`;
  const headers: Record<string, string> = {
    Accept: "application/json",
  };
  const authHeader = buildAuthHeader(auth);
  if (authHeader) headers.Authorization = authHeader;
  const init: RequestInit & { body?: string } = {
    method: route.method,
    headers,
  };
  if (route.body !== undefined) {
    headers["Content-Type"] = "application/json";
    init.body = JSON.stringify({ ...(route.body as object), timestamp: Date.now() });
  }
  // Use the global fetch (Node 18+).
  const response = await fetch(url, init);
  const text = await response.text();
  let json: unknown = null;
  try {
    json = JSON.parse(text);
  } catch {
    json = text;
  }
  return { ok: response.ok, status: response.status, text, json };
}

function buildMcpServer(getPort: GetPort, auth: InternalAuth): McpServer {
  const server = new McpServer({
    name: "Browser Tools MCP (HTTP)",
    version: "1.2.0-sr02",
  });

  for (const tool of TOOLS) {
    if (tool.spec.kind === "compose") {
      const text = tool.spec.text;
      server.tool(tool.name, tool.description, {}, async () => ({
        content: [{ type: "text", text }],
      }));
      continue;
    }

    const route = tool.spec;
    server.tool(tool.name, tool.description, {}, async () => {
      try {
        const port = getPort();
        const result = await callInternalRoute(route, port, auth);
        if (!result.ok) {
          return {
            content: [
              {
                type: "text",
                text: `Internal route ${route.method} ${route.path} returned ${result.status}: ${result.text}`,
              },
            ],
            isError: true,
          };
        }
        const body =
          typeof result.json === "string"
            ? result.json
            : JSON.stringify(result.json, null, 2);
        return { content: [{ type: "text", text: body }] };
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        return {
          content: [
            { type: "text", text: `Tool ${tool.name} failed: ${message}` },
          ],
          isError: true,
        };
      }
    });
  }

  return server;
}

/**
 * Mount POST /mcp (Streamable HTTP) and GET /mcp (returns 405 in stateless
 * mode) on the supplied Express app. The caller is responsible for placing
 * Basic Auth middleware in front of /mcp — this module assumes the request
 * has already been authorised.
 *
 * @param app    The Express application created in browser-connector.ts.
 * @param getPort Lazy accessor for the actual server port (resolved after
 *                listen() picks an available one).
 * @param auth   Credentials for the internal bridge calls back to
 *                127.0.0.1:<PORT>/<route> — usually the same Basic Auth that
 *                guards /mcp itself.
 */
export function mountMcpHttpHandler(
  app: Express,
  getPort: GetPort,
  auth: InternalAuth
): void {
  app.post("/mcp", async (req: Request, res: Response) => {
    let server: McpServer | null = null;
    let transport: StreamableHTTPServerTransport | null = null;
    try {
      server = buildMcpServer(getPort, auth);
      transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: undefined, // stateless
      });
      res.on("close", () => {
        try {
          transport?.close();
        } catch {
          /* noop */
        }
        try {
          server?.close();
        } catch {
          /* noop */
        }
      });
      await server.connect(transport);
      await transport.handleRequest(req, res, req.body);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error("[MCP /mcp] handler error:", message);
      if (!res.headersSent) {
        res.status(500).json({
          jsonrpc: "2.0",
          error: { code: -32603, message: `Internal MCP error: ${message}` },
          id: null,
        });
      }
    }
  });

  // Stateless mode: GET (and DELETE) are not used. Return 405 with a JSON-RPC
  // error so MCP clients that probe the endpoint get a clean answer.
  const methodNotAllowed = (_req: Request, res: Response) => {
    res.status(405).json({
      jsonrpc: "2.0",
      error: { code: -32000, message: "Method Not Allowed (stateless /mcp)" },
      id: null,
    });
  };
  app.get("/mcp", methodNotAllowed);
  app.delete("/mcp", methodNotAllowed);

  console.log("[MCP] Streamable HTTP /mcp endpoint mounted (stateless mode)");
}

export const TOOL_NAMES_FOR_LOG: string[] = TOOLS.map((t) => t.name);
