import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { realpathSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { defineConfig, type Plugin } from "vite";

import { gitBridgePlugin } from "./src/lib/server/gitBridge";
import {
  countLocalSourceReferences,
  findLocalSourceDefinitions,
  findLocalSourceReferences,
  listLocalSourceDirectory,
  readLocalSourceFile,
  scanLocalAgentSessions,
  scanLocalSourceFiles,
  searchLocalSourceTree,
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

/**
 * Upstream UI CSS ships three relational-selector patterns. Registering even
 * one makes WebKit invalidate it after every transcript DOM insertion, so the
 * dependency state is mirrored onto the element that owns it and the CSS can
 * use ordinary compound selectors. Exact source markers make dependency drift
 * fail visibly instead of silently putting the global mutation tax back.
 */
function relationalSelectorPurgePlugin(): Plugin {
  const replaceRequired = (code: string, before: string, after: string, id: string): string => {
    if (!code.includes(before)) {
      throw new Error(`relational selector purge marker missing in ${id}: ${before}`);
    }
    return code.replaceAll(before, after);
  };

  return {
    name: "mac-command-bar-relational-selector-purge",
    enforce: "pre",
    transform(code, id) {
      if (id.includes("/dockview-core/dist/styles/dockview.css")) {
        let next = code
          .replaceAll(
            ".dv-resize-container:has(> .dv-groupview)",
            ".dv-resize-container.dv-resize-container-with-groupview"
          )
          .replaceAll(
            ".dv-tab-group-chip:has(.dv-tab-group-chip-label--empty)",
            ".dv-tab-group-chip.dv-tab-group-chip--empty"
          );
        if (next.includes(":has(")) {
          throw new Error(`unhandled relational selector in ${id}`);
        }
        return next;
      }

      if (id.includes("/dockview-core/dist/esm/overlay/overlay.js")) {
        return replaceRequired(
          code,
          "this._element.className = 'dv-resize-container';",
          "this._element.className = 'dv-resize-container';\n        this._element.classList.toggle('dv-resize-container-with-groupview', this.options.content.classList.contains('dv-groupview'));",
          id
        );
      }

      if (id.includes("/dockview-core/dist/esm/dockview/components/titlebar/tabGroupChip.js")) {
        return replaceRequired(
          code,
          "toggleClass(this._label, 'dv-tab-group-chip-label--empty', !label);",
          "toggleClass(this._label, 'dv-tab-group-chip-label--empty', !label);\n        toggleClass(this._element, 'dv-tab-group-chip--empty', !label);",
          id
        );
      }

      if (id.includes("/markdownRenderer/browser/renderedMarkdown.css")) {
        return replaceRequired(
          code,
          ".rendered-markdown li:has(input[type=checkbox])",
          ".rendered-markdown li.rendered-markdown-checkbox-item",
          id
        );
      }

      if (id.includes("/vs/base/browser/markdownRenderer.js")) {
        return replaceRequired(
          code,
          "input.setAttribute('disabled', '');",
          "input.setAttribute('disabled', '');\n            input.closest('li')?.classList.add('rendered-markdown-checkbox-item');",
          id
        );
      }
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
        case "/__mcb/source/list-directory":
          sendJSON(
            response,
            200,
            await listLocalSourceDirectory(
              String(body.root ?? ""),
              String(body.directory ?? ""),
              Boolean(body.includeExcluded)
            )
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
        case "/__mcb/source/search-tree":
          sendJSON(
            response,
            200,
            await searchLocalSourceTree({
              root: String(body.root ?? ""),
              query: stringOrNull(body.query),
              pageSize: numberOrNull(body.pageSize),
              cursor: numberOrNull(body.cursor),
              includeExcluded: Boolean(body.includeExcluded),
            })
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
    relationalSelectorPurgePlugin(),
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
    fs: {
      allow: [process.cwd(), realpathSync("node_modules")],
    },
    watch: {
      ignored: ["**/.svelte-kit/generated/**"],
    },
  },
  clearScreen: false,
});
