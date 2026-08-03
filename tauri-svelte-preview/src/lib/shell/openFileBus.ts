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
  /** 1-based line to reveal, if any. */
  line?: number;
  /** 1-based column to reveal, if any. */
  column?: number;
}

type Listener = (request: OpenFileRequest) => void;

const listeners = new Set<Listener>();
let pending: OpenFileRequest | null = null;

export function requestOpenFile(request: OpenFileRequest): void {
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
