import type {
  ProjectRoot,
  SourceCodeAction,
  SourceDiagnostic,
  SourceCompletionItem,
  SourceDirectoryEntry,
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
import type {
  ImplementationReceipt,
  PlanReceipt,
  ReviewReceipt,
  SpecComplianceReceipt,
  VerificationReceipt,
  WorkflowDefinitionV1,
  WorkflowRunRecord
} from './shell/workflows/workflowTypes';
import {
  normalizeWorkspaceSnapshot,
  type SessionWorkspaceSnapshot
} from './shell/sessionWorkspaces';
import { trackTauriListener } from './shell/resourceDiagnostics.svelte.ts';

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
  /**
   * Run this one command instead of opening an interactive shell. The session
   * ends when the command does, and its exit code is the command's — which is
   * the only way a caller can tell "it finished" from "it crashed". Leave it out
   * and the session is the ordinary login shell it has always been.
   *
   * Desktop builds older than this field silently drop it and open a shell, so
   * anything that depends on the exit code must check
   * `hasBackendCapability('terminalCommandSpawn')` first — see
   * `src/lib/shell/backendCapabilities.ts`.
   */
  command?: string | null;
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
  kind?: 'user-pty' | 'agent-tool' | 'run-configuration' | 'browser-automation';
  ownedId?: string | null;
  toolTerminalIdentity?: {
    ownedId: string;
    turnId: string;
    toolCallId: string;
    terminalId: string;
  } | null;
};

export type ResourceSnapshot = import('./shell/resources/resourceTypes.ts').ResourceSnapshot;
export type ResourceDiskRoot = import('./shell/resources/resourceTypes.ts').ResourceDiskRoot;
export type ResourceStopRequest = import('./shell/resources/resourceTypes.ts').ResourceStopRequest;
export type ResourceCommandReceipt = import('./shell/resources/resourceTypes.ts').ResourceCommandReceipt;
export type ResourceUnavailable = import('./shell/resources/resourceTypes.ts').ResourceUnavailable;
export type ResourceCleanupRequest = import('./shell/resources/resourceTypes.ts').ResourceCleanupRequest;
export type ResourceCleanupReceipt = import('./shell/resources/resourceTypes.ts').ResourceCleanupReceipt;
export type DiskScanReport = import('./shell/resources/resourceTypes.ts').DiskScanReport;
export type ProviderUsageSnapshot = import('./shell/usage/usageTypes.ts').ProviderUsageSnapshot;
export type UsageHistoryQuery = import('./shell/usage/usageTypes.ts').UsageHistoryQuery;
export type UsageSummary = import('./shell/usage/usageTypes.ts').UsageSummary;
export type UsageBreakdownRow = import('./shell/usage/usageTypes.ts').UsageBreakdownRow;
export type UsageProviderSummaryRow = import('./shell/usage/usageTypes.ts').UsageProviderSummaryRow;
export type UsageDailyRow = import('./shell/usage/usageTypes.ts').UsageDailyRow;
export type UsageDailyTotalsRow = import('./shell/usage/usageTypes.ts').UsageDailyTotalsRow;
export type AgentConversationCapabilities = import('./shell/conversation/conversationTypes.ts').AgentCapabilities;
export type AgentConversationEvent = import('./shell/conversation/conversationTypes.ts').AgentConversationEvent;
export type AgentConversationSnapshot = import('./shell/conversation/conversationTypes.ts').AgentConversationSnapshot;
export type AgentConversationEventPage = import('./shell/conversation/conversationTypes.ts').AgentConversationEventPage;

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

export type AgentGenerationRequest = {
  root: string;
  ownedId: string;
  generation: number;
};

export type PullRequestContext = {
  branch: string;
  base: string;
  commits: string;
  diff: string;
};

export type PullRequestDetails = {
  title: string;
  description: string;
};

export type PullRequestCreated = {
  number: number;
  url: string;
};

export type PullRequestStatus = {
  number: number;
  url: string;
  state: string;
  checks: 'none' | 'pending' | 'passing' | 'failing' | string;
  checkSummary: string;
};

export type PullRequestSummary = {
  number: number;
  title: string;
  url: string;
  state: string;
  headBranch: string;
  isDraft: boolean;
  checks: 'none' | 'pending' | 'passing' | 'failing' | string;
  checkSummary: string;
};

export type GitBranchSummary = {
  name: string;
  isCurrent: boolean;
  upstream: string;
  subject: string;
};

export type GitBranchList = {
  current: string;
  branches: GitBranchSummary[];
};

