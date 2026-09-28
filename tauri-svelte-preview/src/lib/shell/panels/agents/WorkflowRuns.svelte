<script lang="ts">
  import { untrack } from 'svelte';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import Check from '@lucide/svelte/icons/check';
  import CircleStop from '@lucide/svelte/icons/circle-stop';
  import Copy from '@lucide/svelte/icons/copy';
  import Pause from '@lucide/svelte/icons/pause';
  import Play from '@lucide/svelte/icons/play';
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Save from '@lucide/svelte/icons/save';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump.ts';
  import {
    listAgentConversationSessionsFromTauri,
    readAssemblySettingFromTauri,
    writeAssemblySettingFromTauri
  } from '$lib/tauriSource';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions.ts';
  import { addOwnedSession, rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
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
    savedWorkflowDefinition,
    taskWorkflowDefinition,
    type SavedWorkflowStage,
    type SavedWorkflowTemplate,
    type TaskWorkflowProviders,
    type WorkflowProvider
  } from '$lib/shell/workflows/taskWorkflowTemplate.ts';
  import type {
    WorkflowNodeRunRecord,
    WorkflowNodeState,
    WorkflowOutputContract,
    WorkflowRunRecord
  } from '$lib/shell/workflows/workflowTypes.ts';

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

  const TEMPLATES_SETTING_KEY = 'agents.workflow-templates';
  // Stages a saved workflow can add. No Plan stage: saved workflows start from an approved plan.
  const STAGE_PRESETS: { label: string; contract: WorkflowOutputContract; provider: WorkflowProvider; instructions: string }[] = [
    { label: 'Implement', contract: 'ImplementationReceipt', provider: 'codex', instructions: 'Implement the approved plan, keep the diff scoped, and verify the changed behavior.' },
    { label: 'Review', contract: 'ReviewReceipt', provider: 'claude', instructions: 'Review the implementation against the approved plan. Report one blocking finding, or a non-blocking no-finding receipt.' },
    { label: 'Spec check', contract: 'ReviewReceipt', provider: 'claude', instructions: 'Independently check the implementation against each requirement in the approved plan. Return one blocking finding for any requirement that is missing or wrong, or a non-blocking no-finding receipt.' },
    { label: 'Verify', contract: 'VerificationReceipt', provider: 'claude', instructions: 'Run relevant regression, integration, and UI checks. Browser checks must be headless at 1710x990 with the viewport verified after launch; save before and after screenshots to ~/Workbox/screenshots/ and stop the browser process tree. For native Assembly checks, use workbox-native-ui. Report commands, exits, evidence, and cleanup. Set evidenceArtifacts to [] when there is no saved artifact; otherwise each entry must contain id, kind, path, url, and digest.' },
    { label: 'Open PR', contract: 'PullRequestReceipt', provider: 'codex', instructions: 'Commit any verified uncommitted changes, push the branch, then open or update its pull request with before and after UI screenshots when relevant. For Notion tasks use a tsk-<id> branch. End every commit message body with Committed-by: <actual committer>; never add a co-author trailer. Do not merge. Return the PR URL and branch.' },
    { label: 'PR review', contract: 'ReviewReceipt', provider: 'claude', instructions: 'Independently review the opened pull request diff, check results, and before and after UI evidence against the approved plan. Return one blocking finding if the PR is not ready to merge, or a non-blocking no-finding receipt.' },
    { label: 'Merge PR', contract: 'PullRequestMergeReceipt', provider: 'codex', instructions: 'After a passing PR review, merge the PR from the Open PR receipt, verify its merged state and merge commit SHA on GitHub, and return both. If the merge fails, report the failure instead of a success receipt.' }
  ];
  const RECEIPTS: [WorkflowOutputContract, string][] = [
    ['ImplementationReceipt', 'Implementation'],
    ['ReviewReceipt', 'Review'],
    ['SpecComplianceReceipt', 'Spec report (never blocks)'],
    ['VerificationReceipt', 'Verification'],
    ['PullRequestReceipt', 'Pull request'],
    ['PullRequestMergeReceipt', 'Merge']
  ];

  let templates = $state<SavedWorkflowTemplate[]>([]);
  // null runs the built-in planning workflow; otherwise the saved workflow being edited.
  let draft = $state<SavedWorkflowTemplate | null>(null);
  const draftError = $derived.by(() => {
    if (!draft) return null;
    try {
      savedWorkflowDefinition(draft);
      return null;
    } catch (error) {
      return message(error);
    }
  });

  const runs = $derived(filteredWorkflowRuns());
  const selected = $derived(selectedWorkflowRun());
  const orderedNodes = $derived(selected ? orderNodes(selected) : []);
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
    readAssemblySettingFromTauri(TEMPLATES_SETTING_KEY)
      .then((value) => {
        if (!controller.signal.aborted && Array.isArray(value)) templates = value as SavedWorkflowTemplate[];
      })
      .catch((error) => {
        if (!controller.signal.aborted) workflowState.error = `Could not load saved workflows: ${message(error)}`;
      });
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
    if (!trimmed || !root || draftError) return;
    if (draft && !(await saveTemplate())) return;
    await control('Starting workflow', async () => {
      const created = await createWorkflowRun({
        definition: draft ? savedWorkflowDefinition(draft) : taskWorkflowDefinition(providers),
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

  function newId(prefix: string): string {
    return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
  }

  function stageFrom(label: string, earlier: SavedWorkflowStage[]): SavedWorkflowStage {
    const preset = STAGE_PRESETS.find((item) => item.label === label) ?? STAGE_PRESETS[0];
    return {
      id: newId('stage'),
      title: preset.label,
      instructions: preset.instructions,
      provider: preset.provider,
      outputContract: preset.contract,
      redoStageId: preset.contract === 'ReviewReceipt'
        ? earlier.findLast((item) => item.outputContract === 'ImplementationReceipt')?.id ?? null
        : null,
      approvalPrompt: null
    };
  }

  function chooseWorkflow(id: string): void {
    if (id === 'built-in') {
      draft = null;
    } else if (id === 'new') {
      const stages: SavedWorkflowStage[] = [];
      for (const label of ['Implement', 'Review', 'Verify', 'Open PR', 'PR review']) {
        stages.push(stageFrom(label, stages));
      }
      draft = { id: newId('saved'), name: '', stages };
    } else {
      const template = templates.find((item) => item.id === id);
      if (template) draft = $state.snapshot(template);
    }
  }

  function duplicateTemplate(): void {
    if (!draft) return;
    draft = { ...$state.snapshot(draft), id: newId('saved'), name: `${draft.name} copy` };
  }

  function addStage(label: string): void {
    if (draft) draft.stages.push(stageFrom(label, draft.stages));
  }

  function moveStage(index: number, offset: number): void {
    if (!draft) return;
    const [stage] = draft.stages.splice(index, 1);
    draft.stages.splice(index + offset, 0, stage);
  }

  async function saveTemplate(): Promise<boolean> {
    if (!draft || draftError) return false;
    const saved = $state.snapshot(draft);
    const next = templates.some((item) => item.id === saved.id)
      ? templates.map((item) => (item.id === saved.id ? saved : item))
      : [...templates, saved];
    try {
      await writeAssemblySettingFromTauri(TEMPLATES_SETTING_KEY, next);
      templates = next;
      return true;
    } catch (error) {
      workflowState.error = `Could not save workflow: ${message(error)}`;
      return false;
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
      Scripted agent stages. New task pauses for approval after spec, plan, and build; saved workflows run straight through.
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
      <div class="flex items-end gap-2">
        <label class="flex min-w-0 flex-1 flex-col gap-1 text-xs text-muted-foreground">
          Workflow
          <select
            value={draft?.id ?? 'built-in'}
            onchange={(event) => chooseWorkflow(event.currentTarget.value)}
            class="h-8 min-w-0 rounded-lg border border-input bg-background px-2 text-sm text-foreground outline-none focus-visible:border-ring"
          >
            <option value="built-in">New task (spec and plan first)</option>
            {#each templates as template (template.id)}
              <option value={template.id}>{template.name}</option>
            {/each}
            {#if draft && !templates.some((item) => item.id === draft?.id)}
              <option value={draft.id}>{draft.name.trim() || 'Unsaved workflow'}</option>
            {/if}
            <option value="new">New saved workflow…</option>
          </select>
        </label>
        {#if draft}
          <Button size="sm" variant="outline" type="button" disabled={draftError !== null} onclick={() => void saveTemplate()}><Save />Save</Button>
          <Button size="sm" variant="outline" type="button" onclick={duplicateTemplate}><Copy />Duplicate</Button>
        {/if}
      </div>
      <label class="flex flex-col gap-1.5 text-sm font-medium">
        {draft ? 'Approved plan' : 'Task'}
        <textarea
          bind:value={task}
          rows="4"
          placeholder={draft ? 'Paste the reviewed plan and its ordered steps…' : 'Describe the outcome for the agents…'}
          class="min-h-24 resize-y rounded-xl border border-input bg-background px-3 py-2 text-sm font-normal outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
        ></textarea>
      </label>
      {#if draft}
        <input
          bind:value={draft.name}
          placeholder="Workflow name"
          aria-label="Workflow name"
          class="h-8 rounded-lg border border-input bg-background px-2 text-sm outline-none focus-visible:border-ring"
        />
        <ol class="flex flex-col gap-2">
          {#each draft.stages as stage, index (stage.id)}
            <li class="flex flex-col gap-1.5 rounded-lg border border-border bg-background p-2">
              <div class="flex items-center gap-1">
                <span class="flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold">{index + 1}</span>
                <input
                  bind:value={stage.title}
                  aria-label="Stage title"
                  class="h-7 min-w-0 flex-1 rounded-md bg-transparent px-1.5 text-sm font-medium outline-none focus-visible:bg-muted"
                />
                <Button size="icon-xs" variant="ghost" type="button" aria-label="Move up" disabled={index === 0} onclick={() => moveStage(index, -1)}><ArrowUp /></Button>
                <Button size="icon-xs" variant="ghost" type="button" aria-label="Move down" disabled={index === draft.stages.length - 1} onclick={() => moveStage(index, 1)}><ArrowDown /></Button>
                <Button size="icon-xs" variant="ghost" type="button" aria-label="Remove stage" onclick={() => draft?.stages.splice(index, 1)}><X /></Button>
              </div>
              <textarea
                bind:value={stage.instructions}
                rows="2"
                aria-label="Stage instructions"
                class="resize-y rounded-md border border-input bg-background px-2 py-1 text-xs outline-none focus-visible:border-ring"
              ></textarea>
              <div class="grid grid-cols-3 gap-2">
                <label class="flex min-w-0 flex-col gap-1 text-xs text-muted-foreground">
                  Agent
                  <select bind:value={stage.provider} class="h-7 min-w-0 rounded-md border border-input bg-background px-1.5 text-xs text-foreground outline-none focus-visible:border-ring">
                    <option value="codex">Codex</option>
                    <option value="claude">Claude</option>
                    <option value="antigravity">Agy</option>
                  </select>
                </label>
                <label class="flex min-w-0 flex-col gap-1 text-xs text-muted-foreground">
                  Receipt
                  <select
                    value={stage.outputContract}
                    onchange={(event) => {
                      stage.outputContract = event.currentTarget.value as WorkflowOutputContract;
                      if (stage.outputContract !== 'ReviewReceipt') stage.redoStageId = null;
                    }}
                    class="h-7 min-w-0 rounded-md border border-input bg-background px-1.5 text-xs text-foreground outline-none focus-visible:border-ring">
                    {#each RECEIPTS as [contract, label]}
                      <option value={contract}>{label}</option>
                    {/each}
                  </select>
                </label>
                <label class="flex min-w-0 flex-col gap-1 text-xs text-muted-foreground">
                  On finding, redo
                  <select
                    bind:value={stage.redoStageId}
                    disabled={stage.outputContract !== 'ReviewReceipt'}
                    class="h-7 min-w-0 rounded-md border border-input bg-background px-1.5 text-xs text-foreground outline-none focus-visible:border-ring disabled:opacity-50"
                  >
                    <option value={null}>None</option>
                    {#each draft.stages.slice(0, index).filter((item) => item.outputContract === 'ImplementationReceipt') as target (target.id)}
                      <option value={target.id}>{target.title}</option>
                    {/each}
                  </select>
                </label>
              </div>
            </li>
          {/each}
        </ol>
        <select
          value=""
          aria-label="Add stage"
          onchange={(event) => { addStage(event.currentTarget.value); event.currentTarget.value = ''; }}
          class="h-8 rounded-lg border border-dashed border-input bg-background px-2 text-sm text-muted-foreground outline-none focus-visible:border-ring"
        >
          <option value="" disabled>Add stage…</option>
          {#each STAGE_PRESETS as preset (preset.label)}
            <option value={preset.label}>{preset.label}</option>
          {/each}
        </select>
        {#if draft.stages.some((stage, index, all) => stage.outputContract === 'ReviewReceipt' && all.slice(0, index).some((item) => item.outputContract === 'PullRequestReceipt')) && !draft.stages.some((stage) => stage.outputContract === 'PullRequestMergeReceipt')}
          <p class="text-xs text-muted-foreground">Without a Merge PR stage, the run ends after PR review for owner handoff.</p>
        {/if}
        {#if draftError}
          <p class="flex items-start gap-2 rounded-lg bg-destructive/10 px-2.5 py-2 text-xs text-destructive">
            <TriangleAlert class="mt-0.5 size-3.5 shrink-0" />
            {draftError}
          </p>
        {/if}
      {:else}
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
      {/if}
      <Button type="submit" disabled={!task.trim() || !root || busy !== null || draftError !== null}>
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
