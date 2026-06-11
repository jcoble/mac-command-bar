import type { ProjectWorktree } from './tauriSource';

export type WorktreeSafetyKind = 'protected' | 'blocked' | 'ready' | 'review';
export type WorktreeAgeBucket = 'active' | 'stale' | 'unknown';

export type WorktreeSafetySummary = {
  kind: WorktreeSafetyKind;
  ageBucket: WorktreeAgeBucket;
  activeSessionCount: number;
  badge: string;
  reason: string;
  recommendation: string;
  activityLabel: string;
  auditCommand: string;
  backupCommand: string;
  cleanupCommand: string;
  cleanupPlan: string;
};

export type WorktreePrimaryActionKind = 'audit' | 'backup' | 'cleanup';

export type WorktreePrimaryAction = {
  kind: WorktreePrimaryActionKind;
  label: string;
  title: string;
  command: string;
  clipboardMessage: string;
};

export type WorktreeCleanupBrief = {
  headline: string;
  cleanupCandidateCount: number;
  blockedCount: number;
  reviewCount: number;
  protectedCount: number;
  staleCount: number;
  taskIDs: string[];
  report: string;
};

export type WorktreeSafetyOptions = {
  primaryPath?: string | null;
  activeSessionPaths?: Array<string | null | undefined>;
  now?: number | Date;
  staleAfterDays?: number;
};

const defaultStaleAfterDays = 14;

export function buildWorktreeSafetySummary(
  worktree: ProjectWorktree,
  options: WorktreeSafetyOptions = {}
): WorktreeSafetySummary {
  const nowMs = normalizeNow(options.now);
  const staleAfterDays = options.staleAfterDays ?? defaultStaleAfterDays;
  const activity = worktreeActivity(worktree.lastActivity, nowMs, staleAfterDays);
  const primaryPath = options.primaryPath ? normalizePath(options.primaryPath) : '';
  const worktreePath = normalizePath(worktree.path);
  const isPrimaryCheckout = Boolean(primaryPath && primaryPath === worktreePath);
  const activeSessionCount = countActiveSessionPaths(worktreePath, options.activeSessionPaths);
  const deleteEligibility = worktree.deleteEligibility.toLowerCase();

  let kind: WorktreeSafetyKind = 'ready';
  let badge = 'Safe';
  let reason = 'Clean worktree';
  let recommendation = 'Review branch ownership, then remove from the main checkout when finished.';

  if (isPrimaryCheckout) {
    kind = 'protected';
    badge = 'Main';
    reason = 'Primary checkout';
    recommendation = 'Keep this worktree as the repo anchor; clean branches from sibling worktrees instead.';
  } else if (activeSessionCount > 0) {
    kind = 'blocked';
    badge = 'Active';
    reason = `Active ${activeSessionCount === 1 ? 'session' : 'sessions'} owns this worktree`;
    recommendation = 'Resume, close, or move active agent sessions before cleanup.';
  } else if (worktree.isDirty || deleteEligibility.includes('dirty')) {
    kind = 'blocked';
    badge = 'Dirty';
    reason = 'Uncommitted changes';
    recommendation = 'Open it, review git status, then archive, commit, or stash before cleanup.';
  } else if (worktree.hasUnmergedCommits || deleteEligibility.includes('unmerged')) {
    kind = 'blocked';
    badge = 'Unmerged';
    reason = 'Local commits not on a remote branch';
    recommendation = 'Inspect the commit list and merge, push, cherry-pick, or intentionally archive before cleanup.';
  } else if (activity.ageBucket === 'stale') {
    kind = 'ready';
    badge = 'Stale';
    reason = 'Clean and inactive';
    recommendation = 'Good cleanup candidate after confirming no active agent owns it.';
  } else if (deleteEligibility.includes('confirmation') || deleteEligibility.includes('unknown')) {
    kind = 'review';
    badge = 'Review';
    reason = 'Clean, needs confirmation';
    recommendation = 'Confirm no session owns it, then remove it from the main checkout.';
  }

  const auditCommand = worktreeAuditCommand(worktree);
  const backupCommand = worktreeBackupCommand(worktree, nowMs);
  const cleanupCommand = worktreeCleanupCommand(worktree, options.primaryPath);

  return {
    kind,
    ageBucket: activity.ageBucket,
    activeSessionCount,
    badge,
    reason,
    recommendation,
    activityLabel: activity.label,
    auditCommand,
    backupCommand,
    cleanupCommand,
    cleanupPlan: worktreeCleanupPlan(worktree, {
      auditCommand,
      backupCommand,
      cleanupCommand,
      reason,
      recommendation,
      isPrimaryCheckout,
      activeSessionCount
    })
  };
}

