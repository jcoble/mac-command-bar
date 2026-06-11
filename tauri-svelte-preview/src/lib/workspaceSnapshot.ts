import type { ProjectRoot } from './sourceData.ts';
import {
  createDefaultSourceDockLayout,
  normalizeSourceDockLayout,
  type SourceDockLayout
} from './sourceDockLayout.ts';

export const snapshotStorageKey = 'mac-command-bar.workspace-snapshots';

export type WorkspaceSnapshotProvider = 'codex' | 'claude' | 'cmux' | 'manual';
export type WorkspaceSnapshotActivityMode =
  | 'files'
  | 'clipboard'
  | 'conversations'
  | 'sessions'
  | 'agents'
  | 'worktrees'
  | 'git'
  | 'runs';
export type WorkspaceSnapshotTerminalApp =
  | 'Warp'
  | 'Terminal'
  | 'iTerm'
  | 'iTerm2'
  | 'Ghostty'
  | 'WezTerm'
  | 'Alacritty';

export type WorkspaceSnapshotEmbeddedTerminal = {
  sessionID: string;
  cwd: string;
  shell: string | null;
  startedAt: number | null;
};

export type WorkspaceSnapshot = {
  id: string;
  provider: WorkspaceSnapshotProvider;
  sessionID: string;
  title: string;
  model: string | null;
  project: ProjectRoot;
  cwd: string;
  worktreePath: string | null;
  branch: string | null;
  selectedPath: string | null;
  selectedLine: number | null;
  openPaths: string[];
  sourceActivityMode: WorkspaceSnapshotActivityMode;
  sourceTerminalApp: WorkspaceSnapshotTerminalApp;
  embeddedTerminal: WorkspaceSnapshotEmbeddedTerminal | null;
  dockLayout: SourceDockLayout;
  resumeCommand: string | null;
  capturedAt: number;
};

export type WorkspaceSnapshotInput = {
  provider: WorkspaceSnapshotProvider;
  sessionID: string;
  title: string;
  model?: string | null;
  project: ProjectRoot;
  cwd: string;
  worktreePath?: string | null;
  branch?: string | null;
  selectedPath?: string | null;
  selectedLine?: number | null;
  openPaths?: string[];
  sourceActivityMode?: WorkspaceSnapshotActivityMode;
  sourceTerminalApp?: WorkspaceSnapshotTerminalApp;
  embeddedTerminal?: WorkspaceSnapshotEmbeddedTerminal | null;
  dockLayout?: SourceDockLayout;
  resumeCommand?: string | null;
  capturedAt?: number;
};

export type RestoredWorkspaceSnapshot = {
  selectedProjectID: string;
  selectedSourcePaths: Record<string, string>;
  selectedPath: string | null;
  selectedLine: number | null;
  sourceActivityMode: WorkspaceSnapshotActivityMode;
  sourceTerminalApp: WorkspaceSnapshotTerminalApp;
  cwd: string;
  worktreePath: string | null;
  branch: string | null;
  openPaths: string[];
  embeddedTerminal: WorkspaceSnapshotEmbeddedTerminal | null;
  dockLayout: SourceDockLayout;
  resumeCommand: string | null;
};

export function createWorkspaceSnapshot(input: WorkspaceSnapshotInput): WorkspaceSnapshot {
  const project = normalizeProjectRoot(input.project);
  const selectedPath = normalizeOptionalPath(input.selectedPath);

  return {
    id: snapshotID(input.provider, input.sessionID),
    provider: input.provider,
    sessionID: input.sessionID.trim(),
    title: input.title.trim() || 'Untitled conversation',
    model: normalizeOptionalString(input.model),
    project,
    cwd: normalizePath(input.cwd || project.path),
    worktreePath: normalizeOptionalPath(input.worktreePath),
    branch: normalizeOptionalString(input.branch),
    selectedPath,
    selectedLine: normalizeLine(input.selectedLine),
    openPaths: normalizeOpenPaths(input.openPaths, selectedPath),
    sourceActivityMode: input.sourceActivityMode ?? 'conversations',
    sourceTerminalApp: input.sourceTerminalApp ?? 'Warp',
    embeddedTerminal: normalizeEmbeddedTerminal(input.embeddedTerminal),
    dockLayout: normalizeSourceDockLayout(input.dockLayout ?? createDefaultSourceDockLayout()),
    resumeCommand: normalizeOptionalString(input.resumeCommand),
    capturedAt: normalizeCapturedAt(input.capturedAt)
  };
}

