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
  import { onMount } from 'svelte';
  import MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import {
    cleanupPasteText,
    formatPasteCleanupStats,
    pasteCleanupModes,
    type PasteCleanupMode
  } from '$lib/pasteCleanup';
  import {
    orchestrationArtifactChips,
    orchestrationCurrentActivity,
    orchestrationLinkChips,
    orchestrationLoopTallyText,
    orchestrationRunMetrics,
    orchestrationRunSummaryText,
    orchestrationStatusTone,
    orchestrationTimelineItems,
    type OrchestrationTimelineItem
  } from '$lib/orchestrationView';
  import { sourcePreviewAppearance, sourcePreviewAppearanceKey } from '$lib/sourcePreviewAppearance';
  import { buildWorktreeSafetySummary } from '$lib/worktreeSafety';
  import {
    createDefaultSourceDockLayout,
    hideSourceDockPanel,
    moveSourceDockPanel,
    normalizeSourceDockLayout,
    showSourceDockPanel,
    type SourceDockGroupID,
    type SourceDockLayout,
    type SourceDockPanelID
  } from '$lib/sourceDockLayout';
  import {
    buildSourceTree,
    closeOpenSourceTab,
    createProjectRoot,
    defaultProjectRoots,
    demoPreviewFor,
    demoRecordsForProject,
    filterSourceRecords,
    findSourceDefinitionTargets,
    findSourceReferenceTargets,
    findSourceSearchMatches,
    gitCommitGraphKind,
    gitRefLabels,
    formatSourceContextGitSummary,
    formatSourceContextIdentity,
    formatSourceContextRootLabel,
    formatSourceDiagnosticSummary,
    formatSourceIndexSummary,
    flattenSourceTree,
    formatSourceRecordCount,
    formatSourceScanSummary,
    folderIdsForSourceRecord,
    getSourceScanCacheEntry,
    mergeProjectRoots,
    normalizeProjectPath,
    parseQuickOpenQuery,
    previewFromContent,
    rankSourceRecords,
    removeSourceScanCacheEntries,
    scrollTopForSourceTreeReveal,
    selectBackgroundIndexProjects,
    selectPreferredSourceRecord,
    sourceSupportsLanguageIntelligence,
    textMatchesSearchTokens,
    uniqueTaskIDsFromGitMetadata,
    upsertSourceScanCacheEntry,
    upsertOpenSourceTab,
    upsertRecentSourceRecord,
    virtualizeSourceTreeRows,
    type ProjectRoot,
    type SourceScanCache,
    type SourceOpenTab,
    type SourcePreview,
    type SourceRecentRecord,
    type SourceRecord,
    type SourceDefinitionTarget,
    type SourceReferenceTarget,
    type SourceSearchMatch,
    type SourceDiagnostic,
    type SourceLspHover,
    type SourceLspStatus,
    type SourceSymbol,
    type SourceTreeNode,
    type SourceTreeRow
  } from '$lib/sourceData';
  import {
    cancelSourceScanFromTauri,
    commitGitRepositoryFromTauri,
    createSourceScanId,
    defaultSourceScanLimit,
    expandedSourceScanLimit,
    fetchGitRepositoryFromTauri,
    findSourceDefinitionsFromTauri,
    findSourceLspDefinitionsFromTauri,
    findSourceLspHoverFromTauri,
    findSourceLspReferencesFromTauri,
    findSourceLspSymbolsFromTauri,
    findSourceReferencesFromTauri,
    listAgentSessionsFromTauri,
    listGitRepositorySummariesFromTauri,
    listOrchestrationRunsFromTauri,
    listProjectWorktreesFromTauri,
    listRuntimeContextsFromTauri,
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
    revealPathFromTauri,
    revealSourceFileFromTauri,
    pullGitRepositoryFromTauri,
    pushGitRepositoryFromTauri,
    readGitCommitHistoryFromTauri,
    searchSourceFilesFromTauri,
    stageGitPathsFromTauri,
    unstageGitPathsFromTauri,
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
    type SourceGitDiff
  } from '$lib/tauriSource';

  const customProjectRootsStorageKey = 'mac-command-bar.source-browser.custom-project-roots';
  const selectedProjectStorageKey = 'mac-command-bar.source-browser.selected-project';
  const selectedSourcePathStorageKey = 'mac-command-bar.source-browser.selected-source-paths';
  const recentSourceRecordsStorageKey = 'mac-command-bar.source-browser.recent-source-records';
  const openSourceTabsStorageKey = 'mac-command-bar.source-browser.open-source-tabs';
  const sourceActivityModeStorageKey = 'mac-command-bar.source-browser.activity-mode';
  const sourceLayoutPresetStorageKey = 'mac-command-bar.source-browser.layout-preset';
  const sourceLayoutVersionStorageKey = 'mac-command-bar.source-browser.layout-version';
  const sourceTerminalAppStorageKey = 'mac-command-bar.source-browser.terminal-app';
  const sourceDockLayoutStorageKey = 'mac-command-bar.source-browser.dock-layout';
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
  const maxRecentSourceRecords = 24;
  const maxProjectRecentRecords = 5;
  const maxProjectOpenSourceTabs = 8;
  const maxStoredOpenSourceTabs = 64;
  const maxSourceSearchResults = 50;
  const maxSourceDefinitionResults = 20;
  const maxGitCommitHistoryEntries = 24;
  const commandCenterTaskUrls: Record<string, string> = {
    'TSK-127':
      'https://app.notion.com/p/TSK-127-Create-a-native-MAC-OS-app-for-doing-diff-things-in-menu-bar-379394b0689d8053af76fd44c7ffdba4'
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
  const initialRecords = demoRecordsForProject(initialProject);
  const initialPreview = initialRecords[0] ? demoPreviewFor(initialRecords[0]) : null;

  type SourceIntelligenceAction = 'definition' | 'hover' | 'references';
  type SourceEditorIntelligenceCommand = {
    id: number;
    action: SourceIntelligenceAction;
  };
  type SourceEditorLookupRequest = {
    symbolName: string;
    line: number;
    column: number;
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
  let records = $state<SourceRecord[]>(initialRecords);
  let selectedRecord = $state<SourceRecord | null>(initialRecords[0] ?? null);
  let selectedSourceLine = $state<number | null>(null);
  let selectedSourceLineRequestId = $state(0);
  let preview = $state<SourcePreview | null>(initialPreview);
  let sourceDraftContentByPath = $state<Record<string, string>>(
    initialPreview ? { [initialPreview.path]: initialPreview.content } : {}
  );
  let savedSourceContentByPath = $state<Record<string, string>>(
    initialPreview ? { [initialPreview.path]: initialPreview.content } : {}
  );
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
  let sourceActivityMode = $state<SourceActivityMode>('files');
  let sourceActivityFilter = $state('');
  let pasteCleanupInput = $state('');
  let pasteCleanupMode = $state<PasteCleanupMode>('plain');
  let sourceLayoutPreset = $state<SourceLayoutPresetID>('code');
  let sourceTerminalApp = $state<SourceTerminalApp>('Warp');
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
  let hiddenContextCardIDs = $state<Set<SourceContextCardID>>(new Set());
  let activeContextCardID = $state<SourceContextCardID>('orchestration');
  let viewMenuOpen = $state(false);
  let editorActionMenuOpen = $state(false);
  let query = $state('');
  let expandedFolderIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let scanning = $state(false);
  let activeSourceScanId = $state('');
  let sourceScanProgress = $state<NativeSourceScanProgress | null>(null);
  let scanLimitReached = $state(false);
  let runtime = $state('browser preview');
  let error = $state('');
  let fileActionStatus = $state('');
  let fileActionBusy = $state('');
  let addingProject = $state(false);
  let choosingProjectRoot = $state(false);
  let quickOpenVisible = $state(false);
  let quickOpenQuery = $state('');
  let quickOpenIndex = $state(0);
  let quickOpenInput = $state<HTMLInputElement | null>(null);
  let commandPaletteVisible = $state(false);
  let commandPaletteQuery = $state('');
  let commandPaletteIndex = $state(0);
  let commandPaletteInput = $state<HTMLInputElement | null>(null);
  let fileTreeElement = $state<HTMLDivElement | null>(null);
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

  type SourceScanOptions = {
    force?: boolean;
    limit?: number;
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
  let quickOpenResults = $derived(rankSourceRecords(records, quickOpenQuery, 12));
  let selectedIndex = $derived(
    selectedRecord ? records.findIndex((record) => record.path === selectedRecord?.path) + 1 : 0
  );
  let selectedSourceDraftContent = $derived(
    preview ? sourceDraftContentByPath[preview.path] ?? preview.content : ''
  );
  let selectedSourceDirty = $derived(preview ? isSourcePathDirty(preview.path) : false);
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
      !scanLimitReached &&
      query.trim().length === 0 &&
      records.length > 0 &&
      records.length <= suspiciousSourceIndexFileThreshold
  );
  let sourceScanHealthNote = $derived(formatSourceScanHealthNote(records.length, sourceScanNeedsAttention));
  let selectedProjectIndexEntry = $derived(
    getSourceScanCacheEntry(
      sourceScanCache,
      selectedProject,
      defaultSourceScanLimit,
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
  let gitStatusByRelativePath = $derived(
    new Map((projectGitStatus?.files ?? []).map((fileStatus) => [fileStatus.relativePath, fileStatus]))
  );
  let selectedProjectGitChangedFiles = $derived(projectGitStatus?.files ?? []);
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
  let selectedProjectAgentSessions = $derived(
    agentSessions.filter((session) => agentSessionMatchesProject(session, selectedProject))
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
        session.projectPath,
        session.lastActivity,
        agentSessionResumeCommand(session)
      )
    )
  );
  let filteredProjectWorktrees = $derived(
    projectWorktrees.filter((worktree) => {
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
  let selectedProjectGitTaskIDs = $derived(
    uniqueTaskIDsFromGitMetadata(
      selectedProjectRepositorySummaries,
      projectWorktrees,
      gitCommitHistory
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
  let sourceActivityPanelLabel = $derived(sourceActivityLabel(sourceActivityMode));
  let sourceCommandPaletteItems = $derived<SourceCommandPaletteItem[]>([
    {
      id: 'quick-open',
      label: 'Open file',
      detail: 'Cmd+P',
      perform: openQuickOpen
    },
    {
      id: 'go-to-line',
      label: 'Go to line',
      detail: selectedRecord ? `${selectedRecord.relativePath}:line` : 'No file',
      disabled: !selectedRecord,
      perform: openCurrentFileGoToLine
    },
    {
      id: 'scan-project',
      label: 'Scan current project',
      detail: selectedProject.name,
      disabled: scanning,
      perform: () => scanProject(selectedProject, selectedRecord?.path, { force: true })
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
      id: 'save-file',
      label: 'Save file',
      detail: 'Cmd+S',
      disabled: !selectedSourceDirty || fileActionBusy === 'save',
      perform: saveSelectedSourceFile
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
    {
      id: 'activity-refresh',
      label: `Refresh ${sourceActivityPanelLabel}`,
      detail: sourceActivitySummary(sourceActivityMode),
      disabled: sourceActivityRefreshing(sourceActivityMode),
      perform: () => refreshSourceActivityMode(sourceActivityMode)
    },
    ...projectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-cleanup-plan-${worktree.path}`,
      label: `Copy worktree cleanup plan: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).recommendation,
      disabled: false,
      perform: () => copyWorktreeCleanupPlan(worktree)
    })),
    ...projectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-audit-command-${worktree.path}`,
      label: `Copy worktree audit command: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).reason,
      disabled: false,
      perform: () => copyWorktreeAuditCommand(worktree)
    })),
    ...projectWorktrees.slice(0, 8).map((worktree) => ({
      id: `worktree-remove-command-${worktree.path}`,
      label: `Copy worktree remove command: ${worktree.branch}`,
      detail: projectWorktreeSafety(worktree).cleanupCommand,
      disabled: projectWorktreeSafety(worktree).kind === 'protected',
      perform: () => copyWorktreeCleanupCommand(worktree)
    })),
    ...projectWorktrees.slice(0, 8).map((worktree) => ({
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
      disabled: !agentSessionResumeCommand(session).trim(),
      perform: () => openAgentSessionTerminal(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-copy-plan-${session.provider}-${session.id}`,
      label: `Copy session resume plan: ${session.title}`,
      detail: agentSessionProjectLabel(session),
      disabled: !agentSessionResumeCommand(session).trim(),
      perform: () => copyAgentSessionResumePlan(session)
    })),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-copy-shell-command-${session.provider}-${session.id}`,
      label: `Copy shell resume command: ${session.title}`,
      detail: agentSessionResumeShellCommand(session),
      disabled: !agentSessionResumeCommand(session).trim(),
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
    const lastResultIndex = Math.max(0, quickOpenResults.length - 1);
    if (quickOpenIndex > lastResultIndex) {
      quickOpenIndex = lastResultIndex;
    }
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

  async function scanProject(
    project: ProjectRoot,
    preferredPath = selectedSourcePaths[project.id] ?? selectedRecord?.path ?? null,
    options: SourceScanOptions = {}
  ) {
    const generation = ++scanGeneration;
    const scanLimit = options.limit ?? defaultSourceScanLimit;
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
        cachedScan.truncated
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
    error = '';
    runtime = 'scanning source files';
    clearSourceRecordsForIncomingProject(project);

    try {
      const tauriScan = await listSourceFilesFromTauri(project.path, '', scanLimit, scanId);
      if (generation !== scanGeneration) return;
      if (!tauriScan) {
        throw new Error('Local source scanner unavailable. Run inside Tauri or use pnpm dev.');
      }

      const nextRecords = tauriScan.records;
      sourceScanCache = upsertSourceScanCacheEntry(
        sourceScanCache,
        project,
        nextRecords,
        tauriScan.limit,
        Date.now(),
        maxSourceScanCacheEntries,
        tauriScan.truncated
      );
      clearBackgroundIndexError(project.id);

      const nextSelection = applySourceRecords(
        nextRecords,
        preferredPath,
        'local source scan',
        tauriScan.truncated
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
    runtime = 'source scan stopped';
    error = '';
    fileActionStatus = 'Scan stopped';
  }

  function clearSourceRecordsForIncomingProject(project: ProjectRoot) {
    const hasDifferentSelectedRecord =
      selectedRecord !== null && !sourceRecordBelongsToProject(selectedRecord, project);
    const hasDifferentRecords =
      selectedRecord === null &&
      records.length > 0 &&
      records.some((record) => !sourceRecordBelongsToProject(record, project));

    if (!hasDifferentSelectedRecord && !hasDifferentRecords) return;

    records = [];
    selectedRecord = null;
    selectedSourceLine = null;
    scanLimitReached = false;
    preview = null;
    expandedFolderIds = new Set();
    syncSourcePreviewContent(null);
  }

  function sourceRecordBelongsToProject(record: SourceRecord, project: ProjectRoot) {
    const projectPath = normalizeProjectPath(project.path);
    const recordPath = normalizeProjectPath(record.path);
    return recordPath === projectPath || recordPath.startsWith(`${projectPath}/`);
  }

  function resetProjectScanCache(project: ProjectRoot = selectedProject, limit = expandedSourceScanLimit) {
    sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
    fileActionStatus = `Index reset for ${project.name}`;
    return scanProject(project, selectedSourcePaths[project.id], { force: true, limit });
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
      defaultSourceScanLimit
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
        defaultSourceScanLimit,
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
        tauriScan.truncated
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
        gitCommitHistory = nativeHistory;
        gitCommitHistorySource = 'native git log';
        return;
      }

      gitCommitHistory = demoGitCommitHistoryForProject(project);
      gitCommitHistorySource = 'browser preview';
    } catch (historyError) {
      if (selectedProjectID !== projectID) return;

      gitCommitHistory = demoGitCommitHistoryForProject(project);
      gitCommitHistorySource = 'browser preview';
      gitCommitHistoryError =
        historyError instanceof Error ? historyError.message : 'Could not read Git history';
    } finally {
      if (selectedProjectID === projectID) {
        gitCommitHistoryLoading = false;
      }
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
        taskID: 'TSK-127'
      },
      {
        shortSha: 'ebca14e',
        sha: 'ebca14e7594f84f6be0d18f2f0d39c48a2d0002',
        subject: 'feat: add TSK-127 git action controls',
        author: 'MacCommandBar',
        committedAt,
        refs: repoSlug ? `origin/${repoSlug}` : 'origin/main',
        taskID: 'TSK-127'
      },
      {
        shortSha: '27a95ed',
        sha: '27a95ed5b28f23d345ccac9a8c6e63f99ad0003',
        subject: 'feat: add TSK-127 repo dashboard',
        author: 'MacCommandBar',
        committedAt,
        refs: '',
        taskID: 'TSK-127'
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
    return [entry.sha, entry.refs, entry.subject].filter(Boolean).join('\n');
  }

  function gitCommitSummaryText(entry: GitCommitHistoryEntry) {
    const refs = gitCommitRefChips(entry).join(', ');
    const task = entry.taskID ? `Task ${entry.taskID}` : '';

    return [
      entry.shortSha,
      entry.subject,
      refs,
      task,
      entry.author,
      formatGitCommitTime(entry.committedAt)
    ].filter(Boolean).join(' · ');
  }

  function gitCommitRefChips(entry: GitCommitHistoryEntry) {
    return gitRefLabels(entry.refs);
  }

  function gitCommitGraphClass(entry: GitCommitHistoryEntry, index: number) {
    return gitCommitGraphKind(entry.refs, index);
  }

  function gitTaskUrl(taskID: string | null) {
    return taskID ? commandCenterTaskUrls[taskID] ?? null : null;
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

  async function copyGitTaskReference(taskID: string | null) {
    if (!taskID) return;

    await copyActivityCommand(gitTaskReferenceText(taskID), gitTaskUrl(taskID) ? 'Task link copied' : 'Task ID copied');
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

  function agentSessionResumeShellCommand(session: AgentSession) {
    const command = agentSessionResumeCommand(session);
    const path = agentSessionProjectPath(session);
    return path.trim() ? `cd ${shellQuoteForCommand(path)} && ${command}` : command;
  }

  function agentSessionResumePlan(session: AgentSession) {
    const commands = agentSessionResumeCommandList(session);
    const alternateCommands = commands.slice(1);

    return [
      `Session: ${session.title}`,
      `Provider: ${session.provider}`,
      `ID: ${session.id}`,
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

  function projectWorktreeSafety(worktree: ProjectWorktree) {
    return buildWorktreeSafetySummary(worktree, { primaryPath: selectedProject.path });
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

  function copyAgentSessionResumePlan(session: AgentSession) {
    return copyActivityCommand(agentSessionResumePlan(session), 'Session resume plan copied');
  }

  function copyAgentSessionResumeShellCommand(session: AgentSession) {
    return copyActivityCommand(agentSessionResumeShellCommand(session), 'Shell resume command copied');
  }

  function terminalDockSummary() {
    const parts = [
      sourceTerminalApp,
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
      return;
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
        return;
      }

      fileActionStatus = `Opened ${nextTargets[0].symbolName}`;
      await selectSourceDefinitionTarget(nextTargets[0]);
    } catch (definitionError) {
      sourceDefinitionTargets = findSourceDefinitionTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceDefinitionResults
      );
      sourceDefinitionError =
        definitionError instanceof Error ? definitionError.message : 'Could not find definition';

      if (sourceDefinitionTargets[0]) {
        await selectSourceDefinitionTarget(sourceDefinitionTargets[0]);
      }
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
      return;
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
    } catch (referenceError) {
      sourceReferenceTargets = findSourceReferenceTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceSearchResults
      );
      sourceReferenceError =
        referenceError instanceof Error ? referenceError.message : 'Could not find references';
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
    truncated = false
  ): SourceRecord | null {
    records = nextRecords;
    runtime = nextRuntime;
    scanLimitReached = truncated;

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

  async function selectRecord(record: SourceRecord, targetLine: number | null = null) {
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

  function closeQuickOpen() {
    quickOpenVisible = false;
    quickOpenQuery = '';
    quickOpenIndex = 0;
  }

  async function chooseQuickOpenRecord(record: SourceRecord) {
    await selectRecord(record, parsedQuickOpenQuery.targetLine);
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
      quickOpenIndex = Math.min(quickOpenIndex + 1, Math.max(0, quickOpenResults.length - 1));
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      quickOpenIndex = Math.max(quickOpenIndex - 1, 0);
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      const selectedQuickOpenRecord = quickOpenResults[quickOpenIndex];
      if (selectedQuickOpenRecord) {
        void chooseQuickOpenRecord(selectedQuickOpenRecord);
      }
    }
  }

  async function closeSourceTab(tab: SourceOpenTab, event: MouseEvent) {
    event.stopPropagation();

    const closeResult = closeOpenSourceTab(projectOpenSourceTabs, tab.path, selectedRecord?.path);
    const nextOpenSourceTabs = replaceProjectOpenTabs(openSourceTabs, selectedProject.id, closeResult.tabs);
    openSourceTabs = nextOpenSourceTabs;
    persistOpenSourceTabs(nextOpenSourceTabs);

    if (closeResult.nextActivePath === selectedRecord?.path) return;

    if (closeResult.nextActivePath) {
      const nextTab = closeResult.tabs.find((openTab) => openTab.path === closeResult.nextActivePath);
      if (nextTab) {
        await selectOpenTab(nextTab);
      }
      return;
    }

    clearSelectedSourceRecordForProject(selectedProject.id);
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

  async function openAgentSessionTerminal(session: AgentSession) {
    const command = agentSessionResumeCommand(session);
    if (!command.trim()) return;

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
  }

  function isSourcePathDirty(path: string) {
    const draftContent = sourceDraftContentByPath[path];
    const savedContent = savedSourceContentByPath[path];
    return draftContent !== undefined && savedContent !== undefined && draftContent !== savedContent;
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

  function handleEditorSymbolsChange(symbols: SourceSymbol[]) {
    sourceSymbols = symbols;
  }

  function handleEditorDefinitionLookup(request: SourceEditorLookupRequest) {
    void runSourceDefinitionLookup(request);
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

  function handleEditorReferenceLookup(request: SourceEditorLookupRequest) {
    void runSourceReferenceLookup(request);
  }

  function requestSourceIntelligenceAction(action: SourceIntelligenceAction) {
    if (!preview || loading) return;
    if (action === 'hover' && !sourceIntelligenceAvailable) return;

    sourceIntelligenceCommand = {
      id: ++sourceIntelligenceCommandId,
      action
    };
  }

  function selectSourceDiagnostic(diagnostic: SourceDiagnostic) {
    revealSourceLine(diagnostic.line);
  }

  function selectSourceSymbol(symbol: SourceSymbol) {
    revealSourceLine(symbol.line);
  }

  function revealSourceLine(line: number) {
    if (!preview) return;

    selectedSourceLine = Math.max(1, Math.floor(line));
    selectedSourceLineRequestId += 1;
  }

  async function handleProjectChange() {
    const nextProject =
      projectOptions.find((project) => project.id === selectedProjectID) ?? projectOptions[0] ?? initialProject;
    await activateProject(nextProject, { projects: projectOptions });
  }

  function selectSourceActivityMode(mode: SourceActivityMode) {
    markSourceLayoutCustom();
    sourceActivityMode = mode;
    persistSourceActivityMode(mode);
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function applySourceLayoutPreset(presetID: SourceLayoutPresetDefinition['id']) {
    const preset = sourceLayoutPresets.find((candidate) => candidate.id === presetID);
    if (!preset) return;

    sourceLayoutPreset = preset.id;
    sourceActivityMode = preset.activityMode;
    sidePaneWidth = clampSidePaneWidth(preset.sidePaneWidth);
    sidePanePosition = preset.sidePanePosition;
    editorInsightWidth = clampEditorInsightWidth(preset.editorInsightWidth);
    editorInsightCollapsed = preset.editorInsightCollapsed;
    contextPanelCollapsed = preset.contextPanelCollapsed;
    contextPanelMode = preset.contextPanelMode;
    contextPanelPlacement = preset.contextPanelPlacement;
    sourceIntelligencePanel = preset.intelligencePanel;

    persistSourceLayoutPreset(sourceLayoutPreset);
    persistSourceActivityMode(sourceActivityMode);
    persistSidePaneWidth(sidePaneWidth);
    persistSidePanePosition(sidePanePosition);
    persistEditorInsightWidth(editorInsightWidth);
    persistEditorInsightCollapsed(editorInsightCollapsed);
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistContextPanelMode(contextPanelMode);
    persistContextPanelPlacement(contextPanelPlacement);
    sourceDockLayout = sourceDockLayoutFromWorkspace();
    persistSourceDockLayout(sourceDockLayout);

    if (typeof window !== 'undefined') {
      window.setTimeout(measureFileTreeViewport, 0);
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
        void scanProject(selectedProject, selectedRecord?.path, { force: true });
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
      fileActionStatus = 'Browser dock reserved for embedded browser integration';
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
      persistSidePanePosition(sidePanePosition);
    }

    contextPanelCollapsed = contextGroupID === null;
    persistContextPanelCollapsed(contextPanelCollapsed);
    if (contextGroupID !== null) {
      contextPanelPlacement = contextPanelPlacementForDockGroup(contextGroupID);
      persistContextPanelPlacement(contextPanelPlacement);
    }

    editorInsightCollapsed = insightsGroupID === null;
    persistEditorInsightCollapsed(editorInsightCollapsed);
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
    return normalizeSourceDockLayout({
      ...nextLayout,
      preset: sourceLayoutPreset
    });
  }

  function dockGroupIDForPanel(layout: SourceDockLayout, panelID: SourceDockPanelID): SourceDockGroupID | null {
    return layout.groups.find((group) => group.panelIDs.includes(panelID))?.id ?? null;
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
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function toggleContextPanelCollapsed() {
    markSourceLayoutCustom();
    contextPanelCollapsed = !contextPanelCollapsed;
    persistContextPanelCollapsed(contextPanelCollapsed);
    sourceDockLayout = contextPanelCollapsed
      ? hideSourceDockPanel(sourceDockLayout, 'context')
      : showSourceDockPanel(sourceDockLayout, 'context');
    persistSourceDockLayout(sourceDockLayout);
  }

  function toggleEditorInsightCollapsed() {
    markSourceLayoutCustom();
    editorInsightCollapsed = !editorInsightCollapsed;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    sourceDockLayout = editorInsightCollapsed
      ? hideSourceDockPanel(sourceDockLayout, 'insights')
      : showSourceDockPanel(sourceDockLayout, 'insights');
    persistSourceDockLayout(sourceDockLayout);
  }

  function showEditorInsightPanel(panel: SourceIntelligencePanel) {
    markSourceLayoutCustom();
    sourceIntelligencePanel = panel;
    editorInsightCollapsed = false;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    sourceDockLayout = showSourceDockPanel(sourceDockLayout, 'insights');
    persistSourceDockLayout(sourceDockLayout);
  }

  function clearSourceLookupResults() {
    sourceDefinitionTargets = [];
    sourceDefinitionQuery = '';
    sourceDefinitionError = '';
    sourceReferenceTargets = [];
    sourceReferenceQuery = '';
    sourceReferenceError = '';
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
    sourceDockLayout = showSourceDockPanel(sourceDockLayout, 'context');
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
    sourceDockLayout = showSourceDockPanel(sourceDockLayout, 'context');
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
      } else {
        persistContextPaneWidth(contextPaneWidth);
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
      return;
    }

    const direction = event.key === 'ArrowLeft' ? 1 : -1;
    contextPaneWidth = clampContextPaneWidth(contextPaneWidth + direction * 24);
    persistContextPaneWidth(contextPaneWidth);
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
      if (reportDuplicate) {
        projectFormError = 'That path is already listed';
      } else {
        addingProject = false;
        projectFormError = '';
        void activateProject(duplicateProject, {
          forceScan: true,
          scanLimit: expandedSourceScanLimit,
          projects: projectOptions
        });
      }
      return false;
    }

    const nextCustomProjectRoots = mergeProjectRoots([], [...customProjectRoots, nextProject]);
    const nextProjectOptions = mergeProjectRoots(defaultProjectRoots, nextCustomProjectRoots);
    customProjectRoots = nextCustomProjectRoots;
    persistCustomProjectRoots(nextCustomProjectRoots);
    addingProject = false;
    projectFormError = '';
    void activateProject(nextProject, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects: nextProjectOptions
    });
    return true;
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

    const storedCustomProjectRoots = loadStoredCustomProjectRoots();
    const storedProjectOptions = mergeProjectRoots(defaultProjectRoots, storedCustomProjectRoots);
    const storedProjectID = loadStoredSelectedProjectID(storedProjectOptions);
    const storedSelectedSourcePaths = loadStoredSelectedSourcePaths();
    const storedRecentSourceRecords = loadStoredRecentSourceRecords();
    const storedOpenSourceTabs = loadStoredOpenSourceTabs();
    const storedSourceActivityMode = loadStoredSourceActivityMode();
    const storedPasteCleanupMode = loadStoredPasteCleanupMode();
    const storedSourceLayoutPreset = loadStoredSourceLayoutPreset();
    const storedSourceTerminalApp = loadStoredSourceTerminalApp();
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
    selectedProjectID = storedProject.id;
    sourceActivityMode = migrateSourceLayout ? compactPreset.activityMode : storedSourceActivityMode;
    pasteCleanupMode = storedPasteCleanupMode;
    sourceLayoutPreset = migrateSourceLayout ? compactPreset.id : storedSourceLayoutPreset;
    sourceTerminalApp = storedSourceTerminalApp;
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
    void scanProject(storedProject, storedSelectedSourcePaths[storedProject.id]).then(() =>
      indexProjectsInBackground(storedProjectOptions)
    );

    return () => {
      unlistenSourceScanProgress?.();
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
  style={`--side-pane-width: ${sidePaneWidth}px; --editor-insight-width: ${editorInsightWidth}px; --context-pane-width: ${contextPaneWidth}px; --context-pane-height: ${contextPaneHeight}px`}
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
          aria-label={scanning ? 'Stop source scan' : 'Scan source files'}
          title={scanning ? 'Stop source scan' : 'Scan source files'}
          onclick={scanning ? cancelSourceScan : () => scanProject(selectedProject, undefined, { force: true })}
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
          <input bind:value={sourceSearchQuery} placeholder="Search file contents" />
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
          <div class="scan-summary" title={scanSummaryLabel}>{scanSummaryLabel}</div>
          <div class="index-summary" title={selectedProjectIndexSummary}>{selectedProjectIndexSummary}</div>
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
                    <span class={`run-status-badge ${orchestrationStatusClass(run.status)}`}>{run.status}</span>
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
            {#if filteredProjectAgentSessions.length === 0}
              <div class="activity-empty">No conversations</div>
            {:else}
              {#each filteredProjectAgentSessions as session, index (agentSessionRowKey(session, index, 'conversation'))}
                <div class="activity-session-row" title={agentSessionResumePlan(session)}>
                  <span class="agent-provider-badge">{session.provider}</span>
                  <div class="activity-row-main">
                    <strong>{session.title}</strong>
                    <small>{agentSessionProjectLabel(session)} · {agentSessionActivityLabel(session)}</small>
                  </div>
                  <div class="activity-row-actions" aria-label="Conversation actions">
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
                    <small>{agentSessionResumeCommand(session)}</small>
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
                      type="button"
                      aria-label="Copy worktree backup command"
                      title="Copy backup command"
                      disabled={safety.kind === 'protected'}
                      onclick={() => copyWorktreeBackupCommand(worktree)}
                    >
                      <Save size={12} strokeWidth={2} />
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
              {#each filteredGitCommitHistory.slice(0, 8) as entry (entry.sha)}
                <div class="activity-commit-row" title={gitCommitTitle(entry)}>
                  <span class="git-graph-marker" aria-hidden="true"></span>
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
    class:context-side={contextPanelPlacement === 'side' && !contextPanelCollapsed}
    class:context-bottom={contextPanelPlacement === 'bottom' && !contextPanelCollapsed}
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
      <div class="context-identity-pill">
        <span>Project</span>
        <strong>{sourceContextIdentity.projectName}</strong>
      </div>
      <div class="context-identity-pill" title={sourceContextIdentity.rootPath}>
        <span>Root</span>
        <strong>{sourceContextIdentity.rootLabel}</strong>
      </div>
      <div class="context-identity-pill" title={sourceContextIdentity.gitSummary}>
        <span>Branch</span>
        <strong>{sourceContextIdentity.gitSummary}</strong>
      </div>
      <div class="context-identity-pill">
        <span>Runtime</span>
        <strong>{sourceContextIdentity.runtime}</strong>
      </div>
    </div>

    <div
      class="workspace-arrangement"
      class:context-side={contextPanelPlacement === 'side' && !contextPanelCollapsed}
      class:context-bottom={contextPanelPlacement === 'bottom' && !contextPanelCollapsed}
    >
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
            <span>{projectWorktreeSummary} · {projectWorktreeSafetyStats}</span>
          </div>
          <div class="context-card-actions">
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
            {#each projectWorktrees as worktree (worktree.path)}
              {@const safety = projectWorktreeSafety(worktree)}
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
                <em>{safety.reason} · {projectWorktreeActivityLabel(worktree)}</em>
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
                    type="button"
                    aria-label="Copy worktree backup command"
                    title="Copy backup command"
                    disabled={safety.kind === 'protected'}
                    onclick={() => copyWorktreeBackupCommand(worktree)}
                  >
                    <Save size={12} strokeWidth={2} />
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

        <div class="editor-body-grid" class:insights-hidden={editorInsightCollapsed}>
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
                onContentChange={updateSelectedSourceDraft}
                onCommandPaletteRequest={openCommandPalette}
                onDiagnosticsChange={handleEditorDiagnosticsChange}
                onDefinitionLookup={handleEditorDefinitionLookup}
                onGoToLineRequest={openCurrentFileGoToLine}
                onHoverLookup={handleEditorHoverLookup}
                onProblemsRequest={() => showEditorInsightPanel('problems')}
                onQuickOpenRequest={openQuickOpen}
                onReferenceLookup={handleEditorReferenceLookup}
                onSaveRequest={saveSelectedSourceFile}
                onSymbolsRequest={() => showEditorInsightPanel('symbols')}
                onSymbolsChange={handleEditorSymbolsChange}
              />
            {/key}

            {#if editorInsightCollapsed && (sourceDefinitionQuery || sourceDefinitionTargets.length > 0 || sourceDefinitionLoading || sourceReferenceQuery || sourceReferenceTargets.length > 0 || sourceReferenceLoading)}
              <div class="editor-lookup-popover" aria-label="Editor lookup results">
                <div class="editor-lookup-header">
                  <strong>{sourceReferenceQuery ? sourceReferenceSummary : sourceDefinitionSummary}</strong>
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
                </div>
              </div>
            {/if}
          </div>

          {#if !editorInsightCollapsed}
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
                    {#each selectedProjectGitChangedFiles as fileStatus (fileStatus.relativePath)}
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
                  {/if}
                </div>
                <div class="git-history-panel" aria-label="Git commit history">
                  <div class="git-history-heading">
                    <span>History</span>
                    <small>{gitCommitHistorySummary}</small>
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
                  <div class="git-history-list">
                    {#if gitCommitHistoryLoading}
                      <div class="intelligence-empty">Loading history</div>
                    {:else if gitCommitHistoryError}
                      <div class="intelligence-empty">{gitCommitHistoryError}</div>
                    {:else if gitCommitHistory.length === 0}
                      <div class="intelligence-empty">No commits</div>
                    {:else}
                      {#each gitCommitHistory as entry, index (entry.sha)}
                        <div class={`git-history-row ${gitCommitGraphClass(entry, index)}`} title={gitCommitTitle(entry)}>
                          <span class={`git-graph-marker ${gitCommitGraphClass(entry, index)}`} aria-hidden="true"></span>
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
                                >
                                  {entry.taskID}
                                </a>
                              {:else}
                                <span class="git-task-link">{entry.taskID}</span>
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

      {#if sourceDockPanelVisible('terminal')}
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
                aria-label="Hide terminal dock"
                title="Hide terminal dock"
                onclick={() => hideDockPanel('terminal')}
              >
                <X size={13} strokeWidth={2} />
              </button>
            </div>
          </header>

          <div class="terminal-launchpad-grid">
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
                      aria-label="Open active context in terminal"
                      title="Open context in terminal"
                      onclick={() => openActivityTerminalPath(context.cwd)}
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
                      title="Resume agent"
                      onclick={() => openAgentSessionTerminal(session)}
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
                {#each projectWorktrees.slice(0, 3) as worktree (`terminal:${worktree.path}`)}
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
                      title="Open worktree in terminal"
                      onclick={() => openActivityTerminalPath(worktree.path)}
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
    {:else}
      <div class="empty-preview">
        <FileCode2 size={34} strokeWidth={1.55} />
        <strong>No source file loaded</strong>
        <span>Scan a project or choose a file from the tree.</span>
      </div>
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
          placeholder="Open source file"
          autocomplete="off"
        />
      </label>

      <div class="quick-open-results" role="listbox" aria-label="Matching source files">
        {#if quickOpenResults.length === 0}
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
  .run-step-marker {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    color: #081714;
    border-radius: 999px;
    font-weight: 850;
    line-height: 1;
  }

  .run-status-badge {
    height: 20px;
    padding: 0 8px;
    font-size: 10px;
  }

  .run-step-marker {
    width: 18px;
    height: 18px;
    font-size: 10px;
  }

  .run-status-badge.live,
  .run-step-marker.live {
    background: #5ce2cf;
  }

  .run-status-badge.good,
  .run-step-marker.good {
    background: #8bdc9b;
  }

  .run-status-badge.bad,
  .run-step-marker.bad {
    background: #f36f6f;
  }

  .run-status-badge.attention,
  .run-step-marker.attention {
    color: #211606;
    background: #d8aa55;
  }

  .run-status-badge.idle,
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

  .scan-summary {
    min-width: 0;
    margin: -3px 0 7px;
    overflow: hidden;
    color: #7f8b87;
    font-size: 10px;
    font-weight: 720;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    grid-template-rows: auto auto minmax(0, 1fr);
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
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 9px;
    background: rgba(22, 25, 25, 0.98);
    box-shadow: 0 20px 54px rgba(0, 0, 0, 0.36);
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
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    margin: 0 0 5px;
  }

  .context-identity-pill {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 20px;
    padding: 0 6px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.04);
  }

  .context-identity-pill span,
  .context-identity-pill strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .context-identity-pill span {
    color: #87918e;
    font-size: 7px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .context-identity-pill strong {
    color: #f0f4f3;
    font-size: 9px;
    font-weight: 760;
  }

  .workspace-arrangement {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
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
    max-height: 214px;
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

  .terminal-launchpad-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
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

  .editor-toolbar {
    display: grid;
    position: relative;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 6px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
  }

  .editor-file-state {
    display: inline-grid;
    grid-template-columns: 15px minmax(0, auto) auto auto;
    align-items: center;
    justify-self: start;
    gap: 5px;
    min-width: 0;
    max-width: 100%;
    height: 22px;
    padding: 0 7px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.16);
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
    width: 24px;
    height: 22px;
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
    top: calc(100% + 5px);
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
    height: 28px;
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
  }

  .git-history-row.head {
    border-color: rgba(111, 223, 207, 0.3);
    background: rgba(111, 223, 207, 0.07);
  }

  .git-history-row.branch {
    border-color: rgba(132, 201, 222, 0.2);
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

  .git-history-actions button {
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
  .git-history-actions button:focus-visible {
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

  .definition-summary,
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
      grid-template-columns: repeat(2, minmax(0, 1fr));
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
