<script lang="ts">
  /**
   * GitDiffView.svelte — what changed in the file the source-control panel has
   * selected.
   *
   * Self-contained by the shell's panel contract: no required props, no IO of
   * its own. It reads `gitPanel` and renders it; every backend call belongs to
   * `gitService`, which the panel and the integrator drive. Nothing is loaded or
   * measured while hidden, so it is safe inside a parked (display:none) host.
   *
   * Text files use CodeMirror's merge view with full bounded models supplied
   * by Rust when side-by-side mode is selected. Unified text is the default and
   * remains the fallback when full models are unavailable or the tab is hidden.
   */
  import { gitPanel } from '$lib/shell/git/gitPanelStore.svelte';
  import { parseUnifiedDiff, summarizeParsedDiff } from '$lib/shell/git/parseUnifiedDiff';
  import type { DiffMode } from '$lib/shell/sessionWorkspaces';
  import type CodeMirrorGitDiffEditor from '$lib/shell/components/git/CodeMirrorGitDiffEditor.svelte';
  import { requestOpenFile } from '$lib/shell/openFileBus';

  interface Props {
    /** Whether the Diff tab is the center tab in front. A session that
     * remembered a diff has it put back at launch, and CodeMirror must not start
     * for a comparison nobody is looking at. */
    showing?: boolean;
    rootAvailable?: boolean;
    mode?: DiffMode;
    onModeChange?: (mode: DiffMode) => void;
  }
  let { showing = false, rootAvailable = true, mode = 'unified', onModeChange }: Props = $props();

  type DiffEditorComponent = typeof CodeMirrorGitDiffEditor;
  let DiffEditor = $state<DiffEditorComponent | null>(null);
  let diffEditorLoadError = $state<string | null>(null);
  let loadingDiffEditor = false;

  /** Long diffs are trimmed so one huge file cannot stall the panel. */
  const MAX_RENDERED_LINES = 2000;

  const diff = $derived(gitPanel.selectedDiff);
  const parsed = $derived.by(() => {
    if (!diff) return null;

    let unified = diff.diff;
    if (!unified && diff.status === 'untracked' && diff.modifiedContent) {
      const lines = diff.modifiedContent.replace(/\r\n/g, '\n').split('\n');
      if (lines.at(-1) === '') lines.pop();
      unified = [
        `diff --git a/${diff.relativePath} b/${diff.relativePath}`,
        'new file mode 100644',
        '--- /dev/null',
        `+++ b/${diff.relativePath}`,
        `@@ -0,0 +1,${lines.length} @@`,
        ...lines.map((line) => `+${line}`)
      ].join('\n');
    }

    return parseUnifiedDiff(unified);
  });
  const summary = $derived(parsed ? summarizeParsedDiff(parsed) : '');
  const hasNativeModels = $derived(
    diff?.originalContent !== null &&
      diff?.originalContent !== undefined &&
      diff?.modifiedContent !== null &&
      diff?.modifiedContent !== undefined &&
      Boolean(gitPanel.root)
  );
  const renderedLineCount = $derived(
    parsed ? parsed.hunks.reduce((total, hunk) => total + hunk.lines.length, 0) : 0
  );
  const trimmed = $derived(renderedLineCount > MAX_RENDERED_LINES);

  async function ensureDiffEditor(): Promise<void> {
    if (DiffEditor || loadingDiffEditor) return;
    loadingDiffEditor = true;
    diffEditorLoadError = null;
    try {
      DiffEditor = (await import('$lib/shell/components/git/CodeMirrorGitDiffEditor.svelte')).default;
    } catch (error) {
      diffEditorLoadError = error instanceof Error ? error.message : String(error);
    } finally {
      loadingDiffEditor = false;
    }
  }

  $effect(() => {
    if (showing && mode === 'side-by-side' && hasNativeModels) void ensureDiffEditor();
  });

  /** Hunks cut down to the render cap, in order. */
  const sections = $derived.by(() => {
    if (!parsed) return [];
    let budget = MAX_RENDERED_LINES;
    return parsed.sections.map((section) => ({
      label: section.label,
      hunks: section.hunks.map((hunk) => {
        const take = Math.max(0, Math.min(hunk.lines.length, budget));
        budget -= take;
        return { ...hunk, lines: hunk.lines.slice(0, take) };
      })
    }));
  });

  function lineNumber(value: number | null): string {
    return value === null ? '' : String(value);
  }

  function hunkRange(hunk: { beforeStart: number; beforeCount: number; afterStart: number; afterCount: number }): string {
    const start = hunk.beforeCount === 0 ? hunk.afterStart : hunk.beforeStart;
    const count = hunk.beforeCount === 0 ? hunk.afterCount : hunk.beforeCount;
    return `${start}–${start + Math.max(count - 1, 0)}`;
  }

  /**
   * Open the changed file itself, at the line that was clicked.
   *
   * The diff shows what changed; the editor is where it is read properly and,
   * if the project has language intelligence on, where its meaning is. Which
   * mode the file opens in is the project's own setting — nothing here starts
   * a language server.
   */
  function openAtLine(line: number | null): void {
    const root = gitPanel.root;
    const relativePath = diff?.relativePath ?? gitPanel.selectedPath;
    if (!rootAvailable || !root || !relativePath) return;
    const path = `${root.replace(/\/+$/, '')}/${relativePath.replace(/^\/+/, '')}`;
    requestOpenFile({
      path,
      projectRoot: root,
      line: line && line > 0 ? line : undefined
    });
  }

  /** The line a diff row points at in the file as it is now. */
  function currentLineOf(line: { afterLine: number | null; beforeLine: number | null }): number | null {
    return line.afterLine ?? line.beforeLine;
  }

  function marker(kind: string): string {
    if (kind === 'added') return '+';
    if (kind === 'removed') return '-';
    if (kind === 'note') return '';
    return ' ';
  }
