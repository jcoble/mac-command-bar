<script lang="ts">
  import { scanWorkspaceSpaceRoots, workspaceSpaceState } from './workspaceSpaceStore.svelte.ts';
  import { diskProtectionLabel, reclaimableBytes } from './workspaceSpaceViewModel.ts';
  import { formatBytes } from './resourceViewModel.ts';
  import { resourceService } from './resourceService.ts';
  import type {
    DiskProtection,
    ResourceDiskRoot,
    WorkspaceDiskEntry,
    WorkspaceDiskKind
  } from './resourceTypes.ts';

  interface Props { roots: ResourceDiskRoot[]; }
  let { roots }: Props = $props();
  let additionalRoots = $state<ResourceDiskRoot[]>([]);
  let configuredRoots = $derived([...roots, ...additionalRoots]);
  let pathInput = $state('');
  let repositoryInput = $state('');
  let workspaceInput = $state('');
  let kindInput = $state<WorkspaceDiskKind>('build-output');
  let protectionInput = $state<DiskProtection>('safe-candidate');
  let cleaningId = $state<string | null>(null);
  let lastReceipt = $state<string | null>(null);

  function addExplicitRoot(): void {
    const path = pathInput.trim();
    if (!path) return;
    additionalRoots = [
      ...additionalRoots,
      {
        repositoryId: repositoryInput.trim() || 'explicit-root',
        workspaceId: workspaceInput.trim() || path,
        path,
        kind: kindInput,
        protection: protectionInput
      }
    ];
    pathInput = '';
  }

  function removeExplicitRoot(path: string): void {
    additionalRoots = additionalRoots.filter((root) => root.path !== path);
  }

  async function clean(entry: WorkspaceDiskEntry): Promise<void> {
    if (entry.protection !== 'safe-candidate' || reclaimableBytes(entry) === 0) return;
    if (!window.confirm(`Clean ${entry.path}? This removes only the explicit safe candidate.`)) return;
    cleaningId = entry.id;
    lastReceipt = null;
    try {
      const receipt = await resourceService.cleanup({
        repositoryId: entry.repositoryId,
        workspaceId: entry.workspaceId,
        path: entry.path,
        kind: entry.kind,
        protection: entry.protection,
        expectedId: entry.id,
        expectedBytes: entry.bytes,
        maxDepth: 3,
        maxEntries: 2000
      });
      lastReceipt = receipt
        ? `${formatBytes(receipt.reclaimedBytes)} reclaimed; ${receipt.message}`
        : 'Cleanup is available in the desktop app.';
      if (receipt) await scanWorkspaceSpaceRoots(configuredRoots);
    } finally {
      cleaningId = null;
    }
  }
</script>

