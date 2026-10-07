/**
 * Pure view-model for the My Work rail. Components own drawing and interaction;
 * this module owns the durable option shape plus filtering, sorting and grouping.
 */
import { resolveOwnedSessionProject, type OwnedSession } from '../ownedSessions.ts';

export type MyWorkSort = 'recent' | 'name';
export type MyWorkSortDirection = 'asc' | 'desc';
export type MyWorkStatus = 'working' | 'done' | 'settled';

export interface MyWorkViewOptions {
  groupByProject: boolean;
  groupByStatus: boolean;
  sortBy: MyWorkSort;
  sortDirection: MyWorkSortDirection;
}

export interface MyWorkGroup {
  key: string;
  label: string;
  sessions: OwnedSession[];
  /** Project sections inside a status, when both groupings are on. */
  subgroups?: MyWorkGroup[];
}

/** Selected option values per filter group id; an empty list filters nothing. */
export type MyWorkFilters = Record<string, string[]>;

export const MY_WORK_STATUSES: readonly MyWorkStatus[] = ['working', 'done', 'settled'];
/**
 * Which way round each sort starts.
 *
 * Both directions are worth having, but only one of them is what a person means
 * by the sort's own name: "recent activity" means the newest at the top, and
 * "name" means A first. Picking a sort therefore returns to its own natural
 * direction rather than carrying over the other sort's.
 */
export const NATURAL_SORT_DIRECTION: Record<MyWorkSort, MyWorkSortDirection> = {
  recent: 'desc',
  name: 'asc'
};

export const DEFAULT_MY_WORK_VIEW_OPTIONS: MyWorkViewOptions = {
  groupByProject: false,
  groupByStatus: true,
  sortBy: 'recent',
  sortDirection: NATURAL_SORT_DIRECTION.recent
};

/** The rail's filter pills: one source for the pills and for the predicate. */
export const MY_WORK_FILTER_GROUPS = [
  {
    id: 'provider',
    label: 'Provider',
    options: [
      { value: 'claude', label: 'Claude' },
      { value: 'codex', label: 'Codex' },
      { value: 'antigravity', label: 'Agy' }
    ]
  },
  {
    id: 'status',
    label: 'Status',
    options: [
      { value: 'working', label: 'Working' },
      { value: 'done', label: 'Done' },
      { value: 'settled', label: 'Settled' }
    ]
  },
  {
    id: 'location',
    label: 'Location',
    options: [
      { value: 'local', label: 'Local' },
      { value: 'remote', label: 'Remote' }
    ]
  }
];

/** What the direction control says it will do, for the sort in force. */
export function myWorkSortDirectionLabel(
  sortBy: MyWorkSort,
  direction: MyWorkSortDirection
): string {
  if (sortBy === 'name') return direction === 'asc' ? 'A to Z' : 'Z to A';
  return direction === 'desc' ? 'Newest first' : 'Oldest first';
}

const STATUS_LABELS: Record<MyWorkStatus, string> = {
  working: 'Working',
  done: 'Done',
  settled: 'Settled'
};

export function myWorkStatus(
  session: Pick<OwnedSession, 'completedAt' | 'settledAt'>
): MyWorkStatus {
  if (session.settledAt) return 'settled';
  if (session.completedAt) return 'done';
  return 'working';
}

/** Folder basename used as the project identity shown in the rail. */
export function myWorkProject(
  session: Pick<OwnedSession, 'projectPath' | 'cwd' | 'agent' | 'viaCmux'>
): { key: string; label: string } {
  const resolved = resolveOwnedSessionProject(session);
  return {
    key: resolved.path || `provider:${resolved.label}`,
    label: resolved.label
  };
}

function activityRank(session: OwnedSession): number {
  const stamp = session.lastActivity || session.settledAt || session.completedAt;
  if (!stamp) return Number.NEGATIVE_INFINITY;
  const parsed = Date.parse(stamp);
  return Number.isFinite(parsed) ? parsed : Number.NEGATIVE_INFINITY;
}

/** Options inside a group combine as OR; groups combine as AND. */
export function matchesMyWorkFilters(session: OwnedSession, filters: MyWorkFilters): boolean {
  const values: Record<string, string> = {
    provider: session.agent,
    status: myWorkStatus(session),
    location: session.executionEnvironment
  };
  return MY_WORK_FILTER_GROUPS.every((group) => {
    const chosen = filters[group.id] ?? [];
    return chosen.length === 0 || chosen.includes(values[group.id]);
  });
}

