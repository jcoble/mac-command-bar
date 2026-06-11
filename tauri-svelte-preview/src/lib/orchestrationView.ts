import type {
  OrchestrationArtifact,
  OrchestrationAgent,
  OrchestrationEvent,
  OrchestrationLink,
  OrchestrationRun,
  OrchestrationStep
} from './tauriSource';

export type OrchestrationStatusTone = 'bad' | 'attention' | 'good' | 'live' | 'idle';

export type OrchestrationRunMetrics = {
  agentCount: number;
  stepCount: number;
  artifactCount: number;
  linkCount: number;
  eventCount: number;
  runningCount: number;
  completedCount: number;
  failedCount: number;
  attentionCount: number;
  retryCount: number;
  approvalCount: number;
  decisionCount: number;
  scenarioCount: number;
  issueCount: number;
  testCount: number;
  retestCount: number;
  fixCount: number;
  resolvedCount: number;
  verifiedCount: number;
  delegationCount: number;
  handoffCount: number;
};

export type OrchestrationTimelineItem = {
  id: string;
  source: 'event' | 'step' | 'artifact';
  kind: string;
  status: string;
  tone: OrchestrationStatusTone;
  title: string;
  summary: string;
  agentLabel: string;
  timestamp: string | null;
  href: string | null;
  path: string | null;
};

export type OrchestrationAttentionItem = {
  id: string;
  tone: 'bad' | 'attention';
  label: string;
  title: string;
  summary: string;
  agentLabel: string;
  timestamp: string | null;
  href: string | null;
  path: string | null;
};

export type OrchestrationDecisionQueueItem = OrchestrationAttentionItem & {
  runID: string;
  runTitle: string;
  taskID: string | null;
  projectName: string;
};

export type OrchestrationAgentActivityItem = {
  id: string;
  tone: OrchestrationStatusTone;
  label: string;
  status: string;
  activity: string;
  detail: string;
  title: string;
  timestamp: string | null;
};

export type OrchestrationChip = {
  id: string;
  kind: string;
  label: string;
  title: string;
  href: string | null;
  path: string | null;
  status: string;
};

export type OrchestrationRunStage = {
  label: string;
  tone: OrchestrationStatusTone;
  title: string;
};

export type OrchestrationLoopStageMetric = {
  id:
    | 'scenario'
    | 'issue'
    | 'test'
    | 'retest'
    | 'fix'
    | 'resolved'
    | 'verified'
    | 'decision'
    | 'approval';
  label: string;
  value: number;
  tone: OrchestrationStatusTone;
  title: string;
};

type OrchestrationLoopKind =
  | 'scenario'
  | 'issue'
  | 'test'
  | 'retest'
  | 'fix'
  | 'resolved'
  | 'verified'
  | 'delegation'
  | 'handoff';

const retryPattern = /\b(retry|retries|retried|rerun|re-run)\b/i;
const approvalPattern = /\b(approval|approve|approved|signoff|sign-off|confirm|confirmation|decision|manual review)\b/i;
const decisionPattern = /\b(decision|manual review|needs input|needs sign-off|sign-off|required approval|approval required)\b/i;
const scenarioPattern = /\b(scenario|journey|workflow)\b/i;
const issuePattern = /\b(issue|finding|bug|defect)\b/i;
const testPattern = /\b(test|tested|testing|playwright|e2e|ui check|browser)\b/i;
const retestPattern = /\b(retest|re-test|retested|retry|rerun|re-run)\b/i;
const fixPattern = /\b(fix|fixed|repair|patch|resolve|resolved|auto-resolve|autoresolve)\b/i;
const resolvedPattern = /\b(resolved|fixed|closed|passed after fix|verified fix)\b/i;
const verifiedPattern = /\b(ui verified|browser verified|verified in ui|ui passed|validated in browser)\b/i;
const delegationPattern = /\b(delegated|assigned|sub-agent|subagent|fix batch|batch)\b/i;
const handoffPattern = /\b(handoff|handover|summary|report|artifact)\b/i;

