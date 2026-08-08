<script lang="ts">
  /**
   * The one full Session History surface. It is mounted in the existing
   * `session-library` center panel and keeps all projection/UI state in the
   * shared session-library store. Provider scans and rail data remain props.
   */
  import { onMount } from 'svelte';

  import Bot from '@lucide/svelte/icons/bot';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Copy from '@lucide/svelte/icons/copy';
  import FileText from '@lucide/svelte/icons/file-text';
  import Filter from '@lucide/svelte/icons/filter';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';
  import Server from '@lucide/svelte/icons/server';
  import SquareCode from '@lucide/svelte/icons/square-code';
  import Terminal from '@lucide/svelte/icons/terminal';

  import type { AgentSession } from '$lib/tauriSource';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { formatLastActivity } from '$lib/shell/relativeTime';
  import {
    buildSessionLibrary,
    filterSessionHistory,
    groupSessionHistory,
    paginateSessionLibrary,
    toggleSessionLibraryExpansion,
    type SessionHistoryFilters,
    type SessionHistoryScope,
    type SessionLibraryRecord
  } from './sessionLibraryModel';
  import {
    sessionContextMenuRoster,
    type SessionContextMenuAction,
    type SessionContextMenuItem
  } from './sessionLibraryContextMenu';
  import {
    registerSessionLibraryOpenHandler
  } from './sessionLibraryNavigation';
  import {
    inertSessionLibraryService,
    type SessionLibraryAction,
    type SessionLibraryService
  } from './sessionLibraryService';
  import {
    sessionLibraryState,
    type SessionLibraryUiState
  } from './sessionLibraryStore.svelte';
  import SessionContextMenu from './SessionContextMenu.svelte';

  type ProviderIcon = typeof Bot;
  const SCOPE_OPTIONS: Array<[SessionHistoryScope, string]> = [
    ['workspace', 'Workspace'],
    ['project', 'Project'],
    ['all', 'All']
  ];

  interface Props {
    owned: OwnedSession[];
    available: AgentSession[];
    service?: SessionLibraryService;
    store?: SessionLibraryUiState;
    /** The currently selected worktree/project, used by the segmented scopes. */
    workspacePath?: string | null;
    projectPath?: string | null;
    /** Activates the already-registered center tab; never creates a new panel. */
    onOpenTab?(): void;
    onRefresh?(): void | Promise<void>;
  }

  type ContextMenuState = {
    target: 'session' | 'center-tab';
    record: SessionLibraryRecord | null;
    x: number;
    y: number;
    items: SessionContextMenuItem[];
  };

  let {
    owned,
    available,
    service = inertSessionLibraryService,
    store = sessionLibraryState,
    workspacePath = null,
    projectPath = null,
    onOpenTab,
    onRefresh
  }: Props = $props();

  let filtersOpen = $state(false);
  let contextMenu = $state<ContextMenuState | null>(null);
  let now = $state(new Date());

  const records = $derived(buildSessionLibrary(owned, available));
  const filters = $derived<SessionHistoryFilters>({
    query: store.query,
    provider: store.provider,
    project: '',
    worktree: '',
    state: 'all',
    model: store.model,
    dateFrom: store.dateFrom || null,
    dateTo: store.dateTo || null,
    scope: store.scope,
    workspacePath,
    projectPath
  });
  const filtered = $derived(filterSessionHistory(records, filters));
  const ordered = $derived(
    [...filtered].sort((left, right) => {
      const leftTime = left.updatedAt ? Date.parse(left.updatedAt) : 0;
      const rightTime = right.updatedAt ? Date.parse(right.updatedAt) : 0;
      return rightTime - leftTime || left.title.localeCompare(right.title);
    })
  );
  const page = $derived(paginateSessionLibrary(ordered, store.page, store.pageSize));
  const allGroups = $derived(groupSessionHistory(ordered));
  const groups = $derived(groupSessionHistory(page.items));
  const groupCounts = $derived(new Map(allGroups.map((group) => [group.key, group.items.length])));
  const providers = $derived([...new Set(records.map((record) => record.provider))].sort());
  const models = $derived(
    [...new Set(records.map((record) => record.model).filter((model): model is string => Boolean(model)))].sort()
  );
  const expandedKey = $derived(store.expandedKey ?? store.selectedKey);

  function setQuery(value: string): void {
    store.query = value;
    store.page = 1;
  }

  function setScope(scope: SessionHistoryScope): void {
    store.scope = scope;
    store.page = 1;
  }

  function setFilter<K extends keyof SessionLibraryUiState>(key: K, value: SessionLibraryUiState[K]): void {
    store[key] = value;
    if (key !== 'selectedKey' && key !== 'expandedKey') store.page = 1;
  }

  function toggle(record: SessionLibraryRecord): void {
    const next = toggleSessionLibraryExpansion(expandedKey, record.key);
    store.expandedKey = next;
    store.selectedKey = next;
  }

  async function runAction(action: SessionLibraryAction, record: SessionLibraryRecord): Promise<void> {
    store.selectedKey = record.key;
    await service[action](record);
  }

  function providerIcon(provider: string): ProviderIcon {
    if (provider.includes('codex')) return SquareCode;
    if (provider.includes('cmux')) return Terminal;
    return Bot;
  }

  function messageLabel(record: SessionLibraryRecord): string {
    if (!record.messageCount || record.messageCount < 1) return 'No message count';
    return record.messageCount === 1 ? '1 message' : `${record.messageCount} messages`;
  }

  function ageLabel(record: SessionLibraryRecord): string {
    return formatLastActivity(record.updatedAt, now) || 'Age unavailable';
  }

  function sessionId(record: SessionLibraryRecord): string {
    return record.nativeSessionId || record.ownedId || record.key;
  }

  async function copyText(value: string): Promise<void> {
    if (!value || typeof navigator === 'undefined' || !navigator.clipboard?.writeText) return;
    await navigator.clipboard.writeText(value);
  }

  async function copyFirstPrompt(record: SessionLibraryRecord): Promise<void> {
    if (record.firstPrompt) await copyText(record.firstPrompt);
  }

  function openRowMenu(record: SessionLibraryRecord, event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    store.selectedKey = record.key;
    contextMenu = {
      target: 'session',
      record,
      x: event.clientX,
      y: event.clientY,
      items: sessionContextMenuRoster({ target: 'session', record })
    };
  }

  function openCenterTabMenu(event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    const tab = target?.closest('.dv-tab');
    if (!tab) return;
    const label = (tab.textContent ?? '').replace(/\s+/g, ' ').trim();
    if (!/Session History|Session Library/i.test(label)) return;
    event.preventDefault();
    contextMenu = {
      target: 'center-tab',
      record: null,
      x: event.clientX,
      y: event.clientY,
      items: sessionContextMenuRoster({ target: 'center-tab' })
    };
  }

  async function selectContextAction(action: SessionContextMenuAction): Promise<void> {
    const current = contextMenu;
    contextMenu = null;
    if (!current) return;
    const record = current.record;
    if (!record) {
      if (action === 'copy-id') await copyText('session-library');
      return;
    }
    if (action === 'resume-worktree') await runAction('resume', record);
    else if (action === 'copy-id') await copyText(sessionId(record));
    else if (action === 'archive') await runAction('archive', record);
    else if (action === 'delete') await runAction('delete', record);
  }

  function closeContextMenu(): void {
    contextMenu = null;
  }

  onMount(() => {
    const releaseOpenHandler = registerSessionLibraryOpenHandler(() => onOpenTab?.());
    const onContextMenu = (event: Event): void => {
      if (event instanceof MouseEvent) openCenterTabMenu(event);
    };
    const onPointerDown = (event: Event): void => {
      const target = event.target as HTMLElement | null;
      if (target?.closest('[data-testid="session-context-menu"]')) return;
      closeContextMenu();
    };
    document.addEventListener('contextmenu', onContextMenu, true);
    document.addEventListener('pointerdown', onPointerDown, true);
    const timer = window.setInterval(() => (now = new Date()), 60_000);
    return () => {
      releaseOpenHandler();
      document.removeEventListener('contextmenu', onContextMenu, true);
      document.removeEventListener('pointerdown', onPointerDown, true);
      window.clearInterval(timer);
    };
  });