export function restoreWorkspaceSnapshot(snapshot: WorkspaceSnapshot): RestoredWorkspaceSnapshot {
  const selectedSourcePaths =
    snapshot.selectedPath === null ? {} : { [snapshot.project.id]: snapshot.selectedPath };

  return {
    selectedProjectID: snapshot.project.id,
    selectedSourcePaths,
    selectedPath: snapshot.selectedPath,
    selectedLine: snapshot.selectedLine,
    sourceActivityMode: snapshot.sourceActivityMode,
    sourceTerminalApp: snapshot.sourceTerminalApp,
    cwd: snapshot.cwd,
    worktreePath: snapshot.worktreePath,
    branch: snapshot.branch,
    openPaths: snapshot.openPaths,
    embeddedTerminal: normalizeEmbeddedTerminal(snapshot.embeddedTerminal),
    dockLayout: normalizeSourceDockLayout(snapshot.dockLayout),
    resumeCommand: snapshot.resumeCommand
  };
}

export function upsertWorkspaceSnapshot(
  snapshots: WorkspaceSnapshot[],
  snapshot: WorkspaceSnapshot,
  maxSnapshots = 24
): WorkspaceSnapshot[] {
  const nextSnapshots = [
    snapshot,
    ...snapshots.filter((candidate) => candidate.id !== snapshot.id)
  ].sort((left, right) => right.capturedAt - left.capturedAt);

  return nextSnapshots.slice(0, Math.max(1, Math.floor(maxSnapshots)));
}

export function parseStoredWorkspaceSnapshot(value: unknown): WorkspaceSnapshot | null {
  if (typeof value !== 'object' || value === null) return null;
  const snapshot = value as Partial<WorkspaceSnapshot>;
  if (
    !isWorkspaceSnapshotProvider(snapshot.provider) ||
    typeof snapshot.sessionID !== 'string' ||
    typeof snapshot.title !== 'string' ||
    typeof snapshot.cwd !== 'string' ||
    !isProjectRootLike(snapshot.project)
  ) {
    return null;
  }

  return createWorkspaceSnapshot({
    provider: snapshot.provider,
    sessionID: snapshot.sessionID,
    title: snapshot.title,
    model: typeof snapshot.model === 'string' ? snapshot.model : null,
    project: snapshot.project,
    cwd: snapshot.cwd,
    worktreePath: typeof snapshot.worktreePath === 'string' ? snapshot.worktreePath : null,
    branch: typeof snapshot.branch === 'string' ? snapshot.branch : null,
    selectedPath: typeof snapshot.selectedPath === 'string' ? snapshot.selectedPath : null,
    selectedLine: typeof snapshot.selectedLine === 'number' ? snapshot.selectedLine : null,
    openPaths: Array.isArray(snapshot.openPaths)
      ? snapshot.openPaths.filter((path): path is string => typeof path === 'string')
      : [],
    sourceActivityMode: isWorkspaceSnapshotActivityMode(snapshot.sourceActivityMode)
      ? snapshot.sourceActivityMode
      : 'conversations',
    sourceTerminalApp: isWorkspaceSnapshotTerminalApp(snapshot.sourceTerminalApp)
      ? snapshot.sourceTerminalApp
      : 'Warp',
    embeddedTerminal: isWorkspaceSnapshotEmbeddedTerminal(snapshot.embeddedTerminal)
      ? snapshot.embeddedTerminal
      : null,
    dockLayout: snapshot.dockLayout,
    resumeCommand: typeof snapshot.resumeCommand === 'string' ? snapshot.resumeCommand : null,
    capturedAt: typeof snapshot.capturedAt === 'number' ? snapshot.capturedAt : Date.now()
  });
}

function snapshotID(provider: WorkspaceSnapshotProvider, sessionID: string): string {
  return `${provider}:${sessionID.trim() || 'session'}`;
}

