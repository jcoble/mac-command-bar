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
import { hasBackendCapability } from './backendCapabilities.ts';

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

/**
 * What a close actually did. `closeOwned` NEVER rejects: a backend failure is
 * reported HERE, alongside the successor, because the caller needs both — the
 * view and the PTY mapping are gone regardless, so a thrown error would strand
 * the caller with no idea which terminal the manager left on screen.
 */
export type CloseOwnedResult = {
  /** The `ownedId` the manager left VISIBLE, or `null` when no view remains. */
  successor: string | null;
  /** The backend rejection, or `null` when the PTY was closed cleanly. */
  error: unknown;
};

/** Side effects a view reports back to the service (mirrors `xtermFactory`). */
export type TerminalViewHooks = {
  onData(data: string): void;
  onResize(cols: number, rows: number): void;
};

/** The name the desktop app answers with when it can spawn a single command. */
export const TERMINAL_COMMAND_SPAWN_CAPABILITY = 'terminalCommandSpawn';

export type StartOwnedOptions = {
  /** Run the session's resume command AS the session, rather than typing it
   * into a shell. See `startOwned`. */
  runCommandDirectly?: boolean;
};

export type TerminalService = {
  /** Register the ONE backend output listener. Safe to call repeatedly. */
  attach(): Promise<void>;
  /**
   * Spawn a PTY for `owned`, mount its view on `host`, replay the resume command.
   *
   * `runCommandDirectly` changes HOW the resume command is run. Off (the
   * default, and what every agent session uses) the command is typed into a
   * fresh interactive shell, so the shell is still there when the agent quits.
   * On, the session IS the command: the backend spawns `shell -lc <command>`,
   * the session ends when the command does, and the exit code belongs to the
   * command instead of the shell around it. That is what a stack run needs —
   * "the dev server crashed" and "you closed the shell" are the same event
   * otherwise. A desktop build too old to run a command this way falls back to
   * typing it, which is what the shell did before.
   */
  startOwned(
    owned: OwnedSession,
    host: HTMLElement,
    options?: StartOwnedOptions
  ): Promise<string | null>;
  /**
   * Re-attach to a PTY that survived a reload, hydrating saved scrollback.
   * Works for a TOMBSTONE too (`owned.state === 'exited'` with a
   * `ptySessionId`): the backend keeps an exited session's scrollback readable,
   * so the view is built and then marked terminated.
   *
   * `size` is the PTY's REAL geometry, straight off the backend's
   * `TerminalSessionInfo`. Pass it: on a reload only ONE view is visible, and
   * every other host is `display: none`, so `fit()` cannot measure and the view
   * would sit at xterm's 80x24 default while the scrollback it is about to
   * replay was wrapped at the PTY's true width. The size is applied BEFORE the
   * hydrating write, and it also seeds the resize gatekeeper so the first
   * `show()` of an unchanged-size view costs no backend call at all.
   */
  adoptExisting(
    owned: OwnedSession,
    host: HTMLElement,
    size?: { cols: number; rows: number } | null
  ): Promise<boolean>;
  /** Make one owned session's terminal the visible one. */
  show(ownedId: string): void;
  /**
   * Re-measure the VISIBLE terminal after its pane changed size. Safe to call
   * at any time — before `attach`, with no session on screen, after `dispose`.
   */
  refit(): void;
  /**
   * Close a PTY and drop its view. The ONLY path that kills a session.
   *
   * `ptySessionIdHint` is the caller's stored `ptySessionId`, used ONLY when
   * this service has no mapping for `ownedId` (a session whose view was never
   * built — dismissing it must still reap the backend tombstone).
   *
   * Resolves — NEVER rejects — with `{ successor, error }`: the `ownedId` the
   * manager left VISIBLE (or `null` when no view remains) so the caller's
   * active-session state agrees with the manager instead of picking a different
   * successor and showing it twice, plus any backend failure for the caller to
   * report. Both are needed on the failure path, hence the result object.
   */
  closeOwned(ownedId: string, ptySessionIdHint?: string | null): Promise<CloseOwnedResult>;
  /** Unlisten + drop all views. NEVER closes a PTY. */
  dispose(): void;
};

