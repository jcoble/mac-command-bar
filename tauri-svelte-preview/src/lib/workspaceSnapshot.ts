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
  dockLayout?: SourceDockLayout;
  resumeCommand?: string | null;
  capturedAt?: number;
};

export type RestoredWorkspaceSnapshot = {
  selectedProjectID: string;
  selectedSourcePaths: Record<string, string>;
  selectedLine: number | null;
  sourceActivityMode: WorkspaceSnapshotActivityMode;
  sourceTerminalApp: WorkspaceSnapshotTerminalApp;
  cwd: string;
  worktreePath: string | null;
  branch: string | null;
  openPaths: string[];
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
    selectedLine: snapshot.selectedLine,
    sourceActivityMode: snapshot.sourceActivityMode,
    sourceTerminalApp: snapshot.sourceTerminalApp,
    cwd: snapshot.cwd,
    worktreePath: snapshot.worktreePath,
    branch: snapshot.branch,
    openPaths: snapshot.openPaths,
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

function snapshotID(provider: WorkspaceSnapshotProvider, sessionID: string): string {
  return `${provider}:${sessionID.trim() || 'session'}`;
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

function normalizeLine(line: number | null | undefined): number | null {
  if (typeof line !== 'number' || !Number.isFinite(line) || line < 1) return null;
  return Math.round(line);
}

function normalizeCapturedAt(capturedAt: number | null | undefined): number {
  if (typeof capturedAt !== 'number' || !Number.isFinite(capturedAt)) return Date.now();
  return Math.max(0, Math.floor(capturedAt));
}
