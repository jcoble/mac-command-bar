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
   * It renders the diff as text with the added and removed lines coloured, which
   * is the honest view of what the backend gives us: a unified diff carries only
   * a few lines of context around each change, so a side-by-side editor would be
   * showing fragments of the file with the wrong line numbers. The parsed
   * `before`/`after` strings are ready in `parseUnifiedDiff` for the day a
   * side-by-side view is added.
   */
  import { gitPanel } from '$lib/shell/git/gitPanelStore.svelte';
  import { parseUnifiedDiff, summarizeParsedDiff } from '$lib/shell/git/parseUnifiedDiff';

  /** Long diffs are trimmed so one huge file cannot stall the panel. */
  const MAX_RENDERED_LINES = 2000;

  const diff = $derived(gitPanel.selectedDiff);
  const parsed = $derived(diff ? parseUnifiedDiff(diff.diff) : null);
  const summary = $derived(parsed ? summarizeParsedDiff(parsed) : '');
  const renderedLineCount = $derived(
    parsed ? parsed.hunks.reduce((total, hunk) => total + hunk.lines.length, 0) : 0
  );
  const trimmed = $derived(renderedLineCount > MAX_RENDERED_LINES);

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

  function marker(kind: string): string {
    if (kind === 'added') return '+';
    if (kind === 'removed') return '-';
    if (kind === 'note') return '';
    return ' ';
  }
</script>

<div class="diff-view">
  {#if gitPanel.selectedPath === ''}
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
      <div class="body">
        {#each sections as section, sectionIndex (sectionIndex)}
          {#if section.label}
            <p class="section-label">{section.label}</p>
          {/if}
          {#each section.hunks as hunk, hunkIndex (hunkIndex)}
            <p class="hunk-head" title={hunk.header}>
              Lines {hunk.beforeStart}–{hunk.beforeStart + Math.max(hunk.beforeCount - 1, 0)}
              {#if hunk.heading}<span class="hunk-heading">{hunk.heading}</span>{/if}
            </p>
            <div class="hunk">
              {#each hunk.lines as line, index (index)}
                <div class="line {line.kind}">
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
    font-family: ui-monospace, Menlo, monospace;
    font-size: 11px;
    color: #e6e6ee;
  }

  .summary {
    flex: 0 0 auto;
    color: #6d6d7d;
    font-size: 10px;
    white-space: nowrap;
  }

  .notice {
    margin: 0;
    padding: 10px;
    color: #6d6d7d;
    font-size: 11px;
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

  .section-label {
    position: sticky;
    top: 0;
    margin: 0;
    padding: 5px 10px;
    background: #17171d;
    border-bottom: 1px solid #22222c;
    color: #9a9aad;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .hunk-head {
    display: flex;
    gap: 8px;
    margin: 0;
    padding: 6px 10px 4px;
    color: #4c4c5a;
    font-size: 10px;
  }

  .hunk-heading {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, Menlo, monospace;
    color: #5d5d6b;
  }

  .hunk {
    font-family: ui-monospace, Menlo, monospace;
    font-size: 11px;
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
