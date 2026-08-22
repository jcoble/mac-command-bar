/**
 * editorStore.svelte.ts — Svelte 5 runes state for the /next editor panel.
 *
 * Holds STATE ONLY: which files are open (in strip order), which one is
 * showing, and per-file {contents, reveal-line request, loading, error}.
 *
 * Two rules this module exists to enforce, same as the session rail store:
 *
 * 1. **No backend, ever.** Reading a file, warming the language server and
 *    every lookup happen in `EditorPanel.svelte` / `sourceIntelligence.ts`
 *    and land here as plain mutations.
 * 2. **No `$effect`, no derived reads that do work.** Every decision is a pure
 *    function in `editorStoreOps.ts`; this module assigns its result.
 *
 * Nothing here is persisted this slice: reopening the shell starts with an
 * empty editor, which is also what keeps launch free of file reads.
 */
import type { SourcePreview, SourceRecord, SourceSymbol } from '../../sourceData.ts';
import {
  activePathAfterClose,
  closeOpenFile,
  findOpenFile,
  openEditorFileFromRecord,
  patchOpenFile,
  revealLineInOpenFile,
  upsertOpenFile,
  type OpenEditorFile,
  type OpenEditorFileOptions
} from './editorStoreOps.ts';
import { sourceRecordFromPath } from './sourceRecordFromPath.ts';
import {
  setOpenTabDocumentBytes,
  textBytes
} from '../resourceDiagnostics.svelte.ts';

export type { OpenEditorFile } from './editorStoreOps.ts';

/**
 * The single reactive editor state. Read fields directly in components
 * (`editorState.openFiles`, …); mutate only through the functions below.
 */
export const editorState = $state<{
  /** The panel has been shown at least once (or a file open forced it). */
  activated: boolean;
  /** Project root the open files belong to; drives language-server lookups. */
  projectRoot: string | null;
  /** Open files, in strip order. */
  openFiles: OpenEditorFile[];
  /** Path of the file on screen. */
  activePath: string | null;
  /** Symbols Monaco found in the file on screen. */
  symbols: SourceSymbol[];
}>({
  activated: false,
  projectRoot: null,
  openFiles: [],
  activePath: null,
  symbols: []
});

function publishOpenTabDocumentBytes(): void {
  setOpenTabDocumentBytes(
    editorState.openFiles.reduce(
      (total, file) => total + textBytes(file.draftContent ?? file.preview?.content ?? ''),
      0
    )
  );
}

/**
 * Integration seam: the shell calls this the first time the editor tab is
 * shown, handing over the project root of the active session. Idempotent —
 * calling it again only refreshes the root. It performs NO IO: the editor has
 * nothing to load until a file is actually requested.
 */
export function activateEditor(projectRoot?: string | null): void {
  editorState.activated = true;
  if (projectRoot !== undefined) setEditorProjectRoot(projectRoot);
}

/** Point the editor at a project root (language-server lookups use it). */
export function setEditorProjectRoot(projectRoot: string | null): void {
  editorState.projectRoot = projectRoot && projectRoot.trim().length > 0 ? projectRoot : null;
}

/** The file on screen, or `null`. */
export function activeEditorFile(): OpenEditorFile | null {
  return editorState.activePath ? findOpenFile(editorState.openFiles, editorState.activePath) : null;
}

/** The entry for `path`, or `null`. */
export function editorFileFor(path: string): OpenEditorFile | null {
  return findOpenFile(editorState.openFiles, path);
}

/**
 * Put `record` in the strip (no-op if already there) and show it. Returns the
 * entry so the caller can decide whether it still needs reading.
 */
export function openEditorFile(
  record: SourceRecord,
  options: OpenEditorFileOptions = {}
): OpenEditorFile {
  editorState.openFiles = upsertOpenFile(editorState.openFiles, record, options);
  editorState.activePath = record.path;
  publishOpenTabDocumentBytes();
  return findOpenFile(editorState.openFiles, record.path)!;
}

