/**
 * A7 contract checks.  These are intentionally Node-light: the workflow view
 * and store are exercised without starting Tauri, while the service checks
 * prove the boundary and real command wiring by inspection.
 */
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { compileModule } from 'svelte/compiler';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workflowTypesPath = path.join(root, 'src/lib/shell/workflows/workflowTypes.ts');
const workflowStorePath = path.join(root, 'src/lib/shell/workflows/workflowStore.svelte.ts');
const workflowServicePath = path.join(root, 'src/lib/shell/workflows/workflowService.ts');

const types = await import(workflowTypesPath);

function sampleRecord() {
  return {
    id: 'run-a7',
    workflowId: 'template-a7',
    definition: {
      version: 1,
      id: 'template-a7',
      name: 'A7 acceptance run',
      description: '',
      trigger: {},
      inputs: [],
      roles: [{
        id: 'reviewer',
        name: 'Reviewer',
        purpose: 'Check the change.',
        providerPolicy: { provider: 'codex', allowedProviders: ['codex'] },
        modelPolicy: { value: null, overrides: {} },
        effortPolicy: { value: null, overrides: {} },
        permissionPolicy: { value: null, overrides: {} },
        promptTemplateId: '',
        outputContract: 'ReviewReceipt',
        workspacePolicy: { kind: 'read-only-current' },
        retryPolicy: { maxAttempts: 1, retryableCodes: [] }
      }],
      nodes: [{
        id: 'review',
        title: 'Review',
        roleId: 'reviewer',
        dependsOn: [],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 1
      }],
      edges: [],
      concurrency: { global: 8, workflow: 4, providers: {} },
      budgets: {
        maximumActiveAgents: 8,
        maximumChildDepth: 3,
        maximumAttemptsPerNode: 1,
        maximumWallTimeSeconds: 86400,
        maximumTokens: null,
        maximumToolTerminals: 8,
        maximumWorktrees: 4
      },
      completion: { cancelDescendantsOnFailure: false }
    },
    state: 'running',
    phase: 'reviewing',
    inputSnapshot: {},
    inputHash: 'hash-a7',
    createdAtMs: 1,
    updatedAtMs: 2,
    nodes: [{
      id: 'run-a7:review:1',
      nodeId: 'review',
      roleId: 'reviewer',
      state: 'running',
      attempt: 1,
      depth: 0,
      ownedId: 'owned-agent-a7',
      provider: 'codex',
      providerInstanceId: 'codex-1',
      startedAtMs: 2,
      finishedAtMs: null,
      outputContract: 'ReviewReceipt',
      structuredOutput: null,
      artifacts: [{ id: 'artifact-a7', kind: 'test-log', path: '/tmp/a7.log', url: null, digest: null }],
      gate: null,
      leaseId: null,
      failure: null
    }],
    tokensUsed: 10,
    toolTerminalsUsed: 1,
    worktreesAllocated: 0,
    lastSequence: 3
  };
}

// ── Views preserve engine truth and identity ────────────────────────────────
const record = sampleRecord();
const view = types.toWorkflowRunView(record);
assert.equal(view.workflowRunId, 'run-a7');
assert.equal(view.phase, 'reviewing');
assert.equal(view.provenance, 'workflow');
assert.equal(view.nodes[0].agent.provenance, 'workflow');
assert.equal(view.nodes[0].agent.workflowRunId, 'run-a7');
assert.notEqual(view.nodes[0].agent.workflowRunId, view.nodes[0].agent.ownedId);
assert.equal(view.nodes[0].artifacts[0].workflowRunId, 'run-a7');
assert.equal(view.nodes[0].artifacts[0].nodeRunId, 'run-a7:review:1');
assert.equal(types.provenanceLabel('provider-native'), 'Provider-native');
assert.equal(types.provenanceLabel('workflow'), 'Workflow');
const native = types.providerNativeAgentView({ ownedId: 'native-a7', provider: 'claude' });
assert.equal(native.workflowRunId, null);
assert.equal(native.ownedId, 'native-a7');
assert.equal(native.provenance, 'provider-native');

// ── Snapshot store has no IO/reducer/ledger path ────────────────────────────
const outputDir = path.join(root, 'node_modules/.mcb-test-a7/');
const outputPath = path.join(outputDir, 'workflowStore.compiled.mjs');
mkdirSync(outputDir, { recursive: true });
const source = readFileSync(workflowStorePath, 'utf8');
const javascript = stripTypeScriptTypes(source, { mode: 'strip' });
const compiled = compileModule(javascript, { generate: 'client', filename: 'workflowStore.svelte.js' });
writeFileSync(outputPath, compiled.js.code);
let store;
try {
  store = await import(outputPath);
} finally {
  rmSync(outputDir, { recursive: true, force: true });
}
store.resetWorkflowStore();
assert.equal(store.workflowState.runs.length, 0);
store.applyWorkflowSnapshots([record]);
store.selectWorkflowRun('run-a7');
assert.equal(store.selectedWorkflowRun().id, 'run-a7');
assert.equal(store.filteredWorkflowRuns().length, 1);
store.setWorkflowFilter('failed');
assert.equal(store.filteredWorkflowRuns().length, 0);
store.setWorkflowFilter('all');
store.setWorkflowQuery('a7 acceptance');
assert.equal(store.filteredWorkflowRuns().length, 1);
assert.doesNotMatch(source, /orchestrationView|JSONL|read_workflow_event/);

// ── Every engine control goes through the service/wrapper boundary ───────────
const serviceSource = readFileSync(workflowServicePath, 'utf8');
const controlPairs = [
  ['startWorkflowRun', 'startWorkflowRunFromTauri'],
  ['pauseWorkflowRun', 'pauseWorkflowRunFromTauri'],
  ['resumeWorkflowRun', 'resumeWorkflowRunFromTauri'],
  ['cancelWorkflowRun', 'cancelWorkflowRunFromTauri'],
  ['retryWorkflowNode', 'retryWorkflowNodeFromTauri'],
  ['skipWorkflowNode', 'skipWorkflowNodeFromTauri'],
  ['approveWorkflowGate', 'approveWorkflowGateFromTauri'],
  ['submitWorkflowResult', 'submitWorkflowResultFromTauri']
];
for (const [serviceFunction, wrapper] of controlPairs) {
  assert.match(serviceSource, new RegExp(`export async function ${serviceFunction}\\b`));
  assert.match(serviceSource, new RegExp(`return ${wrapper}\\(`));
}
assert.match(serviceSource, /return listWorkflowRunsFromTauri\(\)/);
assert.doesNotMatch(serviceSource, /\binvoke\s*\(/);

// The workflow UI that used to sit on top of this service has been removed —
// the Agents panel replaced it and workflow authoring is not part of the app
// today. The engine, its commands, and this service stay, so the checks above
// still describe the boundary the next workflow surface has to use.
assert.equal(
  existsSync(path.join(root, 'src/lib/shell/components/workflows')),
  false,
  'the superseded workflow components are gone'
);

console.log('agent control center tests passed');
