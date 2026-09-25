<script lang="ts">
  import { untrack } from 'svelte';
  import { hydrate, knownRoots } from '$lib/shell/newSession/projectRootsStore.svelte';
  import {
    listGithubPullRequestsFromTauri,
    readGithubPullRequestFromTauri,
    readGithubPullRequestFileFromTauri,
    replyGithubPullRequestCommentFromTauri,
    runHelperJobFromTauri,
    submitGithubPullRequestReviewFromTauri,
    type GithubFileVersions,
    type GithubReviewSubmission,
    type GithubPullRequestDetail,
    type GithubPullRequestSummary,
    type GithubPullRequestQuery
  } from '$lib/tauriSource';
  import { parseUnifiedDiff } from '$lib/shell/git/parseUnifiedDiff';
  import { openPullRequestDiff } from '$lib/shell/workbenchNavigation';
  import ConversationMessage from '$lib/shell/components/conversation/ConversationMessage.svelte';
  import type CodeMirrorGitDiffEditor from '$lib/shell/components/git/CodeMirrorGitDiffEditor.svelte';

  let { showing }: { showing: boolean } = $props();

  let mode = $state<GithubPullRequestQuery['mode']>('open');
  let projectFilter = $state('');
  let searchInput = $state('');
  let search = $state('');
  let items = $state<GithubPullRequestSummary[]>([]);
  let selected = $state<GithubPullRequestSummary | null>(null);
  let detail = $state<GithubPullRequestDetail | null>(null);
  let detailTab = $state<'conversation' | 'checks' | 'files'>('conversation');
  let detailLoading = $state(false);
  let detailError = $state<string | null>(null);
  let selectedFilePath = $state('');
  let diffMode = $state<'unified' | 'side-by-side'>('unified');
  let fileVersions = $state<GithubFileVersions | null>(null);
  let fileLoading = $state(false);
  let fileError = $state<string | null>(null);
  let openingDiff = $state(false);
  let DiffEditor = $state<typeof CodeMirrorGitDiffEditor | null>(null);
  let draftSummary = $state('');
  let draftLines = $state<Array<{ path: string; line: number; side: 'LEFT' | 'RIGHT'; body: string }>>([]);
  let lineTarget = $state<{ path: string; line: number; side: 'LEFT' | 'RIGHT' } | null>(null);
  let lineBody = $state('');
  let draftHeadSha = $state('');
  let pendingReview = $state<GithubReviewSubmission | null>(null);
  let posting = $state(false);
  let postUncertain = $state(false);
  let postedUrl = $state('');
  let helperDrafting = $state(false);
  let replyTarget = $state<GithubPullRequestDetail['comments'][number] | null>(null);
  let replyBody = $state('');
  let pendingReply = $state<{ commentId: number; body: string; headSha: string } | null>(null);
  let cursor = $state<string | null>(null);
  let totalCount = $state(0);
  let loading = $state(false);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let generation = 0;
  let detailGeneration = 0;
  let fileGeneration = 0;

  const roots = $derived(knownRoots());
  const selectedFile = $derived(detail?.files.find((file) => file.path === selectedFilePath) ?? null);
  const parsedFile = $derived(selectedFile?.patch ? parseUnifiedDiff(selectedFile.patch) : null);
  const headerState = $derived(prState(detail ?? selected));

  function prState(pr: { isDraft: boolean; state: string } | null): { label: string; tone: string } {
    if (!pr) return { label: '', tone: 'idle' };
    if (pr.isDraft) return { label: 'Draft', tone: 'idle' };
    const state = pr.state.toUpperCase();
    if (state === 'MERGED') return { label: 'Merged', tone: 'merged' };
    if (state === 'CLOSED') return { label: 'Closed', tone: 'bad' };
    return { label: 'Open', tone: 'good' };
  }

  function checkTone(check: GithubPullRequestDetail['checks'][number]): string {
    const value = (check.conclusion || check.status).toLowerCase();
    if (value === 'success' || value === 'neutral' || value === 'skipped') return 'good';
    if (/fail|error|cancel|timed_out|action_required/.test(value)) return 'bad';
    return 'attention';
  }

  function formatTime(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  }

  function fileName(path: string): string {
    return path.slice(path.lastIndexOf('/') + 1);
  }

  function fileDir(path: string): string {
    return path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
  }

  async function loadFileVersions(pr: GithubPullRequestDetail, file: GithubPullRequestDetail['files'][number]): Promise<void> {
    const current = ++fileGeneration;
    fileVersions = null;
    fileError = null;
    fileLoading = true;
    try {
      const [versions, editor] = await Promise.all([
        readGithubPullRequestFileFromTauri({
          root: pr.localRoot,
          path: file.path,
          previousPath: file.previousPath,
          status: file.status,
          baseSha: pr.baseSha,
          headSha: pr.headSha
        }),
        import('$lib/shell/components/git/CodeMirrorGitDiffEditor.svelte')
      ]);
      if (current !== fileGeneration) return;
      if (!versions) throw new Error('Full file versions are available in the desktop app.');
      DiffEditor = editor.default;
      fileVersions = versions;
    } catch (reason) {
      if (current === fileGeneration) fileError = reason instanceof Error ? reason.message : String(reason);
    } finally {
      if (current === fileGeneration) fileLoading = false;
    }
  }

  async function openFullDiff(): Promise<void> {
    if (!detail || !selectedFile || openingDiff) return;
    const pr = detail;
    const file = selectedFile;
    openingDiff = true;
    fileError = null;
    try {
      const versions = await readGithubPullRequestFileFromTauri({
        root: pr.localRoot,
        path: file.path,
        previousPath: file.previousPath,
        status: file.status,
        baseSha: pr.baseSha,
        headSha: pr.headSha
      });
      if (detail?.repository !== pr.repository || detail?.number !== pr.number || detail?.headSha !== pr.headSha || selectedFilePath !== file.path) return;
      if (!versions) throw new Error('Full file versions are available in the desktop app.');
      openPullRequestDiff({
        projectRoot: pr.localRoot,
        repository: pr.repository,
        number: pr.number,
        diff: {
          relativePath: file.path,
          status: file.status,
          diff: file.patch ?? '',
          isBinary: false,
          originalContent: versions.originalContent,
          modifiedContent: versions.modifiedContent
        }
      });
    } catch (reason) {
      fileError = reason instanceof Error ? reason.message : String(reason);
    } finally {
      openingDiff = false;
    }
  }

  async function loadDetail(root: string, number: number): Promise<void> {
    const current = ++detailGeneration;
    const previousHeadSha = detail?.headSha;
    detail = null;
    detailError = null;
    detailLoading = true;
    postUncertain = false;
    selectedFilePath = '';
    try {
      const result = await readGithubPullRequestFromTauri(root, number);
      if (current !== detailGeneration) return;
      if (!result) throw new Error('Pull request details are available in the desktop app.');
      detail = result;
      if (draftHeadSha && draftHeadSha !== result.headSha) {
        draftSummary = '';
        draftLines = [];
        lineTarget = null;
        lineBody = '';
        draftHeadSha = '';
        detailError = 'The PR changed. Earlier review drafts were cleared; inspect the new diff before commenting.';
      }
      if (replyTarget && previousHeadSha !== result.headSha) { replyTarget = null; replyBody = ''; pendingReply = null; }
      selectedFilePath = result.files[0]?.path ?? '';
    } catch (reason) {
      if (current === detailGeneration) detailError = reason instanceof Error ? reason.message : String(reason);
    } finally {
      if (current === detailGeneration) detailLoading = false;
    }
  }

  async function load(reset: boolean): Promise<void> {
    const current = ++generation;
    loading = true;
    error = null;
    try {
      await hydrate();
      const page = await listGithubPullRequestsFromTauri({
        roots: knownRoots().map((root) => root.path),
        mode,
        projectFilter: projectFilter || null,
        search: search || null,
        cursor: reset ? null : cursor,
        pageSize: 30
      });
      if (current !== generation) return;
      if (!page) throw new Error('Pull requests are available in the desktop app.');
      items = reset ? page.items : [...items, ...page.items];
      cursor = page.nextCursor;
      totalCount = page.totalCount;
      if (selected && !items.some((item) => item.repository === selected?.repository && item.number === selected?.number)) selected = null;
      if (!selected) selected = items[0] ?? null;
      loaded = true;
    } catch (reason) {
      if (current === generation) error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      if (current === generation) loading = false;
    }
  }

  function changeMode(next: GithubPullRequestQuery['mode']): void {
    if (mode === next) return;
    mode = next;
    void load(true);
  }

  function applySearch(event: SubmitEvent): void {
    event.preventDefault();
    search = searchInput.trim();
    void load(true);
  }

  function selectPullRequest(item: GithubPullRequestSummary): void {
    if (selected?.repository === item.repository && selected.number === item.number) return;
    draftSummary = '';
    draftLines = [];
    lineTarget = null;
    lineBody = '';
    draftHeadSha = '';
    replyTarget = null;
    replyBody = '';
    pendingReply = null;
    detailTab = 'conversation';
    selected = item;
  }

  function startLineDraft(path: string, beforeLine: number | null, afterLine: number | null): void {
    if (!detail || (afterLine === null && beforeLine === null)) return;
    lineTarget = { path, line: afterLine ?? beforeLine ?? 0, side: afterLine === null ? 'LEFT' : 'RIGHT' };
    lineBody = '';
  }

  function startSideBySideDraft(side: 'LEFT' | 'RIGHT', line: number): void {
    if (!selectedFile || !parsedFile?.hunks.some((hunk) => hunk.lines.some((row) =>
      side === 'LEFT' ? row.kind === 'removed' && row.beforeLine === line : row.kind === 'added' && row.afterLine === line
    ))) return;
    startLineDraft(selectedFile.path, side === 'LEFT' ? line : null, side === 'RIGHT' ? line : null);
  }

  function saveLineDraft(): void {
    if (!detail || !lineTarget || !lineBody.trim()) return;
    draftHeadSha = detail.headSha;
    draftLines = [...draftLines, { ...lineTarget, body: lineBody.trim() }];
    lineTarget = null;
    lineBody = '';
  }

  function cancelDrafts(): void {
    draftSummary = '';
    draftLines = [];
    lineTarget = null;
    lineBody = '';
    draftHeadSha = '';
    pendingReview = null;
  }

  async function draftWithHelper(target: 'overall' | 'line'): Promise<void> {
    if (!detail || helperDrafting || (target === 'line' && !lineTarget)) return;
    const pr = detail;
    const line = lineTarget;
    const context = target === 'line'
      ? `Target: ${line?.path}:${line?.line} (${line?.side})\n${pr.files.find((file) => file.path === line?.path)?.patch ?? 'No text patch available.'}`
      : `Target: overall review\n${pr.files.map((file) => `File: ${file.path}\n${file.patch ?? 'No text patch available.'}`).join('\n\n')}`;
    const input = `Repository: ${pr.repository}\nPR #${pr.number}: ${pr.title}\nDescription: ${pr.body.slice(0, 4000)}\n\n${context.slice(0, 28_000)}`;
    helperDrafting = true;
    detailError = null;
    try {
      const draft = await runHelperJobFromTauri('review', input);
      if (detail?.repository !== pr.repository || detail?.number !== pr.number || detail?.headSha !== pr.headSha) return;
      if (target === 'line' && (lineTarget?.path !== line?.path || lineTarget?.line !== line?.line)) return;
      draftHeadSha = pr.headSha;
      if (target === 'line') lineBody = draft.trim(); else draftSummary = draft.trim();
    } catch (reason) {
      detailError = reason instanceof Error ? reason.message : String(reason);
    } finally {
      helperDrafting = false;
    }
  }

  function previewReply(): void {
    if (!detail || !replyTarget || !replyBody.trim() || posting || postUncertain) return;
    postedUrl = '';
    pendingReply = { commentId: Number(replyTarget.id), body: replyBody.trim(), headSha: detail.headSha };
  }

  async function confirmReply(): Promise<void> {
    if (!detail || !pendingReply || posting) return;
    const pr = detail;
    const reply = pendingReply;
    pendingReply = null;
    posting = true;
    detailError = null;
    try {
      postedUrl = await replyGithubPullRequestCommentFromTauri({ root: pr.localRoot, number: pr.number, expectedHeadSha: reply.headSha, commentId: reply.commentId, body: reply.body });
      replyTarget = null;
      replyBody = '';
      await loadDetail(pr.localRoot, pr.number);
    } catch (reason) {
      detailError = reason instanceof Error ? reason.message : String(reason);
      postUncertain = /uncertain|not confirmed/i.test(detailError);
    } finally {
      posting = false;
    }
  }

  function previewReview(): void {
    if (!detail || !draftSummary.trim() || posting || postUncertain) return;
    postedUrl = '';
    pendingReview = {
      root: detail.localRoot,
      number: detail.number,
      expectedHeadSha: draftHeadSha || detail.headSha,
      body: draftSummary.trim(),
      comments: draftLines.map((line) => ({ ...line, body: line.body.trim() })).filter((line) => line.body)
    };
  }

  async function confirmReview(): Promise<void> {
    if (!pendingReview || posting) return;
    const submission = pendingReview;
    posting = true;
    detailError = null;
    pendingReview = null;
    try {
      postedUrl = await submitGithubPullRequestReviewFromTauri(submission);
      cancelDrafts();
      if (selected) await loadDetail(selected.localRoot, selected.number);
    } catch (reason) {
      detailError = reason instanceof Error ? reason.message : String(reason);
      postUncertain = /uncertain|not confirmed/i.test(detailError);
    } finally {
      posting = false;
    }
  }

  $effect(() => {
    if (showing && !loaded && !loading) void load(true);
  });
  $effect(() => {
    const target = selected;
    if (showing && target) untrack(() => void loadDetail(target.localRoot, target.number));
  });
  $effect(() => {
    if (showing && detailTab === 'files' && diffMode === 'side-by-side' && detail && selectedFile) {
      untrack(() => void loadFileVersions(detail, selectedFile));
    } else {
      untrack(() => { fileGeneration += 1; fileVersions = null; });
    }
  });
