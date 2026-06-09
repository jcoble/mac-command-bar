<script lang="ts">
  import {
    Activity,
    Braces,
    ChevronDown,
    ChevronRight,
    FileCode2,
    Folder,
    FolderGit2,
    FolderOpen,
    RefreshCw,
    Search,
    SplitSquareHorizontal
  } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import { sourcePreviewAppearance, sourcePreviewAppearanceKey } from '$lib/sourcePreviewAppearance';
  import {
    buildSourceTree,
    demoPreviewFor,
    demoRecordsForProject,
    filterSourceRecords,
    flattenSourceTree,
    previewFromContent,
    projectRoots,
    type ProjectRoot,
    type SourcePreview,
    type SourceRecord,
    type SourceTreeNode
  } from '$lib/sourceData';
  import { listSourceFilesFromTauri, readSourceFromTauri } from '$lib/tauriSource';

  const initialRecords = demoRecordsForProject(projectRoots[0]);

  let selectedProjectID = $state(projectRoots[0].id);
  let records = $state<SourceRecord[]>(initialRecords);
  let selectedRecord = $state<SourceRecord | null>(initialRecords[0] ?? null);
  let preview = $state<SourcePreview | null>(
    initialRecords[0] ? demoPreviewFor(initialRecords[0]) : null
  );
  let query = $state('');
  let expandedFolderIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let scanning = $state(false);
  let runtime = $state('browser preview');
  let error = $state('');
  let scanGeneration = 0;

  let selectedProject = $derived(
    projectRoots.find((project) => project.id === selectedProjectID) ?? projectRoots[0]
  );
  let filteredRecords = $derived(filterSourceRecords(records, query));
  let sourceTree = $derived(buildSourceTree(filteredRecords));
  let autoExpandFolders = $derived(query.trim().length > 0);
  let visibleTreeRows = $derived(flattenSourceTree(sourceTree, expandedFolderIds, autoExpandFolders));
  let selectedIndex = $derived(
    selectedRecord ? records.findIndex((record) => record.path === selectedRecord?.path) + 1 : 0
  );
  let recordCountLabel = $derived(
    filteredRecords.length === records.length
      ? `${records.length}`
      : `${filteredRecords.length} / ${records.length}`
  );

  async function scanProject(project: ProjectRoot) {
    const generation = ++scanGeneration;
    scanning = true;
    loading = true;
    error = '';
    runtime = 'scanning source files';

    try {
      const tauriRecords = await listSourceFilesFromTauri(project.path);
      if (generation !== scanGeneration) return;

      const nextRecords = tauriRecords ?? demoRecordsForProject(project);
      records = nextRecords;
      expandedFolderIds = new Set();
      runtime = tauriRecords ? 'tauri source scan' : 'browser preview';

      const nextSelection = nextRecords[0] ?? null;
      selectedRecord = nextSelection;
      preview = nextSelection ? previewFromContent(nextSelection, '') : null;

      if (nextSelection) {
        await loadRecord(nextSelection);
      } else {
        loading = false;
      }
    } catch (scanError) {
      if (generation !== scanGeneration) return;
      records = demoRecordsForProject(project);
      selectedRecord = records[0] ?? null;
      preview = selectedRecord ? demoPreviewFor(selectedRecord) : null;
      runtime = 'browser preview';
      error = scanError instanceof Error ? scanError.message : 'Could not scan source files';
      loading = false;
    } finally {
      if (generation === scanGeneration) {
        scanning = false;
      }
    }
  }

  async function loadRecord(record: SourceRecord) {
    loading = true;
    error = '';

    try {
      const tauriPreview = await readSourceFromTauri(record);
      runtime = tauriPreview ? 'tauri file read' : 'browser preview';
      preview = tauriPreview ?? demoPreviewFor(record);
    } catch (previewError) {
      runtime = 'browser preview';
      error = previewError instanceof Error ? previewError.message : 'Could not read source file';
      preview = demoPreviewFor(record);
    } finally {
      loading = false;
    }
  }

  async function selectRecord(record: SourceRecord) {
    selectedRecord = record;
    await loadRecord(record);
  }

  async function handleProjectChange() {
    await scanProject(selectedProject);
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
    void scanProject(selectedProject);
  });