export function buildWorktreeCleanupBrief(
  worktrees: ProjectWorktree[],
  options: WorktreeSafetyOptions = {}
): WorktreeCleanupBrief {
  const entries = worktrees.map((worktree) => ({
    worktree,
    safety: buildWorktreeSafetySummary(worktree, options)
  }));
  const blocked = entries.filter((entry) => entry.safety.kind === 'blocked');
  const ready = entries.filter((entry) => entry.safety.kind === 'ready');
  const review = entries.filter((entry) => entry.safety.kind === 'review');
  const protectedEntries = entries.filter((entry) => entry.safety.kind === 'protected');
  const stale = entries.filter((entry) => entry.safety.ageBucket === 'stale');
  const taskIDs = uniqueTaskIDs(entries.map((entry) => entry.worktree.taskID));
  const headline = formatBriefHeadline({
    blocked: blocked.length,
    ready: ready.length,
    review: review.length,
    stale: stale.length,
    protected: protectedEntries.length
  });

  return {
    headline,
    cleanupCandidateCount: ready.length,
    blockedCount: blocked.length,
    reviewCount: review.length,
    protectedCount: protectedEntries.length,
    staleCount: stale.length,
    taskIDs,
    report: formatCleanupBriefReport({
      headline,
      taskIDs,
      blocked,
      ready,
      review,
      protectedEntries
    })
  };
}

export function prioritizeWorktreesForCleanup(
  worktrees: ProjectWorktree[],
  options: WorktreeSafetyOptions = {}
): ProjectWorktree[] {
  return worktrees
    .map((worktree, index) => ({
      worktree,
      safety: buildWorktreeSafetySummary(worktree, options),
      index
    }))
    .sort((left, right) => {
      const priorityDelta = worktreeCleanupPriority(left.safety) - worktreeCleanupPriority(right.safety);
      if (priorityDelta) return priorityDelta;

      const activityDelta = worktreeActivitySortValue(left.worktree) - worktreeActivitySortValue(right.worktree);
      if (activityDelta) return activityDelta;

      const branchDelta = left.worktree.branch.localeCompare(right.worktree.branch);
      if (branchDelta) return branchDelta;

      return left.index - right.index;
    })
    .map((entry) => entry.worktree);
}

export function buildWorktreeCleanupScript(
  worktrees: ProjectWorktree[],
  options: WorktreeSafetyOptions = {}
): string {
  const entries = worktrees.map((worktree) => ({
    worktree,
    safety: buildWorktreeSafetySummary(worktree, options)
  }));
  const brief = buildWorktreeCleanupBrief(worktrees, options);
  const lines = [
    '#!/usr/bin/env bash',
    'set -euo pipefail',
    '',
    'RUN_BACKUP="${RUN_BACKUP:-0}"',
    'RUN_REMOVE="${RUN_REMOVE:-0}"',
    '',
    `echo ${shellQuote(`Worktree cleanup: ${brief.headline}`)}`,
    ''
  ];

  for (const { worktree, safety } of entries) {
    const branch = worktree.branch || worktree.path;
    lines.push(`# ${branch} - ${safety.reason}`);
    lines.push(`echo ${shellQuote(`Audit ${branch}`)}`);
    lines.push(safety.auditCommand);

    if (safety.kind === 'blocked' && safety.activeSessionCount === 0) {
      lines.push('if [ "$RUN_BACKUP" = "1" ]; then');
      lines.push(`  echo ${shellQuote(`Backup ${branch}`)}`);
      lines.push(`  ${safety.backupCommand}`);
      lines.push('else');
      lines.push(`  echo ${shellQuote(`Backup ${branch}: set RUN_BACKUP=1 to execute`)}`);
      lines.push(`  printf '%s\\n' ${shellQuote(safety.backupCommand)}`);
      lines.push('fi');
    } else if (safety.kind === 'ready') {
      lines.push('if [ "$RUN_REMOVE" = "1" ]; then');
      lines.push(`  echo ${shellQuote(`Remove ${branch}`)}`);
      lines.push(`  ${safety.cleanupCommand}`);
      lines.push('else');
      lines.push(`  echo ${shellQuote(`Remove ${branch}: set RUN_REMOVE=1 to execute`)}`);
      lines.push(`  printf '%s\\n' ${shellQuote(safety.cleanupCommand)}`);
      lines.push('fi');
    } else {
      lines.push(`echo ${shellQuote(`Review ${branch}: ${safety.recommendation}`)}`);
    }

    lines.push('');
  }

  lines.push('echo "Dry run complete. Set RUN_BACKUP=1 and/or RUN_REMOVE=1 to execute guarded actions."');
  return `${lines.join('\n')}\n`;
}

