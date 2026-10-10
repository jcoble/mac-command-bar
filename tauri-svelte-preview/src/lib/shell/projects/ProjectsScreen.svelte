<!--
  ProjectsScreen.svelte — every project on the left, every machine on the right.

  Opened from the home button, over the whole frame; the shell under it stays
  as it was, so Back returns to it. Choosing a machine narrows the list to that
  machine and makes it the one New project and Add folder start on. Clicking a
  project opens a new session there. In a narrow window the two panes become
  tabs.
-->
<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Cloud from '@lucide/svelte/icons/cloud';
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import Laptop from '@lucide/svelte/icons/laptop';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import { onMount } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { ListRow } from '$lib/components/ui/list-row/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import RemoteConnections from '$lib/shell/components/RemoteConnections.svelte';
  import AddProjectDialog from '$lib/shell/projects/AddProjectDialog.svelte';
  import { hydrateProjects, projectRegistry } from '$lib/shell/projects/projectRegistry.svelte.ts';
  import {
    filterProjects,
    projectCountsByMachine,
    projectMachineLabel,
    shortProjectPath,
    visibleProjects
  } from '$lib/shell/projects/projects.ts';
  import { readRemoteAssemblyEnvironmentFromTauri, type RemoteAssemblyEnvironment } from '$lib/tauriSource.ts';

  interface Props {
    onBack: () => void;
    onOpenProject: (projectId: string) => void;
  }

  let { onBack, onOpenProject }: Props = $props();

  let environment = $state<RemoteAssemblyEnvironment>({ profiles: [], readyProfileIds: [] });
  let machine = $state<string | null>(null);
  let query = $state('');
  let addProjectOpen = $state(false);
  let machinesOpen = $state(false);
  let width = $state(0);
  let machinesError = $state('');
  let searchField = $state<HTMLInputElement | null>(null);

  const projects = $derived(visibleProjects(projectRegistry.projects, environment.profiles.map((profile) => profile.id)));
  const rows = $derived(filterProjects(projects, machine, query));
  const counts = $derived(projectCountsByMachine(projects));
  const machines = $derived([
    { id: 'local', name: 'This Mac', connected: true },
    ...environment.profiles.map((profile) => ({
      id: profile.id,
      name: profile.name,
      connected: environment.readyProfileIds.includes(profile.id)
    }))
  ]);
  /** The window can't be narrower than 900px, so this catches the narrowest ones. */
  const narrow = $derived(width > 0 && width < 960);

  onMount(() => {
    const stop = new AbortController();
    searchField?.focus();
    void hydrateProjects();
    void readMachines(stop.signal);
    return () => stop.abort();
  });

  async function readMachines(signal: AbortSignal): Promise<void> {
    try {
      const next = await readRemoteAssemblyEnvironmentFromTauri();
      if (!signal.aborted) environment = next;
    } catch (error) {
      if (!signal.aborted) machinesError = error instanceof Error ? error.message : String(error);
    }
  }
</script>