export function orchestrationStatusTone(status: string): OrchestrationStatusTone {
  const normalized = status.trim().toLowerCase();
  if (!normalized) return 'idle';
  if (
    normalized.includes('fail') ||
    normalized.includes('error') ||
    normalized.includes('cancel') ||
    normalized === 'bad'
  ) {
    return 'bad';
  }
  if (
    normalized.includes('block') ||
    normalized.includes('wait') ||
    normalized.includes('approval') ||
    normalized.includes('confirm') ||
    normalized.includes('decision') ||
    normalized.includes('review')
  ) {
    return 'attention';
  }
  if (
    normalized.includes('success') ||
    normalized.includes('succeed') ||
    normalized.includes('complete') ||
    normalized.includes('pass') ||
    normalized.includes('done') ||
    normalized.includes('skip')
  ) {
    return 'good';
  }
  if (
    normalized.includes('run') ||
    normalized.includes('active') ||
    normalized.includes('work') ||
    normalized.includes('test') ||
    normalized.includes('fix') ||
    normalized.includes('progress')
  ) {
    return 'live';
  }
  return 'idle';
}

export function orchestrationRunMetrics(run: OrchestrationRun): OrchestrationRunMetrics {
  const timeline = orchestrationTimelineItems(run, Number.POSITIVE_INFINITY);
  const statuses = [
    run.status,
    ...run.agents.map((agent) => agent.status),
    ...run.steps.map((step) => step.status),
    ...run.artifacts.map((artifact) => artifact.status),
    ...run.events.map((event) => event.status)
  ];

  let runningCount = 0;
  let completedCount = 0;
  let failedCount = 0;
  let attentionCount = 0;

  for (const status of statuses) {
    switch (orchestrationStatusTone(status)) {
      case 'live':
        runningCount += 1;
        break;
      case 'good':
        completedCount += 1;
        break;
      case 'bad':
        failedCount += 1;
        break;
      case 'attention':
        attentionCount += 1;
        break;
      default:
        break;
    }
  }

  const timelineText = timeline.map(searchTextForTimelineItem);
  const loopKinds = timeline.map(orchestrationLoopKindForTimelineItem);

  return {
    agentCount: run.agents.length,
    stepCount: run.steps.length,
    artifactCount: run.artifacts.length,
    linkCount: run.links.length,
    eventCount: run.events.length,
    runningCount,
    completedCount,
    failedCount,
    attentionCount,
    retryCount: timelineText.filter((text) => retryPattern.test(text)).length,
    approvalCount: timelineText.filter((text) => approvalPattern.test(text)).length,
    decisionCount: timelineText.filter((text) => decisionPattern.test(text)).length,
    scenarioCount: loopKinds.filter((kind) => kind === 'scenario').length,
    issueCount: loopKinds.filter((kind) => kind === 'issue').length,
    testCount: loopKinds.filter((kind) => kind === 'test' || kind === 'retest').length,
    retestCount: loopKinds.filter((kind) => kind === 'retest').length,
    fixCount: loopKinds.filter((kind) => kind === 'fix').length,
    resolvedCount: loopKinds.filter((kind) => kind === 'resolved').length,
    verifiedCount: loopKinds.filter((kind) => kind === 'verified').length,
    delegationCount: loopKinds.filter((kind) => kind === 'delegation').length,
    handoffCount: loopKinds.filter((kind) => kind === 'handoff').length
  };
}

export function orchestrationLoopTallyText(metrics: OrchestrationRunMetrics): string {
  const parts = [
    formatMetricLabel(metrics.scenarioCount, 'scenario', 'scenarios'),
    formatMetricLabel(metrics.issueCount, 'issue', 'issues'),
    formatMetricLabel(metrics.testCount, 'test', 'tests'),
    formatMetricLabel(metrics.retestCount, 'retest', 'retests'),
    formatMetricLabel(metrics.fixCount, 'fix', 'fixes'),
    metrics.resolvedCount ? `${metrics.resolvedCount} resolved` : '',
    metrics.verifiedCount ? `${metrics.verifiedCount} UI verified` : '',
    formatMetricLabel(metrics.delegationCount, 'delegation', 'delegations'),
    formatMetricLabel(metrics.handoffCount, 'handoff', 'handoffs'),
    formatMetricLabel(metrics.decisionCount, 'decision', 'decisions'),
    metrics.approvalCount ? `${metrics.approvalCount} sign-off` : ''
  ].filter(Boolean);

  return parts.join(' · ') || 'no loop events yet';
}

