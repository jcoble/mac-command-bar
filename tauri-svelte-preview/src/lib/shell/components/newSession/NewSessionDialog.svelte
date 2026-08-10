<!--
  NewSessionDialog.svelte — start a conversation: pick an agent, choose where
  it should work, optionally name the session, then press Start session.

  WHAT IT DOES NOT DO
   - It never creates a git worktree. The checkout list offers what already
     exists; "I need a new one" hands over the `git worktree add` line to run in
     a terminal. Making a worktree writes to a repository someone is working in
     and can only be undone by hand, so it stays something a person types.
   - It never starts anything itself. Pressing Start calls `onStart` with four
     plain values and closes; spawning the terminal belongs to the page, which
     owns the session rail (see `_(new-session)-INTEGRATION.md`).

  WHERE THE RULES LIVE
  Every decision — the list of things you can launch, the command preview, what
  counts as a usable folder, how the folder list is merged, the worktree
  command — is a pure function in `newSession/newSessionFlow.ts`, covered by
  `scripts/newSessionFlow.test.mjs`. This file is the screen and nothing more.

  NO `$effect`: the two things that load (the checkouts for a project, and
  checking a typed-in folder) run from the handler of the thing the user did.
  The host calls `reset()` each time the dialog is opened.

  Usage (see the host next to this file):
    <NewSessionDialog bind:open bind:this={dialog} {onStart} />
    dialog.reset({ sessionRoots: rail.owned.map((s) => s.cwd) });
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Copy from '@lucide/svelte/icons/copy';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import {
    canPickFolder,
    listWorktrees,
    pickProjectFolder,
    validateProjectRoot
  } from '$lib/shell/newSession/newSessionBackend';
  import {
    LAUNCH_CATALOG,
    buildCommandPreview,
    buildNewSessionRequest,
    resolveSessionTitle,
    selectLaunchAgent,
    suggestSessionTitle,
    validateNewSession,
    worktreeAddCommand,
    worktreeChoicesFor,
    type LaunchAgent,
    type NewSessionRequest
  } from '$lib/shell/newSession/newSessionFlow';
  import {
    addCustomRoot,
    hydrate,
    initialRootPath,
    knownRoots,
    rememberLastUsed,
    setSessionRoots
  } from '$lib/shell/newSession/projectRootsStore.svelte';
  import type { ProjectWorktree } from '$lib/tauriSource';

  interface Props {
    /** Whether the dialog is showing. */
    open?: boolean;
    /**
     * Start the session the user described. Everything needed is in the one
     * argument: the folder to open the terminal in, the name for its row, which
     * agent is running, and the command to type (`null` for a plain terminal).
     *
     * The dialog closes as soon as this resolves. Anything it throws is shown
     * here instead, with the dialog left open so the user can try again.
     */
    onStart: (request: NewSessionRequest) => void | Promise<void>;
  }

  let { open = $bindable(false), onStart }: Props = $props();

  // ── What the user has chosen ───────────────────────────────────────────────
  let projectPath = $state('');
  let checkoutPath = $state('');
  let selectedLaunch = $state(selectLaunchAgent('codex'));
  let title = $state('');

  // ── What the screen is busy with ───────────────────────────────────────────
  let worktrees = $state<ProjectWorktree[] | null>(null);
  let worktreeMessage = $state<string | null>(null);
  let loadingCheckouts = $state(false);
  let addingFolder = $state(false);
  let typedPath = $state('');
  let typedPathMessage = $state<string | null>(null);
  let checkingTypedPath = $state(false);
  let typedPathOpen = $state(!canPickFolder());
  let advancedOpen = $state(false);
  let newWorktreeBranch = $state('');
  let copiedLabel = $state<string | null>(null);
  let starting = $state(false);
  let startFailure = $state<string | null>(null);
  let dialogContent = $state<HTMLElement | null>(null);

  // ── What follows from it ───────────────────────────────────────────────────
  const roots = $derived(knownRoots());
  const conversationAgents = LAUNCH_CATALOG.filter((option) => option.agent !== 'shell');
  const checkouts = $derived(worktreeChoicesFor({ projectRoot: projectPath, worktrees }));
  /** The folder the terminal actually opens in: the chosen checkout. */
  const cwd = $derived(checkoutPath || projectPath);
  const draft = $derived({ cwd, title, ...selectedLaunch });
  const problems = $derived(validateNewSession(draft));
  const preview = $derived(buildCommandPreview({ cwd, command: selectedLaunch.command }));
  const titlePlaceholder = $derived(suggestSessionTitle({ cwd, agent: selectedLaunch.agent }));
  const resolvedTitle = $derived(resolveSessionTitle({ cwd, agent: selectedLaunch.agent, title }));
  const addWorktreeLine = $derived(worktreeAddCommand(projectPath, newWorktreeBranch));
  const projectLabel = $derived(
    roots.find((root) => root.path === projectPath)?.name ?? 'Choose a project folder'
  );
  const checkoutLabel = $derived(
    checkouts.find((choice) => choice.path === cwd)?.label ?? 'Choose a working copy'
  );
  const readyToStart = $derived(problems.length === 0 && !starting);

  /**
   * Put the dialog back to a clean state and load what the chosen project has.
   * The host calls this every time it opens the dialog, so a second visit is
   * never looking at the last visit's checkouts.
   */
  export function reset(input: { sessionRoots?: string[] } = {}): void {
    hydrate();
    setSessionRoots(input.sessionRoots ?? []);
    startFailure = null;
    typedPathMessage = null;
    typedPath = '';
    typedPathOpen = !canPickFolder();
    advancedOpen = false;
    newWorktreeBranch = '';
    copiedLabel = null;
    pickAgent('codex');
    title = '';
    const path = initialRootPath();
    if (path) {
      void pickProject(path);
    } else {
      projectPath = '';
      checkoutPath = '';
      worktrees = null;
      worktreeMessage = null;
    }
  }

  /** Choose a project and read its checkouts. */
  async function pickProject(path: string): Promise<void> {
    projectPath = path;
    checkoutPath = path;
    worktrees = null;
    worktreeMessage = null;
    await loadCheckouts();
  }

  /** Read the checkouts for the chosen project. Only ever reads. */
  async function loadCheckouts(): Promise<void> {
    if (!projectPath || loadingCheckouts) return;
    loadingCheckouts = true;
    worktreeMessage = null;
    try {
      const answer = await listWorktrees(projectPath);
      if (answer.status === 'ok') {
        worktrees = answer.value;
      } else {
        worktrees = null;
        // Not a failure worth blocking on: the project's own folder is still
        // offered, and starting there is the common case anyway.
        worktreeMessage = answer.message;
      }
      // A reload can drop the checkout that was selected (a worktree removed
      // since last time). Fall back to the project folder rather than leaving a
      // folder selected that is no longer there.
      if (!checkouts.some((choice) => choice.path === checkoutPath)) {
        checkoutPath = projectPath;
      }
    } finally {
      loadingCheckouts = false;
    }
  }

  /** Choose what to run. Picking an agent rewrites the command box. */
  function pickAgent(next: LaunchAgent): void {
    selectedLaunch = selectLaunchAgent(next);
  }

  /** Ask the system for a folder and add it to the list. */
  async function chooseFolder(): Promise<void> {
    if (addingFolder) return;
    addingFolder = true;
    typedPathMessage = null;
    try {
      const answer = await pickProjectFolder();
      if (answer.status !== 'ok') {
        typedPathMessage = answer.message;
        return;
      }
      // The user closed the chooser without picking. Nothing to say.
      if (answer.value === null) return;
      const added = addCustomRoot(answer.value);
      if (!added) {
        typedPathMessage = 'That folder cannot be used as a project.';
        return;
      }
      await pickProject(added.path);
    } finally {
      addingFolder = false;
    }
  }

  /** Add a folder the user typed, once the machine agrees it is really there. */
  async function addTypedPath(): Promise<void> {
    if (checkingTypedPath) return;
    checkingTypedPath = true;
    typedPathMessage = null;
    try {
      const answer = await validateProjectRoot(typedPath);
      if (answer.status === 'failed') {
        typedPathMessage = answer.message;
        return;
      }
      if (answer.status === 'ok' && !answer.value.isDirectory) {
        typedPathMessage = answer.value.message || 'There is no folder at that path.';
        return;
      }
      // `unavailable` means nobody could check — in the browser there is no
      // machine to ask. The path is taken at its word; the shape of it is still
      // checked by the picker itself.
      const added = addCustomRoot(typedPath);
      if (!added) {
        typedPathMessage = 'Type the full path to a folder, starting with a slash.';
        return;
      }
      typedPath = '';
      await pickProject(added.path);
    } finally {
      checkingTypedPath = false;
    }
  }

  /** Put a line on the clipboard and say so for a moment. */
  async function copy(label: string, text: string): Promise<void> {
    if (!text || typeof navigator === 'undefined' || !navigator.clipboard) return;
    try {
      await navigator.clipboard.writeText(text);
      copiedLabel = label;
      setTimeout(() => {
        if (copiedLabel === label) copiedLabel = null;
      }, 1500);
    } catch {
      // A clipboard the browser will not open is not worth an alert: the text
      // is on screen and can be selected.
    }
  }

  /** Move keyboard focus to the first conversation choice whenever the dialog opens. */
  function focusFirstAgent(event: Event): void {
    event.preventDefault();
    requestAnimationFrame(() => {
      dialogContent?.querySelector<HTMLButtonElement>('[data-agent-choice]')?.focus();
    });
  }

  /** Hand the session over to whoever owns the rail, then close. */
  async function start(): Promise<void> {
    const request = buildNewSessionRequest(draft);
    if (!request || starting) return;
    console.warn('mcb next: new-session submit', {
      agent: request.agent,
      cwd: request.cwd,
      title: request.title,
      hasCommand: request.command !== null
    });
    starting = true;
    startFailure = null;
    try {
      rememberLastUsed(projectPath);
      await onStart(request);
      open = false;
    } catch (error) {
      startFailure = error instanceof Error ? error.message : String(error);
    } finally {
      starting = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content
    bind:ref={dialogContent}
    onOpenAutoFocus={focusFirstAgent}
    class="w-[min(620px,calc(100vw-3rem))] sm:max-w-none max-h-[calc(100vh-3rem)] gap-0
           overflow-y-auto rounded-lg bg-background p-0 text-foreground ring-border
           shadow-[var(--shadow-lg)]"
  >
    <Dialog.Header class="gap-1 border-b px-5 pt-5 pb-4">
      <Dialog.Title class="text-[14px] leading-[1.4] font-semibold">New session</Dialog.Title>
      <Dialog.Description class="text-[12px] leading-[1.5] text-muted-foreground">
        Choose who you want to work with and where the conversation should begin.
      </Dialog.Description>
    </Dialog.Header>

    <div class="flex flex-col gap-5 px-5 py-5">
      <!-- ── Agent ─────────────────────────────────────────────────────── -->
      <section class="flex flex-col gap-2.5" aria-labelledby="new-session-agent-label">
        <div class="flex flex-col gap-1">
          <h3 id="new-session-agent-label" class="text-[13px] leading-[1.3] font-medium">
            Choose an agent
          </h3>
          <p class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
            Start a new conversation with Codex or Claude.
          </p>
        </div>
        <div class="grid grid-cols-2 gap-2" role="group" aria-label="Agent">
          {#each conversationAgents as option (option.agent)}
            <button
              type="button"
              data-agent-choice
              aria-pressed={selectedLaunch.agent === option.agent}
              class="flex min-h-[66px] flex-col gap-1 rounded-lg border px-3 py-2.5 text-left
                     transition-colors focus-visible:ring-3 focus-visible:ring-ring/50 outline-none
                     {selectedLaunch.agent === option.agent
                ? 'border-[var(--color-accent)] bg-[var(--color-selected)]'
                : 'border-[var(--color-border)] bg-transparent hover:bg-[var(--color-hover)]'}"
              onclick={() => pickAgent(option.agent)}
            >
              <span class="flex w-full items-center justify-between gap-2">
                <span class="text-[13px] leading-[1.3] font-medium">{option.label}</span>
                {#if selectedLaunch.agent === option.agent}
                  <Check class="size-3.5 shrink-0 text-[var(--color-accent)]" aria-hidden="true" />
                {/if}
              </span>
              <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                {option.hint}
              </span>
            </button>
          {/each}
        </div>
      </section>

      <!-- ── Working location ──────────────────────────────────────────── -->
      <section class="flex flex-col gap-3" aria-labelledby="new-session-location-label">
        <div class="flex flex-col gap-1">
          <h3 id="new-session-location-label" class="text-[13px] leading-[1.3] font-medium">
            {selectedLaunch.agent === 'shell'
              ? 'Where should the terminal open?'
              : 'Where should the agent work?'}
          </h3>
          <p class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
            Choose the project folder for this session.
          </p>
        </div>

        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between gap-3">
            <span class="text-[12px] leading-[1.3] font-medium text-[var(--color-text-2)]">
              Project folder
            </span>
            {#if roots.length > 0}
              <span class="text-[11px] text-[var(--color-text-3)]">
                {roots.length} {roots.length === 1 ? 'saved folder' : 'saved folders'}
              </span>
            {/if}
          </div>

          {#if roots.length === 0}
            <p class="text-[12px] leading-[1.5] text-[var(--color-text-2)]">
              Add a project folder to continue.
            </p>
          {:else}
            <Select.Root
              type="single"
              value={projectPath}
              onValueChange={(value) => void pickProject(value)}
            >
              <Select.Trigger class="w-full text-[13px]" aria-label="Project folder">
                {projectLabel}
              </Select.Trigger>
              <Select.Content>
                {#each roots as root (root.id)}
                  <Select.Item value={root.path} label={root.name} />
                {/each}
              </Select.Content>
            </Select.Root>
            {#if projectPath}
              <p class="truncate text-[11px] text-[var(--color-text-3)]" title={projectPath}>
                {projectPath}
              </p>
            {/if}
          {/if}

          <div class="flex flex-wrap items-center gap-2">
            {#if canPickFolder()}
              <Button
                variant="secondary"
                size="sm"
                class="text-[12px]"
                disabled={addingFolder}
                onclick={() => void chooseFolder()}
              >
                <FolderOpen class="size-3.5" aria-hidden="true" />
                {addingFolder ? 'Choosing…' : 'Choose another folder…'}
              </Button>
              <button
                type="button"
                class="rounded-md px-2 py-1 text-[12px] text-[var(--color-text-2)] transition-colors
                       hover:bg-[var(--color-hover)] hover:text-[var(--color-text)] focus-visible:ring-3
                       focus-visible:ring-ring/50 outline-none"
                aria-expanded={typedPathOpen}
                onclick={() => (typedPathOpen = !typedPathOpen)}
              >
                {typedPathOpen ? 'Hide path entry' : 'Enter a path'}
              </button>
            {/if}
          </div>

          {#if typedPathOpen}
            <div class="flex flex-wrap items-center gap-2">
              <Input
                bind:value={typedPath}
                placeholder="/Users/you/dev/project"
                aria-label="Full path to a project folder"
                class="h-8 min-w-[220px] flex-1 text-[13px]"
                onkeydown={(event: KeyboardEvent) => {
                  if (event.key === 'Enter') {
                    event.preventDefault();
                    void addTypedPath();
                  }
                }}
              />
              <Button
                variant="secondary"
                size="sm"
                class="text-[12px]"
                disabled={checkingTypedPath || typedPath.trim().length === 0}
                onclick={() => void addTypedPath()}
              >
                <FolderPlus class="size-3.5" aria-hidden="true" />
                {checkingTypedPath ? 'Checking…' : 'Add folder'}
              </Button>
            </div>
          {/if}

          {#if typedPathMessage}
            <p class="flex items-start gap-1.5 text-[12px] leading-[1.5] text-destructive">
              <TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
              <span>{typedPathMessage}</span>
            </p>
          {/if}
        </div>

        {#if projectPath}
          <div class="flex flex-col gap-2 border-t border-[var(--color-border)] pt-3">
            <div class="flex items-center gap-2">
              <div class="flex min-w-0 flex-col gap-0.5">
                <span class="text-[12px] leading-[1.3] font-medium text-[var(--color-text-2)]">
                  Working copy
                </span>
                <span class="text-[11px] leading-[1.4] text-[var(--color-text-3)]">
                  Use the main project folder or one of its separate working copies.
                </span>
              </div>
              <button
                type="button"
                class="ml-auto flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[12px]
                       text-[var(--color-text-2)] transition-colors hover:bg-[var(--color-elevated)]
                       hover:text-[var(--color-text)] focus-visible:ring-3 focus-visible:ring-ring/50
                       outline-none disabled:opacity-50"
                disabled={loadingCheckouts}
                onclick={() => void loadCheckouts()}
              >
                <RefreshCw class="size-3" aria-hidden="true" />
                {loadingCheckouts ? 'Refreshing…' : 'Refresh'}
              </button>
            </div>

            <Select.Root type="single" bind:value={checkoutPath}>
              <Select.Trigger class="w-full text-[13px]" aria-label="Working copy">
                {checkoutLabel}
              </Select.Trigger>
              <Select.Content>
                {#each checkouts as choice (choice.path)}
                  <Select.Item value={choice.path} label={choice.label} />
                {/each}
              </Select.Content>
            </Select.Root>

            {#each checkouts.filter((choice) => choice.path === cwd && choice.note) as chosen (chosen.path)}
              <p class="text-[12px] text-[var(--color-text-3)]">This folder {chosen.note}.</p>
            {/each}

            {#if worktreeMessage}
              <p class="text-[12px] leading-[1.5] text-[var(--color-text-3)]">
                {worktreeMessage} You can still start in the main project folder.
              </p>
            {/if}
          </div>
        {/if}
      </section>

      <!-- ── Name ──────────────────────────────────────────────────────── -->
      <section class="flex flex-col gap-2">
        <div class="flex items-baseline gap-2">
          <label for="new-session-title" class="text-[13px] leading-[1.3] font-medium">
            Session name
          </label>
          <span class="text-[11px] text-[var(--color-text-3)]">Optional</span>
        </div>
        <Input
          id="new-session-title"
          bind:value={title}
          placeholder={titlePlaceholder}
          class="h-8 text-[13px]"
        />
        <p class="text-[12px] leading-[1.5] text-[var(--color-text-3)]">
          Leave this blank to use “{resolvedTitle}”.
        </p>
      </section>

      <!-- ── Advanced ──────────────────────────────────────────────────── -->
      <details
        bind:open={advancedOpen}
        class="group rounded-lg border border-[var(--color-border)] bg-transparent"
      >
        <summary
          class="flex cursor-pointer list-none items-center gap-2 rounded-lg px-3 py-2.5 text-left
                 focus-visible:ring-3 focus-visible:ring-ring/50 outline-none
                 [&::-webkit-details-marker]:hidden"
        >
          <span class="flex min-w-0 flex-1 flex-col gap-0.5">
            <span class="text-[13px] leading-[1.3] font-medium">Advanced</span>
            <span class="text-[11px] leading-[1.4] text-[var(--color-text-3)]">
              Change how the session starts or prepare another working copy.
            </span>
          </span>
          <ChevronDown
            class="size-3.5 shrink-0 text-[var(--color-text-3)] transition-transform group-open:rotate-180"
            aria-hidden="true"
          />
        </summary>

        <div class="flex flex-col gap-4 border-t border-[var(--color-border)] px-3 py-3">
          <div class="flex flex-col gap-2">
            <span class="text-[12px] leading-[1.3] font-medium text-[var(--color-text-2)]">
              Terminal only
            </span>
            <button
              type="button"
              aria-pressed={selectedLaunch.agent === 'shell'}
              class="flex flex-col gap-0.5 rounded-md border px-2.5 py-2 text-left transition-colors
                     focus-visible:ring-3 focus-visible:ring-ring/50 outline-none
                     {selectedLaunch.agent === 'shell'
                ? 'border-[var(--color-accent)] bg-[var(--color-selected)]'
                : 'border-[var(--color-border)] hover:bg-[var(--color-hover)]'}"
              onclick={() => pickAgent('shell')}
            >
              <span class="flex w-full items-center justify-between gap-2 text-[12px] font-medium">
                Start without an agent
                {#if selectedLaunch.agent === 'shell'}
                  <Check class="size-3.5 shrink-0 text-[var(--color-accent)]" aria-hidden="true" />
                {/if}
              </span>
              <span class="text-[11px] leading-[1.4] text-[var(--color-text-3)]">
                Open a terminal in the selected folder without starting a conversation.
              </span>
            </button>
          </div>

          <div class="flex flex-col gap-2">
            <label
              for="new-session-command"
              class="text-[12px] leading-[1.3] font-medium text-[var(--color-text-2)]"
            >
              Starting command
            </label>
            <Input
              id="new-session-command"
              bind:value={selectedLaunch.command}
              placeholder="Leave empty to open a terminal only"
              spellcheck={false}
              class="h-8 font-mono text-[13px]"
            />
            <p class="text-[11px] leading-[1.4] text-[var(--color-text-3)]">
              Choosing an agent fills this in. Change it only when you need a custom launch.
            </p>
            <div class="flex items-start gap-2">
              <code
                class="min-w-0 flex-1 rounded-md bg-[var(--color-elevated)] px-2 py-1.5 font-mono
                       text-[12px] leading-[1.5] break-all text-[var(--color-text-2)]"
              >
                {preview || 'Choose a project folder to preview the launch.'}
              </code>
              <Button
                variant="secondary"
                size="sm"
                class="shrink-0 text-[12px]"
                disabled={!preview}
                onclick={() => void copy('preview', preview)}
              >
                {#if copiedLabel === 'preview'}
                  <Check class="size-3.5" aria-hidden="true" />
                  Copied
                {:else}
                  <Copy class="size-3.5" aria-hidden="true" />
                  Copy
                {/if}
              </Button>
            </div>
          </div>

          {#if projectPath}
            <div class="flex flex-col gap-2 border-t border-[var(--color-border)] pt-4">
              <div class="flex flex-col gap-0.5">
                <span class="text-[12px] leading-[1.3] font-medium text-[var(--color-text-2)]">
                  Prepare another working copy
                </span>
                <p class="text-[11px] leading-[1.4] text-[var(--color-text-3)]">
                  CommandBar does not create it automatically. Copy this command, run it yourself,
                  then refresh the working copy list above.
                </p>
              </div>
              <Input
                bind:value={newWorktreeBranch}
                placeholder="New branch name, such as tsk-12-new-thing"
                aria-label="Branch name for the new working copy"
                class="h-8 text-[13px]"
              />
              <div class="flex items-start gap-2">
                <code
                  class="min-w-0 flex-1 rounded-md bg-[var(--color-elevated)] px-2 py-1.5
                         font-mono text-[12px] leading-[1.5] break-all text-[var(--color-text-2)]"
                >
                  {addWorktreeLine}
                </code>
                <Button
                  variant="secondary"
                  size="sm"
                  class="shrink-0 text-[12px]"
                  onclick={() => void copy('worktree', addWorktreeLine)}
                >
                  {#if copiedLabel === 'worktree'}
                    <Check class="size-3.5" aria-hidden="true" />
                    Copied
                  {:else}
                    <Copy class="size-3.5" aria-hidden="true" />
                    Copy
                  {/if}
                </Button>
              </div>
            </div>
          {/if}
        </div>
      </details>

      {#if problems.length > 0 || startFailure}
        <div class="flex flex-col gap-1">
          {#each problems as problem (problem.field + problem.message)}
            <p class="flex items-start gap-1.5 text-[12px] leading-[1.5] text-destructive">
              <TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
              <span>{problem.message}</span>
            </p>
          {/each}
          {#if startFailure}
            <p class="flex items-start gap-1.5 text-[12px] leading-[1.5] text-destructive">
              <TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
              <span>The session could not be started: {startFailure}</span>
            </p>
          {/if}
        </div>
      {/if}
    </div>

    <Dialog.Footer class="mx-0 mb-0 gap-2 rounded-b-lg border-t bg-transparent px-5 py-3">
      <Button variant="ghost" size="sm" class="text-[13px]" onclick={() => (open = false)}>
        Cancel
      </Button>
      <Button size="sm" class="text-[13px]" disabled={!readyToStart} onclick={() => void start()}>
        {starting ? 'Starting…' : `Start ${selectedLaunch.agent} session`}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
