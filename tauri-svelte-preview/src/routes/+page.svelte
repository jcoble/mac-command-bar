<script lang="ts">
  import {
    Activity,
    Braces,
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    ExternalLink,
    FileCode2,
    Folder,
    FolderGit2,
    FolderOpen,
    FolderSearch,
    History,
    MoreHorizontal,
    Network,
    PanelBottom,
    Plus,
    RefreshCw,
    RotateCcw,
    Save,
    Search,
    SplitSquareHorizontal,
    Terminal,
    Trash2,
    X
  } from '@lucide/svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import '@xterm/xterm/css/xterm.css';
  import { onMount, tick } from 'svelte';
  import type { FitAddon as XTermFitAddon } from '@xterm/addon-fit';
  import type { Terminal as XTermTerminal } from '@xterm/xterm';
  import MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import {
    cleanupPasteText,
    formatPasteCleanupStats,
    pasteCleanupModes,
    type PasteCleanupMode
  } from '$lib/pasteCleanup';
  import {
    orchestrationAgentActivityItems,
    orchestrationArtifactChips,
    orchestrationAttentionQueue,
    orchestrationCurrentActivity,
    orchestrationLinkChips,
    orchestrationLoopStageMetrics,
    orchestrationLoopTallyText,
    orchestrationRunHandoffText,
    orchestrationRunMetrics,
    orchestrationRunSummaryText,
    orchestrationRunStage,
    orchestrationStatusTone,
    orchestrationTimelineItems,
    type OrchestrationTimelineItem
  } from '$lib/orchestrationView';
  import { sourcePreviewAppearance, sourcePreviewAppearanceKey } from '$lib/sourcePreviewAppearance';
  import {
    buildWorktreeCleanupBrief,
    buildWorktreeCleanupScript,
    buildWorktreeSafetySummary,
    prioritizeWorktreesForCleanup,
    worktreePrimaryAction
  } from '$lib/worktreeSafety';
  import {
    createWorkspaceSnapshot,
    parseStoredWorkspaceSnapshot,
    restoreWorkspaceSnapshot,
    snapshotStorageKey,
    upsertWorkspaceSnapshot,
    type WorkspaceSnapshot,
    type WorkspaceSnapshotEmbeddedTerminal,
    type WorkspaceSnapshotProvider
  } from '$lib/workspaceSnapshot';
  import {
    activateSourceDockPanel,
    createDefaultSourceDockLayout,
    hideSourceDockPanel,
    moveSourceDockPanel,
    normalizeSourceDockLayout,
    resizeSourceDockGroup,
    showSourceDockPanel,
    sourceDockGroupSize,
    sourceDockPanelDescriptors,
    type SourceDockGroupID,
    type SourceDockLayout,
    type SourceDockPanelID
  } from '$lib/sourceDockLayout';
  import {
    applySourceTextEdits,
    buildSourceTree,
    buildGitTaskSourceGroups,
    closeAllCleanOpenSourceTabs,
    closeOpenSourceTab,
    closeOtherCleanOpenSourceTabs,
    createProjectRoot,
    defaultProjectRoots,
    demoPreviewFor,
    filterSourceRecords,
    findAdjacentSourceDiagnostic,
    findSourceDefinitionTargets,
    findSourceReferenceTargets,
    findSourceSearchMatches,
    formatGitBranchHealthSummary,
    gitCommitGraphKind,
    gitCommitTopologyLabel,
    gitRefLabels,
    formatSourceContextGitSummary,
    formatSourceContextIdentity,
    formatSourceContextRootLabel,
    formatSourceDiagnosticSummary,
    formatSourceIndexSummary,
    flattenSourceTree,
    formatSourceRecordCount,
    formatSourceScanStats,
    formatSourceScanSummary,
    folderIdsForSourceRecord,
    getSourceScanCacheEntry,
    isSuspiciousSourceScanResult,
    mergeProjectRoots,
    normalizeProjectPath,
    navigateSourceHistoryBack,
    navigateSourceHistoryForward,
    parseQuickOpenQuery,
    previewFromContent,
    pushSourceNavigationHistory,
    rankSourceRecords,
    removeSourceScanCacheEntries,
    scrollTopForSourceTreeReveal,
    selectBackgroundIndexProjects,
    selectPreferredSourceRecord,
    sourceNavigationLocationForRecord,
    shouldRepairSuspiciousSourceScan,
    sourceLanguageForPath,
    sourceSupportsLanguageIntelligence,
    taskReferenceUrl,
    textMatchesSearchTokens,
    uniqueTaskIDsFromGitMetadata,
    upsertSourceScanCacheEntry,
    upsertOpenSourceTab,
    upsertRecentSourceRecord,
    virtualizeSourceTreeRows,
    type ProjectRoot,
    type SourceCodeAction,
    type SourceCodeActionLookupRequest,
    type SourceScanCache,
    type SourceOpenTab,
    type SourcePreview,
    type SourceRecentRecord,
    type SourceRecord,
    type SourceDefinitionTarget,
    type SourceReferenceTarget,
    type SourceSearchMatch,
    type SourceScanStats,
    type SourceCompletionItem,
    type SourceDiagnostic,
    type SourceInlayHint,
    type SourceLspHover,
    type SourceLspStatus,
    type SourceNavigationLocation,
    type SourceRenameFileEdit,
    type SourceRenameResult,
    type SourceSemanticToken,
    type SourceSignatureHelp,
    type SourceSymbol,
    type SourceTextEdit,
    type SourceLanguage,
    type SourceTreeNode,
    type SourceTreeRow,
    type SourceWorkspaceSymbol
  } from '$lib/sourceData';
  import {
    cancelSourceScanFromTauri,
    commitGitRepositoryFromTauri,
    createSourceScanId,
    expandedSourceScanLimit,
    fetchGitRepositoryFromTauri,
    findSourceDefinitionsFromTauri,
    findSourceLspCompletionsFromTauri,
    findSourceLspCodeActionsFromTauri,
    findSourceLspDefinitionsFromTauri,
    findSourceLspDocumentHighlightsFromTauri,
    findSourceLspHoverFromTauri,
    findSourceLspInlayHintsFromTauri,
    findSourceLspImplementationsFromTauri,
    findSourceLspReferencesFromTauri,
    findSourceLspSemanticTokensFromTauri,
    findSourceLspSignatureHelpFromTauri,
    findSourceLspSymbolsFromTauri,
    findSourceLspTypeDefinitionsFromTauri,
    findSourceLspWorkspaceSymbolsFromTauri,
    formatSourceWithLspFromTauri,
    findSourceReferencesFromTauri,
    listAgentSessionsFromTauri,
    listGitRepositorySummariesFromTauri,
    listOrchestrationRunsFromTauri,
    listProjectWorktreesFromTauri,
    listRuntimeContextsFromTauri,
    listTerminalSessionsFromTauri,
    listenToTerminalOutput,
    listenToSourceScanProgress,
    listSourceFilesFromTauri,
    nativeSourceScanProgressEvent,
    openPathFromTauri,
    openSourceFileFromTauri,
    openTerminalCommandFromTauri,
    openTerminalPathFromTauri,
    readProjectGitStatusFromTauri,
    readSourceLspDiagnosticsFromTauri,
    readSourceLspStatusFromTauri,
    readSourceGitDiffFromTauri,
    readSourceFromTauri,
    readTerminalSessionScrollbackFromTauri,
    removeProjectWorktreeFromTauri,
    revealPathFromTauri,
    revealSourceFileFromTauri,
    renameSourceWithLspFromTauri,
    resizeTerminalSessionFromTauri,
    pullGitRepositoryFromTauri,
    pushGitRepositoryFromTauri,
    readGitCommitHistoryFromTauri,
    searchSourceFilesFromTauri,
    stageGitPathsFromTauri,
    startTerminalSessionFromTauri,
    unstageGitPathsFromTauri,
    writeTerminalSessionFromTauri,
    closeTerminalSessionFromTauri,
    writeSourceToTauri,
    type NativeSourceScanProgress,
    type AgentSession,
    type GitCommitHistoryEntry,
    type GitRepositorySummary,
    type OrchestrationEvent,
    type OrchestrationRun,
    type ProjectGitFileStatus,
    type ProjectGitStatus,
    type ProjectWorktree,
    type RuntimeContext,
    type SourceGitDiff,
    type TerminalOutputPayload,
    type TerminalSessionInfo
  } from '$lib/tauriSource';

  const customProjectRootsStorageKey = 'mac-command-bar.source-browser.custom-project-roots';
  const selectedProjectStorageKey = 'mac-command-bar.source-browser.selected-project';
  const selectedSourcePathStorageKey = 'mac-command-bar.source-browser.selected-source-paths';
  const recentSourceRecordsStorageKey = 'mac-command-bar.source-browser.recent-source-records';
  const openSourceTabsStorageKey = 'mac-command-bar.source-browser.open-source-tabs';
  const sourceActivityModeStorageKey = 'mac-command-bar.source-browser.activity-mode';
  const sourceLayoutPresetStorageKey = 'mac-command-bar.source-browser.layout-preset';
  const sourceLayoutPresetOverridesStorageKey = 'mac-command-bar.source-browser.layout-preset-overrides';
  const sourceLayoutVersionStorageKey = 'mac-command-bar.source-browser.layout-version';
  const sourceTerminalAppStorageKey = 'mac-command-bar.source-browser.terminal-app';
  const sourceDockLayoutStorageKey = 'mac-command-bar.source-browser.dock-layout';
  const browserDockUrlStorageKey = 'mac-command-bar.source-browser.browser-url';
  const activeWorkspaceSessionStorageKey = 'mac-command-bar.source-browser.active-workspace-session';
  const pasteCleanupModeStorageKey = 'mac-command-bar.source-browser.paste-cleanup-mode';
  const contextPanelModeStorageKey = 'mac-command-bar.source-browser.context-panel-mode';
  const contextPanelPlacementStorageKey = 'mac-command-bar.source-browser.context-panel-placement';
  const sidePanePositionStorageKey = 'mac-command-bar.source-browser.side-pane-position';
  const sidePaneWidthStorageKey = 'mac-command-bar.source-browser.side-pane-width';
  const editorInsightWidthStorageKey = 'mac-command-bar.source-browser.editor-insight-width';
  const editorInsightCollapsedStorageKey = 'mac-command-bar.source-browser.editor-insight-collapsed';
  const contextPaneWidthStorageKey = 'mac-command-bar.source-browser.context-pane-width';
  const contextPaneHeightStorageKey = 'mac-command-bar.source-browser.context-pane-height';
  const contextPanelCollapsedStorageKey = 'mac-command-bar.source-browser.context-panel-collapsed';
  const hiddenContextCardsStorageKey = 'mac-command-bar.source-browser.hidden-context-cards';
  const activeContextCardStorageKey = 'mac-command-bar.source-browser.active-context-card';
  const maxWorkspaceSnapshots = 24;
  const maxRecentSourceRecords = 24;
  const maxProjectRecentRecords = 5;
  const maxProjectOpenSourceTabs = 8;
  const maxStoredOpenSourceTabs = 64;
  const maxSourceNavigationHistoryEntries = 64;
  const maxSourceSearchResults = 50;
  const maxSourceDefinitionResults = 20;
  const maxSourceCompletionResults = 50;
  const maxGitCommitHistoryEntries = 24;
  const commandCenterTaskUrls: Record<string, string> = {
    'TSK-127':
      'https://app.notion.com/p/TSK-127-Create-a-native-MAC-OS-app-for-doing-diff-things-in-menu-bar-379394b0689d8053af76fd44c7ffdba4',
    'TSK-192':
      'https://app.notion.com/p/TSK-192-Add-conversation-workspace-restore-snapshots-37c394b0689d810d9d74e798a144a3e9'
  };
  const sourceScanCacheMaxAgeMs = 5 * 60 * 1000;
  const maxSourceScanCacheEntries = 8;
  const suspiciousSourceIndexFileThreshold = 2;
  const sourceTreeRowHeight = 30;
  const sourceTreeOverscanRows = 8;
  const sourceTreeFallbackViewportHeight = 420;
  const sidePaneDefaultWidth = 407;
  const sidePaneMinWidth = 320;
  const sidePaneMaxWidth = 620;
  const editorInsightDefaultWidth = 260;
  const editorInsightMinWidth = 220;
  const editorInsightMaxWidth = 440;
  const contextPaneDefaultWidth = 330;
  const contextPaneMinWidth = 260;
  const contextPaneMaxWidth = 560;
  const contextPaneDefaultHeight = 260;
  const contextPaneMinHeight = 180;
  const contextPaneMaxHeight = 520;
  const sourceScanProgressEventName = nativeSourceScanProgressEvent;
  const expandedSourceScanLimitShortLabel = `${Math.round(expandedSourceScanLimit / 1000)}K`;
  const sourceLayoutVersion = '2026-06-editor-canvas';
  const initialProject = defaultProjectRoots[0];

  type SourceIntelligenceAction =
    | 'definition'
    | 'format'
    | 'hover'
    | 'implementation'
    | 'quick-fix'
    | 'references'
    | 'rename'
    | 'type-definition';
  type SourceEditorIntelligenceCommand = {
    id: number;
    action: SourceIntelligenceAction;
  };
  type SourceEditorLookupRequest = {
    symbolName: string;
    line: number;
    column: number;
  };
  type SourceEditorInlayHintLookupRequest = {
    startLine: number;
    startColumn: number;
    endLine: number;
    endColumn: number;
  };
  type SourceIntelligencePanel = 'problems' | 'symbols' | 'git';
  type SourceActivityMode =
    | 'files'
    | 'clipboard'
    | 'conversations'
    | 'runs'
    | 'sessions'
    | 'agents'
    | 'worktrees'
    | 'git';
  type SourceLayoutPresetID = 'review' | 'code' | 'git' | 'runs' | 'sessions' | 'custom';
  type SourceTerminalApp = 'Warp' | 'Terminal' | 'iTerm' | 'iTerm2' | 'Ghostty' | 'WezTerm' | 'Alacritty';
  type SourceContextPanelMode = 'grid' | 'stack';
  type SourceContextPanelPlacement = 'top' | 'side' | 'bottom';
  type SourceSidePanePosition = 'left' | 'right';
  type SourceContextCardID = 'orchestration' | 'runtime' | 'agents' | 'worktrees' | 'repo';
  type SourceLayoutPresetDefinition = {
    id: Exclude<SourceLayoutPresetID, 'custom'>;
    label: string;
    title: string;
    activityMode: SourceActivityMode;
    sidePaneWidth: number;
    sidePanePosition: SourceSidePanePosition;
    editorInsightWidth: number;
    editorInsightCollapsed: boolean;
    contextPanelCollapsed: boolean;
    contextPanelMode: SourceContextPanelMode;
    contextPanelPlacement: SourceContextPanelPlacement;
    intelligencePanel: SourceIntelligencePanel;
  };
  type ConcreteSourceLayoutPresetID = SourceLayoutPresetDefinition['id'];
  type SourceLayoutPresetOverride = Omit<SourceLayoutPresetDefinition, 'id' | 'label' | 'title'> & {
    contextPaneWidth: number;
    contextPaneHeight: number;
    dockLayout: SourceDockLayout;
    hiddenContextCardIDs: SourceContextCardID[];
    activeContextCardID: SourceContextCardID;
  };
  type SourceLayoutPresetOverrides = Partial<Record<ConcreteSourceLayoutPresetID, SourceLayoutPresetOverride>>;

  const sourceLayoutPresets: SourceLayoutPresetDefinition[] = [
    {
      id: 'review',
      label: 'Review',
      title: 'Balanced source review with context visible',
      activityMode: 'files',
      sidePaneWidth: sidePaneDefaultWidth,
      sidePanePosition: 'left',
      editorInsightWidth: editorInsightDefaultWidth,
      editorInsightCollapsed: false,
      contextPanelCollapsed: false,
      contextPanelMode: 'grid',
      contextPanelPlacement: 'top',
      intelligencePanel: 'symbols'
    },
    {
      id: 'code',
      label: 'Code',
      title: 'Wide editor with the context cards hidden',
      activityMode: 'files',
      sidePaneWidth: 360,
      sidePanePosition: 'left',
      editorInsightWidth: editorInsightMinWidth,
      editorInsightCollapsed: true,
      contextPanelCollapsed: true,
      contextPanelMode: 'grid',
      contextPanelPlacement: 'top',
      intelligencePanel: 'symbols'
    },
    {
      id: 'git',
      label: 'Git',
      title: 'Repository and task review with the Git inspector open',
      activityMode: 'git',
      sidePaneWidth: 440,
      sidePanePosition: 'left',
      editorInsightWidth: 340,
      editorInsightCollapsed: false,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      intelligencePanel: 'git'
    },
    {
      id: 'runs',
      label: 'Runs',
      title: 'Orchestration runs with agent, test, and artifact context',
      activityMode: 'runs',
      sidePaneWidth: 430,
      sidePanePosition: 'left',
      editorInsightWidth: 280,
      editorInsightCollapsed: true,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      intelligencePanel: 'problems'
    },
    {
      id: 'sessions',
      label: 'Sessions',
      title: 'Live runtime and agent/session review',
      activityMode: 'sessions',
      sidePaneWidth: 420,
      sidePanePosition: 'left',
      editorInsightWidth: 280,
      editorInsightCollapsed: true,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      intelligencePanel: 'problems'
    }
  ];
  const sourceTerminalApps: SourceTerminalApp[] = [
    'Warp',
    'Terminal',
    'iTerm',
    'iTerm2',
    'Ghostty',
    'WezTerm',
    'Alacritty'
  ];
  const dockTabGroupIDs: SourceDockGroupID[] = ['right', 'bottom'];
  const dockPanelDragDataType = 'application/x-mcb-dock-panel';
  const managedDockPanelIDs: SourceDockPanelID[] = ['activity', 'context', 'insights', 'terminal', 'browser'];
  const hideableDockPanelIDs: SourceDockPanelID[] = ['context', 'insights', 'terminal', 'browser'];
  const contextCardOrder: SourceContextCardID[] = ['orchestration', 'runtime', 'agents', 'worktrees', 'repo'];
  const contextCardLabels: Record<SourceContextCardID, string> = {
    orchestration: 'Runs',
    runtime: 'Runtime',
    agents: 'Agents',
    worktrees: 'Worktrees',
    repo: 'Git'
  };

  let customProjectRoots = $state<ProjectRoot[]>([]);
  let selectedSourcePaths = $state<Record<string, string>>({});
  let recentSourceRecords = $state<SourceRecentRecord[]>([]);
  let openSourceTabs = $state<SourceOpenTab[]>([]);
  let workspaceSnapshots = $state<WorkspaceSnapshot[]>([]);
  let activeWorkspaceSessionKey = $state<string | null>(null);
  let sourceScanCache = $state<SourceScanCache>({});
  let backgroundIndexingProjectIDs = $state<Set<string>>(new Set());
  let backgroundIndexErrorByProject = $state<Record<string, string>>({});
  let projectGitStatus = $state<ProjectGitStatus | null>(null);
  let projectGitLoading = $state(false);
  let projectGitError = $state('');
  let runtimeContexts = $state<RuntimeContext[]>([]);
  let runtimeContextsLoading = $state(false);
  let runtimeContextError = $state('');
  let runtimeContextSource = $state('browser preview');
  let projectWorktrees = $state<ProjectWorktree[]>([]);
  let projectWorktreesLoading = $state(false);
  let projectWorktreeError = $state('');
  let projectWorktreeSource = $state('browser preview');
  let gitRepositorySummaries = $state<GitRepositorySummary[]>([]);
  let gitRepositorySummariesLoading = $state(false);
  let gitRepositorySummaryError = $state('');
  let gitRepositorySummarySource = $state('browser preview');
  let gitCommitHistory = $state<GitCommitHistoryEntry[]>([]);
  let selectedGitCommitSha = $state('');
  let gitCommitHistoryLoading = $state(false);
  let gitCommitHistoryError = $state('');
  let gitCommitHistorySource = $state('browser preview');
  let orchestrationRuns = $state<OrchestrationRun[]>([]);
  let orchestrationRunsLoading = $state(false);
  let orchestrationRunError = $state('');
  let orchestrationRunSource = $state('browser preview');
  let agentSessions = $state<AgentSession[]>([]);
  let agentSessionsLoading = $state(false);
  let agentSessionError = $state('');
  let agentSessionSource = $state('browser preview');
  let selectedProjectID = $state(initialProject.id);
  let records = $state<SourceRecord[]>([]);
  let selectedRecord = $state<SourceRecord | null>(null);
  let selectedSourceLine = $state<number | null>(null);
  let selectedSourceLineRequestId = $state(0);
  let sourceNavigationBackStack = $state<SourceNavigationLocation[]>([]);
  let sourceNavigationForwardStack = $state<SourceNavigationLocation[]>([]);
  let preview = $state<SourcePreview | null>(null);
  let sourceDraftContentByPath = $state<Record<string, string>>({});
  let savedSourceContentByPath = $state<Record<string, string>>({});
  let workspaceEditSourceRecordsByPath = $state<Record<string, SourceRecord>>({});
  let sourceDiagnostics = $state<SourceDiagnostic[]>([]);
  let sourceLspDiagnostics = $state<SourceDiagnostic[]>([]);
  let sourceSymbols = $state<SourceSymbol[]>([]);
  let sourceLspStatus = $state<SourceLspStatus | null>(null);
  let sourceLspStatusLoading = $state(false);
  let sourceLspStatusError = $state('');
  let sourceDefinitionTargets = $state<SourceDefinitionTarget[]>([]);
  let sourceDefinitionQuery = $state('');
  let sourceDefinitionLoading = $state(false);
  let sourceDefinitionError = $state('');
  let sourceReferenceTargets = $state<SourceReferenceTarget[]>([]);
  let sourceReferenceQuery = $state('');
  let sourceReferenceLoading = $state(false);
  let sourceReferenceError = $state('');
  let sourceImplementationTargets = $state<SourceDefinitionTarget[]>([]);
  let sourceImplementationQuery = $state('');
  let sourceImplementationLoading = $state(false);
  let sourceImplementationError = $state('');
  let sourceTypeDefinitionTargets = $state<SourceDefinitionTarget[]>([]);
  let sourceTypeDefinitionQuery = $state('');
  let sourceTypeDefinitionLoading = $state(false);
  let sourceTypeDefinitionError = $state('');
  let selectedSourceGitDiff = $state<SourceGitDiff | null>(null);
  let selectedSourceGitDiffLoading = $state(false);
  let selectedSourceGitDiffError = $state('');
  let gitCommitMessage = $state('');
  let gitActionBusy = $state<'stage' | 'unstage' | 'commit' | 'fetch' | 'pull' | 'push' | ''>('');
  let gitActionStatus = $state('');
  let gitActionError = $state('');
  let sourceIntelligenceCommand = $state<SourceEditorIntelligenceCommand | null>(null);
  let sourceIntelligencePanel = $state<SourceIntelligencePanel>('symbols');
  let sourceSearchQuery = $state('');
  let sourceSearchResults = $state<SourceSearchMatch[]>([]);
  let sourceSearchLoading = $state(false);
  let sourceSearchError = $state('');
  let sourceSearchInput = $state<HTMLInputElement | null>(null);
  let sourceActivityMode = $state<SourceActivityMode>('files');
  let sourceActivityFilter = $state('');
  let pasteCleanupInput = $state('');
  let pasteCleanupMode = $state<PasteCleanupMode>('plain');
  let sourceLayoutPreset = $state<SourceLayoutPresetID>('code');
  let sourceLayoutPresetOverrides = $state<SourceLayoutPresetOverrides>({});
  let sourceTerminalApp = $state<SourceTerminalApp>('Warp');
  let embeddedTerminalSession = $state<TerminalSessionInfo | null>(null);
  let embeddedTerminalSessions = $state<TerminalSessionInfo[]>([]);
  let embeddedTerminalSessionsLoading = $state(false);
  let embeddedTerminalSessionsError = $state('');
  let embeddedTerminalStarting = $state(false);
  let embeddedTerminalStatus = $state('Embedded terminal idle');
  let embeddedTerminalError = $state('');
  let browserUrl = $state('');
  let browserInputUrl = $state('');
  let browserFrameKey = $state(0);
  let browserError = $state('');
  let contextPanelMode = $state<SourceContextPanelMode>('grid');
  let contextPanelPlacement = $state<SourceContextPanelPlacement>('top');
  let sidePanePosition = $state<SourceSidePanePosition>('left');
  let sidePaneWidth = $state(sidePaneDefaultWidth);
  let editorInsightWidth = $state(editorInsightDefaultWidth);
  let editorInsightCollapsed = $state(true);
  let contextPaneWidth = $state(contextPaneDefaultWidth);
  let contextPaneHeight = $state(contextPaneDefaultHeight);
  let contextPanelCollapsed = $state(true);
  let sourceDockLayout = $state<SourceDockLayout>(createDefaultSourceDockLayout());
  let draggingDockPanelID = $state<SourceDockPanelID | null>(null);
  let dockDropTargetGroupID = $state<SourceDockGroupID | null>(null);
  let hiddenContextCardIDs = $state<Set<SourceContextCardID>>(new Set());
  let activeContextCardID = $state<SourceContextCardID>('orchestration');
  let viewMenuOpen = $state(false);
  let editorActionMenuOpen = $state(false);
  let query = $state('');
  let expandedFolderIds = $state<Set<string>>(new Set());
  let loading = $state(true);
  let scanning = $state(false);
  let activeSourceScanId = $state('');
  let sourceScanProgress = $state<NativeSourceScanProgress | null>(null);
  let sourceScanStats = $state<SourceScanStats | null>(null);
  let scanLimitReached = $state(false);
  let runtime = $state('pending source scan');
  let error = $state('');
  let fileActionStatus = $state('');
  let fileActionBusy = $state('');
  let addingProject = $state(false);
  let choosingProjectRoot = $state(false);
  let quickOpenVisible = $state(false);
  let quickOpenQuery = $state('');
  let quickOpenIndex = $state(0);
  let quickOpenInput = $state<HTMLInputElement | null>(null);
  let workspaceSymbolResults = $state<SourceWorkspaceSymbol[]>([]);
  let workspaceSymbolLoading = $state(false);
  let workspaceSymbolError = $state('');
  let workspaceSymbolRequestID = 0;
  let commandPaletteVisible = $state(false);
  let commandPaletteQuery = $state('');
  let commandPaletteIndex = $state(0);
  let commandPaletteInput = $state<HTMLInputElement | null>(null);
  let fileTreeElement = $state<HTMLDivElement | null>(null);
  let embeddedTerminalElement = $state<HTMLDivElement | null>(null);
  let fileTreeScrollTop = $state(0);
  let fileTreeViewportHeight = $state(sourceTreeFallbackViewportHeight);
  let pendingTreeRevealPath = $state<string | null>(null);
  let pendingTreeFocusRowIndex = $state<number | null>(null);
  let projectNameInput = $state('');
  let projectPathInput = $state('');
  let projectFormError = $state('');
  let scanGeneration = 0;
  let sourceIntelligenceCommandId = 0;
  let sourceLspDiagnosticsTimer: number | null = null;
  let embeddedTerminal: XTermTerminal | null = null;
  let embeddedTerminalFitAddon: XTermFitAddon | null = null;
  let embeddedTerminalInputDisposable: { dispose: () => void } | null = null;
  let embeddedTerminalRendererLoading = false;

  type SourceScanOptions = {
    force?: boolean;
    limit?: number;
    skipTinyIndexRepair?: boolean;
  };
  type ProjectActivationOptions = {
    projects?: ProjectRoot[];
    forceScan?: boolean;
    scanLimit?: number;
  };
  type SourceCommandPaletteItem = {
    id: string;
    label: string;
    detail: string;
    disabled?: boolean;
    perform: () => void | Promise<void>;
  };
  type GitStatusGroupID = 'staged' | 'unstaged' | 'untracked';
  type GitStatusFileGroup = {
    id: GitStatusGroupID;
    label: string;
    files: ProjectGitFileStatus[];
    action: 'stage' | 'unstage';
  };

  let projectOptions = $derived(mergeProjectRoots(defaultProjectRoots, customProjectRoots));
  let selectedProject = $derived(
    projectOptions.find((project) => project.id === selectedProjectID) ?? projectOptions[0] ?? initialProject
  );
  let selectedProjectIsCustom = $derived(
    customProjectRoots.some((project) => project.id === selectedProject.id)
  );
  let filteredRecords = $derived(filterSourceRecords(records, query));
  let sourceTree = $derived(buildSourceTree(filteredRecords));
  let autoExpandFolders = $derived(query.trim().length > 0);
  let visibleTreeRows = $derived(flattenSourceTree(sourceTree, expandedFolderIds, autoExpandFolders));
  let virtualizedTreeRows = $derived(
    virtualizeSourceTreeRows(
      visibleTreeRows,
      fileTreeScrollTop,
      fileTreeViewportHeight,
      sourceTreeRowHeight,
      sourceTreeOverscanRows
    )
  );
  let projectRecentRecords = $derived(
    recentSourceRecords
      .filter((record) => record.projectID === selectedProject.id)
      .slice(0, maxProjectRecentRecords)
  );
  let projectOpenSourceTabs = $derived(
    openSourceTabs.filter((tab) => tab.projectID === selectedProject.id)
  );
  let parsedQuickOpenQuery = $derived(parseQuickOpenQuery(quickOpenQuery));
  let quickOpenWorkspaceSymbolMode = $derived(parsedQuickOpenQuery.searchQuery.startsWith('#'));
  let quickOpenWorkspaceSymbolQuery = $derived(
    quickOpenWorkspaceSymbolMode ? parsedQuickOpenQuery.searchQuery.slice(1).trim() : ''
  );
  let quickOpenResults = $derived(
    quickOpenWorkspaceSymbolMode ? [] : rankSourceRecords(records, quickOpenQuery, 12)
  );
  let quickOpenActiveResultCount = $derived(
    quickOpenWorkspaceSymbolMode ? workspaceSymbolResults.length : quickOpenResults.length
  );
  let sourceNavigationCanGoBack = $derived(sourceNavigationBackStack.length > 0);
  let sourceNavigationCanGoForward = $derived(sourceNavigationForwardStack.length > 0);
  let selectedIndex = $derived(
    selectedRecord ? records.findIndex((record) => record.path === selectedRecord?.path) + 1 : 0
  );
  let selectedSourceDraftContent = $derived(
    preview ? sourceDraftContentByPath[preview.path] ?? preview.content : ''
  );
  let selectedSourceDirty = $derived(preview ? isSourcePathDirty(preview.path) : false);
  let dirtyProjectSourceRecords = $derived(
    dirtySourceRecordsForProject(projectOpenSourceTabs, workspaceEditSourceRecordsByPath, selectedProject)
  );
  let cleanProjectOpenSourceTabCount = $derived(
    projectOpenSourceTabs.filter((tab) => !isSourcePathDirty(tab.path)).length
  );
  let otherCleanProjectOpenSourceTabCount = $derived(
    projectOpenSourceTabs.filter((tab) => tab.path !== selectedRecord?.path && !isSourcePathDirty(tab.path)).length
  );
  let sourceIntelligenceAvailable = $derived(
    preview ? sourceSupportsLanguageIntelligence(preview.language) : false
  );
  let sourceDiagnosticSummary = $derived(formatSourceDiagnosticSummary(sourceDiagnostics));
  let recordCountLabel = $derived(
    formatSourceRecordCount(filteredRecords.length, records.length, scanLimitReached)
  );
  let scanSummaryLabel = $derived(
    formatSourceScanSummary(filteredRecords.length, records.length, scanLimitReached, query)
  );
  let sourceScanNeedsAttention = $derived(
    !scanning &&
      !loading &&
      query.trim().length === 0 &&
      isSuspiciousSourceScanResult(
        records.length,
        scanLimitReached,
        expandedSourceScanLimit,
        suspiciousSourceIndexFileThreshold
      )
  );
  let sourceScanHealthNote = $derived(formatSourceScanHealthNote(records.length, sourceScanNeedsAttention));
  let selectedProjectIndexEntry = $derived(
    getSourceScanCacheEntry(
      sourceScanCache,
      selectedProject,
      expandedSourceScanLimit,
      Date.now(),
      sourceScanCacheMaxAgeMs
    )
  );
  let selectedProjectIndexSummary = $derived(
    formatSourceIndexSummary(
      selectedProjectIndexEntry,
      backgroundIndexingProjectIDs.has(selectedProject.id),
      backgroundIndexErrorByProject[selectedProject.id] ?? ''
    )
  );
  let sourceScanStatsLabel = $derived(
    formatSourceScanStats(sourceScanStats ?? selectedProjectIndexEntry?.stats ?? null)
  );
  let gitStatusByRelativePath = $derived(
    new Map((projectGitStatus?.files ?? []).map((fileStatus) => [fileStatus.relativePath, fileStatus]))
  );
  let selectedProjectGitChangedFiles = $derived(projectGitStatus?.files ?? []);
  let selectedProjectGitFileGroups = $derived(
    buildGitStatusFileGroups(selectedProjectGitChangedFiles)
  );
  let selectedProjectGitFileGroupSummary = $derived(
    formatGitStatusFileGroupSummary(selectedProjectGitFileGroups)
  );
  let selectedRecordGitStatus = $derived(gitStatusForSourceRecord(selectedRecord));
  let selectedGitPathActionDisabled = $derived(
    !selectedRecordGitStatus || selectedSourceDirty || gitActionBusy !== ''
  );
  let selectedGitUnstageDisabled = $derived(
    !selectedRecordGitStatus || !isGitFileStaged(selectedRecordGitStatus) || gitActionBusy !== ''
  );
  let gitHasStagedChanges = $derived(selectedProjectGitChangedFiles.some(isGitFileStaged));
  let gitCommitDisabled = $derived(
    gitCommitMessage.trim().length === 0 || !gitHasStagedChanges || gitActionBusy !== ''
  );
  let gitRemoteActionDisabled = $derived(projectGitLoading || Boolean(projectGitError) || gitActionBusy !== '');
  let gitCommitHistorySummary = $derived(
    formatGitCommitHistorySummary(
      gitCommitHistory.length,
      gitCommitHistoryLoading,
      gitCommitHistoryError,
      gitCommitHistorySource
    )
  );
  let selectedGitCommit = $derived(
    gitCommitHistory.find((entry) => entry.sha === selectedGitCommitSha) ?? gitCommitHistory[0] ?? null
  );
  let selectedSourceGitSummary = $derived(
    formatSelectedSourceGitSummary(
      selectedRecordGitStatus,
      selectedSourceGitDiff,
      selectedSourceGitDiffLoading,
      selectedSourceGitDiffError
    )
  );
  let projectGitSummary = $derived(
    formatSourceContextGitSummary(projectGitStatus, projectGitLoading, projectGitError)
  );
  let sourceContextIdentity = $derived(
    formatSourceContextIdentity(selectedProject, projectGitSummary, runtime)
  );
  let pasteCleanupOutput = $derived(cleanupPasteText(pasteCleanupInput, pasteCleanupMode));
  let pasteCleanupStats = $derived(formatPasteCleanupStats(pasteCleanupInput, pasteCleanupOutput));
  let selectedProjectRuntimeContexts = $derived(
    runtimeContexts.filter(
      (context) =>
        context.projectID === selectedProject.id || context.projectName === selectedProject.name
    )
  );
  let defaultBrowserUrl = $derived(
    selectedProjectRuntimeContexts[0] ? runtimeContextUrl(selectedProjectRuntimeContexts[0]) : ''
  );
  let activeBrowserUrl = $derived(browserUrl || defaultBrowserUrl);
  let selectedProjectAgentSessions = $derived(
    agentSessions.filter((session) => agentSessionMatchesProject(session, selectedProject))
  );
  let selectedProjectAgentSessionPaths = $derived(
    selectedProjectAgentSessions
      .map((session) => agentSessionProjectPath(session))
      .filter((path) => path.trim().length > 0)
  );
  let selectedProjectOrchestrationRuns = $derived(
    orchestrationRuns.filter((run) => orchestrationRunMatchesProject(run, selectedProject))
  );
  let filteredProjectOrchestrationRuns = $derived(
    selectedProjectOrchestrationRuns.filter((run) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        run.title,
        run.status,
        run.phase,
        run.summary,
        run.projectName,
        run.projectPath,
        run.rootLabel,
        run.taskID,
        ...run.agents.map((agent) => `${agent.provider} ${agent.role} ${agent.title}`),
        ...run.steps.map((step) => `${step.kind} ${step.title} ${step.status} ${step.summary}`)
      )
    )
  );
  let filteredProjectRuntimeContexts = $derived(
    selectedProjectRuntimeContexts.filter((context) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        context.command,
        context.cwd,
        context.projectName,
        context.rootLabel,
        context.port
      )
    )
  );
  let filteredProjectAgentSessions = $derived(
    selectedProjectAgentSessions.filter((session) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        session.provider,
        session.title,
        session.model,
        session.projectPath,
        session.lastActivity,
        agentSessionResumeCommand(session)
      )
    )
  );
  let filteredWorkspaceSnapshots = $derived(
    workspaceSnapshots.filter((snapshot) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        snapshot.title,
        snapshot.provider,
        snapshot.model,
        snapshot.project.name,
        snapshot.project.path,
        snapshot.cwd,
        snapshot.worktreePath,
        snapshot.branch,
        snapshot.selectedPath,
        snapshot.resumeCommand
      )
    )
  );
  let activeWorkspaceSnapshot = $derived(
    activeWorkspaceSessionKey
      ? workspaceSnapshots.find((snapshot) => snapshot.id === activeWorkspaceSessionKey) ?? null
      : null
  );
  let prioritizedProjectWorktrees = $derived(
    prioritizeWorktreesForCleanup(projectWorktrees, {
      primaryPath: selectedProject.path,
      activeSessionPaths: selectedProjectAgentSessionPaths
    })
  );
  let filteredProjectWorktrees = $derived(
    prioritizedProjectWorktrees.filter((worktree) => {
      const safety = projectWorktreeSafety(worktree);
      return activityTextMatchesFilter(
        sourceActivityFilter,
        worktree.repo,
        worktree.path,
        worktree.branch,
        worktree.taskID,
        worktree.deleteEligibility,
        worktree.lastActivity,
        safety.badge,
        safety.reason,
        safety.recommendation,
        safety.activityLabel
      );
    })
  );
  let filteredGitRepositorySummaries = $derived(
    gitRepositorySummaries.filter((summary) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        summary.projectName,
        summary.repo,
        summary.path,
        summary.rootLabel,
        summary.branch,
        summary.taskID,
        summary.lastCommitSha,
        summary.lastCommitSubject,
        summary.error
      )
    )
  );
  let filteredGitCommitHistory = $derived(
    gitCommitHistory.filter((entry) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        entry.sha,
        entry.shortSha,
        entry.subject,
        entry.author,
        entry.refs,
        entry.taskID
      )
    )
  );
  let selectedProjectRepositorySummaries = $derived(
    gitRepositorySummaries.filter(
      (summary) =>
        summary.projectID === selectedProject.id ||
        normalizeProjectPath(summary.path) === normalizeProjectPath(selectedProject.path)
    )
  );
  let selectedProjectPrimaryRepoSummary = $derived(selectedProjectRepositorySummaries[0] ?? null);
  let selectedProjectGitBranchHealth = $derived(
    formatGitBranchHealthSummary({
      branch: projectGitStatus?.branch ?? selectedProjectPrimaryRepoSummary?.branch ?? null,
      ahead: projectGitStatus?.ahead ?? selectedProjectPrimaryRepoSummary?.ahead ?? 0,
      behind: projectGitStatus?.behind ?? selectedProjectPrimaryRepoSummary?.behind ?? 0,
      stagedCount: projectGitStatus
        ? gitStatusGroupFileCount(selectedProjectGitFileGroups, 'staged')
        : selectedProjectPrimaryRepoSummary?.stagedCount ?? 0,
      unstagedCount: projectGitStatus
        ? gitStatusGroupFileCount(selectedProjectGitFileGroups, 'unstaged')
        : selectedProjectPrimaryRepoSummary?.unstagedCount ?? 0,
      untrackedCount: projectGitStatus
        ? gitStatusGroupFileCount(selectedProjectGitFileGroups, 'untracked')
        : selectedProjectPrimaryRepoSummary?.untrackedCount ?? 0,
      changedCount: projectGitStatus
        ? selectedProjectGitChangedFiles.length
        : selectedProjectPrimaryRepoSummary?.dirtyCount ?? 0,
      isDirty: projectGitStatus
        ? selectedProjectGitChangedFiles.length > 0
        : selectedProjectPrimaryRepoSummary?.isDirty ?? false,
      error: projectGitError || selectedProjectPrimaryRepoSummary?.error || null,
      rootLabel: selectedProjectPrimaryRepoSummary?.rootLabel ?? formatSourceContextRootLabel(selectedProject.path),
      lastCommitSha: selectedProjectPrimaryRepoSummary?.lastCommitSha ?? null
    })
  );
  let selectedProjectGitTaskIDs = $derived(
    uniqueTaskIDsFromGitMetadata(
      selectedProjectRepositorySummaries,
      projectWorktrees,
      gitCommitHistory
    )
  );
  let selectedProjectGitTaskSourceGroups = $derived(
    buildGitTaskSourceGroups(
      selectedProjectRepositorySummaries.map((summary) => ({
        taskID: summary.taskID,
        sourceLabel: 'repo',
        sourceDetail: `${summary.rootLabel} · ${summary.branch}`
      })),
      prioritizedProjectWorktrees.map((worktree) => ({
        taskID: worktree.taskID,
        sourceLabel: 'worktree',
        sourceDetail: `${worktree.branch} · ${projectWorktreeActivityLabel(worktree)}`
      })),
      gitCommitHistory.map((entry) => ({
        taskID: entry.taskID,
        sourceLabel: 'commit',
        sourceDetail: `${entry.shortSha} · ${entry.subject}`
      })),
      selectedProjectOrchestrationRuns.map((run) => ({
        taskID: run.taskID,
        sourceLabel: 'run',
        sourceDetail: run.title
      }))
    )
  );
  let runtimeContextSummary = $derived(
    formatRuntimeContextSummary(
      selectedProjectRuntimeContexts.length,
      runtimeContextsLoading,
      runtimeContextError,
      runtimeContextSource
    )
  );
  let projectWorktreeSummary = $derived(
    formatProjectWorktreeSummary(
      projectWorktrees.length,
      projectWorktreesLoading,
      projectWorktreeError,
      projectWorktreeSource
    )
  );
  let projectWorktreeSafetyStats = $derived(formatProjectWorktreeSafetyStats(projectWorktrees));
  let projectWorktreeCleanupBrief = $derived(
    buildWorktreeCleanupBrief(projectWorktrees, {
      primaryPath: selectedProject.path,
      activeSessionPaths: selectedProjectAgentSessionPaths
    })
  );
  let repoDashboardSummary = $derived(
    formatRepoDashboardSummary(
      gitRepositorySummaries,
      gitRepositorySummariesLoading,
      gitRepositorySummaryError,
      gitRepositorySummarySource
    )
  );
  let agentSessionSummary = $derived(
    formatAgentSessionSummary(
      selectedProjectAgentSessions.length,
      agentSessionsLoading,
      agentSessionError,
      agentSessionSource
    )
  );
  let orchestrationRunSummary = $derived(
    formatOrchestrationRunSummary(
      selectedProjectOrchestrationRuns.length,
      orchestrationRunsLoading,
      orchestrationRunError,
      orchestrationRunSource
    )
  );
  let visibleContextCards = $derived(visibleContextCardIDs());
  let activeContextCard = $derived(activeVisibleContextCardID());
  let sourceSearchSummary = $derived(
    formatSourceSearchSummary(sourceSearchResults.length, sourceSearchLoading, sourceSearchError)
  );
  let sourceDefinitionSummary = $derived(
    formatSourceDefinitionSummary(
      sourceDefinitionTargets.length,
      sourceDefinitionLoading,
      sourceDefinitionError,
      sourceDefinitionQuery
    )
  );
  let sourceReferenceSummary = $derived(
    formatSourceReferenceSummary(
      sourceReferenceTargets.length,
      sourceReferenceLoading,
      sourceReferenceError,
      sourceReferenceQuery
    )
  );
  let sourceImplementationSummary = $derived(
    formatSourceImplementationSummary(
      sourceImplementationTargets.length,
      sourceImplementationLoading,
      sourceImplementationError,
      sourceImplementationQuery
    )
  );
  let sourceTypeDefinitionSummary = $derived(
    formatSourceTypeDefinitionSummary(
      sourceTypeDefinitionTargets.length,
      sourceTypeDefinitionLoading,
      sourceTypeDefinitionError,
      sourceTypeDefinitionQuery
    )
  );
  let sourceActivityPanelLabel = $derived(sourceActivityLabel(sourceActivityMode));
  let sourceCommandPaletteItems = $derived<SourceCommandPaletteItem[]>([
    {
      id: 'quick-open',
      label: 'Open file',
      detail: 'Cmd+P',
      perform: openQuickOpen
    },
    {
      id: 'workspace-symbols',
      label: 'Search workspace symbols',
      detail: 'Cmd+P then #symbol',
      disabled: !preview || !sourceIntelligenceAvailable,
      perform: openWorkspaceSymbolQuickOpen
    },
    {
      id: 'go-to-line',
      label: 'Go to line',
      detail: selectedRecord ? `${selectedRecord.relativePath}:line` : 'No file',
      disabled: !selectedRecord,
      perform: openCurrentFileGoToLine
    },
    {
      id: 'source-search-focus',
      label: 'Search file contents',
      detail: 'Cmd+Shift+F',
      perform: focusGlobalSourceSearch
    },
    {
      id: 'navigate-back',
      label: 'Go back',
      detail: 'Cmd+[',
      disabled: !sourceNavigationCanGoBack,
      perform: navigateSourceBack
    },
    {
      id: 'navigate-forward',
      label: 'Go forward',
      detail: 'Cmd+]',
      disabled: !sourceNavigationCanGoForward,
      perform: navigateSourceForward
    },
    {
      id: 'scan-project',
      label: 'Scan current project',
      detail: selectedProject.name,
      disabled: scanning,
      perform: () => scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })
    },
    {
      id: 'scan-project-expanded',
      label: `Scan current project up to ${expandedSourceScanLimit.toLocaleString()} files`,
      detail: selectedProject.name,
      disabled: scanning,
      perform: () => scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })
    },
    {
      id: 'scan-reset-index',
      label: `Reset project index and scan up to ${expandedSourceScanLimit.toLocaleString()} files`,
      detail: selectedProjectIndexSummary,
      disabled: scanning,
      perform: () => resetProjectScanCache(selectedProject)
    },
    {
      id: 'source-copy-scan-diagnostic',
      label: 'Copy source scan diagnostic',
      detail: sourceScanHealthNote || selectedProjectIndexSummary,
      perform: copySourceScanDiagnosticBrief
    },
    {
      id: 'scan-stop',
      label: 'Stop source scan',
      detail: sourceScanProgress?.status ?? 'Cancel the active scanner',
      disabled: !scanning,
      perform: cancelSourceScan
    },
    {
      id: 'project-add-folder',
      label: 'Add project folder',
      detail: 'Choose a local repository',
      disabled: choosingProjectRoot,
      perform: chooseProjectRoot
    },
    {
      id: 'project-open-folder',
      label: 'Open project folder',
      detail: selectedProject.path,
      disabled: !selectedProject.path || fileActionBusy === `activity-open:${selectedProject.path}`,
      perform: () => openActivityPath(selectedProject.path)
    },
    {
      id: 'project-reveal-folder',
      label: 'Reveal project folder',
      detail: selectedProject.path,
      disabled: !selectedProject.path || fileActionBusy === `activity-reveal:${selectedProject.path}`,
      perform: () => revealActivityPath(selectedProject.path)
    },
    {
      id: 'project-open-terminal',
      label: `Open project in ${sourceTerminalApp}`,
      detail: selectedProject.path,
      disabled: !selectedProject.path || fileActionBusy === `activity-terminal:${selectedProject.path}`,
      perform: () => openActivityTerminalPath(selectedProject.path)
    },
    {
      id: 'terminal-open-project',
      label: `Open project shell in ${sourceTerminalApp}`,
      detail: selectedProject.path,
      disabled: !selectedProject.path || fileActionBusy === `activity-terminal:${selectedProject.path}`,
      perform: () => openActivityTerminalPath(selectedProject.path)
    },
    {
      id: 'conversation-save-snapshot',
      label: 'Save workspace snapshot',
      detail: selectedProject.name,
      perform: captureCurrentWorkspaceSnapshot
    },
    {
      id: 'conversation-restore-active',
      label: 'Restore active workspace snapshot',
      detail: activeWorkspaceSnapshot?.title ?? 'No active workspace',
      disabled: !activeWorkspaceSnapshot,
      perform: () => {
        if (activeWorkspaceSnapshot) restoreConversationWorkspaceSnapshot(activeWorkspaceSnapshot);
      }
    },
    {
      id: 'conversation-restore-latest',
      label: 'Restore latest workspace snapshot',
      detail: workspaceSnapshots[0]?.title ?? 'No saved workspace',
      disabled: workspaceSnapshots.length === 0,
      perform: () => {
        const snapshot = workspaceSnapshots[0];
        if (snapshot) restoreConversationWorkspaceSnapshot(snapshot);
      }
    },
    {
      id: 'conversation-resume-latest',
      label: 'Resume latest workspace in terminal',
      detail: workspaceSnapshots[0]?.title ?? 'No saved workspace',
      disabled: !workspaceSnapshots[0]?.resumeCommand,
      perform: () => {
        const snapshot = workspaceSnapshots[0];
        if (snapshot) openWorkspaceSnapshotTerminal(snapshot);
      }
    },
    {
      id: 'conversation-resume-latest-embedded',
      label: 'Resume latest workspace in embedded terminal',
      detail: workspaceSnapshots[0]?.title ?? 'No saved workspace',
      disabled: workspaceSnapshots.length === 0,
      perform: () => {
        const snapshot = workspaceSnapshots[0];
        if (snapshot) openWorkspaceSnapshotEmbeddedTerminal(snapshot);
      }
    },
    {
      id: 'conversation-delete-latest',
      label: 'Delete latest workspace snapshot',
      detail: workspaceSnapshots[0]?.title ?? 'No saved workspace',
      disabled: workspaceSnapshots.length === 0,
      perform: () => {
        const snapshot = workspaceSnapshots[0];
        if (snapshot) deleteWorkspaceSnapshot(snapshot);
      }
    },
    {
      id: 'conversation-copy-active-restore-plan',
      label: 'Copy active workspace restore plan',
      detail: activeWorkspaceSnapshot?.title ?? 'No active workspace',
      disabled: !activeWorkspaceSnapshot,
      perform: () => {
        if (activeWorkspaceSnapshot) copyWorkspaceSnapshotRestorePlan(activeWorkspaceSnapshot);
      }
    },
    {
      id: 'conversation-copy-latest-restore-plan',
      label: 'Copy latest workspace restore plan',
      detail: workspaceSnapshots[0]?.title ?? 'No saved workspace',
      disabled: workspaceSnapshots.length === 0,
      perform: () => {
        const snapshot = workspaceSnapshots[0];
        if (snapshot) copyWorkspaceSnapshotRestorePlan(snapshot);
      }
    },
    ...workspaceSnapshots.slice(0, 8).map((snapshot) => ({
      id: `conversation-copy-restore-plan-${snapshot.id}`,
      label: `Copy restore plan: ${snapshot.title}`,
      detail: workspaceSnapshotScopeLabel(snapshot),
      perform: () => copyWorkspaceSnapshotRestorePlan(snapshot)
    })),
    {
      id: 'save-file',
      label: 'Save file',
      detail: 'Cmd+S',
      disabled: !selectedSourceDirty || fileActionBusy === 'save',
      perform: saveSelectedSourceFile
    },
    {
      id: 'save-all-files',
      label: 'Save all dirty files',
      detail: `${dirtyProjectSourceRecords.length} dirty`,
      disabled: dirtyProjectSourceRecords.length === 0 || fileActionBusy === 'save-all',
      perform: saveAllDirtySourceFiles
    },
    {
      id: 'next-dirty-file',
      label: 'Go to next dirty file',
      detail: `${dirtyProjectSourceRecords.length} dirty`,
      disabled: dirtyProjectSourceRecords.length === 0,
      perform: () => selectAdjacentDirtySourceFile(1)
    },
    {
      id: 'previous-dirty-file',
      label: 'Go to previous dirty file',
      detail: `${dirtyProjectSourceRecords.length} dirty`,
      disabled: dirtyProjectSourceRecords.length === 0,
      perform: () => selectAdjacentDirtySourceFile(-1)
    },
    ...dirtyProjectSourceRecords.slice(0, 8).map((record) => ({
      id: `dirty-file-${record.path}`,
      label: `Open dirty file: ${record.fileName}`,
      detail: record.relativePath,
      perform: () => selectRecord(record)
    })),
    {
      id: 'close-current-tab',
      label: 'Close current tab',
      detail: selectedRecord?.fileName ?? 'No file',
      disabled: !selectedRecord,
      perform: closeSelectedSourceTab
    },
    {
      id: 'close-other-clean-tabs',
      label: 'Close other clean tabs',
      detail: `${otherCleanProjectOpenSourceTabCount} clean`,
      disabled: !selectedRecord || otherCleanProjectOpenSourceTabCount === 0,
      perform: closeOtherCleanSourceTabs
    },
    {
      id: 'close-clean-tabs',
      label: 'Close all clean tabs',
      detail: `${cleanProjectOpenSourceTabCount} clean`,
      disabled: cleanProjectOpenSourceTabCount === 0,
      perform: closeAllCleanSourceTabs
    },
    {
      id: 'format-document',
      label: 'Format document',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('format')
    },
    {
      id: 'rename-symbol',
      label: 'Rename symbol',
      detail: 'F2',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('rename')
    },
    {
      id: 'quick-fix',
      label: 'Quick fix',
      detail: 'Alt+Enter',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('quick-fix')
    },
    {
      id: 'revert-file',
      label: 'Revert file',
      detail: preview?.fileName ?? 'No file',
      disabled: !selectedSourceDirty || fileActionBusy === 'save',
      perform: revertSelectedSourceFile
    },
    {
      id: 'copy-path',
      label: 'Copy file path',
      detail: preview?.relativePath ?? 'No file',
      disabled: !preview || fileActionBusy === 'copy',
      perform: copySelectedPath
    },
    {
      id: 'source-copy-context-brief',
      label: 'Copy source context brief',
      detail: selectedRecord?.relativePath ?? selectedProject.name,
      disabled: !selectedProject.path,
      perform: copySourceContextBrief
    },
    {
      id: 'open-file-native',
      label: 'Open file in IDE',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || fileActionBusy === 'open',
      perform: openSelectedFile
    },
    {
      id: 'reveal-file',
      label: 'Reveal file in Finder',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || fileActionBusy === 'reveal',
      perform: revealSelectedFile
    },
    {
      id: 'go-definition',
      label: 'Go to definition',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading,
      perform: () => requestSourceIntelligenceAction('definition')
    },
    {
      id: 'find-references',
      label: 'Find references',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading,
      perform: () => requestSourceIntelligenceAction('references')
    },
    {
      id: 'find-implementations',
      label: 'Find implementations',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('implementation')
    },
    {
      id: 'go-type-definition',
      label: 'Go to type definition',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('type-definition')
    },
    {
      id: 'show-hover',
      label: 'Show hover',
      detail: preview?.language ?? 'No language',
      disabled: !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('hover')
    },
    {
      id: 'lsp-retry-status',
      label: 'Retry language server status',
      detail: sourceLspStatusLabel(),
      disabled: !preview || !sourceIntelligenceAvailable || sourceLspStatusLoading,
      perform: () => loadSourceLspStatus(preview, selectedProject)
    },
    {
      id: 'lsp-copy-status',
      label: 'Copy language server status',
      detail: sourceLspStatusLabel(),
      disabled: !preview,
      perform: copySourceLspStatusReport
    },
    {
      id: 'lsp-copy-install',
      label: 'Copy language server install command',
      detail: sourceLspInstallCommand() || 'No install command',
      disabled: !sourceLspInstallCommand(),
      perform: copySourceLspInstallCommand
    },
    {
      id: 'insights-toggle',
      label: editorInsightCollapsed ? 'Show editor insights' : 'Hide editor insights',
      detail: 'Problems, symbols, Git',
      perform: toggleEditorInsightCollapsed
    },
    {
      id: 'insights-problems',
      label: 'Show problems',
      detail: sourceDiagnosticSummary,
      perform: () => showEditorInsightPanel('problems')
    },
    {
      id: 'next-problem',
      label: 'Go to next problem',
      detail: 'F8',
      disabled: sourceDiagnostics.length === 0,
      perform: () => selectNextSourceDiagnostic()
    },
    {
      id: 'previous-problem',
      label: 'Go to previous problem',
      detail: 'Shift+F8',
      disabled: sourceDiagnostics.length === 0,
      perform: () => selectPreviousSourceDiagnostic()
    },
    {
      id: 'insights-symbols',
      label: 'Show symbols',
      detail: `${sourceSymbols.length} symbols`,
      perform: () => showEditorInsightPanel('symbols')
    },
    {
      id: 'insights-git',
      label: 'Show selected-file Git',
      detail: selectedSourceGitSummary,
      perform: () => showEditorInsightPanel('git')
    },
    ...sourceSymbols.slice(0, 12).map((symbol) => ({
      id: `symbol-${symbol.kind}-${symbol.name}-${symbol.line}-${symbol.column}`,
      label: `Go to symbol: ${symbol.name}`,
      detail: `${symbol.kind} - line ${symbol.line}`,
      perform: () => selectSourceSymbol(symbol)
    })),
    ...sourceDiagnostics.slice(0, 8).map((diagnostic, index) => ({
      id: `diagnostic-${diagnostic.severity}-${diagnostic.line}-${diagnostic.column}-${index}`,
      label: `Go to problem: ${diagnostic.message}`,
      detail: `${diagnostic.severity} - ${diagnostic.line}:${diagnostic.column}`,
      perform: () => selectSourceDiagnostic(diagnostic)
    })),
    ...sourceLayoutPresets.map((preset) => ({
      id: `layout-${preset.id}`,
      label: `Use ${preset.label} layout`,
      detail: preset.title,
      perform: () => applySourceLayoutPreset(preset.id)
    })),
    {
      id: 'layout-reset-dock',
      label: 'Reset dock layout',
      detail: 'Code layout',
      perform: resetSourceDockLayout
    },
    ...sourceLayoutPresets.map((preset) => ({
      id: `layout-save-${preset.id}`,
      label: `Save current layout as ${preset.label}`,
      detail: 'Custom preset override',
      perform: () => saveSourceLayoutPresetOverride(preset.id)
    })),
    ...sourceLayoutPresets.map((preset) => ({
      id: `layout-reset-${preset.id}`,
      label: `Reset saved ${preset.label} layout`,
      detail: sourceLayoutPresetOverrides[preset.id] ? 'Custom override saved' : 'Default preset',
      disabled: !sourceLayoutPresetOverrides[preset.id],
      perform: () => resetSourceLayoutPresetOverride(preset.id)
    })),
    ...managedDockPanelIDs.flatMap((panelID) =>
      dockPanelMoveTargets(panelID).map((groupID) => ({
        id: `dock-move-${panelID}-${groupID}`,
        label: `Move ${dockPanelLabel(panelID)} to ${dockGroupLabel(groupID, panelID)}`,
        detail: dockPanelPlacementSummary(panelID),
        disabled: dockGroupIDForPanel(sourceDockLayout, panelID) === groupID,
        perform: () => moveDockPanelToManagedGroup(panelID, groupID)
      }))
    ),
    ...hideableDockPanelIDs.map((panelID) => ({
      id: `dock-toggle-${panelID}`,
      label: `${sourceDockPanelVisible(panelID) ? 'Hide' : 'Show'} ${dockPanelLabel(panelID)} panel`,
      detail: dockPanelPlacementSummary(panelID),
      perform: () => toggleDockPanelVisibility(panelID)
    })),
    {
      id: 'side-left',
      label: 'Move explorer left',
      detail: 'Side pane',
      disabled: sidePanePosition === 'left',
      perform: () => selectSidePanePosition('left')
    },
    {
      id: 'side-right',
      label: 'Move explorer right',
      detail: 'Side pane',
      disabled: sidePanePosition === 'right',
      perform: () => selectSidePanePosition('right')
    },
    {
      id: 'context-side',
      label: 'Move context to side',
      detail: 'Context cards',
      disabled: contextPanelPlacement === 'side',
      perform: () => selectContextPanelPlacement('side')
    },
    {
      id: 'context-bottom',
      label: 'Move context to bottom',
      detail: 'Context cards',
      disabled: contextPanelPlacement === 'bottom',
      perform: () => moveDockPanelToGroup('context', 'bottom')
    },
    {
      id: 'context-top',
      label: 'Move context to top',
      detail: 'Context cards',
      disabled: contextPanelPlacement === 'top',
      perform: () => selectContextPanelPlacement('top')
    },
    {
      id: 'context-grid',
      label: 'Use context grid',
      detail: 'Context cards',
      disabled: contextPanelMode === 'grid',
      perform: () => selectContextPanelMode('grid')
    },
    {
      id: 'context-stack',
      label: 'Use context stack',
      detail: 'Context cards',
      disabled: contextPanelMode === 'stack',
      perform: () => selectContextPanelMode('stack')
    },
    {
      id: 'context-toggle',
      label: contextPanelCollapsed ? 'Show context cards' : 'Hide context cards',
      detail: 'Canvas',
      perform: toggleContextPanelCollapsed
    },
    {
      id: 'context-restore',
      label: 'Show hidden context cards',
      detail: `${hiddenContextCardIDs.size} hidden`,
      disabled: hiddenContextCardIDs.size === 0,
      perform: showAllContextCards
    },
    {
      id: 'dock-show-terminal',
      label: 'Show terminal dock',
      detail: terminalDockSummary(),
      disabled: sourceDockPanelVisible('terminal'),
      perform: () => showDockPanel('terminal')
    },
    {
      id: 'dock-hide-terminal',
      label: 'Hide terminal dock',
      detail: terminalDockSummary(),
      disabled: !sourceDockPanelVisible('terminal'),
      perform: () => hideDockPanel('terminal')
    },
    {
      id: 'dock-show-browser',
      label: 'Show browser dock',
      detail: activeBrowserUrl || 'No runtime URL',
      disabled: sourceDockPanelVisible('browser'),
      perform: () => openBrowserDock()
    },
    {
      id: 'dock-hide-browser',
      label: 'Hide browser dock',
      detail: activeBrowserUrl || 'No runtime URL',
      disabled: !sourceDockPanelVisible('browser'),
      perform: () => hideDockPanel('browser')
    },
    {
      id: 'browser-open-runtime',
      label: 'Open active runtime in browser dock',
      detail: defaultBrowserUrl || 'No active runtime',
      disabled: !defaultBrowserUrl,
      perform: () => openBrowserDock(defaultBrowserUrl)
    },
    {
      id: 'browser-reload',
      label: 'Reload browser dock',
      detail: activeBrowserUrl || 'No browser URL',
      disabled: !activeBrowserUrl,
      perform: reloadBrowserFrame
    },
    {
      id: 'browser-open-external',
      label: 'Open browser URL externally',
      detail: activeBrowserUrl || 'No browser URL',
      disabled: !activeBrowserUrl,
      perform: openBrowserUrlExternal
    },
    {
      id: 'terminal-start-embedded',
      label: 'Start embedded terminal',
      detail: selectedProject.path,
      disabled: !selectedProject.path || embeddedTerminalStarting,
      perform: () => startEmbeddedTerminalSession(selectedProject.path)
    },
    {
      id: 'terminal-stop-embedded',
      label: 'Stop embedded terminal',
      detail: embeddedTerminalStatusLabel(),
      disabled: !embeddedTerminalSession,
      perform: closeEmbeddedTerminalSession
    },
    {
      id: 'terminal-fit-embedded',
      label: 'Fit embedded terminal',
      detail: embeddedTerminalStatusLabel(),
      perform: fitEmbeddedTerminal
    },
    {
      id: 'terminal-refresh-embedded',
      label: 'Refresh embedded terminals',
      detail: `${embeddedTerminalSessions.length} live`,
      disabled: embeddedTerminalSessionsLoading,
      perform: loadEmbeddedTerminalSessions
    },
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `terminal-embedded-resume-${session.provider}-${session.id}`,
      label: `Resume in embedded terminal: ${session.title}`,
      detail: agentSessionProjectLabel(session),
      disabled: !agentSessionTerminalCommand(session).trim(),
      perform: () => resumeAgentSessionEmbeddedTerminal(session)
    })),
    ...embeddedTerminalSessions.slice(0, 8).map((session) => ({
      id: `terminal-attach-${session.sessionId}`,
      label: `Attach terminal: ${embeddedTerminalSessionTitle(session)}`,
      detail: session.cwd,
      disabled: embeddedTerminalSession?.sessionId === session.sessionId,
      perform: () => attachEmbeddedTerminalSession(session)
    })),
    ...embeddedTerminalSessions.slice(0, 8).map((session) => ({
      id: `terminal-close-${session.sessionId}`,
      label: `Close terminal: ${embeddedTerminalSessionTitle(session)}`,
      detail: session.cwd,
      perform: () => closeListedEmbeddedTerminalSession(session)
    })),
    ...contextCardOrder.map((cardID) => ({
      id: `context-card-${cardID}`,
      label: `Show ${contextCardLabels[cardID]} card`,
      detail: 'Context cards',
      disabled:
        isContextCardVisible(cardID) &&
        !contextPanelCollapsed &&
        (contextPanelMode !== 'stack' || activeVisibleContextCardID() === cardID),
      perform: () => showContextCard(cardID)
    })),
    {
      id: 'activity-files',
      label: 'Show files',
      detail: sourceActivitySummary('files'),
      disabled: sourceActivityMode === 'files',
      perform: () => selectSourceActivityMode('files')
    },
    {
      id: 'activity-clipboard',
      label: 'Show clipboard cleanup',
      detail: sourceActivitySummary('clipboard'),
      disabled: sourceActivityMode === 'clipboard',
      perform: () => selectSourceActivityMode('clipboard')
    },
    {
      id: 'activity-conversations',
      label: 'Show conversations',
      detail: sourceActivitySummary('conversations'),
      disabled: sourceActivityMode === 'conversations',
      perform: () => selectSourceActivityMode('conversations')
    },
    {
      id: 'activity-runs',
      label: 'Show orchestration runs',
      detail: sourceActivitySummary('runs'),
      disabled: sourceActivityMode === 'runs',
      perform: () => selectSourceActivityMode('runs')
    },
    ...selectedProjectOrchestrationRuns.slice(0, 8).map((run) => ({
      id: `run-focus-${run.id}`,
      label: `Focus run: ${run.title}`,
      detail: orchestrationCurrentActivity(run),
      perform: () => focusOrchestrationRun(run)
    })),
    ...selectedProjectOrchestrationRuns.slice(0, 8).map((run) => ({
      id: `run-copy-summary-${run.id}`,
      label: `Copy run summary: ${run.title}`,
      detail: `${run.status} · ${run.progress}%`,
      perform: () => copyOrchestrationRunSummary(run)
    })),
    ...selectedProjectOrchestrationRuns.slice(0, 8).map((run) => ({
      id: `run-copy-handoff-${run.id}`,
      label: `Copy run handoff: ${run.title}`,
      detail: orchestrationCurrentActivity(run),
      perform: () => copyOrchestrationRunHandoff(run)
    })),
    {
      id: 'activity-sessions',
      label: 'Show active sessions',
      detail: sourceActivitySummary('sessions'),
      disabled: sourceActivityMode === 'sessions',
      perform: () => selectSourceActivityMode('sessions')
    },
    {
      id: 'activity-agents',
      label: 'Show agents',
      detail: sourceActivitySummary('agents'),
      disabled: sourceActivityMode === 'agents',
      perform: () => selectSourceActivityMode('agents')
    },
    {
      id: 'activity-worktrees',
      label: 'Show worktrees',
      detail: sourceActivitySummary('worktrees'),
      disabled: sourceActivityMode === 'worktrees',
      perform: () => selectSourceActivityMode('worktrees')
    },
    {
      id: 'worktree-cleanup-brief',
      label: 'Copy worktree cleanup brief',
      detail: projectWorktreeCleanupBrief.headline,
      disabled: projectWorktrees.length === 0,
      perform: copyProjectWorktreeCleanupBrief
    },
    {
      id: 'worktree-cleanup-script',
      label: 'Copy guarded worktree cleanup script',
      detail: projectWorktreeCleanupBrief.headline,
      disabled: projectWorktrees.length === 0,
      perform: copyProjectWorktreeCleanupScript
    },
    {
      id: 'activity-git',
      label: 'Show Git and tasks',
      detail: sourceActivitySummary('git'),
      disabled: sourceActivityMode === 'git',
      perform: () => selectSourceActivityMode('git')
    },
    {
      id: 'git-refresh-history',
      label: 'Refresh Git history',
      detail: selectedProject.name,
      disabled: gitCommitHistoryLoading,
      perform: () => loadGitCommitHistory(selectedProject)
    },
    {
      id: 'git-copy-selected-commit-detail',
      label: 'Copy selected commit detail',
      detail: selectedGitCommit ? gitCommitSummaryText(selectedGitCommit) : gitCommitHistorySummary,
      disabled: !selectedGitCommit,
      perform: copySelectedGitCommitDetail
    },
    {
      id: 'git-copy-selected-commit-handoff',
      label: 'Copy selected commit handoff',
      detail: selectedGitCommit ? gitCommitSummaryText(selectedGitCommit) : gitCommitHistorySummary,
      disabled: !selectedGitCommit,
      perform: () => {
        if (selectedGitCommit) copyGitCommitHandoff(selectedGitCommit);
      }
    },
    {
      id: 'git-copy-workspace-brief',
      label: 'Copy Git workspace brief',
      detail: `${selectedProject.name} · ${repoDashboardSummary} · ${projectWorktreeCleanupBrief.headline}`,
      disabled: !selectedProject.path,
      perform: copyGitWorkspaceBrief
    },
    ...selectedProjectGitTaskIDs.slice(0, 8).map((taskID) => ({
      id: `git-copy-task-${taskID}`,
      label: `Copy task link: ${taskID}`,
      detail: gitTaskUrl(taskID) ?? 'Task ID only',
      perform: () => copyGitTaskReference(taskID)
    })),
    ...gitCommitHistory.slice(0, 8).map((entry) => ({
      id: `git-copy-commit-${entry.sha}`,
      label: `Copy commit: ${entry.shortSha}`,
      detail: entry.subject,
      perform: () => copyGitCommitSummary(entry)
    })),
    ...gitCommitHistory.slice(0, 8).map((entry) => ({
      id: `git-copy-commit-handoff-${entry.sha}`,
      label: `Copy commit handoff: ${entry.shortSha}`,
      detail: entry.subject,
      perform: () => copyGitCommitHandoff(entry)
    })),
    {
      id: 'activity-refresh',
      label: `Refresh ${sourceActivityPanelLabel}`,
      detail: sourceActivitySummary(sourceActivityMode),
      disabled: sourceActivityRefreshing(sourceActivityMode),
      perform: () => refreshSourceActivityMode(sourceActivityMode)
    },
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => {
      const action = projectWorktreePrimaryAction(worktree);
      return {
        id: `worktree-primary-action-${worktree.path}`,
        label: `Recommended worktree action: ${action.label} ${worktree.branch}`,
        detail: action.title,
        disabled: false,
        perform: () => runWorktreePrimaryAction(worktree)
      };
    }),
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-open-source-${worktree.path}`,
      label: `Open worktree in source browser: ${worktree.branch}`,
      detail: worktree.path,
      disabled: !worktree.path.trim(),
      perform: () => openWorktreeInSourceBrowser(worktree)
    })),
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-cleanup-plan-${worktree.path}`,
      label: `Copy worktree cleanup plan: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).recommendation,
      disabled: false,
      perform: () => copyWorktreeCleanupPlan(worktree)
    })),
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-audit-command-${worktree.path}`,
      label: `Copy worktree audit command: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).reason,
      disabled: false,
      perform: () => copyWorktreeAuditCommand(worktree)
    })),
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-remove-command-${worktree.path}`,
      label: `Copy worktree remove command: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).cleanupCommand,
      disabled: projectWorktreeSafety(worktree).kind === 'protected',
      perform: () => copyWorktreeCleanupCommand(worktree)
    })),
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-backup-command-${worktree.path}`,
      label: `Copy worktree backup command: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).reason,
      disabled: projectWorktreeSafety(worktree).kind === 'protected',
      perform: () => copyWorktreeBackupCommand(worktree)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-resume-${session.provider}-${session.id}`,
      label: `Resume ${session.provider}: ${session.title}`,
      detail: agentSessionProjectLabel(session),
      disabled: !agentSessionTerminalCommand(session).trim(),
      perform: () => openAgentSessionTerminal(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `conversation-save-session-workspace-${session.provider}-${session.id}`,
      label: `Save workspace snapshot: ${session.title}`,
      detail: agentSessionProjectLabel(session),
      disabled: !agentSessionProjectPath(session).trim(),
      perform: () => captureAgentSessionWorkspaceSnapshot(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `conversation-restore-session-workspace-${session.provider}-${session.id}`,
      label: `Restore workspace snapshot: ${session.title}`,
      detail: workspaceSnapshotForAgentSession(session)?.project.path ?? 'No saved workspace',
      disabled: !workspaceSnapshotForAgentSession(session),
      perform: () => restoreAgentSessionWorkspaceSnapshot(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `conversation-copy-session-restore-plan-${session.provider}-${session.id}`,
      label: `Copy workspace restore plan: ${session.title}`,
      detail: workspaceSnapshotForAgentSession(session)?.project.path ?? 'No saved workspace',
      disabled: !workspaceSnapshotForAgentSession(session),
      perform: () => copyAgentSessionWorkspaceRestorePlan(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-copy-plan-${session.provider}-${session.id}`,
      label: `Copy session resume plan: ${session.title}`,
      detail: agentSessionProjectLabel(session),
      disabled: !agentSessionTerminalCommand(session).trim(),
      perform: () => copyAgentSessionResumePlan(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-copy-shell-command-${session.provider}-${session.id}`,
      label: `Copy shell resume command: ${session.title}`,
      detail: agentSessionResumeShellCommand(session),
      disabled: !agentSessionTerminalCommand(session).trim(),
      perform: () => copyAgentSessionResumeShellCommand(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-copy-resume-command-${session.provider}-${session.id}`,
      label: `Copy resume command: ${session.title}`,
      detail: agentSessionResumeCommand(session),
      disabled: !agentSessionResumeCommand(session).trim(),
      perform: () => copyActivityCommand(agentSessionResumeCommand(session), 'Resume command copied')
    })),
    {
      id: 'git-stage-file',
      label: 'Stage selected file',
      detail: selectedRecord?.relativePath ?? 'No file',
      disabled: selectedGitPathActionDisabled || !selectedRecord,
      perform: () => {
        if (selectedRecord) void runGitPathAction('stage', [selectedRecord.relativePath]);
      }
    },
    {
      id: 'git-unstage-file',
      label: 'Unstage selected file',
      detail: selectedRecord?.relativePath ?? 'No file',
      disabled: selectedGitUnstageDisabled || !selectedRecord,
      perform: () => {
        if (selectedRecord) void runGitPathAction('unstage', [selectedRecord.relativePath]);
      }
    },
    {
      id: 'git-fetch',
      label: 'Fetch repository',
      detail: selectedProject.name,
      disabled: gitRemoteActionDisabled,
      perform: () => runGitRemoteAction('fetch')
    },
    {
      id: 'git-pull',
      label: 'Pull repository',
      detail: selectedProject.name,
      disabled: gitRemoteActionDisabled,
      perform: () => runGitRemoteAction('pull')
    },
    {
      id: 'git-push',
      label: 'Push repository',
      detail: selectedProject.name,
      disabled: gitRemoteActionDisabled,
      perform: () => runGitRemoteAction('push')
    }
  ]);
  let commandPaletteResults = $derived(
    sourceCommandPaletteItems.filter((item) =>
      commandPaletteTextMatches(commandPaletteQuery, item.label, item.detail)
    ).slice(0, 12)
  );

  $effect(() => {
    if (!quickOpenVisible) return;
    const lastResultIndex = Math.max(0, quickOpenActiveResultCount - 1);
    if (quickOpenIndex > lastResultIndex) {
      quickOpenIndex = lastResultIndex;
    }
  });

  $effect(() => {
    const query = quickOpenWorkspaceSymbolQuery;
    if (!quickOpenVisible || !quickOpenWorkspaceSymbolMode) {
      workspaceSymbolResults = [];
      workspaceSymbolError = '';
      workspaceSymbolLoading = false;
      return;
    }

    if (!query || !preview || !sourceIntelligenceAvailable) {
      workspaceSymbolResults = [];
      workspaceSymbolError = '';
      workspaceSymbolLoading = false;
      return;
    }

    const timer = window.setTimeout(() => {
      void loadSourceLspWorkspaceSymbols(query);
    }, 180);

    return () => window.clearTimeout(timer);
  });

  $effect(() => {
    if (!commandPaletteVisible) return;
    const lastResultIndex = Math.max(0, commandPaletteResults.length - 1);
    if (commandPaletteIndex > lastResultIndex) {
      commandPaletteIndex = lastResultIndex;
    }
  });

  $effect(() => {
    const element = fileTreeElement;
    if (!element) return;

    measureFileTreeViewport();

    if (typeof ResizeObserver === 'undefined') return;

    let resizeFrame = 0;
    const resizeObserver = new ResizeObserver(() => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = 0;
        measureFileTreeViewport();
      });
    });
    resizeObserver.observe(element);
    return () => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeObserver.disconnect();
    };
  });

  $effect(() => {
    const revealPath = pendingTreeRevealPath;
    const element = fileTreeElement;
    if (!revealPath || !element) return;

    const rowIndex = visibleTreeRows.findIndex((row) => row.node.file?.path === revealPath);
    if (rowIndex < 0) return;

    const nextScrollTop = scrollTopForSourceTreeReveal(
      rowIndex,
      element.scrollTop,
      element.clientHeight || sourceTreeFallbackViewportHeight,
      sourceTreeRowHeight
    );
    pendingTreeRevealPath = null;

    if (Math.abs(element.scrollTop - nextScrollTop) > 0.5) {
      element.scrollTop = nextScrollTop;
    }
    fileTreeScrollTop = element.scrollTop;
  });

  $effect(() => {
    const rowIndex = pendingTreeFocusRowIndex;
    const element = fileTreeElement;
    if (rowIndex === null || !element) return;

    if (rowIndex < 0 || rowIndex >= visibleTreeRows.length) {
      pendingTreeFocusRowIndex = null;
      return;
    }

    const button = element.querySelector<HTMLButtonElement>(
      `button[data-tree-row-index="${rowIndex}"]`
    );
    if (!button) return;

    pendingTreeFocusRowIndex = null;
    button.focus();
  });

  $effect(() => {
    const element = embeddedTerminalElement;
    if (!sourceDockPanelVisible('terminal') || !element) return;

    void ensureEmbeddedTerminalRenderer().then(() => {
      window.setTimeout(fitEmbeddedTerminal, 0);
    });

    if (typeof ResizeObserver === 'undefined') return;

    let resizeFrame = 0;
    const resizeObserver = new ResizeObserver(() => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = 0;
        fitEmbeddedTerminal();
      });
    });
    resizeObserver.observe(element);
    return () => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeObserver.disconnect();
    };
  });

  async function scanProject(
    project: ProjectRoot,
    preferredPath = selectedSourcePaths[project.id] ?? selectedRecord?.path ?? null,
    options: SourceScanOptions = {}
  ) {
    const generation = ++scanGeneration;
    const scanLimit = options.limit ?? expandedSourceScanLimit;
    const cachedScan = options.force
      ? null
      : getSourceScanCacheEntry(
          sourceScanCache,
          project,
          scanLimit,
          Date.now(),
          sourceScanCacheMaxAgeMs
        );

    if (cachedScan) {
      activeSourceScanId = '';
      sourceScanProgress = null;
      scanning = false;
      loading = true;
      error = '';
      runtime = 'cached source scan';
      clearBackgroundIndexError(project.id);

      const nextSelection = applySourceRecords(
        cachedScan.records,
        preferredPath,
        'cached source scan',
        cachedScan.truncated,
        cachedScan.stats ?? null
      );
      if (nextSelection) {
        await loadRecord(nextSelection, generation);
      } else {
        loading = false;
      }
      return;
    }

    scanning = true;
    loading = true;
    const scanId = createSourceScanId();
    activeSourceScanId = scanId;
    sourceScanProgress = null;
    sourceScanStats = null;
    error = '';
    runtime = 'scanning source files';
    clearSourceRecordsForIncomingProject(project, Boolean(options.force));

    try {
      const tauriScan = await listSourceFilesFromTauri(project.path, '', scanLimit, scanId);
      if (generation !== scanGeneration) return;
      if (!tauriScan) {
        throw new Error('Local source scanner unavailable. Run inside Tauri or use pnpm dev.');
      }

      const nextRecords = tauriScan.records;
      const suspiciousScanResult = isSuspiciousSourceScanResult(
        nextRecords.length,
        tauriScan.truncated,
        scanLimit,
        suspiciousSourceIndexFileThreshold
      );
      if (
        !options.skipTinyIndexRepair &&
        shouldRepairSuspiciousSourceScan(
          nextRecords.length,
          tauriScan.truncated,
          scanLimit,
          suspiciousSourceIndexFileThreshold
        )
      ) {
        sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
        fileActionStatus = `Only ${nextRecords.length.toLocaleString()} files indexed for ${project.name}. Rebuilding the project index.`;
        await scanProject(project, preferredPath, {
          force: true,
          limit: Math.max(scanLimit, expandedSourceScanLimit),
          skipTinyIndexRepair: true
        });
        return;
      }

      sourceScanCache = upsertSourceScanCacheEntry(
        sourceScanCache,
        project,
        nextRecords,
        tauriScan.limit,
        Date.now(),
        maxSourceScanCacheEntries,
        tauriScan.truncated,
        tauriScan.stats
      );
      clearBackgroundIndexError(project.id);

      if (options.skipTinyIndexRepair && suspiciousScanResult) {
        fileActionStatus = `Only ${nextRecords.length.toLocaleString()} files indexed for ${project.name}. Check the project root or reset the index.`;
      }

      const nextSelection = applySourceRecords(
        nextRecords,
        preferredPath,
        options.skipTinyIndexRepair && suspiciousScanResult ? 'tiny source scan' : 'local source scan',
        tauriScan.truncated,
        tauriScan.stats ?? null
      );

      if (nextSelection) {
        await loadRecord(nextSelection, generation);
      } else {
        loading = false;
      }
    } catch (scanError) {
      if (generation !== scanGeneration) return;
      records = [];
      selectedRecord = null;
      selectedSourceLine = null;
      scanLimitReached = false;
      sourceScanStats = null;
      preview = null;
      expandedFolderIds = new Set();
      syncSourcePreviewContent(null);
      runtime = 'source scan unavailable';
      error = scanError instanceof Error ? scanError.message : 'Could not scan source files';
      loading = false;
    } finally {
      if (generation === scanGeneration) {
        scanning = false;
        activeSourceScanId = '';
        sourceScanProgress = null;
      }
    }
  }

  function cancelSourceScan() {
    if (!scanning) return;

    if (activeSourceScanId) {
      void cancelSourceScanFromTauri(activeSourceScanId).catch(() => {
        // Stop is optimistic; stale scan results are already ignored by generation.
      });
    }
    scanGeneration += 1;
    scanning = false;
    loading = false;
    activeSourceScanId = '';
    sourceScanProgress = null;
    sourceScanStats = null;
    runtime = 'source scan stopped';
    error = '';
    fileActionStatus = 'Scan stopped';
  }

  function clearSourceRecordsForIncomingProject(project: ProjectRoot, force = false) {
    const hasCurrentSourceState = records.length > 0 || selectedRecord !== null || preview !== null;
    const hasDifferentSelectedRecord =
      selectedRecord !== null && !sourceRecordBelongsToProject(selectedRecord, project);
    const hasDifferentRecords =
      selectedRecord === null &&
      records.length > 0 &&
      records.some((record) => !sourceRecordBelongsToProject(record, project));

    if (!force && !hasDifferentSelectedRecord && !hasDifferentRecords) return;
    if (force && !hasCurrentSourceState && !hasDifferentSelectedRecord && !hasDifferentRecords) return;

    records = [];
    selectedRecord = null;
    selectedSourceLine = null;
    sourceNavigationBackStack = [];
    sourceNavigationForwardStack = [];
    scanLimitReached = false;
    sourceScanStats = null;
    preview = null;
    expandedFolderIds = new Set();
    syncSourcePreviewContent(null);
  }

  function sourceRecordBelongsToProject(record: SourceRecord, project: ProjectRoot) {
    const projectPath = normalizeProjectPath(project.path);
    const recordPath = normalizeProjectPath(record.path);
    return recordPath === projectPath || recordPath.startsWith(`${projectPath}/`);
  }

  function dirtySourceRecordsForProject(
    openTabs: SourceOpenTab[],
    workspaceEditRecordsByPath: Record<string, SourceRecord>,
    project: ProjectRoot
  ) {
    const recordsByPath = new Map<string, SourceRecord>();
    for (const tab of openTabs) {
      recordsByPath.set(tab.path, tab);
    }
    for (const record of Object.values(workspaceEditRecordsByPath)) {
      if (sourceRecordBelongsToProject(record, project)) {
        recordsByPath.set(record.path, record);
      }
    }

    return Array.from(recordsByPath.values()).filter((record) => isSourcePathDirty(record.path));
  }

  function resetProjectScanCache(project: ProjectRoot = selectedProject, limit = expandedSourceScanLimit) {
    sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
    fileActionStatus = `Index reset for ${project.name}`;
    return scanProject(project, selectedSourcePaths[project.id], { force: true, limit });
  }

  function sourceOnboardingScanStatus(project: ProjectRoot) {
    return `Scanning ${project.name} up to ${expandedSourceScanLimit.toLocaleString()} source files`;
  }

  function formatSourceScanHealthNote(totalCount: number, needsAttention: boolean) {
    if (!needsAttention) return '';
    const fileLabel = totalCount === 1 ? 'file' : 'files';
    return `Only ${totalCount.toLocaleString()} ${fileLabel} indexed. If that looks wrong, reset and scan up to ${expandedSourceScanLimit.toLocaleString()} files.`;
  }

  async function indexProjectsInBackground(projects: ProjectRoot[]) {
    const projectsToIndex = selectBackgroundIndexProjects(
      projects,
      selectedProject.id,
      sourceScanCache,
      Date.now(),
      sourceScanCacheMaxAgeMs,
      expandedSourceScanLimit
    );

    for (const project of projectsToIndex) {
      void indexProjectInBackground(project);
    }
  }

  async function indexProjectInBackground(project: ProjectRoot) {
    if (project.id === selectedProject.id || backgroundIndexingProjectIDs.has(project.id)) return;

    setBackgroundProjectIndexing(project.id, true);
    clearBackgroundIndexError(project.id);

    try {
      const tauriScan = await listSourceFilesFromTauri(
        project.path,
        '',
        expandedSourceScanLimit,
        createSourceScanId()
      );
      if (!tauriScan) {
        throw new Error('Local source scanner unavailable');
      }

      const nextRecords = tauriScan.records;
      sourceScanCache = upsertSourceScanCacheEntry(
        sourceScanCache,
        project,
        nextRecords,
        tauriScan.limit,
        Date.now(),
        maxSourceScanCacheEntries,
        tauriScan.truncated,
        tauriScan.stats
      );
      clearBackgroundIndexError(project.id);
    } catch (indexError) {
      backgroundIndexErrorByProject = {
        ...backgroundIndexErrorByProject,
        [project.id]: indexError instanceof Error ? indexError.message : 'Could not index project'
      };
    } finally {
      setBackgroundProjectIndexing(project.id, false);
    }
  }

  async function loadProjectGitStatus(project: ProjectRoot) {
    const projectID = project.id;
    projectGitLoading = true;
    projectGitStatus = null;
    projectGitError = '';

    try {
      const nextStatus = await readProjectGitStatusFromTauri(project.path);
      if (selectedProjectID !== projectID) return;

      projectGitStatus = nextStatus;
      projectGitError = nextStatus ? '' : 'Native Git unavailable';
    } catch (gitError) {
      if (selectedProjectID !== projectID) return;

      projectGitStatus = null;
      projectGitError = gitError instanceof Error ? gitError.message : 'Could not read Git status';
    } finally {
      if (selectedProjectID === projectID) {
        projectGitLoading = false;
      }
    }
  }

  async function loadRuntimeContexts(projects: ProjectRoot[] = projectOptions) {
    runtimeContextsLoading = true;
    runtimeContextError = '';

    try {
      const nativeContexts = await listRuntimeContextsFromTauri(projects);
      if (nativeContexts) {
        runtimeContexts = nativeContexts;
        runtimeContextSource = 'native process scan';
        return;
      }

      runtimeContexts = projects.flatMap((project) => demoRuntimeContextsForProject(project));
      runtimeContextSource = 'browser preview';
    } catch (contextError) {
      runtimeContexts = projects.flatMap((project) => demoRuntimeContextsForProject(project));
      runtimeContextSource = 'browser preview';
      runtimeContextError =
        contextError instanceof Error ? contextError.message : 'Could not read runtime contexts';
    } finally {
      runtimeContextsLoading = false;
    }
  }

  async function loadProjectWorktrees(project: ProjectRoot = selectedProject) {
    projectWorktreesLoading = true;
    projectWorktreeError = '';

    try {
      const nativeWorktrees = await listProjectWorktreesFromTauri(project.path);
      if (nativeWorktrees) {
        projectWorktrees = nativeWorktrees;
        projectWorktreeSource = 'native git scan';
        return;
      }

      projectWorktrees = demoProjectWorktreesForProject(project);
      projectWorktreeSource = 'browser preview';
    } catch (worktreeError) {
      projectWorktrees = demoProjectWorktreesForProject(project);
      projectWorktreeSource = 'browser preview';
      projectWorktreeError =
        worktreeError instanceof Error ? worktreeError.message : 'Could not scan worktrees';
    } finally {
      projectWorktreesLoading = false;
    }
  }

  async function loadGitRepositorySummaries(projects: ProjectRoot[] = projectOptions) {
    gitRepositorySummariesLoading = true;
    gitRepositorySummaryError = '';

    try {
      const nativeSummaries = await listGitRepositorySummariesFromTauri(projects);
      if (nativeSummaries) {
        gitRepositorySummaries = nativeSummaries;
        gitRepositorySummarySource = 'native git dashboard';
        return;
      }

      gitRepositorySummaries = demoGitRepositorySummariesForProjects(projects);
      gitRepositorySummarySource = 'browser preview';
    } catch (summaryError) {
      gitRepositorySummaries = demoGitRepositorySummariesForProjects(projects);
      gitRepositorySummarySource = 'browser preview';
      gitRepositorySummaryError =
        summaryError instanceof Error ? summaryError.message : 'Could not scan repositories';
    } finally {
      gitRepositorySummariesLoading = false;
    }
  }

  async function loadGitCommitHistory(project: ProjectRoot = selectedProject) {
    const projectID = project.id;
    gitCommitHistoryLoading = true;
    gitCommitHistoryError = '';

    try {
      const nativeHistory = await readGitCommitHistoryFromTauri(project.path, maxGitCommitHistoryEntries);
      if (selectedProjectID !== projectID) return;

      if (nativeHistory) {
        setGitCommitHistory(nativeHistory, 'native git log');
        return;
      }

      setGitCommitHistory(demoGitCommitHistoryForProject(project), 'browser preview');
    } catch (historyError) {
      if (selectedProjectID !== projectID) return;

      setGitCommitHistory(demoGitCommitHistoryForProject(project), 'browser preview');
      gitCommitHistoryError =
        historyError instanceof Error ? historyError.message : 'Could not read Git history';
    } finally {
      if (selectedProjectID === projectID) {
        gitCommitHistoryLoading = false;
      }
    }
  }

  function setGitCommitHistory(entries: GitCommitHistoryEntry[], source: string) {
    gitCommitHistory = entries;
    gitCommitHistorySource = source;

    if (!entries.some((entry) => entry.sha === selectedGitCommitSha)) {
      selectedGitCommitSha = entries[0]?.sha ?? '';
    }
  }

  async function loadOrchestrationRuns(projects: ProjectRoot[] = projectOptions) {
    orchestrationRunsLoading = true;
    orchestrationRunError = '';

    try {
      const nativeRuns = await listOrchestrationRunsFromTauri(projects);
      if (nativeRuns) {
        orchestrationRuns = nativeRuns;
        orchestrationRunSource = 'native event store';
        return;
      }

      orchestrationRuns = demoOrchestrationRunsForProjects(projects);
      orchestrationRunSource = 'browser preview';
    } catch (runError) {
      orchestrationRuns = demoOrchestrationRunsForProjects(projects);
      orchestrationRunSource = 'browser preview';
      orchestrationRunError =
        runError instanceof Error ? runError.message : 'Could not read orchestration runs';
    } finally {
      orchestrationRunsLoading = false;
    }
  }

  async function loadAgentSessions() {
    agentSessionsLoading = true;
    agentSessionError = '';

    try {
      const nativeSessions = await listAgentSessionsFromTauri();
      if (nativeSessions) {
        agentSessions = nativeSessions;
        agentSessionSource = 'native session scan';
        return;
      }

      agentSessions = demoAgentSessionsForProject(selectedProject);
      agentSessionSource = 'browser preview';
    } catch (sessionError) {
      agentSessions = demoAgentSessionsForProject(selectedProject);
      agentSessionSource = 'browser preview';
      agentSessionError =
        sessionError instanceof Error ? sessionError.message : 'Could not scan agent sessions';
    } finally {
      agentSessionsLoading = false;
    }
  }

  function demoRuntimeContextsForProject(project: ProjectRoot): RuntimeContext[] {
    return [
      {
        pid: 0,
        command: 'vite',
        port: 5177,
        cwd: project.path,
        projectID: project.id,
        projectName: project.name,
        rootLabel: formatSourceContextRootLabel(project.path)
      }
    ];
  }

  function demoProjectWorktreesForProject(project: ProjectRoot): ProjectWorktree[] {
    return [
      {
        repo: project.name,
        path: project.path,
        branch: 'main',
        taskID: project.id === 'mac-command-bar' ? 'TSK-127' : null,
        isDirty: false,
        hasUnmergedCommits: false,
        lastActivity: null,
        deleteEligibility: 'requires-confirmation'
      }
    ];
  }

  function demoGitRepositorySummariesForProjects(projects: ProjectRoot[]): GitRepositorySummary[] {
    return projects.map((project) => {
      const isMacCommandBar = project.id === 'mac-command-bar';
      return {
        projectID: project.id,
        projectName: project.name,
        repo: project.name,
        path: project.path,
        rootLabel: formatSourceContextRootLabel(project.path),
        branch: 'main',
        taskID: isMacCommandBar ? 'TSK-127' : null,
        isWorktree: false,
        isDirty: false,
        stagedCount: 0,
        unstagedCount: 0,
        untrackedCount: 0,
        dirtyCount: 0,
        ahead: 0,
        behind: 0,
        lastCommitSha: isMacCommandBar ? 'b022003' : null,
        lastCommitSubject: isMacCommandBar ? 'feat: add TSK-127 git history panel' : null,
        lastCommitAt: isMacCommandBar ? new Date().toISOString() : null,
        dirtySinceEpochMs: null,
        error: null
      };
    });
  }

  function demoGitCommitHistoryForProject(project: ProjectRoot): GitCommitHistoryEntry[] {
    const committedAt = new Date().toISOString();
    const repoSlug = project.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

    return [
      {
        shortSha: '3d14498',
        sha: '3d14498f0d8d54d4b89ad51b4e7ed6d3f12d0001',
        subject: 'feat: add TSK-127 git remote controls',
        author: 'MacCommandBar',
        committedAt,
        refs: 'HEAD -> main',
        parentShas: ['ebca14e7594f84f6be0d18f2f0d39c48a2d0002'],
        parentCount: 1,
        taskID: 'TSK-127',
        taskSource: 'subject'
      },
      {
        shortSha: 'ebca14e',
        sha: 'ebca14e7594f84f6be0d18f2f0d39c48a2d0002',
        subject: 'feat: add TSK-127 git action controls',
        author: 'MacCommandBar',
        committedAt,
        refs: repoSlug ? `origin/${repoSlug}` : 'origin/main',
        parentShas: ['27a95ed5b28f23d345ccac9a8c6e63f99ad0003'],
        parentCount: 1,
        taskID: 'TSK-127',
        taskSource: 'refs'
      },
      {
        shortSha: '27a95ed',
        sha: '27a95ed5b28f23d345ccac9a8c6e63f99ad0003',
        subject: 'feat: add TSK-127 repo dashboard',
        author: 'MacCommandBar',
        committedAt,
        refs: '',
        parentShas: [],
        parentCount: 0,
        taskID: 'TSK-127',
        taskSource: 'subject'
      }
    ];
  }

  function demoOrchestrationRunsForProjects(projects: ProjectRoot[]): OrchestrationRun[] {
    const now = new Date().toISOString();

    return projects.map((project) => {
      const taskID = project.id === 'mac-command-bar' ? 'TSK-127' : null;
      const runID = taskID ? `run-${taskID.toLowerCase()}` : `run-${project.id}`;
      const title = taskID ? `${taskID} orchestration model` : `${project.name} project monitor`;
      const rootLabel = formatSourceContextRootLabel(project.path);
      const events = [
        demoOrchestrationEvent({
          runID,
          project,
          rootLabel,
          taskID,
          suffix: 'scenario',
          kind: 'scenario.executed',
          status: 'succeeded',
          title: 'Scenario executed',
          message: 'UI scenario pass produced fix candidates',
          agentID: 'controller',
          agentProvider: 'codex',
          agentRole: 'orchestrator',
          stepID: 'event-schema',
          stepKind: 'scenario',
          timestamp: now
        }),
        demoOrchestrationEvent({
          runID,
          project,
          rootLabel,
          taskID,
          suffix: 'retry',
          kind: 'test.retry',
          status: 'running',
          title: 'Retesting changed files',
          message: 'Retrying after layout and Git context fixes',
          agentID: 'fixer-1',
          agentProvider: 'codex',
          agentRole: 'fix-agent',
          stepID: 'event-store',
          stepKind: 'test',
          timestamp: now
        }),
        demoOrchestrationEvent({
          runID,
          project,
          rootLabel,
          taskID,
          suffix: 'approval',
          kind: taskID ? 'approval.required' : 'run.monitoring',
          status: taskID ? 'waiting-for-approval' : 'running',
          title: taskID ? 'Needs sign-off' : 'Monitoring next action',
          message: taskID
            ? 'Decision needed before deleting dirty or unmerged worktrees'
            : 'Watching for new agent events',
          agentID: 'controller',
          agentProvider: 'codex',
          agentRole: 'orchestrator',
          stepID: 'event-store',
          stepKind: taskID ? 'cleanup' : 'monitor',
          timestamp: now
        })
      ];

      return {
        id: runID,
        title,
        status: 'running',
        phase: 'store',
        progress: 50,
        projectID: project.id,
        projectName: project.name,
        projectPath: project.path,
        rootLabel,
        taskID,
        startedAt: now,
        updatedAt: now,
        summary: 'Preview run seeded until real orchestration events exist',
        agents: [
          {
            id: 'controller',
            provider: 'codex',
            role: 'orchestrator',
            status: 'running',
            title: 'Codex orchestrator',
            lastActivity: now
          },
          {
            id: 'fixer-1',
            provider: 'codex',
            role: 'fix-agent',
            status: taskID ? 'waiting-for-approval' : 'running',
            title: 'Fix and retest agent',
            lastActivity: now
          }
        ],
        steps: [
          {
            id: 'event-schema',
            kind: 'model',
            title: 'Event schema',
            status: 'succeeded',
            summary: 'Run, agent, step, artifact, and link records',
            agentId: 'controller',
            startedAt: now,
            finishedAt: now
          },
          {
            id: 'event-store',
            kind: 'store',
            title: 'JSONL event store',
            status: 'running',
            summary: 'Append-only run events for skills and adapters',
            agentId: 'controller',
            startedAt: now,
            finishedAt: null
          },
          {
            id: 'retest-loop',
            kind: 'test',
            title: 'Retest loop',
            status: 'running',
            summary: 'Retry failing scenarios after fix batches',
            agentId: 'fixer-1',
            startedAt: now,
            finishedAt: null
          }
        ],
        artifacts: taskID
          ? [
              {
                id: `${runID}-handoff`,
                kind: 'handoff',
                title: 'Agent handoff',
                path: `${project.path}/.codex-artifacts/${runID}-handoff.md`,
                url: null,
                status: 'available'
              }
            ]
          : [],
        links: taskID
          ? [
              {
                kind: 'task',
                label: taskID,
                url: commandCenterTaskUrls[taskID]
              }
            ]
          : [],
        events
      };
    });
  }

  function demoOrchestrationEvent(input: {
    runID: string;
    project: ProjectRoot;
    rootLabel: string;
    taskID: string | null;
    suffix: string;
    kind: string;
    status: string;
    title: string;
    message: string;
    agentID: string;
    agentProvider: string;
    agentRole: string;
    stepID: string;
    stepKind: string;
    timestamp: string;
  }): OrchestrationEvent {
    return {
      schemaVersion: 1,
      id: `${input.runID}-${input.suffix}`,
      runId: input.runID,
      timestamp: input.timestamp,
      kind: input.kind,
      status: input.status,
      title: input.title,
      message: input.message,
      projectID: input.project.id,
      projectName: input.project.name,
      projectPath: input.project.path,
      rootLabel: input.rootLabel,
      taskID: input.taskID,
      agentId: input.agentID,
      agentProvider: input.agentProvider,
      agentRole: input.agentRole,
      stepId: input.stepID,
      stepKind: input.stepKind,
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null
    };
  }

  function demoAgentSessionsForProject(project: ProjectRoot): AgentSession[] {
    return [
      {
        provider: 'codex',
        id: 'preview-session',
        title: `Review ${project.name}`,
        model: null,
        projectPath: project.path,
        lastActivity: null,
        resumeCommands: ['codex resume preview-session']
      }
    ];
  }

  function formatRuntimeContextSummary(
    contextCount: number,
    loadingContexts: boolean,
    contextError: string,
    contextSource: string
  ) {
    if (loadingContexts) return 'Scanning local listeners';
    if (contextError) return contextError;
    if (contextCount === 0) return `No listeners from this project · ${contextSource}`;
    return `${contextCount} ${contextCount === 1 ? 'listener' : 'listeners'} · ${contextSource}`;
  }

  function formatProjectWorktreeSummary(
    worktreeCount: number,
    loadingWorktrees: boolean,
    worktreeError: string,
    worktreeSource: string
  ) {
    if (loadingWorktrees) return 'Scanning selected project';
    if (worktreeError) return worktreeError;
    if (worktreeCount === 0) return `No worktrees found · ${worktreeSource}`;
    return `${worktreeCount} ${worktreeCount === 1 ? 'worktree' : 'worktrees'} · ${worktreeSource}`;
  }

  function formatRepoDashboardSummary(
    summaries: GitRepositorySummary[],
    loadingSummaries: boolean,
    summaryError: string,
    summarySource: string
  ) {
    if (loadingSummaries) return 'Scanning configured repos';
    if (summaryError) return summaryError;
    if (summaries.length === 0) return `No repositories found · ${summarySource}`;

    const dirtyCount = summaries.filter((summary) => summary.isDirty || summary.error).length;
    return `${dirtyCount} dirty / ${summaries.length} repos · ${summarySource}`;
  }

  function formatGitCommitHistorySummary(
    commitCount: number,
    loadingHistory: boolean,
    historyError: string,
    historySource: string
  ) {
    if (loadingHistory) return 'Loading history';
    if (historyError) return historyError;
    if (commitCount === 0) return `No commits · ${historySource}`;
    return `${commitCount} ${commitCount === 1 ? 'commit' : 'commits'} · ${historySource}`;
  }

  function formatGitCommitTime(committedAt: string) {
    const date = new Date(committedAt);
    if (Number.isNaN(date.getTime())) return committedAt;

    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    }).format(date);
  }

  function gitCommitTitle(entry: GitCommitHistoryEntry) {
    const parentSummary = gitCommitParentSummary(entry);
    const taskSource = gitCommitTaskSourceLabel(entry);
    return [
      entry.sha,
      entry.refs,
      parentSummary,
      entry.taskID && taskSource ? `${entry.taskID} from ${taskSource}` : '',
      entry.subject
    ].filter(Boolean).join('\n');
  }

  function gitCommitSummaryText(entry: GitCommitHistoryEntry) {
    const refs = gitCommitRefChips(entry).join(', ');
    const taskSource = gitCommitTaskSourceLabel(entry);
    const task = entry.taskID ? `Task ${entry.taskID}${taskSource ? ` from ${taskSource}` : ''}` : '';
    const topology = gitCommitParentSummary(entry);

    return [
      entry.shortSha,
      entry.subject,
      refs,
      topology,
      task,
      entry.author,
      formatGitCommitTime(entry.committedAt)
    ].filter(Boolean).join(' · ');
  }

  function gitWorkspaceBriefText() {
    const branch = projectGitStatus?.branch ?? selectedProjectRepositorySummaries[0]?.branch ?? 'unknown';
    const aheadBehind =
      projectGitStatus
        ? `ahead ${projectGitStatus.ahead} / behind ${projectGitStatus.behind}`
        : 'ahead/behind unknown';
    const changedFiles =
      projectGitStatus
        ? `${selectedProjectGitChangedFiles.length} changed file${selectedProjectGitChangedFiles.length === 1 ? '' : 's'}`
        : 'changed files unknown';
    const tasks = selectedProjectGitTaskIDs.length > 0 ? selectedProjectGitTaskIDs.join(', ') : 'none';
    const repoLines = selectedProjectRepositorySummaries.slice(0, 6).map((summary) => {
      const dirty = summary.isDirty ? `${summary.dirtyCount} dirty` : 'clean';
      const task = summary.taskID ? ` · ${summary.taskID}` : '';
      const remote = `${summary.ahead} ahead / ${summary.behind} behind`;
      return `- ${summary.repo} (${summary.rootLabel}): ${summary.branch}${task} · ${dirty} · ${remote} · ${summary.path}`;
    });
    const commitLines = gitCommitHistory.slice(0, 6).map((entry) => `- ${gitCommitSummaryText(entry)}`);

    return [
      'Git workspace brief',
      `Project: ${selectedProject.name}`,
      `Path: ${selectedProject.path}`,
      `Branch: ${branch}`,
      `Health: ${selectedProjectGitBranchHealth.detail}`,
      `Status: ${changedFiles} · ${aheadBehind}`,
      `Tasks: ${tasks}`,
      '',
      'Repositories:',
      repoLines.length > 0 ? repoLines.join('\n') : '- none loaded',
      '',
      'Worktree cleanup:',
      projectWorktreeCleanupBrief.report,
      '',
      'Recent commits:',
      commitLines.length > 0 ? commitLines.join('\n') : '- none loaded'
    ].join('\n');
  }

  function sourceContextBriefText() {
    const tasks = selectedProjectGitTaskIDs.length > 0 ? selectedProjectGitTaskIDs.join(', ') : 'none';
    const currentFile = selectedRecord
      ? `${selectedRecord.relativePath}${selectedSourceLine ? `:${selectedSourceLine}` : ''}`
      : 'none';
    const openTabs = projectOpenSourceTabs.slice(0, 8).map((tab) => {
      const dirtyPrefix = isSourcePathDirty(tab.path) ? '* ' : '- ';
      return `${dirtyPrefix}${tab.relativePath}`;
    });
    const dirtyFiles = dirtyProjectSourceRecords.slice(0, 8).map((record) => `- ${record.relativePath}`);

    return [
      'Source context brief',
      `Project: ${selectedProject.name}`,
      `Root: ${selectedProject.path}`,
      `Root label: ${sourceContextIdentity.rootLabel}`,
      `Git: ${sourceContextIdentity.gitSummary}`,
      `Tasks: ${tasks}`,
      `Current file: ${currentFile}`,
      `Language: ${preview?.language ?? 'none'}`,
      `Line: ${selectedSourceLine ?? 'unknown'}`,
      `LSP: ${sourceLspStatusLabel()}`,
      `Index: ${selectedProjectIndexSummary}`,
      `Dirty files: ${dirtyProjectSourceRecords.length}`,
      dirtyFiles.length > 0 ? dirtyFiles.join('\n') : '- none',
      `Open tabs: ${projectOpenSourceTabs.length}`,
      openTabs.length > 0 ? openTabs.join('\n') : '- none'
    ].join('\n');
  }

  function sourceScanDiagnosticBrief() {
    const indexEntry = selectedProjectIndexEntry;
    const stats = sourceScanStats ?? indexEntry?.stats ?? null;
    const activeScan = scanning
      ? `running (${(sourceScanProgress?.matchedFiles ?? records.length).toLocaleString()} matched / ${(
          sourceScanProgress?.visitedEntries ?? 0
        ).toLocaleString()} visited)`
      : 'idle';
    const cacheState = indexEntry
      ? `${indexEntry.records.length.toLocaleString()} ${indexEntry.records.length === 1 ? 'file' : 'files'}, limit ${indexEntry.limit.toLocaleString()}, ${
          indexEntry.truncated ? 'truncated' : 'complete'
        }, ${formatRelativeAge(indexEntry.scannedAt)} old`
      : 'none';
    const selectedPath = selectedSourcePaths[selectedProject.id] ?? 'none';
    const currentFile = selectedRecord
      ? `${selectedRecord.relativePath}${selectedSourceLine ? `:${selectedSourceLine}` : ''}`
      : 'none';
    const recentFiles = projectRecentRecords.slice(0, 6).map((record) => `- ${record.relativePath}`);
    const openTabs = projectOpenSourceTabs.slice(0, 6).map((tab) => {
      const dirtyPrefix = isSourcePathDirty(tab.path) ? '* ' : '- ';
      return `${dirtyPrefix}${tab.relativePath}`;
    });
    const statLines = stats
      ? [
          `Visited entries: ${stats.visitedEntries.toLocaleString()}`,
          `Matched files: ${stats.matchedFiles.toLocaleString()}`,
          `Skipped directories: ${stats.skippedDirectories.toLocaleString()}`,
          `Unsupported files: ${stats.unsupportedFiles.toLocaleString()}`,
          `Unreadable entries: ${stats.unreadableEntries.toLocaleString()}`
        ]
      : ['Scanner stats: none'];

    return [
      'Source scan diagnostic',
      `Project: ${selectedProject.name}`,
      `Root: ${selectedProject.path}`,
      `Root label: ${sourceContextIdentity.rootLabel}`,
      `Scan limit: ${expandedSourceScanLimit.toLocaleString()}`,
      `Indexed files: ${records.length.toLocaleString()}`,
      `Filtered files: ${filteredRecords.length.toLocaleString()}`,
      `Query: ${query.trim() || 'none'}`,
      `Limit reached: ${scanLimitReached ? 'yes' : 'no'}`,
      `Needs attention: ${sourceScanNeedsAttention ? 'yes' : 'no'}`,
      `Health: ${sourceScanHealthNote || 'ok'}`,
      `Index cache: ${cacheState}`,
      `Index summary: ${selectedProjectIndexSummary}`,
      `Scan summary: ${scanSummaryLabel}`,
      `Scan stats: ${sourceScanStatsLabel || 'none'}`,
      `Active scan: ${activeScan}`,
      `Loading: ${loading ? 'yes' : 'no'}`,
      `Runtime: ${runtime}`,
      error ? `Error: ${error}` : 'Error: none',
      `Current file: ${currentFile}`,
      `Saved selected path: ${selectedPath}`,
      '',
      'Raw scanner stats:',
      statLines.join('\n'),
      '',
      `Open tabs: ${projectOpenSourceTabs.length}`,
      openTabs.length > 0 ? openTabs.join('\n') : '- none',
      '',
      `Recent files: ${projectRecentRecords.length}`,
      recentFiles.length > 0 ? recentFiles.join('\n') : '- none',
      '',
      'Next actions:',
      `- Reset index and scan up to ${expandedSourceScanLimit.toLocaleString()} files`,
      '- Confirm the project root is the repository root, not a nested folder',
      '- If indexed files stay tiny, inspect skipped directories and unreadable entries'
    ].join('\n');
  }

  function gitCommitRefChips(entry: GitCommitHistoryEntry) {
    return gitRefLabels(entry.refs);
  }

  function gitCommitParentCount(entry: GitCommitHistoryEntry) {
    if (Number.isFinite(entry.parentCount)) return entry.parentCount;
    return entry.parentShas?.length ?? 1;
  }

  function gitCommitGraphClass(entry: GitCommitHistoryEntry, index: number) {
    return gitCommitGraphKind(entry.refs, index, gitCommitParentCount(entry));
  }

  function gitCommitTopology(entry: GitCommitHistoryEntry, index: number) {
    return gitCommitTopologyLabel(entry.refs, index, gitCommitParentCount(entry));
  }

  function gitCommitParentSummary(entry: GitCommitHistoryEntry) {
    const parentCount = gitCommitParentCount(entry);
    if (parentCount > 1) return `${parentCount} parents`;
    if (parentCount === 0) return 'root commit';
    return '';
  }

  function gitCommitTaskSourceLabel(entry: GitCommitHistoryEntry) {
    if (!entry.taskID) return '';
    if (entry.taskSource === 'refs') return 'branch/ref';
    if (entry.taskSource === 'subject') return 'subject';
    return '';
  }

  function selectGitCommit(entry: GitCommitHistoryEntry) {
    selectedGitCommitSha = entry.sha;
  }

  function handleGitCommitRowKeydown(event: KeyboardEvent, entry: GitCommitHistoryEntry) {
    if (event.key !== 'Enter' && event.key !== ' ') return;

    event.preventDefault();
    selectGitCommit(entry);
  }

  function gitCommitDetailText(entry: GitCommitHistoryEntry | null = selectedGitCommit) {
    if (!entry) return 'No commit selected';

    const refs = gitCommitRefChips(entry).join(', ') || 'none';
    const parents = entry.parentShas.length > 0 ? entry.parentShas.join(', ') : 'none';
    const taskSource = gitCommitTaskSourceLabel(entry);
    const task = entry.taskID ? `${entry.taskID}${taskSource ? ` from ${taskSource}` : ''}` : 'none';
    const taskUrl = entry.taskID ? gitTaskUrl(entry.taskID) : null;

    return [
      `Commit: ${entry.sha}`,
      `Subject: ${entry.subject}`,
      `Refs: ${refs}`,
      `Parents: ${parents}`,
      `Task: ${task}`,
      taskUrl ? `Task link: ${taskUrl}` : '',
      `Author: ${entry.author}`,
      `Committed: ${formatGitCommitTime(entry.committedAt)}`
    ].filter(Boolean).join('\n');
  }

  function gitCommitHandoffText(entry: GitCommitHistoryEntry) {
    const refs = gitCommitRefChips(entry).join(', ') || 'none';
    const parents = entry.parentShas.length > 0 ? entry.parentShas.join(', ') : 'none';
    const firstParent = entry.parentShas[0] ?? '';
    const taskReference = entry.taskID ? gitTaskReferenceText(entry.taskID) : 'none';
    const branch = projectGitStatus?.branch ?? selectedProjectRepositorySummaries[0]?.branch ?? 'unknown';
    const inspectCommands = [
      `cd ${shellQuoteForCommand(selectedProject.path)}`,
      `git show --stat --oneline ${entry.sha}`,
      `git show --name-status --format=fuller ${entry.sha}`,
      firstParent ? `git diff --stat ${firstParent} ${entry.sha}` : '',
      firstParent ? `git diff ${firstParent} ${entry.sha} --` : '',
      `git branch --contains ${entry.sha}`
    ].filter(Boolean);

    return [
      'Git commit handoff',
      `Project: ${selectedProject.name}`,
      `Path: ${selectedProject.path}`,
      `Current branch: ${branch}`,
      `Commit: ${entry.sha}`,
      `Short SHA: ${entry.shortSha}`,
      `Subject: ${entry.subject}`,
      `Author: ${entry.author}`,
      `Committed: ${formatGitCommitTime(entry.committedAt)}`,
      `Refs: ${refs}`,
      `Parents: ${parents}`,
      `Topology: ${gitCommitParentSummary(entry) || 'linear'}`,
      `Task: ${entry.taskID ?? 'none'}`,
      `Task reference: ${taskReference}`,
      `Task source: ${gitCommitTaskSourceLabel(entry) || 'none'}`,
      '',
      'Inspect commands:',
      inspectCommands.map((command) => `- ${command}`).join('\n')
    ].join('\n');
  }

  function gitTaskUrl(taskID: string | null) {
    return taskReferenceUrl(taskID, commandCenterTaskUrls);
  }

  function gitTaskReferenceText(taskID: string) {
    return gitTaskUrl(taskID) ?? taskID;
  }

  async function copyGitCommitSha(entry: GitCommitHistoryEntry) {
    await copyActivityCommand(entry.sha, 'Commit SHA copied');
  }

  async function copyGitCommitSummary(entry: GitCommitHistoryEntry) {
    await copyActivityCommand(gitCommitSummaryText(entry), 'Commit summary copied');
  }

  async function copySelectedGitCommitDetail() {
    if (!selectedGitCommit) return;

    await copyActivityCommand(gitCommitDetailText(selectedGitCommit), 'Commit detail copied');
  }

  async function copyGitCommitHandoff(entry: GitCommitHistoryEntry) {
    await copyActivityCommand(gitCommitHandoffText(entry), 'Commit handoff copied');
  }

  async function copyGitTaskReference(taskID: string | null) {
    if (!taskID) return;

    await copyActivityCommand(gitTaskReferenceText(taskID), gitTaskUrl(taskID) ? 'Task link copied' : 'Task ID copied');
  }

  async function copyGitWorkspaceBrief() {
    await copyActivityCommand(gitWorkspaceBriefText(), 'Git workspace brief copied');
  }

  async function copySourceContextBrief() {
    await copyActivityCommand(sourceContextBriefText(), 'Source context brief copied');
  }

  async function copySourceScanDiagnosticBrief() {
    await copyActivityCommand(sourceScanDiagnosticBrief(), 'Scan diagnostic copied');
  }

  function repoDashboardTaskLabel(summary: GitRepositorySummary) {
    return summary.taskID ?? 'none';
  }

  function repoDashboardTaskUrl(summary: GitRepositorySummary) {
    return gitTaskUrl(summary.taskID);
  }

  function repoDashboardDirtyLabel(summary: GitRepositorySummary) {
    if (summary.error) return 'error';
    if (!summary.isDirty) return 'clean';

    const counts = `${summary.dirtyCount} files`;
    if (!summary.dirtySinceEpochMs) return counts;

    return `${counts} · ${formatRelativeAge(summary.dirtySinceEpochMs)}`;
  }

  function repoDashboardRemoteLabel(summary: GitRepositorySummary) {
    const remote = `↑${summary.ahead} ↓${summary.behind}`;
    const commit = summary.lastCommitSha ? ` · ${summary.lastCommitSha}` : '';
    return `${remote}${commit}`;
  }

  function repoDashboardTitle(summary: GitRepositorySummary) {
    const parts = [
      summary.path,
      summary.lastCommitSubject ? `Last commit: ${summary.lastCommitSubject}` : '',
      summary.error ? `Error: ${summary.error}` : ''
    ].filter(Boolean);
    return parts.join('\n');
  }

  function formatRelativeAge(epochMs: number) {
    const elapsedMs = Math.max(0, Date.now() - epochMs);
    const minuteMs = 60 * 1000;
    const hourMs = 60 * minuteMs;
    const dayMs = 24 * hourMs;

    if (elapsedMs < minuteMs) return 'just now';
    if (elapsedMs < hourMs) return `${Math.floor(elapsedMs / minuteMs)}m`;
    if (elapsedMs < dayMs) return `${Math.floor(elapsedMs / hourMs)}h`;
    return `${Math.floor(elapsedMs / dayMs)}d`;
  }

  function isGitFileStaged(fileStatus: ProjectGitFileStatus | null | undefined) {
    return Boolean(fileStatus?.indexStatus && fileStatus.badge !== '?');
  }

  function isGitFileUntracked(fileStatus: ProjectGitFileStatus) {
    return fileStatus.badge === '?' || fileStatus.indexStatus === '?' || fileStatus.worktreeStatus === '?';
  }

  function hasGitFileUnstagedChanges(fileStatus: ProjectGitFileStatus) {
    return isGitFileUntracked(fileStatus) || Boolean(fileStatus.worktreeStatus);
  }

  function buildGitStatusFileGroups(fileStatuses: ProjectGitFileStatus[]): GitStatusFileGroup[] {
    const stagedFiles = fileStatuses.filter(isGitFileStaged);
    const unstagedFiles = fileStatuses.filter(
      (fileStatus) => hasGitFileUnstagedChanges(fileStatus) && !isGitFileUntracked(fileStatus)
    );
    const untrackedFiles = fileStatuses.filter(isGitFileUntracked);

    return [
      { id: 'staged', label: 'Staged', files: stagedFiles, action: 'unstage' },
      { id: 'unstaged', label: 'Unstaged', files: unstagedFiles, action: 'stage' },
      { id: 'untracked', label: 'Untracked', files: untrackedFiles, action: 'stage' }
    ].filter((group) => group.files.length > 0);
  }

  function formatGitStatusFileGroupSummary(groups: GitStatusFileGroup[]) {
    if (groups.length === 0) return 'No changed files';
    return groups.map((group) => `${group.label} ${group.files.length}`).join(' · ');
  }

  function gitStatusGroupFileCount(groups: GitStatusFileGroup[], groupID: GitStatusGroupID) {
    return groups.find((group) => group.id === groupID)?.files.length ?? 0;
  }

  function gitStatusGroupActionLabel(group: GitStatusFileGroup) {
    return group.action === 'unstage' ? 'Unstage all' : 'Stage all';
  }

  async function runGitStatusGroupAction(group: GitStatusFileGroup) {
    await runGitPathAction(
      group.action,
      group.files.map((fileStatus) => fileStatus.relativePath)
    );
  }

  function gitStatusFileTitle(fileStatus: ProjectGitFileStatus) {
    const states = [
      fileStatus.indexStatus ? `Index: ${fileStatus.indexStatus}` : '',
      fileStatus.worktreeStatus ? `Worktree: ${fileStatus.worktreeStatus}` : ''
    ].filter(Boolean);
    return `${fileStatus.relativePath}${states.length > 0 ? `\n${states.join('\n')}` : ''}`;
  }

  function gitStatusFileSummary(fileStatus: ProjectGitFileStatus) {
    if (fileStatus.indexStatus && fileStatus.worktreeStatus) {
      return `${fileStatus.indexStatus} + ${fileStatus.worktreeStatus}`;
    }

    return fileStatus.status || 'changed';
  }

  async function selectGitStatusFile(fileStatus: ProjectGitFileStatus) {
    const record = records.find((sourceRecord) => sourceRecord.relativePath === fileStatus.relativePath);
    if (!record) {
      gitActionStatus = `No indexed source record for ${fileStatus.relativePath}`;
      return;
    }

    await selectRecord(record);
  }

  async function runGitPathAction(action: 'stage' | 'unstage', paths: string[]) {
    const nextPaths = paths.map((path) => path.trim()).filter(Boolean);
    if (nextPaths.length === 0) return;

    gitActionBusy = action;
    gitActionError = '';
    gitActionStatus = '';

    try {
      const result =
        action === 'stage'
          ? await stageGitPathsFromTauri(selectedProject.path, nextPaths)
          : await unstageGitPathsFromTauri(selectedProject.path, nextPaths);

      if (!result) {
        gitActionStatus = 'Native Git unavailable';
        return;
      }

      projectGitStatus = result.status;
      gitActionStatus = result.message;
      if (selectedRecord) void loadSelectedSourceGitDiff(selectedRecord);
      void loadGitRepositorySummaries(projectOptions);
    } catch (gitError) {
      gitActionError = gitError instanceof Error ? gitError.message : 'Could not update Git index';
    } finally {
      gitActionBusy = '';
    }
  }

  async function runGitRemoteAction(action: 'fetch' | 'pull' | 'push') {
    gitActionBusy = action;
    gitActionError = '';
    gitActionStatus = '';

    try {
      const result =
        action === 'fetch'
          ? await fetchGitRepositoryFromTauri(selectedProject.path)
          : action === 'pull'
            ? await pullGitRepositoryFromTauri(selectedProject.path)
            : await pushGitRepositoryFromTauri(selectedProject.path);

      if (!result) {
        gitActionStatus = 'Native Git unavailable';
        return;
      }

      projectGitStatus = result.status;
      gitActionStatus = result.message;
      if (selectedRecord) void loadSelectedSourceGitDiff(selectedRecord);
      void loadGitRepositorySummaries(projectOptions);
      if (action !== 'fetch') void loadGitCommitHistory(selectedProject);
    } catch (gitError) {
      gitActionError = gitError instanceof Error ? gitError.message : `Could not ${action} repository`;
    } finally {
      gitActionBusy = '';
    }
  }

  async function commitGitChanges() {
    const message = gitCommitMessage.trim();
    if (!message) return;

    gitActionBusy = 'commit';
    gitActionError = '';
    gitActionStatus = '';

    try {
      const result = await commitGitRepositoryFromTauri(selectedProject.path, message);
      if (!result) {
        gitActionStatus = 'Native Git unavailable';
        return;
      }

      projectGitStatus = result.status;
      gitCommitMessage = '';
      gitActionStatus = result.message;
      if (selectedRecord) void loadSelectedSourceGitDiff(selectedRecord);
      void loadGitRepositorySummaries(projectOptions);
      void loadGitCommitHistory(selectedProject);
    } catch (gitError) {
      gitActionError = gitError instanceof Error ? gitError.message : 'Could not commit Git changes';
    } finally {
      gitActionBusy = '';
    }
  }

  function formatOrchestrationRunSummary(
    runCount: number,
    loadingRuns: boolean,
    runError: string,
    runSource: string
  ) {
    if (loadingRuns) return 'Reading orchestration events';
    if (runError) return runError;
    if (runCount === 0) return `No runs for this project · ${runSource}`;
    return `${runCount} ${runCount === 1 ? 'run' : 'runs'} · ${runSource}`;
  }

  function orchestrationRunMatchesProject(run: OrchestrationRun, project: ProjectRoot) {
    if (run.projectID && run.projectID === project.id) return true;
    if (run.projectName === project.name) return true;
    if (!run.projectPath) return true;

    const runPath = normalizeProjectPath(run.projectPath);
    const projectPath = normalizeProjectPath(project.path);
    if (runPath === projectPath || runPath.startsWith(`${projectPath}/`)) return true;

    return runPath.toLowerCase().includes(`/worktrees/${project.name.toLowerCase()}/`);
  }

  function orchestrationStatusClass(status: string) {
    return orchestrationStatusTone(status);
  }

  function orchestrationRunTaskUrl(run: OrchestrationRun) {
    return gitTaskUrl(run.taskID);
  }

  function orchestrationRunTimeLabel(run: OrchestrationRun) {
    const value = run.updatedAt ?? run.startedAt;
    if (!value) return 'unknown activity';
    const numericValue = Number(value);
    const date = Number.isFinite(numericValue) ? new Date(numericValue) : new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return formatRelativeAge(date.getTime());
  }

  function orchestrationTimelineTimeLabel(item: OrchestrationTimelineItem) {
    if (!item.timestamp) return item.source;
    const numericValue = Number(item.timestamp);
    const date = Number.isFinite(numericValue) ? new Date(numericValue) : new Date(item.timestamp);
    if (Number.isNaN(date.getTime())) return item.source;
    return formatRelativeAge(date.getTime());
  }

  function orchestrationRunTitle(run: OrchestrationRun) {
    return [
      run.title,
      run.summary,
      run.projectPath,
      run.steps.map((step) => `${step.status}: ${step.title}`).join('\n')
    ]
      .filter(Boolean)
      .join('\n');
  }

  function focusOrchestrationRun(run: OrchestrationRun) {
    selectSourceActivityMode('runs');
    sourceActivityFilter = run.taskID ?? run.title ?? run.id;
  }

  async function copyOrchestrationRunSummary(run: OrchestrationRun) {
    await copyActivityCommand(orchestrationRunSummaryText(run), 'Run summary copied');
  }

  async function copyOrchestrationRunHandoff(run: OrchestrationRun) {
    await copyActivityCommand(orchestrationRunHandoffText(run), 'Run handoff copied');
  }

  async function copyOrchestrationCurrentActivity(run: OrchestrationRun) {
    await copyActivityCommand(orchestrationCurrentActivity(run), 'Current run activity copied');
  }

  async function copyOrchestrationTaskReference(run: OrchestrationRun) {
    if (!run.taskID) return;

    await copyGitTaskReference(run.taskID);
  }

  function orchestrationStepIconLabel(status: string) {
    switch (orchestrationStatusClass(status)) {
      case 'good':
        return '✓';
      case 'bad':
        return '!';
      case 'attention':
        return '?';
      case 'live':
        return '…';
      default:
        return '•';
    }
  }

  function formatAgentSessionSummary(
    sessionCount: number,
    loadingSessions: boolean,
    sessionError: string,
    sessionSource: string
  ) {
    if (loadingSessions) return 'Scanning local sessions';
    if (sessionError) return sessionError;
    if (sessionCount === 0) return `No sessions for this project · ${sessionSource}`;
    return `${sessionCount} ${sessionCount === 1 ? 'session' : 'sessions'} · ${sessionSource}`;
  }

  function agentSessionMatchesProject(session: AgentSession, project: ProjectRoot) {
    if (!session.projectPath) return true;

    const sessionPath = normalizeProjectPath(session.projectPath);
    const projectPath = normalizeProjectPath(project.path);
    if (sessionPath === projectPath || sessionPath.startsWith(`${projectPath}/`)) return true;

    const projectName = project.name.toLowerCase();
    return sessionPath.toLowerCase().includes(`/worktrees/${projectName}/`);
  }

  function agentSessionProjectLabel(session: AgentSession) {
    return session.projectPath ? formatSourceContextRootLabel(session.projectPath) : 'global';
  }

  function agentSessionModelLabel(session: AgentSession) {
    return session.model?.trim() || 'model unknown';
  }

  function agentSessionActivityLabel(session: AgentSession) {
    return session.lastActivity ?? 'unknown activity';
  }

  function agentSessionProjectPath(session: AgentSession) {
    return session.projectPath?.trim() || selectedProject.path;
  }

  function agentSessionResumeCommandList(session: AgentSession) {
    const commands = session.resumeCommands.map((command) => command.trim()).filter(Boolean);
    return commands.length > 0 ? commands : [`${session.provider} resume ${session.id}`];
  }

  function agentSessionResumeCommand(session: AgentSession) {
    return agentSessionResumeCommandList(session)[0] ?? '';
  }

  function isLeadingShellCdCommand(command: string) {
    const trimmed = command.trim();
    return /^cd\s+/.test(trimmed) && trimmed.includes('&&');
  }

  function stripLeadingShellCdCommand(command: string) {
    const trimmed = command.trim();
    if (!isLeadingShellCdCommand(trimmed)) return trimmed;

    const separatorIndex = trimmed.indexOf('&&');
    return separatorIndex >= 0 ? trimmed.slice(separatorIndex + 2).trim() : trimmed;
  }

  function agentSessionTerminalCommand(session: AgentSession) {
    const commands = agentSessionResumeCommandList(session);
    return commands.find((command) => !isLeadingShellCdCommand(command))
      ?? stripLeadingShellCdCommand(commands[0] ?? '');
  }

  function agentSessionResumeShellCommand(session: AgentSession) {
    const commands = agentSessionResumeCommandList(session);
    const shellCommand = commands.find(isLeadingShellCdCommand);
    if (shellCommand) return shellCommand;

    const command = agentSessionTerminalCommand(session);
    const path = agentSessionProjectPath(session);
    return path.trim() ? `cd ${shellQuoteForCommand(path)} && ${command}` : command;
  }

  function captureCurrentWorkspaceSnapshot() {
    captureWorkspaceSnapshot(selectedProjectAgentSessions[0] ?? null);
  }

  function captureAgentSessionWorkspaceSnapshot(session: AgentSession): WorkspaceSnapshot {
    return captureWorkspaceSnapshot(session);
  }

  function captureActiveWorkspaceBeforeSwitch() {
    if (!activeWorkspaceSessionKey) return;

    const activeSession = agentSessions.find(
      (session) => workspaceSnapshotIDForAgentSession(session) === activeWorkspaceSessionKey
    );
    if (!activeSession) return;

    captureAgentSessionWorkspaceSnapshot(activeSession);
  }

  function captureActiveWorkspaceSnapshotBeforeUnload() {
    if (activeWorkspaceSessionKey) {
      captureActiveWorkspaceBeforeSwitch();
      return;
    }

    captureCurrentWorkspaceSnapshot();
  }

  function handleWorkspaceSnapshotVisibilityChange() {
    if (document.visibilityState === 'hidden') {
      captureActiveWorkspaceSnapshotBeforeUnload();
    }
  }

  function captureWorkspaceSnapshot(session: AgentSession | null): WorkspaceSnapshot {
    const cwd = session?.projectPath ?? selectedProject.path;
    const project = workspaceSnapshotProjectForSession(session);
    const snapshot = createWorkspaceSnapshot({
      provider: workspaceSnapshotProviderForSession(session),
      sessionID: session ? workspaceSnapshotSessionIDForSession(session) : selectedProject.id,
      title: session?.title ?? `${selectedProject.name} workspace`,
      model: session?.model ?? null,
      project,
      cwd,
      worktreePath: snapshotWorktreePathForPath(cwd),
      branch: projectGitStatus?.branch ?? selectedProjectRepositorySummaries[0]?.branch ?? null,
      selectedPath: workspaceSnapshotSelectedPathForProject(project),
      selectedLine: selectedSourceLine,
      openPaths: workspaceSnapshotOpenPathsForProject(project),
      sourceActivityMode,
      sourceTerminalApp,
      embeddedTerminal: workspaceSnapshotEmbeddedTerminal(),
      dockLayout: sourceDockLayout,
      resumeCommand: session ? agentSessionTerminalCommand(session) : null,
      capturedAt: Date.now()
    });
    const nextSnapshots = upsertWorkspaceSnapshot(
      workspaceSnapshots,
      snapshot,
      maxWorkspaceSnapshots
    );

    workspaceSnapshots = nextSnapshots;
    persistWorkspaceSnapshots(nextSnapshots);
    fileActionStatus = `Workspace snapshot saved for ${snapshot.title}`;
    return snapshot;
  }

  async function restoreAgentSessionWorkspaceSnapshot(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) {
      fileActionStatus = `No workspace snapshot saved for ${session.title}`;
      return;
    }

    await restoreConversationWorkspaceSnapshot(snapshot);
    markAgentSessionWorkspaceActive(session);
  }

  async function openAgentSessionWorkspace(session: AgentSession) {
    captureActiveWorkspaceBeforeSwitch();
    const snapshot = workspaceSnapshotForAgentSession(session) ?? captureAgentSessionWorkspaceSnapshot(session);
    await restoreConversationWorkspaceSnapshot(snapshot);
    markAgentSessionWorkspaceActive(session);
  }

  function markAgentSessionWorkspaceActive(session: AgentSession) {
    activeWorkspaceSessionKey = workspaceSnapshotIDForAgentSession(session);
    persistActiveWorkspaceSessionKey(activeWorkspaceSessionKey);
  }

  async function openWorkspaceSnapshotTerminal(snapshot: WorkspaceSnapshot) {
    const command = stripLeadingShellCdCommand(snapshot.resumeCommand?.trim() ?? '');
    const path = snapshot.worktreePath ?? snapshot.cwd;
    if (!command && !path.trim()) return;

    fileActionBusy = `workspace-terminal-command:${snapshot.id}`;
    fileActionStatus = '';
    error = '';

    try {
      if (command) {
        const openedCommand = await openTerminalCommandFromTauri(
          path,
          command,
          snapshot.sourceTerminalApp
        );
        if (openedCommand) {
          fileActionStatus = 'Opened workspace resume command';
          return;
        }
      }

      const openedPath = path.trim()
        ? await openTerminalPathFromTauri(path, snapshot.sourceTerminalApp)
        : false;
      if (command) await navigator.clipboard.writeText(command);
      fileActionStatus = command
        ? openedPath
          ? 'Opened workspace terminal and copied resume command'
          : 'Workspace resume command copied'
        : openedPath
          ? 'Opened workspace terminal'
          : 'Native action unavailable';
    } catch (terminalError) {
      try {
        const openedPath = path.trim()
          ? await openTerminalPathFromTauri(path, snapshot.sourceTerminalApp)
          : false;
        if (command) await navigator.clipboard.writeText(command);
        fileActionStatus = command
          ? openedPath
            ? 'Opened workspace terminal and copied resume command'
            : 'Workspace resume command copied'
          : 'Native action unavailable';
      } catch {
        error = terminalError instanceof Error ? terminalError.message : 'Could not open workspace terminal';
      }
    } finally {
      fileActionBusy = '';
    }
  }

  async function openWorkspaceSnapshotEmbeddedTerminal(snapshot: WorkspaceSnapshot) {
    const command = stripLeadingShellCdCommand(snapshot.resumeCommand?.trim() ?? '');
    const path = snapshot.worktreePath ?? snapshot.cwd;
    if (!command && !path.trim()) return;

    await restoreConversationWorkspaceSnapshot(snapshot);
    showDockPanel('terminal');
    await tick();

    fileActionBusy = `workspace-embedded-terminal-command:${snapshot.id}`;
    fileActionStatus = '';
    error = '';
    embeddedTerminalError = '';

    try {
      if (command) {
        if (
          embeddedTerminalSession &&
          normalizeProjectPath(embeddedTerminalSession.cwd) === normalizeProjectPath(path)
        ) {
          await writeTerminalSessionFromTauri(embeddedTerminalSession.sessionId, `${command}\r`);
          embeddedTerminalStatus = 'Running embedded workspace command';
          fileActionStatus = 'Sent workspace command to embedded terminal';
          embeddedTerminal?.focus();
          return;
        }

        await startEmbeddedTerminalSession(path, command);
        fileActionStatus = 'Started embedded workspace command';
        return;
      }

      await openPathEmbeddedTerminal(path);
      fileActionStatus = 'Opened embedded workspace terminal';
    } catch (terminalError) {
      error =
        terminalError instanceof Error
          ? terminalError.message
          : 'Could not resume workspace in embedded terminal';
      embeddedTerminalStatus = 'Embedded workspace resume failed';
    } finally {
      fileActionBusy = '';
    }
  }

  function deleteWorkspaceSnapshot(snapshot: WorkspaceSnapshot) {
    const nextSnapshots = workspaceSnapshots.filter((candidate) => candidate.id !== snapshot.id);
    workspaceSnapshots = nextSnapshots;
    persistWorkspaceSnapshots(nextSnapshots);
    if (activeWorkspaceSessionKey === snapshot.id) {
      activeWorkspaceSessionKey = null;
      persistActiveWorkspaceSessionKey(null);
    }
    fileActionStatus = `Workspace snapshot deleted for ${snapshot.title}`;
  }

  function workspaceSnapshotRestorePlan(snapshot: WorkspaceSnapshot) {
    const selectedFile = snapshot.selectedPath
      ? `${workspaceSnapshotRelativePath(snapshot, snapshot.selectedPath)}${snapshot.selectedLine ? `:${snapshot.selectedLine}` : ''}`
      : 'none';
    const openPathLines = snapshot.openPaths
      .slice(0, 12)
      .map((path) => `- ${workspaceSnapshotRelativePath(snapshot, path)}`);
    const openPathOverflow =
      snapshot.openPaths.length > openPathLines.length
        ? `- ... ${snapshot.openPaths.length - openPathLines.length} more`
        : '';
    const embeddedTerminal = snapshot.embeddedTerminal
      ? `${snapshot.embeddedTerminal.cwd} (${snapshot.embeddedTerminal.sessionID})`
      : 'none';

    return [
      'Workspace restore plan',
      `Title: ${snapshot.title}`,
      `Provider: ${snapshot.provider}`,
      `Session: ${snapshot.sessionID}`,
      `Model: ${snapshot.model ?? 'unknown'}`,
      `Scope: ${workspaceSnapshotScopeLabel(snapshot)}`,
      `Project: ${snapshot.project.name}`,
      `Root: ${snapshot.project.path}`,
      `CWD: ${snapshot.cwd}`,
      `Worktree: ${snapshot.worktreePath ?? 'none'}`,
      `Branch: ${snapshot.branch ?? 'unknown'}`,
      `Selected file: ${selectedFile}`,
      `Open files: ${snapshot.openPaths.length}`,
      openPathLines.length > 0 ? openPathLines.join('\n') : '- none',
      openPathOverflow,
      `Activity pane: ${sourceActivityLabel(snapshot.sourceActivityMode)}`,
      `Terminal app: ${snapshot.sourceTerminalApp}`,
      `Embedded terminal: ${embeddedTerminal}`,
      `Resume command: ${snapshot.resumeCommand ?? 'none'}`,
      `Captured: ${formatWorkspaceSnapshotTime(snapshot.capturedAt)}`
    ].filter(Boolean).join('\n');
  }

  function workspaceSnapshotRelativePath(snapshot: WorkspaceSnapshot, path: string) {
    const normalizedPath = normalizeProjectPath(path);
    const normalizedProjectPath = normalizeProjectPath(snapshot.project.path);
    if (normalizedPath === normalizedProjectPath) return fileNameFromRestoredPath(normalizedPath);
    if (normalizedPath.startsWith(`${normalizedProjectPath}/`)) {
      return normalizedPath.slice(normalizedProjectPath.length + 1);
    }
    return normalizedPath;
  }

  function workspaceSnapshotScopeLabel(snapshot: WorkspaceSnapshot) {
    const snapshotProjectPath = normalizeProjectPath(snapshot.project.path);
    const selectedProjectPath = normalizeProjectPath(selectedProject.path);
    const snapshotCwd = normalizeProjectPath(snapshot.cwd);
    const snapshotWorktreePath = snapshot.worktreePath ? normalizeProjectPath(snapshot.worktreePath) : '';

    if (snapshotProjectPath === selectedProjectPath) return 'current project';
    if (snapshotCwd === selectedProjectPath || snapshotWorktreePath === selectedProjectPath) {
      return 'current worktree';
    }
    if (snapshotWorktreePath) return formatSourceContextRootLabel(snapshotWorktreePath);
    return formatSourceContextRootLabel(snapshot.project.path);
  }

  function formatWorkspaceSnapshotTime(capturedAt: number) {
    const date = new Date(capturedAt);
    if (Number.isNaN(date.getTime())) return 'unknown';

    return `${new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    }).format(date)} (${formatRelativeAge(date.getTime())})`;
  }

  async function copyWorkspaceSnapshotRestorePlan(snapshot: WorkspaceSnapshot) {
    await copyActivityCommand(workspaceSnapshotRestorePlan(snapshot), 'Workspace restore plan copied');
  }

  async function copyAgentSessionWorkspaceRestorePlan(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) {
      fileActionStatus = `No workspace snapshot saved for ${session.title}`;
      return;
    }

    await copyWorkspaceSnapshotRestorePlan(snapshot);
  }

  async function restoreConversationWorkspaceSnapshot(snapshot: WorkspaceSnapshot) {
    const restored = restoreWorkspaceSnapshot(snapshot);
    const project = ensureWorkspaceSnapshotProject(snapshot.project);
    const nextSelectedSourcePaths = {
      ...selectedSourcePaths,
      ...restored.selectedSourcePaths
    };

    selectedSourcePaths = nextSelectedSourcePaths;
    persistSelectedSourcePaths(nextSelectedSourcePaths);
    sourceActivityMode = restored.sourceActivityMode;
    persistSourceActivityMode(sourceActivityMode);
    sourceTerminalApp = restored.sourceTerminalApp;
    persistSourceTerminalApp(sourceTerminalApp);
    sourceDockLayout = restored.dockLayout;
    syncSourceDockLayoutToWorkspace(sourceDockLayout);
    persistSourceDockLayout(sourceDockLayout);
    activeWorkspaceSessionKey = snapshot.id;
    persistActiveWorkspaceSessionKey(activeWorkspaceSessionKey);
    fileActionStatus = `Workspace restored: ${snapshot.title}`;

    await activateWorkspaceSnapshotProject(project);

    restoreWorkspaceOpenTabs(project, restored.openPaths, restored.selectedPath);
    if (restored.selectedPath) {
      await selectRecord(sourceRecordFromRestoredPath(project, restored.selectedPath), restored.selectedLine);
    } else if (restored.selectedLine) {
      revealSourceLine(restored.selectedLine);
    }

    await restoreWorkspaceEmbeddedTerminal(restored.embeddedTerminal);
  }

  function workspaceSnapshotEmbeddedTerminal(): WorkspaceSnapshotEmbeddedTerminal | null {
    if (!embeddedTerminalSession) return null;

    return {
      sessionID: embeddedTerminalSession.sessionId,
      cwd: embeddedTerminalSession.cwd,
      shell: embeddedTerminalSession.shell,
      startedAt: embeddedTerminalSession.startedAt
    };
  }

  async function restoreWorkspaceEmbeddedTerminal(
    savedTerminal: WorkspaceSnapshotEmbeddedTerminal | null
  ) {
    if (!savedTerminal) return;

    try {
      await tick();
      const sessions = await listTerminalSessionsFromTauri();
      if (!sessions) return;

      embeddedTerminalSessions = sessions;
      const matchingSession =
        sessions.find((session) => session.sessionId === savedTerminal.sessionID) ??
        sessions.find(
          (session) =>
            normalizeProjectPath(session.cwd) === normalizeProjectPath(savedTerminal.cwd)
        );

      if (matchingSession) {
        await attachEmbeddedTerminalSession(matchingSession);
        return;
      }

      if (embeddedTerminalSession?.sessionId === savedTerminal.sessionID) {
        embeddedTerminalSession = null;
      }
      embeddedTerminalStatus = 'Saved embedded terminal is no longer live';
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not restore embedded terminal';
    }
  }

  async function activateWorkspaceSnapshotProject(project: ProjectRoot) {
    await activateProject(project, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects: mergeProjectRoots(defaultProjectRoots, customProjectRoots)
    });
  }

  function ensureWorkspaceSnapshotProject(project: ProjectRoot): ProjectRoot {
    const existingProject = projectOptions.find(
      (candidate) =>
        candidate.id === project.id ||
        normalizeProjectPath(candidate.path) === normalizeProjectPath(project.path)
    );
    if (existingProject) return existingProject;

    const nextProject = createProjectRoot(project.name, project.path);
    const nextCustomProjectRoots = mergeProjectRoots([], [...customProjectRoots, nextProject]);
    customProjectRoots = nextCustomProjectRoots;
    persistCustomProjectRoots(nextCustomProjectRoots);
    return nextProject;
  }

  function restoreWorkspaceOpenTabs(
    project: ProjectRoot,
    openPaths: string[],
    selectedPath: string | null = null
  ) {
    const openedAt = Date.now();
    const seenPaths = new Set<string>();
    const restoredTabs = [selectedPath, ...openPaths]
      .filter((path): path is string => typeof path === 'string' && path.trim().length > 0)
      .filter((path) => {
        const normalizedPath = normalizeProjectPath(path);
        if (seenPaths.has(normalizedPath)) return false;
        seenPaths.add(normalizedPath);
        return true;
      })
      .map((path) => sourceRecordFromRestoredPath(project, path))
      .slice(0, maxProjectOpenSourceTabs)
      .map((record, index): SourceOpenTab => ({
        ...record,
        projectID: project.id,
        projectName: project.name,
        openedAt: openedAt + index
      }));
    if (restoredTabs.length === 0) return;

    const nextOpenSourceTabs = replaceProjectOpenTabs(openSourceTabs, project.id, restoredTabs);
    openSourceTabs = nextOpenSourceTabs;
    persistOpenSourceTabs(nextOpenSourceTabs);
  }

  function sourceRecordFromRestoredPath(project: ProjectRoot, path: string): SourceRecord {
    const normalizedPath = normalizeProjectPath(path);
    const existingRecord = records.find((record) => normalizeProjectPath(record.path) === normalizedPath);
    if (existingRecord) return existingRecord;

    const normalizedProjectPath = normalizeProjectPath(project.path);
    const relativePath =
      normalizedPath === normalizedProjectPath
        ? fileNameFromRestoredPath(normalizedPath)
        : normalizedPath.startsWith(`${normalizedProjectPath}/`)
          ? normalizedPath.slice(normalizedProjectPath.length + 1)
          : fileNameFromRestoredPath(normalizedPath);

    return {
      path: normalizedPath,
      relativePath,
      fileName: fileNameFromRestoredPath(relativePath),
      language: sourceLanguageForRestoredPath(normalizedPath),
      byteCount: 0
    };
  }

  function fileNameFromRestoredPath(path: string) {
    return path.split('/').filter(Boolean).at(-1) ?? 'file';
  }

  function sourceLanguageForRestoredPath(path: string): SourceLanguage {
    const normalizedPath = path.toLowerCase();
    if (normalizedPath.endsWith('.cs')) return 'csharp';
    if (normalizedPath.endsWith('.tsx')) return 'tsx';
    if (normalizedPath.endsWith('.ts')) return 'typescript';
    if (normalizedPath.endsWith('.jsx')) return 'jsx';
    if (normalizedPath.endsWith('.js') || normalizedPath.endsWith('.mjs')) return 'javascript';
    if (normalizedPath.endsWith('.svelte')) return 'svelte';
    if (normalizedPath.endsWith('.rs')) return 'rust';
    if (normalizedPath.endsWith('.swift')) return 'swift';
    if (normalizedPath.endsWith('.json')) return 'json';
    if (normalizedPath.endsWith('.md') || normalizedPath.endsWith('.mdx')) return 'markdown';
    if (normalizedPath.endsWith('.toml')) return 'toml';
    if (normalizedPath.endsWith('.yaml') || normalizedPath.endsWith('.yml')) return 'yaml';
    if (normalizedPath.endsWith('.css')) return 'css';
    if (normalizedPath.endsWith('.html')) return 'html';
    if (normalizedPath.endsWith('.xml')) return 'xml';
    if (normalizedPath.endsWith('.sh') || normalizedPath.endsWith('.zsh')) return 'shell';
    return 'plain';
  }

  function workspaceSnapshotProviderForSession(session: AgentSession | null): WorkspaceSnapshotProvider {
    const provider = session?.provider.trim().toLowerCase();
    if (provider?.startsWith('cmux-')) return 'cmux';
    if (provider === 'codex' || provider === 'claude' || provider === 'cmux') return provider;
    return 'manual';
  }

  function workspaceSnapshotSessionIDForSession(session: AgentSession) {
    const sessionID = session.id.trim() || 'session';
    const provider = session.provider.trim().toLowerCase();
    return provider.startsWith('cmux-') ? `${provider}:${sessionID}` : sessionID;
  }

  function workspaceSnapshotProjectForSession(session: AgentSession | null): ProjectRoot {
    const sessionPath = session?.projectPath?.trim();
    if (!sessionPath) return selectedProject;

    const normalizedSessionPath = normalizeProjectPath(sessionPath);
    const existingProject = projectOptions.find(
      (project) => normalizeProjectPath(project.path) === normalizedSessionPath
    );
    if (existingProject) return existingProject;

    if (normalizedSessionPath === normalizeProjectPath(selectedProject.path)) return selectedProject;

    return createProjectRoot(selectedProject.name, normalizedSessionPath);
  }

  function workspaceSnapshotForAgentSession(session: AgentSession): WorkspaceSnapshot | null {
    const snapshotID = workspaceSnapshotIDForAgentSession(session);
    return workspaceSnapshots.find((snapshot) => snapshot.id === snapshotID) ?? null;
  }

  function workspaceSnapshotIDForAgentSession(session: AgentSession) {
    const provider = workspaceSnapshotProviderForSession(session);
    return `${provider}:${workspaceSnapshotSessionIDForSession(session)}`;
  }

  function workspaceSnapshotSelectedPathForProject(project: ProjectRoot) {
    if (!selectedRecord) return selectedSourcePaths[project.id] ?? null;

    const projectPath = normalizeProjectPath(project.path);
    const selectedPath = normalizeProjectPath(selectedRecord.path);
    if (selectedPath === projectPath || selectedPath.startsWith(`${projectPath}/`)) return selectedRecord.path;

    return `${project.path}/${selectedRecord.relativePath}`;
  }

  function workspaceSnapshotOpenPathsForProject(project: ProjectRoot) {
    const projectPath = normalizeProjectPath(project.path);
    return projectOpenSourceTabs.map((tab) => {
      const tabPath = normalizeProjectPath(tab.path);
      if (tabPath === projectPath || tabPath.startsWith(`${projectPath}/`)) return tab.path;
      return `${project.path}/${tab.relativePath}`;
    });
  }

  function snapshotWorktreePathForPath(path: string) {
    const normalizedPath = normalizeProjectPath(path);
    return normalizedPath.includes('/worktrees/') ? normalizedPath : null;
  }

  function agentSessionResumePlan(session: AgentSession) {
    const commands = agentSessionResumeCommandList(session);
    const alternateCommands = commands.slice(1);

    return [
      `Session: ${session.title}`,
      `Provider: ${session.provider}`,
      `ID: ${session.id}`,
      `Model: ${agentSessionModelLabel(session)}`,
      `Project: ${agentSessionProjectPath(session)}`,
      `Activity: ${agentSessionActivityLabel(session)}`,
      '',
      'Shell resume:',
      agentSessionResumeShellCommand(session),
      '',
      alternateCommands.length > 0 ? 'Alternate commands:' : '',
      ...alternateCommands
    ].filter(Boolean).join('\n');
  }

  function shellQuoteForCommand(value: string) {
    return `'${value.replace(/'/g, "'\\''")}'`;
  }

  function sourceLspInstallCommand(language: SourcePreview['language'] | null | undefined = preview?.language) {
    switch (language) {
      case 'csharp':
        return 'dotnet tool install --global csharp-ls';
      case 'typescript':
      case 'tsx':
      case 'javascript':
      case 'jsx':
        return 'npm install -g typescript typescript-language-server';
      default:
        return '';
    }
  }

  function sourceLspRuntimeCommand(status: SourceLspStatus | null = sourceLspStatus) {
    if (!status?.command) return '';

    return [status.command, ...status.args.filter(Boolean)].join(' ');
  }

  function sourceLspStatusLabel() {
    if (sourceLspStatusLoading) return 'lsp...';
    if (!preview) return 'no file';
    if (!sourceSupportsLanguageIntelligence(preview.language)) return 'syntax only';
    if (sourceLspStatus?.available) return 'lsp';
    return 'index fallback';
  }

  function sourceLspStatusReportLines() {
    if (!preview) return ['No source file loaded'];

    const supported = sourceSupportsLanguageIntelligence(preview.language);
    const installCommand = sourceLspInstallCommand(preview.language);
    const runtimeCommand = sourceLspRuntimeCommand();
    const statusReason =
      sourceLspStatusError || sourceLspStatus?.reason || 'Using project index fallback';
    const lines = [
      `File: ${preview.relativePath}`,
      `Language: ${preview.language}`,
      `Mode: ${sourceLspStatusLabel()}`
    ];

    if (!supported) {
      lines.push('Reason: no configured language server for this language');
      return lines;
    }

    if (sourceLspStatusLoading) {
      lines.push('Status: checking language server');
    } else if (sourceLspStatus?.available) {
      lines.push(`Status: ${sourceLspStatus.serverName || sourceLspStatus.languageID} available`);
    } else {
      lines.push(`Status: ${statusReason}`);
      lines.push('Fallback: source index definitions and references remain available');
    }

    if (sourceLspStatus?.languageID) lines.push(`LSP language: ${sourceLspStatus.languageID}`);
    if (sourceLspStatus?.serverName) lines.push(`Server: ${sourceLspStatus.serverName}`);
    if (runtimeCommand) lines.push(`Command: ${runtimeCommand}`);
    if (!sourceLspStatus?.available && installCommand) lines.push(`Install: ${installCommand}`);

    return lines;
  }

  function sourceLspStatusTitle() {
    return sourceLspStatusReportLines().join('\n');
  }

  function sourceLspStatusReport() {
    return sourceLspStatusReportLines().join('\n');
  }

  function copySourceLspStatusReport() {
    return copyActivityCommand(sourceLspStatusReport(), 'Language server status copied');
  }

  function copySourceLspInstallCommand() {
    return copyActivityCommand(sourceLspInstallCommand(), 'Language server install command copied');
  }

  function agentSessionRowKey(session: AgentSession, index: number, scope: string) {
    return [
      scope,
      session.provider,
      session.id,
      session.projectPath ?? '',
      session.lastActivity ?? '',
      index
    ].join(':');
  }

  function runtimeContextUrl(context: RuntimeContext) {
    return `http://localhost:${context.port}`;
  }

  function normalizeBrowserDockUrl(value: string) {
    const trimmedValue = value.trim();
    if (!trimmedValue) return '';

    const withProtocol =
      /^https?:\/\//i.test(trimmedValue)
        ? trimmedValue
        : trimmedValue.startsWith(':')
          ? `http://localhost${trimmedValue}`
          : /^(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])(:\d+)?(\/.*)?$/i.test(trimmedValue)
            ? `http://${trimmedValue}`
            : trimmedValue;

    try {
      const parsedUrl = new URL(withProtocol);
      if (parsedUrl.protocol !== 'http:' && parsedUrl.protocol !== 'https:') return '';
      return parsedUrl.toString();
    } catch {
      return '';
    }
  }

  function persistBrowserDockUrl(url: string) {
    if (typeof window === 'undefined') return;
    if (url.trim()) {
      window.localStorage.setItem(browserDockUrlStorageKey, url);
    } else {
      window.localStorage.removeItem(browserDockUrlStorageKey);
    }
  }

  function loadStoredBrowserDockUrl() {
    if (typeof window === 'undefined') return '';
    return normalizeBrowserDockUrl(window.localStorage.getItem(browserDockUrlStorageKey) ?? '');
  }

  function setBrowserDockUrl(value: string) {
    const normalizedUrl = normalizeBrowserDockUrl(value);
    if (!normalizedUrl) {
      browserError = 'Enter an http or https URL';
      return false;
    }

    browserUrl = normalizedUrl;
    browserInputUrl = normalizedUrl;
    browserError = '';
    browserFrameKey += 1;
    persistBrowserDockUrl(normalizedUrl);
    return true;
  }

  function openBrowserDock(url = activeBrowserUrl) {
    showDockPanel('browser');
    if (url) {
      setBrowserDockUrl(url);
    }
  }

  function openRuntimeContextInBrowserDock(context: RuntimeContext) {
    openBrowserDock(runtimeContextUrl(context));
  }

  function submitBrowserUrl(event: SubmitEvent) {
    event.preventDefault();
    setBrowserDockUrl(browserInputUrl);
  }

  function reloadBrowserFrame() {
    browserFrameKey += 1;
  }

  function openBrowserUrlExternal() {
    if (!activeBrowserUrl || typeof window === 'undefined') return;
    window.open(activeBrowserUrl, '_blank', 'noopener,noreferrer');
  }

  function projectWorktreeSafety(worktree: ProjectWorktree) {
    return buildWorktreeSafetySummary(worktree, {
      primaryPath: selectedProject.path,
      activeSessionPaths: selectedProjectAgentSessionPaths
    });
  }

  function projectWorktreePrimaryAction(worktree: ProjectWorktree) {
    return worktreePrimaryAction(projectWorktreeSafety(worktree));
  }

  function formatProjectWorktreeSafetyStats(worktrees: ProjectWorktree[]) {
    if (projectWorktreesLoading) return 'scanning';
    if (projectWorktreeError) return 'needs refresh';
    if (worktrees.length === 0) return 'none';

    const counts = worktrees.reduce(
      (summary, worktree) => {
        const safety = projectWorktreeSafety(worktree);
        summary[safety.kind] += 1;
        if (safety.ageBucket === 'stale') summary.stale += 1;
        return summary;
      },
      { protected: 0, blocked: 0, ready: 0, review: 0, stale: 0 }
    );
    const parts = [
      counts.blocked ? `${counts.blocked} blocked` : '',
      counts.ready ? `${counts.ready} ready` : '',
      counts.review ? `${counts.review} review` : '',
      counts.stale ? `${counts.stale} stale` : '',
      counts.protected ? `${counts.protected} main` : ''
    ].filter(Boolean);
    return parts.join(' · ') || 'clean';
  }

  function projectWorktreeEligibilityKind(worktree: ProjectWorktree) {
    return projectWorktreeSafety(worktree).kind;
  }

  function projectWorktreeActivityLabel(worktree: ProjectWorktree) {
    return projectWorktreeSafety(worktree).activityLabel;
  }

  function sourceProjectNameForWorktree(worktree: ProjectWorktree) {
    const baseName = worktree.repo.trim() || selectedProject.name;
    const branchName = worktree.taskID?.trim() || worktree.branch.trim();
    return branchName ? `${baseName} ${branchName}` : baseName;
  }

  function openWorktreeInSourceBrowser(worktree: ProjectWorktree) {
    if (!worktree.path.trim()) return false;

    sourceActivityMode = 'files';
    sourceActivityFilter = '';
    persistSourceActivityMode(sourceActivityMode);
    fileActionStatus = `Opening ${worktree.branch} source tree`;
    return addCustomProjectRoot(sourceProjectNameForWorktree(worktree), worktree.path, false);
  }

  function copyWorktreeCleanupPlan(worktree: ProjectWorktree) {
    return copyActivityCommand(projectWorktreeSafety(worktree).cleanupPlan, 'Worktree cleanup plan copied');
  }

  function copyWorktreeAuditCommand(worktree: ProjectWorktree) {
    return copyActivityCommand(projectWorktreeSafety(worktree).auditCommand, 'Worktree audit command copied');
  }

  function copyWorktreeCleanupCommand(worktree: ProjectWorktree) {
    return copyActivityCommand(projectWorktreeSafety(worktree).cleanupCommand, 'Worktree remove command copied');
  }

  function copyWorktreeBackupCommand(worktree: ProjectWorktree) {
    return copyActivityCommand(projectWorktreeSafety(worktree).backupCommand, 'Worktree backup command copied');
  }

  function copyWorktreePrimaryAction(worktree: ProjectWorktree) {
    const action = projectWorktreePrimaryAction(worktree);
    return copyActivityCommand(action.command, action.clipboardMessage);
  }

  async function runWorktreePrimaryAction(worktree: ProjectWorktree) {
    const safety = projectWorktreeSafety(worktree);
    const action = worktreePrimaryAction(safety);
    if (action.kind !== 'cleanup') {
      await copyActivityCommand(action.command, action.clipboardMessage);
      return;
    }

    const busyKey = `worktree-primary:${worktree.path}`;
    fileActionBusy = busyKey;
    fileActionStatus = '';
    error = '';

    try {
      const result = await removeProjectWorktreeFromTauri(selectedProject.path, worktree.path);
      if (!result) {
        await copyTextToClipboard(action.command, 'Native remove unavailable; command copied');
        return;
      }

      projectWorktrees = result.worktrees;
      fileActionStatus = result.message;
      void loadGitRepositorySummaries(projects);
    } catch (removeError) {
      error = removeError instanceof Error ? removeError.message : 'Could not remove worktree';
    } finally {
      if (fileActionBusy === busyKey) fileActionBusy = '';
    }
  }

  function copyProjectWorktreeCleanupBrief() {
    return copyActivityCommand(projectWorktreeCleanupBrief.report, 'Worktree cleanup brief copied');
  }

  function copyProjectWorktreeCleanupScript() {
    const script = buildWorktreeCleanupScript(projectWorktrees, {
      primaryPath: selectedProject.path,
      activeSessionPaths: selectedProjectAgentSessionPaths
    });

    return copyActivityCommand(script, 'Guarded worktree cleanup script copied');
  }

  function copyAgentSessionResumePlan(session: AgentSession) {
    return copyActivityCommand(agentSessionResumePlan(session), 'Session resume plan copied');
  }

  function copyAgentSessionResumeShellCommand(session: AgentSession) {
    return copyActivityCommand(agentSessionResumeShellCommand(session), 'Shell resume command copied');
  }

  function terminalDockSummary() {
    const parts = [
      embeddedTerminalSession ? 'embedded live' : sourceTerminalApp,
      embeddedTerminalSessions.length
        ? `${embeddedTerminalSessions.length} embedded`
        : '',
      selectedProjectAgentSessions.length
        ? `${selectedProjectAgentSessions.length} ${selectedProjectAgentSessions.length === 1 ? 'agent' : 'agents'}`
        : '',
      selectedProjectRuntimeContexts.length ? `${selectedProjectRuntimeContexts.length} active` : '',
      projectWorktrees.length
        ? `${projectWorktrees.length} ${projectWorktrees.length === 1 ? 'worktree' : 'worktrees'}`
        : ''
    ].filter(Boolean);

    return parts.join(' · ') || sourceTerminalApp;
  }

  async function loadEmbeddedTerminalSessions() {
    if (embeddedTerminalSessionsLoading) return;

    embeddedTerminalSessionsLoading = true;
    embeddedTerminalSessionsError = '';

    try {
      const sessions = await listTerminalSessionsFromTauri();
      if (!sessions) {
        embeddedTerminalSessions = [];
        return;
      }

      embeddedTerminalSessions = sessions;
      if (
        embeddedTerminalSession &&
        !sessions.some((session) => session.sessionId === embeddedTerminalSession?.sessionId)
      ) {
        embeddedTerminalSession = null;
        embeddedTerminalStatus = 'Embedded terminal ended';
      }
    } catch (terminalError) {
      embeddedTerminalSessionsError =
        terminalError instanceof Error ? terminalError.message : 'Could not list embedded terminals';
    } finally {
      embeddedTerminalSessionsLoading = false;
    }
  }

  async function ensureEmbeddedTerminalRenderer() {
    if (embeddedTerminal || embeddedTerminalRendererLoading || !embeddedTerminalElement) return;

    embeddedTerminalRendererLoading = true;
    embeddedTerminalStatus = 'Loading embedded terminal';

    try {
      const [{ Terminal: XTerm }, { FitAddon }] = await Promise.all([
        import('@xterm/xterm'),
        import('@xterm/addon-fit')
      ]);

      if (!embeddedTerminalElement || embeddedTerminal) return;

      const terminal = new XTerm({
        convertEol: true,
        cursorBlink: true,
        fontFamily: sourcePreviewAppearance.fontFamily,
        fontSize: 12,
        lineHeight: 1.2,
        scrollback: 3000,
        theme: {
          background: '#101414',
          foreground: '#dce4e2',
          cursor: '#72e2cf',
          selectionBackground: '#274740',
          black: '#1d2423',
          red: '#ff6b7f',
          green: '#72e2cf',
          yellow: '#ffd166',
          blue: '#61afef',
          magenta: '#c678dd',
          cyan: '#56dce0',
          white: '#e8f0ee'
        }
      });
      const fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);
      terminal.open(embeddedTerminalElement);
      embeddedTerminalInputDisposable = terminal.onData((data) => {
        if (!embeddedTerminalSession) {
          embeddedTerminalStatus = 'Start a native terminal first';
          return;
        }

        void writeTerminalSessionFromTauri(embeddedTerminalSession.sessionId, data).catch(() => {
          embeddedTerminalError = 'Could not write to embedded terminal';
        });
      });
      embeddedTerminal = terminal;
      embeddedTerminalFitAddon = fitAddon;
      embeddedTerminalStatus = 'Embedded terminal ready';
      fitEmbeddedTerminal();
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not load embedded terminal';
      embeddedTerminalStatus = 'Embedded terminal unavailable';
    } finally {
      embeddedTerminalRendererLoading = false;
    }
  }

  async function startEmbeddedTerminalSession(cwd = selectedProject.path, startupCommand = '') {
    const root = cwd.trim();
    if (!root || embeddedTerminalStarting) return;
    const command = startupCommand.trim();

    embeddedTerminalStarting = true;
    embeddedTerminalError = '';
    embeddedTerminalStatus = command ? 'Starting embedded command' : 'Starting native terminal';

    try {
      await ensureEmbeddedTerminalRenderer();
      if (!embeddedTerminal) {
        embeddedTerminalStatus = 'Embedded terminal unavailable';
        return;
      }

      fitEmbeddedTerminal();
      const session = await startTerminalSessionFromTauri({
        cwd: root,
        cols: embeddedTerminal.cols || 96,
        rows: embeddedTerminal.rows || 24
      });

      if (!session) {
        embeddedTerminalError = 'Embedded terminal sessions run inside the Tauri app.';
        embeddedTerminalStatus = 'Browser preview cannot start a native PTY';
        embeddedTerminal.writeln('\r\nEmbedded terminal is available in the Tauri app.');
        embeddedTerminal.writeln(`Use Project to open ${sourceTerminalApp} from browser preview.\r\n`);
        return;
      }

      embeddedTerminalSession = session;
      embeddedTerminalSessions = upsertEmbeddedTerminalSession(embeddedTerminalSessions, session);
      embeddedTerminalStatus = command ? 'Running embedded command' : 'Native terminal running';
      embeddedTerminal.reset();
      embeddedTerminal.focus();
      window.setTimeout(fitEmbeddedTerminal, 0);
      if (command) {
        await writeTerminalSessionFromTauri(session.sessionId, `${command}\r`);
      }
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not start embedded terminal';
      embeddedTerminalStatus = 'Embedded terminal failed';
    } finally {
      embeddedTerminalStarting = false;
    }
  }

  async function openPathEmbeddedTerminal(path: string) {
    const root = path.trim();
    if (!root) return;

    showDockPanel('terminal');
    await tick();

    if (
      embeddedTerminalSession &&
      normalizeProjectPath(embeddedTerminalSession.cwd) === normalizeProjectPath(root)
    ) {
      embeddedTerminalStatus = 'Embedded terminal already in this path';
      embeddedTerminal?.focus();
      return;
    }

    await startEmbeddedTerminalSession(root);
  }

  async function attachEmbeddedTerminalSession(session: TerminalSessionInfo) {
    embeddedTerminalError = '';
    embeddedTerminalStatus = 'Attaching embedded terminal';

    try {
      await ensureEmbeddedTerminalRenderer();
      if (!embeddedTerminal) {
        embeddedTerminalStatus = 'Embedded terminal unavailable';
        return;
      }

      embeddedTerminalSession = session;
      embeddedTerminalSessions = upsertEmbeddedTerminalSession(embeddedTerminalSessions, session);
      embeddedTerminal.reset();
      const scrollback = await readTerminalSessionScrollbackFromTauri(session.sessionId);
      if (scrollback) {
        embeddedTerminal.write(scrollback);
      } else {
        embeddedTerminal.writeln(`Attached to ${embeddedTerminalSessionTitle(session)}`);
        embeddedTerminal.writeln('Waiting for terminal output.\r\n');
      }
      embeddedTerminalStatus = 'Native terminal attached';
      embeddedTerminal.focus();
      window.setTimeout(fitEmbeddedTerminal, 0);
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not attach embedded terminal';
      embeddedTerminalStatus = 'Embedded terminal attach failed';
    }
  }

  async function closeEmbeddedTerminalSession() {
    const session = embeddedTerminalSession;
    if (!session) return;

    embeddedTerminalSession = null;
    embeddedTerminalStatus = 'Stopping embedded terminal';

    try {
      await closeTerminalSessionFromTauri(session.sessionId);
      embeddedTerminalSessions = embeddedTerminalSessions.filter(
        (existing) => existing.sessionId !== session.sessionId
      );
      embeddedTerminalStatus = 'Embedded terminal stopped';
      embeddedTerminal?.writeln('\r\n[terminal closed]');
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not stop embedded terminal';
      embeddedTerminalStatus = 'Embedded terminal close failed';
    }
  }

  async function closeListedEmbeddedTerminalSession(session: TerminalSessionInfo) {
    embeddedTerminalSessionsError = '';

    if (embeddedTerminalSession?.sessionId === session.sessionId) {
      await closeEmbeddedTerminalSession();
      return;
    }

    try {
      await closeTerminalSessionFromTauri(session.sessionId);
      embeddedTerminalSessions = embeddedTerminalSessions.filter(
        (existing) => existing.sessionId !== session.sessionId
      );
    } catch (terminalError) {
      embeddedTerminalSessionsError =
        terminalError instanceof Error ? terminalError.message : 'Could not close embedded terminal';
    }
  }

  function fitEmbeddedTerminal() {
    if (!embeddedTerminal || !embeddedTerminalFitAddon || !embeddedTerminalElement) return;

    try {
      embeddedTerminalFitAddon.fit();
      if (embeddedTerminalSession) {
        void resizeTerminalSessionFromTauri(
          embeddedTerminalSession.sessionId,
          embeddedTerminal.cols,
          embeddedTerminal.rows
        ).catch(() => {
          embeddedTerminalError = 'Could not resize embedded terminal';
        });
      }
    } catch {
      embeddedTerminalStatus = 'Terminal fit pending';
    }
  }

  function handleTerminalOutput(payload: TerminalOutputPayload) {
    if (!embeddedTerminalSession || payload.sessionId !== embeddedTerminalSession.sessionId) return;

    if (payload.data) {
      embeddedTerminal?.write(payload.data);
    }

    if (payload.terminated) {
      const exitLabel =
        payload.exitCode !== null
          ? `exit ${payload.exitCode}`
          : payload.signal
            ? `signal ${payload.signal}`
            : 'terminated';
      embeddedTerminal?.writeln(`\r\n[process ${exitLabel}]`);
      embeddedTerminalSessions = embeddedTerminalSessions.filter(
        (existing) => existing.sessionId !== payload.sessionId
      );
      embeddedTerminalSession = null;
      embeddedTerminalStatus = `Embedded terminal ${exitLabel}`;
    }
  }

  function disposeEmbeddedTerminal() {
    const session = embeddedTerminalSession;
    embeddedTerminalSession = null;

    if (session) {
      void closeTerminalSessionFromTauri(session.sessionId);
    }

    embeddedTerminalInputDisposable?.dispose();
    embeddedTerminalInputDisposable = null;
    embeddedTerminalFitAddon = null;
    embeddedTerminal?.dispose();
    embeddedTerminal = null;
  }

  function embeddedTerminalStatusLabel(session = embeddedTerminalSession) {
    if (!session) return embeddedTerminalStatus;

    return `${embeddedTerminalSessionTitle(session)} · ${session.cols}x${session.rows}${session.pid ? ` · pid ${session.pid}` : ''}`;
  }

  function embeddedTerminalSessionTitle(session: TerminalSessionInfo) {
    const segments = session.cwd.split('/').filter(Boolean);
    const cwdName = segments[segments.length - 1] ?? session.cwd;
    return cwdName || session.shell || session.sessionId;
  }

  function upsertEmbeddedTerminalSession(
    sessions: TerminalSessionInfo[],
    session: TerminalSessionInfo
  ): TerminalSessionInfo[] {
    const next = sessions.filter((existing) => existing.sessionId !== session.sessionId);
    next.unshift(session);
    return next;
  }

  function gitStatusForSourceRecord(record: SourceRecord | SourceOpenTab | null): ProjectGitFileStatus | null {
    return record ? gitStatusByRelativePath.get(record.relativePath) ?? null : null;
  }

  function selectedSourceGitBadge() {
    if (selectedSourceGitDiffLoading) return '...';
    if (selectedRecordGitStatus?.badge) return selectedRecordGitStatus.badge;
    if (selectedSourceGitDiff?.status && selectedSourceGitDiff.status !== 'clean') return 'M';
    return '0';
  }

  function formatSelectedSourceGitSummary(
    status: ProjectGitFileStatus | null,
    diff: SourceGitDiff | null,
    loadingDiff: boolean,
    diffError: string
  ) {
    if (loadingDiff) return 'Loading selected file diff';
    if (diffError) return 'Selected file Git diff';
    if (diff?.isBinary) return `${diff.relativePath} · ${diff.status} · binary file`;
    if (diff?.diff) {
      const lineCount = diff.diff.split('\n').length;
      return `${diff.relativePath} · ${diff.status} · ${lineCount} diff lines`;
    }
    if (status) return `${status.relativePath} · ${status.status}`;
    return 'No Git changes for selected file';
  }

  function clearSelectedSourceGitDiff() {
    selectedSourceGitDiff = null;
    selectedSourceGitDiffLoading = false;
    selectedSourceGitDiffError = '';
  }

  async function loadSelectedSourceGitDiff(
    record: SourceRecord | null = selectedRecord,
    project: ProjectRoot = selectedProject
  ) {
    if (!record || selectedRecord?.path !== record.path) {
      clearSelectedSourceGitDiff();
      return;
    }

    const expectedPath = record.path;
    const expectedProjectPath = project.path;
    selectedSourceGitDiff = null;
    selectedSourceGitDiffError = '';
    selectedSourceGitDiffLoading = true;

    try {
      const diff = await readSourceGitDiffFromTauri(expectedProjectPath, expectedPath);
      if (selectedRecord?.path !== expectedPath || selectedProject.path !== expectedProjectPath) return;

      selectedSourceGitDiff = diff;
      if (!diff) selectedSourceGitDiffError = 'Native Git diff unavailable';
    } catch (gitDiffError) {
      if (selectedRecord?.path !== expectedPath || selectedProject.path !== expectedProjectPath) return;

      selectedSourceGitDiffError =
        gitDiffError instanceof Error ? gitDiffError.message : 'Could not read Git diff';
    } finally {
      if (selectedRecord?.path === expectedPath && selectedProject.path === expectedProjectPath) {
        selectedSourceGitDiffLoading = false;
      }
    }
  }

  async function loadSourceLspStatus(
    nextPreview: SourcePreview | null = preview,
    project: ProjectRoot = selectedProject
  ) {
    sourceLspStatus = null;
    sourceLspStatusError = '';

    if (!nextPreview || !sourceSupportsLanguageIntelligence(nextPreview.language)) {
      sourceLspStatusLoading = false;
      return;
    }

    const expectedPath = nextPreview.path;
    const expectedProjectPath = project.path;
    sourceLspStatusLoading = true;

    try {
      const status = await readSourceLspStatusFromTauri(expectedProjectPath, nextPreview.language);
      if (preview?.path !== expectedPath || selectedProject.path !== expectedProjectPath) return;

      sourceLspStatus = status;
      if (!status) {
        sourceLspStatusError = 'Browser preview LSP unavailable';
      } else if (!status.available) {
        sourceLspStatusError = status.reason ?? 'Language server unavailable';
      }
    } catch (lspStatusError) {
      if (preview?.path !== expectedPath || selectedProject.path !== expectedProjectPath) return;

      sourceLspStatusError =
        lspStatusError instanceof Error ? lspStatusError.message : 'Could not read LSP status';
    } finally {
      if (preview?.path === expectedPath && selectedProject.path === expectedProjectPath) {
        sourceLspStatusLoading = false;
      }
    }
  }

  async function runGlobalSourceSearch() {
    const normalizedQuery = sourceSearchQuery.trim();
    sourceSearchError = '';

    if (!normalizedQuery) {
      sourceSearchResults = [];
      return;
    }

    sourceSearchLoading = true;
    try {
      const nativeResults = await searchSourceFilesFromTauri(
        records,
        normalizedQuery,
        maxSourceSearchResults
      );
      sourceSearchResults =
        nativeResults ??
        findSourceSearchMatches(
          records.map((record) => demoPreviewFor(record)),
          normalizedQuery,
          maxSourceSearchResults
        );
      sourceSearchError = nativeResults ? '' : 'Browser preview results';
    } catch (searchError) {
      sourceSearchResults = findSourceSearchMatches(
        records.map((record) => demoPreviewFor(record)),
        normalizedQuery,
        maxSourceSearchResults
      );
      sourceSearchError =
        searchError instanceof Error ? searchError.message : 'Could not search source files';
    } finally {
      sourceSearchLoading = false;
    }
  }

  function formatSourceSearchSummary(resultCount: number, searching: boolean, searchError: string) {
    if (searching) return 'Searching';
    if (searchError) return searchError;
    if (sourceSearchQuery.trim().length === 0) return '';
    return `${resultCount} ${resultCount === 1 ? 'result' : 'results'}`;
  }

  function handleGlobalSourceSearchSubmit(event: SubmitEvent) {
    event.preventDefault();
    void runGlobalSourceSearch();
  }

  async function selectSourceSearchResult(result: SourceSearchMatch) {
    const record = records.find((sourceRecord) => sourceRecord.path === result.path) ?? result;
    await selectRecord(record, result.line);
  }

  function clearSourceSearchResults() {
    sourceSearchResults = [];
    sourceSearchError = '';
    sourceSearchLoading = false;
  }

  async function runSourceDefinitionLookup(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    sourceDefinitionQuery = normalizedSymbolName;
    sourceDefinitionError = '';

    if (!normalizedSymbolName) {
      sourceDefinitionTargets = [];
      fileActionStatus = 'No symbol under cursor';
      return [];
    }

    sourceDefinitionLoading = true;
    fileActionStatus = `Looking up ${normalizedSymbolName}`;
    try {
      const lspTargets = preview
        ? await findSourceLspDefinitionsFromTauri(
            { ...preview, content: selectedSourceDraftContent },
            {
              root: selectedProject.path,
              line: request.line,
              column: request.column,
              limit: maxSourceDefinitionResults
            }
          ).catch(() => null)
        : null;
      const nativeTargets = lspTargets?.length
        ? lspTargets
        : await findSourceDefinitionsFromTauri(
            records,
            normalizedSymbolName,
            maxSourceDefinitionResults
          );
      const nextTargets =
        nativeTargets ??
        findSourceDefinitionTargets(
          records.map((record) => demoPreviewFor(record)),
          normalizedSymbolName,
          maxSourceDefinitionResults
        );

      sourceDefinitionTargets = nextTargets;
      sourceDefinitionError = lspTargets?.length
        ? ''
        : nativeTargets
          ? ''
          : 'Browser preview definitions';

      if (nextTargets.length === 0) {
        fileActionStatus = `No definition for ${normalizedSymbolName}`;
        return nextTargets;
      }

      fileActionStatus = `${nextTargets.length} ${nextTargets.length === 1 ? 'definition' : 'definitions'} for ${normalizedSymbolName}`;
      return nextTargets;
    } catch (definitionError) {
      sourceDefinitionTargets = findSourceDefinitionTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceDefinitionResults
      );
      sourceDefinitionError =
        definitionError instanceof Error ? definitionError.message : 'Could not find definition';

      return sourceDefinitionTargets;
    } finally {
      sourceDefinitionLoading = false;
    }
  }

  function formatSourceDefinitionSummary(
    targetCount: number,
    loadingDefinitions: boolean,
    definitionError: string,
    symbolName: string
  ) {
    if (loadingDefinitions) return 'Finding definition';
    if (definitionError) return definitionError;
    if (!symbolName.trim()) return '';
    return `${targetCount} ${targetCount === 1 ? 'definition' : 'definitions'} for ${symbolName}`;
  }

  async function selectSourceDefinitionTarget(target: SourceDefinitionTarget) {
    const record = records.find((sourceRecord) => sourceRecord.path === target.path) ?? target;
    await selectRecord(record, target.line);
  }

  function clearSourceDefinitionTargets() {
    sourceDefinitionTargets = [];
    sourceDefinitionError = '';
    sourceDefinitionLoading = false;
    sourceDefinitionQuery = '';
  }

  async function runSourceReferenceLookup(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    sourceReferenceQuery = normalizedSymbolName;
    sourceReferenceError = '';

    if (!normalizedSymbolName) {
      sourceReferenceTargets = [];
      fileActionStatus = 'No symbol under cursor';
      return [];
    }

    sourceReferenceLoading = true;
    fileActionStatus = `Finding references for ${normalizedSymbolName}`;
    try {
      const lspTargets = preview
        ? await findSourceLspReferencesFromTauri(
            { ...preview, content: selectedSourceDraftContent },
            {
              root: selectedProject.path,
              line: request.line,
              column: request.column,
              limit: maxSourceSearchResults
            }
          ).catch(() => null)
        : null;
      const nativeTargets = lspTargets?.length
        ? lspTargets
        : await findSourceReferencesFromTauri(
            records,
            normalizedSymbolName,
            maxSourceSearchResults
          );
      sourceReferenceTargets =
        lspTargets?.length
          ? lspTargets
          : nativeTargets ??
            findSourceReferenceTargets(
              records.map((record) => demoPreviewFor(record)),
              normalizedSymbolName,
              maxSourceSearchResults
            );
      sourceReferenceError = lspTargets?.length
        ? ''
        : nativeTargets
          ? ''
          : 'Browser preview references';
      fileActionStatus = `${sourceReferenceTargets.length} references for ${normalizedSymbolName}`;
      return sourceReferenceTargets;
    } catch (referenceError) {
      sourceReferenceTargets = findSourceReferenceTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceSearchResults
      );
      sourceReferenceError =
        referenceError instanceof Error ? referenceError.message : 'Could not find references';
      return sourceReferenceTargets;
    } finally {
      sourceReferenceLoading = false;
    }
  }

  function formatSourceReferenceSummary(
    targetCount: number,
    loadingReferences: boolean,
    referenceError: string,
    symbolName: string
  ) {
    if (loadingReferences) return 'Finding references';
    if (referenceError) return referenceError;
    if (!symbolName.trim()) return '';
    return `${targetCount} ${targetCount === 1 ? 'reference' : 'references'} for ${symbolName}`;
  }

  async function selectSourceReferenceTarget(target: SourceReferenceTarget) {
    const record = records.find((sourceRecord) => sourceRecord.path === target.path) ?? target;
    await selectRecord(record, target.line);
  }

  function clearSourceReferenceTargets() {
    sourceReferenceTargets = [];
    sourceReferenceError = '';
    sourceReferenceLoading = false;
    sourceReferenceQuery = '';
  }

  async function runSourceImplementationLookup(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    sourceImplementationQuery = normalizedSymbolName;
    sourceImplementationError = '';

    if (!normalizedSymbolName) {
      sourceImplementationTargets = [];
      fileActionStatus = 'No symbol under cursor';
      return [];
    }

    if (!preview || !sourceIntelligenceAvailable) {
      sourceImplementationTargets = [];
      sourceImplementationError = 'Language server unavailable';
      fileActionStatus = `No implementation lookup for ${normalizedSymbolName}`;
      return [];
    }

    sourceImplementationLoading = true;
    fileActionStatus = `Finding implementations for ${normalizedSymbolName}`;
    try {
      const lspTargets =
        (await findSourceLspImplementationsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.line,
            column: request.column,
            limit: maxSourceDefinitionResults
          }
        )) ?? [];

      sourceImplementationTargets = lspTargets;
      sourceImplementationError = '';
      fileActionStatus = `${lspTargets.length} ${lspTargets.length === 1 ? 'implementation' : 'implementations'} for ${normalizedSymbolName}`;
      return lspTargets;
    } catch (implementationError) {
      sourceImplementationTargets = [];
      sourceImplementationError =
        implementationError instanceof Error
          ? implementationError.message
          : 'Could not find implementations';
      return sourceImplementationTargets;
    } finally {
      sourceImplementationLoading = false;
    }
  }

  function formatSourceImplementationSummary(
    targetCount: number,
    loadingImplementations: boolean,
    implementationError: string,
    symbolName: string
  ) {
    if (loadingImplementations) return 'Finding implementations';
    if (implementationError) return implementationError;
    if (!symbolName.trim()) return '';
    return `${targetCount} ${targetCount === 1 ? 'implementation' : 'implementations'} for ${symbolName}`;
  }

  async function selectSourceImplementationTarget(target: SourceDefinitionTarget) {
    const record = records.find((sourceRecord) => sourceRecord.path === target.path) ?? target;
    await selectRecord(record, target.line);
  }

  function clearSourceImplementationTargets() {
    sourceImplementationTargets = [];
    sourceImplementationError = '';
    sourceImplementationLoading = false;
    sourceImplementationQuery = '';
  }

  async function runSourceTypeDefinitionLookup(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    sourceTypeDefinitionQuery = normalizedSymbolName;
    sourceTypeDefinitionError = '';

    if (!normalizedSymbolName) {
      sourceTypeDefinitionTargets = [];
      fileActionStatus = 'No symbol under cursor';
      return [];
    }

    if (!preview || !sourceIntelligenceAvailable) {
      sourceTypeDefinitionTargets = [];
      sourceTypeDefinitionError = 'Language server unavailable';
      fileActionStatus = `No type definition lookup for ${normalizedSymbolName}`;
      return [];
    }

    sourceTypeDefinitionLoading = true;
    fileActionStatus = `Finding type definition for ${normalizedSymbolName}`;
    try {
      const lspTargets =
        (await findSourceLspTypeDefinitionsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.line,
            column: request.column,
            limit: maxSourceDefinitionResults
          }
        )) ?? [];

      sourceTypeDefinitionTargets = lspTargets;
      sourceTypeDefinitionError = '';
      fileActionStatus = `${lspTargets.length} ${lspTargets.length === 1 ? 'type definition' : 'type definitions'} for ${normalizedSymbolName}`;
      return lspTargets;
    } catch (typeDefinitionError) {
      sourceTypeDefinitionTargets = [];
      sourceTypeDefinitionError =
        typeDefinitionError instanceof Error
          ? typeDefinitionError.message
          : 'Could not find type definition';
      return sourceTypeDefinitionTargets;
    } finally {
      sourceTypeDefinitionLoading = false;
    }
  }

  function formatSourceTypeDefinitionSummary(
    targetCount: number,
    loadingTypeDefinitions: boolean,
    typeDefinitionError: string,
    symbolName: string
  ) {
    if (loadingTypeDefinitions) return 'Finding type definition';
    if (typeDefinitionError) return typeDefinitionError;
    if (!symbolName.trim()) return '';
    return `${targetCount} ${targetCount === 1 ? 'type definition' : 'type definitions'} for ${symbolName}`;
  }

  async function selectSourceTypeDefinitionTarget(target: SourceDefinitionTarget) {
    const record = records.find((sourceRecord) => sourceRecord.path === target.path) ?? target;
    await selectRecord(record, target.line);
  }

  function clearSourceTypeDefinitionTargets() {
    sourceTypeDefinitionTargets = [];
    sourceTypeDefinitionError = '';
    sourceTypeDefinitionLoading = false;
    sourceTypeDefinitionQuery = '';
  }

  function setBackgroundProjectIndexing(projectID: string, indexing: boolean) {
    const nextProjectIDs = new Set(backgroundIndexingProjectIDs);
    if (indexing) {
      nextProjectIDs.add(projectID);
    } else {
      nextProjectIDs.delete(projectID);
    }
    backgroundIndexingProjectIDs = nextProjectIDs;
  }

  function clearBackgroundIndexError(projectID: string) {
    if (!(projectID in backgroundIndexErrorByProject)) return;

    const nextErrors = { ...backgroundIndexErrorByProject };
    delete nextErrors[projectID];
    backgroundIndexErrorByProject = nextErrors;
  }

  function applySourceRecords(
    nextRecords: SourceRecord[],
    preferredPath: string | null | undefined,
    nextRuntime: string,
    truncated = false,
    stats: SourceScanStats | null = null
  ): SourceRecord | null {
    records = nextRecords;
    runtime = nextRuntime;
    scanLimitReached = truncated;
    sourceScanStats = stats;

    const nextSelection = selectPreferredSourceRecord(
      nextRecords,
      preferredPath,
      selectedRecord?.path
    );
    selectedRecord = nextSelection;
    selectedSourceLine = null;
    preview = nextSelection ? previewFromContent(nextSelection, '') : null;
    expandedFolderIds = nextSelection ? new Set(folderIdsForSourceRecord(nextSelection)) : new Set();
    clearSourceSearchResults();
    clearSourceDefinitionTargets();
    clearSourceReferenceTargets();
    clearSourceImplementationTargets();
    clearSourceTypeDefinitionTargets();
    if (nextSelection) requestSourceTreeReveal(nextSelection);
    return nextSelection;
  }

  async function loadRecord(record: SourceRecord, expectedScanGeneration: number | null = null) {
    loading = true;
    error = '';
    fileActionStatus = '';
    resetSourceIntelligence();
    clearSelectedSourceGitDiff();

    try {
      const tauriPreview = await readSourceFromTauri(record);
      if (expectedScanGeneration !== null && expectedScanGeneration !== scanGeneration) return;

      runtime = tauriPreview ? 'tauri file read' : 'browser preview';
      const nextPreview = tauriPreview ?? demoPreviewFor(record);
      preview = nextPreview;
      syncSourcePreviewContent(nextPreview);
      void loadSourceLspStatus(nextPreview);
      void loadSourceLspDiagnostics(nextPreview);
      void loadSourceLspSymbols(nextPreview);
    } catch (previewError) {
      if (expectedScanGeneration !== null && expectedScanGeneration !== scanGeneration) return;

      runtime = 'browser preview';
      error = previewError instanceof Error ? previewError.message : 'Could not read source file';
      const nextPreview = demoPreviewFor(record);
      preview = nextPreview;
      syncSourcePreviewContent(nextPreview);
      void loadSourceLspStatus(nextPreview);
      void loadSourceLspDiagnostics(nextPreview);
      void loadSourceLspSymbols(nextPreview);
    } finally {
      if (expectedScanGeneration === null || expectedScanGeneration === scanGeneration) {
        loading = false;
        void loadSelectedSourceGitDiff(record);
      }
    }
  }

  function currentSourceNavigationLocation() {
    return selectedRecord ? sourceNavigationLocationForRecord(selectedRecord, selectedSourceLine) : null;
  }

  function recordSourceNavigation(nextLocation: SourceNavigationLocation | null) {
    const nextBackStack = pushSourceNavigationHistory(
      sourceNavigationBackStack,
      currentSourceNavigationLocation(),
      nextLocation,
      maxSourceNavigationHistoryEntries
    );

    if (nextBackStack !== sourceNavigationBackStack) {
      sourceNavigationBackStack = nextBackStack;
      sourceNavigationForwardStack = [];
    }
  }

  async function selectSourceNavigationLocation(location: SourceNavigationLocation) {
    const record = records.find((sourceRecord) => sourceRecord.path === location.path) ??
      sourceRecordFromRestoredPath(selectedProject, location.path);
    await selectRecord(record, location.line, false);
  }

  async function navigateSourceBack() {
    const step = navigateSourceHistoryBack(
      sourceNavigationBackStack,
      sourceNavigationForwardStack,
      currentSourceNavigationLocation()
    );

    sourceNavigationBackStack = step.backStack;
    sourceNavigationForwardStack = step.forwardStack;
    if (step.target) await selectSourceNavigationLocation(step.target);
  }

  async function navigateSourceForward() {
    const step = navigateSourceHistoryForward(
      sourceNavigationBackStack,
      sourceNavigationForwardStack,
      currentSourceNavigationLocation()
    );

    sourceNavigationBackStack = step.backStack;
    sourceNavigationForwardStack = step.forwardStack;
    if (step.target) await selectSourceNavigationLocation(step.target);
  }

  async function selectRecord(
    record: SourceRecord,
    targetLine: number | null = null,
    recordNavigation = true
  ) {
    if (recordNavigation) {
      recordSourceNavigation(sourceNavigationLocationForRecord(record, targetLine));
    }

    selectedRecord = record;
    selectedSourceLine = targetLine;
    if (targetLine) selectedSourceLineRequestId += 1;
    expandFoldersForRecord(record);
    requestSourceTreeReveal(record);
    trackSelectedSourceRecord(record, selectedProject);
    await loadRecord(record);
  }

  async function selectRecentRecord(recentRecord: SourceRecentRecord) {
    const record = records.find((sourceRecord) => sourceRecord.path === recentRecord.path) ?? recentRecord;
    await selectRecord(record);
  }

  async function selectOpenTab(tab: SourceOpenTab) {
    const record = records.find((sourceRecord) => sourceRecord.path === tab.path) ?? tab;
    await selectRecord(record);
  }

  async function selectAdjacentDirtySourceFile(direction: 1 | -1) {
    const dirtyRecords = dirtyProjectSourceRecords;
    if (dirtyRecords.length === 0) {
      fileActionStatus = 'No dirty files';
      return;
    }

    const selectedPath = selectedRecord?.path ?? '';
    const currentIndex = dirtyRecords.findIndex((record) => record.path === selectedPath);
    const nextIndex =
      currentIndex === -1
        ? direction > 0
          ? 0
          : dirtyRecords.length - 1
        : (currentIndex + direction + dirtyRecords.length) % dirtyRecords.length;
    const nextRecord = dirtyRecords[nextIndex];
    await selectRecord(nextRecord);
    fileActionStatus = `Dirty file ${nextIndex + 1} of ${dirtyRecords.length}`;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && commandPaletteVisible) {
      event.preventDefault();
      closeCommandPalette();
      return;
    }

    if (event.key === 'Escape' && viewMenuOpen) {
      event.preventDefault();
      closeViewMenu();
      return;
    }

    if (event.key === 'Escape' && editorActionMenuOpen) {
      event.preventDefault();
      closeEditorActionMenu();
      return;
    }

    if (
      (event.metaKey || event.ctrlKey) &&
      (event.key.toLowerCase() === 'k' || (event.shiftKey && event.key.toLowerCase() === 'p'))
    ) {
      event.preventDefault();
      openCommandPalette();
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
      event.preventDefault();
      if (selectedSourceDirty && fileActionBusy !== 'save') {
        void saveSelectedSourceFile();
      }
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'p') {
      event.preventDefault();
      openQuickOpen();
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      focusGlobalSourceSearch();
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.key === '[') {
      event.preventDefault();
      void navigateSourceBack();
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.key === ']') {
      event.preventDefault();
      void navigateSourceForward();
      return;
    }

    if (event.key === 'F8') {
      event.preventDefault();
      if (event.shiftKey) {
        selectPreviousSourceDiagnostic();
        return;
      }

      selectNextSourceDiagnostic();
    }
  }

  function toggleEditorActionMenu() {
    editorActionMenuOpen = !editorActionMenuOpen;
  }

  function closeEditorActionMenu() {
    editorActionMenuOpen = false;
  }

  function toggleViewMenu() {
    viewMenuOpen = !viewMenuOpen;
    closeEditorActionMenu();
  }

  function closeViewMenu() {
    viewMenuOpen = false;
  }

  function openCommandPalette() {
    commandPaletteVisible = true;
    commandPaletteQuery = '';
    commandPaletteIndex = 0;
    closeViewMenu();
    closeEditorActionMenu();
    window.setTimeout(() => commandPaletteInput?.focus(), 0);
  }

  function closeCommandPalette() {
    commandPaletteVisible = false;
    commandPaletteQuery = '';
    commandPaletteIndex = 0;
  }

  function commandPaletteTextMatches(
    filter: string,
    ...values: Array<string | number | boolean | null | undefined>
  ) {
    return textMatchesSearchTokens(filter, ...values);
  }

  function handleCommandPaletteKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeCommandPalette();
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      commandPaletteIndex = Math.min(commandPaletteIndex + 1, Math.max(0, commandPaletteResults.length - 1));
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      commandPaletteIndex = Math.max(commandPaletteIndex - 1, 0);
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      const selectedCommand = commandPaletteResults[commandPaletteIndex];
      if (selectedCommand && !selectedCommand.disabled) {
        void runCommandPaletteItem(selectedCommand);
      }
    }
  }

  async function runCommandPaletteItem(item: SourceCommandPaletteItem) {
    if (item.disabled) return;

    closeCommandPalette();
    await item.perform();
  }

  function openQuickOpen() {
    quickOpenVisible = true;
    quickOpenQuery = '';
    quickOpenIndex = 0;
    window.setTimeout(() => quickOpenInput?.focus(), 0);
  }

  function openWorkspaceSymbolQuickOpen() {
    quickOpenVisible = true;
    quickOpenQuery = '#';
    quickOpenIndex = 0;
    closeCommandPalette();
    closeViewMenu();
    closeEditorActionMenu();
    window.setTimeout(() => {
      quickOpenInput?.focus();
      const cursor = quickOpenQuery.length;
      quickOpenInput?.setSelectionRange(cursor, cursor);
    }, 0);
  }

  function openCurrentFileGoToLine() {
    if (!selectedRecord) return;

    quickOpenVisible = true;
    quickOpenQuery = `${selectedRecord.relativePath}:`;
    quickOpenIndex = 0;
    window.setTimeout(() => {
      quickOpenInput?.focus();
      const cursor = quickOpenQuery.length;
      quickOpenInput?.setSelectionRange(cursor, cursor);
    }, 0);
  }

  function focusGlobalSourceSearch() {
    selectSourceActivityMode('files');
    closeCommandPalette();
    closeViewMenu();
    closeEditorActionMenu();
    window.setTimeout(() => sourceSearchInput?.focus(), 0);
  }

  function closeQuickOpen() {
    quickOpenVisible = false;
    quickOpenQuery = '';
    quickOpenIndex = 0;
    workspaceSymbolRequestID += 1;
    workspaceSymbolResults = [];
    workspaceSymbolError = '';
    workspaceSymbolLoading = false;
  }

  async function chooseQuickOpenRecord(record: SourceRecord) {
    await selectRecord(record, parsedQuickOpenQuery.targetLine);
    closeQuickOpen();
  }

  async function chooseQuickOpenWorkspaceSymbol(symbol: SourceWorkspaceSymbol) {
    await selectRecord(symbol, symbol.line);
    closeQuickOpen();
  }

  function handleQuickOpenKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeQuickOpen();
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      quickOpenIndex = Math.min(quickOpenIndex + 1, Math.max(0, quickOpenActiveResultCount - 1));
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      quickOpenIndex = Math.max(quickOpenIndex - 1, 0);
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      if (quickOpenWorkspaceSymbolMode) {
        const selectedWorkspaceSymbol = workspaceSymbolResults[quickOpenIndex];
        if (selectedWorkspaceSymbol) {
          void chooseQuickOpenWorkspaceSymbol(selectedWorkspaceSymbol);
        }
        return;
      }

      const selectedQuickOpenRecord = quickOpenResults[quickOpenIndex];
      if (selectedQuickOpenRecord) {
        void chooseQuickOpenRecord(selectedQuickOpenRecord);
      }
    }
  }

  async function closeSourceTab(tab: SourceOpenTab, event: MouseEvent) {
    event.stopPropagation();

    const closeResult = closeOpenSourceTab(projectOpenSourceTabs, tab.path, selectedRecord?.path);
    await applySourceTabCloseResult(closeResult, isSourcePathDirty(tab.path) ? 'Closed tab; draft retained' : 'Closed tab');
  }

  async function closeSelectedSourceTab() {
    if (!selectedRecord) return;

    const selectedTab = projectOpenSourceTabs.find((tab) => tab.path === selectedRecord?.path);
    if (!selectedTab) return;

    const closeResult = closeOpenSourceTab(projectOpenSourceTabs, selectedTab.path, selectedRecord.path);
    await applySourceTabCloseResult(
      closeResult,
      isSourcePathDirty(selectedTab.path) ? 'Closed current tab; draft retained' : 'Closed current tab'
    );
  }

  async function closeOtherCleanSourceTabs() {
    if (!selectedRecord) return;

    const closeResult = closeOtherCleanOpenSourceTabs(
      projectOpenSourceTabs,
      selectedRecord.path,
      dirtyProjectSourcePathSet()
    );
    await applySourceTabCloseResult(
      closeResult,
      closeResult.closedCount === 0
        ? 'No other clean tabs to close'
        : `Closed ${closeResult.closedCount} clean ${closeResult.closedCount === 1 ? 'tab' : 'tabs'}`
    );
  }

  async function closeAllCleanSourceTabs() {
    const closeResult = closeAllCleanOpenSourceTabs(
      projectOpenSourceTabs,
      selectedRecord?.path,
      dirtyProjectSourcePathSet()
    );
    const retainedDraftNote =
      closeResult.retainedDirtyCount > 0
        ? `; kept ${closeResult.retainedDirtyCount} dirty ${closeResult.retainedDirtyCount === 1 ? 'tab' : 'tabs'}`
        : '';
    await applySourceTabCloseResult(
      closeResult,
      closeResult.closedCount === 0
        ? 'No clean tabs to close'
        : `Closed ${closeResult.closedCount} clean ${closeResult.closedCount === 1 ? 'tab' : 'tabs'}${retainedDraftNote}`
    );
  }

  function dirtyProjectSourcePathSet() {
    return new Set(dirtyProjectSourceRecords.map((record) => record.path));
  }

  async function applySourceTabCloseResult(
    closeResult: { tabs: SourceOpenTab[]; nextActivePath: string | null },
    status: string
  ) {
    const nextOpenSourceTabs = replaceProjectOpenTabs(openSourceTabs, selectedProject.id, closeResult.tabs);
    openSourceTabs = nextOpenSourceTabs;
    persistOpenSourceTabs(nextOpenSourceTabs);

    if (closeResult.nextActivePath === selectedRecord?.path) {
      fileActionStatus = status;
      return;
    }

    if (closeResult.nextActivePath) {
      const nextTab = closeResult.tabs.find((openTab) => openTab.path === closeResult.nextActivePath);
      if (nextTab) {
        await selectOpenTab(nextTab);
      }
      fileActionStatus = status;
      return;
    }

    clearSelectedSourceRecordForProject(selectedProject.id);
    fileActionStatus = status;
  }

  function trackSelectedSourceRecord(record: SourceRecord, project: ProjectRoot) {
    const openedAt = Date.now();
    const nextSelectedSourcePaths = {
      ...selectedSourcePaths,
      [project.id]: record.path
    };
    const nextRecentSourceRecords = upsertRecentSourceRecord(
      recentSourceRecords,
      record,
      project,
      openedAt,
      maxRecentSourceRecords
    );
    const nextProjectOpenTabs = upsertOpenSourceTab(
      projectOpenSourceTabs,
      record,
      project,
      openedAt,
      maxProjectOpenSourceTabs
    );
    const nextOpenSourceTabs = replaceProjectOpenTabs(openSourceTabs, project.id, nextProjectOpenTabs);

    selectedSourcePaths = nextSelectedSourcePaths;
    recentSourceRecords = nextRecentSourceRecords;
    openSourceTabs = nextOpenSourceTabs;
    persistSelectedSourcePaths(nextSelectedSourcePaths);
    persistRecentSourceRecords(nextRecentSourceRecords);
    persistOpenSourceTabs(nextOpenSourceTabs);
  }

  function replaceProjectOpenTabs(
    existingTabs: SourceOpenTab[],
    projectID: string,
    projectTabs: SourceOpenTab[]
  ): SourceOpenTab[] {
    return [
      ...existingTabs.filter((tab) => tab.projectID !== projectID),
      ...projectTabs
    ].slice(0, maxStoredOpenSourceTabs);
  }

  function clearSelectedSourceRecordForProject(projectID: string) {
    const nextSelectedSourcePaths = { ...selectedSourcePaths };
    delete nextSelectedSourcePaths[projectID];

    selectedSourcePaths = nextSelectedSourcePaths;
    selectedRecord = null;
    selectedSourceLine = null;
    pendingTreeRevealPath = null;
    pendingTreeFocusRowIndex = null;
    scanLimitReached = false;
    preview = null;
    resetSourceIntelligence();
    clearSelectedSourceGitDiff();
    loading = false;
    error = '';
    fileActionStatus = '';
    persistSelectedSourcePaths(nextSelectedSourcePaths);
  }

  function expandFoldersForRecord(record: SourceRecord) {
    const nextFolderIds = new Set(expandedFolderIds);
    for (const folderID of folderIdsForSourceRecord(record)) {
      nextFolderIds.add(folderID);
    }
    expandedFolderIds = nextFolderIds;
  }

  async function copySelectedPath() {
    if (!preview) return;

    fileActionBusy = 'copy';

    try {
      await copyTextToClipboard(preview.path, 'Path copied');
    } catch (copyError) {
      error = copyError instanceof Error ? copyError.message : 'Could not copy source path';
    } finally {
      fileActionBusy = '';
    }
  }

  async function copyTextToClipboard(text: string, successStatus: string) {
    fileActionStatus = '';
    error = '';

    await navigator.clipboard.writeText(text);
    fileActionStatus = successStatus;
  }

  async function copyActivityCommand(text: string, successStatus = 'Copied') {
    if (!text.trim()) return;

    fileActionBusy = 'activity-copy';

    try {
      await copyTextToClipboard(text, successStatus);
    } catch (copyError) {
      error = copyError instanceof Error ? copyError.message : 'Could not copy activity text';
    } finally {
      fileActionBusy = '';
    }
  }

  async function readPasteCleanupClipboard() {
    fileActionBusy = 'paste-read';

    try {
      pasteCleanupInput = await navigator.clipboard.readText();
      fileActionStatus = 'Clipboard loaded';
      error = '';
    } catch (clipboardError) {
      error = clipboardError instanceof Error ? clipboardError.message : 'Could not read clipboard';
    } finally {
      fileActionBusy = '';
    }
  }

  async function copyPasteCleanupOutput() {
    if (!pasteCleanupOutput.trim()) return;

    fileActionBusy = 'paste-copy';

    try {
      await copyTextToClipboard(pasteCleanupOutput, 'Cleaned text copied');
    } catch (copyError) {
      error = copyError instanceof Error ? copyError.message : 'Could not copy cleaned text';
    } finally {
      fileActionBusy = '';
    }
  }

  function selectPasteCleanupMode(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement | null)?.value;
    if (!isPasteCleanupMode(value)) return;

    pasteCleanupMode = value;
    persistPasteCleanupMode(value);
  }

  async function openActivityPath(path: string) {
    if (!path.trim()) return;

    fileActionBusy = `activity-open:${path}`;
    fileActionStatus = '';
    error = '';

    try {
      const opened = await openPathFromTauri(path);
      fileActionStatus = opened ? 'Opened path' : 'Native action unavailable';
    } catch (openError) {
      error = openError instanceof Error ? openError.message : 'Could not open path';
    } finally {
      fileActionBusy = '';
    }
  }

  async function revealActivityPath(path: string) {
    if (!path.trim()) return;

    fileActionBusy = `activity-reveal:${path}`;
    fileActionStatus = '';
    error = '';

    try {
      const revealed = await revealPathFromTauri(path);
      fileActionStatus = revealed ? 'Revealed path' : 'Native action unavailable';
    } catch (revealError) {
      error = revealError instanceof Error ? revealError.message : 'Could not reveal path';
    } finally {
      fileActionBusy = '';
    }
  }

  async function openActivityTerminalPath(path: string) {
    if (!path.trim()) return;

    fileActionBusy = `activity-terminal:${path}`;
    fileActionStatus = '';
    error = '';

    try {
      const opened = await openTerminalPathFromTauri(path, sourceTerminalApp);
      fileActionStatus = opened ? 'Opened terminal' : 'Native action unavailable';
    } catch (terminalError) {
      error = terminalError instanceof Error ? terminalError.message : 'Could not open terminal';
    } finally {
      fileActionBusy = '';
    }
  }

  async function resumeAgentSessionEmbeddedTerminal(session: AgentSession) {
    const command = agentSessionTerminalCommand(session);
    if (!command.trim()) return;

    captureActiveWorkspaceBeforeSwitch();
    captureAgentSessionWorkspaceSnapshot(session);
    markAgentSessionWorkspaceActive(session);
    showDockPanel('terminal');
    await tick();

    const path = agentSessionProjectPath(session);
    fileActionBusy = `embedded-terminal-command:${session.provider}:${session.id}`;
    fileActionStatus = '';
    error = '';
    embeddedTerminalError = '';

    try {
      if (embeddedTerminalSession) {
        if (normalizeProjectPath(embeddedTerminalSession.cwd) === normalizeProjectPath(path)) {
          await writeTerminalSessionFromTauri(embeddedTerminalSession.sessionId, `${command}\r`);
          embeddedTerminalStatus = 'Running embedded resume command';
          fileActionStatus = 'Sent resume command to embedded terminal';
          embeddedTerminal?.focus();
          return;
        }
      }

      await startEmbeddedTerminalSession(path, command);
      fileActionStatus = 'Started embedded resume command';
    } catch (terminalError) {
      error = terminalError instanceof Error ? terminalError.message : 'Could not resume agent in embedded terminal';
      embeddedTerminalStatus = 'Embedded resume failed';
    } finally {
      fileActionBusy = '';
    }
  }

  async function openAgentSessionTerminal(session: AgentSession) {
    const command = agentSessionTerminalCommand(session);
    if (!command.trim()) return;

    captureActiveWorkspaceBeforeSwitch();
    captureAgentSessionWorkspaceSnapshot(session);
    markAgentSessionWorkspaceActive(session);
    const path = session.projectPath ?? selectedProject.path;
    fileActionBusy = `activity-terminal-command:${session.provider}:${session.id}`;
    fileActionStatus = '';
    error = '';

    try {
      const openedCommand = await openTerminalCommandFromTauri(path, command, sourceTerminalApp);
      if (openedCommand) {
        fileActionStatus = 'Opened terminal resume command';
        return;
      }

      const openedPath = path.trim()
        ? await openTerminalPathFromTauri(path, sourceTerminalApp)
        : false;
      await navigator.clipboard.writeText(command);
      fileActionStatus = openedPath
        ? 'Opened terminal and copied resume command'
        : 'Resume command copied';
    } catch (terminalError) {
      try {
        const openedPath = path.trim()
          ? await openTerminalPathFromTauri(path, sourceTerminalApp)
          : false;
        await navigator.clipboard.writeText(command);
        fileActionStatus = openedPath
          ? 'Opened terminal and copied resume command'
          : 'Resume command copied';
      } catch {
        error = terminalError instanceof Error ? terminalError.message : 'Could not open terminal command';
      }
    } finally {
      fileActionBusy = '';
    }
  }

  async function openSelectedFile() {
    if (!preview) return;

    fileActionBusy = 'open';
    fileActionStatus = '';
    error = '';

    try {
      const opened = await openSourceFileFromTauri(preview.path);
      fileActionStatus = opened ? 'Opened file' : 'Native action unavailable';
    } catch (openError) {
      error = openError instanceof Error ? openError.message : 'Could not open source file';
    } finally {
      fileActionBusy = '';
    }
  }

  async function revealSelectedFile() {
    if (!preview) return;

    fileActionBusy = 'reveal';
    fileActionStatus = '';
    error = '';

    try {
      const revealed = await revealSourceFileFromTauri(preview.path);
      fileActionStatus = revealed ? 'Revealed in Finder' : 'Native action unavailable';
    } catch (revealError) {
      error = revealError instanceof Error ? revealError.message : 'Could not reveal source file';
    } finally {
      fileActionBusy = '';
    }
  }

  async function saveSelectedSourceFile() {
    if (!preview || !selectedRecord || !selectedSourceDirty) return;

    const record = selectedRecord;
    const content = selectedSourceDraftContent;
    fileActionBusy = 'save';
    fileActionStatus = '';
    error = '';

    try {
      const savedPreview = await writeSourceToTauri(record, content);
      if (!savedPreview) {
        fileActionStatus = 'Native save unavailable';
        return;
      }

      commitSourcePreviewContent(savedPreview);
      runtime = 'tauri file write';
      if (selectedRecord?.path === record.path) {
        preview = savedPreview;
      }
      void loadProjectGitStatus(selectedProject);
      void loadSelectedSourceGitDiff(record);
      fileActionStatus = 'Saved file';
    } catch (saveError) {
      error = saveError instanceof Error ? saveError.message : 'Could not save source file';
    } finally {
      fileActionBusy = '';
    }
  }

  async function saveAllDirtySourceFiles() {
    const dirtyRecords = dirtyProjectSourceRecords;
    if (dirtyRecords.length === 0) return;

    fileActionBusy = 'save-all';
    fileActionStatus = '';
    error = '';

    let savedCount = 0;

    try {
      for (const record of dirtyRecords) {
        const content = sourceDraftContentByPath[record.path];
        if (content === undefined || !isSourcePathDirty(record.path)) continue;

        const savedPreview = await writeSourceToTauri(record, content);
        if (!savedPreview) {
          fileActionStatus = `Native save unavailable for ${record.fileName}`;
          return;
        }

        commitSourcePreviewContent(savedPreview);
        if (selectedRecord?.path === record.path) {
          preview = savedPreview;
        }
        savedCount += 1;
      }

      runtime = 'tauri file write';
      void loadProjectGitStatus(selectedProject);
      if (selectedRecord) void loadSelectedSourceGitDiff(selectedRecord);
      fileActionStatus = `Saved ${savedCount} ${savedCount === 1 ? 'file' : 'files'}`;
    } catch (saveError) {
      error = saveError instanceof Error ? saveError.message : 'Could not save all source files';
    } finally {
      fileActionBusy = '';
    }
  }

  function revertSelectedSourceFile() {
    if (!preview || !selectedSourceDirty) return;

    const savedContent = savedSourceContentByPath[preview.path] ?? preview.content;
    sourceDraftContentByPath = {
      ...sourceDraftContentByPath,
      [preview.path]: savedContent
    };
    fileActionStatus = 'Reverted edits';
    error = '';
  }

  function updateSelectedSourceDraft(content: string) {
    if (!preview) return;

    sourceDraftContentByPath = {
      ...sourceDraftContentByPath,
      [preview.path]: content
    };
    scheduleSourceLspDiagnostics();
  }

  function syncSourcePreviewContent(nextPreview: SourcePreview | null) {
    if (!nextPreview) return;

    const existingDraft = sourceDraftContentByPath[nextPreview.path];
    const existingSaved = savedSourceContentByPath[nextPreview.path];
    const hasUnsavedDraft =
      existingDraft !== undefined &&
      existingSaved !== undefined &&
      existingDraft !== existingSaved;
    sourceDraftContentByPath = {
      ...sourceDraftContentByPath,
      [nextPreview.path]: hasUnsavedDraft ? existingDraft : nextPreview.content
    };
    savedSourceContentByPath = {
      ...savedSourceContentByPath,
      [nextPreview.path]: nextPreview.content
    };
  }

  function commitSourcePreviewContent(nextPreview: SourcePreview) {
    sourceDraftContentByPath = {
      ...sourceDraftContentByPath,
      [nextPreview.path]: nextPreview.content
    };
    savedSourceContentByPath = {
      ...savedSourceContentByPath,
      [nextPreview.path]: nextPreview.content
    };
    if (workspaceEditSourceRecordsByPath[nextPreview.path]) {
      const nextWorkspaceEditRecordsByPath = { ...workspaceEditSourceRecordsByPath };
      delete nextWorkspaceEditRecordsByPath[nextPreview.path];
      workspaceEditSourceRecordsByPath = nextWorkspaceEditRecordsByPath;
    }
  }

  function isSourcePathDirty(path: string) {
    const draftContent = sourceDraftContentByPath[path];
    const savedContent = savedSourceContentByPath[path];
    return draftContent !== undefined && savedContent !== undefined && draftContent !== savedContent;
  }

  async function stageExternalWorkspaceEditDrafts(files: SourceRenameFileEdit[]) {
    const selectedPath = preview?.path ?? '';
    const externalFiles = files.filter((file) => file.path !== selectedPath && file.edits.length > 0);
    if (externalFiles.length === 0) {
      return { fileCount: 0, editCount: 0, missingCount: 0 };
    }

    const nextDraftContentByPath = { ...sourceDraftContentByPath };
    const nextSavedContentByPath = { ...savedSourceContentByPath };
    const nextWorkspaceEditRecordsByPath = { ...workspaceEditSourceRecordsByPath };
    let nextProjectOpenTabs = projectOpenSourceTabs;
    let fileCount = 0;
    let editCount = 0;
    let missingCount = 0;

    for (const file of externalFiles) {
      const record = sourceRecordForWorkspaceEditFile(file);
      let draftContent = nextDraftContentByPath[file.path];
      let savedContent = nextSavedContentByPath[file.path];

      if (draftContent === undefined || savedContent === undefined) {
        const sourcePreview = await readSourceFromTauri(record);
        if (!sourcePreview) {
          missingCount += 1;
          continue;
        }

        savedContent = savedContent ?? sourcePreview.content;
        draftContent = draftContent ?? sourcePreview.content;
        nextSavedContentByPath[file.path] = savedContent;
      }

      const nextContent = applySourceTextEdits(draftContent, file.edits);
      const nextRecord = {
        ...record,
        byteCount: new TextEncoder().encode(nextContent).length
      };
      nextDraftContentByPath[file.path] = nextContent;
      nextWorkspaceEditRecordsByPath[file.path] = nextRecord;
      nextProjectOpenTabs = upsertOpenSourceTab(
        nextProjectOpenTabs,
        nextRecord,
        selectedProject,
        Date.now(),
        maxProjectOpenSourceTabs
      );
      fileCount += 1;
      editCount += file.edits.length;
    }

    sourceDraftContentByPath = nextDraftContentByPath;
    savedSourceContentByPath = nextSavedContentByPath;
    workspaceEditSourceRecordsByPath = nextWorkspaceEditRecordsByPath;

    if (fileCount > 0) {
      const nextOpenSourceTabs = replaceProjectOpenTabs(openSourceTabs, selectedProject.id, nextProjectOpenTabs);
      openSourceTabs = nextOpenSourceTabs;
      persistOpenSourceTabs(nextOpenSourceTabs);
    }

    return { fileCount, editCount, missingCount };
  }

  function sourceRecordForWorkspaceEditFile(file: SourceRenameFileEdit): SourceRecord {
    const existingRecord =
      records.find((record) => record.path === file.path) ??
      projectOpenSourceTabs.find((record) => record.path === file.path) ??
      workspaceEditSourceRecordsByPath[file.path];
    if (existingRecord) return existingRecord;

    const relativePath = file.relativePath || sourceRelativePathForPath(file.path);
    return {
      path: file.path,
      relativePath,
      fileName: fileNameFromRestoredPath(relativePath || file.path),
      language: sourceLanguageForPath(file.path),
      byteCount: 0
    };
  }

  function sourceRelativePathForPath(path: string) {
    const normalizedProjectPath = normalizeProjectPath(selectedProject.path);
    const normalizedPath = normalizeProjectPath(path);
    if (normalizedPath.startsWith(`${normalizedProjectPath}/`)) {
      return normalizedPath.slice(normalizedProjectPath.length + 1);
    }

    return fileNameFromRestoredPath(normalizedPath);
  }

  function resetSourceIntelligence() {
    sourceDiagnostics = [];
    sourceLspDiagnostics = [];
    sourceSymbols = [];
    sourceIntelligenceCommand = null;
    sourceLspStatus = null;
    sourceLspStatusError = '';
    sourceLspStatusLoading = false;
    if (sourceLspDiagnosticsTimer !== null) {
      window.clearTimeout(sourceLspDiagnosticsTimer);
      sourceLspDiagnosticsTimer = null;
    }
  }

  function handleEditorDiagnosticsChange(diagnostics: SourceDiagnostic[]) {
    sourceDiagnostics = diagnostics;
  }

  function scheduleSourceLspDiagnostics() {
    if (!preview || !sourceIntelligenceAvailable) return;
    if (sourceLspDiagnosticsTimer !== null) {
      window.clearTimeout(sourceLspDiagnosticsTimer);
    }
    sourceLspDiagnosticsTimer = window.setTimeout(() => {
      sourceLspDiagnosticsTimer = null;
      void loadSourceLspDiagnostics(preview);
    }, 650);
  }

  async function loadSourceLspDiagnostics(
    nextPreview: SourcePreview | null = preview,
    project: ProjectRoot = selectedProject
  ) {
    if (!nextPreview || !sourceSupportsLanguageIntelligence(nextPreview.language)) {
      sourceLspDiagnostics = [];
      return;
    }

    const expectedPath = nextPreview.path;
    const expectedProjectPath = project.path;
    const draftContent = sourceDraftContentByPath[nextPreview.path] ?? nextPreview.content;

    try {
      const diagnostics = await readSourceLspDiagnosticsFromTauri(
        { ...nextPreview, content: draftContent },
        {
          root: project.path,
          line: 1,
          column: 1
        }
      );
      if (preview?.path !== expectedPath || selectedProject.path !== expectedProjectPath) return;
      sourceLspDiagnostics = diagnostics ?? [];
    } catch {
      if (preview?.path === expectedPath && selectedProject.path === expectedProjectPath) {
        sourceLspDiagnostics = [];
      }
    }
  }

  async function loadSourceLspSymbols(
    nextPreview: SourcePreview | null = preview,
    project: ProjectRoot = selectedProject
  ) {
    if (!nextPreview || !sourceSupportsLanguageIntelligence(nextPreview.language)) return;

    const expectedPath = nextPreview.path;
    const expectedProjectPath = project.path;
    const draftContent = sourceDraftContentByPath[nextPreview.path] ?? nextPreview.content;

    try {
      const symbols = await findSourceLspSymbolsFromTauri(
        { ...nextPreview, content: draftContent },
        {
          root: project.path,
          line: 1,
          column: 1,
          limit: 100
        }
      );
      if (preview?.path !== expectedPath || selectedProject.path !== expectedProjectPath) return;
      if (symbols?.length) sourceSymbols = symbols;
    } catch {
      // Parser-provided Monaco symbols stay in place when native LSP is unavailable.
    }
  }

  async function loadSourceLspWorkspaceSymbols(query: string) {
    const normalizedQuery = query.trim();
    const sourcePreview = preview;
    if (!normalizedQuery || !sourcePreview || !sourceSupportsLanguageIntelligence(sourcePreview.language)) {
      workspaceSymbolResults = [];
      workspaceSymbolError = '';
      workspaceSymbolLoading = false;
      return;
    }

    const requestID = ++workspaceSymbolRequestID;
    workspaceSymbolLoading = true;
    workspaceSymbolError = '';

    try {
      const symbols = await findSourceLspWorkspaceSymbolsFromTauri(
        { ...sourcePreview, content: selectedSourceDraftContent },
        {
          root: selectedProject.path,
          query: normalizedQuery,
          limit: 24
        }
      );

      if (requestID !== workspaceSymbolRequestID) return;

      workspaceSymbolResults = symbols ?? [];
    } catch (workspaceSymbolLookupError) {
      if (requestID !== workspaceSymbolRequestID) return;

      workspaceSymbolResults = [];
      workspaceSymbolError =
        workspaceSymbolLookupError instanceof Error
          ? workspaceSymbolLookupError.message
          : 'Workspace symbol search unavailable';
    } finally {
      if (requestID === workspaceSymbolRequestID) {
        workspaceSymbolLoading = false;
      }
    }
  }

  function handleEditorSymbolsChange(symbols: SourceSymbol[]) {
    sourceSymbols = symbols;
  }

  async function handleEditorDefinitionLookup(request: SourceEditorLookupRequest) {
    return runSourceDefinitionLookup(request);
  }

  async function handleEditorHoverLookup(request: SourceEditorLookupRequest): Promise<SourceLspHover | null> {
    if (!preview || !sourceIntelligenceAvailable) return null;

    try {
      return await findSourceLspHoverFromTauri(
        { ...preview, content: selectedSourceDraftContent },
        {
          root: selectedProject.path,
          line: request.line,
          column: request.column
        }
      );
    } catch {
      return null;
    }
  }

  async function handleEditorCompletionLookup(
    request: SourceEditorLookupRequest
  ): Promise<SourceCompletionItem[]> {
    if (!preview || !sourceIntelligenceAvailable) return [];

    try {
      return (
        (await findSourceLspCompletionsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.line,
            column: request.column,
            limit: maxSourceCompletionResults
          }
        )) ?? []
      );
    } catch {
      return [];
    }
  }

  async function handleEditorSignatureHelpLookup(
    request: SourceEditorLookupRequest
  ): Promise<SourceSignatureHelp | null> {
    if (!preview || !sourceIntelligenceAvailable) return null;

    try {
      return await findSourceLspSignatureHelpFromTauri(
        { ...preview, content: selectedSourceDraftContent },
        {
          root: selectedProject.path,
          line: request.line,
          column: request.column
        }
      );
    } catch {
      return null;
    }
  }

  async function handleEditorInlayHintLookup(
    request: SourceEditorInlayHintLookupRequest
  ): Promise<SourceInlayHint[]> {
    if (!preview || !sourceIntelligenceAvailable) return [];

    try {
      return (
        (await findSourceLspInlayHintsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.startLine,
            column: request.startColumn,
            limit: 200
          }
        )) ?? []
      );
    } catch {
      return [];
    }
  }

  async function handleEditorSemanticTokensLookup(
    editorPreview: SourcePreview
  ): Promise<SourceSemanticToken[]> {
    if (!sourceSupportsLanguageIntelligence(editorPreview.language)) return [];

    try {
      return (
        (await findSourceLspSemanticTokensFromTauri(
          editorPreview,
          {
            root: selectedProject.path,
            line: 1,
            column: 1,
            limit: 5000
          }
        )) ?? []
      );
    } catch {
      return [];
    }
  }

  async function handleEditorCodeActionLookup(
    request: SourceCodeActionLookupRequest
  ): Promise<SourceCodeAction[]> {
    if (!preview || !sourceIntelligenceAvailable) return [];

    try {
      return (
        (await findSourceLspCodeActionsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            ...request,
            root: selectedProject.path,
            limit: 50
          }
        )) ?? []
      );
    } catch {
      return [];
    }
  }

  async function handleEditorWorkspaceEditAction(action: SourceCodeAction) {
    try {
      const staged = await stageExternalWorkspaceEditDrafts(action.files);
      if (staged.editCount === 0) return;

      fileActionStatus = `${action.title}: ${staged.editCount.toLocaleString()} external ${staged.editCount === 1 ? 'edit' : 'edits'} staged in ${staged.fileCount.toLocaleString()} ${staged.fileCount === 1 ? 'file' : 'files'}`;
    } catch (workspaceEditError) {
      error =
        workspaceEditError instanceof Error
          ? workspaceEditError.message
          : 'Could not stage workspace edits';
    }
  }

  async function handleEditorFormatDocument(): Promise<SourceTextEdit[]> {
    if (!preview || !sourceIntelligenceAvailable) return [];

    fileActionStatus = 'Formatting source file';
    try {
      const edits =
        (await formatSourceWithLspFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: 1,
            column: 1
          }
        )) ?? [];
      fileActionStatus =
        edits.length === 0
          ? 'No formatting edits'
          : `${edits.length} formatting ${edits.length === 1 ? 'edit' : 'edits'} applied to draft`;
      return edits;
    } catch (formatError) {
      fileActionStatus =
        formatError instanceof Error ? formatError.message : 'Formatting unavailable';
      return [];
    }
  }

  async function handleEditorRename(request: {
    line: number;
    column: number;
    newName: string;
  }): Promise<SourceRenameResult | null> {
    if (!preview || !sourceIntelligenceAvailable) return null;

    fileActionStatus = 'Renaming symbol';
    try {
      const result = await renameSourceWithLspFromTauri(
        { ...preview, content: selectedSourceDraftContent },
        {
          root: selectedProject.path,
          line: request.line,
          column: request.column,
          newName: request.newName
        }
      );
      const files = result?.files ?? [];
      const currentFileEditCount =
        files.find((file) => file.path === preview.path)?.edits.length ?? 0;
      const totalEditCount = files.reduce((count, file) => count + file.edits.length, 0);
      const externalDrafts = await stageExternalWorkspaceEditDrafts(files);
      const renameParts = [
        currentFileEditCount > 0
          ? `${currentFileEditCount.toLocaleString()} current-file ${currentFileEditCount === 1 ? 'edit' : 'edits'}`
          : '',
        externalDrafts.editCount > 0
          ? `${externalDrafts.editCount.toLocaleString()} external ${externalDrafts.editCount === 1 ? 'edit' : 'edits'} staged in ${externalDrafts.fileCount.toLocaleString()} ${externalDrafts.fileCount === 1 ? 'file' : 'files'}`
          : '',
        externalDrafts.missingCount > 0
          ? `${externalDrafts.missingCount.toLocaleString()} external ${externalDrafts.missingCount === 1 ? 'file' : 'files'} could not be read`
          : ''
      ].filter(Boolean);
      fileActionStatus =
        totalEditCount === 0
          ? 'No rename edits'
          : `Rename staged: ${renameParts.join(' · ')}`;
      return result;
    } catch (renameError) {
      fileActionStatus = renameError instanceof Error ? renameError.message : 'Rename unavailable';
      return null;
    }
  }

  async function handleEditorReferenceLookup(request: SourceEditorLookupRequest) {
    return runSourceReferenceLookup(request);
  }

  async function handleEditorDocumentHighlightLookup(request: SourceEditorLookupRequest) {
    if (!preview || !sourceIntelligenceAvailable) return [];

    try {
      return (
        (await findSourceLspDocumentHighlightsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.line,
            column: request.column,
            limit: 100
          }
        )) ?? []
      );
    } catch {
      return [];
    }
  }

  async function handleEditorImplementationLookup(request: SourceEditorLookupRequest) {
    return runSourceImplementationLookup(request);
  }

  async function handleEditorTypeDefinitionLookup(request: SourceEditorLookupRequest) {
    return runSourceTypeDefinitionLookup(request);
  }

  function requestSourceIntelligenceAction(action: SourceIntelligenceAction) {
    if (!preview || loading) return;
    if (
      (action === 'hover' ||
        action === 'implementation' ||
        action === 'type-definition' ||
        action === 'format' ||
        action === 'rename' ||
        action === 'quick-fix') &&
      !sourceIntelligenceAvailable
    ) {
      return;
    }

    sourceIntelligenceCommand = {
      id: ++sourceIntelligenceCommandId,
      action
    };
  }

  function selectSourceDiagnostic(diagnostic: SourceDiagnostic) {
    revealSourceLine(diagnostic.line);
  }

  function selectNextSourceDiagnostic(
    request: { line: number; column: number } = currentSourceDiagnosticNavigationRequest()
  ) {
    selectAdjacentSourceDiagnostic(request, 1);
  }

  function selectPreviousSourceDiagnostic(
    request: { line: number; column: number } = currentSourceDiagnosticNavigationRequest()
  ) {
    selectAdjacentSourceDiagnostic(request, -1);
  }

  function selectAdjacentSourceDiagnostic(
    request: { line: number; column: number },
    direction: 1 | -1
  ) {
    const diagnostic = findAdjacentSourceDiagnostic(
      sourceDiagnostics,
      request.line,
      request.column,
      direction
    );

    if (!diagnostic) {
      fileActionStatus = 'No problems';
      return;
    }

    selectSourceDiagnostic(diagnostic);
  }

  function currentSourceDiagnosticNavigationRequest() {
    return {
      line: selectedSourceLine ?? 1,
      column: 1
    };
  }

  function selectSourceSymbol(symbol: SourceSymbol) {
    revealSourceLine(symbol.line);
  }

  function revealSourceLine(line: number, recordNavigation = true) {
    if (!preview || !selectedRecord) return;

    const nextLine = Math.max(1, Math.floor(line));
    if (recordNavigation) {
      recordSourceNavigation(sourceNavigationLocationForRecord(selectedRecord, nextLine));
    }

    selectedSourceLine = nextLine;
    selectedSourceLineRequestId += 1;
  }

  async function handleProjectChange() {
    const nextProject =
      projectOptions.find((project) => project.id === selectedProjectID) ?? projectOptions[0] ?? initialProject;
    await activateProject(nextProject, { projects: projectOptions, scanLimit: expandedSourceScanLimit });
  }

  function selectSourceActivityMode(mode: SourceActivityMode) {
    markSourceLayoutCustom();
    sourceActivityMode = mode;
    persistSourceActivityMode(mode);
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function applySourceLayoutPreset(presetID: ConcreteSourceLayoutPresetID) {
    const preset = sourceLayoutPresets.find((candidate) => candidate.id === presetID);
    if (!preset) return;
    const override = sourceLayoutPresetOverrides[preset.id];

    sourceLayoutPreset = preset.id;
    sourceActivityMode = override?.activityMode ?? preset.activityMode;
    sidePaneWidth = clampSidePaneWidth(override?.sidePaneWidth ?? preset.sidePaneWidth);
    sidePanePosition = override?.sidePanePosition ?? preset.sidePanePosition;
    editorInsightWidth = clampEditorInsightWidth(override?.editorInsightWidth ?? preset.editorInsightWidth);
    editorInsightCollapsed = override?.editorInsightCollapsed ?? preset.editorInsightCollapsed;
    contextPaneWidth = clampContextPaneWidth(override?.contextPaneWidth ?? contextPaneWidth);
    contextPaneHeight = clampContextPaneHeight(override?.contextPaneHeight ?? contextPaneHeight);
    contextPanelCollapsed = override?.contextPanelCollapsed ?? preset.contextPanelCollapsed;
    contextPanelMode = override?.contextPanelMode ?? preset.contextPanelMode;
    contextPanelPlacement = override?.contextPanelPlacement ?? preset.contextPanelPlacement;
    sourceIntelligencePanel = override?.intelligencePanel ?? preset.intelligencePanel;
    hiddenContextCardIDs = new Set(override?.hiddenContextCardIDs ?? []);
    activeContextCardID = override?.activeContextCardID ?? activeContextCardID;

    persistSourceLayoutPreset(sourceLayoutPreset);
    persistSourceActivityMode(sourceActivityMode);
    persistSidePaneWidth(sidePaneWidth);
    persistSidePanePosition(sidePanePosition);
    persistEditorInsightWidth(editorInsightWidth);
    persistEditorInsightCollapsed(editorInsightCollapsed);
    persistContextPaneWidth(contextPaneWidth);
    persistContextPaneHeight(contextPaneHeight);
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistContextPanelMode(contextPanelMode);
    persistContextPanelPlacement(contextPanelPlacement);
    persistHiddenContextCards(hiddenContextCardIDs);
    persistActiveContextCard(activeContextCardID);
    sourceDockLayout = override?.dockLayout
      ? normalizeSourceDockLayout(override.dockLayout)
      : sourceDockLayoutFromWorkspace();
    syncSourceDockLayoutToWorkspace(sourceDockLayout);
    persistSourceDockLayout(sourceDockLayout);

    if (typeof window !== 'undefined') {
      window.setTimeout(measureFileTreeViewport, 0);
    }
  }

  function currentSaveableSourceLayoutPreset(): SourceLayoutPresetDefinition {
    const activePreset = sourceLayoutPresets.find((preset) => preset.id === sourceLayoutPreset);
    if (activePreset) return activePreset;
    return sourceLayoutPresets.find((preset) => preset.id === 'code') ?? sourceLayoutPresets[0]!;
  }

  function captureSourceLayoutPresetOverride(): SourceLayoutPresetOverride {
    return {
      activityMode: sourceActivityMode,
      sidePaneWidth: clampSidePaneWidth(sidePaneWidth),
      sidePanePosition,
      editorInsightWidth: clampEditorInsightWidth(editorInsightWidth),
      editorInsightCollapsed,
      contextPaneWidth: clampContextPaneWidth(contextPaneWidth),
      contextPaneHeight: clampContextPaneHeight(contextPaneHeight),
      contextPanelCollapsed,
      contextPanelMode,
      contextPanelPlacement,
      intelligencePanel: sourceIntelligencePanel,
      dockLayout: normalizeSourceDockLayout(sourceDockLayout),
      hiddenContextCardIDs: [...hiddenContextCardIDs],
      activeContextCardID
    };
  }

  function saveSourceLayoutPresetOverride(presetID: ConcreteSourceLayoutPresetID) {
    const preset = sourceLayoutPresets.find((candidate) => candidate.id === presetID);
    if (!preset) return;

    sourceLayoutPresetOverrides = {
      ...sourceLayoutPresetOverrides,
      [presetID]: captureSourceLayoutPresetOverride()
    };
    sourceLayoutPreset = presetID;
    persistSourceLayoutPreset(sourceLayoutPreset);
    persistSourceLayoutPresetOverrides(sourceLayoutPresetOverrides);
    fileActionStatus = `${preset.label} layout saved`;
  }

  function resetSourceLayoutPresetOverride(presetID: ConcreteSourceLayoutPresetID) {
    const preset = sourceLayoutPresets.find((candidate) => candidate.id === presetID);
    if (!preset || !sourceLayoutPresetOverrides[presetID]) return;

    const nextOverrides = { ...sourceLayoutPresetOverrides };
    delete nextOverrides[presetID];
    sourceLayoutPresetOverrides = nextOverrides;
    persistSourceLayoutPresetOverrides(sourceLayoutPresetOverrides);
    fileActionStatus = `${preset.label} layout reset`;
    if (sourceLayoutPreset === presetID) {
      applySourceLayoutPreset(presetID);
    }
  }

  function sourceActivityLabel(mode: SourceActivityMode) {
    switch (mode) {
      case 'files':
        return 'Files';
      case 'clipboard':
        return 'Clipboard';
      case 'conversations':
        return 'Conversations';
      case 'runs':
        return 'Runs';
      case 'sessions':
        return 'Active sessions';
      case 'agents':
        return 'Agents';
      case 'worktrees':
        return 'Worktrees';
      case 'git':
        return 'Git and tasks';
    }
  }

  function sourceActivityCount(mode: SourceActivityMode) {
    switch (mode) {
      case 'files':
        return filteredRecords.length;
      case 'clipboard':
        return pasteCleanupOutput.length;
      case 'conversations':
      case 'agents':
        return filteredProjectAgentSessions.length;
      case 'runs':
        return filteredProjectOrchestrationRuns.length;
      case 'sessions':
        return filteredProjectRuntimeContexts.length;
      case 'worktrees':
        return filteredProjectWorktrees.length;
      case 'git':
        return filteredGitRepositorySummaries.length;
    }
  }

  function sourceActivityFilterPlaceholder(mode: SourceActivityMode) {
    switch (mode) {
      case 'files':
        return 'Filter files';
      case 'clipboard':
        return 'Filter clipboard';
      case 'conversations':
        return 'Filter conversations';
      case 'runs':
        return 'Filter orchestration runs';
      case 'sessions':
        return 'Filter active sessions';
      case 'agents':
        return 'Filter agents';
      case 'worktrees':
        return 'Filter worktrees';
      case 'git':
        return 'Filter repos, tasks, commits';
    }
  }

  function activityTextMatchesFilter(
    filter: string,
    ...values: Array<string | number | boolean | null | undefined>
  ) {
    const normalizedFilter = filter.trim().toLowerCase();
    if (!normalizedFilter) return true;

    return values.some((value) => String(value ?? '').toLowerCase().includes(normalizedFilter));
  }

  function sourceActivitySummary(mode: SourceActivityMode) {
    switch (mode) {
      case 'files':
        return scanSummaryLabel;
      case 'clipboard':
        return pasteCleanupStats;
      case 'conversations':
      case 'agents':
        return agentSessionSummary;
      case 'runs':
        return orchestrationRunSummary;
      case 'sessions':
        return runtimeContextSummary;
      case 'worktrees':
        return projectWorktreeSummary;
      case 'git':
        return repoDashboardSummary;
    }
  }

  function refreshSourceActivityMode(mode: SourceActivityMode = sourceActivityMode) {
    switch (mode) {
      case 'files':
        void scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit });
        break;
      case 'clipboard':
        break;
      case 'conversations':
      case 'agents':
        void loadAgentSessions();
        break;
      case 'runs':
        void loadOrchestrationRuns(projectOptions);
        break;
      case 'sessions':
        void loadRuntimeContexts(projectOptions);
        break;
      case 'worktrees':
        void loadProjectWorktrees(selectedProject);
        break;
      case 'git':
        void loadGitRepositorySummaries(projectOptions);
        void loadGitCommitHistory(selectedProject);
        break;
    }
  }

  function sourceActivityRefreshing(mode: SourceActivityMode) {
    switch (mode) {
      case 'files':
        return scanning;
      case 'clipboard':
        return fileActionBusy === 'paste-read';
      case 'conversations':
      case 'agents':
        return agentSessionsLoading;
      case 'runs':
        return orchestrationRunsLoading;
      case 'sessions':
        return runtimeContextsLoading;
      case 'worktrees':
        return projectWorktreesLoading;
      case 'git':
        return gitRepositorySummariesLoading || gitCommitHistoryLoading;
    }
  }

  function loadStoredSourceActivityMode(): SourceActivityMode {
    if (typeof window === 'undefined') return 'files';

    const storedMode = window.localStorage.getItem(sourceActivityModeStorageKey);
    return isSourceActivityMode(storedMode) ? storedMode : 'files';
  }

  function isSourceActivityMode(value: unknown): value is SourceActivityMode {
    return (
      value === 'files' ||
      value === 'clipboard' ||
      value === 'conversations' ||
      value === 'runs' ||
      value === 'sessions' ||
      value === 'agents' ||
      value === 'worktrees' ||
      value === 'git'
    );
  }

  function persistSourceActivityMode(mode: SourceActivityMode) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceActivityModeStorageKey, mode);
  }

  function loadStoredPasteCleanupMode(): PasteCleanupMode {
    if (typeof window === 'undefined') return 'plain';

    const storedMode = window.localStorage.getItem(pasteCleanupModeStorageKey);
    return isPasteCleanupMode(storedMode) ? storedMode : 'plain';
  }

  function isPasteCleanupMode(value: unknown): value is PasteCleanupMode {
    return pasteCleanupModes.includes(value as PasteCleanupMode);
  }

  function persistPasteCleanupMode(mode: PasteCleanupMode) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(pasteCleanupModeStorageKey, mode);
  }

  function markSourceLayoutCustom() {
    if (sourceLayoutPreset === 'custom') return;
    sourceLayoutPreset = 'custom';
    persistSourceLayoutPreset(sourceLayoutPreset);
  }

  function loadStoredSourceLayoutPreset(): SourceLayoutPresetID {
    if (typeof window === 'undefined') return 'code';

    const storedPreset = window.localStorage.getItem(sourceLayoutPresetStorageKey);
    return isSourceLayoutPresetID(storedPreset) ? storedPreset : 'code';
  }

  function isSourceLayoutPresetID(value: unknown): value is SourceLayoutPresetID {
    return (
      value === 'review' ||
      value === 'code' ||
      value === 'git' ||
      value === 'runs' ||
      value === 'sessions' ||
      value === 'custom'
    );
  }

  function persistSourceLayoutPreset(presetID: SourceLayoutPresetID) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceLayoutPresetStorageKey, presetID);
  }

  function loadStoredSourceLayoutPresetOverrides(): SourceLayoutPresetOverrides {
    if (typeof window === 'undefined') return {};

    const storedOverrides = window.localStorage.getItem(sourceLayoutPresetOverridesStorageKey);
    if (!storedOverrides) return {};

    try {
      const parsedOverrides = JSON.parse(storedOverrides) as Record<string, unknown>;
      if (!parsedOverrides || typeof parsedOverrides !== 'object' || Array.isArray(parsedOverrides)) return {};

      return sourceLayoutPresets.reduce<SourceLayoutPresetOverrides>((overrides, preset) => {
        const override = normalizeStoredSourceLayoutPresetOverride(parsedOverrides[preset.id], preset);
        if (override) {
          overrides[preset.id] = override;
        }
        return overrides;
      }, {});
    } catch {
      return {};
    }
  }

  function normalizeStoredSourceLayoutPresetOverride(
    value: unknown,
    preset: SourceLayoutPresetDefinition
  ): SourceLayoutPresetOverride | null {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;

    const candidate = value as Record<string, unknown>;
    const hiddenCards = Array.isArray(candidate.hiddenContextCardIDs)
      ? candidate.hiddenContextCardIDs.filter(isSourceContextCardID)
      : [];

    return {
      activityMode: isSourceActivityMode(candidate.activityMode) ? candidate.activityMode : preset.activityMode,
      sidePaneWidth: clampSidePaneWidth(numericValue(candidate.sidePaneWidth, preset.sidePaneWidth)),
      sidePanePosition: isSourceSidePanePosition(candidate.sidePanePosition)
        ? candidate.sidePanePosition
        : preset.sidePanePosition,
      editorInsightWidth: clampEditorInsightWidth(
        numericValue(candidate.editorInsightWidth, preset.editorInsightWidth)
      ),
      editorInsightCollapsed:
        typeof candidate.editorInsightCollapsed === 'boolean'
          ? candidate.editorInsightCollapsed
          : preset.editorInsightCollapsed,
      contextPaneWidth: clampContextPaneWidth(numericValue(candidate.contextPaneWidth, contextPaneDefaultWidth)),
      contextPaneHeight: clampContextPaneHeight(numericValue(candidate.contextPaneHeight, contextPaneDefaultHeight)),
      contextPanelCollapsed:
        typeof candidate.contextPanelCollapsed === 'boolean'
          ? candidate.contextPanelCollapsed
          : preset.contextPanelCollapsed,
      contextPanelMode: isSourceContextPanelMode(candidate.contextPanelMode)
        ? candidate.contextPanelMode
        : preset.contextPanelMode,
      contextPanelPlacement: isSourceContextPanelPlacement(candidate.contextPanelPlacement)
        ? candidate.contextPanelPlacement
        : preset.contextPanelPlacement,
      intelligencePanel: isSourceIntelligencePanel(candidate.intelligencePanel)
        ? candidate.intelligencePanel
        : preset.intelligencePanel,
      dockLayout: normalizeSourceDockLayout(candidate.dockLayout),
      hiddenContextCardIDs: hiddenCards,
      activeContextCardID: isSourceContextCardID(candidate.activeContextCardID)
        ? candidate.activeContextCardID
        : preset.contextPanelPlacement === 'top'
          ? 'orchestration'
          : activeContextCardID
    };
  }

  function persistSourceLayoutPresetOverrides(overrides: SourceLayoutPresetOverrides) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceLayoutPresetOverridesStorageKey, JSON.stringify(overrides));
  }

  function numericValue(value: unknown, fallback: number) {
    return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
  }

  function isSourceSidePanePosition(value: unknown): value is SourceSidePanePosition {
    return value === 'left' || value === 'right';
  }

  function isSourceContextPanelMode(value: unknown): value is SourceContextPanelMode {
    return value === 'grid' || value === 'stack';
  }

  function isSourceContextPanelPlacement(value: unknown): value is SourceContextPanelPlacement {
    return value === 'top' || value === 'side' || value === 'bottom';
  }

  function isSourceIntelligencePanel(value: unknown): value is SourceIntelligencePanel {
    return value === 'problems' || value === 'symbols' || value === 'git';
  }

  function shouldMigrateSourceLayout() {
    if (typeof window === 'undefined') return false;
    return window.localStorage.getItem(sourceLayoutVersionStorageKey) !== sourceLayoutVersion;
  }

  function persistSourceLayoutVersion() {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceLayoutVersionStorageKey, sourceLayoutVersion);
  }

  function loadStoredSourceTerminalApp(): SourceTerminalApp {
    if (typeof window === 'undefined') return 'Warp';

    const storedTerminal = window.localStorage.getItem(sourceTerminalAppStorageKey);
    return isSourceTerminalApp(storedTerminal) ? storedTerminal : 'Warp';
  }

  function isSourceTerminalApp(value: unknown): value is SourceTerminalApp {
    return sourceTerminalApps.includes(value as SourceTerminalApp);
  }

  function selectSourceTerminalApp(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement | null)?.value;
    if (!isSourceTerminalApp(value)) return;

    sourceTerminalApp = value;
    persistSourceTerminalApp(value);
  }

  function persistSourceTerminalApp(app: SourceTerminalApp) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceTerminalAppStorageKey, app);
  }

  function moveDockPanelToGroup(panelID: SourceDockPanelID, groupID: SourceDockGroupID) {
    applySourceDockLayout(moveSourceDockPanel(sourceDockLayout, panelID, groupID));
  }

  function hideDockPanel(panelID: SourceDockPanelID) {
    applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, panelID));
  }

  function showDockPanel(panelID: SourceDockPanelID) {
    applySourceDockLayout(showSourceDockPanel(sourceDockLayout, panelID));
    if (panelID === 'terminal') {
      fileActionStatus = 'Terminal dock shown';
    }
    if (panelID === 'browser') {
      if (!browserUrl && defaultBrowserUrl) {
        setBrowserDockUrl(defaultBrowserUrl);
      }
      fileActionStatus = 'Browser dock shown';
    }
  }

  function resetSourceDockLayout() {
    applySourceLayoutPreset('code');
    fileActionStatus = 'Dock layout reset';
  }

  function toggleDockPanelVisibility(panelID: SourceDockPanelID) {
    if (!dockPanelCanHide(panelID)) return;
    if (sourceDockPanelVisible(panelID)) {
      hideDockPanel(panelID);
      return;
    }

    showDockPanel(panelID);
  }

  function moveDockPanelToManagedGroup(panelID: SourceDockPanelID, groupID: SourceDockGroupID) {
    if (!dockPanelMoveTargets(panelID).includes(groupID)) return;
    if (panelID === 'activity' && (groupID === 'left' || groupID === 'right')) {
      selectSidePanePosition(groupID);
      return;
    }

    moveDockPanelToGroup(panelID, groupID);
  }

  function moveDockPanelFromTab(panelID: SourceDockPanelID, event: Event) {
    const groupID = (event.currentTarget as HTMLSelectElement | null)?.value as SourceDockGroupID;
    if (!dockPanelMoveTargets(panelID).includes(groupID)) return;

    moveDockPanelToManagedGroup(panelID, groupID);
  }

  function beginDockPanelDrag(panelID: SourceDockPanelID, event: DragEvent) {
    if (dockPanelMoveTargets(panelID).length <= 1) return;

    draggingDockPanelID = panelID;
    dockDropTargetGroupID = null;
    event.dataTransfer?.setData(dockPanelDragDataType, panelID);
    event.dataTransfer?.setData('text/plain', panelID);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function dragOverDockDropZone(event: DragEvent, groupID: SourceDockGroupID) {
    const panelID = draggedDockPanelID(event);
    if (!panelID || !dockPanelMoveTargets(panelID).includes(groupID)) return;

    event.preventDefault();
    dockDropTargetGroupID = groupID;
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  function dropDockPanelOnGroup(event: DragEvent, groupID: SourceDockGroupID) {
    const panelID = draggedDockPanelID(event);
    if (!panelID || !dockPanelMoveTargets(panelID).includes(groupID)) {
      clearDockPanelDrag();
      return;
    }

    event.preventDefault();
    moveDockPanelToManagedGroup(panelID, groupID);
    clearDockPanelDrag();
  }

  function clearDockPanelDrag() {
    draggingDockPanelID = null;
    dockDropTargetGroupID = null;
  }

  function draggedDockPanelID(event: DragEvent): SourceDockPanelID | null {
    const draggedPanelID =
      event.dataTransfer?.getData(dockPanelDragDataType) ||
      event.dataTransfer?.getData('text/plain') ||
      draggingDockPanelID;

    return managedDockPanelIDs.includes(draggedPanelID as SourceDockPanelID)
      ? (draggedPanelID as SourceDockPanelID)
      : null;
  }

  function dockPanelMoveTargets(panelID: SourceDockPanelID): SourceDockGroupID[] {
    switch (panelID) {
      case 'activity':
        return ['left', 'right'];
      case 'context':
        return ['center', 'right', 'bottom'];
      case 'insights':
        return ['right'];
      case 'terminal':
      case 'browser':
        return ['bottom'];
      case 'editor':
        return ['center'];
    }
  }

  function dockPanelCanHide(panelID: SourceDockPanelID) {
    return (
      hideableDockPanelIDs.includes(panelID) &&
      Boolean(sourceDockPanelDescriptors.find((panel) => panel.id === panelID)?.canHide)
    );
  }

  function dockGroupLabel(groupID: SourceDockGroupID, panelID?: SourceDockPanelID) {
    if (panelID === 'context' && groupID === 'center') return 'Top';
    if (groupID === 'left') return 'Left';
    if (groupID === 'right') return 'Right';
    if (groupID === 'bottom') return 'Bottom';
    return 'Center';
  }

  function dockGroupShortcutLabel(groupID: SourceDockGroupID, panelID?: SourceDockPanelID) {
    if (panelID === 'context' && groupID === 'center') return 'T';
    if (groupID === 'left') return 'L';
    if (groupID === 'right') return 'R';
    if (groupID === 'bottom') return 'B';
    return 'C';
  }

  function dockPanelPlacementSummary(panelID: SourceDockPanelID) {
    const groupID = dockGroupIDForPanel(sourceDockLayout, panelID);
    if (groupID === null) return 'Hidden';
    return dockGroupLabel(groupID, panelID);
  }

  function selectDockPanel(panelID: SourceDockPanelID) {
    markSourceLayoutCustom();
    applySourceDockLayout(activateSourceDockPanel(sourceDockLayout, panelID));
    if (panelID === 'context') {
      contextPanelCollapsed = false;
      persistContextPanelCollapsed(contextPanelCollapsed);
    }
    if (panelID === 'insights') {
      editorInsightCollapsed = false;
      persistEditorInsightCollapsed(editorInsightCollapsed);
    }
  }

  function applySourceDockLayout(layout: SourceDockLayout) {
    const normalizedLayout = normalizeSourceDockLayout(layout);
    sourceDockLayout = normalizedLayout;
    syncSourceDockLayoutToWorkspace(normalizedLayout);
    persistSourceDockLayout(normalizedLayout);
  }

  function syncSourceDockLayoutToWorkspace(layout: SourceDockLayout) {
    const normalizedLayout = normalizeSourceDockLayout(layout);
    const activityGroupID = dockGroupIDForPanel(normalizedLayout, 'activity');
    const contextGroupID = dockGroupIDForPanel(normalizedLayout, 'context');
    const insightsGroupID = dockGroupIDForPanel(normalizedLayout, 'insights');

    if (activityGroupID === 'left' || activityGroupID === 'right') {
      sidePanePosition = activityGroupID;
      sidePaneWidth = clampSidePaneWidth(sourceDockGroupSize(normalizedLayout, activityGroupID));
      persistSidePanePosition(sidePanePosition);
      persistSidePaneWidth(sidePaneWidth);
    }

    contextPanelCollapsed = contextGroupID === null;
    persistContextPanelCollapsed(contextPanelCollapsed);
    if (contextGroupID !== null) {
      contextPanelPlacement = contextPanelPlacementForDockGroup(contextGroupID);
      const contextGroupSize = sourceDockGroupSize(normalizedLayout, contextGroupID);
      if (contextGroupID === 'bottom') {
        contextPaneHeight = clampContextPaneHeight(contextGroupSize);
        persistContextPaneHeight(contextPaneHeight);
      } else if (contextGroupID === 'left' || contextGroupID === 'right') {
        contextPaneWidth = clampContextPaneWidth(contextGroupSize);
        persistContextPaneWidth(contextPaneWidth);
      }
      persistContextPanelPlacement(contextPanelPlacement);
    }

    editorInsightCollapsed = insightsGroupID === null;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    if (insightsGroupID !== null && insightsGroupID !== contextGroupID) {
      editorInsightWidth = clampEditorInsightWidth(sourceDockGroupSize(normalizedLayout, insightsGroupID));
      persistEditorInsightWidth(editorInsightWidth);
    }
  }

  function sourceDockLayoutFromWorkspace(): SourceDockLayout {
    const layout = createDefaultSourceDockLayout();
    let nextLayout = moveSourceDockPanel(
      layout,
      'activity',
      sidePanePosition === 'right' ? 'right' : 'left'
    );
    nextLayout = contextPanelCollapsed
      ? hideSourceDockPanel(nextLayout, 'context')
      : moveSourceDockPanel(nextLayout, 'context', dockGroupForContextPanelPlacement(contextPanelPlacement));
    nextLayout = editorInsightCollapsed
      ? hideSourceDockPanel(nextLayout, 'insights')
      : moveSourceDockPanel(nextLayout, 'insights', 'right');
    nextLayout = resizeSourceDockGroup(nextLayout, sidePanePosition, sidePaneWidth);
    if (!contextPanelCollapsed) {
      nextLayout = resizeSourceDockGroup(
        nextLayout,
        dockGroupForContextPanelPlacement(contextPanelPlacement),
        contextPanelPlacement === 'bottom' ? contextPaneHeight : contextPaneWidth
      );
    }
    const contextGroupID = dockGroupIDForPanel(nextLayout, 'context');
    const insightsGroupID = dockGroupIDForPanel(nextLayout, 'insights');
    if (!editorInsightCollapsed && insightsGroupID !== null && insightsGroupID !== contextGroupID) {
      nextLayout = resizeSourceDockGroup(nextLayout, insightsGroupID, editorInsightWidth);
    }
    return normalizeSourceDockLayout({
      ...nextLayout,
      preset: sourceLayoutPreset
    });
  }

  function dockGroupIDForPanel(layout: SourceDockLayout, panelID: SourceDockPanelID): SourceDockGroupID | null {
    return layout.groups.find((group) => group.panelIDs.includes(panelID))?.id ?? null;
  }

  function dockGroupPanelIDs(groupID: SourceDockGroupID): SourceDockPanelID[] {
    return normalizeSourceDockLayout(sourceDockLayout).groups.find((group) => group.id === groupID)?.panelIDs ?? [];
  }

  function dockGroupHasTabs(groupID: SourceDockGroupID) {
    return dockGroupPanelIDs(groupID).length > 1;
  }

  function hasDockPanelTabs() {
    return dockTabGroupIDs.some((groupID) => dockGroupHasTabs(groupID));
  }

  function activeDockPanelForGroup(groupID: SourceDockGroupID): SourceDockPanelID | null {
    const normalizedLayout = normalizeSourceDockLayout(sourceDockLayout);
    const group = normalizedLayout.groups.find((candidateGroup) => candidateGroup.id === groupID);
    if (!group || group.panelIDs.length === 0) return null;
    const activePanelID = normalizedLayout.activePanelByGroup[groupID];
    return activePanelID && group.panelIDs.includes(activePanelID) ? activePanelID : group.panelIDs[0];
  }

  function dockPanelLabel(panelID: SourceDockPanelID) {
    switch (panelID) {
      case 'activity':
        return 'Activity';
      case 'editor':
        return 'Editor';
      case 'context':
        return 'Context';
      case 'insights':
        return 'Insights';
      case 'terminal':
        return 'Terminal';
      case 'browser':
        return 'Browser';
    }
  }

  function shouldRenderDockPanel(panelID: SourceDockPanelID) {
    const groupID = dockGroupIDForPanel(sourceDockLayout, panelID);
    if (groupID === null) return false;
    if (panelID === 'editor') return true;
    if (groupID === 'center') return true;
    const groupPanelIDs = dockGroupPanelIDs(groupID);
    return groupPanelIDs.length <= 1 || activeDockPanelForGroup(groupID) === panelID;
  }

  function sourceDockPanelVisible(panelID: SourceDockPanelID) {
    return dockGroupIDForPanel(sourceDockLayout, panelID) !== null;
  }

  function dockGroupForContextPanelPlacement(placement: SourceContextPanelPlacement): SourceDockGroupID {
    if (placement === 'side') return 'right';
    if (placement === 'bottom') return 'bottom';
    return 'center';
  }

  function contextPanelPlacementForDockGroup(groupID: SourceDockGroupID): SourceContextPanelPlacement {
    if (groupID === 'right' || groupID === 'left') return 'side';
    if (groupID === 'bottom') return 'bottom';
    return 'top';
  }

  function loadStoredSourceDockLayout(): SourceDockLayout | null {
    if (typeof window === 'undefined') return null;

    try {
      const storedLayout = window.localStorage.getItem(sourceDockLayoutStorageKey);
      return storedLayout ? normalizeSourceDockLayout(JSON.parse(storedLayout)) : null;
    } catch {
      return null;
    }
  }

  function persistSourceDockLayout(layout: SourceDockLayout) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(
      sourceDockLayoutStorageKey,
      JSON.stringify(normalizeSourceDockLayout(layout))
    );
  }

  function persistDockGroupSize(groupID: SourceDockGroupID, size: number) {
    sourceDockLayout = resizeSourceDockGroup(sourceDockLayout, groupID, size);
    persistSourceDockLayout(sourceDockLayout);
  }

  function loadStoredSidePanePosition(): SourceSidePanePosition {
    if (typeof window === 'undefined') return 'left';

    const storedPosition = window.localStorage.getItem(sidePanePositionStorageKey);
    return isSidePanePosition(storedPosition) ? storedPosition : 'left';
  }

  function isSidePanePosition(value: unknown): value is SourceSidePanePosition {
    return value === 'left' || value === 'right';
  }

  function selectSidePanePosition(position: SourceSidePanePosition) {
    markSourceLayoutCustom();
    sidePanePosition = position;
    persistSidePanePosition(position);
    sourceDockLayout = moveSourceDockPanel(sourceDockLayout, 'activity', position);
    sourceDockLayout = resizeSourceDockGroup(sourceDockLayout, position, sidePaneWidth);
    persistSourceDockLayout(sourceDockLayout);
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function persistSidePanePosition(position: SourceSidePanePosition) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sidePanePositionStorageKey, position);
  }

  function loadStoredSidePaneWidth() {
    if (typeof window === 'undefined') return sidePaneDefaultWidth;

    const storedWidth = window.localStorage.getItem(sidePaneWidthStorageKey);
    if (storedWidth === null) return sidePaneDefaultWidth;
    return clampSidePaneWidth(Number(storedWidth));
  }

  function persistSidePaneWidth(width: number) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sidePaneWidthStorageKey, String(clampSidePaneWidth(width)));
  }

  function clampSidePaneWidth(width: number) {
    if (!Number.isFinite(width)) return sidePaneDefaultWidth;
    return Math.min(sidePaneMaxWidth, Math.max(sidePaneMinWidth, Math.round(width)));
  }

  function beginSidePaneResize(event: PointerEvent) {
    if (event.button !== 0 || typeof window === 'undefined') return;

    const startX = event.clientX;
    const startWidth = sidePaneWidth;
    event.preventDefault();
    markSourceLayoutCustom();
    window.document.body.classList.add('resizing-source-pane');

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const delta = sidePanePosition === 'left'
        ? moveEvent.clientX - startX
        : startX - moveEvent.clientX;
      sidePaneWidth = clampSidePaneWidth(startWidth + delta);
      window.setTimeout(measureFileTreeViewport, 0);
    };
    const finishResize = () => {
      persistSidePaneWidth(sidePaneWidth);
      persistDockGroupSize(sidePanePosition, sidePaneWidth);
      window.document.body.classList.remove('resizing-source-pane');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', finishResize);
      window.removeEventListener('pointercancel', finishResize);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', finishResize);
    window.addEventListener('pointercancel', finishResize);
  }

  function handleSidePaneResizerKeydown(event: KeyboardEvent) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;

    event.preventDefault();
    markSourceLayoutCustom();
    const direction = event.key === 'ArrowLeft' ? -1 : 1;
    const signedDirection = sidePanePosition === 'left' ? direction : -direction;
    sidePaneWidth = clampSidePaneWidth(sidePaneWidth + signedDirection * 24);
    persistSidePaneWidth(sidePaneWidth);
    persistDockGroupSize(sidePanePosition, sidePaneWidth);
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function toggleContextPanelCollapsed() {
    markSourceLayoutCustom();
    contextPanelCollapsed = !contextPanelCollapsed;
    persistContextPanelCollapsed(contextPanelCollapsed);
    sourceDockLayout = contextPanelCollapsed
      ? hideSourceDockPanel(sourceDockLayout, 'context')
      : activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context');
    persistSourceDockLayout(sourceDockLayout);
  }

  function toggleEditorInsightCollapsed() {
    markSourceLayoutCustom();
    editorInsightCollapsed = !editorInsightCollapsed;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    sourceDockLayout = editorInsightCollapsed
      ? hideSourceDockPanel(sourceDockLayout, 'insights')
      : activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'insights'), 'insights');
    persistSourceDockLayout(sourceDockLayout);
  }

  function showEditorInsightPanel(panel: SourceIntelligencePanel) {
    markSourceLayoutCustom();
    sourceIntelligencePanel = panel;
    editorInsightCollapsed = false;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    sourceDockLayout = activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'insights'), 'insights');
    persistSourceDockLayout(sourceDockLayout);
  }

  function clearSourceLookupResults() {
    sourceDefinitionTargets = [];
    sourceDefinitionQuery = '';
    sourceDefinitionError = '';
    sourceReferenceTargets = [];
    sourceReferenceQuery = '';
    sourceReferenceError = '';
    sourceImplementationTargets = [];
    sourceImplementationQuery = '';
    sourceImplementationError = '';
    sourceTypeDefinitionTargets = [];
    sourceTypeDefinitionQuery = '';
    sourceTypeDefinitionError = '';
  }

  function isContextCardVisible(cardID: SourceContextCardID) {
    return !hiddenContextCardIDs.has(cardID);
  }

  function visibleContextCardIDs() {
    return contextCardOrder.filter(isContextCardVisible);
  }

  function activeVisibleContextCardID() {
    const visibleCardIDs = visibleContextCardIDs();
    return visibleCardIDs.includes(activeContextCardID) ? activeContextCardID : (visibleCardIDs[0] ?? null);
  }

  function shouldRenderContextCard(cardID: SourceContextCardID) {
    if (!isContextCardVisible(cardID)) return false;
    return contextPanelMode !== 'stack' || activeVisibleContextCardID() === cardID;
  }

  function selectActiveContextCard(cardID: SourceContextCardID) {
    markSourceLayoutCustom();
    activeContextCardID = cardID;
    persistActiveContextCard(cardID);
  }

  function hideContextCard(cardID: SourceContextCardID) {
    markSourceLayoutCustom();
    const nextCardIDs = new Set(hiddenContextCardIDs);
    nextCardIDs.add(cardID);
    hiddenContextCardIDs = nextCardIDs;
    persistHiddenContextCards(nextCardIDs);
  }

  function showAllContextCards() {
    markSourceLayoutCustom();
    hiddenContextCardIDs = new Set();
    persistHiddenContextCards(hiddenContextCardIDs);
    contextPanelCollapsed = false;
    sourceDockLayout = activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context');
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistSourceDockLayout(sourceDockLayout);
  }

  function showContextCard(cardID: SourceContextCardID) {
    markSourceLayoutCustom();
    const nextCardIDs = new Set(hiddenContextCardIDs);
    nextCardIDs.delete(cardID);
    hiddenContextCardIDs = nextCardIDs;
    activeContextCardID = cardID;
    contextPanelCollapsed = false;
    sourceDockLayout = activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context');
    persistHiddenContextCards(nextCardIDs);
    persistActiveContextCard(cardID);
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistSourceDockLayout(sourceDockLayout);
  }

  function selectContextPanelMode(mode: SourceContextPanelMode) {
    markSourceLayoutCustom();
    contextPanelMode = mode;
    persistContextPanelMode(mode);
  }

  function selectContextPanelPlacement(placement: SourceContextPanelPlacement) {
    markSourceLayoutCustom();
    contextPanelPlacement = placement;
    contextPanelCollapsed = false;
    persistContextPanelPlacement(placement);
    persistContextPanelCollapsed(contextPanelCollapsed);
    sourceDockLayout = moveSourceDockPanel(
      sourceDockLayout,
      'context',
      dockGroupForContextPanelPlacement(placement)
    );
    sourceDockLayout = resizeSourceDockGroup(
      sourceDockLayout,
      dockGroupForContextPanelPlacement(placement),
      placement === 'bottom' ? contextPaneHeight : contextPaneWidth
    );
    persistSourceDockLayout(sourceDockLayout);
  }

  function loadStoredContextPanelCollapsed() {
    if (typeof window === 'undefined') return false;
    return window.localStorage.getItem(contextPanelCollapsedStorageKey) === 'true';
  }

  function persistContextPanelCollapsed(collapsed: boolean) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(contextPanelCollapsedStorageKey, collapsed ? 'true' : 'false');
  }

  function loadStoredContextPanelMode(): SourceContextPanelMode {
    if (typeof window === 'undefined') return 'grid';

    const storedMode = window.localStorage.getItem(contextPanelModeStorageKey);
    return isContextPanelMode(storedMode) ? storedMode : 'grid';
  }

  function isContextPanelMode(value: unknown): value is SourceContextPanelMode {
    return value === 'grid' || value === 'stack';
  }

  function persistContextPanelMode(mode: SourceContextPanelMode) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(contextPanelModeStorageKey, mode);
  }

  function loadStoredContextPanelPlacement(): SourceContextPanelPlacement {
    if (typeof window === 'undefined') return 'top';

    const storedPlacement = window.localStorage.getItem(contextPanelPlacementStorageKey);
    return isContextPanelPlacement(storedPlacement) ? storedPlacement : 'top';
  }

  function isContextPanelPlacement(value: unknown): value is SourceContextPanelPlacement {
    return value === 'top' || value === 'side' || value === 'bottom';
  }

  function persistContextPanelPlacement(placement: SourceContextPanelPlacement) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(contextPanelPlacementStorageKey, placement);
  }

  function loadStoredHiddenContextCards(): Set<SourceContextCardID> {
    if (typeof window === 'undefined') return new Set();

    const storedCards = window.localStorage.getItem(hiddenContextCardsStorageKey);
    if (!storedCards) return new Set();

    try {
      const parsedCards = JSON.parse(storedCards);
      if (!Array.isArray(parsedCards)) return new Set();

      return new Set(parsedCards.filter(isSourceContextCardID));
    } catch {
      return new Set();
    }
  }

  function isSourceContextCardID(value: unknown): value is SourceContextCardID {
    return (
      value === 'orchestration' ||
      value === 'runtime' ||
      value === 'agents' ||
      value === 'worktrees' ||
      value === 'repo'
    );
  }

  function persistHiddenContextCards(cardIDs: Set<SourceContextCardID>) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(hiddenContextCardsStorageKey, JSON.stringify([...cardIDs]));
  }

  function loadStoredActiveContextCard(): SourceContextCardID {
    if (typeof window === 'undefined') return 'orchestration';

    const storedCardID = window.localStorage.getItem(activeContextCardStorageKey);
    return isSourceContextCardID(storedCardID) ? storedCardID : 'orchestration';
  }

  function persistActiveContextCard(cardID: SourceContextCardID) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(activeContextCardStorageKey, cardID);
  }

  function loadStoredEditorInsightWidth() {
    if (typeof window === 'undefined') return editorInsightDefaultWidth;

    const storedWidth = window.localStorage.getItem(editorInsightWidthStorageKey);
    if (storedWidth === null) return editorInsightDefaultWidth;
    return clampEditorInsightWidth(Number(storedWidth));
  }

  function persistEditorInsightWidth(width: number) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(editorInsightWidthStorageKey, String(clampEditorInsightWidth(width)));
  }

  function loadStoredEditorInsightCollapsed() {
    if (typeof window === 'undefined') return false;
    return window.localStorage.getItem(editorInsightCollapsedStorageKey) === 'true';
  }

  function persistEditorInsightCollapsed(collapsed: boolean) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(editorInsightCollapsedStorageKey, collapsed ? 'true' : 'false');
  }

  function clampEditorInsightWidth(width: number) {
    if (!Number.isFinite(width)) return editorInsightDefaultWidth;
    return Math.min(editorInsightMaxWidth, Math.max(editorInsightMinWidth, Math.round(width)));
  }

  function beginEditorInsightResize(event: PointerEvent) {
    if (event.button !== 0 || typeof window === 'undefined') return;

    const startX = event.clientX;
    const startWidth = editorInsightWidth;
    event.preventDefault();
    markSourceLayoutCustom();
    editorInsightCollapsed = false;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    window.document.body.classList.add('resizing-editor-insight');

    const handlePointerMove = (moveEvent: PointerEvent) => {
      editorInsightWidth = clampEditorInsightWidth(startWidth - (moveEvent.clientX - startX));
    };
    const finishResize = () => {
      persistEditorInsightWidth(editorInsightWidth);
      const insightsGroupID = dockGroupIDForPanel(sourceDockLayout, 'insights');
      if (insightsGroupID !== null) {
        persistDockGroupSize(insightsGroupID, editorInsightWidth);
      }
      window.document.body.classList.remove('resizing-editor-insight');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', finishResize);
      window.removeEventListener('pointercancel', finishResize);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', finishResize);
    window.addEventListener('pointercancel', finishResize);
  }

  function handleEditorInsightResizerKeydown(event: KeyboardEvent) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;

    event.preventDefault();
    markSourceLayoutCustom();
    editorInsightCollapsed = false;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    const direction = event.key === 'ArrowLeft' ? 1 : -1;
    editorInsightWidth = clampEditorInsightWidth(editorInsightWidth + direction * 24);
    persistEditorInsightWidth(editorInsightWidth);
    const insightsGroupID = dockGroupIDForPanel(sourceDockLayout, 'insights');
    if (insightsGroupID !== null) {
      persistDockGroupSize(insightsGroupID, editorInsightWidth);
    }
  }

  function loadStoredContextPaneWidth() {
    if (typeof window === 'undefined') return contextPaneDefaultWidth;

    const storedWidth = window.localStorage.getItem(contextPaneWidthStorageKey);
    if (storedWidth === null) return contextPaneDefaultWidth;
    return clampContextPaneWidth(Number(storedWidth));
  }

  function persistContextPaneWidth(width: number) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(contextPaneWidthStorageKey, String(clampContextPaneWidth(width)));
  }

  function clampContextPaneWidth(width: number) {
    if (!Number.isFinite(width)) return contextPaneDefaultWidth;
    return Math.min(contextPaneMaxWidth, Math.max(contextPaneMinWidth, Math.round(width)));
  }

  function loadStoredContextPaneHeight() {
    if (typeof window === 'undefined') return contextPaneDefaultHeight;

    const storedHeight = window.localStorage.getItem(contextPaneHeightStorageKey);
    if (storedHeight === null) return contextPaneDefaultHeight;
    return clampContextPaneHeight(Number(storedHeight));
  }

  function persistContextPaneHeight(height: number) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(contextPaneHeightStorageKey, String(clampContextPaneHeight(height)));
  }

  function clampContextPaneHeight(height: number) {
    if (!Number.isFinite(height)) return contextPaneDefaultHeight;
    return Math.min(contextPaneMaxHeight, Math.max(contextPaneMinHeight, Math.round(height)));
  }

  function beginContextPaneResize(event: PointerEvent) {
    if (event.button !== 0 || typeof window === 'undefined') return;

    const startX = event.clientX;
    const startY = event.clientY;
    const startWidth = contextPaneWidth;
    const startHeight = contextPaneHeight;
    event.preventDefault();
    markSourceLayoutCustom();
    const resizingClass = contextPanelPlacement === 'bottom'
      ? 'resizing-context-pane-bottom'
      : 'resizing-context-pane';
    window.document.body.classList.add(resizingClass);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      if (contextPanelPlacement === 'bottom') {
        contextPaneHeight = clampContextPaneHeight(startHeight - (moveEvent.clientY - startY));
        return;
      }

      contextPaneWidth = clampContextPaneWidth(startWidth - (moveEvent.clientX - startX));
    };
    const finishResize = () => {
      if (contextPanelPlacement === 'bottom') {
        persistContextPaneHeight(contextPaneHeight);
        persistDockGroupSize('bottom', contextPaneHeight);
      } else {
        persistContextPaneWidth(contextPaneWidth);
        persistDockGroupSize(dockGroupForContextPanelPlacement(contextPanelPlacement), contextPaneWidth);
      }
      window.document.body.classList.remove(resizingClass);
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', finishResize);
      window.removeEventListener('pointercancel', finishResize);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', finishResize);
    window.addEventListener('pointercancel', finishResize);
  }

  function handleContextPaneResizerKeydown(event: KeyboardEvent) {
    if (
      event.key !== 'ArrowLeft' &&
      event.key !== 'ArrowRight' &&
      event.key !== 'ArrowUp' &&
      event.key !== 'ArrowDown'
    ) return;

    event.preventDefault();
    markSourceLayoutCustom();
    if (contextPanelPlacement === 'bottom') {
      const direction = event.key === 'ArrowUp' ? 1 : event.key === 'ArrowDown' ? -1 : 0;
      if (direction === 0) return;
      contextPaneHeight = clampContextPaneHeight(contextPaneHeight + direction * 24);
      persistContextPaneHeight(contextPaneHeight);
      persistDockGroupSize('bottom', contextPaneHeight);
      return;
    }

    const direction = event.key === 'ArrowLeft' ? 1 : -1;
    contextPaneWidth = clampContextPaneWidth(contextPaneWidth + direction * 24);
    persistContextPaneWidth(contextPaneWidth);
    persistDockGroupSize(dockGroupForContextPanelPlacement(contextPanelPlacement), contextPaneWidth);
  }

  function loadStoredCustomProjectRoots(): ProjectRoot[] {
    if (typeof window === 'undefined') return [];

    try {
      const storedValue = window.localStorage.getItem(customProjectRootsStorageKey);
      if (!storedValue) return [];

      const parsedValue: unknown = JSON.parse(storedValue);
      if (!Array.isArray(parsedValue)) return [];

      const parsedRoots = parsedValue
        .map(parseStoredProjectRoot)
        .filter((project): project is ProjectRoot => project !== null);

      return mergeProjectRoots([], parsedRoots);
    } catch {
      return [];
    }
  }

  function parseStoredProjectRoot(value: unknown): ProjectRoot | null {
    if (typeof value !== 'object' || value === null) return null;

    const project = value as Partial<ProjectRoot>;
    if (typeof project.path !== 'string') return null;

    const parsedRoot = createProjectRoot(
      typeof project.name === 'string' ? project.name : '',
      project.path
    );

    return parsedRoot.path ? parsedRoot : null;
  }

  function persistCustomProjectRoots(roots: ProjectRoot[]) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(customProjectRootsStorageKey, JSON.stringify(roots));
  }

  function persistSelectedProjectID(projectID: string) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(selectedProjectStorageKey, projectID);
  }

  function persistSelectedSourcePaths(paths: Record<string, string>) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(selectedSourcePathStorageKey, JSON.stringify(paths));
  }

  function persistRecentSourceRecords(recentRecords: SourceRecentRecord[]) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(recentSourceRecordsStorageKey, JSON.stringify(recentRecords));
  }

  function persistOpenSourceTabs(tabs: SourceOpenTab[]) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(openSourceTabsStorageKey, JSON.stringify(tabs));
  }

  function persistWorkspaceSnapshots(snapshots: WorkspaceSnapshot[]) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(snapshotStorageKey, JSON.stringify(snapshots));
  }

  function loadStoredActiveWorkspaceSessionKey(): string | null {
    if (typeof window === 'undefined') return null;
    const storedKey = window.localStorage.getItem(activeWorkspaceSessionStorageKey)?.trim();
    return storedKey ? storedKey : null;
  }

  function persistActiveWorkspaceSessionKey(sessionKey: string | null) {
    if (typeof window === 'undefined') return;
    const normalizedKey = sessionKey?.trim();
    if (normalizedKey) {
      window.localStorage.setItem(activeWorkspaceSessionStorageKey, normalizedKey);
    } else {
      window.localStorage.removeItem(activeWorkspaceSessionStorageKey);
    }
  }

  function loadStoredSelectedProjectID(roots: ProjectRoot[]): string {
    if (typeof window === 'undefined') return initialProject.id;

    const storedProjectID = window.localStorage.getItem(selectedProjectStorageKey);
    return roots.some((project) => project.id === storedProjectID)
      ? storedProjectID ?? initialProject.id
      : roots[0]?.id ?? initialProject.id;
  }

  function loadStoredSelectedSourcePaths(): Record<string, string> {
    if (typeof window === 'undefined') return {};

    try {
      const storedValue = window.localStorage.getItem(selectedSourcePathStorageKey);
      if (!storedValue) return {};

      const parsedValue: unknown = JSON.parse(storedValue);
      if (typeof parsedValue !== 'object' || parsedValue === null || Array.isArray(parsedValue)) {
        return {};
      }

      return Object.fromEntries(
        Object.entries(parsedValue).filter(
          (entry): entry is [string, string] =>
            typeof entry[0] === 'string' &&
            typeof entry[1] === 'string' &&
            entry[1].trim().length > 0
        )
      );
    } catch {
      return {};
    }
  }

  function loadStoredRecentSourceRecords(): SourceRecentRecord[] {
    if (typeof window === 'undefined') return [];

    try {
      const storedValue = window.localStorage.getItem(recentSourceRecordsStorageKey);
      if (!storedValue) return [];

      const parsedValue: unknown = JSON.parse(storedValue);
      if (!Array.isArray(parsedValue)) return [];

      return parsedValue
        .map(parseStoredProjectSourceRecord)
        .filter((record): record is SourceRecentRecord => record !== null)
        .slice(0, maxRecentSourceRecords);
    } catch {
      return [];
    }
  }

  function loadStoredOpenSourceTabs(): SourceOpenTab[] {
    if (typeof window === 'undefined') return [];

    try {
      const storedValue = window.localStorage.getItem(openSourceTabsStorageKey);
      if (!storedValue) return [];

      const parsedValue: unknown = JSON.parse(storedValue);
      if (!Array.isArray(parsedValue)) return [];

      return parsedValue
        .map(parseStoredProjectSourceRecord)
        .filter((record): record is SourceOpenTab => record !== null)
        .slice(0, maxStoredOpenSourceTabs);
    } catch {
      return [];
    }
  }

  function loadStoredWorkspaceSnapshots(): WorkspaceSnapshot[] {
    if (typeof window === 'undefined') return [];

    try {
      const storedValue = window.localStorage.getItem(snapshotStorageKey);
      if (!storedValue) return [];

      const parsedValue: unknown = JSON.parse(storedValue);
      if (!Array.isArray(parsedValue)) return [];

      return parsedValue
        .map(parseStoredWorkspaceSnapshot)
        .filter((snapshot): snapshot is WorkspaceSnapshot => snapshot !== null)
        .slice(0, maxWorkspaceSnapshots);
    } catch {
      return [];
    }
  }

  function parseStoredProjectSourceRecord(value: unknown): SourceRecentRecord | null {
    if (typeof value !== 'object' || value === null) return null;

    const recentRecord = value as Partial<SourceRecentRecord>;
    if (
      typeof recentRecord.path !== 'string' ||
      typeof recentRecord.relativePath !== 'string' ||
      typeof recentRecord.fileName !== 'string' ||
      typeof recentRecord.language !== 'string' ||
      typeof recentRecord.projectID !== 'string' ||
      typeof recentRecord.projectName !== 'string' ||
      typeof recentRecord.byteCount !== 'number' ||
      typeof recentRecord.openedAt !== 'number' ||
      !Number.isFinite(recentRecord.byteCount) ||
      !Number.isFinite(recentRecord.openedAt)
    ) {
      return null;
    }

    return {
      path: recentRecord.path,
      relativePath: recentRecord.relativePath,
      fileName: recentRecord.fileName,
      language: recentRecord.language,
      byteCount: recentRecord.byteCount,
      projectID: recentRecord.projectID,
      projectName: recentRecord.projectName,
      openedAt: recentRecord.openedAt
    };
  }

  function startAddingProject() {
    addingProject = true;
    projectNameInput = '';
    projectPathInput = '';
    projectFormError = '';
  }

  async function chooseProjectRoot() {
    choosingProjectRoot = true;
    projectFormError = '';

    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Choose project folder'
      });
      const selectedFolder = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;

      if (typeof selectedFolder !== 'string' || selectedFolder.trim().length === 0) {
        return;
      }

      addCustomProjectRoot('', selectedFolder, false);
    } catch {
      startAddingProject();
      projectFormError = 'Folder picker unavailable';
    } finally {
      choosingProjectRoot = false;
    }
  }

  function cancelAddingProject() {
    addingProject = false;
    projectFormError = '';
  }

  function saveProject(event: SubmitEvent) {
    event.preventDefault();
    addCustomProjectRoot(projectNameInput, projectPathInput, true);
  }

  function addCustomProjectRoot(name: string, path: string, reportDuplicate: boolean) {
    const nextProject = createProjectRoot(name, path);

    if (!nextProject.path) {
      projectFormError = 'Path is required';
      return false;
    }

    const duplicateProject = projectOptions.find(
      (project) => normalizeProjectPath(project.path) === nextProject.path
    );

    if (duplicateProject) {
      activateDuplicateProjectRoot(
        duplicateProject,
        projectOptions,
        reportDuplicate ? 'Project already listed. Switching to it now.' : ''
      );
      return false;
    }

    const nextCustomProjectRoots = mergeProjectRoots([], [...customProjectRoots, nextProject]);
    const nextProjectOptions = mergeProjectRoots(defaultProjectRoots, nextCustomProjectRoots);
    customProjectRoots = nextCustomProjectRoots;
    persistCustomProjectRoots(nextCustomProjectRoots);
    addingProject = false;
    projectFormError = '';
    fileActionStatus = sourceOnboardingScanStatus(nextProject);
    void activateProject(nextProject, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects: nextProjectOptions
    });
    return true;
  }

  function activateDuplicateProjectRoot(project: ProjectRoot, projects: ProjectRoot[], message: string) {
    addingProject = false;
    projectFormError = message;
    fileActionStatus = sourceOnboardingScanStatus(project);
    void activateProject(project, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects
    });
  }

  async function activateProject(project: ProjectRoot, options: ProjectActivationOptions = {}) {
    const projects = options.projects ?? projectOptions;
    selectedProjectID = project.id;
    persistSelectedProjectID(project.id);
    void loadProjectGitStatus(project);
    void loadGitCommitHistory(project);
    void loadRuntimeContexts(projects);
    void loadProjectWorktrees(project);
    void loadGitRepositorySummaries(projects);
    void loadAgentSessions();
    void loadOrchestrationRuns(projects);
    await scanProject(project, selectedSourcePaths[project.id], {
      force: options.forceScan,
      limit: options.scanLimit
    });
    void indexProjectsInBackground(projects);
  }

  function removeSelectedProject() {
    if (!selectedProjectIsCustom) return;

    const removedProjectID = selectedProject.id;
    const nextCustomProjectRoots = customProjectRoots.filter(
      (project) => project.id !== removedProjectID
    );
    customProjectRoots = nextCustomProjectRoots;
    persistCustomProjectRoots(nextCustomProjectRoots);

    const nextSelectedSourcePaths = { ...selectedSourcePaths };
    delete nextSelectedSourcePaths[removedProjectID];
    selectedSourcePaths = nextSelectedSourcePaths;
    persistSelectedSourcePaths(nextSelectedSourcePaths);

    const nextRecentSourceRecords = recentSourceRecords.filter(
      (record) => record.projectID !== removedProjectID
    );
    recentSourceRecords = nextRecentSourceRecords;
    persistRecentSourceRecords(nextRecentSourceRecords);

    const nextOpenSourceTabs = openSourceTabs.filter(
      (tab) => tab.projectID !== removedProjectID
    );
    openSourceTabs = nextOpenSourceTabs;
    persistOpenSourceTabs(nextOpenSourceTabs);

    if (selectedProjectID === removedProjectID) {
      const fallbackProject = defaultProjectRoots[0];
      selectedProjectID = fallbackProject.id;
      persistSelectedProjectID(fallbackProject.id);
      void loadProjectGitStatus(fallbackProject);
      void loadGitCommitHistory(fallbackProject);
      void scanProject(fallbackProject, nextSelectedSourcePaths[fallbackProject.id]);
    }
  }

  function measureFileTreeViewport() {
    if (!fileTreeElement) return;

    fileTreeViewportHeight = fileTreeElement.clientHeight || sourceTreeFallbackViewportHeight;
    fileTreeScrollTop = fileTreeElement.scrollTop;
  }

  function handleFileTreeScroll(event: Event) {
    const target = event.currentTarget;
    if (!(target instanceof HTMLDivElement)) return;

    fileTreeScrollTop = target.scrollTop;
    fileTreeViewportHeight = target.clientHeight || sourceTreeFallbackViewportHeight;
  }

  function requestSourceTreeReveal(record: SourceRecord) {
    pendingTreeRevealPath = record.path;
  }

  function focusTreeRowAtIndex(rowIndex: number) {
    if (visibleTreeRows.length === 0) return;

    const nextRowIndex = Math.min(Math.max(rowIndex, 0), visibleTreeRows.length - 1);
    pendingTreeFocusRowIndex = nextRowIndex;

    if (!fileTreeElement) return;

    const nextScrollTop = scrollTopForSourceTreeReveal(
      nextRowIndex,
      fileTreeElement.scrollTop,
      fileTreeElement.clientHeight || sourceTreeFallbackViewportHeight,
      sourceTreeRowHeight
    );

    if (Math.abs(fileTreeElement.scrollTop - nextScrollTop) > 0.5) {
      fileTreeElement.scrollTop = nextScrollTop;
    }
    fileTreeScrollTop = fileTreeElement.scrollTop;
  }

  function parentTreeRowIndex(row: SourceTreeRow, rowIndex: number): number {
    if (row.level === 0) return rowIndex;

    for (let index = rowIndex - 1; index >= 0; index -= 1) {
      if (visibleTreeRows[index]?.level === row.level - 1) return index;
    }
    return rowIndex;
  }

  function handleTreeRowKeydown(row: SourceTreeRow, event: KeyboardEvent) {
    const rowIndex = visibleTreeRows.findIndex((treeRow) => treeRow.node.id === row.node.id);
    if (rowIndex < 0) return;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      focusTreeRowAtIndex(rowIndex + 1);
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      focusTreeRowAtIndex(rowIndex - 1);
      return;
    }

    if (event.key === 'Home') {
      event.preventDefault();
      focusTreeRowAtIndex(0);
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      focusTreeRowAtIndex(visibleTreeRows.length - 1);
      return;
    }

    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      selectTreeNode(row.node);
      return;
    }

    const isFolder = row.node.file === null;
    if (event.key === 'ArrowRight' && isFolder) {
      event.preventDefault();
      if (isFolderExpanded(row.node)) {
        focusTreeRowAtIndex(rowIndex + 1);
      } else {
        toggleFolder(row.node);
      }
      return;
    }

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      if (isFolder && isFolderExpanded(row.node)) {
        toggleFolder(row.node);
      } else {
        focusTreeRowAtIndex(parentTreeRowIndex(row, rowIndex));
      }
    }
  }

  function isFolderExpanded(node: SourceTreeNode): boolean {
    return autoExpandFolders || expandedFolderIds.has(node.id);
  }

  function toggleFolder(node: SourceTreeNode) {
    const nextFolderIds = new Set(expandedFolderIds);
    if (nextFolderIds.has(node.id)) {
      nextFolderIds.delete(node.id);
    } else {
      nextFolderIds.add(node.id);
    }
    expandedFolderIds = nextFolderIds;
  }

  function selectTreeNode(node: SourceTreeNode) {
    if (node.file) {
      void selectRecord(node.file);
      return;
    }
    toggleFolder(node);
  }

  onMount(() => {
    let unlistenSourceScanProgress: (() => void) | null = null;
    let unlistenTerminalOutput: (() => void) | null = null;
    void listenToSourceScanProgress((progress) => {
      if (progress.scanId === activeSourceScanId) {
        sourceScanProgress = progress;
      }
    })
      .then((unlisten) => {
        unlistenSourceScanProgress = unlisten;
      })
      .catch(() => {
        unlistenSourceScanProgress = null;
      });
    void listenToTerminalOutput(handleTerminalOutput)
      .then((unlisten) => {
        unlistenTerminalOutput = unlisten;
      })
      .catch(() => {
        unlistenTerminalOutput = null;
      });
    window.addEventListener('beforeunload', captureActiveWorkspaceSnapshotBeforeUnload);
    window.addEventListener('pagehide', captureActiveWorkspaceSnapshotBeforeUnload);
    document.addEventListener('visibilitychange', handleWorkspaceSnapshotVisibilityChange);

    const storedCustomProjectRoots = loadStoredCustomProjectRoots();
    const storedProjectOptions = mergeProjectRoots(defaultProjectRoots, storedCustomProjectRoots);
    const storedProjectID = loadStoredSelectedProjectID(storedProjectOptions);
    const storedSelectedSourcePaths = loadStoredSelectedSourcePaths();
    const storedRecentSourceRecords = loadStoredRecentSourceRecords();
    const storedOpenSourceTabs = loadStoredOpenSourceTabs();
    const storedWorkspaceSnapshots = loadStoredWorkspaceSnapshots();
    const storedActiveWorkspaceSessionKey = loadStoredActiveWorkspaceSessionKey();
    const storedSourceActivityMode = loadStoredSourceActivityMode();
    const storedPasteCleanupMode = loadStoredPasteCleanupMode();
    const storedSourceLayoutPreset = loadStoredSourceLayoutPreset();
    const storedSourceLayoutPresetOverrides = loadStoredSourceLayoutPresetOverrides();
    const storedSourceTerminalApp = loadStoredSourceTerminalApp();
    const storedBrowserDockUrl = loadStoredBrowserDockUrl();
    const storedSidePanePosition = loadStoredSidePanePosition();
    const storedSidePaneWidth = loadStoredSidePaneWidth();
    const storedEditorInsightWidth = loadStoredEditorInsightWidth();
    const storedEditorInsightCollapsed = loadStoredEditorInsightCollapsed();
    const storedContextPaneWidth = loadStoredContextPaneWidth();
    const storedContextPaneHeight = loadStoredContextPaneHeight();
    const storedContextPanelCollapsed = loadStoredContextPanelCollapsed();
    const storedContextPanelMode = loadStoredContextPanelMode();
    const storedContextPanelPlacement = loadStoredContextPanelPlacement();
    const storedHiddenContextCardIDs = loadStoredHiddenContextCards();
    const storedActiveContextCardID = loadStoredActiveContextCard();
    const storedSourceDockLayout = loadStoredSourceDockLayout();
    const migrateSourceLayout = shouldMigrateSourceLayout();
    const compactPreset =
      sourceLayoutPresets.find((preset) => preset.id === 'code') ?? sourceLayoutPresets[0];
    const storedProject =
      storedProjectOptions.find((project) => project.id === storedProjectID) ??
      storedProjectOptions[0] ??
      initialProject;

    customProjectRoots = storedCustomProjectRoots;
    selectedSourcePaths = storedSelectedSourcePaths;
    recentSourceRecords = storedRecentSourceRecords;
    openSourceTabs = storedOpenSourceTabs;
    workspaceSnapshots = storedWorkspaceSnapshots;
    activeWorkspaceSessionKey = storedActiveWorkspaceSessionKey;
    selectedProjectID = storedProject.id;
    sourceActivityMode = migrateSourceLayout ? compactPreset.activityMode : storedSourceActivityMode;
    pasteCleanupMode = storedPasteCleanupMode;
    sourceLayoutPreset = migrateSourceLayout ? compactPreset.id : storedSourceLayoutPreset;
    sourceLayoutPresetOverrides = storedSourceLayoutPresetOverrides;
    sourceTerminalApp = storedSourceTerminalApp;
    browserUrl = storedBrowserDockUrl;
    browserInputUrl = storedBrowserDockUrl;
    sidePanePosition = migrateSourceLayout ? compactPreset.sidePanePosition : storedSidePanePosition;
    sidePaneWidth = migrateSourceLayout ? compactPreset.sidePaneWidth : storedSidePaneWidth;
    editorInsightWidth = migrateSourceLayout ? compactPreset.editorInsightWidth : storedEditorInsightWidth;
    editorInsightCollapsed = migrateSourceLayout ? compactPreset.editorInsightCollapsed : storedEditorInsightCollapsed;
    contextPaneWidth = storedContextPaneWidth;
    contextPaneHeight = storedContextPaneHeight;
    contextPanelCollapsed = migrateSourceLayout ? compactPreset.contextPanelCollapsed : storedContextPanelCollapsed;
    contextPanelMode = migrateSourceLayout ? compactPreset.contextPanelMode : storedContextPanelMode;
    contextPanelPlacement = migrateSourceLayout ? compactPreset.contextPanelPlacement : storedContextPanelPlacement;
    hiddenContextCardIDs = storedHiddenContextCardIDs;
    activeContextCardID = storedActiveContextCardID;
    sourceDockLayout = migrateSourceLayout || !storedSourceDockLayout
      ? sourceDockLayoutFromWorkspace()
      : storedSourceDockLayout;
    if (!migrateSourceLayout && storedSourceDockLayout) {
      syncSourceDockLayoutToWorkspace(storedSourceDockLayout);
    }
    persistSelectedProjectID(storedProject.id);
    if (migrateSourceLayout) {
      persistSourceLayoutPreset(sourceLayoutPreset);
      persistSourceActivityMode(sourceActivityMode);
      persistSidePanePosition(sidePanePosition);
      persistSidePaneWidth(sidePaneWidth);
      persistEditorInsightWidth(editorInsightWidth);
      persistEditorInsightCollapsed(editorInsightCollapsed);
      persistContextPanelCollapsed(contextPanelCollapsed);
      persistContextPanelMode(contextPanelMode);
      persistContextPanelPlacement(contextPanelPlacement);
    }
    persistSourceDockLayout(sourceDockLayout);
    persistSourceLayoutVersion();
    window.setTimeout(measureFileTreeViewport, 0);
    void loadProjectGitStatus(storedProject);
    void loadGitCommitHistory(storedProject);
    void loadRuntimeContexts(storedProjectOptions);
    void loadProjectWorktrees(storedProject);
    void loadGitRepositorySummaries(storedProjectOptions);
    void loadAgentSessions();
    void loadOrchestrationRuns(storedProjectOptions);
    void loadEmbeddedTerminalSessions();
    void scanProject(storedProject, storedSelectedSourcePaths[storedProject.id], { limit: expandedSourceScanLimit }).then(
      () => indexProjectsInBackground(storedProjectOptions)
    );

    return () => {
      window.removeEventListener('beforeunload', captureActiveWorkspaceSnapshotBeforeUnload);
      window.removeEventListener('pagehide', captureActiveWorkspaceSnapshotBeforeUnload);
      document.removeEventListener('visibilitychange', handleWorkspaceSnapshotVisibilityChange);
      unlistenSourceScanProgress?.();
      unlistenTerminalOutput?.();
      disposeEmbeddedTerminal();
    };
  });
</script>

<svelte:head>
  <title>MacCommandBar Webview Preview</title>
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} />

<main
  class="shell"
  class:side-right={sidePanePosition === 'right'}
  style={`--accent: #5ce2cf; --side-pane-width: ${sidePaneWidth}px; --editor-insight-width: ${editorInsightWidth}px; --context-pane-width: ${contextPaneWidth}px; --context-pane-height: ${contextPaneHeight}px`}
>
  <aside class="activity-shell" aria-label="Workspace browser">
    <nav class="activity-rail" aria-label="Workspace views">
      <button
        class:active={sourceActivityMode === 'files'}
        type="button"
        aria-label="Files"
        title="Files"
        onclick={() => selectSourceActivityMode('files')}
      >
        <FolderGit2 size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Files</span>
        <strong>{sourceActivityCount('files')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'clipboard'}
        type="button"
        aria-label="Clipboard"
        title="Clipboard"
        onclick={() => selectSourceActivityMode('clipboard')}
      >
        <Copy size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Clipboard</span>
        <strong>{sourceActivityCount('clipboard')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'conversations'}
        type="button"
        aria-label="Conversations"
        title="Conversations"
        onclick={() => selectSourceActivityMode('conversations')}
      >
        <History size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Conversations</span>
        <strong>{sourceActivityCount('conversations')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'runs'}
        type="button"
        aria-label="Runs"
        title="Runs"
        onclick={() => selectSourceActivityMode('runs')}
      >
        <Network size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Runs</span>
        <strong>{sourceActivityCount('runs')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'sessions'}
        type="button"
        aria-label="Active sessions"
        title="Active sessions"
        onclick={() => selectSourceActivityMode('sessions')}
      >
        <SplitSquareHorizontal size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Active sessions</span>
        <strong>{sourceActivityCount('sessions')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'agents'}
        type="button"
        aria-label="Agents"
        title="Agents"
        onclick={() => selectSourceActivityMode('agents')}
      >
        <Activity size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Agents</span>
        <strong>{sourceActivityCount('agents')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'worktrees'}
        type="button"
        aria-label="Worktrees"
        title="Worktrees"
        onclick={() => selectSourceActivityMode('worktrees')}
      >
        <FolderSearch size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Worktrees</span>
        <strong>{sourceActivityCount('worktrees')}</strong>
      </button>
      <button
        class:active={sourceActivityMode === 'git'}
        type="button"
        aria-label="Git and tasks"
        title="Git and tasks"
        onclick={() => selectSourceActivityMode('git')}
      >
        <Braces size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Git and tasks</span>
        <strong>{sourceActivityCount('git')}</strong>
      </button>
    </nav>

    <div class="sidebar" aria-label={sourceActivityPanelLabel}>
    <div class="brand-row">
      <div class="brand-mark">
        <Braces size={22} strokeWidth={1.8} />
      </div>
      <div>
        <p class="eyebrow">MacCommandBar</p>
        <h1>Source Browser</h1>
      </div>
    </div>

    <div class="project-controls">
      <div class="project-row">
        <select bind:value={selectedProjectID} onchange={handleProjectChange} aria-label="Project">
          {#each projectOptions as project}
            <option value={project.id}>{project.name}</option>
          {/each}
        </select>
        <button class="icon-button" type="button" aria-label="Choose project folder" title="Choose project folder" disabled={choosingProjectRoot} onclick={chooseProjectRoot}>
          <Plus size={16} strokeWidth={2} />
        </button>
        <button class="icon-button quick-open-trigger" type="button" aria-label="Open source file" title="Open source file" onclick={openQuickOpen}>
          <Search size={16} strokeWidth={1.9} />
        </button>
        <button
          class="scan-button"
          type="button"
          aria-label={scanning ? 'Stop source scan' : `Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}
          title={scanning ? 'Stop source scan' : `Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}
          onclick={scanning ? cancelSourceScan : () => scanProject(selectedProject, undefined, { force: true, limit: expandedSourceScanLimit })}
        >
          {#if scanning}
            <X size={15} strokeWidth={2} />
          {:else}
            <RefreshCw size={15} strokeWidth={1.8} />
          {/if}
          <span>{scanning ? 'Stop' : 'Scan'}</span>
        </button>
      </div>

      <div class="project-path-row">
        <span title={selectedProject.path}>{selectedProject.path}</span>
        {#if selectedProjectIsCustom}
          <button class="icon-button danger" type="button" aria-label="Remove project root" title="Remove project root" onclick={removeSelectedProject}>
            <Trash2 size={14} strokeWidth={1.9} />
          </button>
        {/if}
      </div>

      {#if scanning && sourceScanProgress}
        <div class="scan-progress" aria-live="polite" data-progress-event={sourceScanProgressEventName}>
          <span>{sourceScanProgress.matchedFiles.toLocaleString()} files</span>
          <span>{sourceScanProgress.visitedEntries.toLocaleString()} entries checked</span>
        </div>
      {/if}

      {#if addingProject}
        <form class="project-form" onsubmit={saveProject}>
          <label>
            <span>Name</span>
            <input bind:value={projectNameInput} autocomplete="off" />
          </label>
          <label>
            <span>Path</span>
            <input bind:value={projectPathInput} autocomplete="off" placeholder="/Users/blackcolours/dev/work/project" />
          </label>
          {#if projectFormError}
            <p class="project-form-error">{projectFormError}</p>
          {/if}
          <div class="form-actions">
            <button class="form-button" type="button" onclick={cancelAddingProject}>
              <X size={14} strokeWidth={2} />
              <span>Cancel</span>
            </button>
            <button class="form-button primary" type="submit">
              <Save size={14} strokeWidth={2} />
              <span>Save</span>
            </button>
          </div>
        </form>
      {/if}
    </div>

    {#if sourceActivityMode === 'files'}
      <div class="source-browser-stack">
      <label class="search-box">
        <Search size={16} strokeWidth={1.8} />
        <input bind:value={query} placeholder="Filter source files" />
      </label>

      <form class="global-search-panel" onsubmit={handleGlobalSourceSearchSubmit}>
        <div class="global-search-box">
          <Search size={15} strokeWidth={1.8} />
          <input bind:this={sourceSearchInput} bind:value={sourceSearchQuery} placeholder="Search file contents" />
          <button
            class="source-search-submit"
            type="submit"
            aria-label="Search source contents"
            title="Search source contents"
            disabled={sourceSearchLoading || sourceSearchQuery.trim().length === 0}
          >
            {#if sourceSearchLoading}
              <RefreshCw size={13} strokeWidth={1.8} />
            {:else}
              <Search size={13} strokeWidth={1.8} />
            {/if}
          </button>
        </div>

        {#if sourceSearchSummary}
          <div class="source-search-summary" title={sourceSearchSummary}>{sourceSearchSummary}</div>
        {/if}

        {#if sourceSearchResults.length > 0}
          <div class="source-search-results" aria-label="Source content search results">
            {#each sourceSearchResults as result (`${result.path}:${result.line}:${result.column}`)}
              <button
                type="button"
                title={`${result.relativePath}:${result.line}:${result.column}`}
                onclick={() => selectSourceSearchResult(result)}
              >
                <span>
                  <strong>{result.fileName}</strong>
                  <small>{result.relativePath}:{result.line}:{result.column}</small>
                </span>
                <code>{result.excerpt}</code>
              </button>
            {/each}
          </div>
        {/if}
      </form>

      {#if projectRecentRecords.length > 0}
        <div class="recent-panel" aria-label="Recent source files">
          <div class="recent-heading">
            <span class="recent-heading-icon">
              <History size={15} strokeWidth={1.8} />
            </span>
            <span>Recent</span>
          </div>
          <div class="recent-list">
            {#each projectRecentRecords as recentRecord (recentRecord.path)}
              <button
                class:active={recentRecord.path === selectedRecord?.path}
                type="button"
                title={recentRecord.relativePath}
                onclick={() => selectRecentRecord(recentRecord)}
              >
                <span class="recent-file-icon">
                  <FileCode2 size={14} strokeWidth={1.8} />
                </span>
                <span>
                  <strong>{recentRecord.fileName}</strong>
                  <small>{recentRecord.relativePath}</small>
                </span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <div class="source-list-panel">
        <div class="tree-panel-header">
          <div class="tree-heading">
            <FolderGit2 size={15} strokeWidth={1.8} />
            <span>{selectedProject.name}</span>
            <strong>{recordCountLabel}</strong>
          </div>
          <div class="scan-summary-row">
            <div class="scan-summary" title={scanSummaryLabel}>{scanSummaryLabel}</div>
            <button
              class="scan-diagnostic-button"
              type="button"
              aria-label="Copy source scan diagnostic"
              title="Copy source scan diagnostic"
              onclick={copySourceScanDiagnosticBrief}
            >
              <Copy size={12} strokeWidth={1.9} />
            </button>
          </div>
          <div class="index-summary" title={selectedProjectIndexSummary}>{selectedProjectIndexSummary}</div>
          {#if sourceScanStatsLabel}
            <div class="scan-stats" title={sourceScanStatsLabel}>{sourceScanStatsLabel}</div>
          {/if}
          {#if sourceScanHealthNote}
            <div class="scan-health-note" title={sourceScanHealthNote}>
              <span>{sourceScanHealthNote}</span>
              <button
                type="button"
                aria-label={`Reset index and scan up to ${expandedSourceScanLimit.toLocaleString()} files`}
                title={`Reset index and scan up to ${expandedSourceScanLimit.toLocaleString()} files`}
                onclick={() => resetProjectScanCache(selectedProject)}
              >
                Reset
              </button>
            </div>
          {/if}
          {#if scanLimitReached && !scanning}
            <button
              class="scan-more-button"
              type="button"
              aria-label={`Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}
              title={`Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}
              onclick={() => scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })}
            >
              <Plus size={13} strokeWidth={2} />
              <span>Scan {expandedSourceScanLimitShortLabel}</span>
            </button>
          {/if}
        </div>

        <div
          class="file-tree"
          bind:this={fileTreeElement}
          onscroll={handleFileTreeScroll}
          aria-label="Files in selected project"
        >
          {#if scanning && records.length === 0}
            {#each Array.from({ length: 8 }) as _, index}
              <div class="tree-skeleton" style={`--line-width: ${index % 3 === 0 ? 72 : index % 2 === 0 ? 54 : 86}%`}></div>
            {/each}
          {:else if visibleTreeRows.length === 0}
            <div class="empty-tree">
              <FileCode2 size={17} strokeWidth={1.8} />
              <span>No source files found</span>
            </div>
          {:else}
            <div
              class="tree-virtual-spacer"
              aria-hidden="true"
              style={`--tree-spacer-height: ${virtualizedTreeRows.topSpacerHeight}px`}
            ></div>
            {#each virtualizedTreeRows.rows as row, virtualTreeRowIndex (row.node.id)}
              {@const node = row.node}
              {@const isFolder = node.file === null}
              {@const isExpanded = isFolderExpanded(node)}
              {@const gitStatus = node.file ? gitStatusForSourceRecord(node.file) : null}
              <button
                class:active={!isFolder && node.file?.path === selectedRecord?.path}
                class:folder-row={isFolder}
                class:file-row={!isFolder}
                type="button"
                style={`--tree-level: ${row.level}`}
                title={node.relativePath}
                aria-expanded={isFolder ? isExpanded : undefined}
                data-tree-row-index={virtualizedTreeRows.startIndex + virtualTreeRowIndex}
                onclick={() => selectTreeNode(node)}
                onkeydown={(event) => handleTreeRowKeydown(row, event)}
              >
                <span class="tree-indent"></span>
                <span class="tree-chevron">
                  {#if isFolder}
                    {#if isExpanded}
                      <ChevronDown size={13} strokeWidth={2} />
                    {:else}
                      <ChevronRight size={13} strokeWidth={2} />
                    {/if}
                  {/if}
                </span>
                <span class="tree-icon">
                  {#if isFolder}
                    {#if isExpanded}
                      <FolderOpen size={15} strokeWidth={1.8} />
                    {:else}
                      <Folder size={15} strokeWidth={1.8} />
                    {/if}
                  {:else}
                    <FileCode2 size={15} strokeWidth={1.8} />
                  {/if}
                </span>
                <strong>{node.name}</strong>
                {#if gitStatus}
                  <span class="git-status-badge" title={gitStatus.status}>{gitStatus.badge}</span>
                {/if}
                <small>{isFolder ? node.children.length : node.file?.language}</small>
              </button>
            {/each}
            <div
              class="tree-virtual-spacer"
              aria-hidden="true"
              style={`--tree-spacer-height: ${virtualizedTreeRows.bottomSpacerHeight}px`}
            ></div>
          {/if}
        </div>
      </div>
      </div>
    {:else}
      <div class="activity-panel" aria-label={sourceActivityPanelLabel}>
        <div class="activity-panel-header">
          <div>
            <strong>{sourceActivityPanelLabel}</strong>
            <span>{sourceActivitySummary(sourceActivityMode)}</span>
          </div>
          <button
            class="file-action-button"
            type="button"
            aria-label={`Refresh ${sourceActivityPanelLabel}`}
            title={`Refresh ${sourceActivityPanelLabel}`}
            disabled={sourceActivityRefreshing(sourceActivityMode)}
            onclick={() => refreshSourceActivityMode(sourceActivityMode)}
          >
            <RefreshCw size={14} strokeWidth={1.9} />
          </button>
        </div>

        {#if sourceActivityMode === 'clipboard'}
          <div class="paste-cleanup-panel">
            <div class="paste-cleanup-toolbar">
              <label>
                <span>Mode</span>
                <select bind:value={pasteCleanupMode} onchange={selectPasteCleanupMode} aria-label="Paste cleanup mode">
                  {#each pasteCleanupModes as mode (mode)}
                    <option value={mode}>{mode}</option>
                  {/each}
                </select>
              </label>
              <button
                class="file-action-button"
                type="button"
                aria-label="Read clipboard"
                title="Read clipboard"
                disabled={fileActionBusy === 'paste-read'}
                onclick={readPasteCleanupClipboard}
              >
                <Copy size={13} strokeWidth={1.9} />
                <span>{fileActionBusy === 'paste-read' ? 'Reading' : 'Paste'}</span>
              </button>
              <button
                class="file-action-button"
                type="button"
                aria-label="Clear paste cleanup text"
                title="Clear paste cleanup text"
                disabled={pasteCleanupInput.length === 0}
                onclick={() => (pasteCleanupInput = '')}
              >
                <X size={13} strokeWidth={1.9} />
                <span>Clear</span>
              </button>
            </div>
            <div class="paste-cleanup-grid">
              <label>
                <span>Input</span>
                <textarea
                  class="paste-cleanup-textarea"
                  bind:value={pasteCleanupInput}
                  aria-label="Paste cleanup input"
                  spellcheck="true"
                  placeholder="Paste text to clean"
                ></textarea>
              </label>
              <label>
                <span>Output</span>
                <textarea
                  class="paste-cleanup-textarea"
                  value={pasteCleanupOutput}
                  aria-label="Cleaned paste output"
                  readonly
                  spellcheck="false"
                ></textarea>
              </label>
            </div>
            <div class="paste-cleanup-footer">
              <span>{pasteCleanupStats}</span>
              <button
                class="file-action-button"
                type="button"
                aria-label="Copy cleaned paste output"
                title="Copy cleaned paste output"
                disabled={pasteCleanupOutput.trim().length === 0 || fileActionBusy === 'paste-copy'}
                onclick={copyPasteCleanupOutput}
              >
                <Copy size={13} strokeWidth={1.9} />
                <span>{fileActionBusy === 'paste-copy' ? 'Copying' : 'Copy'}</span>
              </button>
            </div>
          </div>
        {:else}
          <label class="activity-filter-box">
            <Search size={14} strokeWidth={1.9} />
            <input
              bind:value={sourceActivityFilter}
              type="search"
              autocomplete="off"
              spellcheck="false"
              aria-label="Filter workspace activity"
              placeholder={sourceActivityFilterPlaceholder(sourceActivityMode)}
            />
          </label>

          {#if sourceActivityMode === 'runs'}
          <div class="activity-panel-list" aria-label="Orchestration run list">
            {#if filteredProjectOrchestrationRuns.length === 0}
              <div class="activity-empty">No orchestration runs</div>
            {:else}
              {#each filteredProjectOrchestrationRuns as run (run.id)}
                {@const runMetrics = orchestrationRunMetrics(run)}
                {@const runStage = orchestrationRunStage(run, runMetrics)}
                {@const runLoopStages = orchestrationLoopStageMetrics(runMetrics)}
                {@const runAgentItems = orchestrationAgentActivityItems(run, 4)}
                {@const runAttentionItems = orchestrationAttentionQueue(run, 3)}
                {@const runTimeline = orchestrationTimelineItems(run, 6)}
                {@const runArtifacts = orchestrationArtifactChips(run)}
                {@const runLinks = orchestrationLinkChips(run)}
                <div
                  class="activity-run-row"
                  class:bad={orchestrationStatusClass(run.status) === 'bad'}
                  class:attention={orchestrationStatusClass(run.status) === 'attention' ||
                    runMetrics.attentionCount > 0 ||
                    runMetrics.approvalCount > 0}
                  class:live={orchestrationStatusClass(run.status) === 'live'}
                  title={orchestrationRunTitle(run)}
                >
                  <div class="run-row-heading">
                    <span class="run-heading-badges">
                      <span class={`run-status-badge ${orchestrationStatusClass(run.status)}`}>{run.status}</span>
                      <span class={`run-stage-badge ${runStage.tone}`} title={runStage.title}>
                        {runStage.label}
                      </span>
                    </span>
                    {#if run.taskID && orchestrationRunTaskUrl(run)}
                      <a
                        class="repo-task-link"
                        href={orchestrationRunTaskUrl(run) ?? ''}
                        target="_blank"
                        rel="noreferrer"
                      >
                        {run.taskID}
                      </a>
                    {/if}
                  </div>
                  <div class="activity-row-main">
                    <strong>{run.title}</strong>
                    <small>{run.phase} · {run.rootLabel} · {orchestrationRunTimeLabel(run)}</small>
                  </div>
                  <div class="run-current-activity" aria-label="Current run activity">
                    <span>Now</span>
                    <strong>{orchestrationCurrentActivity(run)}</strong>
                  </div>
                  {#if runAgentItems.length > 0}
                    <div class="run-agent-strip" aria-label="Run agent activity">
                      {#each runAgentItems as agent (agent.id)}
                        <span class={`run-agent-pill ${agent.tone}`} title={agent.title}>
                          <em>{agent.label}</em>
                          <strong>{agent.activity}</strong>
                        </span>
                      {/each}
                    </div>
                  {/if}
                  <div class="run-progress-track" aria-label={`Run progress ${run.progress}%`}>
                    <span style={`width: ${Math.max(0, Math.min(100, run.progress))}%`}></span>
                  </div>
                  <div class="run-metrics-row" aria-label="Run metrics">
                    <span>{runMetrics.agentCount} agents</span>
                    <span>{runMetrics.stepCount} steps</span>
                    <span>{runMetrics.eventCount} events</span>
                    <span>{runMetrics.artifactCount} artifacts</span>
                    {#if runMetrics.retryCount > 0}
                      <span>{runMetrics.retryCount} retries</span>
                    {/if}
                    {#if runMetrics.approvalCount > 0}
                      <span>{runMetrics.approvalCount} sign-off</span>
                    {/if}
                  </div>
                  <div class="run-loop-row" aria-label="Run loop tally">
                    <span>{orchestrationLoopTallyText(runMetrics)}</span>
                  </div>
                  {#if runAttentionItems.length > 0}
                    <div class="run-attention-queue" aria-label="Run decisions and blockers">
                      {#each runAttentionItems as item (item.id)}
                        <div class={`run-attention-item ${item.tone}`} title={item.summary || item.title}>
                          <span>{item.label}</span>
                          <div>
                            <strong>{item.title}</strong>
                            <small>{item.agentLabel || item.summary}</small>
                          </div>
                          {#if item.href}
                            <a href={item.href} target="_blank" rel="noreferrer" aria-label={`Open ${item.title}`}>
                              <ExternalLink size={11} strokeWidth={2} />
                            </a>
                          {/if}
                        </div>
                      {/each}
                    </div>
                  {/if}
                  <div class="run-loop-stage-strip" aria-label="Run loop stages">
                    {#each runLoopStages as stage (stage.id)}
                      <span class={`run-loop-stage ${stage.tone}`} title={stage.title}>
                        <em>{stage.label}</em>
                        <strong>{stage.value}</strong>
                      </span>
                    {/each}
                  </div>
                  {#if runTimeline.length > 0}
                    <div class="run-timeline" aria-label="Run timeline">
                      {#each runTimeline as item (item.id)}
                        <div class={`run-timeline-item ${item.tone}`}>
                          <span class={`run-step-marker ${item.tone}`}>
                            {orchestrationStepIconLabel(item.status)}
                          </span>
                          <div>
                            <strong>{item.title}</strong>
                            <small>{item.kind} · {item.summary}</small>
                          </div>
                          <em>{orchestrationTimelineTimeLabel(item)}</em>
                        </div>
                      {/each}
                    </div>
                  {/if}
                  {#if runArtifacts.length > 0 || runLinks.length > 0}
                    <div class="run-artifact-row" aria-label="Run artifacts and links">
                      {#each runArtifacts as artifact (artifact.id)}
                        {#if artifact.href}
                          <a class="run-chip artifact" href={artifact.href} target="_blank" rel="noreferrer">
                            {artifact.label}
                          </a>
                        {:else}
                          <span class="run-chip artifact" title={artifact.path ?? artifact.title}>
                            {artifact.label}
                          </span>
                        {/if}
                      {/each}
                      {#each runLinks as link (link.id)}
                        <a class="run-chip link" href={link.href ?? ''} target="_blank" rel="noreferrer">
                          {link.label}
                        </a>
                      {/each}
                    </div>
                  {/if}
                  <div class="activity-row-actions" aria-label="Run actions">
                    <button
                      type="button"
                      aria-label="Copy run summary"
                      title="Copy run summary"
                      onclick={() => copyOrchestrationRunSummary(run)}
                    >
                      <Activity size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Copy run handoff"
                      title="Copy run handoff"
                      onclick={() => copyOrchestrationRunHandoff(run)}
                    >
                      <FileCode2 size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Copy current run activity"
                      title="Copy current run activity"
                      onclick={() => copyOrchestrationCurrentActivity(run)}
                    >
                      <History size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Copy run id"
                      title="Copy run id"
                      onclick={() => copyActivityCommand(run.id, 'Run id copied')}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    {#if run.taskID}
                      <button
                        type="button"
                        aria-label="Copy run task reference"
                        title="Copy run task reference"
                        onclick={() => copyOrchestrationTaskReference(run)}
                      >
                        <ExternalLink size={12} strokeWidth={2} />
                      </button>
                    {/if}
                    {#if run.projectPath}
                      <button
                        type="button"
                        aria-label="Open run project path"
                        title="Open project path"
                        onclick={() => openActivityPath(run.projectPath)}
                      >
                        <ExternalLink size={12} strokeWidth={2} />
                      </button>
                      <button
                        type="button"
                        aria-label="Open run project in terminal"
                        title="Open project in terminal"
                        onclick={() => openActivityTerminalPath(run.projectPath)}
                      >
                        <Terminal size={12} strokeWidth={2} />
                      </button>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'conversations'}
          <div class="activity-panel-list" aria-label="Conversation list">
            <section class="workspace-snapshot-section" aria-label="Saved workspace snapshots">
              <div class="workspace-snapshot-toolbar">
                <div>
                  <strong>Saved Workspaces</strong>
                  <span>{workspaceSnapshots.length} saved</span>
                </div>
                <button
                  type="button"
                  aria-label="Save current workspace snapshot"
                  title="Save current workspace snapshot"
                  onclick={captureCurrentWorkspaceSnapshot}
                >
                  <Save size={12} strokeWidth={2} />
                </button>
              </div>
              {#if filteredWorkspaceSnapshots.length > 0}
                <div class="workspace-snapshot-list">
                  {#each filteredWorkspaceSnapshots as snapshot (snapshot.id)}
                    <div
                      class="workspace-snapshot-row"
                      class:active={activeWorkspaceSessionKey === snapshot.id}
                      title={snapshot.cwd}
                    >
                      <button
                        type="button"
                        aria-label={`Restore ${snapshot.title}`}
                        onclick={() => restoreConversationWorkspaceSnapshot(snapshot)}
                      >
                        <span class="agent-provider-badge">{snapshot.provider}</span>
                        <div class="activity-row-main">
                          <strong>{snapshot.title}</strong>
                          <small>
                            {snapshot.project.name} · {snapshot.model ?? snapshot.branch ?? snapshot.worktreePath ?? snapshot.cwd}
                          </small>
                        </div>
                      </button>
                      <div class="activity-row-actions" aria-label="Workspace snapshot actions">
                        <button
                          type="button"
                          aria-label="Resume workspace snapshot in embedded terminal"
                          title={
                            snapshot.resumeCommand
                              ? 'Resume workspace in embedded terminal'
                              : 'Open workspace shell in embedded terminal'
                          }
                          onclick={() => openWorkspaceSnapshotEmbeddedTerminal(snapshot)}
                        >
                          <Terminal size={12} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Copy workspace restore plan"
                          title="Copy restore plan"
                          onclick={() => copyWorkspaceSnapshotRestorePlan(snapshot)}
                        >
                          <FileCode2 size={12} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Copy workspace resume command"
                          title="Copy resume command"
                          disabled={!snapshot.resumeCommand}
                          onclick={() => copyActivityCommand(snapshot.resumeCommand ?? '', 'Resume command copied')}
                        >
                          <Copy size={12} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Open workspace path"
                          title="Open workspace path"
                          onclick={() => openActivityPath(snapshot.worktreePath ?? snapshot.cwd)}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Open workspace in terminal"
                          title="Open workspace in terminal"
                          onclick={() => openActivityTerminalPath(snapshot.worktreePath ?? snapshot.cwd)}
                        >
                          <Terminal size={12} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Delete workspace snapshot"
                          title="Delete workspace snapshot"
                          onclick={() => deleteWorkspaceSnapshot(snapshot)}
                        >
                          <Trash2 size={12} strokeWidth={2} />
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="activity-empty compact">No saved workspaces</div>
              {/if}
            </section>

            {#if filteredWorkspaceSnapshots.length === 0 && filteredProjectAgentSessions.length === 0}
              <div class="activity-empty">No conversations</div>
            {/if}
            {#if filteredProjectAgentSessions.length > 0}
              {#each filteredProjectAgentSessions as session, index (agentSessionRowKey(session, index, 'conversation'))}
                {@const sessionSnapshot = workspaceSnapshotForAgentSession(session)}
                <div
                  class="activity-session-row conversation-session-row"
                  class:active={activeWorkspaceSessionKey === workspaceSnapshotIDForAgentSession(session)}
                  title={agentSessionResumePlan(session)}
                >
                  <button
                    type="button"
                    class="conversation-session-open"
                    aria-label="Open conversation workspace"
                    onclick={() => openAgentSessionWorkspace(session)}
                  >
                    <span class="agent-provider-badge">{session.provider}</span>
                    <div class="activity-row-main">
                      <strong>{session.title}</strong>
                      <small>
                        {agentSessionProjectLabel(session)}
                        {#if session.model} · {agentSessionModelLabel(session)}{/if}
                        · {agentSessionActivityLabel(session)}
                      </small>
                    </div>
                  </button>
                  <div class="activity-row-actions" aria-label="Conversation actions">
                    <button
                      type="button"
                      aria-label="Save conversation workspace snapshot"
                      title="Save workspace snapshot"
                      onclick={() => captureAgentSessionWorkspaceSnapshot(session)}
                    >
                      <Save size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Restore conversation workspace"
                      title={sessionSnapshot ? 'Restore workspace snapshot' : 'No saved workspace snapshot'}
                      disabled={!sessionSnapshot}
                      onclick={() => restoreAgentSessionWorkspaceSnapshot(session)}
                    >
                      <RotateCcw size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Copy conversation workspace restore plan"
                      title={sessionSnapshot ? 'Copy workspace restore plan' : 'No saved workspace snapshot'}
                      disabled={!sessionSnapshot}
                      onclick={() => copyAgentSessionWorkspaceRestorePlan(session)}
                    >
                      <FileCode2 size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Copy agent resume command"
                      title="Copy resume command"
                      onclick={() => copyActivityCommand(agentSessionResumeCommand(session), 'Resume command copied')}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    {#if session.projectPath}
                      <button
                        type="button"
                        aria-label="Open agent project path"
                        title="Open project path"
                        onclick={() => openActivityPath(session.projectPath ?? '')}
                      >
                        <ExternalLink size={12} strokeWidth={2} />
                      </button>
                      <button
                        type="button"
                        aria-label="Resume agent in embedded terminal"
                        title="Resume in embedded terminal"
                        onclick={() => resumeAgentSessionEmbeddedTerminal(session)}
                      >
                        <Terminal size={12} strokeWidth={2} />
                      </button>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'sessions'}
          <div class="activity-panel-list" aria-label="Active session list">
            {#if filteredProjectRuntimeContexts.length === 0}
              <div class="activity-empty">No active sessions</div>
            {:else}
              {#each filteredProjectRuntimeContexts as context (`activity:${context.pid}:${context.port}:${context.cwd}`)}
                <div class="activity-runtime-row" title={context.cwd}>
                  <span class="runtime-port">:{context.port}</span>
                  <div class="activity-row-main">
                    <strong>{context.command}</strong>
                    <small>{context.rootLabel} · {context.cwd}</small>
                  </div>
                  <div class="activity-row-actions" aria-label="Active session actions">
                    <button
                      type="button"
                      aria-label="Copy active session command"
                      title="Copy session command"
                      onclick={() => copyActivityCommand(`${context.command} ${context.cwd}`, 'Session command copied')}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    <a
                      class="activity-icon-link"
                      href={runtimeContextUrl(context)}
                      target="_blank"
                      rel="noreferrer"
                      aria-label="Open active session URL"
                      title={runtimeContextUrl(context)}
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                    </a>
                    <button
                      type="button"
                      aria-label="Open active session path"
                      title="Open session path"
                      onclick={() => openActivityPath(context.cwd)}
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Open active session in terminal"
                      title="Open session in terminal"
                      onclick={() => openActivityTerminalPath(context.cwd)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'agents'}
          <div class="activity-panel-list" aria-label="Agent session list">
            {#if filteredProjectAgentSessions.length === 0}
              <div class="activity-empty">No agents</div>
            {:else}
              {#each filteredProjectAgentSessions as session, index (agentSessionRowKey(session, index, 'agent'))}
                <div class="activity-session-row" title={agentSessionResumePlan(session)}>
                  <span class="agent-provider-badge">{session.provider}</span>
                  <div class="activity-row-main">
                    <strong>{session.title}</strong>
                    <small>
                      {#if session.model}{agentSessionModelLabel(session)} · {/if}
                      {agentSessionResumeCommand(session)}
                    </small>
                  </div>
                  <div class="activity-row-actions" aria-label="Agent actions">
                    <button
                      type="button"
                      aria-label="Copy agent resume command"
                      title="Copy resume command"
                      onclick={() => copyActivityCommand(agentSessionResumeCommand(session), 'Resume command copied')}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    {#if session.projectPath}
                      <button
                        type="button"
                        aria-label="Reveal agent project path"
                        title="Reveal project path"
                        onclick={() => revealActivityPath(session.projectPath ?? '')}
                      >
                        <FolderSearch size={12} strokeWidth={2} />
                      </button>
	                      <button
	                        type="button"
	                        aria-label="Resume agent in terminal"
	                        title="Resume in terminal"
	                        onclick={() => openAgentSessionTerminal(session)}
	                      >
	                        <Terminal size={12} strokeWidth={2} />
	                      </button>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'worktrees'}
          <div class="activity-panel-list" aria-label="Worktree list">
            {#if filteredProjectWorktrees.length === 0}
              <div class="activity-empty">No worktrees</div>
            {:else}
              {#each filteredProjectWorktrees as worktree (`activity:${worktree.path}`)}
                {@const safety = projectWorktreeSafety(worktree)}
                {@const primaryAction = projectWorktreePrimaryAction(worktree)}
                {@const eligibilityKind = projectWorktreeEligibilityKind(worktree)}
                <div
                  class="activity-worktree-row"
                  class:blocked={eligibilityKind === 'blocked'}
                  class:protected={eligibilityKind === 'protected'}
                  class:ready={eligibilityKind === 'ready'}
                  title={safety.cleanupPlan}
                >
                  <span class={`worktree-status-badge ${safety.kind}`}>{safety.badge}</span>
                  <div class="activity-row-main worktree-row-main">
                    <strong>
                      <span>{worktree.branch}</span>
                      {#if worktree.taskID && gitTaskUrl(worktree.taskID)}
                        <a
                          class="git-task-link"
                          href={gitTaskUrl(worktree.taskID) ?? ''}
                          target="_blank"
                          rel="noreferrer"
                          aria-label="Open worktree task"
                        >
                          {worktree.taskID}
                        </a>
                      {/if}
                    </strong>
	                    <small class="worktree-safety-line">
	                      <span>{safety.reason}</span>
	                      {#if safety.activeSessionCount > 0}
	                        <span>
	                          {safety.activeSessionCount}
	                          {safety.activeSessionCount === 1 ? 'session' : 'sessions'}
	                        </span>
	                      {/if}
	                      <span>{projectWorktreeActivityLabel(worktree)}</span>
	                    </small>
                    <small class="worktree-recommendation">{safety.recommendation}</small>
                  </div>
                  <div class="activity-row-actions" aria-label="Worktree actions">
                    <button
                      type="button"
                      aria-label="Copy worktree cleanup plan"
                      title="Copy cleanup plan"
                      onclick={() => copyWorktreeCleanupPlan(worktree)}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    <button
                      class={`worktree-primary-action ${primaryAction.kind}`}
                      type="button"
                      aria-label={`${primaryAction.label} worktree: ${worktree.branch}`}
                      title={primaryAction.title}
                      disabled={fileActionBusy === `worktree-primary:${worktree.path}`}
                      onclick={() => runWorktreePrimaryAction(worktree)}
                    >
                      {#if primaryAction.kind === 'cleanup'}
                        <Trash2 size={12} strokeWidth={2} />
                      {:else if primaryAction.kind === 'backup'}
                        <Save size={12} strokeWidth={2} />
                      {:else}
                        <History size={12} strokeWidth={2} />
                      {/if}
                    </button>
                    <button
                      type="button"
                      aria-label="Open worktree in source browser"
                      title="Open worktree in source browser"
                      onclick={() => openWorktreeInSourceBrowser(worktree)}
                    >
                      <FolderOpen size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Copy worktree path"
                      title="Copy worktree path"
                      onclick={() => copyActivityCommand(worktree.path, 'Worktree path copied')}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Open worktree path"
                      title="Open worktree path"
                      onclick={() => openActivityPath(worktree.path)}
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Open worktree in terminal"
                      title="Open worktree in terminal"
                      onclick={() => openActivityTerminalPath(worktree.path)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Open worktree in embedded terminal"
                      title="Open worktree in embedded terminal"
                      onclick={() => openPathEmbeddedTerminal(worktree.path)}
                    >
                      <PanelBottom size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Reveal worktree path"
                      title="Reveal worktree path"
                      onclick={() => revealActivityPath(worktree.path)}
                    >
                      <FolderSearch size={12} strokeWidth={2} />
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'git'}
          <div class="activity-panel-list" aria-label="Git and task list">
            {#if filteredGitRepositorySummaries.length === 0}
              <div class="activity-empty">No repositories</div>
            {:else}
              {#each filteredGitRepositorySummaries as summary (`activity:${summary.projectID}:${summary.path}`)}
                <div
                  class="activity-repo-row"
                  class:dirty={summary.isDirty || summary.error}
                  title={repoDashboardTitle(summary)}
                >
                  <div class="activity-row-main">
                    <strong>{summary.projectName}</strong>
                    <small>{summary.rootLabel}</small>
                  </div>
                  <span class="repo-branch-badge">{summary.branch}</span>
                  {#if summary.taskID && repoDashboardTaskUrl(summary)}
                    <a
                      class="repo-task-link"
                      href={repoDashboardTaskUrl(summary) ?? ''}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {summary.taskID}
                    </a>
                  {:else}
                    <span class="repo-branch-badge">{repoDashboardTaskLabel(summary)}</span>
                  {/if}
                  <small>{repoDashboardDirtyLabel(summary)} · {repoDashboardRemoteLabel(summary)}</small>
                  <div class="activity-row-actions" aria-label="Repository actions">
                    <button
                      type="button"
                      aria-label="Copy repository path"
                      title="Copy repository path"
                      onclick={() => copyActivityCommand(summary.path, 'Repository path copied')}
                    >
                      <Copy size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Open repository path"
                      title="Open repository path"
                      onclick={() => openActivityPath(summary.path)}
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Open repository in terminal"
                      title="Open repository in terminal"
                      onclick={() => openActivityTerminalPath(summary.path)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Reveal repository path"
                      title="Reveal repository path"
                      onclick={() => revealActivityPath(summary.path)}
                    >
                      <FolderSearch size={12} strokeWidth={2} />
                    </button>
                  </div>
                </div>
              {/each}
            {/if}

            <div class="activity-subheading">Recent commits</div>
            {#if filteredGitCommitHistory.length === 0}
              <div class="activity-empty">No commits</div>
            {:else}
              {#each filteredGitCommitHistory.slice(0, 8) as entry, index (entry.sha)}
                <div class="activity-commit-row" title={gitCommitTitle(entry)}>
                  <span
                    class={`git-graph-marker ${gitCommitGraphClass(entry, index)}`}
                    aria-label={gitCommitTopology(entry, index)}
                    title={gitCommitTopology(entry, index)}
                  ></span>
                  <div class="activity-row-main">
                    <strong>{entry.subject}</strong>
                    <small>{entry.shortSha} · {formatGitCommitTime(entry.committedAt)}</small>
                  </div>
                  <div class="activity-commit-meta">
                    {#if entry.taskID && gitTaskUrl(entry.taskID)}
                      <a
                        class="git-task-link"
                        href={gitTaskUrl(entry.taskID) ?? ''}
                        target="_blank"
                        rel="noreferrer"
                        title={`Task from ${gitCommitTaskSourceLabel(entry) || 'Git metadata'}`}
                      >
                        {entry.taskID}
                      </a>
                    {/if}
                    <div class="activity-row-actions" aria-label="Commit actions">
                      <button
                        type="button"
                        aria-label="Copy commit SHA"
                        title="Copy commit SHA"
                        onclick={() => copyGitCommitSha(entry)}
                      >
                        <Copy size={12} strokeWidth={2} />
                      </button>
                      <button
                        type="button"
                        aria-label="Copy commit summary"
                        title="Copy commit summary"
                        onclick={() => copyGitCommitSummary(entry)}
                      >
                        <History size={12} strokeWidth={2} />
                      </button>
                      {#if entry.taskID}
                        <button
                          type="button"
                          aria-label="Copy task reference"
                          title="Copy task reference"
                          onclick={() => copyGitTaskReference(entry.taskID)}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                        </button>
                      {/if}
                    </div>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
          {/if}
        {/if}
      </div>
    {/if}
    </div>
  </aside>

  <button
    class="side-pane-resizer"
    type="button"
    aria-label="Resize side pane"
    title="Resize side pane"
    onpointerdown={beginSidePaneResize}
    onkeydown={handleSidePaneResizerKeydown}
  ></button>

  <section
    class="workspace"
    class:context-side={contextPanelPlacement === 'side' && shouldRenderDockPanel('context')}
    class:context-bottom={contextPanelPlacement === 'bottom' && shouldRenderDockPanel('context')}
    aria-label="Source preview"
  >
    <header class="topbar">
      <div>
        <p class="eyebrow">Source Preview</p>
        <h2>{preview?.fileName ?? 'No file selected'}</h2>
      </div>
      <div class="topbar-tools">
        <button
          class="topbar-command-button"
          type="button"
          aria-label="Open command palette"
          title="Command palette (Cmd+K)"
          onclick={openCommandPalette}
        >
          <Search size={13} strokeWidth={2} />
          <span>Cmd+K</span>
        </button>

        <div class="view-menu-anchor">
          <button
            class="topbar-command-button view-menu-button"
            type="button"
            aria-label="View menu"
            aria-haspopup="menu"
            aria-expanded={viewMenuOpen}
            title="View menu"
            onclick={toggleViewMenu}
          >
            <MoreHorizontal size={15} strokeWidth={2} />
            <span>View</span>
          </button>

          {#if viewMenuOpen}
            <div class="view-menu" role="menu" aria-label="View options">
              <section class="view-menu-section" aria-label="Workspace layout presets">
                <span>Layout</span>
                <div class="view-menu-button-grid">
                  {#each sourceLayoutPresets as preset (preset.id)}
                    <button
                      class:active={sourceLayoutPreset === preset.id}
                      type="button"
                      role="menuitem"
                      aria-label={`Use ${preset.label} layout`}
                      title={preset.title}
                      onclick={() => {
                        applySourceLayoutPreset(preset.id);
                        closeViewMenu();
                      }}
                    >
                      {preset.label}
                    </button>
                  {/each}
                </div>
                <div class="view-menu-button-grid two">
                  <button
                    type="button"
                    role="menuitem"
                    aria-label="Save current layout preset"
                    title={`Save current arrangement as ${currentSaveableSourceLayoutPreset().label}`}
                    onclick={() => {
                      saveSourceLayoutPresetOverride(currentSaveableSourceLayoutPreset().id);
                      closeViewMenu();
                    }}
                  >
                    Save
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    aria-label="Reset saved layout preset"
                    title={`Reset saved ${currentSaveableSourceLayoutPreset().label} layout`}
                    disabled={!sourceLayoutPresetOverrides[currentSaveableSourceLayoutPreset().id]}
                    onclick={() => {
                      resetSourceLayoutPresetOverride(currentSaveableSourceLayoutPreset().id);
                      closeViewMenu();
                    }}
                  >
                    Reset
                  </button>
                </div>
              </section>

              <section class="view-menu-section" aria-label="Side pane position">
                <span>Explorer</span>
                <div class="view-menu-button-grid two">
                  <button
                    class:active={sidePanePosition === 'left'}
                    type="button"
                    role="menuitem"
                    aria-label="Put side pane on the left"
                    onclick={() => {
                      selectSidePanePosition('left');
                      closeViewMenu();
                    }}
                  >
                    Left
                  </button>
                  <button
                    class:active={sidePanePosition === 'right'}
                    type="button"
                    role="menuitem"
                    aria-label="Put side pane on the right"
                    onclick={() => {
                      selectSidePanePosition('right');
                      closeViewMenu();
                    }}
                  >
                    Right
                  </button>
                </div>
              </section>

              <section class="view-menu-section" aria-label="Context card layout">
                <span>Context</span>
                <div class="view-menu-button-grid two">
                  <button
                    class:active={contextPanelPlacement === 'top'}
                    type="button"
                    role="menuitem"
                    aria-label="Put context above editor"
                    onclick={() => {
                      selectContextPanelPlacement('top');
                      closeViewMenu();
                    }}
                  >
                    Top
                  </button>
                  <button
                    class:active={contextPanelPlacement === 'side'}
                    type="button"
                    role="menuitem"
                    aria-label="Put context beside editor"
                    onclick={() => {
                      selectContextPanelPlacement('side');
                      closeViewMenu();
                    }}
                  >
                    Side
                  </button>
                  <button
                    class:active={contextPanelPlacement === 'bottom'}
                    type="button"
                    role="menuitem"
                    aria-label="Put context below editor"
                    onclick={() => {
                      moveDockPanelToGroup('context', 'bottom');
                      closeViewMenu();
                    }}
                  >
                    Bottom
                  </button>
                  <button
                    class:active={contextPanelMode === 'grid'}
                    type="button"
                    role="menuitem"
                    aria-label="Use grid context cards"
                    onclick={() => {
                      selectContextPanelMode('grid');
                      closeViewMenu();
                    }}
                  >
                    Grid
                  </button>
                  <button
                    class:active={contextPanelMode === 'stack'}
                    type="button"
                    role="menuitem"
                    aria-label="Use stacked context cards"
                    onclick={() => {
                      selectContextPanelMode('stack');
                      closeViewMenu();
                    }}
                  >
                    Stack
                  </button>
                </div>
                <button
                  class="view-menu-wide-button"
                  type="button"
                  role="menuitem"
                  onclick={() => {
                    toggleContextPanelCollapsed();
                    closeViewMenu();
                  }}
                >
                  {contextPanelCollapsed ? 'Show context cards' : 'Hide context cards'}
                </button>
              </section>

              <section class="view-menu-section dock-panel-manager" aria-label="Dock panels">
                <span>Panels</span>
                <div class="dock-panel-manager-list">
                  {#each managedDockPanelIDs as panelID (panelID)}
                    <div class="dock-panel-manager-row">
                      <div>
                        <strong>{dockPanelLabel(panelID)}</strong>
                        <small>{dockPanelPlacementSummary(panelID)}</small>
                      </div>
                      <div class="dock-panel-manager-actions">
                        {#if dockPanelCanHide(panelID)}
                          <button
                            class:active={sourceDockPanelVisible(panelID)}
                            type="button"
                            role="menuitem"
                            aria-label={`${sourceDockPanelVisible(panelID) ? 'Hide' : 'Show'} ${dockPanelLabel(panelID)} panel`}
                            onclick={() => toggleDockPanelVisibility(panelID)}
                          >
                            {sourceDockPanelVisible(panelID) ? 'Hide' : 'Show'}
                          </button>
                        {/if}
                        {#each dockPanelMoveTargets(panelID) as groupID (groupID)}
                          <button
                            class:active={dockGroupIDForPanel(sourceDockLayout, panelID) === groupID}
                            type="button"
                            role="menuitem"
                            aria-label={`Move ${dockPanelLabel(panelID)} to ${dockGroupLabel(groupID, panelID)}`}
                            disabled={dockGroupIDForPanel(sourceDockLayout, panelID) === groupID}
                            onclick={() => moveDockPanelToManagedGroup(panelID, groupID)}
                          >
                            {dockGroupLabel(groupID, panelID)}
                          </button>
                        {/each}
                      </div>
                    </div>
                  {/each}
                </div>
                <button
                  class="view-menu-wide-button"
                  type="button"
                  role="menuitem"
                  onclick={() => {
                    resetSourceDockLayout();
                    closeViewMenu();
                  }}
                >
                  Reset dock layout
                </button>
              </section>

              <label class="view-terminal-picker" title={`Open directories in ${sourceTerminalApp}`}>
                <span>Terminal</span>
                <select
                  bind:value={sourceTerminalApp}
                  aria-label="Terminal app"
                  onchange={selectSourceTerminalApp}
                >
                  {#each sourceTerminalApps as app (app)}
                    <option value={app}>{app}</option>
                  {/each}
                </select>
              </label>
            </div>
          {/if}
        </div>
      </div>
    </header>

    <div class="context-identity-strip" aria-label="Current source context" title={sourceContextIdentity.summary}>
      <span class="context-identity-item">
        <span class="context-identity-key">Prj</span>
        <strong class="context-identity-value">{sourceContextIdentity.projectName}</strong>
      </span>
      <span class="context-identity-item" title={sourceContextIdentity.rootPath}>
        <span class="context-identity-key">Root</span>
        <strong class="context-identity-value">{sourceContextIdentity.rootLabel}</strong>
      </span>
      <span class="context-identity-item" title={sourceContextIdentity.gitSummary}>
        <span class="context-identity-key">Git</span>
        <strong class="context-identity-value">{sourceContextIdentity.gitSummary}</strong>
      </span>
      <span class="context-identity-item">
        <span class="context-identity-key">Run</span>
        <strong class="context-identity-value">{sourceContextIdentity.runtime}</strong>
      </span>
    </div>

      <div
        class="dock-panel-tabs"
        class:empty={!hasDockPanelTabs()}
        aria-label="Stacked dock panels"
      >
        {#each dockTabGroupIDs as groupID (groupID)}
          {#if dockGroupHasTabs(groupID)}
            <div class="dock-panel-tab-group" role="tablist" aria-label={`${groupID} dock panels`}>
              {#each dockGroupPanelIDs(groupID) as panelID (panelID)}
                <div
                  class="dock-panel-tab"
                  class:active={activeDockPanelForGroup(groupID) === panelID}
                  role="presentation"
                  draggable={dockPanelMoveTargets(panelID).length > 1}
                  ondragstart={(event) => beginDockPanelDrag(panelID, event)}
                  ondragend={clearDockPanelDrag}
                >
                  <button
                    class="dock-panel-tab-label"
                    type="button"
                    role="tab"
                    aria-selected={activeDockPanelForGroup(groupID) === panelID}
                    aria-label={`Show ${dockPanelLabel(panelID)} panel`}
                    onclick={() => selectDockPanel(panelID)}
                  >
                    {dockPanelLabel(panelID)}
                  </button>
                  {#if dockPanelMoveTargets(panelID).length > 1}
                    <select
                      class="dock-panel-tab-move"
                      aria-label={`Move ${dockPanelLabel(panelID)} panel from tab`}
                      title={`Move ${dockPanelLabel(panelID)}`}
                      value={dockGroupIDForPanel(sourceDockLayout, panelID) ?? ''}
                      onchange={(event) => moveDockPanelFromTab(panelID, event)}
                    >
                      {#each dockPanelMoveTargets(panelID) as targetGroupID (targetGroupID)}
                        <option
                          value={targetGroupID}
                          disabled={dockGroupIDForPanel(sourceDockLayout, panelID) === targetGroupID}
                        >
                          {dockGroupShortcutLabel(targetGroupID, panelID)}
                        </option>
                      {/each}
                    </select>
                  {/if}
                  {#if dockPanelCanHide(panelID)}
                    <button
                      class="dock-panel-tab-close"
                      type="button"
                      aria-label={`Hide ${dockPanelLabel(panelID)} panel from tab`}
                      title={`Hide ${dockPanelLabel(panelID)}`}
                      onclick={() => hideDockPanel(panelID)}
                    >
                      <X size={10} strokeWidth={2} />
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        {/each}
      </div>

      {#if draggingDockPanelID}
        <div
          class="dock-drop-zones"
          aria-label={`Dock targets for ${dockPanelLabel(draggingDockPanelID)}`}
        >
          {#each dockPanelMoveTargets(draggingDockPanelID) as groupID (groupID)}
            <button
              class="dock-drop-zone"
              class:active={dockDropTargetGroupID === groupID}
              type="button"
              ondragenter={(event) => dragOverDockDropZone(event, groupID)}
              ondragover={(event) => dragOverDockDropZone(event, groupID)}
              ondragleave={() => (dockDropTargetGroupID = null)}
              ondrop={(event) => dropDockPanelOnGroup(event, groupID)}
              onclick={() => {
                if (!draggingDockPanelID) return;
                moveDockPanelToManagedGroup(draggingDockPanelID, groupID);
                clearDockPanelDrag();
              }}
            >
              {dockGroupLabel(groupID, draggingDockPanelID)}
            </button>
          {/each}
        </div>
      {/if}

      <div
      class="workspace-arrangement"
      class:context-top={contextPanelPlacement === 'top' && shouldRenderDockPanel('context')}
      class:context-side={contextPanelPlacement === 'side' && shouldRenderDockPanel('context')}
      class:context-bottom={contextPanelPlacement === 'bottom' && shouldRenderDockPanel('context')}
    >
      {#if shouldRenderDockPanel('context')}
      <div class="workspace-context-column">
    <div class="context-panel-grid" class:collapsed={contextPanelCollapsed} class:stacked={contextPanelMode === 'stack'}>
      {#if hiddenContextCardIDs.size > 0}
        <button class="context-restore-button" type="button" onclick={showAllContextCards}>
          Show hidden cards
        </button>
      {/if}
      {#if contextPanelMode === 'stack' && visibleContextCards.length > 0}
        <div class="context-stack-tabs" aria-label="Context card tabs">
          {#each visibleContextCards as cardID (cardID)}
            <button
              class:active={activeContextCard === cardID}
              type="button"
              aria-label={`Show ${contextCardLabels[cardID]} context`}
              title={contextCardLabels[cardID]}
              onclick={() => selectActiveContextCard(cardID)}
            >
              {contextCardLabels[cardID]}
            </button>
          {/each}
        </div>
      {/if}
      {#if shouldRenderContextCard('orchestration')}
      <section class="orchestration-context-panel" aria-label="Orchestration runs">
        <div class="orchestration-context-header">
          <div>
            <strong>Orchestration Runs</strong>
            <span>{orchestrationRunSummary}</span>
          </div>
          <div class="context-card-actions">
            <button
              class="file-action-button"
              type="button"
              aria-label="Refresh orchestration runs"
              title="Refresh orchestration runs"
              disabled={orchestrationRunsLoading}
              onclick={() => loadOrchestrationRuns(projectOptions)}
            >
              <RefreshCw size={14} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button context-card-close"
              type="button"
              aria-label="Hide orchestration runs"
              title="Hide orchestration runs"
              onclick={() => hideContextCard('orchestration')}
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
        </div>
        {#if selectedProjectOrchestrationRuns.length > 0}
          <div class="orchestration-context-list">
            {#each selectedProjectOrchestrationRuns.slice(0, 3) as run (run.id)}
              {@const runMetrics = orchestrationRunMetrics(run)}
              <div
                class="orchestration-context-row"
                class:bad={orchestrationStatusClass(run.status) === 'bad'}
                class:attention={orchestrationStatusClass(run.status) === 'attention' ||
                  runMetrics.attentionCount > 0 ||
                  runMetrics.approvalCount > 0}
              >
                <span class={`run-status-badge ${orchestrationStatusClass(run.status)}`}>{run.status}</span>
                <strong>{run.title}</strong>
                <span title={orchestrationLoopTallyText(runMetrics)}>
                  {run.phase} · {run.progress}% · {orchestrationLoopTallyText(runMetrics)}
                </span>
                <small>{orchestrationCurrentActivity(run)}</small>
              </div>
            {/each}
          </div>
        {/if}
      </section>
      {/if}

      {#if shouldRenderContextCard('runtime')}
      <section class="runtime-context-panel" aria-label="Runtime contexts">
        <div class="runtime-context-header">
          <div>
            <strong>Runtime Contexts</strong>
            <span>{runtimeContextSummary}</span>
          </div>
          <div class="context-card-actions">
            <button
              class="file-action-button"
              type="button"
              aria-label="Refresh runtime contexts"
              title="Refresh runtime contexts"
              disabled={runtimeContextsLoading}
              onclick={() => loadRuntimeContexts(projectOptions)}
            >
              <RefreshCw size={14} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button context-card-close"
              type="button"
              aria-label="Hide runtime contexts"
              title="Hide runtime contexts"
              onclick={() => hideContextCard('runtime')}
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
        </div>
        {#if selectedProjectRuntimeContexts.length > 0}
          <div class="runtime-context-list">
            {#each selectedProjectRuntimeContexts as context (`${context.pid}:${context.port}:${context.cwd}`)}
              <div class="runtime-context-row">
                <span class="runtime-port">:{context.port}</span>
                <a
                  class="runtime-url-link"
                  href={runtimeContextUrl(context)}
                  target="_blank"
                  rel="noreferrer"
                  aria-label="Open runtime URL"
                  title={runtimeContextUrl(context)}
                >
                  localhost
                </a>
                <strong>{context.command}</strong>
                <span>{context.rootLabel}</span>
                <small title={context.cwd}>{context.cwd}</small>
              </div>
            {/each}
          </div>
        {/if}
      </section>
      {/if}

      {#if shouldRenderContextCard('agents')}
      <section class="agent-session-panel" aria-label="Agent sessions">
        <div class="agent-session-header">
          <div>
            <strong>Agent Sessions</strong>
            <span>{agentSessionSummary}</span>
          </div>
          <div class="context-card-actions">
            <button
              class="file-action-button"
              type="button"
              aria-label="Refresh agent sessions"
              title="Refresh agent sessions"
              disabled={agentSessionsLoading}
              onclick={loadAgentSessions}
            >
              <RefreshCw size={14} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button context-card-close"
              type="button"
              aria-label="Hide agent sessions"
              title="Hide agent sessions"
              onclick={() => hideContextCard('agents')}
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
        </div>
        {#if selectedProjectAgentSessions.length > 0}
          <div class="agent-session-list">
            {#each selectedProjectAgentSessions as session, index (agentSessionRowKey(session, index, 'context'))}
              <div class="agent-session-row" title={agentSessionResumePlan(session)}>
                <span class="agent-provider-badge">{session.provider}</span>
                <strong>{session.title}</strong>
                <span>{agentSessionProjectLabel(session)}</span>
                {#if session.model}<span>{agentSessionModelLabel(session)}</span>{/if}
                <small>{agentSessionActivityLabel(session)}</small>
              </div>
            {/each}
          </div>
        {/if}
      </section>
      {/if}

      {#if shouldRenderContextCard('worktrees')}
      <section class="worktree-context-panel" aria-label="Worktree safety">
        <div class="worktree-context-header">
          <div>
            <strong>Worktree Safety</strong>
            <span title={projectWorktreeSafetyStats}>{projectWorktreeSummary} · {projectWorktreeCleanupBrief.headline}</span>
          </div>
          <div class="context-card-actions">
            <button
              class="file-action-button"
              type="button"
              aria-label="Copy worktree cleanup brief"
              title="Copy cleanup brief"
              disabled={projectWorktrees.length === 0}
              onclick={copyProjectWorktreeCleanupBrief}
            >
              <Copy size={13} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button"
              type="button"
              aria-label="Copy worktree cleanup script"
              title="Copy guarded cleanup script"
              disabled={projectWorktrees.length === 0}
              onclick={copyProjectWorktreeCleanupScript}
            >
              <FileCode2 size={13} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button"
              type="button"
              aria-label="Refresh worktrees"
              title="Refresh worktrees"
              disabled={projectWorktreesLoading}
              onclick={() => loadProjectWorktrees(selectedProject)}
            >
              <RefreshCw size={14} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button context-card-close"
              type="button"
              aria-label="Hide worktree safety"
              title="Hide worktree safety"
              onclick={() => hideContextCard('worktrees')}
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
        </div>
        {#if projectWorktrees.length > 0}
          <div class="worktree-context-list">
            {#each prioritizedProjectWorktrees as worktree (worktree.path)}
              {@const safety = projectWorktreeSafety(worktree)}
              {@const primaryAction = projectWorktreePrimaryAction(worktree)}
              {@const eligibilityKind = projectWorktreeEligibilityKind(worktree)}
              <div
                class="worktree-context-row"
                class:blocked={eligibilityKind === 'blocked'}
                class:protected={eligibilityKind === 'protected'}
                class:ready={eligibilityKind === 'ready'}
                title={safety.cleanupPlan}
              >
                <span class={`worktree-status-badge ${safety.kind}`}>{safety.badge}</span>
                <div class="worktree-context-main">
                  <strong>{worktree.branch}</strong>
                  <small title={worktree.path}>{worktree.path}</small>
                </div>
                <span>{worktree.repo}</span>
                {#if worktree.taskID && gitTaskUrl(worktree.taskID)}
                  <a
                    class="git-task-link"
                    href={gitTaskUrl(worktree.taskID) ?? ''}
                    target="_blank"
                    rel="noreferrer"
                    aria-label="Open worktree task"
                  >
                    {worktree.taskID}
                  </a>
                {:else}
                  <span class="worktree-task-empty">no task</span>
                {/if}
	                <em>
	                  {safety.reason}
	                  {#if safety.activeSessionCount > 0}
	                    · {safety.activeSessionCount}
	                    {safety.activeSessionCount === 1 ? 'session' : 'sessions'}
	                  {/if}
	                  · {projectWorktreeActivityLabel(worktree)}
	                </em>
                <small class="worktree-recommendation">{safety.recommendation}</small>
                <div class="worktree-context-actions" aria-label="Worktree cleanup actions">
                  <button
                    type="button"
                    aria-label="Copy worktree cleanup plan"
                    title="Copy cleanup plan"
                    onclick={() => copyWorktreeCleanupPlan(worktree)}
                  >
                    <Copy size={12} strokeWidth={2} />
                  </button>
                  <button
                    class={`worktree-primary-action ${primaryAction.kind}`}
                    type="button"
                    aria-label={`${primaryAction.label} worktree: ${worktree.branch}`}
                    title={primaryAction.title}
                    disabled={fileActionBusy === `worktree-primary:${worktree.path}`}
                    onclick={() => runWorktreePrimaryAction(worktree)}
                  >
                    {#if primaryAction.kind === 'cleanup'}
                      <Trash2 size={12} strokeWidth={2} />
                    {:else if primaryAction.kind === 'backup'}
                      <Save size={12} strokeWidth={2} />
                    {:else}
                      <History size={12} strokeWidth={2} />
                    {/if}
                  </button>
                  <button
                    type="button"
                    aria-label="Open worktree in embedded terminal"
                    title="Open worktree in embedded terminal"
                    onclick={() => openPathEmbeddedTerminal(worktree.path)}
                  >
                    <PanelBottom size={12} strokeWidth={2} />
                  </button>
                  <button
                    type="button"
                    aria-label="Open worktree in source browser"
                    title="Open worktree in source browser"
                    onclick={() => openWorktreeInSourceBrowser(worktree)}
                  >
                    <FolderOpen size={12} strokeWidth={2} />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
      {/if}

      {#if shouldRenderContextCard('repo')}
      <section class="repo-dashboard-panel" aria-label="Repository dashboard">
        <div class="repo-dashboard-header">
          <div>
            <strong>Repo Dashboard</strong>
            <span>{repoDashboardSummary}</span>
          </div>
          <div class="context-card-actions">
            <button
              class="file-action-button"
              type="button"
              aria-label="Refresh repository dashboard"
              title="Refresh repository dashboard"
              disabled={gitRepositorySummariesLoading}
              onclick={() => loadGitRepositorySummaries(projectOptions)}
            >
              <RefreshCw size={14} strokeWidth={1.9} />
            </button>
            <button
              class="file-action-button context-card-close"
              type="button"
              aria-label="Hide repository dashboard"
              title="Hide repository dashboard"
              onclick={() => hideContextCard('repo')}
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
        </div>
        {#if gitRepositorySummaries.length > 0}
          <div class="repo-dashboard-list">
            {#each gitRepositorySummaries as summary (`${summary.projectID}:${summary.path}`)}
              <div
                class="repo-dashboard-row"
                class:dirty={summary.isDirty || summary.error}
                title={repoDashboardTitle(summary)}
              >
                <div class="repo-dashboard-main">
                  <strong>{summary.projectName}</strong>
                  <small>{summary.rootLabel}</small>
                </div>
                <span class="repo-branch-badge">{summary.branch}</span>
                <div class="repo-dashboard-metric">
                  <span>Task</span>
                  {#if summary.taskID && repoDashboardTaskUrl(summary)}
                    <a
                      class="repo-task-link"
                      href={repoDashboardTaskUrl(summary) ?? ''}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {summary.taskID}
                    </a>
                  {:else}
                    <strong>{repoDashboardTaskLabel(summary)}</strong>
                  {/if}
                </div>
                <div class="repo-dashboard-metric">
                  <span>Dirty</span>
                  <strong>{repoDashboardDirtyLabel(summary)}</strong>
                </div>
                <em>{repoDashboardRemoteLabel(summary)}</em>
              </div>
            {/each}
          </div>
        {/if}
      </section>
      {/if}
    </div>
      </div>

      <button
        class="context-pane-resizer"
        type="button"
        aria-label="Resize context pane"
        title="Resize context pane"
        onpointerdown={beginContextPaneResize}
        onkeydown={handleContextPaneResizerKeydown}
      ></button>
      {/if}

      <div class="workspace-main-column">

    {#if projectOpenSourceTabs.length > 0}
      <div class="tab-strip" aria-label="Open source files">
        {#each projectOpenSourceTabs as tab (tab.path)}
          {@const tabGitStatus = gitStatusForSourceRecord(tab)}
          <div
            class="source-tab"
            class:active={tab.path === selectedRecord?.path}
            class:dirty={isSourcePathDirty(tab.path)}
          >
            <button
              class="tab-select-button"
              type="button"
              aria-label={`Select ${tab.fileName}`}
              title={tab.relativePath}
              onclick={() => selectOpenTab(tab)}
            >
              <span class="tab-file-icon">
                <FileCode2 size={14} strokeWidth={1.8} />
              </span>
              <span>{tab.fileName}</span>
              <small>{tab.language}</small>
              {#if tabGitStatus}
                <span class="git-status-badge" title={tabGitStatus.status}>{tabGitStatus.badge}</span>
              {/if}
              {#if isSourcePathDirty(tab.path)}
                <span class="tab-dirty-dot" aria-hidden="true"></span>
              {/if}
            </button>
            <button
              class="tab-close-button"
              type="button"
              aria-label={`Close ${tab.fileName}`}
              title={`Close ${tab.fileName}`}
              onclick={(event) => closeSourceTab(tab, event)}
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
        {/each}
      </div>
    {/if}

    {#if preview}
      <div class="path-row">
        <span>{preview.relativePath}</span>
        <strong>
          {selectedIndex} / {records.length}
          {#if selectedSourceLine}
            · line {selectedSourceLine}
          {/if}
        </strong>
      </div>
    {/if}

    {#if fileActionStatus}
      <div class="file-action-feedback">{fileActionStatus}</div>
    {/if}

    {#if error}
      <div class="inline-error">
        <Activity size={15} strokeWidth={1.8} />
        <span>{error}</span>
      </div>
    {/if}

    {#if preview}
      <div class="editor-frame" class:is-loading={loading}>
        <div class="editor-toolbar" aria-label="Editor controls">
          <div class="editor-file-state" title={preview.relativePath}>
            <FileCode2 size={13} strokeWidth={1.8} />
            <strong>{preview.fileName}</strong>
            {#if selectedSourceDirty}
              <span>modified</span>
            {/if}
            {#if sourceIntelligenceAvailable}
              <span
                class="editor-lsp-state"
                class:ready={sourceLspStatus?.available}
                class:unavailable={!sourceLspStatusLoading && !sourceLspStatus?.available}
                title={sourceLspStatusTitle()}
              >
                {sourceLspStatusLabel()}
              </span>
            {/if}
          </div>
          <div class="editor-menu-anchor">
            <button
              class="editor-icon-button"
              class:active={!editorInsightCollapsed}
              type="button"
              aria-label={editorInsightCollapsed ? 'Show editor insights' : 'Hide editor insights'}
              title={editorInsightCollapsed ? 'Show editor insights' : 'Hide editor insights'}
              onclick={toggleEditorInsightCollapsed}
            >
              <SplitSquareHorizontal size={14} strokeWidth={2} />
            </button>
            <button
              class="editor-icon-button"
              type="button"
              aria-label="Editor actions"
              aria-haspopup="menu"
              aria-expanded={editorActionMenuOpen}
              title="Editor actions"
              onclick={toggleEditorActionMenu}
            >
              <MoreHorizontal size={15} strokeWidth={2} />
            </button>
            {#if editorActionMenuOpen}
              <div class="editor-action-menu" role="menu" aria-label="Editor actions">
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Save source file"
                  disabled={!selectedSourceDirty || fileActionBusy === 'save'}
                  onclick={() => {
                    closeEditorActionMenu();
                    void saveSelectedSourceFile();
                  }}
                >
                  <Save size={13} strokeWidth={2} />
                  <span>Save</span>
                  <kbd>Cmd+S</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Save all source files"
                  disabled={dirtyProjectSourceRecords.length === 0 || fileActionBusy === 'save-all'}
                  onclick={() => {
                    closeEditorActionMenu();
                    void saveAllDirtySourceFiles();
                  }}
                >
                  <Save size={13} strokeWidth={2} />
                  <span>Save all</span>
                  <kbd>{dirtyProjectSourceRecords.length}</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Close current source tab"
                  disabled={!selectedRecord}
                  onclick={() => {
                    closeEditorActionMenu();
                    void closeSelectedSourceTab();
                  }}
                >
                  <X size={13} strokeWidth={2} />
                  <span>Close tab</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Close other clean source tabs"
                  disabled={!selectedRecord || otherCleanProjectOpenSourceTabCount === 0}
                  onclick={() => {
                    closeEditorActionMenu();
                    void closeOtherCleanSourceTabs();
                  }}
                >
                  <X size={13} strokeWidth={2} />
                  <span>Close other clean</span>
                  <kbd>{otherCleanProjectOpenSourceTabCount}</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Close all clean source tabs"
                  disabled={cleanProjectOpenSourceTabCount === 0}
                  onclick={() => {
                    closeEditorActionMenu();
                    void closeAllCleanSourceTabs();
                  }}
                >
                  <X size={13} strokeWidth={2} />
                  <span>Close clean tabs</span>
                  <kbd>{cleanProjectOpenSourceTabCount}</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Format source file"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('format');
                  }}
                >
                  <Braces size={13} strokeWidth={2} />
                  <span>Format</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Rename symbol"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('rename');
                  }}
                >
                  <Braces size={13} strokeWidth={2} />
                  <span>Rename</span>
                  <kbd>F2</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Quick fix"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('quick-fix');
                  }}
                >
                  <Activity size={13} strokeWidth={2} />
                  <span>Quick fix</span>
                  <kbd>Alt+Enter</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Revert source file"
                  disabled={!selectedSourceDirty || fileActionBusy === 'save'}
                  onclick={() => {
                    closeEditorActionMenu();
                    revertSelectedSourceFile();
                  }}
                >
                  <RotateCcw size={13} strokeWidth={2} />
                  <span>Revert</span>
                </button>
                <hr />
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Show hover"
                  disabled={!sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('hover');
                  }}
                >
                  <SplitSquareHorizontal size={13} strokeWidth={2} />
                  <span>Hover</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Go to definition"
                  disabled={!preview || loading}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('definition');
                  }}
                >
                  <Search size={13} strokeWidth={2} />
                  <span>Go to definition</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Find references"
                  disabled={!preview || loading}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('references');
                  }}
                >
                  <Braces size={13} strokeWidth={2} />
                  <span>Find references</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Find implementations"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('implementation');
                  }}
                >
                  <Network size={13} strokeWidth={2} />
                  <span>Find implementations</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Go to type definition"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('type-definition');
                  }}
                >
                  <FileCode2 size={13} strokeWidth={2} />
                  <span>Type definition</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label={editorInsightCollapsed ? 'Show editor insights' : 'Hide editor insights'}
                  onclick={() => {
                    closeEditorActionMenu();
                    toggleEditorInsightCollapsed();
                  }}
                >
                  <SplitSquareHorizontal size={13} strokeWidth={2} />
                  <span>{editorInsightCollapsed ? 'Show insights' : 'Hide insights'}</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Show problems panel"
                  onclick={() => {
                    closeEditorActionMenu();
                    showEditorInsightPanel('problems');
                  }}
                >
                  <Activity size={13} strokeWidth={2} />
                  <span>Problems</span>
                  <kbd>{sourceDiagnostics.length}</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Show symbols panel"
                  onclick={() => {
                    closeEditorActionMenu();
                    showEditorInsightPanel('symbols');
                  }}
                >
                  <FileCode2 size={13} strokeWidth={2} />
                  <span>Symbols</span>
                  <kbd>{sourceSymbols.length}</kbd>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Show Git panel"
                  onclick={() => {
                    closeEditorActionMenu();
                    showEditorInsightPanel('git');
                  }}
                >
                  <FolderGit2 size={13} strokeWidth={2} />
                  <span>Git</span>
                  <kbd>{selectedSourceGitBadge()}</kbd>
                </button>
                <hr />
                <button
                  type="button"
                  role="menuitem"
                  disabled={fileActionBusy === 'copy'}
                  onclick={() => {
                    closeEditorActionMenu();
                    void copySelectedPath();
                  }}
                >
                  {#if fileActionStatus === 'Path copied'}
                    <Check size={13} strokeWidth={2} />
                  {:else}
                    <Copy size={13} strokeWidth={2} />
                  {/if}
                  <span>Copy path</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  disabled={fileActionBusy === 'open'}
                  onclick={() => {
                    closeEditorActionMenu();
                    void openSelectedFile();
                  }}
                >
                  <ExternalLink size={13} strokeWidth={2} />
                  <span>Open in IDE</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  disabled={fileActionBusy === 'reveal'}
                  onclick={() => {
                    closeEditorActionMenu();
                    void revealSelectedFile();
                  }}
                >
                  <FolderSearch size={13} strokeWidth={2} />
                  <span>Reveal file</span>
                </button>
              </div>
            {/if}
          </div>
        </div>

        <div class="editor-body-grid" class:insights-hidden={editorInsightCollapsed || !shouldRenderDockPanel('insights')}>
          <div class="editor-canvas">
            {#key sourcePreviewAppearanceKey}
              <MonacoSourceEditor
                {preview}
                content={selectedSourceDraftContent}
                editable={true}
                externalDiagnostics={sourceLspDiagnostics}
                {loading}
                targetLine={selectedSourceLine}
                targetLineRequestId={selectedSourceLineRequestId}
                intelligenceCommand={sourceIntelligenceCommand}
                onCodeActionLookup={handleEditorCodeActionLookup}
                onContentChange={updateSelectedSourceDraft}
                onCommandPaletteRequest={openCommandPalette}
                onCompletionLookup={handleEditorCompletionLookup}
                onDiagnosticsChange={handleEditorDiagnosticsChange}
                onDefinitionLookup={handleEditorDefinitionLookup}
                onDocumentHighlightLookup={handleEditorDocumentHighlightLookup}
                onFormatDocument={handleEditorFormatDocument}
                onGoToLineRequest={openCurrentFileGoToLine}
                onHoverLookup={handleEditorHoverLookup}
                onImplementationLookup={handleEditorImplementationLookup}
                onInlayHintLookup={handleEditorInlayHintLookup}
                onNavigateBackRequest={navigateSourceBack}
                onNavigateForwardRequest={navigateSourceForward}
                onNextProblemRequest={selectNextSourceDiagnostic}
                onProblemsRequest={() => showEditorInsightPanel('problems')}
                onPreviousProblemRequest={selectPreviousSourceDiagnostic}
                onQuickOpenRequest={openQuickOpen}
                onReferenceLookup={handleEditorReferenceLookup}
                onRename={handleEditorRename}
                onSaveRequest={saveSelectedSourceFile}
                onSemanticTokensLookup={handleEditorSemanticTokensLookup}
                onSignatureHelpLookup={handleEditorSignatureHelpLookup}
                onSymbolsRequest={() => showEditorInsightPanel('symbols')}
                onSymbolsChange={handleEditorSymbolsChange}
                onTypeDefinitionLookup={handleEditorTypeDefinitionLookup}
                onWorkspaceEditAction={handleEditorWorkspaceEditAction}
              />
            {/key}

            {#if editorInsightCollapsed && (sourceDefinitionQuery || sourceDefinitionTargets.length > 0 || sourceDefinitionLoading || sourceReferenceQuery || sourceReferenceTargets.length > 0 || sourceReferenceLoading || sourceImplementationQuery || sourceImplementationTargets.length > 0 || sourceImplementationLoading || sourceTypeDefinitionQuery || sourceTypeDefinitionTargets.length > 0 || sourceTypeDefinitionLoading)}
              <div class="editor-lookup-popover" aria-label="Editor lookup results">
                <div class="editor-lookup-header">
                  <strong>{sourceReferenceQuery ? sourceReferenceSummary : sourceImplementationQuery ? sourceImplementationSummary : sourceTypeDefinitionQuery ? sourceTypeDefinitionSummary : sourceDefinitionSummary}</strong>
                  <button
                    class="editor-lookup-close"
                    type="button"
                    aria-label="Close lookup results"
                    title="Close lookup results"
                    onclick={clearSourceLookupResults}
                  >
                    <X size={13} strokeWidth={2} />
                  </button>
                </div>
                <div class="editor-lookup-list">
                  {#if sourceDefinitionQuery || sourceDefinitionTargets.length > 0 || sourceDefinitionLoading}
                    {#if sourceDefinitionTargets.length === 0 && !sourceDefinitionLoading}
                      <div class="intelligence-empty">No definition</div>
                    {:else}
                      {#each sourceDefinitionTargets as target (`inline:${target.path}:${target.line}:${target.symbolName}`)}
                        <button
                          class="definition-row"
                          type="button"
                          title={target.detail}
                          onclick={() => selectSourceDefinitionTarget(target)}
                        >
                          <strong>{target.kind}</strong>
                          <span>{target.symbolName}</span>
                          <small>{target.relativePath}:{target.line}</small>
                        </button>
                      {/each}
                    {/if}
                  {/if}
                  {#if sourceReferenceQuery || sourceReferenceTargets.length > 0 || sourceReferenceLoading}
                    {#if sourceReferenceTargets.length === 0 && !sourceReferenceLoading}
                      <div class="intelligence-empty">No references</div>
                    {:else}
                      {#each sourceReferenceTargets as target (`inline:${target.path}:${target.line}:${target.column}`)}
                        <button
                          class="reference-row"
                          type="button"
                          title={target.excerpt}
                          onclick={() => selectSourceReferenceTarget(target)}
                        >
                          <strong>{target.line}:{target.column}</strong>
                          <span>{target.fileName}</span>
                          <small>{target.excerpt}</small>
                        </button>
                      {/each}
                    {/if}
                  {/if}
                  {#if sourceImplementationQuery || sourceImplementationTargets.length > 0 || sourceImplementationLoading}
                    {#if sourceImplementationTargets.length === 0 && !sourceImplementationLoading}
                      <div class="intelligence-empty">No implementations</div>
                    {:else}
                      {#each sourceImplementationTargets as target (`inline-implementation:${target.path}:${target.line}:${target.symbolName}`)}
                        <button
                          class="definition-row"
                          type="button"
                          title={target.detail}
                          onclick={() => selectSourceImplementationTarget(target)}
                        >
                          <strong>{target.kind}</strong>
                          <span>{target.symbolName}</span>
                          <small>{target.relativePath}:{target.line}</small>
                        </button>
                      {/each}
                    {/if}
                  {/if}
                  {#if sourceTypeDefinitionQuery || sourceTypeDefinitionTargets.length > 0 || sourceTypeDefinitionLoading}
                    {#if sourceTypeDefinitionTargets.length === 0 && !sourceTypeDefinitionLoading}
                      <div class="intelligence-empty">No type definition</div>
                    {:else}
                      {#each sourceTypeDefinitionTargets as target (`inline-type-definition:${target.path}:${target.line}:${target.symbolName}`)}
                        <button
                          class="definition-row"
                          type="button"
                          title={target.detail}
                          onclick={() => selectSourceTypeDefinitionTarget(target)}
                        >
                          <strong>{target.kind}</strong>
                          <span>{target.symbolName}</span>
                          <small>{target.relativePath}:{target.line}</small>
                        </button>
                      {/each}
                    {/if}
                  {/if}
                </div>
              </div>
            {/if}
          </div>

          {#if !editorInsightCollapsed && shouldRenderDockPanel('insights')}
            <button
              class="editor-insight-resizer"
              type="button"
              aria-label="Resize editor insights"
              title="Resize editor insights"
              onpointerdown={beginEditorInsightResize}
              onkeydown={handleEditorInsightResizerKeydown}
            ></button>

            <aside class="source-intelligence-panel" aria-label="Language intelligence">
            <div class="intelligence-tabs" role="tablist" aria-label="Source insights">
              <button
                class:active={sourceIntelligencePanel === 'problems'}
                type="button"
                role="tab"
                aria-selected={sourceIntelligencePanel === 'problems'}
                onclick={() => sourceIntelligencePanel = 'problems'}
              >
                <Activity size={13} strokeWidth={1.9} />
                <span>Problems</span>
                <strong>{sourceDiagnostics.length}</strong>
              </button>
              <button
                class:active={sourceIntelligencePanel === 'symbols'}
                type="button"
                role="tab"
                aria-selected={sourceIntelligencePanel === 'symbols'}
                onclick={() => sourceIntelligencePanel = 'symbols'}
              >
                <FileCode2 size={13} strokeWidth={1.9} />
                <span>Symbols</span>
                <strong>{sourceSymbols.length}</strong>
              </button>
              <button
                class:active={sourceIntelligencePanel === 'git'}
                type="button"
                role="tab"
                aria-selected={sourceIntelligencePanel === 'git'}
                onclick={() => sourceIntelligencePanel = 'git'}
              >
                <FolderGit2 size={13} strokeWidth={1.9} />
                <span>Git</span>
                <strong>{selectedSourceGitBadge()}</strong>
              </button>
            </div>

            {#if sourceDefinitionQuery || sourceDefinitionTargets.length > 0 || sourceDefinitionLoading}
              <div class="definition-results" aria-label="Definition lookup results">
                <div class="definition-summary">{sourceDefinitionSummary}</div>
                {#if sourceDefinitionTargets.length === 0 && !sourceDefinitionLoading}
                  <div class="intelligence-empty">No definition</div>
                {:else}
                  {#each sourceDefinitionTargets as target (`${target.path}:${target.line}:${target.symbolName}`)}
                    <button
                      class="definition-row"
                      type="button"
                      title={target.detail}
                      onclick={() => selectSourceDefinitionTarget(target)}
                    >
                      <strong>{target.kind}</strong>
                      <span>{target.symbolName}</span>
                      <small>{target.relativePath}:{target.line}</small>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}

            {#if sourceReferenceQuery || sourceReferenceTargets.length > 0 || sourceReferenceLoading}
              <div class="reference-results" aria-label="Reference lookup results">
                <div class="reference-summary">{sourceReferenceSummary}</div>
                {#if sourceReferenceTargets.length === 0 && !sourceReferenceLoading}
                  <div class="intelligence-empty">No references</div>
                {:else}
                  {#each sourceReferenceTargets as target (`${target.path}:${target.line}:${target.column}`)}
                    <button
                      class="reference-row"
                      type="button"
                      title={target.excerpt}
                      onclick={() => selectSourceReferenceTarget(target)}
                    >
                      <strong>{target.line}:{target.column}</strong>
                      <span>{target.fileName}</span>
                      <small>{target.excerpt}</small>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}

            {#if sourceImplementationQuery || sourceImplementationTargets.length > 0 || sourceImplementationLoading}
              <div class="implementation-results" aria-label="Implementation lookup results">
                <div class="implementation-summary">{sourceImplementationSummary}</div>
                {#if sourceImplementationTargets.length === 0 && !sourceImplementationLoading}
                  <div class="intelligence-empty">No implementations</div>
                {:else}
                  {#each sourceImplementationTargets as target (`${target.path}:${target.line}:${target.symbolName}:implementation`)}
                    <button
                      class="definition-row"
                      type="button"
                      title={target.detail}
                      onclick={() => selectSourceImplementationTarget(target)}
                    >
                      <strong>{target.kind}</strong>
                      <span>{target.symbolName}</span>
                      <small>{target.relativePath}:{target.line}</small>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}

            {#if sourceTypeDefinitionQuery || sourceTypeDefinitionTargets.length > 0 || sourceTypeDefinitionLoading}
              <div class="type-definition-results" aria-label="Type definition lookup results">
                <div class="type-definition-summary">{sourceTypeDefinitionSummary}</div>
                {#if sourceTypeDefinitionTargets.length === 0 && !sourceTypeDefinitionLoading}
                  <div class="intelligence-empty">No type definition</div>
                {:else}
                  {#each sourceTypeDefinitionTargets as target (`${target.path}:${target.line}:${target.symbolName}:type-definition`)}
                    <button
                      class="definition-row"
                      type="button"
                      title={target.detail}
                      onclick={() => selectSourceTypeDefinitionTarget(target)}
                    >
                      <strong>{target.kind}</strong>
                      <span>{target.symbolName}</span>
                      <small>{target.relativePath}:{target.line}</small>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}

            {#if sourceIntelligencePanel === 'problems'}
              <div class="intelligence-summary">{sourceDiagnosticSummary}</div>
              <div class="intelligence-list">
                {#if sourceDiagnostics.length === 0}
                  <div class="intelligence-empty">No problems</div>
                {:else}
                  {#each sourceDiagnostics as diagnostic, index (`${diagnostic.line}:${diagnostic.column}:${index}`)}
                    <button
                      class="intelligence-row diagnostic"
                      class:error={diagnostic.severity === 'error'}
                      class:warning={diagnostic.severity === 'warning'}
                      type="button"
                      title={diagnostic.message}
                      onclick={() => selectSourceDiagnostic(diagnostic)}
                    >
                      <strong>{diagnostic.severity}</strong>
                      <span>{diagnostic.message}</span>
                      <small>{diagnostic.line}:{diagnostic.column}</small>
                    </button>
                  {/each}
                {/if}
              </div>
            {:else if sourceIntelligencePanel === 'git'}
              <div class="git-diff-panel" aria-label="Selected file Git diff">
                <details class="git-command-drawer">
                  <summary>
                    <span>Commands</span>
                    <small>{gitActionError || gitActionStatus || (gitHasStagedChanges ? 'staged changes ready' : 'stage, fetch, pull, push')}</small>
                  </summary>
                  <div class="git-controls" aria-label="Git working tree controls">
                  <div class="git-action-row">
                    <button
                      class="git-action-button"
                      type="button"
                      aria-label="Stage selected source file"
                      title={selectedSourceDirty ? 'Save the source file before staging it' : 'Stage selected source file'}
                      disabled={selectedGitPathActionDisabled}
                      onclick={() => selectedRecord && runGitPathAction('stage', [selectedRecord.relativePath])}
                    >
                      <Plus size={12} strokeWidth={2} />
                      <span>{gitActionBusy === 'stage' ? 'Staging' : 'Stage'}</span>
                    </button>
                    <button
                      class="git-action-button"
                      type="button"
                      aria-label="Unstage selected source file"
                      title="Unstage selected source file"
                      disabled={selectedGitUnstageDisabled}
                      onclick={() => selectedRecord && runGitPathAction('unstage', [selectedRecord.relativePath])}
                    >
                      <RotateCcw size={12} strokeWidth={2} />
                      <span>{gitActionBusy === 'unstage' ? 'Unstaging' : 'Unstage'}</span>
                    </button>
                  </div>
                  <div class="git-remote-row">
                    <button
                      class="git-action-button"
                      type="button"
                      aria-label="Fetch selected repository"
                      title="Fetch selected repository"
                      disabled={gitRemoteActionDisabled}
                      onclick={() => runGitRemoteAction('fetch')}
                    >
                      <RefreshCw size={12} strokeWidth={2} />
                      <span>{gitActionBusy === 'fetch' ? 'Fetching' : 'Fetch'}</span>
                    </button>
                    <button
                      class="git-action-button"
                      type="button"
                      aria-label="Pull selected repository"
                      title="Pull selected repository with fast-forward only"
                      disabled={gitRemoteActionDisabled}
                      onclick={() => runGitRemoteAction('pull')}
                    >
                      <ChevronDown size={12} strokeWidth={2} />
                      <span>{gitActionBusy === 'pull' ? 'Pulling' : 'Pull'}</span>
                    </button>
                    <button
                      class="git-action-button"
                      type="button"
                      aria-label="Push selected repository"
                      title="Push selected repository"
                      disabled={gitRemoteActionDisabled}
                      onclick={() => runGitRemoteAction('push')}
                    >
                      <ExternalLink size={12} strokeWidth={2} />
                      <span>{gitActionBusy === 'push' ? 'Pushing' : 'Push'}</span>
                    </button>
                  </div>
                  <div class="git-commit-row">
                    <textarea
                      class="git-commit-input"
                      bind:value={gitCommitMessage}
                      aria-label="Git commit message"
                      placeholder="Commit message"
                      rows="2"
                    ></textarea>
                    <button
                      class="git-action-button commit"
                      type="button"
                      aria-label="Commit staged Git changes"
                      title="Commit staged Git changes"
                      disabled={gitCommitDisabled}
                      onclick={commitGitChanges}
                    >
                      <Check size={12} strokeWidth={2} />
                      <span>{gitActionBusy === 'commit' ? 'Committing' : 'Commit'}</span>
                    </button>
                  </div>
                  {#if gitActionError || gitActionStatus}
                    <div class:error={Boolean(gitActionError)} class="git-action-message">
                      {gitActionError || gitActionStatus}
                    </div>
                  {/if}
                  </div>
                </details>
                <div class="git-status-list" aria-label="Changed Git files">
                  {#if projectGitLoading}
                    <div class="intelligence-empty">Loading changed files</div>
                  {:else if projectGitError}
                    <div class="intelligence-empty">{projectGitError}</div>
                  {:else if selectedProjectGitChangedFiles.length === 0}
                    <div class="intelligence-empty">No changed files</div>
                  {:else}
                    <div class="git-status-overview">{selectedProjectGitFileGroupSummary}</div>
                    {#each selectedProjectGitFileGroups as group (group.id)}
                      <section class="git-status-group" aria-label={`${group.label} Git files`}>
                        <div class="git-status-group-heading">
                          <strong>{group.label}</strong>
                          <span>{group.files.length}</span>
                          <button
                            type="button"
                            disabled={gitActionBusy !== ''}
                            onclick={() => runGitStatusGroupAction(group)}
                          >
                            {gitStatusGroupActionLabel(group)}
                          </button>
                        </div>
                        {#each group.files as fileStatus (`${group.id}:${fileStatus.relativePath}`)}
                          <button
                            class="git-status-row"
                            class:selected={selectedRecord?.relativePath === fileStatus.relativePath}
                            type="button"
                            title={gitStatusFileTitle(fileStatus)}
                            onclick={() => selectGitStatusFile(fileStatus)}
                          >
                            <strong>{fileStatus.badge}</strong>
                            <span>{fileStatus.relativePath}</span>
                            <small>{gitStatusFileSummary(fileStatus)}</small>
                          </button>
                        {/each}
                      </section>
                    {/each}
                  {/if}
                </div>
                <div class="git-history-panel" aria-label="Git commit history">
                  <div class="git-history-heading">
                    <span>History</span>
                    <small>{gitCommitHistorySummary}</small>
                  </div>
                  <div
                    class="git-branch-health-strip"
                    aria-label="Git branch health"
                    title={selectedProjectGitBranchHealth.detail}
                  >
                    {#each selectedProjectGitBranchHealth.chips as chip (`${chip.label}:${chip.value}`)}
                      <span class={`git-branch-health-chip ${chip.tone}`}>
                        <strong>{chip.label}</strong>
                        <span>{chip.value}</span>
                      </span>
                    {/each}
                  </div>
                  {#if selectedProjectGitTaskIDs.length > 0}
                    <div class="git-task-trail" aria-label="Git task links">
                      <span>Tasks</span>
                      {#each selectedProjectGitTaskIDs as taskID (taskID)}
                        {#if gitTaskUrl(taskID)}
                          <a class="git-task-link" href={gitTaskUrl(taskID) ?? ''} target="_blank" rel="noreferrer">
                            {taskID}
                          </a>
                        {:else}
                          <span class="git-task-link">{taskID}</span>
                        {/if}
                      {/each}
                    </div>
                  {/if}
                  {#if selectedProjectGitTaskSourceGroups.length > 0}
                    <div class="git-task-source-map" aria-label="Git task source map">
                      {#each selectedProjectGitTaskSourceGroups as group (group.taskID)}
                        <div class="git-task-source-row" title={group.detailSummary}>
                          {#if gitTaskUrl(group.taskID)}
                            <a class="git-task-link" href={gitTaskUrl(group.taskID) ?? ''} target="_blank" rel="noreferrer">
                              {group.taskID}
                            </a>
                          {:else}
                            <span class="git-task-link">{group.taskID}</span>
                          {/if}
                          <small>{group.sourceSummary}</small>
                          <button
                            type="button"
                            aria-label={`Copy task sources for ${group.taskID}`}
                            title={group.detailSummary}
                            onclick={() => copyActivityCommand(`${group.taskID} · ${group.detailSummary}`, 'Task sources copied')}
                          >
                            <Copy size={11} strokeWidth={2} />
                          </button>
                        </div>
                      {/each}
                    </div>
                  {/if}
                  {#if selectedGitCommit}
                    <div
                      class="git-commit-detail"
                      aria-label="Selected commit detail"
                      title={gitCommitDetailText(selectedGitCommit)}
                    >
                      <div class="git-commit-detail-main">
                        <strong>{selectedGitCommit.shortSha}</strong>
                        <span>{selectedGitCommit.subject}</span>
                        <small>
                          {gitCommitParentSummary(selectedGitCommit) || 'linear'}
                          · {formatGitCommitTime(selectedGitCommit.committedAt)}
                          {#if selectedGitCommit.taskID}
                            · {selectedGitCommit.taskID}
                          {/if}
                        </small>
                      </div>
                      <div class="git-commit-detail-actions" aria-label="Selected commit actions">
                        <button
                          type="button"
                          aria-label="Copy selected commit detail"
                          title="Copy selected commit detail"
                          onclick={copySelectedGitCommitDetail}
                        >
                          <Copy size={11} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Copy selected commit handoff"
                          title="Copy selected commit handoff"
                          onclick={() => copyGitCommitHandoff(selectedGitCommit)}
                        >
                          <FileCode2 size={11} strokeWidth={2} />
                        </button>
                        <button
                          type="button"
                          aria-label="Copy selected commit SHA"
                          title="Copy selected commit SHA"
                          onclick={() => copyGitCommitSha(selectedGitCommit)}
                        >
                          <History size={11} strokeWidth={2} />
                        </button>
                        {#if selectedGitCommit.taskID}
                          <button
                            type="button"
                            aria-label="Copy selected commit task reference"
                            title="Copy selected commit task reference"
                            onclick={() => copyGitTaskReference(selectedGitCommit.taskID)}
                          >
                            <ExternalLink size={11} strokeWidth={2} />
                          </button>
                        {/if}
                      </div>
                    </div>
                  {/if}
                  <div class="git-history-list">
                    {#if gitCommitHistoryLoading}
                      <div class="intelligence-empty">Loading history</div>
                    {:else if gitCommitHistoryError}
                      <div class="intelligence-empty">{gitCommitHistoryError}</div>
                    {:else if gitCommitHistory.length === 0}
                      <div class="intelligence-empty">No commits</div>
                    {:else}
                      {#each gitCommitHistory as entry, index (entry.sha)}
                        <div
                          class={`git-history-row ${gitCommitGraphClass(entry, index)}`}
                          class:selected={selectedGitCommitSha === entry.sha}
                          role="button"
                          tabindex="0"
                          title={gitCommitTitle(entry)}
                          onclick={() => selectGitCommit(entry)}
                          onkeydown={(event) => handleGitCommitRowKeydown(event, entry)}
                        >
                          <span
                            class={`git-graph-marker ${gitCommitGraphClass(entry, index)}`}
                            aria-label={gitCommitTopology(entry, index)}
                            title={gitCommitTopology(entry, index)}
                          ></span>
                          <div class="git-history-main">
                            <strong>{entry.subject}</strong>
                            <small>{entry.shortSha} · {entry.author} · {formatGitCommitTime(entry.committedAt)}</small>
                          </div>
                          <div class="git-history-meta">
                            {#each gitCommitRefChips(entry) as refLabel (refLabel)}
                              <span class="git-ref-label">{refLabel}</span>
                            {/each}
                            {#if entry.taskID}
                              {#if gitTaskUrl(entry.taskID)}
                                <a
                                  class="git-task-link"
                                  href={gitTaskUrl(entry.taskID) ?? ''}
                                  target="_blank"
                                  rel="noreferrer"
                                  title={`Task from ${gitCommitTaskSourceLabel(entry) || 'Git metadata'}`}
                                >
                                  {entry.taskID}
                                </a>
                              {:else}
                                <span
                                  class="git-task-link"
                                  title={`Task from ${gitCommitTaskSourceLabel(entry) || 'Git metadata'}`}
                                >{entry.taskID}</span>
                              {/if}
                            {/if}
                            <div class="git-history-actions" aria-label="Commit quick actions">
                              <button
                                type="button"
                                aria-label="Copy commit SHA"
                                title="Copy commit SHA"
                                onclick={() => copyGitCommitSha(entry)}
                              >
                                <Copy size={11} strokeWidth={2} />
                              </button>
                              <button
                                type="button"
                                aria-label="Copy commit summary"
                                title="Copy commit summary"
                                onclick={() => copyGitCommitSummary(entry)}
                              >
                                <History size={11} strokeWidth={2} />
                              </button>
                              <button
                                type="button"
                                aria-label="Copy commit handoff"
                                title="Copy commit handoff"
                                onclick={() => copyGitCommitHandoff(entry)}
                              >
                                <FileCode2 size={11} strokeWidth={2} />
                              </button>
                              {#if entry.taskID}
                                <button
                                  type="button"
                                  aria-label="Copy task reference"
                                  title="Copy task reference"
                                  onclick={() => copyGitTaskReference(entry.taskID)}
                                >
                                  <ExternalLink size={11} strokeWidth={2} />
                                </button>
                              {/if}
                            </div>
                          </div>
                        </div>
                      {/each}
                    {/if}
                  </div>
                </div>
                <div class="intelligence-summary">{selectedSourceGitSummary}</div>
                {#if selectedSourceGitDiffLoading}
                  <div class="intelligence-empty">Loading Git diff</div>
                {:else if selectedSourceGitDiffError}
                  <div class="intelligence-empty">{selectedSourceGitDiffError}</div>
                {:else if selectedSourceGitDiff?.diff}
                  <pre class="git-diff-block">{selectedSourceGitDiff.diff}</pre>
                {:else}
                  <div class="intelligence-empty">No diff for selected file</div>
                {/if}
              </div>
            {:else}
              <div class="intelligence-summary">{sourceSymbols.length} symbols</div>
              <div class="intelligence-list">
                {#if sourceSymbols.length === 0}
                  <div class="intelligence-empty">No symbols</div>
                {:else}
                  {#each sourceSymbols as symbol (`${symbol.kind}:${symbol.name}:${symbol.line}`)}
                    <button
                      class="intelligence-row"
                      type="button"
                      title={symbol.detail}
                      onclick={() => selectSourceSymbol(symbol)}
                    >
                      <strong>{symbol.kind}</strong>
                      <span>{symbol.name}</span>
                      <small>{symbol.line}</small>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}
            </aside>
          {/if}
        </div>
      </div>
    {:else}
      <div class="empty-preview">
        <FileCode2 size={34} strokeWidth={1.55} />
        <strong>No source file loaded</strong>
        <span>Scan a project or choose a file from the tree.</span>
      </div>
    {/if}

      {#if shouldRenderDockPanel('terminal')}
        <section class="terminal-launchpad" aria-label="Terminal dock">
          <header class="terminal-launchpad-header">
            <div>
              <Terminal size={14} strokeWidth={2} />
              <strong>Terminal</strong>
              <span>{terminalDockSummary()}</span>
            </div>
            <div class="terminal-launchpad-actions">
              <label class="terminal-inline-picker" title={`Open commands in ${sourceTerminalApp}`}>
                <span>App</span>
                <select bind:value={sourceTerminalApp} aria-label="Terminal dock app" onchange={selectSourceTerminalApp}>
                  {#each sourceTerminalApps as app (app)}
                    <option value={app}>{app}</option>
                  {/each}
                </select>
              </label>
              <button
                class="file-action-button"
                type="button"
                aria-label="Open project shell"
                title={selectedProject.path}
                disabled={!selectedProject.path || fileActionBusy === `activity-terminal:${selectedProject.path}`}
                onclick={() => openActivityTerminalPath(selectedProject.path)}
              >
                <Terminal size={13} strokeWidth={2} />
                <span>Project</span>
              </button>
              <button
                class="file-action-button"
                type="button"
                aria-label="Start embedded terminal"
                title={selectedProject.path}
                disabled={!selectedProject.path || embeddedTerminalStarting}
                onclick={() => startEmbeddedTerminalSession(selectedProject.path)}
              >
                <Terminal size={13} strokeWidth={2} />
                <span>{embeddedTerminalStarting ? 'Starting' : 'Start'}</span>
              </button>
              <button
                class="file-action-button"
                type="button"
                aria-label="Stop embedded terminal"
                title="Stop embedded terminal"
                disabled={!embeddedTerminalSession}
                onclick={closeEmbeddedTerminalSession}
              >
                <X size={13} strokeWidth={2} />
                <span>Stop</span>
              </button>
              <button
                class="file-action-button icon-only"
                type="button"
                aria-label="Fit embedded terminal"
                title="Fit embedded terminal"
                onclick={fitEmbeddedTerminal}
              >
                <RefreshCw size={13} strokeWidth={2} />
              </button>
              <button
                class="file-action-button"
                type="button"
                aria-label="Hide terminal dock"
                title="Hide terminal dock"
                onclick={() => hideDockPanel('terminal')}
              >
                <X size={13} strokeWidth={2} />
              </button>
            </div>
          </header>

          <div class:active={Boolean(embeddedTerminalSession)} class="embedded-terminal-panel" aria-label="Embedded terminal">
            <div class="embedded-terminal-toolbar">
              <span>{embeddedTerminalStatusLabel()}</span>
              {#if embeddedTerminalSession?.pid}
                <code>pid {embeddedTerminalSession.pid}</code>
              {/if}
            </div>
            <div class="embedded-terminal-host" bind:this={embeddedTerminalElement}></div>
            {#if embeddedTerminalError}
              <div class="embedded-terminal-error">{embeddedTerminalError}</div>
            {/if}
          </div>

          <div class="terminal-launchpad-grid">
            <div class="terminal-launchpad-list" aria-label="Embedded terminal sessions">
              <div class="terminal-launchpad-title">
                <span>Embedded</span>
                <strong>{embeddedTerminalSessions.length}</strong>
              </div>
              {#if embeddedTerminalSessionsLoading}
                <div class="terminal-launchpad-empty">Refreshing terminals</div>
              {:else if embeddedTerminalSessions.length === 0}
                <div class="terminal-launchpad-empty">No embedded terminals</div>
              {:else}
                {#each embeddedTerminalSessions.slice(0, 3) as session (`terminal-session:${session.sessionId}`)}
                  <div
                    class:active={embeddedTerminalSession?.sessionId === session.sessionId}
                    class="terminal-launchpad-row"
                    title={session.cwd}
                  >
                    <span class="runtime-port">PTY</span>
                    <div>
                      <strong>{embeddedTerminalSessionTitle(session)}</strong>
                      <small>{session.shell}{session.pid ? ` · pid ${session.pid}` : ''}</small>
                    </div>
                    <button
                      type="button"
                      aria-label="Attach embedded terminal session"
                      title="Attach embedded terminal"
                      disabled={embeddedTerminalSession?.sessionId === session.sessionId}
                      onclick={() => attachEmbeddedTerminalSession(session)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                    <button
                      type="button"
                      aria-label="Close listed embedded terminal session"
                      title="Close embedded terminal"
                      onclick={() => closeListedEmbeddedTerminalSession(session)}
                    >
                      <X size={12} strokeWidth={2} />
                    </button>
                  </div>
                {/each}
              {/if}
              {#if embeddedTerminalSessionsError}
                <div class="terminal-launchpad-empty">{embeddedTerminalSessionsError}</div>
              {/if}
            </div>

            <div class="terminal-launchpad-list" aria-label="Active terminal contexts">
              <div class="terminal-launchpad-title">
                <span>Active</span>
                <strong>{selectedProjectRuntimeContexts.length}</strong>
              </div>
              {#if selectedProjectRuntimeContexts.length === 0}
                <div class="terminal-launchpad-empty">No active contexts</div>
              {:else}
                {#each selectedProjectRuntimeContexts.slice(0, 3) as context (`terminal:${context.pid}:${context.port}:${context.cwd}`)}
                  <div class="terminal-launchpad-row" title={context.cwd}>
                    <span class="runtime-port">:{context.port}</span>
                    <div>
                      <strong>{context.command}</strong>
                      <small>{context.rootLabel}</small>
                    </div>
                    <button
                      type="button"
                      aria-label="Open active context in embedded terminal"
                      title="Open context in embedded terminal"
                      onclick={() => openPathEmbeddedTerminal(context.cwd)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                  </div>
                {/each}
              {/if}
            </div>

            <div class="terminal-launchpad-list" aria-label="Agent terminal resumes">
              <div class="terminal-launchpad-title">
                <span>Agents</span>
                <strong>{selectedProjectAgentSessions.length}</strong>
              </div>
              {#if selectedProjectAgentSessions.length === 0}
                <div class="terminal-launchpad-empty">No resumable agents</div>
              {:else}
                {#each selectedProjectAgentSessions.slice(0, 3) as session, index (agentSessionRowKey(session, index, 'terminal'))}
                  <div class="terminal-launchpad-row" title={agentSessionResumePlan(session)}>
                    <span class="agent-provider-badge">{session.provider}</span>
                    <div>
                      <strong>{session.title}</strong>
                      <small>{agentSessionProjectLabel(session)}</small>
                    </div>
                    <button
                      type="button"
                      aria-label="Resume agent from terminal dock"
                      title="Resume agent in embedded terminal"
                      onclick={() => resumeAgentSessionEmbeddedTerminal(session)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                  </div>
                {/each}
              {/if}
            </div>

            <div class="terminal-launchpad-list" aria-label="Worktree terminal shortcuts">
              <div class="terminal-launchpad-title">
                <span>Worktrees</span>
                <strong>{projectWorktrees.length}</strong>
              </div>
              {#if projectWorktrees.length === 0}
                <div class="terminal-launchpad-empty">No worktrees</div>
              {:else}
                {#each prioritizedProjectWorktrees.slice(0, 3) as worktree (`terminal:${worktree.path}`)}
                  {@const safety = projectWorktreeSafety(worktree)}
                  <div class="terminal-launchpad-row" title={safety.cleanupPlan}>
                    <span class={`worktree-status-badge ${safety.kind}`}>{safety.badge}</span>
                    <div>
                      <strong>{worktree.branch}</strong>
                      <small>{safety.reason}</small>
                    </div>
                    <button
                      type="button"
                      aria-label="Open worktree from terminal dock"
                      title="Open worktree in embedded terminal"
                      onclick={() => openPathEmbeddedTerminal(worktree.path)}
                    >
                      <Terminal size={12} strokeWidth={2} />
                    </button>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        </section>
      {/if}

      {#if shouldRenderDockPanel('browser')}
        <section class="browser-dock" aria-label="Browser dock">
          <header class="browser-dock-header">
            <div class="browser-dock-title">
              <Network size={14} strokeWidth={2} />
              <strong>Browser</strong>
              <span>{activeBrowserUrl || 'No runtime URL'}</span>
            </div>
            <div class="browser-dock-actions">
              <button
                class="file-action-button icon-only"
                type="button"
                aria-label="Reload browser dock"
                title="Reload browser dock"
                disabled={!activeBrowserUrl}
                onclick={reloadBrowserFrame}
              >
                <RefreshCw size={13} strokeWidth={2} />
              </button>
              <button
                class="file-action-button icon-only"
                type="button"
                aria-label="Open browser URL externally"
                title="Open browser URL externally"
                disabled={!activeBrowserUrl}
                onclick={openBrowserUrlExternal}
              >
                <ExternalLink size={13} strokeWidth={2} />
              </button>
              <button
                class="file-action-button icon-only"
                type="button"
                aria-label="Hide browser dock"
                title="Hide browser dock"
                onclick={() => hideDockPanel('browser')}
              >
                <X size={13} strokeWidth={2} />
              </button>
            </div>
          </header>

          <form class="browser-url-form" onsubmit={submitBrowserUrl}>
            <input
              bind:value={browserInputUrl}
              aria-label="Browser dock URL"
              autocomplete="off"
              spellcheck="false"
              placeholder="localhost:5177"
            />
            <button
              class="file-action-button"
              type="submit"
              disabled={!browserInputUrl.trim()}
            >
              <Network size={13} strokeWidth={2} />
              <span>Open</span>
            </button>
          </form>

          {#if selectedProjectRuntimeContexts.length > 0}
            <div class="browser-runtime-list" aria-label="Browser runtime shortcuts">
              {#each selectedProjectRuntimeContexts as context (`browser:${context.pid}:${context.port}:${context.cwd}`)}
                <button
                  type="button"
                  class:active={activeBrowserUrl === runtimeContextUrl(context)}
                  title={runtimeContextUrl(context)}
                  onclick={() => openRuntimeContextInBrowserDock(context)}
                >
                  <span>:{context.port}</span>
                  <strong>{context.command}</strong>
                  <small>{context.rootLabel}</small>
                </button>
              {/each}
            </div>
          {/if}

          {#if browserError}
            <div class="browser-error">{browserError}</div>
          {/if}

          {#if activeBrowserUrl}
            <div class="browser-frame-wrap">
              {#key `${browserFrameKey}:${activeBrowserUrl}`}
                <iframe
                  class="browser-frame"
                  title="Browser dock preview"
                  src={activeBrowserUrl}
                  sandbox="allow-downloads allow-forms allow-modals allow-popups allow-same-origin allow-scripts"
                  referrerpolicy="no-referrer"
                ></iframe>
              {/key}
            </div>
          {:else}
            <div class="browser-empty">Start a runtime or enter a localhost URL.</div>
          {/if}
        </section>
      {/if}
      </div>
    </div>
  </section>
</main>

{#if quickOpenVisible}
  <div class="quick-open-layer">
    <button
      class="quick-open-backdrop"
      type="button"
      aria-label="Close quick open"
      onclick={closeQuickOpen}
    ></button>
    <div
      class="quick-open-panel"
      role="dialog"
      aria-modal="true"
      aria-label="Open source file"
    >
      <label class="quick-open-search">
        <span class="quick-open-icon">
          <Search size={17} strokeWidth={1.8} />
        </span>
        <input
          bind:this={quickOpenInput}
          bind:value={quickOpenQuery}
          onkeydown={handleQuickOpenKeydown}
          placeholder={quickOpenWorkspaceSymbolMode ? 'Search workspace symbols' : 'Open source file'}
          autocomplete="off"
        />
      </label>

      <div
        class="quick-open-results"
        role="listbox"
        aria-label={quickOpenWorkspaceSymbolMode ? 'Matching workspace symbols' : 'Matching source files'}
      >
        {#if quickOpenWorkspaceSymbolMode}
          {#if !preview || !sourceIntelligenceAvailable}
            <div class="quick-open-empty">Open a C# or TypeScript file first</div>
          {:else if !quickOpenWorkspaceSymbolQuery}
            <div class="quick-open-empty">Type a symbol name after #</div>
          {:else if workspaceSymbolLoading}
            <div class="quick-open-empty">Searching workspace symbols</div>
          {:else if workspaceSymbolError}
            <div class="quick-open-empty">{workspaceSymbolError}</div>
          {:else if workspaceSymbolResults.length === 0}
            <div class="quick-open-empty">No matching workspace symbols</div>
          {:else}
            {#each workspaceSymbolResults as symbol, index (`${symbol.path}:${symbol.line}:${symbol.column}:${symbol.symbolName}`)}
              <button
                class:active={index === quickOpenIndex}
                type="button"
                role="option"
                aria-selected={index === quickOpenIndex}
                title={symbol.detail}
                onclick={() => chooseQuickOpenWorkspaceSymbol(symbol)}
              >
                <span class="quick-open-result-icon">
                  <FileCode2 size={15} strokeWidth={1.8} />
                </span>
                <span>
                  <strong>{symbol.symbolName}</strong>
                  <small>{symbol.detail}</small>
                </span>
                <em>{symbol.kind}</em>
              </button>
            {/each}
          {/if}
        {:else if quickOpenResults.length === 0}
          <div class="quick-open-empty">No matching source files</div>
        {:else}
          {#each quickOpenResults as record, index (record.path)}
            <button
              class:active={index === quickOpenIndex}
              type="button"
              role="option"
              aria-selected={index === quickOpenIndex}
              title={record.relativePath}
              onclick={() => chooseQuickOpenRecord(record)}
            >
              <span class="quick-open-result-icon">
                <FileCode2 size={15} strokeWidth={1.8} />
              </span>
              <span>
                <strong>{record.fileName}</strong>
                <small>{record.relativePath}</small>
              </span>
              <em>{parsedQuickOpenQuery.targetLine ? `line ${parsedQuickOpenQuery.targetLine}` : record.language}</em>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if commandPaletteVisible}
  <div class="command-palette-layer">
    <button
      class="command-palette-backdrop"
      type="button"
      aria-label="Close command palette"
      onclick={closeCommandPalette}
    ></button>
    <div
      class="command-palette-panel"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
    >
      <label class="command-palette-search">
        <span class="command-palette-icon">
          <Search size={17} strokeWidth={1.8} />
        </span>
        <input
          bind:this={commandPaletteInput}
          bind:value={commandPaletteQuery}
          onkeydown={handleCommandPaletteKeydown}
          placeholder="Run command"
          autocomplete="off"
        />
        <kbd>Cmd+K</kbd>
      </label>

      <div class="command-palette-results" role="listbox" aria-label="Matching commands">
        {#if commandPaletteResults.length === 0}
          <div class="quick-open-empty">No matching commands</div>
        {:else}
          {#each commandPaletteResults as item, index (item.id)}
            <button
              class:active={index === commandPaletteIndex}
              class:disabled={item.disabled}
              type="button"
              role="option"
              aria-selected={index === commandPaletteIndex}
              disabled={item.disabled}
              title={item.detail}
              onclick={() => void runCommandPaletteItem(item)}
            >
              <span class="command-palette-result-icon">
                <MoreHorizontal size={15} strokeWidth={1.8} />
              </span>
              <span>
                <strong>{item.label}</strong>
                <small>{item.detail}</small>
              </span>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: var(--side-pane-width) 6px minmax(0, 1fr);
    gap: 0;
    width: min(1840px, calc(100vw - 16px));
    height: min(1040px, calc(100dvh - 16px));
    min-height: min(680px, calc(100dvh - 16px));
    margin: 8px auto;
    overflow: hidden;
    border: 1px solid rgba(231, 238, 235, 0.12);
    border-radius: 12px;
    background: rgba(24, 26, 26, 0.92);
    box-shadow:
      0 32px 90px rgba(0, 0, 0, 0.32),
      inset 0 1px 0 rgba(255, 255, 255, 0.08);
  }

  .shell.side-right {
    grid-template-columns: minmax(0, 1fr) 6px var(--side-pane-width);
  }

  .shell.side-right .workspace {
    grid-column: 1;
    grid-row: 1;
  }

  .shell.side-right .side-pane-resizer {
    grid-column: 2;
    grid-row: 1;
  }

  .shell.side-right .activity-shell {
    grid-template-columns: minmax(0, 1fr) 46px;
    grid-column: 3;
    grid-row: 1;
    border-right: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.08);
  }

  .shell.side-right .activity-rail {
    grid-column: 2;
    grid-row: 1;
    border-right: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.07);
  }

  .shell.side-right .sidebar {
    grid-column: 1;
    grid-row: 1;
  }

  .activity-shell {
    display: grid;
    grid-template-columns: 46px minmax(0, 1fr);
    min-width: 0;
    overflow: hidden;
    background: rgba(19, 21, 21, 0.94);
    border-right: 1px solid rgba(255, 255, 255, 0.08);
  }

  .activity-rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    width: 46px;
    min-width: 0;
    padding: 10px 4px;
    overflow: hidden;
    border-right: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(9, 12, 12, 0.42);
  }

  .activity-rail button {
    position: relative;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    min-width: 0;
    color: #99a5a1;
    border: 1px solid transparent;
    border-radius: 9px;
    background: transparent;
    cursor: pointer;
  }

  .activity-rail button:hover,
  .activity-rail button:focus-visible,
  .activity-rail button.active {
    color: #f2f6f5;
    border-color: rgba(92, 226, 207, 0.24);
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .activity-rail button.active {
    color: #6fdfcf;
  }

  .activity-rail button strong {
    position: absolute;
    right: 3px;
    bottom: 3px;
    min-width: 16px;
    max-width: 30px;
    height: 16px;
    padding: 0 4px;
    overflow: hidden;
    color: #071b18;
    border-radius: 999px;
    background: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 9px;
    font-weight: 850;
    line-height: 16px;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-rail-label {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .side-pane-resizer {
    width: 6px;
    min-width: 0;
    padding: 0;
    cursor: col-resize;
    border: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.045);
    border-right: 1px solid rgba(255, 255, 255, 0.045);
    background: rgba(255, 255, 255, 0.025);
  }

  .side-pane-resizer:hover,
  .side-pane-resizer:focus-visible {
    outline: 0;
    background: rgba(92, 226, 207, 0.18);
  }

  :global(body.resizing-source-pane) {
    cursor: col-resize;
    user-select: none;
  }

  .sidebar {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    min-width: 0;
    overflow: hidden;
    padding: 14px 12px;
  }

  .brand-row {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    align-items: center;
    gap: 9px;
    margin-bottom: 12px;
  }

  .brand-mark {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: 1px solid rgba(89, 217, 199, 0.38);
    border-radius: 11px;
    color: #5ce2cf;
    background: rgba(31, 83, 76, 0.24);
  }

  .eyebrow {
    margin: 0 0 3px;
    color: #92a19d;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  h1,
  h2 {
    margin: 0;
    color: #f4f7f6;
    font-weight: 760;
    line-height: 1.08;
    letter-spacing: 0;
  }

  h1 {
    font-size: 17px;
  }

  h2 {
    font-size: 22px;
  }

  .project-controls {
    margin-bottom: 12px;
  }

  .source-browser-stack {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .activity-panel {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
  }

  .activity-panel-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding-bottom: 10px;
  }

  .activity-panel-header div {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .activity-panel-header strong,
  .activity-panel-header span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-panel-header strong {
    color: #f0f4f3;
    font-size: 13px;
    font-weight: 820;
  }

  .activity-panel-header span {
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
  }

  .activity-filter-box {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 7px;
    min-width: 0;
    margin-bottom: 10px;
    padding: 7px 9px;
    color: #8d9995;
    border: 1px solid rgba(255, 255, 255, 0.065);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.045);
  }

  .activity-filter-box input {
    min-width: 0;
    color: #e5ebe9;
    font: inherit;
    font-size: 12px;
    font-weight: 720;
    border: 0;
    outline: none;
    background: transparent;
  }

  .activity-filter-box input::placeholder {
    color: #7f8a86;
  }

  .paste-cleanup-panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 10px;
    min-height: 0;
    overflow: hidden;
  }

  .paste-cleanup-toolbar,
  .paste-cleanup-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  .paste-cleanup-toolbar label {
    display: inline-grid;
    grid-template-columns: auto minmax(74px, auto);
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 30px;
    padding: 0 8px;
    color: #9facaa;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    font-size: 10px;
    font-weight: 820;
  }

  .paste-cleanup-toolbar select {
    min-width: 0;
    color: #dffdf8;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
  }

  .paste-cleanup-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(150px, 1fr) minmax(150px, 1fr);
    gap: 9px;
    min-height: 0;
  }

  .paste-cleanup-grid label {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 5px;
    min-width: 0;
    min-height: 0;
  }

  .paste-cleanup-grid label > span,
  .paste-cleanup-footer span {
    color: #8d9995;
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .paste-cleanup-textarea {
    width: 100%;
    min-width: 0;
    min-height: 0;
    padding: 10px;
    resize: none;
    color: #e7ecea;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 8px;
    outline: 0;
    background: rgba(0, 0, 0, 0.18);
    font: 12px/1.45 ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
  }

  .paste-cleanup-textarea:focus {
    border-color: rgba(92, 226, 207, 0.42);
    box-shadow: 0 0 0 2px rgba(92, 226, 207, 0.08);
  }

  .paste-cleanup-footer {
    justify-content: space-between;
  }

  .activity-panel-list {
    display: grid;
    align-content: start;
    gap: 7px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 3px;
    scrollbar-color: rgba(174, 184, 181, 0.52) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .workspace-snapshot-section {
    display: grid;
    gap: 6px;
    min-width: 0;
    padding: 6px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.025);
  }

  .workspace-snapshot-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .workspace-snapshot-toolbar div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .workspace-snapshot-toolbar strong,
  .workspace-snapshot-toolbar span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-snapshot-toolbar strong {
    color: #dfe7e5;
    font-size: 10px;
    font-weight: 820;
    text-transform: uppercase;
  }

  .workspace-snapshot-toolbar span {
    color: #7f8b88;
    font-size: 9px;
    font-weight: 720;
  }

  .workspace-snapshot-toolbar button {
    display: grid;
    place-items: center;
    width: 23px;
    height: 23px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .workspace-snapshot-toolbar button:hover,
  .workspace-snapshot-toolbar button:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.36);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .workspace-snapshot-list {
    display: grid;
    gap: 5px;
    min-width: 0;
  }

  .workspace-snapshot-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 36px;
    padding: 5px;
    border: 1px solid rgba(92, 226, 207, 0.08);
    border-radius: 7px;
    background: rgba(92, 226, 207, 0.045);
  }

  .workspace-snapshot-row.active {
    border-color: color-mix(in srgb, var(--accent) 62%, transparent);
    background: rgba(92, 226, 207, 0.095);
  }

  .workspace-snapshot-row > button {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 7px;
    min-width: 0;
    padding: 0;
    color: inherit;
    text-align: left;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .workspace-snapshot-row > button:hover .activity-row-main strong,
  .workspace-snapshot-row > button:focus-visible .activity-row-main strong {
    color: #9cebe0;
  }

  .workspace-snapshot-row > button:focus-visible {
    outline: 1px solid rgba(92, 226, 207, 0.34);
    outline-offset: 2px;
  }

  .activity-session-row,
  .activity-runtime-row,
  .activity-worktree-row,
  .activity-repo-row,
  .activity-run-row,
  .activity-commit-row {
    display: grid;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 40px;
    padding: 8px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.14);
  }

  .activity-session-row,
  .activity-runtime-row,
  .activity-worktree-row {
    grid-template-columns: auto minmax(0, 1fr) auto;
  }

  .conversation-session-row {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .conversation-session-row.active {
    border-color: color-mix(in srgb, var(--accent) 62%, transparent);
    background: rgba(92, 226, 207, 0.085);
  }

  .conversation-session-open {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 0;
    color: inherit;
    text-align: left;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .conversation-session-open:hover .activity-row-main strong,
  .conversation-session-open:focus-visible .activity-row-main strong {
    color: #9cebe0;
  }

  .conversation-session-open:focus-visible {
    outline: 1px solid rgba(92, 226, 207, 0.34);
    outline-offset: 2px;
  }

  .activity-worktree-row {
    align-items: start;
    min-height: 58px;
    padding: 7px;
  }

  .activity-commit-row {
    grid-template-columns: auto minmax(0, 1fr) auto;
  }

  .activity-repo-row {
    grid-template-columns: minmax(0, 1fr) auto auto auto;
  }

  .activity-run-row {
    align-items: stretch;
    gap: 7px;
  }

  .activity-worktree-row.blocked,
  .activity-run-row.bad,
  .activity-repo-row.dirty {
    background: rgba(216, 170, 85, 0.09);
  }

  .activity-worktree-row.protected {
    background: rgba(255, 255, 255, 0.04);
  }

  .activity-worktree-row.ready {
    background: rgba(92, 226, 207, 0.06);
  }

  .activity-run-row.attention {
    border-color: rgba(216, 170, 85, 0.18);
    background: rgba(216, 170, 85, 0.075);
  }

  .activity-run-row.live {
    background: rgba(92, 226, 207, 0.07);
  }

  .run-row-heading,
  .run-metrics-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .run-row-heading {
    justify-content: space-between;
  }

  .run-heading-badges {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .run-metrics-row {
    flex-wrap: wrap;
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
  }

  .run-loop-row {
    min-width: 0;
    overflow: hidden;
    color: #8fd8cf;
    font-size: 10px;
    font-weight: 780;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-attention-queue {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .run-attention-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 30px;
    padding: 5px 7px;
    border: 1px solid rgba(216, 170, 85, 0.2);
    border-radius: 7px;
    background: rgba(216, 170, 85, 0.08);
  }

  .run-attention-item.bad {
    border-color: rgba(255, 112, 112, 0.24);
    background: rgba(255, 112, 112, 0.08);
  }

  .run-attention-item > span {
    display: inline-flex;
    align-items: center;
    min-height: 18px;
    padding: 0 6px;
    color: #20170a;
    border-radius: 999px;
    background: #d8aa55;
    font-size: 8px;
    font-weight: 900;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .run-attention-item.bad > span {
    color: #220a0a;
    background: #ff8f8f;
  }

  .run-attention-item div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .run-attention-item strong,
  .run-attention-item small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-attention-item strong {
    color: #f3ebe0;
    font-size: 10px;
    font-weight: 850;
  }

  .run-attention-item small {
    color: #b9ada0;
    font-size: 9px;
    font-weight: 760;
  }

  .run-attention-item a {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    color: #d8aa55;
    border-radius: 5px;
    text-decoration: none;
  }

  .run-attention-item a:hover,
  .run-attention-item a:focus-visible {
    color: #f4d08b;
    outline: 0;
    background: rgba(216, 170, 85, 0.14);
  }

  .run-loop-stage-strip {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 4px;
    min-width: 0;
  }

  .run-loop-stage {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 3px;
    min-width: 0;
    height: 24px;
    padding: 0 5px;
    color: #8d9995;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.025);
  }

  .run-loop-stage em,
  .run-loop-stage strong {
    min-width: 0;
    overflow: hidden;
    font-style: normal;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-loop-stage em {
    font-size: 8px;
    font-weight: 820;
    text-transform: uppercase;
  }

  .run-loop-stage strong {
    color: #dfe7e5;
    font-size: 11px;
    font-weight: 860;
  }

  .run-loop-stage.live {
    color: #8fd8cf;
    border-color: rgba(92, 226, 207, 0.18);
    background: rgba(92, 226, 207, 0.07);
  }

  .run-loop-stage.good {
    color: #a6d8ad;
    border-color: rgba(139, 220, 155, 0.18);
    background: rgba(139, 220, 155, 0.07);
  }

  .run-loop-stage.attention {
    color: #e0bf7b;
    border-color: rgba(216, 170, 85, 0.22);
    background: rgba(216, 170, 85, 0.08);
  }

  .run-loop-stage.bad {
    color: #f6aaaa;
    border-color: rgba(243, 111, 111, 0.22);
    background: rgba(243, 111, 111, 0.08);
  }

  .run-current-activity {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 25px;
    padding: 0 7px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.065);
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.16);
  }

  .run-current-activity span {
    color: #6fdfcf;
    font-size: 8px;
    font-weight: 860;
    text-transform: uppercase;
  }

  .run-current-activity strong {
    min-width: 0;
    overflow: hidden;
    color: #dfe7e5;
    font-size: 10px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-agent-strip {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 4px;
    min-width: 0;
  }

  .run-agent-pill {
    display: grid;
    grid-template-columns: minmax(0, auto) minmax(0, 1fr);
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    color: #9aa7a3;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.028);
  }

  .run-agent-pill em,
  .run-agent-pill strong {
    min-width: 0;
    overflow: hidden;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-agent-pill em {
    color: #8d9995;
    font-size: 8px;
    font-style: normal;
    font-weight: 860;
    text-transform: uppercase;
  }

  .run-agent-pill strong {
    color: #d7dfdd;
    font-size: 10px;
    font-weight: 800;
  }

  .run-agent-pill.live {
    color: #8fd8cf;
    border-color: rgba(92, 226, 207, 0.18);
    background: rgba(92, 226, 207, 0.06);
  }

  .run-agent-pill.good {
    color: #a6d8ad;
    border-color: rgba(139, 220, 155, 0.16);
    background: rgba(139, 220, 155, 0.055);
  }

  .run-agent-pill.attention {
    color: #e0bf7b;
    border-color: rgba(216, 170, 85, 0.22);
    background: rgba(216, 170, 85, 0.08);
  }

  .run-agent-pill.bad {
    color: #f6aaaa;
    border-color: rgba(243, 111, 111, 0.22);
    background: rgba(243, 111, 111, 0.08);
  }

  .run-agent-pill.live em,
  .run-agent-pill.live strong {
    color: #9cebe0;
  }

  .run-agent-pill.good em,
  .run-agent-pill.good strong {
    color: #b8e6be;
  }

  .run-agent-pill.attention em,
  .run-agent-pill.attention strong {
    color: #f0cf8e;
  }

  .run-agent-pill.bad em,
  .run-agent-pill.bad strong {
    color: #ffb8b8;
  }

  .run-progress-track {
    height: 6px;
    overflow: hidden;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.07);
  }

  .run-progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, #5ce2cf, #78b7ff);
  }

  .run-step-list {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .run-timeline {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .run-step-row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .run-timeline-item {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 4px 5px;
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.035);
  }

  .run-timeline-item.attention {
    background: rgba(216, 170, 85, 0.09);
  }

  .run-step-row div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .run-timeline-item div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .run-step-row strong,
  .run-step-row small,
  .run-timeline-item strong,
  .run-timeline-item small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-step-row strong {
    color: #dfe7e5;
    font-size: 10px;
    font-weight: 780;
  }

  .run-step-row small {
    color: #87918e;
    font-size: 9px;
    font-weight: 720;
  }

  .run-timeline-item strong {
    color: #dfe7e5;
    font-size: 10px;
    font-weight: 780;
  }

  .run-timeline-item small,
  .run-timeline-item em {
    color: #87918e;
    font-size: 9px;
    font-style: normal;
    font-weight: 720;
  }

  .run-timeline-item em {
    white-space: nowrap;
  }

  .run-artifact-row {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    min-width: 0;
  }

  .run-chip {
    display: inline-flex;
    align-items: center;
    max-width: 150px;
    height: 20px;
    min-width: 0;
    overflow: hidden;
    padding: 0 7px;
    color: #9cebe0;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
    border: 1px solid rgba(92, 226, 207, 0.18);
    border-radius: 999px;
    background: rgba(92, 226, 207, 0.07);
    font-size: 9px;
    font-weight: 820;
  }

  .run-chip.artifact {
    color: #c8d2d0;
    border-color: rgba(255, 255, 255, 0.105);
    background: rgba(255, 255, 255, 0.045);
  }

  .run-chip:hover {
    color: #f3fbfa;
    border-color: rgba(92, 226, 207, 0.34);
    background: rgba(92, 226, 207, 0.12);
  }

  .run-status-badge,
  .run-stage-badge,
  .run-step-marker {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    color: #081714;
    border-radius: 999px;
    font-weight: 850;
    line-height: 1;
  }

  .run-status-badge,
  .run-stage-badge {
    height: 20px;
    padding: 0 8px;
    font-size: 10px;
  }

  .run-stage-badge {
    max-width: 110px;
    overflow: hidden;
    color: #d8e0dd;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: rgba(255, 255, 255, 0.12);
  }

  .run-step-marker {
    width: 18px;
    height: 18px;
    font-size: 10px;
  }

  .run-status-badge.live,
  .run-stage-badge.live,
  .run-step-marker.live {
    background: #5ce2cf;
  }

  .run-status-badge.good,
  .run-stage-badge.good,
  .run-step-marker.good {
    background: #8bdc9b;
  }

  .run-status-badge.bad,
  .run-stage-badge.bad,
  .run-step-marker.bad {
    background: #f36f6f;
  }

  .run-status-badge.attention,
  .run-stage-badge.attention,
  .run-step-marker.attention {
    color: #211606;
    background: #d8aa55;
  }

  .run-status-badge.idle,
  .run-stage-badge.idle,
  .run-step-marker.idle {
    color: #d8e0dd;
    background: rgba(255, 255, 255, 0.12);
  }

  .activity-row-main {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .worktree-row-main {
    gap: 2px;
  }

  .activity-row-main strong,
  .activity-row-main small,
  .activity-repo-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-row-main strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 790;
  }

  .worktree-row-main strong {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .worktree-row-main strong > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-row-main small,
  .activity-repo-row small {
    color: #8d9995;
    font-size: 10px;
    font-weight: 720;
  }

  .worktree-safety-line {
    display: flex;
    flex-wrap: wrap;
    gap: 3px 7px;
    line-height: 1.25;
  }

  .worktree-safety-line span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .worktree-recommendation {
    color: #78837f;
  }

  .activity-repo-row > small {
    grid-column: 1 / 4;
  }

  .activity-commit-meta {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 5px;
    min-width: 0;
  }

  .activity-row-actions {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 3px;
    min-width: 0;
  }

  .activity-row-actions button,
  .activity-icon-link {
    display: grid;
    place-items: center;
    width: 23px;
    height: 23px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .activity-row-actions button:disabled {
    opacity: 0.38;
    cursor: default;
  }

  .activity-row-actions button.worktree-primary-action.backup,
  .worktree-context-actions button.worktree-primary-action.backup {
    color: #d8aa55;
    border-color: rgba(216, 170, 85, 0.2);
    background: rgba(216, 170, 85, 0.08);
  }

  .activity-row-actions button.worktree-primary-action.cleanup,
  .worktree-context-actions button.worktree-primary-action.cleanup {
    color: #79eadb;
    border-color: rgba(92, 226, 207, 0.28);
    background: rgba(92, 226, 207, 0.1);
  }

  .activity-row-actions button:hover,
  .activity-row-actions button:focus-visible,
  .activity-icon-link:hover,
  .activity-icon-link:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.36);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .activity-subheading {
    margin-top: 8px;
    color: #6fdfcf;
    font-size: 10px;
    font-weight: 850;
    text-transform: uppercase;
  }

  .activity-empty {
    display: grid;
    place-items: center;
    min-height: 88px;
    color: #75817d;
    border: 1px dashed rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.025);
    font-size: 12px;
    font-weight: 750;
  }

  .activity-empty.compact {
    min-height: 42px;
    font-size: 10px;
  }

  .project-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 36px 36px 96px;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  select,
  .scan-button,
  .icon-button,
  .form-button {
    height: 36px;
    min-width: 0;
    color: #f3f5f4;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.055);
  }

  select {
    width: 100%;
    padding: 0 12px;
    outline: 0;
  }

  select:focus,
  .scan-button:focus-visible,
  .icon-button:focus-visible,
  .form-button:focus-visible {
    border-color: rgba(92, 226, 207, 0.58);
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
  }

  .scan-button {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    color: #cbd3d1;
    font-size: 12px;
    font-weight: 760;
    cursor: pointer;
  }

  .scan-button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  .scan-button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon-button {
    display: grid;
    place-items: center;
    width: 36px;
    padding: 0;
    color: #cbd3d1;
    cursor: pointer;
  }

  .icon-button:hover {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.1);
  }

  .icon-button.danger {
    width: 28px;
    height: 28px;
    color: #f1a9a0;
    border-radius: 8px;
  }

  .icon-button.danger:hover {
    background: rgba(225, 109, 93, 0.13);
  }

  .project-path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    margin-bottom: 2px;
    color: #818d89;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    font-weight: 650;
  }

  .project-path-row span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-progress {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 10px;
    min-height: 18px;
    margin: 4px 0 2px;
    color: #8fd8cf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 760;
  }

  .scan-progress span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .project-form {
    display: grid;
    gap: 8px;
    padding: 10px;
    margin-top: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
  }

  .project-form label {
    display: grid;
    gap: 5px;
  }

  .project-form label span {
    color: #9aa5a1;
    font-size: 11px;
    font-weight: 760;
  }

  .project-form input {
    height: 32px;
    padding: 0 9px;
    color: #f3f5f4;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    outline: 0;
    background: rgba(0, 0, 0, 0.18);
  }

  .project-form input:focus {
    border-color: rgba(92, 226, 207, 0.58);
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
  }

  .project-form-error {
    margin: 0;
    color: #f1a9a0;
    font-size: 11px;
    font-weight: 700;
  }

  .form-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .form-button {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr);
    align-items: center;
    gap: 6px;
    padding: 0 9px;
    color: #cbd3d1;
    font-size: 12px;
    font-weight: 760;
    cursor: pointer;
  }

  .form-button.primary {
    color: #071b18;
    border-color: rgba(111, 223, 207, 0.72);
    background: #6fdfcf;
  }

  .form-button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .search-box {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    margin-bottom: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9px;
    color: #9aa5a1;
    background: rgba(255, 255, 255, 0.045);
  }

  .search-box:focus-within {
    border-color: rgba(92, 226, 207, 0.58);
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
  }

  .global-search-panel {
    display: grid;
    gap: 7px;
    padding-bottom: 12px;
    margin-bottom: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .global-search-box {
    display: grid;
    grid-template-columns: 17px minmax(0, 1fr) 30px;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 4px 0 10px;
    color: #9aa5a1;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.035);
  }

  .global-search-box:focus-within {
    border-color: rgba(92, 226, 207, 0.48);
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.1);
  }

  .source-search-submit {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    color: #071b18;
    border: 0;
    border-radius: 7px;
    background: #6fdfcf;
    cursor: pointer;
  }

  .source-search-submit:disabled {
    color: #8d9995;
    background: rgba(255, 255, 255, 0.08);
    cursor: default;
  }

  .source-search-summary {
    min-width: 0;
    overflow: hidden;
    color: #8fd8cf;
    font-size: 10px;
    font-weight: 760;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-search-results {
    display: grid;
    gap: 4px;
    max-height: 184px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-width: thin;
  }

  .source-search-results button {
    display: grid;
    gap: 3px;
    width: 100%;
    min-width: 0;
    padding: 7px 8px;
    color: #cbd3d1;
    text-align: left;
    border: 0;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.045);
    cursor: pointer;
  }

  .source-search-results button:hover {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.11);
  }

  .source-search-results span {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  .source-search-results strong,
  .source-search-results small,
  .source-search-results code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-search-results strong {
    font-size: 11px;
    font-weight: 780;
  }

  .source-search-results small {
    color: #8d9995;
    font-size: 10px;
    font-weight: 700;
  }

  .source-search-results code {
    color: #d7dddb;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
  }

  input {
    width: 100%;
    min-width: 0;
    color: #f3f5f4;
    border: 0;
    outline: 0;
    background: transparent;
  }

  input::placeholder {
    color: #6f7976;
  }

  .recent-panel {
    display: grid;
    gap: 7px;
    padding-bottom: 12px;
    margin-bottom: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .recent-heading {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    color: #aeb8b5;
    font-size: 12px;
    font-weight: 700;
  }

  .recent-heading-icon,
  .recent-file-icon {
    display: grid;
    place-items: center;
    min-width: 0;
  }

  .recent-heading-icon {
    color: #6fdfcf;
  }

  .recent-list {
    display: grid;
    gap: 4px;
  }

  .recent-list button {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 38px;
    padding: 0 8px;
    color: #cbd3d1;
    text-align: left;
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
  }

  .recent-list button:hover,
  .recent-list button.active {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.11);
  }

  .recent-file-icon {
    color: #8d9995;
  }

  .recent-list button.active .recent-file-icon {
    color: #6fdfcf;
  }

  .recent-list span {
    display: grid;
    min-width: 0;
  }

  .recent-list strong,
  .recent-list small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recent-list strong {
    font-size: 12px;
    line-height: 1.15;
  }

  .recent-list small {
    color: #7f8b87;
    font-size: 10px;
    font-weight: 700;
    line-height: 1.25;
  }

  .source-list-panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .tree-panel-header {
    min-width: 0;
  }

  .tree-heading {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    color: #aeb8b5;
    font-size: 12px;
    font-weight: 700;
  }

  .tree-heading strong {
    color: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
  }

  .scan-summary-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 22px;
    align-items: center;
    gap: 5px;
    margin: -3px 0 7px;
  }

  .scan-summary {
    min-width: 0;
    overflow: hidden;
    color: #7f8b87;
    font-size: 10px;
    font-weight: 720;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-diagnostic-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 20px;
    border: 1px solid rgba(143, 216, 207, 0.14);
    border-radius: 5px;
    padding: 0;
    background: rgba(143, 216, 207, 0.06);
    color: #8a9693;
  }

  .scan-diagnostic-button:hover {
    border-color: rgba(143, 216, 207, 0.34);
    background: rgba(143, 216, 207, 0.12);
    color: #b8d6d1;
  }

  .index-summary {
    min-width: 0;
    margin: -4px 0 8px;
    overflow: hidden;
    color: #8fd8cf;
    font-size: 10px;
    font-weight: 760;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-stats {
    min-width: 0;
    margin: -5px 0 8px;
    overflow: hidden;
    color: #6f7b78;
    font-size: 9.5px;
    font-weight: 720;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-health-note {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    margin: -1px 0 8px;
    border: 1px solid rgba(216, 170, 85, 0.24);
    border-radius: 6px;
    padding: 6px 7px;
    background: rgba(216, 170, 85, 0.08);
    color: #d8cba8;
    font-size: 10px;
    font-weight: 720;
    line-height: 1.25;
  }

  .scan-health-note span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-health-note button {
    height: 20px;
    border: 1px solid rgba(216, 170, 85, 0.28);
    border-radius: 5px;
    padding: 0 7px;
    background: rgba(216, 170, 85, 0.12);
    color: #f1d99b;
    font-size: 10px;
    font-weight: 820;
  }

  .scan-more-button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
    min-height: 24px;
    margin: 0 0 8px;
    border: 1px solid rgba(111, 223, 207, 0.22);
    border-radius: 6px;
    padding: 0 8px;
    overflow: hidden;
    background: rgba(111, 223, 207, 0.09);
    color: #8fd8cf;
    font-size: 10px;
    font-weight: 800;
    white-space: nowrap;
  }

  .scan-more-button:hover {
    border-color: rgba(111, 223, 207, 0.42);
    background: rgba(111, 223, 207, 0.15);
  }

  .file-tree {
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 4px 4px 4px 0;
    scrollbar-color: rgba(174, 184, 181, 0.58) rgba(255, 255, 255, 0.055);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .file-tree::-webkit-scrollbar {
    width: 10px;
  }

  .file-tree::-webkit-scrollbar-track {
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.055);
  }

  .file-tree::-webkit-scrollbar-thumb {
    border: 2px solid rgba(19, 21, 21, 0.94);
    border-radius: 999px;
    background: rgba(174, 184, 181, 0.58);
  }

  .file-tree::-webkit-scrollbar-thumb:hover {
    background: rgba(218, 225, 223, 0.72);
  }

  .tree-virtual-spacer {
    height: var(--tree-spacer-height);
    min-height: var(--tree-spacer-height);
    pointer-events: none;
  }

  .file-tree button {
    display: grid;
    grid-template-columns: calc(var(--tree-level, 0) * 14px) 14px 18px minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 7px;
    width: 100%;
    height: 30px;
    padding: 0 7px;
    color: #cbd3d1;
    text-align: left;
    border-radius: 7px;
    background: transparent;
    transition:
      background 150ms ease,
      color 150ms ease,
      transform 150ms ease;
  }

  .file-tree button:hover,
  .file-tree button.active {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.12);
  }

  .file-tree button:active {
    transform: translateY(1px);
  }

  .file-tree button.folder-row {
    color: #d7dddb;
    font-weight: 700;
  }

  .tree-chevron,
  .tree-icon {
    display: grid;
    place-items: center;
    width: 16px;
    min-width: 0;
    color: #8d9995;
  }

  .tree-icon {
    color: #5fa7e8;
  }

  .folder-row .tree-icon {
    color: #d0a94f;
  }

  .file-tree strong,
  .file-tree small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-tree strong {
    font-size: 12px;
  }

  .file-tree small {
    color: #7f8b87;
    font-size: 10px;
    font-weight: 700;
  }

  .file-tree button.file-row small {
    display: none;
  }

  .git-status-badge {
    display: inline-grid;
    place-items: center;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    color: #071b18;
    border-radius: 5px;
    background: #d8aa55;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 850;
    line-height: 1;
  }

  .tree-skeleton {
    width: var(--line-width);
    height: 22px;
    margin: 5px 8px;
    border-radius: 7px;
    background: linear-gradient(90deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.105), rgba(255, 255, 255, 0.045));
    background-size: 180% 100%;
    animation: shimmer 1.2s ease-in-out infinite;
  }

  .empty-tree,
  .empty-preview {
    display: grid;
    place-items: center;
    gap: 8px;
    min-height: 128px;
    color: #9aa5a1;
    text-align: center;
    font-size: 12px;
    font-weight: 700;
  }

  .empty-preview {
    flex: 1 1 auto;
    min-height: 520px;
    border: 1px dashed rgba(255, 255, 255, 0.13);
    border-radius: 13px;
    background: rgba(255, 255, 255, 0.035);
  }

  .empty-preview strong {
    color: #f3f6f5;
    font-size: 16px;
  }

  .empty-preview span {
    color: #9aa5a1;
    font-weight: 650;
  }

  .workspace {
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 9px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent 36%),
      rgba(24, 26, 26, 0.96);
  }

  .topbar {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) auto;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
  }

  .topbar > div:first-child {
    min-width: 0;
  }

  .topbar h2 {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    font-size: 18px;
    line-height: 1.1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .topbar .eyebrow {
    display: none;
  }

  .topbar-tools {
    display: inline-flex;
    justify-content: flex-end;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .topbar-command-button {
    display: inline-grid;
    grid-auto-flow: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    color: #aab6b2;
    border: 1px solid rgba(255, 255, 255, 0.105);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.045);
    font-size: 10px;
    font-weight: 820;
    cursor: pointer;
  }

  .topbar-command-button:hover,
  .topbar-command-button:focus-visible,
  .topbar-command-button[aria-expanded='true'] {
    color: #f2f6f5;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .topbar-command-button span {
    min-width: 0;
    white-space: nowrap;
  }

  .view-menu-anchor {
    position: relative;
    display: inline-grid;
    place-items: center;
  }

  .view-menu {
    position: absolute;
    z-index: 9;
    top: calc(100% + 6px);
    right: 0;
    display: grid;
    gap: 8px;
    width: 270px;
    min-width: 0;
    max-height: min(680px, calc(100vh - 104px));
    overflow-y: auto;
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 9px;
    background: rgba(22, 25, 25, 0.98);
    box-shadow: 0 20px 54px rgba(0, 0, 0, 0.36);
    scrollbar-color: rgba(174, 184, 181, 0.52) rgba(255, 255, 255, 0.04);
    scrollbar-width: thin;
  }

  .view-menu-section {
    display: grid;
    gap: 5px;
    min-width: 0;
  }

  .view-menu-section > span,
  .view-terminal-picker > span {
    color: #7f8b88;
    font-size: 9px;
    font-weight: 850;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .view-menu-button-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
    min-width: 0;
  }

  .view-menu-button-grid.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .view-menu-button-grid button,
  .view-menu-wide-button {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    height: 25px;
    padding: 0 7px;
    color: #aab6b2;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 10px;
    font-weight: 820;
    white-space: nowrap;
    cursor: pointer;
  }

  .view-menu-button-grid button:hover,
  .view-menu-button-grid button:focus-visible,
  .view-menu-wide-button:hover,
  .view-menu-wide-button:focus-visible {
    color: #edf4f2;
    outline: 0;
    background: rgba(255, 255, 255, 0.08);
  }

  .view-menu-button-grid button.active {
    color: #dffdf8;
    background: rgba(92, 226, 207, 0.18);
  }

  .view-menu-wide-button {
    width: 100%;
    justify-content: start;
  }

  .dock-panel-manager-list {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .dock-panel-manager-row {
    display: grid;
    grid-template-columns: minmax(0, 74px) minmax(0, 1fr);
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 30px;
    padding: 4px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.025);
  }

  .dock-panel-manager-row > div:first-child {
    display: grid;
    gap: 1px;
    min-width: 0;
  }

  .dock-panel-manager-row strong,
  .dock-panel-manager-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dock-panel-manager-row strong {
    color: #e8eeec;
    font-size: 9.5px;
    font-weight: 850;
  }

  .dock-panel-manager-row small {
    color: #7f8b88;
    font-size: 8.5px;
    font-weight: 760;
  }

  .dock-panel-manager-actions {
    display: flex;
    justify-content: flex-end;
    gap: 3px;
    min-width: 0;
    overflow: hidden;
  }

  .dock-panel-manager-actions button {
    flex: 0 1 auto;
    min-width: 0;
    height: 22px;
    padding: 0 6px;
    color: #aab6b2;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.035);
    font: inherit;
    font-size: 8.5px;
    font-weight: 820;
    white-space: nowrap;
    cursor: pointer;
  }

  .dock-panel-manager-actions button:hover,
  .dock-panel-manager-actions button:focus-visible {
    color: #edf4f2;
    outline: 0;
    background: rgba(255, 255, 255, 0.08);
  }

  .dock-panel-manager-actions button.active,
  .dock-panel-manager-actions button:disabled {
    color: #dffdf8;
    background: rgba(92, 226, 207, 0.16);
    cursor: default;
  }

  .view-terminal-picker {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding-top: 2px;
  }

  .view-terminal-picker select {
    width: 100%;
    height: 26px;
    min-width: 0;
    color: #dffdf8;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
    font-size: 10px;
    font-weight: 820;
  }

  .context-identity-strip {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    min-height: 18px;
    overflow: hidden;
    margin: -1px 0 3px;
    padding: 0 2px;
  }

  .context-identity-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: 0 1 auto;
    min-width: 0;
    max-width: 28%;
    height: 18px;
    color: #cbd3d1;
  }

  .context-identity-key,
  .context-identity-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .context-identity-key {
    flex: 0 0 auto;
    color: #87918e;
    font-size: 7.5px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .context-identity-value {
    color: #f0f4f3;
    font-size: 9px;
    font-weight: 760;
  }

  .dock-panel-tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    min-height: 22px;
    margin: 0 0 5px;
    overflow: hidden;
  }

  .dock-panel-tabs.empty {
    height: 0;
    min-height: 0;
    margin: 0;
  }

  .dock-drop-zones {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    min-height: 22px;
    margin: -2px 0 5px;
    overflow: hidden;
  }

  .dock-drop-zone {
    display: inline-grid;
    place-items: center;
    min-width: 54px;
    height: 22px;
    padding: 0 8px;
    color: #aab6b2;
    border: 1px dashed rgba(92, 226, 207, 0.28);
    border-radius: 6px;
    background: rgba(92, 226, 207, 0.055);
    font-size: 9px;
    font-weight: 860;
    cursor: pointer;
  }

  .dock-drop-zone.active,
  .dock-drop-zone:hover,
  .dock-drop-zone:focus-visible {
    color: #dffdf8;
    border-color: rgba(92, 226, 207, 0.58);
    outline: 0;
    background: rgba(92, 226, 207, 0.14);
  }

  .dock-panel-tab-group {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    max-width: 100%;
    padding: 2px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
  }

  .dock-panel-tab {
    display: inline-flex;
    align-items: center;
    min-width: 0;
    height: 18px;
    overflow: hidden;
    border-radius: 4px;
    background: transparent;
  }

  .dock-panel-tab-label {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    height: 18px;
    padding: 0 6px;
    color: #9facaa;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 9px;
    font-weight: 820;
    line-height: 1;
    cursor: pointer;
  }

  .dock-panel-tab-close {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    height: 18px;
    width: 17px;
    padding: 0;
    color: #9facaa;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 9px;
    font-weight: 820;
    line-height: 1;
    cursor: pointer;
    opacity: 0.72;
  }

  .dock-panel-tab-move {
    display: inline-block;
    min-width: 0;
    height: 18px;
    width: 18px;
    padding: 0;
    color: #9facaa;
    border: 0;
    background: rgba(255, 255, 255, 0.03);
    font: inherit;
    font-size: 8px;
    font-weight: 820;
    line-height: 1;
    cursor: pointer;
    opacity: 0.72;
    appearance: none;
    text-align: center;
  }

  .dock-panel-tab-label:hover,
  .dock-panel-tab-label:focus-visible,
  .dock-panel-tab-move:hover,
  .dock-panel-tab-move:focus-visible,
  .dock-panel-tab-close:hover,
  .dock-panel-tab-close:focus-visible {
    color: #eef6f4;
    outline: 0;
    background: rgba(255, 255, 255, 0.07);
  }

  .dock-panel-tab.active {
    background: rgba(92, 226, 207, 0.18);
  }

  .dock-panel-tab.active .dock-panel-tab-label {
    color: #dffdf8;
  }

  .dock-panel-tab.active .dock-panel-tab-close {
    color: #c9f6ef;
  }

  .dock-panel-tab.active .dock-panel-tab-move {
    color: #c9f6ef;
  }

  .workspace-arrangement {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .workspace-arrangement.context-top {
    grid-template-rows: auto minmax(0, 1fr);
  }

  .workspace-arrangement.context-side {
    grid-template-columns: minmax(0, 1fr) 6px var(--context-pane-width);
    grid-template-rows: minmax(0, 1fr);
    gap: 0;
  }

  .workspace-arrangement.context-bottom {
    grid-template-rows: minmax(0, 1fr) 6px minmax(180px, var(--context-pane-height));
    gap: 0;
  }

  .workspace-context-column,
  .workspace-main-column {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .workspace-context-column {
    display: grid;
    min-height: 0;
  }

  .workspace-main-column {
    display: flex;
    flex-direction: column;
  }

  .workspace-arrangement.context-side .workspace-main-column {
    grid-column: 1;
    grid-row: 1;
  }

  .workspace-arrangement.context-side .workspace-context-column {
    grid-column: 3;
    grid-row: 1;
    padding-left: 6px;
  }

  .workspace-arrangement.context-bottom .workspace-main-column {
    grid-column: 1;
    grid-row: 1;
  }

  .workspace-arrangement.context-bottom .workspace-context-column {
    grid-column: 1;
    grid-row: 3;
    min-height: 0;
    padding-top: 6px;
  }

  .context-pane-resizer {
    display: none;
    width: 6px;
    min-width: 0;
    padding: 0;
    cursor: col-resize;
    border: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.045);
    border-right: 1px solid rgba(255, 255, 255, 0.045);
    background: rgba(255, 255, 255, 0.025);
  }

  .workspace-arrangement.context-side .context-pane-resizer {
    display: block;
    grid-column: 2;
    grid-row: 1;
  }

  .workspace-arrangement.context-bottom .context-pane-resizer {
    display: block;
    grid-column: 1;
    grid-row: 2;
    width: auto;
    height: 6px;
    cursor: row-resize;
    border-top: 1px solid rgba(255, 255, 255, 0.045);
    border-bottom: 1px solid rgba(255, 255, 255, 0.045);
    border-left: 0;
    border-right: 0;
  }

  .context-pane-resizer:hover,
  .context-pane-resizer:focus-visible {
    outline: 0;
    background: rgba(92, 226, 207, 0.18);
  }

  :global(body.resizing-context-pane) {
    cursor: col-resize;
    user-select: none;
  }

  :global(body.resizing-context-pane-bottom) {
    cursor: row-resize;
    user-select: none;
  }

  .context-panel-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 6px;
    min-width: 0;
    margin: -2px 0 8px;
  }

  .context-panel-grid.collapsed {
    display: none;
  }

  .context-panel-grid.stacked {
    grid-template-columns: minmax(0, 1fr);
  }

  .context-stack-tabs {
    display: flex;
    min-width: 0;
    padding: 2px;
    gap: 2px;
    overflow-x: auto;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 8px;
    background: rgba(10, 12, 12, 0.34);
    scrollbar-width: none;
  }

  .context-stack-tabs::-webkit-scrollbar {
    display: none;
  }

  .context-stack-tabs button {
    min-width: max-content;
    height: 24px;
    padding: 0 9px;
    color: #9da8a5;
    font: inherit;
    font-size: 10px;
    font-weight: 800;
    border: 0;
    border-radius: 6px;
    background: transparent;
  }

  .context-stack-tabs button:hover,
  .context-stack-tabs button:focus-visible {
    color: #ecf4f2;
    outline: 0;
    background: rgba(255, 255, 255, 0.06);
  }

  .context-stack-tabs button.active {
    color: #0a1816;
    background: #67dfd1;
  }

  .workspace-arrangement.context-side .context-panel-grid {
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    height: 100%;
    margin: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .workspace-arrangement.context-bottom .context-panel-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    align-content: start;
    height: 100%;
    margin: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .workspace-arrangement.context-side .runtime-context-row,
  .workspace-arrangement.context-side .agent-session-row,
  .workspace-arrangement.context-side .worktree-context-row,
  .workspace-arrangement.context-side .repo-dashboard-row,
  .workspace-arrangement.context-side .orchestration-context-row {
    grid-template-columns: minmax(0, 1fr);
    align-items: start;
  }

  .workspace-arrangement.context-side .orchestration-context-list,
  .workspace-arrangement.context-side .runtime-context-list,
  .workspace-arrangement.context-side .agent-session-list,
  .workspace-arrangement.context-side .worktree-context-list,
  .workspace-arrangement.context-side .repo-dashboard-list {
    max-height: 180px;
  }

  .orchestration-context-panel,
  .runtime-context-panel,
  .agent-session-panel,
  .worktree-context-panel,
  .repo-dashboard-panel {
    display: grid;
    gap: 6px;
    min-width: 0;
    padding: 7px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
  }

  .orchestration-context-header,
  .runtime-context-header,
  .agent-session-header,
  .worktree-context-header,
  .repo-dashboard-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .orchestration-context-header div,
  .runtime-context-header div,
  .agent-session-header div,
  .worktree-context-header div,
  .repo-dashboard-header div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .orchestration-context-header strong,
  .orchestration-context-header span,
  .runtime-context-header strong,
  .runtime-context-header span,
  .agent-session-header strong,
  .agent-session-header span,
  .worktree-context-header strong,
  .worktree-context-header span,
  .repo-dashboard-header strong,
  .repo-dashboard-header span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .orchestration-context-header strong,
  .runtime-context-header strong,
  .agent-session-header strong,
  .worktree-context-header strong,
  .repo-dashboard-header strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 800;
  }

  .orchestration-context-header span,
  .runtime-context-header span,
  .agent-session-header span,
  .worktree-context-header span,
  .repo-dashboard-header span {
    color: #8d9995;
    font-size: 9px;
    font-weight: 740;
  }

  .context-card-actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .context-card-actions .file-action-button {
    width: 24px;
    height: 22px;
    border-radius: 6px;
  }

  .context-card-close {
    color: #8d9995;
  }

  .context-card-close:hover {
    color: #f2f6f5;
    background: rgba(255, 255, 255, 0.08);
  }

  .context-restore-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    height: 24px;
    padding: 0 8px;
    color: #8fd8cf;
    border: 1px solid rgba(92, 226, 207, 0.22);
    border-radius: 6px;
    background: rgba(92, 226, 207, 0.08);
    font-size: 10px;
    font-weight: 800;
    cursor: pointer;
  }

  .orchestration-context-list,
  .runtime-context-list,
  .agent-session-list,
  .worktree-context-list,
  .repo-dashboard-list {
    display: grid;
    gap: 4px;
    max-height: 96px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.48) rgba(255, 255, 255, 0.045);
    scrollbar-width: thin;
  }

  .runtime-context-list {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .agent-session-list {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .worktree-context-list {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .repo-dashboard-list {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .orchestration-context-list {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .orchestration-context-row,
  .runtime-context-row,
  .agent-session-row,
  .worktree-context-row,
  .repo-dashboard-row {
    display: grid;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 5px 6px;
    color: #cbd3d1;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.14);
  }

  .runtime-context-row {
    grid-template-columns: auto auto minmax(0, 0.8fr) minmax(0, 0.75fr) minmax(0, 1.6fr);
  }

  .agent-session-row {
    grid-template-columns: auto minmax(0, 1.1fr) minmax(0, 0.72fr) minmax(0, 1fr);
  }

  .worktree-context-row {
    grid-template-columns:
      auto minmax(0, 0.95fr) minmax(0, 0.5fr) auto minmax(0, 1fr) minmax(0, 1.1fr)
      auto;
  }

  .repo-dashboard-row {
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 0.75fr) minmax(0, 0.7fr) minmax(0, 0.82fr) minmax(0, 0.65fr);
  }

  .orchestration-context-row {
    grid-template-columns: auto minmax(0, 1.1fr) minmax(0, 0.72fr) minmax(0, 0.85fr);
  }

  .orchestration-context-row.bad {
    background: rgba(216, 95, 95, 0.1);
  }

  .orchestration-context-row.attention {
    background: rgba(216, 170, 85, 0.09);
  }

  .worktree-context-row.blocked {
    background: rgba(216, 170, 85, 0.09);
  }

  .worktree-context-row.protected {
    background: rgba(255, 255, 255, 0.04);
  }

  .worktree-context-row.ready {
    background: rgba(92, 226, 207, 0.06);
  }

  .repo-dashboard-row.dirty {
    background: rgba(216, 170, 85, 0.09);
  }

  .runtime-port,
  .orchestration-context-row strong,
  .orchestration-context-row span,
  .orchestration-context-row small,
  .agent-provider-badge,
  .worktree-status-badge,
  .repo-branch-badge,
  .runtime-context-row strong,
  .runtime-context-row span,
  .runtime-context-row small,
  .agent-session-row strong,
  .agent-session-row span,
  .agent-session-row small,
  .worktree-context-row strong,
  .worktree-context-row span,
  .worktree-context-row small,
  .worktree-context-row em,
  .worktree-context-main,
  .worktree-context-main strong,
  .worktree-context-main small,
  .repo-dashboard-main,
  .repo-dashboard-main strong,
  .repo-dashboard-main small,
  .repo-dashboard-metric,
  .repo-dashboard-metric span,
  .repo-dashboard-metric strong,
  .repo-dashboard-row em {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .runtime-port {
    color: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    font-weight: 820;
  }

  .runtime-url-link {
    display: inline-grid;
    place-items: center;
    height: 22px;
    padding: 0 7px;
    color: #7ce5d5;
    text-decoration: none;
    white-space: nowrap;
    border: 1px solid rgba(92, 226, 207, 0.22);
    border-radius: 999px;
    background: rgba(92, 226, 207, 0.08);
    font-size: 10px;
    font-weight: 850;
  }

  .agent-provider-badge {
    display: inline-grid;
    place-items: center;
    height: 20px;
    padding: 0 7px;
    color: #081916;
    border-radius: 999px;
    background: #81d6e4;
    font-size: 10px;
    font-weight: 820;
    text-transform: capitalize;
  }

  .worktree-status-badge {
    display: inline-grid;
    place-items: center;
    height: 20px;
    padding: 0 7px;
    color: #071b18;
    border-radius: 999px;
    background: #6fdfcf;
    font-size: 10px;
    font-weight: 820;
  }

  .worktree-status-badge.review {
    color: #dce5e2;
    background: rgba(255, 255, 255, 0.14);
  }

  .worktree-status-badge.protected {
    color: #dce5e2;
    background: rgba(255, 255, 255, 0.16);
  }

  .worktree-status-badge.ready {
    color: #071b18;
    background: #6fdfcf;
  }

  .worktree-status-badge.blocked {
    color: #211606;
    background: #d8aa55;
  }

  .repo-branch-badge {
    color: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 820;
  }

  .worktree-context-row.blocked .worktree-status-badge {
    color: #211606;
    background: #d8aa55;
  }

  .worktree-context-main {
    display: grid;
    gap: 2px;
  }

  .runtime-context-row strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 780;
  }

  .agent-session-row strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 780;
  }

  .worktree-context-row strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 780;
  }

  .worktree-task-empty {
    color: #6f7a76;
    font-size: 9px;
    font-weight: 760;
  }

  .worktree-context-actions {
    display: inline-flex;
    justify-content: flex-end;
    gap: 3px;
    min-width: 0;
  }

  .worktree-context-actions button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .worktree-context-actions button:hover,
  .worktree-context-actions button:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.36);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .worktree-context-actions button:disabled {
    opacity: 0.38;
    cursor: default;
  }

  .repo-dashboard-main {
    display: grid;
    gap: 2px;
  }

  .repo-dashboard-main strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 780;
  }

  .repo-dashboard-main small {
    color: #8d9995;
    font-size: 10px;
    font-weight: 720;
  }

  .repo-dashboard-metric {
    display: grid;
    gap: 2px;
  }

  .repo-dashboard-metric span {
    color: #6fdfcf;
    font-size: 9px;
    font-weight: 840;
    text-transform: uppercase;
  }

  .repo-dashboard-metric strong {
    color: #cbd3d1;
    font-size: 10px;
    font-weight: 760;
  }

  .repo-task-link {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    max-width: 100%;
    min-height: 20px;
    padding: 0 7px;
    color: #071b18;
    border-radius: 999px;
    background: #6fdfcf;
    font-size: 9px;
    font-weight: 900;
    line-height: 1;
    text-decoration: none;
    white-space: nowrap;
  }

  .runtime-context-row span,
  .runtime-context-row small,
  .agent-session-row span,
  .agent-session-row small,
  .worktree-context-row span,
  .worktree-context-row small,
  .worktree-context-row em,
  .repo-dashboard-row em {
    color: #8d9995;
    font-size: 10px;
    font-style: normal;
    font-weight: 720;
  }

  .tab-strip {
    display: flex;
    gap: 4px;
    min-width: 0;
    padding-bottom: 3px;
    margin: 0 0 6px;
    overflow-x: auto;
  }

  .source-tab {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 22px;
    align-items: center;
    flex: 0 1 190px;
    min-width: 132px;
    max-width: 190px;
    height: 28px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.045);
  }

  .source-tab.active {
    border-color: rgba(92, 226, 207, 0.42);
    background: rgba(92, 226, 207, 0.12);
  }

  .source-tab.dirty {
    border-color: rgba(216, 170, 85, 0.44);
  }

  .tab-select-button {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr) auto 7px;
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 100%;
    padding: 0 5px 0 7px;
    color: #cbd3d1;
    text-align: left;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .tab-file-icon {
    display: grid;
    place-items: center;
    min-width: 0;
    color: #8d9995;
  }

  .source-tab.active .tab-file-icon {
    color: #6fdfcf;
  }

  .tab-dirty-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: #d8aa55;
    box-shadow: 0 0 0 2px rgba(216, 170, 85, 0.14);
  }

  .tab-select-button span,
  .tab-select-button small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-select-button span {
    font-size: 11px;
    font-weight: 760;
  }

  .tab-select-button small {
    color: #8d9995;
    display: none;
    font-size: 9px;
    font-weight: 760;
  }

  .tab-close-button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    color: #8d9995;
    border: 0;
    border-radius: 5px;
    background: transparent;
    cursor: pointer;
  }

  .tab-close-button:hover {
    color: #f2f6f5;
    background: rgba(255, 255, 255, 0.08);
  }

  .tab-select-button:focus-visible,
  .tab-close-button:focus-visible {
    outline: 0;
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
  }

  .path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
    color: #87918e;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
  }

  .path-row span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path-row strong {
    color: #cbd3d1;
  }

  .file-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .file-action-button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 28px;
    color: #b9c5c1;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.045);
    cursor: pointer;
  }

  .file-action-button:hover {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.1);
  }

  .file-action-button:focus-visible {
    border-color: rgba(92, 226, 207, 0.58);
    outline: 0;
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
  }

  .file-action-button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  .file-action-feedback {
    margin: -4px 0 10px;
    color: #7ce5d5;
    font-size: 11px;
    font-weight: 750;
  }

  .inline-error {
    display: inline-grid;
    grid-auto-flow: column;
    align-items: center;
    gap: 7px;
    margin-bottom: 12px;
    color: #f1b8a4;
    font-size: 12px;
    font-weight: 650;
  }

  .editor-frame {
    display: grid;
    grid-template-rows: 28px minmax(0, 1fr);
    flex: 1 1 auto;
    height: auto;
    min-height: 0;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 8px;
    background: #17191e;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.07),
      0 18px 45px rgba(0, 0, 0, 0.2);
  }

  .terminal-launchpad {
    display: grid;
    flex: 0 0 auto;
    gap: 6px;
    min-width: 0;
    max-height: 392px;
    margin-top: 6px;
    overflow: hidden;
    padding: 7px;
    border: 1px solid rgba(255, 255, 255, 0.105);
    border-radius: 8px;
    background: rgba(15, 18, 18, 0.92);
  }

  .terminal-launchpad-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .terminal-launchpad-header > div:first-child,
  .terminal-launchpad-actions,
  .terminal-inline-picker {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .terminal-launchpad-header > div:first-child {
    gap: 6px;
    color: #dce4e2;
  }

  .terminal-launchpad-header strong,
  .terminal-launchpad-header span,
  .terminal-launchpad-row strong,
  .terminal-launchpad-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .terminal-launchpad-header strong {
    font-size: 11px;
    font-weight: 860;
  }

  .terminal-launchpad-header span {
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
  }

  .terminal-launchpad-actions {
    justify-content: end;
    gap: 5px;
  }

  .terminal-launchpad-actions .file-action-button {
    display: inline-flex;
    width: auto;
    min-width: 28px;
    height: 24px;
    gap: 5px;
    padding: 0 8px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 820;
  }

  .terminal-launchpad-actions .file-action-button.icon-only {
    width: 28px;
    padding: 0;
  }

  .terminal-inline-picker {
    gap: 5px;
  }

  .terminal-inline-picker span {
    color: #7f8b88;
    font-size: 8px;
    font-weight: 850;
    text-transform: uppercase;
  }

  .terminal-inline-picker select {
    width: 94px;
    height: 24px;
    min-width: 0;
    color: #dffdf8;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.04);
    font-size: 10px;
    font-weight: 820;
  }

  .embedded-terminal-panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 5px;
    min-width: 0;
    padding: 6px;
    border: 1px solid rgba(92, 226, 207, 0.11);
    border-radius: 7px;
    background: rgba(8, 11, 11, 0.48);
  }

  .embedded-terminal-panel.active {
    border-color: rgba(92, 226, 207, 0.24);
  }

  .embedded-terminal-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: #8d9995;
    font-size: 9px;
    font-weight: 760;
  }

  .embedded-terminal-toolbar span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .embedded-terminal-toolbar code {
    color: #72e2cf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 9px;
  }

  .embedded-terminal-host {
    height: 168px;
    min-height: 128px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    background: #101414;
  }

  .embedded-terminal-host :global(.xterm) {
    height: 100%;
    padding: 6px;
  }

  .embedded-terminal-host :global(.xterm-viewport) {
    background: transparent !important;
  }

  .embedded-terminal-error {
    min-width: 0;
    overflow: hidden;
    color: #d8aa55;
    font-size: 10px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .terminal-launchpad-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .terminal-launchpad-list {
    display: grid;
    align-content: start;
    gap: 4px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .terminal-launchpad-title {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: #8d9995;
    font-size: 9px;
    font-weight: 850;
    text-transform: uppercase;
  }

  .terminal-launchpad-title strong {
    color: #72e2cf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
  }

  .terminal-launchpad-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 24px;
    align-items: center;
    gap: 7px;
    min-width: 0;
    min-height: 30px;
    padding: 4px 5px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
  }

  .terminal-launchpad-row.active {
    border-color: rgba(92, 226, 207, 0.24);
    background: rgba(92, 226, 207, 0.08);
  }

  .terminal-launchpad-row > div {
    display: grid;
    gap: 1px;
    min-width: 0;
  }

  .terminal-launchpad-row strong {
    color: #eef4f2;
    font-size: 10px;
    font-weight: 820;
  }

  .terminal-launchpad-row small {
    color: #8d9995;
    font-size: 9px;
    font-weight: 720;
  }

  .terminal-launchpad-row button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 22px;
    color: #72e2cf;
    border: 1px solid rgba(92, 226, 207, 0.18);
    border-radius: 5px;
    background: rgba(92, 226, 207, 0.08);
  }

  .terminal-launchpad-row button:hover,
  .terminal-launchpad-row button:focus-visible {
    outline: 0;
    background: rgba(92, 226, 207, 0.16);
  }

  .terminal-launchpad-empty {
    display: grid;
    place-items: center;
    min-height: 30px;
    color: #798481;
    border: 1px dashed rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    font-size: 10px;
    font-weight: 760;
  }

  .browser-dock {
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr);
    flex: 0 0 auto;
    gap: 6px;
    min-width: 0;
    max-height: 428px;
    margin-top: 6px;
    overflow: hidden;
    padding: 7px;
    border: 1px solid rgba(255, 255, 255, 0.105);
    border-radius: 8px;
    background: rgba(15, 18, 18, 0.92);
  }

  .browser-dock-header,
  .browser-url-form {
    display: grid;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .browser-dock-header {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .browser-dock-title,
  .browser-dock-actions {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .browser-dock-title {
    gap: 6px;
    color: #dce4e2;
  }

  .browser-dock-title strong,
  .browser-dock-title span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .browser-dock-title strong {
    font-size: 11px;
    font-weight: 860;
  }

  .browser-dock-title span {
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
  }

  .browser-dock-actions {
    justify-content: end;
    gap: 5px;
  }

  .browser-dock-actions .file-action-button {
    display: inline-grid;
    place-items: center;
    width: 28px;
    height: 24px;
    min-width: 28px;
    padding: 0;
    border-radius: 6px;
  }

  .browser-url-form {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .browser-url-form input {
    width: 100%;
    height: 28px;
    min-width: 0;
    padding: 0 9px;
    color: #e6efec;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.045);
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    font-weight: 720;
  }

  .browser-url-form input:focus-visible {
    outline: 1px solid rgba(92, 226, 207, 0.52);
    outline-offset: 1px;
  }

  .browser-url-form .file-action-button {
    display: inline-flex;
    width: auto;
    height: 28px;
    gap: 5px;
    padding: 0 9px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 820;
  }

  .browser-runtime-list {
    display: flex;
    gap: 5px;
    min-width: 0;
    overflow-x: auto;
    padding-bottom: 2px;
    scrollbar-width: thin;
  }

  .browser-runtime-list button {
    display: inline-grid;
    grid-template-columns: auto minmax(0, auto) auto;
    align-items: center;
    flex: 0 0 auto;
    gap: 6px;
    max-width: 220px;
    height: 26px;
    min-width: 0;
    padding: 0 8px;
    color: #b7c3bf;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
  }

  .browser-runtime-list button.active {
    color: #dffdf8;
    border-color: rgba(92, 226, 207, 0.28);
    background: rgba(92, 226, 207, 0.1);
  }

  .browser-runtime-list span,
  .browser-runtime-list small {
    color: #72e2cf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 820;
  }

  .browser-runtime-list strong,
  .browser-runtime-list small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .browser-runtime-list strong {
    font-size: 10px;
    font-weight: 820;
  }

  .browser-runtime-list small {
    max-width: 82px;
    color: #8d9995;
  }

  .browser-frame-wrap {
    min-height: 280px;
    min-width: 0;
    height: 280px;
    overflow: hidden;
    border: 1px solid rgba(92, 226, 207, 0.11);
    border-radius: 7px;
    background: rgba(8, 11, 11, 0.72);
  }

  .browser-frame {
    display: block;
    width: 100%;
    height: 100%;
    min-height: 280px;
    border: 0;
    background: #101414;
  }

  .browser-empty,
  .browser-error {
    display: grid;
    place-items: center;
    min-height: 54px;
    color: #798481;
    border: 1px dashed rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    font-size: 10px;
    font-weight: 760;
  }

  .browser-error {
    min-height: 28px;
    color: #d8aa55;
  }

  .editor-toolbar {
    display: grid;
    position: relative;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 4px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.025);
  }

  .editor-file-state {
    display: inline-grid;
    grid-template-columns: 14px minmax(0, auto) auto auto;
    align-items: center;
    justify-self: start;
    gap: 4px;
    min-width: 0;
    max-width: 100%;
    height: 18px;
    padding: 0 2px;
    color: #cbd3d1;
    border: 0;
    border-radius: 5px;
    background: transparent;
  }

  .editor-file-state strong,
  .editor-file-state span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-state strong {
    font-size: 11px;
    font-weight: 760;
  }

  .editor-file-state span {
    color: #d8aa55;
    font-size: 9px;
    font-weight: 820;
    text-transform: uppercase;
  }

  .editor-file-state .editor-lsp-state {
    color: #9fa7a5;
  }

  .editor-file-state .editor-lsp-state.ready {
    color: #72e2cf;
  }

  .editor-file-state .editor-lsp-state.unavailable {
    color: #d8aa55;
  }

  .editor-menu-anchor {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .editor-icon-button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 20px;
    color: #aeb8b5;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .editor-icon-button:hover,
  .editor-icon-button:focus-visible {
    color: #f2f6f5;
    border-color: rgba(92, 226, 207, 0.4);
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .editor-icon-button.active {
    color: #dffdf8;
    border-color: rgba(92, 226, 207, 0.32);
    background: rgba(92, 226, 207, 0.12);
  }

  .editor-action-menu {
    position: absolute;
    z-index: 8;
    top: calc(100% + 3px);
    right: 0;
    display: grid;
    width: 230px;
    min-width: 0;
    padding: 5px;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 8px;
    background: rgba(22, 25, 25, 0.98);
    box-shadow: 0 18px 44px rgba(0, 0, 0, 0.34);
  }

  .editor-action-menu button {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 26px;
    padding: 0 7px;
    color: #cbd3d1;
    border: 0;
    border-radius: 5px;
    background: transparent;
    font-size: 11px;
    font-weight: 730;
    text-align: left;
    cursor: pointer;
  }

  .editor-action-menu button:hover:not(:disabled),
  .editor-action-menu button:focus-visible {
    color: #f2f6f5;
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .editor-action-menu button:disabled {
    cursor: default;
    opacity: 0.48;
  }

  .editor-action-menu span,
  .editor-action-menu kbd {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-action-menu kbd {
    color: #8d9995;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 9px;
    font-weight: 760;
  }

  .editor-action-menu hr {
    width: 100%;
    height: 1px;
    margin: 4px 0;
    border: 0;
    background: rgba(255, 255, 255, 0.08);
  }

  .editor-body-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 8px var(--editor-insight-width);
    min-height: 0;
  }

  .editor-body-grid.insights-hidden {
    grid-template-columns: minmax(0, 1fr);
  }

  .editor-canvas {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .editor-insight-resizer {
    width: 8px;
    min-width: 0;
    padding: 0;
    cursor: col-resize;
    border: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.045);
    border-right: 1px solid rgba(255, 255, 255, 0.045);
    background: rgba(255, 255, 255, 0.025);
  }

  .editor-insight-resizer:hover,
  .editor-insight-resizer:focus-visible {
    outline: 0;
    background: rgba(92, 226, 207, 0.18);
  }

  :global(body.resizing-editor-insight) {
    cursor: col-resize;
    user-select: none;
  }

  .source-intelligence-panel {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-left: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(20, 23, 24, 0.86);
  }

  .editor-lookup-popover {
    position: absolute;
    right: 12px;
    bottom: 12px;
    z-index: 5;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    width: min(520px, calc(100% - 24px));
    max-height: min(280px, calc(100% - 24px));
    min-width: 0;
    overflow: hidden;
    border: 1px solid rgba(92, 226, 207, 0.24);
    border-radius: 8px;
    background: rgba(18, 21, 21, 0.96);
    box-shadow: 0 18px 54px rgba(0, 0, 0, 0.34);
  }

  .editor-lookup-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 6px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.075);
  }

  .editor-lookup-header strong {
    min-width: 0;
    overflow: hidden;
    color: #e4efed;
    font-size: 11px;
    font-weight: 820;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-lookup-close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    color: #aab6b2;
    border: 0;
    border-radius: 5px;
    background: transparent;
    cursor: pointer;
  }

  .editor-lookup-close:hover,
  .editor-lookup-close:focus-visible {
    color: #f2f6f5;
    outline: 0;
    background: rgba(255, 255, 255, 0.08);
  }

  .editor-lookup-list {
    display: grid;
    gap: 4px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 6px;
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .intelligence-tabs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
    padding: 6px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .intelligence-tabs button {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto;
    align-items: center;
    gap: 5px;
    height: 24px;
    min-width: 0;
    padding: 0 5px;
    color: #9fa9a6;
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 9px;
    font-weight: 800;
    cursor: pointer;
  }

  .intelligence-tabs button.active {
    color: #f2f6f5;
    border-color: rgba(92, 226, 207, 0.36);
    background: rgba(92, 226, 207, 0.11);
  }

  .intelligence-tabs span,
  .intelligence-tabs strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .intelligence-tabs strong {
    color: #7ce5d5;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
  }

  .intelligence-summary {
    min-width: 0;
    padding: 9px 12px;
    overflow: hidden;
    color: #8d9995;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 11px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .intelligence-list {
    flex: 1 1 auto;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 8px;
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .intelligence-list::-webkit-scrollbar {
    width: 10px;
  }

  .intelligence-list::-webkit-scrollbar-track {
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.045);
  }

  .intelligence-list::-webkit-scrollbar-thumb {
    border: 2px solid rgba(20, 23, 24, 0.86);
    border-radius: 999px;
    background: rgba(174, 184, 181, 0.54);
  }

  .git-diff-panel {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .git-command-drawer {
    flex: 0 0 auto;
    min-width: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .git-command-drawer summary {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    color: #dce5e2;
    list-style: none;
    cursor: pointer;
    font-size: 10px;
    font-weight: 840;
  }

  .git-command-drawer summary::-webkit-details-marker {
    display: none;
  }

  .git-command-drawer summary::before {
    width: 0;
    height: 0;
    border-top: 4px solid transparent;
    border-bottom: 4px solid transparent;
    border-left: 5px solid #8d9995;
    content: "";
  }

  .git-command-drawer[open] summary::before {
    transform: rotate(90deg);
  }

  .git-command-drawer summary small {
    min-width: 0;
    overflow: hidden;
    color: #899591;
    font-size: 9px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-controls {
    display: grid;
    flex: 0 0 auto;
    gap: 5px;
    padding: 0 6px 6px;
  }

  .git-action-row,
  .git-remote-row,
  .git-commit-row {
    display: grid;
    min-width: 0;
    gap: 5px;
  }

  .git-action-row {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .git-remote-row {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .git-commit-row {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: stretch;
  }

  .git-action-button {
    display: grid;
    grid-template-columns: 13px minmax(0, auto);
    align-items: center;
    justify-content: center;
    gap: 5px;
    min-width: 0;
    min-height: 24px;
    padding: 0 6px;
    color: #cfd8d5;
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.045);
    font-size: 9px;
    font-weight: 820;
    cursor: pointer;
  }

  .git-action-button.commit {
    color: #dff8f4;
    border-color: rgba(92, 226, 207, 0.28);
    background: rgba(92, 226, 207, 0.12);
  }

  .git-action-button:disabled {
    cursor: default;
    opacity: 0.48;
  }

  .git-action-button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-commit-input {
    width: 100%;
    min-width: 0;
    min-height: 32px;
    padding: 6px 7px;
    resize: none;
    color: #e6ecea;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    outline: none;
    background: rgba(0, 0, 0, 0.22);
    font: inherit;
    font-size: 10px;
    line-height: 1.3;
  }

  .git-commit-input:focus {
    border-color: rgba(92, 226, 207, 0.42);
  }

  .git-action-message {
    min-width: 0;
    overflow: hidden;
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-action-message.error {
    color: #ff8f8f;
  }

  .git-status-list {
    display: grid;
    flex: 0 0 auto;
    gap: 5px;
    max-height: 150px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .git-status-overview {
    min-width: 0;
    overflow: hidden;
    color: #8d9995;
    font-size: 9px;
    font-weight: 780;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-status-group {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .git-status-group-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 5px;
    min-width: 0;
    min-height: 22px;
    color: #aeb8b5;
    font-size: 9px;
    font-weight: 850;
    text-transform: uppercase;
  }

  .git-status-group-heading strong,
  .git-status-group-heading span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-status-group-heading button {
    min-width: 0;
    height: 20px;
    padding: 0 6px;
    color: #cfd8d5;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.04);
    font-size: 8.5px;
    font-weight: 820;
    cursor: pointer;
    text-transform: none;
  }

  .git-status-group-heading button:disabled {
    cursor: default;
    opacity: 0.48;
  }

  .git-status-group-heading button:hover:not(:disabled),
  .git-status-group-heading button:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.11);
  }

  .git-status-row {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr) minmax(0, 76px);
    align-items: center;
    gap: 7px;
    min-width: 0;
    min-height: 30px;
    padding: 5px 7px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.035);
    text-align: left;
    cursor: pointer;
  }

  .git-status-row.selected {
    border-color: rgba(92, 226, 207, 0.32);
    background: rgba(92, 226, 207, 0.11);
  }

  .git-status-row strong,
  .git-status-row span,
  .git-status-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-status-row strong {
    color: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    font-weight: 860;
  }

  .git-status-row span {
    font-size: 10px;
    font-weight: 780;
  }

  .git-status-row small {
    color: #8d9995;
    font-size: 9px;
    font-weight: 760;
  }

  .git-history-panel {
    display: grid;
    flex: 0 0 auto;
    gap: 6px;
    min-height: 0;
    padding: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .git-history-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: #e4eae8;
    font-size: 10px;
    font-weight: 850;
  }

  .git-history-heading small {
    min-width: 0;
    overflow: hidden;
    color: #8d9995;
    font-size: 9px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-task-trail {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
    color: #8d9995;
    font-size: 8.5px;
    font-weight: 820;
  }

  .git-task-trail > span:first-child {
    flex: 0 0 auto;
    color: #aeb8b5;
    text-transform: uppercase;
  }

  .git-branch-health-strip {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(92px, 1fr));
    gap: 4px;
    min-width: 0;
  }

  .git-branch-health-chip {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 5px;
    min-width: 0;
    min-height: 22px;
    padding: 3px 6px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.032);
  }

  .git-branch-health-chip strong,
  .git-branch-health-chip span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-branch-health-chip strong {
    color: #8d9995;
    font-size: 8px;
    font-weight: 860;
    text-transform: uppercase;
  }

  .git-branch-health-chip span {
    color: #dce5e2;
    font-size: 9px;
    font-weight: 780;
  }

  .git-branch-health-chip.clean span {
    color: #72e2cf;
  }

  .git-branch-health-chip.dirty span {
    color: #d8aa55;
  }

  .git-branch-health-chip.warning span {
    color: #9fd0f0;
  }

  .git-branch-health-chip.error span {
    color: #ff8d8d;
  }

  .git-branch-health-chip.muted span {
    color: #9fa9a6;
  }

  .git-task-source-map {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .git-task-source-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 24px;
    padding: 3px 5px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.028);
  }

  .git-task-source-row small {
    min-width: 0;
    overflow: hidden;
    color: #9aa7a3;
    font-size: 8.5px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-task-source-row button {
    display: grid;
    place-items: center;
    width: 19px;
    height: 19px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .git-task-source-row button:hover,
  .git-task-source-row button:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.11);
  }

  .git-commit-detail {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
    padding: 6px 7px;
    border: 1px solid rgba(111, 223, 207, 0.18);
    border-radius: 7px;
    background: rgba(111, 223, 207, 0.055);
  }

  .git-commit-detail-main {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .git-commit-detail-main strong,
  .git-commit-detail-main span,
  .git-commit-detail-main small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-commit-detail-main strong {
    color: #79eadb;
    font-size: 9px;
    font-weight: 860;
  }

  .git-commit-detail-main span {
    color: #f1f5f4;
    font-size: 10px;
    font-weight: 820;
  }

  .git-commit-detail-main small {
    color: #8d9995;
    font-size: 8.5px;
    font-weight: 760;
  }

  .git-commit-detail-actions {
    display: inline-flex;
    gap: 3px;
    min-width: 0;
  }

  .git-history-list {
    display: grid;
    gap: 5px;
    max-height: 170px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .git-history-row {
    position: relative;
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
    min-height: 38px;
    padding: 6px 7px;
    color: #cfd8d5;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.03);
    cursor: pointer;
  }

  .git-history-row.selected {
    border-color: rgba(111, 223, 207, 0.42);
    background: rgba(111, 223, 207, 0.095);
  }

  .git-history-row:focus-visible {
    outline: 1px solid rgba(111, 223, 207, 0.42);
    outline-offset: 2px;
  }

  .git-history-row.head {
    border-color: rgba(111, 223, 207, 0.3);
    background: rgba(111, 223, 207, 0.07);
  }

  .git-history-row.branch {
    border-color: rgba(132, 201, 222, 0.2);
  }

  .git-history-row.merge {
    border-color: rgba(216, 170, 85, 0.24);
  }

  .git-history-row.root {
    border-color: rgba(174, 184, 181, 0.18);
  }

  .git-graph-marker {
    position: relative;
    display: grid;
    place-items: center;
    width: 18px;
    height: 24px;
  }

  .git-graph-marker::before {
    position: absolute;
    inset: -8px auto;
    width: 1px;
    background: rgba(111, 223, 207, 0.22);
    content: "";
  }

  .git-graph-marker::after {
    z-index: 1;
    width: 8px;
    height: 8px;
    border: 2px solid rgba(111, 223, 207, 0.72);
    border-radius: 999px;
    background: #171b1b;
    content: "";
  }

  .git-graph-marker.head::before {
    width: 2px;
    background: rgba(111, 223, 207, 0.44);
  }

  .git-graph-marker.head::after {
    width: 10px;
    height: 10px;
    border-color: #6fdfcf;
    background: #6fdfcf;
    box-shadow: 0 0 0 3px rgba(111, 223, 207, 0.14);
  }

  .git-graph-marker.branch::after {
    border-color: rgba(132, 201, 222, 0.84);
    background: #171b1b;
  }

  .git-graph-marker.merge::before {
    background: linear-gradient(
      180deg,
      rgba(216, 170, 85, 0.15),
      rgba(216, 170, 85, 0.5),
      rgba(111, 223, 207, 0.2)
    );
  }

  .git-graph-marker.merge::after {
    width: 10px;
    height: 10px;
    border-color: rgba(216, 170, 85, 0.9);
    border-radius: 3px;
    background: #171b1b;
  }

  .git-graph-marker.root::before {
    inset: -8px auto 50%;
    background: rgba(174, 184, 181, 0.2);
  }

  .git-graph-marker.root::after {
    border-color: rgba(174, 184, 181, 0.75);
    background: rgba(174, 184, 181, 0.2);
  }

  .git-history-main,
  .git-history-meta {
    min-width: 0;
  }

  .git-history-main {
    display: grid;
    gap: 3px;
  }

  .git-history-main strong,
  .git-history-main small,
  .git-ref-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-history-main strong {
    color: #f1f5f4;
    font-size: 10px;
    font-weight: 820;
  }

  .git-history-main small {
    color: #8d9995;
    font-size: 9px;
    font-weight: 740;
  }

  .git-history-meta {
    display: grid;
    justify-items: end;
    gap: 4px;
    max-width: 112px;
  }

  .git-history-actions {
    display: inline-flex;
    justify-content: flex-end;
    gap: 3px;
    max-width: 112px;
  }

  .git-commit-detail-actions {
    justify-content: flex-end;
    max-width: 112px;
  }

  .git-history-actions button,
  .git-commit-detail-actions button {
    display: grid;
    place-items: center;
    width: 21px;
    height: 21px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .git-history-actions button:hover,
  .git-history-actions button:focus-visible,
  .git-commit-detail-actions button:hover,
  .git-commit-detail-actions button:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.11);
  }

  .git-ref-label {
    max-width: 112px;
    color: #aeb8b5;
    font-size: 8.5px;
    font-weight: 760;
  }

  .git-task-link {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    min-height: 19px;
    padding: 0 6px;
    color: #071b18;
    border-radius: 999px;
    background: #6fdfcf;
    font-size: 8.5px;
    font-weight: 900;
    line-height: 1;
    text-decoration: none;
    white-space: nowrap;
  }

  .git-diff-block {
    flex: 1 1 auto;
    min-height: 0;
    margin: 8px;
    padding: 10px;
    overflow: auto;
    color: #cfd8d5;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.24);
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10.5px;
    line-height: 1.45;
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
    white-space: pre;
  }

  .intelligence-empty {
    display: grid;
    place-items: center;
    min-height: 126px;
    color: #75817d;
    font-size: 12px;
    font-weight: 750;
  }

  .definition-results,
  .implementation-results,
  .type-definition-results,
  .reference-results {
    flex: 0 0 auto;
    max-height: 164px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .definition-results {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .reference-results {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .implementation-results {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .type-definition-results {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .definition-summary,
  .implementation-summary,
  .type-definition-summary,
  .reference-summary {
    margin-bottom: 6px;
    min-width: 0;
    overflow: hidden;
    color: #8d9995;
    font-size: 10px;
    font-weight: 780;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .intelligence-row,
  .definition-row,
  .reference-row {
    display: grid;
    grid-template-columns: minmax(68px, auto) minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    width: 100%;
    min-height: 31px;
    padding: 5px 7px;
    color: #cbd3d1;
    text-align: left;
    border: 0;
    border-radius: 7px;
    background: transparent;
    cursor: pointer;
  }

  .intelligence-row:hover,
  .intelligence-row:focus-visible,
  .definition-row:hover,
  .definition-row:focus-visible,
  .reference-row:hover,
  .reference-row:focus-visible {
    color: #f2f6f5;
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .intelligence-row strong,
  .intelligence-row span,
  .intelligence-row small,
  .definition-row strong,
  .definition-row span,
  .definition-row small,
  .reference-row strong,
  .reference-row span,
  .reference-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .intelligence-row strong,
  .definition-row strong,
  .reference-row strong {
    color: #8fd8cf;
    font-size: 10px;
    font-weight: 850;
    text-transform: uppercase;
  }

  .intelligence-row span,
  .definition-row span,
  .reference-row span {
    font-size: 12px;
    font-weight: 760;
  }

  .intelligence-row small,
  .definition-row small,
  .reference-row small {
    color: #7f8b87;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 750;
  }

  .intelligence-row.diagnostic.error strong {
    color: #f1a9a0;
  }

  .intelligence-row.diagnostic.warning strong {
    color: #d8aa55;
  }

  .quick-open-layer,
  .command-palette-layer {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: start center;
    padding: 72px 16px 16px;
  }

  .quick-open-backdrop,
  .command-palette-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: rgba(7, 9, 10, 0.56);
    backdrop-filter: blur(10px);
    cursor: default;
  }

  .quick-open-panel,
  .command-palette-panel {
    position: relative;
    z-index: 1;
    width: min(720px, calc(100vw - 32px));
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 12px;
    background: rgba(22, 24, 24, 0.98);
    box-shadow: 0 28px 80px rgba(0, 0, 0, 0.44);
  }

  .quick-open-search,
  .command-palette-search {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    height: 50px;
    padding: 0 14px;
    color: #9aa5a1;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.045);
  }

  .command-palette-search {
    grid-template-columns: 22px minmax(0, 1fr) auto;
    height: 46px;
  }

  .quick-open-icon,
  .quick-open-result-icon,
  .command-palette-icon,
  .command-palette-result-icon {
    display: grid;
    place-items: center;
    min-width: 0;
  }

  .quick-open-icon,
  .command-palette-icon {
    color: #6fdfcf;
  }

  .quick-open-search input,
  .command-palette-search input {
    height: 100%;
    font-size: 15px;
    font-weight: 700;
  }

  .command-palette-search kbd {
    color: #7f8b87;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 800;
  }

  .quick-open-results,
  .command-palette-results {
    display: grid;
    gap: 3px;
    max-height: 368px;
    padding: 7px;
    overflow: auto;
  }

  .quick-open-results button,
  .command-palette-results button {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    height: 44px;
    padding: 0 9px;
    color: #cbd3d1;
    text-align: left;
    border: 0;
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
  }

  .command-palette-results button {
    grid-template-columns: 22px minmax(0, 1fr);
    height: 38px;
  }

  .quick-open-results button:hover,
  .quick-open-results button.active,
  .command-palette-results button:hover:not(:disabled),
  .command-palette-results button.active {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.12);
  }

  .command-palette-results button:disabled {
    cursor: default;
    opacity: 0.44;
  }

  .quick-open-result-icon,
  .command-palette-result-icon {
    color: #8d9995;
  }

  .quick-open-results button.active .quick-open-result-icon,
  .command-palette-results button.active .command-palette-result-icon {
    color: #6fdfcf;
  }

  .quick-open-results button span,
  .command-palette-results button span {
    display: grid;
    min-width: 0;
  }

  .quick-open-results strong,
  .quick-open-results small,
  .command-palette-results strong,
  .command-palette-results small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-open-results strong,
  .command-palette-results strong {
    font-size: 13px;
    line-height: 1.15;
  }

  .command-palette-results strong {
    font-size: 12px;
  }

  .quick-open-results small,
  .quick-open-results em,
  .command-palette-results small {
    color: #7f8b87;
    font-size: 10px;
    font-style: normal;
    font-weight: 760;
  }

  .quick-open-empty {
    display: grid;
    place-items: center;
    min-height: 112px;
    color: #9aa5a1;
    font-size: 13px;
    font-weight: 700;
  }

  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -80% 0;
    }
  }

  @media (max-width: 980px) {
    :global(body) {
      min-width: 0;
      overflow: auto;
    }

    .shell {
      grid-template-columns: 1fr;
      width: calc(100vw - 20px);
      height: auto;
      min-height: calc(100dvh - 20px);
      margin: 10px;
    }

    .shell.side-right {
      grid-template-columns: 1fr;
    }

    .activity-shell {
      grid-template-columns: 48px minmax(0, 1fr);
      min-height: 620px;
      border-right: 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }

    .shell.side-right .activity-shell {
      grid-template-columns: 48px minmax(0, 1fr);
      grid-column: auto;
      grid-row: auto;
      border-left: 0;
    }

    .shell.side-right .workspace,
    .shell.side-right .side-pane-resizer,
    .shell.side-right .activity-rail,
    .shell.side-right .sidebar {
      grid-column: auto;
      grid-row: auto;
    }

    .shell.side-right .activity-rail {
      border-right: 1px solid rgba(255, 255, 255, 0.07);
      border-left: 0;
    }

    .activity-rail {
      width: 48px;
      padding: 12px 4px;
    }

    .side-pane-resizer,
    .context-pane-resizer {
      display: none;
    }

    .sidebar {
      padding: 18px 14px;
    }

    .topbar {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .topbar-tools {
      justify-content: flex-start;
    }

    .context-identity-strip {
      flex-wrap: wrap;
      row-gap: 0;
    }

    .context-identity-item {
      max-width: calc(50% - 5px);
    }

    .workspace-arrangement.context-side {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr);
    }

    .workspace-arrangement.context-side .workspace-context-column,
    .workspace-arrangement.context-side .workspace-main-column {
      grid-column: auto;
      grid-row: auto;
    }

    .workspace-arrangement.context-side .workspace-context-column {
      padding-left: 0;
    }

    .workspace-arrangement.context-side .context-pane-resizer {
      display: none;
    }

    .workspace-arrangement.context-side .context-panel-grid {
      max-height: 280px;
    }

    .runtime-context-row {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .worktree-context-row {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .repo-dashboard-row {
      grid-template-columns: minmax(0, 1fr);
    }

    .editor-frame {
      height: 520px;
    }

    .terminal-launchpad-grid {
      grid-template-columns: 1fr;
      overflow-y: auto;
    }

    .terminal-launchpad-header {
      grid-template-columns: 1fr;
      align-items: stretch;
    }

    .terminal-launchpad-actions {
      justify-content: start;
      overflow-x: auto;
    }

    .editor-body-grid {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(0, 1fr) 148px;
    }

    .editor-insight-resizer {
      display: none;
    }

    .source-intelligence-panel {
      border-left: 0;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }
  }
</style>