/** Make a preview tab durable in the active-session strip. */
export function pinEditorFile(path: string): void {
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, { previewTab: false });
}

/** Show an already-open file. */
export function setActiveEditorFile(path: string): void {
  if (findOpenFile(editorState.openFiles, path)) editorState.activePath = path;
}

/** Close one file and pick what to show next. */
export function closeEditorFile(path: string): void {
  const previousActivePath = editorState.activePath;
  const nextActivePath = activePathAfterClose(editorState.openFiles, path, previousActivePath);
  editorState.openFiles = closeOpenFile(editorState.openFiles, path);
  editorState.activePath = nextActivePath;
  publishOpenTabDocumentBytes();
  // The symbol list belongs to whatever was on screen; a switch invalidates it.
  if (nextActivePath !== previousActivePath) editorState.symbols = [];
}

/** Mark a file's read as started (clears any previous failure). */
export function markEditorFileLoading(path: string): void {
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, {
    loading: true,
    error: null
  });
}

/** A read finished: store the contents. */
export function setEditorFilePreview(
  path: string,
  preview: SourcePreview,
  committed = false
): void {
  const file = editorFileFor(path);
  const retainedDraft = committed ? null : file?.draftContent ?? null;
  const conflict =
    !committed && file?.dirty
      ? file.conflict
        ?? (file.preview && file.preview.content !== preview.content
          ? 'File changed on disk while this draft has unsaved edits.'
          : null)
      : null;
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, {
    preview,
    draftContent: retainedDraft ?? preview.content,
    dirty: retainedDraft !== null && retainedDraft !== preview.content,
    conflict,
    saving: false,
    loading: false,
    error: null
  });
  publishOpenTabDocumentBytes();
}

export function clearEditorFileLoading(path: string): void {
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, {
    loading: false
  });
}

/** Keep an unsaved Monaco edit with the session that owns this editor tab. */
export function setEditorFileDraft(path: string, content: string): void {
  const file = editorFileFor(path);
  if (!file?.preview) return;
  // An unchanged draft must not touch state: the editor republishes its
  // content from inside an effect, and a no-op write here loops that effect.
  if (file.draftContent === content) return;
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, {
    draftContent: content,
    dirty: content !== file.preview.content,
    conflict: content !== file.preview.content ? file.conflict : null
  });
  publishOpenTabDocumentBytes();
}

/** Mark or clear the save spinner without replacing the draft. */
export function setEditorFileSaving(path: string, saving: boolean): void {
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, { saving });
}

/** A read failed: `message` is shown to the user as-is, so keep it plain. */
export function setEditorFileError(path: string, message: string): void {
  editorState.openFiles = patchOpenFile(editorState.openFiles, path, {
    loading: false,
    error: message
  });
}

/** Ask Monaco to scroll `path` to `line` (pass `null` to stop asking). */
export function revealEditorLine(path: string, line: number | null): void {
  editorState.openFiles = revealLineInOpenFile(editorState.openFiles, path, line);
}

/** Symbols Monaco extracted from the file on screen. */
export function setEditorSymbols(symbols: SourceSymbol[]): void {
  editorState.symbols = symbols;
}

/**
 * Put a session's lightweight tab descriptors back without reading a file.
 */
export function restoreEditorFiles(
  files: readonly { path: string; draftContent?: string }[],
  activePath: string | null = null
): void {
  editorState.openFiles = files.map((file) => ({
    ...openEditorFileFromRecord(sourceRecordFromPath(editorState.projectRoot, file.path)),
    draftContent: file.draftContent ?? null,
    dirty: file.draftContent !== undefined,
    conflict: null
  }));
  editorState.activePath = editorState.openFiles.some((file) => file.path === activePath)
    ? activePath
    : null;
  editorState.symbols = [];
  publishOpenTabDocumentBytes();
}

/** Drop everything (used when the shell tears the editor down). */
export function resetEditorState(): void {
  editorState.openFiles = [];
  editorState.activePath = null;
  editorState.symbols = [];
  publishOpenTabDocumentBytes();
}
