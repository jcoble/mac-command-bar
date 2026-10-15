<!-- Agents > Messages: the agent-to-agent traffic on every machine, newest first. -->
<script lang="ts">
  import { untrack } from 'svelte';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import type { ChipTone } from '$lib/components/ui/chip/variants.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump.ts';
  import { adoptSession } from '$lib/shell/conversation/conversationService.ts';
  import { listAgentMessagesFromTauri, type AgentMessage } from '$lib/tauriSource.ts';

  interface Props {
    visible: boolean;
    /** Show only this workflow run's messages; null shows all of them. */
    runId: string | null;
    /** The run's name, shown on its filter chip. */
    runLabel: string;
    onClearRun: () => void;
  }
  let { visible, runId, runLabel, onClearRun }: Props = $props();

  const LIMIT = 100;
  const RECEIPT_TONE: Record<AgentMessage['receipt'], ChipTone> = {
    queued: 'live',
    delivered: 'good',
    failed: 'bad',
    expired: 'neutral'
  };
  const time = new Intl.DateTimeFormat(undefined, { dateStyle: 'short', timeStyle: 'short' });

  let messages = $state<AgentMessage[]>([]);
  let hasOlder = $state(false);
  let loading = $state(false);
  let error = $state('');
  let loader: AbortController | null = null;
  // A newer click, or the view going away, makes a slow session lookup stale.
  let opening = 0;

  $effect(() => {
    if (!visible) return;
    const run = runId;
    untrack(() => start(run, false));
    return () => {
      loader?.abort();
      loader = null;
      opening += 1;
    };
  });

  function start(run: string | null, older: boolean): void {
    loader?.abort();
    const controller = new AbortController();
    loader = controller;
    void load(run, older, controller.signal);
  }

  async function load(run: string | null, older: boolean, signal: AbortSignal): Promise<void> {
    loading = true;
    error = '';
    const last = older ? messages.at(-1) : undefined;
    try {
      const page = (await listAgentMessagesFromTauri({
        runId: run,
        before: last ? { createdAtMs: last.createdAtMs, id: last.id } : null,
        limit: LIMIT
      })) ?? [];
      if (signal.aborted) return;
      messages = older ? [...messages, ...page] : page;
      hasOlder = page.length === LIMIT;
    } catch (cause) {
      if (!signal.aborted) error = `Could not load messages: ${cause instanceof Error ? cause.message : String(cause)}`;
    }
    if (!signal.aborted) loading = false;
  }

  /** A sender's label without the trailing `[owned id]` it carries for replies. */
  function senderName(label: string): string {
    return label.replace(/ \[[^\]]*\]$/, '');
  }

  async function open(ownedId: string): Promise<void> {
    const attempt = ++opening;
    try {
      const adopted = await adoptSession(ownedId);
      if (attempt !== opening) return;
      if (!adopted) throw new Error('That session is not available yet.');
      await sessionRowJump(ownedId, 'session');
    } catch (cause) {
      if (attempt === opening) error = `Could not open session: ${cause instanceof Error ? cause.message : String(cause)}`;
    }
  }
</script>

<div class="flex h-full min-h-0 flex-col" data-testid="agent-messages">
  <div class="flex min-w-0 items-center gap-2 border-b border-border px-3 py-2">
    {#if runId}
      <span class="min-w-0 truncate"><Chip tone="neutral">{runLabel}</Chip></span>
      <IconButton label="Show messages from every run" size="xs" onclick={onClearRun}><X /></IconButton>
    {:else}
      <p class="min-w-0 flex-1 truncate text-sm text-muted-foreground">Messages between agents on every machine.</p>
    {/if}
    <Button class="ml-auto" size="sm" variant="ghost" disabled={loading} onclick={() => start(runId, false)}>
      <RefreshCw />Refresh
    </Button>
  </div>
  {#if error}<p role="alert" class="px-3 py-2 text-sm text-destructive">{error}</p>{/if}

  {#if messages.length === 0}
    <EmptyState title={loading ? 'Loading messages' : 'No messages between agents yet.'} />
  {:else}
    <ScrollArea class="min-h-0 flex-1">
      <ul class="flex flex-col gap-1 p-2">
        {#each messages as item (item.id)}
          <li>
            <button
              type="button"
              class="flex w-full min-w-0 flex-col gap-1 rounded-lg px-2 py-2 text-left hover:bg-accent/60 focus-visible:ring-3 focus-visible:ring-ring/50"
              onclick={() => void open(item.toOwnedId)}
            >
              <span class="flex w-full min-w-0 items-center gap-2 text-sm">
                <span class="min-w-0 flex-1 truncate font-medium" title={item.senderName}>{senderName(item.senderName)}</span>
                <Chip tone={RECEIPT_TONE[item.receipt]}>{item.receipt}</Chip>
              </span>
              <span class="truncate text-sm text-muted-foreground" title={item.receiver}>to {item.receiver}</span>
              <span class="text-sm text-muted-foreground">
                {time.format(item.createdAtMs)}{item.machine !== 'mac' ? ` · ${item.machine}` : ''}
              </span>
              <span class="line-clamp-3 text-[13px] whitespace-pre-wrap" title={item.body}>{item.body}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if hasOlder}
        <div class="px-2 pb-2">
          <Button class="w-full" size="sm" variant="ghost" disabled={loading} onclick={() => start(runId, true)}>Load older</Button>
        </div>
      {/if}
    </ScrollArea>
  {/if}
</div>
