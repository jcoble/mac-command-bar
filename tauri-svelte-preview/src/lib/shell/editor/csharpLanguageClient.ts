import "vscode/localExtensionHost";
import "@codingame/monaco-vscode-csharp-default-extension";
import "../extensions/languageContributions";

import { LogLevel } from "@codingame/monaco-vscode-api";
import getFileServiceOverride, {
  RegisteredFileSystemProvider,
  RegisteredMemoryFile,
  registerFileSystemOverlay,
} from "@codingame/monaco-vscode-files-service-override";
import getKeybindingsServiceOverride from "@codingame/monaco-vscode-keybindings-service-override";
import {
  LanguageClientWrapper,
  type LanguageClientConfig,
} from "monaco-languageclient/lcwrapper";
import {
  MonacoVscodeApiWrapper,
  type MonacoVscodeApiConfig,
} from "monaco-languageclient/vscodeApiWrapper";
import * as vscode from "vscode";

import {
  ensureNativeCsharpLanguageClientFromTauri,
  isNativeTauriRuntime,
  markNativeCsharpLanguageClientReadyFromTauri,
} from "$lib/tauriSource";
import {
  nativeCsharpPathIsWithinRoot,
  registerNativeCsharpFileSystem,
} from "./csharpFileSystem";
import { configureMonacoWorkers } from "./monacoWorkers";
import { recordNativeCsharpDiagnostics } from "../problems/csharpDiagnosticsAdapter";
import { currentTheme, registerMonacoApplier } from "../themes/themeService";
import {
  registerCuratedExtensions,
  waitForCuratedExtensions,
} from "../extensions/extensionRuntime";
import { markRustGitScmApiReady } from "../extensions/rustGitScmProvider";

const MAX_WARM_CSHARP_ROOTS = 5;
const BUILD_COMMAND = "mcb.nativeCsharp.build";
const TEST_COMMAND = "mcb.nativeCsharp.test";
const PEEK_REFERENCES_COMMAND = "roslyn.client.peekReferences";
// The standard language client registers workspace/codeLens/refresh. Keeping
// the method named here makes the required protocol capability auditable.
export const NATIVE_CSHARP_CODE_LENS_REFRESH_METHOD =
  "workspace/codeLens/refresh";

export type NativeCsharpDiagnostic = {
  root: string;
  path: string;
  severity: "error" | "warning" | "info" | "hint";
  message: string;
  line: number;
  column: number;
  source?: string;
};

export type NativeCsharpEnsureResult = {
  root: string;
  reused: boolean;
  elapsedMs: number;
};

export type NativeCsharpDocumentResult = NativeCsharpEnsureResult & {
  path: string;
  documentUri: string;
};

type ClientSlot = {
  root: string;
  wrapper: LanguageClientWrapper;
  codeLensDisposable: vscode.Disposable;
  lastUsed: number;
};

type DocumentAction = (documentUri: string) => void | Promise<void>;

const pending = new Map<string, Promise<NativeCsharpEnsureResult>>();
const clients = new Map<string, ClientSlot>();
const diagnosticsListeners = new Set<
  (diagnostics: NativeCsharpDiagnostic[]) => void
>();
let apiReady: Promise<void> | null = null;
let useSequence = 0;
let buildAction: DocumentAction | null = null;
let testAction: DocumentAction | null = null;
let commandsRegistered = false;
const browserWorkspaceRoots = new Set<string>();
const browserWorkspaceFileSystem = new RegisteredFileSystemProvider(true);
let browserWorkspaceOverlayRegistered = false;
let vscodeThemeApplierRegistered = false;

function vscodeEditorConfiguration() {
  const theme = currentTheme();
  const configuration: Record<string, unknown> = {
    ...PROTOTYPE_EDITOR_CONFIGURATION,
    "workbench.colorTheme":
      theme.id === "houston" ? "Houston" : "Default Dark Modern",
  };

  if (theme.id === "houston") {
    // Houston is a real VS Code theme contribution. The prototype's hand-made
    // Monaco overrides reproduce an older approximation and, when kept here,
    // win after the extension theme loads: users see Houston flash and then
    // get repainted with the old palette. Let the extension own all editor,
    // TextMate, and semantic-token colors when Houston is selected.
    delete configuration["workbench.colorCustomizations"];
    delete configuration["editor.tokenColorCustomizations"];
    delete configuration["editor.semanticTokenColorCustomizations"];
  }

  return configuration;
}

