/**
 * sourceIntelligence.ts — everything Monaco asks the machine for.
 *
 * `MonacoSourceEditor.svelte` knows how to draw code and how to run a peek
 * window, but it knows nothing about this project: every lookup arrives as a
 * callback prop. This module is the other half — it owns the small amount of
 * context a lookup needs (project root, the file on screen, that file's
 * current text, the scanned file list) and answers each callback.
 *
 * Ported from the old shell's page-local handlers
 * (`src/routes/+page.svelte:8446`, `:8486`, `:8548`, `:8607`, `:10104-10214`)
 * with the constitution's rules applied:
 *
 *  - **Two tiers, then give up.** Language server first, then the backend's
 *    plain-text scan, then an empty answer. The old shell had a third tier
 *    that answered from bundled demo files; /next never invents results. The
 *    first tier also has a stopwatch on it: a language server that has not
 *    answered within `languageServerLookupDeadlineMs` is left behind and the
 *    plain-text answer is shown instead.
 *  - **Every backend call is counted** with `countInvoke('<command name>')`
 *    immediately before it, so the dev counter tells the truth.
 *  - **Nothing runs on import.** The first backend call of any kind happens
 *    when the user opens a file.
 *
 * WHERE THE "N references" NUMBER COMES FROM (ruled 2026-07-30, replacing the
 * ruling of 2026-07-28 that this comment used to carry).
 *
 * In the desktop app it comes from the language server, one symbol at a time.
 * That is the number the reader actually wants: it counts the uses of the
 * `Send` they are looking at, not every line in the project that contains the
 * word "Send". The reason it was taken away in July was never that it was the
 * wrong number — it was that a hundred of those questions went out at once,
 * the backend answered them one at a time behind a single lock, and nothing
 * cancelled the ones nobody was waiting for any more. That is fixed at both
 * ends now: the backend answers several at once and abandons what it is told
 * to abandon, and this module only ever has a few questions outstanding, in
 * the order the reader will read them.
 *
 * In the browser preview it comes from the plain-text search over the project,
 * because a page served by the dev server has no language server behind it at
 * all. That is the number the browser preview has always shown and it stays
 * exactly as it is, "at least N" wording included. The same search still
 * serves the project-wide search box everywhere.
 *
 * The budgets below are load-bearing for both — see
 * `countReferencesForCodeLens`.
 */
import {
  normalizeProjectPath,
  previewFromContent,
  sourceSupportsLanguageIntelligence,
  type SourceCompletionItem,
  type SourceDefinitionTarget,
  type SourceDiagnostic,
  type SourceDocumentHighlight,
  type SourceInlayHint,
  type SourceLspHover,
  type SourceLspStatus,
  type SourcePreview,
  type SourceRecord,
  type SourceReferenceTarget,
  type SourceSemanticToken,
  type SourceSignatureHelp,
  type SourceSymbol
} from '../../sourceData.ts';
import {
  countSourceReferencesFromTauri,
  findSourceLspCompletionsFromTauri,
  findSourceLspDefinitionsFromTauri,
  findSourceLspDocumentHighlightsFromTauri,
  findSourceLspHoverFromTauri,
  findSourceLspInlayHintsFromTauri,
  findSourceLspReferencesFromTauri,
  findSourceLspSemanticTokensFromTauri,
  findSourceLspSignatureHelpFromTauri,
  isNativeTauriRuntime,
  readSourceFromTauri,
  readSourceLspDiagnosticsFromTauri,
  readSourceLspStatusFromTauri
} from '../../tauriSource.ts';
import { hasBackendCapability } from '../backendCapabilities.ts';
import { countInvoke } from '../devInvokeCounter.svelte.ts';
import { activateEditor } from './editorStore.svelte.ts';
import {
  createReferenceCountBatcher,
  createReferenceCountStore,
  createSemanticReferenceCountScheduler,
  type CodeLensCount
} from './referenceCountBatcher.ts';
import { statusMessageIsAboutThisFile } from '../components/editor/languageServerStatus.ts';
import { sourceRecordFromPath } from './sourceRecordFromPath.ts';

// ── Budgets (from the old shell, except where the margin counts changed) ─────

/** Most definitions one lookup will return. */
export const maxSourceDefinitionResults = 20;
/**
 * Most references one "find references" click will list. The margin counts do
 * NOT use this — see `maxSourceReferenceCountResults`.
 */
export const maxSourceReferenceResults = 50;
/**
 * Most references the language server is asked to list when all we want is the
 * number. Far higher than a peek window's ceiling because a peek is something a
 * person reads and a count is not: cutting it off at fifty would put "50" above
 * a symbol used two hundred times. Should a symbol somehow exceed even this,
 * the margin says "at least" rather than stating a number nobody reached.
 */
export const maxSourceReferenceCountResults = 1_000;
/**
 * How many "how many references?" questions the language server may be working
 * on at once.
 *
 * Small on purpose. A cold server takes seconds over a single one of these, and
 * a file with a hundred symbols in it used to send all hundred at once — which
 * is what made opening a C# file look like the app had died. Four keeps the
 * numbers arriving steadily down the page while leaving the server free to
 * answer the click the reader is actually waiting on.
 */
