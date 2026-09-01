import type { RepositoryCheckout } from '../../tauriSource.ts';
import {
  filterSessionLibrary,
  type SessionHistoryFilters,
  type SessionLibraryRecord
} from '../sessionLibrary/sessionLibraryModel.ts';

export interface SessionHistoryFilterOptions {
  query?: string;
  provider?: string;
  windowState?: SessionHistoryWindowState;
  /**
   * The checkouts each repository still has on disk, keyed by repository root,
   * as git reported them.
   *
   * When a repository is in here, this is the authority on which checkouts it
   * shows: every live one appears even if nothing was ever run in it, and a
   * folder that has since been deleted does not, along with the sessions that
   * ran there. A repository absent from the map keeps the old behaviour of
   * showing whichever checkouts its sessions name, which is what happens for a
   * folder git cannot be asked about — a temporary directory, or a project whose
   * root was never resolved.
   */
  checkouts?: Readonly<Record<string, readonly RepositoryCheckout[]>>;
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

/**
 * Which groups the reader has opened. The sets hold OPEN keys, so everything
 * starts collapsed. That default is load-bearing: this machine has hundreds of
 * past sessions, and mounting every card at once used to freeze the whole app
 * for seconds — the browser engine re-checks sibling styling on every inserted
 * node, which grows quadratic in a long flat list. Opening one group at a time
 * keeps each mount small.
 */
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

/** Apply history scopes without collapsing a repository to one worktree path. */
export function filterSessionHistoryRecords(
  records: readonly SessionLibraryRecord[],
  filters: SessionHistoryFilters = {}
): SessionLibraryRecord[] {
  const { scope = 'all', workspacePath, projectPath, ...libraryFilters } = filters;
  const filtered = filterSessionLibrary(records, libraryFilters);
  if (scope === 'workspace') {
    const workspace = canonicalPath(workspacePath);
    return workspace
      ? filtered.filter((record) => canonicalPath(record.canonicalCwd) === workspace)
      : [];
  }
  if (scope === 'project') {
    const project = canonicalPath(projectPath);
    return project
      ? filtered.filter((record) => identifyPaths(record).projectPath === project)
      : [];
  }
  return filtered;
}

export function sessionHistoryProjectPath(record: SessionLibraryRecord): string {
  return identifyPaths(record).projectPath;
}

function pathName(path: string, fallback: string): string {
  return path.split('/').filter(Boolean).at(-1) || fallback;
}

/**
 * Resolve the repository represented by the shared worktree layout without
 * asking the data source for fields it does not expose yet.
 */
function projectRootFor(path: string): string {
  // The checkout folder under the repository's name is optional. A session run
  // from the container itself — `…/worktrees/mac-command-bar`, no checkout after
  // it — used to match nothing and become a SECOND project of its own, with the
  // same name as the real one. The repository's own sessions and its worktrees
  // then sat in one group while a decoy holding two rows and no worktrees sat in
  // another, which is what "it's not showing the worktrees for it" was.
  const shared = path.match(/^(.*)\/worktrees\/([^/]+)(?:\/.*)?$/);
  if (shared) return canonicalPath(`${shared[1]}/${shared[2]}`);

  const local = path.match(/^(.*)\/\.worktrees(?:\/.*)?$/);
  if (local) return canonicalPath(local[1]);

  return path;
}

function identifyPaths(record: SessionLibraryRecord): PathIdentity {
  const worktreePath = canonicalPath(record.canonicalCwd || record.projectPath) || 'Unknown checkout';
  const recordedProjectPath = canonicalPath(record.projectPath);
  // Git's answer first. It knows which repository a folder belongs to even when
  // the folder's name says nothing — a worktree's `.git` file names the main
  // checkout outright — and it is right for every layout at once, which no
  // reading of the path is. `projectRootFor` below is the fallback for folders
  // git can no longer be asked about: a worktree that has since been deleted.
  const projectPath = canonicalPath(record.projectRoot)
    || projectRootFor(recordedProjectPath || worktreePath)
    || projectRootFor(worktreePath);
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

  // Seed each repository with the checkouts git says it still has, before any
  // session is filed. This is what puts a worktree on screen when the only
  // thing ever run inside it was a dispatched lane, and it fixes the order:
  // the repository's own folder first, then its worktrees.
  const known = options.checkouts ?? {};
  for (const [root, list] of Object.entries(known)) {
    const projectKey = `project:${canonicalPath(root)}`;
    const project = projects.get(projectKey) ?? {
      name: pathName(canonicalPath(root), 'Other sessions'),
      path: canonicalPath(root),
      worktrees: new Map()
    };
    projects.set(projectKey, project);
    for (const checkout of list) {
      const path = canonicalPath(checkout.path);
      const key = `worktree:${path}`;
      if (!project.worktrees.has(key)) {
        project.worktrees.set(key, { name: pathName(path, 'Unknown checkout'), path, rows: [] });
      }
    }
  }

  for (const row of filtered) {
    const projectKey = `project:${row.projectPath}`;
    const live = known[row.projectPath];
    // A session whose folder is gone is not shown. Git listed this repository's
    // checkouts and this one is not among them, so there is nothing left to
    // open, resume, or run in.
    if (live && !live.some((checkout) => canonicalPath(checkout.path) === row.worktreePath)) {
      continue;
    }

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

  // Counted from what was placed, not from what passed the filters: a session
  // whose checkout has been deleted is left out above, and a header claiming it
  // was still there would be wrong by exactly that many.
  const totalCount = projectGroups.reduce((total, project) => total + project.count, 0);
  return { totalCount, providers, projects: projectGroups };
}

export function createSessionHistoryCollapseState(): SessionHistoryCollapseState {
  return { projects: new Set(), worktrees: new Set() };
}

export function isSessionHistoryGroupOpen(
  state: SessionHistoryCollapseState,
  level: 'project' | 'worktree',
  key: string
): boolean {
  return state[`${level}s` as SessionHistoryGroupLevel].has(key);
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