function registerVscodeThemeApplier(): void {
  if (vscodeThemeApplierRegistered) return;
  vscodeThemeApplierRegistered = true;
  registerMonacoApplier((theme) => {
    if (!apiReady) return;
    const useExtensionTheme = theme.id === 'houston';
    void vscode.workspace.getConfiguration().update(
      'workbench.colorTheme',
      useExtensionTheme ? 'Houston' : 'Default Dark Modern',
      vscode.ConfigurationTarget.Global
    );
    void vscode.workspace.getConfiguration().update(
      'workbench.colorCustomizations',
      useExtensionTheme ? undefined : { ...theme.monaco.colors },
      vscode.ConfigurationTarget.Global
    );
    void vscode.workspace.getConfiguration().update(
      'editor.tokenColorCustomizations',
      useExtensionTheme ? undefined : {
        textMateRules: theme.monaco.rules.map((rule) => ({
          scope: rule.token.split(','),
          settings: { foreground: rule.foreground, fontStyle: rule.fontStyle }
        }))
      },
      vscode.ConfigurationTarget.Global
    );
    void vscode.workspace.getConfiguration().update(
      'editor.semanticTokenColorCustomizations',
      useExtensionTheme
        ? undefined
        : PROTOTYPE_EDITOR_CONFIGURATION['editor.semanticTokenColorCustomizations'],
      vscode.ConfigurationTarget.Global
    );
  });
}

const PROTOTYPE_EDITOR_CONFIGURATION = {
  "workbench.colorTheme": "Default Dark Modern",
  "workbench.colorCustomizations": {
    "editor.background": "#111820",
    "editor.foreground": "#DCE6F0",
    "editor.lineHighlightBackground": "#1B2733",
    "editor.selectionBackground": "#315675",
    "editor.inactiveSelectionBackground": "#263C4E",
    "editorLineNumber.foreground": "#657487",
    "editorLineNumber.activeForeground": "#E5EDF5",
    "editorCursor.foreground": "#8FC7E8",
    "editorCodeLens.foreground": "#8FC7E8",
    "editorLink.activeForeground": "#B7DCF2",
    "editorIndentGuide.background1": "#273441",
    "editorIndentGuide.activeBackground1": "#4E687D",
    "editorBracketHighlight.foreground1": "#78C5D6",
    "editorBracketHighlight.foreground2": "#E4B86A",
    "editorBracketHighlight.foreground3": "#D692C2",
    "editorWidget.background": "#17212B",
    "editorWidget.border": "#405466",
    "list.hoverBackground": "#22313E",
    "list.activeSelectionBackground": "#31506A",
    "list.activeSelectionForeground": "#F4F8FC",
    "peekView.border": "#579BC2",
    "peekViewTitle.background": "#1B2834",
    "peekViewTitleLabel.foreground": "#F2F6FA",
    "peekViewTitleDescription.foreground": "#9FB0C2",
    "peekViewEditor.background": "#121B24",
    "peekViewEditor.matchHighlightBackground": "#6B5428",
    "peekViewResult.background": "#0E161E",
    "peekViewResult.fileForeground": "#C9D7E5",
    "peekViewResult.lineForeground": "#AFC0D0",
    "peekViewResult.matchHighlightBackground": "#6B5428",
    "peekViewResult.selectionBackground": "#294C65",
    "peekViewResult.selectionForeground": "#FFFFFF",
  },
  "editor.codeLens": true,
  "editor.codeLensFontFamily":
    "ui-monospace, SFMono-Regular, Menlo, monospace",
  "editor.fontFamily": "ui-monospace, SFMono-Regular, Menlo, monospace",
  "editor.fontSize": 14,
  "editor.lineHeight": 23,
  "editor.semanticHighlighting.enabled": true,
  "editor.bracketPairColorization.enabled": true,
  "editor.guides.bracketPairs": "active",
  "editor.minimap.enabled": false,
  "editor.renderWhitespace": "selection",
  "editor.wordBasedSuggestions": "off",
  "editor.semanticTokenColorCustomizations": {
    enabled: true,
    rules: {
      namespace: "#9EB4C8",
      class: "#78C5D6",
      struct: "#78C5D6",
      interface: "#78C5D6",
      enum: "#78C5D6",
      typeParameter: "#8ED7C6",
      method: "#75BEFF",
      property: "#D7C37A",
      parameter: "#E7B983",
      variable: "#DCE6F0",
      keyword: "#E58AB8",
      string: "#A9D27A",
      number: "#E8C170",
    },
  },
  "editor.tokenColorCustomizations": {
    textMateRules: [
      {
        scope: ["comment", "punctuation.definition.comment"],
        settings: { foreground: "#78899B", fontStyle: "italic" },
      },
      {
        scope: ["keyword", "storage.type", "storage.modifier"],
        settings: { foreground: "#E58AB8" },
      },
      {
        scope: [
          "entity.name.type",
          "entity.name.class",
          "entity.name.interface",
          "support.type",
        ],
        settings: { foreground: "#78C5D6" },
      },
      {
        scope: ["entity.name.function", "support.function"],
        settings: { foreground: "#75BEFF" },
      },
      {
        scope: ["string", "string.quoted"],
        settings: { foreground: "#A9D27A" },
      },
      {
        scope: ["constant.numeric", "constant.language"],
        settings: { foreground: "#E8C170" },
      },
    ],
  },
  "dotnet.codeLens.enableReferencesCodeLens": true,
  "dotnet.codeLens.enableTestsCodeLens": false,
};