export function worktreePrimaryAction(summary: WorktreeSafetySummary): WorktreePrimaryAction {
  if (summary.kind === 'ready') {
    return {
      kind: 'cleanup',
      label: 'Remove',
      title: 'Copy the safe remove command for this clean worktree.',
      command: summary.cleanupCommand,
      clipboardMessage: 'Recommended worktree remove command copied'
    };
  }

  if (summary.kind === 'blocked' && summary.activeSessionCount === 0) {
    return {
      kind: 'backup',
      label: 'Backup',
      title: 'Archive a recoverable backup before deciding whether to remove this worktree.',
      command: summary.backupCommand,
      clipboardMessage: 'Recommended worktree backup command copied'
    };
  }

  const activeReason = summary.activeSessionCount > 0 ? ' while active sessions still point here' : '';
  return {
    kind: 'audit',
    label: 'Audit',
    title: `Copy the audit command before changing this worktree${activeReason}.`,
    command: summary.auditCommand,
    clipboardMessage: 'Recommended worktree audit command copied'
  };
}

export function worktreeAuditCommand(worktree: ProjectWorktree): string {
  return [
    'git',
    '-C',
    shellQuote(worktree.path),
    'status',
    '--short',
    '--branch',
    '&&',
    'git',
    '-C',
    shellQuote(worktree.path),
    'log',
    '--oneline',
    '--decorate',
    '--max-count=8'
  ].join(' ');
}

export function worktreeBackupCommand(worktree: ProjectWorktree, now: number | Date = Date.now()): string {
  const nowMs = normalizeNow(now);
  const timestamp = backupTimestamp(nowMs);
  const archiveDirectory = worktreeArchiveDirectory(worktree, nowMs);
  const gitInWorktree = ['git', '-C', shellQuote(worktree.path)].join(' ');

  return [
    `mkdir -p ${shellQuote(archiveDirectory)}`,
    `${gitInWorktree} status --short --branch > ${shellQuote(`${archiveDirectory}/status.txt`)}`,
    `${gitInWorktree} log --oneline --decorate --max-count=40 > ${shellQuote(`${archiveDirectory}/commits.txt`)}`,
    `${gitInWorktree} diff --binary > ${shellQuote(`${archiveDirectory}/unstaged.patch`)}`,
    `${gitInWorktree} diff --cached --binary > ${shellQuote(`${archiveDirectory}/staged.patch`)}`,
    `${gitInWorktree} ls-files --others --exclude-standard > ${shellQuote(`${archiveDirectory}/untracked.txt`)}`,
    `${gitInWorktree} bundle create ${shellQuote(`${archiveDirectory}/head.bundle`)} HEAD`,
    `${gitInWorktree} stash push --include-untracked -m ${shellQuote(`mcb backup ${worktree.branch} ${timestamp}`)}`
  ].join(' && ');
}

export function worktreeCleanupCommand(
  worktree: ProjectWorktree,
  primaryPath: string | null | undefined
): string {
  if (!primaryPath || normalizePath(primaryPath) === normalizePath(worktree.path)) {
    return '# Open the main checkout first, then run git worktree remove for this sibling path.';
  }

  return [
    'git',
    '-C',
    shellQuote(primaryPath),
    'worktree',
    'remove',
    shellQuote(worktree.path),
    '&&',
    'git',
    '-C',
    shellQuote(primaryPath),
    'worktree',
    'prune'
  ].join(' ');
}

