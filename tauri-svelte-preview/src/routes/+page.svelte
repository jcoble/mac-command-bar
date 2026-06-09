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
    Plus,
    RefreshCw,
    RotateCcw,
    Save,
    Search,
    SplitSquareHorizontal,
    Trash2,
    X
  } from '@lucide/svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import { sourcePreviewAppearance, sourcePreviewAppearanceKey } from '$lib/sourcePreviewAppearance';
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
    scrollTopForSourceTreeReveal,
    selectBackgroundIndexProjects,
    selectPreferredSourceRecord,
    sourceSupportsLanguageIntelligence,
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
    type SourceSymbol,
    type SourceTreeNode,
    type SourceTreeRow
  } from '$lib/sourceData';
  import {
    cancelSourceScanFromTauri,
    createSourceScanId,
    defaultSourceScanLimit,
    expandedSourceScanLimit,
    findSourceDefinitionsFromTauri,
    findSourceReferencesFromTauri,
    listProjectWorktreesFromTauri,
    listRuntimeContextsFromTauri,
    listenToSourceScanProgress,
    listSourceFilesFromTauri,
    nativeSourceScanProgressEvent,
    openSourceFileFromTauri,
    readProjectGitStatusFromTauri,
    readSourceFromTauri,
    revealSourceFileFromTauri,
    searchSourceFilesFromTauri,
    writeSourceToTauri,
    type NativeSourceScanProgress,
    type ProjectGitFileStatus,
    type ProjectGitStatus,
    type ProjectWorktree,
    type RuntimeContext
  } from '$lib/tauriSource';

  const customProjectRootsStorageKey = 'mac-command-bar.source-browser.custom-project-roots';
  const selectedProjectStorageKey = 'mac-command-bar.source-browser.selected-project';
  const selectedSourcePathStorageKey = 'mac-command-bar.source-browser.selected-source-paths';
  const recentSourceRecordsStorageKey = 'mac-command-bar.source-browser.recent-source-records';
  const openSourceTabsStorageKey = 'mac-command-bar.source-browser.open-source-tabs';
  const maxRecentSourceRecords = 24;
  const maxProjectRecentRecords = 5;
  const maxProjectOpenSourceTabs = 8;
  const maxStoredOpenSourceTabs = 64;
  const maxSourceSearchResults = 50;
  const maxSourceDefinitionResults = 20;
  const sourceScanCacheMaxAgeMs = 5 * 60 * 1000;
  const maxSourceScanCacheEntries = 8;
  const sourceTreeRowHeight = 30;
  const sourceTreeOverscanRows = 8;
  const sourceTreeFallbackViewportHeight = 420;
  const sourceScanProgressEventName = nativeSourceScanProgressEvent;
  const initialProject = defaultProjectRoots[0];
  const initialRecords = demoRecordsForProject(initialProject);
  const initialPreview = initialRecords[0] ? demoPreviewFor(initialRecords[0]) : null;

  type SourceIntelligenceAction = 'definition' | 'hover' | 'references';
  type SourceEditorIntelligenceCommand = {
    id: number;
    action: SourceIntelligenceAction;
  };
  type SourceIntelligencePanel = 'problems' | 'symbols';

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
  let sourceSymbols = $state<SourceSymbol[]>([]);
  let sourceDefinitionTargets = $state<SourceDefinitionTarget[]>([]);
  let sourceDefinitionQuery = $state('');
  let sourceDefinitionLoading = $state(false);
  let sourceDefinitionError = $state('');
  let sourceReferenceTargets = $state<SourceReferenceTarget[]>([]);
  let sourceReferenceQuery = $state('');
  let sourceReferenceLoading = $state(false);
  let sourceReferenceError = $state('');
  let sourceIntelligenceCommand = $state<SourceEditorIntelligenceCommand | null>(null);
  let sourceIntelligencePanel = $state<SourceIntelligencePanel>('symbols');
  let sourceSearchQuery = $state('');
  let sourceSearchResults = $state<SourceSearchMatch[]>([]);
  let sourceSearchLoading = $state(false);
  let sourceSearchError = $state('');
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

  type SourceScanOptions = {
    force?: boolean;
    limit?: number;
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
  let projectGitSummary = $derived(
    formatSourceContextGitSummary(projectGitStatus, projectGitLoading, projectGitError)
  );
  let sourceContextIdentity = $derived(
    formatSourceContextIdentity(selectedProject, projectGitSummary, runtime)
  );
  let selectedProjectRuntimeContexts = $derived(
    runtimeContexts.filter(
      (context) =>
        context.projectID === selectedProject.id || context.projectName === selectedProject.name
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

  $effect(() => {
    if (!quickOpenVisible) return;
    const lastResultIndex = Math.max(0, quickOpenResults.length - 1);
    if (quickOpenIndex > lastResultIndex) {
      quickOpenIndex = lastResultIndex;
    }
  });

  $effect(() => {
    const element = fileTreeElement;
    if (!element) return;

    measureFileTreeViewport();

    if (typeof ResizeObserver === 'undefined') return;

    const resizeObserver = new ResizeObserver(measureFileTreeViewport);
    resizeObserver.observe(element);
    return () => resizeObserver.disconnect();
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

    try {
      const tauriScan = await listSourceFilesFromTauri(project.path, '', scanLimit, scanId);
      if (generation !== scanGeneration) return;

      const nextRecords = tauriScan?.records ?? demoRecordsForProject(project);
      sourceScanCache = upsertSourceScanCacheEntry(
        sourceScanCache,
        project,
        nextRecords,
        tauriScan?.limit ?? scanLimit,
        Date.now(),
        maxSourceScanCacheEntries,
        tauriScan?.truncated ?? false
      );
      clearBackgroundIndexError(project.id);

      const nextSelection = applySourceRecords(
        nextRecords,
        preferredPath,
        tauriScan ? 'tauri source scan' : 'browser preview',
        tauriScan?.truncated ?? false
      );

      if (nextSelection) {
        await loadRecord(nextSelection, generation);
      } else {
        loading = false;
      }
    } catch (scanError) {
      if (generation !== scanGeneration) return;
      records = demoRecordsForProject(project);
      selectedRecord = records[0] ?? null;
      selectedSourceLine = null;
      scanLimitReached = false;
      preview = selectedRecord ? demoPreviewFor(selectedRecord) : null;
      if (preview) syncSourcePreviewContent(preview);
      expandedFolderIds = selectedRecord ? new Set(folderIdsForSourceRecord(selectedRecord)) : new Set();
      runtime = 'browser preview';
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
      const nextRecords = tauriScan?.records ?? demoRecordsForProject(project);
      sourceScanCache = upsertSourceScanCacheEntry(
        sourceScanCache,
        project,
        nextRecords,
        tauriScan?.limit ?? defaultSourceScanLimit,
        Date.now(),
        maxSourceScanCacheEntries,
        tauriScan?.truncated ?? false
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
        isDirty: false,
        hasUnmergedCommits: false,
        lastActivity: null,
        deleteEligibility: 'requires-confirmation'
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

  function projectWorktreeEligibilityKind(worktree: ProjectWorktree) {
    if (worktree.deleteEligibility.startsWith('blocked')) return 'blocked';
    if (worktree.isDirty || worktree.hasUnmergedCommits) return 'blocked';
    return 'ready';
  }

  function gitStatusForSourceRecord(record: SourceRecord | SourceOpenTab | null): ProjectGitFileStatus | null {
    return record ? gitStatusByRelativePath.get(record.relativePath) ?? null : null;
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

  async function runSourceDefinitionLookup(symbolName: string) {
    const normalizedSymbolName = symbolName.trim();
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
      const nativeTargets = await findSourceDefinitionsFromTauri(
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
      sourceDefinitionError = nativeTargets ? '' : 'Browser preview definitions';

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

  async function runSourceReferenceLookup(symbolName: string) {
    const normalizedSymbolName = symbolName.trim();
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
      const nativeTargets = await findSourceReferencesFromTauri(
        records,
        normalizedSymbolName,
        maxSourceSearchResults
      );
      sourceReferenceTargets =
        nativeTargets ??
        findSourceReferenceTargets(
          records.map((record) => demoPreviewFor(record)),
          normalizedSymbolName,
          maxSourceSearchResults
        );
      sourceReferenceError = nativeTargets ? '' : 'Browser preview references';
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

    try {
      const tauriPreview = await readSourceFromTauri(record);
      if (expectedScanGeneration !== null && expectedScanGeneration !== scanGeneration) return;

      runtime = tauriPreview ? 'tauri file read' : 'browser preview';
      const nextPreview = tauriPreview ?? demoPreviewFor(record);
      preview = nextPreview;
      syncSourcePreviewContent(nextPreview);
    } catch (previewError) {
      if (expectedScanGeneration !== null && expectedScanGeneration !== scanGeneration) return;

      runtime = 'browser preview';
      error = previewError instanceof Error ? previewError.message : 'Could not read source file';
      const nextPreview = demoPreviewFor(record);
      preview = nextPreview;
      syncSourcePreviewContent(nextPreview);
    } finally {
      if (expectedScanGeneration === null || expectedScanGeneration === scanGeneration) {
        loading = false;
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
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'p') {
      event.preventDefault();
      openQuickOpen();
    }
  }

  function openQuickOpen() {
    quickOpenVisible = true;
    quickOpenQuery = '';
    quickOpenIndex = 0;
    window.setTimeout(() => quickOpenInput?.focus(), 0);
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
    fileActionStatus = '';
    error = '';

    try {
      await navigator.clipboard.writeText(preview.path);
      fileActionStatus = 'Path copied';
    } catch (copyError) {
      error = copyError instanceof Error ? copyError.message : 'Could not copy source path';
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
  }

  function syncSourcePreviewContent(nextPreview: SourcePreview) {
    const existingDraft = sourceDraftContentByPath[nextPreview.path];
    sourceDraftContentByPath = {
      ...sourceDraftContentByPath,
      [nextPreview.path]: existingDraft ?? nextPreview.content
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
    sourceSymbols = [];
    sourceIntelligenceCommand = null;
  }

  function handleEditorDiagnosticsChange(diagnostics: SourceDiagnostic[]) {
    sourceDiagnostics = diagnostics;
  }

  function handleEditorSymbolsChange(symbols: SourceSymbol[]) {
    sourceSymbols = symbols;
  }

  function handleEditorDefinitionLookup(symbolName: string) {
    void runSourceDefinitionLookup(symbolName);
  }

  function handleEditorReferenceLookup(symbolName: string) {
    void runSourceReferenceLookup(symbolName);
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
    selectedProjectID = nextProject.id;
    persistSelectedProjectID(nextProject.id);
    void loadProjectGitStatus(nextProject);
    void loadRuntimeContexts(projectOptions);
    void loadProjectWorktrees(nextProject);
    await scanProject(nextProject, selectedSourcePaths[nextProject.id]);
    void indexProjectsInBackground(projectOptions);
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
        void activateProject(duplicateProject);
      }
      return false;
    }

    const nextCustomProjectRoots = mergeProjectRoots([], [...customProjectRoots, nextProject]);
    customProjectRoots = nextCustomProjectRoots;
    persistCustomProjectRoots(nextCustomProjectRoots);
    addingProject = false;
    projectFormError = '';
    void activateProject(nextProject);
    return true;
  }

  async function activateProject(project: ProjectRoot) {
    selectedProjectID = project.id;
    persistSelectedProjectID(project.id);
    void loadProjectGitStatus(project);
    await scanProject(project, selectedSourcePaths[project.id]);
    void indexProjectsInBackground(projectOptions);
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
    const storedProject =
      storedProjectOptions.find((project) => project.id === storedProjectID) ??
      storedProjectOptions[0] ??
      initialProject;

    customProjectRoots = storedCustomProjectRoots;
    selectedSourcePaths = storedSelectedSourcePaths;
    recentSourceRecords = storedRecentSourceRecords;
    openSourceTabs = storedOpenSourceTabs;
    selectedProjectID = storedProject.id;
    persistSelectedProjectID(storedProject.id);
    window.setTimeout(measureFileTreeViewport, 0);
    void loadProjectGitStatus(storedProject);
    void loadRuntimeContexts(storedProjectOptions);
    void loadProjectWorktrees(storedProject);
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

<main class="shell">
  <aside class="sidebar" aria-label="Project source files">
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
          {#if scanLimitReached && !scanning}
            <button
              class="scan-more-button"
              type="button"
              aria-label={`Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}
              title={`Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}
              onclick={() => scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })}
            >
              <Plus size={13} strokeWidth={2} />
              <span>Scan 5K</span>
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
  </aside>

  <section class="workspace" aria-label="Source preview">
    <header class="topbar">
      <div>
        <p class="eyebrow">Source Preview</p>
        <h2>{preview?.fileName ?? 'No file selected'}</h2>
      </div>
      <div class="status-strip">
        <span class="project-git-pill" title={projectGitSummary}>{projectGitSummary}</span>
        <span>{runtime}</span>
        {#if preview}
          <span>{preview.language}</span>
          <span>{preview.lineCount} lines</span>
        {/if}
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

    <section class="runtime-context-panel" aria-label="Runtime contexts">
      <div class="runtime-context-header">
        <div>
          <strong>Runtime Contexts</strong>
          <span>{runtimeContextSummary}</span>
        </div>
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
      </div>
      {#if selectedProjectRuntimeContexts.length > 0}
        <div class="runtime-context-list">
          {#each selectedProjectRuntimeContexts as context (`${context.pid}:${context.port}:${context.cwd}`)}
            <div class="runtime-context-row">
              <span class="runtime-port">:{context.port}</span>
              <strong>{context.command}</strong>
              <span>{context.rootLabel}</span>
              <small title={context.cwd}>{context.cwd}</small>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="worktree-context-panel" aria-label="Worktree safety">
      <div class="worktree-context-header">
        <div>
          <strong>Worktree Safety</strong>
          <span>{projectWorktreeSummary}</span>
        </div>
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
      </div>
      {#if projectWorktrees.length > 0}
        <div class="worktree-context-list">
          {#each projectWorktrees as worktree (worktree.path)}
            {@const eligibilityKind = projectWorktreeEligibilityKind(worktree)}
            <div class="worktree-context-row" class:blocked={eligibilityKind === 'blocked'}>
              <span class="worktree-status-badge">{eligibilityKind === 'blocked' ? 'Blocked' : 'Ready'}</span>
              <strong>{worktree.branch}</strong>
              <span>{worktree.repo}</span>
              <small title={worktree.path}>{worktree.path}</small>
              <em>{worktree.deleteEligibility}</em>
            </div>
          {/each}
        </div>
      {/if}
    </section>

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
        <div class="file-actions" aria-label="Source file actions">
          <button class="file-action-button" type="button" aria-label="Copy source path" title="Copy source path" disabled={fileActionBusy === 'copy'} onclick={copySelectedPath}>
            {#if fileActionStatus === 'Path copied'}
              <Check size={14} strokeWidth={2} />
            {:else}
              <Copy size={14} strokeWidth={1.9} />
            {/if}
          </button>
          <button class="file-action-button" type="button" aria-label="Open source file" title="Open source file" disabled={fileActionBusy === 'open'} onclick={openSelectedFile}>
            <ExternalLink size={14} strokeWidth={1.9} />
          </button>
          <button class="file-action-button" type="button" aria-label="Reveal source file" title="Reveal source file" disabled={fileActionBusy === 'reveal'} onclick={revealSelectedFile}>
            <FolderSearch size={14} strokeWidth={1.9} />
          </button>
        </div>
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
          <div class="traffic">
            <span></span>
            <span></span>
            <span></span>
          </div>
          <div class="mode-pill">
            <SplitSquareHorizontal size={14} strokeWidth={1.8} />
            <span>{selectedSourceDirty ? 'Modified' : 'Editable'}</span>
          </div>
          <div class="editor-save-actions">
            <button
              class="editor-action-button primary"
              type="button"
              aria-label="Save source file"
              title="Save source file"
              disabled={!selectedSourceDirty || fileActionBusy === 'save'}
              onclick={saveSelectedSourceFile}
            >
              <Save size={13} strokeWidth={2} />
              <span>Save</span>
            </button>
            <button
              class="editor-action-button"
              type="button"
              aria-label="Revert source file"
              title="Revert source file"
              disabled={!selectedSourceDirty || fileActionBusy === 'save'}
              onclick={revertSelectedSourceFile}
            >
              <RotateCcw size={13} strokeWidth={2} />
              <span>Revert</span>
            </button>
          </div>
          <div class="editor-intelligence-actions">
            <button
              class="editor-action-button"
              type="button"
              aria-label="Show hover"
              title="Show hover"
              disabled={!sourceIntelligenceAvailable}
              onclick={() => requestSourceIntelligenceAction('hover')}
            >
              <SplitSquareHorizontal size={13} strokeWidth={2} />
              <span>Hover</span>
            </button>
            <button
              class="editor-action-button"
              type="button"
              aria-label="Go to definition"
              title="Go to definition"
              disabled={!preview || loading}
              onclick={() => requestSourceIntelligenceAction('definition')}
            >
              <Search size={13} strokeWidth={2} />
              <span>Definition</span>
            </button>
            <button
              class="editor-action-button"
              type="button"
              aria-label="Find references"
              title="Find references"
              disabled={!preview || loading}
              onclick={() => requestSourceIntelligenceAction('references')}
            >
              <Braces size={13} strokeWidth={2} />
              <span>References</span>
            </button>
          </div>
          <div class="quality-pill">
            <span>{sourcePreviewAppearance.theme.id}</span>
          </div>
          <div class="quality-pill font-pill">
            <span>{sourcePreviewAppearance.fontFamily.split(',')[0].replaceAll('"', '')}</span>
          </div>
        </div>

        <div class="editor-body-grid">
          {#key sourcePreviewAppearanceKey}
            <MonacoSourceEditor
              {preview}
              content={selectedSourceDraftContent}
              editable={true}
              {loading}
              targetLine={selectedSourceLine}
              targetLineRequestId={selectedSourceLineRequestId}
              intelligenceCommand={sourceIntelligenceCommand}
              onContentChange={updateSelectedSourceDraft}
              onDiagnosticsChange={handleEditorDiagnosticsChange}
              onDefinitionLookup={handleEditorDefinitionLookup}
              onReferenceLookup={handleEditorReferenceLookup}
              onSymbolsChange={handleEditorSymbolsChange}
            />
          {/key}

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
        </div>
      </div>
    {:else}
      <div class="empty-preview">
        <FileCode2 size={34} strokeWidth={1.55} />
        <strong>No source file loaded</strong>
        <span>Scan a project or choose a file from the tree.</span>
      </div>
    {/if}
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

<style>
  .shell {
    display: grid;
    grid-template-columns: 340px minmax(0, 1fr);
    gap: 1px;
    width: min(1180px, calc(100vw - 32px));
    height: min(760px, calc(100dvh - 32px));
    margin: 16px auto;
    overflow: hidden;
    border: 1px solid rgba(231, 238, 235, 0.12);
    border-radius: 18px;
    background: rgba(24, 26, 26, 0.92);
    box-shadow:
      0 32px 90px rgba(0, 0, 0, 0.32),
      inset 0 1px 0 rgba(255, 255, 255, 0.08);
  }

  .sidebar {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    min-width: 0;
    overflow: hidden;
    padding: 22px 18px;
    background: rgba(19, 21, 21, 0.94);
    border-right: 1px solid rgba(255, 255, 255, 0.08);
  }

  .brand-row {
    display: grid;
    grid-template-columns: 42px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    margin-bottom: 22px;
  }

  .brand-mark {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
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
    font-size: 21px;
  }

  h2 {
    font-size: 28px;
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
    min-width: 0;
    padding: 26px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent 36%),
      rgba(24, 26, 26, 0.96);
  }

  .topbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 18px;
    margin-bottom: 14px;
  }

  .status-strip {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .status-strip span,
  .mode-pill,
  .quality-pill {
    display: inline-grid;
    grid-auto-flow: column;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    color: #b9c5c1;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.045);
    font-size: 11px;
    font-weight: 700;
  }

  .project-git-pill {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .context-identity-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    min-width: 0;
    overflow: hidden;
    margin: -2px 0 14px;
  }

  .context-identity-pill {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 34px;
    padding: 0 10px;
    color: #cbd3d1;
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 8px;
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
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .context-identity-pill strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 760;
  }

  .runtime-context-panel,
  .worktree-context-panel {
    display: grid;
    gap: 8px;
    min-width: 0;
    padding: 10px;
    margin: -4px 0 14px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.035);
  }

  .runtime-context-header,
  .worktree-context-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .runtime-context-header div,
  .worktree-context-header div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .runtime-context-header strong,
  .runtime-context-header span,
  .worktree-context-header strong,
  .worktree-context-header span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .runtime-context-header strong,
  .worktree-context-header strong {
    color: #f0f4f3;
    font-size: 12px;
    font-weight: 800;
  }

  .runtime-context-header span,
  .worktree-context-header span {
    color: #8d9995;
    font-size: 10px;
    font-weight: 740;
  }

  .runtime-context-list,
  .worktree-context-list {
    display: grid;
    gap: 5px;
    max-height: 108px;
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

  .worktree-context-list {
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .runtime-context-row,
  .worktree-context-row {
    display: grid;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 7px 8px;
    color: #cbd3d1;
    border-radius: 7px;
    background: rgba(0, 0, 0, 0.14);
  }

  .runtime-context-row {
    grid-template-columns: auto minmax(0, 0.8fr) minmax(0, 0.75fr) minmax(0, 1.6fr);
  }

  .worktree-context-row {
    grid-template-columns: auto minmax(0, 0.9fr) minmax(0, 0.7fr) minmax(0, 1.5fr) minmax(0, 1fr);
  }

  .worktree-context-row.blocked {
    background: rgba(216, 170, 85, 0.09);
  }

  .runtime-port,
  .worktree-status-badge,
  .runtime-context-row strong,
  .runtime-context-row span,
  .runtime-context-row small,
  .worktree-context-row strong,
  .worktree-context-row span,
  .worktree-context-row small,
  .worktree-context-row em {
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

  .worktree-context-row.blocked .worktree-status-badge {
    color: #211606;
    background: #d8aa55;
  }

  .runtime-context-row strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 780;
  }

  .worktree-context-row strong {
    color: #f0f4f3;
    font-size: 11px;
    font-weight: 780;
  }

  .runtime-context-row span,
  .runtime-context-row small,
  .worktree-context-row span,
  .worktree-context-row small,
  .worktree-context-row em {
    color: #8d9995;
    font-size: 10px;
    font-style: normal;
    font-weight: 720;
  }

  .tab-strip {
    display: flex;
    gap: 6px;
    min-width: 0;
    padding-bottom: 4px;
    margin: -2px 0 12px;
    overflow-x: auto;
  }

  .source-tab {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 28px;
    align-items: center;
    flex: 0 1 228px;
    min-width: 148px;
    max-width: 228px;
    height: 36px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9px;
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
    grid-template-columns: 16px minmax(0, 1fr) auto auto 8px;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 100%;
    padding: 0 7px 0 9px;
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
    font-size: 12px;
    font-weight: 760;
  }

  .tab-select-button small {
    color: #8d9995;
    font-size: 10px;
    font-weight: 760;
  }

  .tab-close-button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    color: #8d9995;
    border: 0;
    border-radius: 7px;
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
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
    color: #87918e;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
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
    grid-template-rows: 42px minmax(0, 1fr);
    height: 560px;
    min-height: 0;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 13px;
    background: #17191e;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.07),
      0 18px 45px rgba(0, 0, 0, 0.2);
  }

  .editor-toolbar {
    display: grid;
    grid-template-columns: auto auto auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
  }

  .traffic {
    display: grid;
    grid-template-columns: repeat(3, 10px);
    gap: 6px;
  }

  .traffic span {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: #59635f;
  }

  .traffic span:nth-child(1) {
    background: #e16d5d;
  }

  .traffic span:nth-child(2) {
    background: #d8aa55;
  }

  .traffic span:nth-child(3) {
    background: #67c17d;
  }

  .quality-pill {
    color: #7ce5d5;
  }

  .editor-body-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    min-height: 0;
  }

  .editor-save-actions,
  .editor-intelligence-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .editor-action-button {
    display: grid;
    grid-template-columns: 14px minmax(0, auto);
    align-items: center;
    gap: 5px;
    height: 28px;
    min-width: 0;
    padding: 0 9px;
    color: #b9c5c1;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.045);
    font-size: 11px;
    font-weight: 760;
    cursor: pointer;
  }

  .editor-action-button.primary {
    color: #071b18;
    border-color: rgba(111, 223, 207, 0.68);
    background: #6fdfcf;
  }

  .editor-action-button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-action-button:hover:not(:disabled) {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.1);
  }

  .editor-action-button.primary:hover:not(:disabled) {
    color: #071b18;
    background: #81eadc;
  }

  .editor-action-button:focus-visible {
    border-color: rgba(92, 226, 207, 0.58);
    outline: 0;
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
  }

  .editor-action-button:disabled {
    cursor: default;
    opacity: 0.52;
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

  .intelligence-tabs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    padding: 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .intelligence-tabs button {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto;
    align-items: center;
    gap: 5px;
    height: 28px;
    min-width: 0;
    padding: 0 7px;
    color: #9fa9a6;
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 10px;
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

    .sidebar {
      border-right: 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }

    .topbar {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .context-identity-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .runtime-context-row {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .worktree-context-row {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .editor-frame {
      height: 520px;
    }

    .editor-body-grid {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(0, 1fr) 148px;
    }

    .source-intelligence-panel {
      border-left: 0;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }
  }
</style>
