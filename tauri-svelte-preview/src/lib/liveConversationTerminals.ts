/**
 * Pure, DOM-free-testable multi-terminal manager — the core of
 * "P3 — conversations as live workspaces".
 *
 * Keeps N terminals alive while only one is shown. The intent: switching
 * conversations must NEVER kill a PTY. There is one DOM host per conversation
 * view, N xterm-style views, and exactly one visible at a time. Inactive
 * conversations keep receiving output (so they stay "live") even though their
 * view is hidden.
 *
 * All collaborators are INJECTED so the manager can be unit-tested with spies
 * and no real xterm/DOM/Tauri dependency.
 */

/**
 * The injected terminal's surface (e.g. a thin wrapper over an xterm instance).
 * The manager only ever talks to terminals through this interface.
 */
export type TerminalView = {
  write(data: string): void;
  fit(): void;
  focus(): void;
  setVisible(visible: boolean): void;
  dispose(): void;
};

/**
 * Session-level side effects (PTY writes/resizes, scrollback reads). These are
 * injected so the manager stays pure and testable; in production they bridge to
 * the Tauri terminal backend.
 */
export type LiveConversationTerminalDeps = {
  /** Create a terminal view bound to the given DOM host. */
  createTerminal: (host: HTMLElement) => TerminalView;
  /** Write user/manager input to a live PTY session. */
  writeSession?: (sessionId: string, data: string) => void;
  /** Inform the PTY backend of the visible terminal's new size. */
  resizeSession?: (sessionId: string) => void;
  /** Read saved scrollback for a session (used when (re)hydrating a view). */
  readScrollback?: (sessionId: string) => string | null | undefined;
};

export type EnsureViewOptions = {
  host: HTMLElement;
  sessionId?: string | null;
};

/**
 * Internal bookkeeping record for one conversation's terminal.
 */
export type LiveConversationTerminalRecord = {
  key: string;
  view: TerminalView;
  sessionId: string | null;
  visible: boolean;
  terminated: boolean;
};

export type LiveConversationTerminals = {
  ensureView(key: string, opts: EnsureViewOptions): TerminalView;
  showView(key: string): void;
  activeKey(): string | null;
  liveKeys(): string[];
  hasView(key: string): boolean;
  bindSession(key: string, sessionId: string): void;
  keyForSession(sessionId: string): string | null;
  feed(key: string, data: string): void;
  feedSession(sessionId: string, data: string): void;
  markTerminated(key: string): void;
  closeView(key: string): void;
  disposeAll(): void;
};