</script>

<div class="diff-view" data-selectable="true">
  {#if !rootAvailable}
    <p class="notice">Checkout/Worktree deleted.</p>
  {:else if gitPanel.selectedPath === ''}
    <p class="notice">Pick a changed file to see what changed in it.</p>
  {:else}
    <header class="head">
      <span class="path" title={gitPanel.selectedPath}>{gitPanel.selectedPath}</span>
      {#if diff}
        <span class="summary">{diff.status} · {summary}</span>
      {/if}
    </header>

    {#if gitPanel.diffLoading}
      <p class="notice">Reading the changes…</p>
    {:else if gitPanel.diffError}
      <p class="notice error">{gitPanel.diffError}</p>
    {:else if !parsed}
      <p class="notice">No changes to show for this file.</p>
    {:else if parsed.isBinary}
      <p class="notice">This is a binary file, so there is no line-by-line comparison.</p>
    {:else if parsed.isEmpty}
      <p class="notice">This file has no line changes compared with the last commit.</p>
    {:else}
      {#if hasNativeModels}
        <div class="mode-row" role="group" aria-label="Diff view mode">
          <button
            type="button"
            class:active={mode === 'unified'}
            aria-pressed={mode === 'unified'}
            onclick={() => onModeChange?.('unified')}
          >Unified</button>
          <button
            type="button"
            class:active={mode === 'side-by-side'}
            aria-pressed={mode === 'side-by-side'}
            onclick={() => onModeChange?.('side-by-side')}
          >Side by side</button>
        </div>
      {/if}
      {#if showing && mode === 'side-by-side' && hasNativeModels && diff && gitPanel.root}
        <div class="native-body">
        {#if DiffEditor}
          <DiffEditor
            root={gitPanel.root}
            relativePath={diff.relativePath}
            originalContent={diff.originalContent ?? ''}
            modifiedContent={diff.modifiedContent ?? ''}
            onOpenLine={openAtLine}
          />
        {:else if diffEditorLoadError}
          <p class="notice error">Could not start the diff editor: {diffEditorLoadError}</p>
        {:else}
          <p class="notice">Starting the diff editor…</p>
        {/if}
        </div>
      {:else}
        <div class="body">
        {#each sections as section, sectionIndex (sectionIndex)}
          {#if section.label}
            <p class="section-label">{section.label}</p>
          {/if}
          {#each section.hunks as hunk, hunkIndex (hunkIndex)}
            <button
              type="button"
              class="hunk-head"
              title={`${hunk.header} — open this file in the editor at line ${hunk.afterStart}`}
              onclick={() => openAtLine(hunk.afterStart)}
            >
              Lines {hunkRange(hunk)}
              {#if hunk.heading}<span class="hunk-heading">{hunk.heading}</span>{/if}
            </button>
            <div class="hunk">
              {#each hunk.lines as line, index (index)}
                <div
                  class="line {line.kind}"
                  role="presentation"
                  ondblclick={() => openAtLine(currentLineOf(line))}
                >
                  <span class="gutter">{lineNumber(line.beforeLine)}</span>
                  <span class="gutter">{lineNumber(line.afterLine)}</span>
                  <span class="marker">{marker(line.kind)}</span>
                  <span class="text">{line.text}</span>
                </div>
              {/each}
            </div>
          {/each}
        {/each}
        {#if trimmed}
          <p class="notice">
            Showing the first {MAX_RENDERED_LINES} lines of this diff, out of {renderedLineCount}.
          </p>
        {/if}
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .diff-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
    background: #101014;
    color: #d8d8e0;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid #22222c;
    flex: 0 0 auto;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 12px;
    color: #e6e6ee;
  }

  .summary {
    flex: 0 0 auto;
    color: #6d6d7d;
    font-size: 12px;
    white-space: nowrap;
  }

  .mode-row {
    display: flex;
    gap: 4px;
    padding: 6px 10px;
    border-bottom: 1px solid #22222c;
  }

  .mode-row button {
    padding: 3px 8px;
    border: 1px solid #30303c;
    border-radius: 4px;
    background: transparent;
    color: #8d8d9c;
    cursor: pointer;
    font: inherit;
  }

  .mode-row button.active {
    border-color: #6666a0;
    color: #e6e6ee;
  }

  .notice {
    margin: 0;
    padding: 10px;
    color: #6d6d7d;
    font-size: 12px;
  }

  .notice.error {
    color: #ff9d9d;
  }

  .body {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    padding-bottom: 10px;
  }

  .native-body {
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
  }

  .section-label {
    position: sticky;
    top: 0;
    margin: 0;
    padding: 5px 10px;
    background: #17171d;
    border-bottom: 1px solid #22222c;
    color: #9a9aad;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  /* A hunk heading is the way into the file: clicking it opens the editor at
   * that hunk's first line. It stays a quiet line of text until it is pointed
   * at, so the diff still reads as a diff. */
  .hunk-head {
    display: flex;
    gap: 8px;
    margin: 0;
    padding: 6px 10px 4px;
    width: 100%;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: #4c4c5a;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    text-align: left;
  }

  .hunk-head:hover {
    background: #17171d;
    color: #9a9aad;
  }

  .hunk-head:focus-visible {
    outline: 2px solid #5d5d6b;
    outline-offset: -2px;
  }

  .hunk-heading {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    color: #5d5d6b;
  }

  .hunk {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
  }

  .line {
    display: flex;
    gap: 0;
    white-space: pre;
  }

  .gutter {
    flex: 0 0 auto;
    width: 40px;
    padding-right: 6px;
    text-align: right;
    color: #3f3f4b;
    user-select: none;
  }

  .marker {
    flex: 0 0 auto;
    width: 14px;
    text-align: center;
    color: #5d5d6b;
    user-select: none;
  }

  .text {
    flex: 1 1 auto;
    padding-right: 10px;
    overflow-x: auto;
  }

  .line.added {
    background: rgba(80, 250, 123, 0.09);
    color: #a8e7bb;
  }

  .line.removed {
    background: rgba(255, 85, 85, 0.09);
    color: #f0a3a3;
  }

  .line.note {
    color: #4c4c5a;
    font-style: italic;
  }
</style>