function isWorkspaceSnapshotProvider(value: unknown): value is WorkspaceSnapshotProvider {
  return value === 'codex' || value === 'claude' || value === 'cmux' || value === 'manual';
}

function isWorkspaceSnapshotActivityMode(value: unknown): value is WorkspaceSnapshotActivityMode {
  return (
    value === 'files' ||
    value === 'clipboard' ||
    value === 'conversations' ||
    value === 'sessions' ||
    value === 'agents' ||
    value === 'worktrees' ||
    value === 'git' ||
    value === 'runs'
  );
}

function isWorkspaceSnapshotTerminalApp(value: unknown): value is WorkspaceSnapshotTerminalApp {
  return (
    value === 'Warp' ||
    value === 'Terminal' ||
    value === 'iTerm' ||
    value === 'iTerm2' ||
    value === 'Ghostty' ||
    value === 'WezTerm' ||
    value === 'Alacritty'
  );
}

function isWorkspaceSnapshotEmbeddedTerminal(value: unknown): value is WorkspaceSnapshotEmbeddedTerminal {
  if (typeof value !== 'object' || value === null) return false;
  const terminal = value as Partial<WorkspaceSnapshotEmbeddedTerminal>;
  return (
    typeof terminal.sessionID === 'string' &&
    typeof terminal.cwd === 'string' &&
    (terminal.shell === null || terminal.shell === undefined || typeof terminal.shell === 'string') &&
    (terminal.startedAt === null || terminal.startedAt === undefined || typeof terminal.startedAt === 'number')
  );
}

function isProjectRootLike(value: unknown): value is ProjectRoot {
  if (typeof value !== 'object' || value === null) return false;
  const project = value as Partial<ProjectRoot>;
  return (
    typeof project.id === 'string' &&
    typeof project.name === 'string' &&
    typeof project.path === 'string'
  );
}

function normalizeProjectRoot(project: ProjectRoot): ProjectRoot {
  return {
    id: project.id.trim(),
    name: project.name.trim() || 'Project',
    path: normalizePath(project.path)
  };
}

function normalizeOpenPaths(openPaths: string[] | undefined, selectedPath: string | null): string[] {
  const seen = new Set<string>();
  const normalizedPaths = [
    selectedPath,
    ...(openPaths ?? []).map((path) => normalizeOptionalPath(path))
  ].filter((path): path is string => path !== null);

  return normalizedPaths.filter((path) => {
    if (seen.has(path)) return false;
    seen.add(path);
    return true;
  });
}

function normalizePath(path: string): string {
  return path.trim().replace(/\/+$/, '');
}

function normalizeOptionalPath(path: string | null | undefined): string | null {
  if (typeof path !== 'string') return null;
  const normalizedPath = normalizePath(path);
  return normalizedPath.length === 0 ? null : normalizedPath;
}

function normalizeOptionalString(value: string | null | undefined): string | null {
  if (typeof value !== 'string') return null;
  const normalizedValue = value.trim();
  return normalizedValue.length === 0 ? null : normalizedValue;
}

function normalizeEmbeddedTerminal(
  terminal: WorkspaceSnapshotEmbeddedTerminal | null | undefined
): WorkspaceSnapshotEmbeddedTerminal | null {
  if (!terminal) return null;

  const sessionID = normalizeOptionalString(terminal.sessionID);
  const cwd = normalizeOptionalPath(terminal.cwd);
  if (!sessionID || !cwd) return null;

  return {
    sessionID,
    cwd,
    shell: normalizeOptionalString(terminal.shell),
    startedAt: normalizeTimestamp(terminal.startedAt)
  };
}

function normalizeLine(line: number | null | undefined): number | null {
  if (typeof line !== 'number' || !Number.isFinite(line) || line < 1) return null;
  return Math.round(line);
}

function normalizeTimestamp(timestamp: number | null | undefined): number | null {
  if (typeof timestamp !== 'number' || !Number.isFinite(timestamp) || timestamp < 0) return null;
  return Math.floor(timestamp);
}

function normalizeCapturedAt(capturedAt: number | null | undefined): number {
  if (typeof capturedAt !== 'number' || !Number.isFinite(capturedAt)) return Date.now();
  return Math.max(0, Math.floor(capturedAt));
}
