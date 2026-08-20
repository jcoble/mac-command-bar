/**
 * Pure view-model for the My Work rail. Components own drawing and interaction;
 * this module owns the durable option shape plus filtering, sorting and grouping.
 */
import { loadLayout, saveLayout, type LayoutStorage } from '../layout/layoutStorage.ts';
import { resolveOwnedSessionProject, type OwnedSession } from '../ownedSessions.ts';

export type MyWorkGrouping = 'none' | 'status' | 'project';
export type MyWorkSort = 'recent' | 'name';
export type MyWorkSortDirection = 'asc' | 'desc';
export type MyWorkStatus = 'working' | 'done' | 'settled';

export interface MyWorkViewOptions {
  groupBy: MyWorkGrouping;
  sortBy: MyWorkSort;
  sortDirection: MyWorkSortDirection;
  visibleStatuses: MyWorkStatus[];
}

export interface MyWorkGroup {
  key: string;
  label: string;
  sessions: OwnedSession[];
}

export const MY_WORK_VIEW_OPTIONS_KEY = 'mac-command-bar.next.my-work-view-options';
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
  groupBy: 'status',
  sortBy: 'recent',
  sortDirection: NATURAL_SORT_DIRECTION.recent,
  visibleStatuses: [...MY_WORK_STATUSES]
};

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

export function prepareMyWorkSessions(
  sessions: OwnedSession[],
  options: Pick<MyWorkViewOptions, 'sortBy' | 'visibleStatuses'> &
    Partial<Pick<MyWorkViewOptions, 'sortDirection'>>
): OwnedSession[] {
  const visible = new Set(options.visibleStatuses);
  const indexed = sessions
    .map((session, index) => ({ session, index }))
    .filter(({ session }) => visible.has(myWorkStatus(session)));
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

export function buildMyWorkGroups(
  sessions: OwnedSession[],
  options: MyWorkViewOptions
): MyWorkGroup[] {
  const prepared = prepareMyWorkSessions(sessions, options);
  if (options.groupBy === 'none') {
    return [{ key: 'all', label: '', sessions: prepared }];
  }

  const groups = new Map<string, MyWorkGroup>();
  for (const session of prepared) {
    const identity =
      options.groupBy === 'status'
        ? { key: myWorkStatus(session), label: STATUS_LABELS[myWorkStatus(session)] }
        : myWorkProject(session);
    const group = groups.get(identity.key) ?? { ...identity, sessions: [] };
    group.sessions.push(session);
    groups.set(identity.key, group);
  }

  if (options.groupBy === 'status') {
    return MY_WORK_STATUSES.flatMap((status) => {
      const group = groups.get(status);
      return group ? [group] : [];
    });
  }
  return [...groups.values()].sort((left, right) =>
    left.label.localeCompare(right.label, undefined, { sensitivity: 'base' })
  );
}

function isGrouping(value: unknown): value is MyWorkGrouping {
  return value === 'none' || value === 'status' || value === 'project';
}

function isSort(value: unknown): value is MyWorkSort {
  return value === 'recent' || value === 'name';
}

function isSortDirection(value: unknown): value is MyWorkSortDirection {
  return value === 'asc' || value === 'desc';
}

export function normalizeMyWorkViewOptions(value: unknown): MyWorkViewOptions {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return { ...DEFAULT_MY_WORK_VIEW_OPTIONS, visibleStatuses: [...MY_WORK_STATUSES] };
  }
  const candidate = value as Partial<MyWorkViewOptions>;
  const sortBy = isSort(candidate.sortBy) ? candidate.sortBy : DEFAULT_MY_WORK_VIEW_OPTIONS.sortBy;
  const visibleStatuses = Array.isArray(candidate.visibleStatuses)
    ? MY_WORK_STATUSES.filter((status) => candidate.visibleStatuses?.includes(status))
    : [...MY_WORK_STATUSES];
  return {
    groupBy: isGrouping(candidate.groupBy) ? candidate.groupBy : DEFAULT_MY_WORK_VIEW_OPTIONS.groupBy,
    sortBy,
    // A stored option from before the direction existed carries none, and the
    // sort it was saved with is the direction that sort means.
    sortDirection: isSortDirection(candidate.sortDirection)
      ? candidate.sortDirection
      : NATURAL_SORT_DIRECTION[sortBy],
    visibleStatuses
  };
}

export function readMyWorkViewOptions(storage: LayoutStorage): MyWorkViewOptions {
  return normalizeMyWorkViewOptions(loadLayout<unknown>(storage, MY_WORK_VIEW_OPTIONS_KEY));
}

export function writeMyWorkViewOptions(
  storage: LayoutStorage,
  options: MyWorkViewOptions
): boolean {
  return saveLayout(storage, MY_WORK_VIEW_OPTIONS_KEY, normalizeMyWorkViewOptions(options));
}
