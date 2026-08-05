<script lang="ts">
  import { onMount } from 'svelte';

  import AgentActivityPane from './AgentActivityPane.svelte';
  import AgentHierarchy from './AgentHierarchy.svelte';
  import AgentRuntimeInspector from './AgentRuntimeInspector.svelte';
  import WorkflowGraph from './WorkflowGraph.svelte';
  import WorkflowLaneBoard from './WorkflowLaneBoard.svelte';
  import WorkflowNodeInspector from './WorkflowNodeInspector.svelte';
  import WorkflowRunHeader from './WorkflowRunHeader.svelte';
  import WorkflowRunList from './WorkflowRunList.svelte';
  import WorkflowTemplateEditor from './WorkflowTemplateEditor.svelte';
  import WorkflowTimeline from './WorkflowTimeline.svelte';
  import {
    approveWorkflowGate,
    cancelWorkflowRun,
    createWorkflowRun,
    listWorkflowRuns,
    pauseWorkflowRun,
    resumeWorkflowRun,
    retryWorkflowNode,
    skipWorkflowNode,
    startWorkflowRun,
    submitWorkflowResult,
    subscribeWorkflowSnapshots
  } from '$lib/shell/workflows/workflowService';
  import {
    applyWorkflowSnapshots,
    beginWorkflowLoad,
    failWorkflowLoad,
    filteredWorkflowRuns,
    selectWorkflowNode,
    selectWorkflowRun,
    selectedWorkflowRun,
    setWorkflowFilter,
    setWorkflowQuery,
    upsertWorkflowSnapshot,
    workflowState
  } from '$lib/shell/workflows/workflowStore.svelte';
  import {
    toWorkflowRunView,
    type WorkflowAgentView,
    type WorkflowDefinitionV1,
    type WorkflowResultReceipt,
    type WorkflowRunRecord,
    type WorkflowRunView
  } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    onOpenConversation?(ownedId: string): void;
    onOpenWorktree?(ownedId: string): void;
    onPinToWorking?(ownedId: string): void;
    onCompareReviewers?(workflowRunId: string): void;
  }

  let { onOpenConversation, onOpenWorktree, onPinToWorking, onCompareReviewers }: Props = $props();

  let busyAction = $state<string | null>(null);
  let surface = $state<'run' | 'template'>('run');
  let selectedAgentOwnedId = $state<string | null>(null);
  let subscriptionStop: (() => void) | null = null;

  const listViews = $derived(filteredWorkflowRuns().map((run) => toWorkflowRunView(run)));
  const selectedRunView = $derived<WorkflowRunView | null>(
    selectedWorkflowRun() ? toWorkflowRunView(selectedWorkflowRun()!) : null
  );
  const selectedNodeView = $derived(
    selectedRunView && workflowState.selectedNodeId
      ? selectedRunView.nodes.find((node) => node.nodeId === workflowState.selectedNodeId) ?? null
      : null
  );
  const selectedAgent = $derived<WorkflowAgentView | null>(
    selectedAgentOwnedId
      ? selectedRunView?.nodes.find((node) => node.agent.ownedId === selectedAgentOwnedId)?.agent ?? null
      : selectedNodeView?.agent ?? null
  );
  const reviewerNodes = $derived(
    selectedRunView?.nodes.filter((node) => node.outputContract === 'ReviewReceipt') ?? []
  );

  function idempotencyKey(action: string, runId: string): string {
    const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    return `control-center:${action}:${runId}:${suffix}`;
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function refresh(): Promise<void> {
    beginWorkflowLoad();
    try {
      const snapshots = await listWorkflowRuns();
      applyWorkflowSnapshots(snapshots);
      if (!workflowState.selectedWorkflowRunId && snapshots && snapshots.length > 0) {
        selectWorkflowRun(snapshots[0].id);
      }
    } catch (error) {
      failWorkflowLoad(`Could not read workflow runs: ${describeError(error)}`);
    }
  }

  async function runControl(
    label: string,
    action: () => Promise<WorkflowRunRecord | null>
  ): Promise<void> {
    busyAction = label;
    workflowState.error = null;
    try {
      const snapshot = await action();
      if (!snapshot) {
        workflowState.error = 'This control is available in the desktop app.';
        return;
      }
      upsertWorkflowSnapshot(snapshot);
      selectWorkflowRun(snapshot.id);
    } catch (error) {
      workflowState.error = `${label} did not finish: ${describeError(error)}`;
    } finally {
      busyAction = null;
    }
  }

  function selectedRunId(): string | null {
    return workflowState.selectedWorkflowRunId;
  }

  function startSelected(): void {
    const runId = selectedRunId();
    if (!runId) return;
    void runControl('Start', () => startWorkflowRun({ runId, idempotencyKey: idempotencyKey('start', runId) }));
  }

  function pauseSelected(): void {
    const runId = selectedRunId();
    if (!runId) return;
    void runControl('Pause', () => pauseWorkflowRun({ runId, idempotencyKey: idempotencyKey('pause', runId) }));
  }

  function resumeSelected(): void {
    const runId = selectedRunId();
    if (!runId) return;
    void runControl('Resume', () => resumeWorkflowRun({ runId, idempotencyKey: idempotencyKey('resume', runId) }));
  }

  function cancelSelected(): void {
    const runId = selectedRunId();
    if (!runId) return;
    void runControl('Cancel', () => cancelWorkflowRun({ runId, idempotencyKey: idempotencyKey('cancel', runId) }));
  }

  function retrySelectedNode(): void {
    const runId = selectedRunId();
    const nodeId = workflowState.selectedNodeId;
    if (!runId || !nodeId) return;
    void runControl('Retry node', () => retryWorkflowNode({ runId, nodeId, idempotencyKey: idempotencyKey(`retry:${nodeId}`, runId) }));
  }

  function skipSelectedNode(): void {
    const runId = selectedRunId();
    const nodeId = workflowState.selectedNodeId;
    if (!runId || !nodeId) return;
    void runControl('Skip node', () => skipWorkflowNode({ runId, nodeId, idempotencyKey: idempotencyKey(`skip:${nodeId}`, runId) }));
  }

  function approveSelectedGate(approved: boolean): void {
    const runId = selectedRunId();
    const nodeId = workflowState.selectedNodeId;
    if (!runId || !nodeId) return;
    void runControl(approved ? 'Approve gate' : 'Reject gate', () => approveWorkflowGate({
      runId,
      nodeId,
      approval: { approved },
      idempotencyKey: idempotencyKey(`${approved ? 'approve' : 'reject'}:${nodeId}`, runId)
    }));
  }

  function submitSelectedResult(result: WorkflowResultReceipt): void {
    const runId = selectedRunId();
    const nodeId = workflowState.selectedNodeId;
    if (!runId || !nodeId) return;
    void runControl('Submit result', () => submitWorkflowResult({
      runId,
      nodeId,
      result,
      idempotencyKey: idempotencyKey(`result:${nodeId}`, runId)
    }));
  }

  function createFromTemplate(definition: WorkflowDefinitionV1, input: Record<string, unknown>, startAfterCreate: boolean): void {
    void createRun(definition, input, startAfterCreate);
  }

  async function createRun(
    definition: WorkflowDefinitionV1,
    input: Record<string, unknown>,
    startAfterCreate: boolean
  ): Promise<void> {
    busyAction = startAfterCreate ? 'Create and start' : 'Create draft';
    workflowState.error = null;
    try {
      const snapshot = await createWorkflowRun({
        definition,
        input,
        idempotencyKey: idempotencyKey('create', definition.id)
      });
      if (!snapshot) {
        workflowState.error = 'Creating a workflow is available in the desktop app.';
        return;
      }
      upsertWorkflowSnapshot(snapshot);
      selectWorkflowRun(snapshot.id);
      surface = 'run';
      if (startAfterCreate) {
        const started = await startWorkflowRun({
          runId: snapshot.id,
          idempotencyKey: idempotencyKey('start', snapshot.id)
        });
        if (!started) {
          workflowState.error = 'The draft was created, but starting it is available in the desktop app.';
          return;
        }
        upsertWorkflowSnapshot(started);
      }
    } catch (error) {
      workflowState.error = `Workflow creation did not finish: ${describeError(error)}`;
    } finally {
      busyAction = null;
    }
  }

  function selectAgent(ownedId: string): void {
    selectedAgentOwnedId = ownedId;
    const node = selectedRunView?.nodes.find((candidate) => candidate.agent.ownedId === ownedId);
    if (node) selectWorkflowNode(node.nodeId);
  }

  function openWorkflow(workflowRunId: string): void {
    selectWorkflowRun(workflowRunId);
    surface = 'run';
  }

  onMount(() => {
    let mounted = true;
    void refresh();
    void subscribeWorkflowSnapshots((snapshot) => {
      if (!mounted) return;
      if (Array.isArray(snapshot)) applyWorkflowSnapshots(snapshot);
      else upsertWorkflowSnapshot(snapshot);
    }).then((stop) => {
      if (mounted) subscriptionStop = stop;
      else stop();
    });
    return () => {
      mounted = false;
      subscriptionStop?.();
      subscriptionStop = null;
    };
  });
</script>

<section data-testid="workflow-control-center" class="control-center" aria-label="Agent Control Center">
  {#if workflowState.error}
    <div data-testid="workflow-control-center-error" class="error-banner" role="status"><span data-testid="workflow-control-center-error-copy">{workflowState.error}</span><button data-testid="workflow-control-center-error-dismiss" type="button" onclick={() => (workflowState.error = null)}>Dismiss</button></div>
  {/if}

  <div data-testid="workflow-control-center-workspace" class="workspace">
    <WorkflowRunList
      runs={listViews}
      selectedWorkflowRunId={workflowState.selectedWorkflowRunId}
      query={workflowState.query}
      filter={workflowState.filter}
      loading={workflowState.loading}
      onSelect={(workflowRunId) => { selectWorkflowRun(workflowRunId); surface = 'run'; }}
      onQueryChange={setWorkflowQuery}
      onFilterChange={setWorkflowFilter}
      onRefresh={() => void refresh()}
      onNewWorkflow={() => { selectWorkflowRun(null); surface = 'template'; }}
    />

    <main data-testid="workflow-control-center-main" class="main">
      {#if surface === 'template' || !selectedRunView}
        <WorkflowTemplateEditor
          definition={selectedRunView?.definition ?? null}
          input={(selectedRunView?.inputSnapshot as Record<string, unknown>) ?? {}}
          compact={false}
          onCreate={createFromTemplate}
        />
      {:else}
        <WorkflowRunHeader
          run={selectedRunView}
          busy={busyAction !== null}
          onStart={startSelected}
          onPause={pauseSelected}
          onResume={resumeSelected}
          onCancel={cancelSelected}
          onSaveAsTemplate={() => (surface = 'template')}
        />
        <div data-testid="workflow-control-center-overview" class="overview">
          <WorkflowGraph run={selectedRunView} selectedNodeId={workflowState.selectedNodeId} onSelectNode={(nodeId) => { selectWorkflowNode(nodeId); selectedAgentOwnedId = null; }} />
          <WorkflowLaneBoard run={selectedRunView} selectedNodeId={workflowState.selectedNodeId} onSelectNode={(nodeId) => { selectWorkflowNode(nodeId); selectedAgentOwnedId = null; }} />
        </div>
        <div data-testid="workflow-control-center-activity" class="activity-grid">
          <AgentHierarchy
            run={selectedRunView}
            selectedOwnedId={selectedAgent?.ownedId}
            onSelect={selectAgent}
            onOpenConversation={onOpenConversation}
            onOpenWorktree={onOpenWorktree}
            onPinToWorking={onPinToWorking}
          />
          <WorkflowTimeline run={selectedRunView} selectedNodeId={workflowState.selectedNodeId} onSelectNode={(nodeId) => { selectWorkflowNode(nodeId); selectedAgentOwnedId = null; }} />
        </div>
        <div data-testid="workflow-control-center-inspectors" class="inspector-grid">
          <WorkflowNodeInspector
            node={selectedNodeView}
            definition={selectedNodeView ? selectedRunView.definition.nodes.find((definition) => definition.id === selectedNodeView.nodeId) ?? null : null}
            reviewerNodes={reviewerNodes}
            busy={busyAction !== null}
            onRetry={retrySelectedNode}
            onSkip={skipSelectedNode}
            onApprove={approveSelectedGate}
            onSubmitResult={submitSelectedResult}
            onCompareReviewers={() => onCompareReviewers?.(selectedRunView.workflowRunId)}
          />
          <AgentRuntimeInspector
            agent={selectedAgent}
            node={selectedNodeView}
            usage={{ tokensUsed: selectedRunView.tokensUsed, toolTerminalsUsed: selectedRunView.toolTerminalsUsed }}
            onOpenConversation={onOpenConversation}
            onOpenWorktree={onOpenWorktree}
            onPinToWorking={onPinToWorking}
          />
        </div>
      {/if}
    </main>

    <AgentActivityPane
      runs={listViews}
      onOpenWorkflow={openWorkflow}
      onOpenConversation={onOpenConversation}
      onPinToWorking={onPinToWorking}
    />
  </div>
</section>

<style>
  .control-center { display: flex; flex-direction: column; width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; background: var(--color-bg); color: var(--color-text); }
  .error-banner { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 7px 10px; border-bottom: 1px solid var(--color-bad); color: var(--color-bad); background: var(--color-bad-bg); font-size: 13px; }
  .error-banner button { min-height: 25px; padding: 0 6px; border: 1px solid var(--color-bad); border-radius: 4px; color: var(--color-bad); background: transparent; font-size: 12px; cursor: pointer; }
  .workspace { display: grid; grid-template-columns: minmax(220px, 255px) minmax(0, 1fr) minmax(210px, 260px); flex: 1; min-height: 0; }
  .main { display: flex; flex-direction: column; min-width: 0; min-height: 0; overflow: auto; }
  .overview, .activity-grid, .inspector-grid { display: grid; gap: 10px; padding: 10px 14px 0; }
  .overview { grid-template-columns: minmax(0, 1.15fr) minmax(0, .85fr); }
  .activity-grid, .inspector-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .inspector-grid { padding-bottom: 14px; }
  @media (max-width: 1100px) {
    .workspace { grid-template-columns: minmax(205px, 235px) minmax(0, 1fr); }
    .workspace > :global(.pane) { display: none; }
  }
  @media (max-width: 760px) {
    .workspace { grid-template-columns: 1fr; }
    .workspace > :global(.run-list) { height: auto; max-height: 280px; }
    .overview, .activity-grid, .inspector-grid { grid-template-columns: 1fr; }
  }
</style>
