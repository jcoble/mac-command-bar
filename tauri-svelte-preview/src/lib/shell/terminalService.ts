/**
 * The single terminal-IO owner for the /next shell.
 *
 * Everything that touches a PTY goes through here: ONE backend output listener
 * for the whole app (not one per view), one `ownedId -> ptySessionId` map, and
 * one manager instance holding the live views.
 *
 * Two rules this module exists to enforce:
 *
 * 1. **Exactly one listener.** The old shell registered a listener per terminal,
 *    so output was written N times and every re-open leaked another
 *    subscription. Here `attach()` is idempotent and routes by sessionId through
 *    `manager.feedSession`, which writes to the view REGARDLESS of visibility —
 *    that is what keeps background conversations live.
 * 2. **`dispose()` never kills a PTY.** The old shell's
 *    `disposeEmbeddedTerminal` closed the backend session on teardown, so a
 *    dev-server reload murdered every running agent. Teardown here only
 *    unsubscribes and drops views; `closeOwned` is the ONLY path that closes a
 *    PTY.
 */
// NOTE: explicit `.ts` specifiers (as in `gitGraphViewModel.ts` et al) so this
// module — and therefore its test — loads under `node --experimental-strip-types`.
import {
  createLiveConversationTerminals,
  type TerminalView
} from '../liveConversationTerminals.ts';
import type { OwnedSession } from './ownedSessions.ts';
import {
  closeTerminalSessionFromTauri,
  listTerminalSessionsFromTauri,
  listenToTerminalOutput,
  readTerminalSessionScrollbackFromTauri,
  resizeTerminalSessionFromTauri,
  startTerminalSessionFromTauri,
  writeTerminalSessionFromTauri,
  type TerminalOutputPayload,
  type TerminalSessionInfo,
  type TerminalStartRequest
} from '../tauriSource.ts';

/**
 * The PTY transport, injected so the service can be tested without Tauri.
 * `tauriTerminalBackend` is the production implementation.
 */
export type TerminalBackend = {
  start(request: TerminalStartRequest): Promise<TerminalSessionInfo | null>;
  write(sessionId: string, data: string): Promise<boolean>;
  resize(sessionId: string, cols: number, rows: number): Promise<boolean>;
  close(sessionId: string): Promise<boolean>;
  readScrollback(sessionId: string): Promise<string | null>;
  list(): Promise<TerminalSessionInfo[] | null>;
  listen(handler: (payload: TerminalOutputPayload) => void): Promise<(() => void) | null>;
};

/** Side effects a view reports back to the service (mirrors `xtermFactory`). */
export type TerminalViewHooks = {
  onData(data: string): void;
  onResize(cols: number, rows: number): void;
};

export type TerminalService = {
  /** Register the ONE backend output listener. Safe to call repeatedly. */
  attach(): Promise<void>;
  /** Spawn a PTY for `owned`, mount its view on `host`, replay the resume command. */
  startOwned(owned: OwnedSession, host: HTMLElement): Promise<string | null>;
  /** Re-attach to a PTY that survived a reload, hydrating saved scrollback. */
  adoptExisting(owned: OwnedSession, host: HTMLElement): Promise<boolean>;
  /** Make one owned session's terminal the visible one. */
  show(ownedId: string): void;
  /** Close a PTY and drop its view. The ONLY path that kills a session. */
  closeOwned(ownedId: string): Promise<void>;
  /** Unlisten + drop all views. NEVER closes a PTY. */
  dispose(): void;
};

/**
 * Wrap the Tauri terminal commands 1:1, reporting each call to `count` so the
 * dev HUD can show how much IPC the shell actually does.
 */
