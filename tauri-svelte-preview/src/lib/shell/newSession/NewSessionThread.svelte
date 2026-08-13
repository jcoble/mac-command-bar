<script lang="ts">
  import { onMount } from 'svelte';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Cpu from '@lucide/svelte/icons/cpu';
  import Gauge from '@lucide/svelte/icons/gauge';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Plus from '@lucide/svelte/icons/plus';
  import Send from '@lucide/svelte/icons/send';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import {
    builtInRoots,
    hydrate,
    initialRootPath,
    knownRoots,
    setSessionRoots
  } from '$lib/shell/newSession/projectRootsStore.svelte';
  import {
    accessChoicesFor,
    buildThreadStartRequest,
    defaultThreadStartState,
    displayAccess,
    displayEffort,
    displayProvider,
    effortChoicesFor,
    groupProviderModels,
    type ThreadStartModelGroup,
    type ThreadStartPickerState,
    type ThreadStartProvider,
    type ThreadStartProviderConfig,
    type ThreadStartRequest,
    validateThreadStart
  } from '$lib/shell/newSession/threadStartFlow.ts';
  import {
    listWorktrees,
    type BackendAnswer
  } from '$lib/shell/newSession/newSessionBackend.ts';
  import type { ProjectWorktree } from '$lib/tauriSource.ts';
  import {
    worktreeChoicesFor,
    type WorktreeChoice
  } from '$lib/shell/newSession/newSessionFlow.ts';
  import {
    composerFlip,
    type ComposerFlipReceipt
  } from '$lib/shell/newSession/composerFlip.ts';

  interface Props {
    sessionRoots: string[];
    providerConfigs: ThreadStartProviderConfig[];
    onSend: (request: ThreadStartRequest) => void | Promise<void>;
    onClose: () => void;
  }

  let { sessionRoots, providerConfigs, onSend, onClose }: Props = $props();

  const preferredRoot = (): string => {
    const roots = knownRoots();
    return roots.find((root) => root.id === 'mac-command-bar')?.path
      ?? initialRootPath()
      ?? roots[0]?.path
      ?? builtInRoots[0]?.path
      ?? '';
  };

  let draft = $state<ThreadStartPickerState>(defaultThreadStartState({ projectPath: preferredRoot() }));
  let composerInput = $state<HTMLTextAreaElement | null>(null);
  let branchChoices = $state<WorktreeChoice[]>([]);
  let worktreeLoading = $state(false);
  let worktreeMessage = $state<string | null>(null);
  let submitError = $state<string | null>(null);
  let submitting = $state(false);
  let docked = $state(false);
  let resolveDock: (() => void) | null = null;
  let dockPromise: Promise<void> | null = null;
  let loadSequence = 0;

  const modelGroups = $derived<ThreadStartModelGroup[]>(groupProviderModels(providerConfigs));
  const roots = $derived(knownRoots());
  const problems = $derived(validateThreadStart(draft));
  const selectedModel = $derived(
    modelGroups.find((group) => group.provider === draft.provider)?.models.find((model) => model.id === draft.model)
      ?? modelGroups[0]?.models[0]
      ?? null
  );
  const selectedBranch = $derived(
    branchChoices.find((choice) => choice.path === draft.cwd && (choice.branch ?? 'current') === draft.branch)
      ?? branchChoices.find((choice) => choice.path === draft.cwd)
      ?? branchChoices[0]
      ?? null
  );
  const projectName = $derived(draft.projectPath.split('/').filter(Boolean).at(-1) ?? null);
  const sendDisabled = $derived(submitting || problems.length > 0);
  const effortChoices = $derived(effortChoicesFor(draft.provider, providerConfigs));
  const accessChoices = $derived(accessChoicesFor(draft.provider, providerConfigs));
  const flipOptions = $derived({ docked, onComplete: finishDock });

  function finishDock(_receipt: ComposerFlipReceipt): void {
    resolveDock?.();
    resolveDock = null;
  }

  async function dockComposer(): Promise<void> {
    if (docked) {
      await dockPromise;
      return;
    }
    dockPromise = new Promise<void>((resolve) => {
      resolveDock = resolve;
    });
    docked = true;
    await dockPromise;
  }

  function updateDraft(patch: Partial<ThreadStartPickerState>): void {
    draft = { ...draft, ...patch };
    submitError = null;
  }

  function resetDraft(): void {
    hydrate();
    setSessionRoots(sessionRoots);
    const projectPath = preferredRoot();
    draft = defaultThreadStartState({ projectPath, providerConfigs });
    branchChoices = [];
    worktreeMessage = null;
    submitError = null;
    void loadWorktrees(projectPath);
  }

  async function loadWorktrees(projectPath: string): Promise<void> {
    const sequence = ++loadSequence;
    worktreeLoading = true;
    worktreeMessage = null;
    const answer: BackendAnswer<ProjectWorktree[]> = await listWorktrees(projectPath);
    if (sequence !== loadSequence) return;
    worktreeLoading = false;
    if (answer.status === 'failed') {
      branchChoices = worktreeChoicesFor({ projectRoot: projectPath, worktrees: null });
      worktreeMessage = answer.message;
    } else {
      const listed = answer.status === 'ok' ? answer.value : null;
      branchChoices = worktreeChoicesFor({ projectRoot: projectPath, worktrees: listed });
      if (answer.status === 'unavailable') worktreeMessage = answer.message;
    }
    const first = branchChoices[0];
    updateDraft({
      cwd: first?.path ?? projectPath,
      branch: first?.branch ?? 'current'
    });
  }

  function selectProject(path: string): void {
    updateDraft({ projectPath: path, cwd: path, branch: '' });
    branchChoices = [];
    void loadWorktrees(path);
  }

  function selectProvider(provider: ThreadStartProvider): void {
    if (provider === draft.provider) return;
    const next = defaultThreadStartState({
      projectPath: draft.projectPath,
      cwd: draft.cwd,
      branch: draft.branch,
      provider,
      providerConfigs
    });
    updateDraft({
      provider,
      model: next.model,
      effort: next.effort,
      access: next.access
    });
  }

  function chooseModel(provider: ThreadStartProvider, model: string): void {
    selectProvider(provider);
    updateDraft({ provider, model });
  }

  function chooseBranch(choice: WorktreeChoice): void {
    updateDraft({
      cwd: choice.path,
      branch: choice.branch ?? 'current',
      createNewWorktree: false
    });
  }

  async function submit(): Promise<void> {
    if (submitting) return;
    const request = buildThreadStartRequest(draft);
    if (!request) return;
    submitting = true;
    submitError = null;
    try {
      await dockComposer();
      await onSend(request);
    } catch (error) {
      submitError = error instanceof Error ? error.message : String(error);
    } finally {
      submitting = false;
    }
  }

  function onComposerKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void submit();
    }
  }

  onMount(() => {
    resetDraft();
    composerInput?.focus();
  });