export type GitStashEntry = {
  index: number;
  label: string;
  description: string;
};

export type SourceGitDiff = {
  relativePath: string;
  status: string;
  diff: string;
  isBinary: boolean;
  /** Full bounded text from HEAD (or the selected commit's first parent). */
  originalContent: string | null;
  /** Full bounded text from the working tree (or the selected commit). */
  modifiedContent: string | null;
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

export type GitHistoryPage = {
  root: string;
  relativePath: string | null;
  commits: GitCommitHistoryEntry[];
  nextCursor: string | null;
  complete: boolean;
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
  /**
   * The transcript file the scanner read this session out of, so the app can
   * open it, show it in the file manager, or copy its path. Left out when no
   * single file describes the session, in which case those actions stay off.
   */
  logPath?: string | null;
  /**
   * What the scanner worked out about the session from its title, folder and
   * resume command. All optional: the scanner leaves a field out entirely when
   * it found nothing, and older records predate them, so a row must treat a
   * missing field and an empty one the same way — draw no chip.
   */
  branchHint?: string | null;
  taskId?: string | null;
  pullRequestHint?: string | null;
  sourceLabel?: string | null;
  /**
   * How many turns of the conversation the scanner saw, and the last one of
   * them, already written as the row shows it (`You: …` / `Agent: …`).
   *
   * The count is a floor, not a total: the scanner reads a bounded window of
   * each transcript, so a long session reports the turns inside that window and
   * no more. Optional like the hints above — a scanner that found no
   * conversation leaves them out, and a row then shows neither.
   */
  messageCount?: number | null;
  latestTurnPreview?: string | null;
  /**
   * The last thing each side said, oldest first — at most the user's most
   * recent turn and the agent's most recent one.
   *
   * This is what an expanded card shows, and it is longer than the preview
   * above on purpose: the preview has one line of a row to live in, while the
   * card scrolls. Left out entirely when the scanner read no conversation.
   */
  latestTurns?: AgentSessionTurn[];
  /**
   * The repository this session's folder belongs to, as git reported it during
   * the scan. The History panel groups on this, so a repository's main checkout
   * and its worktrees sit together under the project folder's name. Left out
   * when the folder is gone from disk or was never in a repository.
   */
  projectRoot?: string | null;
};

/** One remembered turn of a scanned session: who spoke, and what they said. */
export type AgentSessionTurn = {
  speaker: 'user' | 'agent';
  text: string;
};

export type AgentConversationSessionMeta = {
  worktree: string | null;
  branch: string | null;
  title: string | null;
  project: string | null;
  ptySessionId: string | null;
  origin: 'app' | 'external' | null;
  source: 'scanned' | 'fresh' | null;
  viaCmux: boolean;
  resumeCommand: string | null;
  completedAt: string | null;
  settledAt: string | null;
  taskId: string | null;
  pullRequest: string | null;
  messageCount: number | null;
  latestTurnPreview: string | null;
  scannedLastActivity: string | null;
};

export type AgentConversationSessionRecord = AgentConversationSessionMeta & {
  ownedId: string;
  provider: import('./shell/conversation/conversationTypes.ts').AgentConversationProvider;
  model: string | null;
  effort: string | null;
  cwd: string;
  state: import('./shell/ownedSessions.ts').AgentRuntimeState;
  suspended: boolean;
  createdAtMs: number;
  lastActivityAtMs: number;
  activeTurnId: string | null;
  pendingPermission: boolean;
  pendingInput: boolean;
  nativeSessionId: string | null;
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

export async function listSourceDirectoryFromTauri(
  root: string,
  directory: string,
  includeExcluded = false
): Promise<SourceDirectoryEntry[] | null> {
  if (!isTauriRuntime()) {
    return postLocalSourceBridge<SourceDirectoryEntry[]>('list-directory', {
      root,
      directory,
      includeExcluded
    });
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDirectoryEntry[]>('list_source_directory', {
    root,
    directory,
    includeExcluded
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
  return trackTauriListener(
    await listen<NativeSourceScanProgress>(nativeSourceScanProgressEvent, (event) => {
      handler(event.payload);
    })
  );
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

export async function readResourceSnapshotFromTauri(): Promise<ResourceSnapshot | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceSnapshot>('read_resource_snapshot');
}

export async function readResourceDiskScanFromTauri(
  roots: ResourceDiskRoot[],
  maxDepth = 3,
  maxEntries = 2000
): Promise<DiskScanReport | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<DiskScanReport>('read_resource_disk_scan', { roots, maxDepth, maxEntries });
}

export async function stopOwnedResourceFromTauri(
  request: ResourceStopRequest
): Promise<ResourceCommandReceipt | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceCommandReceipt>('stop_owned_resource', { request });
}

export async function restartLanguageServerRootFromTauri(
  root: string
): Promise<ResourceUnavailable | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceUnavailable>('restart_language_server_root', { request: { root } });
}

export async function applyResourceMemoryPressureFromTauri(
  level: string
): Promise<ResourceUnavailable | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceUnavailable>('apply_resource_memory_pressure', { level });
}

export async function readLanguageServerLogFromTauri(
  root: string
): Promise<ResourceUnavailable | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceUnavailable>('read_language_server_log', { request: { root } });
}

