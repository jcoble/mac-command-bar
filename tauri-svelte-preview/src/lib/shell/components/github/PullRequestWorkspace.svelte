<script lang="ts">
  import { untrack } from 'svelte';
  import { hydrateProjects, projectRegistry } from '$lib/shell/projects/projectRegistry.svelte';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import { parseRemoteWorkspacePath, remoteWorkspacePath, sessionWorkspaceRoot } from '$lib/workspacePaths';
  import {
    listGithubPullRequestsFromTauri,
    mergeGithubPullRequestFromTauri,
    readGithubPullRequestFromTauri,
    readGithubPullRequestFileFromTauri,
    replyGithubPullRequestCommentFromTauri,
    runHelperJobFromTauri,
    submitGithubPullRequestReviewFromTauri,
    type GithubFileVersions,
    type GithubReviewSubmission,
    type GithubMergeRequest,
    type GithubPullRequestDetail,
    type GithubPullRequestSummary
  } from '$lib/tauriSource';
  import { parseUnifiedDiff } from '$lib/shell/git/parseUnifiedDiff';
  import { openPullRequestDiff, type PullRequestLink } from '$lib/shell/workbenchNavigation';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump';
  import { formatLastActivity, exactLocalTime } from '$lib/shell/relativeTime';
  import { pullRequestSelection } from './pullRequestSelection.svelte';
  import { pullRequestSearchQuery, type PullRequestScope, type PullRequestState } from './pullRequestSearch';
  import ConversationMessage from '$lib/shell/components/conversation/ConversationMessage.svelte';
  import type CodeMirrorGitDiffEditor from '$lib/shell/components/git/CodeMirrorGitDiffEditor.svelte';
  import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import GitCommitHorizontal from '@lucide/svelte/icons/git-commit-horizontal';
  import GitMerge from '@lucide/svelte/icons/git-merge';
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
  import GitPullRequestClosed from '@lucide/svelte/icons/git-pull-request-closed';
  import GitPullRequestDraft from '@lucide/svelte/icons/git-pull-request-draft';
  import Link from '@lucide/svelte/icons/link';
  import MessageSquare from '@lucide/svelte/icons/message-square';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';

  let { showing }: { showing: boolean } = $props();

  const STATE_ITEMS = [
    { value: 'open', label: 'Open' },
    { value: 'merged', label: 'Merged' },
    { value: 'closed', label: 'Closed' },
    { value: 'all', label: 'All' }
  ];
  const SCOPE_LABELS: Record<PullRequestScope, string> = { everyone: 'Everyone', mine: 'Mine', 'needs-review': 'Needs my review' };

  let stateFilter = $state<PullRequestState>('open');
  let scope = $state<PullRequestScope>('everyone');
  let projectFilter = $state('');
  let searchInput = $state('');
  /** The text the current list was searched with, so clearing the box can reload. */
  let searchedText = '';
  let items = $state<GithubPullRequestSummary[]>([]);
  let detail = $state<GithubPullRequestDetail | null>(null);
  let page = $state<'summary' | 'changes'>('summary');
  let linkMissing = $state<PullRequestLink | null>(null);
  let linkCopied = $state(false);
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
  let mergeMethod = $state<GithubMergeRequest['method']>('merge');
  let pendingMerge = $state<(GithubMergeRequest & { repository: string; title: string; baseBranch: string; headBranch: string }) | null>(null);
  let merging = $state(false);
  let mergeUncertain = $state(false);
  let mergeError = $state('');
  let mergedUrl = $state('');
  let cursor = $state<string | null>(null);
  let totalCount = $state(0);
  let loading = $state(false);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let generation = 0;
  let detailGeneration = 0;
  let fileGeneration = 0;

  // Only projects whose remote is on github.com can list pull requests.
  const roots = $derived(projectRegistry.projects.filter((project) => project.machine === 'local' && project.repoKey.toLowerCase().startsWith('github.com/')));
  // Remote projects come from the rail's sessions; each one is searched on its own machine.
  const remoteRoots = $derived([...new Set(rail.owned.filter((session) => session.executionEnvironment === 'remote').map(sessionWorkspaceRoot).filter(Boolean))]);
  const selectedFile = $derived(detail?.files.find((file) => file.path === selectedFilePath) ?? null);
  const parsedFile = $derived(selectedFile?.patch ? parseUnifiedDiff(selectedFile.patch) : null);
  const headerState = $derived(prState(detail ?? pullRequestSelection.selected));
  // Detail loads re-run only when the chosen pull request changes, not when the
  // selection object is filled in from the detail it loaded.
  const selectedKey = $derived(pullRequestSelection.selected ? `${pullRequestSelection.selected.localRoot}\n${pullRequestSelection.selected.number}` : '');
  const changeTotals = $derived(detail ? detail.files.reduce((sum, file) => ({ additions: sum.additions + file.additions, deletions: sum.deletions + file.deletions }), { additions: 0, deletions: 0 }) : null);
  const projectLabel = $derived(roots.find((root) => root.rootPath === projectFilter)?.title ?? remoteRoots.find((root) => root === projectFilter)?.split('/').filter(Boolean).at(-1) ?? 'All local projects');
  // Assembly sessions whose conversation names this pull request.
  const threads = $derived.by(() => {
    if (!detail) return [];
    const { number, localRoot } = detail;
    return rail.owned.filter((session) => session.pullRequest === `PR #${number}` && (session.projectPath === localRoot || sessionWorkspaceRoot(session) === localRoot));
  });
  const activity = $derived(detail ? activityRows(detail) : []);

  type ActivityRow =
    | { kind: 'opened' | 'merged' | 'closed'; key: string; at: string; author: string }
    | { kind: 'commit'; key: string; at: string; author: string; oid: string; headline: string }
    | { kind: 'comment'; key: string; at: string; comment: GithubPullRequestDetail['comments'][number] }
    | { kind: 'review'; key: string; at: string; review: GithubPullRequestDetail['reviews'][number] };

  /** Everything that happened on the pull request, oldest first. */
  function activityRows(pr: GithubPullRequestDetail): ActivityRow[] {
    const rows: ActivityRow[] = [{ kind: 'opened', key: 'opened', at: pr.createdAt, author: pr.author }];
    for (const commit of pr.commits) rows.push({ kind: 'commit', key: `commit-${commit.oid}`, at: commit.committedAt, author: commit.author, oid: commit.oid, headline: commit.headline });
    for (const comment of pr.comments) rows.push({ kind: 'comment', key: `comment-${comment.id}`, at: comment.createdAt, comment });
    pr.reviews.forEach((review, index) => { if (review.body.trim() || review.state !== 'COMMENTED') rows.push({ kind: 'review', key: `review-${index}`, at: review.submittedAt, review }); });
    if (pr.mergedAt) rows.push({ kind: 'merged', key: 'merged', at: pr.mergedAt, author: pr.mergedBy });
    else if (pr.closedAt) rows.push({ kind: 'closed', key: 'closed', at: pr.closedAt, author: '' });
    return rows.sort((left, right) => left.at.localeCompare(right.at));
  }

  function age(value: string): string {
    return formatLastActivity(value, new Date());
  }

  function avatarUrl(login: string): string {
    return `https://github.com/${encodeURIComponent(login)}.png?size=40`;
  }
  const canMerge = $derived(detail?.state === 'OPEN' && !detail.isDraft && detail.mergeable === 'MERGEABLE' && !detailLoading && !posting && !merging && !mergeUncertain);

  function mergeMethodLabel(method: GithubMergeRequest['method']): string {
    return method === 'squash' ? 'Squash and merge' : method === 'rebase' ? 'Rebase and merge' : 'Create merge commit';
  }

  function prState(pr: { isDraft: boolean; state: string } | null): { label: string; tone: string } {
    // A pull request opened from a link has no state until its detail loads.
    if (!pr?.state) return { label: '', tone: 'idle' };
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
    mergeUncertain = false;
    mergeError = '';
    selectedFilePath = '';
    try {
      const result = await readGithubPullRequestFromTauri(root, number);
      if (current !== detailGeneration) return;
      if (!result) throw new Error('Pull request details are available in the desktop app.');
      detail = result;
      // A pull request opened from a link starts with only its number; fill in the rest.
      const selected = pullRequestSelection.selected;
      if (selected && selected.number === result.number && !selected.title) {
        Object.assign(selected, { repository: result.repository, title: result.title, url: result.url, author: result.author, state: result.state, isDraft: result.isDraft, headBranch: result.headBranch, baseBranch: result.baseBranch });
      }
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
      await hydrateProjects();
      if (reset) searchedText = searchInput.trim();
      const result = await listGithubPullRequestsFromTauri({
        roots: parseRemoteWorkspacePath(projectFilter) ? [projectFilter] : roots.map((root) => root.rootPath),
        projectFilter: projectFilter || null,
        search: pullRequestSearchQuery(stateFilter, scope, searchedText),
        cursor: reset ? null : cursor,
        pageSize: 30
      });
      if (current !== generation) return;
      if (!result) throw new Error('Pull requests are available in the desktop app.');
      items = reset ? result.items : [...items, ...result.items];
      cursor = result.nextCursor;
      totalCount = result.totalCount;
      // A pull request stays open while the list changes around it (a link may
      // have opened one this filter does not show).
      if (!pullRequestSelection.selected && !pullRequestSelection.link) pullRequestSelection.selected = items[0] ?? null;
      loaded = true;
    } catch (reason) {
      if (current === generation) error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      if (current === generation) loading = false;
    }
  }

  function onSearchKey(event: KeyboardEvent): void {
    if (event.key === 'Enter') void load(true);
  }

  /** Emptying the box shows the unsearched list again without another Enter. */
  function onSearchInput(): void {
    if (!searchInput.trim() && searchedText) void load(true);
  }

  /** A link names a repository; find the project that has it as its remote. */
  async function openLink(link: PullRequestLink): Promise<void> {
    linkMissing = null;
    const sameRepository = (repository: string) => repository.toLowerCase() === link.repository.toLowerCase();
    const listed = items.find((item) => sameRepository(item.repository) && item.number === link.number);
    if (listed) {
      selectPullRequest(listed);
      return;
    }
    await hydrateProjects();
    const project = projectRegistry.projects.find((candidate) => candidate.repoKey.toLowerCase() === `github.com/${link.repository}`.toLowerCase());
    if (pullRequestSelection.link) return; // a newer link arrived meanwhile
    if (!project) {
      linkMissing = link;
      return;
    }
    selectPullRequest({
      repository: link.repository,
      localRoot: project.machine === 'local' ? project.rootPath : remoteWorkspacePath(project.machine, project.rootPath),
      number: link.number,
      title: '',
      url: `https://github.com/${link.repository}/pull/${link.number}`,
      isDraft: false,
      updatedAt: '',
      state: '',
      author: '',
      headBranch: '',
      baseBranch: ''
    });
  }

  async function copyLink(url: string): Promise<void> {
    await navigator.clipboard.writeText(url);
    linkCopied = true;
  }

  function selectPullRequest(item: GithubPullRequestSummary): void {
    if (pullRequestSelection.selected?.repository === item.repository && pullRequestSelection.selected.number === item.number) return;
    draftSummary = '';
    draftLines = [];
    lineTarget = null;
    lineBody = '';
    draftHeadSha = '';
    replyTarget = null;
    replyBody = '';
    pendingReply = null;
    pendingMerge = null;
    mergedUrl = '';
    mergeError = '';
    mergeMethod = 'merge';
    page = 'summary';
    linkMissing = null;
    linkCopied = false;
    pullRequestSelection.selected = item;
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
    const patchBudget = Math.floor(24_000 / Math.max(pr.files.length, 1));
    const context = target === 'line'
      ? `Target: ${line?.path}:${line?.line} (${line?.side})\n${pr.files.find((file) => file.path === line?.path)?.patch ?? 'No text patch available.'}`
      : `Target: overall review\n${pr.files.map((file) => `File: ${file.path}\n${file.patch?.slice(0, patchBudget) ?? 'No text patch available.'}`).join('\n\n')}`;
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
      if (pullRequestSelection.selected) await loadDetail(pullRequestSelection.selected.localRoot, pullRequestSelection.selected.number);
    } catch (reason) {
      detailError = reason instanceof Error ? reason.message : String(reason);
      postUncertain = /uncertain|not confirmed/i.test(detailError);
    } finally {
      posting = false;
    }
  }

  function previewMerge(): void {
    if (!detail || !canMerge) return;
    pendingMerge = {
      root: detail.localRoot,
      repository: detail.repository,
      number: detail.number,
      title: detail.title,
      headBranch: detail.headBranch,
      baseBranch: detail.baseBranch,
      expectedHeadSha: detail.headSha,
      expectedBaseSha: detail.baseSha,
      method: mergeMethod
    };
  }

  async function confirmMerge(): Promise<void> {
    if (!pendingMerge || merging || posting) return;
    const request = pendingMerge;
    pendingMerge = null;
    merging = true;
    mergeError = '';
    mergedUrl = '';
    try {
      mergedUrl = await mergeGithubPullRequestFromTauri(request);
      cancelDrafts();
      items = items.filter((item) => item.repository !== request.repository || item.number !== request.number);
      if (pullRequestSelection.selected?.repository === request.repository && pullRequestSelection.selected.number === request.number) await loadDetail(request.root, request.number);
    } catch (reason) {
      mergeError = reason instanceof Error ? reason.message : String(reason);
      mergeUncertain = /uncertain/i.test(mergeError);
    } finally {
      merging = false;
    }
  }

  $effect(() => {
    if (showing && !loaded && !loading && !error) void load(true);
  });
  $effect(() => {
    const key = selectedKey;
    const target = untrack(() => pullRequestSelection.selected);
    if (showing && key && target) untrack(() => void loadDetail(target.localRoot, target.number));
  });
  $effect(() => {
    const link = pullRequestSelection.link;
    if (!link) return;
    pullRequestSelection.link = null;
    untrack(() => void openLink(link));
  });
  $effect(() => {
    const currentDetail = detail;
    const currentFile = selectedFile;
    if (showing && page === 'changes' && diffMode === 'side-by-side' && currentDetail && currentFile) {
      untrack(() => void loadFileVersions(currentDetail, currentFile));
    } else {
      untrack(() => { fileGeneration += 1; fileVersions = null; });
    }
  });