export function createLiveConversationTerminals(
  deps: LiveConversationTerminalDeps
): LiveConversationTerminals {
  if (!deps || typeof deps.createTerminal !== 'function') {
    throw new TypeError(
      'createLiveConversationTerminals requires deps.createTerminal to be a function'
    );
  }

  const { createTerminal, writeSession, resizeSession, readScrollback } = deps;

  /** key -> record. Insertion order is preserved (used to pick a fallback active key). */
  const records = new Map<string, LiveConversationTerminalRecord>();
  /** sessionId -> key, the inverse of record.sessionId. */
  const sessionToKey = new Map<string, string>();
  let active: string | null = null;

  function getRecord(key: string): LiveConversationTerminalRecord | null {
    return records.get(key) ?? null;
  }

  /**
   * Detach any session->key mapping that points at `key`. Defensive: a record
   * only ever owns one sessionId, but we sweep so a stale reverse entry can
   * never outlive its record.
   */
  function clearSessionMappingFor(key: string): void {
    const record = records.get(key);
    if (record?.sessionId != null && sessionToKey.get(record.sessionId) === key) {
      sessionToKey.delete(record.sessionId);
    }
    for (const [sessionId, mappedKey] of sessionToKey) {
      if (mappedKey === key) {
        sessionToKey.delete(sessionId);
      }
    }
  }

  function ensureView(key: string, opts: EnsureViewOptions): TerminalView {
    const existing = records.get(key);
    if (existing) {
      // Idempotent: never double-create a view for the same key. If a sessionId
      // is supplied and the record has none yet, adopt it.
      if (opts?.sessionId != null && existing.sessionId == null) {
        bindSession(key, opts.sessionId);
      }
      return existing.view;
    }

    if (!opts || opts.host == null) {
      throw new TypeError(`ensureView("${key}") requires opts.host`);
    }

    const view = createTerminal(opts.host);
    const sessionId = opts.sessionId ?? null;
    const record: LiveConversationTerminalRecord = {
      key,
      view,
      sessionId,
      visible: false,
      terminated: false
    };
    records.set(key, record);

    if (sessionId != null) {
      sessionToKey.set(sessionId, key);
      // Hydrate from saved scrollback if the backend has any for this session.
      const scrollback = readScrollback?.(sessionId);
      if (scrollback) {
        view.write(scrollback);
      }
    }

    // The first view created becomes active when nothing else is showing.
    if (active === null) {
      showView(key);
    }

    return view;
  }

  function showView(key: string): void {
    const target = records.get(key);
    if (!target) {
      return;
    }

    for (const record of records.values()) {
      if (record === target) {
        continue;
      }
      // Spec: every OTHER record is set invisible on each show. Calling
      // setVisible(false) unconditionally (even on already-hidden views) keeps
      // the contract simple and is harmless/idempotent on a real xterm view.
      record.visible = false;
      record.view.setVisible(false);
    }

    target.visible = true;
    target.view.setVisible(true);
    target.view.fit();
    target.view.focus();
    active = key;

    // Only the shown terminal needs a PTY resize; inactive ones are untouched.
    if (target.sessionId != null) {
      resizeSession?.(target.sessionId);
    }
  }

  function activeKey(): string | null {
    return active;
  }

  function liveKeys(): string[] {
    return Array.from(records.keys());
  }

  function hasView(key: string): boolean {
    return records.has(key);
  }

  function bindSession(key: string, sessionId: string): void {
    const record = records.get(key);
    if (!record) {
      return;
    }

    // Drop any previous sessionId this key owned so the reverse map stays 1:1.
    if (record.sessionId != null && record.sessionId !== sessionId) {
      if (sessionToKey.get(record.sessionId) === key) {
        sessionToKey.delete(record.sessionId);
      }
    }
    // If another key claimed this sessionId, release it from that key.
    const priorKey = sessionToKey.get(sessionId);
    if (priorKey != null && priorKey !== key) {
      const priorRecord = records.get(priorKey);
      if (priorRecord && priorRecord.sessionId === sessionId) {
        priorRecord.sessionId = null;
      }
    }

    record.sessionId = sessionId;
    sessionToKey.set(sessionId, key);
  }

  function keyForSession(sessionId: string): string | null {
    return sessionToKey.get(sessionId) ?? null;
  }

  function feed(key: string, data: string): void {
    const record = records.get(key);
    if (!record) {
      return;
    }
    // Write REGARDLESS of visibility — this is what keeps inactive
    // conversations live in the background.
    record.view.write(data);
  }

  function feedSession(sessionId: string, data: string): void {
    const key = sessionToKey.get(sessionId);
    if (key == null) {
      return;
    }
    feed(key, data);
  }

  function markTerminated(key: string): void {
    const record = records.get(key);
    if (!record) {
      return;
    }
    // Keep the view + scrollback on screen; do NOT auto-remove. The user can
    // still read the final output of a finished conversation.
    record.terminated = true;
  }

  function closeView(key: string): void {
    const record = records.get(key);
    if (!record) {
      return;
    }

    clearSessionMappingFor(key);
    records.delete(key);
    // Dispose ONLY this view — never touch the others' PTYs/views.
    record.view.dispose();

    if (active === key) {
      // Pick the next surviving view (most-recently-inserted) as active, or
      // clear if none remain.
      const remaining = Array.from(records.keys());
      const next = remaining.length > 0 ? remaining[remaining.length - 1] : null;
      active = null;
      if (next != null) {
        showView(next);
      }
    }
  }

  function disposeAll(): void {
    for (const record of records.values()) {
      record.view.dispose();
    }
    records.clear();
    sessionToKey.clear();
    active = null;
  }

  return {
    ensureView,
    showView,
    activeKey,
    liveKeys,
    hasView,
    bindSession,
    keyForSession,
    feed,
    feedSession,
    markTerminated,
    closeView,
    disposeAll
  };
}