export async function cleanupWorkspaceDiskEntryFromTauri(
  request: ResourceCleanupRequest
): Promise<ResourceCleanupReceipt | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceCleanupReceipt>('cleanup_workspace_disk_entry', { request });
}

export async function setActiveSourceRootFromTauri(root: string): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<string>('set_active_source_root', { request: { root } });
}

export async function readCurrentProviderUsageFromTauri(
  provider: string | null,
  instanceId: string | null
): Promise<ProviderUsageSnapshot | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProviderUsageSnapshot>('read_current_provider_usage', { provider, instanceId });
}

export async function readUsageSummaryFromTauri(query: UsageHistoryQuery = {}): Promise<UsageSummary | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<UsageSummary>('read_usage_summary', { query });
}

export async function readUsageBreakdownFromTauri(query: UsageHistoryQuery = {}): Promise<UsageBreakdownRow[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<UsageBreakdownRow[]>('read_usage_breakdown', { query });
}

export async function readUsageProviderSummaryFromTauri(query: UsageHistoryQuery = {}): Promise<UsageProviderSummaryRow[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<UsageProviderSummaryRow[]>('read_usage_provider_summary', { query });
}

export async function readUsageDailyFromTauri(query: UsageHistoryQuery = {}): Promise<UsageDailyRow[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<UsageDailyRow[]>('read_usage_daily', { query });
}

export async function readUsageDailyTotalsFromTauri(query: UsageHistoryQuery = {}): Promise<UsageDailyTotalsRow[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<UsageDailyTotalsRow[]>('read_usage_daily_totals', { query });
}

export async function refreshUsageHistoryFromTauri(): Promise<number | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<number>('refresh_usage_history');
}

/** Which service the helper model calls, and therefore which key it needs. */
export type HelperVendor = 'openai' | 'anthropic';

/** What Settings needs to draw the Helper section. The model lists come from
 *  the backend so that the ids live in one place. */
export type HelperSettingsView = {
  vendor: HelperVendor;
  model: string;
  hasKey: boolean;
  openaiModels: string[];
  anthropicModels: string[];
};

/** The answer to the Test button: a sentence either way. */
export type HelperTestResult = {
  ok: boolean;
  message: string;
};

export async function readHelperSettingsFromTauri(): Promise<HelperSettingsView | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<HelperSettingsView>('read_helper_settings');
}

export async function writeHelperSettingsFromTauri(
  settings: { vendor: HelperVendor; model: string }
): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('write_helper_settings', { settings });
}

/** Stores the key in the Keychain. An empty key removes the one that is there. */
export async function setHelperKeyFromTauri(vendor: HelperVendor, key: string): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('set_helper_key', { vendor, key });
}

export async function testHelperFromTauri(): Promise<HelperTestResult | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<HelperTestResult>('test_helper');
}

/**
 * Ask the helper model to do one small job and give back its answer.
 *
 * Unlike its neighbours this cannot answer with `null`: an empty answer and a
 * helper that is switched off would look the same to the caller. Away from the
 * desktop app there is no helper at all, so this rejects with the sentence to
 * show instead. The backend rejects the same way — one plain-English sentence,
 * "No key — helper off" among them.
 */
export async function runHelperJobFromTauri(
  job: 'title' | 'inspect',
  input: string
): Promise<string> {
  if (!isTauriRuntime()) throw new Error('The helper only runs in the desktop app.');
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<string>('run_helper_job', { job, input });
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
  return trackTauriListener(
    await listen<TerminalOutputPayload>(terminalOutputEvent, (event) => {
      handler(event.payload);
    })
  );
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

export async function readNativeCsharpFileFromTauri(
  root: string,
  path: string
): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourcePreview>('read_native_csharp_file', { root, path });
}

