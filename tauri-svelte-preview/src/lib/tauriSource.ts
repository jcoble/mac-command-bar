import type {
  ProjectRoot,
  SourceDefinitionTarget,
  SourcePreview,
  SourceRecord,
  SourceReferenceTarget,
  SourceScanResult,
  SourceSearchMatch
} from './sourceData';

export const defaultSourceScanLimit = 2_000;
export const expandedSourceScanLimit = 5_000;
export const nativeSourceScanProgressEvent = 'source_scan_progress';

export type NativeSourceScanProgress = {
  scanId: string;
  visitedEntries: number;
  matchedFiles: number;
};

export type ProjectGitFileStatus = {
  relativePath: string;
  indexStatus: string;
  worktreeStatus: string;
  status: string;
  badge: string;
};

export type ProjectGitStatus = {
  branch: string | null;
  ahead: number;
  behind: number;
  files: ProjectGitFileStatus[];
};

export type GitActionResult = {
  message: string;
  status: ProjectGitStatus;
};

export type SourceGitDiff = {
  relativePath: string;
  status: string;
  diff: string;
  isBinary: boolean;
};

export type ProjectWorktree = {
  repo: string;
  path: string;
  branch: string;
  taskID: string | null;
  isDirty: boolean;
  hasUnmergedCommits: boolean;
  lastActivity: string | null;
  deleteEligibility: string;
};

export type GitRepositorySummary = {
  projectID: string;
  projectName: string;
  repo: string;
  path: string;
  rootLabel: string;
  branch: string;
  taskID: string | null;
  isWorktree: boolean;
  isDirty: boolean;
  stagedCount: number;
  unstagedCount: number;
  untrackedCount: number;
  dirtyCount: number;
  ahead: number;
  behind: number;
  lastCommitSha: string | null;
  lastCommitSubject: string | null;
  lastCommitAt: string | null;
  dirtySinceEpochMs: number | null;
  error: string | null;
};

export type GitCommitHistoryEntry = {
  shortSha: string;
  sha: string;
  subject: string;
  author: string;
  committedAt: string;
  refs: string;
  taskID: string | null;
};

export type AgentSession = {
  provider: string;
  id: string;
  title: string;
  projectPath: string | null;
  lastActivity: string | null;
  resumeCommands: string[];
};

export type RuntimeContextProject = Pick<ProjectRoot, 'id' | 'name' | 'path'>;

export type RuntimeContext = {
  pid: number;
  command: string;
  port: number;
  cwd: string;
  projectID: string | null;
  projectName: string;
  rootLabel: string;
};

export function createSourceScanId(): string {
  return `source-scan-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export async function listSourceFilesFromTauri(
  root: string,
  query = '',
  limit = defaultSourceScanLimit,
  scanId: string | null = null
): Promise<SourceScanResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceScanResult>('list_source_files', {
    root,
    limit,
    query: query.trim() || null,
    scanId
  });
}

export async function cancelSourceScanFromTauri(scanId: string): Promise<boolean> {
  if (!isTauriRuntime() || !scanId.trim()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('cancel_source_scan', { scanId });
}

export async function listenToSourceScanProgress(
  handler: (progress: NativeSourceScanProgress) => void
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<NativeSourceScanProgress>(nativeSourceScanProgressEvent, (event) => {
    handler(event.payload);
  });
}

export async function readSourceFromTauri(record: SourceRecord): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  const preview = await invoke<SourcePreview>('read_source_file', { path: record.path });
  return {
    ...preview,
    relativePath: record.relativePath,
    language: record.language,
    byteCount: record.byteCount
  };
}

export async function writeSourceToTauri(
  record: SourceRecord,
  content: string
): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  const preview = await invoke<SourcePreview>('write_source_file', {
    path: record.path,
    content
  });
  return {
    ...preview,
    relativePath: record.relativePath,
    language: record.language
  };
}

export async function openSourceFileFromTauri(path: string): Promise<boolean> {
  return runPathCommand('open_source_file', path);
}

export async function revealSourceFileFromTauri(path: string): Promise<boolean> {
  return runPathCommand('reveal_source_file', path);
}

export async function openPathFromTauri(path: string): Promise<boolean> {
  return runPathCommand('open_path', path);
}

export async function revealPathFromTauri(path: string): Promise<boolean> {
  return runPathCommand('reveal_path', path);
}

export async function openTerminalPathFromTauri(
  path: string,
  terminal = 'Warp'
): Promise<boolean> {
  if (!isTauriRuntime() || !path.trim()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('open_terminal_path', {
    path,
    terminal: terminal.trim() || null
  });
  return true;
}

export async function readProjectGitStatusFromTauri(
  root: string
): Promise<ProjectGitStatus | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProjectGitStatus>('project_git_status', { root });
}

export async function readSourceGitDiffFromTauri(
  root: string,
  path: string
): Promise<SourceGitDiff | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceGitDiff>('read_source_git_diff', { root, path });
}

export async function stageGitPathsFromTauri(
  root: string,
  paths: string[]
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('stage_git_paths', { root, paths });
}

export async function unstageGitPathsFromTauri(
  root: string,
  paths: string[]
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('unstage_git_paths', { root, paths });
}

export async function commitGitRepositoryFromTauri(
  root: string,
  message: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('commit_git_repository', { root, message });
}

export async function fetchGitRepositoryFromTauri(
  root: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('fetch_git_repository', { root });
}

export async function pullGitRepositoryFromTauri(
  root: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('pull_git_repository', { root });
}

export async function pushGitRepositoryFromTauri(
  root: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('push_git_repository', { root });
}

export async function readGitCommitHistoryFromTauri(
  root: string,
  limit = 24
): Promise<GitCommitHistoryEntry[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitCommitHistoryEntry[]>('read_git_commit_history', { root, limit });
}

export async function listProjectWorktreesFromTauri(
  root: string
): Promise<ProjectWorktree[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProjectWorktree[]>('list_project_worktrees', { root });
}

export async function listGitRepositorySummariesFromTauri(
  projects: RuntimeContextProject[]
): Promise<GitRepositorySummary[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitRepositorySummary[]>('list_git_repository_summaries', { projects });
}

export async function listAgentSessionsFromTauri(): Promise<AgentSession[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentSession[]>('list_agent_sessions');
}

export async function listRuntimeContextsFromTauri(
  projects: RuntimeContextProject[]
): Promise<RuntimeContext[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<RuntimeContext[]>('list_runtime_contexts', { projects });
}

export async function searchSourceFilesFromTauri(
  records: SourceRecord[],
  query: string,
  limit = 50
): Promise<SourceSearchMatch[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceSearchMatch[]>('search_source_files', {
    records,
    query,
    limit
  });
}

export async function findSourceDefinitionsFromTauri(
  records: SourceRecord[],
  symbolName: string,
  limit = 20
): Promise<SourceDefinitionTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDefinitionTarget[]>('find_source_definitions', {
    records,
    symbolName,
    limit
  });
}

export async function findSourceReferencesFromTauri(
  records: SourceRecord[],
  symbolName: string,
  limit = 50
): Promise<SourceReferenceTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceReferenceTarget[]>('find_source_references', {
    records,
    symbolName,
    limit
  });
}

async function runPathCommand(command: string, path: string): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(command, { path });
  return true;
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