<section class="workspace" data-testid="workspace-space-workspace" aria-label="Workspace space">
  <header><div><p class="eyebrow">Workspace space</p><h2>Bounded disk review</h2><p class="muted">Only explicit roots are scanned. Protected rows stay visible but cannot be removed.</p></div><button type="button" onclick={() => void scanWorkspaceSpaceRoots(configuredRoots)} disabled={workspaceSpaceState.scanning || configuredRoots.length === 0}>{workspaceSpaceState.scanning ? 'Scanning…' : 'Scan space'}</button></header>
  <form class="root-form" onsubmit={(event) => { event.preventDefault(); addExplicitRoot(); }}>
    <label>Explicit root <input bind:value={pathInput} placeholder="/path/to/worktree-or-cache" /></label>
    <label>Repository ID <input bind:value={repositoryInput} placeholder="repo" /></label>
    <label>Workspace ID <input bind:value={workspaceInput} placeholder="workspace" /></label>
    <label>Kind <select bind:value={kindInput}><option value="worktree">Worktree</option><option value="build-output">Build output</option><option value="dependency-cache">Dependency cache</option><option value="agent-data">Agent data</option><option value="other">Other</option></select></label>
    <label>Protection <select bind:value={protectionInput}><option value="active">Active</option><option value="dirty">Dirty</option><option value="unmerged">Unmerged</option><option value="locked">Locked</option><option value="user-data">User data</option><option value="safe-candidate">Safe candidate</option><option value="unknown">Unknown</option></select></label>
    <button type="submit" disabled={!pathInput.trim()}>Add explicit root</button>
  </form>
  {#if configuredRoots.length > 0}<ul class="root-list">{#each configuredRoots as root (root.path)}<li><span>{root.path} · {root.kind} · {root.protection}</span><button type="button" onclick={() => removeExplicitRoot(root.path)}>Remove</button></li>{/each}</ul>{/if}
  {#if workspaceSpaceState.report}<div class="totals"><span>{formatBytes(workspaceSpaceState.report.scannedBytes)} scanned</span><span>{formatBytes(workspaceSpaceState.report.reclaimableBytes)} safe candidates</span>{#if workspaceSpaceState.report.truncated}<span>Scan capped</span>{/if}</div>{#if lastReceipt}<p class="receipt">{lastReceipt}</p>{/if}<div class="cards">{#each workspaceSpaceState.report.entries as entry (entry.id)}<article><div class="card-heading"><strong>{entry.path}</strong><span>{diskProtectionLabel(entry.protection)}</span></div><p>{formatBytes(entry.bytes)} total · {formatBytes(reclaimableBytes(entry))} reclaimable</p><small>{entry.kind} · repository {entry.repositoryId} · workspace {entry.workspaceId}</small>{#if entry.protection === 'safe-candidate'}<button type="button" disabled={cleaningId !== null} onclick={() => void clean(entry)}>{cleaningId === entry.id ? 'Cleaning…' : 'Clean safe candidate'}</button>{/if}</article>{/each}</div>{:else}<p class="empty">{workspaceSpaceState.error ?? 'Start a visible scan to review explicit workspace roots.'}</p>{/if}
</section>

<style>
  .workspace { display: grid; gap: 1rem; padding: 1rem; color: var(--color-text, #eef0f9); } header { display: flex; justify-content: space-between; gap: 1rem; align-items: start; } h2, p { margin: 0; } .eyebrow { text-transform: uppercase; letter-spacing: 0.08em; font-size: 0.72rem; color: var(--color-muted, #a6a7b8); } .muted, small, .empty, .totals { color: var(--color-muted, #a6a7b8); } button { border: 1px solid var(--color-border, #858599); background: var(--color-surface, #17171d); color: inherit; border-radius: 6px; padding: 0.45rem 0.65rem; font: inherit; cursor: pointer; } button:disabled { opacity: 0.55; cursor: not-allowed; } .root-form { display: grid; grid-template-columns: 2fr 1fr 1fr 1fr 1fr auto; gap: 0.5rem; align-items: end; } label { display: grid; gap: 0.25rem; color: var(--color-muted, #a6a7b8); font-size: 0.75rem; } input, select { min-width: 0; border: 1px solid var(--color-border, #858599); background: var(--color-surface, #17171d); color: inherit; border-radius: 5px; padding: 0.4rem; font: inherit; } .root-list { display: grid; gap: 0.35rem; margin: 0; padding: 0; list-style: none; color: var(--color-muted, #a6a7b8); font-size: 0.8rem; } .root-list li { display: flex; justify-content: space-between; gap: 0.5rem; } .totals { display: flex; gap: 1rem; flex-wrap: wrap; } .receipt { color: var(--color-success, #93e5b2); } .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr)); gap: 0.75rem; } article { border: 1px solid var(--color-border-subtle, #383844); border-radius: 8px; padding: 0.8rem; display: grid; gap: 0.4rem; } .card-heading { display: flex; justify-content: space-between; gap: 0.5rem; } .card-heading strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .card-heading span { color: var(--color-muted, #a6a7b8); white-space: nowrap; }
</style>