export async function ensureNativeCsharpLanguageClientFromTauri(
  root: string
): Promise<{ wsUrl: string; root: string } | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<{ wsUrl: string; root: string }>('ensure_native_csharp_language_client', { root });
}

/** What the desktop app says about one project's editor mode. */
export interface WorkspaceLanguageIntelligence {
  root: string;
  enabled: boolean;
  /** Language-server processes running for this project right now. */
  runningServers: number;
  /** Their process ids — the same numbers the resource view shows. */
  serverPids: number[];
  /** How many were stopped by this call. */
  stoppedServers: number;
  message: string;
}

/**
 * Read whether full mode is on for a project. Costs nothing and starts nothing.
 * `null` outside the desktop app: a browser tab has no language servers at all.
 */
export async function readWorkspaceLanguageIntelligenceFromTauri(
  root: string
): Promise<WorkspaceLanguageIntelligence | null> {
  if (!isTauriRuntime() || !root.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkspaceLanguageIntelligence>('read_workspace_language_intelligence', { root });
}

/**
 * Turn full mode on or off for a project. Pass the language of the file on
 * screen and turning it on starts that language's server now; leave it out and
 * the choice is only recorded, with the next file opened starting the server.
 * Off stops that project's language server now.
 */
export async function setWorkspaceLanguageIntelligenceFromTauri(
  root: string,
  enabled: boolean,
  language?: string | null
): Promise<WorkspaceLanguageIntelligence | null> {
  if (!isTauriRuntime() || !root.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkspaceLanguageIntelligence>('set_workspace_language_intelligence', {
    root,
    enabled,
    language: language ?? null
  });
}

export async function markNativeCsharpLanguageClientReadyFromTauri(root: string): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('mark_native_csharp_language_client_ready', { root });
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

/**
 * Move one path to the Finder's Trash. The file-system plugin's `remove`
 * deletes for good, so anything a person can undo goes through here instead.
 * Rejects when the path is outside the folders the window may change.
 */
export async function moveToTrashFromTauri(path: string): Promise<boolean> {
  if (!isTauriRuntime() || !path.trim()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('move_to_trash', { path });
  return true;
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

/**
 * Invoke one of the native browser commands when the shell is running inside
 * Tauri. The browser model keeps the command names and payload shapes in its
 * typed backend; this bridge owns the runtime check and IPC import.
 */
export async function invokeBrowserCommandFromTauri<T>(
  command: string,
  input: unknown
): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error('Native browser commands require the Tauri runtime');
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, input as Record<string, unknown>);
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

export async function discardGitPathsFromTauri(
  root: string,
  paths: string[]
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('discard_git_paths', { root, paths });
}

export async function discardAllGitChangesFromTauri(
  root: string,
  includeUntracked: boolean
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('discard_all_git_changes', { root, includeUntracked });
}

export async function listGitBranchesFromTauri(root: string): Promise<GitBranchList | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitBranchList>('list_git_branches', { root });
}

export async function createGitBranchFromTauri(
  root: string,
  name: string,
  checkout: boolean
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('create_git_branch', { root, name, checkout });
}

export async function switchGitBranchFromTauri(
  root: string,
  name: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('switch_git_branch', { root, name });
}

export async function stashGitChangesFromTauri(
  root: string,
  includeUntracked: boolean,
  message: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('stash_git_changes', { root, includeUntracked, message });
}

export async function popGitStashFromTauri(
  root: string,
  index: number | null
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('pop_git_stash', { root, index });
}

export async function listGitStashesFromTauri(root: string): Promise<GitStashEntry[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitStashEntry[]>('list_git_stashes', { root });
}

export async function amendGitCommitFromTauri(
  root: string,
  message: string
): Promise<GitActionResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitActionResult>('amend_git_commit', { root, message });
}

export async function listOpenPullRequestsFromTauri(
  root: string
): Promise<PullRequestSummary[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PullRequestSummary[]>('list_open_pull_requests', { root });
}

export async function generateCommitMessageFromTauri(
  request: AgentGenerationRequest
): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<string>('generate_commit_message', { request });
}

export async function readPullRequestContextFromTauri(
  root: string
): Promise<PullRequestContext | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PullRequestContext>('read_pull_request_context', { root });
}

export async function generatePullRequestDetailsFromTauri(
  request: AgentGenerationRequest
): Promise<PullRequestDetails | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PullRequestDetails>('generate_pull_request_details', { request });
}

