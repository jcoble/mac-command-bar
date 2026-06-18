<script lang="ts">
  import {
    Activity,
    BookOpen,
    Braces,
    Check,
    ChevronDown,
    ChevronLeft,
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
    PanelLeftOpen,
    Plus,
    RefreshCw,
    RotateCcw,
    Save,
    Search,
    Settings,
    SplitSquareHorizontal,
    Terminal,
    Trash2,
    X
  } from '@lucide/svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import 'dockview-core/dist/styles/dockview.css';
  import '@xterm/xterm/css/xterm.css';
  import { onMount, tick } from 'svelte';
  import type { SerializedDockview, SerializedPaneview } from 'dockview-core';
  import type { FitAddon as XTermFitAddon } from '@xterm/addon-fit';
  import type { SearchAddon as XTermSearchAddon } from '@xterm/addon-search';
  import type { SerializeAddon as XTermSerializeAddon } from '@xterm/addon-serialize';
  import type { WebglAddon as XTermWebglAddon } from '@xterm/addon-webgl';
  import type { Terminal as XTermTerminal } from '@xterm/xterm';
  import MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import SettingsPanel from '$lib/SettingsPanel.svelte';
  import { settings, defaultSettings } from '$lib/settingsStore.svelte';
  import SourceMarkdownPreview from '$lib/SourceMarkdownPreview.svelte';
  import ConversationList from '$lib/components/ConversationList.svelte';
  import Chip from '$lib/components/Chip.svelte';
  import BrowserPanel from '$lib/components/panels/BrowserPanel.svelte';
  import CommandPaletteOverlay from '$lib/components/overlays/CommandPaletteOverlay.svelte';
  import SourceDockviewShell from '$lib/SourceDockviewShell.svelte';
  import SourceWorkbench from '$lib/SourceWorkbench.svelte';
  import WorkbenchContextPanel from '$lib/WorkbenchContextPanel.svelte';
  import SourcePaneResizer from '$lib/SourcePaneResizer.svelte';
  import {
    cleanupPasteReplyDraft,
    cleanupPasteText,
    createPasteCleanupHistoryItem,
    formatPasteCleanupStats,
    pasteCleanupModes,
    summarizePasteCleanupHistoryText,
    type PasteCleanupHistoryItem,
    type PasteCleanupMode
  } from '$lib/pasteCleanup';
  import { agentSessionFocusLane, type AgentSessionFocusLane } from '$lib/agentSessionFocus';
  import {
    buildGitGraphViewModel,
    type GitGraphCommitRow,
    type GitGraphRepositoryRow,
    type GitGraphViewModel
  } from '$lib/gitGraphViewModel';
  import {
    orchestrationAgentActivityItems,
    orchestrationArtifactChips,
    orchestrationAttentionQueue,
    orchestrationCurrentActivity,
    orchestrationDecisionQueueForRuns,
    orchestrationLinkChips,
    orchestrationLiveDigestItems,
    orchestrationLoopStageMetrics,
    orchestrationLoopTallyText,
    orchestrationRunHandoffText,
    orchestrationRunMetrics,
    orchestrationRunSummaryText,
    orchestrationRunStage,
    orchestrationStatusTone,
    orchestrationTimelineDetail,
    orchestrationTimelineItems,
    type OrchestrationTimelineItem
  } from '$lib/orchestrationView';
  import { sourcePreviewAppearance, sourcePreviewAppearanceKey } from '$lib/sourcePreviewAppearance';
  import {
    buildWorktreeCleanupBrief,
    buildWorktreeCleanupScript,
    buildWorktreeDecisionQueue,
    buildWorktreeSafetySummary,
    prioritizeWorktreesForCleanup,
    worktreeDecisionLane,
    worktreePrimaryAction
  } from '$lib/worktreeSafety';
  import { buildWorktreeCleanupPlan } from '$lib/worktreeCleanupPlan';
  import { buildWorktreeCleanupRunbook } from '$lib/worktreeCleanupRunbook';
  import {
    describeWorkspaceSnapshotRestoreReadiness,
    parseStoredWorkspaceSnapshot,
    restoreWorkspaceSnapshot,
    selectStartupWorkspaceSnapshot,
    snapshotStorageKey,
    upsertWorkspaceSnapshot,
    workspaceSnapshotsForWorktreePath,
    type WorkspaceSnapshot,
    type WorkspaceSnapshotEmbeddedTerminal,
    type WorkspaceSnapshotViewState
  } from '$lib/workspaceSnapshot';
  import {
    createWorkspaceSessionSnapshotPlan,
    workspaceSnapshotProviderForSession,
    workspaceSnapshotSessionIDForSession
  } from '$lib/workspaceSnapshotPlan';
  import {
    activateSourceDockPanel,
    createDefaultSourceDockLayout,
    hideSourceDockPanel,
    moveSourceDockPanel,
    normalizeSourceDockLayout,
    resizeSourceDockGroups,
    resizeSourceDockGroup,
    showSourceDockPanel,
    sourceDockGroupSize,
    sourceDockPanelDescriptors,
    type SourceDockGroupID,
    type SourceDockLayout,
    type SourceDockPanelID
  } from '$lib/sourceDockLayout';
  import {
    createSourcePaneviewStackWorkspace,
    createSourceDockviewTabStackWorkspace,
    createSourceDockviewWorkspace,
    sourceDockviewMigrationSlicePlanOptions,
    sourceDockviewMigrationSliceStorageKey,
    sourceDockviewStorageKey,
    type SourcePaneviewStackPanel,
    type SourcePaneviewStackWorkspace,
    type SourceDockviewTabStackPanel,
    type SourceDockviewTabStackWorkspace,
    type SourceDockviewMigrationSliceID,
    type SourceDockviewWorkspace
  } from '$lib/sourceDockviewWorkspace';
  import {
    clampSourcePaneSize,
    deriveSourcePaneState,
    finishSourcePanePointerSize,
    resolveSourcePaneWorkspacePlan,
    restoreSourcePaneExpandedSize,
    type SourcePaneSizingConfig
  } from '$lib/sourcePaneSizing';
  import {
    applySourceTextEdits,
    buildSourceTree,
    buildGitTaskSourceGroups,
    buildProjectActivationScanPlan,
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
    formatGitTaskSourceGroupHandoff,
    formatSourceContextGitSummary,
    formatSourceContextIdentity,
    formatSourceContextRootLabel,
    formatSourceDiagnosticSummary,
    formatSourceIndexSummary,
    flattenSourceTree,
    formatSourceScanEvidence,
    formatSourceScanHealth,
    formatSourceScanRecovery,
    formatSourceScanStats,
    formatSourceScanSummary,
    folderIdsForSourceRecord,
    getSourceScanCacheEntry,
    isSuspiciousSourceScanResult,
    mergeProjectRoots,
    normalizeProjectPath,
    navigateSourceHistoryBack,
    navigateSourceHistoryForward,
    parseStoredSourceScanCache,
    parseQuickOpenQuery,
    previewFromContent,
    pushSourceNavigationHistory,
    rankSourceRecords,
    removeSourceScanCacheEntries,
    scrollTopForSourceTreeReveal,
    selectBackgroundIndexProjects,
    selectPreferredSourceRecord,
    sourceNavigationLocationForRecord,
    sourceScanCacheEntryNeedsRepair,
    shouldRepairSuspiciousSourceScan,
    sourceLanguageForPath,
    sourceSupportsLanguageIntelligence,
    taskReferenceUrl,
    textMatchesSearchTokens,
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
    type SourceScanEvidenceMode,
    type SourceCompletionItem,
    type SourceDiagnostic,
    type SourceInlayHint,
    type GitTaskSourceGroup,
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
    archiveProjectWorktreeFromTauri,
    isNativeTauriRuntime,
    listAgentSessionsFromTauri,
    listGitRepositorySummariesFromTauri,
    listAgentSessionsFromLocalBridge,
    listOrchestrationRunsFromTauri,
    listPlaywrightSessionsFromTauri,
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
    readSourceLspReadinessFromTauri,
    readSourceLspStatusFromTauri,
    readSourceGitDiffFromTauri,
    readSourceFromTauri,
    readTerminalSessionScrollbackFromTauri,
    recordOrchestrationEventToTauri,
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
    validateProjectRootFromTauri,
    writeTerminalSessionFromTauri,
    closeTerminalSessionFromTauri,
    killPlaywrightSessionsFromTauri,
    writeSourceToTauri,
    type NativeSourceScanProgress,
    type AgentSession,
    type GitCommitHistoryEntry,
    type GitRepositorySummary,
    type OrchestrationArtifact,
    type OrchestrationEvent,
    type OrchestrationRun,
    type PlaywrightSessionInfo,
    type ProjectGitFileStatus,
    type ProjectGitStatus,
    type ProjectRootValidationResult,
    type ProjectWorktree,
    type RuntimeContext,
    type SourceGitDiff,
    type TerminalOutputPayload,
    type TerminalSessionInfo
  } from '$lib/tauriSource';

  const customProjectRootsStorageKey = 'mac-command-bar.source-browser.custom-project-roots';
  const selectedProjectStorageKey = 'mac-command-bar.source-browser.selected-project';
  const selectedSourcePathStorageKey = 'mac-command-bar.source-browser.selected-source-paths';
  const sourceScanCacheStorageKey = 'mac-command-bar.source-browser.source-scan-cache';
  const sourceScanCacheSignatureVersion = 'source-scan-cache-v3';
  const recentSourceRecordsStorageKey = 'mac-command-bar.source-browser.recent-source-records';
  const openSourceTabsStorageKey = 'mac-command-bar.source-browser.open-source-tabs';
  const sourceActivityModeStorageKey = 'mac-command-bar.source-browser.activity-mode';
  const sourceLayoutPresetStorageKey = 'mac-command-bar.source-browser.layout-preset';
  const sourceLayoutPresetOverridesStorageKey = 'mac-command-bar.source-browser.layout-preset-overrides';
  const sourceLayoutVersionStorageKey = 'mac-command-bar.source-browser.layout-version';
  const sourceChromeCompactStorageKey = 'mac-command-bar.source-browser.chrome-compact';
  const sourceTerminalAppStorageKey = 'mac-command-bar.source-browser.terminal-app';
  const sourceDockLayoutStorageKey = 'mac-command-bar.source-browser.dock-layout';
  const sourceFocusRestoreLayoutStorageKey = 'mac-command-bar.source-browser.focus-restore-layout';
  const browserDockUrlStorageKey = 'mac-command-bar.source-browser.browser-url';
  const activeWorkspaceSessionStorageKey = 'mac-command-bar.source-browser.active-workspace-session';
  const pasteCleanupModeStorageKey = 'mac-command-bar.source-browser.paste-cleanup-mode';
  const pasteCleanupHistoryStorageKey = 'mac-command-bar.source-browser.paste-cleanup-history';
  const contextPanelModeStorageKey = 'mac-command-bar.source-browser.context-panel-mode';
  const contextPanelPlacementStorageKey = 'mac-command-bar.source-browser.context-panel-placement';
  const sidePanePositionStorageKey = 'mac-command-bar.source-browser.side-pane-position';
  const sidePaneWidthStorageKey = 'mac-command-bar.source-browser.side-pane-width';
  const sidePaneExpandedWidthStorageKey = 'mac-command-bar.source-browser.side-pane-expanded-width';
  const editorInsightWidthStorageKey = 'mac-command-bar.source-browser.editor-insight-width';
  const editorInsightCollapsedStorageKey = 'mac-command-bar.source-browser.editor-insight-collapsed';
  const contextPaneWidthStorageKey = 'mac-command-bar.source-browser.context-pane-width';
  const contextPaneExpandedWidthStorageKey = 'mac-command-bar.source-browser.context-pane-expanded-width';
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
  const maxPasteCleanupHistoryItems = 12;
  const maxVisiblePasteCleanupHistoryItems = 4;
  const commandCenterTaskUrls: Record<string, string> = {
    'TSK-127':
      'https://app.notion.com/p/TSK-127-Create-a-native-MAC-OS-app-for-doing-diff-things-in-menu-bar-379394b0689d8053af76fd44c7ffdba4',
    'TSK-192':
      'https://app.notion.com/p/TSK-192-Add-conversation-workspace-restore-snapshots-37c394b0689d810d9d74e798a144a3e9'
  };
  const sourceScanCacheMaxAgeMs = 12 * 60 * 60 * 1000;
  const maxSourceScanCacheEntries = 8;
  const suspiciousSourceIndexFileThreshold = 24;
  const sourceTreeRowHeight = 30;
  const sourceTreeOverscanRows = 8;
  const sourceTreeFallbackViewportHeight = 420;
  const sidePaneDefaultWidth = 407;
  const sidePaneMinWidth = 40;
  const sidePaneMaxWidth = 1600;
  const sidePaneCollapseThreshold = 48;
  const sidePaneRailOnlyThreshold = 260;
  const editorInsightDefaultWidth = 260;
  const editorInsightMinWidth = 96;
  const editorInsightMaxWidth = 1200;
  const editorInsightCollapseThreshold = 76;
  const contextPaneDefaultWidth = 330;
  const contextPaneMinWidth = 34;
  const contextPaneMaxWidth = 1600;
  const contextPaneCollapseThreshold = 30;
  const contextPaneRailOnlyThreshold = 220;
  const contextPaneDefaultHeight = 260;
  const contextPaneMinHeight = 96;
  const contextPaneMaxHeight = 1100;
  const contextPaneHeightCollapseThreshold = 84;
  const bottomDockDefaultHeight = 300;
  const bottomDockMinHeight = 96;
  const bottomDockFallbackMaxHeight = 1100;
  const bottomDockReservedEditorHeight = 148;
  const bottomDockReservedChromeHeight = 92;
  const bottomDockCollapseThreshold = 84;
  const sourceWorkspaceMinimumEditorWidth = 620;
  const sourceWorkspacePaneGapSize = 0;
  const sourceWorkspacePaneChromeSize = 0;
  const sourceWorkspaceDefaultViewportWidth =
    sourceWorkspaceMinimumEditorWidth +
    sidePaneDefaultWidth +
    contextPaneDefaultWidth +
    (sourceWorkspacePaneGapSize + sourceWorkspacePaneChromeSize) * 2;
  const activityPaneSizingConfig: SourcePaneSizingConfig = {
    defaultSize: sidePaneDefaultWidth,
    minSize: sidePaneMinWidth,
    maxSize: sidePaneMaxWidth,
    collapseThreshold: sidePaneCollapseThreshold,
    railThreshold: sidePaneRailOnlyThreshold,
    railSize: sidePaneMinWidth
  };
  const contextPaneWidthSizingConfig: SourcePaneSizingConfig = {
    defaultSize: contextPaneDefaultWidth,
    minSize: contextPaneMinWidth,
    maxSize: contextPaneMaxWidth,
    collapseThreshold: contextPaneCollapseThreshold,
    railThreshold: contextPaneRailOnlyThreshold,
    railSize: contextPaneMinWidth
  };
  const contextPaneHeightSizingConfig: SourcePaneSizingConfig = {
    defaultSize: contextPaneDefaultHeight,
    minSize: contextPaneMinHeight,
    maxSize: contextPaneMaxHeight,
    collapseThreshold: contextPaneHeightCollapseThreshold
  };
  const orchestrationRefreshIntervalMs = 5_000;
  const defaultOrchestrationEventFilePath = '/tmp/mcb-orchestration-events.jsonl';
  const sourceScanProgressEventName = nativeSourceScanProgressEvent;
  const expandedSourceScanLimitShortLabel = `${Math.round(expandedSourceScanLimit / 1000)}K`;
  const sourceLayoutVersion = '2026-06-center-runtime-no-bottom-row';
  // A/B flag: when true, render the unified Task-2 `SourceWorkbench` (one Dockview with
  // native drag-between-groups, tab switching, and border-drag resizing) as the whole shell.
  // When false, fall back to the existing homemade shell + multi-Dockview path untouched.
  const useUnifiedWorkbench = true;
  const sourceDockviewWorkbenchEnabled = true;
  const sourceDockviewWorkbenchStorageKey = sourceDockviewStorageKey;
  const sourceDockviewActivityEnabled = !sourceDockviewWorkbenchEnabled;
  const sourceDockviewActivitySliceID: SourceDockviewMigrationSliceID = 'activity-only';
  const sourceDockviewActivityPlanOptions =
    sourceDockviewMigrationSlicePlanOptions(sourceDockviewActivitySliceID);
  const sourceDockviewActivityPanelIDs = sourceDockviewActivityPlanOptions.panelIDs ?? [];
  const sourceDockviewActivityStorageKey =
    sourceDockviewMigrationSliceStorageKey(sourceDockviewActivitySliceID);
  const sourceDockviewInsightsEnabled = !sourceDockviewWorkbenchEnabled;
  const sourceDockviewInsightsSliceID: SourceDockviewMigrationSliceID = 'insights-only';
  const sourceDockviewInsightsPlanOptions =
    sourceDockviewMigrationSlicePlanOptions(sourceDockviewInsightsSliceID);
  const sourceDockviewInsightsPanelIDs = sourceDockviewInsightsPlanOptions.panelIDs ?? [];
  const sourceDockviewInsightsStorageKey =
    sourceDockviewMigrationSliceStorageKey(sourceDockviewInsightsSliceID);
  const sourceDockviewContextEnabled = !sourceDockviewWorkbenchEnabled;
  const sourceDockviewContextSliceID: SourceDockviewMigrationSliceID = 'context-insights';
  const sourceDockviewContextPlanOptions =
    sourceDockviewMigrationSlicePlanOptions(sourceDockviewContextSliceID);
  const sourceDockviewContextPanelIDs = sourceDockviewContextPlanOptions.panelIDs ?? [];
  const sourceDockviewContextStorageKey =
    sourceDockviewMigrationSliceStorageKey(sourceDockviewContextSliceID);
  const sourceDockviewCenterEnabled = !sourceDockviewWorkbenchEnabled;
  const sourceDockviewCenterSliceID: SourceDockviewMigrationSliceID = 'center-runtime';
  const sourceDockviewCenterPlanOptions =
    sourceDockviewMigrationSlicePlanOptions(sourceDockviewCenterSliceID);
  const sourceDockviewCenterPanelIDs = sourceDockviewCenterPlanOptions.panelIDs ?? [];
  const sourceDockviewCenterStorageKey =
    sourceDockviewMigrationSliceStorageKey(sourceDockviewCenterSliceID);
  const sourceDockviewBottomEnabled = !sourceDockviewWorkbenchEnabled;
  const sourceDockviewBottomSliceID: SourceDockviewMigrationSliceID = 'bottom-runtime';
  const sourceDockviewBottomPlanOptions =
    sourceDockviewMigrationSlicePlanOptions(sourceDockviewBottomSliceID);
  const sourceDockviewBottomPanelIDs = sourceDockviewBottomPlanOptions.panelIDs ?? [];
  const sourceDockviewBottomStorageKey =
    sourceDockviewMigrationSliceStorageKey(sourceDockviewBottomSliceID);
  const initialProject = defaultProjectRoots[0];
  const macCommandBarRepoPath =
    defaultProjectRoots.find((project) => project.id === 'mac-command-bar')?.path ??
    '/Users/blackcolours/dev/work/mac-command-bar';

  type SourceIntelligenceAction =
    | 'completion'
    | 'definition'
    | 'format'
    | 'hover'
    | 'implementation'
    | 'quick-fix'
    | 'references'
    | 'rename'
    | 'signature-help'
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
  type SourceEditorNavPanel = 'problems' | 'symbols' | 'definitions' | 'references';
  type SourceEditorDisplayMode = 'source' | 'preview';
  type SourceActivityMode =
    | 'files'
    | 'clipboard'
    | 'conversations'
    | 'runs'
    | 'sessions'
    | 'agents'
    | 'worktrees'
    | 'git';
  type SourceEditorFilePanelID = `file:${string}`;
  type SourceFilesPaneID = 'files' | 'search' | 'recent';
  type SourceConversationPaneID = 'active' | 'saved';
  type SourceContextCardDockviewPanelID = SourceContextCardID;
  const sourceEditorFileDockviewStorageKey = `${sourceDockviewStorageKey}.center.editor-files`;
  const sourceFilesDockviewStorageKey = `${sourceDockviewStorageKey}.activity.files.paneview`;
  const sourceFilesDockviewPanels: SourcePaneviewStackPanel<SourceFilesPaneID>[] = [
    { id: 'files', title: 'Files', size: 460, minimumBodySize: 160, isExpanded: true },
    { id: 'search', title: 'Search', size: 190, minimumBodySize: 72, isExpanded: false },
    { id: 'recent', title: 'Recent', size: 170, minimumBodySize: 72, isExpanded: false }
  ];
  const sourceFilesDockviewPanelIDs = sourceFilesDockviewPanels.map((panel) => panel.id);
  const sourceConversationDockviewStorageKey = `${sourceDockviewStorageKey}.activity.conversations.paneview`;
  const sourceConversationDockviewPanels: SourcePaneviewStackPanel<SourceConversationPaneID>[] = [
    { id: 'active', title: 'Active', size: 390, minimumBodySize: 120, isExpanded: true },
    { id: 'saved', title: 'Saved', size: 210, minimumBodySize: 92, isExpanded: false }
  ];
  const sourceConversationDockviewPanelIDs = sourceConversationDockviewPanels.map((panel) => panel.id);
  const sourceContextCardDockviewStorageKey = `${sourceDockviewStorageKey}.context.cards.paneview`;
  type SourceLayoutPresetID = 'review' | 'code' | 'git' | 'runs' | 'sessions' | 'custom';
  type SourceTerminalApp = 'Warp' | 'Terminal' | 'iTerm' | 'iTerm2' | 'Ghostty' | 'WezTerm' | 'Alacritty';
  type SourceContextPanelMode = 'grid' | 'stack';
  type SourceContextPanelPresentationMode = 'grid' | 'stack' | 'tabs';
  type SourceContextPanelPlacement = 'top' | 'side' | 'bottom';
  type SourceContextPanelPresentation = {
    requestedMode: SourceContextPanelMode;
    placement: SourceContextPanelPlacement;
    presentationMode: SourceContextPanelPresentationMode;
    stacked: boolean;
    tabbed: boolean;
    singleCard: boolean;
  };
  type SourceSidePanePosition = 'left' | 'right';
  type SourceWorkspacePaneID = 'activity' | 'context';
  type SourceContextCardID = 'orchestration' | 'runtime' | 'agents' | 'worktrees' | 'repo';
  type GitTaskLedgerTone = 'blocked' | 'ready' | 'review' | 'protected' | 'clean';
  type WorktreeOwnerChipTone = 'live' | 'saved' | 'warning' | 'muted';
  type WorktreeOwnerChip = {
    id: string;
    label: string;
    title: string;
    tone: WorktreeOwnerChipTone;
  };
  type GitTaskLedgerRow = {
    taskID: string;
    sourceSummary: string;
    detailSummary: string;
    worktreeCount: number;
    blockedWorktreeCount: number;
    readyWorktreeCount: number;
    cleanupCandidateCount: number;
    staleCleanWorktreeCount: number;
    backupRequiredWorktreeCount: number;
    activeSessionCount: number;
    savedWorkspaceCount: number;
    commitCount: number;
    runCount: number;
    ownerSummary: string;
    cleanupSummary: string;
    nextAction: string;
    tone: GitTaskLedgerTone;
    primaryWorktree: ProjectWorktree | null;
    latestCommit: GitCommitHistoryEntry | null;
  };
  type SourceLayoutPresetDefinition = {
    id: Exclude<SourceLayoutPresetID, 'custom'>;
    label: string;
    title: string;
    activityMode: SourceActivityMode;
    sidePaneWidth: number;
    sidePanePosition: SourceSidePanePosition;
    editorInsightWidth: number;
    editorInsightCollapsed: boolean;
    contextPaneWidth: number;
    contextPaneHeight: number;
    contextPanelCollapsed: boolean;
    contextPanelMode: SourceContextPanelMode;
    contextPanelPlacement: SourceContextPanelPlacement;
    chromeCompact: boolean;
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
  type SourceLayoutSnapshot = SourceLayoutPresetOverride & {
    preset: SourceLayoutPresetID;
  };
  type SourceLayoutPresetOverrides = Partial<Record<ConcreteSourceLayoutPresetID, SourceLayoutPresetOverride>>;

  const sourceLayoutPresets: SourceLayoutPresetDefinition[] = [
    {
      id: 'review',
      label: 'Review',
      title: 'Balanced source review with side context visible',
      activityMode: 'files',
      sidePaneWidth: sidePaneDefaultWidth,
      sidePanePosition: 'left',
      editorInsightWidth: editorInsightDefaultWidth,
      editorInsightCollapsed: false,
      contextPaneWidth: contextPaneDefaultWidth,
      contextPaneHeight: contextPaneDefaultHeight,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      chromeCompact: true,
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
      contextPaneWidth: contextPaneDefaultWidth,
      contextPaneHeight: contextPaneDefaultHeight,
      contextPanelCollapsed: true,
      contextPanelMode: 'grid',
      contextPanelPlacement: 'top',
      chromeCompact: true,
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
      contextPaneWidth: 360,
      contextPaneHeight: contextPaneDefaultHeight,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      chromeCompact: true,
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
      contextPaneWidth: 360,
      contextPaneHeight: contextPaneDefaultHeight,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      chromeCompact: true,
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
      contextPaneWidth: 360,
      contextPaneHeight: contextPaneDefaultHeight,
      contextPanelCollapsed: false,
      contextPanelMode: 'stack',
      contextPanelPlacement: 'side',
      chromeCompact: true,
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
  const sourceTerminalFallbackApp: SourceTerminalApp = 'Terminal';
  const dockTabGroupIDs: SourceDockGroupID[] = sourceDockviewWorkbenchEnabled
    ? []
    : sourceDockviewBottomEnabled
    ? ['right']
    : ['right', 'bottom'];
  const dockPanelDragDataType = 'application/x-mcb-dock-panel';
  const sourceDockviewWorkbenchPanelIDs: SourceDockPanelID[] = [
    'activity',
    'editor',
    'context',
    'insights',
    'terminal',
    'browser'
  ];
  const managedDockPanelIDs: SourceDockPanelID[] = ['activity', 'context', 'insights', 'terminal', 'browser'];
  const hideableDockPanelIDs: SourceDockPanelID[] = ['activity', 'context', 'insights', 'terminal', 'browser'];
  const contextCardOrder: SourceContextCardID[] = ['orchestration', 'runtime', 'agents', 'worktrees', 'repo'];
  const contextCardLabels: Record<SourceContextCardID, string> = {
    orchestration: 'Runs',
    runtime: 'Runtime',
    agents: 'Agents',
    worktrees: 'Worktrees',
    repo: 'Git'
  };
  const contextCardPaneviewSizes: Record<SourceContextCardID, number> = {
    orchestration: 210,
    runtime: 160,
    agents: 170,
    worktrees: 230,
    repo: 210
  };

  // ── Settings panel + live appearance wiring ──────────────────────────────
  // The settings dialog open state. The gear in the activity rail toggles it.
  let settingsOpen = $state(false);

  // Baseline defaults the store ships with. Comparing the live settings against
  // these lets us apply overrides ONLY when the user has actually changed a
  // value — so an untouched install renders byte-for-byte identical to before
  // this wiring existed (Monaco/terminal/app text all use their old hardcoded
  // values via the unchanged default paths).
  const appearanceDefaults = defaultSettings();

  // Editor (Monaco) override: each field is included only when it differs from
  // the editor default. When nothing changed, the object is empty/undefined and
  // MonacoSourceEditor falls back entirely to sourcePreviewAppearance.
  let editorAppearanceOverride = $derived.by(() => {
    const override: { fontSize?: number; fontFamily?: string; lineHeight?: number } = {};
    if (settings.editor.fontSize !== appearanceDefaults.editor.fontSize) {
      override.fontSize = settings.editor.fontSize;
    }
    if (settings.editor.fontFamily !== appearanceDefaults.editor.fontFamily) {
      override.fontFamily = settings.editor.fontFamily;
    }
    if (settings.editor.lineHeight !== appearanceDefaults.editor.lineHeight) {
      override.lineHeight = settings.editor.lineHeight;
    }
    return Object.keys(override).length > 0 ? override : undefined;
  });

  // App (chrome/UI) font size: only override the inherited baseline when the
  // user changed it. When unchanged, `--app-font-size` is left unset and the
  // shell keeps inheriting the original :root size — no visual change.
  let appFontSizeOverridden = $derived(
    settings.appearance.appFontSize !== appearanceDefaults.appearance.appFontSize
  );

  // Embedded terminal (xterm) font. These are the EXACT values previously
  // hardcoded in the XTerm constructor; they remain the source of truth for the
  // default look. We only swap in a store value when the user actually changed
  // it from the store default — so an untouched install keeps the original full
  // font stack / 15px / 1.2 line height (the store default fontFamily is the
  // bare "Google Sans Mono", which is NOT the same as this fallback stack).
  const EMBEDDED_TERMINAL_FONT_DEFAULTS = {
    fontFamily: '"Google Sans Mono", "SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace',
    fontSize: 15,
    lineHeight: 1.2
  } as const;

  let embeddedTerminalAppearance = $derived({
    fontFamily:
      settings.terminal.fontFamily !== appearanceDefaults.terminal.fontFamily
        ? settings.terminal.fontFamily
        : EMBEDDED_TERMINAL_FONT_DEFAULTS.fontFamily,
    fontSize:
      settings.terminal.fontSize !== appearanceDefaults.terminal.fontSize
        ? settings.terminal.fontSize
        : EMBEDDED_TERMINAL_FONT_DEFAULTS.fontSize,
    lineHeight:
      settings.terminal.lineHeight !== appearanceDefaults.terminal.lineHeight
        ? settings.terminal.lineHeight
        : EMBEDDED_TERMINAL_FONT_DEFAULTS.lineHeight
  });

  let customProjectRoots = $state<ProjectRoot[]>([]);
  let selectedSourcePaths = $state<Record<string, string>>({});
  let recentSourceRecords = $state<SourceRecentRecord[]>([]);
  let openSourceTabs = $state<SourceOpenTab[]>([]);
  let workspaceSnapshots = $state<WorkspaceSnapshot[]>([]);
  let activeWorkspaceSessionKey = $state<string | null>(null);
  let sourceScanCache = $state<SourceScanCache>({});
  let sourceScanModeByProject = $state<Record<string, SourceScanEvidenceMode>>({});
  let backgroundIndexingProjectIDs = $state<Set<string>>(new Set());
  let backgroundIndexErrorByProject = $state<Record<string, string>>({});
  let projectGitStatus = $state<ProjectGitStatus | null>(null);
  let projectGitLoading = $state(false);
  let projectGitError = $state('');
  let runtimeContexts = $state<RuntimeContext[]>([]);
  let runtimeContextsLoading = $state(false);
  let runtimeContextError = $state('');
  let runtimeContextSource = $state('browser preview');
  let playwrightSessions = $state<PlaywrightSessionInfo[]>([]);
  let playwrightSessionsLoading = $state(false);
  let playwrightSessionError = $state('');
  let playwrightSessionSource = $state('browser preview');
  let playwrightSessionsKilling = $state(false);
  let playwrightSessionsRequestID = 0;
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
  let orchestrationEventFilePath = $state(defaultOrchestrationEventFilePath);
  let orchestrationEventRecording = $state(false);
  let orchestrationEventFileChoosing = $state(false);
  let orchestrationEventImportStatus = $state('');
  let orchestrationRunsRefreshInFlight = false;
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
  let sourceMarkdownPreviewModeByPath = $state<Record<string, SourceEditorDisplayMode>>({});
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
  let sourceEditorFileDockviewReady = $state(false);
  let sourceEditorFileDockviewError = $state('');
  let sourceFilesPane = $state<SourceFilesPaneID>('files');
  let sourceFilesDockviewReady = $state(false);
  let sourceFilesDockviewError = $state('');
  let sourceConversationPane = $state<SourceConversationPaneID>('active');
  let sourceConversationDockviewReady = $state(false);
  let sourceConversationDockviewError = $state('');
  let sourceContextCardDockviewReady = $state(false);
  let sourceContextCardDockviewError = $state('');
  let sourceActivityFilter = $state('');
  let sourceActivityFiltersByMode = $state<Record<SourceActivityMode, string>>(createSourceActivityFilterState());
  let editorNavPanel = $state<SourceEditorNavPanel | null>(null);
  let pasteCleanupInput = $state('');
  let pasteCleanupReplyDraft = $state('');
  let pasteCleanupMode = $state<PasteCleanupMode>('plain');
  let pasteCleanupHistory = $state<PasteCleanupHistoryItem[]>([]);
  let sourceLayoutPreset = $state<SourceLayoutPresetID>('code');
  let sourceLayoutPresetOverrides = $state<SourceLayoutPresetOverrides>({});
  let sourceChromeCompact = $state(true);
  let sourceTerminalApp = $state<SourceTerminalApp>('Warp');
  let embeddedTerminalSession = $state<TerminalSessionInfo | null>(null);
  let embeddedTerminalSessions = $state<TerminalSessionInfo[]>([]);
  let embeddedTerminalSessionsLoading = $state(false);
  let embeddedTerminalSessionsError = $state('');
  let embeddedTerminalStarting = $state(false);
  let embeddedTerminalCommandDraft = $state('');
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
  let sidePaneExpandedWidth = $state(sidePaneDefaultWidth);
  let editorInsightWidth = $state(editorInsightDefaultWidth);
  let editorInsightCollapsed = $state(true);
  let contextPaneWidth = $state(contextPaneDefaultWidth);
  let contextPaneExpandedWidth = $state(contextPaneDefaultWidth);
  let contextPaneHeight = $state(contextPaneDefaultHeight);
  let contextPanelCollapsed = $state(true);
  let sourceDockLayout = $state<SourceDockLayout>(createDefaultSourceDockLayout());
  let sourceFocusRestoreLayout = $state<SourceLayoutSnapshot | null>(null);
  let sourceDockviewWorkbenchReady = $state(false);
  let sourceDockviewWorkbenchError = $state('');
  let sourceDockviewActivityReady = $state(false);
  let sourceDockviewActivityError = $state('');
  let sourceDockviewInsightsReady = $state(false);
  let sourceDockviewInsightsError = $state('');
  let sourceDockviewContextReady = $state(false);
  let sourceDockviewContextError = $state('');
  let sourceDockviewCenterReady = $state(false);
  let sourceDockviewCenterError = $state('');
  let sourceDockviewBottomReady = $state(false);
  let sourceDockviewBottomError = $state('');
  let draggingDockPanelID = $state<SourceDockPanelID | null>(null);
  let dockDropTargetGroupID = $state<SourceDockGroupID | null>(null);
  let dockDropTargetPanelID = $state<SourceDockPanelID | null>(null);
  let dockDropTargetPanelPlacement = $state<'before' | 'after' | null>(null);
  let hiddenContextCardIDs = $state<Set<SourceContextCardID>>(new Set());
  let activeContextCardID = $state<SourceContextCardID>('orchestration');
  let viewMenuOpen = $state(false);
  let editorActionMenuOpen = $state(false);
  let activeActivityRowActionMenu = $state<string | null>(null);
  let activeAgentRowActionMenu = $state<string | null>(null);
  let activeWorktreeRowActionMenu = $state<string | null>(null);
  let activeGitRowActionMenu = $state<string | null>(null);
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
  let projectRootValidating = $state(false);
  let projectRootValidationByPath = $state<Record<string, ProjectRootValidationResult>>({});
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
  let sourceWorkspaceElement = $state<HTMLElement | null>(null);
  let sourceWorkspaceWidth = $state(0);
  let sourceWorkspaceHeight = $state(0);
  let fileTreeElement = $state<HTMLDivElement | null>(null);
  let embeddedTerminalElement = $state<HTMLDivElement | null>(null);
  let fileTreeScrollTop = $state(0);
  let fileTreeViewportHeight = $state(sourceTreeFallbackViewportHeight);
  let pendingTreeRevealPath = $state<string | null>(null);
  let pendingTreeFocusRowIndex = $state<number | null>(null);
  let projectNameInput = $state('');
  let projectPathInput = $state('');
  let projectFormError = $state('');
  let projectActivationGeneration = 0;
  let scanGeneration = 0;
  let projectGitStatusRequestID = 0;
  let selectedSourceGitDiffRequestID = 0;
  let sourceIntelligenceCommandId = 0;
  let sourceLspDiagnosticsTimer: number | null = null;
  let sourceDockviewWorkbenchWorkspace: SourceDockviewWorkspace | null = null;
  let sourceDockviewWorkbenchResizeObserver: ResizeObserver | null = null;
  let sourceDockviewWorkbenchHostToken = 0;
  // Unified Task-2 workbench handles (set by <SourceWorkbench> via bindable props).
  let unifiedWorkbenchSetPanelElement = $state<
    (panelID: SourceDockPanelID, element: HTMLElement | null) => void
  >(() => {});
  let unifiedWorkbenchReady = $state(false);
  let unifiedWorkbenchError = $state('');
  let sourceDockviewActivityWorkspace: SourceDockviewWorkspace | null = null;
  let sourceDockviewActivityResizeObserver: ResizeObserver | null = null;
  let sourceDockviewActivityHostToken = 0;
  let sourceDockviewInsightsWorkspace: SourceDockviewWorkspace | null = null;
  let sourceDockviewInsightsResizeObserver: ResizeObserver | null = null;
  let sourceDockviewInsightsHostToken = 0;
  let sourceDockviewContextWorkspace: SourceDockviewWorkspace | null = null;
  let sourceDockviewContextResizeObserver: ResizeObserver | null = null;
  let sourceDockviewContextHostToken = 0;
  let sourceDockviewCenterWorkspace: SourceDockviewWorkspace | null = null;
  let sourceDockviewCenterResizeObserver: ResizeObserver | null = null;
  let sourceDockviewCenterHostToken = 0;
  let sourceDockviewBottomWorkspace: SourceDockviewWorkspace | null = null;
  let sourceDockviewBottomResizeObserver: ResizeObserver | null = null;
  let sourceDockviewBottomHostToken = 0;
  const sourceDockviewPanelElements = new Map<SourceDockPanelID, HTMLElement>();
  let sourceEditorFileDockviewWorkspace: SourceDockviewTabStackWorkspace<SourceEditorFilePanelID> | null = null;
  let sourceEditorFileDockviewResizeObserver: ResizeObserver | null = null;
  let sourceEditorFileDockviewHostToken = 0;
  const sourceEditorFileDockviewPanelElements = new Map<SourceEditorFilePanelID, HTMLElement>();
  let sourceFilesDockviewWorkspace: SourcePaneviewStackWorkspace<SourceFilesPaneID> | null = null;
  let sourceFilesDockviewResizeObserver: ResizeObserver | null = null;
  let sourceFilesDockviewHostToken = 0;
  const sourceFilesDockviewPanelElements = new Map<SourceFilesPaneID, HTMLElement>();
  let sourceConversationDockviewWorkspace: SourcePaneviewStackWorkspace<SourceConversationPaneID> | null = null;
  let sourceConversationDockviewResizeObserver: ResizeObserver | null = null;
  let sourceConversationDockviewHostToken = 0;
  const sourceConversationDockviewPanelElements = new Map<SourceConversationPaneID, HTMLElement>();
  let sourceContextCardDockviewWorkspace: SourcePaneviewStackWorkspace<SourceContextCardDockviewPanelID> | null = null;
  let sourceContextCardDockviewResizeObserver: ResizeObserver | null = null;
  let sourceContextCardDockviewHostToken = 0;
  const sourceContextCardDockviewPanelElements = new Map<SourceContextCardDockviewPanelID, HTMLElement>();
  let embeddedTerminal: XTermTerminal | null = null;
  let embeddedTerminalFitAddon: XTermFitAddon | null = null;
  let embeddedTerminalSearchAddon: XTermSearchAddon | null = null;
  let embeddedTerminalSerializeAddon: XTermSerializeAddon | null = null;
  let embeddedTerminalWebglAddon: XTermWebglAddon | null = null;
  let embeddedTerminalAddonStatus = $state('fit');
  let embeddedTerminalInputDisposable: { dispose: () => void } | null = null;
  const embeddedTerminalDeferredFitDelays = [50, 180, 420];
  let embeddedTerminalFitFrame = 0;
  let embeddedTerminalFitTimers: number[] = [];
  let embeddedTerminalRendererLoading = false;
  let embeddedTerminalRendererPromise: Promise<void> | null = null;

  type SourceScanOptions = {
    force?: boolean;
    limit?: number;
    skipTinyIndexRepair?: boolean;
    preserveSelectedRecord?: boolean;
  };
  type ProjectActivationOptions = {
    projects?: ProjectRoot[];
    forceScan?: boolean;
    scanLimit?: number;
    waitForScan?: boolean;
    preserveSelectedRecordOnScan?: boolean;
    clearFileFilter?: boolean;
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
  let selectedProjectRootValidation = $derived(
    projectRootValidationByPath[projectRootValidationKey(selectedProject.path)] ?? null
  );
  let selectedProjectRootValidationSummary = $derived(
    projectRootValidationSummary(selectedProjectRootValidation)
  );
  let selectedProjectGitRootSuggestion = $derived(
    projectRootGitRootSuggestion(selectedProject, selectedProjectRootValidation)
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
  let sourceEditorFileDockviewPanels = $derived(
    projectOpenSourceTabs.map((tab) => ({
      id: sourceEditorFilePanelID(tab),
      title: sourceEditorFileDockviewTitle(tab)
    }))
  );
  let sourceEditorFileDockviewPanelIDs = $derived(
    sourceEditorFileDockviewPanels.map((panel) => panel.id)
  );
  let selectedSourceEditorFilePanelID = $derived(
    selectedRecord ? sourceEditorFilePanelIDFromPath(selectedRecord.path) : sourceEditorFileDockviewPanels[0]?.id ?? null
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
  let selectedSourceMarkdownPreviewAvailable = $derived(sourceMarkdownPreviewAvailable(preview));
  let selectedSourceEditorDisplayMode = $derived(
    preview && selectedSourceMarkdownPreviewAvailable
      ? sourceMarkdownPreviewModeByPath[preview.path] ?? 'source'
      : 'source'
  );
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
  let sourceIndexLoading = $derived(scanning || Boolean(activeSourceScanId));
  let scanSummaryLabel = $derived(
    formatSourceScanSummary(filteredRecords.length, records.length, scanLimitReached, query)
  );
  let sourceScanHealth = $derived(
    formatSourceScanHealth({
      totalCount: records.length,
      filteredCount: filteredRecords.length,
      truncated: scanLimitReached,
      requestedLimit: expandedSourceScanLimit,
      suspiciousThreshold: suspiciousSourceIndexFileThreshold,
      query,
      scanning,
      loading: sourceIndexLoading,
      error
    })
  );
  let sourceScanNeedsAttention = $derived(sourceScanHealth.needsAttention);
  let sourceScanHealthNote = $derived(sourceScanHealth.needsAttention ? sourceScanHealth.summary : '');
  let selectedProjectSourceScanCacheSignature = $derived(sourceScanCacheSignatureForProject(selectedProject));
  let selectedProjectIndexEntry = $derived(
    selectedProjectSourceScanCacheSignature
      ? getSourceScanCacheEntry(
          sourceScanCache,
          selectedProject,
          expandedSourceScanLimit,
          Date.now(),
          sourceScanCacheMaxAgeMs,
          selectedProjectSourceScanCacheSignature
        )
      : null
  );
  let selectedProjectScanMode = $derived(sourceScanModeByProject[selectedProject.id] ?? 'idle');
  let selectedProjectScanEvidence = $derived(
    formatSourceScanEvidence({
      project: selectedProject,
      mode: selectedProjectScanMode,
      entry: selectedProjectIndexEntry,
      requestedLimit: expandedSourceScanLimit,
      scanning,
      loading: sourceIndexLoading,
      error
    })
  );
  let sourceScanRecovery = $derived(
    formatSourceScanRecovery({
      totalCount: records.length,
      filteredCount: filteredRecords.length,
      truncated: scanLimitReached,
      requestedLimit: expandedSourceScanLimit,
      suspiciousThreshold: suspiciousSourceIndexFileThreshold,
      query,
      scanning,
      loading: sourceIndexLoading,
      error,
      stats: sourceScanStats ?? selectedProjectIndexEntry?.stats ?? null
    })
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
  let sourceSidebarIndexStatusLabel = $derived(sourceSidebarIndexStatus());
  let sourceSidebarScanMetaLabel = $derived(sourceSidebarScanMeta());
  let sourceSidebarTreeCountLabel = $derived(sourceSidebarTreeCount());
  let sourceSidebarTreeStatusLabel = $derived(sourceSidebarTreeStatus());
  let sourceSidebarScanTelemetryLabel = $derived(
    sourceSidebarScanTelemetry(sourceScanStats ?? selectedProjectIndexEntry?.stats ?? null)
  );
  let sourceSidebarCompactStatusLabel = $derived(sourceSidebarCompactStatus());
  let sourceRuntimeNotice = $derived(sourceRuntimeNoticeText(runtime, error));
  let sourceSidebarCompactStatusTitle = $derived(sourceSidebarCompactTitle());
  let sourceSidebarScanTelemetryExpanded = $derived(sourceSidebarScanTelemetryShouldExpand());
  let sourceRuntimeNoticeExpanded = $derived(sourceRuntimeNoticeShouldExpand());
  const sourceWorkspacePlan = $derived(
    resolveSourcePaneWorkspacePlan<SourceWorkspacePaneID>({
      viewportSize: sourceWorkspaceViewportSize(),
      minEditorSize: sourceWorkspaceMinimumEditorWidth,
      gapSize: sourceWorkspacePaneGapSize,
      chromeSize: sourceWorkspacePaneChromeSize,
      items: [
        {
          id: 'activity',
          visible: shouldRenderDockPanel('activity'),
          size: sidePaneWidth,
          config: activityPaneSizingConfig,
          collapsePriority: 2,
          previousExpandedSize: sidePaneExpandedWidth
        },
        {
          id: 'context',
          visible: contextPanelPlacement === 'side' && sourceDockPanelVisible('context'),
          size: contextPaneWidth,
          config: contextPaneWidthSizingConfig,
          collapsePriority: 1,
          previousExpandedSize: contextPaneExpandedWidth
        }
      ]
    })
  );
  let effectiveSidePaneWidth = $derived(sourceWorkspacePlanItem('activity')?.size ?? 0);
  let effectiveContextPaneWidth = $derived(
    contextPanelPlacement === 'side'
      ? sourceWorkspacePlanItem('context')?.size ?? 0
      : contextPaneWidth
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
  let pasteCleanupReplyOutput = $derived(cleanupPasteReplyDraft(pasteCleanupReplyDraft));
  let pasteCleanupReplyStats = $derived(
    formatPasteCleanupStats(pasteCleanupReplyDraft, pasteCleanupReplyOutput)
  );
  let visiblePasteCleanupHistory = $derived(
    pasteCleanupHistory.slice(0, maxVisiblePasteCleanupHistoryItems)
  );
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
  let selectedProjectOrchestrationDecisionQueue = $derived(
    orchestrationDecisionQueueForRuns(selectedProjectOrchestrationRuns, 5)
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
        agentSessionProviderLabel(session),
        session.title,
        session.model,
        session.projectPath,
        session.lastActivity,
        agentSessionResumeCommand(session)
      )
    )
  );
  let filteredConversationAgentSessions = $derived(
    agentSessions.filter((session) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        session.provider,
        agentSessionProviderLabel(session),
        session.title,
        session.model,
        session.projectPath,
        session.lastActivity,
        agentSessionResumeCommand(session)
      )
    )
  );
  // Keys (per workspaceSnapshotIDForAgentSession) with a live terminal attached.
  // Empty for now — the live-terminal manager isn't wired yet, so no LIVE dots;
  // a later pass populates this once terminal lifecycle lands.
  const conversationLiveKeys = new Set<string>();
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
        safety.activityLabel,
        ...worktreeWorkspaceSnapshots(worktree).flatMap((snapshot) => [
          snapshot.title,
          snapshot.provider,
          snapshot.sessionID,
          snapshot.model,
          snapshot.resumeCommand
        ])
      );
    })
  );
  let selectedProjectRepositorySummaries = $derived(
    gitRepositorySummaries.filter(
      (summary) =>
        summary.projectID === selectedProject.id ||
        normalizeProjectPath(summary.path) === normalizeProjectPath(selectedProject.path)
    )
  );
  let repositoryDashboardGitGraph = $derived(
    buildGitGraphViewModel({
      repositories: gitRepositorySummaries,
      commits: []
    })
  );
  let selectedProjectGitGraph = $derived(
    buildGitGraphViewModel({
      repositories: selectedProjectRepositorySummaries,
      commits: gitCommitHistory
    })
  );
  let selectedGitCommitRow = $derived(
    selectedProjectGitGraph.commits.find((entry) => entry.sha === selectedGitCommitSha) ??
      selectedProjectGitGraph.commits[0] ??
      null
  );
  let filteredGitRepositoryRows = $derived(
    repositoryDashboardGitGraph.repositories.filter((row) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        row.searchText,
        row.projectName,
        row.repo,
        row.path,
        row.rootLabel,
        row.branchLabel,
        row.dirty.label,
        row.dirty.detailLabel,
        row.sync.label,
        row.sync.detailLabel,
        row.error,
        ...row.taskSearchTargets.map((target) => target.query)
      )
    )
  );
  let filteredGitCommitRows = $derived(
    selectedProjectGitGraph.commits.filter((row) =>
      activityTextMatchesFilter(
        sourceActivityFilter,
        row.searchText,
        row.sha,
        row.shortSha,
        row.subject,
        row.author,
        row.refs.label,
        row.metaLabel,
        row.detailLabel,
        ...row.taskSearchTargets.map((target) => target.query)
      )
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
  let selectedProjectGitGraphSummary = $derived(
    formatGitGraphSummary(
      selectedProjectGitGraph,
      gitRepositorySummariesLoading,
      gitRepositorySummaryError,
      gitRepositorySummarySource,
      gitCommitHistoryLoading,
      gitCommitHistoryError,
      gitCommitHistorySource
    )
  );
  let selectedProjectGitTaskIDs = $derived(
    uniqueGitTaskIDs([
      ...selectedProjectGitGraph.taskIDs,
      ...prioritizedProjectWorktrees.map((worktree) => worktree.taskID),
      ...selectedProjectOrchestrationRuns.map((run) => run.taskID)
    ])
  );
  let selectedProjectGitTaskSearchSummary = $derived(
    formatGitTaskSearchTargetSummary(selectedProjectGitGraph.taskSearchTargets)
  );
  let selectedProjectGitTaskSourceGroups = $derived(
    buildGitTaskSourceGroups(
      selectedProjectGitGraph.repositories.flatMap((row) =>
        row.taskReferences.map((task) => ({
          taskID: task.id,
          sourceLabel: 'repo',
          sourceDetail: `${row.rootLabel} · ${row.branchLabel} · ${row.dirty.label} · ${row.sync.label}`
        }))
      ),
      prioritizedProjectWorktrees.map((worktree) => ({
        taskID: worktree.taskID,
        sourceLabel: 'worktree',
        sourceDetail: `${worktree.branch} · ${projectWorktreeActivityLabel(worktree)}`
      })),
      selectedProjectGitGraph.commits.flatMap((row) =>
        row.taskReferences.map((task) => ({
          taskID: task.id,
          sourceLabel: 'commit',
          sourceDetail: `${row.shortSha} · ${row.subject}`
        }))
      ),
      selectedProjectOrchestrationRuns.map((run) => ({
        taskID: run.taskID,
        sourceLabel: 'run',
        sourceDetail: run.title
      }))
    )
  );
  let selectedProjectGitTaskLedger = $derived(buildGitTaskLedgerRows());
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
  let projectWorktreeDecisionQueue = $derived(
    buildWorktreeDecisionQueue(projectWorktrees, {
      primaryPath: selectedProject.path,
      activeSessionPaths: selectedProjectAgentSessionPaths
    })
  );
  let projectWorktreeCleanupPlans = $derived(
    prioritizedProjectWorktrees.map((worktree) => projectWorktreeCleanupPlan(worktree))
  );
  let projectWorktreeRunbookSafetyInputs = $derived(
    prioritizedProjectWorktrees.map((worktree) => ({
      path: worktree.path,
      repoName: worktree.repo,
      branch: worktree.branch,
      taskID: worktree.taskID,
      safety: projectWorktreeSafety(worktree)
    }))
  );
  let projectWorktreeCleanupRunbook = $derived(
    buildWorktreeCleanupRunbook({
      cleanupPlans: projectWorktreeCleanupPlans,
      decisionQueue: projectWorktreeDecisionQueue,
      safetySummaries: projectWorktreeRunbookSafetyInputs
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
  let conversationSessionSummary = $derived(
    formatConversationSessionSummary(
      agentSessions.length,
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
  let sourceContextCardDockviewPanels = $derived(
    visibleContextCards.map((cardID) => ({
      id: cardID,
      title: contextCardLabels[cardID],
      size: contextCardPaneviewSizes[cardID],
      minimumBodySize: 72,
      isExpanded: true
    }))
  );
  let sourceContextCardDockviewPanelIDs = $derived(
    sourceContextCardDockviewPanels.map((panel) => panel.id)
  );
  let contextPanelPresentation = $derived(
    resolveContextPanelPresentation(contextPanelMode, contextPanelPlacement)
  );
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
  let sourceDefinitionNavCount = $derived(
    sourceDefinitionTargets.length + sourceImplementationTargets.length + sourceTypeDefinitionTargets.length
  );
  let sourceLookupLoading = $derived(
    sourceDefinitionLoading ||
      sourceReferenceLoading ||
      sourceImplementationLoading ||
      sourceTypeDefinitionLoading
  );
  let sourceLookupSummary = $derived(formatSourceLookupSummary());
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
      id: 'kill-playwright-sessions',
      label: 'Kill Playwright sessions',
      detail: playwrightSessions.length
        ? `${playwrightSessions.length} active from ${playwrightSessionSource}`
        : playwrightSessionError || 'No Playwright sessions',
      disabled: playwrightSessions.length === 0 || playwrightSessionsLoading || playwrightSessionsKilling,
      perform: killPlaywrightSessions
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
      id: 'project-validate-root',
      label: 'Validate project root',
      detail: selectedProjectRootValidationSummary,
      disabled: projectRootValidating,
      perform: () => validateProjectRootForProject(selectedProject, true)
    },
    {
      id: 'project-use-git-root',
      label: 'Use detected Git root',
      detail: selectedProjectGitRootSuggestion || selectedProjectRootValidationSummary,
      disabled: !selectedProjectGitRootSuggestion || projectRootValidating || scanning,
      perform: useValidatedGitRootForSelectedProject
    },
    {
      id: 'project-repair-onboarding',
      label: 'Repair project setup',
      detail: `${selectedProject.name} · validate, clear index, rescan`,
      disabled: projectRootValidating || scanning,
      perform: repairSelectedProjectOnboarding
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
        if (snapshot) openConversationWorkspaceSnapshotEmbeddedTerminal(snapshot);
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
      id: 'conversation-copy-active-repair-plan',
      label: 'Copy active workspace repair plan',
      detail: activeWorkspaceSnapshot
        ? workspaceSnapshotRestoreReadiness(activeWorkspaceSnapshot).label
        : 'No active workspace',
      disabled:
        !activeWorkspaceSnapshot ||
        workspaceSnapshotRestoreReadiness(activeWorkspaceSnapshot).kind !== 'missing-worktree',
      perform: () => {
        if (activeWorkspaceSnapshot) copyWorkspaceSnapshotRepairPlan(activeWorkspaceSnapshot);
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
    {
      id: 'conversation-copy-latest-repair-plan',
      label: 'Copy latest workspace repair plan',
      detail: workspaceSnapshots[0]
        ? workspaceSnapshotRestoreReadiness(workspaceSnapshots[0]).label
        : 'No saved workspace',
      disabled:
        !workspaceSnapshots[0] ||
        workspaceSnapshotRestoreReadiness(workspaceSnapshots[0]).kind !== 'missing-worktree',
      perform: () => {
        const snapshot = workspaceSnapshots[0];
        if (snapshot) copyWorkspaceSnapshotRepairPlan(snapshot);
      }
    },
    ...workspaceSnapshots.slice(0, 8).map((snapshot) => ({
      id: `conversation-restore-snapshot-${snapshot.id}`,
      label: `Restore workspace: ${snapshot.title}`,
      detail: workspaceSnapshotScopeLabel(snapshot),
      perform: () => restoreConversationWorkspaceSnapshot(snapshot)
    })),
    ...workspaceSnapshots.slice(0, 8).map((snapshot) => ({
      id: `conversation-resume-snapshot-terminal-${snapshot.id}`,
      label: `Resume workspace in ${snapshot.sourceTerminalApp}: ${snapshot.title}`,
      detail: snapshot.resumeCommand ?? workspaceSnapshotScopeLabel(snapshot),
      disabled: !snapshot.resumeCommand,
      perform: () => openWorkspaceSnapshotTerminal(snapshot)
    })),
    ...workspaceSnapshots.slice(0, 8).map((snapshot) => ({
      id: `conversation-resume-snapshot-embedded-${snapshot.id}`,
      label: `Resume workspace embedded: ${snapshot.title}`,
      detail: snapshot.resumeCommand ?? workspaceSnapshotScopeLabel(snapshot),
      perform: () => openConversationWorkspaceSnapshotEmbeddedTerminal(snapshot)
    })),
    ...workspaceSnapshots.slice(0, 8).map((snapshot) => ({
      id: `conversation-copy-restore-plan-${snapshot.id}`,
      label: `Copy restore plan: ${snapshot.title}`,
      detail: workspaceSnapshotScopeLabel(snapshot),
      perform: () => copyWorkspaceSnapshotRestorePlan(snapshot)
    })),
    ...workspaceSnapshots.slice(0, 8).map((snapshot) => ({
      id: `conversation-copy-repair-plan-${snapshot.id}`,
      label: `Copy repair plan: ${snapshot.title}`,
      detail: workspaceSnapshotRestoreReadiness(snapshot).detail,
      disabled: workspaceSnapshotRestoreReadiness(snapshot).kind !== 'missing-worktree',
      perform: () => copyWorkspaceSnapshotRepairPlan(snapshot)
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
      id: 'trigger-completions',
      label: 'Trigger completions',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('completion')
    },
    {
      id: 'show-signature-help',
      label: 'Show signature help',
      detail: preview?.fileName ?? 'No file',
      disabled: !preview || loading || !sourceIntelligenceAvailable,
      perform: () => requestSourceIntelligenceAction('signature-help')
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
      id: 'editor-local-problems',
      label: 'Show local problems drawer',
      detail: sourceDiagnosticSummary,
      perform: () => openEditorNavPanel('problems')
    },
    {
      id: 'editor-local-symbols',
      label: 'Show local symbols drawer',
      detail: `${sourceSymbols.length} symbols`,
      perform: () => openEditorNavPanel('symbols')
    },
    {
      id: 'editor-local-definitions',
      label: 'Show local definitions drawer',
      detail: sourceLookupSummary,
      perform: () => openEditorNavPanel('definitions')
    },
    {
      id: 'editor-local-references',
      label: 'Show local references drawer',
      detail: sourceReferenceSummary || 'No references yet',
      perform: () => openEditorNavPanel('references')
    },
    {
      id: 'clear-source-lookups',
      label: 'Clear source lookup results',
      detail: sourceLookupSummary,
      disabled:
        sourceDefinitionNavCount === 0 &&
        sourceReferenceTargets.length === 0 &&
        !sourceDefinitionQuery &&
        !sourceReferenceQuery &&
        !sourceImplementationQuery &&
        !sourceTypeDefinitionQuery,
      perform: clearSourceLookupResults
    },
    {
      id: 'source-copy-intelligence-brief',
      label: 'Copy source intelligence brief',
      detail: preview?.fileName ?? selectedProject.name,
      disabled: !preview,
      perform: copySourceIntelligenceBrief
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
      id: 'lsp-copy-readiness',
      label: 'Copy language server readiness report',
      detail: selectedProject.name,
      perform: copySourceLspReadinessReport
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
      id: 'layout-focus-editor',
      label: 'Focus editor canvas',
      detail: 'Hide context, insights, terminal, and browser',
      perform: focusSourceEditorLayout
    },
    {
      id: 'layout-activity-rail',
      label: 'Collapse explorer to icon rail',
      detail: 'Activity pane',
      disabled: activityPaneRailOnly(),
      perform: collapseActivityPaneToRail
    },
    {
      id: 'layout-context-rail',
      label: 'Collapse context to icon rail',
      detail: 'Context pane',
      disabled: contextPaneRailOnly(),
      perform: collapseContextPaneToRail
    },
    {
      id: 'layout-restore-before-focus',
      label: 'Restore layout before focus',
      detail: sourceFocusRestoreLayout ? 'Return to the previous pane arrangement' : 'No focus restore point',
      disabled: !sourceFocusRestoreLayout,
      perform: restoreSourceLayoutBeforeFocus
    },
    {
      id: 'layout-toggle-chrome',
      label: sourceChromeCompact ? 'Use comfortable editor chrome' : 'Use compact editor chrome',
      detail: sourceChromeCompact ? 'Show source context strip' : 'Tight title row',
      perform: toggleSourceChromeCompact
    },
    {
      id: 'layout-copy-diagnostic',
      label: 'Copy layout diagnostic',
      detail: `${sourceLayoutPreset} layout · ${sourceActivityLabel(sourceActivityMode)}`,
      perform: copySourceLayoutDiagnostic
    },
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
      id: 'terminal-open-or-attach-project',
      label: 'Open or attach project terminal',
      detail: selectedProject.path,
      disabled: !selectedProject.path || embeddedTerminalStarting,
      perform: attachOrStartProjectEmbeddedTerminal
    },
    {
      id: 'terminal-run-draft',
      label: 'Run terminal command draft',
      detail: embeddedTerminalCommandDraft || embeddedTerminalStatusLabel(),
      disabled:
        !embeddedTerminalCommandDraft.trim() ||
        embeddedTerminalStarting ||
        (!embeddedTerminalSession && !selectedProject.path),
      perform: () => submitEmbeddedTerminalCommand()
    },
    {
      id: 'terminal-clear-draft',
      label: 'Clear terminal command draft',
      detail: embeddedTerminalCommandDraft || 'No draft command',
      disabled: !embeddedTerminalCommandDraft.trim(),
      perform: () => {
        embeddedTerminalCommandDraft = '';
        embeddedTerminalStatus = 'Terminal command draft cleared';
      }
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
    {
      id: 'terminal-copy-active-focus-plan',
      label: 'Copy active terminal focus plan',
      detail: embeddedTerminalSession
        ? embeddedTerminalSessionWorkspaceLabel(embeddedTerminalSession)
        : 'No active embedded terminal',
      disabled: !embeddedTerminalSession,
      perform: () => {
        if (embeddedTerminalSession) copyEmbeddedTerminalSessionFocusPlan(embeddedTerminalSession);
      }
    },
    {
      id: 'terminal-restore-active-workspace',
      label: 'Restore active terminal workspace',
      detail: embeddedTerminalSession
        ? embeddedTerminalSessionWorkspaceLabel(embeddedTerminalSession)
        : 'No active embedded terminal',
      disabled: !embeddedTerminalSession,
      perform: () => {
        if (embeddedTerminalSession) restoreEmbeddedTerminalSessionWorkspace(embeddedTerminalSession);
      }
    },
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `terminal-embedded-resume-${session.provider}-${session.id}`,
      label: `Resume in embedded terminal: ${session.title}`,
      detail: agentSessionProjectLabel(session),
      disabled: !agentSessionTerminalCommand(session).trim(),
      perform: () => resumeAgentSessionEmbeddedTerminal(session)
    })),
    ...embeddedTerminalSessions.slice(0, 8).map((session) => ({
      id: `terminal-copy-focus-plan-${session.sessionId}`,
      label: `Copy terminal focus plan: ${embeddedTerminalSessionTitle(session)}`,
      detail: embeddedTerminalSessionWorkspaceLabel(session),
      perform: () => copyEmbeddedTerminalSessionFocusPlan(session)
    })),
    ...embeddedTerminalSessions.slice(0, 8).map((session) => ({
      id: `terminal-restore-workspace-${session.sessionId}`,
      label: `Restore terminal workspace: ${embeddedTerminalSessionTitle(session)}`,
      detail: embeddedTerminalSessionWorkspaceLabel(session),
      perform: () => restoreEmbeddedTerminalSessionWorkspace(session)
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
      id: 'paste-read-clipboard',
      label: 'Read clipboard for cleanup',
      detail: pasteCleanupMode,
      disabled: fileActionBusy === 'paste-read',
      perform: readPasteCleanupClipboard
    },
    {
      id: 'paste-copy-cleaned',
      label: 'Copy cleaned paste',
      detail: pasteCleanupStats,
      disabled: pasteCleanupOutput.trim().length === 0 || fileActionBusy === 'paste-copy',
      perform: copyPasteCleanupOutput
    },
    {
      id: 'paste-copy-reply-draft',
      label: 'Copy paste reply draft',
      detail: pasteCleanupReplyStats,
      disabled: pasteCleanupReplyOutput.trim().length === 0 || fileActionBusy === 'paste-reply-copy',
      perform: copyPasteCleanupReplyDraft
    },
    {
      id: 'paste-clear-input',
      label: 'Clear paste cleanup input',
      detail: pasteCleanupStats,
      disabled: pasteCleanupInput.length === 0,
      perform: clearPasteCleanupInput
    },
    {
      id: 'paste-clear-reply-draft',
      label: 'Clear paste reply draft',
      detail: pasteCleanupReplyStats,
      disabled: pasteCleanupReplyDraft.length === 0,
      perform: clearPasteCleanupReplyDraft
    },
    {
      id: 'paste-clear-history',
      label: 'Clear paste cleanup history',
      detail: `${pasteCleanupHistory.length} saved`,
      disabled: pasteCleanupHistory.length === 0,
      perform: clearPasteCleanupHistory
    },
    ...pasteCleanupHistory.slice(0, 8).map((item) => ({
      id: `paste-restore-history-${item.id}`,
      label: `Restore ${pasteCleanupHistoryKindLabel(item.kind)} history`,
      detail: item.summary,
      perform: () => restorePasteCleanupHistoryItem(item)
    })),
    ...pasteCleanupHistory.slice(0, 8).map((item) => ({
      id: `paste-copy-history-${item.id}`,
      label: `Copy ${pasteCleanupHistoryKindLabel(item.kind)} history`,
      detail: item.summary,
      disabled: fileActionBusy === 'activity-copy',
      perform: () => copyPasteCleanupHistoryItem(item)
    })),
    ...pasteCleanupModes.map((mode) => ({
      id: `paste-mode-${mode}`,
      label: `Use ${mode} paste cleanup`,
      detail: pasteCleanupMode === mode ? 'Current cleanup mode' : 'Clipboard cleanup mode',
      disabled: pasteCleanupMode === mode,
      perform: () => setPasteCleanupMode(mode)
    })),
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
    {
      id: 'orchestration-copy-run-started-command',
      label: 'Copy run-started event command',
      detail: orchestrationEventCommandDetail(),
      perform: () => copyOrchestrationEventCommand('run-started', 'Run started')
    },
    {
      id: 'orchestration-copy-scenario-command',
      label: 'Copy scenario-started event command',
      detail: orchestrationEventCommandDetail(),
      perform: () =>
        copyOrchestrationEventCommand('scenario-started', 'Scenario started', {
          '--scenario': 'Scenario name',
          '--scenario-count': 1
        })
    },
    {
      id: 'orchestration-copy-issue-command',
      label: 'Copy issue-found event command',
      detail: orchestrationEventCommandDetail(),
      perform: () =>
        copyOrchestrationEventCommand('issue-found', 'Issue found', {
          '--issue-id': 'ISSUE-1',
          '--message': 'Short failure summary',
          '--issue-count': 1,
          '--failed-count': 1
        })
    },
    {
      id: 'orchestration-copy-batch-command',
      label: 'Copy fix-batch event command',
      detail: orchestrationEventCommandDetail(),
      perform: () =>
        copyOrchestrationEventCommand('batch-delegated', 'Fix batch', {
          '--agent-role': 'fix-agent',
          '--issue-count': 1,
          '--fix-count': 1,
          '--delegated-count': 1
        })
    },
    {
      id: 'orchestration-copy-ui-verified-command',
      label: 'Copy UI-verified event command',
      detail: orchestrationEventCommandDetail(),
      perform: () =>
        copyOrchestrationEventCommand('ui-verified', 'UI verified', {
          '--scenario': 'Scenario name',
          '--resolved-count': 1,
          '--verified-count': 1
        })
    },
    {
      id: 'orchestration-copy-approval-command',
      label: 'Copy approval-required event command',
      detail: orchestrationEventCommandDetail(),
      perform: () =>
        copyOrchestrationEventCommand('approval-required', 'Approval required', {
          '--message': 'Needs sign-off before continuing',
          '--approval-count': 1
        })
    },
    {
      id: 'orchestration-copy-e2e-loop-sample-command',
      label: 'Copy E2E loop sample command',
      detail: orchestrationEventCommandDetail(),
      perform: () => copyOrchestrationSampleCommand('run-e2e-loop', 'E2E loop sample')
    },
    {
      id: 'orchestration-record-heartbeat',
      label: 'Record orchestration heartbeat',
      detail: orchestrationEventCommandDetail(),
      disabled: orchestrationEventRecording,
      perform: () => recordNativeOrchestrationEvent('run-updated', 'Run heartbeat')
    },
    {
      id: 'orchestration-choose-event-file',
      label: 'Choose orchestration event file',
      detail: orchestrationJsonFileImportPath(),
      disabled: orchestrationEventFileChoosing,
      perform: chooseOrchestrationEventFile
    },
    {
      id: 'orchestration-copy-json-file-import-command',
      label: 'Copy orchestration JSON import command',
      detail: orchestrationJsonFileImportPath(),
      perform: () => copyOrchestrationJsonFileImportCommand()
    },
    ...selectedProjectOrchestrationDecisionQueue.slice(0, 8).map((item) => ({
      id: `run-decision-${item.id}`,
      label: `Focus decision: ${item.title}`,
      detail: `${item.runTitle}${item.taskID ? ` · ${item.taskID}` : ''}`,
      perform: () => {
        const run = selectedProjectOrchestrationRuns.find((entry) => entry.id === item.runID);
        if (run) focusOrchestrationRun(run);
      }
    })),
    ...selectedProjectOrchestrationDecisionQueue.slice(0, 8).map((item) => ({
      id: `run-copy-decision-${item.id}`,
      label: `Copy decision: ${item.title}`,
      detail: `${item.runTitle}${item.summary ? ` · ${item.summary}` : ''}`,
      perform: () =>
        copyActivityCommand(
          `${item.label} · ${item.runTitle} · ${item.title}${item.summary ? ` - ${item.summary}` : ''}`,
          'Run decision copied'
        )
    })),
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
      id: 'worktree-cleanup-runbook',
      label: 'Copy worktree cleanup runbook',
      detail: projectWorktreeCleanupRunbook.headline,
      disabled: projectWorktrees.length === 0,
      perform: copyProjectWorktreeCleanupRunbook
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
      id: 'git-open-selected-commit-task',
      label: 'Open selected commit task',
      detail: selectedGitCommitRow?.taskID
        ? (gitTaskUrl(selectedGitCommitRow.taskID) ?? selectedGitCommitRow.taskID)
        : 'No selected task',
      disabled: !selectedGitCommitRow?.taskID || !gitTaskUrl(selectedGitCommitRow.taskID),
      perform: () => {
        if (selectedGitCommitRow?.taskID) openGitTaskReference(selectedGitCommitRow.taskID);
      }
    },
    {
      id: 'git-copy-workspace-brief',
      label: 'Copy Git workspace brief',
      detail: `${selectedProject.name} · ${selectedProjectGitGraphSummary} · ${projectWorktreeCleanupBrief.headline}`,
      disabled: !selectedProject.path,
      perform: copyGitWorkspaceBrief
    },
    ...selectedProjectGitTaskIDs.slice(0, 8).map((taskID) => ({
      id: `git-copy-task-${taskID}`,
      label: `Copy task link: ${taskID}`,
      detail: gitTaskUrl(taskID) ?? 'Task ID only',
      perform: () => copyGitTaskReference(taskID)
    })),
    ...selectedProjectGitTaskIDs.slice(0, 8).map((taskID) => ({
      id: `git-open-task-${taskID}`,
      label: `Open task: ${taskID}`,
      detail: gitTaskUrl(taskID) ?? 'Task ID only',
      disabled: !gitTaskUrl(taskID),
      perform: () => openGitTaskReference(taskID)
    })),
    ...selectedProjectGitGraph.taskSearchTargets.slice(0, 8).map((target) => ({
      id: `git-copy-task-search-${target.id}`,
      label: `Copy task search: ${target.label}`,
      detail: target.query,
      perform: () => copyActivityCommand(target.query, 'Task search copied')
    })),
    ...selectedProjectGitTaskSourceGroups.slice(0, 8).map((group) => ({
      id: `git-copy-task-sources-${group.taskID}`,
      label: `Copy task sources: ${group.taskID}`,
      detail: group.sourceSummary,
      perform: () => copyGitTaskSourceGroup(group)
    })),
    ...selectedProjectGitTaskLedger.slice(0, 8).map((row) => ({
      id: `git-task-ledger-${row.taskID}`,
      label: `Copy task ledger: ${row.taskID}`,
      detail: row.nextAction,
      perform: () => copyGitTaskLedger(row)
    })),
    ...selectedProjectGitTaskLedger.slice(0, 8).map((row) => ({
      id: `git-focus-task-ledger-${row.taskID}`,
      label: `Focus task ledger: ${row.taskID}`,
      detail: row.nextAction,
      perform: () => focusGitTaskLedger(row.taskID)
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
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => {
      const commandBlock = projectWorktreeRunbookCommandBlock(worktree);
      return {
        id: `worktree-runbook-command-block-${worktree.path}`,
        label: `Copy worktree runbook command block: ${worktree.branch}`,
        detail: commandBlock?.title ?? projectWorktreeSafety(worktree).recommendation,
        disabled: false,
        perform: () => copyWorktreeRunbookCommandBlock(worktree)
      };
    }),
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
    ...prioritizedProjectWorktrees.slice(0, 8).map((worktree) => {
      const snapshot = latestWorktreeWorkspaceSnapshot(worktree);
      return {
        id: `worktree-restore-snapshot-${worktree.path}`,
        label: `Restore saved workspace: ${worktree.branch}`,
        detail: snapshot ? worktreeWorkspaceSnapshotLabel(worktree) : 'No saved workspace for this worktree',
        disabled: !snapshot,
        perform: () => {
          if (snapshot) return restoreConversationWorkspaceSnapshot(snapshot);
        }
      };
    }),
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
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => {
      const snapshot = workspaceSnapshotForAgentSession(session);
      const readiness = snapshot ? workspaceSnapshotRestoreReadiness(snapshot) : null;
      return {
        id: `conversation-copy-session-repair-plan-${session.provider}-${session.id}`,
        label: `Copy workspace repair plan: ${session.title}`,
        detail: readiness?.detail ?? 'No saved workspace',
        disabled: readiness?.kind !== 'missing-worktree',
        perform: () => copyAgentSessionWorkspaceRepairPlan(session)
      };
    }),
    ...selectedProjectAgentSessions.slice(0, 8).map((session) => ({
      id: `agent-copy-plan-${session.provider}-${session.id}`,
      label: `Copy session focus plan: ${session.title}`,
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

  // Apply terminal font settings live to an already-running embedded terminal.
  // Reading the derived fields registers them as dependencies. We only touch the
  // terminal when a value actually differs from its current options, so the
  // first run after mount (values already match the constructor) is a no-op and
  // never triggers a spurious refit/PTY resize.
  $effect(() => {
    const { fontFamily, fontSize, lineHeight } = embeddedTerminalAppearance;
    const terminal = embeddedTerminal;
    if (!terminal) return;

    let changed = false;
    if (terminal.options.fontFamily !== fontFamily) {
      terminal.options.fontFamily = fontFamily;
      changed = true;
    }
    if (terminal.options.fontSize !== fontSize) {
      terminal.options.fontSize = fontSize;
      changed = true;
    }
    if (terminal.options.lineHeight !== lineHeight) {
      terminal.options.lineHeight = lineHeight;
      changed = true;
    }

    if (changed) {
      // Re-measure cell size and resize the PTY to match the new font metrics.
      fitEmbeddedTerminal();
    }
  });

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
    const element = sourceWorkspaceElement;
    if (!element) return;

    measureSourceWorkspaceSize();

    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', measureSourceWorkspaceSize);
      return () => window.removeEventListener('resize', measureSourceWorkspaceSize);
    }

    let resizeFrame = 0;
    const resizeObserver = new ResizeObserver(() => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = 0;
        measureSourceWorkspaceSize();
      });
    });
    resizeObserver.observe(element);
    window.addEventListener('resize', measureSourceWorkspaceSize);
    return () => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeObserver.disconnect();
      window.removeEventListener('resize', measureSourceWorkspaceSize);
    };
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

    void ensureEmbeddedTerminalRenderer().then(scheduleEmbeddedTerminalFit);

    if (typeof ResizeObserver === 'undefined') return;

    let resizeFrame = 0;
    const resizeObserver = new ResizeObserver(() => {
      if (resizeFrame) window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = 0;
        scheduleEmbeddedTerminalFit();
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
    const sourceSignature = sourceScanCacheSignatureForProject(project);
    const cachedScan =
      options.force || !sourceSignature
        ? null
        : getSourceScanCacheEntry(
            sourceScanCache,
            project,
            scanLimit,
            Date.now(),
            sourceScanCacheMaxAgeMs,
            sourceSignature
          );

    const cachedScanNeedsRepair =
      cachedScan !== null &&
      sourceScanCacheEntryNeedsRepair(cachedScan, scanLimit, suspiciousSourceIndexFileThreshold);

    if (cachedScanNeedsRepair) {
      sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
      persistSourceScanCache(sourceScanCache);
      setSourceScanMode(project.id, 'repair');
      fileActionStatus = `Cached index for ${project.name} only had ${cachedScan.records.length.toLocaleString()} files. Rebuilding the project index.`;
    } else if (cachedScan) {
      activeSourceScanId = '';
      sourceScanProgress = null;
      scanning = false;
      loading = true;
      error = '';
      runtime = 'cached source scan';
      setSourceScanMode(project.id, 'cache');
      clearBackgroundIndexError(project.id);

      const nextSelection = applySourceRecords(
        cachedScan.records,
        preferredPath,
        'cached source scan',
        cachedScan.truncated,
        cachedScan.stats ?? null,
        { preserveSelectedRecord: options.preserveSelectedRecord }
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
    setSourceScanMode(project.id, 'native');
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
        persistSourceScanCache(sourceScanCache);
        setSourceScanMode(project.id, 'repair');
        fileActionStatus = `Only ${nextRecords.length.toLocaleString()} files indexed for ${project.name}. Rebuilding the project index.`;
        await scanProject(project, preferredPath, {
          force: true,
          limit: Math.max(scanLimit, expandedSourceScanLimit),
          skipTinyIndexRepair: true,
          preserveSelectedRecord: options.preserveSelectedRecord
        });
        return;
      }

      const shouldCacheScanResult = !(options.skipTinyIndexRepair && suspiciousScanResult);
      const nextSourceSignature = sourceScanCacheSignatureForProject(project);
      if (shouldCacheScanResult && nextSourceSignature) {
        sourceScanCache = upsertSourceScanCacheEntry(
          sourceScanCache,
          project,
          nextRecords,
          tauriScan.limit,
          Date.now(),
          maxSourceScanCacheEntries,
          tauriScan.truncated,
          tauriScan.stats,
          nextSourceSignature
        );
        persistSourceScanCache(sourceScanCache);
      } else if (!shouldCacheScanResult) {
        sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
        persistSourceScanCache(sourceScanCache);
      }
      clearBackgroundIndexError(project.id);

      if (options.skipTinyIndexRepair && suspiciousScanResult) {
        setSourceScanMode(project.id, 'tiny');
        fileActionStatus = `Only ${nextRecords.length.toLocaleString()} files indexed for ${project.name}. Check the project root or reset the index.`;
      }

      const nextSelection = applySourceRecords(
        nextRecords,
        preferredPath,
        options.skipTinyIndexRepair && suspiciousScanResult ? 'tiny source scan' : 'local source scan',
        tauriScan.truncated,
        tauriScan.stats ?? null,
        { preserveSelectedRecord: options.preserveSelectedRecord }
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
      setSourceScanMode(project.id, 'failed');
      loading = false;
    } finally {
      if (generation === scanGeneration) {
        scanning = false;
        activeSourceScanId = '';
        sourceScanProgress = null;
        if (!selectedRecord) loading = false;
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
    setSourceScanMode(selectedProject.id, 'stopped');
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
    persistSourceScanCache(sourceScanCache);
    fileActionStatus = `Index reset for ${project.name}`;
    return scanProject(project, selectedSourcePaths[project.id], { force: true, limit });
  }

  function resetProjectOnboardingScanState(project: ProjectRoot) {
    sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
    persistSourceScanCache(sourceScanCache);
    clearBackgroundIndexError(project.id);
    const nextScanModes = { ...sourceScanModeByProject };
    delete nextScanModes[project.id];
    sourceScanModeByProject = nextScanModes;

    const nextSelectedSourcePaths = { ...selectedSourcePaths };
    delete nextSelectedSourcePaths[project.id];
    selectedSourcePaths = nextSelectedSourcePaths;
    persistSelectedSourcePaths(nextSelectedSourcePaths);

    clearSourceRecordsForIncomingProject(project, true);
  }

  function sourceOnboardingScanStatus(project: ProjectRoot) {
    return `Scanning ${project.name} up to ${expandedSourceScanLimit.toLocaleString()} source files`;
  }

  function projectSetupNoticeText() {
    const scanCap = `${expandedSourceScanLimit.toLocaleString()} file scan cap`;
    if (projectRootValidating) return `Checking project root · ${scanCap}`;

    if (selectedProjectGitRootSuggestion) {
      return `Inside Git repo · detected root ${formatSourceContextRootLabel(selectedProjectGitRootSuggestion)} · ${scanCap}`;
    }

    const rootLabel = selectedProjectRootValidation
      ? projectRootValidationSummary(selectedProjectRootValidation)
      : 'Root not checked';
    const scanLabel = scanning && sourceScanProgress
      ? `Scanning ${sourceScanProgress.matchedFiles.toLocaleString()} files / ${sourceScanProgress.visitedEntries.toLocaleString()} entries`
      : selectedProjectScanEvidence.label;

    return [rootLabel, scanLabel, scanCap].filter(Boolean).join(' · ');
  }

  function projectSetupNoticeTitle() {
    const validation = selectedProjectRootValidation;
    return [
      `Project: ${selectedProject.name}`,
      `Path: ${selectedProject.path}`,
      validation ? `Validation: ${validation.message}` : 'Validation: not checked',
      selectedProjectGitRootSuggestion ? `Detected Git root: ${selectedProjectGitRootSuggestion}` : '',
      `Scan mode: ${selectedProjectScanEvidence.label}`,
      `Scan limit: ${expandedSourceScanLimit.toLocaleString()} files`
    ].filter(Boolean).join('\n');
  }

  function sourceSidebarIndexStatus() {
    const indexError = backgroundIndexErrorByProject[selectedProject.id]?.trim() ?? '';
    const normalizedQuery = query.trim();

    if (error.trim() || indexError) return 'Scan needs attention';
    if (sourceScanNeedsAttention) return sourceScanHealth.summary;
    if (scanning) return 'Scanning source files';
    if (backgroundIndexingProjectIDs.has(selectedProject.id)) return 'Indexing in background';
    if (normalizedQuery.length > 0) return 'File filter active';
    if (selectedProjectIndexEntry) return selectedProjectIndexEntry.truncated ? 'Index ready · limited' : 'Index ready';
    if (sourceIndexLoading) return 'Loading index';
    return 'Index not ready';
  }

  function sourceSidebarScanMeta() {
    const parts = [];
    const normalizedQuery = query.trim();

    if (normalizedQuery.length > 0) {
      parts.push(`Filter: ${normalizedQuery}`);
    }

    if (scanning) {
      parts.push('Native scan');
    } else {
      switch (selectedProjectScanMode) {
        case 'cache':
          parts.push('Cached');
          break;
        case 'native':
          parts.push('Fresh');
          break;
        case 'background':
          parts.push('Background');
          break;
        case 'repair':
          parts.push('Rebuilding');
          break;
        case 'tiny':
          parts.push('Tiny index');
          break;
        case 'failed':
          parts.push('Failed');
          break;
        case 'stopped':
          parts.push('Stopped');
          break;
        default:
          parts.push(selectedProjectIndexEntry ? 'Indexed' : 'Idle');
          break;
      }
    }

    if (selectedProjectIndexEntry) {
      parts.push(formatRelativeAge(selectedProjectIndexEntry.scannedAt));
    }

    parts.push(
      scanLimitReached || selectedProjectIndexEntry?.truncated
        ? `${expandedSourceScanLimit.toLocaleString()} cap reached`
        : `${expandedSourceScanLimit.toLocaleString()} cap`
    );

    return parts.filter(Boolean).join(' · ');
  }

  function sourceSidebarCompactStatus() {
    const visibleCount = filteredRecords.length;
    const totalCount = records.length;
    const countLabel =
      visibleCount === totalCount
        ? `${totalCount.toLocaleString()} files`
        : `${visibleCount.toLocaleString()} / ${totalCount.toLocaleString()} files`;
    const parts = [countLabel, sourceSidebarScanMetaLabel];

    return parts.filter(Boolean).join(' · ');
  }

  function sourceSidebarTreeCount() {
    const visibleCount = filteredRecords.length;
    const totalCount = records.length;
    if (visibleCount === totalCount) return compactCountValue(totalCount);
    return `${compactCountValue(visibleCount)} / ${compactCountValue(totalCount)}`;
  }

  function sourceSidebarTreeStatus() {
    const parts = [sourceSidebarIndexStatusLabel, sourceSidebarScanMetaLabel];
    return parts.filter(Boolean).join(' · ');
  }

  function sourceSidebarCompactTitle() {
    return [
      selectedProjectIndexSummary,
      sourceSidebarCompactStatusLabel,
      sourceSidebarScanTelemetryLabel,
      sourceRuntimeNotice
    ]
      .filter(Boolean)
      .join('\n');
  }

  function sourceSidebarScanTelemetryShouldExpand() {
    return Boolean(
      sourceSidebarScanTelemetryLabel &&
        (sourceScanNeedsAttention ||
          scanLimitReached ||
          selectedProjectScanEvidence.tone === 'warning' ||
          selectedProjectScanEvidence.tone === 'error')
    );
  }

  function sourceRuntimeNoticeShouldExpand() {
    return Boolean(sourceRuntimeNotice && error.trim());
  }

  function sourceSidebarScanTelemetry(stats: SourceScanStats | null | undefined) {
    if (!stats) return '';

    const visitedEntries = sourceScanStatNumber(stats, 'visitedEntries', 'visitedEntryCount');
    const skippedDirectories = sourceScanStatNumber(
      stats,
      'skippedDirectories',
      'skippedDirectoryCount'
    );
    const unsupportedFiles = sourceScanStatNumber(stats, 'unsupportedFiles', 'unsupportedFileCount');
    const unreadableEntries = sourceScanStatNumber(stats, 'unreadableEntries', 'unreadableEntryCount');
    const collectionLimit = sourceScanOptionalStatNumber(stats.collectionLimit);
    const parts = [];

    if (visitedEntries > 0) {
      parts.push(`${visitedEntries.toLocaleString()} entries checked`);
    }
    if (skippedDirectories > 0) {
      parts.push(`${skippedDirectories.toLocaleString()} ${skippedDirectories === 1 ? 'dir' : 'dirs'} skipped`);
    }
    if (unsupportedFiles > 0) {
      parts.push(`${unsupportedFiles.toLocaleString()} unsupported`);
    }
    if (unreadableEntries > 0) {
      parts.push(`${unreadableEntries.toLocaleString()} unreadable`);
    }
    if (collectionLimit !== null && stats.collectionLimitReached) {
      parts.push(`${collectionLimit.toLocaleString()} collection cap reached`);
    }

    return parts.join(' · ');
  }

  function sourceScanStatNumber(
    stats: SourceScanStats,
    primaryKey: keyof SourceScanStats,
    fallbackKey: keyof SourceScanStats
  ) {
    const primaryValue = stats[primaryKey];
    if (typeof primaryValue === 'number' && Number.isFinite(primaryValue)) {
      return Math.max(0, Math.floor(primaryValue));
    }

    const fallbackValue = stats[fallbackKey];
    if (typeof fallbackValue === 'number' && Number.isFinite(fallbackValue)) {
      return Math.max(0, Math.floor(fallbackValue));
    }

    return 0;
  }

  function sourceScanOptionalStatNumber(value: number | null | undefined) {
    return typeof value === 'number' && Number.isFinite(value) ? Math.max(0, Math.floor(value)) : null;
  }

  async function indexProjectsInBackground(projects: ProjectRoot[]) {
    const projectsToIndex = selectBackgroundIndexProjects(
      projects,
      selectedProject.id,
      sourceScanCache,
      Date.now(),
      sourceScanCacheMaxAgeMs,
      expandedSourceScanLimit,
      suspiciousSourceIndexFileThreshold,
      sourceScanCacheSignatureForProject
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
      if (
        isSuspiciousSourceScanResult(
          nextRecords.length,
          tauriScan.truncated,
          tauriScan.limit,
          suspiciousSourceIndexFileThreshold
        )
      ) {
        sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
        persistSourceScanCache(sourceScanCache);
        setSourceScanMode(project.id, 'tiny');
        backgroundIndexErrorByProject = {
          ...backgroundIndexErrorByProject,
          [project.id]: `Only ${nextRecords.length.toLocaleString()} files indexed. Open the project to repair the index.`
        };
        return;
      }

      const sourceSignature = sourceScanCacheSignatureForProject(project);
      if (sourceSignature) {
        sourceScanCache = upsertSourceScanCacheEntry(
          sourceScanCache,
          project,
          nextRecords,
          tauriScan.limit,
          Date.now(),
          maxSourceScanCacheEntries,
          tauriScan.truncated,
          tauriScan.stats,
          sourceSignature
        );
        persistSourceScanCache(sourceScanCache);
      }
      setSourceScanMode(project.id, 'background');
      clearBackgroundIndexError(project.id);
    } catch (indexError) {
      setSourceScanMode(project.id, 'failed');
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
    const requestID = ++projectGitStatusRequestID;
    projectGitLoading = true;
    projectGitStatus = null;
    projectGitError = '';

    try {
      const nextStatus = await readProjectGitStatusFromTauri(project.path);
      if (!isCurrentProjectGitStatusRequest(requestID, projectID)) return;

      projectGitStatus = nextStatus;
      projectGitError = nextStatus ? '' : 'Native Git unavailable';
    } catch (gitError) {
      if (!isCurrentProjectGitStatusRequest(requestID, projectID)) return;

      projectGitStatus = null;
      projectGitError = gitError instanceof Error ? gitError.message : 'Could not read Git status';
    } finally {
      if (isCurrentProjectGitStatusRequest(requestID, projectID)) {
        projectGitLoading = false;
      }
    }
  }

  function isCurrentProjectGitStatusRequest(requestID: number, projectID: string) {
    return requestID === projectGitStatusRequestID && selectedProjectID === projectID;
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

  function sourceScanCacheSignatureForProject(project: ProjectRoot): string | null {
    const projectPath = normalizeProjectPath(project.path);
    const summary =
      gitRepositorySummaries.find(
        (candidate) =>
          candidate.projectID === project.id ||
          normalizeProjectPath(candidate.path) === projectPath
      ) ?? null;

    if (summary && !summary.error) {
      return [
        sourceScanCacheSignatureVersion,
        'git',
        projectPath,
        normalizeProjectPath(summary.path),
        summary.branch || 'detached',
        summary.lastCommitSha || 'no-head',
        summary.ahead,
        summary.behind,
        summary.stagedCount,
        summary.unstagedCount,
        summary.untrackedCount,
        summary.dirtyCount,
        summary.dirtySinceEpochMs ?? 0,
        summary.dirtyStatusFingerprint
      ].join('|');
    }

    if (gitRepositorySummariesLoading || gitRepositorySummaries.length === 0) {
      return null;
    }

    return [sourceScanCacheSignatureVersion, 'path', projectPath].join('|');
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

  async function loadOrchestrationRuns(
    projects: ProjectRoot[] = projectOptions,
    options: { background?: boolean } = {}
  ) {
    const background = options.background === true;
    if (background && orchestrationRunsRefreshInFlight) return;

    orchestrationRunsRefreshInFlight = true;
    if (!background) {
      orchestrationRunsLoading = true;
      orchestrationRunError = '';
    }

    try {
      const nativeRuns = await listOrchestrationRunsFromTauri(projects);
      if (nativeRuns) {
        orchestrationRuns = nativeRuns;
        orchestrationRunSource = 'native event store';
        return;
      }

      if (background) return;
      orchestrationRuns = demoOrchestrationRunsForProjects(projects);
      orchestrationRunSource = 'browser preview';
    } catch (runError) {
      if (background) {
        orchestrationRunError =
          runError instanceof Error ? runError.message : 'Could not read orchestration runs';
        return;
      }
      orchestrationRuns = demoOrchestrationRunsForProjects(projects);
      orchestrationRunSource = 'browser preview';
      orchestrationRunError =
        runError instanceof Error ? runError.message : 'Could not read orchestration runs';
    } finally {
      orchestrationRunsRefreshInFlight = false;
      if (!background) {
        orchestrationRunsLoading = false;
      }
    }
  }

  async function loadAgentSessions() {
    agentSessionsLoading = true;
    agentSessionError = '';

    let sessionScanError = '';
    try {
      const nativeSessions = await listAgentSessionsFromTauri();
      if (nativeSessions) {
        agentSessions = nativeSessions;
        agentSessionSource = 'native session scan';
        agentSessionsLoading = false;
        return;
      }
    } catch (sessionError) {
      sessionScanError =
        sessionError instanceof Error ? sessionError.message : 'Could not scan agent sessions';
    }

    try {
      const bridgeSessions = await listAgentSessionsFromLocalBridge();
      if (bridgeSessions) {
        agentSessions = bridgeSessions;
        agentSessionSource = 'browser local session scan';
        return;
      }
    } catch (bridgeError) {
      sessionScanError =
        sessionScanError
        || (bridgeError instanceof Error ? bridgeError.message : 'Could not scan local agent sessions');
    } finally {
      agentSessionsLoading = false;
    }

    agentSessions = [];
    agentSessionSource = 'browser preview';
    agentSessionError = sessionScanError;
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
        hasUpstream: true,
        lastCommitSha: isMacCommandBar ? 'b022003' : null,
        lastCommitSubject: isMacCommandBar ? 'feat: add TSK-127 git history panel' : null,
        lastCommitAt: isMacCommandBar ? new Date().toISOString() : null,
        dirtySinceEpochMs: null,
        dirtyStatusFingerprint: 'cbf29ce484222325',
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

  function formatGitGraphSummary(
    model: GitGraphViewModel,
    loadingSummaries: boolean,
    summaryError: string,
    summarySource: string,
    loadingHistory: boolean,
    historyError: string,
    historySource: string
  ) {
    const { summary } = model;
    const repoLabel = `${summary.dirtyRepositoryCount} dirty / ${summary.repositoryCount} repos`;
    const commitLabel = `${summary.commitCount} ${summary.commitCount === 1 ? 'commit' : 'commits'}`;
    const taskLabel = `${summary.taskCount} ${summary.taskCount === 1 ? 'task' : 'tasks'}`;
    const sourceLabel = [summarySource, historySource].filter(Boolean).join(' + ');
    const statusLabel = [
      loadingSummaries ? 'scanning repos' : '',
      loadingHistory ? 'loading history' : '',
      summaryError,
      historyError
    ].filter(Boolean).join(' · ');

    return [repoLabel, commitLabel, taskLabel, sourceLabel, statusLabel].filter(Boolean).join(' · ');
  }

  function formatGitTaskSearchTargetSummary(targets: GitGraphViewModel['taskSearchTargets']) {
    if (targets.length === 0) return 'No task search targets';
    return targets.map((target) => `${target.label}: ${target.query}`).join(' · ');
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

  function gitCommitSummaryText(entry: GitCommitHistoryEntry) {
    const row = gitCommitGraphRowForEntry(entry);
    if (row) return gitGraphCommitSummaryText(row);

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
    const branch = projectGitStatus?.branch ?? selectedProjectGitGraph.repositories[0]?.branchLabel ?? 'unknown';
    const aheadBehind =
      projectGitStatus
        ? `ahead ${projectGitStatus.ahead} / behind ${projectGitStatus.behind}`
        : 'ahead/behind unknown';
    const changedFiles =
      projectGitStatus
        ? `${selectedProjectGitChangedFiles.length} changed file${selectedProjectGitChangedFiles.length === 1 ? '' : 's'}`
        : 'changed files unknown';
    const tasks = selectedProjectGitTaskIDs.length > 0 ? selectedProjectGitTaskIDs.join(', ') : 'none';
    const repoLines = selectedProjectGitGraph.repositories.slice(0, 6).map((row) => {
      const task = row.taskID ? ` · ${row.taskID}` : '';
      return `- ${row.repo} (${row.rootLabel}): ${row.branchLabel}${task} · ${row.dirty.label} · ${row.sync.label} · ${row.path}`;
    });
    const commitLines = selectedProjectGitGraph.commits.slice(0, 6).map((row) => `- ${gitGraphCommitSummaryText(row)}`);

    return [
      'Git workspace brief',
      `Project: ${selectedProject.name}`,
      `Path: ${selectedProject.path}`,
      `Branch: ${branch}`,
      `Graph: ${selectedProjectGitGraphSummary}`,
      `Health: ${selectedProjectGitBranchHealth.detail}`,
      `Status: ${changedFiles} · ${aheadBehind}`,
      `Tasks: ${tasks}`,
      `Task search targets: ${selectedProjectGitTaskSearchSummary}`,
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

  function sourceIntelligenceBriefText() {
    const currentFile = selectedRecord
      ? `${selectedRecord.relativePath}${selectedSourceLine ? `:${selectedSourceLine}` : ''}`
      : 'none';
    const definitionLines = sourceDefinitionTargets.slice(0, 8).map((target) => {
      return `- ${target.symbolName} (${target.kind}) ${target.relativePath}:${target.line}`;
    });
    const implementationLines = sourceImplementationTargets.slice(0, 6).map((target) => {
      return `- implementation ${target.symbolName} ${target.relativePath}:${target.line}`;
    });
    const typeDefinitionLines = sourceTypeDefinitionTargets.slice(0, 6).map((target) => {
      return `- type ${target.symbolName} ${target.relativePath}:${target.line}`;
    });
    const referenceLines = sourceReferenceTargets.slice(0, 8).map((target) => {
      return `- ${target.fileName}:${target.line}:${target.column} ${target.excerpt}`;
    });
    const problemLines = sourceDiagnostics.slice(0, 8).map((diagnostic) => {
      return `- ${diagnostic.severity} ${diagnostic.line}:${diagnostic.column} ${diagnostic.message}`;
    });
    const symbolLines = sourceSymbols.slice(0, 8).map((symbol) => {
      return `- ${symbol.kind} ${symbol.name} line ${symbol.line}`;
    });

    return [
      'Source intelligence brief',
      `Project: ${selectedProject.name}`,
      `Root: ${selectedProject.path}`,
      `Current file: ${currentFile}`,
      `Language: ${preview?.language ?? 'none'}`,
      `LSP: ${sourceLspStatusLabel()}`,
      `Drawer: ${editorNavPanel ?? 'closed'}`,
      `Problems: ${sourceDiagnosticSummary}`,
      problemLines.length > 0 ? problemLines.join('\n') : '- none',
      `Symbols: ${sourceSymbols.length}`,
      symbolLines.length > 0 ? symbolLines.join('\n') : '- none',
      `Definitions: ${sourceDefinitionSummary || 'none'}`,
      definitionLines.length > 0 ? definitionLines.join('\n') : '- none',
      `Implementations: ${sourceImplementationSummary || 'none'}`,
      implementationLines.length > 0 ? implementationLines.join('\n') : '- none',
      `Type definitions: ${sourceTypeDefinitionSummary || 'none'}`,
      typeDefinitionLines.length > 0 ? typeDefinitionLines.join('\n') : '- none',
      `References: ${sourceReferenceSummary || 'none'}`,
      referenceLines.length > 0 ? referenceLines.join('\n') : '- none'
    ].join('\n');
  }

  function sourceLayoutDiagnosticText() {
    const normalizedLayout = normalizeSourceDockLayout(sourceDockLayout);
    const currentFile = selectedRecord
      ? `${selectedRecord.relativePath}${selectedSourceLine ? `:${selectedSourceLine}` : ''}`
      : 'none';
    const activePreset = sourceLayoutPresets.find((preset) => preset.id === sourceLayoutPreset);
    const openTabs = projectOpenSourceTabs.slice(0, 8).map((tab) => {
      const dirtyPrefix = isSourcePathDirty(tab.path) ? '* ' : '- ';
      return `${dirtyPrefix}${tab.relativePath}`;
    });
    const groupLines = normalizedLayout.groups.map((group) => {
      const activePanelID = normalizedLayout.activePanelByGroup[group.id];
      const panels = group.panelIDs.map((panelID) => {
        const activePrefix = activePanelID === panelID ? '* ' : '';
        const renderState = shouldRenderDockPanel(panelID) ? 'rendered' : 'tabbed';
        return `${activePrefix}${dockPanelLabel(panelID)} (${renderState})`;
      });

      return `- ${dockGroupLabel(group.id)} (${group.id}, ${Math.round(group.size)}): ${
        panels.length > 0 ? panels.join(', ') : 'empty'
      }`;
    });
    const activePanelLines = Object.entries(normalizedLayout.activePanelByGroup).map(([groupID, panelID]) => {
      const typedGroupID = groupID as SourceDockGroupID;
      return `- ${dockGroupLabel(typedGroupID, panelID)}: ${panelID ? dockPanelLabel(panelID) : 'none'}`;
    });
    const hiddenDockPanels = hiddenDockPanelIDs().map(dockPanelLabel);
    const hiddenContextCards = [...hiddenContextCardIDs].map((cardID) => contextCardLabels[cardID]);
    const visiblePanelPlacements = managedDockPanelIDs.map(
      (panelID) => `${dockPanelLabel(panelID)}=${dockPanelPlacementSummary(panelID)}`
    );
    const overrideIDs = Object.keys(sourceLayoutPresetOverrides);

    return [
      'Source layout diagnostic',
      `Project: ${selectedProject.name}`,
      `Root: ${selectedProject.path}`,
      `Current file: ${currentFile}`,
      `Preset: ${sourceLayoutPreset}${activePreset ? ` (${activePreset.label})` : ''}`,
      `Chrome: ${sourceChromeCompact ? 'compact' : 'comfortable'}`,
      `Activity: ${sourceActivityLabel(sourceActivityMode)} (${sourceActivityMode})`,
      `Activity filter: ${sourceActivityFilter.trim() || 'none'}`,
      `Side pane: ${sidePanePosition}, ${sidePaneWidth}px, ${sourceDockPanelVisible('activity') ? 'visible' : 'hidden'}`,
      `Context: ${contextPanelPlacement}, ${contextPanelMode}, ${contextPanelCollapsed ? 'hidden' : 'visible'}`,
      `Context size: ${contextPaneWidth}px wide / ${contextPaneHeight}px tall`,
      `Active context card: ${contextCardLabels[activeContextCardID]}`,
      `Insights: ${editorInsightCollapsed ? 'hidden' : 'visible'}, ${sourceIntelligencePanel}, ${editorInsightWidth}px`,
      `Bottom dock: ${bottomDockPanelVisible() ? 'visible' : 'hidden'}, ${bottomDockHeight()}px`,
      `Terminal app: ${sourceTerminalApp}`,
      `Embedded terminal: ${embeddedTerminalSession?.sessionId ?? 'none'}`,
      `Browser URL: ${activeBrowserUrl || 'none'}`,
      `Focus restore snapshot: ${sourceFocusRestoreLayout ? sourceFocusRestoreLayout.preset : 'none'}`,
      `Saved preset overrides: ${overrideIDs.length > 0 ? overrideIDs.join(', ') : 'none'}`,
      '',
      'Dock layout:',
      groupLines.join('\n'),
      '',
      'Active dock panels:',
      activePanelLines.length > 0 ? activePanelLines.join('\n') : '- none',
      '',
      `Visible panel placements: ${visiblePanelPlacements.join(', ')}`,
      `Hidden dock panels: ${hiddenDockPanels.length > 0 ? hiddenDockPanels.join(', ') : 'none'}`,
      `Hidden context cards: ${hiddenContextCards.length > 0 ? hiddenContextCards.join(', ') : 'none'}`,
      '',
      `Open tabs: ${projectOpenSourceTabs.length}`,
      openTabs.length > 0 ? openTabs.join('\n') : '- none'
    ].join('\n');
  }

  function sourceScanDiagnosticBrief() {
    const indexEntry = selectedProjectIndexEntry;
    const stats = sourceScanStats ?? indexEntry?.stats ?? null;
    const returnedFiles = stats?.returnedFiles ?? stats?.returnedCount;
    const requestedLimit = stats?.requestedLimit ?? stats?.effectiveLimit;
    const collectionLimit = stats?.collectionLimit;
    const visitedEntries = stats?.visitedEntries ?? stats?.visitedEntryCount;
    const matchedFiles = stats?.matchedFiles ?? stats?.matchedFileCount;
    const skippedDirectories = stats?.skippedDirectories ?? stats?.skippedDirectoryCount;
    const unsupportedFiles = stats?.unsupportedFiles ?? stats?.unsupportedFileCount;
    const unreadableEntries = stats?.unreadableEntries ?? stats?.unreadableEntryCount;
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
          `Returned files: ${formatOptionalSourceScanStat(returnedFiles)}`,
          `Requested limit: ${formatOptionalSourceScanStat(requestedLimit)}`,
          `Collection cap: ${formatOptionalSourceScanStat(collectionLimit)}${
            typeof stats.collectionLimitReached === 'boolean'
              ? ` (${stats.collectionLimitReached ? 'reached' : 'not reached'})`
              : ''
          }`,
          `Visited entries: ${formatOptionalSourceScanStat(visitedEntries)}`,
          `Matched files: ${formatOptionalSourceScanStat(matchedFiles)}`,
          `Skipped directories: ${formatOptionalSourceScanStat(skippedDirectories)}`,
          `Unsupported files: ${formatOptionalSourceScanStat(unsupportedFiles)}`,
          `Unreadable entries: ${formatOptionalSourceScanStat(unreadableEntries)}`
        ]
      : ['Scanner stats: none'];
    const skippedDirectoryLines =
      stats?.skippedDirectorySamples && stats.skippedDirectorySamples.length > 0
        ? stats.skippedDirectorySamples.map(
            (sample) => `- ${sample.path || sample.name}: ${sample.reason}`
          )
        : ['- none'];

    return [
      'Source scan diagnostic',
      `Project: ${selectedProject.name}`,
      `Root: ${selectedProject.path}`,
      `Root label: ${sourceContextIdentity.rootLabel}`,
      `Root validation: ${selectedProjectRootValidationSummary}`,
      `Scan limit: ${expandedSourceScanLimit.toLocaleString()}`,
      `Indexed files: ${records.length.toLocaleString()}`,
      `Filtered files: ${filteredRecords.length.toLocaleString()}`,
      `Query: ${query.trim() || 'none'}`,
      `Limit reached: ${scanLimitReached ? 'yes' : 'no'}`,
      `Needs attention: ${sourceScanNeedsAttention ? 'yes' : 'no'}`,
      `Health: ${sourceScanHealthNote || 'ok'}`,
      `Scan evidence: ${selectedProjectScanEvidence.label}`,
      `Scan evidence detail: ${selectedProjectScanEvidence.detail}`,
      `Index cache: ${cacheState}`,
      `Index summary: ${selectedProjectIndexSummary}`,
      `Scan summary: ${scanSummaryLabel}`,
      `Scan stats: ${sourceScanStatsLabel || 'none'}`,
      `Active scan: ${activeScan}`,
      `Loading: ${loading ? 'yes' : 'no'}`,
      `Runtime: ${runtime}`,
      `Browser source bridge active: ${runtime === 'browser source bridge' ? 'yes' : 'no'}`,
      error ? `Error: ${error}` : 'Error: none',
      `Current file: ${currentFile}`,
      `Saved selected path: ${selectedPath}`,
      '',
      'Skipped directory samples:',
      skippedDirectoryLines.join('\n'),
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

  function formatOptionalSourceScanStat(value: number | null | undefined) {
    return typeof value === 'number' && Number.isFinite(value) ? value.toLocaleString() : 'unknown';
  }

  function gitCommitGraphRowForEntry(entry: GitCommitHistoryEntry) {
    return (
      selectedProjectGitGraph.commits.find((row) => row.sha === entry.sha) ??
      buildGitGraphViewModel({ commits: [entry] }).commits[0] ??
      null
    );
  }

  function gitCommitEntryForRow(row: GitGraphCommitRow) {
    return gitCommitHistory.find((entry) => entry.sha === row.sha) ?? null;
  }

  function gitCommitRefChips(entry: GitCommitHistoryEntry) {
    return gitCommitGraphRowForEntry(entry)?.refs.labels ?? [];
  }

  function gitCommitParentSummary(entry: GitCommitHistoryEntry) {
    return gitCommitGraphRowForEntry(entry)?.parentHint.label ?? '';
  }

  function gitCommitTaskSourceLabel(entry: GitCommitHistoryEntry) {
    const row = gitCommitGraphRowForEntry(entry);
    return row ? gitGraphCommitTaskSourceLabel(row) : '';
  }

  function gitGraphCommitTaskSourceLabel(row: GitGraphCommitRow) {
    if (!row.taskID) return '';
    if (row.taskSource === 'refs') return 'branch/ref';
    if (row.taskSource === 'subject') return 'subject';
    return 'Git metadata';
  }

  function gitGraphCommitSummaryText(row: GitGraphCommitRow) {
    const refs = row.refs.labels.join(', ');
    const taskSource = gitGraphCommitTaskSourceLabel(row);
    const task = row.taskID ? `Task ${row.taskID}${taskSource ? ` from ${taskSource}` : ''}` : '';

    return [
      row.shortSha,
      row.subject,
      refs,
      row.parentHint.label,
      task,
      row.author,
      formatGitCommitTime(row.committedAt)
    ].filter(Boolean).join(' · ');
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

  function gitTaskSourceGroupText(group: GitTaskSourceGroup) {
    return formatGitTaskSourceGroupHandoff(group, gitTaskUrl(group.taskID));
  }

  function normalizeGitTaskID(taskID: string | null | undefined) {
    const normalized = String(taskID ?? '').trim().toUpperCase();
    return /^TSK-\d+$/.test(normalized) ? normalized : null;
  }

  function uniqueGitTaskIDs(taskIDs: Array<string | null | undefined>) {
    const seen = new Set<string>();
    const unique: string[] = [];

    for (const taskID of taskIDs) {
      const normalizedTaskID = normalizeGitTaskID(taskID);
      if (!normalizedTaskID || seen.has(normalizedTaskID)) continue;

      seen.add(normalizedTaskID);
      unique.push(normalizedTaskID);
    }

    return unique;
  }

  function buildGitTaskLedgerRows(): GitTaskLedgerRow[] {
    return selectedProjectGitTaskSourceGroups
      .map((group) => {
        const taskID = normalizeGitTaskID(group.taskID) ?? group.taskID;
        const worktrees = prioritizedProjectWorktrees.filter(
          (worktree) => normalizeGitTaskID(worktree.taskID) === taskID
        );
        const commits = selectedProjectGitGraph.commits
          .filter((row) => row.taskReferences.some((task) => normalizeGitTaskID(task.id) === taskID))
          .map(gitCommitEntryForRow)
          .filter((entry): entry is GitCommitHistoryEntry => Boolean(entry));
        const runs = selectedProjectOrchestrationRuns.filter(
          (run) => normalizeGitTaskID(run.taskID) === taskID
        );
        const worktreeSafetySummaries = worktrees.map(projectWorktreeSafety);
        const blockedWorktreeCount = worktreeSafetySummaries.filter((summary) => summary.kind === 'blocked').length;
        const readyWorktreeCount = worktreeSafetySummaries.filter((summary) => summary.kind === 'ready').length;
        const protectedWorktreeCount = worktreeSafetySummaries.filter((summary) => summary.kind === 'protected').length;
        const cleanupCandidateCount = readyWorktreeCount;
        const staleCleanWorktreeCount = worktreeSafetySummaries.filter(
          (summary) => summary.kind === 'ready' && summary.ageBucket === 'stale'
        ).length;
        const backupRequiredWorktreeCount = worktreeSafetySummaries.filter(
          (summary) => summary.kind === 'blocked' && summary.activeSessionCount === 0
        ).length;
        const activeSessionCount = worktreeSafetySummaries.reduce(
          (total, summary) => total + summary.activeSessionCount,
          0
        );
        const savedWorkspaceCount = worktrees.reduce(
          (total, worktree) => total + worktreeWorkspaceSnapshots(worktree, 10).length,
          0
        );
        const ownerSummary = gitTaskLedgerOwnerSummary({
          activeSessionCount,
          savedWorkspaceCount,
          worktreeCount: worktrees.length
        });
        const cleanupSummary = gitTaskLedgerCleanupSummary({
          activeSessionCount,
          backupRequiredWorktreeCount,
          blockedWorktreeCount,
          cleanupCandidateCount,
          protectedWorktreeCount,
          staleCleanWorktreeCount,
          worktreeCount: worktrees.length
        });
        const nextAction = gitTaskLedgerNextAction({
          activeSessionCount,
          blockedWorktreeCount,
          readyWorktreeCount,
          runCount: runs.length,
          commitCount: commits.length,
          worktreeCount: worktrees.length
        });
        const tone: GitTaskLedgerTone =
          activeSessionCount > 0 || blockedWorktreeCount > 0
            ? 'blocked'
            : readyWorktreeCount > 0
              ? 'ready'
              : protectedWorktreeCount > 0
                ? 'protected'
                : worktrees.length > 0 || runs.length > 0
                  ? 'review'
                  : 'clean';

        return {
          taskID,
          sourceSummary: group.sourceSummary,
          detailSummary: group.detailSummary,
          worktreeCount: worktrees.length,
          blockedWorktreeCount,
          readyWorktreeCount,
          cleanupCandidateCount,
          staleCleanWorktreeCount,
          backupRequiredWorktreeCount,
          activeSessionCount,
          savedWorkspaceCount,
          commitCount: commits.length,
          runCount: runs.length,
          ownerSummary,
          cleanupSummary,
          nextAction,
          tone,
          primaryWorktree: worktrees[0] ?? null,
          latestCommit: commits[0] ?? null
        };
      })
      .sort((left, right) => gitTaskLedgerPriority(left) - gitTaskLedgerPriority(right));
  }

  function gitTaskLedgerOwnerSummary(input: {
    activeSessionCount: number;
    savedWorkspaceCount: number;
    worktreeCount: number;
  }) {
    if (input.activeSessionCount > 0) {
      return `${input.activeSessionCount} active ${input.activeSessionCount === 1 ? 'session' : 'sessions'}`;
    }

    if (input.savedWorkspaceCount > 0) {
      return `${input.savedWorkspaceCount} saved ${input.savedWorkspaceCount === 1 ? 'workspace' : 'workspaces'}`;
    }

    if (input.worktreeCount > 0) return 'no saved workspace';
    return 'no worktree';
  }

  function gitTaskLedgerCleanupSummary(input: {
    activeSessionCount: number;
    backupRequiredWorktreeCount: number;
    blockedWorktreeCount: number;
    cleanupCandidateCount: number;
    protectedWorktreeCount: number;
    staleCleanWorktreeCount: number;
    worktreeCount: number;
  }) {
    if (input.activeSessionCount > 0) return 'session-owned';
    if (input.backupRequiredWorktreeCount > 0) {
      return `${input.backupRequiredWorktreeCount} need backup`;
    }
    if (input.staleCleanWorktreeCount > 0) {
      return `${input.staleCleanWorktreeCount} stale-clean`;
    }
    if (input.cleanupCandidateCount > 0) {
      return `${input.cleanupCandidateCount} clean ${input.cleanupCandidateCount === 1 ? 'candidate' : 'candidates'}`;
    }
    if (input.blockedWorktreeCount > 0) return 'blocked cleanup';
    if (input.protectedWorktreeCount > 0) return 'primary checkout';
    if (input.worktreeCount > 0) return 'review ownership';
    return 'no cleanup target';
  }

  function gitTaskLedgerNextAction(input: {
    activeSessionCount: number;
    blockedWorktreeCount: number;
    readyWorktreeCount: number;
    runCount: number;
    commitCount: number;
    worktreeCount: number;
  }) {
    if (input.activeSessionCount > 0) return 'Resume active session';
    if (input.blockedWorktreeCount > 0) return 'Archive or audit worktree';
    if (input.readyWorktreeCount > 0) return 'Remove clean worktree';
    if (input.runCount > 0) return 'Review run status';
    if (input.worktreeCount > 0) return 'Review worktree';
    if (input.commitCount > 0) return 'Review commits';
    return 'Open task';
  }

  function gitTaskLedgerPriority(row: GitTaskLedgerRow) {
    if (row.activeSessionCount > 0) return 0;
    if (row.blockedWorktreeCount > 0) return 1;
    if (row.readyWorktreeCount > 0) return 2;
    if (row.runCount > 0) return 3;
    if (row.worktreeCount > 0) return 4;
    if (row.commitCount > 0) return 5;
    return 6;
  }

  function gitTaskLedgerTitle(row: GitTaskLedgerRow) {
    return gitTaskLedgerText(row);
  }

  function gitTaskLedgerText(row: GitTaskLedgerRow) {
    const taskUrl = gitTaskUrl(row.taskID);
    return [
      `Task: ${row.taskID}`,
      taskUrl ? `Task link: ${taskUrl}` : '',
      `Next: ${row.nextAction}`,
      `Owner: ${row.ownerSummary}`,
      `Cleanup: ${row.cleanupSummary}`,
      `Sources: ${row.sourceSummary}`,
      `Details: ${row.detailSummary}`,
      `Worktrees: ${row.worktreeCount} (${row.blockedWorktreeCount} blocked, ${row.readyWorktreeCount} ready, ${row.activeSessionCount} active sessions)`,
      `Cleanup candidates: ${row.cleanupCandidateCount} (${row.staleCleanWorktreeCount} stale-clean)`,
      `Backup required: ${row.backupRequiredWorktreeCount}`,
      `Saved workspaces: ${row.savedWorkspaceCount}`,
      `Runs: ${row.runCount}`,
      `Commits: ${row.commitCount}`,
      row.primaryWorktree ? `Primary worktree: ${row.primaryWorktree.branch} · ${row.primaryWorktree.path}` : '',
      row.latestCommit ? `Latest commit: ${gitCommitSummaryText(row.latestCommit)}` : ''
    ].filter(Boolean).join('\n');
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

  async function copyGitTaskSourceGroup(group: GitTaskSourceGroup) {
    await copyActivityCommand(gitTaskSourceGroupText(group), 'Task sources copied');
  }

  function openGitTaskReference(taskID: string | null) {
    const url = gitTaskUrl(taskID);
    if (!url || typeof window === 'undefined') return;

    window.open(url, '_blank', 'noopener,noreferrer');
    fileActionStatus = `Opened ${taskID}`;
  }

  async function focusGitTaskLedger(taskID: string | null) {
    const normalizedTaskID = normalizeGitTaskID(taskID);
    if (!normalizedTaskID || typeof document === 'undefined') return;

    selectSourceActivityMode('git');
    await tick();
    const row = document.querySelector<HTMLElement>(
      `[data-task-ledger-id="${normalizedTaskID}"]`
    );
    row?.scrollIntoView({ block: 'center', inline: 'nearest' });
    row?.focus({ preventScroll: true });
  }

  async function copyGitTaskLedger(row: GitTaskLedgerRow) {
    await copyActivityCommand(gitTaskLedgerText(row), 'Task ledger copied');
  }

  async function copyGitWorkspaceBrief() {
    await copyActivityCommand(gitWorkspaceBriefText(), 'Git workspace brief copied');
  }

  async function copySourceContextBrief() {
    await copyActivityCommand(sourceContextBriefText(), 'Source context brief copied');
  }

  async function copySourceIntelligenceBrief() {
    await copyActivityCommand(sourceIntelligenceBriefText(), 'Source intelligence brief copied');
  }

  async function copySourceLayoutDiagnostic() {
    await copyActivityCommand(sourceLayoutDiagnosticText(), 'Layout diagnostic copied');
  }

  async function copySourceScanDiagnosticBrief() {
    await copyActivityCommand(sourceScanDiagnosticBrief(), 'Scan diagnostic copied');
  }

  function repoDashboardTaskLabel(row: GitGraphRepositoryRow) {
    return row.taskID ?? 'none';
  }

  function repoDashboardTaskUrl(row: GitGraphRepositoryRow) {
    return gitTaskUrl(row.taskID);
  }

  function repoDashboardDirtyLabel(row: GitGraphRepositoryRow) {
    return row.dirty.label;
  }

  function repoDashboardRemoteLabel(row: GitGraphRepositoryRow) {
    const remote = row.sync.label;
    const commit = row.lastCommitSha ? ` · ${row.lastCommitSha}` : '';
    return `${remote}${commit}`;
  }

  function repoDashboardTitle(row: GitGraphRepositoryRow) {
    return row.detailLabel;
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

      projectGitStatusRequestID += 1;
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

      projectGitStatusRequestID += 1;
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

      projectGitStatusRequestID += 1;
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

  function contextOrchestrationLoopStages(runMetrics: ReturnType<typeof orchestrationRunMetrics>) {
    const stages = orchestrationLoopStageMetrics(runMetrics);
    const visibleStages = stages.filter((stage) => stage.value > 0 || stage.tone === 'bad' || stage.tone === 'attention');
    return (visibleStages.length > 0 ? visibleStages : stages.slice(0, 3)).slice(0, 6);
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
    setSourceActivityFilter(run.taskID ?? run.title ?? run.id);
  }

  async function openOrchestrationArtifact(artifact: OrchestrationArtifact) {
    if (artifact.path?.trim()) {
      if (isProjectLocalTextArtifactPath(artifact.path)) {
        await selectRecord(sourceRecordFromRestoredPath(selectedProject, artifact.path));
        fileActionStatus = `Opened artifact: ${artifact.title}`;
        return;
      }

      await openActivityPath(artifact.path);
      return;
    }

    if (artifact.url && typeof window !== 'undefined') {
      window.open(artifact.url, '_blank', 'noopener,noreferrer');
      fileActionStatus = `Opened artifact: ${artifact.title}`;
    }
  }

  function isProjectLocalTextArtifactPath(path: string) {
    const normalizedPath = normalizeProjectPath(path);
    const normalizedProjectPath = normalizeProjectPath(selectedProject.path);
    if (!normalizedPath || !normalizedProjectPath) return false;
    if (normalizedPath !== normalizedProjectPath && !normalizedPath.startsWith(`${normalizedProjectPath}/`)) {
      return false;
    }

    const language = sourceLanguageForRestoredPath(path);
    return language !== 'plain' || /\.(txt|log|out|err)$/i.test(path);
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

  function selectedProjectOrchestrationTaskID() {
    return (
      selectedProjectGitTaskIDs[0] ??
      selectedProjectPrimaryRepoSummary?.taskID ??
      selectedProjectOrchestrationRuns.find((run) => run.taskID)?.taskID ??
      (selectedProject.id === 'mac-command-bar' ? 'TSK-127' : null)
    );
  }

  function selectedProjectOrchestrationRunID() {
    const existingRunID = selectedProjectOrchestrationRuns[0]?.id;
    if (existingRunID) return existingRunID;

    const taskID = selectedProjectOrchestrationTaskID();
    return taskID ? `run-${taskID.toLowerCase()}` : `run-${selectedProject.id}`;
  }

  function orchestrationEventCommandDetail() {
    const taskID = selectedProjectOrchestrationTaskID();
    return `${selectedProjectOrchestrationRunID()}${taskID ? ` · ${taskID}` : ` · ${selectedProject.name}`}`;
  }

  function mcbOrchestrationEventCommand(
    preset: string,
    extraArgs: Record<string, string | number | null | undefined> = {}
  ) {
    const taskID = selectedProjectOrchestrationTaskID();
    const args: Record<string, string | number | null | undefined> = {
      '--run-id': selectedProjectOrchestrationRunID(),
      '--preset': preset,
      '--project-id': selectedProject.id,
      '--project-name': selectedProject.name,
      '--project-path': selectedProject.path,
      '--root-label': selectedProjectPrimaryRepoSummary?.rootLabel ?? formatSourceContextRootLabel(selectedProject.path),
      '--task-id': taskID,
      ...extraArgs
    };

    return `cd ${shellQuoteForCommand(macCommandBarRepoPath)} && scripts/mcb-orch ${orchestrationEventCommandArgs(args)}`;
  }

  function orchestrationEventCommandArgs(args: Record<string, string | number | null | undefined>) {
    return Object.entries(args)
      .filter(([, value]) => value !== null && value !== undefined && String(value).trim().length > 0)
      .map(([key, value]) => `${key} ${shellQuoteForCommand(String(value))}`)
      .join(' ');
  }

  async function copyOrchestrationEventCommand(
    preset: string,
    label: string,
    extraArgs: Record<string, string | number | null | undefined> = {}
  ) {
    await copyActivityCommand(mcbOrchestrationEventCommand(preset, extraArgs), `${label} event command copied`);
  }

  async function copyOrchestrationSampleCommand(sample: string, label: string) {
    const taskID = selectedProjectOrchestrationTaskID();
    const args: Record<string, string | number | null | undefined> = {
      '--sample': sample,
      '--run-id': selectedProjectOrchestrationRunID(),
      '--project-id': selectedProject.id,
      '--project-name': selectedProject.name,
      '--project-path': selectedProject.path,
      '--root-label': selectedProjectPrimaryRepoSummary?.rootLabel ?? formatSourceContextRootLabel(selectedProject.path),
      '--task-id': taskID
    };
    await copyActivityCommand(
      `cd ${shellQuoteForCommand(macCommandBarRepoPath)} && scripts/mcb-orch ${orchestrationEventCommandArgs(args)}`,
      `${label} command copied`
    );
  }

  function orchestrationJsonFileImportPath() {
    return orchestrationEventFilePath.trim() || defaultOrchestrationEventFilePath;
  }

  function mcbOrchestrationJsonFileImportCommand(path = orchestrationJsonFileImportPath()) {
    return `cd ${shellQuoteForCommand(macCommandBarRepoPath)} && scripts/mcb-orch --json-file ${shellQuoteForCommand(path)}`;
  }

  async function copyOrchestrationJsonFileImportCommand() {
    await copyActivityCommand(
      mcbOrchestrationJsonFileImportCommand(),
      'Orchestration JSON import command copied'
    );
  }

  async function chooseOrchestrationEventFile() {
    orchestrationEventFileChoosing = true;
    orchestrationEventImportStatus = '';

    try {
      const selectedPath = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'Orchestration events', extensions: ['jsonl', 'json'] }]
      });
      const nextPath = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;
      if (typeof nextPath !== 'string' || nextPath.trim().length === 0) return;

      orchestrationEventFilePath = nextPath;
      orchestrationEventImportStatus = 'Event file selected';
      fileActionStatus = 'Orchestration event file selected';
    } catch (chooseError) {
      orchestrationEventImportStatus =
        chooseError instanceof Error ? chooseError.message : 'Could not choose event file';
    } finally {
      orchestrationEventFileChoosing = false;
    }
  }

  function orchestrationStatusForEventKind(kind: string) {
    const normalizedKind = kind.toLowerCase();
    if (normalizedKind.includes('approval') || normalizedKind.includes('decision')) return 'waiting-for-approval';
    if (normalizedKind.includes('blocker') || normalizedKind.includes('blocked')) return 'blocked';
    if (normalizedKind.includes('failed') || normalizedKind.includes('failure')) return 'failed';
    if (
      normalizedKind.includes('verified') ||
      normalizedKind.includes('resolved') ||
      normalizedKind.includes('complete') ||
      normalizedKind.includes('succeeded')
    ) {
      return 'succeeded';
    }
    return 'running';
  }

  function orchestrationEventID(kind: string, timestamp: string) {
    const randomID =
      typeof crypto !== 'undefined' && 'randomUUID' in crypto
        ? crypto.randomUUID()
        : Math.random().toString(36).slice(2);
    return `${selectedProjectOrchestrationRunID()}-${kind}-${timestamp}-${randomID}`;
  }

  function nativeOrchestrationEvent(
    kind: string,
    title: string,
    patch: Partial<OrchestrationEvent> = {}
  ): OrchestrationEvent {
    const timestamp = new Date().toISOString();
    const event: OrchestrationEvent = {
      schemaVersion: 1,
      id: orchestrationEventID(kind, timestamp),
      runId: selectedProjectOrchestrationRunID(),
      timestamp,
      kind,
      status: orchestrationStatusForEventKind(kind),
      title,
      message: null,
      projectID: selectedProject.id,
      projectName: selectedProject.name,
      projectPath: selectedProject.path,
      rootLabel: selectedProjectPrimaryRepoSummary?.rootLabel ?? formatSourceContextRootLabel(selectedProject.path),
      taskID: selectedProjectOrchestrationTaskID(),
      agentId: null,
      agentProvider: null,
      agentRole: null,
      stepId: null,
      stepKind: null,
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null,
      scenario: null,
      issueID: null,
      retryAttempt: null,
      approvalSubject: null,
      blockerReason: null,
      decisionPrompt: null,
      scenarioCount: null,
      issueCount: null,
      testCount: null,
      retestCount: null,
      fixCount: null,
      resolvedCount: null,
      verifiedCount: null,
      delegatedCount: null,
      decisionCount: null,
      approvalCount: null,
      failedCount: null
    };

    return { ...event, ...patch } as OrchestrationEvent;
  }

  function orchestrationEventCommandPatchArgs(patch: Partial<OrchestrationEvent>) {
    return {
      '--message': patch.message,
      '--scenario': patch.scenario,
      '--issue-id': patch.issueID,
      '--agent-role': patch.agentRole,
      '--approval-subject': patch.approvalSubject,
      '--blocker-reason': patch.blockerReason,
      '--decision-prompt': patch.decisionPrompt,
      '--scenario-count': patch.scenarioCount,
      '--issue-count': patch.issueCount,
      '--test-count': patch.testCount,
      '--retest-count': patch.retestCount,
      '--fix-count': patch.fixCount,
      '--resolved-count': patch.resolvedCount,
      '--verified-count': patch.verifiedCount,
      '--delegated-count': patch.delegatedCount,
      '--decision-count': patch.decisionCount,
      '--approval-count': patch.approvalCount,
      '--failed-count': patch.failedCount
    };
  }

  async function recordNativeOrchestrationEvent(
    kind: string,
    label: string,
    patch: Partial<OrchestrationEvent> = {}
  ) {
    orchestrationEventRecording = true;
    orchestrationEventImportStatus = '';
    orchestrationRunError = '';

    try {
      const event = nativeOrchestrationEvent(kind, label, patch);
      const run = await recordOrchestrationEventToTauri(event);

      if (!run) {
        await copyActivityCommand(
          mcbOrchestrationEventCommand(kind, orchestrationEventCommandPatchArgs(patch)),
          'Native event unavailable; command copied'
        );
        orchestrationEventImportStatus = 'Native event store unavailable; command copied';
        return;
      }

      orchestrationRuns = [run, ...orchestrationRuns.filter((entry) => entry.id !== run.id)];
      orchestrationRunSource = 'native event store';
      orchestrationEventImportStatus = `${label} recorded`;
      fileActionStatus = `${label} recorded`;
      focusOrchestrationRun(run);
      void loadOrchestrationRuns(projectOptions, { background: true });
    } catch (recordError) {
      orchestrationRunError = recordError instanceof Error ? recordError.message : 'Could not record orchestration event';
      orchestrationEventImportStatus = orchestrationRunError;
    } finally {
      orchestrationEventRecording = false;
    }
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

  function formatConversationSessionSummary(
    sessionCount: number,
    loadingSessions: boolean,
    sessionError: string,
    sessionSource: string
  ) {
    if (loadingSessions) return 'Scanning local conversations';
    if (sessionError) return sessionError;
    if (sessionCount === 0) return `No local conversations · ${sessionSource}`;
    return `${sessionCount} ${sessionCount === 1 ? 'conversation' : 'conversations'} · ${sessionSource}`;
  }

  function agentSessionMatchesProject(session: AgentSession, project: ProjectRoot) {
    if (!session.projectPath) return true;

    const sessionPath = normalizeProjectPath(session.projectPath);
    const projectPath = normalizeProjectPath(project.path);
    if (sessionPath === projectPath || sessionPath.startsWith(`${projectPath}/`)) return true;

    return projectWorktreeSlugCandidates(project).some((slug) =>
      sessionPath.toLowerCase().includes(`/worktrees/${slug}/`)
    );
  }

  function projectWorktreeSlugCandidates(project: ProjectRoot) {
    return Array.from(new Set([
      project.id,
      project.name,
      fileNameFromRestoredPath(project.path),
      project.name.replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    ]
      .map((value) => value.trim().toLowerCase())
      .flatMap((value) => [value, value.replace(/\s+/g, '-'), value.replace(/[^a-z0-9]+/g, '-')])
      .map((value) => value.replace(/^-+|-+$/g, ''))
      .filter(Boolean)));
  }

  function agentSessionProjectLabel(session: AgentSession) {
    return session.projectPath ? formatSourceContextRootLabel(session.projectPath) : 'global';
  }

  function agentSessionProviderLabel(session: AgentSession) {
    const provider = session.provider.trim().toLowerCase();
    const cmuxAgent = provider.startsWith('cmux-') ? provider.slice('cmux-'.length) : '';
    if (cmuxAgent) return `CMUX ${agentSessionProviderName(cmuxAgent)}`;
    return agentSessionProviderName(provider);
  }

  function agentSessionProviderBadgeLabel(session: AgentSession) {
    const provider = session.provider.trim().toLowerCase();
    if (provider.startsWith('cmux-')) return 'CMUX';
    return agentSessionProviderName(provider);
  }

  function agentSessionProviderName(provider: string) {
    switch (provider) {
      case 'codex':
        return 'Codex';
      case 'claude':
        return 'Claude';
      case 'gemini':
        return 'Gemini';
      case 'opencode':
        return 'OpenCode';
      case 'cursor':
      case 'cursor-agent':
        return 'Cursor';
      default:
        return provider ? `${provider.slice(0, 1).toUpperCase()}${provider.slice(1)}` : 'Agent';
    }
  }

  function agentSessionModelLabel(session: AgentSession) {
    return session.model?.trim() || 'model unknown';
  }

  function agentSessionActivityLabel(session: AgentSession) {
    return formatActivityTimestamp(session.lastActivity);
  }

  function agentSessionTaskID(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    const candidates = [
      snapshot?.branch,
      snapshot?.title,
      session.title,
      agentSessionProjectLabel(session),
      session.projectPath
    ];

    for (const candidate of candidates) {
      const match = String(candidate ?? '').match(/\b(?:TSK|tsk)-\d+\b/);
      const taskID = normalizeGitTaskID(match?.[0]);
      if (taskID) return taskID;
    }

    return null;
  }

  function worktreeForAgentSession(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    const paths = [snapshot?.worktreePath, snapshot?.cwd, session.projectPath]
      .map((path) => normalizeProjectPath(path ?? ''))
      .filter(Boolean);

    return prioritizedProjectWorktrees.find((worktree) => {
      const worktreePath = normalizeProjectPath(worktree.path);
      return paths.some((path) => path === worktreePath || path.startsWith(`${worktreePath}/`));
    }) ?? null;
  }

  function agentSessionBranchLabel(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    const worktree = worktreeForAgentSession(session, snapshot);
    if (snapshot?.branch) return snapshot.branch;
    if (worktree?.branch) return worktree.branch;

    const sessionPath = normalizeProjectPath(session.projectPath ?? '');
    const projectPath = normalizeProjectPath(selectedProject.path);
    if (sessionPath === projectPath || !sessionPath) {
      return projectGitStatus?.branch ?? selectedProjectPrimaryRepoSummary?.branch ?? 'branch unknown';
    }

    return 'branch unknown';
  }

  function agentSessionRootLabel(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    const worktree = worktreeForAgentSession(session, snapshot);
    if (worktree) return fileNameFromRestoredPath(worktree.path);
    if (snapshot?.worktreePath) return fileNameFromRestoredPath(snapshot.worktreePath);
    if (snapshot?.cwd) return formatSourceContextRootLabel(snapshot.cwd);
    return agentSessionProjectLabel(session);
  }

  function agentSessionPathLabel(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    const worktree = worktreeForAgentSession(session, snapshot);
    const path = snapshot?.worktreePath ?? snapshot?.cwd ?? worktree?.path ?? session.projectPath ?? '';
    return path ? formatSourceContextRootLabel(path) : 'global';
  }

  function agentSessionFileContextLabel(snapshot: WorkspaceSnapshot | null) {
    if (!snapshot) return 'no saved file context';
    return workspaceSnapshotFileStateLabel(snapshot);
  }

  function agentSessionResumeMetaLabel(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    return [
      agentSessionActivityLabel(session),
      session.provider.startsWith('cmux-') ? agentSessionProviderLabel(session) : '',
      agentSessionBranchLabel(session, snapshot),
      agentSessionRootLabel(session, snapshot),
      session.model ? agentSessionModelLabel(session) : ''
    ].filter(Boolean).join(' · ');
  }

  function formatActivityTimestamp(value: string | null | undefined) {
    if (!value) return 'unknown activity';

    const numericValue = Number(value);
    const epochMs = Number.isFinite(numericValue)
      ? numericValue > 1_000_000_000_000
        ? numericValue
        : numericValue * 1000
      : new Date(value).getTime();
    if (Number.isNaN(epochMs)) return value;

    return formatRelativeAge(epochMs);
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

  function withAgentTerminalColorEnv(command: string) {
    const trimmed = command.trim();
    if (!/^(codex\s+resume\b|claude\s+(--resume|-r|--continue|-c)\b)/.test(trimmed)) {
      return trimmed;
    }
    if (/^env\s+/.test(trimmed) && /\bFORCE_COLOR=/.test(trimmed)) return trimmed;

    return [
      'env -u NO_COLOR',
      'TERM=xterm-256color',
      'COLORTERM=truecolor',
      'TERM_PROGRAM=MacCommandBar',
      'CLICOLOR=1',
      'CLICOLOR_FORCE=1',
      'FORCE_COLOR=3',
      `COLORFGBG=${shellQuoteForCommand('15;0')}`,
      trimmed
    ].join(' ');
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
    captureWorkspaceSnapshot(activeWorkspaceAgentSession());
  }

  function captureAgentSessionWorkspaceSnapshot(session: AgentSession): WorkspaceSnapshot {
    return captureWorkspaceSnapshot(session);
  }

  function createAgentSessionShellSnapshot(session: AgentSession): WorkspaceSnapshot {
    const sessionPath = agentSessionProjectPath(session);
    const project = ensureWorkspaceSnapshotProject(
      createProjectRoot(fileNameFromRestoredPath(sessionPath), sessionPath)
    );
    const plan = createWorkspaceSessionSnapshotPlan({
      selectedProject: project,
      projectOptions,
      session,
      selectedRecord: null,
      selectedSourcePaths: {},
      openSourceTabs: [],
      selectedLine: null,
      sourceActivityMode: 'conversations',
      sourceTerminalApp,
      browserUrl: null,
      viewState: {
        ...workspaceSnapshotViewState(),
        sourceActivityFilter: ''
      },
      embeddedTerminal: null,
      dockLayout: showSourceDockPanel(sourceDockLayout, 'terminal'),
      branch: null,
      capturedAt: Date.now()
    });
    const snapshot = plan.snapshot;
    const nextSnapshots = upsertWorkspaceSnapshot(
      workspaceSnapshots,
      snapshot,
      maxWorkspaceSnapshots
    );

    workspaceSnapshots = nextSnapshots;
    persistWorkspaceSnapshots(nextSnapshots);
    return snapshot;
  }

  function activeWorkspaceAgentSession(): AgentSession | null {
    if (!activeWorkspaceSessionKey) return null;

    return agentSessions.find(
      (session) => workspaceSnapshotIDForAgentSession(session) === activeWorkspaceSessionKey
    ) ?? null;
  }

  function captureActiveWorkspaceBeforeSwitch() {
    if (!activeWorkspaceSessionKey) return;

    const activeSession = activeWorkspaceAgentSession();
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
    const plan = createWorkspaceSessionSnapshotPlan({
      selectedProject,
      projectOptions,
      session,
      selectedRecord,
      selectedSourcePaths,
      openSourceTabs: projectOpenSourceTabs,
      branch: projectGitStatus?.branch ?? selectedProjectRepositorySummaries[0]?.branch ?? null,
      selectedLine: selectedSourceLine,
      sourceActivityMode,
      sourceTerminalApp,
      browserUrl: activeBrowserUrl || null,
      viewState: workspaceSnapshotViewState(),
      embeddedTerminal: workspaceSnapshotEmbeddedTerminal(),
      dockLayout: sourceDockLayout,
      capturedAt: Date.now()
    });
    const snapshot = plan.snapshot;
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
    const snapshot = workspaceSnapshotForAgentSession(session) ?? createAgentSessionShellSnapshot(session);
    await restoreConversationWorkspaceSnapshot(snapshot);
    markAgentSessionWorkspaceActive(session);
  }

  async function switchToConversationWorkspace(session: AgentSession) {
    captureActiveWorkspaceBeforeSwitch();
    const snapshot = workspaceSnapshotForAgentSession(session) ?? createAgentSessionShellSnapshot(session);
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);

    await restoreConversationWorkspaceSnapshot(snapshot);
    markAgentSessionWorkspaceActive(session);

    if (!readiness.canResumeEmbedded) {
      fileActionStatus =
        readiness.kind === 'missing-worktree'
          ? 'Conversation restored; repair worktree before terminal resume'
          : `Conversation restored: ${session.title}`;
      return;
    }

    await openWorkspaceSnapshotEmbeddedTerminal(snapshot, { restoreWorkspace: false });
  }

  function markAgentSessionWorkspaceActive(session: AgentSession) {
    activeWorkspaceSessionKey = workspaceSnapshotIDForAgentSession(session);
    persistActiveWorkspaceSessionKey(activeWorkspaceSessionKey);
  }

  async function openWorkspaceSnapshotPath(snapshot: WorkspaceSnapshot) {
    const path = snapshot.worktreePath ?? snapshot.cwd;
    if (!path.trim()) return;
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    if (readiness.kind === 'missing-worktree') {
      await copyWorkspaceSnapshotRepairPlan(
        snapshot,
        'Workspace repair plan copied before opening path'
      );
      return;
    }

    await openActivityPath(path);
  }

  async function openWorkspaceSnapshotPathTerminal(snapshot: WorkspaceSnapshot) {
    const path = snapshot.worktreePath ?? snapshot.cwd;
    if (!path.trim()) return;
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    if (readiness.kind === 'missing-worktree') {
      await copyWorkspaceSnapshotRepairPlan(
        snapshot,
        'Workspace repair plan copied before opening terminal'
      );
      return;
    }

    await openActivityTerminalPath(path);
  }

  async function openWorkspaceSnapshotTerminal(snapshot: WorkspaceSnapshot) {
    const command = withAgentTerminalColorEnv(
      stripLeadingShellCdCommand(snapshot.resumeCommand?.trim() ?? '')
    );
    const path = snapshot.worktreePath ?? snapshot.cwd;
    if (!command && !path.trim()) return;
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    if (readiness.kind === 'missing-worktree') {
      await copyWorkspaceSnapshotRepairPlan(
        snapshot,
        'Workspace repair plan copied before terminal resume'
      );
      return;
    }

    await restoreConversationWorkspaceSnapshot(snapshot);

    fileActionBusy = `workspace-terminal-command:${snapshot.id}`;
    fileActionStatus = '';
    error = '';

    try {
      if (command) {
        const openedCommand = await openTerminalCommandWithFallback(
          path,
          command,
          snapshot.sourceTerminalApp
        );
        if (openedCommand.opened) {
          fileActionStatus = terminalOpenStatus(
            openedCommand,
            'Opened workspace resume command',
            'opened workspace resume command in Terminal'
          );
          return;
        }
      }

      const openedPath = path.trim()
        ? await openTerminalPathWithFallback(path, snapshot.sourceTerminalApp)
        : { opened: false, app: snapshot.sourceTerminalApp, usedFallback: false };
      if (command) await navigator.clipboard.writeText(command);
      fileActionStatus = command
        ? openedPath.opened
          ? 'Opened workspace terminal and copied resume command'
          : 'Workspace resume command copied'
        : openedPath.opened
          ? 'Opened workspace terminal'
          : 'Native action unavailable';
    } catch (terminalError) {
      try {
        const openedPath = path.trim()
          ? await openTerminalPathWithFallback(path, snapshot.sourceTerminalApp)
          : { opened: false, app: snapshot.sourceTerminalApp, usedFallback: false };
        if (command) await navigator.clipboard.writeText(command);
        fileActionStatus = command
          ? openedPath.opened
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

  async function openWorkspaceSnapshotEmbeddedTerminal(
    snapshot: WorkspaceSnapshot,
    options: { restoreWorkspace?: boolean } = {}
  ) {
    const shouldRestoreWorkspace = options.restoreWorkspace ?? true;
    const command = withAgentTerminalColorEnv(
      stripLeadingShellCdCommand(snapshot.resumeCommand?.trim() ?? '')
    );
    const path = snapshot.worktreePath ?? snapshot.cwd;
    if (!command && !path.trim()) return;
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    if (readiness.kind === 'missing-worktree') {
      await copyWorkspaceSnapshotRepairPlan(
        snapshot,
        'Workspace repair plan copied before embedded terminal resume'
      );
      return;
    }

    if (shouldRestoreWorkspace) {
      await restoreConversationWorkspaceSnapshot(snapshot);
    }
    showDockPanel('terminal');
    await tick();

    fileActionBusy = `workspace-embedded-terminal-command:${snapshot.id}`;
    fileActionStatus = '';
    error = '';
    embeddedTerminalError = '';

    try {
      const savedSession = embeddedTerminalSessionForSnapshot(snapshot);
      if (savedSession) {
        await attachEmbeddedTerminalSession(savedSession);
        fileActionStatus = `Attached conversation terminal: ${snapshot.title}`;
        return;
      }

      const snapshotFallbackRoots = [snapshot.worktreePath, snapshot.cwd, snapshot.project?.path];
      if (command) {
        const session = await startEmbeddedTerminalSession(path, command, snapshotFallbackRoots);
        if (session) {
          bindWorkspaceSnapshotEmbeddedTerminal(snapshot, session);
          // Keep the cwd-fallback notice if one was raised; otherwise confirm resume.
          fileActionStatus = fileActionStatus.startsWith('Working directory ‹')
            ? fileActionStatus
            : `Started conversation terminal: ${snapshot.title}`;
        } else {
          fileActionStatus = 'Embedded terminal unavailable';
        }
        return;
      }

      const session = await startEmbeddedTerminalSession(path, '', snapshotFallbackRoots);
      if (session) {
        bindWorkspaceSnapshotEmbeddedTerminal(snapshot, session);
        fileActionStatus = fileActionStatus.startsWith('Working directory ‹')
          ? fileActionStatus
          : `Started conversation shell: ${snapshot.title}`;
      } else {
        fileActionStatus = 'Embedded terminal unavailable';
      }
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

  function workspaceSnapshotRepairPlan(snapshot: WorkspaceSnapshot) {
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    const projectPath = snapshot.project.path;
    const worktreePath = snapshot.worktreePath ?? snapshot.cwd;
    const auditCommands = [
      `git -C ${shellQuoteForCommand(projectPath)} worktree list --porcelain`,
      `git -C ${shellQuoteForCommand(projectPath)} worktree prune --dry-run --verbose`
    ];
    const recreateCommand =
      snapshot.branch && snapshot.worktreePath
        ? `# git -C ${shellQuoteForCommand(projectPath)} worktree add ${shellQuoteForCommand(snapshot.worktreePath)} ${shellQuoteForCommand(snapshot.branch)}`
        : '';

    return [
      'Workspace context repair plan',
      `Title: ${snapshot.title}`,
      `Provider: ${snapshot.provider}`,
      `Session: ${snapshot.sessionID}`,
      `Project: ${snapshot.project.name}`,
      `Root: ${projectPath}`,
      `CWD: ${snapshot.cwd}`,
      `Saved worktree: ${worktreePath}`,
      `Branch: ${snapshot.branch ?? 'unknown'}`,
      `Status: ${readiness.label} - ${readiness.detail}`,
      `Repair: ${readiness.repairDetail ?? 'No repair is required before restore.'}`,
      '',
      'Audit before changing anything:',
      ...auditCommands,
      '',
      recreateCommand ? 'Optional recreate command after audit:' : '',
      recreateCommand,
      '',
      'After repair:',
      '- Re-scan worktrees in MacCommandBar.',
      '- Restore this workspace snapshot again.'
    ].filter(Boolean).join('\n');
  }

  function workspaceSnapshotRestorePlan(snapshot: WorkspaceSnapshot) {
    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    const repairPlan =
      readiness.kind === 'missing-worktree'
        ? ['Repair plan:', workspaceSnapshotRepairPlan(snapshot)]
        : [];
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
      `Restore status: ${readiness.label} - ${readiness.detail}`,
      `Activity pane: ${sourceActivityLabel(snapshot.sourceActivityMode)}`,
      `Terminal app: ${snapshot.sourceTerminalApp}`,
      `Browser URL: ${snapshot.browserUrl ?? 'none'}`,
      `View state: ${workspaceSnapshotViewStateLabel(snapshot.viewState)}`,
      `Embedded terminal: ${embeddedTerminal}`,
      `Resume command: ${snapshot.resumeCommand ?? 'none'}`,
      ...repairPlan,
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

  function workspaceSnapshotEnvironmentLabel(snapshot: WorkspaceSnapshot) {
    return [
      workspaceSnapshotScopeLabel(snapshot),
      snapshot.branch ?? '',
      snapshot.worktreePath ? fileNameFromRestoredPath(snapshot.worktreePath) : '',
      `saved ${formatRelativeAge(snapshot.capturedAt)}`
    ].filter(Boolean).join(' · ');
  }

  function workspaceSnapshotFileStateLabel(snapshot: WorkspaceSnapshot) {
    const selectedFile = snapshot.selectedPath
      ? workspaceSnapshotRelativePath(snapshot, snapshot.selectedPath)
      : 'no selected file';
    const openCount = snapshot.openPaths.length;
    return `${selectedFile}${snapshot.selectedLine ? `:${snapshot.selectedLine}` : ''} · ${openCount} open`;
  }

  function agentSessionWorkspaceStateLabel(session: AgentSession, snapshot: WorkspaceSnapshot | null) {
    if (!snapshot) {
      return `${agentSessionProjectLabel(session)} · unsaved workspace`;
    }

    return workspaceSnapshotEnvironmentLabel(snapshot);
  }

  function agentSessionWorkspaceReadinessLabel(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) return 'No saved workspace';

    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    return `${readiness.label} · ${readiness.detail}`;
  }

  function agentSessionWorkspaceSummaryLines(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) {
      return [
        'Saved workspace: none',
        'Restore state: save a workspace snapshot before switching away if you want full context restore.'
      ];
    }

    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    const selectedFile = snapshot.selectedPath
      ? `${workspaceSnapshotRelativePath(snapshot, snapshot.selectedPath)}${snapshot.selectedLine ? `:${snapshot.selectedLine}` : ''}`
      : 'none';
    const openFileCount = snapshot.openPaths.length;

    return [
      `Saved workspace: ${readiness.label} - ${readiness.detail}`,
      `Snapshot root: ${snapshot.project.path}`,
      `Snapshot cwd: ${snapshot.cwd}`,
      `Snapshot worktree: ${snapshot.worktreePath ?? 'none'}`,
      `Snapshot branch: ${snapshot.branch ?? 'unknown'}`,
      `Selected file: ${selectedFile}`,
      `Open files: ${openFileCount}`,
      `Captured: ${formatWorkspaceSnapshotTime(snapshot.capturedAt)}`,
      readiness.repairDetail ? `Repair: ${readiness.repairDetail}` : ''
    ].filter(Boolean);
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

  async function copyWorkspaceSnapshotRepairPlan(
    snapshot: WorkspaceSnapshot,
    successStatus = 'Workspace repair plan copied'
  ) {
    await copyActivityCommand(workspaceSnapshotRepairPlan(snapshot), successStatus);
  }

  function workspaceSnapshotForEmbeddedTerminalSession(session: TerminalSessionInfo) {
    return [...workspaceSnapshots]
      .sort((left, right) => right.capturedAt - left.capturedAt)
      .find((snapshot) => snapshot.embeddedTerminal?.sessionID === session.sessionId) ?? null;
  }

  function embeddedTerminalSessionWorkspaceLabel(session: TerminalSessionInfo) {
    const snapshot = workspaceSnapshotForEmbeddedTerminalSession(session);
    if (!snapshot) return 'No saved workspace';

    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    return `${readiness.label} · ${snapshot.title}`;
  }

  function embeddedTerminalSessionMetaLabel(session: TerminalSessionInfo) {
    const parts = [
      session.shell,
      session.pid ? `pid ${session.pid}` : '',
      `${session.cols}x${session.rows}`,
      embeddedTerminalSessionWorkspaceLabel(session)
    ].filter(Boolean);

    return parts.join(' · ');
  }

  function embeddedTerminalSessionFocusPlan(session: TerminalSessionInfo) {
    const snapshot = workspaceSnapshotForEmbeddedTerminalSession(session);
    const readiness = snapshot ? workspaceSnapshotRestoreReadiness(snapshot) : null;
    const restoreLines = snapshot
      ? [
          '',
          'Matched workspace snapshot:',
          `Title: ${snapshot.title}`,
          `Provider: ${snapshot.provider}`,
          `Session: ${snapshot.sessionID}`,
          `Project: ${snapshot.project.name}`,
          `Root: ${snapshot.project.path}`,
          `CWD: ${snapshot.cwd}`,
          `Worktree: ${snapshot.worktreePath ?? 'none'}`,
          `Branch: ${snapshot.branch ?? 'unknown'}`,
          `Restore status: ${readiness?.label ?? 'unknown'} - ${readiness?.detail ?? 'unknown'}`,
          `Resume command: ${snapshot.resumeCommand ?? 'none'}`,
          `Captured: ${formatWorkspaceSnapshotTime(snapshot.capturedAt)}`
        ]
      : [
          '',
          'Matched workspace snapshot: none',
          'Action: attach the PTY directly, then save a workspace snapshot if this terminal should be restorable later.'
        ];
    const repairLines =
      snapshot && readiness?.kind === 'missing-worktree'
        ? ['', 'Repair plan:', workspaceSnapshotRepairPlan(snapshot)]
        : [];

    return [
      'Embedded terminal focus plan',
      `Session: ${session.sessionId}`,
      `CWD: ${session.cwd}`,
      `Shell: ${session.shell}`,
      `PID: ${session.pid ?? 'unknown'}`,
      `Size: ${session.cols}x${session.rows}`,
      `Started: ${formatWorkspaceSnapshotTime(session.startedAt)}`,
      `Active: ${embeddedTerminalSession?.sessionId === session.sessionId ? 'yes' : 'no'}`,
      ...restoreLines,
      ...repairLines
    ].filter(Boolean).join('\n');
  }

  async function copyEmbeddedTerminalSessionFocusPlan(session: TerminalSessionInfo) {
    await copyActivityCommand(embeddedTerminalSessionFocusPlan(session), 'Terminal focus plan copied');
  }

  async function restoreEmbeddedTerminalSessionWorkspace(session: TerminalSessionInfo) {
    const snapshot = workspaceSnapshotForEmbeddedTerminalSession(session);
    if (!snapshot) {
      showDockPanel('terminal');
      await tick();
      await attachEmbeddedTerminalSession(session);
      fileActionStatus = 'Attached embedded terminal; no saved workspace found';
      return;
    }

    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    if (readiness.kind === 'missing-worktree') {
      await copyWorkspaceSnapshotRepairPlan(
        snapshot,
        'Workspace repair plan copied before terminal workspace restore'
      );
      return;
    }

    await restoreConversationWorkspaceSnapshot(snapshot);
    showDockPanel('terminal');
    await tick();
    await attachEmbeddedTerminalSession(session);
    fileActionStatus = `Restored terminal workspace: ${snapshot.title}`;
  }

  async function copyAgentSessionWorkspaceRestorePlan(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) {
      fileActionStatus = `No workspace snapshot saved for ${session.title}`;
      return;
    }

    await copyWorkspaceSnapshotRestorePlan(snapshot);
  }

  async function copyAgentSessionWorkspaceRepairPlan(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) {
      fileActionStatus = `No workspace snapshot saved for ${session.title}`;
      return;
    }

    await copyWorkspaceSnapshotRepairPlan(snapshot);
  }

  function agentSessionMissingWorktreeSnapshot(session: AgentSession) {
    const snapshot = workspaceSnapshotForAgentSession(session);
    if (!snapshot) return null;

    const readiness = workspaceSnapshotRestoreReadiness(snapshot);
    return readiness.kind === 'missing-worktree' ? snapshot : null;
  }

  async function copyAgentSessionMissingWorktreeRepairPlan(
    session: AgentSession,
    successStatus = 'Workspace repair plan copied'
  ) {
    const snapshot = agentSessionMissingWorktreeSnapshot(session);
    if (!snapshot) return false;

    await copyWorkspaceSnapshotRepairPlan(snapshot, successStatus);
    return true;
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
    setBrowserDockUrl(restored.browserUrl ?? '');
    applyWorkspaceSnapshotViewState(restored.viewState);
    sourceDockLayout = ensureWorkbenchRightPanels(sourceDockLayoutWithWorkspaceViewState(restored.dockLayout, restored.viewState));
    syncSourceDockLayoutToWorkspace(sourceDockLayout);
    persistSourceDockLayout(sourceDockLayout);
    activeWorkspaceSessionKey = snapshot.id;
    persistActiveWorkspaceSessionKey(activeWorkspaceSessionKey);
    fileActionStatus = `Workspace restored: ${snapshot.title}`;

    await activateWorkspaceSnapshotProject(project, restored.selectedPath);

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

  function workspaceSnapshotViewState(): WorkspaceSnapshotViewState {
    return {
      contextPanelMode,
      contextPanelPlacement,
      contextPanelCollapsed,
      editorInsightCollapsed,
      sidePanePosition,
      sidePaneWidth,
      contextPaneWidth,
      contextPaneHeight,
      editorInsightWidth,
      sourceChromeCompact,
      sourceActivityFilter,
      hiddenContextCardIDs: [...hiddenContextCardIDs],
      activeContextCardID,
      sourceIntelligencePanel
    };
  }

  function applyWorkspaceSnapshotViewState(viewState: WorkspaceSnapshotViewState) {
    contextPanelMode = viewState.contextPanelMode;
    contextPanelPlacement = viewState.contextPanelPlacement;
    contextPanelCollapsed = viewState.contextPanelCollapsed;
    editorInsightCollapsed = viewState.editorInsightCollapsed;
    sidePanePosition = viewState.sidePanePosition;
    sidePaneWidth = clampSidePaneWidth(viewState.sidePaneWidth);
    contextPaneWidth = clampContextPaneWidth(viewState.contextPaneWidth);
    contextPaneHeight = clampContextPaneHeight(viewState.contextPaneHeight);
    editorInsightWidth = clampEditorInsightWidth(viewState.editorInsightWidth);
    sourceChromeCompact = viewState.sourceChromeCompact;
    setSourceActivityFilter(viewState.sourceActivityFilter);
    hiddenContextCardIDs = new Set(viewState.hiddenContextCardIDs);
    activeContextCardID = viewState.activeContextCardID;
    sourceIntelligencePanel = viewState.sourceIntelligencePanel;
    persistContextPanelMode(contextPanelMode);
    persistContextPanelPlacement(contextPanelPlacement);
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistEditorInsightCollapsed(editorInsightCollapsed);
    persistSidePanePosition(sidePanePosition);
    persistSidePaneWidth(sidePaneWidth);
    persistContextPaneWidth(contextPaneWidth);
    persistContextPaneHeight(contextPaneHeight);
    persistEditorInsightWidth(editorInsightWidth);
    persistSourceChromeCompact(sourceChromeCompact);
    persistHiddenContextCards(hiddenContextCardIDs);
    persistActiveContextCard(activeContextCardID);
  }

  function workspaceSnapshotViewStateLabel(viewState: WorkspaceSnapshotViewState) {
    const hiddenCount = viewState.hiddenContextCardIDs.length;
    return [
      `context ${viewState.contextPanelMode} ${viewState.contextPanelPlacement}${viewState.contextPanelCollapsed ? ' hidden' : ''}`,
      `active ${contextCardLabels[viewState.activeContextCardID]}`,
      `explorer ${viewState.sidePanePosition} ${viewState.sidePaneWidth}px`,
      `context ${viewState.contextPaneWidth}px/${viewState.contextPaneHeight}px`,
      `insight ${viewState.editorInsightWidth}px`,
      `chrome ${viewState.sourceChromeCompact ? 'compact' : 'comfortable'}`,
      viewState.sourceActivityFilter ? `filter "${viewState.sourceActivityFilter}"` : 'no activity filter',
      hiddenCount === 0 ? 'no hidden cards' : `${hiddenCount} hidden`,
      `insights ${viewState.editorInsightCollapsed ? 'hidden' : viewState.sourceIntelligencePanel}`
    ].join(' · ');
  }

  function workspaceSnapshotRestoreReadiness(snapshot: WorkspaceSnapshot) {
    const snapshotProjectPath = normalizeProjectPath(snapshot.project.path);
    const selectedProjectPath = normalizeProjectPath(selectedProject.path);
    const snapshotWorktreePath = snapshot.worktreePath ? normalizeProjectPath(snapshot.worktreePath) : '';
    const shouldUseCurrentWorktreeScan =
      snapshotProjectPath === selectedProjectPath ||
      Boolean(snapshotWorktreePath && snapshotWorktreePath === selectedProjectPath);

    return describeWorkspaceSnapshotRestoreReadiness(snapshot, {
      liveTerminalSessionIDs: embeddedTerminalSessions.map((session) => session.sessionId),
      knownWorktreePaths: shouldUseCurrentWorktreeScan
        ? projectWorktrees.map((worktree) => worktree.path)
        : []
    });
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
      const matchingSession = sessions.find((session) => session.sessionId === savedTerminal.sessionID);

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

  async function activateWorkspaceSnapshotProject(
    project: ProjectRoot,
    restoredSelectedPath: string | null
  ) {
    await activateProject(project, {
      scanLimit: expandedSourceScanLimit,
      projects: mergeProjectRoots(defaultProjectRoots, customProjectRoots),
      waitForScan: false,
      preserveSelectedRecordOnScan: Boolean(restoredSelectedPath)
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

  function workspaceSnapshotForAgentSession(session: AgentSession): WorkspaceSnapshot | null {
    const snapshotID = workspaceSnapshotIDForAgentSession(session);
    return workspaceSnapshots.find((snapshot) => snapshot.id === snapshotID) ?? null;
  }

  function agentSessionForWorkspaceSnapshot(snapshot: WorkspaceSnapshot): AgentSession | null {
    return agentSessions.find((session) => workspaceSnapshotIDForAgentSession(session) === snapshot.id)
      ?? null;
  }

  async function openConversationWorkspaceSnapshotEmbeddedTerminal(snapshot: WorkspaceSnapshot) {
    const session = agentSessionForWorkspaceSnapshot(snapshot);
    if (session) {
      await switchToConversationWorkspace(session);
      return;
    }

    captureActiveWorkspaceBeforeSwitch();
    await openWorkspaceSnapshotEmbeddedTerminal(snapshot);
  }

  function embeddedTerminalSnapshotForSession(session: TerminalSessionInfo): WorkspaceSnapshotEmbeddedTerminal {
    return {
      sessionID: session.sessionId,
      cwd: session.cwd,
      shell: session.shell,
      startedAt: session.startedAt
    };
  }

  function embeddedTerminalSessionForSnapshot(snapshot: WorkspaceSnapshot) {
    const savedSessionID = snapshot.embeddedTerminal?.sessionID?.trim();
    if (!savedSessionID) return null;

    return embeddedTerminalSessions.find((session) => session.sessionId === savedSessionID)
      ?? (embeddedTerminalSession?.sessionId === savedSessionID ? embeddedTerminalSession : null);
  }

  function bindWorkspaceSnapshotEmbeddedTerminal(
    snapshot: WorkspaceSnapshot,
    session: TerminalSessionInfo
  ) {
    const nextSnapshot: WorkspaceSnapshot = {
      ...snapshot,
      embeddedTerminal: embeddedTerminalSnapshotForSession(session),
      capturedAt: Date.now()
    };
    const nextSnapshots = upsertWorkspaceSnapshot(workspaceSnapshots, nextSnapshot, maxWorkspaceSnapshots);
    workspaceSnapshots = nextSnapshots;
    persistWorkspaceSnapshots(nextSnapshots);
  }

  function worktreeWorkspaceSnapshots(worktree: ProjectWorktree, limit = 3): WorkspaceSnapshot[] {
    return workspaceSnapshotsForWorktreePath(workspaceSnapshots, worktree.path, limit);
  }

  function latestWorktreeWorkspaceSnapshot(worktree: ProjectWorktree): WorkspaceSnapshot | null {
    return worktreeWorkspaceSnapshots(worktree, 1)[0] ?? null;
  }

  function worktreeActiveSessions(worktree: ProjectWorktree, limit = 3): AgentSession[] {
    const worktreePath = normalizeProjectPath(worktree.path);
    const maxSessions = Math.max(0, Math.floor(limit));
    if (!worktreePath || maxSessions === 0) return [];

    return selectedProjectAgentSessions
      .filter((session) => {
        const sessionPath = normalizeProjectPath(agentSessionProjectPath(session));
        return sessionPath === worktreePath || sessionPath.startsWith(`${worktreePath}/`);
      })
      .slice(0, maxSessions);
  }

  function worktreeOwnerChips(
    worktree: ProjectWorktree,
    safety: ReturnType<typeof projectWorktreeSafety>
  ): WorktreeOwnerChip[] {
    const snapshots = worktreeWorkspaceSnapshots(worktree, 6);
    const activeSessions = worktreeActiveSessions(worktree, 3);
    const latestSnapshot = snapshots[0] ?? null;
    const chips: WorktreeOwnerChip[] = [];

    if (activeSessions.length > 0 || safety.activeSessionCount > 0) {
      const liveCount = Math.max(activeSessions.length, safety.activeSessionCount);
      chips.push({
        id: 'live-sessions',
        label: `${liveCount} live`,
        title: activeSessions.length > 0
          ? activeSessions.map((session) => `${session.provider}: ${session.title}`).join('\n')
          : safety.reason,
        tone: 'live'
      });
    }

    if (latestSnapshot) {
      chips.push({
        id: 'saved-workspaces',
        label: snapshots.length === 1 ? '1 saved' : `${snapshots.length} saved`,
        title: `${latestSnapshot.title}\n${workspaceSnapshotEnvironmentLabel(latestSnapshot)}`,
        tone: 'saved'
      });

      chips.push({
        id: 'latest-provider',
        label: latestSnapshot.provider,
        title: `${latestSnapshot.provider} ${latestSnapshot.model ?? ''}`.trim(),
        tone: 'muted'
      });

      if (
        latestSnapshot.branch &&
        worktree.branch &&
        latestSnapshot.branch !== worktree.branch
      ) {
        chips.push({
          id: 'branch-mismatch',
          label: 'branch drift',
          title: `Saved on ${latestSnapshot.branch}; worktree is ${worktree.branch}`,
          tone: 'warning'
        });
      }
    }

    if (chips.length === 0) {
      chips.push({
        id: 'no-owner',
        label: 'no snapshot',
        title: 'No saved conversation/session is tied to this worktree yet.',
        tone: 'muted'
      });
    }

    return chips.slice(0, 4);
  }

  function worktreeWorkspaceSnapshotLabel(worktree: ProjectWorktree) {
    const snapshots = worktreeWorkspaceSnapshots(worktree);
    const latestSnapshot = snapshots[0] ?? null;
    if (!latestSnapshot) return 'No saved workspace';

    const countLabel = snapshots.length === 1 ? '1 saved' : `${snapshots.length} saved`;
    return `${countLabel} · ${latestSnapshot.provider} · ${latestSnapshot.title}`;
  }

  function worktreeWorkspaceSnapshotTitle(worktree: ProjectWorktree) {
    const latestSnapshot = latestWorktreeWorkspaceSnapshot(worktree);
    if (!latestSnapshot) return 'No saved workspace';

    return `Restore ${latestSnapshot.title} · ${workspaceSnapshotEnvironmentLabel(latestSnapshot)}`;
  }

  function workspaceSnapshotIDForAgentSession(session: AgentSession) {
    const provider = workspaceSnapshotProviderForSession(session);
    return `${provider}:${workspaceSnapshotSessionIDForSession(session)}`;
  }

  function agentSessionResumePlan(session: AgentSession) {
    const commands = agentSessionResumeCommandList(session);
    const alternateCommands = commands.slice(1);
    const workspaceLines = agentSessionWorkspaceSummaryLines(session);

    return [
      'Session focus plan',
      `Session: ${session.title}`,
      `Provider: ${session.provider}`,
      `ID: ${session.id}`,
      `Model: ${agentSessionModelLabel(session)}`,
      `Project: ${agentSessionProjectPath(session)}`,
      `Activity: ${agentSessionActivityLabel(session)}`,
      `Terminal app: ${sourceTerminalApp}`,
      '',
      'Workspace:',
      ...workspaceLines,
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
      case 'rust':
        return 'rustup component add rust-analyzer';
      case 'svelte':
        return 'npm install -g svelte-language-server typescript';
      case 'python':
        return 'npm install -g pyright';
      case 'go':
        return 'go install golang.org/x/tools/gopls@latest';
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

  function sourceLspReadinessReportLines(statuses: SourceLspStatus[] | null) {
    if (!statuses) {
      return [
        `Project: ${selectedProject.name}`,
        `Root: ${selectedProject.path}`,
        'Runtime: native Tauri unavailable',
        'Open the Tauri app to validate local language servers.'
      ];
    }

    return [
      `Project: ${selectedProject.name}`,
      `Root: ${selectedProject.path}`,
      `Language servers: ${statuses.filter((status) => status.available).length}/${statuses.length} available`,
      '',
      ...statuses.flatMap((status) => {
        const runtimeCommand = sourceLspRuntimeCommand(status);
        const installCommand = sourceLspInstallCommand(status.language);
        const lines = [
          `${status.languageID || status.language}: ${status.available ? 'available' : status.reason || 'unavailable'}`,
          `  Server: ${status.serverName}`,
          runtimeCommand ? `  Command: ${runtimeCommand}` : ''
        ];
        if (!status.available && installCommand) {
          lines.push(`  Install: ${installCommand}`);
        }
        return lines.filter(Boolean);
      })
    ];
  }

  function sourceLspReadinessReport(statuses: SourceLspStatus[] | null) {
    return sourceLspReadinessReportLines(statuses).join('\n');
  }

  function copySourceLspStatusReport() {
    return copyActivityCommand(sourceLspStatusReport(), 'Language server status copied');
  }

  async function copySourceLspReadinessReport() {
    const statuses = await readSourceLspReadinessFromTauri(selectedProject.path);
    return copyActivityCommand(sourceLspReadinessReport(statuses), 'Language server readiness copied');
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
      activeSessionPaths: selectedProjectAgentSessionPaths,
      savedWorkspacePaths: worktreeWorkspaceSnapshots(worktree, 10).map((snapshot) => snapshot.worktreePath)
    });
  }

  function projectWorktreeGitSummary(worktree: ProjectWorktree): GitRepositorySummary | null {
    const worktreePath = normalizeProjectPath(worktree.path);
    if (!worktreePath) return null;

    return gitRepositorySummaries.find((summary) => normalizeProjectPath(summary.path) === worktreePath) ?? null;
  }

  function projectWorktreeLastActivityAgeDays(worktree: ProjectWorktree) {
    const lastActivityTime = Date.parse(worktree.lastActivity ?? '');
    if (!Number.isFinite(lastActivityTime)) return null;

    return Math.max(0, Math.floor((Date.now() - lastActivityTime) / 86_400_000));
  }

  function projectWorktreeCleanupPlan(worktree: ProjectWorktree) {
    const safety = projectWorktreeSafety(worktree);
    const gitSummary = projectWorktreeGitSummary(worktree);
    const taskID = normalizeGitTaskID(worktree.taskID);

    return buildWorktreeCleanupPlan(
      {
        repoName: worktree.repo,
        repoRootPath: selectedProject.path,
        path: worktree.path,
        branch: worktree.branch,
        isMainRoot: normalizeProjectPath(worktree.path) === normalizeProjectPath(selectedProject.path),
        isProtected: safety.kind === 'protected',
        dirtyCount: gitSummary?.unstagedCount ?? (worktree.isDirty ? 1 : 0),
        stagedCount: gitSummary?.stagedCount ?? 0,
        untrackedCount: gitSummary?.untrackedCount ?? 0,
        aheadCount: gitSummary?.ahead ?? (worktree.hasUnmergedCommits ? 1 : 0),
        behindCount: gitSummary?.behind ?? 0,
        hasUpstream: gitSummary?.hasUpstream ?? !worktree.hasUnmergedCommits,
        lastActivityAgeDays: projectWorktreeLastActivityAgeDays(worktree),
        activeSessionCount: safety.activeSessionCount,
        savedWorkspaceCount: safety.savedWorkspaceCount,
        isLocked: worktree.isLocked,
        lockedReason: worktree.lockedReason,
        isPrunable: worktree.isPrunable,
        prunableReason: worktree.prunableReason,
        deleteEligibility: worktree.deleteEligibility,
        task: taskID
          ? {
              id: taskID,
              url: gitTaskUrl(taskID),
              title: sourceProjectNameForWorktree(worktree)
            }
          : null
      },
      {
        staleAfterDays: 14,
        backupDirectory: `${selectedProject.path}/.worktree-cleanup-backups`
      }
    );
  }

  function worktreeCleanupPlanLabel(plan: ReturnType<typeof projectWorktreeCleanupPlan>) {
    if (plan.lane === 'safe-remove') return 'remove';
    if (plan.lane === 'backup-first') return 'backup';
    if (plan.lane === 'review-first') return 'review';
    if (plan.lane === 'blocked-active-session') return 'active';
    if (plan.lane === 'blocked-protected') return 'keep';
    if (plan.lane === 'blocked-locked') return 'locked';
    if (plan.lane === 'review-prunable') return 'prune';
    if (plan.lane === 'review-saved-workspace') return 'saved';
    if (plan.lane === 'review-confirmation') return 'review';
    return 'keep';
  }

  function worktreeCleanupPlanNextStep(plan: ReturnType<typeof projectWorktreeCleanupPlan>) {
    return plan.commandPlan.steps[0] ?? plan.explanation;
  }

  function formatWorktreeCleanupPlanReport(
    worktree: ProjectWorktree,
    plan: ReturnType<typeof projectWorktreeCleanupPlan>
  ) {
    const taskLine = plan.taskDisplay
      ? `Task: ${plan.taskDisplay.href ? `${plan.taskDisplay.id} ${plan.taskDisplay.href}` : plan.taskDisplay.id}`
      : 'Task: none';

    return [
      `Worktree: ${worktree.branch || '(detached)'}`,
      `Repo: ${worktree.repo}`,
      `Path: ${worktree.path}`,
      taskLine,
      `Lane: ${plan.lane}`,
      `State: ${plan.explanation}`,
      '',
      'Command plan:',
      ...plan.commandPlan.steps.map((step, index) => `${index + 1}. ${step}`)
    ].join('\n');
  }

  function projectWorktreeCleanupRunbookItem(worktree: ProjectWorktree) {
    const worktreePath = normalizeProjectPath(worktree.path);
    return (
      projectWorktreeCleanupRunbook.items.find((item) => normalizeProjectPath(item.path) === worktreePath) ?? null
    );
  }

  function projectWorktreeRunbookCommandBlock(worktree: ProjectWorktree) {
    return projectWorktreeCleanupRunbookItem(worktree)?.commandBlocks[0] ?? null;
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

    rememberSourceActivityFilter();
    sourceActivityMode = 'files';
    setSourceActivityFilter('');
    persistSourceActivityMode(sourceActivityMode);
    fileActionStatus = `Opening ${worktree.branch} source tree`;
    void addCustomProjectRoot(sourceProjectNameForWorktree(worktree), worktree.path, false);
    return true;
  }

  function copyWorktreeCleanupPlan(worktree: ProjectWorktree) {
    return copyActivityCommand(
      formatWorktreeCleanupPlanReport(worktree, projectWorktreeCleanupPlan(worktree)),
      'Worktree cleanup plan copied'
    );
  }

  function copyWorktreeRunbookCommandBlock(worktree: ProjectWorktree) {
    const item = projectWorktreeCleanupRunbookItem(worktree);
    const commandBlock = item?.commandBlocks[0] ?? null;
    const copyText =
      commandBlock?.copyText ??
      item?.copyText ??
      formatWorktreeCleanupPlanReport(worktree, projectWorktreeCleanupPlan(worktree));

    return copyActivityCommand(
      copyText,
      commandBlock ? 'Worktree runbook command block copied' : 'Worktree runbook item copied'
    );
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

  function confirmWorktreePrimaryAction(
    worktree: ProjectWorktree,
    action: ReturnType<typeof worktreePrimaryAction>
  ) {
    if (action.kind !== 'cleanup') return true;
    if (typeof window === 'undefined') return false;

    return window.confirm(
      [
        `Remove clean worktree ${worktree.branch}?`,
        worktree.path,
        '',
        'This runs the native guarded worktree remove action. Dirty or protected worktrees are still refused.'
      ].join('\n')
    );
  }

  async function runWorktreePrimaryAction(worktree: ProjectWorktree) {
    const safety = projectWorktreeSafety(worktree);
    const action = worktreePrimaryAction(safety);
    if (action.kind === 'audit') {
      await copyActivityCommand(action.command, action.clipboardMessage);
      return;
    }
    if (!confirmWorktreePrimaryAction(worktree, action)) {
      fileActionStatus = 'Worktree cleanup cancelled';
      return;
    }

    const busyKey = `worktree-primary:${worktree.path}`;
    fileActionBusy = busyKey;
    fileActionStatus = '';
    error = '';

    try {
      if (action.kind === 'backup') {
        const result = await archiveProjectWorktreeFromTauri(selectedProject.path, worktree.path);
        if (!result) {
          await copyTextToClipboard(action.command, 'Native archive unavailable; command copied');
          return;
        }

        projectWorktrees = result.worktrees;
        fileActionStatus = `${result.message}: ${result.archivePath}`;
        void loadGitRepositorySummaries(projects);
        return;
      }

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

  function copyProjectWorktreeCleanupRunbook() {
    return copyActivityCommand(projectWorktreeCleanupRunbook.copyText, 'Worktree cleanup runbook copied');
  }

  function copyProjectWorktreeCleanupScript() {
    const script = buildWorktreeCleanupScript(projectWorktrees, {
      primaryPath: selectedProject.path,
      activeSessionPaths: selectedProjectAgentSessionPaths
    });

    return copyActivityCommand(script, 'Guarded worktree cleanup script copied');
  }

  function copyAgentSessionResumePlan(session: AgentSession) {
    return copyActivityCommand(agentSessionResumePlan(session), 'Session focus plan copied');
  }

  function copyAgentSessionResumeShellCommand(session: AgentSession) {
    return copyActivityCommand(agentSessionResumeShellCommand(session), 'Shell resume command copied');
  }

  function runAgentSessionFocusLane(session: AgentSession, lane: AgentSessionFocusLane) {
    switch (lane.action) {
      case 'repair':
        return copyAgentSessionWorkspaceRepairPlan(session);
      case 'restore':
      case 'files-only':
        return openAgentSessionWorkspace(session);
      case 'reattach':
      case 'resume':
        return switchToConversationWorkspace(session);
      case 'save-workspace':
        captureAgentSessionWorkspaceSnapshot(session);
        return;
    }
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
      playwrightSessions.length
        ? `${playwrightSessions.length} Playwright`
        : '',
      projectWorktrees.length
        ? `${projectWorktrees.length} ${projectWorktrees.length === 1 ? 'worktree' : 'worktrees'}`
        : ''
    ].filter(Boolean);

    return parts.join(' · ') || sourceTerminalApp;
  }

  type ExternalTerminalOpenResult = {
    opened: boolean;
    app: SourceTerminalApp;
    usedFallback: boolean;
    fallbackFrom?: SourceTerminalApp;
  };

  function terminalActionErrorMessage(terminalError: unknown) {
    return terminalError instanceof Error ? terminalError.message : String(terminalError ?? '');
  }

  function shouldFallbackExternalTerminal(terminalError: unknown, app: SourceTerminalApp) {
    if (app === sourceTerminalFallbackApp) return false;

    const message = terminalActionErrorMessage(terminalError);
    if (/unsupported terminal app/i.test(message)) return false;
    if (/terminal command execution is not supported/i.test(message)) return false;

    return true;
  }

  function persistSourceTerminalFallback(app: SourceTerminalApp) {
    sourceTerminalApp = app;
    persistSourceTerminalApp(app);
  }

  async function openTerminalPathWithFallback(
    path: string,
    preferredApp: SourceTerminalApp
  ): Promise<ExternalTerminalOpenResult> {
    try {
      const opened = await openTerminalPathFromTauri(path, preferredApp);
      return { opened, app: preferredApp, usedFallback: false };
    } catch (terminalError) {
      if (!shouldFallbackExternalTerminal(terminalError, preferredApp)) throw terminalError;

      const opened = await openTerminalPathFromTauri(path, sourceTerminalFallbackApp);
      if (opened) persistSourceTerminalFallback(sourceTerminalFallbackApp);
      return {
        opened,
        app: sourceTerminalFallbackApp,
        usedFallback: true,
        fallbackFrom: preferredApp
      };
    }
  }

  async function openTerminalCommandWithFallback(
    path: string,
    command: string,
    preferredApp: SourceTerminalApp
  ): Promise<ExternalTerminalOpenResult> {
    try {
      const opened = await openTerminalCommandFromTauri(path, command, preferredApp);
      return { opened, app: preferredApp, usedFallback: false };
    } catch (terminalError) {
      if (!shouldFallbackExternalTerminal(terminalError, preferredApp)) throw terminalError;

      const opened = await openTerminalCommandFromTauri(path, command, sourceTerminalFallbackApp);
      if (opened) persistSourceTerminalFallback(sourceTerminalFallbackApp);
      return {
        opened,
        app: sourceTerminalFallbackApp,
        usedFallback: true,
        fallbackFrom: preferredApp
      };
    }
  }

  function terminalOpenStatus(
    result: ExternalTerminalOpenResult,
    normalStatus: string,
    fallbackStatus: string
  ) {
    if (!result.usedFallback || !result.fallbackFrom) return normalStatus;
    return `${result.fallbackFrom} unavailable; ${fallbackStatus}`;
  }

  function embeddedTerminalAddonLabel() {
    const addonParts = embeddedTerminalAddonStatus
      .split(',')
      .map((part) => part.trim())
      .filter((part) => part && (part !== 'webgl' || embeddedTerminalWebglAddon));
    if (embeddedTerminalSearchAddon && !addonParts.includes('search')) addonParts.push('search');
    if (embeddedTerminalSerializeAddon && !addonParts.includes('serialize')) addonParts.push('serialize');
    if (embeddedTerminalWebglAddon && !addonParts.includes('webgl')) addonParts.push('webgl');
    return addonParts.join(', ') || 'fit';
  }

  function openEmbeddedTerminalWebLink(event: MouseEvent, uri: string) {
    event.preventDefault();
    window.open(uri, '_blank', 'noopener,noreferrer');
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

  async function loadPlaywrightSessions(force = false) {
    if (playwrightSessionsLoading && !force) return;

    const requestID = ++playwrightSessionsRequestID;
    playwrightSessionsLoading = true;
    playwrightSessionError = '';

    try {
      const sessions = await listPlaywrightSessionsFromTauri();
      if (requestID !== playwrightSessionsRequestID) return;
      if (!sessions) {
        playwrightSessions = [];
        playwrightSessionSource = 'browser preview';
        return;
      }

      playwrightSessions = sessions;
      playwrightSessionSource = 'native process scan';
    } catch (sessionError) {
      if (requestID !== playwrightSessionsRequestID) return;
      playwrightSessions = [];
      playwrightSessionSource = 'native process scan';
      playwrightSessionError =
        sessionError instanceof Error ? sessionError.message : 'Could not scan Playwright sessions';
    } finally {
      if (requestID === playwrightSessionsRequestID) {
        playwrightSessionsLoading = false;
      }
    }
  }

  async function killPlaywrightSessions() {
    if (playwrightSessionsKilling) return;

    playwrightSessionsKilling = true;
    playwrightSessionError = '';

    try {
      const result = await killPlaywrightSessionsFromTauri();
      if (!result) {
        playwrightSessionError = 'Playwright cleanup runs in the Tauri app.';
        fileActionStatus = 'Open the Tauri app to stop Playwright sessions';
        return;
      }

      const stoppedCount = result.terminatedPids.length;
      const failedCount = result.failedPgids.length;
      fileActionStatus = failedCount
        ? `Stopped ${stoppedCount} Playwright processes; ${failedCount} failed`
        : stoppedCount
          ? `Stopped ${stoppedCount} Playwright processes`
          : 'No Playwright sessions to stop';
      await loadPlaywrightSessions(true);
    } catch (cleanupError) {
      playwrightSessionError =
        cleanupError instanceof Error ? cleanupError.message : 'Could not stop Playwright sessions';
      fileActionStatus = 'Playwright cleanup failed';
    } finally {
      playwrightSessionsKilling = false;
    }
  }

  function embeddedTerminalHostReady() {
    if (!embeddedTerminalElement) return false;
    const rect = embeddedTerminalElement.getBoundingClientRect();
    const width = Math.round(rect.width || embeddedTerminalElement.clientWidth);
    const height = Math.round(rect.height || embeddedTerminalElement.clientHeight);
    return width >= 40 && height >= 40;
  }

  function remountEmbeddedTerminalRenderer() {
    if (!embeddedTerminal || !embeddedTerminalElement || !embeddedTerminal.element) return;
    if (embeddedTerminal.element.parentElement === embeddedTerminalElement) return;

    embeddedTerminalElement.replaceChildren(embeddedTerminal.element);
    scheduleEmbeddedTerminalFit();
  }

  async function ensureEmbeddedTerminalRenderer() {
    if (embeddedTerminal) {
      remountEmbeddedTerminalRenderer();
      return;
    }
    if (!embeddedTerminalElement) return;
    if (embeddedTerminalRendererPromise) {
      await embeddedTerminalRendererPromise;
      remountEmbeddedTerminalRenderer();
      return;
    }

    embeddedTerminalRendererLoading = true;
    embeddedTerminalStatus = 'Loading embedded terminal';

    embeddedTerminalRendererPromise = (async () => {
      try {
        const [
          { Terminal: XTerm },
          { FitAddon },
          { SearchAddon },
          { SerializeAddon },
          { UnicodeGraphemesAddon },
          { WebLinksAddon }
        ] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
          import('@xterm/addon-search'),
          import('@xterm/addon-serialize'),
          import('@xterm/addon-unicode-graphemes'),
          import('@xterm/addon-web-links')
        ]);

        if (!embeddedTerminalElement || embeddedTerminal) return;

        const terminal = new XTerm({
          convertEol: true,
          cursorBlink: true,
          cursorStyle: 'block',
          allowProposedApi: true,
          macOptionIsMeta: true,
          // Sourced from settings.terminal via embeddedTerminalAppearance, which
          // falls back to the exact original hardcoded defaults when unchanged.
          fontFamily: embeddedTerminalAppearance.fontFamily,
          fontSize: embeddedTerminalAppearance.fontSize,
          fontWeight: 500,
          fontWeightBold: 760,
          lineHeight: embeddedTerminalAppearance.lineHeight,
          scrollback: 8000,
          theme: {
            background: '#282a36',
            foreground: '#f8f8f2',
            cursor: '#f8f8f2',
            cursorAccent: '#282a36',
            selectionBackground: '#44475a',
            black: '#000000',
            red: '#ff5555',
            green: '#50fa7b',
            yellow: '#f1fa8c',
            blue: '#bd93f9',
            magenta: '#ff79c6',
            cyan: '#8be9fd',
            white: '#bbbbbb',
            brightBlack: '#555555',
            brightRed: '#ff5555',
            brightGreen: '#50fa7b',
            brightYellow: '#f1fa8c',
            brightBlue: '#caa9fa',
            brightMagenta: '#ff79c6',
            brightCyan: '#8be9fd',
            brightWhite: '#ffffff'
          }
        });
        const fitAddon = new FitAddon();
        const searchAddon = new SearchAddon({ highlightLimit: 2000 });
        const serializeAddon = new SerializeAddon();
        const unicodeGraphemesAddon = new UnicodeGraphemesAddon();
        const webLinksAddon = new WebLinksAddon(openEmbeddedTerminalWebLink);
        terminal.loadAddon(fitAddon);
        terminal.loadAddon(searchAddon);
        terminal.loadAddon(serializeAddon);
        terminal.loadAddon(unicodeGraphemesAddon);
        terminal.loadAddon(webLinksAddon);
        terminal.open(embeddedTerminalElement);
        let webglAddon: XTermWebglAddon | null = null;
        try {
          const { WebglAddon } = await import('@xterm/addon-webgl');
          webglAddon = new WebglAddon();
          webglAddon.onContextLoss(() => {
            webglAddon?.dispose();
            if (embeddedTerminalWebglAddon === webglAddon) {
              embeddedTerminalWebglAddon = null;
              embeddedTerminalAddonStatus = embeddedTerminalAddonLabel();
            }
          });
          terminal.loadAddon(webglAddon);
        } catch {
          webglAddon = null;
        }
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
        embeddedTerminalSearchAddon = searchAddon;
        embeddedTerminalSerializeAddon = serializeAddon;
        embeddedTerminalWebglAddon = webglAddon;
        embeddedTerminalAddonStatus = ['fit', 'search', 'serialize', 'unicode', 'links', webglAddon ? 'webgl' : 'canvas']
          .filter(Boolean)
          .join(', ');
        embeddedTerminalStatus = 'Embedded terminal ready';
        scheduleEmbeddedTerminalFit();
      } catch (terminalError) {
        embeddedTerminalError =
          terminalError instanceof Error ? terminalError.message : 'Could not load embedded terminal';
        embeddedTerminalStatus = 'Embedded terminal unavailable';
      } finally {
        embeddedTerminalRendererLoading = false;
      }
    })();

    try {
      await embeddedTerminalRendererPromise;
    } finally {
      embeddedTerminalRendererPromise = null;
    }
  }

  // A cwd is unusable when it is empty or points at an OS temp/ephemeral dir
  // (e.g. /private/var/folders/.../T). Spawning a PTY there makes the shell
  // error out (compinit/compdef failures) so the agent session cannot resume.
  function looksLikeEphemeralCwd(path: string): boolean {
    const normalized = normalizeProjectPath(path);
    if (!normalized) return true;
    return (
      normalized.startsWith('/private/var/folders/') ||
      normalized.startsWith('/var/folders/') ||
      normalized === '/tmp' ||
      normalized.startsWith('/tmp/') ||
      normalized === '/private/tmp' ||
      normalized.startsWith('/private/tmp/')
    );
  }

  // Best-effort $HOME for the running user. There is no frontend home-dir API,
  // but every known project path lives under /Users/<user>, so derive it from
  // the active project (falling back to the bundled repo path).
  function homeDirGuess(): string {
    const candidates = [selectedProject?.path, macCommandBarRepoPath];
    for (const candidate of candidates) {
      const normalized = normalizeProjectPath(candidate ?? '');
      const match = normalized.match(/^(\/Users\/[^/]+)(?:\/|$)/);
      if (match) return match[1];
    }
    return '';
  }

  // Resolve the first candidate that exists, is a directory, and is not an
  // ephemeral temp dir. Uses the existing validate_project_root bridge (which
  // works in both the Tauri runtime and the dev bridge) and also considers the
  // detected git root of any otherwise-rejected candidate.
  async function firstValidTerminalCwd(candidates: Array<string | null | undefined>): Promise<string> {
    const seen = new Set<string>();
    const queue = candidates
      .map((candidate) => normalizeProjectPath(candidate ?? ''))
      .filter((candidate) => candidate.length > 0);

    for (let index = 0; index < queue.length; index += 1) {
      const candidate = queue[index];
      if (seen.has(candidate)) continue;
      seen.add(candidate);
      if (looksLikeEphemeralCwd(candidate)) continue;

      let validation: ProjectRootValidationResult | null = null;
      try {
        validation = await validateProjectRootFromTauri(candidate);
      } catch {
        validation = null;
      }

      if (validation?.exists && validation.isDirectory) {
        return candidate;
      }

      // If the candidate itself is gone but its git root survives, try that next.
      const gitRoot = normalizeProjectPath(validation?.gitRoot ?? '');
      if (gitRoot && !seen.has(gitRoot) && !looksLikeEphemeralCwd(gitRoot)) {
        queue.push(gitRoot);
      }
    }

    return '';
  }

  async function startEmbeddedTerminalSession(
    cwd = selectedProject.path,
    startupCommand = '',
    fallbackRoots: Array<string | null | undefined> = []
  ) {
    const requestedRoot = cwd.trim();
    if (!requestedRoot || embeddedTerminalStarting) return null;
    const command = startupCommand.trim();

    embeddedTerminalStarting = true;
    embeddedTerminalError = '';
    embeddedTerminalStatus = command ? 'Starting embedded command' : 'Starting native terminal';

    try {
      await ensureEmbeddedTerminalRenderer();
      if (!embeddedTerminal) {
        embeddedTerminalStatus = 'Embedded terminal unavailable';
        return null;
      }

      // Resolution order: requested cwd → caller fallbacks (project/git roots) →
      // active project path → $HOME. Never spawn in a dead/ephemeral directory.
      const home = homeDirGuess();
      const resolvedRoot = await firstValidTerminalCwd([
        requestedRoot,
        ...fallbackRoots,
        selectedProject?.path,
        home
      ]);
      let root = resolvedRoot || (home ? home : requestedRoot);
      const usedFallback = normalizeProjectPath(root) !== normalizeProjectPath(requestedRoot);
      if (usedFallback) {
        fileActionStatus = `Working directory ‹${requestedRoot}› is unavailable — opened terminal in ‹${root}›.`;
      }

      fitEmbeddedTerminal();
      let session = await startTerminalSessionFromTauri({
        cwd: root,
        cols: embeddedTerminal.cols || 96,
        rows: embeddedTerminal.rows || 24
      }).catch((startError) => {
        // A bad cwd can reject the native call outright; retry once at $HOME.
        if (home && normalizeProjectPath(home) !== normalizeProjectPath(root)) {
          return null;
        }
        throw startError;
      });

      if (!session && home && normalizeProjectPath(home) !== normalizeProjectPath(root)) {
        const retryRoot = root;
        root = home;
        session = await startTerminalSessionFromTauri({
          cwd: root,
          cols: embeddedTerminal.cols || 96,
          rows: embeddedTerminal.rows || 24
        });
        if (session) {
          fileActionStatus = `Working directory ‹${retryRoot}› is unavailable — opened terminal in ‹${root}›.`;
        }
      }

      if (!session) {
        embeddedTerminalError = 'Embedded terminal sessions run inside the Tauri app.';
        embeddedTerminalStatus = 'Browser preview cannot start a native PTY';
        embeddedTerminal.writeln('\r\nEmbedded terminal is available in the Tauri app.');
        embeddedTerminal.writeln(`Use Project to open ${sourceTerminalApp} from browser preview.\r\n`);
        return null;
      }

      embeddedTerminalSession = session;
      embeddedTerminalSessions = upsertEmbeddedTerminalSession(embeddedTerminalSessions, session);
      embeddedTerminalStatus = command ? 'Running embedded command' : 'Native terminal running';
      embeddedTerminal.reset();
      embeddedTerminal.focus();
      scheduleEmbeddedTerminalFit();
      if (command) {
        await writeTerminalSessionFromTauri(session.sessionId, `${command}\r`);
      }
      return session;
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not start embedded terminal';
      embeddedTerminalStatus = 'Embedded terminal failed';
      return null;
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

    const matchingSession = embeddedTerminalSessionForPath(root);
    if (matchingSession) {
      await attachEmbeddedTerminalSession(matchingSession);
      return;
    }

    await startEmbeddedTerminalSession(root);
  }

  async function attachOrStartProjectEmbeddedTerminal() {
    await openPathEmbeddedTerminal(selectedProject.path);
  }

  async function submitEmbeddedTerminalCommand(event?: SubmitEvent) {
    event?.preventDefault();
    const command = embeddedTerminalCommandDraft.trim();
    if (!command || embeddedTerminalStarting) return;

    showDockPanel('terminal');
    await tick();

    embeddedTerminalError = '';

    try {
      if (embeddedTerminalSession) {
        await ensureEmbeddedTerminalRenderer();
        await writeTerminalSessionFromTauri(embeddedTerminalSession.sessionId, `${command}\r`);
        embeddedTerminalStatus = 'Sent command to embedded terminal';
        embeddedTerminalCommandDraft = '';
        embeddedTerminal?.focus();
        return;
      }

      const root = selectedProject.path.trim();
      if (!root) {
        embeddedTerminalError = 'Choose a project or attach a terminal first';
        embeddedTerminalStatus = 'No terminal command target';
        return;
      }

      await startEmbeddedTerminalSession(root, command);
      embeddedTerminalCommandDraft = '';
    } catch (terminalError) {
      embeddedTerminalError =
        terminalError instanceof Error ? terminalError.message : 'Could not run terminal command';
      embeddedTerminalStatus = 'Terminal command failed';
    }
  }

  function embeddedTerminalSessionForPath(path: string) {
    const root = normalizeProjectPath(path);
    if (!root) return null;

    return embeddedTerminalSessions.find(
      (session) => normalizeProjectPath(session.cwd) === root
    ) ?? null;
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
      scheduleEmbeddedTerminalFit();
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
    if (!embeddedTerminalHostReady()) {
      embeddedTerminalStatus = 'Terminal fit pending';
      return;
    }

    try {
      remountEmbeddedTerminalRenderer();
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

  function clearEmbeddedTerminalFitSchedule() {
    if (typeof window === 'undefined') return;

    if (embeddedTerminalFitFrame) {
      window.cancelAnimationFrame(embeddedTerminalFitFrame);
      embeddedTerminalFitFrame = 0;
    }
    for (const timer of embeddedTerminalFitTimers) {
      window.clearTimeout(timer);
    }
    embeddedTerminalFitTimers = [];
  }

  function scheduleEmbeddedTerminalFit() {
    if (typeof window === 'undefined') {
      fitEmbeddedTerminal();
      return;
    }

    clearEmbeddedTerminalFitSchedule();
    embeddedTerminalFitFrame = window.requestAnimationFrame(() => {
      embeddedTerminalFitFrame = 0;
      fitEmbeddedTerminal();
    });
    embeddedTerminalFitTimers = embeddedTerminalDeferredFitDelays.map((delay) =>
      window.setTimeout(fitEmbeddedTerminal, delay)
    );
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

    clearEmbeddedTerminalFitSchedule();
    embeddedTerminalInputDisposable?.dispose();
    embeddedTerminalInputDisposable = null;
    embeddedTerminalFitAddon = null;
    embeddedTerminalSearchAddon = null;
    embeddedTerminalSerializeAddon = null;
    embeddedTerminalWebglAddon = null;
    embeddedTerminalAddonStatus = 'fit';
    embeddedTerminal?.dispose();
    embeddedTerminal = null;
  }

  function embeddedTerminalStatusLabel(session = embeddedTerminalSession) {
    if (!session) return embeddedTerminalStatus;

    return `${embeddedTerminalSessionTitle(session)} · ${session.cols}x${session.rows}${session.pid ? ` · pid ${session.pid}` : ''} · ${embeddedTerminalAddonLabel()}`;
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
    selectedSourceGitDiffRequestID += 1;
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
    const requestID = ++selectedSourceGitDiffRequestID;
    selectedSourceGitDiff = null;
    selectedSourceGitDiffError = '';
    selectedSourceGitDiffLoading = true;

    try {
      const diff = await readSourceGitDiffFromTauri(expectedProjectPath, expectedPath);
      if (!isCurrentSelectedSourceGitDiffRequest(requestID, expectedPath, expectedProjectPath)) return;

      selectedSourceGitDiff = diff;
      if (!diff) selectedSourceGitDiffError = 'Native Git diff unavailable';
    } catch (gitDiffError) {
      if (!isCurrentSelectedSourceGitDiffRequest(requestID, expectedPath, expectedProjectPath)) return;

      selectedSourceGitDiffError =
        gitDiffError instanceof Error ? gitDiffError.message : 'Could not read Git diff';
    } finally {
      if (isCurrentSelectedSourceGitDiffRequest(requestID, expectedPath, expectedProjectPath)) {
        selectedSourceGitDiffLoading = false;
      }
    }
  }

  function isCurrentSelectedSourceGitDiffRequest(
    requestID: number,
    expectedPath: string,
    expectedProjectPath: string
  ) {
    return (
      requestID === selectedSourceGitDiffRequestID &&
      selectedRecord?.path === expectedPath &&
      selectedProject.path === expectedProjectPath
    );
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

  async function findSourceDefinitionTargetsForEditor(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    if (!normalizedSymbolName) return [];

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
      if (lspTargets?.length) return lspTargets;

      const nativeTargets = await findSourceDefinitionsFromTauri(
        records,
        normalizedSymbolName,
        maxSourceDefinitionResults
      );
      return (
        nativeTargets ??
        findSourceDefinitionTargets(
          records.map((record) => demoPreviewFor(record)),
          normalizedSymbolName,
          maxSourceDefinitionResults
        )
      );
    } catch {
      return findSourceDefinitionTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceDefinitionResults
      );
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

  async function findSourceReferenceTargetsForEditor(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    if (!normalizedSymbolName) return [];

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
      if (lspTargets?.length) return lspTargets;

      const nativeTargets = await findSourceReferencesFromTauri(
        records,
        normalizedSymbolName,
        maxSourceSearchResults
      );
      return (
        nativeTargets ??
        findSourceReferenceTargets(
          records.map((record) => demoPreviewFor(record)),
          normalizedSymbolName,
          maxSourceSearchResults
        )
      );
    } catch {
      return findSourceReferenceTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceSearchResults
      );
    }
  }

  async function countSourceReferencesForCodeLens(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    if (!normalizedSymbolName) return 0;

    try {
      const lspTargets =
        preview && sourceIntelligenceAvailable
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
      if (lspTargets?.length) return lspTargets.length;

      const nativeTargets = await findSourceReferencesFromTauri(
        records,
        normalizedSymbolName,
        maxSourceSearchResults
      ).catch(() => null);
      if (nativeTargets) return nativeTargets.length;

      return findSourceReferenceTargets(
        records.map((record) => demoPreviewFor(record)),
        normalizedSymbolName,
        maxSourceSearchResults
      ).length;
    } catch {
      return 0;
    }
  }

  async function loadEditorExternalSourcePreview(record: SourceRecord) {
    const sourceRecord = sourceRecordFromRestoredPath(selectedProject, record.path);

    if (preview?.path === sourceRecord.path) {
      return previewFromContent(
        { ...sourceRecord, byteCount: new TextEncoder().encode(selectedSourceDraftContent).length },
        selectedSourceDraftContent
      );
    }

    const draft = sourceDraftContentByPath[sourceRecord.path];
    const saved = savedSourceContentByPath[sourceRecord.path];
    if (draft !== undefined && saved !== undefined) {
      return previewFromContent(
        { ...sourceRecord, byteCount: new TextEncoder().encode(draft).length },
        draft
      );
    }

    const sourcePreview = await readSourceFromTauri(sourceRecord);
    if (sourcePreview) {
      syncSourcePreviewContent(sourcePreview);
      return sourcePreview;
    }

    const demoPreview = demoPreviewFor(sourceRecord);
    return demoPreview.content.length > 0 ? demoPreview : null;
  }

  async function navigateEditorExternalSource(request: { path: string; line: number; column: number }) {
    const record = sourceRecordFromRestoredPath(selectedProject, request.path);
    await selectRecord(record, request.line);
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

  async function findSourceImplementationTargetsForEditor(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    if (!normalizedSymbolName || !preview || !sourceIntelligenceAvailable) return [];

    try {
      return (
        (await findSourceLspImplementationsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.line,
            column: request.column,
            limit: maxSourceDefinitionResults
          }
        )) ?? []
      );
    } catch {
      return [];
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

  async function findSourceTypeDefinitionTargetsForEditor(request: SourceEditorLookupRequest) {
    const normalizedSymbolName = request.symbolName.trim();
    if (!normalizedSymbolName || !preview || !sourceIntelligenceAvailable) return [];

    try {
      return (
        (await findSourceLspTypeDefinitionsFromTauri(
          { ...preview, content: selectedSourceDraftContent },
          {
            root: selectedProject.path,
            line: request.line,
            column: request.column,
            limit: maxSourceDefinitionResults
          }
        )) ?? []
      );
    } catch {
      return [];
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

  function formatSourceLookupSummary() {
    if (sourceReferenceQuery || sourceReferenceTargets.length > 0 || sourceReferenceLoading) {
      return sourceReferenceSummary || 'References';
    }
    if (sourceImplementationQuery || sourceImplementationTargets.length > 0 || sourceImplementationLoading) {
      return sourceImplementationSummary || 'Implementations';
    }
    if (sourceTypeDefinitionQuery || sourceTypeDefinitionTargets.length > 0 || sourceTypeDefinitionLoading) {
      return sourceTypeDefinitionSummary || 'Type definitions';
    }
    if (sourceDefinitionQuery || sourceDefinitionTargets.length > 0 || sourceDefinitionLoading) {
      return sourceDefinitionSummary || 'Definitions';
    }
    return 'No lookups yet';
  }

  function openEditorNavPanel(panel: SourceEditorNavPanel) {
    editorNavPanel = panel;
  }

  function toggleEditorNavPanel(panel: SourceEditorNavPanel) {
    editorNavPanel = editorNavPanel === panel ? null : panel;
  }

  function closeEditorNavPanel() {
    editorNavPanel = null;
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

  function setSourceScanMode(projectID: string, mode: SourceScanEvidenceMode) {
    sourceScanModeByProject = { ...sourceScanModeByProject, [projectID]: mode };
  }

  function applySourceRecords(
    nextRecords: SourceRecord[],
    preferredPath: string | null | undefined,
    nextRuntime: string,
    truncated = false,
    stats: SourceScanStats | null = null,
    options: { preserveSelectedRecord?: boolean } = {}
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
    const shouldPreserveSelectedRecord =
      Boolean(options.preserveSelectedRecord) &&
      Boolean(preferredPath) &&
      selectedRecord?.path === preferredPath &&
      nextSelection?.path !== preferredPath;

    if (shouldPreserveSelectedRecord) {
      return null;
    }

    selectedRecord = nextSelection;
    selectedSourceLine = null;
    preview = nextSelection ? previewFromContent(nextSelection, '') : null;
    expandedFolderIds = nextSelection ? new Set(folderIdsForSourceRecord(nextSelection)) : new Set();
    clearSourceSearchResults();
    clearSourceDefinitionTargets();
    clearSourceReferenceTargets();
    clearSourceImplementationTargets();
    clearSourceTypeDefinitionTargets();
    closeEditorNavPanel();
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

      runtime = tauriPreview
        ? isNativeTauriRuntime()
          ? 'tauri file read'
          : 'browser source bridge'
        : 'browser preview';
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
      event.key === 'Escape' &&
      (activeActivityRowActionMenu || activeAgentRowActionMenu || activeWorktreeRowActionMenu || activeGitRowActionMenu)
    ) {
      event.preventDefault();
      closeRowActionMenus();
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
    if (editorActionMenuOpen) closeRowActionMenus();
  }

  function closeEditorActionMenu() {
    editorActionMenuOpen = false;
  }

  function activityRowActionMenuID(scope: string, id: string) {
    return `${scope}:${id}`;
  }

  function activityRowActionMenuOpen(scope: string, id: string) {
    return activeActivityRowActionMenu === activityRowActionMenuID(scope, id);
  }

  function openActivityRowActionMenu(scope: string, id: string) {
    activeActivityRowActionMenu = activityRowActionMenuID(scope, id);
    closeAgentRowActionMenu();
    closeWorktreeRowActionMenu();
    closeGitRowActionMenu();
    closeEditorActionMenu();
    closeViewMenu();
  }

  function toggleActivityRowActionMenu(scope: string, id: string) {
    const menuID = activityRowActionMenuID(scope, id);
    if (activeActivityRowActionMenu === menuID) {
      closeActivityRowActionMenu();
      return;
    }

    openActivityRowActionMenu(scope, id);
  }

  function closeActivityRowActionMenu() {
    activeActivityRowActionMenu = null;
  }

  function agentRowActionMenuID(session: AgentSession, scope: 'conversation' | 'agent') {
    return `${scope}:${session.provider}:${session.id}`;
  }

  function agentRowActionMenuOpen(session: AgentSession, scope: 'conversation' | 'agent') {
    return activeAgentRowActionMenu === agentRowActionMenuID(session, scope);
  }

  function openAgentRowActionMenu(session: AgentSession, scope: 'conversation' | 'agent') {
    activeAgentRowActionMenu = agentRowActionMenuID(session, scope);
    closeActivityRowActionMenu();
    closeWorktreeRowActionMenu();
    closeGitRowActionMenu();
    closeEditorActionMenu();
    closeViewMenu();
  }

  function toggleAgentRowActionMenu(session: AgentSession, scope: 'conversation' | 'agent') {
    const menuID = agentRowActionMenuID(session, scope);
    if (activeAgentRowActionMenu === menuID) {
      closeAgentRowActionMenu();
      return;
    }

    openAgentRowActionMenu(session, scope);
  }

  function closeAgentRowActionMenu() {
    activeAgentRowActionMenu = null;
  }

  function worktreeRowActionMenuID(worktree: ProjectWorktree, scope: 'activity' | 'context' | 'queue') {
    return `${scope}:${worktree.path}`;
  }

  function worktreeRowActionMenuOpen(worktree: ProjectWorktree, scope: 'activity' | 'context' | 'queue') {
    return activeWorktreeRowActionMenu === worktreeRowActionMenuID(worktree, scope);
  }

  function openWorktreeRowActionMenu(worktree: ProjectWorktree, scope: 'activity' | 'context' | 'queue') {
    activeWorktreeRowActionMenu = worktreeRowActionMenuID(worktree, scope);
    closeActivityRowActionMenu();
    closeAgentRowActionMenu();
    closeGitRowActionMenu();
    closeEditorActionMenu();
    closeViewMenu();
  }

  function toggleWorktreeRowActionMenu(worktree: ProjectWorktree, scope: 'activity' | 'context' | 'queue') {
    const menuID = worktreeRowActionMenuID(worktree, scope);
    if (activeWorktreeRowActionMenu === menuID) {
      closeWorktreeRowActionMenu();
      return;
    }

    openWorktreeRowActionMenu(worktree, scope);
  }

  function closeWorktreeRowActionMenu() {
    activeWorktreeRowActionMenu = null;
  }

  function gitRowActionMenuID(id: string, scope: 'repository' | 'task-ledger') {
    return `${scope}:${id}`;
  }

  function gitRowActionMenuOpen(id: string, scope: 'repository' | 'task-ledger') {
    return activeGitRowActionMenu === gitRowActionMenuID(id, scope);
  }

  function openGitRowActionMenu(id: string, scope: 'repository' | 'task-ledger') {
    activeGitRowActionMenu = gitRowActionMenuID(id, scope);
    closeActivityRowActionMenu();
    closeAgentRowActionMenu();
    closeWorktreeRowActionMenu();
    closeEditorActionMenu();
    closeViewMenu();
  }

  function toggleGitRowActionMenu(id: string, scope: 'repository' | 'task-ledger') {
    const menuID = gitRowActionMenuID(id, scope);
    if (activeGitRowActionMenu === menuID) {
      closeGitRowActionMenu();
      return;
    }

    openGitRowActionMenu(id, scope);
  }

  function closeGitRowActionMenu() {
    activeGitRowActionMenu = null;
  }

  function closeRowActionMenus() {
    closeActivityRowActionMenu();
    closeAgentRowActionMenu();
    closeWorktreeRowActionMenu();
    closeGitRowActionMenu();
  }

  function toggleViewMenu() {
    viewMenuOpen = !viewMenuOpen;
    closeEditorActionMenu();
    if (viewMenuOpen) closeRowActionMenus();
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
    closeRowActionMenus();
    // Focus now happens in CommandPaletteOverlay's $effect on `visible`.
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
    closeRowActionMenus();
    window.setTimeout(() => quickOpenInput?.focus(), 0);
  }

  function openWorkspaceSymbolQuickOpen() {
    quickOpenVisible = true;
    quickOpenQuery = '#';
    quickOpenIndex = 0;
    closeCommandPalette();
    closeViewMenu();
    closeEditorActionMenu();
    closeRowActionMenus();
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
    await closeSourceTabRecord(tab);
  }

  async function closeSourceTabRecord(tab: SourceOpenTab) {
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

  function rememberPasteCleanupHistory(kind: PasteCleanupHistoryItem['kind'], text: string) {
    const normalizedText = text.trim();
    if (!normalizedText) return;

    const item = createPasteCleanupHistoryItem(kind, normalizedText, pasteCleanupMode);
    const nextHistory = [
      item,
      ...pasteCleanupHistory.filter(
        (entry) => entry.kind !== item.kind || entry.text !== item.text || entry.mode !== item.mode
      )
    ].slice(0, maxPasteCleanupHistoryItems);
    pasteCleanupHistory = nextHistory;
    persistPasteCleanupHistory(nextHistory);
  }

  async function copyPasteCleanupOutput() {
    if (!pasteCleanupOutput.trim()) return;

    fileActionBusy = 'paste-copy';

    try {
      await copyTextToClipboard(pasteCleanupOutput, 'Cleaned text copied');
      rememberPasteCleanupHistory('cleaned', pasteCleanupOutput);
    } catch (copyError) {
      error = copyError instanceof Error ? copyError.message : 'Could not copy cleaned text';
    } finally {
      fileActionBusy = '';
    }
  }

  async function copyPasteCleanupReplyDraft() {
    if (!pasteCleanupReplyOutput.trim()) return;

    fileActionBusy = 'paste-reply-copy';

    try {
      await copyTextToClipboard(pasteCleanupReplyOutput, 'Reply draft copied');
      rememberPasteCleanupHistory('reply', pasteCleanupReplyOutput);
    } catch (copyError) {
      error = copyError instanceof Error ? copyError.message : 'Could not copy reply draft';
    } finally {
      fileActionBusy = '';
    }
  }

  async function copyPasteCleanupHistoryItem(item: PasteCleanupHistoryItem) {
    await copyActivityCommand(item.text, `${pasteCleanupHistoryKindLabel(item.kind)} copied`);
  }

  function restorePasteCleanupHistoryItem(item: PasteCleanupHistoryItem) {
    selectSourceActivityMode('clipboard');
    if (item.kind === 'reply') {
      pasteCleanupReplyDraft = item.text;
    } else {
      pasteCleanupInput = item.text;
      setPasteCleanupMode(item.mode);
    }

    fileActionStatus = `${pasteCleanupHistoryKindLabel(item.kind)} restored`;
    error = '';
  }

  function clearPasteCleanupHistory() {
    pasteCleanupHistory = [];
    persistPasteCleanupHistory([]);
    fileActionStatus = 'Paste cleanup history cleared';
    error = '';
  }

  function pasteCleanupHistoryKindLabel(kind: PasteCleanupHistoryItem['kind']) {
    return kind === 'reply' ? 'reply draft' : 'cleaned text';
  }

  function pasteCleanupHistoryItemTitle(item: PasteCleanupHistoryItem) {
    return `${pasteCleanupHistoryKindLabel(item.kind)} · ${item.mode} · ${item.charCount.toLocaleString()} chars`;
  }

  function selectPasteCleanupMode(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement | null)?.value;
    if (!isPasteCleanupMode(value)) return;

    setPasteCleanupMode(value);
  }

  function setPasteCleanupMode(mode: PasteCleanupMode) {
    pasteCleanupMode = mode;
    persistPasteCleanupMode(mode);
  }

  function clearPasteCleanupInput() {
    pasteCleanupInput = '';
    fileActionStatus = 'Paste cleanup cleared';
    error = '';
  }

  function clearPasteCleanupReplyDraft() {
    pasteCleanupReplyDraft = '';
    fileActionStatus = 'Reply draft cleared';
    error = '';
  }

  async function copyTauriRunCommand() {
    await copyActivityCommand('pnpm tauri dev', 'Tauri run command copied');
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
      const opened = await openTerminalPathWithFallback(path, sourceTerminalApp);
      fileActionStatus = opened.opened
        ? terminalOpenStatus(opened, 'Opened terminal', 'opened terminal')
        : 'Native action unavailable';
    } catch (terminalError) {
      error = terminalError instanceof Error ? terminalError.message : 'Could not open terminal';
    } finally {
      fileActionBusy = '';
    }
  }

  async function resumeAgentSessionEmbeddedTerminal(session: AgentSession) {
    return switchToConversationWorkspace(session);
  }

  async function openAgentSessionTerminal(session: AgentSession) {
    const command = withAgentTerminalColorEnv(agentSessionTerminalCommand(session));
    if (!command.trim()) return;
    if (
      await copyAgentSessionMissingWorktreeRepairPlan(
        session,
        'Workspace repair plan copied before terminal resume'
      )
    ) {
      return;
    }

    captureActiveWorkspaceBeforeSwitch();
    captureAgentSessionWorkspaceSnapshot(session);
    markAgentSessionWorkspaceActive(session);
    const path = session.projectPath ?? selectedProject.path;
    fileActionBusy = `activity-terminal-command:${session.provider}:${session.id}`;
    fileActionStatus = '';
    error = '';

    try {
      const openedCommand = await openTerminalCommandWithFallback(path, command, sourceTerminalApp);
      if (openedCommand.opened) {
        fileActionStatus = terminalOpenStatus(
          openedCommand,
          'Opened terminal resume command',
          'opened terminal resume command'
        );
        return;
      }

      const openedPath = path.trim()
        ? await openTerminalPathWithFallback(path, sourceTerminalApp)
        : { opened: false, app: sourceTerminalApp, usedFallback: false };
      await navigator.clipboard.writeText(command);
      fileActionStatus = openedPath.opened
        ? 'Opened terminal and copied resume command'
        : 'Resume command copied';
    } catch (terminalError) {
      try {
        const openedPath = path.trim()
          ? await openTerminalPathWithFallback(path, sourceTerminalApp)
          : { opened: false, app: sourceTerminalApp, usedFallback: false };
        await navigator.clipboard.writeText(command);
        fileActionStatus = openedPath.opened
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

  function sourceMarkdownPreviewAvailable(nextPreview: SourcePreview | null) {
    if (!nextPreview) return false;
    const normalizedPath = nextPreview.path.toLowerCase();
    return nextPreview.language === 'markdown' || normalizedPath.endsWith('.md') || normalizedPath.endsWith('.mdx');
  }

  function setSelectedSourceEditorDisplayMode(mode: SourceEditorDisplayMode) {
    if (!preview || !sourceMarkdownPreviewAvailable(preview)) return;

    sourceMarkdownPreviewModeByPath = {
      ...sourceMarkdownPreviewModeByPath,
      [preview.path]: mode
    };
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
    clearSourceLookupResults();
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
    return findSourceDefinitionTargetsForEditor(request);
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
    return findSourceReferenceTargetsForEditor(request);
  }

  async function handleEditorReferenceCountLookup(request: SourceEditorLookupRequest) {
    return countSourceReferencesForCodeLens(request);
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
    return findSourceImplementationTargetsForEditor(request);
  }

  async function handleEditorTypeDefinitionLookup(request: SourceEditorLookupRequest) {
    return findSourceTypeDefinitionTargetsForEditor(request);
  }

  function requestSourceIntelligenceAction(action: SourceIntelligenceAction) {
    if (!preview || loading) return;
    if (
      (action === 'hover' ||
        action === 'implementation' ||
        action === 'type-definition' ||
        action === 'format' ||
        action === 'rename' ||
        action === 'quick-fix' ||
        action === 'completion' ||
        action === 'signature-help') &&
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

  function handleProjectChange() {
    const nextProject =
      projectOptions.find((project) => project.id === selectedProjectID) ?? projectOptions[0] ?? initialProject;
    void activateProject(nextProject, {
      projects: projectOptions,
      scanLimit: expandedSourceScanLimit,
      clearFileFilter: true
    });
  }

  function createSourceActivityFilterState(
    overrides: Partial<Record<SourceActivityMode, string>> = {}
  ): Record<SourceActivityMode, string> {
    return {
      files: overrides.files ?? '',
      clipboard: overrides.clipboard ?? '',
      conversations: overrides.conversations ?? '',
      runs: overrides.runs ?? '',
      sessions: overrides.sessions ?? '',
      agents: overrides.agents ?? '',
      worktrees: overrides.worktrees ?? '',
      git: overrides.git ?? ''
    };
  }

  function rememberSourceActivityFilter(mode: SourceActivityMode = sourceActivityMode, filter = sourceActivityFilter) {
    sourceActivityFiltersByMode = {
      ...sourceActivityFiltersByMode,
      [mode]: filter
    };
  }

  function setSourceActivityFilter(filter: string) {
    sourceActivityFilter = filter;
    rememberSourceActivityFilter(sourceActivityMode, filter);
  }

  function selectSourceActivityMode(mode: SourceActivityMode) {
    markSourceLayoutCustom();
    if (!sourceDockPanelVisible('activity')) {
      showDockPanel('activity');
    } else if (activityPaneRailOnly()) {
      expandActivityPaneFromRail();
    }
    rememberSourceActivityFilter();
    sourceActivityMode = mode;
    sourceActivityFilter = sourceActivityFiltersByMode[mode] ?? '';
    persistSourceActivityMode(mode);
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function applySourceLayoutPreset(presetID: ConcreteSourceLayoutPresetID) {
    const preset = sourceLayoutPresets.find((candidate) => candidate.id === presetID);
    if (!preset) return;
    const override = sourceLayoutPresetOverrides[preset.id];
    const nextActivityMode = override?.activityMode ?? preset.activityMode;

    sourceLayoutPreset = preset.id;
    rememberSourceActivityFilter();
    sourceActivityMode = nextActivityMode;
    sourceActivityFilter = sourceActivityFiltersByMode[nextActivityMode] ?? '';
    sidePaneWidth = clampSidePaneWidth(override?.sidePaneWidth ?? preset.sidePaneWidth);
    sidePanePosition = override?.sidePanePosition ?? preset.sidePanePosition;
    editorInsightWidth = clampEditorInsightWidth(override?.editorInsightWidth ?? preset.editorInsightWidth);
    editorInsightCollapsed = override?.editorInsightCollapsed ?? preset.editorInsightCollapsed;
    contextPaneWidth = clampContextPaneWidth(override?.contextPaneWidth ?? preset.contextPaneWidth);
    contextPaneHeight = clampContextPaneHeight(override?.contextPaneHeight ?? preset.contextPaneHeight);
    contextPanelCollapsed = override?.contextPanelCollapsed ?? preset.contextPanelCollapsed;
    contextPanelMode = override?.contextPanelMode ?? preset.contextPanelMode;
    contextPanelPlacement = override?.contextPanelPlacement ?? preset.contextPanelPlacement;
    sourceChromeCompact = override?.chromeCompact ?? preset.chromeCompact;
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
    persistSourceChromeCompact(sourceChromeCompact);
    persistHiddenContextCards(hiddenContextCardIDs);
    persistActiveContextCard(activeContextCardID);
    const nextDockLayout = override?.dockLayout
      ? normalizeSourceDockLayout(override.dockLayout)
      : sourceDockLayoutFromWorkspace();
    applySourceDockLayout(nextDockLayout);

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
      chromeCompact: sourceChromeCompact,
      intelligencePanel: sourceIntelligencePanel,
      dockLayout: normalizeSourceDockLayout(sourceDockLayout),
      hiddenContextCardIDs: [...hiddenContextCardIDs],
      activeContextCardID
    };
  }

  function captureSourceLayoutSnapshot(): SourceLayoutSnapshot {
    return {
      preset: sourceLayoutPreset,
      ...captureSourceLayoutPresetOverride()
    };
  }

  function applySourceLayoutSnapshot(snapshot: SourceLayoutSnapshot, status: string) {
    sourceLayoutPreset = snapshot.preset;
    rememberSourceActivityFilter();
    sourceActivityMode = snapshot.activityMode;
    sourceActivityFilter = sourceActivityFiltersByMode[snapshot.activityMode] ?? '';
    sidePaneWidth = clampSidePaneWidth(snapshot.sidePaneWidth);
    sidePanePosition = snapshot.sidePanePosition;
    editorInsightWidth = clampEditorInsightWidth(snapshot.editorInsightWidth);
    editorInsightCollapsed = snapshot.editorInsightCollapsed;
    contextPaneWidth = clampContextPaneWidth(snapshot.contextPaneWidth);
    contextPaneHeight = clampContextPaneHeight(snapshot.contextPaneHeight);
    contextPanelCollapsed = snapshot.contextPanelCollapsed;
    contextPanelMode = snapshot.contextPanelMode;
    contextPanelPlacement = snapshot.contextPanelPlacement;
    sourceChromeCompact = snapshot.chromeCompact;
    sourceIntelligencePanel = snapshot.intelligencePanel;
    hiddenContextCardIDs = new Set(snapshot.hiddenContextCardIDs);
    activeContextCardID = snapshot.activeContextCardID;
    const nextDockLayout = normalizeSourceDockLayout(snapshot.dockLayout);

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
    persistSourceChromeCompact(sourceChromeCompact);
    persistHiddenContextCards(hiddenContextCardIDs);
    persistActiveContextCard(activeContextCardID);
    applySourceDockLayout(nextDockLayout);
    fileActionStatus = status;

    if (typeof window !== 'undefined') {
      window.setTimeout(measureFileTreeViewport, 0);
    }
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
        return pasteCleanupHistory.length;
      case 'conversations':
        return filteredConversationAgentSessions.length;
      case 'agents':
        return filteredProjectAgentSessions.length;
      case 'runs':
        return filteredProjectOrchestrationRuns.length;
      case 'sessions':
        return filteredProjectRuntimeContexts.length;
      case 'worktrees':
        return filteredProjectWorktrees.length;
      case 'git':
        return filteredGitRepositoryRows.length;
    }
  }

  function sourceActivityCountLabel(mode: SourceActivityMode) {
    return compactCountValue(sourceActivityCount(mode));
  }

  function sourceActivityButtonTitle(mode: SourceActivityMode) {
    const count = sourceActivityCount(mode);
    const label = sourceActivityLabel(mode);
    return count > 0 ? `${label} · ${count.toLocaleString()}` : label;
  }

  function sourceActivityBadgeVisible(mode: SourceActivityMode) {
    return sourceActivityMode === mode && sourceActivityCount(mode) > 0;
  }

  function compactCountValue(value: number | null | undefined) {
    const count =
      typeof value === 'number' && Number.isFinite(value) ? Math.max(0, Math.floor(value)) : 0;
    if (count >= 10000) return `${Math.round(count / 1000)}K`;
    if (count >= 1000) {
      const thousands = Math.round(count / 100) / 10;
      return `${Number.isInteger(thousands) ? thousands.toFixed(0) : thousands.toFixed(1)}K`;
    }
    return count.toLocaleString();
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
        return conversationSessionSummary;
      case 'agents':
        return agentSessionSummary;
      case 'runs':
        return orchestrationRunSummary;
      case 'sessions':
        return runtimeContextSummary;
      case 'worktrees':
        return projectWorktreeSummary;
      case 'git':
        return selectedProjectGitGraphSummary;
    }
  }

  function sourceRuntimeNoticeText(currentRuntime: string, currentError: string) {
    const nativeScannerUnavailable = currentError.includes('Local source scanner unavailable');
    if (nativeScannerUnavailable) {
      return 'Local source bridge unavailable. Run pnpm dev/preview or the Tauri app for filesystem scans.';
    }

    const browserPreviewRuntime =
      currentRuntime === 'browser preview' || currentRuntime === 'browser source bridge';
    if (!browserPreviewRuntime) return '';

    return 'Browser preview is using the local filesystem bridge for source files. Run the Tauri app for native Git, LSP, and terminal actions.';
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

  async function selectSourceFilesPane(paneID: SourceFilesPaneID) {
    sourceFilesPane = paneID;
    await tick();
    if (paneID === 'files') {
      measureFileTreeViewport();
    } else if (paneID === 'search') {
      sourceSearchInput?.focus();
    }
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

  function loadStoredPasteCleanupHistory(): PasteCleanupHistoryItem[] {
    if (typeof window === 'undefined') return [];

    try {
      const storedHistory = JSON.parse(window.localStorage.getItem(pasteCleanupHistoryStorageKey) ?? '[]');
      if (!Array.isArray(storedHistory)) return [];

      return storedHistory
        .map(normalizeStoredPasteCleanupHistoryItem)
        .filter((item): item is PasteCleanupHistoryItem => Boolean(item))
        .slice(0, maxPasteCleanupHistoryItems);
    } catch {
      return [];
    }
  }

  function normalizeStoredPasteCleanupHistoryItem(value: unknown): PasteCleanupHistoryItem | null {
    if (!value || typeof value !== 'object') return null;

    const item = value as Partial<PasteCleanupHistoryItem>;
    if (item.kind !== 'cleaned' && item.kind !== 'reply') return null;
    if (!isPasteCleanupMode(item.mode)) return null;
    if (typeof item.text !== 'string' || item.text.trim().length === 0) return null;

    const text = item.text.trim();
    const createdAt = typeof item.createdAt === 'number' && Number.isFinite(item.createdAt)
      ? item.createdAt
      : Date.now();

    return {
      id: typeof item.id === 'string' && item.id.trim().length > 0
        ? item.id
        : `${item.kind}-${createdAt}`,
      kind: item.kind,
      mode: item.mode,
      text,
      summary: typeof item.summary === 'string' && item.summary.trim().length > 0
        ? item.summary
        : summarizePasteCleanupHistoryText(text),
      charCount: typeof item.charCount === 'number' && Number.isFinite(item.charCount)
        ? item.charCount
        : text.length,
      createdAt
    };
  }

  function persistPasteCleanupHistory(history: PasteCleanupHistoryItem[]) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(
      pasteCleanupHistoryStorageKey,
      JSON.stringify(history.slice(0, maxPasteCleanupHistoryItems))
    );
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

  function loadStoredSourceChromeCompact() {
    if (typeof window === 'undefined') return true;
    const storedValue = window.localStorage.getItem(sourceChromeCompactStorageKey);
    if (storedValue === null) return true;
    return storedValue !== 'false';
  }

  function selectSourceChromeCompact(compact: boolean) {
    markSourceLayoutCustom();
    sourceChromeCompact = compact;
    persistSourceChromeCompact(compact);
    fileActionStatus = compact ? 'Compact editor chrome enabled' : 'Comfortable editor chrome enabled';
  }

  function toggleSourceChromeCompact() {
    selectSourceChromeCompact(!sourceChromeCompact);
  }

  function persistSourceChromeCompact(compact: boolean) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceChromeCompactStorageKey, compact ? 'true' : 'false');
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
      contextPaneWidth: clampContextPaneWidth(numericValue(candidate.contextPaneWidth, preset.contextPaneWidth)),
      contextPaneHeight: clampContextPaneHeight(numericValue(candidate.contextPaneHeight, preset.contextPaneHeight)),
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
      chromeCompact:
        typeof candidate.chromeCompact === 'boolean'
          ? candidate.chromeCompact
          : preset.chromeCompact,
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

  function loadStoredSourceFocusRestoreLayout(): SourceLayoutSnapshot | null {
    if (typeof window === 'undefined') return null;

    const storedSnapshot = window.localStorage.getItem(sourceFocusRestoreLayoutStorageKey);
    if (!storedSnapshot) return null;

    try {
      return normalizeStoredSourceLayoutSnapshot(JSON.parse(storedSnapshot));
    } catch {
      return null;
    }
  }

  function normalizeStoredSourceLayoutSnapshot(value: unknown): SourceLayoutSnapshot | null {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;

    const candidate = value as Record<string, unknown>;
    const presetID = isSourceLayoutPresetID(candidate.preset) ? candidate.preset : 'custom';
    const fallbackPreset =
      sourceLayoutPresets.find((preset) => preset.id === presetID) ??
      sourceLayoutPresets.find((preset) => preset.id === 'code') ??
      sourceLayoutPresets[0];
    const normalizedOverride = normalizeStoredSourceLayoutPresetOverride(candidate, fallbackPreset);
    if (!normalizedOverride) return null;

    return {
      preset: presetID,
      ...normalizedOverride
    };
  }

  function persistSourceFocusRestoreLayout(snapshot: SourceLayoutSnapshot | null) {
    if (typeof window === 'undefined') return;
    if (!snapshot) {
      window.localStorage.removeItem(sourceFocusRestoreLayoutStorageKey);
      return;
    }

    window.localStorage.setItem(
      sourceFocusRestoreLayoutStorageKey,
      JSON.stringify(snapshot)
    );
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

  function clearMigratedSourceDockviewLayouts() {
    if (typeof window === 'undefined') return;
    window.localStorage.removeItem(sourceDockviewStorageKey);
    window.localStorage.removeItem(sourceDockviewActivityStorageKey);
    window.localStorage.removeItem(sourceDockviewInsightsStorageKey);
    window.localStorage.removeItem(sourceDockviewContextStorageKey);
    window.localStorage.removeItem(sourceDockviewCenterStorageKey);
    window.localStorage.removeItem(sourceDockviewBottomStorageKey);
    window.localStorage.removeItem(sourceFilesDockviewStorageKey);
    window.localStorage.removeItem(sourceConversationDockviewStorageKey);
    window.localStorage.removeItem(sourceContextCardDockviewStorageKey);
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

  function moveDockPanelToGroup(panelID: SourceDockPanelID, groupID: SourceDockGroupID, targetIndex?: number) {
    applySourceDockLayout(moveSourceDockPanel(sourceDockLayout, panelID, groupID, targetIndex));
  }

  function hideDockPanel(panelID: SourceDockPanelID) {
    applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, panelID));
  }

  function showDockPanel(panelID: SourceDockPanelID) {
    if (panelID === 'activity') {
      const nextSidePaneWidth = sidePaneWidth <= sidePaneRailOnlyThreshold
        ? sidePaneDefaultWidth
        : sidePaneWidth;
      sidePaneWidth = clampSidePaneWidth(nextSidePaneWidth);
      applySourceDockLayout(
        resizeSourceDockGroup(
          moveSourceDockPanel(sourceDockLayout, 'activity', sidePanePosition),
          sidePanePosition,
          sidePaneWidth
        )
      );
      fileActionStatus = 'Activity panel shown';
      if (typeof window !== 'undefined') {
        window.setTimeout(measureFileTreeViewport, 0);
      }
      return;
    }

    if (panelID === 'context') {
      const contextGroupID = dockGroupForContextPanelPlacement(contextPanelPlacement);
      const nextContextPaneWidth =
        contextPanelPlacement === 'side' && contextPaneWidth <= contextPaneRailOnlyThreshold
          ? contextPaneDefaultWidth
          : contextPaneWidth;
      contextPaneWidth = clampContextPaneWidth(nextContextPaneWidth);
      applySourceDockLayout(
        resizeSourceDockGroup(
          moveSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context', contextGroupID),
          contextGroupID,
          contextPanelPlacement === 'bottom' ? contextPaneHeight : contextPaneWidth
        )
      );
      fileActionStatus = 'Context panel shown';
      return;
    }

    applySourceDockLayout(showSourceDockPanel(sourceDockLayout, panelID));
    if (panelID === 'terminal') {
      fileActionStatus = 'Terminal dock shown';
      scheduleEmbeddedTerminalFit();
    }
    if (panelID === 'browser') {
      if (!browserUrl && defaultBrowserUrl) {
        setBrowserDockUrl(defaultBrowserUrl);
      }
      fileActionStatus = 'Browser dock shown';
    }
  }

  function resetSourceDockLayout() {
    if (sourceLayoutPresetOverrides.review) {
      const nextOverrides = { ...sourceLayoutPresetOverrides };
      delete nextOverrides.review;
      sourceLayoutPresetOverrides = nextOverrides;
      persistSourceLayoutPresetOverrides(sourceLayoutPresetOverrides);
    }
    applySourceLayoutPreset('review');
    fileActionStatus = 'Dock layout reset';
  }

  function focusSourceEditorLayout() {
    if (!sourceEditorFocusActive()) {
      sourceFocusRestoreLayout = captureSourceLayoutSnapshot();
      persistSourceFocusRestoreLayout(sourceFocusRestoreLayout);
    }

    markSourceLayoutCustom();
    rememberSourceActivityFilter();
    sourceActivityMode = 'files';
    sourceActivityFilter = sourceActivityFiltersByMode.files ?? '';
    contextPanelCollapsed = true;
    editorInsightCollapsed = true;
    sourceChromeCompact = true;
    persistSourceActivityMode(sourceActivityMode);
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistEditorInsightCollapsed(editorInsightCollapsed);
    persistSourceChromeCompact(sourceChromeCompact);

    const editorFocusHiddenPanelIDs: SourceDockPanelID[] = ['activity', 'context', 'insights', 'terminal', 'browser'];
    let nextLayout = sourceDockLayout;
    for (const panelID of editorFocusHiddenPanelIDs) {
      nextLayout = hideSourceDockPanel(nextLayout, panelID);
    }

    applySourceDockLayout(activateSourceDockPanel(nextLayout, 'editor'));
    fileActionStatus = 'Editor canvas focused';
    if (typeof window !== 'undefined') {
      window.setTimeout(measureFileTreeViewport, 0);
    }
  }

  function restoreSourceLayoutBeforeFocus() {
    if (!sourceFocusRestoreLayout) return;

    applySourceLayoutSnapshot(sourceFocusRestoreLayout, 'Previous layout restored');
    sourceFocusRestoreLayout = null;
    persistSourceFocusRestoreLayout(null);
  }

  function sourceEditorFocusActive() {
    const editorFocusHiddenPanelIDs: SourceDockPanelID[] = ['activity', 'context', 'insights', 'terminal', 'browser'];
    return (
      sourceChromeCompact &&
      editorFocusHiddenPanelIDs.every((panelID) => !sourceDockPanelVisible(panelID))
    );
  }

  function activityPaneRailOnly() {
    return sourceWorkspacePlanItem('activity')?.state === 'rail';
  }

  function contextPaneRailOnly() {
    // Dockview owns sizing in the unified workbench, so the homemade width-collapse plan must not
    // force the context panel into its narrow rail-only presentation.
    if (useUnifiedWorkbench) return false;
    return contextPanelPlacement === 'side' && sourceWorkspacePlanItem('context')?.state === 'rail';
  }

  function activityPaneViewportCollapsed() {
    const item = sourceWorkspacePlanItem('activity');
    return shouldRenderDockPanel('activity') && item?.reason === 'viewport-collapsed';
  }

  function contextPaneViewportCollapsed() {
    // The unified workbench lets Dockview size the context group; never auto-collapse it here.
    if (useUnifiedWorkbench) return false;
    const item = sourceWorkspacePlanItem('context');
    return (
      contextPanelPlacement === 'side' &&
      sourceDockPanelVisible('context') &&
      item?.reason === 'viewport-collapsed'
    );
  }

  function effectiveActivityPaneVisible() {
    return shouldRenderDockPanel('activity') && !activityPaneViewportCollapsed();
  }

  function effectiveContextPaneVisible() {
    if (!sourceDockPanelVisible('context')) return false;
    return contextPanelPlacement !== 'side' || !contextPaneViewportCollapsed();
  }

  function sourceWorkspaceViewportSize() {
    const measuredWidth = Math.max(0, Math.round(sourceWorkspaceWidth));
    if (measuredWidth > 0) return measuredWidth;

    if (typeof window !== 'undefined') {
      return Math.max(0, Math.round(window.innerWidth - 8));
    }

    return sourceWorkspaceDefaultViewportWidth;
  }

  function sourceWorkspacePlanItem(id: SourceWorkspacePaneID) {
    return sourceWorkspacePlan.items.find((item) => item.id === id) ?? null;
  }

  function expandActivityPaneFromRail() {
    sidePaneWidth = restoreSourcePaneExpandedSize(sidePaneWidth, activityPaneSizingConfig, {
      previousExpandedSize: sidePaneExpandedWidth
    });
    persistSidePaneWidth(sidePaneWidth);
    persistDockGroupSize(sidePanePosition, sidePaneWidth);
  }

  function snapActivityPaneToRail() {
    sidePaneWidth = clampSidePaneWidth(sidePaneMinWidth);
    persistSidePaneWidth(sidePaneWidth);
    persistDockGroupSize(sidePanePosition, sidePaneWidth);
  }

  function collapseActivityPaneToRail() {
    markSourceLayoutCustom();
    snapActivityPaneToRail();
    applySourceDockLayout(resizeSourceDockGroup(
      showSourceDockPanel(sourceDockLayout, 'activity'),
      sidePanePosition,
      sidePaneWidth
    ));
    persistSidePaneWidth(sidePaneWidth);
    if (typeof window !== 'undefined') {
      window.setTimeout(measureFileTreeViewport, 0);
    }
    fileActionStatus = 'Activity panel collapsed to rail';
  }

  function activityRailToggleLabel() {
    return activityPaneRailOnly() ? 'Expand explorer from icon rail' : 'Collapse explorer to icon rail';
  }

  function activityRailToggleDirection() {
    if (sidePanePosition === 'left') {
      return activityPaneRailOnly() ? 'right' : 'left';
    }

    return activityPaneRailOnly() ? 'left' : 'right';
  }

  function toggleActivityPaneRail() {
    markSourceLayoutCustom();

    if (!sourceDockPanelVisible('activity')) {
      showDockPanel('activity');
      return;
    }

    if (activityPaneRailOnly()) {
      expandActivityPaneFromRail();
      fileActionStatus = 'Activity panel expanded';
      if (typeof window !== 'undefined') {
        window.setTimeout(measureFileTreeViewport, 0);
      }
      return;
    }

    collapseActivityPaneToRail();
  }

  function expandContextPaneFromRail() {
    if (contextPanelPlacement !== 'side') return;
    contextPaneWidth = restoreSourcePaneExpandedSize(contextPaneWidth, contextPaneWidthSizingConfig, {
      previousExpandedSize: contextPaneExpandedWidth
    });
    persistContextPaneWidth(contextPaneWidth);
    persistDockGroupSize(dockGroupForContextPanelPlacement(contextPanelPlacement), contextPaneWidth);
  }

  function snapContextPaneToRail(updateDockGroup = true) {
    contextPaneWidth = clampContextPaneWidth(contextPaneMinWidth);
    persistContextPaneWidth(contextPaneWidth);
    if (updateDockGroup) {
      persistDockGroupSize(dockGroupForContextPanelPlacement('side'), contextPaneWidth);
    }
  }

  function collapseContextPaneToRail() {
    markSourceLayoutCustom();
    contextPanelPlacement = 'side';
    contextPanelCollapsed = false;
    snapContextPaneToRail(false);
    const contextGroupID = dockGroupForContextPanelPlacement(contextPanelPlacement);
    applySourceDockLayout(resizeSourceDockGroup(
      moveSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context', contextGroupID),
      contextGroupID,
      contextPaneWidth
    ));
    persistContextPanelPlacement(contextPanelPlacement);
    persistContextPanelCollapsed(contextPanelCollapsed);
    persistContextPaneWidth(contextPaneWidth);
    fileActionStatus = 'Context panel collapsed to rail';
  }

  function toggleDockPanelVisibility(panelID: SourceDockPanelID) {
    if (!dockPanelCanHide(panelID)) return;
    if (sourceDockPanelVisible(panelID)) {
      hideDockPanel(panelID);
      return;
    }

    showDockPanel(panelID);
  }

  function moveDockPanelToManagedGroup(
    panelID: SourceDockPanelID,
    groupID: SourceDockGroupID,
    targetIndex?: number
  ) {
    if (!dockPanelMoveTargets(panelID).includes(groupID)) return;
    if (panelID === 'activity' && (groupID === 'left' || groupID === 'right')) {
      selectSidePanePosition(groupID);
      return;
    }

    moveDockPanelToGroup(panelID, groupID, targetIndex);
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
    dockDropTargetPanelID = null;
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
    dockDropTargetPanelID = null;
    dockDropTargetPanelPlacement = null;
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  function dragOverDockPanelTab(event: DragEvent, targetPanelID: SourceDockPanelID) {
    const panelID = draggedDockPanelID(event);
    if (!panelID || panelID === targetPanelID) return;

    const targetGroupID = dockGroupIDForPanel(sourceDockLayout, targetPanelID);
    if (!targetGroupID || !dockPanelMoveTargets(panelID).includes(targetGroupID)) return;

    event.preventDefault();
    dockDropTargetGroupID = targetGroupID;
    dockDropTargetPanelID = targetPanelID;
    dockDropTargetPanelPlacement = dockPanelTabDropPlacement(event);
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  function dockPanelTabDropPlacement(event: DragEvent): 'before' | 'after' {
    const target = event.currentTarget as HTMLElement | null;
    const rect = target?.getBoundingClientRect();
    if (!rect) return 'before';

    return event.clientX > rect.left + rect.width / 2 ? 'after' : 'before';
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

  function dropDockPanelOnTab(event: DragEvent, targetPanelID: SourceDockPanelID) {
    const panelID = draggedDockPanelID(event);
    const targetGroupID = dockGroupIDForPanel(sourceDockLayout, targetPanelID);
    if (
      !panelID ||
      panelID === targetPanelID ||
      !targetGroupID ||
      !dockPanelMoveTargets(panelID).includes(targetGroupID)
    ) {
      clearDockPanelDrag();
      return;
    }

    event.preventDefault();
    const targetPanelIDs = dockGroupPanelIDs(targetGroupID);
    const targetIndex = targetPanelIDs.indexOf(targetPanelID);
    const currentIndex = targetPanelIDs.indexOf(panelID);
    const dropPlacement = dockPanelTabDropPlacement(event);
    let insertionIndex = targetIndex + (dropPlacement === 'after' ? 1 : 0);
    if (currentIndex >= 0 && currentIndex < insertionIndex) {
      insertionIndex -= 1;
    }
    moveDockPanelToManagedGroup(panelID, targetGroupID, insertionIndex);
    clearDockPanelDrag();
  }

  function clearDockPanelDrag() {
    draggingDockPanelID = null;
    dockDropTargetGroupID = null;
    dockDropTargetPanelID = null;
    dockDropTargetPanelPlacement = null;
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
        return ['center'];
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
    syncSourceDockviewWorkbenchLayout(normalizedLayout);
    syncSourceDockviewActivityLayout(normalizedLayout);
    syncSourceDockviewContextLayout(normalizedLayout);
    syncSourceDockviewInsightsLayout(normalizedLayout);
    syncSourceDockviewCenterLayout(normalizedLayout);
    syncSourceDockviewBottomLayout(normalizedLayout);
    persistSourceDockLayout(normalizedLayout);
  }

  function syncSourceDockviewWorkbenchLayout(layout: SourceDockLayout) {
    if (!sourceDockviewWorkbenchWorkspace) return;

    sourceDockviewWorkbenchWorkspace.syncLayout(layout);
    scheduleSourceDockviewPanelElementSync();
    if (sourceDockviewWorkbenchOwnsPanel('terminal')) scheduleEmbeddedTerminalFit();
  }

  type SourceDockviewGroup = SourceDockviewWorkspace['api']['groups'][number];

  function syncSourceDockLayoutFromWorkbenchGroups(activePanelID: SourceDockPanelID | null = null) {
    const workspace = sourceDockviewWorkbenchWorkspace;
    if (!workspace) return;

    const nextLayout = sourceDockLayoutFromDockviewGroups(workspace.api, activePanelID);
    if (!nextLayout) return;

    sourceDockLayout = nextLayout;
    syncSourceDockLayoutToWorkspace(nextLayout);
    persistSourceDockLayout(nextLayout);
    if (activePanelID === 'terminal' || sourceDockviewWorkbenchOwnsPanel('terminal')) {
      scheduleEmbeddedTerminalFit();
    }
  }

  function sourceDockLayoutFromDockviewGroups(
    api: SourceDockviewWorkspace['api'],
    activePanelID: SourceDockPanelID | null
  ): SourceDockLayout | null {
    const currentLayout = normalizeSourceDockLayout(sourceDockLayout);
    const entries = api.groups
      .map((group) => sourceDockviewGroupEntry(group))
      .filter((entry): entry is NonNullable<typeof entry> => entry !== null);
    const editorEntry = entries.find((entry) => entry.panelIDs.includes('editor'));
    if (!editorEntry) return null;

    const groups = currentLayout.groups.map((group) => ({
      ...group,
      panelIDs: [] as SourceDockPanelID[]
    }));
    const visiblePanelIDs = new Set<SourceDockPanelID>();
    const activePanelByGroup: SourceDockLayout['activePanelByGroup'] = {};

    for (const entry of entries) {
      const groupID = sourceDockGroupIDForDockviewGroup(entry.rect, editorEntry.rect, entry.panelIDs);
      const targetGroup = groups.find((group) => group.id === groupID);
      if (!targetGroup) continue;

      targetGroup.panelIDs.push(...entry.panelIDs.filter((panelID) => !visiblePanelIDs.has(panelID)));
      for (const panelID of entry.panelIDs) {
        visiblePanelIDs.add(panelID);
      }
      targetGroup.size = sourceDockGroupSizeFromDockviewRect(groupID, entry.rect, currentLayout);
      activePanelByGroup[groupID] = entry.activePanelID ?? activePanelByGroup[groupID];
    }

    const activeGroupID = activePanelID
      ? groups.find((group) => group.panelIDs.includes(activePanelID))?.id
      : null;
    if (activePanelID && activeGroupID) {
      activePanelByGroup[activeGroupID] = activePanelID;
    }

    return normalizeSourceDockLayout({
      ...currentLayout,
      preset: 'custom',
      groups,
      hiddenPanelIDs: currentLayout.hiddenPanelIDs.filter((panelID) => !visiblePanelIDs.has(panelID)),
      activePanelByGroup
    });
  }

  function sourceDockviewGroupEntry(group: SourceDockviewGroup) {
    const panelIDs = group.panels
      .map((panel) => sourceDockviewWorkbenchPanelID(panel.id))
      .filter((panelID): panelID is SourceDockPanelID => panelID !== null);
    if (panelIDs.length === 0) return null;

    return {
      panelIDs,
      activePanelID: sourceDockviewWorkbenchPanelID(group.activePanel?.id),
      rect: group.element.getBoundingClientRect()
    };
  }

  function sourceDockviewWorkbenchPanelID(value: unknown): SourceDockPanelID | null {
    return sourceDockviewWorkbenchPanelIDs.includes(value as SourceDockPanelID)
      ? (value as SourceDockPanelID)
      : null;
  }

  function syncSourceDockviewPanelElements() {
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
  }

  function scheduleSourceDockviewPanelElementSync() {
    syncSourceDockviewPanelElements();
    queueMicrotask(syncSourceDockviewPanelElements);
    if (typeof window !== 'undefined') {
      window.setTimeout(syncSourceDockviewPanelElements, 0);
      window.setTimeout(syncSourceDockviewPanelElements, 80);
    }
  }

  function sourceDockGroupIDForDockviewGroup(
    rect: DOMRect,
    editorRect: DOMRect,
    panelIDs: SourceDockPanelID[]
  ): SourceDockGroupID {
    const tolerance = 8;
    if (panelIDs.includes('editor')) return 'center';
    if (rect.right <= editorRect.left + tolerance) return 'left';
    if (rect.left >= editorRect.right - tolerance) return 'right';
    if (rect.top >= editorRect.bottom - tolerance) return 'bottom';
    return 'center';
  }

  function sourceDockGroupSizeFromDockviewRect(
    groupID: SourceDockGroupID,
    rect: DOMRect,
    layout: SourceDockLayout
  ) {
    if (groupID === 'left' || groupID === 'right') return Math.max(1, Math.round(rect.width));
    if (groupID === 'bottom') return Math.max(1, Math.round(rect.height));
    return sourceDockGroupSize(layout, groupID);
  }

  function syncSourceDockviewActivityLayout(layout: SourceDockLayout) {
    if (!sourceDockviewActivityWorkspace) return;

    sourceDockviewActivityWorkspace.syncLayout(layout);
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
  }

  function syncSourceDockviewContextLayout(layout: SourceDockLayout) {
    if (!sourceDockviewContextWorkspace) return;

    sourceDockviewContextWorkspace.syncLayout(layout);
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
  }

  function syncSourceDockviewInsightsLayout(layout: SourceDockLayout) {
    if (!sourceDockviewInsightsWorkspace) return;

    sourceDockviewInsightsWorkspace.syncLayout(layout);
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
  }

  function syncSourceDockviewCenterLayout(layout: SourceDockLayout) {
    if (!sourceDockviewCenterWorkspace) return;

    sourceDockviewCenterWorkspace.syncLayout(layout);
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
    if (sourceDockviewCenterOwnsPanel('terminal')) scheduleEmbeddedTerminalFit();
  }

  function syncSourceDockviewBottomLayout(layout: SourceDockLayout) {
    if (!sourceDockviewBottomWorkspace) return;

    sourceDockviewBottomWorkspace.syncLayout(layout);
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
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
    if (!contextPanelCollapsed && contextGroupID !== null) {
      nextLayout = activateSourceDockPanel(nextLayout, 'context');
    }
    return normalizeSourceDockLayout({
      ...nextLayout,
      preset: sourceLayoutPreset
    });
  }

  function ensureWorkbenchRightPanels(layout: SourceDockLayout): SourceDockLayout {
    // Under the unified workbench, Dockview owns panel visibility through its tabs, so the legacy
    // collapse flags must not hide context/insights — doing so left the right pane empty (only its
    // icon rail showed). Force both back into the visible right group (moveSourceDockPanel un-hides).
    if (!useUnifiedWorkbench) return layout;
    let next = moveSourceDockPanel(layout, 'context', 'right');
    next = moveSourceDockPanel(next, 'insights', 'right');
    return next;
  }

  function sourceDockLayoutWithWorkspaceViewState(
    layout: SourceDockLayout,
    viewState: WorkspaceSnapshotViewState
  ): SourceDockLayout {
    let nextLayout = normalizeSourceDockLayout(layout);
    const activityGroupID = dockGroupIDForPanel(nextLayout, 'activity');
    const contextGroupID = dockGroupIDForPanel(nextLayout, 'context');
    const insightsGroupID = dockGroupIDForPanel(nextLayout, 'insights');

    if (activityGroupID === 'left' || activityGroupID === 'right') {
      nextLayout = resizeSourceDockGroup(nextLayout, activityGroupID, viewState.sidePaneWidth);
    }
    if (contextGroupID === 'bottom') {
      nextLayout = resizeSourceDockGroup(nextLayout, contextGroupID, viewState.contextPaneHeight);
    } else if (contextGroupID === 'left' || contextGroupID === 'right') {
      nextLayout = resizeSourceDockGroup(nextLayout, contextGroupID, viewState.contextPaneWidth);
    }
    if (insightsGroupID !== null && insightsGroupID !== contextGroupID) {
      nextLayout = resizeSourceDockGroup(nextLayout, insightsGroupID, viewState.editorInsightWidth);
    }

    return normalizeSourceDockLayout(nextLayout);
  }

  function sourceDockLayoutWithStoredPaneSizes(layout: SourceDockLayout): SourceDockLayout {
    const normalizedLayout = normalizeSourceDockLayout(layout);
    const activityGroupID = dockGroupIDForPanel(normalizedLayout, 'activity');
    const contextGroupID = dockGroupIDForPanel(normalizedLayout, 'context');
    const insightsGroupID = dockGroupIDForPanel(normalizedLayout, 'insights');
    const sizesByGroupID: Partial<Record<SourceDockGroupID, number>> = {};

    if (activityGroupID === 'left' || activityGroupID === 'right') {
      sizesByGroupID[activityGroupID] = sidePaneWidth;
    }
    if (contextGroupID === 'bottom') {
      sizesByGroupID[contextGroupID] = contextPaneHeight;
    } else if (contextGroupID === 'left' || contextGroupID === 'right') {
      sizesByGroupID[contextGroupID] = contextPaneWidth;
    }
    if (insightsGroupID !== null && insightsGroupID !== contextGroupID) {
      sizesByGroupID[insightsGroupID] = editorInsightWidth;
    }

    return resizeSourceDockGroups(normalizedLayout, sizesByGroupID);
  }

  function restoreLegacyPaneWidth(
    storedWidth: number,
    expandedWidth: number,
    config: SourcePaneSizingConfig
  ) {
    const paneState = deriveSourcePaneState({ visible: true, size: storedWidth }, config);
    if (paneState !== 'rail') return storedWidth;

    const minState = deriveSourcePaneState({ visible: true, size: config.minSize }, config);
    return minState === 'rail' && storedWidth <= config.minSize
      ? storedWidth
      : restoreSourcePaneExpandedSize(storedWidth, config, {
          previousExpandedSize: expandedWidth
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

  function editorInsightsDockColumnVisible() {
    return (
      !editorInsightCollapsed &&
      shouldRenderDockPanel('insights') &&
      !sourceDockviewWorkbenchOwnsPanel('insights') &&
      !sourceDockviewContextOwnsPanel('insights')
    );
  }

  function sourceIntelligencePanelMounted() {
    // In the unified workbench, Dockview owns the Insights tab; mount its content whenever the
    // panel is present in the layout, independent of the old `editorInsightCollapsed` rail state
    // (which otherwise leaves the Insights tab empty under the `code` preset default).
    if (useUnifiedWorkbench) return sourceDockviewWorkbenchOwnsPanel('insights');
    return (
      !editorInsightCollapsed &&
      sourceDockPanelVisible('insights') &&
      (shouldRenderDockPanel('insights') ||
        sourceDockviewWorkbenchOwnsPanel('insights') ||
        sourceDockviewContextOwnsPanel('insights'))
    );
  }

  function hiddenDockPanelIDs() {
    return normalizeSourceDockLayout(sourceDockLayout).hiddenPanelIDs.filter((panelID) =>
      hideableDockPanelIDs.includes(panelID)
    );
  }

  function restoreHiddenDockPanel(panelID: SourceDockPanelID) {
    if (!hideableDockPanelIDs.includes(panelID)) return;

    showDockPanel(panelID);
    fileActionStatus = `${dockPanelLabel(panelID)} panel restored`;
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

  function loadStoredSourceDockviewLayout(
    storageKey: string,
    panelIDs?: string[]
  ): SerializedDockview | null {
    if (typeof window === 'undefined') return null;

    try {
      const storedLayout = window.localStorage.getItem(storageKey);
      if (!storedLayout) return null;

      const parsedLayout = JSON.parse(storedLayout) as SerializedDockview;
      if (panelIDs && !sourceDockviewLayoutOnlyContainsPanels(parsedLayout, panelIDs)) {
        return null;
      }

      return parsedLayout;
    } catch {
      return null;
    }
  }

  function sourceDockviewLayoutOnlyContainsPanels(
    layout: SerializedDockview,
    panelIDs: string[]
  ) {
    const expectedPanelIDs = new Set<string>(panelIDs);
    const storedPanelIDs = Object.keys(layout.panels ?? {});

    return (
      storedPanelIDs.length > 0 &&
      storedPanelIDs.every((panelID) => expectedPanelIDs.has(panelID)) &&
      panelIDs.every((panelID) => storedPanelIDs.includes(panelID))
    );
  }

  function loadStoredSourcePaneviewLayout(
    storageKey: string,
    panelIDs: string[]
  ): SerializedPaneview | null {
    if (typeof window === 'undefined') return null;

    try {
      const storedLayout = window.localStorage.getItem(storageKey);
      if (!storedLayout) return null;

      const parsedLayout = JSON.parse(storedLayout) as SerializedPaneview;
      if (!sourcePaneviewLayoutOnlyContainsPanels(parsedLayout, panelIDs)) {
        return null;
      }

      return parsedLayout;
    } catch {
      return null;
    }
  }

  function sourcePaneviewLayoutOnlyContainsPanels(
    layout: SerializedPaneview,
    panelIDs: string[]
  ) {
    const expectedPanelIDs = new Set<string>(panelIDs);
    const storedPanelIDs = (layout.views ?? []).map((view) => view.data.id);

    return (
      storedPanelIDs.length > 0 &&
      storedPanelIDs.every((panelID) => expectedPanelIDs.has(panelID)) &&
      panelIDs.every((panelID) => storedPanelIDs.includes(panelID))
    );
  }

  function persistSourceDockviewLayout(storageKey: string, layout: SerializedDockview) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(storageKey, JSON.stringify(layout));
  }

  function persistSourcePaneviewLayout(storageKey: string, layout: SerializedPaneview) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(storageKey, JSON.stringify(layout));
  }

  function sourceEditorFilePanelID(tab: Pick<SourceOpenTab, 'path'>): SourceEditorFilePanelID {
    return sourceEditorFilePanelIDFromPath(tab.path);
  }

  function sourceEditorFilePanelIDFromPath(path: string): SourceEditorFilePanelID {
    return `file:${path}` as SourceEditorFilePanelID;
  }

  function sourceEditorFilePathFromPanelID(panelID: SourceEditorFilePanelID): string {
    return panelID.slice('file:'.length);
  }

  function sourceEditorFileTabForPanelID(panelID: SourceEditorFilePanelID): SourceOpenTab | null {
    const path = sourceEditorFilePathFromPanelID(panelID);
    return projectOpenSourceTabs.find((tab) => tab.path === path) ?? null;
  }

  function sourceEditorFileDockviewTitle(tab: SourceOpenTab): string {
    return `${isSourcePathDirty(tab.path) ? '* ' : ''}${tab.fileName}`;
  }

  function sourceDockviewWorkbenchHostAction(node: HTMLElement) {
    const token = ++sourceDockviewWorkbenchHostToken;
    void initializeSourceDockviewWorkbench(node, token);

    return {
      destroy() {
        if (sourceDockviewWorkbenchHostToken === token) {
          sourceDockviewWorkbenchHostToken += 1;
          disposeSourceDockviewWorkbench();
        }
      }
    };
  }

  function sourceDockviewActivityHostAction(node: HTMLElement) {
    const token = ++sourceDockviewActivityHostToken;
    void initializeSourceDockviewActivity(node, token);

    return {
      destroy() {
        if (sourceDockviewActivityHostToken === token) {
          sourceDockviewActivityHostToken += 1;
          disposeSourceDockviewActivity();
        }
      }
    };
  }

  function sourceDockviewInsightsHostAction(node: HTMLElement) {
    const token = ++sourceDockviewInsightsHostToken;
    void initializeSourceDockviewInsights(node, token);

    return {
      destroy() {
        if (sourceDockviewInsightsHostToken === token) {
          sourceDockviewInsightsHostToken += 1;
          disposeSourceDockviewInsights();
        }
      }
    };
  }

  function sourceDockviewContextHostAction(node: HTMLElement) {
    const token = ++sourceDockviewContextHostToken;
    void initializeSourceDockviewContext(node, token);

    return {
      destroy() {
        if (sourceDockviewContextHostToken === token) {
          sourceDockviewContextHostToken += 1;
          disposeSourceDockviewContext();
        }
      }
    };
  }

  function sourceDockviewCenterHostAction(node: HTMLElement) {
    const token = ++sourceDockviewCenterHostToken;
    void initializeSourceDockviewCenter(node, token);

    return {
      destroy() {
        if (sourceDockviewCenterHostToken === token) {
          sourceDockviewCenterHostToken += 1;
          disposeSourceDockviewCenter();
        }
      }
    };
  }

  function sourceDockviewBottomHostAction(node: HTMLElement) {
    const token = ++sourceDockviewBottomHostToken;
    void initializeSourceDockviewBottom(node, token);

    return {
      destroy() {
        if (sourceDockviewBottomHostToken === token) {
          sourceDockviewBottomHostToken += 1;
          disposeSourceDockviewBottom();
        }
      }
    };
  }

  function sourceEditorFileDockviewHostAction(node: HTMLElement) {
    const token = ++sourceEditorFileDockviewHostToken;
    void initializeSourceEditorFileDockview(node, token);

    return {
      destroy() {
        if (sourceEditorFileDockviewHostToken === token) {
          sourceEditorFileDockviewHostToken += 1;
          disposeSourceEditorFileDockview();
        }
      }
    };
  }

  function sourceFilesDockviewHostAction(node: HTMLElement) {
    const token = ++sourceFilesDockviewHostToken;
    void initializeSourceFilesDockview(node, token);

    return {
      destroy() {
        if (sourceFilesDockviewHostToken === token) {
          sourceFilesDockviewHostToken += 1;
          disposeSourceFilesDockview();
        }
      }
    };
  }

  function sourceConversationDockviewHostAction(node: HTMLElement) {
    const token = ++sourceConversationDockviewHostToken;
    void initializeSourceConversationDockview(node, token);

    return {
      destroy() {
        if (sourceConversationDockviewHostToken === token) {
          sourceConversationDockviewHostToken += 1;
          disposeSourceConversationDockview();
        }
      }
    };
  }

  function sourceContextCardDockviewHostAction(node: HTMLElement) {
    const token = ++sourceContextCardDockviewHostToken;
    void initializeSourceContextCardDockview(node, token);

    return {
      destroy() {
        if (sourceContextCardDockviewHostToken === token) {
          sourceContextCardDockviewHostToken += 1;
          disposeSourceContextCardDockview();
        }
      }
    };
  }

  function sourceDockviewPanelAction(node: HTMLElement, panelID: SourceDockPanelID) {
    const handleDockviewLayout = () => {
      if (panelID === 'terminal') {
        void ensureEmbeddedTerminalRenderer().then(scheduleEmbeddedTerminalFit);
      }
    };

    node.addEventListener('source-dockview-layout', handleDockviewLayout);
    registerSourceDockviewPanelElement(panelID, node);

    return {
      update(nextPanelID: SourceDockPanelID) {
        if (nextPanelID === panelID) return;
        unregisterSourceDockviewPanelElement(panelID, node);
        panelID = nextPanelID;
        registerSourceDockviewPanelElement(panelID, node);
      },
      destroy() {
        node.removeEventListener('source-dockview-layout', handleDockviewLayout);
        unregisterSourceDockviewPanelElement(panelID, node);
      }
    };
  }

  function sourceFilesDockviewPanelAction(node: HTMLElement, panelID: SourceFilesPaneID) {
    registerSourceFilesDockviewPanelElement(panelID, node);

    return {
      update(nextPanelID: SourceFilesPaneID) {
        if (nextPanelID === panelID) return;
        unregisterSourceFilesDockviewPanelElement(panelID, node);
        panelID = nextPanelID;
        registerSourceFilesDockviewPanelElement(panelID, node);
      },
      destroy() {
        unregisterSourceFilesDockviewPanelElement(panelID, node);
      }
    };
  }

  function sourceConversationDockviewPanelAction(node: HTMLElement, panelID: SourceConversationPaneID) {
    registerSourceConversationDockviewPanelElement(panelID, node);

    return {
      update(nextPanelID: SourceConversationPaneID) {
        if (nextPanelID === panelID) return;
        unregisterSourceConversationDockviewPanelElement(panelID, node);
        panelID = nextPanelID;
        registerSourceConversationDockviewPanelElement(panelID, node);
      },
      destroy() {
        unregisterSourceConversationDockviewPanelElement(panelID, node);
      }
    };
  }

  function sourceContextCardDockviewPanelAction(node: HTMLElement, panelID: SourceContextCardDockviewPanelID) {
    registerSourceContextCardDockviewPanelElement(panelID, node);

    return {
      update(nextPanelID: SourceContextCardDockviewPanelID) {
        if (nextPanelID === panelID) return;
        unregisterSourceContextCardDockviewPanelElement(panelID, node);
        panelID = nextPanelID;
        registerSourceContextCardDockviewPanelElement(panelID, node);
      },
      destroy() {
        unregisterSourceContextCardDockviewPanelElement(panelID, node);
      }
    };
  }

  function sourceEditorFileDockviewPanelAction(node: HTMLElement, panelID: SourceEditorFilePanelID) {
    registerSourceEditorFileDockviewPanelElement(panelID, node);

    return {
      update(nextPanelID: SourceEditorFilePanelID) {
        if (nextPanelID === panelID) return;
        unregisterSourceEditorFileDockviewPanelElement(panelID, node);
        panelID = nextPanelID;
        registerSourceEditorFileDockviewPanelElement(panelID, node);
      },
      destroy() {
        unregisterSourceEditorFileDockviewPanelElement(panelID, node);
      }
    };
  }

  async function initializeSourceDockviewWorkbench(node: HTMLElement, token: number) {
    if (!sourceDockviewWorkbenchEnabled) return;

    sourceDockviewWorkbenchReady = false;
    sourceDockviewWorkbenchError = '';

    await tick();
    if (token !== sourceDockviewWorkbenchHostToken) return;

    disposeSourceDockviewWorkbench();

    try {
      const workspace = await createSourceDockviewWorkspace(node, {
        layout: sourceDockLayout,
        storedLayout: loadStoredSourceDockviewLayout(sourceDockviewWorkbenchStorageKey),
        onDidLayoutChange: (layout) => {
          persistSourceDockviewLayout(sourceDockviewWorkbenchStorageKey, layout);
          syncSourceDockLayoutFromWorkbenchGroups();
          if (sourceDockviewWorkbenchOwnsPanel('terminal')) scheduleEmbeddedTerminalFit();
        },
        onDidPanelClose: (panelID) => {
          if (panelID === 'editor') {
            syncSourceDockviewWorkbenchLayout(sourceDockLayout);
            fileActionStatus = 'Editor tab restored';
            return;
          }
          if (panelID === 'context') {
            contextPanelCollapsed = true;
            persistContextPanelCollapsed(contextPanelCollapsed);
            applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, 'context'));
            fileActionStatus = 'Context panel hidden';
            return;
          }
          if (panelID === 'insights') {
            editorInsightCollapsed = true;
            persistEditorInsightCollapsed(editorInsightCollapsed);
            applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, 'insights'));
            fileActionStatus = 'Insights panel hidden';
            return;
          }
          applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, panelID));
          fileActionStatus = `${dockPanelLabel(panelID)} dock hidden`;
        },
        onDidActivePanelChange: (panelID) => {
          if (!panelID) return;
          if (panelID === 'context') {
            contextPanelCollapsed = false;
            persistContextPanelCollapsed(contextPanelCollapsed);
          }
          if (panelID === 'insights') {
            editorInsightCollapsed = false;
            persistEditorInsightCollapsed(editorInsightCollapsed);
          }
          sourceDockLayout = activateSourceDockPanel(sourceDockLayout, panelID);
          persistSourceDockLayout(sourceDockLayout);
          if (panelID === 'terminal') scheduleEmbeddedTerminalFit();
        },
        onDidPanelMove: (panelID, layout) => {
          persistSourceDockviewLayout(sourceDockviewWorkbenchStorageKey, layout);
          syncSourceDockLayoutFromWorkbenchGroups(panelID);
        }
      });

      if (token !== sourceDockviewWorkbenchHostToken) {
        workspace.dispose();
        return;
      }

      sourceDockviewWorkbenchWorkspace = workspace;
      for (const panelID of sourceDockviewPanelElements.keys()) {
        syncSourceDockviewPanelElement(panelID);
      }
      sourceDockviewWorkbenchResizeObserver = new ResizeObserver(() =>
        layoutSourceDockviewWorkbench(node)
      );
      sourceDockviewWorkbenchResizeObserver.observe(node);
      sourceDockviewWorkbenchReady = true;
      layoutSourceDockviewWorkbench(node);
    } catch (dockviewError) {
      sourceDockviewWorkbenchError =
        dockviewError instanceof Error ? dockviewError.message : 'Dockview workbench unavailable';
      sourceDockviewWorkbenchReady = false;
      disposeSourceDockviewWorkbench();
    }
  }

  async function initializeSourceDockviewActivity(node: HTMLElement, token: number) {
    if (!sourceDockviewActivityEnabled) return;

    sourceDockviewActivityReady = false;
    sourceDockviewActivityError = '';

    await tick();
    if (token !== sourceDockviewActivityHostToken) return;

    disposeSourceDockviewActivity();

    try {
      const workspace = await createSourceDockviewWorkspace(node, {
        layout: sourceDockLayout,
        ...sourceDockviewActivityPlanOptions,
        storedLayout: loadStoredSourceDockviewLayout(sourceDockviewActivityStorageKey),
        onDidLayoutChange: (layout) =>
          persistSourceDockviewLayout(sourceDockviewActivityStorageKey, layout),
        onDidPanelClose: (panelID) => {
          if (panelID !== 'activity') return;
          applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, 'activity'));
          fileActionStatus = 'Activity panel hidden';
        },
        onDidActivePanelChange: (panelID) => {
          if (panelID !== 'activity') return;
          sourceDockLayout = activateSourceDockPanel(sourceDockLayout, panelID);
          persistSourceDockLayout(sourceDockLayout);
        }
      });

      if (token !== sourceDockviewActivityHostToken) {
        workspace.dispose();
        return;
      }

      sourceDockviewActivityWorkspace = workspace;
      for (const panelID of sourceDockviewPanelElements.keys()) {
        syncSourceDockviewPanelElement(panelID);
      }
      sourceDockviewActivityResizeObserver = new ResizeObserver(() =>
        layoutSourceDockviewActivity(node)
      );
      sourceDockviewActivityResizeObserver.observe(node);
      sourceDockviewActivityReady = true;
      layoutSourceDockviewActivity(node);
    } catch (dockviewError) {
      sourceDockviewActivityError =
        dockviewError instanceof Error ? dockviewError.message : 'Dockview activity dock unavailable';
      sourceDockviewActivityReady = false;
      disposeSourceDockviewActivity();
    }
  }

  async function initializeSourceEditorFileDockview(node: HTMLElement, token: number) {
    sourceEditorFileDockviewReady = false;
    sourceEditorFileDockviewError = '';

    await tick();
    if (token !== sourceEditorFileDockviewHostToken) return;

    disposeSourceEditorFileDockview();

    if (sourceEditorFileDockviewPanels.length === 0) {
      sourceEditorFileDockviewReady = true;
      return;
    }

    try {
      const workspace = await createSourceDockviewTabStackWorkspace(node, {
        panels: sourceEditorFileDockviewPanels,
        rootPanelID: selectedSourceEditorFilePanelID ?? undefined,
        storedLayout: loadStoredSourceDockviewLayout(
          sourceEditorFileDockviewStorageKey,
          sourceEditorFileDockviewPanelIDs
        ),
        onDidLayoutChange: (layout) =>
          persistSourceDockviewLayout(sourceEditorFileDockviewStorageKey, layout),
        onDidPanelClose: (panelID) => {
          const tab = sourceEditorFileTabForPanelID(panelID);
          if (!tab) return;
          void closeSourceTabRecord(tab);
        },
        onDidActivePanelChange: (panelID) => {
          if (!panelID) return;
          const tab = sourceEditorFileTabForPanelID(panelID);
          if (!tab || tab.path === selectedRecord?.path) return;
          void selectOpenTab(tab);
        }
      });

      if (token !== sourceEditorFileDockviewHostToken) {
        workspace.dispose();
        return;
      }

      sourceEditorFileDockviewWorkspace = workspace;
      for (const panelID of sourceEditorFileDockviewPanelElements.keys()) {
        syncSourceEditorFileDockviewPanelElement(panelID);
      }
      sourceEditorFileDockviewResizeObserver = new ResizeObserver(() =>
        layoutSourceEditorFileDockview(node)
      );
      sourceEditorFileDockviewResizeObserver.observe(node);
      sourceEditorFileDockviewReady = true;
      layoutSourceEditorFileDockview(node);
    } catch (dockviewError) {
      sourceEditorFileDockviewError =
        dockviewError instanceof Error ? dockviewError.message : 'Editor file tabs unavailable';
      sourceEditorFileDockviewReady = false;
      disposeSourceEditorFileDockview();
    }
  }

  async function initializeSourceFilesDockview(node: HTMLElement, token: number) {
    sourceFilesDockviewReady = false;
    sourceFilesDockviewError = '';

    await tick();
    if (token !== sourceFilesDockviewHostToken) return;

    disposeSourceFilesDockview();

    try {
      const workspace = await createSourcePaneviewStackWorkspace(node, {
        panels: sourceFilesDockviewPanels,
        storedLayout: loadStoredSourcePaneviewLayout(
          sourceFilesDockviewStorageKey,
          sourceFilesDockviewPanelIDs
        ),
        onDidLayoutChange: (layout) =>
          persistSourcePaneviewLayout(sourceFilesDockviewStorageKey, layout)
      });

      if (token !== sourceFilesDockviewHostToken) {
        workspace.dispose();
        return;
      }

      sourceFilesDockviewWorkspace = workspace;
      for (const panelID of sourceFilesDockviewPanelElements.keys()) {
        syncSourceFilesDockviewPanelElement(panelID);
      }
      sourceFilesDockviewResizeObserver = new ResizeObserver(() =>
        layoutSourceFilesDockview(node)
      );
      sourceFilesDockviewResizeObserver.observe(node);
      sourceFilesDockviewReady = true;
      layoutSourceFilesDockview(node);
    } catch (dockviewError) {
      sourceFilesDockviewError =
        dockviewError instanceof Error ? dockviewError.message : 'Source files panels unavailable';
      sourceFilesDockviewReady = false;
      disposeSourceFilesDockview();
    }
  }

  async function initializeSourceConversationDockview(node: HTMLElement, token: number) {
    sourceConversationDockviewReady = false;
    sourceConversationDockviewError = '';

    await tick();
    if (token !== sourceConversationDockviewHostToken) return;

    disposeSourceConversationDockview();

    try {
      const workspace = await createSourcePaneviewStackWorkspace(node, {
        panels: sourceConversationDockviewPanels,
        storedLayout: loadStoredSourcePaneviewLayout(
          sourceConversationDockviewStorageKey,
          sourceConversationDockviewPanelIDs
        ),
        onDidLayoutChange: (layout) =>
          persistSourcePaneviewLayout(sourceConversationDockviewStorageKey, layout)
      });

      if (token !== sourceConversationDockviewHostToken) {
        workspace.dispose();
        return;
      }

      sourceConversationDockviewWorkspace = workspace;
      for (const panelID of sourceConversationDockviewPanelElements.keys()) {
        syncSourceConversationDockviewPanelElement(panelID);
      }
      sourceConversationDockviewResizeObserver = new ResizeObserver(() =>
        layoutSourceConversationDockview(node)
      );
      sourceConversationDockviewResizeObserver.observe(node);
      sourceConversationDockviewReady = true;
      layoutSourceConversationDockview(node);
    } catch (dockviewError) {
      sourceConversationDockviewError =
        dockviewError instanceof Error ? dockviewError.message : 'Conversation panels unavailable';
      sourceConversationDockviewReady = false;
      disposeSourceConversationDockview();
    }
  }

  async function initializeSourceContextCardDockview(node: HTMLElement, token: number) {
    sourceContextCardDockviewReady = false;
    sourceContextCardDockviewError = '';

    await tick();
    if (token !== sourceContextCardDockviewHostToken) return;

    disposeSourceContextCardDockview();

    if (sourceContextCardDockviewPanels.length === 0) {
      sourceContextCardDockviewReady = true;
      return;
    }

    try {
      const workspace = await createSourcePaneviewStackWorkspace(node, {
        panels: sourceContextCardDockviewPanels,
        storedLayout: loadStoredSourcePaneviewLayout(
          sourceContextCardDockviewStorageKey,
          sourceContextCardDockviewPanelIDs
        ),
        onDidLayoutChange: (layout) =>
          persistSourcePaneviewLayout(sourceContextCardDockviewStorageKey, layout)
      });

      if (token !== sourceContextCardDockviewHostToken) {
        workspace.dispose();
        return;
      }

      sourceContextCardDockviewWorkspace = workspace;
      for (const panelID of sourceContextCardDockviewPanelElements.keys()) {
        syncSourceContextCardDockviewPanelElement(panelID);
      }
      sourceContextCardDockviewResizeObserver = new ResizeObserver(() =>
        layoutSourceContextCardDockview(node)
      );
      sourceContextCardDockviewResizeObserver.observe(node);
      sourceContextCardDockviewReady = true;
      layoutSourceContextCardDockview(node);
    } catch (dockviewError) {
      sourceContextCardDockviewError =
        dockviewError instanceof Error ? dockviewError.message : 'Context card panels unavailable';
      sourceContextCardDockviewReady = false;
      disposeSourceContextCardDockview();
    }
  }

  async function initializeSourceDockviewContext(node: HTMLElement, token: number) {
    if (!sourceDockviewContextEnabled) return;

    sourceDockviewContextReady = false;
    sourceDockviewContextError = '';

    await tick();
    if (token !== sourceDockviewContextHostToken) return;

    disposeSourceDockviewContext();

    try {
      const workspace = await createSourceDockviewWorkspace(node, {
        layout: sourceDockLayout,
        ...sourceDockviewContextPlanOptions,
        storedLayout: loadStoredSourceDockviewLayout(sourceDockviewContextStorageKey),
        onDidLayoutChange: (layout) =>
          persistSourceDockviewLayout(sourceDockviewContextStorageKey, layout),
        onDidPanelClose: (panelID) => {
          if (panelID === 'context') {
            contextPanelCollapsed = true;
            persistContextPanelCollapsed(contextPanelCollapsed);
            applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, 'context'));
            fileActionStatus = 'Context panel hidden';
            return;
          }
          if (panelID === 'insights') {
            editorInsightCollapsed = true;
            persistEditorInsightCollapsed(editorInsightCollapsed);
            applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, 'insights'));
            fileActionStatus = 'Insights panel hidden';
          }
        },
        onDidActivePanelChange: (panelID) => {
          if (panelID !== 'context' && panelID !== 'insights') return;
          if (panelID === 'context') {
            contextPanelCollapsed = false;
            persistContextPanelCollapsed(contextPanelCollapsed);
          }
          if (panelID === 'insights') {
            editorInsightCollapsed = false;
            persistEditorInsightCollapsed(editorInsightCollapsed);
          }
          sourceDockLayout = activateSourceDockPanel(sourceDockLayout, panelID);
          persistSourceDockLayout(sourceDockLayout);
        }
      });

      if (token !== sourceDockviewContextHostToken) {
        workspace.dispose();
        return;
      }

      sourceDockviewContextWorkspace = workspace;
      for (const panelID of sourceDockviewPanelElements.keys()) {
        syncSourceDockviewPanelElement(panelID);
      }
      sourceDockviewContextResizeObserver = new ResizeObserver(() =>
        layoutSourceDockviewContext(node)
      );
      sourceDockviewContextResizeObserver.observe(node);
      sourceDockviewContextReady = true;
      layoutSourceDockviewContext(node);
    } catch (dockviewError) {
      sourceDockviewContextError =
        dockviewError instanceof Error ? dockviewError.message : 'Dockview context dock unavailable';
      sourceDockviewContextReady = false;
      disposeSourceDockviewContext();
    }
  }

  async function initializeSourceDockviewInsights(node: HTMLElement, token: number) {
    if (!sourceDockviewInsightsEnabled) return;

    sourceDockviewInsightsReady = false;
    sourceDockviewInsightsError = '';

    await tick();
    if (token !== sourceDockviewInsightsHostToken) return;

    disposeSourceDockviewInsights();

    try {
      const workspace = await createSourceDockviewWorkspace(node, {
        layout: sourceDockLayout,
        ...sourceDockviewInsightsPlanOptions,
        storedLayout: loadStoredSourceDockviewLayout(sourceDockviewInsightsStorageKey),
        onDidLayoutChange: (layout) =>
          persistSourceDockviewLayout(sourceDockviewInsightsStorageKey, layout),
        onDidPanelClose: (panelID) => {
          if (panelID !== 'insights') return;
          editorInsightCollapsed = true;
          persistEditorInsightCollapsed(editorInsightCollapsed);
          applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, 'insights'));
          fileActionStatus = 'Insights panel hidden';
        }
      });

      if (token !== sourceDockviewInsightsHostToken) {
        workspace.dispose();
        return;
      }

      sourceDockviewInsightsWorkspace = workspace;
      for (const panelID of sourceDockviewPanelElements.keys()) {
        syncSourceDockviewPanelElement(panelID);
      }
      sourceDockviewInsightsResizeObserver = new ResizeObserver(() =>
        layoutSourceDockviewInsights(node)
      );
      sourceDockviewInsightsResizeObserver.observe(node);
      sourceDockviewInsightsReady = true;
      layoutSourceDockviewInsights(node);
    } catch (dockviewError) {
      sourceDockviewInsightsError =
        dockviewError instanceof Error ? dockviewError.message : 'Dockview insights unavailable';
      sourceDockviewInsightsReady = false;
      disposeSourceDockviewInsights();
    }
  }

  async function initializeSourceDockviewCenter(node: HTMLElement, token: number) {
    if (!sourceDockviewCenterEnabled) return;

    sourceDockviewCenterReady = false;
    sourceDockviewCenterError = '';

    await tick();
    if (token !== sourceDockviewCenterHostToken) return;

    disposeSourceDockviewCenter();

    try {
      const workspace = await createSourceDockviewWorkspace(node, {
        layout: sourceDockLayout,
        ...sourceDockviewCenterPlanOptions,
        storedLayout: null,
        restoreStoredLayout: false,
        onDidLayoutChange: (layout) => {
          persistSourceDockviewLayout(sourceDockviewCenterStorageKey, layout);
          if (sourceDockviewCenterOwnsPanel('terminal')) scheduleEmbeddedTerminalFit();
        },
        onDidPanelClose: (panelID) => {
          if (panelID === 'editor') {
            syncSourceDockviewCenterLayout(sourceDockLayout);
            fileActionStatus = 'Editor tab restored';
            return;
          }
          if (panelID !== 'terminal' && panelID !== 'browser') return;
          applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, panelID));
          fileActionStatus = `${dockPanelLabel(panelID)} dock hidden`;
        },
        onDidActivePanelChange: (panelID) => {
          if (panelID !== 'editor' && panelID !== 'terminal' && panelID !== 'browser') return;
          sourceDockLayout = activateSourceDockPanel(sourceDockLayout, panelID);
          persistSourceDockLayout(sourceDockLayout);
          if (panelID === 'terminal') scheduleEmbeddedTerminalFit();
        }
      });

      if (token !== sourceDockviewCenterHostToken) {
        workspace.dispose();
        return;
      }

      sourceDockviewCenterWorkspace = workspace;
      for (const panelID of sourceDockviewPanelElements.keys()) {
        syncSourceDockviewPanelElement(panelID);
      }
      sourceDockviewCenterResizeObserver = new ResizeObserver(() =>
        layoutSourceDockviewCenter(node)
      );
      sourceDockviewCenterResizeObserver.observe(node);
      sourceDockviewCenterReady = true;
      layoutSourceDockviewCenter(node);
    } catch (dockviewError) {
      sourceDockviewCenterError =
        dockviewError instanceof Error ? dockviewError.message : 'Dockview center dock unavailable';
      sourceDockviewCenterReady = false;
      disposeSourceDockviewCenter();
    }
  }

  async function initializeSourceDockviewBottom(node: HTMLElement, token: number) {
    if (!sourceDockviewBottomEnabled) return;

    sourceDockviewBottomReady = false;
    sourceDockviewBottomError = '';

    await tick();
    if (token !== sourceDockviewBottomHostToken) return;

    disposeSourceDockviewBottom();

    try {
      const workspace = await createSourceDockviewWorkspace(node, {
        layout: sourceDockLayout,
        ...sourceDockviewBottomPlanOptions,
        storedLayout: loadStoredSourceDockviewLayout(sourceDockviewBottomStorageKey),
        onDidLayoutChange: (layout) => {
          persistSourceDockviewLayout(sourceDockviewBottomStorageKey, layout);
          if (sourceDockPanelVisible('terminal')) scheduleEmbeddedTerminalFit();
        },
        onDidPanelClose: (panelID) => {
          if (panelID !== 'terminal' && panelID !== 'browser') return;
          applySourceDockLayout(hideSourceDockPanel(sourceDockLayout, panelID));
          fileActionStatus = `${dockPanelLabel(panelID)} dock hidden`;
        },
        onDidActivePanelChange: (panelID) => {
          if (panelID !== 'terminal' && panelID !== 'browser') return;
          sourceDockLayout = activateSourceDockPanel(sourceDockLayout, panelID);
          persistSourceDockLayout(sourceDockLayout);
          if (panelID === 'terminal') scheduleEmbeddedTerminalFit();
        }
      });

      if (token !== sourceDockviewBottomHostToken) {
        workspace.dispose();
        return;
      }

      sourceDockviewBottomWorkspace = workspace;
      for (const panelID of sourceDockviewPanelElements.keys()) {
        syncSourceDockviewPanelElement(panelID);
      }
      sourceDockviewBottomResizeObserver = new ResizeObserver(() =>
        layoutSourceDockviewBottom(node)
      );
      sourceDockviewBottomResizeObserver.observe(node);
      sourceDockviewBottomReady = true;
      layoutSourceDockviewBottom(node);
    } catch (dockviewError) {
      sourceDockviewBottomError =
        dockviewError instanceof Error ? dockviewError.message : 'Dockview bottom dock unavailable';
      sourceDockviewBottomReady = false;
      disposeSourceDockviewBottom();
    }
  }

  function registerSourceDockviewPanelElement(panelID: SourceDockPanelID, element: HTMLElement) {
    sourceDockviewPanelElements.set(panelID, element);
    syncSourceDockviewPanelElement(panelID);
  }

  function unregisterSourceDockviewPanelElement(panelID: SourceDockPanelID, element: HTMLElement) {
    if (sourceDockviewPanelElements.get(panelID) !== element) return;
    sourceDockviewPanelElements.delete(panelID);
    syncSourceDockviewPanelElement(panelID);
  }

  function registerSourceFilesDockviewPanelElement(panelID: SourceFilesPaneID, element: HTMLElement) {
    sourceFilesDockviewPanelElements.set(panelID, element);
    syncSourceFilesDockviewPanelElement(panelID);
  }

  function unregisterSourceFilesDockviewPanelElement(panelID: SourceFilesPaneID, element: HTMLElement) {
    if (sourceFilesDockviewPanelElements.get(panelID) !== element) return;
    sourceFilesDockviewPanelElements.delete(panelID);
    syncSourceFilesDockviewPanelElement(panelID);
  }

  function registerSourceConversationDockviewPanelElement(panelID: SourceConversationPaneID, element: HTMLElement) {
    sourceConversationDockviewPanelElements.set(panelID, element);
    syncSourceConversationDockviewPanelElement(panelID);
  }

  function unregisterSourceConversationDockviewPanelElement(
    panelID: SourceConversationPaneID,
    element: HTMLElement
  ) {
    if (sourceConversationDockviewPanelElements.get(panelID) !== element) return;
    sourceConversationDockviewPanelElements.delete(panelID);
    syncSourceConversationDockviewPanelElement(panelID);
  }

  function registerSourceContextCardDockviewPanelElement(
    panelID: SourceContextCardDockviewPanelID,
    element: HTMLElement
  ) {
    sourceContextCardDockviewPanelElements.set(panelID, element);
    syncSourceContextCardDockviewPanelElement(panelID);
  }

  function unregisterSourceContextCardDockviewPanelElement(
    panelID: SourceContextCardDockviewPanelID,
    element: HTMLElement
  ) {
    if (sourceContextCardDockviewPanelElements.get(panelID) !== element) return;
    sourceContextCardDockviewPanelElements.delete(panelID);
    syncSourceContextCardDockviewPanelElement(panelID);
  }

  function registerSourceEditorFileDockviewPanelElement(
    panelID: SourceEditorFilePanelID,
    element: HTMLElement
  ) {
    sourceEditorFileDockviewPanelElements.set(panelID, element);
    syncSourceEditorFileDockviewPanelElement(panelID);
  }

  function unregisterSourceEditorFileDockviewPanelElement(
    panelID: SourceEditorFilePanelID,
    element: HTMLElement
  ) {
    if (sourceEditorFileDockviewPanelElements.get(panelID) !== element) return;
    sourceEditorFileDockviewPanelElements.delete(panelID);
    syncSourceEditorFileDockviewPanelElement(panelID);
  }

  function syncSourceDockviewPanelElement(panelID: SourceDockPanelID) {
    const element = sourceDockviewPanelElements.get(panelID) ?? null;

    if (useUnifiedWorkbench) {
      // Route the panel's existing content root into the unified Task-2 workbench bridge.
      // Hand the old single-Dockview workbench `null` so the two never fight over the element.
      unifiedWorkbenchSetPanelElement(panelID, sourceDockPanelVisible(panelID) ? element : null);
      sourceDockviewWorkbenchWorkspace?.setPanelElement(panelID, null);
      return;
    }

    sourceDockviewWorkbenchWorkspace?.setPanelElement(
      panelID,
      sourceDockviewWorkbenchOwnsPanel(panelID) ? element : null
    );
    if (sourceDockviewWorkbenchEnabled) return;

    const contextOwnsPanel = sourceDockviewContextOwnsPanel(panelID);
    const centerOwnsPanel = sourceDockviewCenterOwnsPanel(panelID);
    const bottomOwnsPanel = sourceDockviewBottomOwnsPanel(panelID);

    sourceDockviewActivityWorkspace?.setPanelElement(
      panelID,
      panelID === 'activity' && sourceDockviewActivityOwnsPanel(panelID) ? element : null
    );
    sourceDockviewContextWorkspace?.setPanelElement(
      panelID,
      contextOwnsPanel ? element : null
    );
    sourceDockviewInsightsWorkspace?.setPanelElement(
      panelID,
      panelID === 'insights' && !contextOwnsPanel ? element : null
    );
    sourceDockviewCenterWorkspace?.setPanelElement(
      panelID,
      centerOwnsPanel ? element : null
    );
    sourceDockviewBottomWorkspace?.setPanelElement(
      panelID,
      bottomOwnsPanel ? element : null
    );
  }

  function syncAllSourceDockviewPanelElements() {
    for (const panelID of sourceDockviewPanelElements.keys()) {
      syncSourceDockviewPanelElement(panelID);
    }
  }

  function handleUnifiedWorkbenchOwnershipChange() {
    if (!useUnifiedWorkbench) return;
    syncAllSourceDockviewPanelElements();
    if (sourceDockPanelVisible('terminal')) scheduleEmbeddedTerminalFit();
  }

  function syncSourceFilesDockviewPanelElement(panelID: SourceFilesPaneID) {
    sourceFilesDockviewWorkspace?.setPanelElement(
      panelID,
      sourceFilesDockviewPanelElements.get(panelID) ?? null
    );
  }

  function syncSourceConversationDockviewPanelElement(panelID: SourceConversationPaneID) {
    sourceConversationDockviewWorkspace?.setPanelElement(
      panelID,
      sourceConversationDockviewPanelElements.get(panelID) ?? null
    );
  }

  function syncSourceContextCardDockviewPanelElement(panelID: SourceContextCardDockviewPanelID) {
    sourceContextCardDockviewWorkspace?.setPanelElement(
      panelID,
      sourceContextCardDockviewPanelElements.get(panelID) ?? null
    );
  }

  function syncSourceEditorFileDockviewPanelElement(panelID: SourceEditorFilePanelID) {
    sourceEditorFileDockviewWorkspace?.setPanelElement(
      panelID,
      sourceEditorFileDockviewPanelElements.get(panelID) ?? null
    );
  }

  function syncSourceContextCardDockviewPanels() {
    if (!sourceContextCardDockviewWorkspace) return;

    sourceContextCardDockviewWorkspace.syncPanels(sourceContextCardDockviewPanels);
    for (const panelID of sourceContextCardDockviewPanelElements.keys()) {
      syncSourceContextCardDockviewPanelElement(panelID);
    }
  }

  function syncSourceEditorFileDockviewPanels() {
    if (!sourceEditorFileDockviewWorkspace) return;

    sourceEditorFileDockviewWorkspace.syncPanels(
      sourceEditorFileDockviewPanels,
      selectedSourceEditorFilePanelID ?? undefined
    );
    for (const panelID of sourceEditorFileDockviewPanelElements.keys()) {
      syncSourceEditorFileDockviewPanelElement(panelID);
    }
  }

  function sourceFilesDockviewPanelTitle(panelID: SourceFilesPaneID) {
    return sourceFilesDockviewPanels.find((panel) => panel.id === panelID)?.title ?? panelID;
  }

  function sourceConversationDockviewPanelTitle(panelID: SourceConversationPaneID) {
    return sourceConversationDockviewPanels.find((panel) => panel.id === panelID)?.title ?? panelID;
  }

  function sourceDockviewWorkbenchOwnsPanel(panelID: SourceDockPanelID) {
    return sourceDockviewWorkbenchEnabled && sourceDockPanelVisible(panelID);
  }

  function sourceDockviewActivityOwnsPanel(panelID: SourceDockPanelID) {
    return sourceDockviewActivityEnabled && panelID === 'activity' && sourceDockPanelVisible('activity');
  }

  function sourceDockviewContextOwnsPanel(panelID: SourceDockPanelID) {
    if (!sourceDockviewContextEnabled) return false;
    if (panelID === 'context') return sourceDockPanelVisible('context');
    if (panelID !== 'insights') return false;

    const contextGroupID = dockGroupIDForPanel(sourceDockLayout, 'context');
    const insightsGroupID = dockGroupIDForPanel(sourceDockLayout, 'insights');
    return contextGroupID !== null && insightsGroupID === contextGroupID;
  }

  function sourceDockviewCenterOwnsPanel(panelID: SourceDockPanelID) {
    if (!sourceDockviewCenterEnabled) return false;
    if (panelID === 'editor') return sourceDockPanelVisible('editor');
    if (panelID !== 'terminal' && panelID !== 'browser') return false;
    return sourceDockPanelVisible(panelID);
  }

  function sourceDockviewBottomOwnsPanel(panelID: SourceDockPanelID) {
    if (!sourceDockviewBottomEnabled) return false;
    if (panelID !== 'terminal' && panelID !== 'browser') return false;
    return false;
  }

  function runtimeDockPanelRendered() {
    return false;
  }

  function layoutSourceDockviewWorkbench(node: HTMLElement) {
    if (!sourceDockviewWorkbenchWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceDockviewWorkbenchWorkspace.api.layout(width, height, true);
    if (sourceDockviewWorkbenchOwnsPanel('terminal')) scheduleEmbeddedTerminalFit();
  }

  function layoutSourceDockviewActivity(node: HTMLElement) {
    if (!sourceDockviewActivityWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceDockviewActivityWorkspace.api.layout(width, height, true);
  }

  function layoutSourceDockviewContext(node: HTMLElement) {
    if (!sourceDockviewContextWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceDockviewContextWorkspace.api.layout(width, height, true);
  }

  function layoutSourceDockviewInsights(node: HTMLElement) {
    if (!sourceDockviewInsightsWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceDockviewInsightsWorkspace.api.layout(width, height, true);
  }

  function layoutSourceDockviewCenter(node: HTMLElement) {
    if (!sourceDockviewCenterWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceDockviewCenterWorkspace.api.layout(width, height, true);
    if (sourceDockviewCenterOwnsPanel('terminal')) scheduleEmbeddedTerminalFit();
  }

  function layoutSourceDockviewBottom(node: HTMLElement) {
    if (!sourceDockviewBottomWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceDockviewBottomWorkspace.api.layout(width, height, true);
    if (sourceDockPanelVisible('terminal')) scheduleEmbeddedTerminalFit();
  }

  function layoutSourceEditorFileDockview(node: HTMLElement) {
    if (!sourceEditorFileDockviewWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceEditorFileDockviewWorkspace.api.layout(width, height, true);
  }

  function layoutSourceFilesDockview(node: HTMLElement) {
    if (!sourceFilesDockviewWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceFilesDockviewWorkspace.api.layout(width, height);
  }

  function layoutSourceConversationDockview(node: HTMLElement) {
    if (!sourceConversationDockviewWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceConversationDockviewWorkspace.api.layout(width, height);
  }

  function layoutSourceContextCardDockview(node: HTMLElement) {
    if (!sourceContextCardDockviewWorkspace) return;

    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    sourceContextCardDockviewWorkspace.api.layout(width, height);
  }

  function disposeSourceDockviewWorkbench() {
    sourceDockviewWorkbenchResizeObserver?.disconnect();
    sourceDockviewWorkbenchResizeObserver = null;
    sourceDockviewWorkbenchWorkspace?.dispose();
    sourceDockviewWorkbenchWorkspace = null;
    sourceDockviewWorkbenchReady = false;
  }

  function disposeSourceDockviewActivity() {
    sourceDockviewActivityResizeObserver?.disconnect();
    sourceDockviewActivityResizeObserver = null;
    sourceDockviewActivityWorkspace?.dispose();
    sourceDockviewActivityWorkspace = null;
    sourceDockviewActivityReady = false;
  }

  function disposeSourceDockviewInsights() {
    sourceDockviewInsightsResizeObserver?.disconnect();
    sourceDockviewInsightsResizeObserver = null;
    sourceDockviewInsightsWorkspace?.dispose();
    sourceDockviewInsightsWorkspace = null;
    sourceDockviewInsightsReady = false;
  }

  function disposeSourceDockviewContext() {
    sourceDockviewContextResizeObserver?.disconnect();
    sourceDockviewContextResizeObserver = null;
    sourceDockviewContextWorkspace?.dispose();
    sourceDockviewContextWorkspace = null;
    sourceDockviewContextReady = false;
  }

  function disposeSourceDockviewCenter() {
    sourceDockviewCenterResizeObserver?.disconnect();
    sourceDockviewCenterResizeObserver = null;
    sourceDockviewCenterWorkspace?.dispose();
    sourceDockviewCenterWorkspace = null;
    sourceDockviewCenterReady = false;
  }

  function disposeSourceDockviewBottom() {
    sourceDockviewBottomResizeObserver?.disconnect();
    sourceDockviewBottomResizeObserver = null;
    sourceDockviewBottomWorkspace?.dispose();
    sourceDockviewBottomWorkspace = null;
    sourceDockviewBottomReady = false;
  }

  function disposeSourceEditorFileDockview() {
    sourceEditorFileDockviewResizeObserver?.disconnect();
    sourceEditorFileDockviewResizeObserver = null;
    sourceEditorFileDockviewWorkspace?.dispose();
    sourceEditorFileDockviewWorkspace = null;
    sourceEditorFileDockviewReady = false;
  }

  function disposeSourceFilesDockview() {
    sourceFilesDockviewResizeObserver?.disconnect();
    sourceFilesDockviewResizeObserver = null;
    sourceFilesDockviewWorkspace?.dispose();
    sourceFilesDockviewWorkspace = null;
    sourceFilesDockviewReady = false;
  }

  function disposeSourceConversationDockview() {
    sourceConversationDockviewResizeObserver?.disconnect();
    sourceConversationDockviewResizeObserver = null;
    sourceConversationDockviewWorkspace?.dispose();
    sourceConversationDockviewWorkspace = null;
    sourceConversationDockviewReady = false;
  }

  function disposeSourceContextCardDockview() {
    sourceContextCardDockviewResizeObserver?.disconnect();
    sourceContextCardDockviewResizeObserver = null;
    sourceContextCardDockviewWorkspace?.dispose();
    sourceContextCardDockviewWorkspace = null;
    sourceContextCardDockviewReady = false;
  }

  $effect(() => {
    sourceEditorFileDockviewPanels;
    selectedSourceEditorFilePanelID;
    syncSourceEditorFileDockviewPanels();
  });

  $effect(() => {
    sourceContextCardDockviewPanels;
    syncSourceContextCardDockviewPanels();
  });

  function persistDockGroupSize(groupID: SourceDockGroupID, size: number) {
    applySourceDockLayout(resizeSourceDockGroup(sourceDockLayout, groupID, size));
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
    applySourceDockLayout(
      resizeSourceDockGroup(
        moveSourceDockPanel(sourceDockLayout, 'activity', position),
        position,
        sidePaneWidth
      )
    );
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

  function loadStoredSidePaneExpandedWidth() {
    if (typeof window === 'undefined') return sidePaneDefaultWidth;

    const storedExpandedWidth = window.localStorage.getItem(sidePaneExpandedWidthStorageKey);
    if (storedExpandedWidth !== null) {
      return restoreSourcePaneExpandedSize(Number(storedExpandedWidth), activityPaneSizingConfig);
    }

    return restoreSourcePaneExpandedSize(
      Number(window.localStorage.getItem(sidePaneWidthStorageKey)),
      activityPaneSizingConfig
    );
  }

  function persistSidePaneWidth(width: number) {
    if (typeof window === 'undefined') return;
    const clampedWidth = clampSidePaneWidth(width);
    window.localStorage.setItem(sidePaneWidthStorageKey, String(clampedWidth));
    persistSidePaneExpandedWidth(clampedWidth);
  }

  function persistSidePaneExpandedWidth(width: number) {
    const clampedWidth = clampSidePaneWidth(width);
    if (
      deriveSourcePaneState(
        { visible: true, size: clampedWidth },
        activityPaneSizingConfig
      ) !== 'expanded'
    ) return;

    sidePaneExpandedWidth = clampedWidth;
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sidePaneExpandedWidthStorageKey, String(clampedWidth));
  }

  function clampSidePaneWidth(width: number) {
    return clampSourcePaneSize(width, activityPaneSizingConfig);
  }

  function beginSidePaneResize(event: PointerEvent) {
    if (event.button !== 0 || typeof window === 'undefined') return;

    const startX = event.clientX;
    const startWidth = sidePaneWidth;
    let latestRawWidth = startWidth;
    event.preventDefault();
    markSourceLayoutCustom();
    window.document.body.classList.add('resizing-source-pane');

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const delta = sidePanePosition === 'left'
        ? moveEvent.clientX - startX
        : startX - moveEvent.clientX;
      latestRawWidth = startWidth + delta;
      sidePaneWidth = clampSidePaneWidth(latestRawWidth);
      window.setTimeout(measureFileTreeViewport, 0);
    };
    const finishResize = () => {
      const finishedSize = finishSourcePanePointerSize(latestRawWidth, activityPaneSizingConfig, {
        previousExpandedSize: restoreSourcePaneExpandedSize(startWidth, activityPaneSizingConfig, {
          previousExpandedSize: sidePaneExpandedWidth
        })
      });

      if (finishedSize.state === 'collapsed') {
        persistSidePaneExpandedWidth(finishedSize.persistedSize);
        hideDockPanel('activity');
        fileActionStatus = 'Activity pane hidden';
      } else if (finishedSize.state === 'rail') {
        sidePaneWidth = finishedSize.size;
        persistSidePaneWidth(sidePaneWidth);
        persistDockGroupSize(sidePanePosition, sidePaneWidth);
        fileActionStatus = 'Activity panel collapsed to rail';
      } else {
        sidePaneWidth = finishedSize.size;
        persistSidePaneWidth(sidePaneWidth);
        persistDockGroupSize(sidePanePosition, sidePaneWidth);
      }
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
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      toggleActivityPaneRail();
      return;
    }

    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;

    event.preventDefault();
    markSourceLayoutCustom();
    const direction = event.key === 'ArrowLeft' ? -1 : 1;
    const signedDirection = sidePanePosition === 'left' ? direction : -direction;
    if (sidePaneWidth <= sidePaneMinWidth && signedDirection < 0) {
      hideDockPanel('activity');
      fileActionStatus = 'Activity pane hidden';
      return;
    }
    const rawNextSidePaneWidth = sidePaneWidth + signedDirection * 24;
    if (signedDirection < 0) {
      const finishedSize = finishSourcePanePointerSize(rawNextSidePaneWidth, activityPaneSizingConfig, {
        previousExpandedSize: sidePaneExpandedWidth
      });
      if (finishedSize.state === 'collapsed') {
        persistSidePaneExpandedWidth(finishedSize.persistedSize);
        hideDockPanel('activity');
        fileActionStatus = 'Activity pane hidden';
        return;
      }
      if (finishedSize.state === 'rail') {
        sidePaneWidth = finishedSize.size;
        persistSidePaneWidth(sidePaneWidth);
        persistDockGroupSize(sidePanePosition, sidePaneWidth);
        fileActionStatus = 'Activity panel collapsed to rail';
        window.setTimeout(measureFileTreeViewport, 0);
        return;
      }
    }

    const nextSidePaneWidth = clampSidePaneWidth(rawNextSidePaneWidth);
    sidePaneWidth = nextSidePaneWidth;
    persistSidePaneWidth(sidePaneWidth);
    persistDockGroupSize(sidePanePosition, sidePaneWidth);
    window.setTimeout(measureFileTreeViewport, 0);
  }

  function toggleContextPanelCollapsed() {
    markSourceLayoutCustom();
    contextPanelCollapsed = !contextPanelCollapsed;
    persistContextPanelCollapsed(contextPanelCollapsed);
    const contextGroupID = dockGroupForContextPanelPlacement(contextPanelPlacement);
    const nextLayout = contextPanelCollapsed
      ? hideSourceDockPanel(sourceDockLayout, 'context')
      : activateSourceDockPanel(
          moveSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context', contextGroupID),
          'context'
        );
    applySourceDockLayout(nextLayout);
  }

  function toggleEditorInsightCollapsed() {
    markSourceLayoutCustom();
    editorInsightCollapsed = !editorInsightCollapsed;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    const nextLayout = editorInsightCollapsed
      ? hideSourceDockPanel(sourceDockLayout, 'insights')
      : activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'insights'), 'insights');
    applySourceDockLayout(nextLayout);
  }

  function showEditorInsightPanel(panel: SourceIntelligencePanel) {
    markSourceLayoutCustom();
    sourceIntelligencePanel = panel;
    editorInsightCollapsed = false;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    applySourceDockLayout(
      activateSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'insights'), 'insights')
    );
  }

  function clearSourceLookupResults() {
    sourceDefinitionTargets = [];
    sourceDefinitionQuery = '';
    sourceDefinitionError = '';
    sourceDefinitionLoading = false;
    sourceReferenceTargets = [];
    sourceReferenceQuery = '';
    sourceReferenceError = '';
    sourceReferenceLoading = false;
    sourceImplementationTargets = [];
    sourceImplementationQuery = '';
    sourceImplementationError = '';
    sourceImplementationLoading = false;
    sourceTypeDefinitionTargets = [];
    sourceTypeDefinitionQuery = '';
    sourceTypeDefinitionError = '';
    sourceTypeDefinitionLoading = false;
    if (editorNavPanel === 'definitions' || editorNavPanel === 'references') {
      closeEditorNavPanel();
    }
  }

  function isContextCardVisible(cardID: SourceContextCardID) {
    return !hiddenContextCardIDs.has(cardID);
  }

  function visibleContextCardIDs() {
    return contextCardOrder.filter(isContextCardVisible);
  }

  function contextCardsTabbed() {
    return contextPanelPresentation.tabbed;
  }

  function contextCardsStacked() {
    return contextPanelPresentation.stacked;
  }

  function contextCardsUsePaneviewStack() {
    // In the unified workbench the context panel always renders the expanded, Dockview-sized
    // paneview card stack (Runs / Runtime / Agents / Worktrees / Git) — never the rail or a
    // single tabbed card — regardless of the old grid/stack mode or placement state.
    if (useUnifiedWorkbench) return sourceDockviewWorkbenchOwnsPanel('context');
    return contextCardsTabbed() && (sourceDockviewWorkbenchOwnsPanel('context') || !contextPaneRailOnly());
  }

  function contextCardsUseRailTabs() {
    if (useUnifiedWorkbench) return false;
    return contextCardsTabbed() && !sourceDockviewWorkbenchOwnsPanel('context') && contextPaneRailOnly();
  }

  function resolveContextPanelPresentation(
    mode: SourceContextPanelMode,
    placement: SourceContextPanelPlacement
  ): SourceContextPanelPresentation {
    const presentationMode: SourceContextPanelPresentationMode =
      mode === 'stack' || placement === 'side' ? 'tabs' : 'grid';

    return {
      requestedMode: mode,
      placement,
      presentationMode,
      stacked: presentationMode !== 'grid',
      tabbed: presentationMode === 'tabs',
      singleCard: presentationMode === 'tabs'
    };
  }

  function activeVisibleContextCardID() {
    const visibleCardIDs = visibleContextCardIDs();
    return visibleCardIDs.includes(activeContextCardID) ? activeContextCardID : (visibleCardIDs[0] ?? null);
  }

  function shouldRenderContextCard(cardID: SourceContextCardID) {
    if (!isContextCardVisible(cardID)) return false;
    if (contextCardsUsePaneviewStack()) return true;
    return !contextPanelPresentation.singleCard || activeVisibleContextCardID() === cardID;
  }

  function selectActiveContextCard(cardID: SourceContextCardID) {
    markSourceLayoutCustom();
    if (contextPaneRailOnly()) {
      expandContextPaneFromRail();
    }
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
    if (contextPaneRailOnly()) {
      expandContextPaneFromRail();
    }
    persistContextPanelCollapsed(contextPanelCollapsed);
    applySourceDockLayout(
      activateSourceDockPanel(
        moveSourceDockPanel(
          showSourceDockPanel(sourceDockLayout, 'context'),
          'context',
          dockGroupForContextPanelPlacement(contextPanelPlacement)
        ),
        'context'
      )
    );
  }

  function showContextCard(cardID: SourceContextCardID) {
    markSourceLayoutCustom();
    const nextCardIDs = new Set(hiddenContextCardIDs);
    nextCardIDs.delete(cardID);
    hiddenContextCardIDs = nextCardIDs;
    activeContextCardID = cardID;
    contextPanelCollapsed = false;
    if (contextPaneRailOnly()) {
      expandContextPaneFromRail();
    }
    persistHiddenContextCards(nextCardIDs);
    persistActiveContextCard(cardID);
    persistContextPanelCollapsed(contextPanelCollapsed);
    applySourceDockLayout(
      activateSourceDockPanel(
        moveSourceDockPanel(
          showSourceDockPanel(sourceDockLayout, 'context'),
          'context',
          dockGroupForContextPanelPlacement(contextPanelPlacement)
        ),
        'context'
      )
    );
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
    const contextGroupID = dockGroupForContextPanelPlacement(placement);
    applySourceDockLayout(
      resizeSourceDockGroup(
        moveSourceDockPanel(showSourceDockPanel(sourceDockLayout, 'context'), 'context', contextGroupID),
        contextGroupID,
        placement === 'bottom' ? contextPaneHeight : contextPaneWidth
      )
    );
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
    let latestRawWidth = startWidth;
    event.preventDefault();
    markSourceLayoutCustom();
    editorInsightCollapsed = false;
    persistEditorInsightCollapsed(editorInsightCollapsed);
    window.document.body.classList.add('resizing-editor-insight');

    const handlePointerMove = (moveEvent: PointerEvent) => {
      latestRawWidth = startWidth - (moveEvent.clientX - startX);
      editorInsightWidth = clampEditorInsightWidth(latestRawWidth);
    };
    const finishResize = () => {
      if (latestRawWidth <= editorInsightCollapseThreshold) {
        hideDockPanel('insights');
        fileActionStatus = 'Insights pane hidden';
      } else {
        persistEditorInsightWidth(editorInsightWidth);
        const insightsGroupID = dockGroupIDForPanel(sourceDockLayout, 'insights');
        if (insightsGroupID !== null) {
          persistDockGroupSize(insightsGroupID, editorInsightWidth);
        }
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
    if (editorInsightWidth <= editorInsightMinWidth && direction < 0) {
      hideDockPanel('insights');
      fileActionStatus = 'Insights pane hidden';
      return;
    }
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

  function loadStoredContextPaneExpandedWidth() {
    if (typeof window === 'undefined') return contextPaneDefaultWidth;

    const storedExpandedWidth = window.localStorage.getItem(contextPaneExpandedWidthStorageKey);
    if (storedExpandedWidth !== null) {
      return restoreSourcePaneExpandedSize(Number(storedExpandedWidth), contextPaneWidthSizingConfig);
    }

    return restoreSourcePaneExpandedSize(
      Number(window.localStorage.getItem(contextPaneWidthStorageKey)),
      contextPaneWidthSizingConfig
    );
  }

  function persistContextPaneWidth(width: number) {
    if (typeof window === 'undefined') return;
    const clampedWidth = clampContextPaneWidth(width);
    window.localStorage.setItem(contextPaneWidthStorageKey, String(clampedWidth));
    persistContextPaneExpandedWidth(clampedWidth);
  }

  function persistContextPaneExpandedWidth(width: number) {
    const clampedWidth = clampContextPaneWidth(width);
    if (
      deriveSourcePaneState(
        { visible: true, size: clampedWidth },
        contextPaneWidthSizingConfig
      ) !== 'expanded'
    ) return;

    contextPaneExpandedWidth = clampedWidth;
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(contextPaneExpandedWidthStorageKey, String(clampedWidth));
  }

  function clampContextPaneWidth(width: number) {
    return clampSourcePaneSize(width, contextPaneWidthSizingConfig);
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
    return clampSourcePaneSize(height, contextPaneHeightSizingConfig);
  }

  function beginContextPaneResize(event: PointerEvent) {
    if (event.button !== 0 || typeof window === 'undefined') return;

    const startX = event.clientX;
    const startY = event.clientY;
    const startWidth = contextPaneWidth;
    const startHeight = contextPaneHeight;
    let latestRawSize = contextPanelPlacement === 'bottom' ? startHeight : startWidth;
    event.preventDefault();
    markSourceLayoutCustom();
    const resizingClass = contextPanelPlacement === 'bottom'
      ? 'resizing-context-pane-bottom'
      : 'resizing-context-pane';
    window.document.body.classList.add(resizingClass);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      if (contextPanelPlacement === 'bottom') {
        latestRawSize = startHeight - (moveEvent.clientY - startY);
        contextPaneHeight = clampContextPaneHeight(latestRawSize);
        return;
      }

      latestRawSize = startWidth - (moveEvent.clientX - startX);
      contextPaneWidth = clampContextPaneWidth(latestRawSize);
    };
    const finishResize = () => {
      const finishedSize = finishSourcePanePointerSize(
        latestRawSize,
        contextPanelPlacement === 'bottom' ? contextPaneHeightSizingConfig : contextPaneWidthSizingConfig,
        {
          previousExpandedSize:
            contextPanelPlacement === 'bottom'
              ? contextPaneHeight
              : restoreSourcePaneExpandedSize(startWidth, contextPaneWidthSizingConfig, {
                  previousExpandedSize: contextPaneExpandedWidth
                })
        }
      );

      if (finishedSize.state === 'collapsed') {
        if (contextPanelPlacement === 'side') {
          persistContextPaneExpandedWidth(finishedSize.persistedSize);
        }
        hideDockPanel('context');
        fileActionStatus = 'Context pane hidden';
      } else if (contextPanelPlacement === 'side' && finishedSize.state === 'rail') {
        contextPaneWidth = finishedSize.size;
        persistContextPaneWidth(contextPaneWidth);
        persistDockGroupSize(dockGroupForContextPanelPlacement(contextPanelPlacement), contextPaneWidth);
        fileActionStatus = 'Context panel collapsed to rail';
      } else if (contextPanelPlacement === 'bottom') {
        contextPaneHeight = finishedSize.size;
        persistContextPaneHeight(contextPaneHeight);
        persistDockGroupSize('bottom', contextPaneHeight);
      } else {
        contextPaneWidth = finishedSize.size;
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
      if (contextPaneHeight <= contextPaneMinHeight && direction < 0) {
        hideDockPanel('context');
        fileActionStatus = 'Context pane hidden';
        return;
      }
      const rawNextContextPaneHeight = contextPaneHeight + direction * 24;
      if (direction < 0) {
        const finishedSize = finishSourcePanePointerSize(rawNextContextPaneHeight, contextPaneHeightSizingConfig, {
          previousExpandedSize: contextPaneHeight
        });
        if (finishedSize.state === 'collapsed') {
          hideDockPanel('context');
          fileActionStatus = 'Context pane hidden';
          return;
        }
        contextPaneHeight = finishedSize.size;
      } else {
        contextPaneHeight = clampContextPaneHeight(rawNextContextPaneHeight);
      }
      persistContextPaneHeight(contextPaneHeight);
      persistDockGroupSize('bottom', contextPaneHeight);
      return;
    }

    const direction = event.key === 'ArrowLeft' ? 1 : -1;
    if (contextPaneWidth <= contextPaneMinWidth && direction < 0) {
      hideDockPanel('context');
      fileActionStatus = 'Context pane hidden';
      return;
    }
    const rawNextContextPaneWidth = contextPaneWidth + direction * 24;
    if (direction < 0) {
      const finishedSize = finishSourcePanePointerSize(rawNextContextPaneWidth, contextPaneWidthSizingConfig, {
        previousExpandedSize: contextPaneExpandedWidth
      });
      if (finishedSize.state === 'collapsed') {
        persistContextPaneExpandedWidth(finishedSize.persistedSize);
        hideDockPanel('context');
        fileActionStatus = 'Context pane hidden';
        return;
      }
      if (finishedSize.state === 'rail') {
        contextPaneWidth = finishedSize.size;
        persistContextPaneWidth(contextPaneWidth);
        persistDockGroupSize(dockGroupForContextPanelPlacement(contextPanelPlacement), contextPaneWidth);
        fileActionStatus = 'Context panel collapsed to rail';
        return;
      }
    }

    const nextContextPaneWidth = clampContextPaneWidth(rawNextContextPaneWidth);
    contextPaneWidth = nextContextPaneWidth;
    persistContextPaneWidth(contextPaneWidth);
    persistDockGroupSize(dockGroupForContextPanelPlacement(contextPanelPlacement), contextPaneWidth);
  }

  function bottomDockPanelVisible() {
    return sourceDockviewBottomOwnsPanel('terminal') || sourceDockviewBottomOwnsPanel('browser');
  }

  function bottomDockHeight() {
    return clampBottomDockHeight(sourceDockGroupSize(sourceDockLayout, 'bottom'));
  }

  function bottomDockSizingConfig(): SourcePaneSizingConfig {
    return {
      defaultSize: bottomDockDefaultHeight,
      minSize: bottomDockMinHeight,
      maxSize: bottomDockAvailableMaxHeight(),
      collapseThreshold: bottomDockCollapseThreshold
    };
  }

  function bottomDockAvailableMaxHeight() {
    const measuredHeight = Math.max(0, Math.round(sourceWorkspaceHeight));
    const viewportHeight =
      measuredHeight > 0
        ? measuredHeight
        : typeof window !== 'undefined'
          ? Math.max(0, Math.round(window.innerHeight))
          : bottomDockFallbackMaxHeight;
    const availableHeight = viewportHeight - bottomDockReservedEditorHeight - bottomDockReservedChromeHeight;

    return Math.max(bottomDockMinHeight, Math.round(availableHeight));
  }

  function clampBottomDockHeight(height: number) {
    return clampSourcePaneSize(height, bottomDockSizingConfig());
  }

  function beginBottomDockResize(event: PointerEvent) {
    if (event.button !== 0 || typeof window === 'undefined') return;

    const startY = event.clientY;
    const startHeight = bottomDockHeight();
    let latestRawHeight = startHeight;
    event.preventDefault();
    markSourceLayoutCustom();
    window.document.body.classList.add('resizing-bottom-dock');

    const handlePointerMove = (moveEvent: PointerEvent) => {
      latestRawHeight = startHeight - (moveEvent.clientY - startY);
      persistDockGroupSize('bottom', clampBottomDockHeight(latestRawHeight));
    };
    const finishResize = () => {
      const finishedSize = finishSourcePanePointerSize(latestRawHeight, bottomDockSizingConfig(), {
        previousExpandedSize: startHeight
      });
      if (finishedSize.state === 'collapsed') {
        let nextLayout = sourceDockLayout;
        if (sourceDockPanelVisible('terminal')) {
          nextLayout = hideSourceDockPanel(nextLayout, 'terminal');
        }
        if (sourceDockPanelVisible('browser')) {
          nextLayout = hideSourceDockPanel(nextLayout, 'browser');
        }
        applySourceDockLayout(nextLayout);
        fileActionStatus = 'Bottom dock hidden';
      } else {
        persistDockGroupSize('bottom', finishedSize.size);
      }
      window.document.body.classList.remove('resizing-bottom-dock');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', finishResize);
      window.removeEventListener('pointercancel', finishResize);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', finishResize);
    window.addEventListener('pointercancel', finishResize);
  }

  function handleBottomDockResizerKeydown(event: KeyboardEvent) {
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;

    event.preventDefault();
    markSourceLayoutCustom();
    const direction = event.key === 'ArrowUp' ? 1 : -1;
    if (bottomDockHeight() <= bottomDockMinHeight && direction < 0) {
      let nextLayout = sourceDockLayout;
      if (sourceDockPanelVisible('terminal')) {
        nextLayout = hideSourceDockPanel(nextLayout, 'terminal');
      }
      if (sourceDockPanelVisible('browser')) {
        nextLayout = hideSourceDockPanel(nextLayout, 'browser');
      }
      applySourceDockLayout(nextLayout);
      fileActionStatus = 'Bottom dock hidden';
      return;
    }
    const rawNextBottomDockHeight = bottomDockHeight() + direction * 24;
    if (direction < 0) {
      const finishedSize = finishSourcePanePointerSize(rawNextBottomDockHeight, bottomDockSizingConfig(), {
        previousExpandedSize: bottomDockHeight()
      });
      if (finishedSize.state === 'collapsed') {
        let nextLayout = sourceDockLayout;
        if (sourceDockPanelVisible('terminal')) {
          nextLayout = hideSourceDockPanel(nextLayout, 'terminal');
        }
        if (sourceDockPanelVisible('browser')) {
          nextLayout = hideSourceDockPanel(nextLayout, 'browser');
        }
        applySourceDockLayout(nextLayout);
        fileActionStatus = 'Bottom dock hidden';
        return;
      }
      persistDockGroupSize('bottom', finishedSize.size);
      return;
    }

    persistDockGroupSize('bottom', clampBottomDockHeight(rawNextBottomDockHeight));
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

  function persistSourceScanCache(cache: SourceScanCache) {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(sourceScanCacheStorageKey, JSON.stringify(cache));
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

  function loadStoredSourceScanCache(): SourceScanCache {
    if (typeof window === 'undefined') return {};

    try {
      const storedValue = window.localStorage.getItem(sourceScanCacheStorageKey);
      if (!storedValue) return {};

      return parseStoredSourceScanCache(
        JSON.parse(storedValue),
        Date.now(),
        sourceScanCacheMaxAgeMs,
        maxSourceScanCacheEntries
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

      await addCustomProjectRoot('', selectedFolder, false);
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
    void addCustomProjectRoot(projectNameInput, projectPathInput, true);
  }

  function projectRootValidationKey(path: string) {
    return normalizeProjectPath(path);
  }

  function rememberProjectRootValidation(validation: ProjectRootValidationResult) {
    projectRootValidationByPath = {
      ...projectRootValidationByPath,
      [projectRootValidationKey(validation.path)]: validation
    };
  }

  function projectRootValidationSummary(validation: ProjectRootValidationResult | null) {
    if (!validation) return 'Root not checked';
    if (!validation.exists) return `Missing root: ${validation.path}`;
    if (!validation.isDirectory) return 'Path is a file';
    if (!validation.isGitRepository && validation.gitRoot) return 'Inside Git root';
    return validation.isGitRepository ? 'Git root ready' : 'Not a Git root';
  }

  function projectRootGitRootSuggestion(
    project: ProjectRoot,
    validation: ProjectRootValidationResult | null
  ) {
    const gitRoot = validation?.gitRoot?.trim();
    if (!validation || !validation.exists || !validation.isDirectory || validation.isGitRepository || !gitRoot) {
      return '';
    }

    const normalizedGitRoot = normalizeProjectPath(gitRoot);
    return normalizedGitRoot && normalizedGitRoot !== normalizeProjectPath(project.path)
      ? normalizedGitRoot
      : '';
  }

  async function validateProjectRootForProject(project: ProjectRoot, report = false): Promise<ProjectRootValidationResult | null | false> {
    projectRootValidating = true;

    try {
      const validation = await validateProjectRootFromTauri(project.path);
      if (!validation) {
        if (report) {
          fileActionStatus = 'Project root validation unavailable in this runtime';
        }
        return null;
      }

      rememberProjectRootValidation(validation);

      if (!validation.exists || !validation.isDirectory) {
        fileActionStatus = validation.message;
        return false;
      }

      if (report || !validation.isGitRepository) {
        fileActionStatus = validation.message;
      }

      return validation;
    } catch (validationError) {
      if (report) {
        fileActionStatus =
          validationError instanceof Error ? validationError.message : 'Could not validate project root';
      }
      return false;
    } finally {
      projectRootValidating = false;
    }
  }

  async function validateProjectRootBeforeAdd(project: ProjectRoot): Promise<ProjectRootValidationResult | null | false> {
    const validation = await validateProjectRootForProject(project, true);
    if (validation === false) {
      projectFormError = fileActionStatus || 'Could not validate project root';
    }
    return validation;
  }

  async function addCustomProjectRoot(name: string, path: string, reportDuplicate: boolean) {
    const requestedProject = createProjectRoot(name, path);

    if (!requestedProject.path) {
      projectFormError = 'Path is required';
      return false;
    }

    const validation = await validateProjectRootBeforeAdd(requestedProject);
    if (validation === false) return false;

    const nextProject = projectRootForValidatedAdd(requestedProject, validation);
    const normalizedNextProjectPath = normalizeProjectPath(nextProject.path);
    const duplicateProject = projectOptions.find(
      (project) => normalizeProjectPath(project.path) === normalizedNextProjectPath
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
    if (
      validation &&
      !validation.isGitRepository &&
      validation.gitRoot &&
      normalizeProjectPath(validation.gitRoot) === nextProject.path
    ) {
      fileActionStatus = `Using Git root ${validation.gitRoot}. ${fileActionStatus}`;
    } else if (validation && !validation.isGitRepository) {
      fileActionStatus = `${validation.message} ${fileActionStatus}`;
    }
    void activateProject(nextProject, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects: nextProjectOptions,
      clearFileFilter: true
    });
    return true;
  }

  function projectRootForValidatedAdd(
    project: ProjectRoot,
    validation: ProjectRootValidationResult | null
  ): ProjectRoot {
    const gitRoot = validation?.gitRoot?.trim();
    if (!gitRoot || normalizeProjectPath(gitRoot) === project.path) {
      return project;
    }

    return createProjectRoot(project.name, gitRoot);
  }

  async function repairSavedNestedProjectRoot(
    project: ProjectRoot,
    validation: ProjectRootValidationResult | null | false,
    projects: ProjectRoot[],
    scanLimit: number
  ) {
    if (!validation) return false;

    const gitRoot = projectRootGitRootSuggestion(project, validation);
    if (!gitRoot) return false;

    const normalizedProjectPath = normalizeProjectPath(project.path);
    const savedCustomProject = customProjectRoots.find(
      (candidate) =>
        candidate.id === project.id ||
        normalizeProjectPath(candidate.path) === normalizedProjectPath
    );
    if (!savedCustomProject) return false;

    const normalizedGitRoot = normalizeProjectPath(gitRoot);
    const duplicateProject = projects.find(
      (candidate) =>
        candidate.id !== project.id &&
        normalizeProjectPath(candidate.path) === normalizedGitRoot
    );
    const repairedProject = duplicateProject ?? createProjectRoot(project.name, normalizedGitRoot);
    const nextCustomProjectRoots = mergeProjectRoots(
      [],
      [
        ...customProjectRoots.filter((candidate) => {
          const candidatePath = normalizeProjectPath(candidate.path);
          return (
            candidate.id !== savedCustomProject.id &&
            candidatePath !== normalizedProjectPath &&
            candidatePath !== normalizedGitRoot
          );
        }),
        ...(duplicateProject ? [] : [repairedProject])
      ]
    );
    const nextProjectOptions = mergeProjectRoots(defaultProjectRoots, nextCustomProjectRoots);
    const nextSelectedSourcePaths = { ...selectedSourcePaths };
    if (nextSelectedSourcePaths[project.id] && !nextSelectedSourcePaths[repairedProject.id]) {
      nextSelectedSourcePaths[repairedProject.id] = nextSelectedSourcePaths[project.id];
    }
    delete nextSelectedSourcePaths[project.id];

    customProjectRoots = nextCustomProjectRoots;
    selectedSourcePaths = nextSelectedSourcePaths;
    persistCustomProjectRoots(nextCustomProjectRoots);
    persistSelectedSourcePaths(nextSelectedSourcePaths);
    sourceScanCache = removeSourceScanCacheEntries(sourceScanCache, project);
    persistSourceScanCache(sourceScanCache);
    fileActionStatus = `Detected nested project root ${project.path}. Scanning Git root ${repairedProject.path}.`;

    await activateProject(repairedProject, {
      forceScan: true,
      scanLimit,
      projects: nextProjectOptions,
      clearFileFilter: true
    });
    return true;
  }

  async function useValidatedGitRootForSelectedProject() {
    let validation = selectedProjectRootValidation;
    if (!projectRootGitRootSuggestion(selectedProject, validation)) {
      const nextValidation = await validateProjectRootForProject(selectedProject, true);
      if (!nextValidation) return false;
      validation = nextValidation;
    }

    const gitRoot = projectRootGitRootSuggestion(selectedProject, validation);
    if (!gitRoot) {
      fileActionStatus = `${selectedProject.name} is already using the repository root.`;
      return false;
    }

    const nextProject = createProjectRoot(selectedProject.name, gitRoot);
    const duplicateProject = projectOptions.find(
      (project) => normalizeProjectPath(project.path) === nextProject.path
    );

    if (duplicateProject) {
      activateDuplicateProjectRoot(
        duplicateProject,
        projectOptions,
        `Using existing Git root ${duplicateProject.path}.`
      );
      return true;
    }

    const nextCustomProjectRoots = selectedProjectIsCustom
      ? customProjectRoots.map((project) => (project.id === selectedProject.id ? nextProject : project))
      : [...customProjectRoots, nextProject];
    const nextProjectOptions = mergeProjectRoots(defaultProjectRoots, nextCustomProjectRoots);
    customProjectRoots = nextCustomProjectRoots;
    persistCustomProjectRoots(nextCustomProjectRoots);
    fileActionStatus = `Switched ${selectedProject.name} to Git root ${nextProject.path}.`;
    void activateProject(nextProject, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects: nextProjectOptions,
      clearFileFilter: true
    });
    return true;
  }

  async function repairSelectedProjectOnboarding() {
    const project = selectedProject;
    fileActionStatus = `Checking ${project.name} project setup`;

    const validation = await validateProjectRootForProject(project, true);
    if (selectedProject.id !== project.id || validation === false) return false;

    if (projectRootGitRootSuggestion(project, validation)) {
      return useValidatedGitRootForSelectedProject();
    }

    await activateProject(project, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects: projectOptions
    });
    return true;
  }

  function activateDuplicateProjectRoot(project: ProjectRoot, projects: ProjectRoot[], message: string) {
    addingProject = false;
    projectFormError = '';
    const scanStatus = sourceOnboardingScanStatus(project);
    fileActionStatus = message ? `${message} ${scanStatus}` : scanStatus;
    void activateProject(project, {
      forceScan: true,
      scanLimit: expandedSourceScanLimit,
      projects,
      clearFileFilter: true
    });
  }

  async function activateProject(project: ProjectRoot, options: ProjectActivationOptions = {}) {
    const projects = options.projects ?? projectOptions;
    const activationGeneration = ++projectActivationGeneration;
    const scanLimit = options.scanLimit ?? expandedSourceScanLimit;
    selectedProjectID = project.id;
    persistSelectedProjectID(project.id);
    if (options.clearFileFilter) {
      query = '';
    }
    if (options.forceScan) {
      resetProjectOnboardingScanState(project);
    }

    const validation = await validateProjectRootForProject(project);
    if (activationGeneration !== projectActivationGeneration) return;
    const repairedNestedRoot = await repairSavedNestedProjectRoot(
      project,
      validation,
      projects,
      scanLimit
    );
    if (repairedNestedRoot || activationGeneration !== projectActivationGeneration) return;

    await loadGitRepositorySummaries(projects);
    if (activationGeneration !== projectActivationGeneration) return;

    void loadProjectGitStatus(project);
    void loadGitCommitHistory(project);
    void loadRuntimeContexts(projects);
    void loadProjectWorktrees(project);
    void loadAgentSessions();
    void loadOrchestrationRuns(projects);

    const sourceSignature = sourceScanCacheSignatureForProject(project);
    const cachedScan =
      options.forceScan || !sourceSignature
        ? null
        : getSourceScanCacheEntry(
            sourceScanCache,
            project,
            scanLimit,
            Date.now(),
            sourceScanCacheMaxAgeMs,
            sourceSignature
          );
    const activationScanPlan = buildProjectActivationScanPlan({
      project,
      entry: cachedScan,
      forceScan: Boolean(options.forceScan),
      limit: scanLimit,
      suspiciousThreshold: suspiciousSourceIndexFileThreshold
    });
    if (activationScanPlan.shouldScan) {
      fileActionStatus = activationScanPlan.status;
    }
    const activationScanForcesRefresh = activationScanPlan.shouldScan;

    const scanCompletion = scanProject(project, selectedSourcePaths[project.id], {
      force: activationScanForcesRefresh,
      limit: scanLimit,
      preserveSelectedRecord: options.preserveSelectedRecordOnScan
    })
      .then(() => {
        if (activationGeneration !== projectActivationGeneration) return;
        void indexProjectsInBackground(projects);
      })
      .catch((activationError) => {
        if (activationGeneration !== projectActivationGeneration) return;
        fileActionStatus =
          activationError instanceof Error ? activationError.message : `Could not activate ${project.name}`;
      });

    if (options.waitForScan) {
      await scanCompletion;
    }
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
      query = '';
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

  function measureSourceWorkspaceSize() {
    if (typeof window === 'undefined') return;

    const measuredWidth = sourceWorkspaceElement?.clientWidth ?? window.innerWidth;
    const measuredHeight = sourceWorkspaceElement?.clientHeight ?? window.innerHeight;
    sourceWorkspaceWidth = Math.max(0, Math.round(measuredWidth));
    sourceWorkspaceHeight = Math.max(0, Math.round(measuredHeight));
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
    const orchestrationRefreshTimer = window.setInterval(() => {
      if (document.visibilityState !== 'visible') return;
      void loadOrchestrationRuns(projectOptions, { background: true });
    }, orchestrationRefreshIntervalMs);

    const storedCustomProjectRoots = loadStoredCustomProjectRoots();
    const storedProjectOptions = mergeProjectRoots(defaultProjectRoots, storedCustomProjectRoots);
    const storedProjectID = loadStoredSelectedProjectID(storedProjectOptions);
    const storedSelectedSourcePaths = loadStoredSelectedSourcePaths();
    const storedSourceScanCache = loadStoredSourceScanCache();
    const storedRecentSourceRecords = loadStoredRecentSourceRecords();
    const storedOpenSourceTabs = loadStoredOpenSourceTabs();
    const storedWorkspaceSnapshots = loadStoredWorkspaceSnapshots();
    const storedActiveWorkspaceSessionKey = loadStoredActiveWorkspaceSessionKey();
    const storedSourceActivityMode = loadStoredSourceActivityMode();
    const storedPasteCleanupMode = loadStoredPasteCleanupMode();
    const storedPasteCleanupHistory = loadStoredPasteCleanupHistory();
    const storedSourceLayoutPreset = loadStoredSourceLayoutPreset();
    const storedSourceLayoutPresetOverrides = loadStoredSourceLayoutPresetOverrides();
    const storedSourceFocusRestoreLayout = loadStoredSourceFocusRestoreLayout();
    const storedSourceChromeCompact = loadStoredSourceChromeCompact();
    const storedSourceTerminalApp = loadStoredSourceTerminalApp();
    const storedBrowserDockUrl = loadStoredBrowserDockUrl();
    const storedSidePanePosition = loadStoredSidePanePosition();
    const storedSidePaneWidth = loadStoredSidePaneWidth();
    const storedSidePaneExpandedWidth = loadStoredSidePaneExpandedWidth();
    const storedEditorInsightWidth = loadStoredEditorInsightWidth();
    const storedEditorInsightCollapsed = loadStoredEditorInsightCollapsed();
    const storedContextPaneWidth = loadStoredContextPaneWidth();
    const storedContextPaneExpandedWidth = loadStoredContextPaneExpandedWidth();
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
    const startupWorkspaceSnapshot = storedActiveWorkspaceSessionKey
      ? selectStartupWorkspaceSnapshot(storedWorkspaceSnapshots, {
          activeSessionKey: storedActiveWorkspaceSessionKey
        })
      : null;
    const startupSnapshotProjectAlreadyKnown = startupWorkspaceSnapshot
      ? storedProjectOptions.some(
          (project) =>
            project.id === startupWorkspaceSnapshot.project.id ||
            normalizeProjectPath(project.path) === normalizeProjectPath(startupWorkspaceSnapshot.project.path)
        )
      : true;
    const startupCustomProjectRoots = startupWorkspaceSnapshot && !startupSnapshotProjectAlreadyKnown
      ? mergeProjectRoots([], [...storedCustomProjectRoots, startupWorkspaceSnapshot.project])
      : storedCustomProjectRoots;
    const startupProjectOptions = mergeProjectRoots(defaultProjectRoots, startupCustomProjectRoots);
    const startupProject = startupWorkspaceSnapshot
      ? startupProjectOptions.find(
          (project) =>
            project.id === startupWorkspaceSnapshot.project.id ||
            normalizeProjectPath(project.path) === normalizeProjectPath(startupWorkspaceSnapshot.project.path)
        ) ?? startupWorkspaceSnapshot.project
      : storedProject;

    customProjectRoots = startupCustomProjectRoots;
    if (!startupSnapshotProjectAlreadyKnown) {
      persistCustomProjectRoots(startupCustomProjectRoots);
    }
    selectedSourcePaths = storedSelectedSourcePaths;
    sourceScanCache = storedSourceScanCache;
    recentSourceRecords = storedRecentSourceRecords;
    openSourceTabs = storedOpenSourceTabs;
    workspaceSnapshots = storedWorkspaceSnapshots;
    activeWorkspaceSessionKey = startupWorkspaceSnapshot?.id ?? storedActiveWorkspaceSessionKey;
    selectedProjectID = startupProject.id;
    sourceActivityMode = migrateSourceLayout ? compactPreset.activityMode : storedSourceActivityMode;
    pasteCleanupMode = storedPasteCleanupMode;
    pasteCleanupHistory = storedPasteCleanupHistory;
    sourceLayoutPreset = migrateSourceLayout ? compactPreset.id : storedSourceLayoutPreset;
    sourceLayoutPresetOverrides = storedSourceLayoutPresetOverrides;
    sourceFocusRestoreLayout = migrateSourceLayout ? null : storedSourceFocusRestoreLayout;
    sourceChromeCompact = migrateSourceLayout ? compactPreset.chromeCompact : storedSourceChromeCompact;
    sourceTerminalApp = storedSourceTerminalApp;
    browserUrl = storedBrowserDockUrl;
    browserInputUrl = storedBrowserDockUrl;
    sidePanePosition = migrateSourceLayout ? compactPreset.sidePanePosition : storedSidePanePosition;
    sidePaneWidth = migrateSourceLayout
      ? compactPreset.sidePaneWidth
      : restoreLegacyPaneWidth(storedSidePaneWidth, storedSidePaneExpandedWidth, activityPaneSizingConfig);
    sidePaneExpandedWidth = migrateSourceLayout ? compactPreset.sidePaneWidth : storedSidePaneExpandedWidth;
    editorInsightWidth = migrateSourceLayout ? compactPreset.editorInsightWidth : storedEditorInsightWidth;
    editorInsightCollapsed = migrateSourceLayout ? compactPreset.editorInsightCollapsed : storedEditorInsightCollapsed;
    contextPaneWidth = restoreLegacyPaneWidth(
      storedContextPaneWidth,
      storedContextPaneExpandedWidth,
      contextPaneWidthSizingConfig
    );
    contextPaneExpandedWidth = storedContextPaneExpandedWidth;
    contextPaneHeight = storedContextPaneHeight;
    contextPanelCollapsed = migrateSourceLayout ? compactPreset.contextPanelCollapsed : storedContextPanelCollapsed;
    contextPanelMode = migrateSourceLayout ? compactPreset.contextPanelMode : storedContextPanelMode;
    contextPanelPlacement = migrateSourceLayout ? compactPreset.contextPanelPlacement : storedContextPanelPlacement;
    hiddenContextCardIDs = storedHiddenContextCardIDs;
    activeContextCardID = storedActiveContextCardID;
    const restoredSourceDockLayout =
      !migrateSourceLayout && storedSourceDockLayout
        ? sourceDockLayoutWithStoredPaneSizes(storedSourceDockLayout)
        : null;
    sourceDockLayout = ensureWorkbenchRightPanels(restoredSourceDockLayout ?? sourceDockLayoutFromWorkspace());
    if (!migrateSourceLayout && storedSourceDockLayout) {
      syncSourceDockLayoutToWorkspace(sourceDockLayout);
    }
    persistSelectedProjectID(startupProject.id);
    if (migrateSourceLayout) {
      clearMigratedSourceDockviewLayouts();
      persistSourceLayoutPreset(sourceLayoutPreset);
      persistSourceActivityMode(sourceActivityMode);
      persistSidePanePosition(sidePanePosition);
      persistSidePaneWidth(sidePaneWidth);
      persistEditorInsightWidth(editorInsightWidth);
      persistEditorInsightCollapsed(editorInsightCollapsed);
      persistContextPanelCollapsed(contextPanelCollapsed);
      persistContextPanelMode(contextPanelMode);
      persistContextPanelPlacement(contextPanelPlacement);
      persistSourceChromeCompact(sourceChromeCompact);
      persistSourceFocusRestoreLayout(sourceFocusRestoreLayout);
    }
    persistSourceDockLayout(sourceDockLayout);
    persistSourceLayoutVersion();
    window.setTimeout(measureFileTreeViewport, 0);
    void loadEmbeddedTerminalSessions();
    void loadPlaywrightSessions();
    if (startupWorkspaceSnapshot) {
      fileActionStatus = `Restoring workspace snapshot: ${startupWorkspaceSnapshot.title}`;
      void restoreConversationWorkspaceSnapshot(startupWorkspaceSnapshot).then(() =>
        indexProjectsInBackground(startupProjectOptions)
      );
    } else {
      void loadProjectGitStatus(startupProject);
      void loadGitCommitHistory(startupProject);
      void loadRuntimeContexts(startupProjectOptions);
      void loadProjectWorktrees(startupProject);
      void loadAgentSessions();
      void loadOrchestrationRuns(startupProjectOptions);
      void (async () => {
        await loadGitRepositorySummaries(startupProjectOptions);
        await scanProject(startupProject, storedSelectedSourcePaths[startupProject.id], { limit: expandedSourceScanLimit });
        await indexProjectsInBackground(startupProjectOptions);
      })();
    }

    return () => {
      window.removeEventListener('beforeunload', captureActiveWorkspaceSnapshotBeforeUnload);
      window.removeEventListener('pagehide', captureActiveWorkspaceSnapshotBeforeUnload);
      document.removeEventListener('visibilitychange', handleWorkspaceSnapshotVisibilityChange);
      window.clearInterval(orchestrationRefreshTimer);
      unlistenSourceScanProgress?.();
      unlistenTerminalOutput?.();
      disposeEmbeddedTerminal();
      disposeSourceDockviewWorkbench();
      disposeSourceDockviewActivity();
      disposeSourceDockviewContext();
      disposeSourceDockviewInsights();
      disposeSourceDockviewCenter();
      disposeSourceDockviewBottom();
      disposeSourceFilesDockview();
      disposeSourceConversationDockview();
      disposeSourceContextCardDockview();
    };
  });
</script>

<svelte:head>
  <title>MacCommandBar Webview Preview</title>
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} />

<main
  bind:this={sourceWorkspaceElement}
  class="shell"
  class:side-right={sidePanePosition === 'right'}
  class:activity-hidden={!effectiveActivityPaneVisible()}
  class:activity-rail-only={activityPaneRailOnly()}
  class:activity-force-collapsed={activityPaneViewportCollapsed()}
  class:context-force-collapsed={contextPaneViewportCollapsed()}
  class:layout-pressure={sourceWorkspacePlan.overflowSize > 0}
  style={`--accent: #5ce2cf; --side-pane-width: ${effectiveSidePaneWidth}px; --editor-insight-width: ${editorInsightWidth}px; --context-pane-width: ${effectiveContextPaneWidth}px; --context-pane-height: ${contextPaneHeight}px; --bottom-dock-height: ${bottomDockHeight()}px${appFontSizeOverridden ? `; --app-font-size: ${settings.appearance.appFontSize}px` : ''}`}
>
  <SourceDockviewShell
    shellClass="source-dockview-workbench-shell"
    hostClass="source-dockview-workbench-host"
    errorClass="source-dockview-workbench-error"
    enabled={sourceDockviewWorkbenchEnabled && !useUnifiedWorkbench}
    ready={sourceDockviewWorkbenchReady}
    error={sourceDockviewWorkbenchError}
    hostAction={sourceDockviewWorkbenchHostAction}
  >
  {#if sourceDockviewWorkbenchOwnsPanel('activity') || effectiveActivityPaneVisible()}
  <SourceDockviewShell
    shellClass="source-dockview-activity-shell"
    hostClass="source-dockview-activity-host"
    errorClass="source-dockview-activity-error"
    enabled={sourceDockviewActivityEnabled}
    ready={sourceDockviewActivityReady}
    error={sourceDockviewActivityError}
    hostAction={sourceDockviewActivityHostAction}
  >
  <aside class="activity-shell" aria-label="Workspace browser" use:sourceDockviewPanelAction={'activity'}>
    <nav class="activity-rail" aria-label="Workspace views">
      <button
        class:active={sourceActivityMode === 'files'}
        type="button"
        aria-label="Files"
        title={sourceActivityButtonTitle('files')}
        onclick={() => selectSourceActivityMode('files')}
      >
        <FolderGit2 size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Files</span>
        {#if sourceActivityBadgeVisible('files')}
          <strong>{sourceActivityCountLabel('files')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'clipboard'}
        type="button"
        aria-label="Clipboard"
        title={sourceActivityButtonTitle('clipboard')}
        onclick={() => selectSourceActivityMode('clipboard')}
      >
        <Copy size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Clipboard</span>
        {#if sourceActivityBadgeVisible('clipboard')}
          <strong>{sourceActivityCountLabel('clipboard')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'conversations'}
        type="button"
        aria-label="Conversations"
        title={sourceActivityButtonTitle('conversations')}
        onclick={() => selectSourceActivityMode('conversations')}
      >
        <History size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Conversations</span>
        {#if sourceActivityBadgeVisible('conversations')}
          <strong>{sourceActivityCountLabel('conversations')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'runs'}
        type="button"
        aria-label="Runs"
        title={sourceActivityButtonTitle('runs')}
        onclick={() => selectSourceActivityMode('runs')}
      >
        <Network size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Runs</span>
        {#if sourceActivityBadgeVisible('runs')}
          <strong>{sourceActivityCountLabel('runs')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'sessions'}
        type="button"
        aria-label="Active sessions"
        title={sourceActivityButtonTitle('sessions')}
        onclick={() => selectSourceActivityMode('sessions')}
      >
        <SplitSquareHorizontal size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Active sessions</span>
        {#if sourceActivityBadgeVisible('sessions')}
          <strong>{sourceActivityCountLabel('sessions')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'agents'}
        type="button"
        aria-label="Agents"
        title={sourceActivityButtonTitle('agents')}
        onclick={() => selectSourceActivityMode('agents')}
      >
        <Activity size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Agents</span>
        {#if sourceActivityBadgeVisible('agents')}
          <strong>{sourceActivityCountLabel('agents')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'worktrees'}
        type="button"
        aria-label="Worktrees"
        title={sourceActivityButtonTitle('worktrees')}
        onclick={() => selectSourceActivityMode('worktrees')}
      >
        <FolderSearch size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Worktrees</span>
        {#if sourceActivityBadgeVisible('worktrees')}
          <strong>{sourceActivityCountLabel('worktrees')}</strong>
        {/if}
      </button>
      <button
        class:active={sourceActivityMode === 'git'}
        type="button"
        aria-label="Git and tasks"
        title={sourceActivityButtonTitle('git')}
        onclick={() => selectSourceActivityMode('git')}
      >
        <Braces size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Git and tasks</span>
        {#if sourceActivityBadgeVisible('git')}
          <strong>{sourceActivityCountLabel('git')}</strong>
        {/if}
      </button>
      <button
        class="activity-rail-settings"
        type="button"
        data-testid="activity-rail-settings"
        aria-label="Settings"
        title="Settings"
        onclick={() => (settingsOpen = !settingsOpen)}
      >
        <Settings size={19} strokeWidth={1.8} />
        <span class="activity-rail-label">Settings</span>
      </button>
      <button
        class="activity-rail-toggle"
        type="button"
        data-testid="activity-rail-toggle"
        aria-label={activityRailToggleLabel()}
        title={activityRailToggleLabel()}
        onclick={toggleActivityPaneRail}
      >
        {#if activityRailToggleDirection() === 'left'}
          <ChevronLeft size={18} strokeWidth={1.9} />
        {:else}
          <ChevronRight size={18} strokeWidth={1.9} />
        {/if}
        <span class="activity-rail-label">{activityRailToggleLabel()}</span>
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
        <button class="icon-button" type="button" aria-label="Choose project folder" title="Choose project folder" disabled={choosingProjectRoot || projectRootValidating} onclick={chooseProjectRoot}>
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
          <span>{scanning ? 'Stop' : `Scan ${expandedSourceScanLimitShortLabel}`}</span>
        </button>
      </div>

      <div class="project-path-row">
        <span class="project-path" title={selectedProject.path}>{selectedProject.path}</span>
        {#if selectedProjectIsCustom}
          <button class="icon-button danger" type="button" aria-label="Remove project root" title="Remove project root" onclick={removeSelectedProject}>
            <Trash2 size={14} strokeWidth={1.9} />
          </button>
        {/if}
      </div>
      <div class="project-setup-row" title={projectSetupNoticeTitle()} aria-live={scanning ? 'polite' : 'off'}>
        {#each projectSetupNoticeText().split(' · ') as setupSegment, setupIndex (setupIndex)}
          <Chip size="xs" tone="muted">{setupSegment}</Chip>
        {/each}
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
            <button class="form-button" type="button" disabled={projectRootValidating} onclick={cancelAddingProject}>
              <X size={14} strokeWidth={2} />
              <span>Cancel</span>
            </button>
            <button class="form-button primary" type="submit" disabled={projectRootValidating}>
              <Save size={14} strokeWidth={2} />
              <span>{projectRootValidating ? 'Checking' : 'Save'}</span>
            </button>
          </div>
        </form>
      {/if}
    </div>

    {#if sourceActivityMode === 'files'}
      <div class="source-browser-stack">
      <SourceDockviewShell
        shellClass="source-dockview-files-shell"
        hostClass="source-dockview-files-host"
        errorClass="source-dockview-files-error"
        enabled={true}
        ready={sourceFilesDockviewReady}
        error={sourceFilesDockviewError}
        hostAction={sourceFilesDockviewHostAction}
      >
      <div
        id="source-files-search-pane"
        class="source-files-pane source-files-search-pane"
        role="tabpanel"
        aria-label="Search source files"
        use:sourceFilesDockviewPanelAction={'search'}
      >
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
      </div>

      <div
        id="source-files-recent-pane"
        class="source-files-pane source-files-recent-pane"
        role="tabpanel"
        aria-label="Recent source files"
        use:sourceFilesDockviewPanelAction={'recent'}
      >
      {#if projectRecentRecords.length > 0}
        <div class="recent-panel" aria-label="Recent source files">
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
      {:else}
        <div class="empty-tree">
          <History size={17} strokeWidth={1.8} />
          <span>No recent source files</span>
        </div>
      {/if}
      </div>

      <div
        id="source-files-tree-pane"
        class="source-list-panel source-files-pane"
        role="tabpanel"
        aria-label="Files in selected project"
        use:sourceFilesDockviewPanelAction={'files'}
      >
        <div class="tree-panel-header">
          <div class="source-section-toolbar">
            <div class="source-section-header tree-heading source-section-title" title={sourceSidebarTreeStatusLabel}>
              <FolderGit2 size={15} strokeWidth={1.8} />
              <span>{selectedProject.name}</span>
            </div>
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
          <div class="scan-summary-row">
            <div
              class="scan-summary"
              data-tone={selectedProjectScanEvidence.tone}
              title={sourceSidebarCompactStatusTitle}
            >
              <span class="index-summary" data-tone={selectedProjectScanEvidence.tone}>
                {sourceSidebarIndexStatusLabel}
              </span>
              <span class="scan-evidence" data-tone={selectedProjectScanEvidence.tone}>
                {sourceSidebarScanMetaLabel}
              </span>
            </div>
          </div>
          {#if sourceSidebarScanTelemetryExpanded}
            <div class="scan-stats" title={sourceScanStatsLabel}>{sourceSidebarScanTelemetryLabel}</div>
          {/if}
          {#if sourceRuntimeNoticeExpanded}
            <div class="scan-runtime-note" title={sourceRuntimeNotice}>
              <span>{sourceRuntimeNotice}</span>
              <button
                type="button"
                aria-label="Copy Tauri run command"
                title="Copy Tauri run command"
                onclick={copyTauriRunCommand}
              >
                Tauri
              </button>
            </div>
          {/if}
          {#if selectedProjectGitRootSuggestion}
            <div class="scan-root-correction" title={`Detected Git root: ${selectedProjectGitRootSuggestion}`}>
              <span>
                <strong>Nested root</strong>
                Use {formatSourceContextRootLabel(selectedProjectGitRootSuggestion)}
              </span>
              <button
                type="button"
                aria-label="Use detected Git root"
                title={selectedProjectGitRootSuggestion}
                disabled={projectRootValidating || scanning}
                onclick={useValidatedGitRootForSelectedProject}
              >
                Use root
              </button>
            </div>
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
          {#if sourceScanRecovery.visible}
            <div class="scan-recovery-panel" title={sourceScanRecovery.detail}>
              <span>
                <strong>{sourceScanRecovery.title}</strong>
                {sourceScanRecovery.detail}
              </span>
              <div class="scan-recovery-actions">
                <button
                  type="button"
                  aria-label="Reset source index"
                  title="Reset source index"
                  onclick={() => resetProjectScanCache(selectedProject)}
                >
                  <RotateCcw size={12} strokeWidth={2} />
                </button>
                <button
                  type="button"
                  aria-label="Choose project root"
                  title="Choose project root"
                  disabled={choosingProjectRoot || projectRootValidating}
                  onclick={chooseProjectRoot}
                >
                  <FolderSearch size={12} strokeWidth={2} />
                </button>
                <button
                  type="button"
                  aria-label="Copy source scan diagnostic"
                  title="Copy source scan diagnostic"
                  onclick={copySourceScanDiagnosticBrief}
                >
                  <Copy size={12} strokeWidth={2} />
                </button>
              </div>
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
      </SourceDockviewShell>
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
            <div class="paste-cleanup-top">
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
                  onclick={clearPasteCleanupInput}
                >
                  <X size={13} strokeWidth={1.9} />
                  <span>Clear</span>
                </button>
              </div>
              {#if visiblePasteCleanupHistory.length > 0}
                <div class="paste-cleanup-history" aria-label="Paste cleanup history">
                  {#each visiblePasteCleanupHistory as item (item.id)}
                    <div class="paste-history-chip" class:reply={item.kind === 'reply'}>
                      <button
                        class="paste-history-restore"
                        type="button"
                        aria-label={`Restore paste cleanup ${pasteCleanupHistoryKindLabel(item.kind)}`}
                        title={pasteCleanupHistoryItemTitle(item)}
                        onclick={() => restorePasteCleanupHistoryItem(item)}
                      >
                        <History size={11} strokeWidth={2} />
                        <span>{pasteCleanupHistoryKindLabel(item.kind)}</span>
                        <strong>{item.summary}</strong>
                      </button>
                      <button
                        class="paste-history-copy"
                        type="button"
                        aria-label={`Copy paste cleanup ${pasteCleanupHistoryKindLabel(item.kind)}`}
                        title="Copy history item"
                        onclick={() => copyPasteCleanupHistoryItem(item)}
                      >
                        <Copy size={11} strokeWidth={2} />
                      </button>
                    </div>
                  {/each}
                  <button
                    class="paste-history-clear"
                    type="button"
                    aria-label="Clear paste cleanup history"
                    title="Clear paste cleanup history"
                    onclick={clearPasteCleanupHistory}
                  >
                    <X size={11} strokeWidth={2} />
                  </button>
                </div>
              {/if}
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
              <label>
                <span>Reply</span>
                <textarea
                  class="paste-cleanup-textarea"
                  bind:value={pasteCleanupReplyDraft}
                  aria-label="Paste cleanup reply draft"
                  spellcheck="true"
                  placeholder="Draft the reply to copy back"
                ></textarea>
              </label>
            </div>
            <div class="paste-cleanup-footer">
              <span>Cleaned {pasteCleanupStats} · Reply {pasteCleanupReplyStats}</span>
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
              <button
                class="file-action-button"
                type="button"
                aria-label="Copy paste reply draft"
                title="Copy paste reply draft"
                disabled={pasteCleanupReplyOutput.trim().length === 0 || fileActionBusy === 'paste-reply-copy'}
                onclick={copyPasteCleanupReplyDraft}
              >
                <Copy size={13} strokeWidth={1.9} />
                <span>{fileActionBusy === 'paste-reply-copy' ? 'Copying' : 'Copy Reply'}</span>
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
          <div class="run-ingest-strip" aria-label="Orchestration ingest">
            <input
              bind:value={orchestrationEventFilePath}
              type="text"
              autocomplete="off"
              spellcheck="false"
              aria-label="Orchestration event file path"
              title="JSON or JSONL orchestration event file"
            />
            <button
              type="button"
              aria-label="Choose orchestration event file"
              title="Choose orchestration event file"
              disabled={orchestrationEventFileChoosing}
              onclick={chooseOrchestrationEventFile}
            >
              <FolderOpen size={12} strokeWidth={2} />
            </button>
            <button
              type="button"
              aria-label="Copy orchestration JSON import command"
              title="Copy orchestration JSON import command"
              onclick={copyOrchestrationJsonFileImportCommand}
            >
              <Copy size={12} strokeWidth={2} />
            </button>
            <button
              type="button"
              aria-label="Record orchestration heartbeat"
              title="Record orchestration heartbeat"
              disabled={orchestrationEventRecording}
              onclick={() =>
                recordNativeOrchestrationEvent('run-updated', 'Run heartbeat', {
                  message: `Heartbeat from ${selectedProject.name}`
                })}
            >
              <Activity size={12} strokeWidth={2} />
            </button>
            {#if orchestrationEventImportStatus}
              <small>{orchestrationEventImportStatus}</small>
            {/if}
          </div>
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
                {@const runLiveDigest = orchestrationLiveDigestItems(run, 5)}
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
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openActivityRowActionMenu('run', run.id);
                  }}
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
                  {#if runLiveDigest.length > 0}
                    <div class="run-live-digest" aria-label="Live run ingest signals">
                      {#each runLiveDigest as item (item.id)}
                        <span
                          class={`run-live-digest-chip ${item.tone}`}
                          title={item.detail ? `${item.title}\n${item.detail}` : item.title}
                        >
                          <em>{item.label}</em>
                          <strong>{item.title}</strong>
                        </span>
                      {/each}
                    </div>
                  {/if}
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
                            <small>{item.kind} · {orchestrationTimelineDetail(item)}</small>
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
                          <button
                            class="run-chip artifact"
                            type="button"
                            title={artifact.path ?? artifact.title}
                            aria-label={`Open run artifact: ${artifact.title}`}
                            onclick={() => openOrchestrationArtifact(artifact)}
                          >
                            {artifact.label}
                          </button>
                        {/if}
                      {/each}
                      {#each runLinks as link (link.id)}
                        <a class="run-chip link" href={link.href ?? ''} target="_blank" rel="noreferrer">
                          {link.label}
                        </a>
                      {/each}
                    </div>
                  {/if}
                  <div class="activity-row-actions run-activity-actions row-action-menu-anchor" aria-label="Run actions">
                    <button
                      type="button"
                      aria-label="Run actions"
                      aria-haspopup="menu"
                      aria-expanded={activityRowActionMenuOpen('run', run.id)}
                      title="Run actions"
                      onclick={() => toggleActivityRowActionMenu('run', run.id)}
                    >
                      <MoreHorizontal size={13} strokeWidth={2} />
                    </button>
                    {#if activityRowActionMenuOpen('run', run.id)}
                      <div class="row-action-menu" role="menu" aria-label="Run actions">
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy run summary"
                          title="Copy run summary"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            copyOrchestrationRunSummary(run);
                          }}
                        >
                          <Activity size={12} strokeWidth={2} />
                          <span>Copy summary</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy run handoff"
                          title="Copy run handoff"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            copyOrchestrationRunHandoff(run);
                          }}
                        >
                          <FileCode2 size={12} strokeWidth={2} />
                          <span>Copy handoff</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy current run activity"
                          title="Copy current run activity"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            copyOrchestrationCurrentActivity(run);
                          }}
                        >
                          <History size={12} strokeWidth={2} />
                          <span>Copy activity</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy run id"
                          title="Copy run id"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            copyActivityCommand(run.id, 'Run id copied');
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy run id</span>
                        </button>
                        {#if run.taskID}
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Copy run task reference"
                            title="Copy run task reference"
                            onclick={() => {
                              closeActivityRowActionMenu();
                              copyOrchestrationTaskReference(run);
                            }}
                          >
                            <ExternalLink size={12} strokeWidth={2} />
                            <span>Copy task ref</span>
                          </button>
                        {/if}
                        {#if run.projectPath}
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Open run project path"
                            title="Open project path"
                            onclick={() => {
                              closeActivityRowActionMenu();
                              openActivityPath(run.projectPath);
                            }}
                          >
                            <ExternalLink size={12} strokeWidth={2} />
                            <span>Open path</span>
                          </button>
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Open run project in terminal"
                            title="Open project in terminal"
                            onclick={() => {
                              closeActivityRowActionMenu();
                              openActivityTerminalPath(run.projectPath);
                            }}
                          >
                            <Terminal size={12} strokeWidth={2} />
                            <span>Open terminal</span>
                          </button>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'conversations'}
          <div class="activity-panel-list conversation-activity-list" aria-label="Conversation list">
            <SourceDockviewShell
              shellClass="source-dockview-conversation-shell"
              hostClass="source-dockview-conversation-host"
              errorClass="source-dockview-conversation-error"
              enabled={true}
              ready={sourceConversationDockviewReady}
              error={sourceConversationDockviewError}
              hostAction={sourceConversationDockviewHostAction}
            >
            <section
              id="conversation-saved-pane"
              class="workspace-snapshot-section conversation-dockview-pane"
              role="tabpanel"
              aria-label="Saved workspace snapshots"
              use:sourceConversationDockviewPanelAction={'saved'}
            >
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
                    {@const readiness = workspaceSnapshotRestoreReadiness(snapshot)}
                    <div
                      class="workspace-snapshot-row"
                      class:active={activeWorkspaceSessionKey === snapshot.id}
                      title={workspaceSnapshotRestorePlan(snapshot)}
                      oncontextmenu={(event) => {
                        event.preventDefault();
                        openActivityRowActionMenu('workspace-snapshot', snapshot.id);
                      }}
                    >
                      <button
                        type="button"
                        aria-label={`Restore ${snapshot.title}`}
                        onclick={() => restoreConversationWorkspaceSnapshot(snapshot)}
                      >
                        <span class="agent-provider-badge">{snapshot.provider}</span>
                        <div class="activity-row-main">
                          <strong>{snapshot.title}</strong>
                          <small>{workspaceSnapshotEnvironmentLabel(snapshot)}</small>
                          <small class="workspace-snapshot-file-state">{workspaceSnapshotFileStateLabel(snapshot)}</small>
                          <span class={`workspace-snapshot-readiness ${readiness.tone}`} title={readiness.detail}>
                            {readiness.label} · {readiness.detail}
                          </span>
                        </div>
                      </button>
                      <div
                        class="activity-row-actions workspace-snapshot-actions row-action-menu-anchor"
                        aria-label="Workspace snapshot actions"
                      >
                        <button
                          type="button"
                          aria-label="Workspace snapshot actions"
                          aria-haspopup="menu"
                          aria-expanded={activityRowActionMenuOpen('workspace-snapshot', snapshot.id)}
                          title="Workspace snapshot actions"
                          onclick={() => toggleActivityRowActionMenu('workspace-snapshot', snapshot.id)}
                        >
                          <MoreHorizontal size={13} strokeWidth={2} />
                        </button>
                        {#if activityRowActionMenuOpen('workspace-snapshot', snapshot.id)}
                          <div class="row-action-menu" role="menu" aria-label="Workspace snapshot actions">
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Resume workspace snapshot in embedded terminal"
                              title={
                                readiness.kind === 'missing-worktree'
                                  ? 'Copy missing worktree repair plan'
                                  : snapshot.resumeCommand
                                  ? 'Resume workspace in embedded terminal'
                                  : 'Open workspace shell in embedded terminal'
                              }
                              onclick={() => {
                                closeActivityRowActionMenu();
                                openConversationWorkspaceSnapshotEmbeddedTerminal(snapshot);
                              }}
                            >
                              <Terminal size={12} strokeWidth={2} />
                              <span>Resume embedded</span>
                            </button>
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Copy workspace restore plan"
                              title="Copy restore plan"
                              onclick={() => {
                                closeActivityRowActionMenu();
                                copyWorkspaceSnapshotRestorePlan(snapshot);
                              }}
                            >
                              <FileCode2 size={12} strokeWidth={2} />
                              <span>Copy restore plan</span>
                            </button>
                            {#if readiness.kind === 'missing-worktree'}
                              <button
                                type="button"
                                role="menuitem"
                                aria-label="Copy workspace repair plan"
                                title={readiness.repairLabel ?? 'Copy repair plan'}
                                onclick={() => {
                                  closeActivityRowActionMenu();
                                  copyWorkspaceSnapshotRepairPlan(snapshot);
                                }}
                              >
                                <FolderSearch size={12} strokeWidth={2} />
                                <span>Copy repair plan</span>
                              </button>
                            {/if}
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Copy workspace resume command"
                              title="Copy resume command"
                              disabled={!snapshot.resumeCommand}
                              onclick={() => {
                                closeActivityRowActionMenu();
                                copyActivityCommand(snapshot.resumeCommand ?? '', 'Resume command copied');
                              }}
                            >
                              <Copy size={12} strokeWidth={2} />
                              <span>Copy resume</span>
                            </button>
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Open workspace path"
                              title="Open workspace path"
                              onclick={() => {
                                closeActivityRowActionMenu();
                                openWorkspaceSnapshotPath(snapshot);
                              }}
                            >
                              <ExternalLink size={12} strokeWidth={2} />
                              <span>Open path</span>
                            </button>
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Open workspace in terminal"
                              title="Open workspace in terminal"
                              onclick={() => {
                                closeActivityRowActionMenu();
                                openWorkspaceSnapshotPathTerminal(snapshot);
                              }}
                            >
                              <Terminal size={12} strokeWidth={2} />
                              <span>Open terminal</span>
                            </button>
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Delete workspace snapshot"
                              title="Delete workspace snapshot"
                              onclick={() => {
                                closeActivityRowActionMenu();
                                deleteWorkspaceSnapshot(snapshot);
                              }}
                            >
                              <Trash2 size={12} strokeWidth={2} />
                              <span>Delete snapshot</span>
                            </button>
                          </div>
                        {/if}
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="activity-empty compact">No saved workspaces</div>
              {/if}
            </section>

            <div
              id="conversation-active-pane"
              class="conversation-active-pane conversation-dockview-pane"
              role="tabpanel"
              aria-label="Active conversations"
              use:sourceConversationDockviewPanelAction={'active'}
            >
              <ConversationList
                sessions={filteredConversationAgentSessions}
                activeKey={activeWorkspaceSessionKey}
                liveKeys={conversationLiveKeys}
                keyFor={workspaceSnapshotIDForAgentSession}
                filter={true}
                onOpen={switchToConversationWorkspace}
                onSave={captureAgentSessionWorkspaceSnapshot}
                onRestore={restoreAgentSessionWorkspaceSnapshot}
                onRepair={copyAgentSessionWorkspaceRepairPlan}
                onCopyResume={(session) =>
                  copyActivityCommand(agentSessionResumeCommand(session), 'Resume command copied')}
              />
            </div>
            </SourceDockviewShell>
          </div>
        {:else if sourceActivityMode === 'sessions'}
          <div class="activity-panel-list" aria-label="Active session list">
            {#if filteredProjectRuntimeContexts.length === 0}
              <div class="activity-empty">No active sessions</div>
            {:else}
              {#each filteredProjectRuntimeContexts as context (`activity:${context.pid}:${context.port}:${context.cwd}`)}
                <div
                  class="activity-runtime-row"
                  title={context.cwd}
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openActivityRowActionMenu('runtime', `${context.pid}:${context.port}:${context.cwd}`);
                  }}
                >
                  <span class="runtime-port">:{context.port}</span>
                  <div class="activity-row-main">
                    <strong>{context.command}</strong>
                    <small>{context.rootLabel} · {context.cwd}</small>
                  </div>
                  <div class="activity-row-actions runtime-activity-actions row-action-menu-anchor" aria-label="Active session actions">
                    <button
                      type="button"
                      aria-label="Active session actions"
                      aria-haspopup="menu"
                      aria-expanded={activityRowActionMenuOpen('runtime', `${context.pid}:${context.port}:${context.cwd}`)}
                      title="Active session actions"
                      onclick={() => toggleActivityRowActionMenu('runtime', `${context.pid}:${context.port}:${context.cwd}`)}
                    >
                      <MoreHorizontal size={13} strokeWidth={2} />
                    </button>
                    {#if activityRowActionMenuOpen('runtime', `${context.pid}:${context.port}:${context.cwd}`)}
                      <div class="row-action-menu" role="menu" aria-label="Active session actions">
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy active session command"
                          title="Copy session command"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            copyActivityCommand(`${context.command} ${context.cwd}`, 'Session command copied');
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy command</span>
                        </button>
                        <a
                          class="row-action-menu-link"
                          role="menuitem"
                          href={runtimeContextUrl(context)}
                          target="_blank"
                          rel="noreferrer"
                          aria-label="Open active session URL"
                          title={runtimeContextUrl(context)}
                          onclick={() => closeActivityRowActionMenu()}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                          <span>Open URL</span>
                        </a>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open active session path"
                          title="Open session path"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            openActivityPath(context.cwd);
                          }}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                          <span>Open path</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open active session in terminal"
                          title="Open session in terminal"
                          onclick={() => {
                            closeActivityRowActionMenu();
                            openActivityTerminalPath(context.cwd);
                          }}
                        >
                          <Terminal size={12} strokeWidth={2} />
                          <span>Open terminal</span>
                        </button>
                      </div>
                    {/if}
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
                {@const sessionSnapshot = workspaceSnapshotForAgentSession(session)}
                {@const sessionReadiness = sessionSnapshot ? workspaceSnapshotRestoreReadiness(sessionSnapshot) : null}
                {@const sessionFocusLane = agentSessionFocusLane(session, sessionReadiness)}
                <div
                  class="activity-session-row agent-activity-row"
                  title={agentSessionResumePlan(session)}
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openAgentRowActionMenu(session, 'agent');
                  }}
                >
                  <span class="agent-provider-badge" title={agentSessionProviderLabel(session)}>
                    {agentSessionProviderBadgeLabel(session)}
                  </span>
                  <div class="activity-row-main agent-activity-main">
                    <strong>{session.title}</strong>
                    <small class="agent-activity-meta">{agentSessionResumeMetaLabel(session, sessionSnapshot)}</small>
                    <small class="agent-activity-state">{agentSessionWorkspaceStateLabel(session, sessionSnapshot)}</small>
                    {#if sessionReadiness}
                      <span class={`workspace-snapshot-readiness ${sessionReadiness.tone}`} title={sessionReadiness.detail}>
                        {sessionReadiness.label}
                      </span>
                    {:else}
                      <span class="workspace-snapshot-readiness neutral" title="Save a workspace snapshot to restore this session context later.">
                        No saved workspace
                      </span>
                    {/if}
                    <span
                      class={`agent-session-focus-lane ${sessionFocusLane.tone}`}
                      aria-label="Recommended session focus action"
                      title={`${sessionFocusLane.title} · ${sessionFocusLane.detail}`}
                    >
                      <em>{sessionFocusLane.label}</em>
                      <strong>{sessionFocusLane.detail}</strong>
                    </span>
                  </div>
                  <div class="activity-row-actions agent-activity-actions row-action-menu-anchor" aria-label="Agent actions">
                    <button
                      type="button"
                      aria-label="Agent actions"
                      aria-haspopup="menu"
                      aria-expanded={agentRowActionMenuOpen(session, 'agent')}
                      title="Agent actions"
                      onclick={() => toggleAgentRowActionMenu(session, 'agent')}
                    >
                      <MoreHorizontal size={13} strokeWidth={2} />
                    </button>
                    {#if agentRowActionMenuOpen(session, 'agent')}
                      <div class="row-action-menu" role="menu" aria-label="Agent actions">
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy agent focus plan"
                          title={agentSessionWorkspaceReadinessLabel(session)}
                          onclick={() => {
                            closeAgentRowActionMenu();
                            copyAgentSessionResumePlan(session);
                          }}
                        >
                          <FileCode2 size={12} strokeWidth={2} />
                          <span>Copy focus plan</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open agent workspace"
                          onclick={() => {
                            closeAgentRowActionMenu();
                            openAgentSessionWorkspace(session);
                          }}
                        >
                          <RotateCcw size={12} strokeWidth={2} />
                          <span>{sessionSnapshot ? 'Restore workspace' : 'Open workspace'}</span>
                        </button>
                        {#if sessionSnapshot && sessionReadiness?.kind === 'missing-worktree'}
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Copy agent workspace repair plan"
                            onclick={() => {
                              closeAgentRowActionMenu();
                              copyAgentSessionWorkspaceRepairPlan(session);
                            }}
                          >
                            <FolderSearch size={12} strokeWidth={2} />
                            <span>Copy repair plan</span>
                          </button>
                        {/if}
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy agent resume command"
                          onclick={() => {
                            closeAgentRowActionMenu();
                            copyActivityCommand(agentSessionResumeCommand(session), 'Resume command copied');
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy resume command</span>
                        </button>
                        {#if session.projectPath}
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Reveal agent project path"
                            onclick={() => {
                              closeAgentRowActionMenu();
                              revealActivityPath(session.projectPath ?? '');
                            }}
                          >
                            <FolderSearch size={12} strokeWidth={2} />
                            <span>Reveal project path</span>
                          </button>
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Resume agent in terminal"
                            onclick={() => {
                              closeAgentRowActionMenu();
                              openAgentSessionTerminal(session);
                            }}
                          >
                            <Terminal size={12} strokeWidth={2} />
                            <span>Resume in terminal</span>
                          </button>
                        {/if}
                      </div>
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
              <div class="activity-worktree-runbook" aria-label="Worktree cleanup runbook controls">
                <div
                  class="worktree-runbook-strip"
                  aria-label="Worktree cleanup runbook summary"
                  title={projectWorktreeCleanupRunbook.headline}
                >
                  <span class="worktree-runbook-chip safe">
                    <strong>{projectWorktreeCleanupRunbook.counts.safeRemovable}</strong>
                    <span>safe</span>
                  </span>
                  <span class="worktree-runbook-chip backup">
                    <strong>{projectWorktreeCleanupRunbook.counts.backupRequired}</strong>
                    <span>backup</span>
                  </span>
                  <span class="worktree-runbook-chip blocked">
                    <strong>{projectWorktreeCleanupRunbook.counts.blocked}</strong>
                    <span>blocked</span>
                  </span>
                  <span class="worktree-runbook-chip saved">
                    <strong>{projectWorktreeCleanupRunbook.counts.savedWorkspaceReview}</strong>
                    <span>saved</span>
                  </span>
                </div>
                <button
                  type="button"
                  aria-label="Copy worktree cleanup runbook"
                  title="Copy cleanup runbook"
                  onclick={copyProjectWorktreeCleanupRunbook}
                >
                  <Braces size={12} strokeWidth={2} />
                </button>
              </div>
              {#each filteredProjectWorktrees as worktree (`activity:${worktree.path}`)}
                {@const safety = projectWorktreeSafety(worktree)}
                {@const cleanupPlan = projectWorktreeCleanupPlan(worktree)}
                {@const decisionLane = worktreeDecisionLane(safety)}
                {@const primaryAction = projectWorktreePrimaryAction(worktree)}
                {@const eligibilityKind = projectWorktreeEligibilityKind(worktree)}
                {@const latestSnapshot = latestWorktreeWorkspaceSnapshot(worktree)}
                {@const ownerChips = worktreeOwnerChips(worktree, safety)}
                <div
                  class="activity-worktree-row"
                  class:blocked={eligibilityKind === 'blocked'}
                  class:protected={eligibilityKind === 'protected'}
                  class:ready={eligibilityKind === 'ready'}
                  title={formatWorktreeCleanupPlanReport(worktree, cleanupPlan)}
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openWorktreeRowActionMenu(worktree, 'activity');
                  }}
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
                      <span class={`worktree-decision-lane ${decisionLane.tone}`} title={decisionLane.detail}>
                        {decisionLane.label}
                      </span>
                      <span>{safety.reason}</span>
                      {#if safety.activeSessionCount > 0}
                        <span>
                          {safety.activeSessionCount}
                          {safety.activeSessionCount === 1 ? 'session' : 'sessions'}
                        </span>
                      {/if}
                      <span>{projectWorktreeActivityLabel(worktree)}</span>
                    </small>
                    <div class="worktree-owner-strip" aria-label="Worktree session ownership">
                      {#each ownerChips as chip (chip.id)}
                        <span class={`worktree-owner-chip ${chip.tone}`} title={chip.title}>{chip.label}</span>
                      {/each}
                    </div>
                    <small class="worktree-plan-line" title={cleanupPlan.explanation}>
                      <span class={`worktree-plan-lane ${cleanupPlan.lane}`}>
                        {worktreeCleanupPlanLabel(cleanupPlan)}
                      </span>
                      <span>{cleanupPlan.explanation}</span>
                    </small>
                    <small class="worktree-recommendation">{safety.recommendation}</small>
                    {#if latestSnapshot}
                      <button
                        class="worktree-snapshot-chip"
                        type="button"
                        aria-label="Restore latest saved workspace for worktree"
                        title={worktreeWorkspaceSnapshotTitle(worktree)}
                        onclick={() => restoreConversationWorkspaceSnapshot(latestSnapshot)}
                      >
                        <History size={11} strokeWidth={2} />
                        <span>{worktreeWorkspaceSnapshotLabel(worktree)}</span>
                      </button>
                    {/if}
                  </div>
                  <div class="activity-row-actions worktree-activity-actions row-action-menu-anchor" aria-label="Worktree actions">
                    <button
                      type="button"
                      aria-label="Worktree actions"
                      aria-haspopup="menu"
                      aria-expanded={worktreeRowActionMenuOpen(worktree, 'activity')}
                      title="Worktree actions"
                      onclick={() => toggleWorktreeRowActionMenu(worktree, 'activity')}
                    >
                      <MoreHorizontal size={13} strokeWidth={2} />
                    </button>
                    {#if worktreeRowActionMenuOpen(worktree, 'activity')}
                      <div class="row-action-menu" role="menu" aria-label="Worktree actions">
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy worktree cleanup plan"
                          title="Copy cleanup plan"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            copyWorktreeCleanupPlan(worktree);
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy cleanup plan</span>
                        </button>
                        <button
                          class={`worktree-primary-action ${primaryAction.kind}`}
                          type="button"
                          role="menuitem"
                          aria-label={`${primaryAction.label} worktree: ${worktree.branch}`}
                          title={primaryAction.title}
                          disabled={fileActionBusy === `worktree-primary:${worktree.path}`}
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            runWorktreePrimaryAction(worktree);
                          }}
                        >
                          {#if primaryAction.kind === 'cleanup'}
                            <Trash2 size={12} strokeWidth={2} />
                          {:else if primaryAction.kind === 'backup'}
                            <Save size={12} strokeWidth={2} />
                          {:else}
                            <History size={12} strokeWidth={2} />
                          {/if}
                          <span>{primaryAction.label}</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open worktree in source browser"
                          title="Open worktree in source browser"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            openWorktreeInSourceBrowser(worktree);
                          }}
                        >
                          <FolderOpen size={12} strokeWidth={2} />
                          <span>Open in source browser</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy worktree path"
                          title="Copy worktree path"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            copyActivityCommand(worktree.path, 'Worktree path copied');
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy path</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open worktree path"
                          title="Open worktree path"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            openActivityPath(worktree.path);
                          }}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                          <span>Open path</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open worktree in terminal"
                          title="Open worktree in terminal"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            openActivityTerminalPath(worktree.path);
                          }}
                        >
                          <Terminal size={12} strokeWidth={2} />
                          <span>Open terminal</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open worktree in embedded terminal"
                          title="Open worktree in embedded terminal"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            openPathEmbeddedTerminal(worktree.path);
                          }}
                        >
                          <PanelBottom size={12} strokeWidth={2} />
                          <span>Open embedded terminal</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Reveal worktree path"
                          title="Reveal worktree path"
                          onclick={() => {
                            closeWorktreeRowActionMenu();
                            revealActivityPath(worktree.path);
                          }}
                        >
                          <FolderSearch size={12} strokeWidth={2} />
                          <span>Reveal path</span>
                        </button>
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else if sourceActivityMode === 'git'}
          <div class="activity-panel-list activity-git-panel" data-testid="git-activity-panel" aria-label="Source Control">
            <section class="activity-git-source-control" aria-label="Source Control changes">
              <div class="activity-git-heading">
                <div>
                  <strong>Source Control</strong>
                  <small>{selectedProject.name} · {formatSourceContextRootLabel(selectedProject.path)}</small>
                </div>
                <button
                  class="activity-git-refresh"
                  type="button"
                  aria-label="Refresh source control status"
                  title="Refresh source control status"
                  disabled={projectGitLoading}
                  onclick={() => loadProjectGitStatus(selectedProject)}
                >
                  <RefreshCw size={13} strokeWidth={2} />
                </button>
              </div>
              <div
                class="git-branch-health-strip activity-git-health-strip"
                data-testid="git-branch-summary"
                aria-label="Source control branch health"
                title={selectedProjectGitBranchHealth.detail}
              >
                {#each selectedProjectGitBranchHealth.chips as chip (`activity:${chip.label}:${chip.value}`)}
                  <span class={`git-branch-health-chip ${chip.tone}`}>
                    <strong>{chip.label}</strong>
                    <span>{chip.value}</span>
                  </span>
                {/each}
              </div>
              <details class="git-command-drawer activity-git-command-drawer" data-testid="git-command-drawer">
                <summary>
                  <span>Commands</span>
                  <small>fetch, pull, push, commit</small>
                </summary>
                <div class="activity-git-command-strip" aria-label="Source control commands">
                  <div class="activity-git-remote-row">
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
                  <div class="git-commit-row activity-git-commit-row">
                    <textarea
                      class="git-commit-input"
                      bind:value={gitCommitMessage}
                      aria-label="Git commit message"
                      placeholder="Message (Cmd+Enter to commit staged changes)"
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
                </div>
              </details>
              <div class="activity-git-status-heading">
                <strong>Changes</strong>
                <span>{selectedProjectGitFileGroupSummary}</span>
              </div>
              <div
                class="git-status-list activity-source-control-list"
                data-testid="git-changed-files"
                aria-label="Source Control changed files"
              >
                {#if projectGitLoading}
                  <div class="activity-empty compact">Loading changed files</div>
                {:else if projectGitError}
                  <div class="activity-empty compact">{projectGitError}</div>
                {:else if selectedProjectGitChangedFiles.length === 0}
                  <div class="activity-empty compact">No changed files</div>
                {:else}
                  {#each selectedProjectGitFileGroups as group (group.id)}
                    {#if group.files.length > 0}
                      <details
                        class="git-status-group"
                        data-testid={`git-status-group-${group.id}`}
                        aria-label={`${group.label} Git files`}
                        open
                      >
                        <summary class="git-status-group-heading">
                          <strong>{group.label}</strong>
                          <span>{group.files.length}</span>
                          <button
                            type="button"
                            disabled={gitActionBusy !== ''}
                            onclick={(event) => {
                              event.preventDefault();
                              event.stopPropagation();
                              void runGitStatusGroupAction(group);
                            }}
                          >
                            {gitStatusGroupActionLabel(group)}
                          </button>
                        </summary>
                        {#each group.files as fileStatus (`activity:${group.id}:${fileStatus.relativePath}`)}
                          <button
                            class="git-status-row"
                            class:selected={selectedRecord?.relativePath === fileStatus.relativePath}
                            data-git-path={fileStatus.relativePath}
                            data-git-status={fileStatus.status}
                            type="button"
                            title={gitStatusFileTitle(fileStatus)}
                            onclick={() => selectGitStatusFile(fileStatus)}
                          >
                            <strong>{fileStatus.badge}</strong>
                            <span>{fileStatus.relativePath}</span>
                            <small>{gitStatusFileSummary(fileStatus)}</small>
                          </button>
                        {/each}
                      </details>
                    {/if}
                  {/each}
                {/if}
              </div>
              {#if gitActionError || gitActionStatus}
                <div class:error={Boolean(gitActionError)} class="git-action-message">
                  {gitActionError || gitActionStatus}
                </div>
              {/if}
            </section>

            <details class="activity-git-secondary-section" data-testid="git-repositories-section">
              <summary>
                <span>Repositories</span>
                <small>{filteredGitRepositoryRows.length}</small>
              </summary>
            {#if filteredGitRepositoryRows.length === 0}
              <div class="activity-empty">No repositories</div>
            {:else}
              {#each filteredGitRepositoryRows as row (`activity:${row.id}`)}
                <div
                  class="activity-repo-row"
                  class:dirty={row.dirty.isDirty || row.error}
                  title={repoDashboardTitle(row)}
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openGitRowActionMenu(row.id, 'repository');
                  }}
                >
                  <div class="activity-row-main">
                    <strong>{row.projectName}</strong>
                    <small>{row.rootLabel}</small>
                  </div>
                  <span class="repo-branch-badge">{row.branchLabel}</span>
                  {#if row.taskID && repoDashboardTaskUrl(row)}
                    <a
                      class="repo-task-link"
                      href={repoDashboardTaskUrl(row) ?? ''}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {row.taskID}
                    </a>
                  {:else}
                    <span class="repo-branch-badge">{repoDashboardTaskLabel(row)}</span>
                  {/if}
                  <small>{repoDashboardDirtyLabel(row)} · {repoDashboardRemoteLabel(row)}</small>
                  <div class="activity-row-actions repository-activity-actions row-action-menu-anchor" aria-label="Repository actions">
                    <button
                      type="button"
                      aria-label="Repository actions"
                      aria-haspopup="menu"
                      aria-expanded={gitRowActionMenuOpen(row.id, 'repository')}
                      title="Repository actions"
                      onclick={() => toggleGitRowActionMenu(row.id, 'repository')}
                    >
                      <MoreHorizontal size={13} strokeWidth={2} />
                    </button>
                    {#if gitRowActionMenuOpen(row.id, 'repository')}
                      <div class="row-action-menu" role="menu" aria-label="Repository actions">
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Copy repository path"
                          title="Copy repository path"
                          onclick={() => {
                            closeGitRowActionMenu();
                            copyActivityCommand(row.path, 'Repository path copied');
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy path</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open repository path"
                          title="Open repository path"
                          onclick={() => {
                            closeGitRowActionMenu();
                            openActivityPath(row.path);
                          }}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                          <span>Open path</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Open repository in terminal"
                          title="Open repository in terminal"
                          onclick={() => {
                            closeGitRowActionMenu();
                            openActivityTerminalPath(row.path);
                          }}
                        >
                          <Terminal size={12} strokeWidth={2} />
                          <span>Open terminal</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label="Reveal repository path"
                          title="Reveal repository path"
                          onclick={() => {
                            closeGitRowActionMenu();
                            revealActivityPath(row.path);
                          }}
                        >
                          <FolderSearch size={12} strokeWidth={2} />
                          <span>Reveal path</span>
                        </button>
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
            {/if}
            </details>

            {#if selectedProjectGitTaskLedger.length > 0}
              <details class="activity-git-secondary-section" data-testid="git-task-ledger-section">
                <summary>
                  <span>Task ledger</span>
                  <small>{selectedProjectGitTaskLedger.length}</small>
                </summary>
              {#each selectedProjectGitTaskLedger as row (row.taskID)}
                {@const ledgerWorktree = row.primaryWorktree}
                {@const ledgerAction = ledgerWorktree ? projectWorktreePrimaryAction(ledgerWorktree) : null}
                <div
                  class={`activity-task-ledger-row ${row.tone}`}
                  data-task-ledger-id={row.taskID}
                  tabindex="-1"
                  title={gitTaskLedgerTitle(row)}
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openGitRowActionMenu(row.taskID, 'task-ledger');
                  }}
                >
                  <div class="activity-row-main">
                    <strong class="task-ledger-title">
                      {#if gitTaskUrl(row.taskID)}
                        <a class="git-task-link" href={gitTaskUrl(row.taskID) ?? ''} target="_blank" rel="noreferrer">
                          {row.taskID}
                        </a>
                      {:else}
                        <span>{row.taskID}</span>
                      {/if}
                      <span>{row.nextAction}</span>
                    </strong>
                    <small>{row.sourceSummary} · {row.ownerSummary} · {row.cleanupSummary}</small>
                  </div>
                  <div class="activity-task-ledger-chips" aria-label={`${row.taskID} task metadata`}>
                    {#if row.worktreeCount > 0}
                      <span class="task-ledger-chip">wt {row.worktreeCount}</span>
                    {/if}
                    {#if row.blockedWorktreeCount > 0}
                      <span class="task-ledger-chip blocked">blocked {row.blockedWorktreeCount}</span>
                    {/if}
                    {#if row.readyWorktreeCount > 0}
                      <span class="task-ledger-chip ready">ready {row.readyWorktreeCount}</span>
                    {/if}
                    {#if row.staleCleanWorktreeCount > 0}
                      <span class="task-ledger-chip stale">stale {row.staleCleanWorktreeCount}</span>
                    {/if}
                    {#if row.backupRequiredWorktreeCount > 0}
                      <span class="task-ledger-chip backup">backup {row.backupRequiredWorktreeCount}</span>
                    {/if}
                    {#if row.activeSessionCount > 0}
                      <span class="task-ledger-chip active">active {row.activeSessionCount}</span>
                    {/if}
                    {#if row.savedWorkspaceCount > 0}
                      <span class="task-ledger-chip saved">saved {row.savedWorkspaceCount}</span>
                    {/if}
                    {#if row.runCount > 0}
                      <span class="task-ledger-chip">runs {row.runCount}</span>
                    {/if}
                    {#if row.commitCount > 0}
                      <span class="task-ledger-chip">commits {row.commitCount}</span>
                    {/if}
                  </div>
                  <div class="activity-row-actions task-ledger-actions row-action-menu-anchor" aria-label="Task ledger actions">
                    <button
                      type="button"
                      aria-label={`Task ledger actions for ${row.taskID}`}
                      aria-haspopup="menu"
                      aria-expanded={gitRowActionMenuOpen(row.taskID, 'task-ledger')}
                      title="Task ledger actions"
                      onclick={() => toggleGitRowActionMenu(row.taskID, 'task-ledger')}
                    >
                      <MoreHorizontal size={13} strokeWidth={2} />
                    </button>
                    {#if gitRowActionMenuOpen(row.taskID, 'task-ledger')}
                      <div class="row-action-menu" role="menu" aria-label={`Task ledger actions for ${row.taskID}`}>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label={`Open task reference for ${row.taskID}`}
                          title="Open task reference"
                          disabled={!gitTaskUrl(row.taskID)}
                          onclick={() => {
                            closeGitRowActionMenu();
                            openGitTaskReference(row.taskID);
                          }}
                        >
                          <ExternalLink size={12} strokeWidth={2} />
                          <span>Open task reference</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label={`Copy task ledger for ${row.taskID}`}
                          title="Copy task ledger"
                          onclick={() => {
                            closeGitRowActionMenu();
                            copyGitTaskLedger(row);
                          }}
                        >
                          <Copy size={12} strokeWidth={2} />
                          <span>Copy task ledger</span>
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          aria-label={`Open worktree for ${row.taskID}`}
                          title="Open task worktree in source browser"
                          disabled={!ledgerWorktree}
                          onclick={() => {
                            closeGitRowActionMenu();
                            if (ledgerWorktree) openWorktreeInSourceBrowser(ledgerWorktree);
                          }}
                        >
                          <FolderOpen size={12} strokeWidth={2} />
                          <span>Open worktree</span>
                        </button>
                        <button
                          class={ledgerAction ? `worktree-primary-action ${ledgerAction.kind}` : 'worktree-primary-action'}
                          type="button"
                          role="menuitem"
                          aria-label={`Run worktree action for ${row.taskID}`}
                          title={ledgerAction?.title ?? 'No worktree action'}
                          disabled={!ledgerWorktree || (ledgerWorktree ? fileActionBusy === `worktree-primary:${ledgerWorktree.path}` : false)}
                          onclick={() => {
                            closeGitRowActionMenu();
                            if (ledgerWorktree) runWorktreePrimaryAction(ledgerWorktree);
                          }}
                        >
                          {#if ledgerAction?.kind === 'cleanup'}
                            <Trash2 size={12} strokeWidth={2} />
                          {:else if ledgerAction?.kind === 'backup'}
                            <Save size={12} strokeWidth={2} />
                          {:else}
                            <History size={12} strokeWidth={2} />
                          {/if}
                          <span>{ledgerAction?.label ?? 'Run worktree action'}</span>
                        </button>
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
              </details>
            {/if}

            <details class="activity-git-secondary-section" data-testid="git-history-section">
              <summary>
                <span>Recent commits</span>
                <small>{filteredGitCommitRows.length}</small>
              </summary>
              <div
                class="git-graph-summary-strip activity-git-graph-summary"
                aria-label="Git graph view model summary"
                title={selectedProjectGitTaskSearchSummary}
              >
                <span>{selectedProjectGitGraphSummary}</span>
                {#if selectedProjectGitGraph.taskSearchTargets.length > 0}
                  <small>{selectedProjectGitGraph.taskSearchTargets.length} search targets</small>
                {/if}
              </div>
              {#if filteredGitCommitRows.length === 0}
                <div class="activity-empty">No commits</div>
              {:else}
                {#each filteredGitCommitRows.slice(0, 8) as row (row.sha)}
                {@const entry = gitCommitEntryForRow(row)}
                <div
                  class="activity-commit-row"
                  title={row.detailLabel}
                  oncontextmenu={(event) => {
                    event.preventDefault();
                    openActivityRowActionMenu('commit', row.sha);
                  }}
                >
                  <span
                    class={`git-graph-marker ${row.graphKind}`}
                    aria-label={row.topologyLabel}
                    title={row.topologyLabel}
                  ></span>
                  <div class="activity-row-main">
                    <strong>{row.subject}</strong>
                    <small>{row.shortSha} · {formatGitCommitTime(row.committedAt)}</small>
                  </div>
                  <div class="activity-commit-meta">
                    {#if row.taskID && gitTaskUrl(row.taskID)}
                      <a
                        class="git-task-link"
                        href={gitTaskUrl(row.taskID) ?? ''}
                        target="_blank"
                        rel="noreferrer"
                        title={`Task from ${gitGraphCommitTaskSourceLabel(row) || 'Git metadata'}`}
                      >
                        {row.taskID}
                      </a>
                    {/if}
                    <div class="activity-row-actions commit-activity-actions row-action-menu-anchor" aria-label="Commit actions">
                      <button
                        type="button"
                        aria-label="Commit actions"
                        aria-haspopup="menu"
                        aria-expanded={activityRowActionMenuOpen('commit', row.sha)}
                        title="Commit actions"
                        onclick={() => toggleActivityRowActionMenu('commit', row.sha)}
                      >
                        <MoreHorizontal size={13} strokeWidth={2} />
                      </button>
                      {#if activityRowActionMenuOpen('commit', row.sha)}
                        <div class="row-action-menu" role="menu" aria-label="Commit actions">
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Copy commit SHA"
                            title="Copy commit SHA"
                            disabled={!entry}
                            onclick={() => {
                              closeActivityRowActionMenu();
                              if (entry) copyGitCommitSha(entry);
                            }}
                          >
                            <Copy size={12} strokeWidth={2} />
                            <span>Copy SHA</span>
                          </button>
                          <button
                            type="button"
                            role="menuitem"
                            aria-label="Copy commit summary"
                            title="Copy commit summary"
                            disabled={!entry}
                            onclick={() => {
                              closeActivityRowActionMenu();
                              if (entry) copyGitCommitSummary(entry);
                            }}
                          >
                            <History size={12} strokeWidth={2} />
                            <span>Copy summary</span>
                          </button>
                          {#if row.taskID}
                            <button
                              type="button"
                              role="menuitem"
                              aria-label="Copy task reference"
                              title="Copy task reference"
                              onclick={() => {
                                closeActivityRowActionMenu();
                                copyGitTaskReference(row.taskID);
                              }}
                            >
                              <ExternalLink size={12} strokeWidth={2} />
                              <span>Copy task ref</span>
                            </button>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  </div>
                </div>
              {/each}
            {/if}
            </details>
          </div>
          {/if}
        {/if}
      </div>
    {/if}
    </div>
  </aside>
  </SourceDockviewShell>

  {#if !sourceDockviewWorkbenchEnabled}
    <SourcePaneResizer
      className="side-pane-resizer"
      ariaLabel="Resize side pane"
      title="Drag to resize. Press Enter or double-click to collapse or expand."
      orientation="vertical"
      onpointerdown={beginSidePaneResize}
      ondblclick={toggleActivityPaneRail}
      onkeydown={handleSidePaneResizerKeydown}
    />
  {/if}
  {:else if !sourceDockviewWorkbenchEnabled && !activityPaneViewportCollapsed()}
    <button
      class="activity-restore-button"
      type="button"
      aria-label="Show Activity panel"
      title="Show Activity panel"
      onclick={() => showDockPanel('activity')}
    >
      <PanelLeftOpen size={15} strokeWidth={2} />
      <span>Activity</span>
    </button>
  {/if}

  <section
    class="workspace"
    class:chrome-compact={sourceChromeCompact}
    class:context-side={contextPanelPlacement === 'side' && shouldRenderDockPanel('context')}
    class:context-bottom={contextPanelPlacement === 'bottom' && shouldRenderDockPanel('context')}
    aria-label="Source preview"
  >
    <header class="topbar">
      <div>
        <p class="eyebrow">Source Preview</p>
        <h2 title={sourceContextIdentity.summary}>{preview?.fileName ?? 'No file selected'}</h2>
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
                <button
                  class="view-menu-wide-button"
                  type="button"
                  role="menuitem"
                  aria-label="Focus editor canvas"
                  title="Hide context, insights, terminal, and browser"
                  onclick={() => {
                    focusSourceEditorLayout();
                    closeViewMenu();
                  }}
                >
                  Focus editor
                </button>
                <button
                  class="view-menu-wide-button"
                  type="button"
                  role="menuitem"
                  aria-label="Restore layout before focus"
                  title="Restore the pane arrangement saved before Focus editor"
                  disabled={!sourceFocusRestoreLayout}
                  onclick={() => {
                    restoreSourceLayoutBeforeFocus();
                    closeViewMenu();
                  }}
                >
                  Restore previous
                </button>
                <div class="view-menu-button-grid two">
                  <button
                    class:active={sourceChromeCompact}
                    type="button"
                    role="menuitem"
                    aria-label="Use compact editor chrome"
                    onclick={() => {
                      selectSourceChromeCompact(true);
                      closeViewMenu();
                    }}
                  >
                    Compact
                  </button>
                  <button
                    class:active={!sourceChromeCompact}
                    type="button"
                    role="menuitem"
                    aria-label="Use comfortable editor chrome"
                    onclick={() => {
                      selectSourceChromeCompact(false);
                      closeViewMenu();
                    }}
                  >
                    Comfort
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
                <button
                  class="view-menu-wide-button"
                  class:active={activityPaneRailOnly()}
                  type="button"
                  role="menuitem"
                  aria-label="Collapse explorer to icon rail"
                  title="Shrink the source browser to the activity icon rail"
                  disabled={activityPaneRailOnly()}
                  onclick={() => {
                    collapseActivityPaneToRail();
                    closeViewMenu();
                  }}
                >
                  Collapse to icon rail
                </button>
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
                  class:active={contextPaneRailOnly()}
                  type="button"
                  role="menuitem"
                  aria-label="Collapse context to icon rail"
                  title="Shrink the context cards to the icon rail"
                  disabled={contextPaneRailOnly()}
                  onclick={() => {
                    collapseContextPaneToRail();
                    closeViewMenu();
                  }}
                >
                  Collapse to icon rail
                </button>
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

      {#if !sourceDockviewWorkbenchEnabled}
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
                  class:drop-target={dockDropTargetPanelID === panelID}
                  class:drop-after={dockDropTargetPanelID === panelID && dockDropTargetPanelPlacement === 'after'}
                  role="presentation"
                  draggable={dockPanelMoveTargets(panelID).length > 1}
                  ondragstart={(event) => beginDockPanelDrag(panelID, event)}
                  ondragenter={(event) => dragOverDockPanelTab(event, panelID)}
                  ondragover={(event) => dragOverDockPanelTab(event, panelID)}
                  ondragleave={() => {
                    if (dockDropTargetPanelID === panelID) dockDropTargetPanelID = null;
                  }}
                  ondrop={(event) => dropDockPanelOnTab(event, panelID)}
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

      {#if hiddenDockPanelIDs().length > 0}
        <div class="hidden-dock-panel-rail" aria-label="Hidden dock panels">
          {#each hiddenDockPanelIDs() as panelID (panelID)}
            <button
              class="hidden-dock-panel-button"
              type="button"
              aria-label={`Restore ${dockPanelLabel(panelID)} panel`}
              title={`Restore ${dockPanelLabel(panelID)} panel`}
              onclick={() => restoreHiddenDockPanel(panelID)}
            >
              {#if panelID === 'activity'}
                <FolderGit2 size={13} strokeWidth={1.9} />
              {:else if panelID === 'context'}
                <Network size={13} strokeWidth={1.9} />
              {:else if panelID === 'insights'}
                <Search size={13} strokeWidth={1.9} />
              {:else if panelID === 'terminal'}
                <Terminal size={13} strokeWidth={1.9} />
              {:else}
                <ExternalLink size={13} strokeWidth={1.9} />
              {/if}
              <span>{dockPanelLabel(panelID)}</span>
            </button>
          {/each}
        </div>
      {/if}

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
      {/if}

      <div
      class="workspace-arrangement"
      class:context-top={contextPanelPlacement === 'top' && effectiveContextPaneVisible()}
      class:context-side={contextPanelPlacement === 'side' && effectiveContextPaneVisible()}
      class:context-bottom={contextPanelPlacement === 'bottom' && effectiveContextPaneVisible()}
      class:context-rail-only={contextPaneRailOnly()}
    >
      {#if sourceDockviewWorkbenchOwnsPanel('context') || effectiveContextPaneVisible()}
      {#if useUnifiedWorkbench}
      <!--
        Unified workbench: render a clean, spacious collapsible-section stack as the context
        panel content and route it into the Dockview via the same setPanelElement bridge. The
        legacy grid/stack/rail context markup (the else branch below) is bypassed entirely.
      -->
      <div class="workbench-context-host" use:sourceDockviewPanelAction={'context'}>
        <WorkbenchContextPanel
          runs={selectedProjectOrchestrationRuns}
          runsSummary={orchestrationRunSummary}
          runsLoading={orchestrationRunsLoading}
          runtimeContexts={selectedProjectRuntimeContexts}
          runtimeSummary={runtimeContextSummary}
          runtimeLoading={runtimeContextsLoading}
          agents={selectedProjectAgentSessions}
          agentsSummary={agentSessionSummary}
          agentsLoading={agentSessionsLoading}
          worktrees={prioritizedProjectWorktrees}
          worktreesSummary={projectWorktreeSummary}
          worktreesSafetyStats={projectWorktreeSafetyStats}
          worktreesLoading={projectWorktreesLoading}
          repos={selectedProjectRepositorySummaries}
          reposSummary={repoDashboardSummary}
          reposLoading={gitRepositorySummariesLoading}
          projectName={selectedProject.name}
        />
      </div>
      {/if}
      {/if}

      <div class="workspace-main-column">
      <SourceDockviewShell
        shellClass="source-dockview-center-shell"
        hostClass="source-dockview-center-host"
        errorClass="source-dockview-center-error"
        enabled={sourceDockviewCenterEnabled}
        ready={sourceDockviewCenterReady}
        error={sourceDockviewCenterError}
        hostAction={sourceDockviewCenterHostAction}
      >
      <section class="source-editor-dock-panel" aria-label="Source editor" use:sourceDockviewPanelAction={'editor'}>

    {#if projectOpenSourceTabs.length > 0}
      <SourceDockviewShell
        shellClass="source-dockview-editor-files-shell"
        hostClass="source-dockview-editor-files-host"
        errorClass="source-dockview-editor-files-error"
        enabled={true}
        ready={sourceEditorFileDockviewReady}
        error={sourceEditorFileDockviewError}
        hostAction={sourceEditorFileDockviewHostAction}
      >
        {#each projectOpenSourceTabs as tab (tab.path)}
          {@const tabGitStatus = gitStatusForSourceRecord(tab)}
          <section
            class="source-editor-file-pane"
            class:active={tab.path === selectedRecord?.path}
            class:dirty={isSourcePathDirty(tab.path)}
            aria-label={`Source editor for ${tab.fileName}`}
            data-source-path={tab.path}
            use:sourceEditorFileDockviewPanelAction={sourceEditorFilePanelID(tab)}
          >
            {#if tab.path === selectedRecord?.path}

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
            <span class="editor-file-glyph" aria-hidden="true">
              <FileCode2 size={13} strokeWidth={1.8} />
            </span>
            <div class="editor-file-title">
              <strong>{preview.fileName}</strong>
              <small>
                {selectedIndex} / {records.length}
                {#if selectedSourceLine}
                  · line {selectedSourceLine}
                {/if}
              </small>
            </div>
            {#if selectedSourceDirty}
              <Chip size="xs" tone="attention">modified</Chip>
            {/if}
            {#if sourceIntelligenceAvailable}
              <span class="editor-lsp-chip" title={sourceLspStatusTitle()}>
                <Chip
                  size="xs"
                  tone={sourceLspStatus?.available ? 'good' : (!sourceLspStatusLoading ? 'attention' : 'muted')}
                >
                  {sourceLspStatusLabel()}
                </Chip>
              </span>
            {/if}
          </div>
          {#if sourceIntelligenceAvailable}
            <div class="editor-lsp-recovery-strip" aria-label="Language server recovery actions">
              <button
                class="editor-lsp-recovery-action"
                type="button"
                aria-label="Retry language server status"
                title="Retry language server status"
                disabled={sourceLspStatusLoading}
                onclick={() => loadSourceLspStatus(preview, selectedProject)}
              >
                <RefreshCw size={13} strokeWidth={2} />
              </button>
              <button
                class="editor-lsp-recovery-action"
                type="button"
                aria-label="Copy language server status"
                title="Copy language server status"
                onclick={copySourceLspStatusReport}
              >
                <Copy size={13} strokeWidth={2} />
              </button>
              <button
                class="editor-lsp-recovery-action"
                type="button"
                aria-label="Copy language server install command"
                title="Copy language server install command"
                disabled={sourceLspStatus?.available || !sourceLspInstallCommand()}
                onclick={copySourceLspInstallCommand}
              >
                <Terminal size={13} strokeWidth={2} />
              </button>
            </div>
          {/if}
          {#if selectedSourceMarkdownPreviewAvailable}
            <div class="editor-mode-toggle" role="tablist" aria-label="Markdown editor mode">
              <button
                class:active={selectedSourceEditorDisplayMode === 'source'}
                type="button"
                role="tab"
                aria-selected={selectedSourceEditorDisplayMode === 'source'}
                title="Edit Markdown source"
                onclick={() => setSelectedSourceEditorDisplayMode('source')}
              >
                <FileCode2 size={12} strokeWidth={2} />
                <span>Source</span>
              </button>
              <button
                class:active={selectedSourceEditorDisplayMode === 'preview'}
                type="button"
                role="tab"
                aria-selected={selectedSourceEditorDisplayMode === 'preview'}
                title="Preview Markdown"
                onclick={() => setSelectedSourceEditorDisplayMode('preview')}
              >
                <BookOpen size={12} strokeWidth={2} />
                <span>Preview</span>
              </button>
            </div>
          {/if}
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
                  aria-label="Trigger completions"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('completion');
                  }}
                >
                  <Braces size={13} strokeWidth={2} />
                  <span>Completions</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  aria-label="Show signature help"
                  disabled={!preview || loading || !sourceIntelligenceAvailable}
                  onclick={() => {
                    closeEditorActionMenu();
                    requestSourceIntelligenceAction('signature-help');
                  }}
                >
                  <Activity size={13} strokeWidth={2} />
                  <span>Signature help</span>
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

        <div class="editor-local-nav" aria-label="Editor navigation">
          <button
            class:active={editorNavPanel === 'problems'}
            type="button"
            aria-label="Show editor problems"
            title={sourceDiagnosticSummary}
            onclick={() => toggleEditorNavPanel('problems')}
          >
            <Activity size={12} strokeWidth={2} />
            <span>Problems</span>
            <strong>{sourceDiagnostics.length}</strong>
          </button>
          <button
            class:active={editorNavPanel === 'symbols'}
            type="button"
            aria-label="Show editor symbols"
            title={`${sourceSymbols.length} symbols`}
            onclick={() => toggleEditorNavPanel('symbols')}
          >
            <FileCode2 size={12} strokeWidth={2} />
            <span>Symbols</span>
            <strong>{sourceSymbols.length}</strong>
          </button>
          <button
            class:active={editorNavPanel === 'definitions'}
            type="button"
            aria-label="Show definition results"
            title={sourceLookupSummary}
            onclick={() => toggleEditorNavPanel('definitions')}
          >
            <Search size={12} strokeWidth={2} />
            <span>Defs</span>
            <strong>{sourceLookupLoading ? '...' : sourceDefinitionNavCount}</strong>
          </button>
          <button
            class:active={editorNavPanel === 'references'}
            type="button"
            aria-label="Show reference results"
            title={sourceReferenceSummary || 'References'}
            onclick={() => toggleEditorNavPanel('references')}
          >
            <Braces size={12} strokeWidth={2} />
            <span>Refs</span>
            <strong>{sourceReferenceLoading ? '...' : sourceReferenceTargets.length}</strong>
          </button>
        </div>

        {#if editorNavPanel}
          <div class="editor-nav-drawer" aria-label="Editor navigation drawer">
            <div class="editor-nav-drawer-header">
              <strong>
                {editorNavPanel === 'problems'
                  ? 'Problems'
                  : editorNavPanel === 'symbols'
                    ? 'Symbols'
                    : editorNavPanel === 'references'
                      ? 'References'
                      : 'Definitions'}
              </strong>
              <span>
                {editorNavPanel === 'problems'
                  ? sourceDiagnosticSummary
                  : editorNavPanel === 'symbols'
                    ? `${sourceSymbols.length} symbols`
                    : editorNavPanel === 'references'
                      ? sourceReferenceSummary || 'No references yet'
                      : sourceLookupSummary}
              </span>
              <button
                class="editor-nav-close"
                type="button"
                aria-label="Close editor navigation drawer"
                title="Close"
                onclick={closeEditorNavPanel}
              >
                <X size={12} strokeWidth={2} />
              </button>
            </div>

            <div class="editor-nav-list">
              {#if editorNavPanel === 'problems'}
                {#if sourceDiagnostics.length === 0}
                  <div class="intelligence-empty compact">No problems</div>
                {:else}
                  {#each sourceDiagnostics as diagnostic, index (`nav:${diagnostic.line}:${diagnostic.column}:${index}`)}
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
              {:else if editorNavPanel === 'symbols'}
                {#if sourceSymbols.length === 0}
                  <div class="intelligence-empty compact">No symbols</div>
                {:else}
                  {#each sourceSymbols as symbol (`nav:${symbol.kind}:${symbol.name}:${symbol.line}`)}
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
              {:else if editorNavPanel === 'references'}
                {#if sourceReferenceTargets.length === 0 && !sourceReferenceLoading}
                  <div class="intelligence-empty compact">No references</div>
                {:else}
                  {#each sourceReferenceTargets as target (`nav:${target.path}:${target.line}:${target.column}`)}
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
              {:else}
                {#if sourceDefinitionTargets.length === 0 && sourceImplementationTargets.length === 0 && sourceTypeDefinitionTargets.length === 0 && !sourceLookupLoading}
                  <div class="intelligence-empty compact">No definitions</div>
                {:else}
                  {#each sourceDefinitionTargets as target (`nav-definition:${target.path}:${target.line}:${target.symbolName}`)}
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
                  {#each sourceImplementationTargets as target (`nav-implementation:${target.path}:${target.line}:${target.symbolName}`)}
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
                  {#each sourceTypeDefinitionTargets as target (`nav-type-definition:${target.path}:${target.line}:${target.symbolName}`)}
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

        <div class="editor-body-grid" class:insights-hidden={!editorInsightsDockColumnVisible()}>
          <div class="editor-canvas">
            {#if selectedSourceMarkdownPreviewAvailable && selectedSourceEditorDisplayMode === 'preview'}
              <SourceMarkdownPreview
                content={selectedSourceDraftContent}
                fileName={preview.fileName}
                relativePath={preview.relativePath}
                dirty={selectedSourceDirty}
              />
            {:else}
              {#key sourcePreviewAppearanceKey}
                <MonacoSourceEditor
                  {preview}
                  content={selectedSourceDraftContent}
                  editable={true}
                  appearanceOverride={editorAppearanceOverride}
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
                  onExternalNavigation={navigateEditorExternalSource}
                  onExternalPreviewLookup={loadEditorExternalSourcePreview}
                  onFormatDocument={handleEditorFormatDocument}
                  onGoToLineRequest={openCurrentFileGoToLine}
                  onHoverLookup={handleEditorHoverLookup}
                  onImplementationLookup={handleEditorImplementationLookup}
                  onInlayHintLookup={handleEditorInlayHintLookup}
                  onNavigateBackRequest={navigateSourceBack}
                  onNavigateForwardRequest={navigateSourceForward}
                  onNextProblemRequest={selectNextSourceDiagnostic}
                  onProblemsRequest={() => openEditorNavPanel('problems')}
                  onPreviousProblemRequest={selectPreviousSourceDiagnostic}
                  onQuickOpenRequest={openQuickOpen}
                  onReferenceCountLookup={handleEditorReferenceCountLookup}
                  onReferenceLookup={handleEditorReferenceLookup}
                  onRename={handleEditorRename}
                  onSaveRequest={saveSelectedSourceFile}
                  onSemanticTokensLookup={handleEditorSemanticTokensLookup}
                  onSignatureHelpLookup={handleEditorSignatureHelpLookup}
                  onSymbolsRequest={() => openEditorNavPanel('symbols')}
                  onSymbolsChange={handleEditorSymbolsChange}
                  onTypeDefinitionLookup={handleEditorTypeDefinitionLookup}
                  onWorkspaceEditAction={handleEditorWorkspaceEditAction}
                />
              {/key}
            {/if}

            {#if editorInsightCollapsed && editorNavPanel === null && (sourceDefinitionQuery || sourceDefinitionTargets.length > 0 || sourceDefinitionLoading || sourceImplementationQuery || sourceImplementationTargets.length > 0 || sourceImplementationLoading || sourceTypeDefinitionQuery || sourceTypeDefinitionTargets.length > 0 || sourceTypeDefinitionLoading)}
              <div class="editor-lookup-popover" aria-label="Editor lookup results">
                <div class="editor-lookup-header">
                  <strong>{sourceImplementationQuery ? sourceImplementationSummary : sourceTypeDefinitionQuery ? sourceTypeDefinitionSummary : sourceDefinitionSummary}</strong>
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

          {#if sourceIntelligencePanelMounted()}
            {#if editorInsightsDockColumnVisible()}
            <button
              class="editor-insight-resizer"
              type="button"
              aria-label="Resize editor insights"
              title="Resize editor insights"
              onpointerdown={beginEditorInsightResize}
              onkeydown={handleEditorInsightResizerKeydown}
            ></button>
            {/if}

            <SourceDockviewShell
              shellClass={editorInsightsDockColumnVisible()
                ? 'source-dockview-insights-shell'
                : 'source-dockview-insights-parking'}
              hostClass="source-dockview-insights-host"
              errorClass="source-dockview-insights-error"
              enabled={editorInsightsDockColumnVisible() && sourceDockviewInsightsEnabled}
              ready={editorInsightsDockColumnVisible() && sourceDockviewInsightsReady}
              error={editorInsightsDockColumnVisible() ? sourceDockviewInsightsError : ''}
              hostAction={sourceDockviewInsightsHostAction}
            >
              <aside
                class="source-intelligence-panel"
                aria-label="Language intelligence"
                use:sourceDockviewPanelAction={'insights'}
              >
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
                    <div class="git-insights-section-heading">
                      <span class="git-insights-section-title">Changes</span>
                      <span class="git-insights-section-count">{selectedProjectGitChangedFiles.length}</span>
                    </div>
                    <div class="git-status-overview">{selectedProjectGitFileGroupSummary}</div>
                    {#each selectedProjectGitFileGroups as group (group.id)}
                      <section class="git-status-group" data-group={group.id} aria-label={`${group.label} Git files`}>
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
                  <div class="git-history-heading git-insights-section-heading">
                    <span class="git-insights-section-title">History</span>
                    <span class="git-insights-section-count">{selectedProjectGitGraph.commits.length}</span>
                    <small>{gitCommitHistorySummary}</small>
                  </div>
                  <div class="git-graph-summary-strip" aria-label="Git graph view model summary" title={selectedProjectGitTaskSearchSummary}>
                    <span>{selectedProjectGitGraphSummary}</span>
                    {#if selectedProjectGitGraph.summary.mergeCommitCount > 0}
                      <small>{selectedProjectGitGraph.summary.mergeCommitCount} merges</small>
                    {/if}
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
                  {#if selectedProjectGitTaskIDs.length > 0 || selectedProjectGitTaskSourceGroups.length > 0}
                  <section class="git-insights-section" aria-label="Git tasks">
                    <div class="git-insights-section-heading">
                      <span class="git-insights-section-title">Tasks</span>
                      <span class="git-insights-section-count">{selectedProjectGitTaskSourceGroups.length || selectedProjectGitTaskIDs.length}</span>
                    </div>
                  {#if selectedProjectGitTaskSourceGroups.length === 0 && selectedProjectGitTaskIDs.length > 0}
                    <div class="git-task-trail" aria-label="Git task links">
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
                            onclick={() => copyGitTaskSourceGroup(group)}
                          >
                            <Copy size={11} strokeWidth={2} />
                          </button>
                        </div>
                      {/each}
                    </div>
                  {/if}
                  </section>
                  {/if}
                  {#if selectedGitCommit && selectedGitCommitRow}
                    <details
                      class="git-commit-detail-drawer"
                      aria-label="Selected commit detail"
                      title={selectedGitCommitRow.detailLabel}
                    >
                      <summary class="git-commit-detail-summary">
                        <span class="git-commit-detail-summary-main">
                          <strong>{selectedGitCommitRow.subject}</strong>
                          <small>{selectedGitCommitRow.metaLabel}</small>
                        </span>
                        <span class="git-commit-detail-summary-ref">{selectedGitCommitRow.refs.label}</span>
                      </summary>
                      <div class="git-commit-detail-body">
                        <div class="git-commit-detail-facts" aria-label="Selected commit metadata">
                          <span>
                            <strong>Commit</strong>
                            <small>{selectedGitCommitRow.sha}</small>
                          </span>
                          <span>
                            <strong>Refs</strong>
                            <small>{selectedGitCommitRow.refs.label}</small>
                          </span>
                          <span>
                            <strong>Parents</strong>
                            <small>{selectedGitCommitRow.parentHint.label}</small>
                          </span>
                          {#if selectedGitCommitRow.taskID}
                            <span>
                              <strong>Task</strong>
                              <small>{selectedGitCommitRow.taskID}</small>
                            </span>
                          {/if}
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
                          {#if selectedGitCommitRow.taskID}
                            {#if gitTaskUrl(selectedGitCommitRow.taskID)}
                              <button
                                type="button"
                                aria-label="Open selected commit task reference"
                                title="Open selected commit task reference"
                                onclick={() => openGitTaskReference(selectedGitCommitRow.taskID)}
                              >
                                <ExternalLink size={11} strokeWidth={2} />
                              </button>
                            {/if}
                            <button
                              type="button"
                              aria-label="Copy selected commit task reference"
                              title="Copy selected commit task reference"
                              onclick={() => copyGitTaskReference(selectedGitCommitRow.taskID)}
                            >
                              <Copy size={11} strokeWidth={2} />
                            </button>
                          {/if}
                        </div>
                      </div>
                    </details>
                  {/if}
                  <div class="git-history-list">
                    {#if gitCommitHistoryLoading}
                      <div class="intelligence-empty">Loading history</div>
                    {:else if gitCommitHistoryError}
                      <div class="intelligence-empty">{gitCommitHistoryError}</div>
                    {:else if selectedProjectGitGraph.commits.length === 0}
                      <div class="intelligence-empty">No commits</div>
                    {:else}
                      {#each selectedProjectGitGraph.commits as row (row.sha)}
                        {@const entry = gitCommitEntryForRow(row)}
                        <div
                          class={`git-history-row ${row.graphKind}`}
                          class:selected={selectedGitCommitSha === row.sha}
                          role="button"
                          tabindex="0"
                          title={row.detailLabel}
                          onclick={() => entry && selectGitCommit(entry)}
                          onkeydown={(event) => entry && handleGitCommitRowKeydown(event, entry)}
                        >
                          <span
                            class={`git-graph-marker ${row.graphKind}`}
                            aria-label={row.topologyLabel}
                            title={row.topologyLabel}
                          ></span>
                          <div class="git-history-main">
                            <strong>{row.subject}</strong>
                            <small>{row.metaLabel}</small>
                          </div>
                          <div class="git-history-meta">
                            <div class="git-history-badges" aria-label="Commit ownership badges">
                              {#each row.ownershipBadges as badge (`${badge.tone}:${badge.label}`)}
                                <span class={`git-history-badge ${badge.tone}`} title={badge.title}>{badge.label}</span>
                              {/each}
                            </div>
                            {#if row.taskID}
                              {#if gitTaskUrl(row.taskID)}
                                <a
                                  class="git-task-link"
                                  href={gitTaskUrl(row.taskID) ?? ''}
                                  target="_blank"
                                  rel="noreferrer"
                                  title={`Task from ${gitGraphCommitTaskSourceLabel(row) || 'Git metadata'}`}
                                >
                                  {row.taskID}
                                </a>
                              {:else}
                                <span
                                  class="git-task-link"
                                  title={`Task from ${gitGraphCommitTaskSourceLabel(row) || 'Git metadata'}`}
                                >{row.taskID}</span>
                              {/if}
                            {/if}
                            <div class="git-history-actions" aria-label="Commit quick actions">
                              <button
                                type="button"
                                aria-label="Copy commit SHA"
                                title="Copy commit SHA"
                                disabled={!entry}
                                onclick={() => entry && copyGitCommitSha(entry)}
                              >
                                <Copy size={11} strokeWidth={2} />
                              </button>
                              <button
                                type="button"
                                aria-label="Copy commit summary"
                                title="Copy commit summary"
                                disabled={!entry}
                                onclick={() => entry && copyGitCommitSummary(entry)}
                              >
                                <History size={11} strokeWidth={2} />
                              </button>
                              <button
                                type="button"
                                aria-label="Copy commit handoff"
                                title="Copy commit handoff"
                                disabled={!entry}
                                onclick={() => entry && copyGitCommitHandoff(entry)}
                              >
                                <FileCode2 size={11} strokeWidth={2} />
                              </button>
                              {#if row.taskID}
                                <button
                                  type="button"
                                  aria-label="Copy task reference"
                                  title="Copy task reference"
                                  onclick={() => copyGitTaskReference(row.taskID)}
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
            </SourceDockviewShell>
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
            {:else}
              <button
                class="editor-file-placeholder"
                type="button"
                title={tab.relativePath}
                onclick={() => selectOpenTab(tab)}
              >
                <FileCode2 size={28} strokeWidth={1.55} />
                <strong>{tab.fileName}</strong>
                <span>{tab.relativePath}</span>
                <small>
                  {tab.language}
                  {#if tabGitStatus}
                    · {tabGitStatus.status}
                  {/if}
                  {#if isSourcePathDirty(tab.path)}
                    · modified
                  {/if}
                </small>
              </button>
            {/if}
          </section>
        {/each}
      </SourceDockviewShell>
    {:else}
      {#if fileActionStatus}
        <div class="file-action-feedback">{fileActionStatus}</div>
      {/if}

      {#if error}
        <div class="inline-error">
          <Activity size={15} strokeWidth={1.8} />
          <span>{error}</span>
        </div>
      {/if}

      <div class="empty-preview">
        <FileCode2 size={34} strokeWidth={1.55} />
        <strong>No source file loaded</strong>
        <span>Scan a project or choose a file from the tree.</span>
      </div>
    {/if}
      </section>
      </SourceDockviewShell>

      <div class="source-runtime-panel-stage" aria-hidden="true">

      {#if sourceDockPanelVisible('terminal')}
        <section class="terminal-launchpad" aria-label="Terminal dock" use:sourceDockviewPanelAction={'terminal'}>
          <header class="terminal-launchpad-header">
            <div class="terminal-launchpad-identity">
              <span class="terminal-launchpad-glyph" aria-hidden="true">
                <Terminal size={14} strokeWidth={2} />
              </span>
              <strong>Terminal</strong>
              <span class="terminal-launchpad-status" title={terminalDockSummary()}>
                {#each terminalDockSummary().split(' · ') as statusSegment, statusIndex (statusIndex)}
                  <Chip size="xs" tone={/\b(embedded live|active)\b/.test(statusSegment) ? 'live' : 'muted'}>
                    {statusSegment}
                  </Chip>
                {/each}
              </span>
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
                onclick={attachOrStartProjectEmbeddedTerminal}
              >
                <Terminal size={13} strokeWidth={2} />
                <span>{embeddedTerminalStarting ? 'Starting' : 'Open'}</span>
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
                onclick={scheduleEmbeddedTerminalFit}
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
            <div class="embedded-terminal-toolbar" title={embeddedTerminalStatusLabel()}>
              <span class="embedded-terminal-meta">{embeddedTerminalStatusLabel()}</span>
              {#if embeddedTerminalSession?.pid}
                <code>pid {embeddedTerminalSession.pid}</code>
              {/if}
            </div>
            <div class="embedded-terminal-host" bind:this={embeddedTerminalElement}></div>
            {#if embeddedTerminalError}
              <div class="embedded-terminal-error">{embeddedTerminalError}</div>
            {/if}
          </div>

        </section>
      {/if}

      {#if sourceDockPanelVisible('browser')}
        <BrowserPanel
          url={activeBrowserUrl}
          bind:inputUrl={browserInputUrl}
          frameKey={browserFrameKey}
          error={browserError}
          runtimeContexts={selectedProjectRuntimeContexts}
          {runtimeContextUrl}
          panelAction={sourceDockviewPanelAction}
          onSubmit={submitBrowserUrl}
          onReload={reloadBrowserFrame}
          onOpenExternal={openBrowserUrlExternal}
          onHide={() => hideDockPanel('browser')}
          onOpenRuntimeContext={openRuntimeContextInBrowserDock}
        />
      {/if}
      </div>
      </div>
    </div>
  </section>
  </SourceDockviewShell>

  {#if useUnifiedWorkbench}
    <SourceWorkbench
      layout={sourceDockLayout}
      bind:setPanelElement={unifiedWorkbenchSetPanelElement}
      bind:ready={unifiedWorkbenchReady}
      bind:error={unifiedWorkbenchError}
      onPanelOwnershipChange={handleUnifiedWorkbenchOwnershipChange}
    />
  {/if}

  {#if sourceDockviewWorkbenchEnabled && hiddenDockPanelIDs().length > 0}
    <div class="hidden-dock-panel-rail workbench-hidden-dock-panel-rail" aria-label="Hidden dock panels">
      {#each hiddenDockPanelIDs() as panelID (panelID)}
        <button
          class="hidden-dock-panel-button"
          type="button"
          aria-label={`Restore ${dockPanelLabel(panelID)} panel`}
          title={`Restore ${dockPanelLabel(panelID)} panel`}
          onclick={() => restoreHiddenDockPanel(panelID)}
        >
          {#if panelID === 'activity'}
            <FolderGit2 size={13} strokeWidth={1.9} />
          {:else if panelID === 'context'}
            <Network size={13} strokeWidth={1.9} />
          {:else if panelID === 'insights'}
            <Search size={13} strokeWidth={1.9} />
          {:else if panelID === 'terminal'}
            <Terminal size={13} strokeWidth={1.9} />
          {:else}
            <ExternalLink size={13} strokeWidth={1.9} />
          {/if}
          <span>{dockPanelLabel(panelID)}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if sourceDockviewWorkbenchEnabled}
    <div class="workbench-global-controls">
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
        <div class="view-menu workbench-view-menu" role="menu" aria-label="View options">
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

        </div>
      {/if}
    </div>
  {/if}
</main>

<SettingsPanel bind:open={settingsOpen} />

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
  <CommandPaletteOverlay
    visible={commandPaletteVisible}
    bind:query={commandPaletteQuery}
    bind:index={commandPaletteIndex}
    commands={commandPaletteResults}
    onKeydown={handleCommandPaletteKeydown}
    onSelect={runCommandPaletteItem}
    onClose={closeCommandPalette}
  />
{/if}

<style>
  .shell {
    --pane-resizer-size: 0px;
    --pane-resizer-hit-size: 10px;
    --bottom-dock-resizer-size: 8px;
    /* App/chrome font size. When the user has NOT changed it, --app-font-size is
       left unset and this falls back to --text-md — the exact size the shell
       already inherited from :root, so the default look is unchanged. When set,
       UI text scales to taste. */
    font-size: var(--app-font-size, var(--text-md));
    position: relative;
    display: grid;
    grid-template-columns: var(--side-pane-width) minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    gap: 0;
    width: 100vw;
    height: 100dvh;
    min-height: min(520px, 100dvh);
    margin: 0;
    overflow: hidden;
    border: 0;
    border-radius: 0;
    background: #191a21;
    box-shadow: none;
  }

  .shell.side-right {
    grid-template-columns: minmax(0, 1fr) var(--side-pane-width);
  }

  .shell.activity-hidden {
    grid-template-columns: minmax(0, 1fr);
  }

  .shell.side-right.activity-hidden {
    grid-template-columns: minmax(0, 1fr);
  }

  .source-dockview-activity-shell {
    grid-column: 1;
    grid-row: 1;
  }

  .shell.activity-rail-only {
    grid-template-columns: 56px minmax(0, 1fr);
  }

  .shell.side-right.activity-rail-only {
    grid-template-columns: minmax(0, 1fr) 56px;
  }

  .shell.activity-hidden .workspace {
    grid-column: 1;
    grid-row: 1;
  }

  .shell.side-right.activity-hidden .workspace {
    grid-column: 1;
    grid-row: 1;
  }

  .shell.side-right .workspace {
    grid-column: 1;
    grid-row: 1;
  }

  .shell.side-right .side-pane-resizer {
    right: var(--side-pane-width);
    left: auto;
    transform: translateX(50%);
  }

  .shell.side-right .source-dockview-activity-shell {
    grid-column: 2;
    grid-row: 1;
  }

  .shell.side-right .activity-shell {
    grid-template-columns: minmax(0, 1fr) 46px;
    grid-column: 2;
    grid-row: 1;
    border-right: 0;
    border-left: 0;
  }

  .shell.side-right .activity-rail {
    grid-column: 2;
    grid-row: 1;
    border-right: 0;
    border-left: 0;
  }

  .shell.side-right .sidebar {
    grid-column: 1;
    grid-row: 1;
  }

  .shell.activity-rail-only .activity-shell {
    grid-template-columns: 46px;
  }

  .shell.activity-rail-only .source-dockview-activity-shell {
    min-width: 0;
  }

  .shell.activity-rail-only .sidebar {
    display: none;
  }

  .shell.activity-rail-only.side-right .activity-shell {
    grid-template-columns: 46px;
  }

  .shell.activity-rail-only.side-right .activity-rail {
    grid-column: 1;
    border-left: 0;
  }

  .activity-shell {
    display: grid;
    grid-template-columns: 46px minmax(0, 1fr);
    min-width: 0;
    overflow: hidden;
    background: #191a21;
    border-right: 0;
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
    border-right: 0;
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

  /* Settings gear sits at the bottom of the rail, just above the collapse
     control. It claims the free space (margin-top:auto) so both it and the
     toggle that follows are anchored to the bottom. */
  .activity-rail button.activity-rail-settings {
    margin-top: auto;
  }

  .activity-rail button.activity-rail-toggle {
    margin-top: auto;
    color: #6fdfcf;
    border-color: rgba(92, 226, 207, 0.18);
    background: rgba(92, 226, 207, 0.055);
  }

  /* When the settings gear is present it already consumes the free space, so
     the toggle should not add a second auto-margin gap above itself. */
  .activity-rail button.activity-rail-settings + .activity-rail-toggle {
    margin-top: 0;
  }

  .activity-rail button.activity-rail-toggle:hover,
  .activity-rail button.activity-rail-toggle:focus-visible {
    color: #f2fffc;
    border-color: rgba(92, 226, 207, 0.38);
    background: rgba(92, 226, 207, 0.14);
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

  .activity-restore-button {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 24;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    max-width: 132px;
    padding: 0 9px;
    overflow: hidden;
    color: #d9f8f3;
    border: 1px solid rgba(92, 226, 207, 0.28);
    border-radius: 8px;
    background: rgba(18, 29, 28, 0.9);
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.22);
    font-size: 11px;
    font-weight: 780;
    cursor: pointer;
  }

  .activity-restore-button:hover,
  .activity-restore-button:focus-visible {
    outline: 0;
    border-color: rgba(92, 226, 207, 0.55);
    background: rgba(32, 75, 69, 0.86);
  }

  .activity-restore-button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .shell.side-right .activity-restore-button {
    right: 10px;
    left: auto;
  }

  .side-pane-resizer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--side-pane-width);
    z-index: 18;
    width: var(--pane-resizer-hit-size);
    min-width: 0;
    padding: 0;
    touch-action: none;
    cursor: col-resize;
    border: 0;
    background: transparent;
    transform: translateX(-50%);
  }

  .shell.activity-rail-only .side-pane-resizer {
    left: 56px;
  }

  .shell.side-right.activity-rail-only .side-pane-resizer {
    right: 56px;
    left: auto;
    transform: translateX(50%);
  }

  .side-pane-resizer::after {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 2px;
    height: 56px;
    border-radius: 999px;
    background: rgba(126, 240, 223, 0.58);
    box-shadow: 0 0 0 1px rgba(7, 21, 20, 0.46);
    content: "";
    opacity: 0;
    transform: translate(-50%, -50%);
    transition:
      background 140ms ease,
      height 140ms ease,
      opacity 140ms ease,
      width 140ms ease;
  }

  .context-pane-resizer::after {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 2px;
    height: 38px;
    border-radius: 999px;
    background: rgba(126, 240, 223, 0.46);
    box-shadow: 0 0 0 1px rgba(7, 21, 20, 0.46);
    content: "";
    opacity: 0;
    transform: translate(-50%, -50%);
    transition:
      background 140ms ease,
      height 140ms ease,
      opacity 140ms ease,
      width 140ms ease;
  }

  .side-pane-resizer:hover,
  .side-pane-resizer:focus-visible {
    outline: 0;
    background: transparent;
  }

  .side-pane-resizer:hover::after,
  .side-pane-resizer:focus-visible::after,
  .context-pane-resizer:hover::after,
  .context-pane-resizer:focus-visible::after,
  .bottom-dock-resizer:hover::after,
  .bottom-dock-resizer:focus-visible::after {
    background: rgba(223, 253, 248, 0.72);
    box-shadow: 0 0 0 1px rgba(7, 21, 20, 0.56), 0 0 12px rgba(92, 226, 207, 0.22);
    opacity: 1;
  }

  :global(body.resizing-source-pane) {
    cursor: col-resize;
    user-select: none;
  }

  :global(body.resizing-source-pane) .side-pane-resizer {
    background: transparent;
  }

  :global(body.resizing-source-pane) .side-pane-resizer::after {
    width: 3px;
    height: 72px;
    background: rgba(223, 253, 248, 0.86);
    opacity: 1;
  }

  .sidebar {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    min-width: 0;
    overflow: hidden;
    padding: 14px 12px;
    container-type: inline-size;
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
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    margin-bottom: 12px;
  }

  .source-browser-stack {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    gap: 8px;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .source-files-pane {
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .source-files-search-pane {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 8px;
  }

  .source-files-search-pane .global-search-panel {
    grid-template-rows: auto auto minmax(0, 1fr);
    min-height: 0;
    padding-bottom: 0;
    margin-bottom: 0;
    border-bottom: 0;
  }

  .source-files-search-pane .source-search-results {
    max-height: none;
  }

  .source-files-recent-pane {
    display: grid;
    min-height: 0;
  }

  .source-files-recent-pane .recent-panel {
    min-height: 0;
    padding-bottom: 0;
    margin-bottom: 0;
    border-bottom: 0;
    overflow: hidden;
  }

  .source-files-recent-pane .recent-list {
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-width: thin;
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

  .paste-cleanup-top {
    display: grid;
    gap: 7px;
    min-width: 0;
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

  .paste-cleanup-history {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .paste-history-chip {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 24px;
    flex: 1 1 132px;
    align-items: center;
    min-width: 0;
    overflow: hidden;
    border: 1px solid rgba(92, 226, 207, 0.12);
    border-radius: 7px;
    background: rgba(92, 226, 207, 0.045);
  }

  .paste-history-chip.reply {
    border-color: rgba(216, 170, 85, 0.13);
    background: rgba(216, 170, 85, 0.045);
  }

  .paste-history-restore {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 26px;
    padding: 0 7px;
    color: #cbd8d5;
    text-align: left;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .paste-history-restore span {
    color: #8fd8cf;
    font-size: 8px;
    font-weight: 900;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .paste-history-chip.reply .paste-history-restore span {
    color: #d8c385;
  }

  .paste-history-restore strong {
    min-width: 0;
    overflow: hidden;
    color: #dfe8e5;
    font-size: 10px;
    font-weight: 780;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .paste-history-copy,
  .paste-history-clear {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    color: #96a39f;
    border: 0;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }

  .paste-history-clear {
    border: 1px solid rgba(255, 255, 255, 0.075);
    background: rgba(255, 255, 255, 0.035);
  }

  .paste-history-restore:hover,
  .paste-history-restore:focus-visible,
  .paste-history-copy:hover,
  .paste-history-copy:focus-visible,
  .paste-history-clear:hover,
  .paste-history-clear:focus-visible {
    color: #e8f6f2;
    outline: 0;
    background: rgba(92, 226, 207, 0.11);
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

  .run-ingest-strip {
    display: grid;
    grid-template-columns: minmax(0, 1fr) repeat(3, 26px);
    align-items: center;
    gap: 5px;
    min-width: 0;
    padding: 5px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.12);
  }

  .run-ingest-strip input {
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    overflow: hidden;
    color: #cfd8d5;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 7px;
    outline: 0;
    background: rgba(0, 0, 0, 0.16);
    font: 10px/1.2 ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    text-overflow: ellipsis;
  }

  .run-ingest-strip input:focus {
    border-color: rgba(92, 226, 207, 0.36);
    box-shadow: 0 0 0 2px rgba(92, 226, 207, 0.08);
  }

  .run-ingest-strip button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    color: #95a29f;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .run-ingest-strip button:hover,
  .run-ingest-strip button:focus-visible {
    color: #e9f5f2;
    border-color: rgba(92, 226, 207, 0.36);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .run-ingest-strip button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .run-ingest-strip small {
    min-width: 0;
    overflow: hidden;
    grid-column: 1 / -1;
    color: #8fd8cf;
    font-size: 9px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .activity-git-panel {
    grid-template-rows: minmax(0, auto);
    gap: 8px;
  }

  .conversation-activity-list {
    align-content: stretch;
    grid-template-rows: minmax(0, 1fr);
    overflow: hidden;
  }

  .conversation-active-pane {
    display: grid;
    align-content: start;
    gap: 7px;
    min-width: 0;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-width: thin;
  }

  .conversation-activity-list .workspace-snapshot-section {
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-width: thin;
  }

  .activity-git-source-control {
    display: grid;
    gap: 7px;
    min-width: 0;
    padding: 7px;
    border: 1px solid rgba(92, 226, 207, 0.12);
    border-radius: 8px;
    background: rgba(92, 226, 207, 0.035);
  }

  .activity-git-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .activity-git-heading div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .activity-git-heading strong,
  .activity-git-heading small,
  .activity-git-status-heading strong,
  .activity-git-status-heading span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-git-heading strong {
    color: #eef6f3;
    font-size: 13px;
    font-weight: 860;
  }

  .activity-git-heading small {
    color: #8d9995;
    font-size: 9px;
    font-weight: 760;
  }

  .activity-git-refresh {
    display: grid;
    place-items: center;
    width: 25px;
    height: 25px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .activity-git-refresh:hover,
  .activity-git-refresh:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.36);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .activity-git-refresh:disabled {
    cursor: default;
    opacity: 0.48;
  }

  .activity-git-status-heading {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: #dfe8e5;
    font-size: 10px;
    font-weight: 850;
  }

  .activity-git-status-heading span {
    color: #8d9995;
    font-size: 9px;
    font-weight: 760;
  }

  .activity-git-secondary-section {
    display: grid;
    min-width: 0;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding-top: 5px;
  }

  .activity-git-secondary-section summary {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 25px;
    color: #cbd6d3;
    list-style: none;
    cursor: pointer;
    font-size: 10px;
    font-weight: 850;
  }

  .activity-git-secondary-section summary::-webkit-details-marker {
    display: none;
  }

  .activity-git-secondary-section summary::before {
    width: 0;
    height: 0;
    border-top: 4px solid transparent;
    border-bottom: 4px solid transparent;
    border-left: 5px solid rgba(174, 184, 181, 0.76);
    content: "";
    transition: transform 140ms ease, border-left-color 140ms ease;
  }

  .activity-git-secondary-section[open] summary::before {
    transform: rotate(90deg);
  }

  .activity-git-secondary-section summary span,
  .activity-git-secondary-section summary small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-git-secondary-section summary small {
    color: #7ff0df;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 9px;
    font-weight: 820;
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

  .workspace-snapshot-readiness {
    display: inline-flex;
    min-width: 0;
    overflow: hidden;
    color: #8c9a96;
    font-size: 8px;
    font-weight: 760;
    line-height: 1.15;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-snapshot-file-state {
    color: #6f7b78;
    font-size: 8px;
  }

  .workspace-snapshot-readiness.ready {
    color: #76e6cf;
  }

  .workspace-snapshot-readiness.warning {
    color: #d8aa55;
  }

  .workspace-snapshot-readiness.blocked {
    color: #ff9d8e;
  }

  .workspace-snapshot-readiness.neutral {
    color: #8c9a96;
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
  .activity-runtime-row {
    grid-template-columns: auto minmax(0, 1fr) auto;
  }

  .activity-worktree-row {
    grid-template-columns: auto minmax(0, 1fr) auto;
  }


  .agent-activity-row {
    grid-template-columns: auto minmax(0, 1fr) auto;
    grid-template-areas: "badge main actions";
    align-items: start;
    gap: 6px 8px;
    min-height: 76px;
    overflow: hidden;
    padding: 7px;
  }

  .agent-activity-row > .agent-provider-badge {
    grid-area: badge;
    align-self: start;
    max-width: 76px;
  }

  .agent-activity-main {
    grid-area: main;
    gap: 2px;
    align-content: center;
    min-width: 0;
    overflow: hidden;
  }

  .agent-activity-state {
    color: #73817c;
    font-size: 8px;
  }

  .agent-activity-row .workspace-snapshot-readiness,
  .agent-activity-row .agent-session-focus-lane {
    justify-self: start;
    max-width: 100%;
  }

  .agent-activity-row .agent-session-focus-lane {
    min-height: 16px;
    padding: 0 5px;
  }

  .agent-activity-row .agent-session-focus-lane strong {
    display: none;
  }

  .agent-activity-actions {
    grid-area: actions;
    justify-content: flex-end;
    max-width: 28px;
    overflow: visible;
    opacity: 0.46;
    transition: opacity 120ms ease;
  }

  .agent-activity-row:hover .agent-activity-actions,
  .agent-activity-row:focus-within .agent-activity-actions {
    opacity: 1;
  }

  .agent-activity-actions button {
    flex: 0 0 auto;
  }

  .activity-worktree-row {
    align-items: start;
    min-height: 58px;
    padding: 7px;
  }

  .activity-worktree-row .activity-row-actions {
    grid-column: 3;
    align-self: start;
    justify-content: flex-end;
    max-width: 28px;
    overflow: visible;
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

  .run-live-digest {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
  }


  .run-live-digest-chip {
    display: inline-grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 4px;
    max-width: 184px;
    min-width: 0;
    min-height: 20px;
    padding: 0 6px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.04);
  }


  .run-live-digest-chip em,
  .run-live-digest-chip strong {
    min-width: 0;
    overflow: hidden;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .run-live-digest-chip em {
    color: #8fbdb6;
    font-size: 8px;
    font-style: normal;
    font-weight: 900;
    text-transform: uppercase;
  }

  .run-live-digest-chip strong {
    color: #dce5e2;
    font-size: 9px;
    font-weight: 820;
  }

  .run-live-digest-chip.live {
    color: #76dfd1;
    border-color: rgba(92, 226, 207, 0.22);
    background: rgba(92, 226, 207, 0.08);
  }

  .run-live-digest-chip.good {
    color: #9bdfae;
    border-color: rgba(139, 220, 155, 0.2);
    background: rgba(139, 220, 155, 0.07);
  }

  .run-live-digest-chip.attention {
    color: #e8c47d;
    border-color: rgba(216, 170, 85, 0.24);
    background: rgba(216, 170, 85, 0.085);
  }

  .run-live-digest-chip.bad {
    color: #ffaaa5;
    border-color: rgba(255, 112, 112, 0.24);
    background: rgba(255, 112, 112, 0.08);
  }

  .run-live-digest-chip.idle {
    color: #a8b2af;
    border-color: rgba(255, 255, 255, 0.075);
    background: rgba(255, 255, 255, 0.035);
  }

  .run-live-digest-chip.live em,
  .run-live-digest-chip.live strong,
  .run-live-digest-chip.good em,
  .run-live-digest-chip.good strong,
  .run-live-digest-chip.attention em,
  .run-live-digest-chip.attention strong,
  .run-live-digest-chip.bad em,
  .run-live-digest-chip.bad strong,
  .run-live-digest-chip.idle em,
  .run-live-digest-chip.idle strong {
    color: inherit;
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


  .worktree-owner-chip {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    max-width: 96px;
    height: 17px;
    padding: 0 6px;
    overflow: hidden;
    color: #aeb9b6;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 8px;
    font-weight: 850;
    line-height: 17px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .worktree-owner-chip.live {
    color: #071b18;
    border-color: rgba(92, 226, 207, 0.42);
    background: #67dfd1;
  }

  .worktree-owner-chip.saved {
    color: #8fe7dc;
    border-color: rgba(92, 226, 207, 0.18);
    background: rgba(92, 226, 207, 0.08);
  }

  .worktree-owner-chip.warning {
    color: #e8c47d;
    border-color: rgba(216, 170, 85, 0.24);
    background: rgba(216, 170, 85, 0.08);
  }

  .worktree-owner-chip.muted {
    color: #8d9995;
  }


  .activity-worktree-runbook {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 26px;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 5px 6px;
    border: 1px solid rgba(92, 226, 207, 0.14);
    border-radius: 9px;
    background: rgba(92, 226, 207, 0.045);
  }

  .activity-worktree-runbook button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    color: #9feadf;
    border: 1px solid rgba(92, 226, 207, 0.18);
    border-radius: 7px;
    background: rgba(8, 12, 11, 0.44);
    cursor: pointer;
  }

  .activity-worktree-runbook button:hover {
    color: #071b18;
    border-color: rgba(92, 226, 207, 0.55);
    background: #67dfd1;
  }


  .worktree-snapshot-chip {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 4px;
    justify-self: start;
    max-width: 126px;
    height: 18px;
    min-width: 0;
    padding: 0 6px;
    color: #8fe7dc;
    border: 1px solid rgba(92, 226, 207, 0.18);
    border-radius: 999px;
    background: rgba(92, 226, 207, 0.08);
    font-size: 8px;
    font-weight: 820;
    cursor: pointer;
  }

  .worktree-snapshot-chip span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .worktree-snapshot-chip:hover,
  .worktree-snapshot-chip:focus-visible {
    color: #eafaf7;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.14);
  }


  .worktree-plan-lane {
    display: inline-grid;
    flex: 0 0 auto;
    place-items: center;
    height: 17px;
    min-width: 42px;
    padding: 0 6px;
    border: 1px solid rgba(255, 255, 255, 0.085);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 8px;
    font-weight: 880;
    letter-spacing: 0;
    line-height: 17px;
    text-transform: uppercase;
  }

  .worktree-plan-lane.safe-remove {
    color: #071b18;
    border-color: rgba(92, 226, 207, 0.42);
    background: #67dfd1;
  }

  .worktree-plan-lane.backup-first,
  .worktree-plan-lane.review-first,
  .worktree-plan-lane.review-prunable,
  .worktree-plan-lane.review-saved-workspace,
  .worktree-plan-lane.review-confirmation {
    color: #e8c47d;
    border-color: rgba(216, 170, 85, 0.24);
    background: rgba(216, 170, 85, 0.08);
  }

  .worktree-plan-lane.blocked-active-session,
  .worktree-plan-lane.blocked-locked {
    color: #ffbd9f;
    border-color: rgba(255, 142, 96, 0.24);
    background: rgba(255, 142, 96, 0.09);
  }

  .worktree-plan-lane.blocked-protected,
  .worktree-plan-lane.keep {
    color: #aeb9b6;
  }

  .worktree-recommendation {
    color: #78837f;
  }

  .activity-repo-row > small {
    grid-column: 1 / 4;
  }

  .activity-task-ledger-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
  }

  .activity-task-ledger-row.blocked {
    border-color: rgba(216, 170, 85, 0.28);
    background: rgba(216, 170, 85, 0.055);
  }

  .activity-task-ledger-row.ready {
    border-color: rgba(92, 226, 207, 0.24);
    background: rgba(92, 226, 207, 0.055);
  }

  .task-ledger-title {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .task-ledger-title > span {
    min-width: 0;
    overflow: hidden;
    color: #dce6e3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-task-ledger-chips {
    display: inline-flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 3px;
    max-width: 138px;
    min-width: 0;
  }

  .task-ledger-chip {
    max-width: 72px;
    height: 18px;
    padding: 0 6px;
    overflow: hidden;
    color: #9ba7a4;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 9px;
    font-weight: 820;
    line-height: 18px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .task-ledger-chip.blocked,
  .task-ledger-chip.active {
    color: #e6c170;
    border-color: rgba(216, 170, 85, 0.24);
    background: rgba(216, 170, 85, 0.08);
  }

  .task-ledger-chip.ready {
    color: #7ff0df;
    border-color: rgba(92, 226, 207, 0.26);
    background: rgba(92, 226, 207, 0.08);
  }

  .task-ledger-chip.stale {
    color: #7ff0df;
    border-color: rgba(92, 226, 207, 0.22);
    background: rgba(92, 226, 207, 0.065);
  }

  .task-ledger-chip.saved {
    color: #8fe7dc;
    border-color: rgba(92, 226, 207, 0.18);
    background: rgba(92, 226, 207, 0.055);
  }

  .task-ledger-chip.backup {
    color: #e6c170;
    border-color: rgba(216, 170, 85, 0.22);
    background: rgba(216, 170, 85, 0.075);
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

  .row-action-menu-anchor {
    position: relative;
    overflow: visible;
  }

  .row-action-menu {
    position: absolute;
    z-index: 14;
    top: calc(100% + 4px);
    right: 0;
    display: grid;
    width: 196px;
    min-width: 0;
    padding: 5px;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 8px;
    background: rgba(21, 24, 24, 0.98);
    box-shadow: 0 18px 44px rgba(0, 0, 0, 0.34);
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

  .row-action-menu button {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr);
    align-items: center;
    justify-content: stretch;
    gap: 7px;
    width: 100%;
    height: 26px;
    padding: 0 7px;
    color: #cbd3d1;
    border: 0;
    border-radius: 5px;
    background: transparent;
    font-size: 11px;
    font-weight: 730;
    text-align: left;
  }

  .row-action-menu-link {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr);
    align-items: center;
    justify-content: stretch;
    gap: 7px;
    width: 100%;
    height: 26px;
    padding: 0 7px;
    color: #cbd3d1;
    border-radius: 5px;
    font-size: 11px;
    font-weight: 730;
    text-align: left;
    text-decoration: none;
  }

  .row-action-menu button:hover:not(:disabled),
  .row-action-menu button:focus-visible,
  .row-action-menu-link:hover,
  .row-action-menu-link:focus-visible {
    color: #f2f6f5;
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .row-action-menu button span,
  .row-action-menu-link span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-panel-list .conversation-session-row:has(.row-action-menu),
  .activity-panel-list .agent-activity-row:has(.row-action-menu),
  .activity-panel-list .workspace-snapshot-row:has(.row-action-menu),
  .activity-panel-list .activity-runtime-row:has(.row-action-menu),
  .activity-panel-list .activity-worktree-row:has(.row-action-menu),
  .activity-panel-list .activity-repo-row:has(.row-action-menu),
  .activity-panel-list .activity-task-ledger-row:has(.row-action-menu),
  .activity-panel-list .activity-commit-row:has(.row-action-menu) {
    align-items: start;
  }

  .activity-panel-list .conversation-session-row .row-action-menu-anchor:has(.row-action-menu),
  .activity-panel-list .run-activity-actions:has(.row-action-menu),
  .activity-panel-list .workspace-snapshot-actions:has(.row-action-menu),
  .activity-panel-list .runtime-activity-actions:has(.row-action-menu),
  .activity-panel-list .agent-activity-actions:has(.row-action-menu),
  .activity-panel-list .worktree-activity-actions:has(.row-action-menu),
  .activity-panel-list .repository-activity-actions:has(.row-action-menu),
  .activity-panel-list .task-ledger-actions:has(.row-action-menu),
  .activity-panel-list .activity-commit-meta:has(.row-action-menu),
  .activity-panel-list .commit-activity-actions:has(.row-action-menu) {
    display: contents;
  }

  .activity-panel-list .conversation-session-row .row-action-menu-anchor:has(.row-action-menu) > button,
  .activity-panel-list .run-activity-actions:has(.row-action-menu) > button,
  .activity-panel-list .workspace-snapshot-actions:has(.row-action-menu) > button,
  .activity-panel-list .runtime-activity-actions:has(.row-action-menu) > button,
  .activity-panel-list .worktree-activity-actions:has(.row-action-menu) > button,
  .activity-panel-list .repository-activity-actions:has(.row-action-menu) > button,
  .activity-panel-list .task-ledger-actions:has(.row-action-menu) > button,
  .activity-panel-list .commit-activity-actions:has(.row-action-menu) > button {
    justify-self: end;
  }

  .activity-panel-list .run-activity-actions:has(.row-action-menu) > button {
    justify-self: end;
  }


  .activity-panel-list .agent-activity-actions:has(.row-action-menu) > button {
    grid-area: actions;
    justify-self: end;
  }

  .activity-panel-list .workspace-snapshot-actions:has(.row-action-menu) > button,
  .activity-panel-list .runtime-activity-actions:has(.row-action-menu) > button,
  .activity-panel-list .commit-activity-actions:has(.row-action-menu) > button {
    grid-column: 3;
    grid-row: 1;
  }

  .activity-panel-list .workspace-snapshot-actions:has(.row-action-menu) > button {
    grid-column: 2;
  }

  .activity-panel-list .worktree-activity-actions:has(.row-action-menu) > button,
  .activity-panel-list .task-ledger-actions:has(.row-action-menu) > button {
    grid-column: 3;
    grid-row: 1;
  }

  .activity-panel-list .repository-activity-actions:has(.row-action-menu) > button {
    grid-column: 4;
    grid-row: 1;
  }

  .activity-panel-list .row-action-menu {
    position: static;
    grid-column: 1 / -1;
    width: 100%;
    margin-top: -1px;
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
    gap: var(--space-2);
    min-width: 0;
    max-width: 100%;
    margin-bottom: var(--space-3);
  }

  select,
  .scan-button,
  .icon-button,
  .form-button {
    height: 36px;
    min-width: 0;
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  select {
    width: 100%;
    padding: 0 var(--space-3);
    outline: 0;
  }

  select:focus,
  .scan-button:focus-visible,
  .icon-button:focus-visible,
  .form-button:focus-visible {
    border-color: var(--color-focus);
    box-shadow: var(--focus-ring);
  }

  .scan-button {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    color: var(--color-text-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
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
    color: var(--color-text-2);
    cursor: pointer;
  }

  .icon-button:hover {
    color: var(--color-text);
    background: var(--color-live-bg);
  }

  .icon-button.danger {
    width: 28px;
    height: 28px;
    color: var(--color-bad);
    border-radius: var(--radius-md);
  }

  .icon-button.danger:hover {
    background: var(--color-bad-bg);
  }

  .project-path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-2);
    min-height: 28px;
    margin-bottom: var(--space-1);
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .project-path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .project-setup-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    min-height: 18px;
    margin: var(--space-1) 0 var(--space-2);
    line-height: 1.25;
  }

  .scan-progress {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    min-height: 18px;
    margin: var(--space-1) 0;
    color: var(--color-live);
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
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

  @container (max-width: 330px) {
    .sidebar {
      padding: 10px 7px;
    }

    .brand-row {
      grid-template-columns: 28px minmax(0, 1fr);
      gap: 7px;
      margin-bottom: 10px;
    }

    .brand-mark {
      width: 28px;
      height: 28px;
      border-radius: 9px;
    }

    .eyebrow {
      margin-bottom: 1px;
      font-size: 9px;
    }

    h1 {
      font-size: 14px;
    }

    .project-controls {
      margin-bottom: 8px;
    }

    .project-row {
      grid-template-columns: minmax(74px, 1fr) 30px 30px 34px;
      gap: 5px;
      margin-bottom: 6px;
    }

    select,
    .scan-button,
    .icon-button,
    .form-button {
      height: 30px;
      border-radius: 8px;
    }

    select {
      padding: 0 8px;
    }

    .icon-button {
      width: 30px;
    }

    .scan-button {
      grid-template-columns: 1fr;
      place-items: center;
      gap: 0;
      padding: 0;
    }

    .scan-button span {
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

    .project-path-row,
    .project-setup-row {
      font-size: 9px;
    }

    .search-box {
      grid-template-columns: 15px minmax(0, 1fr);
      gap: 6px;
      height: 32px;
      padding: 0 8px;
      margin-bottom: 8px;
    }

    .global-search-panel {
      gap: 5px;
      padding-bottom: 8px;
      margin-bottom: 8px;
    }

    .global-search-box {
      grid-template-columns: 15px minmax(0, 1fr) 24px;
      gap: 5px;
      height: 30px;
      padding: 0 3px 0 8px;
    }

    .source-search-submit {
      width: 22px;
      height: 22px;
      border-radius: 6px;
    }
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
    gap: 6px;
    padding-bottom: 8px;
    margin-bottom: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .source-sidebar-section {
    min-width: 0;
  }

  .source-sidebar-section.collapsed {
    flex: 0 0 auto;
  }

  .source-section-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 22px;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .source-section-header {
    display: grid;
    grid-template-columns: 14px 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-width: 0;
    height: 26px;
    padding: 0 5px;
    color: #aeb8b5;
    text-align: left;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }

  .source-section-header:hover,
  .source-section-header:focus-visible {
    color: #eef6f3;
    outline: 0;
    background: rgba(255, 255, 255, 0.055);
  }

  .source-section-header svg {
    min-width: 0;
    color: #8d9995;
  }

  .source-section-header svg:nth-of-type(2) {
    color: #6fdfcf;
  }

  .source-section-header span,
  .source-section-header small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-section-header small {
    color: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 820;
  }

  .source-section-title {
    grid-template-columns: 18px minmax(0, 1fr);
    cursor: default;
  }

  .source-section-title:hover,
  .source-section-title:focus-visible {
    color: #aeb8b5;
    background: transparent;
  }

  .recent-file-icon {
    display: grid;
    place-items: center;
    min-width: 0;
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

  .source-list-panel.collapsed {
    grid-template-rows: auto;
    flex: 0 0 auto;
  }

  .tree-panel-header {
    min-width: 0;
  }

  .tree-heading {
    height: 27px;
  }

  .scan-summary-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 22px;
    align-items: center;
    gap: 5px;
    margin: -3px 0 7px;
  }

  .scan-summary {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    color: #7f8b87;
    font-size: 10px;
    font-weight: 720;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-summary .index-summary,
  .scan-summary .scan-evidence {
    min-width: 0;
    margin: 0;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-summary .index-summary {
    flex: 0 1 auto;
    max-width: min(42%, 150px);
  }

  .scan-summary .scan-evidence {
    flex: 1 1 0;
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
    margin: -4px 0 4px;
    overflow: hidden;
    color: #8fd8cf;
    font-size: 10px;
    font-weight: 760;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .index-summary[data-tone='warning'] {
    color: #d8cba8;
  }

  .index-summary[data-tone='error'] {
    color: #ff9a9a;
  }

  .index-summary[data-tone='muted'] {
    color: #8a9693;
  }

  .scan-evidence {
    min-width: 0;
    margin: 0 0 7px;
    overflow: hidden;
    color: #6f7b78;
    font-size: 9.5px;
    font-weight: 720;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-evidence[data-tone='active'] {
    color: #8fd8cf;
  }

  .scan-evidence[data-tone='warning'] {
    color: #d8cba8;
  }

  .scan-evidence[data-tone='error'] {
    color: #ff9a9a;
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

  .scan-runtime-note {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    margin: -1px 0 8px;
    border: 1px solid rgba(143, 216, 207, 0.2);
    border-radius: 6px;
    padding: 6px 7px;
    background: rgba(143, 216, 207, 0.07);
    color: #a8c8c3;
    font-size: 10px;
    font-weight: 720;
    line-height: 1.25;
  }

  .scan-runtime-note span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-runtime-note button {
    height: 20px;
    border: 1px solid rgba(143, 216, 207, 0.24);
    border-radius: 5px;
    padding: 0 7px;
    background: rgba(143, 216, 207, 0.1);
    color: #bce8e2;
    font-size: 10px;
    font-weight: 820;
  }

  .scan-root-correction {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    margin: -1px 0 8px;
    border: 1px solid rgba(216, 170, 85, 0.22);
    border-radius: 6px;
    padding: 5px 7px;
    background: rgba(216, 170, 85, 0.07);
    color: #d8cba8;
    font-size: 10px;
    font-weight: 720;
    line-height: 1.25;
  }

  .scan-root-correction span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-root-correction strong {
    margin-right: 5px;
    color: #f0c978;
    font-weight: 840;
  }

  .scan-root-correction button {
    height: 20px;
    border: 1px solid rgba(216, 170, 85, 0.28);
    border-radius: 5px;
    padding: 0 7px;
    background: rgba(216, 170, 85, 0.12);
    color: #f1d99b;
    font-size: 10px;
    font-weight: 820;
  }

  .scan-root-correction button:hover:not(:disabled) {
    border-color: rgba(216, 170, 85, 0.44);
    background: rgba(216, 170, 85, 0.18);
    color: #ffe5a8;
  }

  .scan-root-correction button:disabled {
    opacity: 0.48;
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

  .scan-recovery-panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    margin: -2px 0 8px;
    border: 1px solid rgba(111, 223, 207, 0.18);
    border-radius: 6px;
    padding: 6px 7px;
    background: rgba(111, 223, 207, 0.07);
    color: #a8c8c3;
    font-size: 10px;
    font-weight: 690;
    line-height: 1.25;
  }

  .scan-recovery-panel > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scan-recovery-panel strong {
    margin-right: 6px;
    color: #6fdfcf;
    font-weight: 820;
  }

  .scan-recovery-actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .scan-recovery-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 21px;
    height: 20px;
    border: 1px solid rgba(143, 216, 207, 0.18);
    border-radius: 5px;
    padding: 0;
    background: rgba(143, 216, 207, 0.08);
    color: #9fc5bf;
  }

  .scan-recovery-actions button:hover:not(:disabled) {
    border-color: rgba(143, 216, 207, 0.38);
    background: rgba(143, 216, 207, 0.14);
    color: #d6f2ed;
  }

  .scan-recovery-actions button:disabled {
    opacity: 0.44;
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
    position: relative;
    display: flex;
    flex-direction: column;
    grid-column: 2;
    grid-row: 1;
    align-self: stretch;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 9px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent 36%),
      rgba(24, 26, 26, 0.96);
  }

  .workspace.chrome-compact {
    padding: 6px;
  }

  .topbar {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) auto;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
  }

  .workspace.chrome-compact .topbar {
    gap: 6px;
    min-height: 26px;
    margin-bottom: 3px;
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

  .workspace.chrome-compact .topbar h2 {
    font-size: 13px;
    line-height: 1.05;
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

  .workspace.chrome-compact .topbar-command-button {
    width: 26px;
    height: 24px;
    padding: 0;
    gap: 0;
  }

  .workspace.chrome-compact .topbar-command-button span {
    display: none;
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

  .view-menu-button-grid button.active,
  .view-menu-wide-button.active {
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

  .workspace.chrome-compact .context-identity-strip {
    display: none;
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

  .workspace.chrome-compact .dock-panel-tabs {
    min-height: 19px;
    margin-bottom: 3px;
  }

  .dock-panel-tabs.empty {
    height: 0;
    min-height: 0;
    margin: 0;
  }

  .workspace.chrome-compact .dock-panel-tabs.empty {
    height: 0;
    min-height: 0;
    margin: 0;
  }

  .hidden-dock-panel-rail {
    position: absolute;
    top: 38px;
    right: 7px;
    z-index: 18;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    min-width: 0;
    max-height: calc(100% - 76px);
    padding: 3px;
    overflow: auto;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 9px;
    background: rgba(12, 16, 16, 0.86);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.24);
    backdrop-filter: blur(14px);
  }

  .hidden-dock-panel-button {
    display: inline-grid;
    place-items: center;
    grid-template-columns: 20px;
    min-width: 0;
    width: 26px;
    height: 26px;
    padding: 0;
    overflow: hidden;
    color: #aeb8b5;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.04);
    font: inherit;
    cursor: pointer;
  }

  .hidden-dock-panel-button span {
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

  .hidden-dock-panel-button:hover,
  .hidden-dock-panel-button:focus-visible {
    color: #dffdf8;
    border-color: rgba(92, 226, 207, 0.32);
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .workbench-global-controls {
    position: absolute;
    top: 7px;
    right: 8px;
    z-index: 36;
    display: inline-grid;
    place-items: center;
    min-width: 0;
  }

  .workbench-global-controls > .topbar-command-button {
    height: 25px;
    min-width: 32px;
    background: rgba(18, 20, 21, 0.9);
    backdrop-filter: blur(14px);
  }

  .workbench-view-menu {
    z-index: 37;
  }

  .workbench-hidden-dock-panel-rail {
    top: 40px;
    right: 8px;
    z-index: 35;
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

  .workspace.chrome-compact .dock-panel-tab {
    height: 16px;
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

  .workspace.chrome-compact .dock-panel-tab-label {
    height: 16px;
    padding: 0 5px;
    font-size: 8.5px;
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

  .dock-panel-tab.drop-target {
    background: rgba(92, 226, 207, 0.09);
  }

  .dock-panel-tab.drop-target:not(.drop-after) {
    box-shadow: inset 2px 0 0 #5ce2cf;
  }

  .dock-panel-tab.drop-target.drop-after {
    box-shadow: inset -2px 0 0 #5ce2cf;
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
    position: relative;
    display: grid;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    grid-template-rows: minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .workspace-arrangement.context-top {
    grid-template-rows: auto minmax(0, 1fr);
  }

  .workspace-arrangement.context-side {
    grid-template-columns: minmax(0, 1fr) var(--context-pane-width);
    grid-template-rows: minmax(0, 1fr);
    gap: 0;
  }

  .workspace-arrangement.context-side.context-rail-only {
    grid-template-columns: minmax(0, 1fr) 34px;
  }

  .workspace-arrangement.context-bottom {
    grid-template-rows: minmax(0, 1fr) minmax(96px, var(--context-pane-height));
    gap: 0;
  }

  .workspace-context-column,
  .workspace-main-column {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }


  .workspace-main-column {
    display: flex;
    flex-direction: column;
  }

  .source-runtime-panel-stage {
    position: fixed;
    left: -1px;
    bottom: -1px;
    width: 1px;
    height: 1px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    visibility: hidden;
    opacity: 0;
    pointer-events: none;
    contain: layout paint;
  }

  .workspace-arrangement.context-side .workspace-main-column {
    grid-column: 1;
    grid-row: 1;
  }


  .workspace-arrangement.context-bottom .workspace-main-column {
    grid-column: 1;
    grid-row: 1;
  }


  .context-pane-resizer {
    position: absolute;
    display: none;
    z-index: 18;
    width: var(--pane-resizer-hit-size);
    min-width: 0;
    padding: 0;
    touch-action: none;
    cursor: col-resize;
    border: 0;
    background: transparent;
  }

  .workspace-arrangement.context-side .context-pane-resizer {
    display: block;
    top: 0;
    right: var(--context-pane-width);
    bottom: 0;
    transform: translateX(50%);
  }

  .workspace-arrangement.context-side.context-rail-only .context-pane-resizer {
    right: 34px;
  }

  .workspace-arrangement.context-bottom .context-pane-resizer {
    display: block;
    right: 0;
    bottom: var(--context-pane-height);
    left: 0;
    width: auto;
    height: var(--pane-resizer-hit-size);
    cursor: row-resize;
    border: 0;
    transform: translateY(50%);
  }

  .context-pane-resizer:hover,
  .context-pane-resizer:focus-visible {
    outline: 0;
    background: transparent;
  }

  .workspace-arrangement.context-bottom .context-pane-resizer::after {
    width: 42px;
    height: 2px;
  }

  :global(body.resizing-context-pane) {
    cursor: col-resize;
    user-select: none;
  }

  :global(body.resizing-context-pane) .context-pane-resizer::after,
  :global(body.resizing-context-pane-bottom) .context-pane-resizer::after {
    background: rgba(223, 253, 248, 0.82);
    opacity: 1;
  }

  :global(body.resizing-context-pane-bottom) {
    cursor: row-resize;
    user-select: none;
  }

  .bottom-dock-resizer {
    position: relative;
    flex: 0 0 var(--bottom-dock-resizer-size);
    width: auto;
    height: var(--bottom-dock-resizer-size);
    min-height: var(--bottom-dock-resizer-size);
    margin: 5px 0 0;
    padding: 0;
    touch-action: none;
    cursor: row-resize;
    border: 0;
    border-top: 1px solid rgba(92, 226, 207, 0.1);
    border-bottom: 1px solid rgba(92, 226, 207, 0.1);
    background: rgba(92, 226, 207, 0.055);
  }

  .bottom-dock-resizer:hover,
  .bottom-dock-resizer:focus-visible {
    outline: 0;
    background: rgba(92, 226, 207, 0.2);
  }

  .bottom-dock-resizer::after {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 68px;
    height: 3px;
    border-radius: 999px;
    background: rgba(126, 240, 223, 0.58);
    box-shadow: 0 0 0 1px rgba(7, 21, 20, 0.46);
    content: "";
    transform: translate(-50%, -50%);
    transition: background 140ms ease, width 140ms ease;
  }

  :global(body.resizing-bottom-dock) {
    cursor: row-resize;
    user-select: none;
  }

  :global(body.resizing-bottom-dock) .bottom-dock-resizer {
    background: rgba(92, 226, 207, 0.24);
  }

  :global(body.resizing-bottom-dock) .bottom-dock-resizer::after {
    width: 96px;
    height: 4px;
    background: rgba(223, 253, 248, 0.86);
  }


  :global(.source-dockview-context-cards-host) {
    grid-area: 1 / 1;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }


  :global(.source-dockview-files-shell .source-paneview-panel-host),
  :global(.source-dockview-conversation-shell .source-paneview-panel-host) {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    align-items: stretch;
    justify-items: stretch;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }


  .workspace-arrangement.context-side .agent-session-focus-lane {
    grid-area: lane;
    width: 100%;
    min-height: 16px;
    padding: 0 4px;
  }


  .workspace-arrangement.context-side .worktree-context-row > span:not(.worktree-status-badge):not(.worktree-decision-lane),
  .workspace-arrangement.context-side .worktree-context-row > em,
  .workspace-arrangement.context-side .worktree-context-row > .git-task-link,
  .workspace-arrangement.context-side .worktree-task-empty,
  .workspace-arrangement.context-side .worktree-recommendation,
  .workspace-arrangement.context-side .worktree-next-check {
    display: none;
  }


  .workspace-arrangement.context-side .orchestration-context-list,
  .workspace-arrangement.context-side .orchestration-decision-queue,
  .workspace-arrangement.context-side .runtime-context-list,
  .workspace-arrangement.context-side .agent-session-list,
  .workspace-arrangement.context-side .worktree-decision-queue,
  .workspace-arrangement.context-side .worktree-context-list,
  .workspace-arrangement.context-side .repo-dashboard-list {
    align-content: start;
    align-items: start;
    grid-auto-rows: max-content;
    max-height: none;
  }


  .workspace-arrangement.context-side .context-loop-stage {
    height: 16px;
    padding: 0 4px;
    font-size: 9px;
  }

  .workspace-arrangement.context-side .runtime-port,
  .workspace-arrangement.context-side .runtime-url-link,
  .workspace-arrangement.context-side .agent-provider-badge,
  .workspace-arrangement.context-side .worktree-status-badge,
  .workspace-arrangement.context-side .worktree-decision-lane,
  .workspace-arrangement.context-side .repo-branch-badge,
  .workspace-arrangement.context-side .repo-task-link {
    min-height: 0;
    height: 16px;
    padding: 0 5px;
    font-size: 9px;
  }

  .workspace-arrangement.context-side .runtime-url-link,
  .workspace-arrangement.context-side .repo-task-link {
    justify-self: start;
  }


  .orchestration-decision-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 5px;
    min-width: 0;
    min-height: 28px;
    padding: 4px 5px;
    border: 1px solid rgba(216, 170, 85, 0.2);
    border-radius: 5px;
    background: rgba(216, 170, 85, 0.075);
  }

  .orchestration-decision-item.bad {
    border-color: rgba(255, 112, 112, 0.24);
    background: rgba(255, 112, 112, 0.08);
  }

  .orchestration-decision-item > span {
    min-width: 0;
    padding: 0 5px;
    color: #20170a;
    border-radius: 999px;
    background: #d8aa55;
    font-size: 8px;
    font-weight: 900;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .orchestration-decision-item.bad > span {
    color: #220a0a;
    background: #ff8f8f;
  }

  .orchestration-decision-item button,
  .orchestration-decision-item a {
    min-width: 0;
    border: 0;
    background: transparent;
  }

  .orchestration-decision-item button {
    display: grid;
    gap: 1px;
    padding: 0;
    color: #f3ebe0;
    text-align: left;
    cursor: pointer;
  }

  .orchestration-decision-item button:disabled {
    cursor: default;
    opacity: 0.56;
  }

  .orchestration-decision-item button:not(:disabled):hover strong,
  .orchestration-decision-item button:not(:disabled):focus-visible strong {
    color: #f4d08b;
  }

  .orchestration-decision-item button:not(:disabled):focus-visible {
    outline: 1px solid rgba(216, 170, 85, 0.34);
    outline-offset: 2px;
  }

  .orchestration-decision-item strong,
  .orchestration-decision-item small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .orchestration-decision-item strong {
    font-size: 10px;
    font-weight: 850;
  }

  .orchestration-decision-item small {
    color: #b9ada0;
    font-size: 9px;
    font-weight: 760;
  }

  .orchestration-decision-item > button:not(:first-of-type),
  .orchestration-decision-item a {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    color: #d8aa55;
    border-radius: 5px;
    text-decoration: none;
  }

  .orchestration-decision-item > button:not(:first-of-type):hover,
  .orchestration-decision-item > button:not(:first-of-type):focus-visible,
  .orchestration-decision-item a:hover,
  .orchestration-decision-item a:focus-visible {
    color: #f4d08b;
    outline: 0;
    background: rgba(216, 170, 85, 0.14);
  }

  .worktree-decision-queue {
    display: grid;
    gap: 4px;
    max-height: 132px;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.48) rgba(255, 255, 255, 0.045);
    scrollbar-width: thin;
  }

  .worktree-decision-queue:has(.row-action-menu) {
    max-height: 180px;
  }

  .worktree-decision-group {
    display: grid;
    gap: 3px;
    min-width: 0;
    padding: 4px;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.12);
  }

  .worktree-decision-group.blocked {
    border-color: rgba(216, 170, 85, 0.2);
    background: rgba(216, 170, 85, 0.07);
  }

  .worktree-decision-group.ready {
    border-color: rgba(92, 226, 207, 0.18);
    background: rgba(92, 226, 207, 0.055);
  }

  .worktree-decision-group.review,
  .worktree-decision-group.protected {
    background: rgba(255, 255, 255, 0.032);
  }


  .worktree-decision-group-header div,
  .worktree-decision-item div {
    display: grid;
    gap: 1px;
    min-width: 0;
  }

  .worktree-decision-group-header strong,
  .worktree-decision-group-header small,
  .worktree-decision-group-header span,
  .worktree-decision-item strong,
  .worktree-decision-item small,
  .worktree-decision-more {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }


  .worktree-decision-items {
    display: grid;
    gap: 3px;
    min-width: 0;
    overflow: visible;
  }

  .worktree-decision-item {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 4px;
    min-width: 0;
    min-height: 24px;
    overflow: visible;
    padding: 3px 4px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.16);
  }

  .worktree-decision-item strong {
    color: #edf4f2;
    font-size: 10px;
    font-weight: 800;
  }

  .worktree-decision-item small,
  .worktree-decision-more {
    color: #8d9995;
    font-size: 9px;
    font-weight: 720;
  }

  .worktree-decision-item button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    color: #91a19d;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.035);
    cursor: pointer;
  }

  .worktree-decision-item .row-action-menu button {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr);
    place-items: initial;
    align-items: center;
    justify-content: stretch;
    width: 100%;
    height: 26px;
    padding: 0 7px;
    color: #cbd3d1;
    border: 0;
    border-radius: 5px;
    background: transparent;
    font-size: 11px;
    font-weight: 730;
    text-align: left;
  }

  .worktree-decision-item .row-action-menu {
    position: static;
    grid-column: 1 / -1;
    width: 100%;
    margin-top: 2px;
  }

  .worktree-decision-item button.worktree-primary-action.backup {
    color: #d8aa55;
    border-color: rgba(216, 170, 85, 0.22);
    background: rgba(216, 170, 85, 0.09);
  }

  .worktree-decision-item button.worktree-primary-action.cleanup {
    color: #79eadb;
    border-color: rgba(92, 226, 207, 0.3);
    background: rgba(92, 226, 207, 0.11);
  }

  .worktree-decision-item button:hover,
  .worktree-decision-item button:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.36);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .worktree-decision-item button:disabled {
    opacity: 0.38;
    cursor: default;
  }


  .context-loop-stage {
    display: inline-grid;
    grid-template-columns: minmax(0, auto) auto;
    align-items: center;
    gap: 3px;
    min-width: 0;
    height: 18px;
    padding: 0 5px;
    color: #aeb9b6;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 8px;
    font-weight: 850;
  }

  .context-loop-stage.live {
    color: #6fdfcf;
    border-color: rgba(92, 226, 207, 0.2);
    background: rgba(92, 226, 207, 0.075);
  }

  .context-loop-stage.good {
    color: #8fd8a4;
    border-color: rgba(123, 216, 159, 0.18);
    background: rgba(123, 216, 159, 0.065);
  }

  .context-loop-stage.attention {
    color: #e8c47d;
    border-color: rgba(216, 170, 85, 0.22);
    background: rgba(216, 170, 85, 0.085);
  }

  .context-loop-stage.bad {
    color: #ff9d9d;
    border-color: rgba(255, 112, 112, 0.22);
    background: rgba(255, 112, 112, 0.08);
  }

  .context-loop-stage em,
  .context-loop-stage strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .context-loop-stage em {
    font-style: normal;
    opacity: 0.82;
  }

  .context-loop-stage strong {
    color: inherit;
    font-size: 9px;
    font-weight: 900;
  }


  .runtime-port,
  .orchestration-context-main,
  .orchestration-context-title,
  .orchestration-context-title .git-task-link,
  .orchestration-context-meta,
  .orchestration-context-meta span,
  .orchestration-context-current,
  .orchestration-context-row strong,
  .orchestration-context-row span,
  .orchestration-context-row small,
  .agent-provider-badge,
  .worktree-status-badge,
  .worktree-decision-lane,
  .worktree-plan-lane,
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


  .agent-provider-badge {
    display: inline-grid;
    place-items: center;
    height: 20px;
    max-width: 64px;
    min-width: 0;
    overflow: hidden;
    padding: 0 7px;
    color: #081916;
    border-radius: 999px;
    background: #81d6e4;
    font-size: 10px;
    font-weight: 820;
    text-overflow: ellipsis;
    text-transform: capitalize;
    white-space: nowrap;
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

  .worktree-decision-lane {
    display: inline-grid;
    place-items: center;
    min-width: 0;
    min-height: 18px;
    max-width: 92px;
    padding: 0 6px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.045);
    font-size: 8px;
    font-weight: 900;
    line-height: 1;
    text-transform: uppercase;
  }

  .worktree-decision-lane.blocked {
    color: #ffd8a8;
    border-color: rgba(216, 170, 85, 0.24);
    background: rgba(216, 170, 85, 0.1);
  }

  .worktree-decision-lane.backup {
    color: #f0c979;
    border-color: rgba(216, 170, 85, 0.24);
    background: rgba(216, 170, 85, 0.08);
  }

  .worktree-decision-lane.cleanup {
    color: #7ce5d5;
    border-color: rgba(92, 226, 207, 0.25);
    background: rgba(92, 226, 207, 0.08);
  }

  .worktree-decision-lane.review {
    color: #cbd3d1;
    border-color: rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.045);
  }

  .worktree-decision-lane.protected {
    color: #9fa9a6;
    border-color: rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.035);
  }


  .repo-branch-badge {
    color: #6fdfcf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 820;
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


  .agent-session-focus-lane {
    display: inline-grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 4px;
    max-width: 100%;
    min-width: 0;
    min-height: 18px;
    padding: 0 5px;
    overflow: hidden;
    color: #9fb4af;
    text-align: left;
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.035);
  }

  button.agent-session-focus-lane {
    width: 100%;
    font: inherit;
    cursor: pointer;
  }

  button.agent-session-focus-lane:hover,
  button.agent-session-focus-lane:focus-visible {
    color: #eaf5f2;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.12);
  }

  .agent-session-focus-lane em,
  .agent-session-focus-lane strong {
    min-width: 0;
    overflow: hidden;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .agent-session-focus-lane em {
    color: inherit;
    font-size: 8px;
    font-style: normal;
    font-weight: 900;
    text-transform: uppercase;
  }

  .agent-session-focus-lane strong {
    color: inherit;
    font-size: 8px;
    font-weight: 820;
  }

  .agent-session-focus-lane.ready {
    color: #76e6cf;
    border-color: rgba(92, 226, 207, 0.2);
    background: rgba(92, 226, 207, 0.075);
  }

  .agent-session-focus-lane.warning {
    color: #e1bd76;
    border-color: rgba(216, 170, 85, 0.22);
    background: rgba(216, 170, 85, 0.08);
  }

  .agent-session-focus-lane.blocked {
    color: #ffaaa5;
    border-color: rgba(255, 112, 112, 0.24);
    background: rgba(255, 112, 112, 0.08);
  }

  .agent-session-focus-lane.neutral {
    color: #9fb4af;
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
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    height: auto;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 8px;
    background: #17191e;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.07),
      0 18px 45px rgba(0, 0, 0, 0.2);
  }

  .source-editor-dock-panel {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .source-editor-file-pane {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .editor-file-placeholder {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 5px;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    color: #8f9a96;
    border: 0;
    background: #17191e;
    cursor: pointer;
  }

  .editor-file-placeholder strong,
  .editor-file-placeholder span,
  .editor-file-placeholder small {
    max-width: min(520px, 86%);
    overflow: hidden;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-placeholder strong {
    color: #f2f6f5;
    font-size: 13px;
    font-weight: 820;
  }

  .editor-file-placeholder span,
  .editor-file-placeholder small {
    font-size: 10px;
    font-weight: 760;
  }

  .editor-file-placeholder:hover,
  .editor-file-placeholder:focus-visible {
    color: #bff8ef;
    outline: 0;
  }

  :global(.source-dockview-workbench-shell .editor-frame),
  :global(.source-dockview-workbench-shell .empty-preview),
  :global(.source-dockview-workbench-shell .source-dockview-editor-files-shell),
  :global(.source-dockview-center-shell .editor-frame),
  :global(.source-dockview-center-shell .empty-preview),
  :global(.source-dockview-center-shell .source-dockview-editor-files-shell) {
    height: 100%;
  }

  :global(.source-dockview-workbench-shell .source-dockview-attached-panel.terminal-launchpad),
  :global(.source-dockview-center-shell .source-dockview-attached-panel.terminal-launchpad) {
    flex: 1 1 0;
    align-self: stretch;
    width: 100%;
    height: 100% !important;
    min-height: 0;
    max-height: none;
    margin-top: 0;
  }

  :global(.source-dockview-workbench-shell .embedded-terminal-panel),
  :global(.source-dockview-workbench-shell .embedded-terminal-host),
  :global(.source-dockview-center-shell .embedded-terminal-panel),
  :global(.source-dockview-center-shell .embedded-terminal-host) {
    height: 100% !important;
    min-height: 0;
  }

  .terminal-launchpad {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    flex: 1 1 auto;
    gap: 6px;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    max-height: none;
    margin-top: 0;
    overflow: hidden;
    padding: var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  :global(.source-dockview-bottom-shell .terminal-launchpad) {
    flex: 0 0 auto;
    height: var(--bottom-dock-height);
    min-height: 96px;
    margin-top: 6px;
  }

  .terminal-launchpad-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }

  .terminal-launchpad-identity,
  .terminal-launchpad-actions,
  .terminal-inline-picker {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .terminal-launchpad-identity {
    gap: var(--space-2);
    color: var(--color-text);
  }

  .terminal-launchpad-glyph {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    color: var(--color-text-2);
  }

  .terminal-launchpad-identity strong {
    flex-shrink: 0;
    min-width: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .terminal-launchpad-status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    overflow: hidden;
  }

  .terminal-launchpad-actions {
    justify-content: end;
    gap: var(--space-2);
  }

  .terminal-launchpad-actions .file-action-button {
    display: inline-flex;
    width: auto;
    min-width: 28px;
    height: 26px;
    gap: var(--space-1);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .terminal-launchpad-actions .file-action-button.icon-only {
    width: 28px;
    padding: 0;
  }

  .terminal-inline-picker {
    gap: var(--space-1);
  }

  .terminal-inline-picker span {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .terminal-inline-picker select {
    width: 94px;
    height: 26px;
    min-width: 0;
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-elevated);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .embedded-terminal-panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: var(--space-1);
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-elevated);
    overflow: hidden;
  }

  .embedded-terminal-panel.active {
    box-shadow: inset 0 0 0 1px var(--color-live-bg);
  }

  .embedded-terminal-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    min-height: 24px;
    padding: 0 var(--space-2);
    border-bottom: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .embedded-terminal-meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    letter-spacing: 0.01em;
  }

  .embedded-terminal-toolbar code {
    color: var(--color-text-3);
    font-family: "Google Sans Mono", "SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace;
    font-size: var(--text-xs);
  }

  .embedded-terminal-host {
    height: 100%;
    min-height: 0;
    overflow: hidden;
    border: 0;
    border-radius: 0;
    background: #282a36;
  }

  .embedded-terminal-host :global(.xterm) {
    height: 100%;
    padding: 5px 8px 6px;
    font-family: "Google Sans Mono", "SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace;
  }

  .embedded-terminal-host :global(.xterm-viewport) {
    background: transparent !important;
  }

  .embedded-terminal-host :global(.xterm-screen) {
    min-height: 100%;
  }

  .embedded-terminal-error {
    min-width: 0;
    overflow: hidden;
    color: #d8aa55;
    font-family: "Google Sans Mono", "SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-toolbar {
    display: grid;
    position: relative;
    grid-template-columns: minmax(0, 1fr) repeat(3, auto);
    align-items: center;
    gap: var(--space-2);
    height: 28px;
    padding: 0 var(--space-2);
    border-bottom: 1px solid var(--color-border);
    background: transparent;
  }

  .editor-mode-toggle {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: center;
    width: 128px;
    min-width: 0;
    height: 22px;
    padding: 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .editor-mode-toggle button {
    display: inline-grid;
    grid-template-columns: 12px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    height: 18px;
    padding: 0 var(--space-1);
    color: var(--color-text-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .editor-mode-toggle button.active {
    color: var(--color-live);
    background: var(--color-live-bg);
  }

  .editor-mode-toggle button:hover,
  .editor-mode-toggle button:focus-visible {
    color: var(--color-text);
    outline: 0;
  }

  .editor-mode-toggle span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-state {
    display: inline-grid;
    grid-template-columns: 14px minmax(0, 1fr) auto auto;
    align-items: center;
    justify-self: start;
    gap: var(--space-2);
    min-width: 0;
    max-width: 100%;
    height: 20px;
    padding: 0 var(--space-1);
    color: var(--color-text);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
  }

  .editor-file-glyph {
    display: inline-flex;
    align-items: center;
    color: var(--color-text-3);
  }

  .editor-file-state strong,
  .editor-file-state small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-title {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }

  .editor-file-state strong {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .editor-file-state small {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .editor-lsp-chip {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .editor-lsp-recovery-strip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    height: 22px;
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    background: transparent;
  }

  .editor-lsp-recovery-action {
    display: grid;
    place-items: center;
    width: 24px;
    height: 20px;
    padding: 0;
    color: var(--color-text-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }

  .editor-lsp-recovery-action:hover:not(:disabled),
  .editor-lsp-recovery-action:focus-visible {
    color: var(--color-text);
    outline: 0;
    background: var(--color-live-bg);
  }

  .editor-lsp-recovery-action:disabled {
    cursor: default;
    opacity: 0.38;
  }

  .editor-local-nav {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 3px;
    min-width: 0;
    height: 24px;
    padding: 2px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(10, 13, 14, 0.42);
  }

  .editor-local-nav button {
    display: inline-grid;
    grid-template-columns: 13px auto;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-width: 0;
    height: 20px;
    padding: 0 5px;
    color: #9ca8a4;
    border: 1px solid rgba(255, 255, 255, 0.065);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.03);
    font-size: 10px;
    font-weight: 800;
    cursor: pointer;
  }

  .editor-local-nav button.active {
    color: #dffdf8;
    border-color: rgba(92, 226, 207, 0.32);
    background: rgba(92, 226, 207, 0.12);
  }

  .editor-local-nav button:hover,
  .editor-local-nav button:focus-visible {
    color: #f1f7f5;
    border-color: rgba(92, 226, 207, 0.28);
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .editor-local-nav strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-local-nav span {
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

  .editor-local-nav strong {
    color: #72e2cf;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 860;
  }

  .editor-nav-drawer {
    display: grid;
    grid-template-rows: 28px minmax(0, 1fr);
    max-height: 190px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-bottom: 1px solid rgba(92, 226, 207, 0.14);
    background: rgba(14, 18, 18, 0.94);
  }

  .editor-nav-drawer-header {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 22px;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 0 6px 0 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .editor-nav-drawer-header strong,
  .editor-nav-drawer-header span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-nav-drawer-header strong {
    color: #eef6f4;
    font-size: 11px;
    font-weight: 860;
  }

  .editor-nav-drawer-header span {
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
  }

  .editor-nav-close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    color: #9fa9a6;
    border: 0;
    border-radius: 5px;
    background: transparent;
    cursor: pointer;
  }

  .editor-nav-close:hover,
  .editor-nav-close:focus-visible {
    color: #f2f6f5;
    outline: 0;
    background: rgba(255, 255, 255, 0.08);
  }

  .editor-nav-list {
    display: grid;
    align-content: start;
    gap: 3px;
    min-height: 0;
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 5px;
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .editor-menu-anchor {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .editor-icon-button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 22px;
    color: var(--color-text-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }

  .editor-icon-button:hover,
  .editor-icon-button:focus-visible {
    color: var(--color-text);
    outline: 0;
    background: var(--color-live-bg);
  }

  .editor-icon-button.active {
    color: var(--color-live);
    background: var(--color-live-bg);
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
    flex: 1 1 auto;
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

  .activity-source-control-list {
    max-height: none;
    padding: 0;
    border-bottom: 0;
    overflow: visible;
  }

  .activity-source-control-list .git-status-group {
    gap: 2px;
  }

  .activity-source-control-list .git-status-group + .git-status-group {
    padding-top: 5px;
    border-top: 1px solid rgba(255, 255, 255, 0.055);
  }

  .activity-source-control-list .git-status-group-heading {
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    cursor: pointer;
    min-height: 21px;
    padding: 0 2px;
  }

  .activity-source-control-list .git-status-group-heading::-webkit-details-marker {
    display: none;
  }

  .activity-source-control-list .git-status-group-heading::before {
    width: 0;
    height: 0;
    border-top: 4px solid transparent;
    border-bottom: 4px solid transparent;
    border-left: 5px solid rgba(174, 184, 181, 0.76);
    content: "";
    transform: rotate(90deg);
    transition: transform 140ms ease, border-left-color 140ms ease;
  }

  .activity-source-control-list .git-status-group:not([open]) .git-status-group-heading::before {
    transform: rotate(0deg);
  }

  .activity-source-control-list .git-status-row {
    min-height: 27px;
    padding: 4px 5px;
    border-color: transparent;
    border-radius: 5px;
    background: transparent;
  }

  .activity-source-control-list .git-status-row:hover,
  .activity-source-control-list .git-status-row:focus-visible {
    border-color: rgba(92, 226, 207, 0.16);
    outline: 0;
    background: rgba(255, 255, 255, 0.045);
  }

  .activity-source-control-list .git-status-row.selected {
    border-color: rgba(92, 226, 207, 0.3);
    background: rgba(92, 226, 207, 0.095);
  }

  .activity-git-health-strip {
    grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
  }

  .activity-git-command-drawer {
    min-width: 0;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.1);
  }

  .activity-git-command-drawer summary {
    height: 27px;
    padding: 0 8px;
  }

  .activity-git-command-strip {
    display: grid;
    gap: 5px;
    min-width: 0;
    padding: 0 6px 6px;
  }

  .activity-git-remote-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 5px;
    min-width: 0;
  }

  .activity-git-commit-row {
    grid-template-columns: minmax(0, 1fr) minmax(78px, auto);
  }

  .activity-git-command-strip .git-action-button {
    min-height: 25px;
  }

  .activity-git-command-strip .git-commit-input {
    min-height: 34px;
    background: rgba(0, 0, 0, 0.24);
  }

  .activity-git-graph-summary {
    margin-top: -3px;
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

  .git-graph-summary-strip,
  .git-task-search-targets {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    min-height: 20px;
    overflow: hidden;
    padding: 3px 5px;
    color: #9aa7a3;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.028);
    font-size: 8.5px;
    font-weight: 780;
  }

  .git-graph-summary-strip span,
  .git-graph-summary-strip small,
  .git-task-search-targets small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-graph-summary-strip span {
    flex: 1 1 auto;
    color: #dce5e2;
    font-weight: 830;
  }

  .git-graph-summary-strip small {
    flex: 0 0 auto;
    color: #8d9995;
  }

  .git-task-search-targets {
    flex-wrap: wrap;
    overflow: visible;
  }

  .git-task-trail > span:first-child {
    flex: 0 0 auto;
    color: #aeb8b5;
    text-transform: uppercase;
  }

  .git-task-search-targets > span:first-child {
    flex: 0 0 auto;
    color: #aeb8b5;
    font-size: 8px;
    font-weight: 860;
    text-transform: uppercase;
  }

  .git-task-search-targets small {
    max-width: 160px;
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

  .git-commit-detail-drawer {
    display: grid;
    gap: 5px;
    min-width: 0;
    padding: 0;
    border: 1px solid rgba(111, 223, 207, 0.18);
    border-radius: 7px;
    background: rgba(111, 223, 207, 0.055);
  }

  .git-commit-detail-drawer[open] {
    background: rgba(111, 223, 207, 0.07);
  }

  .git-commit-detail-drawer summary::-webkit-details-marker {
    display: none;
  }

  .git-commit-detail-summary {
    display: grid;
    grid-template-columns: 10px minmax(0, 1fr) minmax(0, 86px);
    align-items: center;
    gap: 6px;
    min-height: 28px;
    min-width: 0;
    padding: 4px 7px;
    cursor: pointer;
    list-style: none;
  }

  .git-commit-detail-summary::before {
    color: #7f8f8b;
    font-size: 11px;
    font-weight: 900;
    content: ">";
    transition: transform 140ms ease;
  }

  .git-commit-detail-drawer[open] .git-commit-detail-summary::before {
    transform: rotate(90deg);
  }

  .git-commit-detail-summary-main {
    display: grid;
    gap: 1px;
    min-width: 0;
  }

  .git-commit-detail-summary-main strong,
  .git-commit-detail-summary-main small,
  .git-commit-detail-summary-ref,
  .git-commit-detail-facts strong,
  .git-commit-detail-facts small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-commit-detail-summary-main strong {
    color: #f1f5f4;
    font-size: 10px;
    font-weight: 820;
  }

  .git-commit-detail-summary-main small,
  .git-commit-detail-summary-ref {
    color: #8d9995;
    font-size: 8.5px;
    font-weight: 760;
  }

  .git-commit-detail-summary-ref {
    justify-self: end;
    max-width: 86px;
  }

  .git-commit-detail-body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 7px;
    min-width: 0;
    padding: 0 7px 7px 23px;
  }

  .git-commit-detail-facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(74px, 1fr));
    gap: 4px;
    min-width: 0;
  }

  .git-commit-detail-facts span {
    display: grid;
    gap: 1px;
    min-width: 0;
    padding: 3px 5px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.12);
  }

  .git-commit-detail-facts strong {
    color: #9aa7a3;
    font-size: 7.5px;
    font-weight: 860;
    text-transform: uppercase;
  }

  .git-commit-detail-facts small {
    color: #dce5e2;
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
    grid-template-columns: 14px minmax(0, 1fr) minmax(42px, auto);
    align-items: center;
    gap: 5px;
    min-width: 0;
    min-height: 30px;
    padding: 4px 6px;
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
    width: 14px;
    height: 22px;
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
    gap: 1px;
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
    font-size: 9.5px;
    font-weight: 820;
  }

  .git-history-main small {
    color: #8d9995;
    font-size: 8.5px;
    font-weight: 740;
  }

  .git-history-meta {
    display: grid;
    justify-items: end;
    gap: 4px;
    max-width: 112px;
  }

  .git-history-badges {
    display: inline-flex;
    justify-content: flex-end;
    gap: 3px;
    max-width: 112px;
    min-width: 0;
    overflow: hidden;
  }

  .git-history-badge {
    max-width: 68px;
    min-width: 0;
    overflow: hidden;
    padding: 1px 5px;
    color: #aeb8b5;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 8px;
    font-weight: 850;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-history-badge.head {
    color: #6fdfcf;
    border-color: rgba(111, 223, 207, 0.24);
    background: rgba(111, 223, 207, 0.075);
  }

  .git-history-badge.upstream {
    color: #84c9de;
    border-color: rgba(132, 201, 222, 0.22);
    background: rgba(132, 201, 222, 0.07);
  }

  .git-history-badge.task {
    color: #c8b6ff;
    border-color: rgba(200, 182, 255, 0.22);
  }

  .git-history-badge.tag,
  .git-history-badge.merge {
    color: #d8aa55;
    border-color: rgba(216, 170, 85, 0.22);
  }

  .git-history-actions {
    position: absolute;
    top: 50%;
    right: 5px;
    display: inline-flex;
    justify-content: flex-end;
    gap: 3px;
    max-width: 112px;
    padding: 2px;
    border-radius: 7px;
    background: color-mix(in srgb, #151a1a 86%, transparent);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.22);
    opacity: 0;
    pointer-events: none;
    transform: translateY(-50%);
    transition: opacity 120ms ease, visibility 120ms ease;
    visibility: hidden;
  }

  .git-commit-detail-actions {
    justify-content: flex-end;
    max-width: 112px;
  }

  .git-history-row:hover .git-history-actions,
  .git-history-row:focus-within .git-history-actions {
    opacity: 1;
    pointer-events: auto;
    visibility: visible;
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

  .intelligence-empty.compact {
    min-height: 42px;
    font-size: 10px;
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

  /* ============================================================
   * Insights panel readability pass (TSK-346) — scoped overrides.
   * All rules are scoped under .source-intelligence-panel so the
   * shared git- and intelligence- base styles still serve the Activity
   * Bar source-control view and the editor navigation drawer.
   * Aesthetic: modern, minimal, spacious, borderless (VS Code),
   * driven entirely by design tokens.
   * ============================================================ */

  /* ── Panel shell ────────────────────────────────────────────── */
  .source-intelligence-panel {
    border-left: 1px solid var(--color-border);
    background: var(--color-bg);
    color: var(--color-text);
  }

  /* ── Tab switcher — borderless segmented, active = accent ────── */
  .source-intelligence-panel .intelligence-tabs {
    gap: var(--space-1);
    padding: var(--space-2);
    border-bottom: 1px solid var(--color-border);
  }

  .source-intelligence-panel .intelligence-tabs button {
    height: 28px;
    gap: var(--space-1);
    padding: 0 var(--space-2);
    color: var(--color-text-3);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.02em;
    transition: background 130ms ease, color 130ms ease;
  }

  .source-intelligence-panel .intelligence-tabs button:hover {
    color: var(--color-text);
    background: var(--color-surface);
  }

  .source-intelligence-panel .intelligence-tabs button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .source-intelligence-panel .intelligence-tabs button.active {
    color: var(--color-on-accent);
    border-color: transparent;
    background: var(--color-accent);
  }

  .source-intelligence-panel .intelligence-tabs strong {
    color: var(--color-text-3);
    font-family: inherit;
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-semibold);
  }

  .source-intelligence-panel .intelligence-tabs button.active strong {
    color: var(--color-on-accent);
    opacity: 0.78;
  }

  /* ── Summaries / empty / loading states ─────────────────────── */
  .source-intelligence-panel .intelligence-summary {
    padding: var(--space-2) var(--space-3);
    color: var(--color-text-3);
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .intelligence-empty {
    min-height: 96px;
    color: var(--color-text-3);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .intelligence-list {
    padding: var(--space-2);
  }

  /* ── Shared section heading (Changes / History / Tasks) ──────── */
  .source-intelligence-panel .git-insights-section {
    display: grid;
    gap: var(--space-1);
    min-width: 0;
  }

  .source-intelligence-panel .git-insights-section-heading {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-1) 0;
  }

  .source-intelligence-panel .git-insights-section-title {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .source-intelligence-panel .git-insights-section-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 16px;
    padding: 0 var(--space-2);
    color: var(--color-text-3);
    background: var(--color-elevated);
    border-radius: var(--radius-pill);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  /* a trailing muted note inside a section heading (e.g. history summary) */
  .source-intelligence-panel .git-insights-section-heading small {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
    text-align: right;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Intelligence rows (Problems / Symbols) ─────────────────── */
  .source-intelligence-panel .intelligence-row {
    grid-template-columns: minmax(56px, auto) minmax(0, 1fr) auto;
    gap: var(--space-2);
    min-height: 34px;
    padding: var(--space-2);
    color: var(--color-text);
    border-radius: var(--radius-sm);
    transition: background 120ms ease;
  }

  .source-intelligence-panel .intelligence-row:hover,
  .source-intelligence-panel .intelligence-row:focus-visible {
    color: var(--color-text);
    background: var(--color-surface);
  }

  .source-intelligence-panel .intelligence-row:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .source-intelligence-panel .intelligence-row strong {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .source-intelligence-panel .intelligence-row span {
    color: var(--color-text);
    font-size: var(--text-sm);
    font-weight: var(--weight-normal);
  }

  .source-intelligence-panel .intelligence-row small {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  /* Problems: tone-code severity via tokens */
  .source-intelligence-panel .intelligence-row.diagnostic.error strong {
    color: var(--color-bad);
  }

  .source-intelligence-panel .intelligence-row.diagnostic.warning strong {
    color: var(--color-attention);
  }

  /* ── Git container ──────────────────────────────────────────── */
  .source-intelligence-panel .git-diff-panel {
    gap: 0;
  }

  /* ── Commands drawer ────────────────────────────────────────── */
  .source-intelligence-panel .git-command-drawer {
    border-bottom: 1px solid var(--color-border);
  }

  .source-intelligence-panel .git-command-drawer summary {
    height: 34px;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .source-intelligence-panel .git-command-drawer summary::before {
    border-left-color: var(--color-text-3);
  }

  .source-intelligence-panel .git-command-drawer summary small {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
    letter-spacing: 0;
    text-transform: none;
  }

  .source-intelligence-panel .git-controls {
    gap: var(--space-2);
    padding: 0 var(--space-3) var(--space-3);
  }

  .source-intelligence-panel .git-action-row,
  .source-intelligence-panel .git-remote-row,
  .source-intelligence-panel .git-commit-row {
    gap: var(--space-2);
  }

  .source-intelligence-panel .git-action-button {
    min-height: 28px;
    gap: var(--space-1);
    padding: 0 var(--space-2);
    color: var(--color-text-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .source-intelligence-panel .git-action-button:hover:not(:disabled),
  .source-intelligence-panel .git-action-button:focus-visible {
    color: var(--color-text);
    border-color: var(--color-border);
    outline: none;
    background: var(--color-elevated);
  }

  .source-intelligence-panel .git-action-button.commit {
    color: var(--color-on-accent);
    border-color: transparent;
    background: var(--color-accent);
  }

  .source-intelligence-panel .git-action-button.commit:hover:not(:disabled) {
    color: var(--color-on-accent);
    background: var(--color-accent);
    opacity: 0.9;
  }

  .source-intelligence-panel .git-commit-input {
    padding: var(--space-2);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-size: var(--text-sm);
    line-height: 1.4;
  }

  .source-intelligence-panel .git-commit-input:focus {
    border-color: transparent;
    box-shadow: var(--focus-ring);
  }

  .source-intelligence-panel .git-action-message {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .source-intelligence-panel .git-action-message.error {
    color: var(--color-bad);
  }

  /* ── Changes (status list) ──────────────────────────────────── */
  .source-intelligence-panel .git-status-list {
    gap: var(--space-1);
    max-height: 180px;
    padding: var(--space-3);
    border-bottom: 1px solid var(--color-border);
  }

  .source-intelligence-panel .git-status-overview {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .source-intelligence-panel .git-status-group {
    gap: var(--space-1);
  }

  .source-intelligence-panel .git-status-group-heading {
    min-height: 22px;
    gap: var(--space-2);
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.04em;
  }

  .source-intelligence-panel .git-status-group-heading span {
    color: var(--color-text-3);
    font-variant-numeric: tabular-nums;
  }

  .source-intelligence-panel .git-status-group-heading button {
    height: 22px;
    padding: 0 var(--space-2);
    color: var(--color-text-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .source-intelligence-panel .git-status-group-heading button:hover:not(:disabled),
  .source-intelligence-panel .git-status-group-heading button:focus-visible {
    color: var(--color-text);
    border-color: var(--color-border);
    outline: none;
    background: var(--color-elevated);
  }

  /* File rows — VS Code source-control style */
  .source-intelligence-panel .git-status-row {
    grid-template-columns: 16px minmax(0, 1fr) auto;
    gap: var(--space-2);
    min-height: 28px;
    padding: var(--space-1) var(--space-2);
    color: var(--color-text);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    transition: background 120ms ease;
  }

  .source-intelligence-panel .git-status-row:hover,
  .source-intelligence-panel .git-status-row:focus-visible {
    border-color: transparent;
    outline: none;
    background: var(--color-surface);
  }

  .source-intelligence-panel .git-status-row.selected {
    border-color: transparent;
    background: color-mix(in srgb, var(--color-accent) 14%, transparent);
  }

  .source-intelligence-panel .git-status-row:focus-visible {
    box-shadow: var(--focus-ring);
  }

  /* status glyph (M/A/D/U) — tone-coded via tokens */
  .source-intelligence-panel .git-status-row strong {
    justify-self: center;
    color: var(--color-text-3);
    font-family: inherit;
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  .source-intelligence-panel .git-status-row span {
    color: var(--color-text);
    font-size: var(--text-sm);
    font-weight: var(--weight-normal);
  }

  /* dim the directory portion of the path, keep the filename emphasized:
     handled by markup elsewhere; here we keep the secondary note muted */
  .source-intelligence-panel .git-status-row small {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  /* Tone the status glyph by group (VS Code source-control convention):
     staged = good/live, unstaged (modified) = attention, untracked = muted. */
  .source-intelligence-panel .git-status-group[data-group="staged"] .git-status-row strong {
    color: var(--color-good);
  }

  .source-intelligence-panel .git-status-group[data-group="unstaged"] .git-status-row strong {
    color: var(--color-attention);
  }

  .source-intelligence-panel .git-status-group[data-group="untracked"] .git-status-row strong {
    color: var(--color-text-3);
  }

  /* ── History panel ──────────────────────────────────────────── */
  .source-intelligence-panel .git-history-panel {
    gap: var(--space-2);
    padding: var(--space-3);
    border-bottom: 1px solid var(--color-border);
  }

  .source-intelligence-panel .git-history-heading {
    color: var(--color-text);
  }

  /* compact summary strip — borderless, muted */
  .source-intelligence-panel .git-graph-summary-strip {
    min-height: 0;
    gap: var(--space-2);
    padding: 0;
    color: var(--color-text-3);
    border: none;
    background: transparent;
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .source-intelligence-panel .git-graph-summary-strip span {
    color: var(--color-text-2);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .git-graph-summary-strip small {
    color: var(--color-text-3);
  }

  /* BRANCH/SYNC/WORKTREE/ROOT/HEAD — clean key→value chips */
  .source-intelligence-panel .git-branch-health-strip {
    gap: var(--space-1);
  }

  .source-intelligence-panel .git-branch-health-chip {
    gap: var(--space-2);
    min-height: 24px;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .source-intelligence-panel .git-branch-health-chip strong {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.04em;
  }

  .source-intelligence-panel .git-branch-health-chip span {
    color: var(--color-text);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .git-branch-health-chip.clean span {
    color: var(--color-good);
  }

  .source-intelligence-panel .git-branch-health-chip.dirty span {
    color: var(--color-attention);
  }

  .source-intelligence-panel .git-branch-health-chip.warning span {
    color: var(--color-attention);
  }

  .source-intelligence-panel .git-branch-health-chip.error span {
    color: var(--color-bad);
  }

  .source-intelligence-panel .git-branch-health-chip.muted span {
    color: var(--color-text-3);
  }

  /* ── Tasks ──────────────────────────────────────────────────── */
  .source-intelligence-panel .git-task-trail {
    flex-wrap: wrap;
    gap: var(--space-1);
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .git-task-source-map {
    gap: var(--space-1);
  }

  .source-intelligence-panel .git-task-source-row {
    gap: var(--space-2);
    min-height: 28px;
    padding: var(--space-1) var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    transition: background 120ms ease;
  }

  .source-intelligence-panel .git-task-source-row:hover {
    background: var(--color-surface);
  }

  .source-intelligence-panel .git-task-source-row small {
    color: var(--color-text-2);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .source-intelligence-panel .git-task-source-row button {
    width: 22px;
    height: 22px;
    color: var(--color-text-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .source-intelligence-panel .git-task-source-row button:hover,
  .source-intelligence-panel .git-task-source-row button:focus-visible {
    color: var(--color-text);
    border-color: var(--color-border);
    outline: none;
    background: var(--color-elevated);
  }

  /* task pill — readable accent chip */
  .source-intelligence-panel .git-task-link {
    min-height: 20px;
    padding: 0 var(--space-2);
    color: var(--color-on-accent);
    background: var(--color-accent);
    border-radius: var(--radius-pill);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
  }

  /* ── Selected commit detail drawer ──────────────────────────── */
  .source-intelligence-panel .git-commit-detail-drawer {
    gap: var(--space-1);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .source-intelligence-panel .git-commit-detail-drawer[open] {
    background: var(--color-elevated);
  }

  .source-intelligence-panel .git-commit-detail-summary {
    min-height: 30px;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
  }

  .source-intelligence-panel .git-commit-detail-summary::before {
    color: var(--color-text-3);
  }

  .source-intelligence-panel .git-commit-detail-summary-main strong {
    color: var(--color-text);
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
  }

  .source-intelligence-panel .git-commit-detail-summary-main small,
  .source-intelligence-panel .git-commit-detail-summary-ref {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .source-intelligence-panel .git-commit-detail-facts span {
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
  }

  .source-intelligence-panel .git-commit-detail-facts strong {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.04em;
  }

  .source-intelligence-panel .git-commit-detail-facts small {
    color: var(--color-text-2);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  /* ── Commit history rows — clean two-line rows ──────────────── */
  .source-intelligence-panel .git-history-list {
    gap: var(--space-1);
    max-height: 220px;
  }

  .source-intelligence-panel .git-history-row {
    grid-template-columns: 14px minmax(0, 1fr) minmax(42px, auto);
    gap: var(--space-2);
    min-height: 36px;
    padding: var(--space-1) var(--space-2);
    color: var(--color-text);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    transition: background 120ms ease;
  }

  .source-intelligence-panel .git-history-row:hover {
    background: var(--color-surface);
  }

  .source-intelligence-panel .git-history-row.selected {
    border-color: transparent;
    background: color-mix(in srgb, var(--color-accent) 14%, transparent);
  }

  .source-intelligence-panel .git-history-row.head,
  .source-intelligence-panel .git-history-row.branch,
  .source-intelligence-panel .git-history-row.merge,
  .source-intelligence-panel .git-history-row.root {
    border-color: transparent;
  }

  .source-intelligence-panel .git-history-row:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .source-intelligence-panel .git-history-main strong {
    color: var(--color-text);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .git-history-main small {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  /* subtle graph lane markers */
  .source-intelligence-panel .git-graph-marker::before {
    background: var(--color-border);
  }

  .source-intelligence-panel .git-graph-marker::after {
    border-color: var(--color-text-3);
    background: var(--color-bg);
  }

  .source-intelligence-panel .git-graph-marker.head::before {
    background: color-mix(in srgb, var(--color-accent) 50%, transparent);
  }

  .source-intelligence-panel .git-graph-marker.head::after {
    border-color: var(--color-accent);
    background: var(--color-accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent) 16%, transparent);
  }

  .source-intelligence-panel .git-graph-marker.merge::after {
    border-color: var(--color-attention);
  }

  /* ownership / ref badges — small tone chips */
  .source-intelligence-panel .git-history-badge {
    padding: 1px var(--space-2);
    color: var(--color-text-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .source-intelligence-panel .git-history-badge.head {
    color: var(--color-live);
    border-color: var(--color-border);
    background: var(--color-live-bg);
  }

  .source-intelligence-panel .git-history-badge.upstream {
    color: var(--color-text-2);
    border-color: var(--color-border);
    background: var(--color-surface);
  }

  .source-intelligence-panel .git-history-badge.task {
    color: var(--color-text-2);
    border-color: var(--color-border);
  }

  .source-intelligence-panel .git-history-badge.tag,
  .source-intelligence-panel .git-history-badge.merge {
    color: var(--color-attention);
    border-color: var(--color-border);
    background: var(--color-attention-bg);
  }

  .source-intelligence-panel .git-history-actions {
    gap: var(--space-1);
    padding: var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--color-elevated);
    box-shadow: var(--shadow-sm);
  }

  .source-intelligence-panel .git-history-actions button,
  .source-intelligence-panel .git-commit-detail-actions button {
    width: 22px;
    height: 22px;
    color: var(--color-text-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .source-intelligence-panel .git-history-actions button:hover,
  .source-intelligence-panel .git-history-actions button:focus-visible,
  .source-intelligence-panel .git-commit-detail-actions button:hover,
  .source-intelligence-panel .git-commit-detail-actions button:focus-visible {
    color: var(--color-text);
    border-color: var(--color-border);
    outline: none;
    background: var(--color-elevated);
  }

  .source-intelligence-panel .git-ref-label {
    color: var(--color-text-3);
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  /* ── Diff block ─────────────────────────────────────────────── */
  .source-intelligence-panel .git-diff-block {
    margin: var(--space-3);
    padding: var(--space-3);
    color: var(--color-text-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    font-size: var(--text-sm);
    line-height: 1.5;
  }

  .quick-open-layer {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: start center;
    padding: 72px 16px 16px;
  }

  .quick-open-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: rgba(7, 9, 10, 0.56);
    backdrop-filter: blur(10px);
    cursor: default;
  }

  .quick-open-panel {
    position: relative;
    z-index: 1;
    width: min(720px, calc(100vw - 32px));
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 12px;
    background: rgba(22, 24, 24, 0.98);
    box-shadow: 0 28px 80px rgba(0, 0, 0, 0.44);
  }

  .quick-open-search {
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

  .quick-open-icon,
  .quick-open-result-icon {
    display: grid;
    place-items: center;
    min-width: 0;
  }

  .quick-open-icon {
    color: #6fdfcf;
  }

  .quick-open-search input {
    height: 100%;
    font-size: 15px;
    font-weight: 700;
  }

  .quick-open-results {
    display: grid;
    gap: 3px;
    max-height: 368px;
    padding: 7px;
    overflow: auto;
  }

  .quick-open-results button {
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

  .quick-open-results button:hover,
  .quick-open-results button.active {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.12);
  }

  .quick-open-result-icon {
    color: #8d9995;
  }

  .quick-open-results button.active .quick-open-result-icon {
    color: #6fdfcf;
  }

  .quick-open-results button span {
    display: grid;
    min-width: 0;
  }

  .quick-open-results strong,
  .quick-open-results small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-open-results strong {
    font-size: 13px;
    line-height: 1.15;
  }

  .quick-open-results small,
  .quick-open-results em {
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

  @media (max-width: 1120px) {
    .shell:not(.activity-hidden) {
      grid-template-columns: 56px minmax(0, 1fr);
    }

    .shell.side-right:not(.activity-hidden) {
      grid-template-columns: minmax(0, 1fr) 56px;
    }

    .shell:not(.activity-hidden) .activity-shell {
      grid-template-columns: 46px;
    }

    .shell:not(.activity-hidden) .sidebar {
      display: none;
    }
  }

  @media (max-width: 720px) {
    :global(body) {
      min-width: 0;
      overflow: auto;
    }

    .shell {
      grid-template-columns: 1fr;
      width: 100vw;
      height: auto;
      min-height: 100dvh;
      margin: 0;
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
