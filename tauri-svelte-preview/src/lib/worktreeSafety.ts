import type { ProjectWorktree } from './tauriSource';

export type WorktreeSafetyKind = 'protected' | 'blocked' | 'ready' | 'review';
export type WorktreeAgeBucket = 'active' | 'stale' | 'unknown';

export type WorktreeSafetySummary = {
  kind: WorktreeSafetyKind;
  ageBucket: WorktreeAgeBucket;
  badge: string;
  reason: string;
  recommendation: string;
  activityLabel: string;
  auditCommand: string;
  backupCommand: string;
  cleanupCommand: string;
  cleanupPlan: string;
};

export type WorktreeSafetyOptions = {
  primaryPath?: string | null;
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
  } else if (worktree.isDirty || deleteEligibility.includes('dirty')) {
    kind = 'blocked';
    badge = 'Dirty';
    reason = 'Uncommitted changes';
    recommendation = 'Open it, review git status, then commit or stash before cleanup.';
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
      isPrimaryCheckout
    })
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
  const timestamp = backupTimestamp(normalizeNow(now));
  return [
    'git',
    '-C',
    shellQuote(worktree.path),
    'stash',
    'push',
    '--include-untracked',
    '-m',
    shellQuote(`mcb backup ${worktree.branch} ${timestamp}`)
  ].join(' ');
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
  }
): string {
  const lines = [
    `Worktree: ${worktree.branch}`,
    `Path: ${worktree.path}`,
    `Repo: ${worktree.repo}`,
    worktree.taskID ? `Task: ${worktree.taskID}` : '',
    `State: ${details.reason}`,
    `Recommended: ${details.recommendation}`,
    ''
  ].filter(Boolean);

  if (details.isPrimaryCheckout) {
    lines.push('Audit current state:');
    lines.push(details.auditCommand);
    lines.push('');
    lines.push('Do not remove the primary checkout from the worktree list.');
  } else {
    lines.push('Audit before cleanup:');
    lines.push(details.auditCommand);
    lines.push('');
    lines.push('Backup dirty/untracked work if needed:');
    lines.push(details.backupCommand);
    lines.push('');
    lines.push('Remove once clean and no active session owns it:');
    lines.push(details.cleanupCommand);
  }

  return lines.join('\n');
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