</script>

{#snippet stateIcon(pr: { isDraft: boolean; state: string })}
  {@const tone = prState(pr).tone}
  {#if pr.isDraft}<GitPullRequestDraft size={14} aria-hidden="true" />
  {:else if tone === 'merged'}<GitMerge size={14} aria-hidden="true" />
  {:else if tone === 'bad'}<GitPullRequestClosed size={14} aria-hidden="true" />
  {:else}<GitPullRequest size={14} aria-hidden="true" />{/if}
{/snippet}

{#snippet avatar(login: string)}
  {#if login}<img class="avatar" src={avatarUrl(login)} alt="" width="20" height="20" loading="lazy" decoding="async" referrerpolicy="no-referrer" />{/if}
{/snippet}

<div class="workspace" data-selectable="true">
  <aside class="queue" aria-label="Pull request queue">
    <div class="queue-heading">
      <h2>Pull requests</h2>
      <IconButton label="Refresh pull requests" disabled={loading} onclick={() => void load(true)}><RefreshCw /></IconButton>
    </div>
    <Input type="search" aria-label="Search pull requests" placeholder="Search, head:branch, author:name" maxlength={150} bind:value={searchInput} onkeydown={onSearchKey} oninput={onSearchInput} />
    <SegmentedControl size="sm" class="w-full" items={STATE_ITEMS} value={stateFilter} aria-label="Pull request state" onValueChange={(value) => { stateFilter = value as PullRequestState; void load(true); }} />
    <div class="filters">
      <Select.Root type="single" value={scope} onValueChange={(value) => { scope = value as PullRequestScope; void load(true); }}>
        <Select.Trigger size="sm" class="min-w-0 flex-1" aria-label="Whose pull requests">{SCOPE_LABELS[scope]}</Select.Trigger>
        <Select.Content>
          {#each Object.entries(SCOPE_LABELS) as [value, label] (value)}<Select.Item {value} {label} />{/each}
        </Select.Content>
      </Select.Root>
      <Select.Root type="single" value={projectFilter || '*'} onValueChange={(value) => { projectFilter = value === '*' ? '' : value; void load(true); }}>
        <Select.Trigger size="sm" class="min-w-0 flex-1" aria-label="Filter by project"><span class="truncate">{projectLabel}</span></Select.Trigger>
        <Select.Content>
          <Select.Item value="*" label="All local projects" />
          {#each roots as root (root.id)}<Select.Item value={root.rootPath} label={root.title} />{/each}
          {#each remoteRoots as root (root)}<Select.Item value={root} label={`${root.split('/').filter(Boolean).at(-1)} (remote)`} />{/each}
        </Select.Content>
      </Select.Root>
    </div>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if loading && items.length === 0}<p class="notice">Loading pull requests…</p>{/if}
    {#if loaded && items.length === 0 && !loading && !error}<p class="notice">No pull requests match these filters.</p>{/if}
    <div class="rows">
      {#each items as item (`${item.repository}#${item.number}`)}
        <button type="button" class="row" class:selected={pullRequestSelection.selected?.repository === item.repository && pullRequestSelection.selected?.number === item.number} onclick={() => selectPullRequest(item)}>
          <span class="row-icon {prState(item).tone}">{@render stateIcon(item)}</span>
          <span class="row-text">
            <span class="row-title">{item.title}</span>
            <span class="row-meta">#{item.number} · {item.repository.split('/').at(-1)} · {item.author}{#if item.updatedAt} · <span title={exactLocalTime(item.updatedAt)}>{age(item.updatedAt)}</span>{/if}</span>
          </span>
        </button>
      {/each}
    </div>
    {#if cursor}<Button variant="ghost" size="sm" class="mt-1" disabled={loading} onclick={() => void load(false)}>{loading ? 'Loading…' : `Load more · ${items.length} of ${totalCount}`}</Button>{/if}
  </aside>
  <section class="detail" aria-label="Selected pull request">
    {#if linkMissing}
      <p class="notice" role="status">github.com/{linkMissing.repository} is not a project in Assembly, so #{linkMissing.number} cannot open here. Add the repository as a project, or <a href={`https://github.com/${linkMissing.repository}/pull/${linkMissing.number}`} target="_blank" rel="noreferrer">open it on GitHub</a>.</p>
    {/if}
    {#if pullRequestSelection.selected}
      {@const pr = pullRequestSelection.selected}
      <div class="detail-bar">
        <SegmentedControl size="sm" aria-label="Pull request view" value={page} items={[{ value: 'summary', label: 'Summary' }, { value: 'changes', label: 'Changes' }]} onValueChange={(value) => { page = value === 'changes' ? 'changes' : 'summary'; }} />
        {#if changeTotals}<span class="change-totals"><span class="add">+{changeTotals.additions}</span> <span class="del">−{changeTotals.deletions}</span></span>{/if}
        <div class="bar-actions">
          <IconButton label={linkCopied ? 'Link copied' : 'Copy link'} onclick={() => void copyLink(pr.url)}><Link /></IconButton>
          <IconButton label="Refresh this pull request" disabled={detailLoading || posting} onclick={() => void loadDetail(pr.localRoot, pr.number)}><RefreshCw /></IconButton>
          <a class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })} href={pr.url} target="_blank" rel="noreferrer" aria-label="Open on GitHub" title="Open on GitHub"><ExternalLink /></a>
        </div>
      </div>
      {#if detailLoading}<p class="notice">Loading pull request…</p>{/if}
      {#if detailError}<p class="notice error" role="alert">{detailError}</p>{/if}
      {#if postedUrl}<p class="notice" role="status">Review posted. <a href={postedUrl} target="_blank" rel="noreferrer">View it on GitHub</a></p>{/if}
      {#if mergedUrl}<p class="notice" role="status">Pull request merged. <a href={mergedUrl} target="_blank" rel="noreferrer">View it on GitHub</a></p>{/if}
      {#if page === 'summary'}
        <div class="summary">
          <div class="summary-main">
            <div class="pr-kicker">{#if headerState.label}<span class="state-pill {headerState.tone}">{@render stateIcon(detail ?? pr)}{headerState.label}</span>{/if}<span>{pr.repository.split('/').at(-1)} #{pr.number}</span></div>
            <h1 class="pr-title">{detail?.title || pr.title || `#${pr.number}`}</h1>
            <div class="pr-meta">
              {#if detail?.author || pr.author}{@render avatar(detail?.author || pr.author)}<strong>{detail?.author || pr.author}</strong>{/if}
              {#if detail?.createdAt}<span title={exactLocalTime(detail.createdAt)}>{age(detail.createdAt)}</span>{/if}
              {#if detail?.headBranch || pr.headBranch}<span class="branches"><code>{detail?.headBranch || pr.headBranch}</code><ArrowRight size={13} aria-hidden="true" /><code>{detail?.baseBranch || pr.baseBranch}</code></span>{/if}
            </div>
            {#if detail}
              <div class="pr-body"><ConversationMessage text={detail.body || 'No description.'} role="assistant" showImages /></div>
              <section class="activity" aria-label="Activity">
                <h2>Activity</h2>
                {#each activity as row (row.key)}
                  {#if row.kind === 'comment'}
                    <div class="entry">
                      <div class="entry-head">{@render avatar(row.comment.author)}<strong>{row.comment.author}</strong><span>{row.comment.path ? 'commented on' : 'commented'}</span>{#if row.comment.path}<code>{row.comment.path}{row.comment.line ? `:${row.comment.line}` : ''}</code>{/if}<span class="entry-age" title={exactLocalTime(row.at)}>{age(row.at)}</span></div>
                      <div class="entry-body"><ConversationMessage text={row.comment.body} role="assistant" showImages /></div>
                      {#if row.comment.path && row.comment.replyToId === null && detail.state === 'OPEN'}<Button variant="ghost" size="sm" class="ml-3 mb-2" onclick={() => { replyTarget = row.comment; replyBody = ''; }}>Reply</Button>{/if}
                    </div>
                  {:else if row.kind === 'review'}
                    <div class="entry">
                      <div class="entry-head">{@render avatar(row.review.author)}<strong>{row.review.author}</strong><span class="review-state">{row.review.state.replaceAll('_', ' ').toLowerCase()}</span><span class="entry-age" title={exactLocalTime(row.at)}>{age(row.at)}</span></div>
                      {#if row.review.body.trim()}<div class="entry-body"><ConversationMessage text={row.review.body} role="assistant" showImages /></div>{/if}
                    </div>
                  {:else}
                    <div class="event">
                      <span class="event-icon {row.kind}">
                        {#if row.kind === 'commit'}<GitCommitHorizontal size={14} aria-hidden="true" />
                        {:else if row.kind === 'merged'}<GitMerge size={14} aria-hidden="true" />
                        {:else if row.kind === 'closed'}<GitPullRequestClosed size={14} aria-hidden="true" />
                        {:else}<GitPullRequest size={14} aria-hidden="true" />{/if}
                      </span>
                      <span class="event-text">{#if row.kind === 'commit'}{row.headline}{:else if row.kind === 'opened'}{row.author} opened this pull request{:else if row.kind === 'merged'}{row.author || 'Someone'} merged this pull request{:else}This pull request was closed{/if}</span>
                      {#if row.kind === 'commit'}<code class="oid">{row.oid.slice(0, 7)}</code>{@render avatar(row.author)}{/if}
                      <span class="entry-age" title={exactLocalTime(row.at)}>{age(row.at)}</span>
                    </div>
                  {/if}
                {/each}
                {#if replyTarget}
                  <div class="composer">
                    <strong>Reply to {replyTarget.author} on {replyTarget.path}:{replyTarget.line}</strong>
                    <textarea aria-label="Review reply" rows="3" bind:value={replyBody}></textarea>
                    <div class="draft-actions"><Button variant="ghost" size="sm" onclick={() => { replyTarget = null; replyBody = ''; }}>Cancel</Button><Button size="sm" disabled={!replyBody.trim() || posting || postUncertain} onclick={previewReply}>Review reply…</Button></div>
                  </div>
                {/if}
              </section>
              {#if detail.state === 'OPEN'}
                <section class="composer" aria-label="Review comment">
                  <textarea id="pr-review-summary" aria-label="Overall review comment" rows="3" placeholder="Leave a review comment…" bind:value={draftSummary} oninput={() => { if (detail && draftSummary.trim()) draftHeadSha = detail.headSha; }}></textarea>
                  {#each draftLines as line, index (index)}
                    <div class="draft-line"><strong>{line.path}:{line.line} · {line.side === 'LEFT' ? 'before' : 'after'}</strong><textarea rows="2" aria-label={`Draft comment on ${line.path} line ${line.line}`} bind:value={line.body}></textarea><Button variant="ghost" size="sm" onclick={() => draftLines = draftLines.filter((_, row) => row !== index)}>Remove</Button></div>
                  {/each}
                  <div class="draft-actions">
                    <Button variant="ghost" size="sm" disabled={helperDrafting} onclick={() => void draftWithHelper('overall')}>{helperDrafting ? 'Drafting…' : 'Draft with Helper'}</Button>
                    <span class="spacer"></span>
                    <Button variant="ghost" size="sm" disabled={!draftSummary.trim() && draftLines.length === 0 && !lineTarget} onclick={cancelDrafts}>Discard</Button>
                    <Button size="sm" disabled={!draftSummary.trim() || posting || postUncertain} onclick={previewReview}>Review and post…</Button>
                  </div>
                </section>
              {/if}
              <section class="merge-card" aria-label="Merge pull request">
                {#if detail.state === 'MERGED'}
                  <p class="merge-fact">This pull request was merged into <code>{detail.baseBranch}</code>.</p>
                {:else if detail.state === 'OPEN'}
                  <div class="merge-fact">
                    <span class:merge-good={detail.checks.length > 0 && detail.checks.every((check) => checkTone(check) === 'good')} aria-hidden="true">{detail.checks.length > 0 && detail.checks.every((check) => checkTone(check) === 'good') ? '✓' : '•'}</span>
                    {detail.checks.length === 0 ? 'No checks reported' : `${detail.checks.filter((check) => checkTone(check) === 'good').length} of ${detail.checks.length} checks successful`}
                  </div>
                  <div class="merge-fact">
                    <span class:merge-good={detail.mergeable === 'MERGEABLE'} aria-hidden="true">{detail.mergeable === 'MERGEABLE' ? '✓' : '•'}</span>
                    {detail.mergeable === 'MERGEABLE' ? 'This branch has no conflicts with the base branch.' : detail.mergeable === 'CONFLICTING' ? 'This branch has conflicts with the base branch.' : 'GitHub is still checking whether this branch can merge.'}
                  </div>
                  <div class="merge-controls">
                    <Button size="sm" disabled={!canMerge} onclick={previewMerge}>{merging ? 'Merging…' : 'Merge pull request'}</Button>
                    <span>using</span>
                    <Select.Root type="single" value={mergeMethod} disabled={merging} onValueChange={(value) => { mergeMethod = value as GithubMergeRequest['method']; }}>
                      <Select.Trigger size="sm" aria-label="Merge method">{mergeMethodLabel(mergeMethod)}</Select.Trigger>
                      <Select.Content>
                        <Select.Item value="merge" label="Create merge commit" />
                        <Select.Item value="squash" label="Squash and merge" />
                        <Select.Item value="rebase" label="Rebase and merge" />
                      </Select.Content>
                    </Select.Root>
                  </div>
                  {#if detail.isDraft}<p class="merge-note">Mark this draft ready for review on GitHub before merging.</p>{/if}
                  {#if mergeUncertain}<p class="merge-note">Refresh this PR to check whether the merge completed before trying again.</p>{/if}
                  {#if mergeError}<p class="merge-note error" role="alert">{mergeError}</p>{/if}
                {:else}
                  <p class="merge-fact">This pull request was closed without merging.</p>
                {/if}
              </section>
            {/if}
          </div>
          {#if detail}
            <aside class="summary-side" aria-label="Pull request facts">
              <section>
                <h3>Threads</h3>
                {#each threads as thread (thread.ownedId)}
                  <button type="button" class="side-row" onclick={() => void sessionRowJump(thread.ownedId, 'session')}><MessageSquare size={14} aria-hidden="true" /><span class="truncate">{thread.title || 'Untitled session'}</span></button>
                {:else}<p class="side-empty">No threads</p>{/each}
              </section>
              <section>
                <h3>Comments</h3>
                {#each detail.comments as comment (comment.id)}
                  <p class="side-row">{@render avatar(comment.author)}<span class="truncate"><strong>{comment.author}</strong> {comment.body}</span></p>
                {:else}<p class="side-empty">No comments</p>{/each}
              </section>
              <section>
                <h3>Reviews</h3>
                {#each detail.reviews as review, index (index)}
                  <p class="side-row">{@render avatar(review.author)}<strong>{review.author}</strong><span class="side-meta">{review.state.replaceAll('_', ' ').toLowerCase()}</span></p>
                {:else}<p class="side-empty">No reviews</p>{/each}
                {#if detail.reviewers.length}<p class="side-empty">Requested: {detail.reviewers.join(', ')}</p>{/if}
              </section>
              <section>
                <h3>Checks</h3>
                {#each detail.checks as check, index (index)}
                  <p class="side-row"><i class="check-dot {checkTone(check)}" aria-hidden="true"></i><span class="truncate">{check.name || 'Check'}</span>{#if check.url}<a class="side-meta" href={check.url} target="_blank" rel="noreferrer">{(check.conclusion || check.status || 'unknown').replaceAll('_', ' ').toLowerCase()}</a>{:else}<span class="side-meta">{(check.conclusion || check.status || 'unknown').replaceAll('_', ' ').toLowerCase()}</span>{/if}</p>
                {:else}<p class="side-empty">No checks</p>{/each}
              </section>
            </aside>
          {/if}
        </div>
      {:else if detail}
        <!-- SEAM (TSK-1393 lane 1): the multi-file diff component replaces this
             block — file list, unified/side-by-side diff and line-comment drafts.
             Inputs it needs are already here: `detail.files` (path, status,
             additions, deletions, patch), `startLineDraft`, `openFullDiff`. -->
        <div class="changes">
          {#if draftLines.length}<p class="notice">{draftLines.length} line comment{draftLines.length === 1 ? '' : 's'} in your review draft. Post {draftLines.length === 1 ? 'it' : 'them'} from Summary.</p>{/if}
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
        </div>
      {/if}
      {#if pendingReview}
        <div class="confirm-backdrop" role="presentation">
          <div class="confirm-review" role="dialog" aria-modal="true" aria-label="Confirm pull request review">
            <h3>Post this review?</h3>
            <p><strong>{pullRequestSelection.selected.repository} #{pendingReview.number}</strong> · head {pendingReview.expectedHeadSha.slice(0, 10)}</p>
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
        <div class="confirm-backdrop" role="presentation"><div class="confirm-review" role="dialog" aria-modal="true" aria-label="Confirm review reply"><h3>Post this reply?</h3><p><strong>{pullRequestSelection.selected.repository} #{pullRequestSelection.selected.number}</strong> · comment {pendingReply.commentId}</p><pre>{pendingReply.body}</pre><p>GitHub will notify participants. Assembly will check the PR head again before submitting.</p><div class="draft-actions"><button type="button" onclick={() => pendingReply = null}>Cancel</button><button type="button" class="confirm-submit" onclick={() => void confirmReply()}>Post reply to GitHub</button></div></div></div>
      {/if}
      {#if pendingMerge}
        <div class="confirm-backdrop" role="presentation">
          <div class="confirm-review" role="dialog" aria-modal="true" aria-label="Confirm pull request merge">
            <h3>Merge this pull request?</h3>
            <p><strong>{pendingMerge.repository} #{pendingMerge.number}</strong> · {pendingMerge.title}</p>
            <p><code>{pendingMerge.headBranch}</code> into <code>{pendingMerge.baseBranch}</code> using {mergeMethodLabel(pendingMerge.method)}.</p>
            <p>Head {pendingMerge.expectedHeadSha.slice(0, 10)} · base {pendingMerge.expectedBaseSha.slice(0, 10)}</p>
            {#if draftSummary.trim() || draftLines.length > 0}<p>Your unposted review draft will be cleared if the merge succeeds.</p>{/if}
            <p>Assembly checks the PR again before asking GitHub to merge it. GitHub enforces the repository's merge rules.</p>
            <div class="draft-actions"><button type="button" onclick={() => pendingMerge = null}>Cancel</button><button type="button" class="confirm-submit" onclick={() => void confirmMerge()}>Merge on GitHub</button></div>
          </div>
        </div>
      {/if}
    {:else if !linkMissing}<p class="notice">Choose a pull request to review.</p>{/if}
  </section>
</div>

<style>
  .workspace{display:grid;grid-template-columns:minmax(240px,300px) minmax(0,1fr);height:100%;min-height:0;background:var(--color-bg);color:var(--color-text)}
  .queue{display:flex;flex-direction:column;gap:var(--space-2);min-width:0;min-height:0;border-right:1px solid var(--color-border);padding:var(--space-4) var(--space-3)}
  .queue-heading{display:flex;align-items:center;justify-content:space-between;gap:var(--space-2);padding:0 var(--space-1)}.queue-heading h2{font-size:var(--text-heading);font-weight:var(--text-heading-weight);margin:0}
  .filters{display:flex;gap:var(--space-2);min-width:0}
  .rows{overflow:auto;min-height:0;flex:1;margin:0 calc(-1 * var(--space-1))}
  .row{display:flex;align-items:flex-start;gap:var(--space-2);text-align:left;width:100%;padding:var(--space-2);border:0;border-radius:8px;background:transparent;color:inherit;font:inherit;cursor:pointer}.row:hover{background:var(--color-hover)}.row.selected{background:var(--color-selected)}.row:focus-visible{outline:2px solid var(--color-focus);outline-offset:-2px}
  .row-icon{display:flex;flex:none;padding-top:2px;color:var(--color-good)}.row-icon.merged{color:var(--color-accent)}.row-icon.bad{color:var(--color-bad)}.row-icon.idle{color:var(--color-text-3)}
  .row-text{display:flex;flex-direction:column;gap:2px;min-width:0}.row-title,.row-meta{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.row-title{font-size:var(--text-quiet)}.row-meta{font-size:12px;color:var(--color-text-3)}
  .detail{min-width:0;overflow:auto;padding:0 var(--space-6) var(--space-6)}
  .detail-bar{position:sticky;top:0;z-index:2;display:flex;align-items:center;gap:var(--space-2);padding:var(--space-3) 0;background:var(--color-bg)}
  .change-totals{font:12px ui-monospace,monospace}.add{color:var(--color-good)}.del{color:var(--color-bad)}
  .bar-actions{display:flex;align-items:center;gap:var(--space-1);margin-left:auto}
  .notice{font-size:var(--text-quiet);color:var(--color-text-3);margin:var(--space-3) 0}.notice.error{color:var(--color-bad)}.notice a{color:var(--color-accent)}
  .summary{display:grid;grid-template-columns:minmax(0,760px) minmax(200px,280px);gap:var(--space-6);align-items:start;padding-top:var(--space-3)}
  .summary-main{min-width:0}
  .pr-kicker{display:flex;align-items:center;gap:var(--space-2);font-size:var(--text-quiet);color:var(--color-text-3)}
  .state-pill{display:inline-flex;align-items:center;gap:5px;border-radius:999px;padding:3px 10px;font-size:12px;font-weight:600;background:var(--color-elevated);color:var(--color-text-2)}.state-pill.good{background:var(--color-good-bg);color:var(--color-good)}.state-pill.bad{background:var(--color-bad-bg);color:var(--color-bad)}.state-pill.merged{background:var(--color-selected);color:var(--color-accent)}
  .pr-title{font-size:22px;font-weight:600;line-height:1.3;margin:var(--space-2) 0;overflow-wrap:anywhere}
  .pr-meta{display:flex;flex-wrap:wrap;align-items:center;gap:var(--space-2);font-size:var(--text-quiet);color:var(--color-text-3)}.pr-meta strong{color:var(--color-text);font-weight:500}
  .branches{display:inline-flex;align-items:center;gap:var(--space-1)}
  code{font:12px ui-monospace,monospace;color:var(--color-text-2)}
  .avatar{width:20px;height:20px;flex:none;border-radius:50%;background:var(--color-elevated)}
  .pr-body{margin-top:var(--space-5);font-size:var(--text-body);overflow-wrap:anywhere}
  .activity{margin-top:var(--space-5);padding-top:var(--space-5);border-top:1px solid var(--color-border)}.activity h2{font-size:var(--text-heading);font-weight:var(--text-heading-weight);margin:0 0 var(--space-3)}
  .event,.entry{border-radius:12px;background:var(--color-surface);margin-bottom:var(--space-2);font-size:var(--text-quiet)}
  .event{display:flex;align-items:center;gap:var(--space-3);padding:var(--space-2) var(--space-3)}
  .event-icon{display:flex;flex:none;color:var(--color-text-3)}.event-icon.opened{color:var(--color-good)}.event-icon.merged{color:var(--color-accent)}.event-icon.closed{color:var(--color-bad)}
  .event-text{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .oid{color:var(--color-text-3)}.entry-age{flex:none;margin-left:auto;font-size:12px;color:var(--color-text-3)}.event .entry-age{margin-left:0}
  .entry{overflow:hidden}.entry-head{display:flex;flex-wrap:wrap;align-items:center;gap:var(--space-2);padding:var(--space-2) var(--space-3);color:var(--color-text-3)}.entry-head strong{color:var(--color-text);font-weight:500}
  .entry-body{padding:0 var(--space-3) var(--space-3) calc(var(--space-3) + 28px);overflow-wrap:anywhere}
  .review-state{text-transform:capitalize}
  .composer{display:flex;flex-direction:column;gap:var(--space-2);margin-top:var(--space-3);border-radius:12px;background:var(--color-surface);padding:var(--space-3)}.composer strong{font-size:12px}
  .composer textarea{display:block;width:100%;resize:vertical;min-height:56px;background:var(--color-bg);border:1px solid var(--color-field-border);border-radius:8px;color:var(--color-text);padding:var(--space-2);font:var(--text-quiet)/1.5 inherit}.composer textarea:focus-visible{outline:none;border-color:var(--color-focus)}
  .draft-line{display:flex;flex-direction:column;gap:var(--space-1)}
  .draft-actions{display:flex;align-items:center;justify-content:flex-end;gap:var(--space-2)}.spacer{flex:1}
  .merge-card{margin-top:var(--space-4);border-radius:12px;background:var(--color-surface);overflow:hidden;font-size:var(--text-quiet)}
  .merge-fact{display:flex;align-items:center;gap:var(--space-3);margin:0;padding:var(--space-3) var(--space-4);border-bottom:1px solid var(--color-border)}.merge-fact:last-child{border-bottom:0}
  .merge-fact span{color:var(--color-text-3);font-size:15px}.merge-fact span.merge-good{color:var(--color-good)}
  .merge-controls{display:flex;flex-wrap:wrap;align-items:center;gap:var(--space-2);padding:var(--space-3) var(--space-4)}.merge-controls span{color:var(--color-text-2)}
  .merge-note{margin:0;padding:0 var(--space-4) var(--space-3);color:var(--color-text-3)}.merge-note.error{color:var(--color-bad)}
  .summary-side{position:sticky;top:56px;display:flex;flex-direction:column;font-size:var(--text-quiet)}
  .summary-side section{padding:var(--space-3) 0;border-bottom:1px solid var(--color-border)}.summary-side section:last-child{border-bottom:0}
  .summary-side h3{font-size:var(--text-quiet);font-weight:500;color:var(--color-text-3);margin:0 0 var(--space-2)}
  .side-row{display:flex;align-items:center;gap:var(--space-2);width:100%;min-width:0;margin:0;padding:var(--space-1) 0;border:0;background:transparent;color:var(--color-text);font:inherit;text-align:left}
  button.side-row{cursor:pointer;border-radius:8px}button.side-row:hover{background:var(--color-hover)}button.side-row:focus-visible{outline:2px solid var(--color-focus)}
  .side-row strong{font-weight:500}.side-meta{margin-left:auto;flex:none;color:var(--color-text-3);text-transform:capitalize}.side-empty{margin:0;color:var(--color-text-3)}
  .check-dot{width:8px;height:8px;flex:none;border-radius:50%;background:var(--color-attention)}.check-dot.good{background:var(--color-good)}.check-dot.bad{background:var(--color-bad)}
  .changes{padding-top:var(--space-2)}
  .files-layout{display:flex;flex-direction:column;height:72vh;min-height:450px;border-radius:12px;background:var(--color-surface);overflow:hidden}.file-list{min-width:0;max-height:160px;flex:none;border-bottom:1px solid var(--color-border);overflow:auto;padding:4px 0}.file-list button{display:flex;align-items:center;gap:8px;width:100%;padding:5px 10px;text-align:left;border:0;background:transparent;color:inherit;font:inherit;font-size:12px;cursor:pointer}.file-list button:hover{background:var(--color-hover)}.file-list button.selected{background:var(--color-selected)}
  .file-status{width:14px;flex:none;text-align:center;font:600 11px ui-monospace,monospace;color:var(--color-attention)}.file-status.added{color:var(--color-good)}.file-status.removed{color:var(--color-bad)}.file-name{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.file-name small{margin-left:6px}.file-counts{display:flex;gap:5px;flex:none;font:11px ui-monospace,monospace}.file-counts .add{color:var(--color-good)}.file-counts .del{color:var(--color-bad)}
  .file-list small,.file-diff h3 small{color:var(--color-text-3);font-weight:400}.file-diff{min-width:0;min-height:0;flex:1;overflow:auto;padding:0 0 16px}.file-toolbar{position:sticky;top:0;left:0;z-index:1;display:flex;flex-wrap:wrap;align-items:center;gap:8px;padding:7px 12px;background:var(--color-surface);border-bottom:1px solid var(--color-border)}.file-toolbar h3{flex-basis:100%;min-width:0;font:12px ui-monospace,monospace;margin:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.hunk-heading{background:var(--color-elevated);padding:7px 12px;font:11px ui-monospace,monospace;color:var(--color-text-3)}.diff-line{display:flex;min-width:max-content;font:11px/1.55 ui-monospace,monospace;white-space:pre}.diff-line.added{background:rgba(80,250,123,.12)}.diff-line.removed{background:rgba(255,85,85,.13)}.line-no{width:48px;flex:none;text-align:right;padding:0 7px;color:var(--color-text-3);user-select:none}.line-text{padding:0 12px;tab-size:4}
  .diff-modes{display:flex;flex:none;border:1px solid var(--color-border);border-radius:6px;overflow:hidden}.diff-modes button{border:0;background:transparent;padding:4px 9px;font-size:11px;color:var(--color-text-2);cursor:pointer}.diff-modes button.active{background:var(--color-elevated);color:var(--color-text)}.native-diff{height:560px;min-height:0}
  .open-full-diff{flex:none;border:1px solid var(--color-accent);border-radius:6px;background:var(--color-accent);color:var(--color-on-accent);padding:5px 10px;font-size:11px;font-weight:600;cursor:pointer}.open-full-diff:disabled{opacity:.5;cursor:default}
  .diff-line .add-comment{width:24px;flex:none;border:0;background:transparent;color:var(--color-text-3);opacity:0;cursor:pointer}.diff-line:hover .add-comment,.diff-line .add-comment:focus-visible{opacity:1}.diff-line .add-comment:disabled{visibility:hidden}
  .line-compose{margin:8px 12px;padding:12px;border:1px solid var(--color-border);border-radius:6px}.line-compose strong{display:block;font-size:12px;margin-bottom:8px}.line-compose textarea{display:block;width:100%;resize:vertical;min-height:56px;background:var(--color-bg);border:1px solid var(--color-border);border-radius:6px;color:var(--color-text);padding:9px;font:12px/1.5 inherit}.line-compose .draft-actions{margin-top:8px}.line-compose .draft-actions button{border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);color:inherit;padding:6px 10px;font-size:12px;cursor:pointer}.line-compose .draft-actions button:disabled{opacity:.5;cursor:default}
  .confirm-backdrop{position:fixed;inset:0;z-index:1000;display:grid;place-items:center;background:rgba(0,0,0,.72)}.confirm-review{width:min(680px,calc(100vw - 40px));max-height:calc(100vh - 40px);overflow:auto;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:0 20px 70px rgba(0,0,0,.45);padding:22px}.confirm-review h3{font-size:18px;margin:0 0 12px}.confirm-review h4{font-size:12px;margin:20px 0 6px}.confirm-review p{font-size:12px;color:var(--color-text-3)}.confirm-review pre{white-space:pre-wrap;overflow-wrap:anywhere;font:12px/1.5 ui-monospace,monospace;border:1px solid var(--color-border);border-radius:6px;padding:10px;max-height:250px;overflow:auto}.confirm-review .draft-actions button{border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);color:inherit;padding:7px 10px;font-size:12px;cursor:pointer}.confirm-submit{background:var(--color-accent)!important;color:var(--color-bg)!important}
  @media(max-width:1100px){.summary{grid-template-columns:minmax(0,1fr)}.summary-side{position:static}}
  @media(max-width:700px){.workspace{grid-template-columns:minmax(180px,35%) minmax(0,1fr)}.queue{padding:10px 6px}.detail{padding:0 14px 14px}}
</style>
