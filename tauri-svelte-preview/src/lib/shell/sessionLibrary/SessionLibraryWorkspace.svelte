<script lang="ts">
  /**
   * One canonical Session Library surface. Center and right registrations pass
   * the same store/service and mount this component in only one host at a time.
   * The workspace projects current rail/provider props; it does not fetch or
   * persist anything while it is constructed.
   */
  import Archive from '@lucide/svelte/icons/archive';
  import Copy from '@lucide/svelte/icons/copy';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import FolderGit2 from '@lucide/svelte/icons/folder-git-2';
  import Play from '@lucide/svelte/icons/play';
  import Search from '@lucide/svelte/icons/search';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';

  import type { AgentSession } from '$lib/tauriSource';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import {
    buildSessionLibrary,
    filterSessionLibrary,
    groupSessionLibrary,
    paginateSessionLibrary,
    type SessionLibraryFilters,
    type SessionLibraryRecord
  } from './sessionLibraryModel';
  import {
    inertSessionLibraryService,
    type SessionLibraryService
  } from './sessionLibraryService';
  import {
    sessionLibraryState,
    type SessionLibraryUiState
  } from './sessionLibraryStore.svelte';

  type SessionLibraryDensity = 'compact' | 'comfortable';

  interface Props {
    owned: OwnedSession[];
    available: AgentSession[];
    service?: SessionLibraryService;
    store?: SessionLibraryUiState;
    onRefresh?(): void | Promise<void>;
  }

  let {
    owned,
    available,
    service = inertSessionLibraryService,
    store = sessionLibraryState,
    onRefresh
  }: Props = $props();

  let density = $state<SessionLibraryDensity>('comfortable');

  const records = $derived(buildSessionLibrary(owned, available));
  const filters = $derived<SessionLibraryFilters>({
    query: store.query,
    provider: store.provider,
    project: store.project,
    worktree: store.worktree,
    state: store.state,
    model: store.model,
    dateFrom: store.dateFrom || null,
    dateTo: store.dateTo || null
  });
  const filtered = $derived(filterSessionLibrary(records, filters));
  const page = $derived(paginateSessionLibrary(filtered, store.page, store.pageSize));
  const groups = $derived(groupSessionLibrary(page.items));
  const providers = $derived([...new Set(records.map((record) => record.provider))].sort());
  const models = $derived(
    [...new Set(records.map((record) => record.model).filter((model): model is string => Boolean(model)))].sort()
  );

  function setQuery(value: string): void {
    store.query = value;
    store.page = 1;
  }

  function setFilter<K extends keyof SessionLibraryUiState>(key: K, value: SessionLibraryUiState[K]): void {
    store[key] = value;
    if (key !== 'selectedKey') store.page = 1;
  }

  function setDensity(value: SessionLibraryDensity): void {
    density = value;
  }

  function select(record: SessionLibraryRecord): void {
    store.selectedKey = record.key;
  }

  async function act(action: 'resume' | 'open' | 'fork' | 'archive' | 'delete', record: SessionLibraryRecord): Promise<void> {
    store.selectedKey = record.key;
    await service[action](record);
  }

  function stateLabel(record: SessionLibraryRecord): string {
    return record.state === 'resumable'
      ? 'Resumable'
      : record.state[0].toUpperCase() + record.state.slice(1);
  }
</script>

<section
  data-testid="session-library-workspace"
  class:comfortable={density === 'comfortable'}
  class="library-workspace min-w-0"
  aria-label="Session Library"