export const semanticReferenceCountMaxInFlight = 4;
/** Most completion items one lookup will return. */
export const maxSourceCompletionResults = 50;
/**
 * Monaco resolves margin counts one symbol at a time, as each one scrolls into
 * view. Instead of asking the backend per symbol, the first request opens a
 * window this long and every count asked for during it travels in one request.
 */
export const codeLensReferenceCountBatchWindowMs = 50;
/**
 * The most time that one request may spend reading the project before it
 * answers with whatever it counted so far. A ceiling, not a wait — a project
 * of about four thousand files is counted in roughly 400ms and answers then.
 * Nothing on screen is blocked while it runs.
 */
export const codeLensReferenceCountDeadlineMs = 1_500;
/** How many times a failed language-server count may be asked again. */
export const semanticCountRetryLimit = 3;
/** How long to wait before each language-server count retry, in milliseconds. */
export const semanticCountRetryDelaysMs = [2_000, 6_000, 12_000];
/**
 * How long a counted number stays good for. Project-wide counts move when
 * files elsewhere change, which is not something typing in the open file does
 * — so counts must NOT be thrown away on every keystroke. That is what used to
 * leave the margin blank while a big project was being typed in.
 */
export const codeLensReferenceCountCacheMs = 30_000;
/**
 * Where the plain-text counts are filed. They are counted by NAME across the
 * whole project, so the same name has the same number in every file and they
 * all belong in one drawer — unlike the language server's numbers, which are
 * about one symbol in one file and are filed under that file.
 */
export const wholeProjectCountsDrawer = 'the whole project';
/**
 * Most symbols one file gets margin numbers for. The same ceiling the editor
 * applies to the lens rows it draws.
 */
export const maxCodeLensSymbols = 120;
/**
 * How long the language server gets to answer a "find references" or "go to
 * definition" click before the plain-text search answers instead.
 *
 * The language server's answer is the better one — it knows which `Send` you
 * clicked — so it is used whenever it arrives in time. But a cold C# server
 * loading a large solution does not answer for tens of seconds, and waiting on
 * it left the editor saying "Finding references…" for half a minute. The
 * plain-text search over the scanned files comes back in a moment and is the
 * only answer the web preview has ever had, which is why the same click there
 * has always felt instant. One second is long enough for a warm server to win
 * and short enough that a cold one is never waited on.
 */
export const languageServerLookupDeadlineMs = 1_000;

/** Whether a reference-count question should run, wait, or use plain text. */
export type CountingReadiness = 'ask-it' | 'wait-for-it' | 'no-server';

/**
 * Decide what a reported server state means for one reference-count question.
 * Waiting is allowed only when the caller knows both that this server can be
 * woken and that its later status changes will reach the editor.
 */
export function countingReadinessForStatus(
  state: string | undefined,
  available: boolean | undefined,
  canWait: boolean
): CountingReadiness {
  if (state === 'ready') return 'ask-it';
  if (state === 'starting' || state === 'indexing') {
    return canWait ? 'wait-for-it' : 'no-server';
  }
  if (state === 'not-running' && available === true && canWait) return 'wait-for-it';
  return 'no-server';
}

// ── The shapes Monaco hands us ────────────────────────────────────────────────

/** What Monaco knows about the spot the user is asking about. */
export interface SourceLookupRequest {
  symbolName: string;
  /** 1-based. */
  line: number;
  /** 1-based. */
  column: number;
  /**
   * Which file the spot is in, when the editor knows. Only the margin's row
   * spots carry it, and only so that a number counted against one file can
   * never be drawn above another one; every other lookup leaves it out and is
   * resolved against the file on screen as before.
   */
  filePath?: string;
}

