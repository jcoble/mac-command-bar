import type {
  OrchestrationArtifact,
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

export type OrchestrationChip = {
  id: string;
  kind: string;
  label: string;
  title: string;
  href: string | null;
  path: string | null;
  status: string;
};

const retryPattern = /\b(retry|retries|retried|rerun|re-run)\b/i;
const approvalPattern = /\b(approval|approve|approved|signoff|sign-off|confirm|confirmation|decision|manual review)\b/i;

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
    retryCount: timeline.filter((item) => retryPattern.test(searchTextForTimelineItem(item))).length,
    approvalCount: timeline.filter((item) => approvalPattern.test(searchTextForTimelineItem(item))).length
  };
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

function searchTextForTimelineItem(item: OrchestrationTimelineItem): string {
  return [item.kind, item.status, item.title, item.summary, item.agentLabel]
    .filter(Boolean)
    .join(' ');
}