>
  <header data-testid="session-library-header" class="library-header">
    <div class="library-title-row">
      <div class="library-title-wrap">
        <h2 data-testid="session-library-title">Session Library</h2>
        <span data-testid="session-library-result-count" class="result-count">{page.totalItems}</span>
      </div>
      <div data-testid="session-library-density" class="density-controls" aria-label="Density">
        <button
          data-testid="session-library-density-compact"
          type="button"
          class:chosen={density === 'compact'}
          aria-pressed={density === 'compact'}
          onclick={() => setDensity('compact')}
        >Compact</button>
        <button
          data-testid="session-library-density-comfortable"
          type="button"
          class:chosen={density === 'comfortable'}
          aria-pressed={density === 'comfortable'}
          onclick={() => setDensity('comfortable')}
        >Comfortable</button>
      </div>
    </div>

    <label data-testid="session-library-search-label" class="search-field">
      <Search data-testid="session-library-search-icon" class="size-3.5 shrink-0" aria-hidden="true" />
      <input
        data-testid="session-library-search"
        type="search"
        value={store.query}
        placeholder="Search title, provider, worktree…"
        oninput={(event) => setQuery(event.currentTarget.value)}
      />
    </label>

    <div data-testid="session-library-filters" class="filter-grid">
      <label data-testid="session-library-provider-filter" class="filter-label">Provider
        <select
          data-testid="session-library-provider"
          value={store.provider}
          onchange={(event) => setFilter('provider', event.currentTarget.value)}
        >
          <option value="">All providers</option>
          {#each providers as provider (provider)}<option value={provider}>{provider}</option>{/each}
        </select>
      </label>
      <label data-testid="session-library-state-filter" class="filter-label">State
        <select
          data-testid="session-library-state"
          value={store.state}
          onchange={(event) => setFilter('state', event.currentTarget.value as SessionLibraryUiState['state'])}
        >
          <option value="all">All states</option>
          <option value="working">Working</option>
          <option value="resumable">Resumable</option>
          <option value="done">Done</option>
          <option value="settled">Settled</option>
        </select>
      </label>
      <label data-testid="session-library-model-filter" class="filter-label">Model
        <select
          data-testid="session-library-model"
          value={store.model}
          onchange={(event) => setFilter('model', event.currentTarget.value)}
        >
          <option value="">All models</option>
          {#each models as model (model)}<option value={model}>{model}</option>{/each}
        </select>
      </label>
      <label data-testid="session-library-project-filter" class="filter-label">Project
        <input data-testid="session-library-project" value={store.project} oninput={(event) => setFilter('project', event.currentTarget.value)} />
      </label>
      <label data-testid="session-library-worktree-filter" class="filter-label">Worktree
        <input data-testid="session-library-worktree" value={store.worktree} oninput={(event) => setFilter('worktree', event.currentTarget.value)} />
      </label>
      <label data-testid="session-library-date-from-filter" class="filter-label">From
        <input data-testid="session-library-date-from" type="date" value={store.dateFrom} oninput={(event) => setFilter('dateFrom', event.currentTarget.value)} />
      </label>
      <label data-testid="session-library-date-to-filter" class="filter-label">To
        <input data-testid="session-library-date-to" type="date" value={store.dateTo} oninput={(event) => setFilter('dateTo', event.currentTarget.value)} />
      </label>
    </div>
  </header>

  <div data-testid="session-library-list" class="library-list" role="list">
    {#if groups.length > 0}
      {#each groups as group (group.state)}
        <section data-testid="session-library-group" data-state={group.state} class="library-group" role="group" aria-label={group.label}>
          <h3 data-testid="session-library-group-heading" class="group-heading">
            <span>{group.label}</span><span>{group.items.length}</span>
          </h3>
          {#each group.items as record (record.key)}
            <div
              data-testid="session-library-row"
              data-session-key={record.key}
              class:selected={store.selectedKey === record.key}
              class="library-row"
              role="button"
              tabindex="0"
              aria-pressed={store.selectedKey === record.key}
              onclick={() => select(record)}
              onkeydown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') select(record);
              }}
            >
              <div class="row-main">
                <span data-testid="session-library-state-dot" class="state-dot" data-state={record.state} aria-hidden="true"></span>
                <span data-testid="session-library-row-title" class="row-title" title={record.title}>{record.title}</span>
                <span data-testid="session-library-row-state" class="row-state">{stateLabel(record)}</span>
              </div>
              <div data-testid="session-library-row-meta" class="row-meta">
                <span><FolderGit2 class="size-3" aria-hidden="true" />{record.canonicalCwd || 'No worktree'}</span>
                <span>{record.provider}</span>
                {#if record.model}<span>{record.model}</span>{/if}
                {#if record.nativeSessionId}<span title={record.nativeSessionId}>#{record.nativeSessionId}</span>{/if}
              </div>
              {#if record.description}
                <p data-testid="session-library-row-preview" class="row-preview" title={record.description}>{record.description}</p>
              {/if}
              <div data-testid="session-library-row-actions" class="row-actions">
                {#if record.state === 'resumable'}
                  <button data-testid="session-library-resume" type="button" title="Resume" aria-label={`Resume ${record.title}`} onclick={(event) => { event.stopPropagation(); void act('resume', record); }}><Play class="size-3.5" aria-hidden="true" /></button>
                {/if}
                <button data-testid="session-library-open" type="button" title="Open" aria-label={`Open ${record.title}`} onclick={(event) => { event.stopPropagation(); void act('open', record); }}><ExternalLink class="size-3.5" aria-hidden="true" /></button>
                <button data-testid="session-library-fork" type="button" title="Fork" aria-label={`Fork ${record.title}`} onclick={(event) => { event.stopPropagation(); void act('fork', record); }}><Copy class="size-3.5" aria-hidden="true" /></button>
                {#if record.state === 'done'}
                  <button data-testid="session-library-archive" type="button" title="Archive to Settled" aria-label={`Archive ${record.title}`} onclick={(event) => { event.stopPropagation(); void act('archive', record); }}><Archive class="size-3.5" aria-hidden="true" /></button>
                {:else if record.state === 'settled'}
                  <button data-testid="session-library-unsettle" type="button" title="Move back to Done" aria-label={`Move ${record.title} back to Done`} onclick={(event) => { event.stopPropagation(); void act('archive', record); }}><Undo2 class="size-3.5" aria-hidden="true" /></button>
                {/if}
                <button data-testid="session-library-delete" type="button" title="Delete" aria-label={`Delete ${record.title}`} onclick={(event) => { event.stopPropagation(); void act('delete', record); }}><Trash2 class="size-3.5" aria-hidden="true" /></button>
              </div>
            </div>
          {/each}
        </section>
      {/each}
    {:else}
      <p data-testid="session-library-empty" class="empty-state">No sessions match these filters.</p>
    {/if}
  </div>

  <footer data-testid="session-library-footer" class="library-footer">
    <button data-testid="session-library-refresh" type="button" class="footer-button" onclick={() => onRefresh?.()}>Refresh</button>
    <label data-testid="session-library-page-size-label" class="page-size-label">Rows
      <select data-testid="session-library-page-size" value={store.pageSize} onchange={(event) => { store.pageSize = Number(event.currentTarget.value); store.page = 1; }}>
        <option value="25">25</option><option value="50">50</option><option value="100">100</option>
      </select>
    </label>
    <button data-testid="session-library-page-previous" type="button" class="footer-button" disabled={page.page <= 1} onclick={() => { store.page = page.page - 1; }}>Previous</button>
    <span data-testid="session-library-page-status" class="page-status">Page {page.page} of {page.totalPages}</span>
    <button data-testid="session-library-page-next" type="button" class="footer-button" disabled={page.page >= page.totalPages} onclick={() => { store.page = page.page + 1; }}>Next</button>
  </footer>
</section>

<style>
  .library-workspace { display: flex; height: 100%; min-height: 0; flex-direction: column; color: var(--color-text); background: var(--color-bg); }
  .library-header { display: flex; flex-direction: column; gap: 8px; border-bottom: 1px solid var(--color-border); padding: 10px; }
  .library-title-row, .library-title-wrap, .density-controls, .row-main, .row-meta, .row-actions, .library-footer { display: flex; align-items: center; }
  .library-title-row { justify-content: space-between; gap: 8px; }
  .library-title-wrap { gap: 8px; }
  h2 { margin: 0; font-size: 15px; font-weight: 650; }
  .result-count, .row-state, .page-status { color: var(--color-text-2); font-size: 12px; }
  .density-controls { gap: 2px; }
  .density-controls button, .footer-button { border-radius: 4px; padding: 4px 7px; color: var(--color-text-2); font-size: 12px; }
  .density-controls button:hover, .density-controls button.chosen, .footer-button:hover:not(:disabled) { background: var(--color-elevated); color: var(--color-text); }
  .search-field { display: flex; align-items: center; gap: 6px; border: 1px solid var(--color-border); border-radius: 5px; padding: 5px 7px; }
  .search-field input, .filter-label input, .filter-label select, .page-size-label select { min-width: 0; border: 0; outline: 0; background: transparent; color: var(--color-text); font: inherit; font-size: 13px; }
  .search-field input { width: 100%; }
  .filter-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(115px, 1fr)); gap: 6px; }
  .filter-label { display: flex; min-width: 0; flex-direction: column; gap: 2px; color: var(--color-text-2); font-size: 12px; }
  .filter-label input, .filter-label select { width: 100%; border: 1px solid var(--color-border); border-radius: 4px; padding: 4px 5px; }
  .library-list { min-height: 0; flex: 1 1 auto; overflow: auto; padding: 5px; }
  .library-group + .library-group { margin-top: 8px; }
  .group-heading { display: flex; align-items: center; justify-content: space-between; margin: 0; padding: 4px 7px; color: var(--color-text-2); font-size: 12px; font-weight: 650; letter-spacing: 0.05em; text-transform: uppercase; }
  .library-row { position: relative; display: flex; min-width: 0; flex-direction: column; gap: 3px; border-bottom: 1px solid var(--color-border); border-radius: 5px; padding: 8px 7px; outline: none; }
  .library-row:hover, .library-row:focus-visible, .library-row.selected { background: var(--color-elevated); }
  .row-main { min-width: 0; gap: 6px; }
  .row-title { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .row-meta { min-width: 0; flex-wrap: wrap; gap: 6px; padding-left: 13px; color: var(--color-text-2); font-size: 12px; }
  .row-meta span { display: inline-flex; min-width: 0; align-items: center; gap: 3px; max-width: 32ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-preview { overflow: hidden; margin: 0; padding-left: 13px; color: var(--color-text-2); font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .row-actions { justify-content: flex-end; gap: 2px; opacity: 0; }
  .library-row:hover .row-actions, .library-row:focus-within .row-actions, .library-row:focus-visible .row-actions { opacity: 1; }
  .row-actions button { display: inline-flex; height: 24px; width: 24px; align-items: center; justify-content: center; border-radius: 4px; color: var(--color-text-2); }
  .row-actions button:hover, .row-actions button:focus-visible { background: var(--color-surface); color: var(--color-text); box-shadow: 0 0 0 2px var(--color-focus); }
  .state-dot { width: 7px; height: 7px; flex: 0 0 auto; border-radius: 999px; background: var(--color-idle); }
  .state-dot[data-state='working'] { background: var(--color-live); }
  .state-dot[data-state='resumable'] { background: var(--color-attention); }
  .state-dot[data-state='done'] { background: var(--color-good); }
  .state-dot[data-state='settled'] { background: var(--color-text-3); }
  .empty-state { padding: 20px 8px; color: var(--color-text-2); font-size: 13px; text-align: center; }
  .library-footer { gap: 8px; border-top: 1px solid var(--color-border); padding: 7px 10px; }
  .page-size-label { display: inline-flex; align-items: center; gap: 4px; margin-left: auto; color: var(--color-text-2); font-size: 12px; }
  .footer-button:disabled { cursor: default; opacity: 0.45; }
  .comfortable .library-row { padding-block: 10px; }
  .comfortable .row-actions { min-height: 24px; }
</style>