export function orchestrationRunStage(
  run: OrchestrationRun,
  metrics = orchestrationRunMetrics(run)
): OrchestrationRunStage {
  if (metrics.failedCount > 0) {
    return {
      label: 'Blocked',
      tone: 'bad',
      title: formatMetricLabel(metrics.failedCount, 'failure', 'failures')
    };
  }

  if (metrics.decisionCount > 0 || metrics.approvalCount > 0) {
    const title = [
      formatMetricLabel(metrics.decisionCount, 'decision', 'decisions'),
      metrics.approvalCount ? `${metrics.approvalCount} sign-off` : ''
    ].filter(Boolean).join(' · ');

    return {
      label: metrics.approvalCount > 0 ? 'Needs sign-off' : 'Needs decision',
      tone: 'attention',
      title
    };
  }

  if (metrics.retestCount > 0) {
    return {
      label: 'Retesting',
      tone: 'live',
      title: `${metrics.retestCount} retest${metrics.retestCount === 1 ? '' : 's'}`
    };
  }

  if (metrics.fixCount > 0) {
    return {
      label: 'Fixing',
      tone: 'live',
      title: `${metrics.fixCount} fix${metrics.fixCount === 1 ? '' : 'es'}`
    };
  }

  if (metrics.testCount > 0) {
    return {
      label: 'Testing',
      tone: 'live',
      title: `${metrics.testCount} test${metrics.testCount === 1 ? '' : 's'}`
    };
  }

  if (metrics.verifiedCount > 0) {
    return {
      label: 'UI verified',
      tone: 'good',
      title: `${metrics.verifiedCount} UI verification${metrics.verifiedCount === 1 ? '' : 's'}`
    };
  }

  if (metrics.resolvedCount > 0) {
    return {
      label: 'Resolved',
      tone: 'good',
      title: `${metrics.resolvedCount} resolved`
    };
  }

  if (metrics.issueCount > 0) {
    return {
      label: 'Issues found',
      tone: 'attention',
      title: `${metrics.issueCount} issue${metrics.issueCount === 1 ? '' : 's'}`
    };
  }

  if (metrics.scenarioCount > 0) {
    return {
      label: 'Scenarios',
      tone: 'good',
      title: `${metrics.scenarioCount} scenario${metrics.scenarioCount === 1 ? '' : 's'}`
    };
  }

  const statusTone = orchestrationStatusTone(run.status);
  return {
    label: statusTone === 'good' ? 'Complete' : statusTone === 'live' ? 'Running' : 'Queued',
    tone: statusTone,
    title: run.summary || run.status || 'No orchestration activity yet'
  };
}

export function orchestrationLoopStageMetrics(
  metrics: OrchestrationRunMetrics
): OrchestrationLoopStageMetric[] {
  return [
    {
      id: 'scenario',
      label: 'Scen',
      value: metrics.scenarioCount,
      tone: metrics.scenarioCount > 0 ? 'good' : 'idle',
      title: stageMetricTitle(metrics.scenarioCount, 'scenario', 'scenarios', 'mapped')
    },
    {
      id: 'issue',
      label: 'Issue',
      value: metrics.issueCount,
      tone: metrics.issueCount > 0 ? 'attention' : 'idle',
      title: stageMetricTitle(metrics.issueCount, 'issue', 'issues', 'found')
    },
    {
      id: 'test',
      label: 'Test',
      value: metrics.testCount,
      tone: metrics.testCount > 0 ? 'live' : 'idle',
      title: stageMetricTitle(metrics.testCount, 'test', 'tests', 'run')
    },
    {
      id: 'retest',
      label: 'Retest',
      value: metrics.retestCount,
      tone: metrics.retestCount > 0 ? 'live' : 'idle',
      title: stageMetricTitle(metrics.retestCount, 'retest', 'retests', 'run')
    },
    {
      id: 'fix',
      label: 'Fix',
      value: metrics.fixCount,
      tone: metrics.fixCount > 0 ? 'live' : 'idle',
      title: stageMetricTitle(metrics.fixCount, 'fix', 'fixes', 'batched')
    },
    {
      id: 'resolved',
      label: 'Done',
      value: metrics.resolvedCount,
      tone: metrics.resolvedCount > 0 ? 'good' : 'idle',
      title: stageMetricTitle(metrics.resolvedCount, 'resolved', 'resolved', '')
    },
    {
      id: 'verified',
      label: 'UI',
      value: metrics.verifiedCount,
      tone: metrics.verifiedCount > 0 ? 'good' : 'idle',
      title: stageMetricTitle(metrics.verifiedCount, 'UI verification', 'UI verifications', 'passed')
    },
    {
      id: 'decision',
      label: 'Decide',
      value: metrics.decisionCount,
      tone: metrics.decisionCount > 0 ? 'attention' : 'idle',
      title: stageMetricTitle(metrics.decisionCount, 'decision', 'decisions', 'needed')
    },
    {
      id: 'approval',
      label: 'Sign',
      value: metrics.approvalCount,
      tone: metrics.approvalCount > 0 ? 'attention' : 'idle',
      title: stageMetricTitle(metrics.approvalCount, 'sign-off', 'sign-offs', 'needed')
    }
  ];
}

