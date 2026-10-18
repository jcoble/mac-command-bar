<!--
  Agent-to-agent messages as a list plus a reader. The drawer shows one session's
  messages (to it, from it, or inside a run it started); the full screen shows every
  machine's, with search, sort and filters. The backend does all filtering and paging,
  and a "messages changed" event reloads the list while it is visible.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import type { ChipTone } from '$lib/components/ui/chip/variants.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump.ts';
  import { adoptSession } from '$lib/shell/conversation/conversationService.ts';
  import { reloadLimit } from '$lib/shell/panels/agents/agentMessagesModel.svelte.ts';
  import {
    listAgentMessagesFromTauri,
    listenAgentMessagesChanged,
    readRemoteAssemblyEnvironmentFromTauri,
    type AgentMessage,
    type AgentMessageFilter,
    type RemoteAssemblyProfile
  } from '$lib/tauriSource.ts';

  type Sort = NonNullable<AgentMessageFilter['sort']>;
  type Status = AgentMessage['receipt'] | 'all';

  interface Props {
    visible: boolean;
    layout: 'drawer' | 'screen';
    /** The drawer's session; the screen ignores it and shows every session. */
    session: string | null;
    /** The screen's run filter, and its name for the chip. */
    runId?: string | null;
    runLabel?: string;
    onClearRun?: () => void;
    /** A message's run was picked in the reader. */
    onShowRun: (runId: string, label: string) => void;
    /** A visible list finished loading, so its messages count as read. */
    onSeen?: () => void;
    /** A sender's or receiver's chat actually opened. */
    onOpened?: () => void;
  }
  let { visible, layout, session, runId = null, runLabel = '', onClearRun, onShowRun, onSeen, onOpened }: Props = $props();

  const LIMIT = 100;
  const RECEIPT_TONE: Record<AgentMessage['receipt'], ChipTone> = {
    queued: 'live',
    delivered: 'good',
    failed: 'bad',
    expired: 'neutral'
  };
  const SORTS: { value: Sort; label: string }[] = [
    { value: 'time', label: 'Newest first' },
    { value: 'sender', label: 'Sender A–Z' },
    { value: 'receiver', label: 'Receiver A–Z' }
  ];
  const STATUSES: { value: Status; label: string }[] = [
    { value: 'all', label: 'Any status' },
    { value: 'queued', label: 'Queued' },
    { value: 'delivered', label: 'Delivered' },
    { value: 'failed', label: 'Failed' },
    { value: 'expired', label: 'Expired' }
  ];
  const time = new Intl.DateTimeFormat(undefined, { dateStyle: 'short', timeStyle: 'short' });

  let messages = $state<AgentMessage[]>([]);
  let selected = $state<AgentMessage | null>(null);
  let hasOlder = $state(false);
  let loading = $state(false);
  let error = $state('');
  let search = $state('');
  let sort = $state<Sort>('time');
  let machine = $state('all');
  let status = $state<Status>('all');
  let profiles = $state<RemoteAssemblyProfile[]>([]);
  let loader: AbortController | null = null;
  let openLookup: AbortController | null = null;
  /** New criteria whose results have not landed yet; a live reload that cancels them still owes the check. */
  let queryPending = false;

  const waiting = $derived(layout === 'drawer' && !session);

  /** Message ids are per machine, so a row is keyed by both. */
  function keyOf(item: AgentMessage): string {
    return `${item.machine}:${item.id}`;
  }

  function criteria(): Omit<AgentMessageFilter, 'limit' | 'before'> {
    return layout === 'drawer'
      ? { session }
      : { runId, machine: machine === 'all' ? null : machine, status: status === 'all' ? null : status, search: search.trim(), sort };
  }

  // Every change of search, sort or filter starts a new first page and cancels the one before.
  $effect(() => {
    if (!visible || waiting) return;
    const filter = criteria();
    untrack(() => {
      if (layout === 'drawer') {
        messages = [];
        selected = null;
      }
      start('query', filter);
    });
    return () => {
      loader?.abort();
      loader = null;
    };
  });

  // While the list shows, a stored or updated message reloads it.
  $effect(() => {
    if (!visible || waiting) return;
    const controller = new AbortController();
    void follow(controller.signal);
    return () => {
      controller.abort();
      openLookup?.abort();
      openLookup = null;
    };
  });

  $effect(() => {
    if (layout !== 'screen') return;
    const controller = new AbortController();
    void readMachines(controller.signal);
    return () => controller.abort();
  });

  async function follow(signal: AbortSignal): Promise<void> {
    try {
      await listenAgentMessagesChanged(() => start('live', criteria()), signal);
    } catch {
      // Without the event the list still loads on every open and filter change.
    }
  }

  async function readMachines(signal: AbortSignal): Promise<void> {
    try {
      const environment = await readRemoteAssemblyEnvironmentFromTauri();
      if (signal.aborted) return;
      profiles = environment.profiles.filter((profile) => environment.readyProfileIds.includes(profile.id));
    } catch {
      // The machine filter keeps All machines and This Mac.
    }
  }

  function start(mode: 'query' | 'live' | 'older', filter: Omit<AgentMessageFilter, 'limit' | 'before'>): void {
    loader?.abort();
    const controller = new AbortController();
    loader = controller;
    if (mode === 'query') queryPending = true;
    void load(mode, filter, controller.signal);
  }

  async function load(mode: 'query' | 'live' | 'older', filter: Omit<AgentMessageFilter, 'limit' | 'before'>, signal: AbortSignal): Promise<void> {
    loading = true;
    error = '';
    const last = mode === 'older' ? messages.at(-1) : undefined;
    // A live reload refetches the whole loaded window from the top, up to the backend's cap.
    const limit = mode === 'live' ? reloadLimit(messages.length) : LIMIT;
    const key = !last || filter.sort === undefined || filter.sort === 'time' ? undefined
      : filter.sort === 'sender' ? last.senderName : last.receiver;
    try {
      const page = (await listAgentMessagesFromTauri({
        ...filter,
        before: last ? { createdAtMs: last.createdAtMs, id: last.id, key } : null,
        limit
      }, signal)) ?? [];
      if (signal.aborted) return;
      messages = mode === 'older' ? [...messages, ...page] : page;
      hasOlder = page.length === limit;
      const newCriteria = mode !== 'older' && queryPending;
      if (mode !== 'older') queryPending = false;
      if (selected) {
        const current = keyOf(selected);
        const fresh = page.find((item) => keyOf(item) === current);
        // New criteria drop a message they no longer match; a live reload keeps it in the reader.
        if (fresh) selected = fresh;
        else if (newCriteria) selected = null;
      }
      onSeen?.();
    } catch (cause) {
      if (!signal.aborted) error = `Could not load messages: ${cause instanceof Error ? cause.message : String(cause)}`;
    }
    if (!signal.aborted) loading = false;
  }

  /** A sender's label without the trailing `[owned id]` it carries for replies. */
  function senderName(label: string): string {
    return label.replace(/ \[[^\]]*\]$/, '');
  }

  function machineName(id: string): string {
    return id === 'mac' ? 'This Mac' : profiles.find((profile) => profile.id === id)?.name ?? id;
  }

  function runName(id: string): string {
    return `Run ${id.replace(/^workflow-run-/, '').slice(0, 8)}`;
  }

  async function open(ownedId: string): Promise<void> {
    openLookup?.abort();
    const lookup = new AbortController();
    openLookup = lookup;
    try {
      const adopted = await adoptSession(ownedId, lookup.signal);
      if (lookup.signal.aborted) return;
      if (!adopted) throw new Error('That session is not available yet.');
      const opened = await sessionRowJump(ownedId, 'session');
      if (!lookup.signal.aborted && opened) onOpened?.();
    } catch (cause) {
      if (!lookup.signal.aborted) error = `Could not open session: ${cause instanceof Error ? cause.message : String(cause)}`;
    }
  }