/**
 * Backend commands that carry INTERACTIVE TERMINAL INPUT rather than shell
 * overhead: bytes the terminal owes the program on the other end of the PTY.
 *
 * `write_terminal_session` is the only one. It is issued for a keystroke, a
 * paste, the resume command — and, because a full-screen TUI (`claude`) enables
 * DECSET 1004 focus reporting, for the `\x1b[I` / `\x1b[O` xterm sends on every
 * focus change. One per session switch is therefore CORRECT and unsuppressable
 * (suppressing it would lie to the agent about focus); it just must not be
 * counted as backend chatter. Lives here, next to the names it classifies, so
 * the HUD cannot drift out of sync with the command the backend actually calls.
 */
const TERMINAL_INPUT_COMMANDS: ReadonlySet<string> = new Set(['write_terminal_session']);

/** True when `command` is interactive PTY input (see `TERMINAL_INPUT_COMMANDS`). */
export function isTerminalInputCommand(command: string): boolean {
  return TERMINAL_INPUT_COMMANDS.has(command);
}

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

/**
 * How much of a re-attached session's scrollback is actually replayed into the
 * view, in UTF-16 code units.
 *
 * The backend ring now holds up to 16 MB, but the view keeps only 20000 lines
 * (see `xtermFactory`). Everything older than that tail is parsed by xterm —
 * escape sequences and all — purely to be dropped off the top of its own
 * buffer. 4 MB is a deliberate over-estimate of what 20000 lines can hold
 * (~200 bytes/line), so the cap costs nothing visible and bounds the worst-case
 * re-attach at a quarter of the ring.
 */
const REPLAY_TAIL_MAX_CHARS = 4 * 1024 * 1024;

/**
 * The tail of `scrollback` that can plausibly fill a view, at most
 * `REPLAY_TAIL_MAX_CHARS`. Slicing by code unit can land between the halves of
 * a surrogate pair, so a leading LOW surrogate — the orphaned second half — is
 * dropped rather than written as a lone unpaired unit.
 */
function replayTail(scrollback: string): string {
  if (scrollback.length <= REPLAY_TAIL_MAX_CHARS) {
    return scrollback;
  }
  const tail = scrollback.slice(-REPLAY_TAIL_MAX_CHARS);
  const first = tail.charCodeAt(0);
  return first >= 0xdc00 && first <= 0xdfff ? tail.slice(1) : tail;
}

