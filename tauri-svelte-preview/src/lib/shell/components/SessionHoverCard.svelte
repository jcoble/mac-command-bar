<script lang="ts">
  import Clock3 from '@lucide/svelte/icons/clock-3';
  import Coins from '@lucide/svelte/icons/coins';
  import Cpu from '@lucide/svelte/icons/cpu';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Laptop from '@lucide/svelte/icons/laptop';
  import Folder from '@lucide/svelte/icons/folder';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  interface Props {
    title: string;
    statusLabel: string;
    statusDetail?: string;
    project: string;
    worktree: string;
    machine?: string | null;
    branch?: string | null;
    /** Which kind of agent runs this session. */
    provider?: string | null;
    model?: string | null;
    lastActivity?: string | null;
    usage?: string | null;
    /** A plain summary of the last error, when the session hit one. */
    error?: string | null;
    statusTone?: 'working' | 'attention' | 'idle' | 'stopped' | 'done' | 'failed';
  }

  let {
    title,
    statusLabel,
    statusDetail = statusLabel,
    project,
    worktree,
    machine = null,
    branch = null,
    provider = null,
    model = null,
    lastActivity = null,
    usage = null,
    error = null,
    statusTone = 'idle'
  }: Props = $props();

  const detailPath = $derived(worktree || project);
</script>

<aside class="session-card" data-testid="session-hover-card" aria-label="Session details">
  <div class="session-card-title" title={title}>{title}</div>
  <div class="session-card-sub">{statusDetail}</div>

  <div class="session-card-lines">
    {#if project || detailPath}
      <div class="session-card-line" title={detailPath || project}>
        <Folder aria-hidden="true" />
        <span class="session-card-value">{detailPath || project}</span>
      </div>
    {/if}
    {#if machine}
      <div class="session-card-line" title={machine}>
        <Laptop aria-hidden="true" />
        <span class="session-card-value">{machine}</span>
      </div>
    {/if}
    {#if branch}
      <div class="session-card-line" title={branch}>
        <GitBranch aria-hidden="true" />
        <span class="session-card-value mono">{branch}</span>
      </div>
    {/if}
    <!-- The row shows the provider as a glyph and nothing else, so this is the
         one place the agent and its model are spelled out. -->
    {#if provider || model}
      <div
        data-testid="session-hover-card-agent"
        class="session-card-line"
        title={[provider, model].filter(Boolean).join(' · ')}
      >
        <Cpu aria-hidden="true" />
        <span class="session-card-value">{provider ?? 'Agent'}</span>
        {#if model}
          <span class="session-card-model">{model}</span>
        {/if}
      </div>
    {/if}
    {#if lastActivity}
      <div class="session-card-line" title={lastActivity}>
        <Clock3 aria-hidden="true" />
        <span class="session-card-value">Last activity {lastActivity}</span>
      </div>
    {/if}
    {#if error}
      <div data-testid="session-hover-card-error" class="session-card-line error" title={error}>
        <TriangleAlert aria-hidden="true" />
        <span class="session-card-value">{error}</span>
      </div>
    {/if}
  </div>

  {#if usage}
    <div class="session-card-foot">
      <span class="status-pill {statusTone}"><span class="status-dot" aria-hidden="true"></span>{statusLabel}</span>
      <span class="usage-line"><Coins aria-hidden="true" />{usage}</span>
    </div>
  {:else}
    <div class="session-card-foot">
      <span class="status-pill {statusTone}"><span class="status-dot" aria-hidden="true"></span>{statusLabel}</span>
    </div>
  {/if}
</aside>

<style>
  /* Same surface, hairline and shadow as the row's own menu, so the two
     floating layers of the rail read as one material. */
  .session-card {
    width: 336px;
    max-width: calc(100vw - 24px);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: var(--floating-content-inset);
    color: var(--color-text);
    background: var(--popout-surface);
    box-shadow: var(--shadow-md);
    pointer-events: none;
  }

  .session-card-title {
    color: var(--color-text);
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .session-card-sub {
    margin: 0 0 12px;
    color: var(--secondary-label);
    font-size: 13px;
    line-height: 18px;
  }

  .session-card-lines {
    display: flex;
    flex-direction: column;
    gap: var(--floating-row-gap);
  }

  .session-card-line {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 10px;
    color: var(--secondary-label);
    font-size: 13px;
  }

  .session-card-line :global(svg) {
    width: 12px;
    height: 12px;
    flex: 0 0 12px;
    color: var(--secondary-label);
  }

  .session-card-value {
    min-width: 0;
    overflow: hidden;
    color: color-mix(in srgb, var(--color-text) 75%, transparent);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The model sits beside the agent name as its own quiet chip. */
  .session-card-model {
    flex: 0 0 auto;
    margin-left: auto;
    padding: 1px 7px 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text-2);
    font-size: 13px;
    line-height: 18px;
    white-space: nowrap;
  }

  .session-card-line.error,
  .session-card-line.error :global(svg),
  .session-card-line.error .session-card-value { color: var(--color-bad); }

  .session-card-value.mono {
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .session-card-foot {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 8px;
    margin-top: 13px;
    padding-top: 11px;
    border-top: 1px solid var(--color-border);
    color: var(--secondary-label);
    font-size: 13px;
    line-height: 18px;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 19px;
    padding: 0 7px;
    border-radius: 5px;
    font-size: 13px;
    font-weight: 550;
    line-height: 17.25px;
    white-space: nowrap;
  }

  .status-pill.working { color: var(--color-accent); background: color-mix(in srgb, var(--color-accent) 14%, transparent); }
  .status-pill.attention { color: var(--color-attention); background: var(--color-attention-bg); }
  .status-pill.idle { color: var(--color-idle); background: color-mix(in srgb, var(--color-idle) 12%, transparent); }
  .status-pill.stopped { color: var(--secondary-label); background: var(--row-selected); }
  .status-pill.done { color: var(--color-good); background: var(--color-good-bg); }
  .status-pill.failed { color: var(--color-bad); background: var(--color-bad-bg); }
  .status-dot { width: 7px; height: 7px; flex: 0 0 auto; border-radius: 999px; background: currentColor; }
  .status-pill.stopped .status-dot { border: 1px solid currentColor; background: transparent; }

  .usage-line {
    display: inline-flex;
    min-width: 0;
    align-items: center;
    gap: 5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .usage-line :global(svg) {
    width: 12px;
    height: 12px;
    flex: 0 0 12px;
  }
</style>