</script>

<div class="workspace" data-selectable="true">
  <aside class="queue" aria-label="Pull request queue">
    <div class="queue-heading"><h2>Pull Requests</h2><button type="button" onclick={() => void load(true)} disabled={loading} aria-label="Refresh pull requests">↻</button></div>
    <div class="modes" role="group" aria-label="Pull request filter">
      <button type="button" class:active={mode === 'open'} onclick={() => changeMode('open')}>Open</button>
      <button type="button" class:active={mode === 'mine'} onclick={() => changeMode('mine')}>Mine</button>
      <button type="button" class:active={mode === 'needs-review'} onclick={() => changeMode('needs-review')}>Needs review</button>
    </div>
    <form onsubmit={applySearch}><input aria-label="Search pull requests" placeholder="Search pull requests" bind:value={searchInput}><button type="submit">Search</button></form>
    <select aria-label="Filter by project" bind:value={projectFilter} onchange={() => void load(true)}>
      <option value="">All projects</option>
      {#each roots as root (root.path)}<option value={root.path}>{root.name}</option>{/each}
    </select>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if loading && items.length === 0}<p class="notice">Loading pull requests…</p>{/if}
    {#if loaded && items.length === 0 && !loading && !error}<p class="notice">No pull requests match this view.</p>{/if}
    <div class="rows">
      {#each items as item (`${item.repository}#${item.number}`)}
        <button type="button" class="row" class:selected={selected?.repository === item.repository && selected?.number === item.number} onclick={() => selectPullRequest(item)}>
          <span class="row-title">{item.title} <span>#{item.number}</span></span>
          <span class="row-meta">{item.repository} · {item.isDraft ? 'Draft · ' : ''}{item.author}</span>
        </button>
      {/each}
    </div>
    {#if cursor}<button type="button" class="more" disabled={loading} onclick={() => void load(false)}>{loading ? 'Loading…' : `Load more · ${items.length} of ${totalCount}`}</button>{/if}
  </aside>
  <section class="detail" aria-label="Selected pull request">
    {#if selected}
      <header class="pr-header">
        <p class="repo">{selected.repository}</p>
        <h2 class="pr-title">{selected.title} <span class="pr-number">#{selected.number}</span></h2>
        <div class="pr-meta">
          <span class="state-pill {headerState.tone}">{headerState.label}</span>
          <span><strong>{selected.author}</strong> wants to merge into <code>{selected.baseBranch}</code> from <code>{selected.headBranch}</code></span>
        </div>
        <div class="pr-actions">
          <a class="action primary" href={selected.url} target="_blank" rel="noreferrer">Open on GitHub</a>
          <button type="button" class="action" disabled={detailLoading || posting} onclick={() => void loadDetail(selected.localRoot, selected.number)}>{detailLoading ? 'Refreshing…' : 'Refresh'}</button>
        </div>
        {#if detail}
          <p class="pr-facts">
            <span>Reviewers: {detail.reviewers.length ? detail.reviewers.join(', ') : 'None yet'}</span>
            <span>Review: {detail.reviewDecision || 'No decision'}</span>
            <span>Mergeability: {detail.mergeable || 'Unknown'}</span>
          </p>
        {/if}
      </header>
      <nav aria-label="Pull request details">
        <button type="button" class:active={detailTab === 'conversation'} onclick={() => detailTab = 'conversation'}>Conversation{#if detail}<span class="count">{detail.comments.length + detail.reviews.length}</span>{/if}</button>
        <button type="button" class:active={detailTab === 'checks'} onclick={() => detailTab = 'checks'}>Checks{#if detail}<span class="count">{detail.checks.length}</span>{/if}</button>
        <button type="button" class:active={detailTab === 'files'} onclick={() => detailTab = 'files'}>Files changed{#if detail}<span class="count">{detail.files.length}{detail.moreFiles ? '+' : ''}</span>{/if}</button>
      </nav>
      {#if detailLoading}<p class="notice">Loading pull request details…</p>{/if}
      {#if detailError}<p class="notice error" role="alert">{detailError}</p>{/if}
      {#if postedUrl}<p class="notice" role="status">Review posted. <a href={postedUrl} target="_blank" rel="noreferrer">View it on GitHub</a></p>{/if}
      {#if detail}
        {#if detailTab === 'conversation'}
          <div class="conversation">
            <section class="entry"><div class="entry-head"><strong>{detail.author}</strong><span>opened this pull request</span></div><div class="body-text"><ConversationMessage text={detail.body || 'No description.'} role="assistant" /></div></section>
            {#each detail.comments as comment (comment.id)}
              <section class="entry"><div class="entry-head"><strong>{comment.author}</strong><span>{comment.path ? 'commented on' : 'commented'}</span>{#if comment.path}<code>{comment.path}{comment.line ? `:${comment.line}` : ''}</code>{/if}<span class="entry-meta">{formatTime(comment.createdAt)}</span></div><div class="body-text"><ConversationMessage text={comment.body} role="assistant" /></div>{#if comment.path && comment.replyToId === null && detail.state === 'OPEN'}<button type="button" class="reply-button" onclick={() => { replyTarget = comment; replyBody = ''; }}>Reply</button>{/if}</section>
            {/each}
            {#if replyTarget}
              <section class="review-draft"><h3>Reply to {replyTarget.author} on {replyTarget.path}:{replyTarget.line}</h3><textarea aria-label="Review reply" rows="3" bind:value={replyBody}></textarea><div class="draft-actions"><button type="button" onclick={() => { replyTarget = null; replyBody = ''; }}>Cancel</button><button type="button" disabled={!replyBody.trim() || posting || postUncertain} onclick={previewReply}>Review reply…</button></div></section>
            {/if}
            {#each detail.reviews as review, index (index)}
              <section class="entry"><div class="entry-head"><strong>{review.author}</strong><span>reviewed</span><span class="review-state">{review.state.replaceAll('_', ' ').toLowerCase()}</span><span class="entry-meta">{formatTime(review.submittedAt)}</span></div><div class="body-text"><ConversationMessage text={review.body || 'No review summary.'} role="assistant" /></div></section>
            {/each}
          </div>
        {:else if detailTab === 'checks'}
          {#if detail.checks.length === 0}<p class="notice">No checks reported for this pull request.</p>{:else}
          <div class="checks">
            {#each detail.checks as check, index (index)}
              <div class="check"><i class="check-dot {checkTone(check)}" aria-hidden="true"></i><strong>{check.name || 'Check'}</strong><span>{(check.conclusion || check.status || 'Unknown').replaceAll('_', ' ').toLowerCase()}</span>{#if check.url}<a href={check.url} target="_blank" rel="noreferrer">Details</a>{/if}</div>
            {/each}
          </div>
          {/if}
        {:else}
          {#if detail.moreFiles}<p class="notice">Showing the first 100 files. Open GitHub to inspect the remaining files.</p>{/if}
          <div class="files-layout">
            <aside class="file-list" aria-label="Changed files">
              {#each detail.files as file (file.path)}
                <button type="button" class:selected={selectedFilePath === file.path} onclick={() => selectedFilePath = file.path} ondblclick={() => { selectedFilePath = file.path; void openFullDiff(); }} title={`${file.path}\nDouble-click to open in the Diff tab`}>
                  <span class="file-status {file.status}">{file.status.charAt(0).toUpperCase()}</span>
                  <span class="file-name">{fileName(file.path)}{#if fileDir(file.path)}<small>{fileDir(file.path)}</small>{/if}</span>
                  <span class="file-counts"><span class="add">+{file.additions}</span><span class="del">−{file.deletions}</span></span>
                </button>
              {/each}
            </aside>
            <div class="file-diff">
              {#if selectedFile}
                <div class="file-toolbar">
                  <h3 title={selectedFile.path}>{selectedFile.path} <small>{selectedFile.status}</small></h3>
                  <div class="diff-modes" role="group" aria-label="Diff view mode">
                    <button type="button" class:active={diffMode === 'unified'} aria-pressed={diffMode === 'unified'} onclick={() => diffMode = 'unified'}>Unified</button>
                    <button type="button" class:active={diffMode === 'side-by-side'} aria-pressed={diffMode === 'side-by-side'} onclick={() => diffMode = 'side-by-side'}>Side by side</button>
                  </div>
                  <button type="button" class="open-full-diff" disabled={openingDiff} onclick={() => void openFullDiff()} title="Open this file in the full-width Diff tab">{openingDiff ? 'Opening diff…' : 'Open in Diff tab ↗'}</button>
                </div>
                {#if diffMode === 'side-by-side'}
                  {#if fileLoading}<p class="notice">Loading both file versions…</p>
                  {:else if fileError}<p class="notice error" role="alert">{fileError}</p>
                  {:else if fileVersions && DiffEditor}<p class="notice">Click a changed line to draft a comment.</p><div class="native-diff"><DiffEditor root={detail.localRoot} relativePath={selectedFile.path} originalContent={fileVersions.originalContent} modifiedContent={fileVersions.modifiedContent} onReviewLine={startSideBySideDraft} /></div>{/if}
                {:else if parsedFile && !parsedFile.isEmpty && !parsedFile.isBinary}
                  {#each parsedFile.hunks as hunk, hunkIndex (hunkIndex)}
                    <div class="hunk-heading">{hunk.header}</div>
                    {#each hunk.lines as line, lineIndex (lineIndex)}
                      <div class="diff-line {line.kind}"><button type="button" class="add-comment" title="Draft a comment on this changed line" aria-label={`Draft comment on ${selectedFile.path} line ${line.afterLine ?? line.beforeLine}`} disabled={line.kind !== 'added' && line.kind !== 'removed'} onclick={() => startLineDraft(selectedFile.path, line.beforeLine, line.afterLine)}>+</button><span class="line-no">{line.beforeLine ?? ''}</span><span class="line-no">{line.afterLine ?? ''}</span><span class="line-text">{line.kind === 'added' ? '+' : line.kind === 'removed' ? '−' : ' '}{line.text}</span></div>
                      {#if lineTarget?.path === selectedFile.path && lineTarget.line === (line.afterLine ?? line.beforeLine) && lineTarget.side === (line.afterLine === null ? 'LEFT' : 'RIGHT')}
                        <div class="line-compose"><strong>{lineTarget.path}:{lineTarget.line} · {lineTarget.side === 'LEFT' ? 'before' : 'after'}</strong><textarea rows="3" aria-label="Line review comment" placeholder="Write a comment for this line…" bind:value={lineBody}></textarea><div class="draft-actions"><button type="button" disabled={helperDrafting} onclick={() => void draftWithHelper('line')}>Draft with Helper</button><button type="button" onclick={() => { lineTarget = null; lineBody = ''; }}>Cancel line</button><button type="button" disabled={!lineBody.trim()} onclick={saveLineDraft}>Add to draft</button></div></div>
                      {/if}
                    {/each}
                  {/each}
                {:else}<p class="notice">A text patch is not available for this file.</p>{/if}
                {#if diffMode === 'side-by-side' && lineTarget?.path === selectedFile.path}
                  <div class="line-compose"><strong>{lineTarget.path}:{lineTarget.line} · {lineTarget.side === 'LEFT' ? 'before' : 'after'}</strong><textarea rows="3" aria-label="Line review comment" placeholder="Write a comment for this line…" bind:value={lineBody}></textarea><div class="draft-actions"><button type="button" disabled={helperDrafting} onclick={() => void draftWithHelper('line')}>Draft with Helper</button><button type="button" onclick={() => { lineTarget = null; lineBody = ''; }}>Cancel line</button><button type="button" disabled={!lineBody.trim()} onclick={saveLineDraft}>Add to draft</button></div></div>
                {/if}
              {:else}<p class="notice">Choose a changed file.</p>{/if}
            </div>
          </div>
        {/if}
        <section class="review-draft" aria-label="Review draft">
          <h3>Review draft</h3>
          <p>Comments stay here until you review and confirm the exact text and target.</p>
          <label for="pr-review-summary">Overall review comment</label>
          <textarea id="pr-review-summary" rows="4" placeholder="Write an overall review comment…" bind:value={draftSummary} oninput={() => { if (detail && draftSummary.trim()) draftHeadSha = detail.headSha; }}></textarea>
          <button type="button" class="helper-button" disabled={helperDrafting || detail.state !== 'OPEN'} onclick={() => void draftWithHelper('overall')}>{helperDrafting ? 'Drafting…' : 'Draft with Helper'}</button>
          {#each draftLines as line, index (index)}
            <div class="draft-line"><strong>{line.path}:{line.line} · {line.side === 'LEFT' ? 'before' : 'after'}</strong><textarea rows="3" aria-label={`Draft comment on ${line.path} line ${line.line}`} bind:value={line.body}></textarea><button type="button" onclick={() => draftLines = draftLines.filter((_, row) => row !== index)}>Remove</button></div>
          {/each}
          <div class="draft-actions"><button type="button" disabled={!draftSummary.trim() && draftLines.length === 0 && !lineTarget} onclick={cancelDrafts}>Discard draft</button><button type="button" disabled={!draftSummary.trim() || posting || postUncertain} onclick={previewReview}>Review and post…</button></div>
        </section>
      {/if}
      {#if pendingReview}
        <div class="confirm-backdrop" role="presentation">
          <div class="confirm-review" role="dialog" aria-modal="true" aria-label="Confirm pull request review">
            <h3>Post this review?</h3>
            <p><strong>{selected.repository} #{pendingReview.number}</strong> · head {pendingReview.expectedHeadSha.slice(0, 10)}</p>
            <h4>Overall comment</h4><pre>{pendingReview.body}</pre>
            {#each pendingReview.comments as comment}
              <h4>{comment.path}:{comment.line} · {comment.side === 'LEFT' ? 'before' : 'after'}</h4><pre>{comment.body}</pre>
            {/each}
            <p>GitHub will notify participants. Assembly will check the PR head again before submitting.</p>
            <div class="draft-actions"><button type="button" onclick={() => pendingReview = null}>Cancel</button><button type="button" class="confirm-submit" onclick={() => void confirmReview()}>Post review to GitHub</button></div>
          </div>
        </div>
      {/if}
      {#if pendingReply}
        <div class="confirm-backdrop" role="presentation"><div class="confirm-review" role="dialog" aria-modal="true" aria-label="Confirm review reply"><h3>Post this reply?</h3><p><strong>{selected.repository} #{selected.number}</strong> · comment {pendingReply.commentId}</p><pre>{pendingReply.body}</pre><p>GitHub will notify participants. Assembly will check the PR head again before submitting.</p><div class="draft-actions"><button type="button" onclick={() => pendingReply = null}>Cancel</button><button type="button" class="confirm-submit" onclick={() => void confirmReply()}>Post reply to GitHub</button></div></div></div>
      {/if}
    {:else}<p class="notice">Choose a pull request to review.</p>{/if}
  </section>
</div>

<style>
  .workspace{display:grid;grid-template-columns:minmax(240px,300px) minmax(0,1fr);height:100%;min-height:0;background:var(--color-bg);color:var(--color-text)}
  .queue{display:flex;flex-direction:column;min-width:0;min-height:0;border-right:1px solid var(--color-border);padding:16px 12px}
  .queue-heading{display:flex;align-items:center;justify-content:space-between;gap:8px;padding:0 4px 12px}.queue-heading h2{font-size:17px;font-weight:650;margin:0}.queue-heading button{font-size:20px}
  button,select,input{font:inherit;color:inherit}.queue-heading button,.modes button,form button,.more{border:1px solid var(--color-border);background:var(--color-surface);border-radius:6px;cursor:pointer}
  .modes{display:flex;gap:4px;margin-bottom:12px}.modes button{font-size:11px;padding:5px 7px;white-space:nowrap}.modes button.active{background:var(--color-elevated);color:var(--color-text)}
  form{display:flex;gap:5px;margin-bottom:8px}input,select{min-width:0;border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);padding:7px 8px;font-size:12px}input{flex:1}form button{padding:5px 8px;font-size:11px}select{width:100%;margin-bottom:8px}
  .rows{overflow:auto;min-height:0;flex:1}.row{display:block;text-align:left;width:100%;padding:10px;border:0;border-radius:7px;background:transparent;cursor:pointer}.row:hover{background:var(--color-elevated)}.row.selected{background:var(--color-elevated);box-shadow:inset 2px 0 var(--color-accent)}.row-title,.row-meta{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.row-title{font-size:12px}.row-title span,.row-meta{color:var(--color-text-3)}.row-meta{font-size:11px;margin-top:4px}.more{padding:8px;margin-top:7px;font-size:11px}
  .detail{min-width:0;overflow:auto;padding:26px 32px 32px}.pr-header,.detail nav,.conversation,.checks,.review-draft{max-width:1100px}.pr-header{background:var(--color-surface);border-radius:12px;padding:18px 20px}.repo{font-size:12px;color:var(--color-text-3);margin:0}
  .pr-title{font-size:26px;font-weight:600;line-height:1.25;letter-spacing:-.01em;margin:6px 0 12px;overflow-wrap:anywhere}.pr-number{color:var(--color-accent);font-weight:500}
  .pr-meta{display:flex;flex-wrap:wrap;align-items:center;gap:10px;font-size:13px;color:var(--color-text-2)}.pr-meta strong{color:var(--color-text)}code{font:12px ui-monospace,monospace;background:var(--color-elevated);border-radius:4px;padding:2px 6px;color:var(--color-text-2)}
  .state-pill{display:inline-flex;align-items:center;border-radius:999px;padding:3px 11px;font-size:12px;font-weight:600;background:var(--color-elevated);color:var(--color-text-2)}.state-pill.good{background:var(--color-good-bg);color:var(--color-good)}.state-pill.bad{background:var(--color-bad-bg);color:var(--color-bad)}.state-pill.merged{background:var(--color-selected);color:var(--color-accent)}
  .pr-actions{display:flex;gap:8px;margin-top:14px}.action{display:inline-flex;align-items:center;border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);padding:6px 12px;font-size:12px;color:var(--color-text);text-decoration:none;cursor:pointer}.action:hover{background:var(--color-hover)}.action.primary{background:var(--color-accent);border-color:var(--color-accent);color:var(--color-on-accent)}.action:disabled{opacity:.5;cursor:default}
  .pr-facts{display:flex;flex-wrap:wrap;gap:6px 18px;font-size:12px;color:var(--color-text-3);margin:14px 0 0}
  .detail nav{display:flex;gap:22px;border-bottom:1px solid var(--color-border);margin-top:18px;font-size:12px}.detail nav button{display:inline-flex;align-items:center;gap:6px;border:0;border-bottom:2px solid transparent;background:transparent;padding:10px 2px;color:var(--color-text-2);cursor:pointer}.detail nav button.active{border-bottom-color:var(--color-accent);color:var(--color-text)}.count{min-width:18px;border-radius:999px;background:var(--color-elevated);padding:0 6px;font-size:11px;line-height:17px;text-align:center;color:var(--color-text-2)}.detail a{font-size:12px}.notice{font-size:12px;color:var(--color-text-3);margin:13px 4px}.notice.error{color:var(--color-bad)}
  .entry{border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);margin-top:14px;font-size:12px;overflow:hidden}.entry-head{display:flex;flex-wrap:wrap;align-items:center;gap:6px;padding:9px 14px;background:var(--color-elevated);border-bottom:1px solid var(--color-border);color:var(--color-text-2)}.entry-head strong{color:var(--color-text)}.entry-meta{margin-left:auto;color:var(--color-text-3)}.review-state{text-transform:capitalize;color:var(--color-text)}.body-text{overflow-wrap:anywhere;margin:0;padding:12px 14px}.entry .reply-button{margin:0 14px 12px}
  .checks{margin-top:14px;border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);overflow:hidden}.check{display:flex;align-items:center;gap:10px;border-bottom:1px solid var(--color-border);padding:10px 14px;font-size:12px}.check:last-child{border-bottom:0}.check span{margin-left:auto;color:var(--color-text-3);text-transform:capitalize}.check-dot{width:8px;height:8px;flex:none;border-radius:50%;background:var(--color-attention)}.check-dot.good{background:var(--color-good)}.check-dot.bad{background:var(--color-bad)}
  .files-layout{display:flex;flex-direction:column;height:72vh;min-height:450px;margin-top:14px;border:1px solid var(--color-border);border-radius:7px;overflow:hidden}.file-list{min-width:0;max-height:160px;flex:none;border-bottom:1px solid var(--color-border);overflow:auto;padding:4px 0}.file-list button{display:flex;align-items:center;gap:8px;width:100%;padding:5px 10px;text-align:left;border:0;background:transparent;font-size:12px;cursor:pointer}.file-list button:hover{background:var(--color-hover)}.file-list button.selected{background:var(--color-selected);box-shadow:inset 2px 0 var(--color-accent)}
  .file-status{width:14px;flex:none;text-align:center;font:600 11px ui-monospace,monospace;color:var(--color-attention)}.file-status.added{color:var(--color-good)}.file-status.removed{color:var(--color-bad)}.file-name{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.file-name small{margin-left:6px}.file-counts{display:flex;gap:5px;flex:none;font:11px ui-monospace,monospace}.file-counts .add{color:var(--color-good)}.file-counts .del{color:var(--color-bad)}
  .file-list small,.file-diff h3 small{color:var(--color-text-3);font-weight:400}.file-diff{min-width:0;min-height:0;flex:1;overflow:auto;padding:0 0 16px}.file-toolbar{position:sticky;top:0;left:0;z-index:1;display:flex;flex-wrap:wrap;align-items:center;gap:8px;padding:7px 12px;background:var(--color-surface);border-bottom:1px solid var(--color-border)}.file-toolbar h3{flex-basis:100%;min-width:0;font:12px ui-monospace,monospace;margin:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.hunk-heading{background:var(--color-elevated);padding:7px 12px;font:11px ui-monospace,monospace;color:var(--color-text-3)}.diff-line{display:flex;min-width:max-content;font:11px/1.55 ui-monospace,monospace;white-space:pre}.diff-line.added{background:rgba(80,250,123,.12)}.diff-line.removed{background:rgba(255,85,85,.13)}.line-no{width:48px;flex:none;text-align:right;padding:0 7px;color:var(--color-text-3);user-select:none}.line-text{padding:0 12px;tab-size:4}
  .diff-modes{display:flex;flex:none;border:1px solid var(--color-border);border-radius:6px;overflow:hidden}.diff-modes button{border:0;background:transparent;padding:4px 9px;font-size:11px;color:var(--color-text-2);cursor:pointer}.diff-modes button.active{background:var(--color-elevated);color:var(--color-text)}.native-diff{height:560px;min-height:0}
  .open-full-diff{flex:none;border:1px solid var(--color-accent);border-radius:6px;background:var(--color-accent);color:var(--color-on-accent);padding:5px 10px;font-size:11px;font-weight:600;cursor:pointer}.open-full-diff:disabled{opacity:.5;cursor:default}
  .diff-line .add-comment{width:24px;flex:none;border:0;background:transparent;color:var(--color-text-3);opacity:0;cursor:pointer}.diff-line:hover .add-comment,.diff-line .add-comment:focus-visible{opacity:1}.diff-line .add-comment:disabled{visibility:hidden}
  .review-draft{border-radius:12px;background:var(--color-surface);margin-top:24px;padding:18px 20px}.review-draft h3{font-size:14px;margin:0}.review-draft p{font-size:12px;color:var(--color-text-3);margin:6px 0 16px}.review-draft label,.line-compose strong,.draft-line strong{display:block;font-size:12px;margin-bottom:8px}.review-draft textarea{display:block;width:100%;resize:vertical;min-height:64px;background:var(--color-bg);border:1px solid var(--color-border);border-radius:6px;color:var(--color-text);padding:9px;font:12px/1.5 inherit}.line-compose,.draft-line{margin-top:14px;padding:12px;border:1px solid var(--color-border);border-radius:6px}.draft-actions{display:flex;justify-content:flex-end;gap:8px;margin-top:12px}.draft-actions button,.draft-line button{border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);padding:7px 10px;font-size:12px;cursor:pointer}.draft-actions button:disabled{opacity:.5;cursor:default}.draft-line button{margin-top:8px}
  .helper-button,.reply-button{border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);padding:6px 9px;font-size:11px;cursor:pointer;margin-top:8px}
  .confirm-backdrop{position:fixed;inset:0;z-index:1000;display:grid;place-items:center;background:rgba(0,0,0,.72)}.confirm-review{width:min(680px,calc(100vw - 40px));max-height:calc(100vh - 40px);overflow:auto;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:0 20px 70px rgba(0,0,0,.45);padding:22px}.confirm-review h3{font-size:18px;margin:0 0 12px}.confirm-review h4{font-size:12px;margin:20px 0 6px}.confirm-review p{font-size:12px;color:var(--color-text-3)}.confirm-review pre{white-space:pre-wrap;overflow-wrap:anywhere;font:12px/1.5 ui-monospace,monospace;border:1px solid var(--color-border);border-radius:6px;padding:10px;max-height:250px;overflow:auto}.confirm-submit{background:var(--color-accent)!important;color:var(--color-bg)!important}
  @media(max-width:700px){.workspace{grid-template-columns:minmax(180px,35%) minmax(0,1fr)}.queue{padding:10px 6px}.detail{padding:14px}}
</style>