export function createTerminalService(opts: {
  backend: TerminalBackend;
  createView: (host: HTMLElement, hooks: TerminalViewHooks) => TerminalView;
  onExit?(ownedId: string, payload: TerminalOutputPayload): void;
  /**
   * How long after a live re-attach the repaint nudge fires, in ms. Injectable
   * ONLY so the test does not have to sleep; production takes the default. It
   * must be long enough for the replayed scrollback to have been written and
   * the TUI to be reading the PTY again, and short enough that the user does
   * not stare at a broken frame.
   */
  repaintNudgeMs?: number;
}): TerminalService {
  const { backend, createView, onExit } = opts;
  const repaintNudgeMs = opts.repaintNudgeMs ?? 220;

  /**
   * ownedId -> ptySessionId. The service's own routing table: `ownedId` is the
   * stable key everywhere else in the shell, the PTY id is mutable (a restart
   * mints a new one), so it lives here and nowhere else.
   */
  const ptyByOwned = new Map<string, string>();
  /**
   * ptySessionId -> ownedId. The exact inverse of `ptyByOwned`, kept because
   * the exit path needs it: `setPty` runs BEFORE any view exists, so a PTY
   * whose view failed to build (or was never built) is still resolvable here
   * when its `terminated` payload lands. Resolving that only through the
   * manager's `keyForSession` silently dropped those tombstones — the rail row
   * stayed "live" over a dead process, forever.
   */
  const ownedByPty = new Map<string, string>();
  /**
   * ptySessionId -> the LAST (cols,rows) this service sent to the backend, as
   * `"<cols>x<rows>"`. The single resize gatekeeper: `showView` fits the view
   * on every switch and the fit reports geometry unconditionally, so switching
   * between two same-sized terminals used to cost one redundant
   * `resize_terminal_session` per switch. Seeded from the PTY's real size at
   * start/adopt, so even the FIRST show of an unchanged view is free.
   */
  const lastSizeByPty = new Map<string, string>();
  /**
   * sessionId -> scrollback, populated for the duration of ONE `adoptExisting`
   * call. The manager's `readScrollback` dep is synchronous, so the async read
   * has to land here before `ensureView` runs; the entry is dropped right after.
   */
  const scrollbackCache = new Map<string, string>();
  /**
   * ptySessionId -> the pending post-re-attach repaint nudge (see
   * `scheduleRepaintNudge`). Held so a close/exit/dispose that beats the timer
   * can cancel it instead of resizing a PTY nobody owns any more.
   */
  const nudgeTimers = new Map<string, ReturnType<typeof setTimeout>>();
  /**
   * The ownedId whose view is currently being constructed. `createTerminal` is
   * `(host) => view` with no key, so this hands the key to the closure that
   * `ensureView` is about to invoke.
   */
  let creatingFor: string | null = null;
  /**
   * The exact grid the view being constructed must adopt, applied the instant
   * it exists and therefore BEFORE `ensureView` hydrates it from scrollback.
   * That ordering is the whole point: sizing after the write would re-wrap
   * text that was already laid out at the wrong width.
   */
  let creatingSize: { cols: number; rows: number } | null = null;
  let unlisten: (() => void) | null = null;
  let attaching: Promise<void> | null = null;
  /**
   * `dispose()` has run. Load-bearing for the in-flight-attach case: `dispose`
   * can only call the unlisten it can SEE, and while `backend.listen` is still
   * pending there is none — so without this flag the subscription lands AFTER
   * teardown, is never removed, and pins the disposed manager (and every view
   * it holds) for the lifetime of the page. A disposed service is terminal: it
   * is never re-attached, the next mount builds a new one.
   */
  let disposed = false;

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
    const previousPty = ptyByOwned.get(ownedId);
    if (previousPty != null && previousPty !== ptyId) {
      forgetPty(previousPty);
    }
    ptyByOwned.set(ownedId, ptyId);
    ownedByPty.set(ptyId, ownedId);
  }

  /** Forget everything keyed by `ptyId`. Called when a session stops being ours. */
  function forgetPty(ptyId: string): void {
    cancelRepaintNudge(ptyId);
    ownedByPty.delete(ptyId);
    lastSizeByPty.delete(ptyId);
    scrollbackCache.delete(ptyId);
  }

  /** Drop any pending repaint nudge for `ptyId`. Safe to call for an unknown id. */
  function cancelRepaintNudge(ptyId: string): void {
    const timer = nudgeTimers.get(ptyId);
    if (timer !== undefined) {
      clearTimeout(timer);
      nudgeTimers.delete(ptyId);
    }
  }

  /**
   * Force the TUI attached to `ptyId` to repaint its whole frame, shortly after
   * a re-attach has replayed its scrollback.
   *
   * Why this exists: the backend caps scrollback at 16 MB and trims from the
   * FRONT on a CHARACTER boundary, not an ANSI-sequence boundary (and
   * `replayTail` above then takes the last 4 MB of that, on a code-unit
   * boundary). A session with enough output to hit either cut therefore replays
   * starting mid-escape, so the terminal
   * re-renders garbage — and a replay can never rebuild a live full-screen frame
   * anyway (the bytes that drew claude's input box scrolled out of the buffer
   * long ago; only the program can draw it again).
   *
   * So ask the program: resize to (cols, rows - 1) and straight back. The PTY
   * delivers SIGWINCH twice, and a full-screen TUI redraws its entire frame —
   * input box included — over the corrupted replay. Two backend calls, once, at
   * re-attach time.
   *
   * Deliberately NOT routed through `resizePty`: the two calls are redundant by
   * design and the gate exists to suppress exactly that. The gate's memo is left
   * holding `cols x rows` — which is both where the PTY starts and where it ends
   * — so a later `fit()` at that size still sends nothing.
   */
  function scheduleRepaintNudge(ptyId: string, cols: number, rows: number): void {
    cancelRepaintNudge(ptyId);
    const settled = `${cols}x${rows}`;
    const timer = setTimeout(() => {
      nudgeTimers.delete(ptyId);
      // Bail if the PTY stopped being ours (closed, exited, re-mapped) or if its
      // geometry has moved on since: a REAL resize already delivered a SIGWINCH
      // of its own, and re-asserting the stale size would fight the live one.
      if (ownedByPty.get(ptyId) == null || lastSizeByPty.get(ptyId) !== settled) {
        return;
      }
      const nudged = rows > 1 ? rows - 1 : rows + 1;
      void (async () => {
        try {
          await backend.resize(ptyId, cols, nudged);
          // Re-check between the two: a close in the gap must not be followed by
          // a resize, and leaving the PTY one row short would be worse than not
          // nudging at all.
          if (ownedByPty.get(ptyId) == null) {
            return;
          }
          await backend.resize(ptyId, cols, rows);
        } catch {
          // Best effort: a failed nudge costs a stale frame, never a broken
          // session, and there is nobody to report it to.
        }
      })();
    }, repaintNudgeMs);
    nudgeTimers.set(ptyId, timer);
  }

  /**
   * The ONE place a backend resize is issued. Skips the call when the PTY is
   * already at `(cols, rows)` — the manager fits on every `showView`, so
   * without this a switch between two same-sized terminals costs a pointless
   * IPC round trip each time.
   */
  function resizePty(ptyId: string, cols: number, rows: number): void {
    const next = `${cols}x${rows}`;
    if (lastSizeByPty.get(ptyId) === next) {
      return;
    }
    lastSizeByPty.set(ptyId, next);
    void backend.resize(ptyId, cols, rows);
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
      const view = createView(host, {
        onData(data: string): void {
          const ptyId = ptyByOwned.get(ownedId);
          if (ptyId) {
            void backend.write(ptyId, data);
          }
        },
        onResize(cols: number, rows: number): void {
          const ptyId = ptyByOwned.get(ownedId);
          if (ptyId) {
            resizePty(ptyId, cols, rows);
          }
        }
      });
      // Size it here — inside `createTerminal`, i.e. before `ensureView` gets
      // the chance to write the saved scrollback into it.
      if (creatingSize && view.resize) {
        view.resize(creatingSize.cols, creatingSize.rows);
      }
      return view;
    },
    writeSession: (sessionId: string, data: string): void => {
      void backend.write(sessionId, data);
    },
    resizeSession: (): void => {
      // Intentionally empty, and it MUST stay that way. `showView` calls
      // `view.fit()` immediately before this, and fit reports the real geometry
      // through the `onResize` hook above, which funnels into `resizePty` — the
      // single gatekeeper. This dep only receives a sessionId (no cols/rows),
      // so anything it sent would be a guess AND a second call for one switch.
      // The gatekeeper would swallow a duplicate anyway; keeping this empty
      // means there is exactly one code path that can talk to the backend.
    },
    readScrollback: (sessionId: string): string | null =>
      scrollbackCache.get(sessionId) ?? null
  });

  /** Run `ensureView` with `key` visible to the `createTerminal` closure. */
  function ensureViewFor(
    ownedId: string,
    options: {
      host: HTMLElement;
      sessionId?: string | null;
      size?: { cols: number; rows: number } | null;
    }
  ): TerminalView {
    creatingFor = ownedId;
    creatingSize = options.size ?? null;
    let view: TerminalView;
    try {
      view = manager.ensureView(ownedId, { host: options.host, sessionId: options.sessionId });
    } finally {
      creatingFor = null;
      creatingSize = null;
    }
    // The manager auto-shows only the FIRST view, and `showView` only hides
    // views that ALREADY exist — so a view created while another session is
    // active is never told to hide. Say it explicitly: anything that is not the
    // active key must be invisible the moment it exists.
    if (manager.activeKey() !== ownedId) {
      view.setVisible(false);
    }
    return view;
  }

  async function attach(): Promise<void> {
    if (unlisten || disposed) {
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
        // Resolve through THIS service's map first: it is written in
        // `startOwned`/`adoptExisting` before a view can possibly exist, so a
        // PTY that died before (or without) its view was built still reports
        // its exit. The manager is the fallback for the same reason it is the
        // fallback everywhere — it only knows sessions that got a view.
        const ownedId =
          ownedByPty.get(payload.sessionId) ?? manager.keyForSession(payload.sessionId);
        if (ownedId == null) {
          return;
        }
        // The PTY is gone: its geometry memo must not survive to suppress a
        // resize if this ownedId is later restarted onto a new session id, and a
        // pending repaint nudge has nothing left to talk to.
        lastSizeByPty.delete(payload.sessionId);
        cancelRepaintNudge(payload.sessionId);
        manager.markTerminated(ownedId);
        // The view and its scrollback stay on screen; flipping session STATE is
        // the store's job (this service only reports the exit).
        onExit?.(ownedId, payload);
      });
      if (disposed) {
        // Teardown happened while this listen was in flight, so `dispose` had
        // no stop function to call. Retire it HERE instead of storing a
        // subscription nothing will ever remove.
        stop?.();
        return;
      }
      unlisten = stop ?? null;
    })();
    try {
      await attaching;
    } finally {
      attaching = null;
    }
  }

  async function startOwned(
    owned: OwnedSession,
    host: HTMLElement,
    options: StartOwnedOptions = {}
  ): Promise<string | null> {
    const hadView = manager.hasView(owned.ownedId);
    // Only a caller that asked for it, and only a desktop build that can do it.
    // An old build drops the field and opens a plain shell without saying so, so
    // asking outright is the only way to know which of the two happened — and
    // the answer decides whether the command still has to be typed in below.
    const runsAsCommand =
      Boolean(options.runCommandDirectly) &&
      Boolean(owned.resumeCommand) &&
      (await hasBackendCapability(TERMINAL_COMMAND_SPAWN_CAPABILITY));
    const info = await backend.start({
      cwd: owned.cwd,
      ownedId: owned.ownedId,
      command: runsAsCommand ? owned.resumeCommand : null
    });
    if (!info) {
      // Never leave a half-built view behind for a session that has no PTY.
      if (!hadView && manager.hasView(owned.ownedId)) {
        manager.closeView(owned.ownedId);
      }
      return null;
    }

    // Map BEFORE the view exists so the very first keystroke can already route
    // — and so an exit that beats the view still finds its owned session.
    setPty(owned.ownedId, info.sessionId);
    // Seed the gatekeeper with the size the backend actually opened the PTY at,
    // so a fit that agrees with it sends nothing.
    lastSizeByPty.set(info.sessionId, `${info.cols}x${info.rows}`);
    ensureViewFor(owned.ownedId, { host, size: { cols: info.cols, rows: info.rows } });
    // Bind immediately: output for this sessionId starts arriving on the shared
    // listener the moment the process spawns, and an unbound session is dropped.
    manager.bindSession(owned.ownedId, info.sessionId);

    // The session that IS the command is already running it; typing it again
    // would run it twice.
    if (owned.resumeCommand && !runsAsCommand) {
      await backend.write(info.sessionId, `${owned.resumeCommand}\r`);
    }

    return info.sessionId;
  }

  async function adoptExisting(
    owned: OwnedSession,
    host: HTMLElement,
    size?: { cols: number; rows: number } | null
  ): Promise<boolean> {
    const ptyId = owned.ptySessionId;
    if (!ptyId) {
      // Nothing survived: the caller has to `startOwned` instead.
      return false;
    }

    const usableSize =
      size != null && size.cols > 0 && size.rows > 0
        ? { cols: Math.trunc(size.cols), rows: Math.trunc(size.rows) }
        : null;

    const scrollback = await backend.readScrollback(ptyId);
    if (scrollback) {
      // Only the tail: the ring is 16 MB, the view keeps 20000 lines.
      scrollbackCache.set(ptyId, replayTail(scrollback));
    }
    try {
      setPty(owned.ownedId, ptyId);
      if (usableSize) {
        // The PTY is ALREADY this size — record it before anything can fit, so
        // the first show of an unchanged view issues no backend resize.
        lastSizeByPty.set(ptyId, `${usableSize.cols}x${usableSize.rows}`);
      }
      // `sessionId` here makes the manager hydrate the new view from the cache
      // above (its readScrollback dep is synchronous, hence the staging map);
      // `size` is applied to the view first, so the replay wraps at the same
      // width the PTY wrote it at even though this host may be hidden.
      ensureViewFor(owned.ownedId, { host, sessionId: ptyId, size: usableSize });
      manager.bindSession(owned.ownedId, ptyId);
      if (owned.state === 'exited') {
        // A tombstone: the PTY is gone but the backend still holds its final
        // scrollback (Task 1). The view exists so that output stays readable —
        // it must never look live, and `setPty` above is what lets a later
        // `closeOwned` reap the backend record.
        manager.markTerminated(owned.ownedId);
      }
    } finally {
      scrollbackCache.delete(ptyId);
    }
    // Only a LIVE PTY gets the nudge: a tombstone has no process to signal, and
    // without the PTY's real geometry there is nothing safe to nudge back TO
    // (guessing would leave the gate holding a size the PTY never had).
    if (owned.state !== 'exited' && usableSize) {
      scheduleRepaintNudge(ptyId, usableSize.cols, usableSize.rows);
    }
    return true;
  }

  function show(ownedId: string): void {
    manager.showView(ownedId);
  }

  /**
   * Re-measure the VISIBLE terminal against its host and, when the grid really
   * changed, send exactly one resize to its PTY. The layout calls this whenever
   * a pane is resized (a dragged divider, a window resize), which can fire many
   * times a second — that is safe because `fit()` reports the new geometry
   * through the same `onResize` hook every other fit uses, and `resizePty`
   * drops anything that matches the size the PTY is already at.
   *
   * Hidden terminals are deliberately left alone: their host has no size to
   * measure, and `show()` fits them on the way back in. Doing nothing is always
   * a valid outcome here — no visible terminal means no work, and no view is
   * ever created.
   */
  function refit(): void {
    if (disposed) {
      return;
    }
    const activeOwnedId = manager.activeKey();
    if (activeOwnedId == null) {
      return;
    }
    manager.viewFor(activeOwnedId)?.fit();
  }

  async function closeOwned(
    ownedId: string,
    ptySessionIdHint?: string | null
  ): Promise<CloseOwnedResult> {
    // No mapping means no view was ever built for this session (e.g. an exited
    // one that was dismissed before its host mounted). Fall back to the
    // caller's stored id so the backend record is still reaped — but only if no
    // OTHER owned session currently holds it, or we would kill their PTY.
    const mapped = ptyByOwned.get(ownedId) ?? null;
    const hintUsable =
      mapped === null &&
      ptySessionIdHint != null &&
      ptySessionIdHint !== '' &&
      !Array.from(ptyByOwned.values()).includes(ptySessionIdHint);
    const ptyId = mapped ?? (hintUsable ? ptySessionIdHint : null);
    // Drop the mapping first so a concurrent close can't double-kill the PTY.
    ptyByOwned.delete(ownedId);
    if (mapped) {
      forgetPty(mapped);
    }
    manager.closeView(ownedId);
    // Read the successor BEFORE the awaited close: the manager already picked
    // and showed it inside `closeView`, and the caller must adopt that choice
    // rather than show a different one.
    const successor = manager.activeKey();
    let error: unknown = null;
    if (ptyId) {
      scrollbackCache.delete(ptyId);
      try {
        await backend.close(ptyId);
      } catch (caught) {
        // NEVER rethrow: the view and the mapping are already gone, and the
        // successor is the only thing that keeps a terminal on screen — losing
        // it to a throw would hide the survivor behind the empty-state overlay.
        error = caught;
      }
    }
    return { successor, error };
  }

  function dispose(): void {
    // NO backend.close — ever. A reload/unmount must leave every agent running;
    // `adoptExisting` picks them back up on the next mount.
    disposed = true;
    unlisten?.();
    unlisten = null;
    // A nudge scheduled by a view this teardown is about to destroy has no
    // audience — and firing it after an unmount would be IO from a dead shell.
    for (const timer of nudgeTimers.values()) {
      clearTimeout(timer);
    }
    nudgeTimers.clear();
    manager.disposeAll();
    ptyByOwned.clear();
    ownedByPty.clear();
    // The PTYs live on but the VIEWS do not: the next mount rebuilds them from
    // scratch, and a remembered size would suppress the resize that new view
    // legitimately needs.
    lastSizeByPty.clear();
    scrollbackCache.clear();
  }

  return { attach, startOwned, adoptExisting, show, refit, closeOwned, dispose };
}
