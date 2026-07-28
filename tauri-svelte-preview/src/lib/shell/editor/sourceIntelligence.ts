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
 *    that answered from bundled demo files; /next never invents results.
 *  - **Every backend call is counted** with `countInvoke('<command name>')`
 *    immediately before it, so the dev counter tells the truth.
 *  - **Nothing runs on import.** The first backend call of any kind happens
 *    when the user opens a file.
 *
 * The reference-count budget below is load-bearing and is copied verbatim —
 * see `countReferencesForCodeLens`.
 */
import {
  previewFromContent,
  sourceSupportsLanguageIntelligence,
  type SourceCompletionItem,
  type SourceDefinitionTarget,
  type SourceDocumentHighlight,
  type SourceInlayHint,
  type SourceLspHover,
  type SourcePreview,
  type SourceRecord,
  type SourceReferenceTarget,
  type SourceSemanticToken,
  type SourceSignatureHelp
} from '../../sourceData.ts';
import {
  findSourceDefinitionsFromTauri,
  findSourceLspCompletionsFromTauri,
  findSourceLspDefinitionsFromTauri,
  findSourceLspDocumentHighlightsFromTauri,
  findSourceLspHoverFromTauri,
  findSourceLspInlayHintsFromTauri,
  findSourceLspReferencesFromTauri,
  findSourceLspSemanticTokensFromTauri,
  findSourceLspSignatureHelpFromTauri,
  findSourceReferencesFromTauri,
  isNativeTauriRuntime,
  readSourceFromTauri
} from '../../tauriSource.ts';
import { countInvoke } from '../devInvokeCounter.svelte.ts';
import { activateEditor } from './editorStore.svelte.ts';
import { sourceRecordFromPath } from './sourceRecordFromPath.ts';

// ── Budgets (ported verbatim from the old shell) ──────────────────────────────

/** Most definitions one lookup will return. */
export const maxSourceDefinitionResults = 20;
/** Most references (and reference counts) one lookup will return. */
export const maxSourceReferenceResults = 50;
/** Most completion items one lookup will return. */
export const maxSourceCompletionResults = 50;
/**
 * A reference COUNT in the margin is a low-priority annotation, so it gets a
 * short budget. A cold language server can block for its full 6-second
 * timeout, and roughly fifteen visible margin counts all waiting that long is
 * what used to freeze the editor for half a minute.
 */
export const codeLensReferenceCountTimeoutMs = 700;
/**
 * Above this many scanned files the plain-text fallback for a margin count is
 * skipped on the desktop app: it reads file contents across the whole index,
 * which for a large project means re-reading the project per symbol (the
 * other half of that freeze). The language server owns big projects. In the
 * browser preview the fallback resolves in memory, so it stays — dropping it
 * there made every count disappear once already.
 */
export const maxCodeLensNativeReferenceScanRecords = 1500;

// ── The shapes Monaco hands us ────────────────────────────────────────────────

/** What Monaco knows about the spot the user is asking about. */
export interface SourceLookupRequest {
  symbolName: string;
  /** 1-based. */
  line: number;
  /** 1-based. */
  column: number;
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
  onReferenceCountLookup(request: SourceLookupRequest): Promise<number | null>;
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
  /** The scanned file list, when a scan exists; the plain-text tier searches it. */
  setRecords(records: SourceRecord[]): void;
  /** Forget the remembered contents of one file (call after it is written). */
  invalidatePreview(path: string): void;
  /** Forget every remembered file (call when the project changes). */
  invalidateAllPreviews(): void;
  /** Read-only view of the current context, for status lines and tests. */
  readonly projectRoot: string | null;
  /** The callbacks to spread onto `MonacoSourceEditor`. */
  readonly callbacks: SourceIntelligenceCallbacks;
}

/**
 * Resolve `promise`, but give up after `ms` and answer "unknown" instead of
 * waiting. Used only where a slow answer is worse than no answer.
 */
function raceCountTimeout<T>(promise: Promise<T>, ms: number): Promise<T | null> {
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      resolve(null);
    }, ms);
    const finish = (value: T | null) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(value);
    };
    promise.then(finish, () => finish(null));
  });
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
  let records: SourceRecord[] = [];
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

  // ── Tier 2: the backend's plain-text scan ──────────────────────────────────

  async function nativeDefinitions(symbolName: string): Promise<SourceDefinitionTarget[] | null> {
    if (records.length === 0) return null;
    countInvoke('find_source_definitions');
    return findSourceDefinitionsFromTauri(records, symbolName, maxSourceDefinitionResults).catch(
      () => null
    );
  }

  async function nativeReferences(symbolName: string): Promise<SourceReferenceTarget[] | null> {
    if (records.length === 0) return null;
    countInvoke('find_source_references');
    return findSourceReferencesFromTauri(records, symbolName, maxSourceReferenceResults).catch(
      () => null
    );
  }

  // ── The callbacks ──────────────────────────────────────────────────────────

  async function findDefinitions(
    request: SourceLookupRequest
  ): Promise<SourceDefinitionTarget[]> {
    const symbolName = request.symbolName.trim();
    if (!symbolName) return [];
    try {
      const lspTargets = await lspDefinitions(request);
      if (lspTargets?.length) return lspTargets;
      return (await nativeDefinitions(symbolName)) ?? [];
    } catch {
      return [];
    }
  }

  async function findReferences(request: SourceLookupRequest): Promise<SourceReferenceTarget[]> {
    const symbolName = request.symbolName.trim();
    if (!symbolName) return [];
    try {
      const lspTargets = await lspReferences(request);
      if (lspTargets?.length) return lspTargets;
      return (await nativeReferences(symbolName)) ?? [];
    } catch {
      return [];
    }
  }

  /**
   * The "N references" number drawn above a symbol. Returns `null` for
   * "unknown", which draws nothing at all — better than a wrong number.
   */
  async function countReferencesForCodeLens(
    request: SourceLookupRequest
  ): Promise<number | null> {
    const symbolName = request.symbolName.trim();
    if (!symbolName) return null;

    if (activePreview && languageIntelligenceAvailable()) {
      const lspTargets = await raceCountTimeout(
        lspReferences(request),
        codeLensReferenceCountTimeoutMs
      );
      if (lspTargets) return lspTargets.length;
    }

    if (isNativeTauriRuntime() && records.length > maxCodeLensNativeReferenceScanRecords) {
      return null;
    }

    try {
      const nativeTargets = await nativeReferences(symbolName);
      return nativeTargets ? nativeTargets.length : null;
    } catch {
      return null;
    }
  }

  /**
   * Contents of some OTHER file — what a peek window shows for each hit, and
   * what "jump to definition" previews. Answers from the file on screen or
   * from memory when it can, and reads from disk at most once per file.
   */
  async function loadExternalPreview(record: SourceRecord): Promise<SourcePreview | null> {
    const sourceRecord = sourceRecordFromPath(projectRoot, record.path, records);

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
      // Files remembered under the old project must not answer for the new one.
      externalPreviewCache.clear();
    },
    setActivePreview(preview: SourcePreview | null): void {
      activePreview = preview;
      draftContent = preview?.content ?? '';
    },
    setDraftContent(content: string): void {
      draftContent = content;
    },
    setRecords(nextRecords: SourceRecord[]): void {
      records = nextRecords;
    },
    invalidatePreview(path: string): void {
      externalPreviewCache.delete(path);
    },
    invalidateAllPreviews(): void {
      externalPreviewCache.clear();
    },
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
