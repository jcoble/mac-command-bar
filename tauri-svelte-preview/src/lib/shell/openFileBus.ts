/**
 * openFileBus.ts — cross-panel "open this file in the editor" requests.
 *
 * Pure fan-out, no IO, no Svelte: the explorer, palette, and git panels call
 * `requestOpenFile`; the editor subscribes with `onOpenFile`. A request made
 * before the editor exists is parked (last one wins) and delivered to the
 * first subscriber, so early clicks are never dropped.
 */

export interface OpenFileRequest {
  /** Absolute path of the file to open. */
  path: string;
  /** Workspace that owns the file, when the producer already knows it. */
  projectRoot?: string;
  /** Open for reading only: no editing, no saving. Set by producers that know
   * the file is not the active workspace's to change. */
  readOnly?: boolean;
  /** Single-click explorer open: reuse the one unpinned preview tab. */
  preview?: boolean;
  /** Double-click or explicit open: keep this file as a durable tab. */
  pin?: boolean;
  /** 1-based line to reveal, if any. */
  line?: number;
  /** 1-based column to reveal, if any. */
  column?: number;
}

type Listener = (request: OpenFileRequest) => void;

const listeners = new Set<Listener>();
let pending: OpenFileRequest | null = null;
let unavailableRoot: string | null = null;

export function setUnavailableOpenFileRoot(root: string | null): void {
  unavailableRoot = root?.trim().replace(/\/+$/, '') || null;
}

export function resolveConversationFilePath(path: string, sessionCwd: string): string {
  const candidate = path.trim();
  if (candidate.startsWith('/') || candidate.startsWith('~')) return candidate;
  return `${sessionCwd.replace(/\/+$/, '')}/${candidate.replace(/^\.\//, '')}`;
}

export function requestOpenFile(request: OpenFileRequest): void {
  if (
    unavailableRoot &&
    (request.path === unavailableRoot || request.path.startsWith(`${unavailableRoot}/`))
  ) {
    return;
  }
  if (listeners.size === 0) {
    pending = request;
    return;
  }
  for (const listener of listeners) listener(request);
}

/** Subscribe; returns the unsubscribe. A parked request is delivered at once. */
export function onOpenFile(listener: Listener): () => void {
  listeners.add(listener);
  if (pending) {
    const request = pending;
    pending = null;
    listener(request);
  }
  return () => {
    listeners.delete(listener);
  };
}