export function orchestrationTimelineItems(
  run: OrchestrationRun,
  limit = 8
): OrchestrationTimelineItem[] {
  const items = [
    ...run.events.map((event, index) => orchestrationEventTimelineItem(event, index)),
    ...run.steps.map((step, index) => orchestrationStepTimelineItem(run, step, index)),
    ...run.artifacts.map((artifact, index) => orchestrationArtifactTimelineItem(run, artifact, index))
  ];

  return items
    .map((item, index) => ({ item, index }))
    .sort((left, right) => {
      const timeDelta = timestampValue(right.item.timestamp) - timestampValue(left.item.timestamp);
      return timeDelta || left.index - right.index;
    })
    .slice(0, Math.max(0, limit))
    .map(({ item }) => item);
}

export function orchestrationCurrentActivity(run: OrchestrationRun): string {
  const timeline = orchestrationTimelineItems(run, Number.POSITIVE_INFINITY);
  const activeItem =
    timeline.find((item) => item.tone === 'attention') ??
    timeline.find((item) => item.tone === 'live') ??
    timeline[0];

  if (!activeItem) return run.summary || run.status || 'No activity yet';

  const agentPrefix = activeItem.agentLabel ? `${activeItem.agentLabel}: ` : '';
  const summarySuffix = activeItem.summary ? ` - ${activeItem.summary}` : '';
  return `${agentPrefix}${activeItem.title}${summarySuffix}`;
}

export function orchestrationAttentionQueue(
  run: OrchestrationRun,
  limit = 4
): OrchestrationAttentionItem[] {
  return orchestrationTimelineItems(run, Number.POSITIVE_INFINITY)
    .filter((item) => item.tone === 'bad' || item.tone === 'attention')
    .slice(0, Math.max(0, limit))
    .map((item) => ({
      id: item.id,
      tone: item.tone as 'bad' | 'attention',
      label: attentionLabelForTimelineItem(item),
      title: item.title,
      summary: item.summary,
      agentLabel: item.agentLabel,
      timestamp: item.timestamp,
      href: item.href,
      path: item.path
    }));
}

export function orchestrationDecisionQueueForRuns(
  runs: OrchestrationRun[],
  limit = 6
): OrchestrationDecisionQueueItem[] {
  return runs
    .flatMap((run) =>
      orchestrationAttentionQueue(run, Number.POSITIVE_INFINITY).map((item) => ({
        ...item,
        id: `${run.id}:${item.id}`,
        runID: run.id,
        runTitle: run.title,
        taskID: run.taskID,
        projectName: run.projectName
      }))
    )
    .sort((left, right) => {
      const toneDelta = orchestrationTonePriority(left.tone) - orchestrationTonePriority(right.tone);
      if (toneDelta) return toneDelta;

      const timeDelta = timestampValue(right.timestamp) - timestampValue(left.timestamp);
      if (timeDelta) return timeDelta;

      return left.runTitle.localeCompare(right.runTitle);
    })
    .slice(0, Math.max(0, limit));
}

