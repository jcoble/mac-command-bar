<script lang="ts">
  import WorkingSpinner from '$lib/shell/components/conversation/WorkingSpinner.svelte';
  import { untrack } from 'svelte';
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
  import { sessionContextUsage } from '$lib/shell/panels/context/sessionContextModel.ts';
  import { listAgentConversationSessionsFromTauri } from '$lib/tauriSource';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions.ts';
  import { addOwnedSession, rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import {
    answerWorkflowPause,
    approveWorkflowGate,
    cancelWorkflowRun,
    createWorkflowRun,
    listWorkflowFiles,
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
  import type {
    WorkflowNodeRunRecord,
    WorkflowNodeState,
    WorkflowRunRecord
  } from '$lib/shell/workflows/workflowTypes.ts';

  interface Props {
    visible: boolean;
    root: string;
  }

  let { visible, root }: Props = $props();
  let composing = $state(false);
  let taskKey = $state('');
  let taskTitle = $state('');
  let taskBody = $state('');
  let workflowFiles = $state<string[]>([]);
  let workflowPath = $state('');
  let busy = $state<string | null>(null);
  let loadGeneration = 0;
  let actionController: AbortController | null = null;
  let now = $state(Date.now());

  const runs = $derived(filteredWorkflowRuns());
  const selected = $derived(selectedWorkflowRun());
  const orderedNodes = $derived(selected ? orderNodes(selected) : []);
  const waitingNode = $derived(selected?.nodes.find((node) => node.state === 'waiting-approval') ?? null);
  const pausedNode = $derived(selected?.nodes.find((node) => node.state === 'waiting-input') ?? null);
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

  function orderNodes(run: WorkflowRunRecord): WorkflowNodeRunRecord[] {
    const byId = new Map(run.nodes.map((node) => [node.nodeId, node]));
    const ordered: WorkflowNodeRunRecord[] = [];
    const visited = new Set<string>();
    function visit(id: string): void {
      if (visited.has(id)) return;
      visited.add(id);
      const node = byId.get(id);
      if (!node) return;
      run.definition.nodes.find((item) => item.id === id)?.dependsOn.forEach(visit);
      ordered.push(node);
    }
    run.definition.nodes.forEach((node) => visit(node.id));
    return ordered;
  }

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
    const stop = untrack(() => subscribeWorkflowSnapshots((snapshot) => {
      if (controller.signal.aborted || generation !== loadGeneration) return;
      if (Array.isArray(snapshot)) applyWorkflowSnapshots(snapshot);
      else upsertWorkflowSnapshot(snapshot);
    }));
    void refresh(controller.signal, generation);
    void loadWorkflowFiles(root, controller.signal);
    return () => {
      controller.abort();
      actionController?.abort();
      actionController = null;
      stop();
      loadGeneration += 1;
      resetWorkflowStore();
    };
  });

  async function loadWorkflowFiles(repoRoot: string, signal: AbortSignal): Promise<void> {
    if (!repoRoot) return;
    try {
      const files = await listWorkflowFiles(repoRoot);
      if (signal.aborted || !files) return;
      workflowFiles = files;
      if (!files.includes(workflowPath)) workflowPath = files[0] ?? '';
    } catch (error) {
      if (!signal.aborted) workflowState.error = `Could not list workflows: ${message(error)}`;
    }
  }

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

  const LIVE_STATES: WorkflowNodeState[] = ['starting', 'running'];
  const ticking = $derived(
    orderedNodes.some((node) => node.startedAtMs && !node.finishedAtMs && LIVE_STATES.includes(node.state))
  );

  // One clock for every running stage's elapsed time; stops when hidden or nothing is running.
  $effect(() => {
    if (!visible || !ticking) return;
    now = Date.now();
    const timer = window.setInterval(() => (now = Date.now()), 1000);
    return () => window.clearInterval(timer);
  });

  function elapsed(node: WorkflowNodeRunRecord): string {
    if (!node.startedAtMs) return '';
    const end = node.finishedAtMs ?? (LIVE_STATES.includes(node.state) ? now : null);
    if (end === null) return '';
    const seconds = Math.max(0, Math.floor((end - node.startedAtMs) / 1000));
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ${String(seconds % 60).padStart(2, '0')}s`;
    return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`;
  }

  const compactTokens = new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 });

  function usageCompact(node: WorkflowNodeRunRecord): string {
    if (!node.usage) return 'Usage not reported';
    const usage = sessionContextUsage(null, node.usage);
    if (usage.usedTokens !== null && usage.contextWindow !== null) {
      return `Ctx ${compactTokens.format(usage.usedTokens)}/${compactTokens.format(usage.contextWindow)}`;
    }
    if (usage.totalTokens !== null) return `Total ${compactTokens.format(usage.totalTokens)} tokens`;
    if (usage.usedTokens !== null) return `Ctx ${compactTokens.format(usage.usedTokens)} tokens`;
    return 'Usage not reported';
  }

  function usageSummary(node: WorkflowNodeRunRecord): string {
    if (!node.usage) return 'Usage and cost not reported';
    const usage = sessionContextUsage(null, node.usage);
    const parts: string[] = [];
    if (usage.usedTokens !== null) {
      const used = usage.usedTokens.toLocaleString();
      parts.push(usage.contextWindow === null
        ? `Context ${used} tokens`
        : `Context ${used} / ${usage.contextWindow.toLocaleString()}`);
    }
    if (usage.totalTokens !== null) parts.push(`Total ${usage.totalTokens.toLocaleString()} tokens`);
    if (usage.inputTokens !== null && usage.outputTokens !== null) {
      parts.push(`Reported ${usage.inputTokens.toLocaleString()} in, ${usage.outputTokens.toLocaleString()} out`);
    }
    parts.push('Cost not reported');
    return parts.join(' · ');
  }

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
    const task = { key: taskKey.trim(), title: taskTitle.trim(), body: taskBody.trim() };
    if (!task.key || !task.title || !root || !workflowPath) return;
    await control('Starting workflow', async () => {
      const created = await createWorkflowRun({
        repoRoot: root,
        workflowPath,
        task,
        input: {},
        idempotencyKey: key('create')
      });
      if (!created) return null;
      const started = await startWorkflowRun({ runId: created.id, idempotencyKey: key('start') });
      return started ?? created;
    });
    if (!workflowState.error) {
      taskKey = '';
      taskTitle = '';
      taskBody = '';
      composing = false;
    }
  }

  async function openStage(ownedId: string | null): Promise<void> {
    if (!ownedId) return;
    try {
      if (!rail.owned.some((session) => session.ownedId === ownedId)) {
        const record = (await listAgentConversationSessionsFromTauri())?.find((session) => session.ownedId === ownedId);
        if (!record) throw new Error('Agent conversation is not available yet.');
        if (!rail.owned.some((session) => session.ownedId === ownedId)) addOwnedSession(ownedSessionFromBackend(record));
      }
      await sessionRowJump(ownedId, 'session');
    } catch (error) {
      workflowState.error = `Could not open agent: ${message(error)}`;
    }
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
    <p class="min-w-0 text-sm text-muted-foreground">
      Runs of this project's workflows from <code>.assembly/workflows/</code>.
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
    <form class="flex max-h-[60%] flex-col gap-3 overflow-y-auto border-b border-border bg-card p-3" onsubmit={(event) => { event.preventDefault(); void createAndStart(); }}>
      {#if workflowFiles.length === 0}
        <p class="text-sm text-muted-foreground">This project has no workflows yet. Ask an agent to write one in <code>.assembly/workflows/</code>.</p>
      {:else}
        <label class="flex min-w-0 flex-col gap-1 text-xs text-muted-foreground">
          Workflow
          <select bind:value={workflowPath} class="h-8 rounded-lg border border-input bg-background px-2 text-sm outline-none focus-visible:border-ring min-w-0 text-foreground">
            {#each workflowFiles as file (file)}
              <option value={file}>{file.replace('.assembly/workflows/', '')}</option>
            {/each}
          </select>
        </label>
        <div class="grid grid-cols-[8rem_1fr] gap-2">
          <input bind:value={taskKey} placeholder="TSK-123" aria-label="Task key" class="h-8 min-w-0 rounded-lg border border-input bg-background px-2 text-sm outline-none focus-visible:border-ring" />
          <input bind:value={taskTitle} placeholder="Task title" aria-label="Task title" class="h-8 min-w-0 rounded-lg border border-input bg-background px-2 text-sm outline-none focus-visible:border-ring" />
        </div>
        <textarea
          bind:value={taskBody}
          rows="4"
          placeholder="Task description for the agents…"
          aria-label="Task description"
          class="min-h-24 resize-y rounded-xl border border-input bg-background px-3 py-2 text-sm font-normal outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
        ></textarea>
        <Button type="submit" disabled={!taskKey.trim() || !taskTitle.trim() || !root || !workflowPath || busy !== null}>
          <Play />
          Start workflow
        </Button>
      {/if}
    </form>
  {/if}

  {#if workflowState.loading && runs.length === 0}
    <div class="p-4 text-sm text-muted-foreground"><WorkingSpinner size={14} /> Loading workflows…</div>
  {:else if runs.length === 0}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <p class="font-medium">No workflows yet</p>
      <p class="max-w-72 text-sm text-muted-foreground">Start a run from one of this project's workflow files.</p>
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
              <span class="min-w-0 flex-1 truncate text-sm font-semibold">{(run.inputSnapshot as { task?: { title?: string } }).task?.title ?? run.definition.name}</span>
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
              {#each orderedNodes as node, index (node.id)}
                <li>
                  <button
                    type="button"
                    class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-muted"
                    onclick={() => void openStage(node.ownedId)}
                  >
                    <span class="flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold">{index + 1}</span>
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-sm font-medium">{selected.definition.nodes.find((item) => item.id === node.nodeId)?.title ?? node.nodeId}</span>
                      <span class="flex min-w-0 gap-1 text-xs text-muted-foreground">
                        <span class="truncate">{node.provider} · {node.roleId}</span>
                        {#if elapsed(node)}<span class="shrink-0 tabular-nums">· {elapsed(node)}</span>{/if}
                      </span>
                      {#if node.startedAtMs}
                        <span class="block truncate text-xs text-muted-foreground" title={usageSummary(node)}>{usageCompact(node)}</span>
                      {/if}
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
              {#if pausedNode}
                <Button size="sm" disabled={busy !== null} onclick={() => void control('Another round', () => answerWorkflowPause({ runId: selected.id, nodeId: pausedNode.nodeId, decision: 'another-round', reply: null, idempotencyKey: key('answer') }))}><RotateCcw />Run another round</Button>
                <Button size="sm" variant="outline" disabled={busy !== null} onclick={() => void control('Accept', () => answerWorkflowPause({ runId: selected.id, nodeId: pausedNode.nodeId, decision: 'accept', reply: null, idempotencyKey: key('answer') }))}><Check />Accept as is</Button>
              {/if}
              {#if failedNode}
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
