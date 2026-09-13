<!--
  DraftSessionSurface.svelte — what "+" opens.

  Not a form and not a dialog: an empty session, in the Session tab, with the
  ordinary composer at the bottom and the caret already in it. Everything that
  used to be a field of the new-session form is a control ON that composer —
  provider, project and branch on the left of the footer, and model, effort and
  approval through the same `ComposerConfigMenu` a running session uses.

  Nothing is created here. The surface holds one `ThreadStartPickerState` and
  hands it to `onSend` on the first message; the route owns every side effect
  from there. Abandoning it — switching session, or the close button — discards
  the state and leaves nothing behind.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { hydrateOwned, rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Search from '@lucide/svelte/icons/search';
  import Monitor from '@lucide/svelte/icons/monitor';
  import Server from '@lucide/svelte/icons/server';
  import Plus from '@lucide/svelte/icons/plus';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import RemoteDirectoryField from './RemoteDirectoryField.svelte';
  import ConversationComposer from '$lib/shell/components/conversation/ConversationComposer.svelte';
  import type {
    AgentConversationConfigField,
    AgentConversationConfigState
  } from '$lib/shell/conversation/conversationConfig.ts';
  import {
    addCustomRoot,
    hydrate,
    initialRootPath,
    knownRoots,
    setSessionRoots
  } from '$lib/shell/newSession/projectRootsStore.svelte';
  import {
    accessChoicesFor,
    buildThreadStartRequest,
    canSelectThreadStartGitRef,
    defaultThreadStartState,
    displayProvider,
    effortChoicesFor,
    filterThreadStartGitRefs,
    groupProviderModels,
    validateThreadStart,
    type ThreadStartPickerState,
    type ThreadStartProvider,
    type ThreadStartProviderConfig,
    type ThreadStartRequest
  } from '$lib/shell/newSession/threadStartFlow.ts';
  import {
    initProjectRepository,
    listGitRefs,
    pickProjectFolder,
    type BackendAnswer,
    type ProjectGitRef
  } from '$lib/shell/newSession/newSessionBackend.ts';
  import { rememberAgentConfigChoice } from '$lib/shell/conversation/agentConfigMemory';
  import {
    connectRemoteAssemblyFromTauri,
    readRemoteAssemblyEnvironmentFromTauri,
    removeRemoteAssemblyProfileFromTauri,
    type RemoteAssemblyEnvironment,
    type RemoteAssemblyProfile,
    type ExecutionEnvironment
  } from '$lib/tauriSource.ts';

  interface Props {
    /** Folders the sessions on the rail are running in, so the project picker
     * knows about projects nobody added by hand. */
    sessionRoots: string[];
    presetProjectPath: string | null;
    providerConfigs: ThreadStartProviderConfig[];
    stopSignal: AbortSignal;
    onSend: (request: ThreadStartRequest) => void | Promise<void>;
    onClose: () => void;
  }

  let { sessionRoots, presetProjectPath, providerConfigs, stopSignal, onSend, onClose }: Props = $props();

  const PROVIDERS: readonly ThreadStartProvider[] = ['codex', 'claude', 'antigravity'];
  /** This build has no create-worktree command, so only existing checkouts. */
  const canCreateWorktree = false;

  function preferredRoot(preset: string | null): string {
    const requested = preset?.trim();
    if (requested?.startsWith('/')) return requested;
    return initialRootPath() ?? knownRoots()[0]?.path ?? '';
  }

  // The preset is applied on mount after SQLite hydration; this is only what
  // the first frame paints.
  let draft = $state<ThreadStartPickerState>(
    defaultThreadStartState({ projectPath: preferredRoot(null) })
  );
  let composer = $state<{ focus(): void } | null>(null);
  let gitRefs = $state<ProjectGitRef[]>([]);
  let refsLoading = $state(false);
  let refsMessage = $state<string | null>(null);
  let refSearch = $state('');
  let submitting = $state(false);
  let submitError = $state('');
  let loadSequence = 0;
  let remoteAssembly = $state<RemoteAssemblyEnvironment>({ profiles: [], readyProfileIds: [] });
  let remoteSetupOpen = $state(false);
  let remoteConnecting = $state(false);
  let remoteStatus = $state('');
  let remoteError = $state('');
  let connectionOwner: AbortController | null = null;
  let remoteProfile = $state<RemoteAssemblyProfile>({
    id: '',
    name: '',
    sshTarget: '',
    sourceRoot: '',
    defaultCwd: ''
  });

  const roots = $derived(knownRoots());
  const modelGroups = $derived(groupProviderModels(providerConfigs));
  const problems = $derived(validateThreadStart(draft));
  const filteredRefs = $derived(filterThreadStartGitRefs(gitRefs, refSearch));
  const selectedRef = $derived(gitRefs.find((ref) => ref.name === draft.branch) ?? null);
  const projectName = $derived(draft.projectPath.split('/').filter(Boolean).at(-1) ?? '');
  const selectedRemoteProfile = $derived(
    remoteAssembly.profiles.find((profile) => profile.id === draft.remoteProfileId) ?? null
  );

  function emptyRemoteProfile(): RemoteAssemblyProfile {
    return {
      id: crypto.randomUUID(),
      name: '',
      sshTarget: '',
      sourceRoot: '',
      defaultCwd: ''
    };
  }

  /**
   * The draft's own answer to the question the composer's settings menu asks a
   * running session. Same shape, same menu — the difference is that a change
   * lands in this state instead of being written to an agent that does not
   * exist yet.
   */
  const configState = $derived<AgentConversationConfigState>({
    model: draft.model || null,
    availableModels:
      modelGroups
        .find((group) => group.provider === draft.provider)
        ?.models.filter((model) => model.available)
        .map((model) => model.id) ?? [],
    reasoningEffort: draft.effort || null,
    availableEfforts: effortChoicesFor(draft.provider, providerConfigs),
    approvalPolicy: draft.access || null,
    availableApprovalPolicies: accessChoicesFor(draft.provider, providerConfigs)
  });

  function updateDraft(patch: Partial<ThreadStartPickerState>): void {
    draft = { ...draft, ...patch };
    submitError = '';
  }

  async function loadRefs(projectPath: string): Promise<void> {
    const sequence = ++loadSequence;
    if (stopSignal.aborted) return;
    refsLoading = true;
    refsMessage = null;
    let answer: BackendAnswer<ProjectGitRef[]>;
    try {
      if (stopSignal.aborted) return;
      const repository = await initProjectRepository(projectPath);
      if (stopSignal.aborted) return;
      if (repository.status === 'failed') {
        if (sequence === loadSequence) {
          refsLoading = false;
          gitRefs = [];
          refsMessage = repository.message;
        }
        return;
      }
      if (stopSignal.aborted) return;
      answer = await listGitRefs(projectPath);
      if (stopSignal.aborted) return;
    } catch (error) {
      if (!stopSignal.aborted && sequence === loadSequence) {
        refsLoading = false;
        gitRefs = [];
        refsMessage = describeError(error);
      }
      return;
    }
    if (sequence !== loadSequence) return;
    refsLoading = false;
    if (answer.status === 'failed') {
      gitRefs = [];
      refsMessage = answer.message;
    } else {
      gitRefs = answer.status === 'ok' ? answer.value : [];
      if (answer.status === 'unavailable') refsMessage = answer.message;
    }
    const first = gitRefs.find((ref) => ref.isCurrent)
      ?? gitRefs.find((ref) => ref.checkoutPath)
      ?? null;
    updateDraft({
      cwd: first?.checkoutPath ?? projectPath,
      branch: first?.name ?? '',
      branchesAvailable: gitRefs.length > 0
    });
  }

  function selectProject(path: string): void {
    updateDraft({ projectPath: path, cwd: path, branch: '', branchesAvailable: false });
    gitRefs = [];
    refSearch = '';
    void loadRefs(path);
  }

  function selectEnvironment(
    environment: ExecutionEnvironment,
    profile: RemoteAssemblyProfile | null = null
  ): void {
    if (environment === draft.executionEnvironment && profile?.id === draft.remoteProfileId) return;
    if (environment === 'remote') {
      const cwd = profile?.defaultCwd.trim();
      if (!profile || !cwd) {
        remoteProfile = emptyRemoteProfile();
        remoteSetupOpen = true;
        return;
      }
      loadSequence += 1;
      gitRefs = [];
      refsLoading = false;
      refsMessage = null;
      updateDraft({
        executionEnvironment: environment,
        remoteProfileId: profile.id,
        projectPath: cwd,
        cwd,
        branch: '',
        branchesAvailable: false
      });
      return;
    }
    const projectPath = preferredRoot(presetProjectPath);
    updateDraft({
      executionEnvironment: environment,
      remoteProfileId: null,
      projectPath,
      cwd: projectPath,
      branch: '',
      branchesAvailable: false
    });
    if (projectPath) void loadRefs(projectPath);
  }

  function describeError(error: unknown): string {
    if (error instanceof Error) return error.message;
    if (error && typeof error === 'object' && 'message' in error) {
      const message = (error as { message?: unknown }).message;
      if (typeof message === 'string') return message;
    }
    return String(error);
  }

  async function connectRemote(profile = remoteProfile): Promise<void> {
    if (remoteConnecting || stopSignal.aborted) return;
    if (!profile.name.trim() || !profile.sshTarget.trim() || !profile.defaultCwd.trim()) {
      remoteError = 'Machine name, SSH destination, and remote working directory are required.';
      return;
    }
    const owner = new AbortController();
    connectionOwner = owner;
    remoteConnecting = true;
    remoteError = '';
    remoteStatus = 'Connecting…';
    try {
      const result = await connectRemoteAssemblyFromTauri(profile, owner.signal, (status) => {
        if (!owner.signal.aborted) remoteStatus = status;
      });
      if (owner.signal.aborted || stopSignal.aborted) return;
      remoteProfile = result.profile;
      const ids = new Set(result.sessions.map((session) => session.ownedId));
      hydrateOwned([...rail.owned.filter((session) => !ids.has(session.ownedId)), ...result.sessions.map(ownedSessionFromBackend)]);
      remoteAssembly = await readRemoteAssemblyEnvironmentFromTauri();
      if (owner.signal.aborted || stopSignal.aborted) return;
      remoteStatus = 'Connected. The existing backend is ready.';
      if (result.profile.defaultCwd) selectEnvironment('remote', result.profile);
    } catch (error) {
      if (!stopSignal.aborted) {
        remoteStatus = '';
        remoteError = owner.signal.aborted ? 'Connection cancelled.' : describeError(error);
      }
    } finally {
      if (connectionOwner === owner) { connectionOwner = null; remoteConnecting = false; }
    }
  }

  function cancelRemote() {
    if (connectionOwner) { connectionOwner.abort(); remoteStatus = 'Cancelling…'; }
    else { remoteSetupOpen = false; remoteStatus = ''; remoteError = ''; }
  }

  function editRemote(profile: RemoteAssemblyProfile): void {
    remoteProfile = { ...profile };
    remoteSetupOpen = true;
  }

  async function removeRemote(profile: RemoteAssemblyProfile): Promise<void> {
    if (remoteConnecting || stopSignal.aborted) return;
    remoteConnecting = true;
    submitError = '';
    try {
      remoteAssembly = await removeRemoteAssemblyProfileFromTauri(profile.id);
      if (stopSignal.aborted) return;
      if (draft.remoteProfileId === profile.id) selectEnvironment('local');
      if (remoteProfile.id === profile.id) remoteProfile = emptyRemoteProfile();
    } catch (error) {
      if (!stopSignal.aborted) submitError = describeError(error);
    } finally {
      remoteConnecting = false;
    }
  }

  async function addProject(): Promise<void> {
    if (stopSignal.aborted) return;
    const answer = await pickProjectFolder();
    if (stopSignal.aborted) return;
    if (answer.status !== 'ok') {
      refsMessage = answer.message;
      return;
    }
    if (!answer.value) return;
    addCustomRoot(answer.value);
    selectProject(answer.value);
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
    updateDraft({ provider, model: next.model, effort: next.effort, access: next.access });
  }

  function chooseRef(ref: ProjectGitRef): void {
    if (!canSelectThreadStartGitRef(ref, canCreateWorktree)) return;
    updateDraft({ cwd: ref.checkoutPath ?? draft.projectPath, branch: ref.name });
  }

  function changeConfig(field: AgentConversationConfigField, value: string): void {
    if (field === 'model') updateDraft({ model: value });
    else if (field === 'reasoningEffort') updateDraft({ effort: value });
    else updateDraft({ access: value });
    // A pick here is the choice the next new session opens on.
    rememberAgentConfigChoice(draft.provider, { [field]: value });
  }

  async function send(): Promise<void> {
    if (submitting || stopSignal.aborted) return;
    let request = buildThreadStartRequest(draft);
    if (!request) {
      submitError = problems[0]?.message ?? 'This draft is not ready to send.';
      return;
    }
    submitting = true;
    submitError = '';
    try {
      if (draft.executionEnvironment === 'local' && !draft.branchesAvailable) {
        if (stopSignal.aborted) return;
        const made = await initProjectRepository(draft.projectPath);
        if (stopSignal.aborted) return;
        if (made.status !== 'ok') {
          submitError = made.message;
          return;
        }
        if (stopSignal.aborted) return;
        await loadRefs(draft.projectPath);
        if (stopSignal.aborted) return;
        request = buildThreadStartRequest(draft);
        if (!request) {
          submitError = problems[0]?.message ?? 'This draft is not ready to send.';
          return;
        }
      }
      if (stopSignal.aborted) return;
      await onSend(request);
      if (stopSignal.aborted) return;
    } catch (error) {
      if (!stopSignal.aborted) submitError = describeError(error);
    } finally {
      submitting = false;
    }
  }

  onMount(() => {
    const owner = { active: true };
    const stopConnection = () => connectionOwner?.abort();
    stopSignal.addEventListener('abort', stopConnection, { once: true });
    setSessionRoots(sessionRoots);
    const sequence = ++loadSequence;
    void hydrateDraft(owner, sequence);
    void hydrateRemoteAssembly(owner);
    return () => {
      owner.active = false;
      stopSignal.removeEventListener('abort', stopConnection);
      stopConnection();
      loadSequence += 1;
    };
  });

  async function hydrateDraft(owner: { active: boolean }, sequence: number): Promise<void> {
    if (stopSignal.aborted) return;
    try {
      if (stopSignal.aborted) return;
      await hydrate();
      if (stopSignal.aborted) return;
    } catch (error) {
      if (!stopSignal.aborted && owner.active && sequence === loadSequence) submitError = describeError(error);
      return;
    }
    if (stopSignal.aborted || !owner.active || sequence !== loadSequence) return;
    const projectPath = preferredRoot(presetProjectPath);
    draft = defaultThreadStartState({ projectPath, providerConfigs });
    if (projectPath) void loadRefs(projectPath);
    composer?.focus();
  }

  async function hydrateRemoteAssembly(owner: { active: boolean }): Promise<void> {
    if (stopSignal.aborted) return;
    try {
      if (stopSignal.aborted) return;
      const environment = await readRemoteAssemblyEnvironmentFromTauri();
      if (stopSignal.aborted) return;
      if (!owner.active) return;
      remoteAssembly = environment;
      remoteProfile = emptyRemoteProfile();
    } catch {
      // Remote setup controls stay empty when the desktop backend is unavailable.
    }
  }
</script>

{#snippet draftControls()}
  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button {...props} data-testid="draft-session-environment" variant="ghost" size="xs" class="draft-control">
          {draft.executionEnvironment === 'remote' ? selectedRemoteProfile?.name ?? 'Remote' : 'This Mac'}
          <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content class="draft-machine-menu" side="top" align="start" sideOffset={8} avoidCollisions collisionPadding={12}>
      <DropdownMenu.Label>Work in</DropdownMenu.Label>
      <DropdownMenu.Item onSelect={() => selectEnvironment('local')}>
        <Monitor aria-hidden="true" class="size-4" />
        <span class="draft-machine-name">This Mac</span>
        {#if draft.executionEnvironment === 'local'}<Check aria-hidden="true" class="draft-machine-selected size-4" />{/if}
      </DropdownMenu.Item>
      {#each remoteAssembly.profiles as profile (profile.id)}
        <DropdownMenu.Item title={profile.defaultCwd} onSelect={() => { remoteSetupOpen = true; remoteProfile = { ...profile }; void connectRemote(profile); }}>
          <Server aria-hidden="true" class="size-4" />
          <span class="draft-machine-name">{profile.name}</span>
          {#if draft.remoteProfileId === profile.id}
            <Check aria-hidden="true" class="draft-machine-selected size-4" />
          {/if}
        </DropdownMenu.Item>
      {/each}
      <DropdownMenu.Separator />
      <DropdownMenu.Item onSelect={() => {
        remoteProfile = emptyRemoteProfile();
        remoteSetupOpen = true;
      }}>
        <Plus aria-hidden="true" class="size-4" />
        <span class="draft-machine-name">Add remote machine…</span>
      </DropdownMenu.Item>
      {#if remoteAssembly.profiles.length > 0}
        <DropdownMenu.Item onSelect={() => (remoteSetupOpen = true)}>
          <Settings2 aria-hidden="true" class="size-4" />
          <span class="draft-machine-name">Manage remote machines…</span>
        </DropdownMenu.Item>
      {/if}
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button {...props} data-testid="draft-session-provider" variant="ghost" size="xs" class="draft-control">
          {displayProvider(draft.provider)}
          <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content side="top" align="start" sideOffset={8} avoidCollisions collisionPadding={12}>
      <DropdownMenu.Label>Which agent runs this</DropdownMenu.Label>
      {#each PROVIDERS as provider (provider)}
        <DropdownMenu.Item
          data-testid={`draft-session-provider-${provider}`}
          onSelect={() => selectProvider(provider)}
        >
          <span class="draft-check">
            {#if draft.provider === provider}<Check aria-hidden="true" class="size-3.5" />{/if}
          </span>
          {provider === 'antigravity' ? 'Antigravity' : provider}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button {...props} disabled={draft.executionEnvironment === 'remote'} data-testid="draft-session-project" variant="ghost" size="xs" class="draft-control">
          {projectName || 'Choose a project'}
          <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content
      side="top"
      align="start"
      sideOffset={8}
      avoidCollisions
      collisionPadding={12}
      class="w-[300px]"
    >
      <DropdownMenu.Label>Project workspace</DropdownMenu.Label>
      {#each roots as root (root.path)}
        <DropdownMenu.Item
          data-testid={`draft-session-project-${root.id}`}
          class="items-start gap-2"
          onSelect={() => selectProject(root.path)}
        >
          <span class="draft-check">
            {#if draft.projectPath === root.path}<Check aria-hidden="true" class="size-3.5" />{/if}
          </span>
          <span class="flex min-w-0 flex-col gap-0.5">
            <span>{root.name}</span>
            <span class="draft-hint truncate">{root.path}</span>
          </span>
        </DropdownMenu.Item>
      {/each}
      {#if roots.length === 0}
        <DropdownMenu.Item disabled>No project workspaces yet</DropdownMenu.Item>
      {/if}
      <DropdownMenu.Separator />
      <DropdownMenu.Item data-testid="draft-session-new-project" onSelect={() => void addProject()}>
        <FolderPlus aria-hidden="true" class="size-3.5" />
        <span>New project</span>
      </DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <DropdownMenu.Root onOpenChange={(open) => { if (!open) refSearch = ''; }}>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button {...props} disabled={draft.executionEnvironment === 'remote'} data-testid="draft-session-branch" variant="ghost" size="xs" class="draft-control draft-branch">
          <GitBranch aria-hidden="true" class="size-3.5" />
          <span class="truncate">{(selectedRef?.name ?? draft.branch) || 'Choose a branch'}</span>
          <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content
      side="top"
      align="start"
      sideOffset={8}
      avoidCollisions
      collisionPadding={12}
      class="max-h-[430px] w-[min(440px,calc(100vw-32px))] overflow-y-auto"
    >
      <div class="draft-ref-search">
        <Search aria-hidden="true" class="size-3.5" />
        <Input
          data-testid="draft-session-ref-search"
          aria-label="Search branches"
          placeholder="Search branches"
          bind:value={refSearch}
          onclick={(event) => event.stopPropagation()}
          onkeydown={(event) => event.stopPropagation()}
          class="h-[30px] border-0 bg-transparent shadow-none"
        />
      </div>
      <DropdownMenu.Separator />
      {#if refsLoading}
        <DropdownMenu.Item disabled>Reading branches…</DropdownMenu.Item>
      {:else}
        {#each filteredRefs.visible as ref (ref.name)}
          <DropdownMenu.Item
            data-testid={`draft-session-ref-${ref.name}`}
            disabled={!canSelectThreadStartGitRef(ref, canCreateWorktree)}
            title={canSelectThreadStartGitRef(ref, canCreateWorktree)
              ? ref.checkoutPath ?? undefined
              : 'needs a worktree'}
            onSelect={() => chooseRef(ref)}
          >
            <span class="draft-check">
              {#if draft.branch === ref.name}<Check aria-hidden="true" class="size-3.5" />{/if}
            </span>
            <span class="draft-ref-name">{ref.name}</span>
            <span class="draft-ref-tags">
              {#if ref.isCurrent}<span>current</span>{/if}
              {#if ref.isDefault}<span>default</span>{/if}
              {#if !canSelectThreadStartGitRef(ref, canCreateWorktree)}<small>needs a worktree</small>{/if}
            </span>
          </DropdownMenu.Item>
        {/each}
        {#if filteredRefs.total === 0}<DropdownMenu.Item disabled>No matching branches</DropdownMenu.Item>{/if}
      {/if}
      {#if filteredRefs.total > filteredRefs.visible.length}
        <div class="draft-ref-footer">
          Showing {filteredRefs.visible.length} of {filteredRefs.total} branches
        </div>
      {/if}
    </DropdownMenu.Content>
  </DropdownMenu.Root>
{/snippet}

<section class="draft-surface" data-testid="draft-session-surface" aria-label="New session">
  <div class="draft-topline">
    <span data-testid="draft-session-note">
      New project folders get a local Git repository when selected. No session is created until you send.
    </span>
    <Button
      data-testid="draft-session-close"
      variant="ghost"
      size="icon-sm"
      aria-label="Discard this draft"
      onclick={onClose}
    >
      <X aria-hidden="true" class="size-4" />
    </Button>
  </div>

  <div class="draft-transcript" data-testid="draft-session-transcript">
    {#if remoteSetupOpen}
      <div class="remote-setup" data-testid="draft-session-remote-setup">
        <div class="remote-setup-heading">
          <strong>Remote machines</strong>
          <span>Connect to an Assembly backend already installed on this machine.</span>
        </div>
        {#if remoteAssembly.profiles.length > 0}
          <div class="remote-profile-list">
            {#each remoteAssembly.profiles as profile (profile.id)}
              <div class="remote-profile-row">
                <span><strong>{profile.name}</strong><small>{profile.sshTarget}{remoteAssembly.readyProfileIds.includes(profile.id) ? ' · Ready' : ' · Not connected'}</small></span>
                <Button variant="ghost" size="xs" disabled={remoteConnecting} onclick={() => editRemote(profile)}>Edit</Button>
                <Button variant="ghost" size="xs" disabled={remoteConnecting} onclick={() => void removeRemote(profile)}>Remove</Button>
              </div>
            {/each}
          </div>
        {/if}
        <label>
          <span>Machine name</span>
          <Input disabled={remoteConnecting} bind:value={remoteProfile.name} placeholder="Agent Workbox" autocomplete="off" />
        </label>
        <label>
          <span>SSH destination</span>
          <Input disabled={remoteConnecting} bind:value={remoteProfile.sshTarget} placeholder="user@hostname" autocomplete="off" />
        </label>
        <RemoteDirectoryField label="Remote working directory" sshTarget={remoteProfile.sshTarget}
          bind:value={remoteProfile.defaultCwd} placeholder="/home/user/project" disabled={remoteConnecting} />
        {#if remoteStatus}<p class="remote-connection-status" role="status" aria-live="polite">{remoteStatus}</p>{/if}
        {#if remoteError}<p class="remote-connection-status" role="alert">{remoteError}</p>{/if}
        <div class="remote-setup-actions">
          <Button variant="ghost" size="sm" onclick={cancelRemote}>{remoteConnecting ? 'Cancel connection' : 'Done'}</Button>
          <Button size="sm" disabled={remoteConnecting || !remoteProfile.name.trim() || !remoteProfile.sshTarget.trim() || !remoteProfile.defaultCwd.trim()}
            onclick={() => void connectRemote()}>{remoteConnecting ? 'Connecting…' : 'Connect'}</Button>
        </div>
      </div>
    {:else}
      <p>Start the conversation below.</p>
    {/if}
    {#if refsMessage}
      <p class="draft-warning" data-testid="draft-session-refs-note">
        <span>
          {refsMessage}
        </span>
        <button
          class="draft-warning-dismiss"
          type="button"
          aria-label="Dismiss"
          onclick={() => (refsMessage = null)}
        >
          <X aria-hidden="true" class="size-3" />
        </button>
      </p>
    {/if}
  </div>

  <ConversationComposer
    bind:this={composer}
    provider={draft.provider}
    draft={draft.prompt}
    attachments={[]}
    sending={submitting}
    {configState}
    pendingConfig={{}}
    commands={[]}
    sendError={submitError}
    leadingControls={draftControls}
    onDraftChange={(value) => updateDraft({ prompt: value })}
    onSend={send}
    onConfigChange={changeConfig}
  />
</section>

<style>
  .draft-surface {
    position: absolute;
    inset: 0;
    z-index: 3;
    display: flex;
    flex-direction: column;
    min-height: 0;
    /* The card's own surface colour, not the backdrop, so the draft reads as
       part of the panel. It stays opaque: this layer sits over a conversation
       that keeps its size and its terminals while the draft is open, and it
       has to hide them. */
    background: var(--color-surface);
    color: var(--color-text);
    font: 13px ui-sans-serif, system-ui;
  }

  .draft-topline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px 6px 14px;
    color: var(--color-text-3);
    font-size: 12px;
  }

  .draft-transcript {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 24px;
    color: var(--color-text-3);
    text-align: center;
  }

  .draft-transcript p { margin: 0; font-size: 13px; }
  .remote-setup {
    display: flex;
    width: min(520px, 100%);
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    border: 1px solid var(--color-border);
    border-radius: 12px;
    background: var(--color-surface-raised);
    color: var(--color-text);
    text-align: left;
  }

  .remote-setup-heading,
  .remote-setup label {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .remote-setup-heading span,
  .remote-setup label > span { color: var(--color-text-3); font-size: 12px; }
  .remote-profile-list { display: flex; flex-direction: column; gap: 4px; }
  .remote-profile-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 0;
    border-bottom: 1px solid var(--color-border);
  }
  .remote-profile-row > span { display: flex; min-width: 0; flex: 1; flex-direction: column; }
  .remote-profile-row small { overflow: hidden; color: var(--color-text-3); text-overflow: ellipsis; }
  .remote-connection-status { color: var(--color-text-2); font-size: 12px; }
  .remote-setup-actions { display: flex; justify-content: flex-end; gap: 6px; }
  .draft-warning {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--color-attention);
  }

  /* Every notice the reader is shown has to be closable, or it sits on the
     surface for the rest of the draft with nothing to do about it. */
  .draft-warning-dismiss {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-left: auto;
    padding: 0;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
  }

  .draft-warning-dismiss:hover {
    opacity: 1;
    background: color-mix(in srgb, var(--color-hover) 70%, transparent);
  }

  .draft-warning-dismiss:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: 1px;
  }

  :global(.draft-control) {
    max-width: 200px;
    gap: 4px;
    padding: 0 6px;
    color: var(--color-text-2);
    font-size: 13px;
    font-weight: 400;
  }

  :global(.draft-control:hover) { color: var(--color-text); }
  :global(.draft-branch) { max-width: 220px; min-width: 0; }

  :global(.draft-machine-menu) {
    width: 280px;
    max-width: calc(100vw - 24px);
    padding: 6px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--color-text) 9%, var(--color-surface));
    box-shadow: 0 8px 24px rgb(0 0 0 / 25%);
    animation: none;
    --menu-row-radius: 3px;
    --menu-row-inset: 10px 12px;
    --menu-row-gap: 12px;
  }

  :global(.draft-machine-menu [data-slot="dropdown-menu-label"]) {
    padding: 8px 12px;
    color: var(--color-text-3);
    font-size: 12px;
  }

  :global(.draft-machine-menu [data-slot="dropdown-menu-separator"]) {
    margin: 5px 6px;
  }

  :global(.draft-machine-menu [data-slot="dropdown-menu-item"] > svg) {
    color: var(--color-text-2);
  }

  :global(.draft-machine-menu [data-slot="dropdown-menu-item"] > .draft-machine-selected) {
    color: var(--color-accent);
  }

  .draft-machine-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .draft-check {
    display: inline-flex;
    width: 14px;
    min-width: 14px;
    justify-content: center;
    color: var(--color-accent);
  }

  .draft-hint { color: var(--secondary-label); font-size: 13px; }

  .draft-ref-search {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 5px;
    color: var(--secondary-label);
  }

  .draft-ref-name {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    color: var(--color-text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .draft-ref-tags { display: inline-flex; flex: 0 0 auto; align-items: center; gap: 4px; }

  .draft-ref-tags span {
    padding: 1px 5px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    color: var(--secondary-label);
    font-size: 12px;
    line-height: 1.4;
  }

  .draft-ref-tags small { color: var(--secondary-label); font-size: 12px; }

  .draft-ref-footer {
    padding: 7px 8px 4px;
    border-top: 1px solid var(--color-border);
    color: var(--secondary-label);
    font-size: 12px;
  }
</style>