export async function createPullRequestFromTauri(input: {
  root: string;
  title: string;
  description: string;
  base: string;
  draft: boolean;
}): Promise<PullRequestCreated | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PullRequestCreated>('create_pull_request', input);
}

export async function readPullRequestStatusFromTauri(
  root: string,
  branch: string
): Promise<PullRequestStatus | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<PullRequestStatus>('read_pull_request_status', { root, branch });
}

export async function readGitCommitHistoryFromTauri(
  root: string,
  cursor: string | null = null,
  relativePath: string | null = null
): Promise<GitHistoryPage | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GitHistoryPage>('read_git_commit_history', { root, cursor, relativePath });
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

export async function listAgentSessionsForProjectFromTauri(
  path: string
): Promise<AgentSession[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentSession[]>('list_agent_sessions_for_project', { projectPath: path });
}

/** One checkout of a repository: its own folder, or one of its worktrees. */
export type RepositoryCheckout = {
  path: string;
  branch: string;
  /** True for the repository's own folder rather than one of its worktrees. */
  isMain: boolean;
};

/**
 * The checkouts of each repository that are still on disk, keyed by the
 * repository root that was asked about.
 *
 * History draws its tree from this rather than from wherever sessions happen to
 * have been run, so a worktree that only ever hosted dispatched lanes still
 * appears. Deleted checkouts are left out by the backend.
 */
export async function listRepositoryCheckoutsFromTauri(
  roots: string[]
): Promise<Record<string, RepositoryCheckout[]> | null> {
  if (!isTauriRuntime() || roots.length === 0) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<Record<string, RepositoryCheckout[]>>('list_repository_checkouts', { roots });
}

export async function readAgentConversationCapabilitiesFromTauri(
  ownedId: string
): Promise<AgentConversationCapabilities | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentConversationCapabilities>('read_agent_conversation_capabilities', { ownedId });
}

export async function readAgentConversationSnapshotFromTauri(
  ownedId: string
): Promise<AgentConversationSnapshot | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentConversationSnapshot | null>('read_agent_conversation_snapshot', { ownedId });
}

export async function listAgentConversationSessionsFromTauri(): Promise<AgentConversationSessionRecord[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentConversationSessionRecord[]>('list_agent_conversation_sessions');
}

export async function listAgentConversationEventsFromTauri(
  ownedId: string,
  fromSequence = 0
): Promise<AgentConversationEvent[] | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentConversationEvent[]>('list_agent_conversation_events', { ownedId, fromSequence });
}

/** The page of stored events just older than `beforeSequence`, for scrolling up. */
export async function listAgentConversationEventsBeforeFromTauri(
  ownedId: string,
  beforeSequence: number,
  maxBytes: number
): Promise<AgentConversationEventPage | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentConversationEventPage>('list_agent_conversation_events_before', {
    ownedId,
    beforeSequence,
    maxBytes
  });
}

export async function updateAgentConversationSessionMetaFromTauri(input: {
  ownedId: string;
  model: string | null;
  effort: string | null;
  meta: AgentConversationSessionMeta;
}): Promise<AgentConversationSessionRecord | null> {
  if (!isTauriRuntime() || !input.ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AgentConversationSessionRecord>('update_agent_conversation_session_meta', {
    request: input
  });
}

/** Takes a conversation out of the store for good; the agent's own transcript
 * on disk stays. Answers whether there was anything to delete. */
export async function deleteAgentConversationSessionFromTauri(ownedId: string): Promise<boolean> {
  if (!isTauriRuntime() || !ownedId.trim()) return false;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('delete_agent_conversation_session', { ownedId });
}

export async function writeAgentConversationWorkspaceFromTauri(
  ownedId: string,
  snapshot: SessionWorkspaceSnapshot
): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  if (!ownedId.trim()) return false;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('write_agent_conversation_workspace', {
    ownedId,
    snapshotJson: JSON.stringify(snapshot)
  });
  return true;
}

export async function readAgentConversationWorkspaceFromTauri(
  ownedId: string
): Promise<SessionWorkspaceSnapshot | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  const snapshot = await invoke<string | null>('read_agent_conversation_workspace', { ownedId });
  if (snapshot === null) return null;
  const parsed: unknown = JSON.parse(snapshot);
  return normalizeWorkspaceSnapshot(parsed);
}

