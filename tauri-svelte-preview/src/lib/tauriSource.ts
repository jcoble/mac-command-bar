import type {
  ProjectRoot,
  SourceCodeAction,
  SourceDiagnostic,
  SourceCompletionItem,
  SourceDefinitionTarget,
  SourceDocumentHighlight,
  SourceInlayHint,
  SourceLspCodeActionRequest,
  SourceLspHover,
  SourceLspLookupRequest,
  SourceLspRenameRequest,
  SourceLspWorkspaceSymbolRequest,
  SourceLspStatus,
  SourcePreview,
  SourceRecord,
  SourceReferenceCountResult,
  SourceReferenceTarget,
  SourceRenameResult,
  SourceScanResult,
  SourceSearchMatch,
  SourceSemanticToken,
  SourceSignatureHelp,
  SourceSymbol,
  SourceTextEdit,
  SourceWorkspaceSymbol
} from './sourceData';

export const defaultSourceScanLimit = 10_000;
export const expandedSourceScanLimit = 25_000;
export const nativeSourceScanProgressEvent = 'source_scan_progress';
export const terminalOutputEvent = 'terminal_output';

export type NativeSourceScanProgress = {
  scanId: string;
  visitedEntries: number;
  matchedFiles: number;
};

export type TerminalStartRequest = {
  cwd: string;
  shell?: string | null;
  cols?: number | null;
  rows?: number | null;
  ownedId?: string | null;
};

export type TerminalSessionInfo = {
  sessionId: string;
  cwd: string;
  shell: string;
  cols: number;
  rows: number;
  pid: number | null;
  startedAt: number;
  exited: boolean;
  exitCode: number | null;
  signal: string | null;
};

export type TerminalOutputPayload = {
  sessionId: string;
  data: string;
  terminated: boolean;
  exitCode: number | null;
  signal: string | null;
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
  hasUpstream: boolean;
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
  isPrunable?: boolean;
  prunableReason?: string | null;
  isLocked?: boolean;
  lockedReason?: string | null;
  lastActivity: string | null;
  deleteEligibility: string;
};

export type ProjectWorktreeActionResult = {
  message: string;
  worktrees: ProjectWorktree[];
};

export type ProjectWorktreeArchiveResult = ProjectWorktreeActionResult & {
  archivePath: string;
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
  hasUpstream: boolean;
  lastCommitSha: string | null;
  lastCommitSubject: string | null;
  lastCommitAt: string | null;
  dirtySinceEpochMs: number | null;
  dirtyStatusFingerprint: string;
  error: string | null;
};

export type GitCommitHistoryEntry = {
  shortSha: string;
  sha: string;
  subject: string;
  author: string;
  committedAt: string;
  refs: string;
  parentShas: string[];
  parentCount: number;
  taskID: string | null;
  taskSource: string | null;
};