{#snippet projectsPane()}
  <section class="pane" aria-label="Projects">
    <div class="flex items-center gap-2">
      <h2 class="pane-title">Projects</h2>
      <Input class="ml-auto h-8 max-w-64" placeholder="Search projects…" aria-label="Search projects" bind:value={query} bind:ref={searchField} />
    </div>
    <div class="flex items-center gap-2">
      {#if machine}
        <Chip data-testid="projects-machine-filter">
          Showing: {projectMachineLabel(machine, environment.profiles)}
          <button type="button" class="chip-clear" aria-label="Show every machine" onclick={() => (machine = null)}><X /></button>
        </Chip>
      {/if}
      <Button class="ml-auto" size="sm" onclick={() => (addProjectOpen = true)}><Plus />New project</Button>
      <Button variant="ghost" size="sm" onclick={() => (addProjectOpen = true)}><FolderPlus />Add folder</Button>
    </div>
    {#if projectRegistry.error || machinesError}
      <p class="text-[13px] text-destructive">{projectRegistry.error || machinesError}</p>
    {/if}
    <div class="pane-list">
      {#if projects.length === 0}
        <EmptyState title="No projects yet." body="A project is a folder Assembly works in.">
          {#snippet actions()}
            <Button size="sm" onclick={() => (addProjectOpen = true)}>New project</Button>
            <Button variant="ghost" size="sm" onclick={() => (addProjectOpen = true)}>Add existing folder</Button>
          {/snippet}
        </EmptyState>
      {:else if rows.length === 0}
        <p class="px-2 py-4 text-[13px] text-muted-foreground">No projects match.</p>
      {:else}
        {#each rows as project (project.id)}
          <ListRow data-testid="projects-row" onclick={() => onOpenProject(project.id)}>
            <span class="truncate font-medium">{project.title}</span>
            <Chip>{projectMachineLabel(project.machine, environment.profiles)}</Chip>
            <span class="ml-auto truncate text-sm text-muted-foreground" title={project.rootPath}>{shortProjectPath(project.rootPath)}</span>
          </ListRow>
        {/each}
      {/if}
    </div>
  </section>
{/snippet}

{#snippet machinesPane()}
  <section class="pane" aria-label="Machines">
    <h2 class="pane-title">Machines</h2>
    <div class="pane-list">
      {#each machines as entry (entry.id)}
        <ListRow
          data-testid="projects-machine"
          selected={machine === entry.id}
          onclick={() => (machine = machine === entry.id ? null : entry.id)}
        >
          <span class="dot" class:connected={entry.connected} aria-label={entry.connected ? 'Connected' : 'Disconnected'}></span>
          {#if entry.id === 'local'}<Laptop class="size-4 shrink-0" />{:else}<Cloud class="size-4 shrink-0" />{/if}
          <span class="truncate">{entry.name}</span>
          <Chip tone="count" class="ml-auto">{counts[entry.id] ?? 0}</Chip>
        </ListRow>
      {/each}
    </div>
    <Button variant="ghost" size="sm" class="self-start" onclick={() => (machinesOpen = true)}><Plus />Add machine</Button>
  </section>
{/snippet}

<!-- Escape anywhere inside closes the screen, like Back. Dialogs portal out
     of this element, so their Escape never reaches it. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="projects-screen"
  role="region"
  aria-label="Projects"
  bind:clientWidth={width}
  data-testid="projects-screen"
  onkeydown={(event) => {
    if (event.key === 'Escape') onBack();
  }}
>
  <header class="flex items-center gap-2">
    <Button variant="ghost" size="sm" onclick={onBack}><ArrowLeft />Back</Button>
  </header>
  {#if narrow}
    <Tabs.Root value="projects" class="min-h-0 flex-1">
      <Tabs.List>
        <Tabs.Trigger value="projects">Projects</Tabs.Trigger>
        <Tabs.Trigger value="machines">Machines</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="projects" class="min-h-0 data-[state=active]:flex">{@render projectsPane()}</Tabs.Content>
      <Tabs.Content value="machines" class="min-h-0 data-[state=active]:flex">{@render machinesPane()}</Tabs.Content>
    </Tabs.Root>
  {:else}
    <div class="panes">
      {@render projectsPane()}
      {@render machinesPane()}
    </div>
  {/if}
</div>

{#if addProjectOpen}
  <AddProjectDialog
    profiles={environment.profiles}
    {machine}
    onAdded={() => (addProjectOpen = false)}
    onAddRemote={() => (machinesOpen = true)}
    onManageRemotes={() => (machinesOpen = true)}
    onClose={() => (addProjectOpen = false)}
  />
{/if}

<Dialog.Root bind:open={machinesOpen}>
  <Dialog.Content class="max-h-[85vh] overflow-y-auto p-0 sm:max-w-[640px]">
    <Dialog.Title class="sr-only">Machines</Dialog.Title>
    {#if machinesOpen}
      <RemoteConnections onChange={(next) => (environment = next)} onClose={() => (machinesOpen = false)} />
    {/if}
  </Dialog.Content>
</Dialog.Root>

<style>
  .projects-screen {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5) var(--space-5);
    background: var(--color-bg);
  }

  .panes {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 320px);
    gap: var(--space-4);
    min-height: 0;
  }

  .pane {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    padding: var(--space-4);
    border-radius: 16px;
    background: var(--color-surface);
  }

  .pane-title {
    margin: 0;
    font-size: var(--text-heading);
    font-weight: var(--text-heading-weight);
  }

  .pane-list {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    overflow-y: auto;
  }

  .chip-clear {
    display: grid;
    place-items: center;
    margin-right: -4px;
    border-radius: 50%;
  }

  .chip-clear :global(svg) {
    width: 12px;
    height: 12px;
  }

  .chip-clear:hover {
    color: var(--color-text);
  }

  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  .dot.connected {
    background: var(--color-accent);
  }
</style>
