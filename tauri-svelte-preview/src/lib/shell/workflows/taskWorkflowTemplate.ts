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

export interface SavedWorkflowStage {
  id: string;
  title: string;
  instructions: string;
  provider: WorkflowProvider;
  outputContract: WorkflowOutputContract;
  redoStageId: string | null;
  approvalPrompt: string | null;
}

export interface SavedWorkflowTemplate {
  id: string;
  name: string;
  stages: SavedWorkflowStage[];
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
    name: 'Spec, review, plan, review, implement, review, verify, open PR, review PR, merge',
    description: 'A visible task loop from reviewed spec and plan through pull request merge or owner handoff.',
    trigger: { kind: 'manual' },
    inputs: [{ id: 'task', required: true }],
    roles: [
      role(
        'specifier',
        'Spec author',
        'Write a scoped design spec in docs/superpowers/specs/ using the task and current workspace. State concrete requirements and acceptance criteria. Make no implementation changes. Return the spec path, summary, and requirement list.',
        providers.plan,
        'SpecReceipt',
        { kind: 'shared-current', fileAllowList: [] }
      ),
      role(
        'planner',
        'Controller',
        'Read the approved spec and turn it into a concise implementation plan grounded in the current workspace. Limit orderedSteps to implementation work; the workflow handles verification, PR opening, PR review, and merge in later stages. Set ownerMerge true only when the task explicitly asks the owner to perform the merge personally; otherwise set it false.',
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
        'Independently review the spec, plan, or implementation against the task. For plan review, block any orderedSteps that include verification, PR opening, PR review, or merge, since the workflow handles those stages. For implementation review, check both code and spec requirements. Report one blocking finding, or a non-blocking no-finding receipt.',
        providers.review,
        'ReviewReceipt',
        { kind: 'read-only-current' }
      ),
      role(
        'verifier',
        'Verifier',
        'Run relevant regression, integration, and UI checks. Browser checks must be headless at 1710x990 with the viewport verified after launch; save before and after screenshots to ~/Workbox/screenshots/ and stop the browser process tree. For native Assembly checks, use workbox-native-ui. Report commands, exits, evidence, and cleanup. Set evidenceArtifacts to [] when there is no saved artifact; otherwise each entry must contain id, kind, path, url, and digest.',
        providers.review,
        'VerificationReceipt',
        { kind: 'read-only-current' }
      ),
      role(
        'pr-author',
        'PR author',
        'Commit any verified uncommitted changes, push the branch, then open or update its pull request with before and after UI screenshots as artifacts when relevant. For Notion tasks use a tsk-<id> branch. End every commit message body with Committed-by: <actual committer>; never add a co-author trailer. Do not merge in this stage; the PR reviewer and PR merger run later. Return the PR URL and branch for final review.',
        providers.implement,
        'PullRequestReceipt',
        { kind: 'shared-current', fileAllowList: [] }
      ),
      role(
        'pr-reviewer',
        'PR reviewer',
        'Independently review the opened pull request diff, check results, and before and after UI evidence against the task. Return one blocking finding if the PR is not ready to merge, or a non-blocking no-finding receipt.',
        providers.review,
        'ReviewReceipt',
        { kind: 'read-only-current' }
      ),
      role(
        'pr-merger',
        'PR merger',
        'After a passing final PR review, merge the PR from the Open PR receipt, verify its merged state and merge commit SHA on GitHub, and return both. If the merge fails, report the failure instead of a success receipt.',
        providers.implement,
        'PullRequestMergeReceipt',
        { kind: 'read-only-current' }
      )
    ],
    nodes: [
      {
        id: 'spec',
        title: 'Spec',
        roleId: 'specifier',
        dependsOn: [],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'spec-review',
        title: 'Spec review',
        roleId: 'reviewer',
        dependsOn: ['spec'],
        condition: { redoNodeId: 'spec' },
        fanOut: null,
        approvalGate: { id: 'approve-spec', prompt: 'Approve the reviewed spec before planning.' },
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'plan',
        title: 'Plan',
        roleId: 'planner',
        dependsOn: ['spec', 'spec-review'],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'plan-review',
        title: 'Plan review',
        roleId: 'reviewer',
        dependsOn: ['plan'],
        condition: { redoNodeId: 'plan' },
        fanOut: null,
        approvalGate: { id: 'approve-plan', prompt: 'Approve the reviewed plan before implementation.' },
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'implement',
        title: 'Implement',
        roleId: 'implementer',
        dependsOn: ['plan', 'plan-review'],
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
        dependsOn: ['plan', 'implement'],
        condition: { redoNodeId: 'implement' },
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'verify',
        title: 'Verify',
        roleId: 'verifier',
        dependsOn: ['review'],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'open-pr',
        title: 'Open PR',
        roleId: 'pr-author',
        dependsOn: ['implement', 'verify'],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'pr-review',
        title: 'PR review',
        roleId: 'pr-reviewer',
        dependsOn: ['implement', 'verify', 'open-pr'],
        condition: { redoNodeId: 'implement' },
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 2
      },
      {
        id: 'merge-pr',
        title: 'Merge PR or hand off',
        roleId: 'pr-merger',
        dependsOn: ['plan', 'open-pr', 'pr-review'],
        condition: { ownerMergeNodeId: 'plan' },
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

/** A saved sequence for work whose plan and ordered steps are already approved. */
export function savedWorkflowDefinition(template: SavedWorkflowTemplate): WorkflowDefinitionV1 {
  if (!template.id.trim() || !template.name.trim() || template.stages.length === 0) {
    throw new Error('Name the workflow and add at least one stage.');
  }
  if (template.id === 'task-plan-implement-review') {
    throw new Error('The built-in planning workflow id is reserved.');
  }
  const seen = new Set<string>();
  let lastPullRequest = -1;
  let lastPullRequestReview = -1;
  for (const [index, stage] of template.stages.entries()) {
    if (!stage.id.trim() || !stage.title.trim() || !stage.instructions.trim() || seen.has(stage.id)) {
      throw new Error('Each stage needs a unique id, title, and instructions.');
    }
    if (!['codex', 'claude', 'antigravity'].includes(stage.provider)) {
      throw new Error(`Choose a supported provider for ${stage.title}.`);
    }
    if (!['PlanReceipt', 'ImplementationReceipt', 'ReviewReceipt', 'SpecComplianceReceipt',
      'VerificationReceipt', 'PullRequestReceipt', 'PullRequestMergeReceipt'].includes(stage.outputContract)) {
      throw new Error(`Choose a supported result for ${stage.title}.`);
    }
    if (stage.outputContract === 'ReviewReceipt' && !stage.redoStageId) {
      throw new Error(`Choose a correction stage for ${stage.title}.`);
    }
    if (stage.redoStageId) {
      const target = template.stages.slice(0, index).find((item) => item.id === stage.redoStageId);
      if (stage.outputContract !== 'ReviewReceipt' || !target ||
        !['PlanReceipt', 'ImplementationReceipt'].includes(target.outputContract)) {
        throw new Error(`Choose an earlier plan or implementation stage for ${stage.title}.`);
      }
    }
    if (lastPullRequest >= 0 && !['PullRequestReceipt', 'ReviewReceipt', 'PullRequestMergeReceipt'].includes(stage.outputContract)) {
      throw new Error('After Open PR, only PR review and optional Merge stages may follow.');
    }
    if (stage.outputContract === 'PullRequestReceipt') {
      if (lastPullRequest >= 0) throw new Error('Use one Open PR stage; a correction reruns it.');
      const earlier = template.stages.slice(0, index);
      const implementation = earlier.findLastIndex((item) => item.outputContract === 'ImplementationReceipt');
      const review = earlier.findLastIndex((item) => item.outputContract === 'ReviewReceipt');
      const verification = earlier.findLastIndex((item) => item.outputContract === 'VerificationReceipt');
      if (implementation < 0 || review <= implementation || verification <= review) {
        throw new Error('Open PR follows implementation, review, and verification in that order.');
      }
      lastPullRequest = index;
    }
    if (stage.outputContract === 'ReviewReceipt' && lastPullRequest >= 0 && index > lastPullRequest) {
      lastPullRequestReview = index;
    }
    if (stage.outputContract === 'PullRequestMergeReceipt' &&
      (lastPullRequest < 0 || lastPullRequestReview <= lastPullRequest)) {
      throw new Error('Review the opened PR before a merge stage.');
    }
    if (stage.outputContract === 'PullRequestMergeReceipt' && index !== template.stages.length - 1) {
      throw new Error('Merge must be the last stage.');
    }
    seen.add(stage.id);
  }
  if (lastPullRequest >= 0 && lastPullRequestReview <= lastPullRequest) {
    throw new Error('Review the opened PR before this workflow can finish.');
  }

  const roles = template.stages.map((stage) => role(
    stage.id,
    stage.title,
    stage.instructions,
    stage.provider,
    stage.outputContract,
    ['ImplementationReceipt', 'PullRequestReceipt'].includes(stage.outputContract)
      ? { kind: 'shared-current', fileAllowList: [] }
      : { kind: 'read-only-current' }
  ));
  const nodes = template.stages.map((stage, index) => {
    const dependencies = new Set<string>();
    const earlier = template.stages.slice(0, index);
    if (index > 0) dependencies.add(template.stages[index - 1].id);
    if (stage.redoStageId) dependencies.add(stage.redoStageId);
    if (stage.outputContract === 'PullRequestReceipt') {
      for (const contract of ['ImplementationReceipt', 'VerificationReceipt']) {
        const source = earlier.findLast((item) => item.outputContract === contract);
        if (source) dependencies.add(source.id);
      }
    }
    if (stage.outputContract === 'ReviewReceipt' && lastPullRequest >= 0 && index > lastPullRequest) {
      dependencies.add(template.stages[lastPullRequest].id);
    }
    if (stage.outputContract === 'PullRequestMergeReceipt') {
      dependencies.add(template.stages[lastPullRequest].id);
      dependencies.add(template.stages[lastPullRequestReview].id);
    }
    return {
      id: stage.id,
      title: stage.title,
      roleId: stage.id,
      dependsOn: [...dependencies],
      condition: stage.redoStageId ? { redoNodeId: stage.redoStageId } : null,
      fanOut: null,
      approvalGate: stage.approvalPrompt?.trim()
        ? { id: `approve-${stage.id}`, prompt: stage.approvalPrompt.trim() }
        : null,
      timeoutSeconds: stage.outputContract === 'ImplementationReceipt' ? 7200 : 3600,
      maxAttempts: 2
    };
  });
  return {
    version: 1,
    id: template.id,
    name: template.name.trim(),
    description: 'A saved sequence for an approved plan.',
    trigger: { kind: 'manual' },
    inputs: [{ id: 'task', required: true }],
    roles,
    nodes,
    edges: [],
    concurrency: { global: 1, workflow: 1, providers: { codex: 1, claude: 1, antigravity: 1 } },
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