export function tauriTerminalBackend(count: (command: string) => void): TerminalBackend {
  return {
    start(request: TerminalStartRequest): Promise<TerminalSessionInfo | null> {
      count('start_terminal_session');
      return startTerminalSessionFromTauri(request);
    },
    write(sessionId: string, data: string): Promise<boolean> {
      count('write_terminal_session');
      return writeTerminalSessionFromTauri(sessionId, data);
    },
    resize(sessionId: string, cols: number, rows: number): Promise<boolean> {
      count('resize_terminal_session');
      return resizeTerminalSessionFromTauri(sessionId, cols, rows);
    },
    close(sessionId: string): Promise<boolean> {
      count('close_terminal_session');
      return closeTerminalSessionFromTauri(sessionId);
    },
    readScrollback(sessionId: string): Promise<string | null> {
      count('read_terminal_session_scrollback');
      return readTerminalSessionScrollbackFromTauri(sessionId);
    },
    list(): Promise<TerminalSessionInfo[] | null> {
      count('list_terminal_sessions');
      return listTerminalSessionsFromTauri();
    },
    listen(handler: (payload: TerminalOutputPayload) => void): Promise<(() => void) | null> {
      count('listen:terminal_output');
      return listenToTerminalOutput(handler);
    }
  };
}

export function createTerminalService(opts: {
  backend: TerminalBackend;
  createView: (host: HTMLElement, hooks: TerminalViewHooks) => TerminalView;
  onExit?(ownedId: string, payload: TerminalOutputPayload): void;
}): TerminalService {
  const { backend, createView, onExit } = opts;

  /**
   * ownedId -> ptySessionId. The service's own routing table: `ownedId` is the
   * stable key everywhere else in the shell, the PTY id is mutable (a restart
   * mints a new one), so it lives here and nowhere else.
   */
  const ptyByOwned = new Map<string, string>();
  /**
   * sessionId -> scrollback, populated for the duration of ONE `adoptExisting`
   * call. The manager's `readScrollback` dep is synchronous, so the async read
   * has to land here before `ensureView` runs; the entry is dropped right after.
   */
  const scrollbackCache = new Map<string, string>();
  /**
   * The ownedId whose view is currently being constructed. `createTerminal` is
   * `(host) => view` with no key, so this hands the key to the closure that
   * `ensureView` is about to invoke.
   */
  let creatingFor: string | null = null;
  let unlisten: (() => void) | null = null;
  let attaching: Promise<void> | null = null;

  /**
   * Point `ownedId` at `ptyId`, evicting any stale claim on either side so the
   * map stays 1:1 — a restarted session must never leave the previous PTY id
   * reachable, and two owned sessions must never share one PTY.
   */
  function setPty(ownedId: string, ptyId: string): void {
    for (const [otherOwned, otherPty] of ptyByOwned) {
      if (otherPty === ptyId && otherOwned !== ownedId) {
        ptyByOwned.delete(otherOwned);
      }
    }
    ptyByOwned.set(ownedId, ptyId);
  }

  const manager = createLiveConversationTerminals({
    createTerminal: (host: HTMLElement): TerminalView => {
      const ownedId = creatingFor;
      if (ownedId == null) {
        throw new Error('createTerminal called outside ensureViewFor — no owned session in scope');
      }
      // Input is routed by ownedId, resolved to a PTY id at KEYSTROKE time: the
      // view outlives any single PTY (restart mints a new id), so capturing the
      // id here would send later keystrokes to a dead session.
      return createView(host, {
        onData(data: string): void {
          const ptyId = ptyByOwned.get(ownedId);
          if (ptyId) {
            void backend.write(ptyId, data);
          }
        },
        onResize(cols: number, rows: number): void {
          const ptyId = ptyByOwned.get(ownedId);
          if (ptyId) {
            void backend.resize(ptyId, cols, rows);
          }
        }
      });
    },
    writeSession: (sessionId: string, data: string): void => {
      void backend.write(sessionId, data);
    },
    resizeSession: (): void => {
      // Intentionally empty: `showView` calls `view.fit()` immediately before
      // this, and fit reports the new geometry through `onResize` above — which
      // already issues the backend resize with real cols/rows. This dep only
      // gets a sessionId, so re-sending here would be a duplicate guess.
    },
    readScrollback: (sessionId: string): string | null =>
      scrollbackCache.get(sessionId) ?? null
  });

  /** Run `ensureView` with `key` visible to the `createTerminal` closure. */
  function ensureViewFor(
    ownedId: string,
    options: { host: HTMLElement; sessionId?: string | null }
  ): TerminalView {
    creatingFor = ownedId;
    try {
      return manager.ensureView(ownedId, options);
    } finally {
      creatingFor = null;
    }
  }

  async function attach(): Promise<void> {
    if (unlisten) {
      return;
    }
    if (attaching) {
      // Two callers raced: share the in-flight subscription instead of opening
      // a second one.
      return attaching;
    }
    attaching = (async () => {
      const stop = await backend.listen((payload: TerminalOutputPayload): void => {
        if (payload.data) {
          // Feed FIRST, including on the terminating payload, so the last bytes
          // an agent printed are not lost. The manager writes to hidden views
          // too — that is what keeps background sessions live.
          manager.feedSession(payload.sessionId, payload.data);
        }
        if (!payload.terminated) {
          return;
        }
        const ownedId = manager.keyForSession(payload.sessionId);
        if (ownedId == null) {
          return;
        }
        manager.markTerminated(ownedId);
        // The view and its scrollback stay on screen; flipping session STATE is
        // the store's job (this service only reports the exit).
        onExit?.(ownedId, payload);
      });
      unlisten = stop ?? null;
    })();
    try {
      await attaching;
    } finally {
      attaching = null;
    }
  }

  async function startOwned(owned: OwnedSession, host: HTMLElement): Promise<string | null> {
    const hadView = manager.hasView(owned.ownedId);
    const info = await backend.start({ cwd: owned.cwd, ownedId: owned.ownedId });
    if (!info) {
      // Never leave a half-built view behind for a session that has no PTY.
      if (!hadView && manager.hasView(owned.ownedId)) {
        manager.closeView(owned.ownedId);
      }
      return null;
    }

    // Map BEFORE the view exists so the very first keystroke can already route.
    setPty(owned.ownedId, info.sessionId);
    ensureViewFor(owned.ownedId, { host });
    // Bind immediately: output for this sessionId starts arriving on the shared
    // listener the moment the process spawns, and an unbound session is dropped.
    manager.bindSession(owned.ownedId, info.sessionId);

    if (owned.resumeCommand) {
      await backend.write(info.sessionId, `${owned.resumeCommand}\r`);
    }

    return info.sessionId;
  }

  async function adoptExisting(owned: OwnedSession, host: HTMLElement): Promise<boolean> {
    const ptyId = owned.ptySessionId;
    if (!ptyId) {
      // Nothing survived: the caller has to `startOwned` instead.
      return false;
    }

    const scrollback = await backend.readScrollback(ptyId);
    if (scrollback) {
      scrollbackCache.set(ptyId, scrollback);
    }
    try {
      setPty(owned.ownedId, ptyId);
      // `sessionId` here makes the manager hydrate the new view from the cache
      // above (its readScrollback dep is synchronous, hence the staging map).
      ensureViewFor(owned.ownedId, { host, sessionId: ptyId });
      manager.bindSession(owned.ownedId, ptyId);
    } finally {
      scrollbackCache.delete(ptyId);
    }
    return true;
  }

  function show(ownedId: string): void {
    manager.showView(ownedId);
  }

  async function closeOwned(ownedId: string): Promise<void> {
    const ptyId = ptyByOwned.get(ownedId);
    // Drop the mapping first so a concurrent close can't double-kill the PTY.
    ptyByOwned.delete(ownedId);
    manager.closeView(ownedId);
    if (ptyId) {
      scrollbackCache.delete(ptyId);
      await backend.close(ptyId);
    }
  }

  function dispose(): void {
    // NO backend.close — ever. A reload/unmount must leave every agent running;
    // `adoptExisting` picks them back up on the next mount.
    unlisten?.();
    unlisten = null;
    manager.disposeAll();
    ptyByOwned.clear();
    scrollbackCache.clear();
  }

  return { attach, startOwned, adoptExisting, show, closeOwned, dispose };
}
