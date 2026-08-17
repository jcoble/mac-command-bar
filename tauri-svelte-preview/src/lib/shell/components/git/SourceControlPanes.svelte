<script lang="ts">
  /**
   * SourceControlPanes.svelte — the whole source-control surface: the
   * repository strip, the changed files with the commit box, and the commit
   * history drawn as a graph.
   *
   * It replaces the body of `GitPanel.svelte` and keeps that panel's contract:
   * no required props, no IO of its own, no `$effect`. Every git call is made
   * by `gitService` (the working copy) or `gitCommitFilesService` (a commit's
   * files), and only ever from a click. The integrator points the panel at a
   * repository with `gitService.activate(root)`, exactly as it does today.
   *
   * THE DIFF IS NOT DRAWN HERE. Clicking a file — whether a changed file or a
   * file inside a commit — writes it into the state `GitDiffView` already
   * reads, so the changes show up wherever that view is mounted. This round
   * that is meant to be a panel of its own in the middle of the shell; see
   * `_(git)-INTEGRATION.md` next to this file.
   *
   * WRITING vs READING. In the desktop app everything works. In a browser the
   * dev server can READ the repository (see `src/lib/server/gitBridge.ts`) but
   * deliberately cannot change it, so committing, staging and the remote
   * buttons are switched off there and say why. A browser tab left open on a
   * repository with work in progress must not be able to commit it by accident.
   */
  import { canChangeRepository, READ_ONLY_IN_BROWSER_MESSAGE } from '$lib/shell/git/gitBackendExtra';
  import { getConversationSession } from '$lib/shell/conversation/conversationStore.svelte';
  import {
    gitCommitFilesService as defaultCommitFilesService,
    type GitCommitFilesService
  } from '$lib/shell/git/gitCommitFilesService';
  import {
    gitCommitFiles as defaultCommitFilesState,
    type GitCommitFilesState
  } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import { isNotARepositoryError } from '$lib/shell/git/gitPanelStore.svelte';
  import { gitService as defaultService, type GitService } from '$lib/shell/git/gitService';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import {
    generateCommitMessageFromTauri,
    type AgentGenerationRequest
  } from '$lib/tauriSource';

  import ChangesPane from './ChangesPane.svelte';
  import DiscardConfirmDialog from './DiscardConfirmDialog.svelte';
  import GraphPane from './GraphPane.svelte';
  import RepoPane from './RepoPane.svelte';
  import PullRequestList from './pr/PullRequestList.svelte';
  import PullRequestPanel from './pr/PullRequestPanel.svelte';
  import {
    describeDiscardQuestion,
    type DiscardQuestion,
    type DiscardTarget
  } from './discardConfirm';
  import { isGitFileUntracked } from '$lib/shell/git/gitPanelStore.svelte';

  interface Props {
    /** The working-copy service. The shell's singleton unless a page says otherwise. */
    service?: GitService;
    /** The per-commit file service, likewise. */
    commitFiles?: GitCommitFilesService;
    /** The state that service writes, read by the graph's open rows. */
    commitFilesState?: GitCommitFilesState;
    /** Can this page change the repository? Defaults to "only in the desktop app". */
    canWrite?: boolean;
    /** A file's changes were just picked. The panes have no idea where the diff
     * is drawn — the shell mounts `GitDiffView` as a tab of its own — so this is
     * how they ask for it to be brought to the front. */
    onShowDiff?: () => void;
  }
  let {
    service = defaultService,
    commitFiles = defaultCommitFilesService,
    commitFilesState = defaultCommitFilesState,
    canWrite = canChangeRepository(),
    onShowDiff
  }: Props = $props();

  const panel = $derived(service.state);

  /** A folder with no repository in it is an ordinary thing to be looking at,
   * not a fault — so it gets a plain sentence instead of git's `fatal:` line.
   * Any other failure keeps its own message. */
  const notARepository = $derived(
    isNotARepositoryError(panel.statusError) || isNotARepositoryError(panel.historyError)
  );

  interface AgentIdentity {
    ownedId: string;
    generation: number;
  }

  function activeAgentIdentity(): AgentIdentity | null {
    const ownedId = rail.activeOwnedId;
    if (!ownedId) return null;
    const conversation = getConversationSession(ownedId);
    if (
      !conversation ||
      conversation.generation < 1 ||
      conversation.executionOwner !== 'structured' ||
      !['connected', 'reconnecting'].includes(conversation.connectionState)
    ) {
      return null;
    }
    return { ownedId, generation: conversation.generation };
  }

  const activeAgent = $derived(activeAgentIdentity());
  const agentUnavailableReason = 'No active agent session is running. Start an agent conversation to generate this message.';
  let showPullRequest = $state(false);

  /**
   * THE ONLY ROUTE TO A DISCARD IN THIS SHELL.
   *
   * The panes never call `discardPaths` or `discardAll`; they hand a request up
   * here, this decides what the question says, and only pressing the button in
   * the dialog runs anything. Keeping the pending request in one place is what
   * makes that true — there is no second path a later change can add by
   * accident without going through this file.
   */
  let pendingDiscard = $state<{ question: DiscardQuestion; run: () => void } | null>(null);

  function askToDiscardFiles(targets: DiscardTarget[]): void {
    if (targets.length === 0) return;
    pendingDiscard = {
      question: describeDiscardQuestion({ scope: 'file', targets }),
      run: () => void service.discardPaths(targets.map((target) => target.relativePath))
    };
  }

  function askToDiscardAll(): void {
    const files = panel.status?.files ?? [];
    if (files.length === 0) return;
    const untrackedCount = files.filter(isGitFileUntracked).length;
    pendingDiscard = {
      question: describeDiscardQuestion({ scope: 'all', targets: [], untrackedCount }),
      run: () => void service.discardAll(untrackedCount > 0)
    };
  }

  function confirmDiscard(): void {
    const pending = pendingDiscard;
    pendingDiscard = null;
    pending?.run();
  }

  async function generateCommitMessage(): Promise<string> {
    const root = panel.root;
    const agent = activeAgent;
    if (!root) throw new Error('No repository is selected.');
    if (!agent) throw new Error(agentUnavailableReason);
    const request: AgentGenerationRequest = {
      root,
      ownedId: agent.ownedId,
      generation: agent.generation
    };
    const result = await generateCommitMessageFromTauri(request);
    if (!result) throw new Error(READ_ONLY_IN_BROWSER_MESSAGE);
    return result;
  }

  /** The card the Stats & Usage screen uses for "there is nothing to show". */
  const EMPTY_CARD =
    'flex shrink-0 flex-col gap-1 rounded-[var(--radius-md)] border px-3 py-4 text-center ' +
    'border-[color-mix(in_srgb,var(--color-border)_36%,transparent)] ' +
    'bg-[color-mix(in_srgb,var(--color-elevated)_38%,var(--color-surface))]';