</script>

<div class="thread-start-layer" data-testid="new-session-thread-layer">
  <section class="thread-start-pane" data-testid="new-session-thread-pane" aria-label="New session">
    <header class="thread-start-topbar">
      <div class="thread-start-tab">
        <Plus class="size-3.5 text-[var(--color-accent)]" aria-hidden="true" />
        <span>New session</span>
      </div>
      <span class="thread-start-status">unsaved · nothing has been started yet</span>
      <Button
        data-testid="new-session-thread-close"
        variant="ghost"
        size="icon-sm"
        class="ml-auto text-[var(--secondary-label)] hover:text-foreground"
        aria-label="Close new session"
        onclick={onClose}
      >
        <X class="size-4" aria-hidden="true" />
      </Button>
    </header>

    <div
      class:docked
      class="thread-start-body"
      data-testid="new-session-thread-body"
      data-composer-state={docked ? 'docked' : 'hero'}
    >
      <div class="thread-start-stage">
        <div class="thread-start-hero-copy" aria-hidden={docked}>
          <h1 data-testid="new-session-thread-heading">
            {#if projectName}What should we build in{:else}{/if}
            <span class:leading={!projectName} class="thread-start-project-inline">
              <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                  {#snippet child({ props })}
                    <Button
                      {...props}
                      data-testid="new-session-thread-project"
                      variant="ghost"
                      class="thread-start-project-trigger"
                    >
                      {projectName ?? 'Pick a project'}
                    </Button>
                  {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content class="thread-start-menu thread-start-project-menu" align="center" sideOffset={8}>
                  <DropdownMenu.Label>Project workspace</DropdownMenu.Label>
                  {#each roots as root (root.path)}
                    <DropdownMenu.Item data-testid={`new-session-thread-project-${root.id}`} onSelect={() => selectProject(root.path)}>
                      <span class="thread-start-check">{#if draft.projectPath === root.path}<Check class="size-3.5" aria-hidden="true" />{/if}</span>
                      <span class="flex min-w-0 flex-col gap-0.5">
                        <span>{root.name}</span>
                        <span class="thread-start-menu-hint truncate">{root.path}</span>
                      </span>
                    </DropdownMenu.Item>
                  {/each}
                  {#if roots.length === 0}
                    <DropdownMenu.Item disabled>No project workspaces yet</DropdownMenu.Item>
                  {/if}
                </DropdownMenu.Content>
              </DropdownMenu.Root>
              <span class="thread-start-project-suffix">{projectName ? '?' : ' to start'}</span>
            </span>
          </h1>
          <p class="thread-start-description">
            Describe the work. The session starts when you send, on the branch and settings shown below.
          </p>
        </div>

      <div
        use:composerFlip={flipOptions}
        class="thread-start-composer"
        data-testid="new-session-thread-composer"
        data-position={docked ? 'docked' : 'hero'}
      >
        <textarea
          data-testid="new-session-thread-input"
          aria-label="Describe what to build"
          placeholder="Describe what you want to build…"
          value={draft.prompt}
          oninput={(event) => updateDraft({ prompt: event.currentTarget.value })}
          onkeydown={onComposerKeydown}
          bind:this={composerInput}
        ></textarea>

        <div class="thread-start-pills" data-testid="new-session-thread-pills">
          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <Button
                  {...props}
                  data-testid="new-session-thread-model"
                  variant="ghost"
                  size="sm"
                  class="thread-start-pill"
                >
                  <Cpu class="size-3.5" aria-hidden="true" />
                  <span>{selectedModel?.label ?? 'Choose model'}</span>
                  <ChevronDown class="thread-start-chevron" aria-hidden="true" />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content class="thread-start-menu thread-start-model-menu" align="start" sideOffset={8}>
              {#each modelGroups as group, groupIndex (group.provider)}
                <DropdownMenu.Label>{group.label}</DropdownMenu.Label>
                {#each group.models as model (group.provider + ':' + model.id)}
                  <DropdownMenu.Item
                    data-testid={`new-session-thread-model-${group.provider}-${model.id}`}
                    class="thread-start-menu-item"
                    disabled={!model.available}
                    title={model.unavailableReason ?? undefined}
                    onSelect={() => chooseModel(group.provider, model.id)}
                  >
                    <span class="thread-start-check">
                      {#if draft.provider === group.provider && draft.model === model.id}
                        <Check class="size-3.5" aria-hidden="true" />
                      {/if}
                    </span>
                    <span class="thread-start-model-copy">
                      <span class="thread-start-menu-name">{model.label}</span>
                      <span class="thread-start-menu-provider">{group.label}</span>
                    </span>
                  </DropdownMenu.Item>
                {/each}
                {#if groupIndex < modelGroups.length - 1}<DropdownMenu.Separator />{/if}
              {/each}
            </DropdownMenu.Content>
          </DropdownMenu.Root>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <Button {...props} data-testid="new-session-thread-effort" variant="ghost" size="sm" class="thread-start-pill">
                  <Gauge class="size-3.5" aria-hidden="true" />
                  <span class="thread-start-pill-key">Effort</span>
                  <span>{displayEffort(draft.effort)}</span>
                  <ChevronDown class="thread-start-chevron" aria-hidden="true" />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content class="thread-start-menu" align="start" sideOffset={8}>
              <DropdownMenu.Label>How hard it thinks</DropdownMenu.Label>
              {#each effortChoices as effort (effort)}
                <DropdownMenu.Item data-testid={`new-session-thread-effort-${effort}`} onSelect={() => updateDraft({ effort })}>
                  <span class="thread-start-check">{#if draft.effort === effort}<Check class="size-3.5" aria-hidden="true" />{/if}</span>
                  {displayEffort(effort)}
                </DropdownMenu.Item>
              {/each}
            </DropdownMenu.Content>
          </DropdownMenu.Root>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <Button {...props} data-testid="new-session-thread-access" variant="ghost" size="sm" class="thread-start-pill">
                  <ShieldCheck class="size-3.5" aria-hidden="true" />
                  <span class="thread-start-pill-key">Access</span>
                  <span>{displayAccess(draft.access)}</span>
                  <ChevronDown class="thread-start-chevron" aria-hidden="true" />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content class="thread-start-menu" align="start" sideOffset={8}>
              <DropdownMenu.Label>What it may do</DropdownMenu.Label>
              {#each accessChoices as access (access)}
                <DropdownMenu.Item data-testid={`new-session-thread-access-${access}`} onSelect={() => updateDraft({ access })}>
                  <span class="thread-start-check">{#if draft.access === access}<Check class="size-3.5" aria-hidden="true" />{/if}</span>
                  <span class="flex flex-col gap-0.5">
                    <span>{displayAccess(access)}</span>
                    <span class="thread-start-menu-hint">Provider setting</span>
                  </span>
                </DropdownMenu.Item>
              {/each}
            </DropdownMenu.Content>
          </DropdownMenu.Root>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <Button {...props} data-testid="new-session-thread-branch" variant="ghost" size="sm" class="thread-start-pill">
                  <GitBranch class="size-3.5" aria-hidden="true" />
                  <span>{draft.createNewWorktree ? `new worktree · ${draft.branch || 'branch'}` : (draft.branch || 'Choose branch')}</span>
                  <ChevronDown class="thread-start-chevron" aria-hidden="true" />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content class="thread-start-menu thread-start-branch-menu" align="start" sideOffset={8}>
              <DropdownMenu.Label>Existing checkouts</DropdownMenu.Label>
              {#if worktreeLoading}
                <DropdownMenu.Item disabled>Reading branches…</DropdownMenu.Item>
              {:else}
                {#each branchChoices as choice (choice.path)}
                  <DropdownMenu.Item data-testid={`new-session-thread-branch-${choice.branch ?? 'current'}`} onSelect={() => chooseBranch(choice)}>
                    <span class="thread-start-check">{#if selectedBranch?.path === choice.path && !draft.createNewWorktree}<Check class="size-3.5" aria-hidden="true" />{/if}</span>
                    <span class="flex min-w-0 flex-col gap-0.5">
                      <span>{choice.branch ?? 'Current branch'}</span>
                      <span class="thread-start-menu-hint truncate">{choice.path}</span>
                    </span>
                  </DropdownMenu.Item>
                {/each}
                {#if branchChoices.length === 0}<DropdownMenu.Item disabled>No existing checkout found</DropdownMenu.Item>{/if}
              {/if}
              <DropdownMenu.Separator />
              <DropdownMenu.CheckboxItem
                data-testid="new-session-thread-new-worktree"
                checked={draft.createNewWorktree}
                onCheckedChange={(checked) => updateDraft({ createNewWorktree: checked })}
              >
                <GitBranch class="size-3.5" aria-hidden="true" />
                <span class="flex min-w-0 flex-col gap-0.5">
                  <span>New worktree</span>
                  <span class="thread-start-menu-hint">Unavailable in this build</span>
                </span>
              </DropdownMenu.CheckboxItem>
            </DropdownMenu.Content>
          </DropdownMenu.Root>

          <Button
            data-testid="new-session-thread-send"
            class="thread-start-send"
            disabled={sendDisabled}
            onclick={() => void submit()}
          >
            {submitting ? 'Starting…' : 'Send'}
            <Send class="size-3.5" aria-hidden="true" />
          </Button>
        </div>
      </div>

      {#if worktreeMessage}
        <p class="thread-start-note" data-testid="new-session-thread-worktree-note">{worktreeMessage}</p>
      {/if}
      {#if problems.length || submitError}
        <div class="thread-start-errors" data-testid="new-session-thread-errors" role="alert">
          {#each problems as problem (problem.field + problem.message)}
            <p><TriangleAlert class="size-3.5" aria-hidden="true" />{problem.message}</p>
          {/each}
          {#if submitError}<p><TriangleAlert class="size-3.5" aria-hidden="true" />{submitError}</p>{/if}
        </div>
      {/if}

      <p class="thread-start-footnote">
        Pills stay editable until you send. Nothing is created while this pane is a draft.
        {#if draft.provider}<span> · {displayProvider(draft.provider)}</span>{/if}
      </p>
      </div>
    </div>
  </section>
</div>

<style>
  .thread-start-layer {
    position: absolute;
    /* Keep Bits' fixed-position menu wrapper (z-50) above the pane while a
       picker is open; the pane itself still sits above the shell regions. */
    z-index: 40;
    inset: 0 0 0 300px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
    background: var(--color-bg);
  }

  .thread-start-pane {
    display: flex;
    width: min(840px, 100%);
    height: min(760px, 100%);
    flex-direction: column;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    box-shadow: var(--shadow-lg);
    overflow: visible;
  }

  .thread-start-topbar {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 40px;
    padding: 0 10px 0 12px;
    border-bottom: 1px solid var(--color-border);
    color: var(--secondary-label);
    font-size: 13px;
  }

  .thread-start-tab {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 26px;
    padding: 0 10px;
    border-radius: 7px;
    background: var(--color-surface);
    color: var(--color-text);
    font-weight: 550;
  }

  .thread-start-status { color: var(--secondary-label); }

  .thread-start-body {
    position: relative;
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 72px 36px 32px;
    overflow: hidden;
    text-align: center;
  }

  .thread-start-body.docked { justify-content: flex-end; }

  .thread-start-stage {
    position: relative;
    width: min(48rem, 100%);
  }

  .thread-start-hero-copy {
    position: absolute;
    right: 0;
    bottom: calc(100% + 24px);
    left: 0;
    opacity: 1;
    visibility: visible;
  }

  .docked .thread-start-hero-copy {
    opacity: 0;
    visibility: hidden;
  }

  h1 {
    margin: 0 0 8px;
    color: var(--color-text);
    font-size: 24px;
    font-weight: 400;
    letter-spacing: -0.02em;
    line-height: 1.3;
  }

  .thread-start-project-inline {
    display: inline;
    margin-left: 0.22em;
    font-size: 0;
    vertical-align: baseline;
  }

  .thread-start-project-inline.leading { margin-left: 0; }

  .thread-start-project-suffix { font-size: 24px; }

  :global(.thread-start-project-trigger) {
    display: inline;
    height: auto;
    min-height: 0;
    padding: 0;
    border: 0;
    border-bottom: 1px dotted color-mix(in srgb, var(--color-text) 60%, transparent);
    border-radius: 0;
    color: inherit;
    background: transparent;
    font: inherit;
    font-size: 24px;
    font-weight: 400;
    letter-spacing: inherit;
    line-height: inherit;
    vertical-align: baseline;
  }

  :global(.thread-start-project-trigger:hover) {
    color: var(--color-accent);
    background: transparent;
  }

  :global(.thread-start-project-trigger:focus-visible) {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: 3px;
  }

  .thread-start-description {
    max-width: 500px;
    margin: 0 auto;
    color: var(--secondary-label);
    font-size: 13px;
    line-height: 1.6;
  }

  .thread-start-composer {
    padding: 14px 14px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    text-align: left;
    box-shadow: var(--shadow-sm);
  }

  .thread-start-composer:focus-within {
    box-shadow: 0 0 0 2px var(--color-focus);
  }

  textarea {
    display: block;
    width: 100%;
    min-height: 116px;
    resize: vertical;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--color-text);
    caret-color: var(--color-accent);
    font: 14px/1.55 inherit;
  }

  textarea::placeholder { color: var(--secondary-label); }

  .thread-start-pills {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 7px;
    margin-top: 12px;
    padding-top: 11px;
    border-top: 1px solid var(--color-border);
  }

  :global(.thread-start-pill) {
    height: 28px;
    gap: 6px;
    padding: 0 9px;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    color: var(--color-text-2);
    font-size: 13px;
    font-weight: 550;
  }

  :global(.thread-start-pill:hover) { color: var(--color-text); }
  .thread-start-pill-key { color: var(--secondary-label); font-weight: 500; }
  :global(.thread-start-chevron) { width: 12px; height: 12px; opacity: 0.55; }

  :global(.thread-start-send) {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    margin-left: auto;
    padding: 0 13px;
    border-radius: 8px;
    background: var(--color-accent);
    color: var(--color-on-accent);
    font-size: 13px;
    font-weight: 650;
  }

  :global(.thread-start-send:disabled) { opacity: 0.45; }

  :global(.thread-start-menu) {
    width: 248px;
    z-index: 80;
    border-color: var(--color-border);
    background: var(--color-surface);
    box-shadow: var(--shadow-lg);
  }

  :global(.thread-start-model-menu) { width: 292px; }
  :global(.thread-start-project-menu) { width: 300px; }
  :global(.thread-start-branch-menu) { width: 320px; }

  :global(.thread-start-menu-item) { align-items: flex-start; gap: 9px; padding: 7px 8px; }
  :global(.thread-start-menu-item[data-disabled]) { cursor: not-allowed; }
  .thread-start-check { display: inline-flex; width: 14px; min-width: 14px; justify-content: center; color: var(--color-accent); }
  .thread-start-model-copy { display: flex; min-width: 0; flex: 1 1 auto; flex-direction: column; gap: 1px; }
  .thread-start-menu-name { overflow: hidden; color: var(--color-text); font-size: 13px; font-weight: 550; text-overflow: ellipsis; white-space: nowrap; }
  .thread-start-menu-provider,
  .thread-start-menu-hint { color: var(--secondary-label); font-size: 13px; }

  .thread-start-note,
  .thread-start-footnote {
    margin: 12px auto 0;
    color: var(--secondary-label);
    font-size: 13px;
    line-height: 1.5;
  }

  .thread-start-note { color: var(--color-attention); }

  .thread-start-errors {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 14px;
    color: var(--color-bad);
    font-size: 13px;
    line-height: 1.5;
    text-align: left;
  }

  .thread-start-errors p { display: flex; align-items: flex-start; gap: 6px; margin: 0; }

  @media (max-width: 980px) {
    .thread-start-layer { left: 0; padding: 18px; }
    .thread-start-body { padding: 36px 24px 24px; }
  }

  @media (prefers-reduced-motion: no-preference) {
    .thread-start-hero-copy { transition: opacity 120ms ease; }
  }
</style>