export function orchestrationAgentActivityItems(
  run: OrchestrationRun,
  limit = 4
): OrchestrationAgentActivityItem[] {
  return run.agents
    .map((agent, index) => orchestrationAgentActivityItem(run, agent, index))
    .sort((left, right) => {
      const toneDelta = orchestrationTonePriority(left.tone) - orchestrationTonePriority(right.tone);
      if (toneDelta) return toneDelta;
      return timestampValue(right.timestamp) - timestampValue(left.timestamp);
    })
    .slice(0, Math.max(0, limit));
}

export function orchestrationRunSummaryText(run: OrchestrationRun): string {
  const metrics = orchestrationRunMetrics(run);
  const attentionQueue = orchestrationAttentionQueue(run, 3);
  const tally = [
    `${metrics.completedCount} done`,
    `${metrics.runningCount} running`,
    `${metrics.failedCount} failed`,
    `${metrics.attentionCount} attention`,
    `${metrics.retryCount} retries`,
    `${metrics.approvalCount} sign-off`
  ].join(' · ');
  const taskLine = run.taskID ? `Task: ${run.taskID}` : '';

  return [
    run.title,
    `Status: ${run.status} · ${run.progress}%`,
    `Project: ${run.projectName} · ${run.rootLabel}`,
    taskLine,
    `Phase: ${run.phase}`,
    `Current: ${orchestrationCurrentActivity(run)}`,
    `Tally: ${tally}`,
    `Loop: ${orchestrationLoopTallyText(metrics)}`,
    attentionQueue.length > 0 ? `Needs attention: ${attentionQueue.map((item) => item.title).join(' · ')}` : '',
    `Artifacts: ${metrics.artifactCount} · Links: ${metrics.linkCount} · Events: ${metrics.eventCount}`,
    orchestrationRunTimelineText(run) ? `Timeline:\n${orchestrationRunTimelineText(run)}` : ''
  ].filter(Boolean).join('\n');
}

export function orchestrationRunHandoffText(run: OrchestrationRun): string {
  const metrics = orchestrationRunMetrics(run);
  const stage = orchestrationRunStage(run, metrics);
  const attentionQueue = orchestrationAttentionQueue(run, 5);
  const agentItems = orchestrationAgentActivityItems(run, 8);
  const timeline = orchestrationRunTimelineText(run, 8);
  const artifactLines = run.artifacts.slice(0, 8).map((artifact) => {
    const target = artifact.path ?? artifact.url ?? artifact.status;
    return `- ${artifact.kind || 'artifact'} · ${artifact.title || 'Artifact'} · ${target}`;
  });
  const linkLines = run.links.slice(0, 8).map((link) => {
    return `- ${link.kind || 'link'} · ${link.label || link.kind || 'link'} · ${link.url}`;
  });
  const attentionLines = attentionQueue.map((item) => {
    const agent = item.agentLabel ? `${item.agentLabel} · ` : '';
    const summary = item.summary ? ` - ${item.summary}` : '';
    return `- ${item.label} · ${agent}${item.title}${summary}`;
  });
  const agentLines = agentItems.map((agent) => {
    const detail = agent.detail ? ` - ${agent.detail}` : '';
    return `- ${agent.label} · ${agent.status} · ${agent.activity}${detail}`;
  });

  return [
    'Orchestration run handoff',
    `Run: ${run.title}`,
    `ID: ${run.id}`,
    `Status: ${run.status} · ${run.progress}% · ${stage.label}`,
    `Project: ${run.projectName} · ${run.rootLabel}`,
    `Path: ${run.projectPath}`,
    `Task: ${run.taskID ?? 'none'}`,
    `Phase: ${run.phase}`,
    `Current: ${orchestrationCurrentActivity(run)}`,
    `Loop tally: ${orchestrationLoopTallyText(metrics)}`,
    `Counts: ${metrics.agentCount} agents · ${metrics.stepCount} steps · ${metrics.eventCount} events · ${metrics.artifactCount} artifacts · ${metrics.linkCount} links`,
    '',
    'Needs attention:',
    attentionLines.length > 0 ? attentionLines.join('\n') : '- none',
    '',
    'Agents:',
    agentLines.length > 0 ? agentLines.join('\n') : '- none',
    '',
    'Recent timeline:',
    timeline || '- none',
    '',
    'Artifacts:',
    artifactLines.length > 0 ? artifactLines.join('\n') : '- none',
    '',
    'Links:',
    linkLines.length > 0 ? linkLines.join('\n') : '- none'
  ].join('\n');
}