</script>

{#snippet reader(item: AgentMessage)}
  <div class="flex min-w-0 flex-col gap-2 p-3" data-testid="agent-message-reader">
    <div class="flex min-w-0 flex-wrap items-center gap-2">
      <span class="min-w-0 truncate text-[13px] font-medium" title={item.senderName}>{senderName(item.senderName)}</span>
      <span class="min-w-0 truncate text-sm text-muted-foreground">to {item.receiver}</span>
      <Chip tone={RECEIPT_TONE[item.receipt]}>{item.receipt}</Chip>
      {#if item.runId}
        {@const run = item.runId}
        <Button size="xs" variant="ghost" onclick={() => onShowRun(run, runName(run))}>{runName(run)}</Button>
      {/if}
    </div>
    <p class="text-sm text-muted-foreground">{time.format(item.createdAtMs)} · {machineName(item.machine)}</p>
    <p class="text-[13px] break-words whitespace-pre-wrap">{item.body}</p>
    <div class="flex flex-wrap gap-1">
      <Button size="sm" variant="ghost" onclick={() => void open(item.toOwnedId)}>Open receiver</Button>
      {#if !item.fromOwnedId.startsWith('workflow:')}
        <Button size="sm" variant="ghost" onclick={() => void open(item.fromOwnedId)}>Open sender</Button>
      {/if}
    </div>
  </div>
{/snippet}

<div class="flex h-full min-h-0 flex-col" data-testid="agent-messages">
  {#if layout === 'screen'}
    <div class="flex min-w-0 flex-wrap items-center gap-2 pb-3">
      <Input class="w-72" placeholder="Search sender, receiver or text" aria-label="Search messages" bind:value={search} />
      <Select.Root type="single" value={sort} onValueChange={(value) => (sort = value as Sort)}>
        <Select.Trigger size="sm" aria-label="Sort">{SORTS.find((item) => item.value === sort)?.label}</Select.Trigger>
        <Select.Content>
          <Select.Group>
            {#each SORTS as item (item.value)}<Select.Item value={item.value} label={item.label} />{/each}
          </Select.Group>
        </Select.Content>
      </Select.Root>
      <Select.Root type="single" bind:value={machine}>
        <Select.Trigger size="sm" aria-label="Machine">{machine === 'all' ? 'All machines' : machineName(machine)}</Select.Trigger>
        <Select.Content>
          <Select.Group>
            <Select.Item value="all" label="All machines" />
            <Select.Item value="mac" label="This Mac" />
            {#each profiles as profile (profile.id)}<Select.Item value={profile.id} label={profile.name} />{/each}
          </Select.Group>
        </Select.Content>
      </Select.Root>
      <Select.Root type="single" value={status} onValueChange={(value) => (status = value as Status)}>
        <Select.Trigger size="sm" aria-label="Status">{STATUSES.find((item) => item.value === status)?.label}</Select.Trigger>
        <Select.Content>
          <Select.Group>
            {#each STATUSES as item (item.value)}<Select.Item value={item.value} label={item.label} />{/each}
          </Select.Group>
        </Select.Content>
      </Select.Root>
      {#if runId}
        <span class="flex min-w-0 items-center gap-1">
          <Chip tone="neutral">{runLabel}</Chip>
          <IconButton label="Show messages from every run" size="xs" onclick={() => onClearRun?.()}><X /></IconButton>
        </span>
      {/if}
    </div>
  {/if}
  {#if error}<p role="alert" class="px-3 py-2 text-sm text-destructive">{error}</p>{/if}

  {#if waiting}
    <EmptyState title="Select a session" body="Its messages, and those in runs it started, appear here." />
  {:else}
    <div class={layout === 'screen'
      ? 'grid min-h-0 flex-1 grid-cols-[minmax(0,420px)_minmax(0,1fr)] gap-4'
      : 'flex min-h-0 flex-1 flex-col'}>
      <div class={['flex min-h-0 flex-1 flex-col', layout === 'screen' && 'rounded-xl bg-card']}>
        {#if messages.length === 0}
          <EmptyState title={loading ? 'Loading messages' : 'No messages yet.'} />
        {:else}
          <ScrollArea class="min-h-0 flex-1">
            <ul class="flex flex-col gap-1 p-2">
              {#each messages as item (keyOf(item))}
                <!-- Rows off screen skip layout and paint, so many loaded pages stay cheap. -->
                <li class="[contain-intrinsic-size:auto_76px] [content-visibility:auto]">
                  <button
                    type="button"
                    class={['flex w-full min-w-0 flex-col gap-0.5 rounded-lg px-2 py-2 text-left hover:bg-accent/60 focus-visible:ring-3 focus-visible:ring-ring/50',
                      selected && keyOf(selected) === keyOf(item) && 'bg-secondary']}
                    aria-pressed={selected !== null && keyOf(selected) === keyOf(item)}
                    onclick={() => (selected = item)}
                  >
                    <span class="flex w-full min-w-0 items-center gap-2 text-sm">
                      <span class="min-w-0 flex-1 truncate font-medium">{senderName(item.senderName)} → {item.receiver}</span>
                      <Chip tone={RECEIPT_TONE[item.receipt]}>{item.receipt}</Chip>
                    </span>
                    <span class="text-sm text-muted-foreground">
                      {time.format(item.createdAtMs)}{layout === 'screen' || item.machine !== 'mac' ? ` · ${machineName(item.machine)}` : ''}
                    </span>
                    <span class="truncate text-[13px]">{item.body}</span>
                  </button>
                </li>
              {/each}
            </ul>
            {#if hasOlder}
              <div class="px-2 pb-2">
                <Button class="w-full" size="sm" variant="ghost" disabled={loading} onclick={() => start('older', criteria())}>Load older</Button>
              </div>
            {/if}
          </ScrollArea>
        {/if}
      </div>
      {#if layout === 'screen'}
        <div class="min-h-0 rounded-xl bg-card">
          {#if selected}
            <ScrollArea class="h-full">{@render reader(selected)}</ScrollArea>
          {:else}
            <EmptyState title="Select a message" body="Its full text appears here." />
          {/if}
        </div>
      {:else if selected}
        <!-- A fixed height: the scroll viewport fills its box, so the box needs a size. -->
        <ScrollArea class="h-56 flex-none border-t border-border">{@render reader(selected)}</ScrollArea>
      {/if}
    </div>
  {/if}
</div>