</script>

<!-- One padded column of cards, with real space between them, so the strip, the
     changes and the history read as separate things rather than one flat list. -->
<div
  class="flex h-full min-h-0 w-full flex-col gap-2 overflow-hidden bg-[var(--color-bg)] p-2
         text-[var(--color-text)]"
  aria-label="Source control"
>
  <RepoPane
    {panel}
    {service}
    {canWrite}
    readOnlyReason={READ_ONLY_IN_BROWSER_MESSAGE}
    onOpenPullRequest={() => (showPullRequest = true)}
  />

  {#if showPullRequest && panel.activated && !panel.desktopOnly && !notARepository}
    <PullRequestPanel
      root={panel.root}
      status={panel.status}
      {activeAgent}
      {canWrite}
      onClose={() => (showPullRequest = false)}
    />
  {/if}

  {#if !panel.activated}
    <div class={EMPTY_CARD}>
      <strong class="text-[13px] leading-[18px] font-medium text-[var(--color-text)]">
        No project selected yet
      </strong>
      <p class="text-[12px] leading-[16px] text-[var(--color-text-2)]">
        Pick a session and this panel will show that folder's source control.
      </p>
    </div>
  {:else if panel.desktopOnly}
    <div class={EMPTY_CARD}>
      <strong class="text-[13px] leading-[18px] font-medium text-[var(--color-text)]">
        Source control runs in the desktop app
      </strong>
      <p class="text-[12px] leading-[16px] text-[var(--color-text-2)]">
        Nothing is loaded here.
      </p>
    </div>
  {:else if notARepository}
    <div class={EMPTY_CARD}>
      <strong class="text-[13px] leading-[18px] font-medium text-[var(--color-text)]">
        This folder is not a git repository
      </strong>
      <p class="text-[12px] leading-[16px] text-[var(--color-text-2)]">
        Open a folder that has a .git directory in it, or one inside a repository.
      </p>
    </div>
  {:else}
    <PullRequestList root={panel.root} />
    <ChangesPane
      {panel}
      {service}
      {canWrite}
      readOnlyReason={READ_ONLY_IN_BROWSER_MESSAGE}
      onGenerateCommitMessage={generateCommitMessage}
      agentAvailable={activeAgent !== null}
      {agentUnavailableReason}
      onRequestDiscard={askToDiscardFiles}
      onRequestDiscardAll={askToDiscardAll}
      {onShowDiff}
    />
    <GraphPane {panel} {service} {commitFiles} files={commitFilesState} {onShowDiff} />
  {/if}

  <DiscardConfirmDialog
    question={pendingDiscard?.question ?? null}
    open={pendingDiscard !== null}
    onOpenChange={(next) => {
      if (!next) pendingDiscard = null;
    }}
    onConfirm={confirmDiscard}
  />
</div>
