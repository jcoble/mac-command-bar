<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import CircleStop from '@lucide/svelte/icons/circle-stop';
  import Pause from '@lucide/svelte/icons/pause';
  import Play from '@lucide/svelte/icons/play';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump.ts';
  import {
    approveWorkflowGate,
    cancelWorkflowRun,
    createWorkflowRun,
    listWorkflowRuns,
    pauseWorkflowRun,
    redirectWorkflowNode,
    resumeWorkflowRun,
    retryWorkflowNode,
    startWorkflowRun,
    subscribeWorkflowSnapshots
  } from '$lib/shell/workflows/workflowService.ts';
  import {
    applyWorkflowSnapshots,
    beginWorkflowLoad,
    failWorkflowLoad,
    filteredWorkflowRuns,
    resetWorkflowStore,
    selectWorkflowRun,
    selectedWorkflowRun,
    upsertWorkflowSnapshot,
    workflowState
  } from '$lib/shell/workflows/workflowStore.svelte.ts';
  import {
    taskWorkflowDefinition,
    type TaskWorkflowProviders,
    type WorkflowProvider
  } from '$lib/shell/workflows/taskWorkflowTemplate.ts';
  import type { WorkflowNodeState, WorkflowRunRecord } from '$lib/shell/workflows/workflowTypes.ts';

  interface Props {
    visible: boolean;
    root: string;
  }

  let { visible, root }: Props = $props();
  let composing = $state(false);
  let task = $state('');
  let providers = $state<TaskWorkflowProviders>({
    plan: 'claude',
    implement: 'codex',
    review: 'claude'
  });
  let busy = $state<string | null>(null);
  let loadGeneration = 0;
  let actionController: AbortController | null = null;
  let now = $state(Date.now());

  const STAGES = [
    ['plan', 'Plan'],
    ['implement', 'Build'],
    ['review', 'Review']
  ] as const;

  const runs = $derived(filteredWorkflowRuns());
  const selected = $derived(selectedWorkflowRun());
  const waitingNode = $derived(selected?.nodes.find((node) => node.state === 'waiting-approval') ?? null);
  const failedNode = $derived(selected?.nodes.find((node) => node.state === 'failed') ?? null);
  const runningNode = $derived(selected?.nodes.find((node) => node.state === 'running') ?? null);
  const nextNode = $derived(
    waitingNode
      ? selected?.nodes.find((node) => node.state === 'blocked' || node.state === 'ready') ?? null
      : null
  );
  const runningDefinition = $derived(
    runningNode ? selected?.definition.nodes.find((node) => node.id === runningNode.nodeId) ?? null : null
  );
  const stale = $derived(
    Boolean(
      runningNode?.startedAtMs &&
      runningDefinition &&
      now - runningNode.startedAtMs >= runningDefinition.timeoutSeconds * 1000
    )
  );
  const completedCount = $derived(
    selected?.nodes.filter((node) => ['completed', 'skipped'].includes(node.state)).length ?? 0
  );

  const stateTone: Record<WorkflowNodeState, 'neutral' | 'live' | 'good' | 'bad' | 'attention'> = {
    blocked: 'neutral',
    ready: 'live',
    queued: 'live',
    starting: 'live',
    running: 'live',
    'waiting-approval': 'attention',
    'waiting-input': 'attention',
    completed: 'good',
    failed: 'bad',
    cancelled: 'neutral',
    skipped: 'neutral'
  };

  $effect(() => {
    if (!visible) return;
    const controller = new AbortController();
    const generation = ++loadGeneration;
    const stop = subscribeWorkflowSnapshots((snapshot) => {
      if (controller.signal.aborted || generation !== loadGeneration) return;
      if (Array.isArray(snapshot)) applyWorkflowSnapshots(snapshot);
      else upsertWorkflowSnapshot(snapshot);
    });
    void refresh(controller.signal, generation);
    return () => {
      controller.abort();
      actionController?.abort();
      actionController = null;
      stop();
      loadGeneration += 1;
      resetWorkflowStore();
    };
  });

  $effect(() => {
    if (!visible || !runningNode?.startedAtMs || !runningDefinition) return;
    const controller = new AbortController();
    const remaining = Math.max(
      0,
      runningNode.startedAtMs + runningDefinition.timeoutSeconds * 1000 - Date.now()
    );
    const timer = window.setTimeout(() => {
      if (!controller.signal.aborted) now = Date.now();
    }, remaining);
    return () => {
      controller.abort();
      window.clearTimeout(timer);
    };
  });

  function key(action: string): string {
    return `agents:${action}:${crypto.randomUUID()}`;
  }

  function message(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function refresh(signal: AbortSignal, generation = loadGeneration): Promise<void> {
    beginWorkflowLoad();
    try {
      const snapshots = await listWorkflowRuns();
      if (signal.aborted || generation !== loadGeneration) return;
      applyWorkflowSnapshots(snapshots);
      if (!workflowState.selectedWorkflowRunId && snapshots?.length) {
        selectWorkflowRun(snapshots.at(-1)?.id ?? null);
      }
    } catch (error) {
      if (!signal.aborted && generation === loadGeneration) {
        failWorkflowLoad(`Could not load workflows: ${message(error)}`);
      }
    }
  }

  async function control(
    label: string,
    action: () => Promise<WorkflowRunRecord | null>
  ): Promise<void> {
    actionController?.abort();
    const controller = new AbortController();
    actionController = controller;
    const generation = loadGeneration;
    busy = label;
    workflowState.error = null;
    try {
      const run = await action();
      if (controller.signal.aborted || generation !== loadGeneration || !run) return;
      upsertWorkflowSnapshot(run);
      selectWorkflowRun(run.id);
    } catch (error) {
      if (!controller.signal.aborted && generation === loadGeneration) {
        workflowState.error = `${label}: ${message(error)}`;
      }
    } finally {
      controller.abort();
      if (actionController === controller) actionController = null;
      if (generation === loadGeneration) busy = null;
    }
  }

  async function createAndStart(): Promise<void> {
    const trimmed = task.trim();
    if (!trimmed || !root) return;
    await control('Starting workflow', async () => {
      const created = await createWorkflowRun({
        definition: taskWorkflowDefinition(providers),
        input: { task: trimmed, cwd: root },
        idempotencyKey: key('create')
      });
      if (!created) return null;
      const started = await startWorkflowRun({ runId: created.id, idempotencyKey: key('start') });
      return started ?? created;
    });
    if (!workflowState.error) {
      task = '';
      composing = false;
    }
  }

  function setProvider(stage: keyof TaskWorkflowProviders, value: string): void {
    providers[stage] = value as WorkflowProvider;
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
    <p class="min-w-0 text-sm text-muted-foreground">
      Scripted stages with an approval at each handoff.
    </p>
    <Button size="sm" variant={composing ? 'secondary' : 'default'} onclick={() => (composing = !composing)}>
      <Plus />
      New
    </Button>
  </div>

  {#if workflowState.error}
    <div class="border-b border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
      {workflowState.error}
    </div>
  {/if}

  {#if composing}
    <form class="flex flex-col gap-3 border-b border-border bg-card p-3" onsubmit={(event) => { event.preventDefault(); void createAndStart(); }}>
      <label class="flex flex-col gap-1.5 text-sm font-medium">
        Task
        <textarea
          bind:value={task}
          rows="4"
          placeholder="Describe the outcome for the agents…"
          class="min-h-24 resize-y rounded-xl border border-input bg-background px-3 py-2 text-sm font-normal outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
        ></textarea>
      </label>
      <div class="grid grid-cols-3 gap-2">
        {#each STAGES as [stage, label]}
          <label class="flex min-w-0 flex-col gap-1 text-xs text-muted-foreground">
            {label}
            <select
              value={providers[stage]}
              onchange={(event) => setProvider(stage, event.currentTarget.value)}
              class="h-8 min-w-0 rounded-lg border border-input bg-background px-2 text-sm text-foreground outline-none focus-visible:border-ring"
            >
              <option value="codex">Codex</option>
              <option value="claude">Claude</option>
              <option value="antigravity">Agy</option>
            </select>
          </label>
        {/each}
      </div>
      <Button type="submit" disabled={!task.trim() || !root || busy !== null}>
        <Play />
        Start workflow
      </Button>
    </form>
  {/if}

  {#if workflowState.loading && runs.length === 0}
    <div class="p-4 text-sm text-muted-foreground">Loading workflows…</div>
  {:else if runs.length === 0}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <p class="font-medium">No workflows yet</p>
      <p class="max-w-72 text-sm text-muted-foreground">Start with one task. You will see each agent and approve every handoff.</p>
    </div>
  {:else}
    <ScrollArea class="min-h-0 flex-1">
      <div class="flex flex-col gap-2 p-2">
        {#each [...runs].reverse() as run (run.id)}
          <button
            type="button"
            class="flex min-w-0 flex-col gap-2 rounded-xl border px-3 py-2.5 text-left transition-colors {selected?.id === run.id ? 'border-primary/60 bg-primary/10' : 'border-border bg-card hover:bg-muted'}"
            onclick={() => selectWorkflowRun(run.id)}
          >
            <span class="flex w-full min-w-0 items-center gap-2">
              <span class="min-w-0 flex-1 truncate text-sm font-semibold">{(run.inputSnapshot as { task?: string }).task ?? run.definition.name}</span>
              <Chip tone={run.state === 'failed' ? 'bad' : run.state === 'completed' ? 'good' : run.state === 'waiting-approval' ? 'attention' : 'live'}>{run.state}</Chip>
            </span>
            <span class="text-xs text-muted-foreground">{run.phase} · {run.nodes.filter((node) => ['completed', 'skipped'].includes(node.state)).length}/{run.nodes.length} stages</span>
          </button>
        {/each}

        {#if selected}
          <section class="mt-1 rounded-2xl border border-border bg-card p-3">
            <div class="mb-3 flex items-center gap-2">
              <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
                <div
                  class="h-full rounded-full bg-primary transition-[width] duration-200"
                  style:width={`${(completedCount / selected.nodes.length) * 100}%`}
                ></div>
              </div>
              <span class="text-xs tabular-nums text-muted-foreground">{completedCount}/{selected.nodes.length}</span>
            </div>

            <ol class="flex flex-col gap-1.5">
              {#each selected.nodes as node, index (node.id)}
                <li>
                  <button
                    type="button"
                    class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-muted"
                    onclick={() => void sessionRowJump(node.ownedId, 'session')}
                  >
                    <span class="flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold">{index + 1}</span>
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-sm font-medium">{selected.definition.nodes.find((item) => item.id === node.nodeId)?.title ?? node.nodeId}</span>
                      <span class="block truncate text-xs text-muted-foreground">{node.provider} · {node.roleId}</span>
                    </span>
                    <Chip tone={stateTone[node.state]}>{node.state}</Chip>
                  </button>
                  {#if node.failure}
                    <p class="mx-2 rounded-lg bg-destructive/10 px-2 py-1.5 text-xs text-destructive">{node.failure.message}</p>
                  {/if}
                </li>
              {/each}
            </ol>

            {#if stale}
              <p class="mt-3 flex items-start gap-2 rounded-lg border border-border bg-muted px-2.5 py-2 text-xs text-foreground">
                <TriangleAlert class="mt-0.5 size-3.5 shrink-0" />
                This stage has passed its expected time. It is still running; review or stop it when ready.
              </p>
            {/if}

            <div class="mt-3 flex flex-wrap gap-2 border-t border-border pt-3">
              {#if selected.state === 'draft' || selected.state === 'queued'}
                <Button size="sm" disabled={busy !== null} onclick={() => void control('Start', () => startWorkflowRun({ runId: selected.id, idempotencyKey: key('start') }))}><Play />Start</Button>
              {:else if selected.state === 'paused'}
                <Button size="sm" disabled={busy !== null} onclick={() => void control('Resume', () => resumeWorkflowRun({ runId: selected.id, idempotencyKey: key('resume') }))}><Play />Resume</Button>
              {:else if selected.state === 'running'}
                <Button size="sm" variant="outline" disabled={busy !== null} onclick={() => void control('Pause', () => pauseWorkflowRun({ runId: selected.id, idempotencyKey: key('pause') }))}><Pause />Pause after stage</Button>
              {/if}
              {#if waitingNode}
                <Button size="sm" disabled={busy !== null} onclick={() => void control('Approve', () => approveWorkflowGate({ runId: selected.id, nodeId: waitingNode.nodeId, approval: { approved: true }, idempotencyKey: key('approve') }))}><Check />Approve handoff</Button>
                {#if nextNode}
                  <label class="flex h-8 items-center gap-2 rounded-lg border border-input bg-background px-2 text-xs text-muted-foreground">
                    Next
                    <select
                      value={nextNode.provider}
                      disabled={busy !== null}
                      onchange={(event) => void control('Redirect', () => redirectWorkflowNode({ runId: selected.id, nodeId: nextNode.nodeId, provider: event.currentTarget.value, idempotencyKey: key('redirect') }))}
                      class="bg-transparent text-sm text-foreground outline-none"
                    >
                      <option value="codex">Codex</option>
                      <option value="claude">Claude</option>
                      <option value="antigravity">Agy</option>
                    </select>
                  </label>
                {/if}
              {/if}
              {#if failedNode && failedNode.attempt < 2}
                <Button size="sm" variant="outline" disabled={busy !== null} onclick={() => void control('Retry', () => retryWorkflowNode({ runId: selected.id, nodeId: failedNode.nodeId, idempotencyKey: key('retry') }))}><RotateCcw />Retry stage</Button>
              {/if}
              {#if !['completed', 'cancelled', 'failed'].includes(selected.state)}
                <Button size="sm" variant="destructive" disabled={busy !== null} onclick={() => void control('Stop', () => cancelWorkflowRun({ runId: selected.id, idempotencyKey: key('cancel') }))}><CircleStop />Stop</Button>
              {/if}
            </div>
          </section>
        {/if}
      </div>
    </ScrollArea>
  {/if}
</div>
