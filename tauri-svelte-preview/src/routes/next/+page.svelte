<script lang="ts">
  /**
   * /next — the shell orchestrator. Thin by construction: it owns the terminal
   * sessions and nothing else. Every other panel brings its own state and its
   * own loader; this page only says which project they are pointed at and when
   * they may start — see `shellPanels.ts` and `panelActivation.ts`.
   *
   * IO lives **only in explicit functions**, never in an `$effect`. Launch IO is
   * still just the rail: list surviving PTYs, reconcile them against the stored
   * owned sessions, re-attach them (live AND tombstones), scan for resumable
   * agents. `hydrateOwned` runs AFTER `reconcileOwnedSessions`.
   */
  import { onMount, tick } from 'svelte';

  import '$lib/shell/styles/nextTokens.css';
  /* Tailwind + the shadcn component variables. Imported HERE and nowhere else:
     the old shell shares `app.css` with this page and must keep rendering
     exactly as it does today, so this file must never reach that route. */
  import '$lib/shell/styles/next.css';
  /* Four colours the dock needs that the shared token file has no name for.
     A stylesheet rather than something the theme service sets, because the dock
     is painted on the first frame, before any theme has been applied. */
  import '$lib/shell/styles/themeChrome.css';

  import ActivityBar from '$lib/shell/components/ActivityBar.svelte';
  import BrowserPanel from '$lib/shell/components/BrowserPanel.svelte';
  import DockPanel from '$lib/shell/components/DockPanel.svelte';
  import EditorPanel from '$lib/shell/components/EditorPanel.svelte';
  import GitDiffView from '$lib/shell/components/GitDiffView.svelte';
  import SessionsColumn from '$lib/shell/components/SessionsColumn.svelte';
  import ShellFrame from '$lib/shell/components/ShellFrame.svelte';
  import ShellOverlays from '$lib/shell/components/ShellOverlays.svelte';
  import ShellSidebar from '$lib/shell/components/ShellSidebar.svelte';
  import TerminalSurface from '$lib/shell/components/TerminalSurface.svelte';
  import { countInvoke } from '$lib/shell/devInvokeCounter.svelte';
  import { editorState, resetEditorState } from '$lib/shell/editor/editorStore.svelte';
  import { explorer, selectPath, setScrollTop } from '$lib/shell/explorer/explorerStore.svelte';
  import {
    SESSIONS_MAX_WIDTH,
    SESSIONS_MIN_WIDTH,
    SESSIONS_STRIP_WIDTH,
    SESSIONS_WIDTH,
    type RegionWidthLimits,
    type ShellRegionId
  } from '$lib/shell/layout/frame';
  import { DEFAULT_SIDEBAR_VIEW, type SidebarViewId } from '$lib/shell/layout/sidebarViews';
  import type { NewSessionRequest } from '$lib/shell/newSession/newSessionFlow';
  import { requestOpenFile } from '$lib/shell/openFileBus';
  import {
    adoptAgentSession,
    createFreshSession,
    reconcileOwnedSessions
  } from '$lib/shell/ownedSessions';
  import {
    captureWorkspace,
    pruneWorkspaces,
    readWorkspaces,
    writeWorkspaces,
    type SessionWorkspaceSnapshot
  } from '$lib/shell/sessionWorkspaces';
  import { readSessionsCollapsed, writeSessionsCollapsed } from '$lib/shell/sessionStrip';
  import { registerShellCommands } from '$lib/shell/shellCommands';
  import { shellPanels } from '$lib/shell/shellPanels';
  import {
    noteSessionRemoved,
    noteTerminalExit,
    registerStackHandlers,
    type StackStartRequest
  } from '$lib/shell/stacks/stackService';
  import { recordStackStart, stackIdForOwnedId } from '$lib/shell/stacks/stackStore.svelte';
  import {
    addOwnedSession,
    completeOwnedSession,
    hydrateOwned,
    loadStoredOwned,
    rail,
    removeOwnedSession,
    reopenOwnedSession,
    setActiveOwned,
    setAvailable,
    updateOwnedSession
  } from '$lib/shell/stores/sessionRailStore.svelte';
  import { createTerminalService, tauriTerminalBackend } from '$lib/shell/terminalService';
  import { applyStoredTheme, clearTheme } from '$lib/shell/themes/themeService';
  import { loadXtermModules, makeTerminalView } from '$lib/shell/xtermFactory';
  import {
    listAgentSessionsFromLocalBridge,
    listAgentSessionsFromTauri,
    type AgentSession
  } from '$lib/tauriSource';

  /** Hosts mount before the service finishes async init: parked here, drained later. */
  const pendingHosts = new Map<string, HTMLElement>();
  /** Owned ids whose surviving PTY still needs `adoptExisting` once its host mounts. */
  const awaitingReattach = new Set<string>();
  /** Sessions with a restart already under way. Added before the first await, so
   * a second click on "Start again" cannot get past it while the first click is
   * still waiting on the backend — two starts for one row would leave two agents
   * resuming the same conversation, with only one of them reachable. */
  const restarting = new Set<string>();
  /** ptySessionId -> the PTY's REAL grid (launch `backend.list()`), fed to
   * `adoptExisting`: a HIDDEN host cannot be measured, so without it a survivor's
   * view keeps 80x24 and wraps its replay wrong. */
  const livePtySizes = new Map<string, { cols: number; rows: number }>();

  /** What each session had open, by owned id. Read once at start-up, then kept
   * in step by `snapshotWorkspace` — the editor and the file tree are one of
   * each for the whole shell, so this is what keeps two sessions in the same
   * repository from overwriting each other's tabs. */
  let workspaces: Record<string, SessionWorkspaceSnapshot> = {};
  /** True only while `restoreWorkspace` is replaying a session's files. The
   * editor asks to come to the front for every file opened, which is right for a
   * click and wrong here: switching session must not pull the user off the
   * terminal they were watching. */
  let restoringWorkspace = false;

  let service: ReturnType<typeof createTerminalService> | null = null;
  let disposed = false;
  let frameControls: {
    resetLayout(): void;
    showCenterPanel(id: string): void;
    setRegionWidth(id: ShellRegionId, width: number, limits?: RegionWidthLimits): void;
    setRegionLimits(id: ShellRegionId, limits: RegionWidthLimits): void;
    regionWidth(id: ShellRegionId): number | null;
  } | null = null;
  /** The tool column's own controls; it builds after the frame does. */
  let sidebarControls: {
    resetLayout(): void;
    expandSourceControl(): void;
    selectView(id: SidebarViewId): void;
  } | null = null;
  /** Which tool view is open. The column decides it and says so; the page holds
   * the answer only because the icon strip that draws it is a separate region
   * of the frame, on the far right edge. */
  let activeView = $state<SidebarViewId>(DEFAULT_SIDEBAR_VIEW);
  /** The overlay layer, for opening the dialogs it owns. */
  let overlays: { openSettings(): void; openNewSession(): void } | null = null;
  let refitScheduled = false;
  /** Its own state, NOT `rail.error`: ShellFrame mounts before this page's
   * start-up, and `scanRail` clears `rail.error` — which would erase a mount
   * failure on every launch and leave a blank shell with no message. */
  let layoutError = $state<string | null>(null);

  /** Palette actions for the panels. Pure bookkeeping — nothing runs until the
   * user picks one — so it belongs here at component init, not in an effect. */
  registerShellCommands({
    showPanel: (id) => frameControls?.showCenterPanel(id),
    expandSourceControl: () => sidebarControls?.expandSourceControl(),
    // Opening a view is what lets that view read anything, so nothing else has
    // to be called here — the column reports the change and the load follows.
    showView: (id) => sidebarControls?.selectView(id),
    openNewSession: () => overlays?.openNewSession()
  });

  /**
   * Is the sessions column folded up to a strip? The PAGE owns this rather
   * than the column, because folding is a WIDTH: the column says it wants to
   * fold, and the frame is what actually makes the region 52px wide.
   *
   * Read once here, at component init — an explicit read, not an effect.
   */
  let sessionsCollapsed = $state(
    typeof window === 'undefined' ? false : readSessionsCollapsed(window.localStorage)
  );

  /** Tell the frame how wide the sessions column is now. The limits go with
   * the width: folded, the column is fixed at strip width so the divider
   * beside it cannot be dragged; open, it can be dragged again. */
  function applySessionsWidth(collapsed: boolean): void {
    frameControls?.setRegionWidth(
      'sessions',
      collapsed ? SESSIONS_STRIP_WIDTH : SESSIONS_WIDTH,
      collapsed
        ? { minimumWidth: SESSIONS_STRIP_WIDTH, maximumWidth: SESSIONS_STRIP_WIDTH }
        : { minimumWidth: SESSIONS_MIN_WIDTH, maximumWidth: SESSIONS_MAX_WIDTH }
    );
  }

  /** Fold the sessions column up, or open it out. Remembered under its own
   * key so the next launch comes back the way it was left. */
  function collapseSessions(collapsed: boolean): void {
    sessionsCollapsed = collapsed;
    writeSessionsCollapsed(window.localStorage, collapsed);
    applySessionsWidth(collapsed);
  }

  /** "Reset layout" means ALL of it: the grid regions (so both side columns go
   * back to their default widths), the center tabs, and the tool column, which
   * remembers its section sizes and its open view under its own keys. A folded
   * sessions column is part of that arrangement, so it opens out too. */
  function resetLayout(): void {
    frameControls?.resetLayout();
    sidebarControls?.resetLayout();
    if (sessionsCollapsed) collapseSessions(false);
  }

  /** Coalesce dockview's layout bursts into one refit per frame. */
  function scheduleRefit(): void {
    if (refitScheduled) return;
    refitScheduled = true;
    requestAnimationFrame(() => {
      refitScheduled = false;
      service?.refit();
    });
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /** EXPLICIT IO: scan for resumable agent sessions (Tauri first, bridge second).
   * ONE counted call per rescan, named for the transport that actually ran. */
  async function scanRail(): Promise<void> {
    if (rail.scanning) return;
    rail.scanning = true;
    rail.error = null;
    let nativeError: string | null = null;
    try {
      let sessions: AgentSession[] | null = null;
      try {
        sessions = await listAgentSessionsFromTauri();
      } catch (error) {
        nativeError = describeError(error);
      }
      // A `null` native result with no error = not under Tauri: nothing invoked.
      countInvoke((sessions ?? nativeError) ? 'list_agent_sessions' : 'bridge:agent-sessions');
      sessions ??= await listAgentSessionsFromLocalBridge();
      if (disposed) return;
      setAvailable(sessions ?? []);
      // Surface the native failure only if the bridge produced nothing either.
      if (sessions === null && nativeError) rail.error = `session scan failed: ${nativeError}`;
    } catch (error) {
      if (!disposed) rail.error = `session scan failed: ${nativeError ?? describeError(error)}`;
    } finally {
      rail.scanning = false;
    }
  }

  /** Park a freshly mounted host, and re-attach immediately if one is owed. */
  function registerHost(ownedId: string, host: HTMLElement): void {
    pendingHosts.set(ownedId, host);
    // Fire-and-forget, but never unhandled: this path has no awaiting caller.
    void reattachIfPending(ownedId).catch((error) => {
      if (!disposed) rail.error = `re-attach failed: ${describeError(error)}`;
    });
  }

  /** Wait for TerminalSurface to mount the host for `ownedId`.
   *
   * A host that is no longer IN the page is not an answer: a session whose
   * terminal was closed loses its host div, and this map still holds the one it
   * used to have. Building a terminal on that detached element would leave the
   * session running with nothing on screen — so a host that has been taken out
   * of the page is dropped here, and the wait continues for the one Svelte is
   * about to mount in its place. */
  async function hostFor(ownedId: string): Promise<HTMLElement | null> {
    const mounted = (): HTMLElement | null => {
      const host = pendingHosts.get(ownedId);
      if (!host) return null;
      if (host.isConnected) return host;
      pendingHosts.delete(ownedId);
      return null;
    };
    for (let attempt = 0; attempt < 12; attempt += 1) {
      const host = mounted();
      if (host) return host;
      await tick();
      if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 8));
    }
    return mounted();
  }

  /** Re-attach one reload survivor; `awaitingReattach` guards double-adopting. */
  async function reattachIfPending(ownedId: string): Promise<void> {
    if (!service || disposed || !awaitingReattach.has(ownedId)) return;
    awaitingReattach.delete(ownedId);
    const host = pendingHosts.get(ownedId);
    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
    if (!host || !session) return;
    const size = session.ptySessionId ? (livePtySizes.get(session.ptySessionId) ?? null) : null;
    const attached = await service.adoptExisting(session, host, size);
    if (!attached) {
      updateOwnedSession(ownedId, { state: 'exited', ptySessionId: null });
      return;
    }
    if (rail.activeOwnedId === null) await selectOwned(ownedId);
  }

  /** Remember the editor tabs and file tree this session is leaving behind.
   * Stored straight away: a reload can come at any moment, and the write is a
   * few hundred bytes. */
  function snapshotWorkspace(ownedId: string): void {
    workspaces = {
      ...workspaces,
      [ownedId]: captureWorkspace({
        openFiles: editorState.openFiles,
        activePath: editorState.activePath,
        expandedFolderIds: explorer.expandedFolderIds,
        selectedPath: explorer.selectedPath,
        scrollTop: explorer.scrollTop
      })
    };
    writeWorkspaces(window.localStorage, workspaces);
  }

  /**
   * Put back the editor tabs and file tree this session had.
   *
   * The files go back through the same "open this file" request the explorer
   * uses, in strip order, with the file that was showing asked for last so it is
   * the one left in front. The tree's state is assigned directly, AFTER
   * `sessionPicked` has pointed the explorer at the project: listing the same
   * folder again does nothing, and a scan of a different folder never closes
   * folders the user had open. A highlighted file that no longer exists loses
   * its highlight when that scan lands, which is the right answer.
   */
  function restoreWorkspace(ownedId: string): void {
    // Start-up re-attaching a session picks it, which is indistinguishable from
    // a click. Replaying files then would read files before launch is over; the
    // end of start-up calls this itself once the gate is open.
    if (!shellPanels.loadsAllowed()) return;
    // Every restore starts from an empty editor. A session that has never had a
    // file open gets one, and that emptiness is the whole point: it is the other
    // session's tabs not being there.
    resetEditorState();
    const snapshot = workspaces[ownedId];
    if (!snapshot) return;

    restoringWorkspace = true;
    try {
      for (const path of snapshot.openPaths) requestOpenFile({ path });
      if (snapshot.activePath) requestOpenFile({ path: snapshot.activePath });
    } finally {
      restoringWorkspace = false;
    }
    explorer.expandedFolderIds = new Set(snapshot.expandedFolderIds);
    selectPath(snapshot.selectedPath);
    setScrollTop(snapshot.scrollTop);
  }

  async function selectOwned(ownedId: string): Promise<void> {
    const previous = rail.activeOwnedId;
    const switching = previous !== ownedId;
    // Save the session being left BEFORE anything points the panels elsewhere.
    // Gated the same way as the restore below: during start-up the panels are
    // still empty, and saving that emptiness would overwrite the tabs the
    // session actually had (rows are clickable for seconds while the first
    // scan runs — including the close button, which switches sessions too).
    if (switching && previous !== null && shellPanels.loadsAllowed()) snapshotWorkspace(previous);
    setActiveOwned(ownedId);
    service?.show(ownedId);
    // Point the file tree, the context cards and any tab the user has already
    // opened at this session's project. Ignored while start-up is still
    // re-attaching sessions, so a reload still loads nothing on its own.
    shellPanels.sessionPicked();
    // Clicking the session you are already on changes nothing. Putting the
    // stored record back here would throw away every file opened since the last
    // switch, which is the opposite of what a click on your own row means.
    if (switching) restoreWorkspace(ownedId);
  }

  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
  async function adopt(record: AgentSession): Promise<void> {
    if (!service || disposed) return;
    const owned = adoptAgentSession(record);
    addOwnedSession(owned);
    const host = await hostFor(owned.ownedId);
    if (!host) {
      rail.error = `no terminal host for "${owned.title}"`;
      return;
    }
    const ptySessionId = await service.startOwned(owned, host);
    if (!ptySessionId) {
      updateOwnedSession(owned.ownedId, { state: 'exited' });
      rail.error = `failed to start a terminal for "${owned.title}"`;
      return;
    }
    // Persist the PTY id: reload re-attach reads it back out of localStorage.
    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
    await selectOwned(owned.ownedId);
  }

  /**
   * EXPLICIT IO: start a session the user described in the new-session dialog.
   *
   * Mirrors `adopt` step for step; the difference is where the description comes
   * from. `createFreshSession` makes a plain shell — agent 'other', nothing to
   * replay — and the dialog knows better on both counts, so BOTH are set before
   * `startOwned`: that is the call which types the command in.
   */
  async function startNewSession(request: NewSessionRequest): Promise<void> {
    if (!service || disposed) return;
    const owned = {
      ...createFreshSession({ cwd: request.cwd, title: request.title }),
      agent: request.agent,
      resumeCommand: request.command
    };
    addOwnedSession(owned);
    const host = await hostFor(owned.ownedId);
    if (!host) {
      rail.error = `no terminal host for "${owned.title}"`;
      return;
    }
    const ptySessionId = await service.startOwned(owned, host);
    if (!ptySessionId) {
      updateOwnedSession(owned.ownedId, { state: 'exited' });
      rail.error = `failed to start a terminal for "${owned.title}"`;
      return;
    }
    // Persist the PTY id: reload re-attach reads it back out of localStorage.
    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
    await selectOwned(owned.ownedId);
  }

  /**
   * EXPLICIT IO: run a saved stack. A stack IS a session — it appears on the
   * rail like any other and its terminal is the one you watch.
   *
   * `runCommandDirectly` is what makes the stacks pane honest: the session is
   * the command rather than a shell with the command typed into it, so when the
   * dev server dies the session ends with the server's exit code instead of
   * dropping back to a prompt that reads as "still starting". A desktop build
   * too old for that falls back to typing the command in — see `startOwned`.
   *
   * Answers with the new session's id, or `null` when no terminal could be
   * opened; the pane says so in its own words rather than guessing.
   */
  async function onStartStack(request: StackStartRequest): Promise<string | null> {
    if (!service || disposed) return null;
    const owned = {
      ...createFreshSession({ cwd: request.cwd, title: request.title }),
      resumeCommand: request.script
    };
    addOwnedSession(owned);
    const host = await hostFor(owned.ownedId);
    if (!host) return null;
    const ptySessionId = await service.startOwned(owned, host, { runCommandDirectly: true });
    if (!ptySessionId) {
      updateOwnedSession(owned.ownedId, { state: 'exited' });
      return null;
    }
    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
    await selectOwned(owned.ownedId);
    return owned.ownedId;
  }

  /**
   * EXPLICIT IO: start a finished session up again, in place.
   *
   * It stays the SAME session — same `ownedId`, so the files it had open, the
   * branch and task the scanner gave it, and the day it was marked done all
   * survive. Being picked back up is not a new piece of work, and it does not
   * un-finish a finished one either: a done session started again stays under
   * Done until the user reopens it.
   *
   * What it cannot keep is the process. A terminal that has ended cannot be
   * revived, so this spawns a NEW one in the same folder and replays the
   * session's resume command — the same thing resuming a scanned session does,
   * which is why the agent picks the conversation up where it left off. A
   * session started here rather than found on disk has no resume command and
   * gets a plain shell back.
   *
   * Only one restart per row can be in flight. The row keeps reading "finished"
   * — and so keeps offering the button — for as long as the first click is
   * waiting on the backend, so without the `restarting` guard a double-click
   * would spawn two terminals for one session. The second would replace the
   * first in the service's bookkeeping while the first process kept running,
   * leaving two agents appending to the same transcript and only one of them
   * showing up anywhere the user could reach it.
   */
  async function restartOwned(ownedId: string): Promise<void> {
    // Checked and claimed before the first await, so a second click cannot slip
    // through the window the first one opens.
    if (!service || disposed || restarting.has(ownedId)) return;
    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
    // Only a finished session can be started again; a running one already is.
    if (!session || session.state !== 'exited') return;
    const label = session.title || ownedId;
    restarting.add(ownedId);

    /**
     * Put the terminal the manager promoted back on screen. Closing a view
     * makes another session's view visible and `closeOwned` reports which one;
     * every other caller adopts that answer. On the paths below that never
     * reach `selectOwned(ownedId)` this is the only thing standing between the
     * user and another session's scrollback sitting under this session's title.
     */
    const adoptSuccessor = async (successor: string | null): Promise<void> => {
      if (successor === null) return;
      if (!rail.owned.some((entry) => entry.ownedId === successor)) return;
      await selectOwned(successor);
    };

    /** Whoever the first close promoted, kept where every exit can see it. */
    let successor: string | null = null;

    try {
      // The old terminal is over: drop its view and let the backend forget the
      // dead process. One session has one view, so without this the new
      // terminal would open underneath the last one's final output — and the
      // backend record of the finished process would be left with nothing able
      // to reach it. A refusal here is not worth stopping for or reporting: it
      // means the backend could not tidy away something that is already dead,
      // and the session is about to get a working terminal regardless.
      awaitingReattach.delete(ownedId);
      const closed = await service.closeOwned(ownedId, session.ptySessionId);
      successor = closed?.successor ?? null;

      // Say the row is running BEFORE asking for a terminal host: the terminal
      // surface only keeps a host on screen for a session it believes has a
      // terminal, so while the row still reads as finished there is nothing for
      // `hostFor` to wait for. The old PTY id goes at the same time — it names
      // a process that no longer exists.
      updateOwnedSession(ownedId, { state: 'live', ptySessionId: null });

      const host = await hostFor(ownedId);
      if (!host) {
        updateOwnedSession(ownedId, { state: 'exited' });
        rail.error = `no terminal host for "${label}"`;
        await adoptSuccessor(successor);
        return;
      }
      // `startOwned` reads the folder and the resume command off this record;
      // the PTY id it had is cleared so nothing can point at the old process.
      // A stack's session keeps its one-command spawn on restart — typed into
      // a shell instead, the exit code would belong to the shell and a crashed
      // dev server would read as "started" again.
      const restartedStackId = stackIdForOwnedId(ownedId);
      const ptySessionId = await service.startOwned(
        { ...session, ptySessionId: null },
        host,
        restartedStackId !== null ? { runCommandDirectly: true } : undefined
      );
      if (!ptySessionId) {
        updateOwnedSession(ownedId, { state: 'exited' });
        rail.error = `could not start "${label}" again: no new terminal opened`;
        await adoptSuccessor(successor);
        return;
      }

      // Ask again what the row says now. The moment it read "running" its Close
      // and Remove buttons came back, and either of them could have been used
      // while the terminal was still starting. Neither could reach this PTY —
      // it did not exist yet — so adopting it here would put back a session the
      // user has just closed, or attach a live process to a row that is gone.
      const current = rail.owned.find((entry) => entry.ownedId === ownedId);
      if (!current || current.state === 'exited') {
        const closedAgain = await service.closeOwned(ownedId, ptySessionId);
        await adoptSuccessor(closedAgain?.successor ?? null);
        return;
      }

      // Persist the new PTY id: reload re-attach reads it back out of storage.
      updateOwnedSession(ownedId, { ptySessionId, state: 'live' });
      // A restarted stack run is a run again — without this the stacks pane
      // keeps the old exit on record and says "stopped" under a live server.
      if (restartedStackId !== null) recordStackStart(restartedStackId, ownedId);
      await selectOwned(ownedId);
    } catch (error) {
      // The row goes back to finished rather than sitting there claiming to be
      // running: nothing started, and the card's buttons must still offer this.
      updateOwnedSession(ownedId, { state: 'exited' });
      if (!disposed) {
        rail.error = `could not start "${label}" again: ${describeError(error)}`;
        // Same reason as the early returns above: the close at the top of this
        // function already put someone else's terminal on screen.
        await adoptSuccessor(successor);
      }
    } finally {
      restarting.delete(ownedId);
    }
  }

  /**
   * EXPLICIT IO: the ONLY path that kills a PTY. `service.closeOwned` never
   * rejects — it reports `{ successor, error }` — so a failed close still hands
   * back the terminal the manager left visible.
   *
   * The SESSION survives this. Closing a terminal ends the process and its
   * screen; it does not end the piece of work, which stays on the list as a
   * finished row until the user marks it done and removes it. `removeSession`
   * is the only thing that takes a row off the list.
   */
  async function closeTerminal(ownedId: string): Promise<void> {
    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
    // The row is staying, so its host stays mounted and stays claimed; only the
    // re-attach that is now pointless is dropped.
    awaitingReattach.delete(ownedId);
    const result = await service?.closeOwned(ownedId, session?.ptySessionId ?? null);
    if (result?.error) {
      rail.error = `close failed for "${session?.title ?? ownedId}": ${describeError(result.error)}`;
    }
    // The PTY id is cleared with the state: it names a process that is gone, and
    // leaving it stored would have the next launch try to re-attach to it.
    updateOwnedSession(ownedId, { state: 'exited', ptySessionId: null });
    // Picked BEFORE the await: adopt it only while it still exists. It goes
    // through `selectOwned` like every other session change, and the order is
    // what makes that safe: `rail.activeOwnedId` is still the session whose
    // terminal just closed, so the tabs and tree on screen are saved as ITS
    // workspace, and only then does the successor's own state come back.
    // Pointing the rail at the successor directly saved this session's files
    // into the successor's record on the next switch.
    const successor = result?.successor ?? null;
    if (successor !== null && rail.owned.some((entry) => entry.ownedId === successor)) {
      await selectOwned(successor);
    }
  }

  /**
   * EXPLICIT IO: take a session off the list for good. The transcript on disk is
   * untouched; only CommandBar's record of it goes.
   *
   * The close runs every time, not just for a session that is still running. A
   * session whose process ended on its own keeps both its terminal on screen and
   * its record in the backend, and dropping the row is the last chance to clear
   * either — the row is what the ids were reachable through.
   */
  async function removeSession(ownedId: string): Promise<void> {
    await closeTerminal(ownedId);
    pendingHosts.delete(ownedId);
    awaitingReattach.delete(ownedId);
    removeOwnedSession(ownedId);
    // A removed row takes its stack tag with it, rather than leaving one
    // pointing at a session that is gone.
    noteSessionRemoved(ownedId);
    // The row is gone, so the tabs and tree it remembered go with it — pruning
    // against what is left also clears anything an earlier build orphaned.
    workspaces = pruneWorkspaces(
      workspaces,
      rail.owned.map((entry) => entry.ownedId)
    );
    writeWorkspaces(window.localStorage, workspaces);
  }

  onMount(() => {
    // First, and synchronous: it only touches the DOM, and every panel below
    // paints in the theme it sets.
    applyStoredTheme();
    disposed = false;
    // The stacks pane never spawns or kills anything itself — the page owns the
    // rail, the terminal service and the terminal hosts, so it does the work and
    // the pane asks for it. Pure bookkeeping; nothing runs until a click.
    registerStackHandlers({
      onStartStack,
      onStopStack: (ownedId) => closeTerminal(ownedId),
      onSelectSession: (ownedId) => selectOwned(ownedId)
    });
    void (async () => {
      try {
        const backend = tauriTerminalBackend(countInvoke);
        const modules = await loadXtermModules();
        if (disposed) return;

        service = createTerminalService({
          backend,
          createView: (host, hooks) => makeTerminalView(modules, host, hooks),
          onExit: (ownedId, payload) => {
            updateOwnedSession(ownedId, { state: 'exited' });
            // The stacks pane learns how its run ended from here and nowhere
            // else: no timer anywhere reads process states. A session that is
            // not a stack's costs one map lookup.
            noteTerminalExit(ownedId, { exitCode: payload.exitCode, signal: payload.signal });
          }
        });
        await service.attach();
        if (disposed) return;

        // Rail hydration — the ONLY launch IO (constitution).
        const live = (await backend.list()) ?? [];
        if (disposed) return;
        for (const i of live) livePtySizes.set(i.sessionId, { cols: i.cols, rows: i.rows });
        const { owned, reattachable } = reconcileOwnedSessions(loadStoredOwned(), live);
        // Tombstones re-attach too (final scrollback + a reapable PTY id); live first.
        const attachable = [
          ...reattachable,
          ...owned.filter((entry) => entry.state === 'exited' && entry.ptySessionId)
        ];
        // Claim them BEFORE the hosts mount, or `registerHost` races past.
        for (const session of attachable) awaitingReattach.add(session.ownedId);
        hydrateOwned(owned);
        // The per-session tabs and tree state, read once. Sessions that did not
        // survive the reconcile take their records with them.
        workspaces = pruneWorkspaces(
          readWorkspaces(window.localStorage),
          owned.map((entry) => entry.ownedId)
        );
        writeWorkspaces(window.localStorage, workspaces);
        // One survivor's failure must cost neither the others their re-attach nor
        // the Resume group its scan: collect, keep going, report once.
        const failed: string[] = [];
        for (const session of attachable) {
          await hostFor(session.ownedId);
          try {
            await reattachIfPending(session.ownedId);
          } catch (error) {
            failed.push(`"${session.title}" (${describeError(error)})`);
          }
        }
        // scanRail CLEARS rail.error, so the re-attach report goes after it.
        await scanRail();
        if (failed.length > 0 && !disposed) {
          const prefix = rail.error ? `${rail.error}; ` : '';
          rail.error = `${prefix}could not re-attach ${failed.join(', ')}`;
        }
      } catch (error) {
        if (!disposed) rail.error = `shell start-up failed: ${describeError(error)}`;
      } finally {
        // Launch is over — including when it failed, or the file tree and the
        // context cards would never load again. From here, a session being
        // selected is the user's doing and those panels may follow it.
        if (!disposed) shellPanels.allowSessionLoads();
        // A session re-attached during start-up was "picked" before the gate was
        // open, so its pick was ignored. Repeat it now that loads are allowed, or
        // a reload comes back with empty panes until the user clicks a session.
        if (!disposed && rail.activeOwnedId !== null) shellPanels.sessionPicked();
        // Same story for the files that session had open: the pick that would
        // have restored them happened before the gate opened, so a reload would
        // otherwise come back with an empty editor.
        if (!disposed && rail.activeOwnedId !== null) restoreWorkspace(rail.activeOwnedId);
      }
    })();

    /** Remember the session on screen when the page goes away. Leaving the
     * window is not a session switch, so nothing else would have saved it, and a
     * reload would come back to an empty editor. `pagehide` is the event
     * browsers still fire for both a reload and a close. */
    const saveOnLeaving = (): void => {
      if (rail.activeOwnedId !== null) snapshotWorkspace(rail.activeOwnedId);
    };
    window.addEventListener('pagehide', saveOnLeaving);

    return () => {
      window.removeEventListener('pagehide', saveOnLeaving);
      // Navigating away inside the app ends here instead, and it is the same
      // last chance to remember what the session on screen had open.
      if (!disposed && rail.activeOwnedId !== null) snapshotWorkspace(rail.activeOwnedId);
      disposed = true;
      // dispose() drops views + the listener ONLY. Every PTY survives.
      service?.dispose();
      service = null;
      pendingHosts.clear();
      awaitingReattach.clear();
      livePtySizes.clear();
      // The theme painted inline colors onto <html>, above the scoping that
      // keeps the old shell on its own palette. Leaving this page takes them
      // back off, so a same-document navigation to the old shell renders it
      // exactly as it was found.
      clearTheme();
    };
  });