function worktreeCleanupPlan(
  worktree: ProjectWorktree,
  details: {
    auditCommand: string;
    backupCommand: string;
    cleanupCommand: string;
    reason: string;
    recommendation: string;
    isPrimaryCheckout: boolean;
    activeSessionCount: number;
  }
): string {
  const lines = [
    `Worktree: ${worktree.branch}`,
    `Path: ${worktree.path}`,
    `Repo: ${worktree.repo}`,
    worktree.taskID ? `Task: ${worktree.taskID}` : '',
    `State: ${details.reason}`,
    details.activeSessionCount > 0 ? `Active sessions: ${details.activeSessionCount}` : '',
    `Recommended: ${details.recommendation}`,
    ''
  ].filter(Boolean);

  if (details.isPrimaryCheckout) {
    lines.push('Audit current state:');
    lines.push(details.auditCommand);
    lines.push('');
    lines.push('Do not remove the primary checkout from the worktree list.');
  } else {
    if (details.activeSessionCount > 0) {
      lines.push('Do not remove while active sessions point here. Resume, close, or move them first.');
      lines.push('');
    }
    lines.push('Audit before cleanup:');
    lines.push(details.auditCommand);
    lines.push('');
    lines.push('Archive a recoverable backup before cleanup:');
    lines.push(details.backupCommand);
    lines.push('');
    lines.push('Remove once clean and no active session owns it:');
    lines.push(details.cleanupCommand);
  }

  return lines.join('\n');
}

function formatBriefHeadline(counts: {
  blocked: number;
  ready: number;
  review: number;
  stale: number;
  protected: number;
}): string {
  const parts = [
    counts.blocked ? `${counts.blocked} blocked` : '',
    counts.ready ? `${counts.ready} ready` : '',
    counts.review ? `${counts.review} review` : '',
    counts.stale ? `${counts.stale} stale` : '',
    counts.protected ? `${counts.protected} main` : ''
  ].filter(Boolean);

  return parts.join(' · ') || 'no worktrees';
}

function worktreeCleanupPriority(safety: WorktreeSafetySummary): number {
  if (safety.kind === 'blocked' && safety.activeSessionCount > 0) return 0;
  if (safety.kind === 'blocked' && safety.badge === 'Dirty') return 1;
  if (safety.kind === 'blocked' && safety.badge === 'Unmerged') return 2;
  if (safety.kind === 'blocked') return 3;
  if (safety.kind === 'ready' && safety.ageBucket === 'stale') return 4;
  if (safety.kind === 'ready') return 5;
  if (safety.kind === 'review') return 6;
  if (safety.kind === 'protected') return 7;
  return 8;
}

function worktreeActivitySortValue(worktree: ProjectWorktree): number {
  if (!worktree.lastActivity) return Number.POSITIVE_INFINITY;
  const value = new Date(worktree.lastActivity).getTime();
  return Number.isNaN(value) ? Number.POSITIVE_INFINITY : value;
}

function formatCleanupBriefReport(details: {
  headline: string;
  taskIDs: string[];
  blocked: Array<{ worktree: ProjectWorktree; safety: WorktreeSafetySummary }>;
  ready: Array<{ worktree: ProjectWorktree; safety: WorktreeSafetySummary }>;
  review: Array<{ worktree: ProjectWorktree; safety: WorktreeSafetySummary }>;
  protectedEntries: Array<{ worktree: ProjectWorktree; safety: WorktreeSafetySummary }>;
}): string {
  const lines = [
    'Worktree cleanup brief',
    details.headline,
    details.taskIDs.length > 0 ? `Tasks: ${details.taskIDs.join(', ')}` : '',
    `Blocked: ${details.blocked.length}`,
    `Cleanup candidates: ${details.ready.length}`,
    `Review: ${details.review.length}`,
    ''
  ].filter(Boolean);

  appendBriefSection(lines, 'Needs attention:', details.blocked);
  appendBriefSection(lines, 'Cleanup candidates:', details.ready);
  appendBriefSection(lines, 'Review:', details.review);
  appendBriefSection(lines, 'Protected main checkouts:', details.protectedEntries);

  return lines.join('\n');
}

