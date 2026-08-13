import type { SessionLibraryRecord } from '../sessionLibrary/sessionLibraryModel.ts';

export interface SessionHistoryFilterOptions {
  query?: string;
  provider?: string;
  windowState?: SessionHistoryWindowState;
}

export const SESSION_HISTORY_ROW_WINDOW = 25;

export interface SessionHistoryWindowState {
  query: string;
  provider: string;
  visibleRowsByWorktree: Readonly<Record<string, number>>;
}

export interface SessionHistoryProviderOption {
  value: string;
  label: string;
}

export interface SessionHistoryRow {
  record: SessionLibraryRecord;
  displayTitle: string;
  excerpt: string;
  providerLabel: string;
  statusHint: 'Live' | 'Open' | null;
  activityTime: number;
}

export interface SessionHistoryWorktreeGroup {
  key: string;
  name: string;
  path: string;
  count: number;
  activityTime: number;
  rows: SessionHistoryRow[];
  olderCount: number;
}

export interface SessionHistoryProjectGroup {
  key: string;
  name: string;
  path: string;
  count: number;
  activityTime: number;
  singleCheckout: boolean;
  worktrees: SessionHistoryWorktreeGroup[];
}

export interface SessionHistoryViewModel {
  totalCount: number;
  providers: SessionHistoryProviderOption[];
  projects: SessionHistoryProjectGroup[];
}

export interface SessionHistoryCollapseState {
  projects: ReadonlySet<string>;
  worktrees: ReadonlySet<string>;
}

type SessionHistoryGroupLevel = keyof SessionHistoryCollapseState;

interface PathIdentity {
  projectPath: string;
  projectName: string;
  worktreePath: string;
  worktreeName: string;
}

interface IndexedRow extends SessionHistoryRow, PathIdentity {
  searchText: string;
}

function normalizedWindowFilters(
  options: SessionHistoryFilterOptions
): Pick<SessionHistoryWindowState, 'query' | 'provider'> {
  return {
    query: compactText(options.query).toLocaleLowerCase(),
    provider: compactText(options.provider).toLocaleLowerCase()
  };
}

export function createSessionHistoryWindowState(
  options: SessionHistoryFilterOptions = {}
): SessionHistoryWindowState {
  return { ...normalizedWindowFilters(options), visibleRowsByWorktree: {} };
}

export function resetSessionHistoryWindowOnFilterChange(
  state: SessionHistoryWindowState,
  options: SessionHistoryFilterOptions
): SessionHistoryWindowState {
  const next = normalizedWindowFilters(options);
  if (state.query === next.query && state.provider === next.provider) return state;
  return { ...next, visibleRowsByWorktree: {} };
}

export function extendSessionHistoryWindow(
  state: SessionHistoryWindowState,
  worktreeKey: string
): SessionHistoryWindowState {
  const visible = state.visibleRowsByWorktree[worktreeKey] ?? SESSION_HISTORY_ROW_WINDOW;
  return {
    ...state,
    visibleRowsByWorktree: {
      ...state.visibleRowsByWorktree,
      [worktreeKey]: visible + SESSION_HISTORY_ROW_WINDOW
    }
  };
}

function canonicalPath(value: string | null | undefined): string {
  const raw = (value ?? '').trim().replaceAll('\\', '/').replace(/\/+/g, '/');
  if (!raw || raw === '/') return raw;
  return raw.replace(/\/$/, '');
}

function pathName(path: string, fallback: string): string {
  return path.split('/').filter(Boolean).at(-1) || fallback;
}

/**
 * Resolve the repository represented by the shared worktree layout without
 * asking the data source for fields it does not expose yet.
 */
function projectRootFor(path: string): string {
  const shared = path.match(/^(.*)\/worktrees\/([^/]+)\/[^/]+(?:\/.*)?$/);
  if (shared) return canonicalPath(`${shared[1]}/${shared[2]}`);

  const local = path.match(/^(.*)\/\.worktrees\/[^/]+(?:\/.*)?$/);
  if (local) return canonicalPath(local[1]);

  return path;
}

