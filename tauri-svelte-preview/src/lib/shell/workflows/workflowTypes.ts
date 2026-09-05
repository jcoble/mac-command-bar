/**
 * Typed frontend contracts for the WorkflowEngine snapshot.
 *
 * `WorkflowRunRecord` and the related records mirror the serde/camelCase
 * shapes in `src-tauri/src/workflow.rs`.  The `*View` types add only stable
 * presentation identity and provenance; they never replace the engine's
 * state/phase with a client-side guess.
 */

export type WorkflowRunState =
  | 'draft'
  | 'queued'
  | 'running'
  | 'waiting-approval'
  | 'waiting-input'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type WorkflowNodeState =
  | 'blocked'
  | 'ready'
  | 'queued'
  | 'starting'
  | 'running'
  | 'waiting-approval'
  | 'waiting-input'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'skipped';

/** This is the engine's typed phase.  Never derive it from assistant prose. */
export type WorkflowLoopPhase =
  | 'draft'
  | 'scheduling'
  | 'implementing'
  | 'reviewing'
  | 'verifying'
  | 'approval'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type WorkflowOutputContract =
  | 'ImplementationReceipt'
  | 'ReviewReceipt'
  | 'SpecComplianceReceipt'
  | 'VerificationReceipt'
  | 'PlanReceipt';

export type WorkflowProvenance = 'provider-native' | 'workflow';

export const WORKFLOW_PROVENANCE_LABELS: Record<WorkflowProvenance, string> = {
  'provider-native': 'Provider-native',
  workflow: 'Workflow'
};

export function provenanceLabel(provenance: WorkflowProvenance): string {
  return WORKFLOW_PROVENANCE_LABELS[provenance];
}

export interface WorkflowDefinitionV1 {
  version: number;
  id: string;
  name: string;
  description: string;
  trigger: unknown;
  inputs: unknown[];
  roles: AgentRoleDefinition[];
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdge[];
  concurrency: WorkflowConcurrencyPolicy;
  budgets: WorkflowBudgetPolicy;
  completion: WorkflowCompletionPolicy;
}

export interface AgentRoleDefinition {
  id: string;
  name: string;
  purpose: string;
  providerPolicy: ProviderSelectionPolicy;
  modelPolicy: ConfigSelectionPolicy;
  effortPolicy: ConfigSelectionPolicy;
  permissionPolicy: ConfigSelectionPolicy;
  promptTemplateId: string;
  outputContract: WorkflowOutputContract;
  workspacePolicy: WorkflowWorkspacePolicy;
  retryPolicy: WorkflowRetryPolicy;
}

export interface WorkflowNodeDefinition {
  id: string;
  title: string;
  roleId: string;
  dependsOn: string[];
  condition: unknown | null;
  fanOut: unknown | null;
  approvalGate: WorkflowApprovalGate | null;
  timeoutSeconds: number;
  maxAttempts: number;
}

export interface WorkflowEdge {
  from: string;
  to: string;
}

export interface WorkflowApprovalGate {
  id: string;
  prompt: string;
}

export interface ProviderSelectionPolicy {
  provider: string;
  allowedProviders: string[];
}

export interface ConfigSelectionPolicy {
  value: string | null;
  overrides: Record<string, string>;
}

export type WorkflowWorkspacePolicy =
  | { kind: 'read-only-current' }
  | { kind: 'shared-current'; fileAllowList: string[] }
  | { kind: 'dedicated-existing'; worktreeId: string }
  | { kind: 'dedicated-new'; branchTemplate: string }
  | { kind: 'none' };

export interface WorkflowRetryPolicy {
  maxAttempts: number;
  retryableCodes: string[];
}

export interface WorkflowConcurrencyPolicy {
  global: number;
  workflow: number;
  providers: Record<string, number>;
}

export interface WorkflowBudgetPolicy {
  maximumActiveAgents: number;
  maximumChildDepth: number;
  maximumAttemptsPerNode: number;
  maximumWallTimeSeconds: number;
  maximumTokens: number | null;
  maximumToolTerminals: number;
  maximumWorktrees: number;
}

export interface WorkflowCompletionPolicy {
  cancelDescendantsOnFailure: boolean;
}

/** Exact serialized `WorkflowArtifactRef`. */
export interface WorkflowArtifactRef {
  id: string;
  kind: string;
  path: string | null;
  url: string | null;
  digest: string | null;
}

/** Exact serialized `WorkflowGateRecord`. */
export interface WorkflowGateRecord {
  id: string;
  nodeRunId: string;
  approved: boolean | null;
  structuredInput: unknown | null;
}