/** The visible range Monaco wants inline hints for. */
export interface SourceInlayHintRequest {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

/** The callback set handed straight to `MonacoSourceEditor` as props. */
export interface SourceIntelligenceCallbacks {
  onDefinitionLookup(request: SourceLookupRequest): Promise<SourceDefinitionTarget[]>;
  onReferenceLookup(request: SourceLookupRequest): Promise<SourceReferenceTarget[]>;
  onReferenceCountLookup(request: SourceLookupRequest): Promise<CodeLensCount | null>;
  /**
   * The symbols a file's margin rows sit above, when the language server can
   * name them better than the editor's own reader of the text can. Answers
   * `null` when it cannot — the editor then falls back to reading the text,
   * which is all the browser preview has ever done.
   */
  onCodeLensAnchorLookup(preview: SourcePreview): Promise<SourceSymbol[] | null>;
  /**
   * This file was edited or saved, so the numbers counted for it are out of
   * date. Only that file's numbers are let go; every other file's stand.
   */
  onReferenceCountsOutOfDate(filePath: string): void;
  onExternalPreviewLookup(record: SourceRecord): Promise<SourcePreview | null>;
  onHoverLookup(request: SourceLookupRequest): Promise<SourceLspHover | null>;
  onCompletionLookup(request: SourceLookupRequest): Promise<SourceCompletionItem[]>;
  onDocumentHighlightLookup(request: SourceLookupRequest): Promise<SourceDocumentHighlight[]>;
  onSignatureHelpLookup(request: SourceLookupRequest): Promise<SourceSignatureHelp | null>;
  onInlayHintLookup(request: SourceInlayHintRequest): Promise<SourceInlayHint[]>;
  onSemanticTokensLookup(preview: SourcePreview): Promise<SourceSemanticToken[]>;
}

/** The service the editor panel drives. */
export interface SourceIntelligence {
  /** Project the open files belong to; language-server lookups need it. */
  setProjectRoot(projectRoot: string | null): void;
  /** The file on screen (its contents are what lookups are resolved against). */
  setActivePreview(preview: SourcePreview | null): void;
  /** The file on screen's current text — the same as the preview while read-only. */
  setDraftContent(content: string): void;
  /** Forget the remembered contents of one file (call after it is written). */
  invalidatePreview(path: string): void;
  /** Forget every remembered file (call when the project changes). */
  invalidateAllPreviews(): void;
  /**
   * What the language server says is wrong with the file on screen.
   *
   * PUSHED, not pulled: Monaco has no diagnostics callback, so this is not one
   * of `callbacks` — the editor panel asks for it and hands the answer down as
   * `externalDiagnostics`. Empty when there is no language server for this file,
   * when there is no file, or when the read fails.
   */
  loadActiveFileDiagnostics(): Promise<SourceDiagnostic[]>;
  /** Read-only view of the current context, for status lines and tests. */
  readonly projectRoot: string | null;
  /** The callbacks to spread onto `MonacoSourceEditor`. */
  readonly callbacks: SourceIntelligenceCallbacks;
}

/**
 * Wait for a lookup, but only for so long: answers `null` once `deadlineMs` has
 * passed, so the caller can fall back to something faster.
 *
 * Nothing here can stop the lookup it gave up on — it keeps running in the
 * backend and its answer is simply dropped. Both of its endings are handled, so
 * a slow lookup that fails long after everyone stopped caring cannot surface as
 * an unhandled rejection.
 */
function answerOrGiveUp<T>(lookup: Promise<T | null>, deadlineMs: number): Promise<T | null> {
  return new Promise<T | null>((resolve) => {
    const giveUp = setTimeout(() => resolve(null), deadlineMs);
    lookup.then(
      (answer) => {
        clearTimeout(giveUp);
        resolve(answer);
      },
      () => {
        clearTimeout(giveUp);
        resolve(null);
      }
    );
  });
}

/**
 * Can whatever is behind this page count references in one pass?
 *
 * In the desktop app the answer is a promise the backend makes by name, because
 * a desktop build older than this page would quietly answer nothing at all
 * rather than failing — see `backendCapabilities.ts`. In the browser preview
 * there is nothing to check: the counting is served by the same dev server that
 * served this page, so the two are never out of step.
 *
 * When the answer is no, margin counts are simply not drawn. The way they were
 * worked out before — one reading of the whole project per symbol, a hundred or
 * more of them as a file opened — is what made the editor stop responding for
 * ten seconds, so it is not somewhere worth falling back to.
 */
async function backendCanCountReferences(): Promise<boolean> {
  if (!isNativeTauriRuntime()) return true;
  return hasBackendCapability('referenceCounts');
}

/**
 * Build a source-intelligence service. One per editor — and there is only
 * ever ONE editor, because Monaco registers its providers for the whole page:
 * a second editor would silently take over every lookup.
 */
export function createSourceIntelligence(): SourceIntelligence {
  let projectRoot: string | null = null;
  let activePreview: SourcePreview | null = null;
  let draftContent = '';
  /**
   * Remembered contents per file. A peek window resolves the same file once
   * per group; without this the lazy preview resolver re-reads it every time.
   * Dropped for a file when that file is written, and wholesale on a project
   * change — a stale preview after an edit would be a correctness bug.
   */
  const externalPreviewCache = new Map<string, SourcePreview>();

  /** Does the file on screen have a language server behind it? */
  function languageIntelligenceAvailable(): boolean {
    return activePreview ? sourceSupportsLanguageIntelligence(activePreview.language) : false;
  }

  /** The file on screen, carrying its current text. */
  function previewWithDraft(): SourcePreview | null {
    return activePreview ? { ...activePreview, content: draftContent } : null;
  }

  function lookupRoot(): string {
    return projectRoot ?? '';
  }

  // ── Tier 1: the language server ────────────────────────────────────────────

  async function lspDefinitions(
    request: SourceLookupRequest
  ): Promise<SourceDefinitionTarget[] | null> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return null;
    countInvoke('find_source_lsp_definitions');
    return findSourceLspDefinitionsFromTauri(preview, {
      root: lookupRoot(),
      line: request.line,
      column: request.column,
      limit: maxSourceDefinitionResults
    }).catch(() => null);
  }