export function orchestrationRunTimelineText(run: OrchestrationRun, limit = 6): string {
  return orchestrationTimelineItems(run, limit)
    .map((item) => {
      const agent = item.agentLabel ? `${item.agentLabel} · ` : '';
      const summary = item.summary ? ` - ${item.summary}` : '';
      return `- ${item.tone} · ${agent}${item.title}${summary}`;
    })
    .join('\n');
}

export function orchestrationArtifactChips(run: OrchestrationRun, limit = 4): OrchestrationChip[] {
  return run.artifacts.slice(0, Math.max(0, limit)).map((artifact) => ({
    id: artifact.id,
    kind: artifact.kind || 'artifact',
    label: artifact.kind || 'artifact',
    title: artifact.title,
    href: artifact.url,
    path: artifact.path,
    status: artifact.status
  }));
}

export function orchestrationLinkChips(run: OrchestrationRun, limit = 4): OrchestrationChip[] {
  return run.links.slice(0, Math.max(0, limit)).map((link, index) => ({
    id: `${link.kind}:${link.url}:${index}`,
    kind: link.kind || 'link',
    label: link.label || link.kind || 'link',
    title: link.url,
    href: link.url,
    path: null,
    status: 'linked'
  }));
}

function orchestrationEventTimelineItem(
  event: OrchestrationEvent,
  index: number
): OrchestrationTimelineItem {
  const title = event.title ?? event.message ?? titleFromKind(event.kind);
  return {
    id: event.id || `${event.runId}:event:${index}`,
    source: 'event',
    kind: event.kind || 'event',
    status: event.status || 'updated',
    tone: orchestrationStatusTone(event.status),
    title,
    summary: event.message ?? event.stepKind ?? event.artifactKind ?? '',
    agentLabel: formatAgentLabel(event.agentProvider, event.agentRole, event.agentId),
    timestamp: event.timestamp,
    href: event.artifactUrl ?? event.linkUrl,
    path: event.artifactPath
  };
}

function orchestrationStepTimelineItem(
  run: OrchestrationRun,
  step: OrchestrationStep,
  index: number
): OrchestrationTimelineItem {
  return {
    id: step.id || `${run.id}:step:${index}`,
    source: 'step',
    kind: step.kind || 'step',
    status: step.status || 'pending',
    tone: orchestrationStatusTone(step.status),
    title: step.title || titleFromKind(step.kind),
    summary: step.summary,
    agentLabel: formatAgentLabel(null, null, step.agentId),
    timestamp: step.finishedAt ?? step.startedAt,
    href: null,
    path: null
  };
}

function orchestrationArtifactTimelineItem(
  run: OrchestrationRun,
  artifact: OrchestrationArtifact,
  index: number
): OrchestrationTimelineItem {
  return {
    id: artifact.id || `${run.id}:artifact:${index}`,
    source: 'artifact',
    kind: artifact.kind || 'artifact',
    status: artifact.status || 'available',
    tone: orchestrationStatusTone(artifact.status),
    title: artifact.title || artifact.kind || 'Artifact',
    summary: artifact.path ?? artifact.url ?? '',
    agentLabel: '',
    timestamp: null,
    href: artifact.url,
    path: artifact.path
  };
}

function orchestrationAgentActivityItem(
  run: OrchestrationRun,
  agent: OrchestrationAgent,
  index: number
): OrchestrationAgentActivityItem {
  const latestItem = latestAgentTimelineItem(run, agent);
  const agentTone = orchestrationStatusTone(agent.status);
  const tone =
    agentTone === 'bad' || agentTone === 'attention' || agentTone === 'live'
      ? agentTone
      : latestItem?.tone ?? agentTone;
  const label =
    [agent.provider, agent.role].map((value) => value.trim()).filter(Boolean).join(' ') ||
    agent.title ||
    agent.id ||
    `agent ${index + 1}`;
  const status = agent.status || latestItem?.status || 'unknown';
  const activity = latestItem?.title || agent.title || titleFromKind(agent.role || agent.provider || status);
  const detail = latestItem?.summary || status;
  const timestamp = latestItem?.timestamp ?? agent.lastActivity;
  const title = [label, status, activity, detail].filter(Boolean).join('\n');

  return {
    id: agent.id || `${run.id}:agent:${index}`,
    tone,
    label,
    status,
    activity,
    detail,
    title,
    timestamp
  };
}