/** Exact serialized `WorkflowFailure`. */
export interface WorkflowFailure {
  code: string;
  message: string;
  budget: string | null;
}

/** Exact serialized `WorkflowNodeRunRecord`. */
export interface WorkflowNodeRunRecord {
  id: string;
  nodeId: string;
  roleId: string;
  state: WorkflowNodeState;
  attempt: number;
  depth: number;
  ownedId: string;
  provider: string;
  providerInstanceId: string | null;
  startedAtMs: number | null;
  finishedAtMs: number | null;
  outputContract: WorkflowOutputContract;
  structuredOutput: unknown | null;
  artifacts: WorkflowArtifactRef[];
  gate: WorkflowGateRecord | null;
  leaseId: string | null;
  failure: WorkflowFailure | null;
}

/** Exact serialized `WorkflowRunRecord`; this is what Tauri returns. */
export interface WorkflowRunRecord {
  id: string;
  workflowId: string;
  definition: WorkflowDefinitionV1;
  state: WorkflowRunState;
  phase: WorkflowLoopPhase;
  inputSnapshot: unknown;
  inputHash: string;
  createdAtMs: number;
  updatedAtMs: number;
  nodes: WorkflowNodeRunRecord[];
  tokensUsed: number;
  toolTerminalsUsed: number;
  worktreesAllocated: number;
  lastSequence: number;
}

/** The five structured output contracts accepted by the engine. */
export interface ImplementationReceipt {
  changedFiles: string[];
  summary: string;
  tests: unknown[];
  knownRisks: string[];
  commit: string | null;
  branch: string | null;
  worktree: string | null;
  artifacts: WorkflowArtifactRef[];
}

export interface ReviewReceipt {
  severity: string;
  file: string;
  line: number;
  evidence: string;
  recommendation: string;
  confidence: number;
  blocking: boolean;
}

export interface SpecComplianceReceipt {
  requirementId: string;
  status: string;
  evidence: unknown;
  gap: string | null;
  recommendedAction: string | null;
}

export interface VerificationReceipt {
  command: string;
  exit: number;
  duration: number;
  evidenceArtifacts: WorkflowArtifactRef[];
  cleanupReceipt: unknown;
}

export interface PlanReceipt {
  orderedSteps: unknown[];
  dependencies: unknown[];
  risk: unknown;
  estimatedParallelLanes: number;
}

export type WorkflowResultReceipt =
  | ImplementationReceipt
  | ReviewReceipt
  | SpecComplianceReceipt
  | VerificationReceipt
  | PlanReceipt;

/* -------------------------------------------------------------------------- */
/* Presentation views                                                         */
/* -------------------------------------------------------------------------- */

export interface WorkflowArtifactView extends WorkflowArtifactRef {
  workflowRunId: string;
  nodeRunId: string;
  provenance: WorkflowProvenance;
}

export interface WorkflowGateView extends WorkflowGateRecord {
  workflowRunId: string;
  provenance: WorkflowProvenance;
}

export interface WorkflowAgentView {
  workflowRunId: string | null;
  nodeRunId: string | null;
  /** Provider/session identity. It must never be confused with workflowRunId. */
  ownedId: string;
  roleId: string | null;
  provider: string;
  providerInstanceId: string | null;
  state: WorkflowNodeState | 'provider-native';
  attempt: number | null;
  depth: number | null;
  outputContract: WorkflowOutputContract | null;
  leaseId: string | null;
  failure: WorkflowFailure | null;
  provenance: WorkflowProvenance;
}

export interface WorkflowNodeView extends Omit<WorkflowNodeRunRecord, 'artifacts' | 'gate'> {
  workflowRunId: string;
  artifacts: WorkflowArtifactView[];
  gate: WorkflowGateView | null;
  agent: WorkflowAgentView;
  provenance: WorkflowProvenance;
}

export interface WorkflowRunView extends Omit<WorkflowRunRecord, 'nodes'> {
  /** Stable workflow identity, deliberately distinct from every node ownedId. */
  workflowRunId: string;
  nodes: WorkflowNodeView[];
  provenance: WorkflowProvenance;
}

export function toWorkflowArtifactView(
  workflowRunId: string,
  nodeRunId: string,
  artifact: WorkflowArtifactRef
): WorkflowArtifactView {
  return {
    ...artifact,
    workflowRunId,
    nodeRunId,
    provenance: 'workflow'
  };
}