export type AgentSession = {
  provider: string;
  id: string;
  title: string;
  description?: string | null;
  model: string | null;
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

export type PlaywrightProcessInfo = {
  pid: number;
  pgid: number;
  command: string;
  name: string;
  label: string;
  elapsed: string;
  args: string;
};

export type PlaywrightSessionInfo = {
  pgid: number;
  label: string;
  pids: number[];
  processes: PlaywrightProcessInfo[];
};

export type PlaywrightCleanupFailure = {
  pgid: number;
  pid: number | null;
  message: string;
};

export type PlaywrightCleanupResult = {
  sessions: PlaywrightSessionInfo[];
  terminatedPgids: number[];
  terminatedPids: number[];
  failedPgids: PlaywrightCleanupFailure[];
};

export type ProjectRootValidationResult = {
  path: string;
  exists: boolean;
  isDirectory: boolean;
  isGitRepository: boolean;
  gitRoot: string | null;
  message: string;
};

export type OrchestrationEvent = {
  schemaVersion: number;
  id: string;
  runId: string;
  timestamp: string;
  kind: string;
  status: string;
  title: string | null;
  message: string | null;
  projectID: string | null;
  projectName: string | null;
  projectPath: string | null;
  rootLabel: string | null;
  taskID: string | null;
  agentId: string | null;
  agentProvider: string | null;
  agentRole: string | null;
  stepId: string | null;
  stepKind: string | null;
  artifactId: string | null;
  artifactKind: string | null;
  artifactPath: string | null;
  artifactUrl: string | null;
  linkKind: string | null;
  linkLabel: string | null;
  linkUrl: string | null;
  scenario?: string | null;
  issueID?: string | null;
  retryAttempt?: number | null;
  approvalSubject?: string | null;
  blockerReason?: string | null;
  decisionPrompt?: string | null;
  scenarioCount?: number | null;
  issueCount?: number | null;
  testCount?: number | null;
  retestCount?: number | null;
  fixCount?: number | null;
  resolvedCount?: number | null;
  verifiedCount?: number | null;
  delegatedCount?: number | null;
  decisionCount?: number | null;
  approvalCount?: number | null;
  failedCount?: number | null;
};

export type OrchestrationRun = {
  id: string;
  title: string;
  status: string;
  phase: string;
  progress: number;
  projectID: string | null;
  projectName: string;
  projectPath: string;
  rootLabel: string;
  taskID: string | null;
  startedAt: string | null;
  updatedAt: string | null;
  summary: string;
  agents: OrchestrationAgent[];
  steps: OrchestrationStep[];
  artifacts: OrchestrationArtifact[];
  links: OrchestrationLink[];
  events: OrchestrationEvent[];
};

export type OrchestrationAgent = {
  id: string;
  provider: string;
  role: string;
  status: string;
  title: string;
  lastActivity: string | null;
};

export type OrchestrationStep = {
  id: string;
  kind: string;
  title: string;
  status: string;
  summary: string;
  agentId: string | null;
  startedAt: string | null;
  finishedAt: string | null;
};

export type OrchestrationArtifact = {
  id: string;
  kind: string;
  title: string;
  path: string | null;
  url: string | null;
  status: string;
};

export type OrchestrationLink = {
  kind: string;
  label: string;
  url: string;
};

export function createSourceScanId(): string {
  return `source-scan-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export function isNativeTauriRuntime(): boolean {
  return isTauriRuntime();
}

export async function validateProjectRootFromTauri(
  path: string
): Promise<ProjectRootValidationResult | null> {
  if (!path.trim()) {
    return null;
  }

  if (!isTauriRuntime()) {
    return postLocalSourceBridge<ProjectRootValidationResult>('validate', { path });
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProjectRootValidationResult>('validate_project_root', { path });
}

export async function listSourceFilesFromTauri(
  root: string,
  query = '',
  limit = defaultSourceScanLimit,
  scanId: string | null = null
): Promise<SourceScanResult | null> {
  if (!isTauriRuntime()) {
    return postLocalSourceBridge<SourceScanResult>('list', {
      root,
      query: query.trim() || null,
      limit,
      scanId
    });
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

export async function startTerminalSessionFromTauri(
  request: TerminalStartRequest
): Promise<TerminalSessionInfo | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<TerminalSessionInfo>('start_terminal_session', { request });
}

export async function listTerminalSessionsFromTauri(): Promise<TerminalSessionInfo[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<TerminalSessionInfo[]>('list_terminal_sessions');
}

export async function readTerminalSessionScrollbackFromTauri(
  sessionId: string
): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<string | null>('read_terminal_session_scrollback', { sessionId });
}

export async function writeTerminalSessionFromTauri(
  sessionId: string,
  data: string
): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('write_terminal_session', { sessionId, data });
}

export async function resizeTerminalSessionFromTauri(
  sessionId: string,
  cols: number,
  rows: number
): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('resize_terminal_session', { sessionId, cols, rows });
}

export async function closeTerminalSessionFromTauri(sessionId: string): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('close_terminal_session', { sessionId });
}

export async function listenToTerminalOutput(
  handler: (payload: TerminalOutputPayload) => void
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<TerminalOutputPayload>(terminalOutputEvent, (event) => {
    handler(event.payload);
  });
}

export async function readSourceFromTauri(record: SourceRecord): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) {
    const preview = await postLocalSourceBridge<SourcePreview>('read', { path: record.path });
    return preview
      ? {
          ...preview,
          relativePath: record.relativePath,
          language: record.language,
          byteCount: record.byteCount
        }
      : null;
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
    const preview = await postLocalSourceBridge<SourcePreview>('write', {
      path: record.path,
      content
    });
    return preview
      ? {
          ...preview,
          relativePath: record.relativePath,
          language: record.language
        }
      : null;
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

export async function openTerminalCommandFromTauri(
  path: string,
  command: string,
  terminal = 'Warp'
): Promise<boolean> {
  if (!isTauriRuntime() || !path.trim() || !command.trim()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('open_terminal_command', {
    path,
    command,
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

export async function removeProjectWorktreeFromTauri(
  root: string,
  path: string
): Promise<ProjectWorktreeActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProjectWorktreeActionResult>('remove_project_worktree', { root, path });
}

export async function archiveProjectWorktreeFromTauri(
  root: string,
  path: string
): Promise<ProjectWorktreeArchiveResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProjectWorktreeArchiveResult>('archive_project_worktree', { root, path });
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

export async function listAgentSessionsFromLocalBridge(): Promise<AgentSession[] | null> {
  return postLocalSourceBridge<AgentSession[]>('agent-sessions', {});
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

export async function listPlaywrightSessionsFromTauri(): Promise<PlaywrightSessionInfo[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PlaywrightSessionInfo[]>('list_playwright_sessions');
}

export async function killPlaywrightSessionsFromTauri(): Promise<PlaywrightCleanupResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PlaywrightCleanupResult>('kill_playwright_sessions');
}

export async function listOrchestrationRunsFromTauri(
  projects: RuntimeContextProject[]
): Promise<OrchestrationRun[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<OrchestrationRun[]>('list_orchestration_runs', { projects });
}

export async function recordOrchestrationEventToTauri(
  event: OrchestrationEvent
): Promise<OrchestrationRun | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<OrchestrationRun>('record_orchestration_event', { event });
}

export async function searchSourceFilesFromTauri(
  records: SourceRecord[],
  query: string,
  limit = 50
): Promise<SourceSearchMatch[] | null> {
  if (!isTauriRuntime()) {
    return postLocalSourceBridge<SourceSearchMatch[]>('search', {
      records,
      query,
      limit
    });
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
    return postLocalSourceBridge<SourceDefinitionTarget[]>('definitions', {
      records,
      symbolName,
      limit
    });
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDefinitionTarget[]>('find_source_definitions', {
    records,
    symbolName,
    limit
  });
}

export async function readSourceLspStatusFromTauri(
  root: string,
  language: string
): Promise<SourceLspStatus | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceLspStatus>('read_source_lsp_status', {
    root,
    language
  });
}

export async function readSourceLspReadinessFromTauri(
  root: string
): Promise<SourceLspStatus[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceLspStatus[]>('list_source_lsp_statuses', { root });
}

/**
 * Proactively re-point any already-running language server(s) at a freshly-selected
 * project root so the cold re-index warms in the background on switch, rather than on the
 * first file-open under the new project. No-op (returns null) outside the Tauri runtime,
 * and a no-op in the backend when no server is running for that root's languages. Returns
 * the count of running servers that were re-pointed.
 */
export async function warmSourceLspForRootFromTauri(root: string): Promise<number | null> {
  if (!isTauriRuntime() || !root.trim()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<number>('warm_source_lsp_for_root', { root });
}

export async function findSourceLspDefinitionsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceDefinitionTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDefinitionTarget[]>('find_source_lsp_definitions', {
    preview,
    request
  });
}

export async function findSourceLspCompletionsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceCompletionItem[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceCompletionItem[]>('find_source_lsp_completions', {
    preview,
    request
  });
}

export async function findSourceLspReferencesFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceReferenceTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceReferenceTarget[]>('find_source_lsp_references', {
    preview,
    request
  });
}

export async function findSourceLspImplementationsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceDefinitionTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDefinitionTarget[]>('find_source_lsp_implementations', {
    preview,
    request
  });
}

export async function findSourceLspTypeDefinitionsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceDefinitionTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDefinitionTarget[]>('find_source_lsp_type_definitions', {
    preview,
    request
  });
}

export async function findSourceLspDocumentHighlightsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceDocumentHighlight[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDocumentHighlight[]>('find_source_lsp_document_highlights', {
    preview,
    request
  });
}

export async function formatSourceWithLspFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceTextEdit[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceTextEdit[]>('format_source_with_lsp', {
    preview,
    request
  });
}

export async function renameSourceWithLspFromTauri(
  preview: SourcePreview,
  request: SourceLspRenameRequest
): Promise<SourceRenameResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceRenameResult>('rename_source_with_lsp', {
    preview,
    request
  });
}

export async function findSourceLspCodeActionsFromTauri(
  preview: SourcePreview,
  request: SourceLspCodeActionRequest
): Promise<SourceCodeAction[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceCodeAction[]>('find_source_lsp_code_actions', {
    preview,
    request
  });
}

export async function findSourceLspSignatureHelpFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceSignatureHelp | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceSignatureHelp | null>('find_source_lsp_signature_help', {
    preview,
    request
  });
}

export async function findSourceLspInlayHintsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceInlayHint[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceInlayHint[]>('find_source_lsp_inlay_hints', {
    preview,
    request
  });
}

export async function findSourceLspSemanticTokensFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceSemanticToken[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceSemanticToken[]>('find_source_lsp_semantic_tokens', {
    preview,
    request
  });
}

export async function findSourceLspHoverFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceLspHover | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceLspHover | null>('find_source_lsp_hover', {
    preview,
    request
  });
}

export async function findSourceLspSymbolsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceSymbol[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceSymbol[]>('find_source_lsp_symbols', {
    preview,
    request
  });
}

export async function findSourceLspWorkspaceSymbolsFromTauri(
  preview: SourcePreview,
  request: SourceLspWorkspaceSymbolRequest
): Promise<SourceWorkspaceSymbol[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceWorkspaceSymbol[]>('find_source_lsp_workspace_symbols', {
    preview,
    request
  });
}

export async function readSourceLspDiagnosticsFromTauri(
  preview: SourcePreview,
  request: SourceLspLookupRequest
): Promise<SourceDiagnostic[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDiagnostic[]>('read_source_lsp_diagnostics', {
    preview,
    request
  });
}

export async function findSourceReferencesFromTauri(
  records: SourceRecord[],
  symbolName: string,
  limit = 50
): Promise<SourceReferenceTarget[] | null> {
  if (!isTauriRuntime()) {
    return postLocalSourceBridge<SourceReferenceTarget[]>('references', {
      records,
      symbolName,
      limit
    });
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceReferenceTarget[]>('find_source_references', {
    records,
    symbolName,
    limit
  });
}

/**
 * Count how many lines mention each of `symbolNames`, across the whole project
 * under `root`, in one pass.
 *
 * Deliberately takes no file list: the backend walks the project itself. The
 * margin counts used to send the entire scanned file list across the bridge
 * once per symbol, and with a hundred-odd symbols on screen that alone was
 * enough to lock up the app.
 */
export async function countSourceReferencesFromTauri(
  root: string,
  symbolNames: string[],
  deadlineMs?: number
): Promise<SourceReferenceCountResult | null> {
  if (!isTauriRuntime()) {
    return postLocalSourceBridge<SourceReferenceCountResult>('reference-counts', {
      root,
      symbolNames,
      deadlineMs: deadlineMs ?? null
    });
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceReferenceCountResult>('count_source_references', {
    root,
    symbolNames,
    deadlineMs: deadlineMs ?? null
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

async function postLocalSourceBridge<T>(
  action: string,
  payload: Record<string, unknown>
): Promise<T | null> {
  if (typeof window === 'undefined') {
    return null;
  }

  let response: Response;
  try {
    response = await fetch(`/__mcb/source/${action}`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json'
      },
      body: JSON.stringify(payload)
    });
  } catch {
    return null;
  }

  const contentType = response.headers.get('content-type') ?? '';
  const text = await response.text();
  if (!contentType.includes('application/json')) {
    return null;
  }

  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    const message =
      body && typeof body === 'object' && 'error' in body
        ? String((body as { error: unknown }).error)
        : `Local source bridge failed with HTTP ${response.status}`;
    throw new Error(message);
  }

  return body as T;
}
