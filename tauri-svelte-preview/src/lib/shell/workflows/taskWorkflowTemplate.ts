import type {
  AgentRoleDefinition,
  WorkflowDefinitionV1,
  WorkflowOutputContract,
  WorkflowWorkspacePolicy
} from './workflowTypes';

export type WorkflowProvider = 'codex' | 'claude' | 'antigravity';

export interface TaskWorkflowProviders {
  plan: WorkflowProvider;
  implement: WorkflowProvider;
  review: WorkflowProvider;
}

function role(
  id: string,
  name: string,
  purpose: string,
  provider: WorkflowProvider,
  outputContract: WorkflowOutputContract,
  workspacePolicy: WorkflowWorkspacePolicy
): AgentRoleDefinition {
  return {
    id,
    name,
    purpose,
    providerPolicy: { provider, allowedProviders: ['codex', 'claude', 'antigravity'] },
    modelPolicy: { value: null, overrides: {} },
    effortPolicy: { value: null, overrides: {} },
    permissionPolicy: { value: null, overrides: {} },
    promptTemplateId: '',
    outputContract,
    workspacePolicy,
    retryPolicy: {
      maxAttempts: 2,
      retryableCodes: ['runtime', 'agent-turn', 'output-contract']
    }
  };
}

/** The one owner-driven workflow offered by the Agents panel. */
export function taskWorkflowDefinition(
  providers: TaskWorkflowProviders
): WorkflowDefinitionV1 {
  return {
    version: 1,
    id: 'task-plan-implement-review',
    name: 'Plan, implement, review',
    description: 'A visible three-stage task loop with approval at each handoff.',
    trigger: { kind: 'manual' },
    inputs: [{ id: 'task', required: true }],
    roles: [
      role(
        'planner',
        'Planner',
        'Turn the task into a concise implementation plan grounded in the current workspace.',
        providers.plan,
        'PlanReceipt',
        { kind: 'read-only-current' }
      ),
      role(
        'implementer',
        'Implementer',
        'Implement the approved plan, keep the diff scoped, and verify the changed behavior.',
        providers.implement,
        'ImplementationReceipt',
        { kind: 'shared-current', fileAllowList: [] }
      ),
      role(
        'reviewer',
        'Reviewer',
        'Review the implementation against the task and its approved plan. Report one blocking finding, or a non-blocking no-finding receipt.',
        providers.review,
        'ReviewReceipt',
        { kind: 'read-only-current' }
      )
    ],
    nodes: [
      {
        id: 'plan',
        title: 'Plan',
        roleId: 'planner',
        dependsOn: [],
        condition: null,
        fanOut: null,
        approvalGate: { id: 'approve-plan', prompt: 'Approve the plan before implementation.' },
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'implement',
        title: 'Implement',
        roleId: 'implementer',
        dependsOn: ['plan'],
        condition: null,
        fanOut: null,
        approvalGate: {
          id: 'approve-implementation',
          prompt: 'Approve the implementation before review.'
        },
        timeoutSeconds: 7200,
        maxAttempts: 2
      },
      {
        id: 'review',
        title: 'Review',
        roleId: 'reviewer',
        dependsOn: ['implement'],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      }
    ],
    edges: [],
    concurrency: {
      global: 1,
      workflow: 1,
      providers: { codex: 1, claude: 1, antigravity: 1 }
    },
    budgets: {
      maximumActiveAgents: 1,
      maximumChildDepth: 0,
      maximumAttemptsPerNode: 2,
      maximumWallTimeSeconds: 14_400,
      maximumTokens: null,
      maximumToolTerminals: 4,
      maximumWorktrees: 0
    },
    completion: { cancelDescendantsOnFailure: true }
  };
}
