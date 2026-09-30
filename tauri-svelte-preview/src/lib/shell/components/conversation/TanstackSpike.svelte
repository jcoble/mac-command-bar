<script lang="ts">
  import { createChat } from '@tanstack/ai-svelte';
  import { onDestroy, untrack } from 'svelte';
  import { assemblySpikeAdapter, spikeWindow } from '../../conversation/tanstackSpike';
  import { conversationSessions } from '../../conversation/conversationStore.svelte';
  import { loadOlderConversationEvents, loadNewerConversationEvents } from '../../conversation/conversationService';
  let { ownedId }: { ownedId: string } = $props();
  const id = untrack(() => ownedId);
  const adapter = assemblySpikeAdapter(id, () => chat.setMessages(spikeWindow(id)));
  const chat = createChat({
    threadId: id, connection: adapter.connection, live: true,
    initialMessages: spikeWindow(id),
    onCustomEvent: () => chat.setMessages(spikeWindow(id))
  });
  onDestroy(() => chat.dispose());
  const workspace = $derived(conversationSessions[id]);
  let draft = $state('');
  let error = $state('');
  let paging = false;
  // Window replacement/trim is authoritative, including trimming the newer end.
  $effect(() => {
    const oldest = workspace?.oldestLoadedSequence;
    const reachedEnd = workspace?.reachedTranscriptEnd;
    untrack(() => { if (oldest !== undefined || reachedEnd !== undefined) chat.setMessages(spikeWindow(id)); });
  });
  async function perform(work: Promise<unknown>) {
    try { error = ''; await work; } catch (cause) { error = String(cause); }
  }
  async function page(older: boolean) {
    if (paging) return;
    paging = true;
    try {
      await (older ? loadOlderConversationEvents(id) : loadNewerConversationEvents(id));
      chat.setMessages(spikeWindow(id));
    } catch (cause) { error = String(cause); }
    finally { paging = false; }
  }
  function send() { const text = draft; draft = ''; void perform(chat.sendMessage(text)); }
</script>

<div class="spike">
  <header>TanStack client spike · SQL window {Math.round((workspace?.loadedEventsBytes ?? 0) / 1024)} KiB · {chat.messages.length} messages · {chat.status}</header>
  <div class="transcript" onscroll={(event) => {
    const node = event.currentTarget;
    if (node.scrollTop < 40 && !workspace.reachedTranscriptStart) void page(true);
    else if (node.scrollHeight - node.scrollTop - node.clientHeight < 40 && !workspace.reachedTranscriptEnd) void page(false);
  }}>
    <button onclick={() => void page(true)} disabled={workspace?.reachedTranscriptStart}>Older SQL page</button>
    {#each chat.messages as message (message.id)}
      <article>
        <strong>{message.role}</strong>
        {#if message.parts.some((part) => part.type === 'tool-call')}
          <details><summary>Tool group · {message.parts.filter((part) => part.type === 'tool-call').map((part) => part.name).join(', ')}</summary>
            <pre class="tools">{JSON.stringify(message.parts, null, 2)}</pre>
          </details>
        {/if}
        {#each message.parts as part}
          {#if part.type === 'text'}<p>{part.content}</p>{/if}
          {#if part.type === 'thinking'}<details><summary>Reasoning</summary>{part.content}</details>{/if}
        {/each}
      </article>
    {/each}
    <button onclick={() => void page(false)} disabled={workspace?.reachedTranscriptEnd}>Newer SQL page</button>
  </div>
  {#each Object.values(workspace?.pendingApprovals ?? {}) as approval}
    <aside>{approval.title}
      {#each approval.options as option}
        <button onclick={() => void perform(adapter.approve(approval.requestId, option.optionId))}>{option.name}</button>
      {/each}
    </aside>
  {/each}
  {#if error || chat.error}<p role="alert">{error || chat.error?.message}</p>{/if}
  <footer>
    <textarea bind:value={draft} aria-label="Spike message" rows="3"></textarea>
    <button onclick={send} disabled={!draft.trim() || chat.isLoading}>Send through TanStack</button>
    <button onclick={() => void perform(adapter.interrupt())}>Interrupt in Rust</button>
    <button onclick={() => { const text = draft; draft = ''; void perform(adapter.steer(text)); }} disabled={!draft.trim()}>Steer in Rust</button>
  </footer>
</div>

<style>
  .spike { display:flex; flex-direction:column; min-height:0; height:100%; padding:12px; gap:8px; }
  .transcript { overflow:auto; flex:1; min-height:0; }
  article { padding:12px; content-visibility:auto; contain-intrinsic-size:auto 120px; }
  p { white-space:pre-wrap; }
  .tools { max-height:240px; overflow:auto; white-space:pre-wrap; }
  textarea { width:100%; }
  button { padding:6px 10px; margin:4px; }
  header { font-size:12px; }
</style>