function normalizedPath(path: string): string {
  return path.replaceAll("\\", "/").replace(/\/+$/, "");
}

function registerBrowserWorkspaceRoot(root: string): void {
  if (!browserWorkspaceOverlayRegistered) {
    browserWorkspaceOverlayRegistered = true;
    registerFileSystemOverlay(1, browserWorkspaceFileSystem);
  }
  if (browserWorkspaceRoots.has(root)) return;
  browserWorkspaceRoots.add(root);
  browserWorkspaceFileSystem.registerFile(
    new RegisteredMemoryFile(
      vscode.Uri.file(`${root}/.mcb-browser-workspace`),
      ""
    )
  );
}

function severityOf(
  severity: vscode.DiagnosticSeverity
): NativeCsharpDiagnostic["severity"] {
  switch (severity) {
    case vscode.DiagnosticSeverity.Error:
      return "error";
    case vscode.DiagnosticSeverity.Warning:
      return "warning";
    case vscode.DiagnosticSeverity.Hint:
      return "hint";
    default:
      return "info";
  }
}

function diagnosticsForRoot(root: string): NativeCsharpDiagnostic[] {
  const diagnostics: NativeCsharpDiagnostic[] = [];
  for (const [uri, entries] of vscode.languages.getDiagnostics()) {
    if (
      uri.scheme !== "file" ||
      !nativeCsharpPathIsWithinRoot(root, uri.fsPath)
    )
      continue;
    for (const diagnostic of entries) {
      diagnostics.push({
        root,
        path: uri.fsPath,
        severity: severityOf(diagnostic.severity),
        message: diagnostic.message,
        line: diagnostic.range.start.line + 1,
        column: diagnostic.range.start.character + 1,
        source: diagnostic.source,
      });
    }
  }
  return diagnostics;
}

function publishDiagnostics(): void {
  for (const root of clients.keys()) {
    const diagnostics = diagnosticsForRoot(root);
    recordNativeCsharpDiagnostics(root, diagnostics);
    for (const listener of diagnosticsListeners) listener(diagnostics);
  }
}

function registerDocumentActions(): void {
  if (commandsRegistered) return;
  commandsRegistered = true;
  vscode.commands.registerCommand(BUILD_COMMAND, (documentUri: string) =>
    buildAction?.(documentUri)
  );
  vscode.commands.registerCommand(TEST_COMMAND, (documentUri: string) =>
    testAction?.(documentUri)
  );
  vscode.commands.registerCommand(
    PEEK_REFERENCES_COMMAND,
    async (
      uriValue: string,
      positionValue: { line: number; character: number }
    ) => {
      const uri = vscode.Uri.parse(uriValue);
      const position = new vscode.Position(
        positionValue.line,
        positionValue.character
      );
      const references =
        (await vscode.commands.executeCommand<vscode.Location[]>(
          "vscode.executeReferenceProvider",
          uri,
          position
        )) ?? [];
      await vscode.commands.executeCommand(
        "editor.action.showReferences",
        uri,
        position,
        references
      );
    }
  );
  vscode.languages.onDidChangeDiagnostics(publishDiagnostics);
}