function identifyPaths(record: SessionLibraryRecord): PathIdentity {
  const worktreePath = canonicalPath(record.canonicalCwd || record.projectPath) || 'Unknown checkout';
  const recordedProjectPath = canonicalPath(record.projectPath);
  const projectPath = projectRootFor(recordedProjectPath || worktreePath) || projectRootFor(worktreePath);
  return {
    projectPath,
    projectName: pathName(projectPath, 'Other sessions'),
    worktreePath,
    worktreeName: pathName(worktreePath, 'Unknown checkout')
  };
}

function compactText(value: string | null | undefined): string {
  return (value ?? '').replace(/\s+/g, ' ').trim();
}

function firstUsefulExcerpt(record: SessionLibraryRecord): string {
  return compactText(
    record.firstPrompt
      || record.latestTurns.find((turn) => turn.speaker === 'user')?.text
      || record.description
      || record.latestTurns[0]?.text
  ).replace(/^(?:you|user|agent):\s*/i, '');
}

function displayTitleFor(record: SessionLibraryRecord, excerpt: string): string {
  const title = compactText(record.title);
  const titleIsIdentifier = Boolean(
    title
      && (title === record.nativeSessionId || title === record.ownedId || title === record.key)
  );
  if (title && !titleIsIdentifier) return title;
  return excerpt || title || 'Untitled session';
}

function providerLabelFor(provider: string): string {
  const value = compactText(provider);
  if (!value) return 'Unknown';
  return value
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((part) => `${part[0]?.toUpperCase() ?? ''}${part.slice(1)}`)
    .join(' ');
}

function statusHintFor(record: SessionLibraryRecord): 'Live' | 'Open' | null {
  if (record.source !== 'owned' || !record.owned) return null;
  const activeRuntimeStates = new Set([
    'starting',
    'ready',
    'working',
    'waiting-approval',
    'waiting-input',
    'interrupting'
  ]);
  if (record.owned.state === 'live' || (record.runtimeState && activeRuntimeStates.has(record.runtimeState))) {
    return 'Live';
  }
  return record.owned.state === 'background' ? 'Open' : null;
}

function timestamp(value: string | null): number {
  const parsed = value ? Date.parse(value) : Number.NaN;
  return Number.isFinite(parsed) ? parsed : 0;
}

function indexRow(record: SessionLibraryRecord): IndexedRow {
  const paths = identifyPaths(record);
  const excerpt = firstUsefulExcerpt(record);
  const displayTitle = displayTitleFor(record, excerpt);
  return {
    record,
    ...paths,
    displayTitle,
    excerpt,
    providerLabel: providerLabelFor(record.provider),
    statusHint: statusHintFor(record),
    activityTime: timestamp(record.updatedAt),
    searchText: [
      displayTitle,
      excerpt,
      record.title,
      record.description,
      record.firstPrompt,
      ...record.latestTurns.map((turn) => turn.text),
      paths.projectName,
      paths.projectPath,
      paths.worktreeName,
      paths.worktreePath
    ]
      .map(compactText)
      .filter(Boolean)
      .join(' ')
      .toLocaleLowerCase()
  };
}

function compareRows(left: SessionHistoryRow, right: SessionHistoryRow): number {
  return right.activityTime - left.activityTime || left.displayTitle.localeCompare(right.displayTitle);
}

function compareGroups(
  left: Pick<SessionHistoryProjectGroup, 'activityTime' | 'name'>,
  right: Pick<SessionHistoryProjectGroup, 'activityTime' | 'name'>
): number {
  return right.activityTime - left.activityTime || left.name.localeCompare(right.name);
}