</script>

<section
  data-testid="session-library-workspace"
  data-session-history-workspace="true"
  class="history-workspace min-w-0"
  aria-label="Agent Session History"
>
  <header data-testid="session-history-header" class="history-header">
    <div class="history-heading-row">
      <div class="history-heading">
        <div class="history-title-line">
          <h2 data-testid="session-library-title">Agent Session History</h2>
          <span data-testid="session-history-total-count" class="count-badge">{filtered.length}</span>
        </div>
        <p data-testid="session-history-host-line" class="history-subtitle">
          {filtered.length} {filtered.length === 1 ? 'session' : 'sessions'} from Local Mac
        </p>
      </div>
      <div class="history-header-actions">
        <span data-testid="session-history-host" class="host-chip"><Server class="size-4" aria-hidden="true" />Local Mac</span>
        <button
          data-testid="session-history-filter-toggle"
          type="button"
          class:active={filtersOpen}
          class="icon-button"
          aria-label="Filter sessions"
          aria-pressed={filtersOpen}
          title="Filter sessions"
          onclick={() => (filtersOpen = !filtersOpen)}
        ><Filter class="size-4" aria-hidden="true" /></button>
        <button
          data-testid="session-history-refresh"
          type="button"
          class="icon-button"
          aria-label="Refresh session history"
          title="Refresh session history"
          onclick={() => void onRefresh?.()}
        ><RefreshCw class="size-4" aria-hidden="true" /></button>
      </div>
    </div>

    <div data-testid="session-history-scope" class="scope-tabs" role="tablist" aria-label="Session history scope">
      {#each SCOPE_OPTIONS as scope (scope[0])}
        <button
          data-testid={`session-history-scope-${scope[0]}`}
          type="button"
          role="tab"
          aria-selected={store.scope === scope[0]}
          class:chosen={store.scope === scope[0]}
          onclick={() => setScope(scope[0])}
        >{scope[1]}</button>
      {/each}
    </div>

    <label data-testid="session-history-search-label" class="search-field">
      <Search class="size-4 shrink-0" aria-hidden="true" />
      <input
        data-testid="session-history-search"
        type="search"
        value={store.query}
        placeholder="Search all sessions"
        aria-label="Search all sessions"
        oninput={(event) => setQuery(event.currentTarget.value)}
      />
    </label>

    {#if filtersOpen}
      <div data-testid="session-history-filters" class="filter-grid">
        <label>Provider
          <select data-testid="session-history-provider" value={store.provider} onchange={(event) => setFilter('provider', event.currentTarget.value)}>
            <option value="">All providers</option>
            {#each providers as provider (provider)}<option value={provider}>{provider}</option>{/each}
          </select>
        </label>
        <label>Model
          <select data-testid="session-history-model-filter" value={store.model} onchange={(event) => setFilter('model', event.currentTarget.value)}>
            <option value="">All models</option>
            {#each models as model (model)}<option value={model}>{model}</option>{/each}
          </select>
        </label>
        <label>From
          <input data-testid="session-history-date-from" type="date" value={store.dateFrom} oninput={(event) => setFilter('dateFrom', event.currentTarget.value)} />
        </label>
        <label>To
          <input data-testid="session-history-date-to" type="date" value={store.dateTo} oninput={(event) => setFilter('dateTo', event.currentTarget.value)} />
        </label>
      </div>
    {/if}
  </header>

  <div data-testid="session-history-list" class="history-list" role="list">
    {#if groups.length === 0}
      <p data-testid="session-history-empty" class="empty-state">No sessions match this view.</p>
    {:else}
      {#each groups as group (group.key)}
        <section data-testid="session-history-group" data-project-path={group.path ?? ''} class="history-group" role="group" aria-label={group.name}>
          <div class="group-heading">
            <div class="group-title"><ChevronDown class="size-4" aria-hidden="true" /><span>{group.name}</span></div>
            <span data-testid="session-history-group-count" class="group-count">{groupCounts.get(group.key) ?? group.items.length}</span>
          </div>

          {#each group.items as record (record.key)}
            {@const ProviderIcon = providerIcon(record.provider)}
            {@const isExpanded = expandedKey === record.key}
            <article
              data-testid="session-history-row"
              data-session-key={record.key}
              class:expanded={isExpanded}
              class:selected={store.selectedKey === record.key}
              class="history-row"
              role="listitem"
              oncontextmenu={(event) => openRowMenu(record, event)}
            >
              <button
                data-testid="session-history-row-trigger"
                type="button"
                class="row-trigger"
                aria-expanded={isExpanded}
                onclick={() => toggle(record)}
                onkeydown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    toggle(record);
                  }
                }}
              >
                <span class="row-chevron" aria-hidden="true">
                  {#if isExpanded}<ChevronDown class="size-4" />{:else}<ChevronRight class="size-4" />{/if}
                </span>
                <span class="row-copy">
                  <span data-testid="session-history-row-title" class="row-title" title={record.title}>{record.title}</span>
                  <span data-testid="session-history-row-summary" class="row-summary">{record.description ?? 'No summary from the session index.'}</span>
                  <span class="row-meta">
                    <span data-testid="session-history-provider-icon" class="provider-icon" title={record.provider} aria-label={`Provider ${record.provider}`}><ProviderIcon class="size-4" aria-hidden="true" /></span>
                    <span data-testid="session-history-message-count">{messageLabel(record)}</span>
                    <span data-testid="session-history-age" title={record.updatedAt ?? undefined}>{ageLabel(record)}</span>
                    {#if record.model}<span data-testid="session-history-model" class="model-label">{record.model}</span>{/if}
                  </span>
                </span>
              </button>

              <button
                data-testid="session-history-row-menu"
                type="button"
                class="row-menu-button"
                aria-label={`More actions for ${record.title}`}
                title="More actions"
                onclick={(event) => openRowMenu(record, event)}
              >⋯</button>

              {#if isExpanded}
                <div data-testid="session-history-details" class="details-card">
                  <div class="details-actions">
                    <button data-testid="session-history-action-resume" type="button" class="primary-action" onclick={() => void runAction('resume', record)}>
                      <span aria-hidden="true">▶</span>Resume in Worktree
                    </button>
                    <button data-testid="session-history-action-continue" type="button" class="secondary-action" disabled title="Future-native: continue in a new session is not wired yet.">
                      <span aria-hidden="true">＋</span>Continue in New Session
                    </button>
                    <button data-testid="session-history-action-log" type="button" class="secondary-action" disabled title="Future-native: transcript log viewing is not wired yet.">
                      <FileText class="size-4" aria-hidden="true" />View Log
                    </button>
                  </div>

                  <div data-testid="session-history-first-prompt" class="detail-block">
                    <div class="detail-heading"><span>FIRST PROMPT</span>
                      <button type="button" class="copy-button" disabled={!record.firstPrompt} title={record.firstPrompt ? 'Copy first prompt' : 'Future-native: first prompt is not in the session index.'} aria-label="Copy first prompt" onclick={() => void copyFirstPrompt(record)}><Copy class="size-4" aria-hidden="true" />Copy</button>
                    </div>
                    {#if record.firstPrompt}<p>{record.firstPrompt}</p>{:else}<p class="unavailable">First prompt is not available from the session index yet.</p>{/if}
                  </div>

                  <div data-testid="session-history-latest-turns" class="detail-block">
                    <div class="detail-heading"><span>LATEST TURNS</span><span class="turn-count">{record.latestTurns.length}</span></div>
                    {#if record.latestTurns.length > 0}
                      {#each record.latestTurns as turn, turnIndex (`${record.key}:${turnIndex}`)}
                        <div class="turn-block">
                          <span class="turn-speaker">{turn.speaker === 'user' ? 'YOU' : 'AGENT'}</span>
                          <p>{turn.text}</p>
                        </div>
                      {/each}
                    {:else}
                      <p class="unavailable">Recent turns are not available from the session index yet.</p>
                    {/if}
                  </div>
                </div>
              {/if}
            </article>
          {/each}
        </section>
      {/each}
    {/if}
  </div>

  <footer data-testid="session-history-footer" class="history-footer">
    <span>{page.items.length} shown of {page.totalItems}</span>
    <label>Rows
      <select data-testid="session-history-page-size" value={store.pageSize} onchange={(event) => { store.pageSize = Number(event.currentTarget.value); store.page = 1; }}>
        <option value="25">25</option><option value="50">50</option><option value="100">100</option>
      </select>
    </label>
    <button data-testid="session-history-page-previous" type="button" disabled={page.page <= 1} onclick={() => (store.page = page.page - 1)}>Previous</button>
    <span data-testid="session-history-page-status">Page {page.page} of {page.totalPages}</span>
    <button data-testid="session-history-page-next" type="button" disabled={page.page >= page.totalPages} onclick={() => (store.page = page.page + 1)}>Next</button>
  </footer>
</section>

{#if contextMenu}
  <SessionContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    items={contextMenu.items}
    onSelect={(action) => void selectContextAction(action)}
    onClose={closeContextMenu}
  />
{/if}

<style>
  .history-workspace { display: flex; height: 100%; min-height: 0; flex-direction: column; overflow: hidden; color: var(--color-text); background: var(--color-bg); }
  .history-header { display: flex; flex: 0 0 auto; flex-direction: column; gap: 12px; border-bottom: 1px solid var(--color-border); padding: 16px 18px 14px; }
  .history-heading-row, .history-title-line, .history-header-actions, .host-chip, .scope-tabs, .search-field, .row-meta, .group-title, .history-footer, .details-actions, .detail-heading { display: flex; align-items: center; }
  .history-heading-row { justify-content: space-between; gap: 16px; }
  .history-heading { min-width: 0; }
  .history-title-line { gap: 10px; }
  h2 { margin: 0; font-size: 19px; font-weight: 650; letter-spacing: -0.01em; }
  .count-badge, .group-count { border-radius: 999px; background: var(--color-elevated); color: var(--color-text-2); font-size: 12px; line-height: 1; }
  .count-badge { padding: 6px 9px; }
  .history-subtitle { margin: 4px 0 0; color: var(--color-text-2); font-size: 13px; }
  .history-header-actions { gap: 7px; }
  .host-chip { gap: 7px; color: var(--color-text); font-size: 13px; white-space: nowrap; }
  .icon-button, .row-menu-button, .copy-button { display: inline-flex; align-items: center; justify-content: center; border-radius: var(--radius-sm); color: var(--color-text-2); }
  .icon-button { height: 30px; width: 30px; }
  .icon-button:hover, .icon-button:focus-visible, .icon-button.active { background: var(--color-elevated); color: var(--color-text); outline: none; box-shadow: var(--focus-ring); }
  .scope-tabs { width: min(100%, 620px); overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); }
  .scope-tabs button { flex: 1 1 0; padding: 8px 12px; color: var(--color-text-2); font-size: 13px; }
  .scope-tabs button:hover, .scope-tabs button:focus-visible { background: var(--color-elevated); color: var(--color-text); outline: none; }
  .scope-tabs button.chosen { background: var(--color-selected); color: var(--color-text); box-shadow: inset 0 -2px 0 var(--color-accent); }
  .search-field { gap: 9px; border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: 9px 11px; background: var(--color-surface); color: var(--color-text-2); }
  .search-field:focus-within { border-color: var(--color-selected-border); box-shadow: var(--focus-ring); }
  .search-field input { min-width: 0; width: 100%; border: 0; outline: 0; background: transparent; color: var(--color-text); font: inherit; font-size: 14px; }
  .search-field input::placeholder { color: var(--color-text-3); }
  .filter-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 8px; }
  .filter-grid label { display: flex; min-width: 0; flex-direction: column; gap: 4px; color: var(--color-text-2); font-size: 12px; }
  .filter-grid input, .filter-grid select, .history-footer select { min-width: 0; border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 6px 7px; background: var(--color-surface); color: var(--color-text); font: inherit; font-size: 12px; }
  .history-list { min-height: 0; flex: 1 1 auto; overflow: auto; padding: 8px 18px 18px; }
  .history-group + .history-group { margin-top: 16px; }
  .group-heading { display: flex; align-items: center; justify-content: space-between; padding: 9px 4px 7px; color: var(--color-text-2); font-size: 13px; font-weight: 650; }
  .group-title { min-width: 0; gap: 7px; }
  .group-title span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .group-count { min-width: 26px; padding: 5px 7px; text-align: center; }
  .history-row { position: relative; display: grid; grid-template-columns: minmax(0, 1fr) auto; border-top: 1px solid var(--color-border); background: transparent; }
  .history-row:hover, .history-row:focus-within, .history-row.selected { background: var(--color-surface); }
  .history-row.expanded { background: var(--color-surface); box-shadow: inset 3px 0 0 var(--color-accent); }
  .row-trigger { display: flex; min-width: 0; gap: 9px; padding: 12px 6px 12px 4px; text-align: left; }
  .row-trigger:hover, .row-trigger:focus-visible { outline: none; }
  .row-chevron { flex: 0 0 auto; padding-top: 1px; color: var(--color-text-3); }
  .row-copy { display: flex; min-width: 0; flex-direction: column; gap: 5px; }
  .row-title, .row-summary { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-title { color: var(--color-text); font-size: 15px; font-weight: 570; }
  .row-summary { color: var(--color-text-2); font-size: 13px; }
  .row-meta { flex-wrap: wrap; gap: 9px; color: var(--color-text-3); font-size: 12px; }
  .provider-icon { display: inline-flex; align-items: center; color: var(--color-accent); }
  .model-label { max-width: 22ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-menu-button { align-self: start; height: 30px; width: 30px; margin: 9px 7px 0 0; font-size: 18px; line-height: 1; }
  .row-menu-button:hover, .row-menu-button:focus-visible { background: var(--color-elevated); color: var(--color-text); outline: none; box-shadow: var(--focus-ring); }
  .details-card { grid-column: 1 / -1; margin: 0 12px 14px 36px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg); box-shadow: var(--shadow-sm); }
  .details-actions { flex-wrap: wrap; gap: 8px; border-bottom: 1px solid var(--color-border); padding: 12px; }
  .primary-action, .secondary-action { display: inline-flex; align-items: center; gap: 7px; border-radius: var(--radius-md); padding: 8px 11px; font-size: 13px; }
  .primary-action { background: var(--color-text); color: var(--color-bg); }
  .primary-action:hover, .primary-action:focus-visible { background: var(--color-accent); outline: none; }
  .secondary-action { background: var(--color-elevated); color: var(--color-text); }
  .secondary-action:hover:not(:disabled), .secondary-action:focus-visible:not(:disabled) { background: var(--color-hover); outline: none; box-shadow: var(--focus-ring); }
  .secondary-action:disabled { cursor: not-allowed; color: var(--color-disabled-text); opacity: 0.7; }
  .detail-block { padding: 14px 13px; }
  .detail-block + .detail-block { border-top: 1px solid var(--color-border); }
  .detail-heading { justify-content: space-between; color: var(--color-text-2); font-size: 12px; font-weight: 700; letter-spacing: 0.1em; }
  .copy-button { gap: 6px; padding: 4px 6px; font-size: 12px; letter-spacing: 0; }
  .copy-button:hover:not(:disabled), .copy-button:focus-visible:not(:disabled) { background: var(--color-elevated); color: var(--color-text); outline: none; }
  .copy-button:disabled { cursor: not-allowed; color: var(--color-disabled-text); }
  .detail-block p { margin: 10px 0 0; color: var(--color-text); font-size: 14px; line-height: 1.45; white-space: pre-wrap; }
  .detail-block p.unavailable { color: var(--color-text-3); font-style: italic; }
  .turn-count { border-radius: 999px; background: var(--color-elevated); padding: 3px 7px; letter-spacing: 0; }
  .turn-block { margin-top: 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: 10px 11px; background: var(--color-surface); }
  .turn-speaker { color: var(--color-text-2); font-size: 12px; font-weight: 700; letter-spacing: 0.08em; }
  .turn-block p { margin-top: 6px; }
  .empty-state { padding: 40px 12px; color: var(--color-text-2); text-align: center; }
  .history-footer { flex: 0 0 auto; gap: 10px; border-top: 1px solid var(--color-border); padding: 9px 18px; color: var(--color-text-2); font-size: 12px; }
  .history-footer label { display: inline-flex; align-items: center; gap: 5px; margin-left: auto; }
  .history-footer button { border-radius: var(--radius-sm); padding: 5px 7px; color: var(--color-text-2); }
  .history-footer button:hover:not(:disabled), .history-footer button:focus-visible { background: var(--color-elevated); color: var(--color-text); outline: none; }
  .history-footer button:disabled { cursor: default; color: var(--color-disabled-text); }
  @media (max-width: 720px) {
    .history-header, .history-list { padding-inline: 12px; }
    .host-chip { display: none; }
    .details-card { margin-left: 14px; }
    .history-footer { padding-inline: 12px; }
  }
</style>
