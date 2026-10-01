<script lang="ts">
  import { createChat } from '@tanstack/ai-svelte';
  import { onDestroy, untrack } from 'svelte';
  import { assemblySpikeAdapter, spikeWindow } from '../../conversation/tanstackSpike';
  import { conversationSessions, setConversationAttachments, setConversationSelectedChild } from '../../conversation/conversationStore.svelte';
  import { loadOlderConversationEvents, loadNewerConversationEvents, readChildConversationTranscript, saveConversationClipboardImage, cleanupConversationAttachment } from '../../conversation/conversationService';
  import AttachmentLightbox from './AttachmentLightbox.svelte';
  let { ownedId }: { ownedId: string } = $props();
  const id = untrack(() => ownedId);
  const adapter = assemblySpikeAdapter(id);
  const chat = createChat({
    threadId: id, connection: adapter.connection, live: true,
    initialMessages: spikeWindow(id)
  });
  const childRead = new AbortController();
  let disposed = false;
  let saving = $state(false);
  let preview = $state('');
  onDestroy(() => {
    disposed = true;
    childRead.abort();
    setConversationSelectedChild(id, null);
    if (preview) URL.revokeObjectURL(preview);
    chat.dispose();
  });
  const workspace = $derived(conversationSessions[id]);
  let draft = $state('');
  let error = $state('');
  let paging = false;
  // Window replacement/trim is authoritative, including trimming the newer end.
  $effect(() => {
    const oldest = workspace?.oldestLoadedSequence;
    const reachedEnd = workspace?.reachedTranscriptEnd;
    Object.values(workspace?.sentAttachments ?? {}).flatMap((items) => items.map((item) => item.previewUrl));
    untrack(() => { if (oldest !== undefined || reachedEnd !== undefined) adapter.syncWindow(); });
  });
  async function perform(work: Promise<unknown>) {
    try { error = ''; await work; } catch (cause) { error = String(cause); }
  }
  async function page(older: boolean) {
    if (paging) return;
    paging = true;
    try {
      await (older ? loadOlderConversationEvents(id) : loadNewerConversationEvents(id));
      adapter.syncWindow();
    } catch (cause) { error = String(cause); }
    finally { paging = false; }
  }
  async function attach(file: File | undefined) {
    if (!file || saving) return;
    saving = true;
    const url = URL.createObjectURL(file);
    preview = url;
    try {
      const saved = await saveConversationClipboardImage(id, file);
      if (disposed) await cleanupConversationAttachment(id, saved);
      else setConversationAttachments(id, [...(conversationSessions[id]?.attachments ?? []), saved]);
    } finally {
      URL.revokeObjectURL(url);
      if (!disposed) { preview = ''; saving = false; }
    }
  }
  async function selectChild(childId: string | null) {
    setConversationSelectedChild(id, childId);
    adapter.syncWindow();
    const state = conversationSessions[id];
    if (!childId || !state?.nativeSessionId) return;
    await readChildConversationTranscript({
      ownedId: id, provider: state.provider, nativeSessionId: state.nativeSessionId,
      childSessionId: childId, signal: childRead.signal
    });
    if (!disposed) adapter.syncWindow();
  }
  function send() {
    const text = draft;
    draft = '';
    void perform(chat.sendMessage({ content: [
      { type: 'text', content: text },
      ...(workspace?.attachments ?? []).map((attachment) => ({ type: 'image' as const,
        source: { type: 'url' as const, value: attachment.previewUrl, mimeType: attachment.mimeType } }))
    ] }));
  }
</script>

<div class="spike">
  <header>TanStack client spike · SQL window {Math.round((workspace?.loadedEventsBytes ?? 0) / 1024)} KiB · {chat.messages.length} messages · {chat.error ? "error" : chat.sessionGenerating ? "streaming" : chat.isLoading ? "sending" : "ready"}</header>
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
          {#if part.type === 'image'}
            <AttachmentLightbox src={part.source.value} name="Attached image" variant="timeline" />
          {/if}
          {#if part.type === 'subagent'}
            <button onclick={() => void perform(selectChild(workspace?.selectedChildId === part.subagent.id ? null : part.subagent.id))}>
              {part.subagent.name} · {part.subagent.status} · {workspace?.selectedChildId === part.subagent.id ? 'Close transcript' : 'Open transcript'}
            </button>
            {#each part.subagent.messages as childMessage (childMessage.id)}
              <strong>{childMessage.role}</strong>
              {#each childMessage.parts as childPart}
                {#if childPart.type === 'text'}<p>{childPart.content}</p>{/if}
              {/each}
            {/each}
          {/if}
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
    <label>Attach image <input type="file" accept="image/*" disabled={saving} onchange={(event) => {
      const file = event.currentTarget.files?.[0]; event.currentTarget.value = ''; void perform(attach(file));
    }} /></label>
    {#if preview}<AttachmentLightbox src={preview} name="Saving image" variant="composer" />{/if}
    {#each workspace?.attachments ?? [] as attachment (attachment.id)}
      <AttachmentLightbox src={attachment.previewUrl} name={attachment.name} variant="composer" />
    {/each}
    <textarea bind:value={draft} aria-label="Spike message" rows="3"></textarea>
    <button onclick={send} disabled={saving || (!draft.trim() && !workspace?.attachments.length) || chat.isLoading}>Send through TanStack</button>
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
