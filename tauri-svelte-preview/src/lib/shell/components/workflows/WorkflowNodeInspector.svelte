<script lang="ts">
  import type {
    ReviewReceipt,
    WorkflowNodeDefinition,
    WorkflowNodeView,
    WorkflowResultReceipt
  } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    node: WorkflowNodeView | null;
    definition?: WorkflowNodeDefinition | null;
    reviewerNodes?: WorkflowNodeView[];
    busy?: boolean;
    onRetry?(): void;
    onSkip?(): void;
    onApprove?(approved: boolean, structuredInput?: Record<string, unknown>): void;
    onSubmitResult?(result: WorkflowResultReceipt): void;
    onCompareReviewers?(): void;
  }

  let {
    node,
    definition = null,
    reviewerNodes = [],
    busy = false,
    onRetry,
    onSkip,
    onApprove,
    onSubmitResult,
    onCompareReviewers
  }: Props = $props();

  let resultText = $state('{}');
  let resultError = $state<string | null>(null);
  let gateInputText = $state('{}');
  let gateInputError = $state<string | null>(null);

  $effect(() => {
    resultText = node?.structuredOutput ? JSON.stringify(node.structuredOutput, null, 2) : '{}';
    resultError = null;
    gateInputText = node?.gate?.structuredInput
      ? JSON.stringify(node.gate.structuredInput, null, 2)
      : '{}';
    gateInputError = null;
  });

  const canSkip = $derived(
    node !== null && !['completed', 'running', 'starting'].includes(node.state)
  );
  const canRetry = $derived(node?.state === 'failed');
  const waitingForApproval = $derived(node?.state === 'waiting-approval');

  function submit(): void {
    resultError = null;
    try {
      const parsed = JSON.parse(resultText) as WorkflowResultReceipt;
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        throw new Error('A structured object is required.');
      }
      onSubmitResult?.(parsed);
    } catch (error) {
      resultError = error instanceof Error ? error.message : 'Result must be valid JSON.';
    }
  }

  function decideGate(approved: boolean): void {
    gateInputError = null;
    try {
      const parsed = JSON.parse(gateInputText) as Record<string, unknown>;
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        throw new Error('Gate input must be a JSON object.');
      }
      onApprove?.(approved, parsed);
    } catch (error) {
      gateInputError = error instanceof Error ? error.message : 'Gate input must be valid JSON.';
    }
  }

  function reviewSummary(review: WorkflowNodeView): string {
    const output = review.structuredOutput as Partial<ReviewReceipt> | null;
    if (!output || typeof output !== 'object') return 'No review receipt yet.';
    return `${output.severity ?? 'severity unknown'} · ${output.file ?? 'file unknown'} · ${output.blocking ? 'blocking' : 'non-blocking'}`;
  }
</script>

