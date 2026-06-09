<script lang="ts">
  import {
    Activity,
    Braces,
    ChevronRight,
    FileCode2,
    FolderGit2,
    Search,
    Sparkles,
    SplitSquareHorizontal
  } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { demoPreviewFor, sourceRecords, type SourcePreview, type SourceRecord } from '$lib/sourceData';
  import { readSourceFromTauri } from '$lib/tauriSource';

  let selectedRecord = $state<SourceRecord>(sourceRecords[0]);
  let preview = $state<SourcePreview>(demoPreviewFor(sourceRecords[0]));
  let highlighted = $state('');
  let query = $state('');
  let loading = $state(true);
  let runtime = $state('browser preview');
  let error = $state('');

  let filteredRecords = $derived(sourceRecords.filter((record) => {
    const value = query.trim().toLowerCase();
    if (!value) return true;
    return `${record.relativePath} ${record.fileName} ${record.language}`.toLowerCase().includes(value);
  }));

  let selectedIndex = $derived(sourceRecords.findIndex((record) => record.path === selectedRecord.path) + 1);

  async function renderPreview(nextPreview: SourcePreview) {
    const { codeToHtml } = await import('shiki');
    highlighted = await codeToHtml(nextPreview.content, {
      lang: nextPreview.language,
      theme: 'github-dark-default'
    });
  }

  async function selectRecord(record: SourceRecord) {
    selectedRecord = record;
    await loadRecord(record);
  }

  async function loadRecord(record: SourceRecord) {
    loading = true;
    error = '';

    try {
      const tauriPreview = await readSourceFromTauri(record);
      runtime = tauriPreview ? 'tauri file read' : 'browser preview';
      preview = tauriPreview ?? demoPreviewFor(record);
      await renderPreview(preview);
    } catch (previewError) {
      runtime = 'browser preview';
      error = previewError instanceof Error ? previewError.message : 'Could not read source file';
      preview = demoPreviewFor(record);
      await renderPreview(preview);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void loadRecord(selectedRecord);
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
        <h1>Webview Source</h1>
      </div>
    </div>

    <label class="search-box">
      <Search size={16} strokeWidth={1.8} />
      <input bind:value={query} placeholder="Filter source files" />
    </label>

    <div class="tree-heading">
      <FolderGit2 size={15} strokeWidth={1.8} />
      <span>EdiPlatform</span>
      <strong>{sourceRecords.length}</strong>
    </div>

    <div class="file-list">
      {#each filteredRecords as record}
        <button
          class:active={record.path === selectedRecord.path}
          type="button"
          onclick={() => selectRecord(record)}
        >
          <FileCode2 size={16} strokeWidth={1.8} />
          <span>
            <strong>{record.fileName}</strong>
            <small>{record.relativePath}</small>
          </span>
          <ChevronRight size={15} strokeWidth={1.8} />
        </button>
      {/each}
    </div>
  </aside>

  <section class="workspace" aria-label="Source preview">
    <header class="topbar">
      <div>
        <p class="eyebrow">Source Preview</p>
        <h2>{preview.fileName}</h2>
      </div>
      <div class="status-strip">
        <span>{runtime}</span>
        <span>{preview.language}</span>
        <span>{preview.lineCount} lines</span>
      </div>
    </header>

    <div class="path-row">
      <span>{preview.relativePath}</span>
      <strong>{selectedIndex} / {sourceRecords.length}</strong>
    </div>

    {#if error}
      <div class="inline-error">
        <Activity size={15} strokeWidth={1.8} />
        <span>{error}</span>
      </div>
    {/if}

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
          <Sparkles size={14} strokeWidth={1.8} />
          <span>Shiki</span>
        </div>
      </div>

      {#if loading}
        <div class="skeleton-code" aria-label="Loading source preview">
          {#each Array.from({ length: 13 }) as _, index}
            <span style={`--line-width: ${index % 4 === 0 ? 48 : index % 3 === 0 ? 66 : 86}%`}></span>
          {/each}
        </div>
      {:else}
        <div class="code-scroll" data-testid="highlighted-source">
          {@html highlighted}
        </div>
      {/if}
    </div>
  </section>
</main>

<style>
  .shell {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
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
    font-size: 22px;
  }

  h2 {
    font-size: 28px;
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

  .file-list {
    display: grid;
    gap: 4px;
  }

  .file-list button {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) 16px;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 50px;
    padding: 8px 10px;
    color: #cbd3d1;
    text-align: left;
    border-radius: 8px;
    background: transparent;
    transition:
      background 150ms ease,
      color 150ms ease,
      transform 150ms ease;
  }

  .file-list button:hover,
  .file-list button.active {
    color: #f2f6f5;
    background: rgba(92, 226, 207, 0.12);
  }

  .file-list button:active {
    transform: translateY(1px);
  }

  .file-list strong,
  .file-list small {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-list strong {
    font-size: 13px;
  }

  .file-list small {
    margin-top: 2px;
    color: #7f8b87;
    font-size: 10px;
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
    background: #101212;
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

  .code-scroll {
    height: calc(100% - 42px);
    overflow: auto;
    background: #111313;
  }

  .code-scroll :global(.shiki) {
    min-width: max-content;
    min-height: 100%;
    margin: 0;
    padding: 18px 0 22px;
    background: transparent !important;
    color: #dce5e2;
    font-family:
      "SF Mono",
      ui-monospace,
      Menlo,
      Monaco,
      Consolas,
      monospace;
    font-size: 13px;
    line-height: 1.58;
    letter-spacing: 0;
    tab-size: 4;
  }

  .code-scroll :global(.shiki code) {
    counter-reset: line;
  }

  .code-scroll :global(.shiki .line) {
    display: block;
    min-height: 20px;
    padding-right: 28px;
    white-space: pre;
    counter-increment: line;
  }

  .code-scroll :global(.shiki .line::before) {
    display: inline-block;
    width: 46px;
    margin-right: 16px;
    color: #52605c;
    text-align: right;
    content: counter(line);
    user-select: none;
  }

  .skeleton-code {
    display: grid;
    gap: 12px;
    padding: 22px;
  }

  .skeleton-code span {
    display: block;
    width: var(--line-width);
    height: 13px;
    border-radius: 999px;
    background: linear-gradient(90deg, #242929, #333a38, #242929);
    background-size: 180% 100%;
    animation: shimmer 1.2s ease-in-out infinite;
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
