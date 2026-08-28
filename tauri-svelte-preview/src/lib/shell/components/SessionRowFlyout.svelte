<script lang="ts">
  import { AGENT_ICONS, agentDisplayName } from '$lib/shell/agentIcons.ts';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import { resolveOwnedSessionProject, type OwnedSession } from '$lib/shell/ownedSessions';
  import { sessionLabel } from '$lib/shell/sessionStrip';

  interface Props {
    session: OwnedSession;
    top: number;
    left: number;
  }

  let { session, top, left }: Props = $props();
  const label = $derived(sessionLabel(session));
  const project = $derived(resolveOwnedSessionProject(session));
  const status = $derived(deriveOwnedLibraryState(session));
  const ProviderIcon = $derived(AGENT_ICONS[session.agent]);
  const provider = $derived(agentDisplayName(session.agent, session.viaCmux));
</script>

<aside
  data-testid="session-row-flyout"
  class="flyout"
  style={`top:${top}px;left:${left}px`}
  aria-label={`Details for ${label}`}
>
  <header>
    <span class="mark" data-agent={session.agent}><ProviderIcon aria-hidden="true" /></span>
    <span class="heading">
      <strong>{label}</strong>
      <span>{provider} · {status}</span>
    </span>
  </header>
  <dl>
    <dt>Project</dt><dd>{project.label}</dd>
    {#if session.branch}<dt>Branch</dt><dd>{session.branch}</dd>{/if}
    <dt>Checkout</dt><dd>{session.cwd || session.projectPath || 'Checkout/Worktree deleted.'}</dd>
  </dl>
</aside>

<style>
  .flyout{position:fixed;z-index:900;width:320px;box-sizing:border-box;padding:14px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface);box-shadow:var(--shadow-lg);color:var(--color-text);pointer-events:none}
  header{display:flex;align-items:center;gap:10px;margin-bottom:12px}
  .mark{display:grid;width:36px;height:36px;flex:0 0 auto;place-items:center;border-radius:var(--radius-sm);background:var(--color-elevated);color:var(--color-text-2)}
  .mark :global(svg){width:20px;height:20px}.mark[data-agent='claude']{color:var(--agent-mark-claude)}.mark[data-agent='codex']{color:var(--agent-mark-codex)}
  .heading{display:flex;min-width:0;flex-direction:column;gap:2px}.heading strong{overflow:hidden;font-size:14px;text-overflow:ellipsis;white-space:nowrap}.heading span{color:var(--color-text-2);font-size:12px;text-transform:capitalize}
  dl{display:grid;grid-template-columns:auto minmax(0,1fr);gap:6px 12px;margin:0;font-size:12px}dt{color:var(--color-text-3)}dd{min-width:0;margin:0;overflow:hidden;color:var(--color-text-2);text-overflow:ellipsis;white-space:nowrap}
</style>