</script>

<svelte:head>
  <title>MacCommandBar Webview Preview</title>
</svelte:head>

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

    <div class="project-row">
      <select bind:value={selectedProjectID} onchange={handleProjectChange} aria-label="Project">
        {#each projectRoots as project}
          <option value={project.id}>{project.name}</option>
        {/each}
      </select>
      <button class="scan-button" type="button" disabled={scanning} onclick={() => scanProject(selectedProject)}>
        <RefreshCw size={15} strokeWidth={1.8} />
        <span>{scanning ? 'Scanning' : 'Scan'}</span>
      </button>
    </div>

    <label class="search-box">
      <Search size={16} strokeWidth={1.8} />
      <input bind:value={query} placeholder="Filter source files" />
    </label>

    <div class="tree-heading">
      <FolderGit2 size={15} strokeWidth={1.8} />
      <span>{selectedProject.name}</span>
      <strong>{recordCountLabel}</strong>
    </div>

    <div class="file-tree" aria-label="Files in selected project">
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
        {#each visibleTreeRows as row (row.node.id)}
          {@const node = row.node}
          {@const isFolder = node.file === null}
          {@const isExpanded = isFolderExpanded(node)}
        <button
          class:active={!isFolder && node.file?.path === selectedRecord?.path}
          class:folder-row={isFolder}
          class:file-row={!isFolder}
          type="button"
          style={`--tree-level: ${row.level}`}
          title={node.relativePath}
          aria-expanded={isFolder ? isExpanded : undefined}
          onclick={() => selectTreeNode(node)}
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
          <small>{isFolder ? node.children.length : node.file?.language}</small>
        </button>
        {/each}
      {/if}
    </div>
  </aside>

  <section class="workspace" aria-label="Source preview">
    <header class="topbar">
      <div>
        <p class="eyebrow">Source Preview</p>
        <h2>{preview?.fileName ?? 'No file selected'}</h2>
      </div>
      <div class="status-strip">
        <span>{runtime}</span>
        {#if preview}
          <span>{preview.language}</span>
          <span>{preview.lineCount} lines</span>
        {/if}
      </div>
    </header>

    {#if preview}
      <div class="path-row">
        <span>{preview.relativePath}</span>
        <strong>{selectedIndex} / {records.length}</strong>
      </div>
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
            <span>Read only</span>
          </div>
          <div class="quality-pill">
            <span>{sourcePreviewAppearance.theme.id}</span>
          </div>
          <div class="quality-pill font-pill">
            <span>{sourcePreviewAppearance.fontFamily.split(',')[0].replaceAll('"', '')}</span>
          </div>
        </div>

        {#key sourcePreviewAppearanceKey}
          <MonacoSourceEditor {preview} {loading} />
        {/key}
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
    grid-template-rows: auto auto auto auto minmax(0, 1fr);
    min-width: 0;
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

  .project-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 96px;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  select,
  .scan-button {
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
  .scan-button:focus-visible {
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

  .search-box {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    margin-bottom: 18px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9px;
    color: #9aa5a1;
    background: rgba(255, 255, 255, 0.045);
  }

  .search-box:focus-within {
    border-color: rgba(92, 226, 207, 0.58);
    box-shadow: 0 0 0 3px rgba(92, 226, 207, 0.13);
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

  .file-tree {
    min-height: 0;
    overflow: auto;
    padding: 4px 2px 4px 0;
  }

  .file-tree button {
    display: grid;
    grid-template-columns: calc(var(--tree-level, 0) * 14px) 14px 18px minmax(0, 1fr) auto;
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

  .path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
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
    grid-template-columns: auto minmax(0, 1fr) auto;
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

    .editor-frame {
      height: 520px;
    }
  }
</style>