async function ensureApi(root: string): Promise<void> {
  if (isNativeTauriRuntime()) {
    registerNativeCsharpFileSystem(root);
  } else {
    registerBrowserWorkspaceRoot(root);
  }
  if (!apiReady) {
    apiReady = (async () => {
      registerCuratedExtensions();
      const workspaceUri = vscode.Uri.file(root);
      const config: MonacoVscodeApiConfig = {
        $type: "extended",
        viewsConfig: { $type: "EditorService" },
        serviceOverrides: {
          ...getFileServiceOverride(),
          ...getKeybindingsServiceOverride(),
        },
        logLevel: LogLevel.Info,
        workspaceConfig: {
          workspaceProvider: {
            trusted: true,
            workspace: { folderUri: workspaceUri },
            async open() {
              return true;
            },
          },
        },
        userConfiguration: {
          json: JSON.stringify(vscodeEditorConfiguration()),
        },
        monacoWorkerFactory: configureMonacoWorkers,
      };
      await new MonacoVscodeApiWrapper(config).start();
      await waitForCuratedExtensions();
      markRustGitScmApiReady();
      registerVscodeThemeApplier();
      registerDocumentActions();
    })();
  }
  await apiReady;

  const workspaceUri = vscode.Uri.file(root).toString();
  if (
    !vscode.workspace.workspaceFolders?.some(
      (folder) => folder.uri.toString() === workspaceUri
    )
  ) {
    vscode.workspace.updateWorkspaceFolders(
      vscode.workspace.workspaceFolders?.length ?? 0,
      0,
      { uri: vscode.Uri.file(root), name: root.split("/").pop() || root }
    );
  }
}

/**
 * Initialize the VS Code-compatible Monaco services before the editor imports
 * or creates a standalone model. The service container is global and can only
 * be initialized once, so this is deliberately safe to call for every root.
 */
export async function prepareNativeCsharpEditorServices(
  root: string
): Promise<void> {
  await ensureApi(normalizedPath(root));
}

async function evictColdRootIfNeeded(): Promise<void> {
  if (clients.size < MAX_WARM_CSHARP_ROOTS) return;
  const oldest = [...clients.values()].sort(
    (left, right) => left.lastUsed - right.lastUsed
  )[0];
  if (!oldest) return;
  clients.delete(oldest.root);
  oldest.codeLensDisposable.dispose();
  await oldest.wrapper.dispose();
  recordNativeCsharpDiagnostics(oldest.root, []);
}

async function startClient(
  requestedRoot: string,
  startedAt: number
): Promise<NativeCsharpEnsureResult> {
  // This must precede both the WebSocket client and any Monaco editor/model.
  // `registerCustomProvider` refuses to run after standalone services exist.
  await ensureApi(requestedRoot);
  const endpoint = await ensureNativeCsharpLanguageClientFromTauri(
    requestedRoot
  );
  if (!endpoint)
    throw new Error("Native C# is available only in the Tauri desktop app.");
  const root = normalizedPath(endpoint.root);
  const existing = clients.get(root);
  if (existing) {
    existing.lastUsed = ++useSequence;
    return { root, reused: true, elapsedMs: performance.now() - startedAt };
  }
  await evictColdRootIfNeeded();
  const workspaceUri = vscode.Uri.file(root);
  const workspaceFolder =
    vscode.workspace.workspaceFolders?.find(
      (folder) => folder.uri.toString() === workspaceUri.toString()
    ) ??
    ({
      index: vscode.workspace.workspaceFolders?.length ?? 0,
      name: root.split("/").pop() || root,
      uri: workspaceUri,
    } satisfies vscode.WorkspaceFolder);
  const selector = {
    language: "csharp",
    scheme: "file",
    pattern: new vscode.RelativePattern(workspaceFolder, "**/*.cs"),
  };
  const withinRoot = (uri: vscode.Uri) =>
    uri.scheme === "file" && nativeCsharpPathIsWithinRoot(root, uri.fsPath);
  const config: LanguageClientConfig = {
    languageId: "csharp",
    connection: {
      options: {
        $type: "WebSocketUrl",
        url: endpoint.wsUrl,
        startOptions: { onCall() {}, reportStatus: true },
        stopOptions: { onCall() {}, reportStatus: true },
      },
    },
    clientOptions: {
      documentSelector: [selector as never],
      workspaceFolder,
      middleware: {
        provideCodeLenses: (document, token, next) =>
          withinRoot(document.uri) ? next(document, token) : [],
        resolveCodeLens: (codeLens, token, next) => next(codeLens, token),
        handleDiagnostics: (uri, diagnostics, next) => {
          if (withinRoot(uri)) next(uri, diagnostics);
        },
        workspace: {
          configuration: async (params, token, next) => {
            const fallbackResult = await next(params, token);
            const fallback = Array.isArray(fallbackResult)
              ? fallbackResult
              : [];
            return params.items.map((item, index) => {
              if (
                item.section ===
                "csharp|code_lens.dotnet_enable_references_code_lens"
              )
                return true;
              if (
                item.section ===
                "csharp|code_lens.dotnet_enable_tests_code_lens"
              )
                return false;
              return fallback[index] ?? null;
            });
          },
        },
      },
    },
  };
  const wrapper = new LanguageClientWrapper(config);
  await wrapper.start();
  await markNativeCsharpLanguageClientReadyFromTauri(root);
  const codeLensDisposable = vscode.languages.registerCodeLensProvider(
    selector,
    {
      provideCodeLenses(document) {
        if (!withinRoot(document.uri)) return [];
        const range = new vscode.Range(0, 0, 0, 0);
        const documentUri = document.uri.toString();
        return [
          new vscode.CodeLens(range, {
            title: "$(tools) Build project",
            command: BUILD_COMMAND,
            arguments: [documentUri],
          }),
          new vscode.CodeLens(range, {
            title: "$(beaker) Test project",
            command: TEST_COMMAND,
            arguments: [documentUri],
          }),
        ];
      },
    }
  );
  clients.set(root, {
    root,
    wrapper,
    codeLensDisposable,
    lastUsed: ++useSequence,
  });
  publishDiagnostics();
  return { root, reused: false, elapsedMs: performance.now() - startedAt };
}