  /** See `SourceIntelligence.loadActiveFileDiagnostics`. */
  async function loadActiveFileDiagnostics(): Promise<SourceDiagnostic[]> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return [];
    try {
      countInvoke('read_source_lsp_diagnostics');
      return (
        (await readSourceLspDiagnosticsFromTauri(preview, {
          root: lookupRoot(),
          line: 1,
          column: 1
        })) ?? []
      );
    } catch {
      return [];
    }
  }

  async function lspReferences(
    request: SourceLookupRequest
  ): Promise<SourceReferenceTarget[] | null> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return null;
    countInvoke('find_source_lsp_references');
    return findSourceLspReferencesFromTauri(preview, {
      root: lookupRoot(),
      line: request.line,
      column: request.column,
      limit: maxSourceReferenceResults
    }).catch(() => null);
  }

  // ── The callbacks ──────────────────────────────────────────────────────────
  //
  // There used to be a second tier under these two: a plain-text search over a
  // list of scanned files this service was handed. Nothing ever handed it one —
  // /next has no project scan of its own — so the list was always empty and the
  // tier could never answer. It was removed on 2026-07-30 rather than left
  // sitting there looking like a safety net that works. The plain-text search
  // itself is very much alive; it is what the project-wide search box runs and
  // what the browser preview counts references with.

  async function findDefinitions(
    request: SourceLookupRequest
  ): Promise<SourceDefinitionTarget[]> {
    const symbolName = request.symbolName.trim();
    if (!symbolName) return [];
    try {
      const lspTargets = await answerOrGiveUp(
        lspDefinitions(request),
        languageServerLookupDeadlineMs
      );
      return lspTargets ?? [];
    } catch {
      return [];
    }
  }

  async function findReferences(request: SourceLookupRequest): Promise<SourceReferenceTarget[]> {
    const symbolName = request.symbolName.trim();
    if (!symbolName) return [];
    try {
      const lspTargets = await answerOrGiveUp(
        lspReferences(request),
        languageServerLookupDeadlineMs
      );
      return lspTargets ?? [];
    } catch {
      return [];
    }
  }

  // ── Margin counts ──────────────────────────────────────────────────────────
  //
  // Two machines can answer "how many references?", and which one does depends
  // entirely on whether there is a language server behind this page. See the
  // note at the top of the file for the ruling. Both file their answers in the
  // one store below.

  /**
   * The one place a counted number is remembered, for either machine.
   *
   * Filed under the project it was counted in, so opening another workspace
   * puts its numbers in a drawer of their own rather than emptying this one —
   * coming back finds the first workspace's numbers where they were left.
   */
  const countStore = createReferenceCountStore({ cacheMs: codeLensReferenceCountCacheMs });

  /** The plain-text search over the project: one pass answers many names. */
  const referenceCountBatcher = createReferenceCountBatcher({
    windowMs: codeLensReferenceCountBatchWindowMs,
    memory: countStore.drawer(() => projectRoot, wholeProjectCountsDrawer),
    // Uncapped: the count is an exact number the backend worked out in one
    // pass, so there is no reason to round it off to "50+". The old ceiling
    // existed because each count cost its own reading of the project and was
    // stopped early to keep that affordable.
    maxCount: Number.POSITIVE_INFINITY,
    async countReferences(symbolNames: string[]) {
      if (!projectRoot) return null;
      if (!(await backendCanCountReferences())) return null;
      countInvoke('count_source_references');
      const startedAt = Date.now();
      const result = await countSourceReferencesFromTauri(
        projectRoot,
        symbolNames,
        codeLensReferenceCountDeadlineMs
      );
      reportLensTiming(
        `read the project for ${symbolNames.length} name(s) in ${Date.now() - startedAt}ms`
      );
      return result;
    }
  });

  /**
   * A symbol whose number the language server has been, or is about to be,
   * asked for. It stays here until the answer lands or the reader moves on.
   */
  interface WaitingSpot {
    request: SourceLookupRequest;
    /** The file as it was when the question was asked. */
    preview: SourcePreview;
    /** Everyone who wants to hear the answer. */
    waiters: ((count: CodeLensCount | null) => void)[];
    askedAt: number;
    /** How many failed answers have already caused this question to be asked again. */
    tries: number;
  }

  const waitingSpots = new Map<string, WaitingSpot>();

  /**
   * Symbols whose questions are held back because the language server is still
   * starting up or still reading the project. They go out the moment it says it
   * is ready; until then the margin keeps saying it is still counting.
   */
  let heldUntilServerIsReady: string[] = [];

  const semanticScheduler = createSemanticReferenceCountScheduler({
    maxInFlight: semanticReferenceCountMaxInFlight,
    async countFor(key: string) {
      const spot = waitingSpots.get(key);
      if (!spot) return null;
      return askLanguageServerToCount(spot);
    },
    onCounted(key: string, count: CodeLensCount | null) {
      const spot = waitingSpots.get(key);
      if (!spot) return;
      if (count === null && spot.tries < semanticCountRetryLimit) {
        spot.tries += 1;
        const retryDelayMs = semanticCountRetryDelaysMs[spot.tries - 1];
        reportLensTiming(
          `${spot.request.symbolName}: no answer after ${
            Date.now() - spot.askedAt
          }ms — asking again in ${retryDelayMs / 1_000}s (try ${spot.tries + 1} of ${
            semanticCountRetryLimit + 1
          })`
        );
        const askAgain = () => {
          if (waitingSpots.get(key) !== spot) return;
          semanticScheduler.request([key]);
        };
        if (typeof window === 'undefined') {
          setTimeout(askAgain, retryDelayMs);
        } else {
          window.setTimeout(askAgain, retryDelayMs);
        }
        return;
      }

      waitingSpots.delete(key);
      if (count) {
        countStore.remember(projectRoot, normalizeProjectPath(spot.preview.path), key, count);
      }
      reportLensTiming(
        `${spot.request.symbolName}: ${
          count ? `${count.count} reference(s)` : 'no answer'
        } after ${Date.now() - spot.askedAt}ms`
      );
      for (const waiter of spot.waiters) waiter(count);
    }
  });

  /**
   * Which line, which name. The file is not in here because the store already
   * files a number under its file, and the column is left out because text
   * typed earlier on the line moves the symbol sideways without changing how
   * many places use it.
   */
  function countKeyFor(request: SourceLookupRequest): string {
    return `${request.line}:${request.symbolName}`;
  }

  /**
   * Ask the language server how many places use this exact symbol, and turn the
   * list it answers with into a number.
   *
   * The declaration itself is dropped when the server includes it, so the
   * number means "used in N places" the way it does in every other editor. A
   * symbol that calls itself keeps those uses: only the line the margin row
   * sits on is taken out.
   */
  async function askLanguageServerToCount(spot: WaitingSpot): Promise<CodeLensCount | null> {
    countInvoke('find_source_lsp_references');
    const targets = await findSourceLspReferencesFromTauri(spot.preview, {
      root: lookupRoot(),
      line: spot.request.line,
      column: spot.request.column,
      limit: maxSourceReferenceCountResults
    }).catch(() => null);
    if (!targets) return null;

    const declarationPath = normalizeProjectPath(spot.preview.path);
    const uses = targets.filter(
      (target) =>
        target.line !== spot.request.line || normalizeProjectPath(target.path) !== declarationPath
    );
    return { count: uses.length, atLeast: targets.length >= maxSourceReferenceCountResults };
  }

  /**
   * Is the language server for the file on screen in a position to count?
   *
   *  - `ask-it`      — it is running and has finished reading the project.
   *  - `wait-for-it` — it is still starting up or still reading. The margin
   *                    keeps saying it is counting, and the questions go out
   *                    when it tells us it is ready.
   *  - `no-server`   — there is none, it is switched off, or this is the
   *                    browser preview. The plain-text count answers instead.
   *
   * A desktop build too old to say what state it is in is treated as
   * `no-server` on purpose. Those builds also answer language-server questions
   * strictly one at a time, so a hundred counts would queue up behind each
   * other exactly as they used to; the plain-text count is both faster and
   * safer there.
   */
  /**
   * The last answer per project and language, and when it was given.
   *
   * A file's margin rows are asked about in a burst — twenty at once is
   * ordinary — and asking the app what the language server is doing twenty
   * times over would be its own little storm. One answer serves the burst, and
   * a status change throws it away immediately.
   */
  const rememberedReadiness = new Map<string, { at: number; answer: Promise<CountingReadiness> }>();
  /** How long one reading of the server's state is reused for. */
  const readinessMemoryMs = 3_000;

  async function languageServerCountingReadiness(
    preview: SourcePreview
  ): Promise<CountingReadiness> {
    if (!isNativeTauriRuntime()) return 'no-server';
    if (!sourceSupportsLanguageIntelligence(preview.language)) return 'no-server';
    if (!projectRoot) return 'no-server';

    const memoKey = `${projectRoot}|${preview.language}`;
    const remembered = rememberedReadiness.get(memoKey);
    if (remembered && Date.now() - remembered.at < readinessMemoryMs) return remembered.answer;

    const answer = readLanguageServerCountingReadiness(projectRoot, preview.language);
    rememberedReadiness.set(memoKey, { at: Date.now(), answer });
    return answer;
  }

  async function readLanguageServerCountingReadiness(
    root: string,
    language: SourcePreview['language']
  ): Promise<CountingReadiness> {
    countInvoke('read_source_lsp_status');
    const status = await readSourceLspStatusFromTauri(root, language).catch(() => null);
    const reported = status as
      | (SourceLspStatus & { state?: unknown; available?: unknown })
      | null;
    const state = typeof reported?.state === 'string' ? reported.state : undefined;
    const available =
      typeof reported?.available === 'boolean' ? reported.available : undefined;
    const mightWait =
      state === 'starting' ||
      state === 'indexing' ||
      (state === 'not-running' && available === true);
    if (!mightWait) return countingReadinessForStatus(state, available, false);

    // Waiting is honest only because the document-symbols request for this same
    // file is already on its way to start or move the server, and a later status
    // update will release these questions. Without either ability, the margin
    // would say "counting references…" for the rest of the session.
    const canWatch = await watchLanguageServerStatus();
    const canWake = canWatch && (await hasBackendCapability('lspDocumentSymbols'));
    return countingReadinessForStatus(state, available, canWake);
  }

  /** Set up once we first need it; answers whether the app can tell us. */
  let statusWatch: Promise<boolean> | null = null;

  function watchLanguageServerStatus(): Promise<boolean> {
    statusWatch ??= (async () => {
      if (!(await hasBackendCapability('lspStatusEvents'))) return false;
      try {
        const { listen } = await import('@tauri-apps/api/event');
        await listen<{ state?: string; root: string; language: string }>(
          'source-lsp-status-changed',
          (event) => {
            // Whatever we last worked out about the server is now out of date.
            rememberedReadiness.clear();
            if (
              event.payload?.state === 'ready' &&
              statusMessageIsAboutThisFile(
                event.payload,
                projectRoot,
                activePreview?.language ?? null
              )
            ) {
              releaseHeldQuestions();
            }
          }
        );
        return true;
      } catch {
        return false;
      }
    })();
    return statusWatch;
  }

  function releaseHeldQuestions(): void {
    if (heldUntilServerIsReady.length === 0) return;
    const released = heldUntilServerIsReady;
    heldUntilServerIsReady = [];
    reportLensTiming(`the language server is ready — asking about ${released.length} symbol(s)`);
    semanticScheduler.request(released);
  }

  /**
   * The "N references" number drawn above a symbol.
   *
   * `null` means we cannot count this one at all, and the editor draws no
   * number rather than a wrong one. A promise that has not settled yet means
   * the counting is still going on, and the margin keeps saying so.
   */
  async function countReferencesForCodeLens(
    request: SourceLookupRequest
  ): Promise<CodeLensCount | null> {
    const symbolName = request.symbolName.trim();
    if (!symbolName) return null;

    const preview = previewWithDraft();
    const spotRequest: SourceLookupRequest = { ...request, symbolName };
    const key = countKeyFor(spotRequest);

    if (preview) {
      // The spot belongs to a file other than the one on screen — which is the
      // only file anything here can count against. Better to show no number
      // than one worked out for the wrong file.
      if (
        request.filePath &&
        normalizeProjectPath(request.filePath) !== normalizeProjectPath(preview.path)
      ) {
        return null;
      }

      const filePath = normalizeProjectPath(preview.path);
      const remembered = countStore.get(projectRoot, filePath, key);
      if (remembered) return remembered;

      const readiness = await languageServerCountingReadiness(preview);
      if (readiness !== 'no-server') {
        // The server turned ready between one symbol and the next: whatever was
        // held back while it was starting goes out now, so the whole file's
        // rows fill in rather than the ones asked about after it woke up.
        if (readiness === 'ask-it') releaseHeldQuestions();
        return countBySemantics(key, spotRequest, preview, readiness === 'wait-for-it');
      }
    }

    return referenceCountBatcher.count(symbolName);
  }

  /** Put this symbol in the language server's queue and wait for its turn. */
  function countBySemantics(
    key: string,
    request: SourceLookupRequest,
    preview: SourcePreview,
    holdBack: boolean
  ): Promise<CodeLensCount | null> {
    return new Promise((resolve) => {
      const existing = waitingSpots.get(key);
      if (existing) {
        existing.waiters.push(resolve);
        return;
      }

      waitingSpots.set(key, {
        request,
        preview,
        waiters: [resolve],
        askedAt: Date.now(),
        tries: 0
      });

      if (holdBack) {
        heldUntilServerIsReady.push(key);
      } else {
        semanticScheduler.request([key]);
      }
    });
  }

  /**
   * Stop counting for the file that was on screen. Everything still queued is
   * dropped; anyone still waiting is told we have no number, which takes the
   * "counting references…" line off a margin nobody is looking at any more.
   */
  function stopCountingForTheOldFile(): void {
    semanticScheduler.clear();
    heldUntilServerIsReady = [];
    const abandoned = [...waitingSpots.values()];
    waitingSpots.clear();
    for (const spot of abandoned) {
      for (const waiter of spot.waiters) waiter(null);
    }
  }

  /** Every remembered number in every project, let go. */
  function forgetReferenceCounts(): void {
    countStore.forgetEverything();
  }

  /**
   * One file was edited or saved, so its numbers are counted again.
   *
   * Only the language server's numbers, and only for that file: they are about
   * this file's symbols and editing it can genuinely change them. The
   * plain-text counts are left alone on purpose. They count a NAME across the
   * project's saved files, which typing in the open buffer does not change, and
   * throwing them away every time the reader paused would put the browser
   * preview back to reading the whole project every couple of seconds.
   */
  function forgetFileReferenceCounts(filePath: string): void {
    if (!filePath) return;
    countStore.forgetFile(projectRoot, normalizeProjectPath(filePath));
  }

  /**
   * The symbols a file's margin rows sit above.
   *
   * The editor can find these itself by reading the text, but it only knows how
   * to read TypeScript, JavaScript and C#. The language server knows the shape
   * of whatever it is looking at, which is what puts margin rows on Rust and
   * Svelte files for the first time. When the desktop app is too old to answer
   * this question, or answers with nothing, we say so and the editor reads the
   * text as before.
   */
  async function lookupCodeLensAnchors(asked: SourcePreview): Promise<SourceSymbol[] | null> {
    if (!isNativeTauriRuntime() || !projectRoot) return null;

    // Ask about the file on screen, not about the copy the editor handed us:
    // the editor describes a file by the language Monaco paints it in, and
    // Monaco paints Svelte as HTML. The language server needs to be told it is
    // Svelte, and only this side knows that.
    const preview = activePreview;
    if (!preview) return null;
    if (normalizeProjectPath(asked.path) !== normalizeProjectPath(preview.path)) return null;
    if (!sourceSupportsLanguageIntelligence(preview.language)) return null;
    if (!(await hasBackendCapability('lspDocumentSymbols'))) return null;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      countInvoke('find_source_lsp_document_symbols');
      const startedAt = Date.now();
      const symbols = await invoke<
        { name: string; kind: string; line: number; character: number }[]
      >('find_source_lsp_document_symbols', {
        root: projectRoot,
        language: preview.language,
        path: preview.path
      });
      if (!Array.isArray(symbols) || symbols.length === 0) return null;

      reportLensTiming(
        `the language server named ${symbols.length} symbol(s) in ${preview.fileName} in ${
          Date.now() - startedAt
        }ms`
      );
      // The backend counts lines and characters from zero; everything on this
      // side of the wire counts from one.
      return symbols.slice(0, maxCodeLensSymbols).map((symbol) => ({
        name: symbol.name,
        kind: String(symbol.kind).toLowerCase(),
        line: symbol.line + 1,
        column: symbol.character + 1,
        detail: symbol.name
      }));
    } catch {
      return null;
    }
  }

  /**
   * One line about how the margin numbers are getting on, when somebody has
   * asked to see them. Switch it on from the browser console with
   * `window.mcbShowLensTiming = true`; it is off for everyone else.
   */
  function reportLensTiming(sentence: string): void {
    if ((globalThis as { mcbShowLensTiming?: unknown }).mcbShowLensTiming !== true) return;
    console.info(`Reference counts: ${sentence}`);
  }

  /**
   * Contents of some OTHER file — what a peek window shows for each hit, and
   * what "jump to definition" previews. Answers from the file on screen or
   * from memory when it can, and reads from disk at most once per file.
   */
  async function loadExternalPreview(record: SourceRecord): Promise<SourcePreview | null> {
    const sourceRecord = sourceRecordFromPath(projectRoot, record.path);

    if (activePreview?.path === sourceRecord.path) {
      return previewFromContent(
        { ...sourceRecord, byteCount: new TextEncoder().encode(draftContent).length },
        draftContent
      );
    }

    const cached = externalPreviewCache.get(sourceRecord.path);
    if (cached) return cached;

    countInvoke('read_source_file');
    const preview = await readSourceFromTauri(sourceRecord).catch(() => null);
    if (preview) externalPreviewCache.set(sourceRecord.path, preview);
    return preview;
  }

  async function lookupHover(request: SourceLookupRequest): Promise<SourceLspHover | null> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return null;
    try {
      countInvoke('find_source_lsp_hover');
      return await findSourceLspHoverFromTauri(preview, {
        root: lookupRoot(),
        line: request.line,
        column: request.column
      });
    } catch {
      return null;
    }
  }

  async function lookupCompletions(
    request: SourceLookupRequest
  ): Promise<SourceCompletionItem[]> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return [];
    try {
      countInvoke('find_source_lsp_completions');
      return (
        (await findSourceLspCompletionsFromTauri(preview, {
          root: lookupRoot(),
          line: request.line,
          column: request.column,
          limit: maxSourceCompletionResults
        })) ?? []
      );
    } catch {
      return [];
    }
  }

  async function lookupDocumentHighlights(
    request: SourceLookupRequest
  ): Promise<SourceDocumentHighlight[]> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return [];
    try {
      countInvoke('find_source_lsp_document_highlights');
      return (
        (await findSourceLspDocumentHighlightsFromTauri(preview, {
          root: lookupRoot(),
          line: request.line,
          column: request.column,
          limit: 100
        })) ?? []
      );
    } catch {
      return [];
    }
  }

  async function lookupSignatureHelp(
    request: SourceLookupRequest
  ): Promise<SourceSignatureHelp | null> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return null;
    try {
      countInvoke('find_source_lsp_signature_help');
      return await findSourceLspSignatureHelpFromTauri(preview, {
        root: lookupRoot(),
        line: request.line,
        column: request.column
      });
    } catch {
      return null;
    }
  }

  async function lookupInlayHints(request: SourceInlayHintRequest): Promise<SourceInlayHint[]> {
    const preview = previewWithDraft();
    if (!preview || !languageIntelligenceAvailable()) return [];
    try {
      countInvoke('find_source_lsp_inlay_hints');
      return (
        (await findSourceLspInlayHintsFromTauri(preview, {
          root: lookupRoot(),
          line: request.startLine,
          column: request.startColumn,
          limit: 200
        })) ?? []
      );
    } catch {
      return [];
    }
  }

  /** Colouring for the file Monaco is about to paint (its own copy of it). */
  async function lookupSemanticTokens(preview: SourcePreview): Promise<SourceSemanticToken[]> {
    if (!sourceSupportsLanguageIntelligence(preview.language)) return [];
    try {
      countInvoke('find_source_lsp_semantic_tokens');
      return (
        (await findSourceLspSemanticTokensFromTauri(preview, {
          root: lookupRoot(),
          line: 1,
          column: 1,
          limit: 5000
        })) ?? []
      );
    } catch {
      return [];
    }
  }

  const callbacks: SourceIntelligenceCallbacks = {
    onDefinitionLookup: findDefinitions,
    onReferenceLookup: findReferences,
    onReferenceCountLookup: countReferencesForCodeLens,
    onCodeLensAnchorLookup: lookupCodeLensAnchors,
    onReferenceCountsOutOfDate: forgetFileReferenceCounts,
    onExternalPreviewLookup: loadExternalPreview,
    onHoverLookup: lookupHover,
    onCompletionLookup: lookupCompletions,
    onDocumentHighlightLookup: lookupDocumentHighlights,
    onSignatureHelpLookup: lookupSignatureHelp,
    onInlayHintLookup: lookupInlayHints,
    onSemanticTokensLookup: lookupSemanticTokens
  };

  return {
    setProjectRoot(nextProjectRoot: string | null): void {
      const normalized =
        nextProjectRoot && nextProjectRoot.trim().length > 0 ? nextProjectRoot : null;
      if (normalized === projectRoot) return;
      projectRoot = normalized;
      // The numbers counted for the old project are NOT thrown away — they are
      // filed under it, and coming back finds them. What must stop is the
      // counting that was still going on for the file that was on screen.
      externalPreviewCache.clear();
      stopCountingForTheOldFile();
    },
    setActivePreview(preview: SourcePreview | null): void {
      if (preview?.path !== activePreview?.path) stopCountingForTheOldFile();
      activePreview = preview;
      draftContent = preview?.content ?? '';
    },
    setDraftContent(content: string): void {
      draftContent = content;
    },
    invalidatePreview(path: string): void {
      externalPreviewCache.delete(path);
      forgetFileReferenceCounts(path);
    },
    invalidateAllPreviews(): void {
      externalPreviewCache.clear();
      forgetReferenceCounts();
    },
    loadActiveFileDiagnostics,
    get projectRoot(): string | null {
      return projectRoot;
    },
    callbacks
  };
}