export function prepareMyWorkSessions(
  sessions: OwnedSession[],
  options: Pick<MyWorkViewOptions, 'sortBy'> & Partial<Pick<MyWorkViewOptions, 'sortDirection'>>
): OwnedSession[] {
  const indexed = sessions.map((session, index) => ({ session, index }));
  const direction = options.sortDirection ?? NATURAL_SORT_DIRECTION[options.sortBy];
  const flip = direction === 'asc' ? 1 : -1;

  indexed.sort((left, right) => {
    // Compared one way round and turned over afterwards, so both directions
    // order equal rows the same: by where they already were.
    const ascending =
      options.sortBy === 'name'
        ? left.session.title.localeCompare(right.session.title, undefined, {
            sensitivity: 'base'
          })
        : activityRank(left.session) - activityRank(right.session);
    return ascending * flip || left.index - right.index;
  });
  return indexed.map(({ session }) => session);
}

function groupSessions(sessions: OwnedSession[], by: 'status' | 'project'): MyWorkGroup[] {
  const groups = new Map<string, MyWorkGroup>();
  for (const session of sessions) {
    const identity =
      by === 'status'
        ? { key: myWorkStatus(session), label: STATUS_LABELS[myWorkStatus(session)] }
        : myWorkProject(session);
    const group = groups.get(identity.key) ?? { ...identity, sessions: [] };
    group.sessions.push(session);
    groups.set(identity.key, group);
  }

  if (by === 'status') {
    return MY_WORK_STATUSES.flatMap((status) => {
      const group = groups.get(status);
      return group ? [group] : [];
    });
  }
  return [...groups.values()].sort((left, right) =>
    left.label.localeCompare(right.label, undefined, { sensitivity: 'base' })
  );
}

export function buildMyWorkGroups(
  sessions: OwnedSession[],
  options: MyWorkViewOptions
): MyWorkGroup[] {
  const prepared = prepareMyWorkSessions(sessions, options);
  if (options.groupByProject && options.groupByStatus) {
    return groupSessions(prepared, 'status').map((status) => ({
      ...status,
      subgroups: groupSessions(status.sessions, 'project').map((project) => ({
        ...project,
        key: `${status.key}::${project.key}`
      }))
    }));
  }
  if (options.groupByProject) return groupSessions(prepared, 'project');
  if (options.groupByStatus) return groupSessions(prepared, 'status');
  return [{ key: 'all', label: '', sessions: prepared }];
}

function isSort(value: unknown): value is MyWorkSort {
  return value === 'recent' || value === 'name';
}

function isSortDirection(value: unknown): value is MyWorkSortDirection {
  return value === 'asc' || value === 'desc';
}

export function normalizeMyWorkViewOptions(value: unknown): MyWorkViewOptions {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return { ...DEFAULT_MY_WORK_VIEW_OPTIONS };
  }
  // A stored option from before the two toggles carries one `groupBy` value;
  // an old `visibleStatuses` is dropped because the status pill replaced it.
  const candidate = value as Partial<MyWorkViewOptions> & { groupBy?: unknown };
  const legacy = ['none', 'status', 'project'].includes(candidate.groupBy as string)
    ? candidate.groupBy
    : null;
  const sortBy = isSort(candidate.sortBy) ? candidate.sortBy : DEFAULT_MY_WORK_VIEW_OPTIONS.sortBy;
  return {
    groupByProject:
      typeof candidate.groupByProject === 'boolean'
        ? candidate.groupByProject
        : legacy ? legacy === 'project' : DEFAULT_MY_WORK_VIEW_OPTIONS.groupByProject,
    groupByStatus:
      typeof candidate.groupByStatus === 'boolean'
        ? candidate.groupByStatus
        : legacy ? legacy === 'status' : DEFAULT_MY_WORK_VIEW_OPTIONS.groupByStatus,
    sortBy,
    // A stored option from before the direction existed carries none, and the
    // sort it was saved with is the direction that sort means.
    sortDirection: isSortDirection(candidate.sortDirection)
      ? candidate.sortDirection
      : NATURAL_SORT_DIRECTION[sortBy]
  };
}

/** Keeps only known filter groups and their known option values, in option order. */
export function normalizeMyWorkFilters(value: unknown): MyWorkFilters {
  const candidate = value && typeof value === 'object' ? (value as Record<string, unknown>) : {};
  return Object.fromEntries(
    MY_WORK_FILTER_GROUPS.map((group) => {
      const stored = candidate[group.id];
      const chosen = Array.isArray(stored) ? stored : [];
      return [group.id, group.options.map((option) => option.value).filter((v) => chosen.includes(v))];
    })
  );
}