</script>

<svelte:head>
  <title>CommandBar · next</title>
</svelte:head>

<!-- Every region is a top-level snippet: an implicit `{#snippet rail()}` child would
     shadow the imported `rail` store and break every `rail.owned` read. -->
{#snippet sessionsArea()}
  <SessionsColumn
    owned={rail.owned} available={rail.available} activeOwnedId={rail.activeOwnedId}
    scanning={rail.scanning} collapsed={sessionsCollapsed}
    onSelect={selectOwned} onAdopt={adopt} onClose={closeTerminal} onRestart={restartOwned}
    onComplete={(ownedId) => completeOwnedSession(ownedId, new Date())}
    onReopen={reopenOwnedSession} onRemove={removeSession}
    onRescan={scanRail} onCollapse={collapseSessions}
    onNewSession={() => overlays?.openNewSession()}
  />
{/snippet}
{#snippet toolsArea()}
  <ShellSidebar
    onReady={(controls) => (sidebarControls = controls)}
    onActiveViewChange={(id) => (activeView = id)}
    onSourceControlVisible={(visible) => shellPanels.sourceControlVisible(visible)}
    onWorktreesVisible={(visible) => shellPanels.worktreesVisible(visible)}
    onStacksVisible={(visible) => shellPanels.stacksVisible(visible)}
    onContextVisible={(visible) => shellPanels.contextVisible(visible)}
    onOpenSession={(ownedId) => void selectOwned(ownedId)}
    onShowDiff={() => frameControls?.showCenterPanel('diff')}
  />
{/snippet}
{#snippet activityArea()}
  <ActivityBar
    activeId={activeView}
    onSelect={(id) => sidebarControls?.selectView(id)}
    onOpenSettings={() => overlays?.openSettings()}
  />
{/snippet}
{#snippet dockArea()}<DockPanel onReset={resetLayout} />{/snippet}
{#snippet sessionArea()}
  <TerminalSurface owned={rail.owned} activeOwnedId={rail.activeOwnedId} {registerHost} />
{/snippet}
{#snippet editorArea()}
  <!-- Opening a file is a request to READ it: bring the editor forward, not load it out of sight.
       Except while a session's files are being put back — that is not a request for anything, and
       it must not drag the user off the terminal they were watching. -->
  <EditorPanel
    onFileOpened={() => {
      if (!restoringWorkspace) frameControls?.showCenterPanel('editor');
    }}
  />
{/snippet}
{#snippet browserArea()}<BrowserPanel />{/snippet}
<!-- The changes to whichever file source control has selected. `GitDiffView`
     reads that selection itself and takes no props, so it can simply live here
     as a tab of its own — which is what gives a diff the width of the middle
     instead of a column. -->
{#snippet diffArea()}<GitDiffView />{/snippet}

<main class="next-shell">
  <ShellFrame
    sessions={sessionsArea} tools={toolsArea} activity={activityArea} dock={dockArea}
    center={{
      session: sessionArea,
      editor: editorArea,
      browser: browserArea,
      diff: diffArea
    }}
    onSessionPanelLayout={scheduleRefit}
    onCenterPanelShown={(id) => shellPanels.panelShown(id)}
    onReady={(controls) => {
      frameControls = controls;
      // Say what the sessions column is, once, here, where the frame first
      // exists — in BOTH cases, not only the folded one.
      //
      // Two separate things remember the column: the stored grid layout, which
      // carries its width AND the limits it may be dragged between, and the
      // fold flag under its own key. They are written at different moments —
      // the flag straight away, the grid a quarter of a second later — so a
      // reload in between leaves the flag saying "open" and the grid still
      // holding the folded 52px with its minimum and maximum both pinned there.
      // Saying nothing in the open case is what let that stand: the column came
      // back as an unreadable 52px sliver whose divider could not be dragged,
      // with the button that would unfold it clipped out of reach.
      if (sessionsCollapsed) {
        applySessionsWidth(true);
      } else {
        controls.setRegionLimits('sessions', {
          minimumWidth: SESSIONS_MIN_WIDTH,
          maximumWidth: SESSIONS_MAX_WIDTH
        });
        // Only rescue a column that came back narrower than it is allowed to
        // be. Any other width is one the user dragged, and it survives.
        const restored = controls.regionWidth('sessions');
        if (restored !== null && restored < SESSIONS_MIN_WIDTH) {
          controls.setRegionWidth('sessions', SESSIONS_WIDTH);
        }
      }
      // One timer tick later: the tab area announces the tab it restored
      // through a microtask, and those all arrive before any timer. Waiting
      // means a restored tab loads nothing, while a real click still does.
      setTimeout(() => shellPanels.allowPanelLoads(), 0);
    }}
    onError={(message) => (layoutError = `layout failed: ${message}`)}
  />

  <ShellOverlays
    bind:this={overlays}
    onResetLayout={resetLayout}
    onRescanSessions={scanRail}
    onStartNewSession={startNewSession}
    newSessionRoots={rail.owned.map((session) => session.cwd)}
    message={[layoutError, rail.error].filter(Boolean).join('; ') || null}
  />
</main>

<style>
  /* ShellFrame owns the geometry now, but it still needs a definite height to
     measure against — at 0x0 the grid mounts and renders nothing. */
  .next-shell {
    position: relative;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: #101014;
    color: #d8d8e0;
  }
</style>
