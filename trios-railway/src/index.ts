#!/usr/bin/env node

// @ts-ignore - SDK types not resolved properly
import { createStreamableHTTPServer } from "@modelcontextprotocol/sdk";
import express from "express";
import cors from "cors";
import bodyParser from "body-parser";

const PORT = process.env.PORT || "3026";
const MCP_SERVER_NAME = "trios-railway";

// Railway API token from environment
const RAILWAY_API_TOKEN = process.env.RAILWAY_API_TOKEN;
const RAILWAY_PROJECT_ID = process.env.RAILWAY_PROJECT_ID || "";

// MCP tools for Railway service management
const tools = [
  {
    name: "railway_service_deploy",
    description: "Deploy or redeploy a Railway service with environment variable upsert",
    inputSchema: {
      type: "object",
      properties: {
        name: {
          type: "string",
          description: "Service name"
        },
        env: {
          type: "object",
          description: "Environment variables to set"
        }
      }
    }
  },
  {
    name: "railway_service_list",
    description: "List all Railway services in the project",
    inputSchema: {
      type: "object",
      properties: {}
    }
  },
  {
    name: "railway_service_delete",
    description: "Delete a Railway service by ID (requires confirm: true)",
    inputSchema: {
      type: "object",
      properties: {
        serviceId: {
          type: "string",
          description: "Service ID to delete"
        },
        confirm: {
          type: "boolean",
          description: "Set to true to confirm deletion"
        }
      }
    }
  },
  {
    name: "railway_service_redeploy",
    description: "Redeploy an existing Railway service",
    inputSchema: {
      type: "object",
      properties: {
        serviceId: {
          type: "string",
          description: "Service ID to redeploy"
        }
      }
    }
  },
  {
    name: "railway_audit_migrate_sql",
    description: "Generate Neon DDL for audit tables",
    inputSchema: {
      type: "object",
      properties: {
        tables: {
          type: "array",
          description: "Array of table names to generate DDL for",
          items: {
            type: "string"
          }
        }
      }
    }
  },
  {
    name: "railway_experience_append",
    description: "Append a structured line to L7 experience log",
    inputSchema: {
      type: "object",
      properties: {
        entry: {
          type: "string",
          description: "Experience log entry (formatted string)"
        }
      }
    }
  }
];

// Execute Railway CLI commands
async function executeRailwayCommand(command: string, args: string[]): Promise<{stdout: string, stderr: string}> {
  const { exec } = await import("child_process");

  return new Promise((resolve, reject) => {
    exec(
      `railway ${command} ${args.join(" ")}`,
      { env: { ...process.env, RAILWAY_TOKEN: RAILWAY_API_TOKEN } },
      (error, stdout, stderr) => {
        if (error) {
          reject({ stdout: "", stderr: error.message });
        } else {
          resolve({ stdout: stdout.toString(), stderr: stderr.toString() });
        }
      }
    );
  });
}

// Tool implementations
const handlers = {
  async railway_service_deploy(args: any) {
    const { name, env } = args;
    const envArgs = env
      ? Object.entries(env).map(([k, v]) => `--env ${k}=${v}`).join(" ")
      : "";
    const result = await executeRailwayCommand("up", [name, envArgs, "--detach"]);
    return {
      content: [{
        type: "text",
        text: result.stdout || "Service deployment initiated"
      }],
      isError: result.stderr.length > 0
    };
  },

  async railway_service_list(args: any) {
    const result = await executeRailwayCommand("list", ["--json"]);
    try {
      const services = JSON.parse(result.stdout);
      return {
        content: [{
          type: "text",
          text: JSON.stringify(services, null, 2)
        }],
        isError: false
      };
    } catch (e) {
      return {
        content: [{ type: "text", text: result.stdout }],
        isError: true
      };
    }
  },

  async railway_service_delete(args: any) {
    const { serviceId, confirm } = args;
    if (!confirm) {
      return {
        content: [{
          type: "text",
          text: "Error: confirm must be true to delete service"
        }],
        isError: true
      };
    }
    const result = await executeRailwayCommand("rm", [serviceId]);
    return {
      content: [{
        type: "text",
        text: result.stdout || "Service deleted"
      }],
      isError: result.stderr.length > 0
    };
  },

  async railway_service_redeploy(args: any) {
    const { serviceId } = args;
    const result = await executeRailwayCommand("up", [serviceId, "--detach"]);
    return {
      content: [{
        type: "text",
        text: result.stdout || "Service redeploy initiated"
      }],
      isError: result.stderr.length > 0
    };
  },

  async railway_audit_migrate_sql(args: any) {
    const { tables } = args;
    const ddl = tables.map((table: string) => `
CREATE TABLE IF NOT EXISTS ${table} (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMP DEFAULT NOW(),
  data JSONB
);`).join("\n");
    return {
      content: [{ type: "text", text: ddl }],
      isError: false
    };
  },

  async railway_experience_append(args: any) {
    const { entry } = args;
    const fs = await import("fs");
    const path = await import("path");
    const experienceDir = path.join(process.cwd(), ".trinity", "experience");
    const filename = `trios_${new Date().toISOString().slice(0, 10).replace(/-/g, "")}.trinity`;
    const filePath = path.join(experienceDir, filename);

    await fs.promises.mkdir(experienceDir, { recursive: true });
    await fs.promises.appendFile(filePath, `${entry}\n`);
    return {
      content: [{ type: "text", text: `Appended to ${filename}` }],
      isError: false
    };
  }
};

// Start Express server for health check
const app = express();
app.use(cors());
app.use(bodyParser.json());
app.get("/health", (req, res) => {
  res.json({
    status: "healthy",
    version: "1.0.0",
    tools: tools.length,
    server: MCP_SERVER_NAME
  });
});

// Start HTTP server
app.listen(PORT, () => {
  console.log(`${MCP_SERVER_NAME} MCP server listening on http://0.0.0.0:${PORT}/mcp`);
  console.log(`Health check at http://0.0.0.0:${PORT}/health`);
});

// Create MCP server
const server = createStreamableHTTPServer({
  name: MCP_SERVER_NAME,
  version: "1.0.0",
  tools,
}, handlers);

server.listen(PORT);
