import editorWorkerUrl from "monaco-editor/esm/vs/editor/editor.worker?worker&url";
import extensionHostWorkerUrl from "@codingame/monaco-vscode-api/workers/extensionHost.worker?url";
import textMateWorkerUrl from "@codingame/monaco-vscode-textmate-service-override/worker?worker&url";
import jsonWorkerUrl from "@codingame/monaco-vscode-standalone-json-language-features/worker?worker&url";
import typeScriptWorkerUrl from "@codingame/monaco-vscode-standalone-typescript-language-features/worker?worker&url";

type MonacoWorkerEnvironment = {
  getWorker?: (moduleId: string, label: string) => Worker;
  getWorkerUrl?: (moduleId: string, label: string) => string | undefined;
  getWorkerOptions?: (
    moduleId: string,
    label: string
  ) => WorkerOptions | undefined;
  vscodeApiInitialised?: boolean;
  [key: string]: unknown;
};

function workerUrlFor(label: string): string | undefined {
  switch (label) {
    case "webWorkerExtensionHostIframe":
      // Vite 8 returns HTTP 500 for the package's pnpm-nested HTML URL in dev.
      // The self-contained, version-pinned copy is shipped with the native app.
      return "/vscode/webWorkerExtensionHostIframe.html";
    case "extensionHostWorkerMain":
      return extensionHostWorkerUrl;
    case "TextMateWorker":
      return textMateWorkerUrl;
    case "json":
      return jsonWorkerUrl;
    case "typescript":
    case "javascript":
      return typeScriptWorkerUrl;
    default:
      return editorWorkerUrl;
  }
}

/**
 * Give Monaco URLs that Vite has resolved and emitted itself. The default
 * monaco-languageclient factory leaves bare package names inside `new URL()`;
 * Vite 8 serves those as paths beneath monaco-languageclient and returns 404.
 */
export function configureMonacoWorkers(): void {
  const target = self as unknown as {
    MonacoEnvironment?: MonacoWorkerEnvironment;
  };
  const existing = target.MonacoEnvironment ?? {};
  target.MonacoEnvironment = {
    ...existing,
    getWorkerUrl: (_moduleId, label) => workerUrlFor(label),
    getWorkerOptions: () => ({ type: "module" }),
  };
}

/**
 * The extended VS Code service layer replaces Monaco's standalone theme
 * service. Its theme service deliberately supports VS Code themes, not
 * `monaco.editor.defineTheme`, so callers must not mix the two APIs.
 */
export function monacoVscodeApiIsInitialized(): boolean {
  return (
    (
      globalThis as typeof globalThis & {
        MonacoEnvironment?: MonacoWorkerEnvironment;
      }
    ).MonacoEnvironment?.vscodeApiInitialised === true
  );
}