/**
 * The shell's one source-intelligence service. It is a module value because
 * there is exactly one code editor in the shell (Monaco's providers are
 * page-wide), and because the integrator needs a stable handle to call
 * `activate` on before the editor panel has ever been shown.
 */
export const sourceIntelligence = createSourceIntelligence();

/**
 * Integration seam: called when the editor tab is first shown, with the
 * project root of the active session. Idempotent, and does NO backend work —
 * it only tells the panel and the lookups which project to ask about. Reading
 * a file happens when the user opens one.
 */
export function activate(projectRoot?: string | null): void {
  sourceIntelligence.setProjectRoot(projectRoot ?? null);
  activateEditor(projectRoot ?? null);
}

/** Forget one file's remembered contents (call after that file is written). */
export function invalidatePreview(path: string): void {
  sourceIntelligence.invalidatePreview(path);
}

/** What came back from turning the C# language server off or on. */
export interface CsharpLanguageServerToggleResult {
  /** Whether the server is allowed to run now. */
  enabled: boolean;
  /** How many running servers were stopped (0 when it was not running). */
  stoppedServers: number;
  /** A whole sentence to show the reader. */
  message: string;
  /** False when this build of the app has no such switch; `message` says so. */
  supported: boolean;
}

/**
 * Turn the C# language server off or on.
 *
 * The C# server is by far the most expensive thing the app starts — around
 * 800MB once it has read a large solution — and a reader who is not writing C#
 * gets nothing back for it. Switching it off keeps the margin counts and the
 * project-wide search working, because those read the files directly; what goes
 * away is the squiggles under mistakes and the precision of go-to-definition on
 * an overloaded name.
 *
 * Call this on start-up with the reader's saved setting as well as when the
 * switch is flipped: the desktop app forgets between launches and starts the
 * server allowed.
 */
export async function setCsharpLanguageServerEnabled(
  enabled: boolean
): Promise<CsharpLanguageServerToggleResult> {
  const unsupported: CsharpLanguageServerToggleResult = {
    enabled: true,
    stoppedServers: 0,
    supported: false,
    message:
      'This build of the app cannot do this yet — restart the desktop app after updating.'
  };

  if (!isNativeTauriRuntime()) return unsupported;
  if (!(await hasBackendCapability('csharpLanguageServerToggle'))) return unsupported;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    countInvoke('set_csharp_language_server_enabled');
    const result = await invoke<Omit<CsharpLanguageServerToggleResult, 'supported'>>(
      'set_csharp_language_server_enabled',
      { enabled }
    );
    return { ...result, supported: true };
  } catch {
    return {
      enabled: !enabled,
      stoppedServers: 0,
      supported: true,
      message: 'The C# language server setting could not be changed just now. Please try again.'
    };
  }
}
