<!--
  WorkflowsScreen.svelte — every workflow run on one full page.

  Opened from the Agents panel's Workflows tab, over the whole frame like the
  Projects screen; the shell under it stays as it was, so Back returns to it.
  The Board shows each run in the column of the step it is at; a card opens
  that run's page, with its Map and its Cost.
-->
<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { onMount } from 'svelte';

  import * as Breadcrumb from '$lib/components/ui/breadcrumb/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { listenAgentMessagesChanged } from '$lib/tauriSource.ts';
  import { parseRemoteWorkspacePath } from '$lib/workspacePaths.ts';
  import { listWorkflowRuns } from './workflowService.ts';
  import { workflowErrorText, type WorkflowRunRecord } from './workflowTypes.ts';

  interface Props {
    /** The active session's project; a remote project's runs are on that machine's engine. */
    root: string;
    onBack: () => void;
  }

  let { root, onBack }: Props = $props();

  const machine = $derived(parseRemoteWorkspacePath(root)?.profileId ?? null);
  let runs = $state<WorkflowRunRecord[]>([]);
  let loaded = $state(false);
  let error = $state('');
  /** Ends every read and listener when the screen closes. */
  const stop = new AbortController();

  onMount(() => {
    void follow(stop.signal);
    return () => stop.abort();
  });

  /** Reads the runs, then again on every change, one read at a time. */
  async function follow(signal: AbortSignal): Promise<void> {
    let reading = false, again = false;
    const reread = async (): Promise<void> => {
      if (reading) { again = true; return; }
      reading = true;
      do { again = false; await read(signal); } while (again && !signal.aborted);
      reading = false;
    };
    await listenAgentMessagesChanged(() => void reread(), signal);
    await reread();
  }

  async function read(signal: AbortSignal): Promise<void> {
    try {
      const next = await listWorkflowRuns(machine);
      if (signal.aborted) return;
      runs = next ?? [];
      error = next === null ? 'Workflow runs are shown in the desktop app.' : '';
    } catch (cause) {
      if (!signal.aborted) error = `Could not load workflows: ${workflowErrorText(cause)}`;
    } finally {
      if (!signal.aborted) loaded = true;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="workflows-screen"
  role="region"
  aria-label="Workflows"
  data-testid="workflows-screen"
  onkeydown={(event) => { if (event.key === 'Escape') onBack(); }}
>
  <header class="flex min-w-0 items-center gap-3">
    <Button variant="ghost" size="sm" onclick={onBack}><ArrowLeft />Back</Button>
    <Breadcrumb.Root>
      <Breadcrumb.List class="text-[13px]">
        <Breadcrumb.Item><Breadcrumb.Page>Workflows</Breadcrumb.Page></Breadcrumb.Item>
      </Breadcrumb.List>
    </Breadcrumb.Root>
  </header>

  {#if error}
    <p class="flex items-start gap-2 rounded-lg bg-destructive/10 px-3 py-2 text-[13px] text-destructive">
      <TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />{error}
    </p>
  {/if}

  {#if loaded && runs.length === 0 && !error}
    <p class="text-[13px] text-muted-foreground">No workflow runs yet.</p>
  {/if}
</div>

<style>
  .workflows-screen {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    padding: var(--space-3) var(--space-5) var(--space-5);
    background: var(--color-bg);
  }
</style>