function appendBriefSection(
  lines: string[],
  title: string,
  entries: Array<{ worktree: ProjectWorktree; safety: WorktreeSafetySummary }>
): void {
  if (entries.length === 0) return;

  lines.push(title);
  for (const { worktree, safety } of entries) {
    const task = worktree.taskID ? ` · ${worktree.taskID}` : '';
    lines.push(`- ${worktree.branch}${task}: ${safety.reason} · ${safety.recommendation}`);
    lines.push(`  ${worktree.path}`);
  }
  lines.push('');
}

function uniqueTaskIDs(values: Array<string | null | undefined>): string[] {
  const seen = new Set<string>();
  const taskIDs: string[] = [];

  for (const value of values) {
    const taskID = value?.trim();
    if (!taskID || seen.has(taskID)) continue;

    seen.add(taskID);
    taskIDs.push(taskID);
  }

  return taskIDs;
}

function countActiveSessionPaths(
  worktreePath: string,
  activeSessionPaths: Array<string | null | undefined> | undefined
): number {
  if (!activeSessionPaths || activeSessionPaths.length === 0) return 0;

  return activeSessionPaths.filter((path) => {
    if (!path) return false;
    const sessionPath = normalizePath(path);
    return sessionPath === worktreePath || sessionPath.startsWith(`${worktreePath}/`);
  }).length;
}

function worktreeActivity(
  lastActivity: string | null,
  nowMs: number,
  staleAfterDays: number
): { label: string; ageBucket: WorktreeAgeBucket } {
  if (!lastActivity) return { label: 'activity unknown', ageBucket: 'unknown' };

  const epochMs = new Date(lastActivity).getTime();
  if (Number.isNaN(epochMs)) return { label: lastActivity, ageBucket: 'unknown' };

  const elapsedMs = Math.max(0, nowMs - epochMs);
  const dayMs = 24 * 60 * 60 * 1000;
  const ageLabel = formatRelativeAge(elapsedMs);
  const ageBucket = elapsedMs >= staleAfterDays * dayMs ? 'stale' : 'active';
  return { label: ageLabel === 'just now' ? 'active just now' : `active ${ageLabel} ago`, ageBucket };
}

function formatRelativeAge(elapsedMs: number): string {
  const minuteMs = 60 * 1000;
  const hourMs = 60 * minuteMs;
  const dayMs = 24 * hourMs;

  if (elapsedMs < minuteMs) return 'just now';
  if (elapsedMs < hourMs) return `${Math.floor(elapsedMs / minuteMs)}m`;
  if (elapsedMs < dayMs) return `${Math.floor(elapsedMs / hourMs)}h`;
  return `${Math.floor(elapsedMs / dayMs)}d`;
}

function backupTimestamp(nowMs: number): string {
  return new Date(nowMs).toISOString().replace(/[:.]/g, '-');
}

function worktreeArchiveDirectory(worktree: ProjectWorktree, nowMs: number): string {
  const repoSegment = safePathSegment(worktree.repo || 'repo');
  const branchSegment = safePathSegment(worktree.branch || 'worktree');
  const archiveName = safePathSegment(`${repoSegment}-${branchSegment}-${backupTimestamp(nowMs)}`);
  return `${worktreeArchiveRoot(worktree.path)}/${repoSegment}/${archiveName}`;
}

function worktreeArchiveRoot(worktreePath: string): string {
  const normalizedPath = normalizePath(worktreePath);
  const marker = '/worktrees/';
  const markerIndex = normalizedPath.indexOf(marker);
  if (markerIndex >= 0) return `${normalizedPath.slice(0, markerIndex)}/worktree-archives`;

  return `${parentPath(normalizedPath)}/worktree-archives`;
}

function parentPath(path: string): string {
  const trimmed = path.replace(/\/+$/, '');
  const lastSlash = trimmed.lastIndexOf('/');
  if (lastSlash <= 0) return '.';
  return trimmed.slice(0, lastSlash);
}

function safePathSegment(value: string): string {
  return value.trim().replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '') || 'worktree';
}

function normalizeNow(now: number | Date | undefined): number {
  if (now instanceof Date) return now.getTime();
  return typeof now === 'number' ? now : Date.now();
}

function normalizePath(path: string): string {
  return path.replace(/\/+$/, '');
}

function shellQuote(value: string): string {
  return `'${value.replace(/'/g, "'\\''")}'`;
}