export function ensureNativeCsharpLanguageClient(
  root: string
): Promise<NativeCsharpEnsureResult> {
  if (!isNativeTauriRuntime()) {
    return Promise.reject(
      new Error("Native C# is available only in the Tauri desktop app.")
    );
  }
  const key = normalizedPath(root);
  const existing = clients.get(key);
  if (existing) {
    existing.lastUsed = ++useSequence;
    return Promise.resolve({ root: key, reused: true, elapsedMs: 0 });
  }
  const inFlight = pending.get(key);
  if (inFlight) return inFlight;
  const startedAt = performance.now();
  const promise = startClient(key, startedAt).finally(() =>
    pending.delete(key)
  );
  pending.set(key, promise);
  return promise;
}

/**
 * Attach a real file:// document to the VS Code document service. Creating a
 * Monaco model alone does not fire workspace.onDidOpenTextDocument, which is
 * the event vscode-languageclient uses to send textDocument/didOpen to Roslyn.
 */
export async function ensureNativeCsharpDocument(
  root: string,
  path: string
): Promise<NativeCsharpDocumentResult> {
  const ensured = await ensureNativeCsharpLanguageClient(root);
  const normalizedDocumentPath = normalizedPath(path);
  if (!nativeCsharpPathIsWithinRoot(ensured.root, normalizedDocumentPath)) {
    throw new Error(
      "The active C# document is outside its Roslyn workspace root."
    );
  }

  const fileUri = vscode.Uri.file(normalizedDocumentPath);
  let document = await vscode.workspace.openTextDocument(fileUri);
  if (document.languageId !== "csharp") {
    document = await vscode.languages.setTextDocumentLanguage(
      document,
      "csharp"
    );
  }

  return {
    ...ensured,
    path: normalizedDocumentPath,
    documentUri: document.uri.toString(),
  };
}

export function setNativeCsharpDocumentActions(actions: {
  build: DocumentAction;
  test: DocumentAction;
}): () => void {
  buildAction = actions.build;
  testAction = actions.test;
  return () => {
    if (buildAction === actions.build) buildAction = null;
    if (testAction === actions.test) testAction = null;
  };
}

export function subscribeNativeCsharpDiagnostics(
  listener: (diagnostics: NativeCsharpDiagnostic[]) => void
): () => void {
  diagnosticsListeners.add(listener);
  publishDiagnostics();
  return () => diagnosticsListeners.delete(listener);
}

export function nativeCsharpClientIsWarm(root: string): boolean {
  return clients.has(normalizedPath(root));
}
