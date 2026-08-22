<script lang="ts">
  /**
   * EditorPanel.svelte — the /next code-reading panel.
   *
   * Thin by construction. It owns three things and nothing else:
   *  1. the strip of open files,
   *  2. reading a file when one is requested, and
   *  3. handing `CodeMirrorSourceEditor` the lookup callbacks from
   *     `sourceIntelligence`.
   *
   * Rules it exists to keep:
   *  - **One editor, ever.** Switching files swaps the document inside one
   *    CodeMirror view, so inactive tabs hold only their saved text and view state.
   *  - **Nothing loads at start-up.** The panel subscribes to open-file
   *    requests when it mounts (free, no backend), and CodeMirror itself is only
   *    downloaded once a file is on screen in front of the reader — not merely
   *    in the strip, because putting a session's files back fills the strip out
   *    of sight. The language server is warmed on the first file opened per
   *    project, never before.
   *  - **Read mode is the default.** Opening a file colours it and stops
   *    there. A language server starts only for a project whose switch in the
   *    top strip has been turned on, and turning it off stops that server. The
   *    choice belongs to the project — one server serves every session and
   *    every view on it — and is remembered between launches.
   *  - **No `$effect` reads a file.** Every read is started by a user action: a
   *    file-open request, or a click in the strip. The one effect that starts
   *    anything starts the editor, and only for a file already on screen.
   */
  import { onMount } from 'svelte';
  import X from '@lucide/svelte/icons/x';
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';

  import { upgradeUnknownLanguage } from './editor/editorLanguage.ts';
  import {
    createLanguageServerGate,
    readLanguageServerState,
    statusMessageIsAboutThisFile
  } from './editor/languageServerStatus.ts';
  import FileIcon from './explorer/FileIcon.svelte';
  import { canonicalPath } from './explorer/explorerStore.svelte';
  import {
    isMarkdownFile,
    markdownPreviewDefault,
    type MarkdownView
  } from './editor/markdownPreview.ts';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import SourceMarkdownPreview from '$lib/SourceMarkdownPreview.svelte';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import {
    languageIntelligenceOn,
    LANGUAGE_INTELLIGENCE_SETTING_KEY,
    normalizeLanguageIntelligenceChoices,
    withLanguageIntelligenceChoice,
    workspaceKey,
    type LanguageIntelligenceChoices
  } from '$lib/shell/editor/languageIntelligenceMode';
  import {
    clearLanguageIntelligenceBar,
    publishLanguageIntelligenceBar,
    setLanguageIntelligenceSwitch
  } from '$lib/shell/editor/languageIntelligenceBar.svelte';
  import { onOpenFile, type OpenFileRequest } from '$lib/shell/openFileBus';
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import { countInvoke } from '$lib/shell/devInvokeCounter.svelte';
  import {
    activateEditor,
    activeEditorFile,
    clearEditorFileLoading,
    closeEditorFile,
    editorFileFor,
    editorState,
    markEditorFileLoading,
    openEditorFile,
    pinEditorFile,
    resetEditorState,
    revealEditorLine,
    setActiveEditorFile,
    setEditorFileDraft,
    setEditorFileError,
    setEditorFilePreview,
    setEditorFileSaving,
    setEditorSymbols
  } from '$lib/shell/editor/editorStore.svelte';
  import { modelPathToDisposeOnClose, needsRead } from '$lib/shell/editor/editorStoreOps';
  import {
    sourceIntelligence,
    type SourceInlayHintRequest
  } from '$lib/shell/editor/sourceIntelligence';
  import { settings } from '$lib/settingsStore.svelte';
  import { sourceRecordFromPath } from '$lib/shell/editor/sourceRecordFromPath';
  import { openFileTimeline } from '$lib/shell/workbenchNavigation';
  import {
    isNativeTauriRuntime,
    readAssemblySettingFromTauri,
    readSourceFromTauri,
    readSourceLspStatusFromTauri,
    setWorkspaceLanguageIntelligenceFromTauri,
    warmSourceLspForRootFromTauri,
    writeAssemblySettingFromTauri,
    writeSourceToTauri
  } from '$lib/tauriSource';
  import type CodeMirrorSourceEditor from '$lib/CodeMirrorSourceEditor.svelte';
  import type {
    SourceDiagnostic,
    SourceInlayHint,
    SourceRecord,
    SourceSymbol
  } from '$lib/sourceData';

  /**
   * The code editor is a large download, so it is fetched with the first file
   * the user opens rather than with the shell. The import here is types only —
   * it adds nothing to the page — and the real module arrives in
   * `ensureCodeEditor`.
   */
  interface Props {
    /** Called when an open-file request has become a real open, so the shell can
     * bring this panel's tab to the front. The panel itself stays unaware of the
     * tab area — it just says a file arrived. */
    onFileOpened?: () => void;
    /** Whether this panel is the center tab in front. Putting a session's files
     * back opens them out of sight, and the code editor is far too expensive to
     * start for a file nobody is looking at. */
    showing?: boolean;
    /** False when the selected session's checkout no longer exists. */
    rootAvailable?: boolean;
    /** Clears every session's saved editor strip after the close outcome succeeds. */
    onCloseAllEditors?: () => void | Promise<boolean | void>;
  }
  let { onFileOpened, showing = false, rootAvailable = true, onCloseAllEditors }: Props = $props();

  type CodeEditorComponent = typeof CodeMirrorSourceEditor;
  let CodeEditor = $state<CodeEditorComponent | null>(null);
  let codeEditor = $state<{
    captureViewStates(paths: readonly string[]): Record<string, object>;
    disposeTabModel(path: string): boolean;
    disposeAllTabModels(): void;
    releaseSessionResources(): void;
  } | null>(null);
  let editorLoadError = $state<string | null>(null);
  let loadingEditorComponent = false;
  type CloseRequest = { kind: 'file'; path: string } | { kind: 'all' };
  let closeRequest = $state<CloseRequest | null>(null);
  let closeDialogOpen = $state(false);
  let closeActionBusy = $state(false);

  /** Paths whose read is in flight, so a double click cannot read twice. */
  const readsInFlight = new Set<string>();
  let sessionResourceGeneration = 0;
  let restoredViewStates = $state<Record<string, object>>({});
  /** Projects whose language server has already been pointed at the project. */
  const warmedProjectRoots = new Set<string>();

  /** The remembered choices. SQLite hydrates them once; every change writes them back. */
  let languageIntelligenceChoices = $state<LanguageIntelligenceChoices>({});
  let languageIntelligenceHydrated = false;
  let languageIntelligenceHydration: Promise<void> | null = null;
  /** What the desktop app last said about this project's mode. */
  let languageIntelligenceNote = $state<string | null>(null);
  /** True while a switch is being acted on, so it cannot be flipped twice. */
  let languageIntelligenceBusy = $state(false);
  let nativeCsharpRoot: string | null = null;
  let backendOwnerRoot: string | null = null;
  let requestedOwnerKey: string | null = null;
  let ownerSelectionGeneration = 0;
  let ownerTransitionTail: Promise<void> = Promise.resolve();
  let currentOwnerTransition: ReturnType<typeof setWorkspaceLanguageIntelligenceFromTauri> =
    Promise.resolve(null);
  /** Language-server processes the desktop app reports for this project. */
  let languageServerPids = $state<number[]>([]);
  let destroyed = false;

  function hydrateLanguageIntelligenceChoices(): Promise<void> {
    if (languageIntelligenceHydrated) return Promise.resolve();
    if (languageIntelligenceHydration) return languageIntelligenceHydration;
    const hydration = readAssemblySettingFromTauri(LANGUAGE_INTELLIGENCE_SETTING_KEY)
      .then((stored) => {
        if (!destroyed) {
          languageIntelligenceChoices = normalizeLanguageIntelligenceChoices(stored);
        }
        languageIntelligenceHydrated = true;
      })
      .finally(() => {
        if (languageIntelligenceHydration === hydration) {
          languageIntelligenceHydration = null;
        }
      });
    languageIntelligenceHydration = hydration;
    return hydration;
  }

  const activeFile = $derived(activeEditorFile());
  const activeFileMissing = $derived(
    Boolean(activeFile?.error && /(?:os error 2|No such file)/i.test(activeFile.error))
  );
  /**
   * Files opened for reading only, keyed by path. A file link in a conversation
   * that points outside the workspace opens this way: the reader can see what
   * it pointed at, but the session does not own the file, so nothing here may
   * edit or save over it.
   */
  let readOnlyByPath = $state<Record<string, boolean>>({});
  const activeFileReadOnly = $derived(Boolean(activeFile && readOnlyByPath[activeFile.path]));
  const activeServerEnabled = $derived.by(() => {
    const language = activeFile?.language?.toLowerCase();
    if (language === 'csharp' || language === 'c#') return settings.intelligence.languageServerEnabled.csharp;
    if (language === 'typescript' || language === 'javascript' || language === 'tsx' || language === 'jsx') {
      return settings.intelligence.languageServerEnabled.typescript;
    }
    if (language === 'rust') return settings.intelligence.languageServerEnabled.rust;
    return null;
  });
  /** Is this project in full mode — language server allowed to run? Settings
   * can switch every server off at once, and then no project is, whatever its
   * own switch says: the switch reads Off, and its title says which one held. */
  const fullMode = $derived(
    !activeFileReadOnly
      && settings.intelligence.languageServers
      && activeServerEnabled === true
  );
  /**
   * The sentence on hover: what the mode means for this project, and the
   * process numbers behind it so they can be found in the resource view.
   */
  const languageIntelligenceTitle = $derived.by(() => {
    if (!editorState.projectRoot) return 'Open a file in a project to switch this on.';
    if (!settings.intelligence.languageServers) {
      return 'Editor-only — Supercharged is switched off in Settings.';
    }
    if (activeServerEnabled === false) {
      return 'Editor-only — this language server is switched off in Settings.';
    }
    if (activeFile?.language && activeServerEnabled === null) {
      return `Editor-only — no language server for ${activeFile.language}.`;
    }
    const note =
      languageIntelligenceNote ??
      (fullMode
        ? 'Language intelligence is on for this project. One language server serves every session and view on it.'
        : 'Read mode: files open with colouring only, and no language server is started.');
    if (languageServerPids.length === 0) return note;
    const numbers = languageServerPids.join(', ');
    const noun = languageServerPids.length === 1 ? 'Process' : 'Processes';
    return `${note} ${noun} ${numbers}.`;
  });
  /**
   * Which view each Markdown file is on. Markdown opens source-first; the only
   * way to write `rendered` here is the active file's explicit Preview toggle.
   * Leaving a file releases that preview choice instead of keeping rendered DOM
   * logically hidden behind the strip.
   */
  let markdownViewByPath = $state<Record<string, MarkdownView>>({});
  const activeFileIsMarkdown = $derived(isMarkdownFile(activeFile?.fileName));
  const markdownView = $derived(
    activeFile && activeFileIsMarkdown ? (markdownViewByPath[activeFile.path] ?? 'raw') : 'raw'
  );
  const MARKDOWN_VIEW_ITEMS = [
    { value: 'raw', label: 'Source' },
    { value: 'rendered', label: 'Preview' }
  ] as const;

  /** Record the first view for a file, without overwriting a person's choice. */
  function rememberMarkdownDefault(path: string, fileName: string, origin: 'jump' | 'strip'): void {
    if (!isMarkdownFile(fileName) || markdownViewByPath[path]) return;
    markdownViewByPath = {
      ...markdownViewByPath,
      [path]: markdownPreviewDefault(fileName, origin)
    };
  }

  function setMarkdownView(view: MarkdownView): void {
    const path = activeFile?.path;
    if (!path) return;
    markdownViewByPath = { ...markdownViewByPath, [path]: view };
  }

  function releaseMarkdownView(path: string): void {
    if (!markdownViewByPath[path]) return;
    const { [path]: _released, ...rest } = markdownViewByPath;
    markdownViewByPath = rest;
  }

  /**
   * What the language server says is wrong, per file. Kept per file rather than
   * for "the file on screen" because switching tabs must not show the last
   * file's squiggles on this one while a fresh read is still in flight.
   */
  let diagnosticsByPath = $state<Record<string, SourceDiagnostic[]>>({});
  /**
   * The one empty answer, reused. The fallback below must keep a stable
   * identity: a fresh `[]` per template evaluation reads as a changed prop
   * to the editor's effect and can spin it in a loop.
   */
  const NO_DIAGNOSTICS: SourceDiagnostic[] = [];
  /**
   * How long to wait before asking a second time, in milliseconds.
   *
   * LOAD-BEARING. The language server answers a file it has only just opened
   * with an empty list, so the first read after a file lands usually comes back
   * with nothing and the squiggles appear only when something else happens to
   * ask again. The old shell has hidden this behind the same wait since it was
   * written (`src/routes/+page.svelte`, `scheduleSourceLspDiagnostics`). Without
   * it, "it worked when I clicked around" is the bug report.
   */
  const DIAGNOSTICS_SETTLE_MS = 650;
  /** The pending second read, so switching files quickly does not queue several. */
  let diagnosticsTimer: ReturnType<typeof setTimeout> | null = null;

  /**
   * The last thing the desktop app said about the open file's language server —
   * either the answer to `read_source_lsp_status` or a pushed
   * pushed status message. Both carry the same `state` and
   * `detail` fields, so either one can be shown as-is.
   *
   * `null` means nobody has said anything: a browser tab (there is no language
   * server behind a browser) or a desktop build older than these fields. In
   * both cases no chip appears and the panel behaves exactly as it used to.
   */
  let languageServerStatus = $state<unknown>(null);
  /** Which project and language that answer was about, so a stale one is dropped. */
  let languageServerSubject: { root: string; language: string } | null = null;
  /**
   * Holds back work that a server which is still starting up or reading the
   * project could not answer anyway. It lets everything through the moment the
   * server says it is ready — and, if that never happens, on its own time limit.
   */
  const languageServerGate = createLanguageServerGate();
  /** Counts inline-hint requests, so only the newest one survives a wait. */
  let inlayHintRequestCount = 0;

  /** The language of the file on screen, or null when nothing is open. */
  function activeFileLanguage(): string | null {
    return activeEditorFile()?.language ?? null;
  }

  /** Record what the desktop app just said, and let waiting work know. */
  function applyLanguageServerStatus(
    status: unknown,
    subject: { root: string; language: string } | null
  ): void {
    languageServerStatus = status;
    languageServerSubject = subject;
    languageServerGate.setState(readLanguageServerState(status));
  }

  /**
   * Ask what the language server for this file is doing. Called when a file
   * opens or the strip selection changes — never on a timer. Everything after
   * that arrives as a pushed message.
   */
  async function refreshLanguageServerStatus(): Promise<void> {
    if (activeFileReadOnly) {
      if (languageServerStatus !== null || languageServerSubject !== null) {
        applyLanguageServerStatus(null, null);
      }
      return;
    }
    const root = editorState.projectRoot;
    const language = activeFileLanguage();

    // A different project or a different language means the last answer was
    // about someone else's server; drop it rather than show it against this file.
    if (
      languageServerSubject &&
      (languageServerSubject.root !== root || languageServerSubject.language !== language)
    ) {
      applyLanguageServerStatus(null, null);
    }

    if (!root || !language || !isNativeTauriRuntime()) return;

    countInvoke('read_source_lsp_status');
    let answer: unknown = null;
    try {
      answer = await readSourceLspStatusFromTauri(root, language);
    } catch {
      // No answer is not a state worth reporting, so the chip stays away.
      answer = null;
    }
    // Superseded: the user moved to another file while this was in flight.
    if (destroyed || editorState.projectRoot !== root || activeFileLanguage() !== language) return;
    applyLanguageServerStatus(answer, { root, language });
  }

  /** Apply status updates fanned out by the shared source-intelligence listener. */
  function handleLanguageServerStatus(status: unknown): void {
    if (activeFileReadOnly) return;
    const root = editorState.projectRoot;
    const language = activeFileLanguage();
    if (!root || !language) return;
    // Several servers can be running at once, so a message about another
    // project or another language must not move this file's chip.
    if (!statusMessageIsAboutThisFile(status, root, language)) return;
    applyLanguageServerStatus(status, { root, language });
  }

  /** Ask what is wrong with the file on screen, and remember it against that file. */
  async function loadDiagnosticsForActiveFile(): Promise<void> {
    if (activeFileReadOnly) return;
    const path = editorState.activePath;
    if (!path) return;
    const generation = sessionResourceGeneration;
    const diagnostics = await sourceIntelligence.loadActiveFileDiagnostics();
    // Superseded: the user moved on while this was in flight, so this answer is
    // about a file that is no longer on screen.
    if (destroyed || generation !== sessionResourceGeneration || editorState.activePath !== path) return;
    diagnosticsByPath = { ...diagnosticsByPath, [path]: diagnostics };
  }

  /** Read the diagnostics now, and once more after the server has settled. */
  function refreshDiagnosticsForActiveFile(): void {
    if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
    diagnosticsTimer = null;
    if (activeFileReadOnly) return;
    void loadDiagnosticsForActiveFile();
    scheduleSecondDiagnosticsRead();
  }

  /**
   * Line up the second read.
   *
   * While the server is still starting up or reading the project the second
   * read is pointless — it would come back empty for the same reason the first
   * one did — so it waits for the server to say it is ready and only then
   * starts the settle wait. When nothing is known about the server (an older
   * desktop build, or a browser tab) this is the original behaviour with no
   * delay of any kind added: the timer starts immediately, as it always did.
   */
  function scheduleSecondDiagnosticsRead(): void {
    if (!languageServerGate.isBusy()) {
      startDiagnosticsSettleTimer();
      return;
    }
    const path = editorState.activePath;
    void languageServerGate.waitUntilReady().then(() => {
      if (destroyed || editorState.activePath !== path) return;
      startDiagnosticsSettleTimer();
    });
  }

  function startDiagnosticsSettleTimer(): void {
    if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
    diagnosticsTimer = setTimeout(() => {
      diagnosticsTimer = null;
      if (!destroyed) void loadDiagnosticsForActiveFile();
    }, DIAGNOSTICS_SETTLE_MS);
  }

  /**
   * Inline type hints, held back until the server can actually answer.
   *
   * The editor asks for these the instant a file appears. A server that is
   * still reading the project answers "no hints", the editor believes it, and
   * nothing asks again — which is why hints used to be missing for the rest of
   * the session on a cold start. Making the request WAIT instead of answering
   * it emptily means the same request is answered properly once the server is
   * ready. With nothing known about the server this passes straight through.
   */
  async function lookupInlayHintsWhenServerCanAnswer(
    request: SourceInlayHintRequest
  ): Promise<SourceInlayHint[]> {
    if (activeFileReadOnly) return [];
    if (languageServerGate.isBusy()) {
      const path = editorState.activePath;
      const ticket = ++inlayHintRequestCount;
      await languageServerGate.waitUntilReady();
      // Only the newest request survives the wait. The editor asks again for
      // every scroll, so answering a whole queue of stale ranges at once would
      // simply move the pile-up to the end of the wait instead of removing it.
      if (destroyed || ticket !== inlayHintRequestCount || editorState.activePath !== path) {
        return [];
      }
    }
    return sourceIntelligence.callbacks.onInlayHintLookup(request);
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /**
   * The record for a path, with its language filled in when the usual mapping
   * did not recognise the file. SQL is the clearest case: `.sql` was not on the
   * old list, so those files opened as flat grey text even though the editor has
   * had the SQL colouring rules loaded all along. Files the old mapping already
   * knew are untouched.
   *
   * LOAD-BEARING ORDER: the record is what the read wrapper copies the language
   * back out of, so it has to be right before the file is read, not after.
   */
  function recordForPath(path: string, projectRoot = editorState.projectRoot): SourceRecord {
    const record = sourceRecordFromPath(projectRoot, path);
    const language = upgradeUnknownLanguage(record.path, record.language);
    return language === record.language ? record : { ...record, language };
  }

  /** Download the code editor the first time it is needed. */
  async function ensureCodeEditor(): Promise<void> {
    if (loadingEditorComponent) return;
    loadingEditorComponent = true;
    try {
      const root = editorState.projectRoot;
      // Wait until the first editor can receive the real, path-bearing root.
      if (!root) return;
      if (!CodeEditor) {
        const module = await import('$lib/CodeMirrorSourceEditor.svelte');
        if (!destroyed) CodeEditor = module.default;
      }
    } catch (error) {
      if (!destroyed) editorLoadError = `Could not start the code editor: ${describeError(error)}`;
    } finally {
      loadingEditorComponent = false;
    }
  }

  function queueLanguageIntelligenceOwner(
    projectRoot: string | null,
    enabled: boolean,
    language: string | null = null,
    force = false
  ): ReturnType<typeof setWorkspaceLanguageIntelligenceFromTauri> {
    const root = projectRoot ? workspaceKey(projectRoot) : null;
    const ownerKey = `${root ?? ''}|${enabled}`;
    if (!force && requestedOwnerKey === ownerKey) return currentOwnerTransition;
    requestedOwnerKey = ownerKey;

    const transition = ownerTransitionTail.then(async () => {
      if (!isNativeTauriRuntime()) return null;
      const commandRoot = root ?? backendOwnerRoot;
      if (!commandRoot) return null;
      if (backendOwnerRoot !== root) warmedProjectRoots.clear();
      countInvoke('set_workspace_language_intelligence');
      const answer = await setWorkspaceLanguageIntelligenceFromTauri(
        commandRoot,
        Boolean(root && enabled),
        root && enabled ? language : null
      );
      backendOwnerRoot = root && enabled ? root : null;
      return answer;
    });
    ownerTransitionTail = transition.then(() => undefined, () => undefined);
    currentOwnerTransition = transition;
    return transition;
  }

  /** Hand the ordered backend owner this project's current remembered choice. */
  function applySavedModeForProject(projectRoot: string): Promise<void> {
    const root = workspaceKey(projectRoot);
    return (async () => {
      await hydrateLanguageIntelligenceChoices();
      if (
        destroyed
        || !rootAvailable
        || !editorState.projectRoot
        || workspaceKey(editorState.projectRoot) !== root
      ) return;
      try {
        const answer = await queueLanguageIntelligenceOwner(
          root,
          settings.intelligence.languageServers
        );
        if (destroyed || !answer) return;
        if (editorState.projectRoot && workspaceKey(editorState.projectRoot) === root) {
          languageServerPids = answer.serverPids;
        }
      } catch {
        // A desktop build that has never heard of the switch leaves every
        // project in read mode, which is the safe half of the choice.
      }
    })();
  }

  /**
   * Tell the language server which project this file belongs to before warming
   * the file's language.
   */
  async function warmLanguageServer(projectRoot: string | null): Promise<void> {
    if (!projectRoot || activeServerEnabled !== true) return;
    try {
      await applySavedModeForProject(projectRoot);
    } catch (error) {
      if (!destroyed) {
        languageIntelligenceNote = `The saved editor mode could not be read: ${describeError(error)}`;
      }
      return;
    }
    if (destroyed || !settings.intelligence.languageServers) return;
    if (warmedProjectRoots.has(projectRoot)) return;
    warmedProjectRoots.add(projectRoot);
    countInvoke('warm_source_lsp_for_root');
    try {
      await warmSourceLspForRootFromTauri(projectRoot);
    } catch (error) {
      // Warming only costs a cold first lookup, but a reader who has the switch
      // on is owed the reason rather than a project that quietly stays cold.
      if (!destroyed) {
        languageIntelligenceNote = `This project's language server could not be warmed: ${describeError(error)}`;
      }
    }
  }

  /**
   * Turn language intelligence on or off for the project on screen.
   *
   * Off asks the desktop app to stop the process and reclaim its memory. On
   * records the choice and starts the server for the file already open, saying
   * so — including when there is no server to start.
   */
  async function switchLanguageIntelligence(enabled: boolean): Promise<void> {
    const root = editorState.projectRoot;
    if (!root || activeFileReadOnly || languageIntelligenceBusy) return;
    languageIntelligenceBusy = true;
    try {
      await hydrateLanguageIntelligenceChoices();
      if (destroyed) return;
      // Persisted before the desktop call, so the next launch keeps the choice
      // even if an older desktop build cannot act on it.
      const nextChoices = withLanguageIntelligenceChoice(
        languageIntelligenceChoices,
        root,
        enabled
      );
      await writeAssemblySettingFromTauri(LANGUAGE_INTELLIGENCE_SETTING_KEY, nextChoices);
      if (
        destroyed
        || !rootAvailable
        || !editorState.projectRoot
        || workspaceKey(editorState.projectRoot) !== workspaceKey(root)
      ) return;
      languageIntelligenceChoices = nextChoices;
      if (!enabled) {
        warmedProjectRoots.delete(root);
        diagnosticsByPath = {};
      }

      // Settings is where the C# server is switched off, and that holds. This
      // switch used to lift that setting back on whenever it was flipped on for
      // a project — so turning the server off in Settings kept undoing itself,
      // and the two switches read as one that "would not stay off". A C# start
      // that Settings has refused now says so in the note under the switch.

      // The file already on screen is the one the reader wants answered, so its
      // language is what the desktop app starts a server for — now, rather than
      // at the next file opened. With nothing open there is nothing to start,
      // and the answer says so.
      const answer = await queueLanguageIntelligenceOwner(
        root,
        enabled,
        enabled ? activeFileLanguage() : null,
        true
      );
      if (destroyed) return;
      languageIntelligenceNote = answer?.message ?? null;
      languageServerPids = answer?.serverPids ?? [];

      if (enabled) {
        await warmLanguageServer(root);
        if (destroyed) return;
        void ensureCodeEditor();
      }
      if (!destroyed) void refreshEditorIntelligenceForActiveFile();
    } catch (error) {
      if (!destroyed) {
        languageIntelligenceNote = `The switch could not be changed: ${describeError(error)}`;
      }
    } finally {
      if (!destroyed) languageIntelligenceBusy = false;
    }
  }

  /** Keep the lookup service pointed at whatever is on screen. */
  function syncIntelligenceWithActiveFile(): void {
    if (activeFileReadOnly && (languageServerStatus !== null || languageServerSubject !== null)) {
      applyLanguageServerStatus(null, null);
    }
    sourceIntelligence.setProjectRoot(activeFileReadOnly ? null : editorState.projectRoot);
    sourceIntelligence.setActivePreview(activeFileReadOnly ? null : activeEditorFile()?.preview ?? null);
  }

  // Session restore and project switching update the shared editor store
  // without calling this panel's open/select handlers. Keep the extracted
  // intelligence service attached to that state continuously; otherwise the
  // file can be visible while counts are asked with projectRoot = null.
  $effect(() => {
    editorState.projectRoot;
    editorState.activePath;
    activeEditorFile()?.preview;
    syncIntelligenceWithActiveFile();
  });

  $effect(() => {
    const root = rootAvailable ? editorState.projectRoot : null;
    languageIntelligenceChoices;
    const languageServersEnabled = settings.intelligence.languageServers;
    const generation = ++ownerSelectionGeneration;
    void (async () => {
      await hydrateLanguageIntelligenceChoices();
      if (destroyed || generation !== ownerSelectionGeneration) return;
      const enabled = Boolean(
        root
          && languageServersEnabled
          && activeServerEnabled === true
      );
      try {
        const answer = await queueLanguageIntelligenceOwner(root, enabled);
        if (
          destroyed
          || !answer
          || root !== (rootAvailable ? editorState.projectRoot : null)
        ) return;
        languageServerPids = answer.serverPids;
      } catch {
        // An older desktop build leaves the selected project in read mode.
      }
    })();
  });

  $effect(() => {
    const root = rootAvailable && fullMode ? editorState.projectRoot : null;
    if (root === nativeCsharpRoot) return;
    nativeCsharpRoot = root;
    void import('$lib/shell/editor/csharpLanguageClient').then(({ setNativeCsharpActiveRoot }) =>
      setNativeCsharpActiveRoot(root)
    ).catch(() => undefined);
  });

  /** EXPLICIT IO: read one file and show it. */
  async function readFileIntoEditor(record: SourceRecord): Promise<void> {
    if (closeActionBusy || !rootAvailable) return;
    if (readsInFlight.has(record.path)) return;
    const generation = sessionResourceGeneration;
    const readOnly = Boolean(readOnlyByPath[record.path]);
    const projectRoot = editorState.projectRoot;
    readsInFlight.add(record.path);
    markEditorFileLoading(record.path);
    if (!readOnly) void warmLanguageServer(editorState.projectRoot);
    try {
      // The record has to be built first: the read wrapper copies the relative
      // path, language and size back out of it onto the preview it returns.
      countInvoke('read_source_file');
      const preview = await readSourceFromTauri(record);
      if (
        destroyed
        || generation !== sessionResourceGeneration
        || editorState.activePath !== record.path
        || (!readOnly && editorState.projectRoot !== projectRoot)
        || !editorFileFor(record.path)
      ) {
        clearEditorFileLoading(record.path);
        return;
      }
      if (preview) {
        setEditorFilePreview(record.path, preview);
      } else {
        setEditorFileError(record.path, 'This file could not be read from here.');
      }
    } catch (error) {
      if (!destroyed && generation === sessionResourceGeneration && editorFileFor(record.path)) {
        setEditorFileError(record.path, `Could not read this file: ${describeError(error)}`);
      }
    } finally {
      if (generation === sessionResourceGeneration) readsInFlight.delete(record.path);
      syncIntelligenceWithActiveFile();
      if (!readOnly) void refreshEditorIntelligenceForActiveFile();
    }
  }

  function updateActiveDraft(content: string): void {
    if (closeActionBusy) return;
    const file = activeEditorFile();
    if (!file || readOnlyByPath[file.path]) return;
    setEditorFileDraft(file.path, content);
  }

  async function saveEditorFile(path: string): Promise<boolean> {
    if (!rootAvailable) return false;
    const file = editorFileFor(path);
    if (!file?.dirty) return true;
    if (file.saving || readOnlyByPath[path] || file.draftContent === null || file.conflict) return false;
    const generation = sessionResourceGeneration;
    const content = file.draftContent;
    setEditorFileSaving(file.path, true);
    try {
      const saved = await writeSourceToTauri(recordForPath(file.path), content);
      if (!saved) throw new Error('The file could not be written from here.');
      if (destroyed || generation !== sessionResourceGeneration || !editorFileFor(file.path)) return false;
      setEditorFilePreview(file.path, saved, content);
      return true;
    } catch (error) {
      if (!destroyed && generation === sessionResourceGeneration && editorFileFor(file.path)) {
        setEditorFileSaving(file.path, false);
        setEditorFileError(file.path, `Could not save this file: ${describeError(error)}`);
      }
      return false;
    }
  }

  async function saveActiveFile(): Promise<void> {
    const file = activeEditorFile();
    if (file) await saveEditorFile(file.path);
  }

  /**
   * Find out what the language server is doing, then ask it about the file.
   *
   * The order is the point: knowing the server is still reading the project is
   * what lets the panel hold back the work it could not answer yet, instead of
   * firing everything at it the instant a file lands.
   */
  async function refreshEditorIntelligenceForActiveFile(): Promise<void> {
    if (activeFileReadOnly) return;
    await refreshLanguageServerStatus();
    if (destroyed || activeFileReadOnly) return;
    refreshDiagnosticsForActiveFile();
  }

  /**
   * Open a file by path — the one entry point. Used by the open-file bus, by
   * the strip, and by "jump to definition" landing in another file. Opening a
   * file IS a user action, so it may load even if the panel has not been shown
   * yet; that is also what marks the editor as in use.
   *
   * Returns whether the file was actually taken on — a blank path opens nothing.
   */
  function openPath(
    path: string,
    line?: number | null,
    projectRoot?: string,
    origin: 'jump' | 'strip' = 'jump',
    readOnly = false,
    previewTab = false,
    pinTab = false
  ): boolean {
    if (closeActionBusy || !rootAvailable || !path.trim()) return false;
    if (editorState.activePath && editorState.activePath !== path) {
      releaseMarkdownView(editorState.activePath);
    }
    if (!readOnly) activateEditor(projectRoot);
    const record = recordForPath(path, projectRoot);
    const previewToReplace = editorState.openFiles.find(
      (file) => file.previewTab && file.path !== record.path
    );
    const replacedPreviewPath =
      previewTab && !pinTab ? previewToReplace?.path ?? null : null;
    const entry = openEditorFile(record, { preview: previewTab, pin: pinTab });
    if (replacedPreviewPath) {
      releaseMarkdownView(replacedPreviewPath);
      codeEditor?.disposeTabModel(replacedPreviewPath);
      sourceIntelligence.releasePreview(replacedPreviewPath);
      const { [replacedPreviewPath]: _closed, ...rest } = diagnosticsByPath;
      diagnosticsByPath = rest;
      const { [replacedPreviewPath]: _wasReadOnly, ...remaining } = readOnlyByPath;
      readOnlyByPath = remaining;
    }
    rememberMarkdownDefault(record.path, entry.fileName, origin);
    if (typeof line === 'number' && line > 0) revealEditorLine(record.path, line);
    syncIntelligenceWithActiveFile();
    if (needsRead(entry)) void readFileIntoEditor(record);
    return true;
  }

  function handleOpenFileRequest(request: OpenFileRequest): void {
    if (closeActionBusy) return;
    // Marked before the open, so the file is never editable for a frame. The
    // record's path is the key: it is what the strip and the editor hold.
    const record = recordForPath(request.path, request.projectRoot);
    if (request.readOnly) {
      readOnlyByPath = { ...readOnlyByPath, [record.path]: true };
    } else if (readOnlyByPath[record.path]) {
      const { [record.path]: _wasReadOnly, ...remaining } = readOnlyByPath;
      readOnlyByPath = remaining;
    }
    // Only once the file is in the strip. The read runs after this and may still
    // fail — the tab is the right place to show that, so it stays in front.
    if (
      openPath(
        request.path,
        request.line,
        request.projectRoot,
        'jump',
        Boolean(request.readOnly),
        Boolean(request.preview),
        Boolean(request.pin)
      )
    ) {
      onFileOpened?.();
    }
  }

  function selectOpenFile(path: string): void {
    if (closeActionBusy) return;
    if (editorState.activePath && editorState.activePath !== path) {
      releaseMarkdownView(editorState.activePath);
    }
    setActiveEditorFile(path);
    syncIntelligenceWithActiveFile();
    if (!activeFileReadOnly) void refreshEditorIntelligenceForActiveFile();
    const entry = editorFileFor(path);
    // A file restored into the strip never went through `openPath`, so this is
    // where it gets its first view. Picking a tab is editing, not reading.
    if (entry) rememberMarkdownDefault(path, entry.fileName, 'strip');
    if (entry && needsRead(entry)) {
      void readFileIntoEditor(recordForPath(path));
    }
  }

  function closeFileNow(path: string, discard = false): void {
    const disposePath = discard ? path : modelPathToDisposeOnClose(editorFileFor(path));
    closeEditorFile(path);
    releaseMarkdownView(path);
    if (disposePath) codeEditor?.disposeTabModel(disposePath);
    sourceIntelligence.releasePreview(path);
    syncIntelligenceWithActiveFile();
    // Whatever is in front now may be a different language, with a different
    // server behind it — so the chip must not keep the closed file's answer.
    void refreshLanguageServerStatus();
    // A closed file stops holding its diagnostics; nothing can show them now.
    const { [path]: _closed, ...rest } = diagnosticsByPath;
    diagnosticsByPath = rest;
    // The read-only mark belongs to the request that opened the file. Keeping
    // it meant a later open of the same path arrived already locked.
    const { [path]: _wasReadOnly, ...remaining } = readOnlyByPath;
    readOnlyByPath = remaining;
  }

  function closeOpenFileAt(path: string): void {
    if (closeActionBusy) return;
    if (editorFileFor(path)?.dirty) {
      closeRequest = { kind: 'file', path };
      closeDialogOpen = true;
      return;
    }
    closeFileNow(path);
  }

  function closeOtherOpenFiles(path: string): void {
    if (closeActionBusy) return;
    for (const file of editorState.openFiles) {
      if (file.path !== path && !file.dirty) closeFileNow(file.path);
    }
  }

  function closeSavedOpenFiles(): void {
    if (closeActionBusy) return;
    for (const file of editorState.openFiles) {
      if (!file.dirty) closeFileNow(file.path);
    }
  }

  async function closeAllOpenEditorsNow(): Promise<void> {
    const generation = sessionResourceGeneration;
    closeActionBusy = true;
    try {
      const cleared = await onCloseAllEditors?.();
      if (cleared === false) {
        editorLoadError = 'Could not clear saved editor tabs.';
        return;
      }
      if (generation !== sessionResourceGeneration) return;
      for (const file of editorState.openFiles) sourceIntelligence.releasePreview(file.path);
      codeEditor?.disposeAllTabModels();
      resetEditorState();
      markdownViewByPath = {};
      diagnosticsByPath = {};
      readOnlyByPath = {};
    } catch (error) {
      editorLoadError = `Could not clear saved editor tabs: ${describeError(error)}`;
    } finally {
      closeActionBusy = false;
    }
  }

  function closeAllOpenEditors(): void {
    if (closeActionBusy) return;
    if (editorState.openFiles.some((file) => file.dirty)) {
      closeRequest = { kind: 'all' };
      closeDialogOpen = true;
      return;
    }
    void closeAllOpenEditorsNow();
  }

  async function confirmCloseSave(): Promise<void> {
    const request = closeRequest;
    if (!request) return;
    const generation = sessionResourceGeneration;
    closeActionBusy = true;
    if (request.kind === 'file') {
      const saved = await saveEditorFile(request.path);
      if (generation !== sessionResourceGeneration) {
        closeActionBusy = false;
        return;
      }
      closeActionBusy = false;
      closeDialogOpen = false;
      closeRequest = null;
      if (saved) closeFileNow(request.path);
      return;
    }
    const dirtyPaths = editorState.openFiles.filter((file) => file.dirty).map((file) => file.path);
    let saved = true;
    for (const path of dirtyPaths) {
      if (generation !== sessionResourceGeneration) {
        saved = false;
        break;
      }
      if (!(await saveEditorFile(path))) {
        saved = false;
        break;
      }
    }
    if (generation !== sessionResourceGeneration) {
      closeActionBusy = false;
      return;
    }
    if (!saved) closeActionBusy = false;
    closeDialogOpen = false;
    closeRequest = null;
    if (saved) {
      closeActionBusy = false;
      await closeAllOpenEditorsNow();
    }
  }

  function confirmCloseDiscard(): void {
    const request = closeRequest;
    closeDialogOpen = false;
    closeRequest = null;
    if (!request) return;
    if (request.kind === 'file') closeFileNow(request.path, true);
    else void closeAllOpenEditorsNow();
  }

  function handleCloseDialogChange(open: boolean): void {
    closeDialogOpen = open;
    if (!open && !closeActionBusy) closeRequest = null;
  }

  export function requestCloseActive(): void {
    if (editorState.activePath) closeOpenFileAt(editorState.activePath);
  }

  function openTimelineFor(file: { relativePath: string }): void {
    const projectRoot = canonicalPath(editorState.projectRoot ?? '');
    if (!projectRoot) return;
    void openFileTimeline({ projectRoot, relativePath: file.relativePath });
  }

  export function captureViewStates(paths: readonly string[]): Record<string, object> {
    return codeEditor?.captureViewStates(paths) ?? {};
  }

  /** Linked-worktree inspection tabs are live-only and never belong to SQLite. */
  export function workspaceOwnedPaths(): string[] {
    return editorState.openFiles
      .filter((file) => !readOnlyByPath[file.path] && !file.previewTab)
      .map((file) => file.path);
  }

  export function restoreViewStates(
    files: readonly { path: string; viewState?: object }[]
  ): void {
    const next: Record<string, object> = {};
    for (const file of files) {
      if (file.viewState) next[file.path] = file.viewState;
    }
    restoredViewStates = next;
  }

  function consumeRestoredViewState(path: string): void {
    delete restoredViewStates[path];
  }

  export function releaseSessionResources(paths: readonly string[]): void {
    sessionResourceGeneration += 1;
    closeRequest = null;
    closeDialogOpen = false;
    if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
    diagnosticsTimer = null;
    sourceIntelligence.setActivePreview(null);
    codeEditor?.releaseSessionResources();
    for (const path of paths) {
      readsInFlight.delete(path);
      sourceIntelligence.releasePreview(path);
      releaseMarkdownView(path);
    }
    const departing = new Set(paths);
    diagnosticsByPath = Object.fromEntries(
      Object.entries(diagnosticsByPath).filter(([path]) => !departing.has(path))
    );
    readOnlyByPath = Object.fromEntries(
      Object.entries(readOnlyByPath).filter(([path]) => !departing.has(path))
    );
    restoredViewStates = {};
    resetEditorState();
  }

  function retryRead(path: string): void {
    if (closeActionBusy || !rootAvailable) return;
    void readFileIntoEditor(recordForPath(path));
  }

  /** The code editor followed a definition or a reference into another file. */
  function navigateToExternalSource(request: {
    path: string;
    line: number;
    column: number;
  }): void {
    if (activeFileReadOnly) return;
    openPath(request.path, request.line);
  }

  function handleSymbolsChange(symbols: SourceSymbol[]): void {
    setEditorSymbols(symbols);
  }

  /**
   * Fetch and start the code editor for the file on screen.
   *
   * This is the only route to `ensureCodeEditor` other than the language switch,
   * and it is deliberately tied to the panel being in front rather than to a
   * file arriving in the strip. Even the lightweight editor is kept off the
   * startup path when a restored file is not visible.
   */
  $effect(() => {
    if (!showing || !editorState.activePath) return;
    const entry = activeEditorFile();
    if (entry && needsRead(entry)) void readFileIntoEditor(recordForPath(entry.path));
    void ensureCodeEditor();
  });

  /**
   * Keep the top strip's copy of the language-server controls in step.
   *
   * It calls nothing: it copies state the panel already holds into the shared
   * module the top strip reads, so the chip and the switch can sit beside the
   * run button while this panel stays their only owner. The status pipeline is
   * not run twice.
   */
  $effect(() => {
    publishLanguageIntelligenceBar({
      language: activeFile?.language ?? null,
      status: languageServerStatus,
      fullMode,
      busy: languageIntelligenceBusy,
      hasProject: Boolean(editorState.projectRoot) && !activeFileReadOnly,
      title: languageIntelligenceTitle
    });
  });

  onMount(() => {
    void hydrateLanguageIntelligenceChoices().catch(() => undefined);

    // Subscribing costs nothing and loads nothing; it just means a click in the
    // explorer made before this panel was ever shown still opens its file.
    const unsubscribe = onOpenFile(handleOpenFileRequest);

    // The switch in the top strip is this panel's own; flipping it there runs
    // exactly the same code as flipping it here once did.
    setLanguageIntelligenceSwitch((enabled) => void switchLanguageIntelligence(enabled));

    // Listening for status updates is likewise free, and it is the only way the
    // chip ever changes after a file opens — nothing here polls.
    const unsubscribeStatus = sourceIntelligence.subscribeToLanguageServerStatus(
      handleLanguageServerStatus
    );

    return () => {
      destroyed = true;
      if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
      diagnosticsTimer = null;
      unsubscribeStatus();
      // Anything still waiting on the server has nowhere to go now.
      languageServerGate.releaseAll();
      // With no panel there is nothing truthful to show in the top strip.
      clearLanguageIntelligenceBar();
      unsubscribe();
      resetEditorState();
    };
  });
</script>

<div class="editor-panel">
  {#if editorState.openFiles.length === 0}
    <div class="editor-empty">
      <p class="empty-title">No file open</p>
      <p class="empty-hint">Open a file from the explorer or palette.</p>
    </div>
  {:else}
    <div class="editor-header">
      <div class="file-strip" role="tablist" aria-label="Open files">
        {#each editorState.openFiles as file (file.path)}
          <ContextMenu.Root>
            <ContextMenu.Trigger>
              {#snippet child({ props })}
                <div {...props} class="file-chip" class:active={file.path === editorState.activePath}>
                  <button
                    type="button"
                    role="tab"
                    aria-selected={file.path === editorState.activePath}
                    class="file-name"
                    title={file.relativePath}
                    onclick={() => selectOpenFile(file.path)}
                  >
                    <FileIcon fileName={file.fileName} size={13} />
                    {#if file.previewTab}<em>{file.fileName}</em>{:else}{file.fileName}{/if}
                    {#if file.loading}<span class="chip-note">reading</span>{/if}
                    {#if file.error}<span class="chip-note error">failed</span>{/if}
                    {#if file.conflict}<span class="chip-note error">conflict</span>{/if}
                  </button>
                  <IconButton
                    label={`Close ${file.fileName}`}
                    size="sm"
                    side="bottom"
                    class="file-close text-[var(--color-text-2)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
                    onclick={() => closeOpenFileAt(file.path)}
                  >
                    <X class="size-3.5" aria-hidden="true" />
                  </IconButton>
                </div>
              {/snippet}
            </ContextMenu.Trigger>
            <ContextMenu.Content class="w-[220px]" aria-label={`Actions for ${file.fileName}`}>
              {#if file.previewTab}
                <ContextMenu.Item
                  onSelect={() => pinEditorFile(file.path)}
                >Pin Tab</ContextMenu.Item>
              {/if}
              <ContextMenu.Item
                onSelect={() => closeOpenFileAt(file.path)}
              >Close</ContextMenu.Item>
              <ContextMenu.Item
                disabled={!editorState.openFiles.some((candidate) => candidate.path !== file.path && !candidate.dirty)}
                onSelect={() => closeOtherOpenFiles(file.path)}
              >Close other clean files</ContextMenu.Item>
              <ContextMenu.Item
                disabled={!editorState.openFiles.some((candidate) => !candidate.dirty)}
                onSelect={closeSavedOpenFiles}
              >Close saved files</ContextMenu.Item>
              <ContextMenu.Item
                onSelect={() => openTimelineFor(file)}
              >File Timeline</ContextMenu.Item>
            </ContextMenu.Content>
          </ContextMenu.Root>
        {/each}
      </div>

      <!-- Only the open file's own controls belong here. The project's
           language-server chip and switch sit in the strip along the top of the
           shell, in `LanguageIntelligenceControls.svelte`. -->
      <div class="editor-controls">
        <IconButton label="Close all open editors" size="sm" side="bottom" onclick={closeAllOpenEditors}>
          <X class="size-3.5" aria-hidden="true" />
        </IconButton>
        <!-- Markdown reads two ways, so the file says which one it is on. Source
             is the ordinary editor; Preview is the same document rendered. -->
        {#if activeFileIsMarkdown}
          <span data-testid="markdown-view-toggle" class="shrink-0">
            <SegmentedControl
              size="sm"
              items={MARKDOWN_VIEW_ITEMS}
              value={markdownView}
              aria-label="Markdown view"
              onValueChange={(value) => setMarkdownView(value as MarkdownView)}
            />
          </span>
        {/if}
      </div>
    </div>

    <div class="editor-canvas">
      {#if editorLoadError}
        <p class="canvas-message error">{editorLoadError}</p>
      {:else if activeFile?.error && activeFileMissing}
        <p class="canvas-message">File no longer exists at {activeFile.path}</p>
      {:else if activeFile?.error}
        <div class="canvas-message error">
          <p>{activeFile.error}</p>
          <button
            type="button"
            class="retry"
            onclick={() => activeFile && retryRead(activeFile.path)}
          >
            Try again
          </button>
        </div>
      {:else if !rootAvailable}
        <p class="canvas-message">Checkout/Worktree deleted.</p>
      {:else if showing && activeFile?.preview && activeFileIsMarkdown && markdownView === 'rendered'}
        <SourceMarkdownPreview
          content={activeFile.draftContent ?? activeFile.preview.content}
          fileName={activeFile.fileName}
          relativePath={activeFile.relativePath}
          dirty={activeFile.dirty ?? false}
        />
      {:else if activeFile?.preview}
        {#if CodeEditor}
          <CodeEditor
            bind:this={codeEditor}
            {...sourceIntelligence.callbacks}
            onInlayHintLookup={activeFileReadOnly ? undefined : lookupInlayHintsWhenServerCanAnswer}
            preview={activeFile.preview}
            content={activeFile.draftContent ?? activeFile.preview.content}
            editable={rootAvailable && !activeFileReadOnly && !closeActionBusy}
            visible={showing}
            loading={activeFile.loading}
            targetLine={activeFile.targetLine}
            targetLineRequestId={activeFile.targetLineRequestId}
            externalDiagnostics={activeFileReadOnly ? NO_DIAGNOSTICS : diagnosticsByPath[activeFile.path] ?? NO_DIAGNOSTICS}
            {restoredViewStates}
            onExternalNavigation={activeFileReadOnly ? undefined : navigateToExternalSource}
            onContentChange={activeFileReadOnly ? undefined : updateActiveDraft}
            onRestoredViewStateConsumed={consumeRestoredViewState}
            onSaveRequest={() => void saveActiveFile()}
            onSymbolsChange={handleSymbolsChange}
          />
        {:else}
          <p class="canvas-message">Starting the code editor…</p>
        {/if}
      {:else}
        <p class="canvas-message">Reading {activeFile?.fileName ?? 'file'}…</p>
      {/if}
    </div>

    <div class="editor-status">
      <span class="status-path">{activeFile?.relativePath ?? ''}</span>
      <span class="status-detail" title={activeFile?.conflict ?? undefined}>
        {activeFile?.language ?? ''}
        {#if editorState.symbols.length > 0}
          · {editorState.symbols.length}
          {editorState.symbols.length === 1 ? 'symbol' : 'symbols'}
        {/if}
        {#if activeFileReadOnly}
          · read-only
        {:else if activeFile?.saving}
          · saving
        {:else if activeFile?.conflict}
          · conflict
        {:else if activeFile?.dirty}
          · unsaved
        {:else}
          · editable
        {/if}
      </span>
    </div>
  {/if}
</div>

{#if closeRequest}
  <AlertDialog.Root open={closeDialogOpen} onOpenChange={handleCloseDialogChange}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Unsaved changes</AlertDialog.Title>
        <AlertDialog.Description>
          {closeRequest.kind === 'all'
            ? 'Some open files have unsaved drafts. Save all before closing them?'
            : 'This file has an unsaved draft. Save it before closing?'}
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Cancel disabled={closeActionBusy}>Cancel</AlertDialog.Cancel>
        <AlertDialog.Action
          disabled={closeActionBusy || !rootAvailable}
          onclick={() => void confirmCloseSave()}
        >{closeRequest.kind === 'all' ? 'Save All' : 'Save'}</AlertDialog.Action>
        <AlertDialog.Action
          variant="destructive"
          disabled={closeActionBusy}
          onclick={confirmCloseDiscard}
        >{closeRequest.kind === 'all' ? 'Discard All' : 'Discard'}</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
{/if}

<style>
  .editor-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-width: 0;
    overflow: hidden;
    /* No fill of its own. This is a panel body in normal flow covering nothing,
       and filling it with the backdrop colour painted a black rectangle over
       the card underneath — which carries the surface colour and the light
       gradient down its top edge. Most visible with no file open, where the
       body IS the whole panel. The header and status rows still state their own
       surface, because those are bands rather than the body. */
    color: var(--color-text);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
  }

  .editor-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 100%;
    color: var(--color-text-2);
    font-size: 13px;
    user-select: none;
  }

  .editor-empty p {
    margin: 0;
  }

  .empty-hint {
    color: var(--color-text-3);
    font-size: 12px;
  }

  /* The strip of open files and, pinned to the right, the controls that belong
   * to the open file. The strip scrolls when there are many files; those
   * controls do not go with it, so they stay reachable however many tabs are
   * open.
   *
   * The height is stated rather than left to the tallest child because the
   * centre pane's pill tabs are placed directly beneath this row and read the
   * same value. It is what the row already measured — a 28px close button
   * between 3px of padding, over a hairline — so nothing moves. */
  .editor-header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    height: var(--editor-tab-row-height);
    overflow: hidden;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    padding: 3px 8px 3px 4px;
  }

  .file-strip {
    display: flex;
    align-items: stretch;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--color-border-strong) transparent;
  }

  .file-strip::-webkit-scrollbar {
    height: 4px;
  }

  .file-strip::-webkit-scrollbar-thumb {
    border-radius: 2px;
    background: var(--color-border-strong);
  }

  .file-chip {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    border: 1px solid transparent;
    border-radius: 5px;
    background: transparent;
  }

  .file-chip.active {
    background: var(--color-elevated);
    border-color: var(--color-border);
  }

  .file-name {
    background: transparent;
    border: none;
    color: var(--color-text-2);
    cursor: pointer;
    font-family: inherit;
    font-size: 12px;
    padding: 3px 4px 3px 8px;
    white-space: nowrap;
  }

  /* The file-type icon sits on the same line as the name, and the gap is what
   * keeps it off the text. */
  .file-name {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .file-chip.active .file-name {
    color: var(--color-text);
  }

  .file-name:hover {
    color: var(--color-text);
  }

  /* The complete right edge is one non-shrinking sibling of the scrollable
   * file strip. Tabs can move underneath their own clip, but these controls
   * retain their full width and never enter that scrolling region. */
  .editor-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    min-width: max-content;
  }

  .chip-note {
    color: var(--color-text-3);
    font-size: 12px;
    margin-left: 5px;
  }

  .chip-note.error {
    color: var(--color-bad);
  }

  .editor-canvas {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .canvas-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    margin: 0;
    color: var(--color-text-2);
    font-size: 13px;
    text-align: center;
    padding: 0 16px;
  }

  .canvas-message p {
    margin: 0;
  }

  .canvas-message.error {
    color: var(--color-bad);
  }

  .retry {
    background: var(--color-elevated);
    border: 0;
    border-radius: 5px;
    color: var(--color-text-2);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 2px 7px;
  }

  .retry:hover {
    color: var(--color-text);
    background: var(--color-hover);
  }

  .editor-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex: 0 0 auto;
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
    color: var(--color-text-2);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 3px 8px;
  }

  .status-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-detail {
    color: var(--color-text-3);
    white-space: nowrap;
  }
</style>
