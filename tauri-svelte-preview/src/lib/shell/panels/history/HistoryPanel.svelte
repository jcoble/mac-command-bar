<script lang="ts">
  /**
   * HistoryPanel.svelte — the History tab of the right column.
   *
   * Every session this machine has run, grouped by the project it ran in, most
   * recent first. A project with hundreds of sessions in it opens showing the
   * newest twenty-five per checkout and reads the rest as the reader scrolls,
   * because the alternative is a panel that takes a second to draw every time
   * it is shown.
   *
   * The panel owns running the actions, since it is the only thing here holding
   * the session service and the confirm dialog. Deciding WHICH actions a
   * session supports is `sessionHistoryActions`'s job, and drawing a card is
   * `SessionHistoryCard`'s.
   */
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import History from '@lucide/svelte/icons/history';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Server from '@lucide/svelte/icons/server';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { onDestroy, untrack } from 'svelte';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import WorkingSpinner from '$lib/shell/components/conversation/WorkingSpinner.svelte';
  import {
    buildSessionHistoryViewModel,
    createSessionHistoryCollapseState,
    createSessionHistoryWindowState,
    extendSessionHistoryWindow,
    filterSessionHistoryRecords,
    isSessionHistoryGroupOpen,
    resetSessionHistoryWindowOnFilterChange,
    sessionHistoryProjectPath,
    toggleSessionHistoryGroup,
    type SessionHistoryProjectGroup,
    type SessionHistoryRow,
    type SessionHistoryWorktreeGroup
  } from '$lib/shell/history/sessionHistoryViewModel.ts';
  import {
    sessionHistoryLoadOutcome,
    type SessionHistoryLoadOutcome
  } from '$lib/shell/history/sessionHistoryLoad.ts';
  import {
    buildSessionLibrary,
    type SessionHistoryScope,
    type SessionHistoryFilters,
    type SessionLibraryRecord
  } from '$lib/shell/sessionLibrary/sessionLibraryModel.ts';
  import { sessionLibraryHost } from '$lib/shell/sessionLibrary/sessionLibraryService.ts';
  import { sessionLibraryState, setSessionLibraryQuery } from '$lib/shell/sessionLibrary/sessionLibraryStore.svelte.ts';
  import {
    ensureStructuredConversation,
    loadConversationForRead
  } from '$lib/shell/conversation/conversationService.ts';
  import { warmAgentConversationConfig } from '$lib/shell/conversation/conversationConfig.ts';
  import { setConversationAgentConfigState } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions.ts';
  import { addOwnedSession, rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import {
    beginAgentConversationImportFromTauri,
    finishAgentConversationImportFromTauri,
    listAgentConversationSessionsFromTauri,
    listRepositoryCheckoutsFromTauri,
    openPathFromTauri,
    revealPathFromTauri,
    type RepositoryCheckout
  } from '$lib/tauriSource.ts';
  import { openFileInEditor, showCenterTab } from '$lib/shell/workbenchNavigation.ts';
  import type { SessionHistoryWorkspace } from '$lib/shell/sessionWorkspaces.ts';

  const SCOPE_OPTIONS: readonly { value: SessionHistoryScope; label: string }[] = [
    { value: 'workspace', label: 'Workspace' },
    { value: 'project', label: 'Project' },
    { value: 'all', label: 'All' }
  ];

  import SessionHistoryCard from './SessionHistoryCard.svelte';
  import {
    sessionHistoryActions,
    sessionHistoryIdentity,
    sessionResumeCommand,
    sessionTranscriptImport,
    type SessionHistoryActionId
  } from './sessionHistoryActions.ts';

  interface Props {
    /** True while this panel's tab is the selected one. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
    workspaceState?: SessionHistoryWorkspace;
    onWorkspaceStateChange?(ownedId: string | null, state: SessionHistoryWorkspace): void;
  }
  let { visible, root, ownedId, workspaceState, onWorkspaceStateChange }: Props = $props();
  const workspaceOwnedId = untrack(() => ownedId);
  const initialWorkspaceState = untrack(() => workspaceState);

  if (initialWorkspaceState) sessionLibraryState.scope = initialWorkspaceState.scope;

  // Read once, at init: the page registers its actions in its own component
  // body, which runs before this panel is created.
  const host = sessionLibraryHost();

  function restoredCollapseState(): ReturnType<typeof createSessionHistoryCollapseState> {
    return {
      projects: new Set(initialWorkspaceState?.openProjectKey ? [initialWorkspaceState.openProjectKey] : []),
      worktrees: new Set(initialWorkspaceState?.openWorktreeKeys ?? [])
    };
  }

  const query = $derived(sessionLibraryState.query);
  let expandedKey = $state<string | null>(null);
  let collapseState = $state(restoredCollapseState());
  let windowState = $state(createSessionHistoryWindowState());
  let pendingDelete = $state<SessionLibraryRecord | null>(null);
  /** Redrawn only while the panel is on screen, so ages do not go stale in it. */
  let now = $state(new Date());

  const summaryLibrary = $derived(buildSessionLibrary(rail.owned, rail.available));
  const activeRecord = $derived(summaryLibrary.find((record) => record.ownedId === ownedId) ?? null);
  const projectPath = $derived(
    activeRecord ? sessionHistoryProjectPath(activeRecord) : root.trim()
  );
  const historyFilters = $derived<SessionHistoryFilters>({
    query: sessionLibraryState.query,
    provider: sessionLibraryState.provider,
    model: sessionLibraryState.model,
    dateFrom: sessionLibraryState.dateFrom || null,
    dateTo: sessionLibraryState.dateTo || null,
    scope: sessionLibraryState.scope,
    workspacePath: root.trim() || null,
    projectPath: projectPath || null
  });
  const historyFilterKey = $derived([
    historyFilters.query,
    historyFilters.provider,
    historyFilters.model,
    historyFilters.dateFrom,
    historyFilters.dateTo,
    historyFilters.scope,
    historyFilters.workspacePath,
    historyFilters.projectPath
  ].join('\0'));
  const summaryRecords = $derived(
    visible ? filterSessionHistoryRecords(summaryLibrary, historyFilters) : []
  );
  let loadedRecords = $state<SessionLibraryRecord[]>([]);
  let loadOutcome = $state<SessionHistoryLoadOutcome | null>(null);
  /** The project whose sessions are being read, so the row can say it is working.
   *  Opening one reads every session record it holds, which on a large project
   *  is seconds of nothing happening otherwise. */
  let loadingProjectKey = $state<string | null>(null);
  let loadingOlder = $state(false);
  let historyViewport = $state<HTMLElement | null>(null);
  const restoreWorktreeKeys = new Set(initialWorkspaceState?.openWorktreeKeys ?? []);
  const restoreScrollTop = initialWorkspaceState?.scrollTop ?? 0;
  let restoreScrollPending = restoreScrollTop > 0;
  let restoredProjectLoaded = false;
  let loadVersion = 0;
  let loadStopController = new AbortController();
  type SessionHistoryDetail = Pick<SessionLibraryRecord, 'firstPrompt' | 'latestTurns'>;
  let detailsByKey = $state<Record<string, SessionHistoryDetail>>({});
  let detailLoadingKey = $state<string | null>(null);

  function releaseDetails(keys?: ReadonlySet<string>): void {
    loadVersion += 1;
    if (!keys) {
      detailsByKey = {};
      detailLoadingKey = null;
      host.service.release();
      return;
    }
    const next = { ...detailsByKey };
    for (const key of keys) delete next[key];
    detailsByKey = next;
    if (detailLoadingKey && keys.has(detailLoadingKey)) detailLoadingKey = null;
    host.service.release(keys);
  }

  function stopHistoryLoads(): void {
    loadStopController.abort();
    loadStopController = new AbortController();
    loadingOlder = false;
  }

  onDestroy(() => {
    stopHistoryLoads();
    releaseDetails();
  });

  function publishWorkspaceState(): void {
    onWorkspaceStateChange?.(workspaceOwnedId, {
      scope: sessionLibraryState.scope,
      openProjectKey: [...collapseState.projects][0] ?? null,
      openWorktreeKeys: [...collapseState.worktrees],
      expandedKey,
      scrollTop: historyViewport?.scrollTop ?? restoreScrollTop
    });
  }

  /**
   * The checkouts each repository still has, asked of git once the sessions have
   * named their repositories.
   *
   * This is what lets a worktree appear that no session was ever run in — one
   * that only ever hosted dispatched lanes — and what keeps a checkout that has
   * since been deleted out. Empty until the answer arrives, and the tree simply
   * shows the checkouts the sessions name until then.
   */
  let checkouts = $state<Record<string, RepositoryCheckout[]>>({});

  /** Lightweight rail summaries build only the top-level project headers. */
  const summaryViewModel = $derived(
    buildSessionHistoryViewModel(summaryRecords, { query, windowState })
  );
  const loadedProjects = $derived(new Map(
    buildSessionHistoryViewModel(summaryRecords, { query, windowState, checkouts })
      .projects.flatMap((project) => {
        const loadedKeys = new Set(loadedRecords.map((record) => record.key));
        const worktrees = project.worktrees.map((worktree) => {
          const rows = worktree.rows.filter((row) => loadedKeys.has(row.record.key));
          return {
            ...worktree,
            rows,
            olderCount: worktree.olderCount + worktree.rows.length - rows.length
          };
        });
        const hasRows = worktrees.some((worktree) => worktree.rows.length > 0);
        if (!hasRows && !isSessionHistoryGroupOpen(collapseState, 'project', project.key)) {
          return [];
        }
        return [[project.key, { ...project, worktrees }] as const];
      })
  ));

  /** A new search starts every checkout back at its first page of cards. */
  function search(value: string): void {
    setSessionLibraryQuery(value);
    windowState = resetSessionHistoryWindowOnFilterChange(windowState, { query: value });
    releaseDetails();
    loadedRecords = [];
    loadOutcome = null;
    checkouts = {};
    collapseState = createSessionHistoryCollapseState();
  }

  function setScope(value: string): void {
    if (!SCOPE_OPTIONS.some((option) => option.value === value)) return;
    sessionLibraryState.scope = value as SessionHistoryScope;
    windowState = createSessionHistoryWindowState({
      query,
      provider: sessionLibraryState.provider
    });
    publishWorkspaceState();
  }

  async function restoreProject(project: SessionHistoryProjectGroup): Promise<void> {
    await toggleProject(project, true);
    if (!visible) return;
    for (const key of restoreWorktreeKeys) {
      if (!isSessionHistoryGroupOpen(collapseState, 'worktree', key)) {
        collapseState = toggleSessionHistoryGroup(collapseState, 'worktree', key);
      }
    }
    publishWorkspaceState();
  }

  $effect(() => {
    if (visible) return;
    stopHistoryLoads();
    releaseDetails();
    loadedRecords = [];
    loadOutcome = null;
    checkouts = {};
    collapseState = restoredCollapseState();
    expandedKey = null;
  });

  $effect(() => {
    root;
    ownedId;
    historyFilterKey;
    stopHistoryLoads();
    releaseDetails();
    loadedRecords = [];
    loadOutcome = null;
    checkouts = {};
    collapseState = restoredCollapseState();
    loadingProjectKey = null;
    expandedKey = null;
    restoredProjectLoaded = false;
  });

  // Restore only after the root-change reset above has torn down the outgoing
  // projection. Running these in the opposite order immediately closes the
  // group that the incoming session asked to reopen.
  $effect(() => {
    const projectKey = workspaceState?.openProjectKey ?? null;
    if (!visible || projectKey === null) return;
    const project = summaryViewModel.projects.find((candidate) => candidate.key === projectKey);
    if (!project || restoredProjectLoaded) return;
    restoredProjectLoaded = true;
    void restoreProject(project);
  });

  $effect(() => {
    loadedRecords.length;
    const viewport = historyViewport;
    if (!visible || !viewport || !restoreScrollPending) return;
    viewport.scrollTop = restoreScrollTop;
    restoreScrollPending = false;
  });

  $effect(() => {
    if (!visible) return;
    now = new Date();
  });

  async function toggleProject(
    project: SessionHistoryProjectGroup,
    reload = false
  ): Promise<void> {
    stopHistoryLoads();
    const stopSignal = loadStopController.signal;
    releaseDetails();
    const closing = !reload
      && isSessionHistoryGroupOpen(collapseState, 'project', project.key);
    const version = ++loadVersion;
    loadedRecords = [];
    loadOutcome = null;
    checkouts = {};
    collapseState = createSessionHistoryCollapseState();
    loadingProjectKey = null;
    if (closing) {
      publishWorkspaceState();
      return;
    }

    const keys = new Set(project.worktrees.flatMap((worktree) =>
      worktree.rows.map((row) => row.record.key)
    ));
    collapseState = toggleSessionHistoryGroup(collapseState, 'project', project.key);
    loadingProjectKey = project.key;
    const heldRecords = host.service.records.filter((record) => keys.has(record.key));
    const fullyHeld = heldRecords.length === keys.size;
    if (!fullyHeld) host.service.release(keys);

    try {
      const [refreshed, nextCheckouts] = await Promise.allSettled([
        fullyHeld
          ? Promise.resolve(heldRecords)
          : host.service.refresh(keys, { projectPath: project.path }),
        listRepositoryCheckoutsFromTauri([project.path])
      ]);
      if (stopSignal.aborted || !visible || version !== loadVersion) {
        if (!fullyHeld) host.service.release(keys);
        return;
      }
      const outcome = sessionHistoryLoadOutcome({
        requestedKeys: keys,
        refreshed,
        checkouts: nextCheckouts
      });
      loadOutcome = outcome;
      loadedRecords = [...outcome.records];
      checkouts = outcome.checkouts;
      host.service.release(keys);
      if (refreshed.status === 'rejected') {
        console.error('[history] could not load project', refreshed.reason);
      }
      if (outcome.state === 'incomplete' && !reload && host.rescan) {
        await host.rescan();
        if (!stopSignal.aborted && visible && version === loadVersion) {
          await toggleProject(project, true);
        }
      }
    } finally {
      // Cleared whatever happened, and only for the read still in front: a
      // slow project answering after the reader has opened another one must
      // not take that one's spinner away with it.
      if (version === loadVersion) {
        loadingProjectKey = null;
        publishWorkspaceState();
      }
    }
  }

  function toggleWorktree(key: string): void {
    releaseDetails();
    collapseState = toggleSessionHistoryGroup(collapseState, 'worktree', key);
    publishWorkspaceState();
  }

  async function showOlder(worktreeKey: string, stopSignal: AbortSignal): Promise<void> {
    if (stopSignal.aborted || !visible) return;
    const project = summaryViewModel.projects.find((candidate) =>
      candidate.worktrees.some((worktree) => worktree.key === worktreeKey)
    );
    const worktree = project?.worktrees.find((candidate) => candidate.key === worktreeKey);
    if (!project || !worktree) return;
    const loadedKeys = new Set(loadedRecords.map((record) => record.key));
    const nextWindowState = extendSessionHistoryWindow(windowState, worktreeKey);
    windowState = nextWindowState;
    const nextProject = buildSessionHistoryViewModel(summaryRecords, {
      query,
      windowState: nextWindowState
    }).projects.find((candidate) => candidate.key === project.key);
    const nextWorktree = nextProject?.worktrees.find((candidate) => candidate.key === worktreeKey);
    const keys = new Set((nextWorktree?.rows ?? [])
      .filter((row) => !loadedKeys.has(row.record.key))
      .map((row) => row.record.key));
    if (keys.size === 0) return;
    const version = ++loadVersion;
    try {
      const refreshed = await host.service.refresh(keys, { projectPath: project.path });
      if (stopSignal.aborted || !visible || version !== loadVersion) return;
      loadedRecords = [...new Map(
        [...loadedRecords, ...refreshed].map((record) => [record.key, record])
      ).values()];
    } catch (error) {
      if (version === loadVersion) console.error('[history] could not load older sessions', error);
    } finally {
      if (version === loadVersion) host.service.release(keys);
    }
  }

  async function loadOlderVisibleSessions(): Promise<void> {
    if (loadingOlder || loadStopController.signal.aborted || !visible) return;
    const project = summaryViewModel.projects.find((candidate) =>
      isSessionHistoryGroupOpen(collapseState, 'project', candidate.key)
    );
    const loadedProject = project ? loadedProjects.get(project.key) : null;
    if (!loadedProject) return;
    const pending = loadedProject.worktrees.filter((worktree) =>
      worktree.olderCount > 0
      && (loadedProject.singleCheckout
        || isSessionHistoryGroupOpen(collapseState, 'worktree', worktree.key))
    );
    if (pending.length === 0) return;

    loadingOlder = true;
    const stopSignal = loadStopController.signal;
    try {
      for (const worktree of pending) {
        if (stopSignal.aborted) return;
        await showOlder(worktree.key, stopSignal);
      }
    } finally {
      if (!stopSignal.aborted) loadingOlder = false;
    }
  }

  async function handleHistoryScroll(): Promise<void> {
    const viewport = historyViewport;
    if (!viewport) return;
    publishWorkspaceState();
    if (viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight > 120) return;
    await loadOlderVisibleSessions();
  }

  $effect(() => {
    const viewport = historyViewport;
    if (!visible || !viewport) return;
    const onScroll = (): void => {
      void handleHistoryScroll();
    };
    viewport.addEventListener('scroll', onScroll, { passive: true });
    return () => viewport.removeEventListener('scroll', onScroll);
  });

  async function copyText(value: string | null): Promise<void> {
    if (!value || typeof navigator === 'undefined' || !navigator.clipboard?.writeText) return;
    await navigator.clipboard.writeText(value);
  }

  /**
   * The words an error carries.
   *
   * A native command rejects with the plain `{ code, message, recoverable }`
   * object the backend serialized, not with an Error, and `String()` turns that
   * into "[object Object]" — which is what the rail used to show. Read the
   * message off either shape.
   */
  function describeError(error: unknown): string {
    if (error instanceof Error) return error.message;
    if (error && typeof error === 'object') {
      const { message } = error as { message?: unknown };
      if (typeof message === 'string' && message.trim()) return message;
    }
    return String(error);
  }

  /**
   * Pick a past session back up as a session of this app's own.
   *
   * Two steps, and both are needed for the one button to mean what it says. The
   * import writes the past conversation into this app's storage as a new
   * app-owned session and hands back its id, so the rail is told about that row
   * before anything opens it — opening a session reads the rail for the agent it
   * belongs to. That much puts the transcript on screen. What makes it a session
   * you can SEND to is the second step: the ACP adapter is started for it, the
   * same call adopting a scanned session makes, which is also what fills in the
   * model and effort the composer offers. No CLI and no terminal are involved —
   * the adapter is the whole runtime here.
   */
  async function resumeAsAssemblySession(record: SessionLibraryRecord): Promise<void> {
    const request = sessionTranscriptImport(record);
    if (!request) return;
    try {
      // Naming the session reads none of its transcript, so the rail, the title
      // and the agent are all there straight away. The two slow halves — reading
      // the records, and starting the agent — then run together below rather
      // than one behind the other.
      const ownedId = await beginAgentConversationImportFromTauri(request);
      if (!ownedId) return;
      const stored = (await listAgentConversationSessionsFromTauri()) ?? [];
      const imported = stored.find((session) => session.ownedId === ownedId);
      if (!imported) {
        rail.error = `the imported transcript for "${record.title}" was not stored`;
        console.error('[resume]', rail.error);
        return;
      }
      addOwnedSession(ownedSessionFromBackend(imported));
      await host.service.open({ ...record, ownedId });
      // A session you asked to resume is one you want to look at, so bring it
      // forward the same way adopting a scanned session does.
      showCenterTab('session');
      const [transcript, conversation] = await Promise.allSettled([
        finishAgentConversationImportFromTauri(ownedId),
        ensureStructuredConversation({
          ownedId,
          provider: request.provider,
          cwd: request.cwd,
          nativeSessionId: request.nativeSessionId,
          nativeSessionMode: 'resume'
        })
      ]);
      // Both halves wrote to the same session and either could have landed last.
      // Starting the agent reads the conversation back as part of its own work,
      // and when that read wins the race it happens before the records exist.
      // Reading it once more, after both are done, is what makes the transcript
      // appear rather than an empty session under a correct title.
      await loadConversationForRead(ownedId);
      // Either half can fail on its own and the other still has value. A
      // transcript that would not load is a failed resume and is said so. An
      // agent that would not start is not: the conversation is imported, on
      // screen and readable, and the usual reason is that the folder it was
      // recorded in has since been deleted. Throwing there put an error over a
      // conversation that had loaded perfectly well.
      if (transcript.status === 'rejected') throw transcript.reason;
      if (conversation.status === 'rejected') {
        rail.error = `"${record.title}" is open and readable, but its agent could not start: ${describeError(conversation.reason)}`;
        console.warn('[resume]', rail.error, conversation.reason);
      } else if (conversation.value) {
        // Ensuring the conversation records it; it does not ask the agent
        // anything. A session picked up from a transcript has never run, so the
        // models, effort levels and approval policies the composer offers do not
        // exist yet. This asks once and stops the adapter again, and what comes
        // back is stored — every read after this is an ordinary read.
        try {
          const warmed = await warmAgentConversationConfig(ownedId, conversation.value.generation);
          setConversationAgentConfigState(ownedId, warmed);
        } catch (error) {
          // The conversation is imported, on screen and readable. Not knowing
          // what the agent offers is worth a sentence, not a failed resume.
          rail.error = `"${record.title}" is open, but its agent settings could not be read: ${describeError(error)}`;
          console.warn('[resume]', rail.error, error);
        }
      }
    } catch (error) {
      rail.error = `could not resume "${record.title}" as an Assembly session: ${describeError(error)}`;
      console.error('[resume]', rail.error, error);
    }
  }

  /**
   * Run one of a card's actions. Nothing here re-checks whether the action is
   * allowed beyond the roster's own answer, and nothing here fails quietly: an
   * action the record cannot support never becomes pressable in the first place.
   */
  async function runAction(record: SessionLibraryRecord, id: SessionHistoryActionId): Promise<void> {
    const allowed = sessionHistoryActions(record).find((action) => action.id === id);
    if (!allowed?.enabled) return;

    switch (id) {
      case 'resume-assembly':
        await resumeAsAssemblySession(record);
        return;
      case 'view-log':
        if (record.logPath) openFileInEditor({ path: record.logPath });
        return;
      case 'open-log':
        if (record.logPath) await openPathFromTauri(record.logPath);
        return;
      case 'reveal-log':
        if (record.logPath) await revealPathFromTauri(record.logPath);
        return;
      case 'open-working-directory':
        await openPathFromTauri(record.canonicalCwd);
        return;
      case 'copy-resume-command':
        await copyText(sessionResumeCommand(record));
        return;
      case 'copy-session-id':
        await copyText(sessionHistoryIdentity(record));
        return;
      case 'copy-log-path':
        await copyText(record.logPath);
        return;
      case 'delete':
        // Nothing is removed until the question below is answered.
        pendingDelete = record;
        return;
    }
  }

  async function confirmDelete(): Promise<void> {
    const record = pendingDelete;
    pendingDelete = null;
    if (record) await host.service.delete(record);
  }

  function toggleCard(row: SessionHistoryRow): void {
    if (expandedKey === row.record.key) {
      releaseDetails(new Set([row.record.key]));
      expandedKey = null;
      publishWorkspaceState();
      return;
    }
    releaseDetails();
    expandedKey = row.record.key;
    publishWorkspaceState();
    void loadCardDetails(row.record);
  }

  async function loadCardDetails(record: SessionLibraryRecord): Promise<void> {
    const key = record.key;
    const version = ++loadVersion;
    detailLoadingKey = key;
    const projectPath = record.projectPath?.trim() || record.canonicalCwd.trim();
    try {
      const refreshed = await host.service.refresh(
        new Set([key]),
        projectPath ? { projectPath } : undefined,
        { includeDetails: true }
      );
      if (!visible || version !== loadVersion || expandedKey !== key) {
        host.service.release(new Set([key]));
        return;
      }
      const detail = refreshed.find((candidate) => candidate.key === key);
      if (detail) {
        detailsByKey = {
          ...detailsByKey,
          [key]: { firstPrompt: detail.firstPrompt, latestTurns: detail.latestTurns }
        };
      }
      host.service.release(new Set([key]));
    } catch (error) {
      if (version === loadVersion) console.error('[history] could not load session details', error);
    } finally {
      if (version === loadVersion) detailLoadingKey = null;
    }
  }
</script>

<div class="history-panel flex h-full min-h-0 flex-col">
  <PanelHeader title="History" count={summaryViewModel.totalCount}>
    {#snippet actions()}
      <IconButton
        label="Look for sessions again"
        disabled={rail.scanning}
        onclick={() => host.rescan?.()}
      >
        <RefreshCw />
      </IconButton>
    {/snippet}
  </PanelHeader>

  <div data-testid="session-history-host-line" class="flex items-center gap-1.5 px-3 pt-1 text-xs text-muted-foreground">
    <Server class="size-3.5" aria-hidden="true" />
    <span data-testid="session-history-host">{summaryViewModel.totalCount} {summaryViewModel.totalCount === 1 ? 'session' : 'sessions'} from Local Mac</span>
  </div>
  <div data-testid="session-history-scope" class="px-3 py-1">
    <SegmentedControl
      items={SCOPE_OPTIONS}
      value={sessionLibraryState.scope}
      size="sm"
      aria-label="Session history scope"
      onValueChange={setScope}
    />
  </div>
  <div class="px-3 py-2">
    <Input
      class="h-7"
      placeholder="Search sessions"
      autocomplete="off"
      spellcheck="false"
      value={query}
      oninput={(event) => search(event.currentTarget.value)}
    />
  </div>

  <!-- The sessions of one checkout, drawn the same whether they sit under a
       worktree heading or directly under the project. -->
  {#snippet sessionRows(worktree: SessionHistoryWorktreeGroup)}
    {#each worktree.rows as row (row.record.key)}
      <SessionHistoryCard
        record={row.record}
        title={row.displayTitle}
        excerpt={row.excerpt}
        expanded={expandedKey === row.record.key}
        {now}
        onToggle={() => toggleCard(row)}
        onAction={(id) => void runAction(row.record, id)}
        onCopyText={(value) => void copyText(value)}
        onOpenLog={(path) => openFileInEditor({ path })}
        detail={detailsByKey[row.record.key]}
        detailsLoading={detailLoadingKey === row.record.key}
      />
    {/each}
    {#if worktree.olderCount > 0 && loadingOlder}
      <div class="flex justify-center px-3 py-1.5" aria-label="Loading older sessions">
        <WorkingSpinner seed={worktree.key} size={12} />
      </div>
    {/if}
  {/snippet}

  <!-- The list only exists in the document while its tab is showing. A hidden
       tab used to keep every session card mounted under display:none, and the
       browser engine walks that invisible text character by character during
       mouse hit-tests and caret queries — over a hundred thousand hidden
       characters froze the whole app for seconds at a time. The panel's own
       state (search, open groups, paging) lives above and survives; reopening
       the tab rebuilds the cards from it instantly. -->
  {#if visible}
  <!-- The kit's scroll area ships as `type="hover"`, which leaves a reader who
       is not already pointing at the list with no sign that there is more of
       it. This one keeps its bar on screen. -->
  <ScrollArea
    type="always"
    class="min-h-0 flex-1"
    bind:viewportRef={historyViewport}
  >
    {#if summaryViewModel.projects.length === 0}
      <EmptyState
        title={query ? 'Nothing matches that search' : 'No past sessions yet'}
        body={query
          ? 'Try a shorter search, or part of a project name.'
          : 'Sessions you have run appear here, grouped by project, with everything needed to pick one back up.'}
      >
        {#snippet icon()}<History />{/snippet}
      </EmptyState>
    {:else}
      <!-- Groups start collapsed and open one click at a time — mounting every
           card at once is what used to freeze the app. A search is the
           exception: its results are already few, so they all show. -->
      {#each summaryViewModel.projects as project (project.key)}
        {@const open = isSessionHistoryGroupOpen(collapseState, 'project', project.key)}
        {@const loadedProject = loadedProjects.get(project.key)}
        <Collapsible.Root {open} onOpenChange={() => void toggleProject(project)}>
          <h2 class="sticky top-0 z-10 bg-card">
            <Collapsible.Trigger
              class="flex min-h-7 w-full items-center gap-2 px-3 py-1.5 text-left
                     text-[13px] leading-tight font-medium text-foreground outline-none
                     transition-colors hover:bg-accent/60 focus-visible:ring-3
                     focus-visible:ring-ring/50"
            >
              {#if loadingProjectKey === project.key}
                <WorkingSpinner seed={project.key} size={14} />
              {:else}
                <ChevronRight
                  class={`size-3.5 shrink-0 transition-transform duration-150 ${open ? 'rotate-90' : ''}`}
                  aria-hidden="true"
                />
              {/if}
              <span class="min-w-0 flex-1 truncate">{project.name}</span>
              <Chip tone="count">{project.count}</Chip>
            </Collapsible.Trigger>
          </h2>

          <Collapsible.Content>
            {#if loadOutcome?.state === 'failed'}
              <div class="flex items-center justify-between gap-2 px-3 py-2 text-sm text-muted-foreground">
                <span>Could not load sessions.</span>
                <Button size="sm" variant="ghost" onclick={() => void toggleProject(project, true)}>
                  Retry
                </Button>
              </div>
            {:else}
            {#if loadedProject}
            <!-- One checkout means no second heading to sit under, so its
                 sessions stay at the project's own depth. Everything else steps
                 in once per level it is nested, against a rail — without it a
                 worktree heading and the sessions of the worktree above it
                 shared a left edge and the tree read as one flat list. -->
            <div class="history-nest">
              {#each loadedProject.worktrees as worktree (worktree.key)}
                {@const worktreeOpen =
                  loadedProject.singleCheckout
                  || isSessionHistoryGroupOpen(collapseState, 'worktree', worktree.key)}
                {#if loadedProject.singleCheckout}
                  {@render sessionRows(worktree)}
                {:else}
                  <Collapsible.Root
                    open={worktreeOpen}
                    onOpenChange={() => toggleWorktree(worktree.key)}
                  >
                    <Collapsible.Trigger
                      class="flex min-h-6 w-full items-center gap-2 py-1 pr-3 pl-2 text-left text-sm
                             leading-tight text-muted-foreground outline-none transition-colors
                             hover:bg-accent/60 focus-visible:ring-3 focus-visible:ring-ring/50"
                    >
                      <ChevronRight
                        class={`size-3 shrink-0 transition-transform duration-150 ${worktreeOpen ? 'rotate-90' : ''}`}
                        aria-hidden="true"
                      />
                      <span class="min-w-0 flex-1 truncate">{worktree.name}</span>
                      <Chip tone="count">{worktree.count}</Chip>
                    </Collapsible.Trigger>
                    <Collapsible.Content>
                      <div class="history-nest">
                        {@render sessionRows(worktree)}
                      </div>
                    </Collapsible.Content>
                  </Collapsible.Root>
                {/if}
              {/each}
            </div>
            {/if}
            {#if loadOutcome?.state === 'incomplete'}
              <div class="flex items-center justify-between gap-2 px-3 py-2 text-sm text-muted-foreground">
                <span>{loadOutcome.missingKeys.length} sessions could not be loaded.</span>
                <Button
                  size="sm"
                  variant="ghost"
                  onclick={async () => {
                    await host.rescan?.();
                    await toggleProject(project, true);
                  }}
                >
                  Retry
                </Button>
              </div>
            {/if}
            {/if}
          </Collapsible.Content>
        </Collapsible.Root>
      {/each}
    {/if}
  </ScrollArea>
  {/if}
</div>

<AlertDialog.Root
  open={pendingDelete !== null}
  onOpenChange={(open) => {
    if (!open) pendingDelete = null;
  }}
>
  <AlertDialog.Content data-testid="session-history-delete-confirm">
    <AlertDialog.Header>
      <AlertDialog.Title class="flex items-center gap-1.5 text-[14px] leading-snug font-semibold">
        <TriangleAlert class="size-4 shrink-0 text-[var(--color-bad)]" aria-hidden="true" />
        Delete this session?
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-relaxed text-muted-foreground">
        “{pendingDelete?.title ?? ''}” is removed from this app. The transcript file on disk is
        left alone, and this cannot be undone from here.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">Keep it</AlertDialog.Cancel>
      <AlertDialog.Action
        size="sm"
        variant="destructive"
        class="text-[13px]"
        onclick={() => void confirmDelete()}
      >
        Delete session
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<style>
  /*
    One step in per level of nesting, against a rail that runs the height of the
    group. Everything used to sit at the same left edge — a worktree heading, the
    sessions inside it, and the next heading below — so the tree read as a flat
    list and there was no way to see what belonged to what.

    The rail is drawn on the container rather than on each row so it stays
    unbroken between a heading and the cards under it.
  */
  .history-nest {
    margin-left: var(--space-3);
    padding-left: var(--space-2);
    border-left: 1px solid color-mix(in srgb, var(--color-text) 12%, transparent);
  }

  /*
    The kit's scroll thumb is a 7% hairline, which on this surface is a
    scrollbar nobody can see. The bar is the only thing telling a reader that a
    project has more sessions below the fold, so it is drawn to be read.
  */
  .history-panel :global([data-slot='scroll-area-thumb']) {
    background: color-mix(in srgb, var(--color-text) 24%, transparent);
  }
</style>