export function toWorkflowGateView(
  workflowRunId: string,
  gate: WorkflowGateRecord
): WorkflowGateView {
  return { ...gate, workflowRunId, provenance: 'workflow' };
}

export function toWorkflowAgentView(
  workflowRunId: string,
  node: WorkflowNodeRunRecord
): WorkflowAgentView {
  return {
    workflowRunId,
    nodeRunId: node.id,
    ownedId: node.ownedId,
    roleId: node.roleId,
    provider: node.provider,
    providerInstanceId: node.providerInstanceId,
    state: node.state,
    attempt: node.attempt,
    depth: node.depth,
    outputContract: node.outputContract,
    leaseId: node.leaseId,
    failure: node.failure,
    provenance: 'workflow'
  };
}

export function toWorkflowNodeView(
  workflowRunId: string,
  node: WorkflowNodeRunRecord
): WorkflowNodeView {
  return {
    ...node,
    workflowRunId,
    artifacts: node.artifacts.map((artifact) =>
      toWorkflowArtifactView(workflowRunId, node.id, artifact)
    ),
    gate: node.gate ? toWorkflowGateView(workflowRunId, node.gate) : null,
    agent: toWorkflowAgentView(workflowRunId, node),
    provenance: 'workflow'
  };
}

export function toWorkflowRunView(record: WorkflowRunRecord): WorkflowRunView {
  return {
    ...record,
    workflowRunId: record.id,
    nodes: record.nodes.map((node) => toWorkflowNodeView(record.id, node)),
    provenance: 'workflow'
  };
}

/** A provider-native child can be shown beside workflow agents without being
 * assigned a fake workflow identity. */
export function providerNativeAgentView(input: {
  ownedId: string;
  provider: string;
  providerInstanceId?: string | null;
  state?: WorkflowAgentView['state'];
  roleId?: string | null;
}): WorkflowAgentView {
  return {
    workflowRunId: null,
    nodeRunId: null,
    ownedId: input.ownedId,
    roleId: input.roleId ?? null,
    provider: input.provider,
    providerInstanceId: input.providerInstanceId ?? null,
    state: input.state ?? 'provider-native',
    attempt: null,
    depth: null,
    outputContract: null,
    leaseId: null,
    failure: null,
    provenance: 'provider-native'
  };
}

/* -------------------------------------------------------------------------- */
/* Tauri command DTOs                                                          */
/* -------------------------------------------------------------------------- */

export interface CreateWorkflowRunDto {
  definition: WorkflowDefinitionV1;
  input: Record<string, unknown>;
  idempotencyKey: string;
}

export interface WorkflowRunIdempotencyDto {
  runId: string;
  idempotencyKey: string;
}

export interface WorkflowNodeIdempotencyDto extends WorkflowRunIdempotencyDto {
  nodeId: string;
}

export interface ApproveWorkflowGateDto extends WorkflowNodeIdempotencyDto {
  approval: { approved: boolean; [key: string]: unknown };
}

export interface RedirectWorkflowNodeDto extends WorkflowNodeIdempotencyDto {
  provider: string;
}

export interface SubmitWorkflowResultDto extends WorkflowNodeIdempotencyDto {
  result: WorkflowResultReceipt;
}

export type StartWorkflowRunDto = WorkflowRunIdempotencyDto;
export type PauseWorkflowRunDto = WorkflowRunIdempotencyDto;
export type ResumeWorkflowRunDto = WorkflowRunIdempotencyDto;
export type CancelWorkflowRunDto = WorkflowRunIdempotencyDto;
export type RetryWorkflowNodeDto = WorkflowNodeIdempotencyDto;
export type SkipWorkflowNodeDto = WorkflowNodeIdempotencyDto;

/** Useful for callers that want to describe one control action without IO. */
export type WorkflowCommandDto =
  | { kind: 'start'; input: StartWorkflowRunDto }
  | { kind: 'pause'; input: PauseWorkflowRunDto }
  | { kind: 'resume'; input: ResumeWorkflowRunDto }
  | { kind: 'cancel'; input: CancelWorkflowRunDto }
  | { kind: 'retry-node'; input: RetryWorkflowNodeDto }
  | { kind: 'redirect-node'; input: RedirectWorkflowNodeDto }
  | { kind: 'skip-node'; input: SkipWorkflowNodeDto }
  | { kind: 'approve-gate'; input: ApproveWorkflowGateDto }
  | { kind: 'submit-result'; input: SubmitWorkflowResultDto };

export type WorkflowFilter = 'all' | 'active' | 'waiting' | 'failed' | 'completed';
