import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import type { IncomingMessage, ServerResponse } from "node:http";
import { defineConfig, type Plugin } from "vite";

import { gitBridgePlugin } from "./src/lib/server/gitBridge";
import {
  countLocalSourceReferences,
  findLocalSourceDefinitions,
  findLocalSourceReferences,
  readLocalSourceFile,
  scanLocalAgentSessions,
  scanLocalSourceFiles,
  searchLocalSourceFiles,
  validateLocalProjectRoot,
  writeLocalSourceFile,
} from "./src/lib/server/localSourceFs";
import type { SourceRecord } from "./src/lib/sourceData";

function localSourceBridgePlugin(): Plugin {
  return {
    name: "mac-command-bar-local-source-bridge",
    configureServer(server) {
      server.middlewares.use(createLocalSourceBridgeMiddleware());
    },
    configurePreviewServer(server) {
      server.middlewares.use(createLocalSourceBridgeMiddleware());
    },
  };
}

function createLocalSourceBridgeMiddleware() {
  return async (
    request: IncomingMessage,
    response: ServerResponse,
    next: () => void
  ) => {
    const pathname = request.url?.split("?")[0] ?? "";
    if (!pathname.startsWith("/__mcb/source/")) {
      next();
      return;
    }

    if (request.method !== "POST") {
      sendJSON(response, 405, { error: "Method not allowed" });
      return;
    }

    try {
      const body = await readJSONBody(request);
      switch (pathname) {
        case "/__mcb/source/validate":
          sendJSON(
            response,
            200,
            await validateLocalProjectRoot(String(body.path ?? ""))
          );
          break;
        case "/__mcb/source/list":
          sendJSON(
            response,
            200,
            await scanLocalSourceFiles({
              root: String(body.root ?? ""),
              query: stringOrNull(body.query),
              limit: numberOrNull(body.limit),
            })
          );
          break;
        case "/__mcb/source/read":
          sendJSON(
            response,
            200,
            await readLocalSourceFile(String(body.path ?? ""))
          );
          break;
        case "/__mcb/source/agent-sessions":
          sendJSON(response, 200, await scanLocalAgentSessions());
          break;
        case "/__mcb/source/write":
          sendJSON(
            response,
            200,
            await writeLocalSourceFile(
              String(body.path ?? ""),
              String(body.content ?? "")
            )
          );
          break;
        case "/__mcb/source/search":
          sendJSON(
            response,
            200,
            await searchLocalSourceFiles(
              sourceRecordsFromBody(body.records),
              String(body.query ?? ""),
              numberOrUndefined(body.limit)
            )
          );
          break;
        case "/__mcb/source/definitions":
          sendJSON(
            response,
            200,
            await findLocalSourceDefinitions(
              sourceRecordsFromBody(body.records),
              String(body.symbolName ?? ""),
              numberOrUndefined(body.limit)
            )
          );
          break;
        case "/__mcb/source/references":
          sendJSON(
            response,
            200,
            await findLocalSourceReferences(
              sourceRecordsFromBody(body.records),
              String(body.symbolName ?? ""),
              numberOrUndefined(body.limit)
            )
          );
          break;
        case "/__mcb/source/reference-counts":
          sendJSON(
            response,
            200,
            await countLocalSourceReferences(
              String(body.root ?? ""),
              stringsFromBody(body.symbolNames),
              numberOrNull(body.deadlineMs)
            )
          );
          break;
        default:
          sendJSON(response, 404, { error: "Unknown source bridge route" });
          break;
      }
    } catch (error) {
      sendJSON(response, 500, {
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };
}

async function readJSONBody(
  request: IncomingMessage
): Promise<Record<string, unknown>> {
  const chunks: Buffer[] = [];

  for await (const chunk of request) {
    chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : chunk);
  }

  const text = Buffer.concat(chunks).toString("utf8").trim();
  if (!text) return {};

  const parsed = JSON.parse(text);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Request body must be a JSON object");
  }

  return parsed as Record<string, unknown>;
}

function sendJSON(response: ServerResponse, statusCode: number, body: unknown) {
  response.statusCode = statusCode;
  response.setHeader("content-type", "application/json; charset=utf-8");
  response.end(JSON.stringify(body));
}

function sourceRecordsFromBody(value: unknown): SourceRecord[] {
  return Array.isArray(value) ? (value as SourceRecord[]) : [];
}

function stringsFromBody(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((entry) => typeof entry === "string")
    : [];
}

function stringOrNull(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function numberOrNull(value: unknown): number | null {
  return typeof value === "number" ? value : null;
}

function numberOrUndefined(value: unknown): number | undefined {
  return typeof value === "number" ? value : undefined;
}

export default defineConfig({
  // Tailwind only ever compiles `src/lib/shell/styles/next.css`, which only the
  // /next route imports. The old shell never loads that file, so adding the
  // plugin here cannot change how the old page renders.
  //
  // The git bridge lets the source-control panel READ a repository from a
  // browser tab, so the panes and the commit graph can be looked at without the
  // desktop app. It cannot change a repository — see `src/lib/server/gitBridge.ts`.
  plugins: [
    gitBridgePlugin(),
    localSourceBridgePlugin(),
    tailwindcss(),
    sveltekit(),
  ],
  worker: {
    format: "es",
  },
  server: {
    host: "127.0.0.1",
    port: 5177,
    strictPort: true,
    watch: {
      ignored: ["**/.svelte-kit/generated/**"],
    },
  },
  clearScreen: false,
});