<section data-testid="workflow-node-inspector" class="panel" aria-label="Workflow node inspector">
  <header data-testid="workflow-node-inspector-header" class="panel-header">
    <div data-testid="workflow-node-inspector-title" class="panel-title">Node inspector</div>
    {#if node}
      <span data-testid="workflow-node-inspector-state" class={`state state-${node.state}`}>{node.state.replaceAll('-', ' ')}</span>
    {/if}
  </header>

  {#if !node}
    <p data-testid="workflow-node-inspector-empty" class="empty">Select a node from the graph, lanes, or timeline.</p>
  {:else}
    <div data-testid="workflow-node-inspector-body" class="body">
      <div data-testid="workflow-node-identity" class="identity">
        <span data-testid="workflow-node-title" class="node-title">{definition?.title ?? node.nodeId}</span>
        <span data-testid="workflow-node-owned-id" class="mono">ownedId: {node.ownedId}</span>
        <span data-testid="workflow-node-run-id" class="mono">workflowRunId: {node.workflowRunId}</span>
        <span data-testid="workflow-node-provenance" class="provenance">{node.provenance === 'workflow' ? 'Workflow' : 'Provider-native'}</span>
      </div>

      <dl data-testid="workflow-node-facts" class="facts">
        <div data-testid="workflow-node-fact-role"><dt>Role</dt><dd>{node.roleId}</dd></div>
        <div data-testid="workflow-node-fact-provider"><dt>Provider</dt><dd>{node.provider}</dd></div>
        <div data-testid="workflow-node-fact-attempt"><dt>Attempt</dt><dd>{node.attempt}</dd></div>
        <div data-testid="workflow-node-fact-depth"><dt>Depth</dt><dd>{node.depth}</dd></div>
        <div data-testid="workflow-node-fact-contract"><dt>Contract</dt><dd>{node.outputContract}</dd></div>
        <div data-testid="workflow-node-fact-lease"><dt>Lease</dt><dd>{node.leaseId ?? 'None'}</dd></div>
      </dl>

      {#if definition?.dependsOn.length}
        <p data-testid="workflow-node-dependencies" class="copy">Depends on: {definition.dependsOn.join(', ')}</p>
      {/if}
      {#if definition?.approvalGate}
        <p data-testid="workflow-node-gate-prompt" class="gate-copy">Gate: {definition.approvalGate.prompt || 'Approval required before continuing.'}</p>
      {/if}
      {#if node.failure}
        <div data-testid="workflow-node-failure" class="failure"><strong data-testid="workflow-node-failure-code">{node.failure.code}</strong><span data-testid="workflow-node-failure-message">{node.failure.message}</span></div>
      {/if}

      {#if node.artifacts.length > 0}
        <section data-testid="workflow-node-artifacts" class="subsection">
          <h3 data-testid="workflow-node-artifacts-heading">Artifacts</h3>
          <ul data-testid="workflow-node-artifact-items" class="artifact-list">
            {#each node.artifacts as artifact (artifact.id)}
              <li data-testid={`workflow-node-artifact-${artifact.id}`}>
                <span data-testid={`workflow-node-artifact-label-${artifact.id}`}>{artifact.kind}: {artifact.id}</span>
                {#if artifact.url}
                  <a data-testid={`workflow-node-artifact-link-${artifact.id}`} href={artifact.url} target="_blank" rel="noreferrer">Open</a>
                {:else if artifact.path}
                  <span data-testid={`workflow-node-artifact-path-${artifact.id}`} class="artifact-path">{artifact.path}</span>
                {/if}
                {#if artifact.digest}<span data-testid={`workflow-node-artifact-digest-${artifact.id}`} class="artifact-digest">digest {artifact.digest}</span>{/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <div data-testid="workflow-node-controls" class="controls">
        {#if waitingForApproval}
          <label data-testid="workflow-node-gate-input-label" class="gate-input-label">
            <span data-testid="workflow-node-gate-input-copy">Gate response (JSON object)</span>
            <textarea
              data-testid="workflow-node-gate-input"
              value={gateInputText}
              aria-label="Structured gate response"
              oninput={(event) => {
                gateInputText = (event.currentTarget as HTMLTextAreaElement).value;
                gateInputError = null;
              }}
            ></textarea>
          </label>
          {#if gateInputError}<p data-testid="workflow-node-gate-input-error" class="error">{gateInputError}</p>{/if}
          <button data-testid="workflow-node-approve" class="action accent" type="button" disabled={busy} onclick={() => decideGate(true)}>Approve gate</button>
          <button data-testid="workflow-node-reject" class="action danger" type="button" disabled={busy} onclick={() => decideGate(false)}>Reject gate</button>
        {/if}
        {#if canRetry}
          <button data-testid="workflow-node-retry" class="action" type="button" disabled={busy} onclick={() => onRetry?.()}>Retry node</button>
        {/if}
        {#if canSkip}
          <button data-testid="workflow-node-skip" class="action" type="button" disabled={busy} onclick={() => onSkip?.()}>Skip node</button>
        {/if}
      </div>

      <section data-testid="workflow-node-result-editor" class="subsection">
        <h3 data-testid="workflow-node-result-heading">Structured result</h3>
        <textarea data-testid="workflow-node-result-input" value={resultText} aria-label="Structured workflow result" oninput={(event) => (resultText = (event.currentTarget as HTMLTextAreaElement).value)}></textarea>
        {#if resultError}<p data-testid="workflow-node-result-error" class="error">{resultError}</p>{/if}
        <button data-testid="workflow-node-submit-result" class="action accent" type="button" disabled={busy} onclick={submit}>Submit result</button>
      </section>

      {#if reviewerNodes.length > 0}
        <section data-testid="workflow-reviewer-comparison" class="subsection">
          <div data-testid="workflow-reviewer-comparison-header" class="subsection-header">
            <h3 data-testid="workflow-reviewer-comparison-heading">Reviewer findings</h3>
            <button data-testid="workflow-reviewer-compare" class="action" type="button" onclick={() => onCompareReviewers?.()}>Compare findings</button>
          </div>
          <ul data-testid="workflow-reviewer-items" class="review-list">
            {#each reviewerNodes as review (review.id)}
              <li data-testid={`workflow-reviewer-item-${review.id}`}><span data-testid={`workflow-reviewer-label-${review.id}`}>{review.nodeId}</span><span data-testid={`workflow-reviewer-summary-${review.id}`}>{reviewSummary(review)}</span></li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .panel-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 9px 10px; border-bottom: 1px solid var(--color-border); }
  .panel-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .state, .provenance { padding: 3px 5px; border-radius: 4px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; text-transform: capitalize; }
  .state-running, .state-starting { color: var(--color-live); background: var(--color-live-bg); }
  .state-waiting-approval, .state-waiting-input { color: var(--color-attention); background: var(--color-attention-bg); }
  .state-completed, .state-skipped { color: var(--color-good); background: var(--color-good-bg); }
  .state-failed, .state-cancelled { color: var(--color-bad); background: var(--color-bad-bg); }
  .provenance { color: var(--color-accent); background: var(--color-selected); text-transform: none; }
  .body { display: grid; gap: 10px; min-height: 0; padding: 10px; overflow: auto; }
  .identity { display: grid; gap: 3px; }
  .node-title { font-size: 15px; font-weight: 650; }
  .mono { overflow: hidden; color: var(--color-text-2); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
  .facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(115px, 1fr)); gap: 6px; margin: 0; }
  .facts div { min-width: 0; padding: 6px; border: 1px solid var(--color-border); border-radius: 5px; background: var(--color-bg); }
  dt { color: var(--color-text-3); font-size: 12px; }
  dd { margin: 3px 0 0; overflow: hidden; font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .copy, .gate-copy { margin: 0; color: var(--color-text-2); font-size: 13px; }
  .gate-copy { color: var(--color-attention); }
  .failure { display: grid; gap: 3px; padding: 7px; border: 1px solid var(--color-bad); border-radius: 5px; color: var(--color-bad); background: var(--color-bad-bg); font-size: 13px; }
  .controls, .subsection-header { display: flex; align-items: center; flex-wrap: wrap; gap: 5px; }
  .gate-input-label { display: grid; flex: 1 1 100%; gap: 4px; color: var(--color-text-2); font-size: 12px; }
  .gate-input-label textarea { min-height: 70px; }
  .subsection { display: grid; gap: 6px; padding-top: 8px; border-top: 1px solid var(--color-border); }
  h3 { margin: 0; font-size: 13px; font-weight: 600; }
  .action { min-height: 28px; padding: 0 8px; border: 0; border-radius: 5px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; cursor: pointer; }
  .action:hover, .action:focus-visible { color: var(--color-text); background: var(--color-hover); outline: none; }
  .action.accent { color: var(--color-on-accent); background: var(--color-accent); }
  .action.danger { color: var(--color-bad); background: var(--color-bad-bg); }
  .action:disabled { cursor: default; opacity: .5; }
  textarea { min-height: 120px; resize: vertical; padding: 7px; border: 1px solid var(--color-border); border-radius: 5px; color: var(--color-text); background: var(--color-bg); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; }
  textarea:focus-visible { outline: 1px solid var(--color-focus-solid); outline-offset: 1px; }
  .error { margin: 0; color: var(--color-bad); font-size: 12px; }
  .review-list { display: grid; gap: 5px; margin: 0; padding-left: 16px; color: var(--color-text-2); font-size: 12px; }
  .review-list li { display: grid; gap: 2px; }
  .review-list li span:first-child { color: var(--color-text); font-weight: 600; }
  .artifact-list { display: grid; gap: 5px; margin: 0; padding-left: 16px; color: var(--color-text-2); font-size: 12px; }
  .artifact-list li { display: grid; gap: 2px; }
  .artifact-list a { color: var(--color-accent); }
  .artifact-path, .artifact-digest { overflow: hidden; color: var(--color-text-3); text-overflow: ellipsis; white-space: nowrap; }
  .empty { margin: 0; padding: 14px; color: var(--color-text-2); font-size: 13px; }
</style>