export async function deleteAgentConversationWorkspaceFromTauri(ownedId: string): Promise<void> {
  if (!isTauriRuntime() || !ownedId.trim()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('delete_agent_conversation_workspace', { ownedId });
}

export async function clearAgentConversationWorkspaceEditorsFromTauri(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('clear_agent_conversation_workspace_editors');
}

export async function clearAgentConversationWorkspaceTabsFromTauri(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('clear_agent_conversation_workspace_tabs');
}

export async function writeAssemblySettingFromTauri(
  settingKey: string,
  value: unknown
): Promise<void> {
  if (!isTauriRuntime() || !settingKey.trim()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<void>('write_assembly_setting', {
    settingKey,
    valueJson: JSON.stringify(value)
  });
}

export async function readAssemblySettingFromTauri(settingKey: string): Promise<unknown> {
  if (!isTauriRuntime() || !settingKey.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  const valueJson = await invoke<string | null>('read_assembly_setting', { settingKey });
  return valueJson === null ? null : JSON.parse(valueJson) as unknown;
}

/** Everything the backend needs to read a past session's transcript file in. */
export interface AgentConversationTranscriptImport {
  provider: import('./shell/conversation/conversationTypes.ts').AgentConversationProvider;
  nativeSessionId: string;
  transcriptPath: string;
  cwd: string;
  /** What the past session was already called. Stored with the row, so the rail
   * still knows the name after it next reloads from the database. */
  title?: string | null;
}

/**
 * Name a provider's past transcript as an app-owned conversation and hand back
 * its id, without reading any of it. This is the fast half of resuming: the
 * session exists, with its name and its agent, and can be shown at once.
 *
 * Reading the transcript is `finishAgentConversationImportFromTauri`, which the
 * caller runs next — alongside starting the agent, not before it.
 */
export async function beginAgentConversationImportFromTauri(
  request: AgentConversationTranscriptImport
): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<string>('begin_agent_conversation_import', { ...request });
}

/**
 * Read the end of a named import's transcript file into it, and report how many
 * events that added. The backend reads a bounded window of the file, so a long
 * session arrives with its most recent part first.
 */
export async function finishAgentConversationImportFromTauri(
  ownedId: string
): Promise<number | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<number>('finish_agent_conversation_import', { ownedId });
}

/** What one reach further back into a transcript found. */
export interface ExtendedImport {
  added: number;
  reachedStart: boolean;
}

/** Read further back into an imported transcript, past what it already holds.
 * Reports what that added and whether the beginning has now been reached. A
 * stretch of transcript holding nothing a reader wants is not the beginning,
 * so the two are answered separately. */
export async function extendAgentConversationImportFromTauri(
  ownedId: string
): Promise<ExtendedImport | null> {
  if (!isTauriRuntime() || !ownedId.trim()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ExtendedImport>('extend_agent_conversation_import', { ownedId });
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

export async function listWorkflowRunsFromTauri(): Promise<WorkflowRunRecord[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord[]>('list_workflow_runs');
}

export async function createWorkflowRunFromTauri(
  definition: WorkflowDefinitionV1,
  input: Record<string, unknown>,
  idempotencyKey: string
): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('create_workflow_run', { definition, input, idempotencyKey });
}

export async function startWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('start_workflow_run', { runId, idempotencyKey });
}

export async function pauseWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('pause_workflow_run', { runId, idempotencyKey });
}

export async function resumeWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('resume_workflow_run', { runId, idempotencyKey });
}

export async function cancelWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('cancel_workflow_run', { runId, idempotencyKey });
}

export async function retryWorkflowNodeFromTauri(runId: string, nodeId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('retry_workflow_node', { runId, nodeId, idempotencyKey });
}

export async function skipWorkflowNodeFromTauri(runId: string, nodeId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('skip_workflow_node', { runId, nodeId, idempotencyKey });
}

export async function approveWorkflowGateFromTauri(
  runId: string,
  nodeId: string,
  approval: { approved: boolean; [key: string]: unknown },
  idempotencyKey: string
): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('approve_workflow_gate', { runId, nodeId, approval, idempotencyKey });
}

export async function submitWorkflowResultFromTauri(
  runId: string,
  nodeId: string,
  result: ImplementationReceipt | ReviewReceipt | SpecComplianceReceipt | VerificationReceipt | PlanReceipt,
  idempotencyKey: string
): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('submit_workflow_result', { runId, nodeId, result, idempotencyKey });
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
/**
 * Open the web inspector on the shell's own window.
 *
 * A no-op in a browser tab, which has the browser's own inspector already.
 */
export async function openMainDevtoolsFromTauri(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('open_main_devtools');
}

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

export function isTauriRuntime(): boolean {
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