export function buildSessionHistoryViewModel(
  records: readonly SessionLibraryRecord[],
  options: SessionHistoryFilterOptions = {}
): SessionHistoryViewModel {
  const indexed = records.map(indexRow);
  const providers = [...new Set(indexed.map((row) => row.record.provider).filter(Boolean))]
    .sort((left, right) => providerLabelFor(left).localeCompare(providerLabelFor(right)))
    .map((value) => ({ value, label: providerLabelFor(value) }));
  const { query, provider } = normalizedWindowFilters(options);
  const windowMatchesFilters = options.windowState?.query === query
    && options.windowState.provider === provider;
  const filtered = indexed.filter((row) => {
    if (query && !row.searchText.includes(query)) return false;
    return !provider || row.record.provider.toLocaleLowerCase() === provider;
  });

  const projects = new Map<string, {
    name: string;
    path: string;
    worktrees: Map<string, { name: string; path: string; rows: SessionHistoryRow[] }>;
  }>();

  for (const row of filtered) {
    const projectKey = `project:${row.projectPath}`;
    let project = projects.get(projectKey);
    if (!project) {
      project = { name: row.projectName, path: row.projectPath, worktrees: new Map() };
      projects.set(projectKey, project);
    }

    const worktreeKey = `worktree:${row.worktreePath}`;
    let worktree = project.worktrees.get(worktreeKey);
    if (!worktree) {
      worktree = { name: row.worktreeName, path: row.worktreePath, rows: [] };
      project.worktrees.set(worktreeKey, worktree);
    }
    worktree.rows.push(row);
  }

  const projectGroups = [...projects.entries()].map(([key, project]): SessionHistoryProjectGroup => {
    const worktrees = [...project.worktrees.entries()]
      .map(([worktreeKey, worktree]): SessionHistoryWorktreeGroup => {
        const allRows = worktree.rows.toSorted(compareRows);
        const visibleRowCount = query
          ? allRows.length
          : windowMatchesFilters
            ? options.windowState?.visibleRowsByWorktree[worktreeKey] ?? SESSION_HISTORY_ROW_WINDOW
            : SESSION_HISTORY_ROW_WINDOW;
        const rows = allRows.slice(0, visibleRowCount);
        return {
          key: worktreeKey,
          name: worktree.name,
          path: worktree.path,
          count: allRows.length,
          activityTime: allRows[0]?.activityTime ?? 0,
          rows,
          olderCount: Math.max(0, allRows.length - rows.length)
        };
      })
      .toSorted(compareGroups);
    return {
      key,
      name: project.name,
      path: project.path,
      count: worktrees.reduce((total, worktree) => total + worktree.count, 0),
      activityTime: worktrees[0]?.activityTime ?? 0,
      singleCheckout: worktrees.length === 1,
      worktrees
    };
  }).toSorted(compareGroups);

  return { totalCount: filtered.length, providers, projects: projectGroups };
}

export function createSessionHistoryCollapseState(): SessionHistoryCollapseState {
  return { projects: new Set(), worktrees: new Set() };
}

export function isSessionHistoryGroupOpen(
  state: SessionHistoryCollapseState,
  level: 'project' | 'worktree',
  key: string
): boolean {
  return !state[`${level}s` as SessionHistoryGroupLevel].has(key);
}

export function toggleSessionHistoryGroup(
  state: SessionHistoryCollapseState,
  level: 'project' | 'worktree',
  key: string
): SessionHistoryCollapseState {
  const collection = `${level}s` as SessionHistoryGroupLevel;
  const next = new Set(state[collection]);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  return { ...state, [collection]: next };
}

export function visibleSessionHistoryRows(
  projects: readonly SessionHistoryProjectGroup[],
  state: SessionHistoryCollapseState
): SessionHistoryRow[] {
  const rows: SessionHistoryRow[] = [];
  for (const project of projects) {
    if (!isSessionHistoryGroupOpen(state, 'project', project.key)) continue;
    for (const worktree of project.worktrees) {
      if (!project.singleCheckout && !isSessionHistoryGroupOpen(state, 'worktree', worktree.key)) continue;
      rows.push(...worktree.rows);
    }
  }
  return rows;
}