function latestAgentTimelineItem(
  run: OrchestrationRun,
  agent: OrchestrationAgent
): OrchestrationTimelineItem | null {
  const items = [
    ...run.events
      .filter((event) => eventMatchesAgent(event, agent))
      .map((event, index) => orchestrationEventTimelineItem(event, index)),
    ...run.steps
      .filter((step) => step.agentId === agent.id)
      .map((step, index) => orchestrationStepTimelineItem(run, step, index))
  ];

  return items.sort((left, right) => timestampValue(right.timestamp) - timestampValue(left.timestamp))[0] ?? null;
}

function eventMatchesAgent(event: OrchestrationEvent, agent: OrchestrationAgent): boolean {
  if (event.agentId && event.agentId === agent.id) return true;
  if (event.agentProvider && event.agentRole) {
    return event.agentProvider === agent.provider && event.agentRole === agent.role;
  }
  return false;
}

function formatAgentLabel(
  provider: string | null | undefined,
  role: string | null | undefined,
  id: string | null | undefined
): string {
  return [provider, role, id].map((value) => String(value ?? '').trim()).filter(Boolean).join(' ');
}

function titleFromKind(kind: string): string {
  const words = kind
    .split(/[._:-]+/)
    .map((word) => word.trim())
    .filter(Boolean);
  if (words.length === 0) return 'Run update';
  return words.map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
}

function timestampValue(value: string | null): number {
  if (!value) return 0;
  const numericValue = Number(value);
  const date = Number.isFinite(numericValue) ? new Date(numericValue) : new Date(value);
  return Number.isNaN(date.getTime()) ? 0 : date.getTime();
}

function orchestrationTonePriority(tone: OrchestrationStatusTone): number {
  switch (tone) {
    case 'bad':
      return 0;
    case 'attention':
      return 1;
    case 'live':
      return 2;
    case 'idle':
      return 3;
    case 'good':
      return 4;
  }
}

function searchTextForTimelineItem(item: OrchestrationTimelineItem): string {
  return [item.kind, item.status, item.title, item.summary, item.agentLabel]
    .filter(Boolean)
    .join(' ');
}

function attentionLabelForTimelineItem(item: OrchestrationTimelineItem): string {
  if (item.tone === 'bad') return 'Blocker';
  if (decisionPattern.test(searchTextForTimelineItem(item))) return 'Decision';
  if (approvalPattern.test(searchTextForTimelineItem(item))) return 'Sign-off';
  return 'Attention';
}

function formatMetricLabel(count: number, singular: string, plural: string): string {
  if (count <= 0) return '';
  return `${count} ${count === 1 ? singular : plural}`;
}

function stageMetricTitle(count: number, singular: string, plural: string, suffix: string): string {
  const label = count === 1 ? singular : plural;
  return `${count} ${label}${suffix ? ` ${suffix}` : ''}`;
}

function orchestrationLoopKindForTimelineItem(item: OrchestrationTimelineItem): OrchestrationLoopKind | null {
  const kindTitleStatus = [item.kind, item.title, item.status].filter(Boolean).join(' ');
  const allText = searchTextForTimelineItem(item);

  if (handoffPattern.test(kindTitleStatus)) return 'handoff';
  if (verifiedPattern.test(kindTitleStatus)) return 'verified';
  if (retestPattern.test(kindTitleStatus)) return 'retest';
  if (issuePattern.test(kindTitleStatus)) return 'issue';
  if (testPattern.test(kindTitleStatus)) return 'test';
  if (delegationPattern.test(kindTitleStatus)) return 'delegation';
  if (resolvedPattern.test(allText)) return 'resolved';
  if (fixPattern.test(kindTitleStatus)) return 'fix';
  if (scenarioPattern.test(kindTitleStatus)) return 'scenario';
  return null;
}
