<script lang="ts">
  import { tick } from 'svelte';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import TerminalSurface from './TerminalSurface.svelte';
  import ConversationMessage from './conversation/ConversationMessage.svelte';
  import {
    conversationSessions,
    setConversationAttachments,
    setConversationDraft,
    setConversationMode,
    setConversationScrollTop,
    setConversationSelectedChild
  } from '$lib/shell/conversation/conversationStore.svelte';
  import {
    readChildConversationTranscript,
    respondToStructuredApproval,
    saveConversationClipboardImage,
    sendStructuredMessage,
    startConversationTranscriptMirror
  } from '$lib/shell/conversation/conversationService';

  interface Props {
    owned: OwnedSession[];
    activeOwnedId: string | null;
    registerHost(ownedId: string, host: HTMLElement): void;
    onHostLayout?(ownedId: string): void;
  }
  let { owned, activeOwnedId, registerHost, onHostLayout }: Props = $props();
  const active = $derived(owned.find((item) => item.ownedId === activeOwnedId) ?? null);
  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
  const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode !== 'raw');
  const visibleTimeline = $derived(conversation?.selectedChildId ? conversation.childTimeline : conversation?.timeline ?? []);
  const selectedChild = $derived(conversation?.children.find((child) => child.childId === conversation.selectedChildId) ?? null);
  const commands = $derived(active?.agent === 'claude'
    ? ['/help', '/model', '/permissions', '/compact']
    : ['/help', '/model', '/permissions', '/review', '/compact', '/copy', '/status', '/skills', '/agent', '/apps', '/plugins']);
  const commandQuery = $derived(conversation?.draft.startsWith('/') ? conversation.draft.toLowerCase() : '');
  const matchingCommands = $derived(commandQuery ? commands.filter((command) => command.startsWith(commandQuery)) : []);
  const remainingContext = $derived.by(() => {
    const used = conversation?.metadata.usedTokens;
    const window = conversation?.metadata.contextWindow;
    if (used == null || window == null || window <= 0) return null;
    return Math.max(0, Math.round(((window - used) / window) * 100));
  });
  let messagesHost = $state<HTMLDivElement | null>(null);
  let lastScrolledTurn = '';
  let attachmentError = $state('');

  $effect(() => {
    const latest = visibleTimeline[visibleTimeline.length - 1];
    const turnKey = latest ? `${conversation?.selectedChildId ?? 'parent'}:${visibleTimeline.length}:${latest.itemId}` : '';
    if (!messagesHost || !turnKey || turnKey === lastScrolledTurn) return;
    lastScrolledTurn = turnKey;
    void tick().then(() => { if (messagesHost) messagesHost.scrollTop = messagesHost.scrollHeight; });
  });

  $effect(() => {
    if (messagesHost && conversation && visibleTimeline.length && messagesHost.scrollTop === 0 && conversation.scrollTop > 0) {
      messagesHost.scrollTop = conversation.scrollTop;
    }
  });

  $effect(() => {
    if (structured && (active?.agent === 'claude' || active?.agent === 'codex') && active.nativeSessionId && active.ptySessionId) {
      startConversationTranscriptMirror({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
    }
  });

  async function send(): Promise<void> {
    if (!activeOwnedId || !conversation?.draft.trim() || conversation.sending || conversation.selectedChildId) return;
    const text = conversation.draft;
    setConversationDraft(activeOwnedId, '');
    try {
      await sendStructuredMessage(activeOwnedId, text, active?.ptySessionId);
      if (active && (active.agent === 'codex' || active.agent === 'claude') && opensProviderPicker(active.agent, text)) {
        setConversationMode(active.ownedId, 'raw');
      }
    }
    catch { setConversationDraft(activeOwnedId, text); }
  }

  function opensProviderPicker(provider: 'codex' | 'claude', text: string): boolean {
    const command = text.trim().split(/\s+/, 1)[0];
    if (provider === 'claude') return command === '/model' || command === '/permissions';
    return ['/model', '/permissions', '/skills', '/agent', '/subagents', '/apps', '/plugins'].includes(command);
  }

  async function openProviderPicker(command: '/model' | '/permissions'): Promise<void> {
    if (!active || !conversation || !active.ptySessionId || conversation.sending) return;
    setConversationDraft(active.ownedId, '');
    await sendStructuredMessage(active.ownedId, command, active.ptySessionId);
    setConversationMode(active.ownedId, 'raw');
  }

  async function selectChild(childId: string | null): Promise<void> {
    if (!active || !conversation || !active.nativeSessionId) return;
    setConversationSelectedChild(active.ownedId, childId);
    if (!childId) return;
    await readChildConversationTranscript({
      ownedId: active.ownedId,
      provider: conversation.provider,
      nativeSessionId: active.nativeSessionId,
      childSessionId: childId
    }).catch(() => undefined);
  }

  async function paste(event: ClipboardEvent): Promise<void> {
    if (!active || !conversation || conversation.selectedChildId) return;
    const files = [...(event.clipboardData?.files ?? [])].filter((file) => file.type.startsWith('image/'));
    if (!files.length) return;
    event.preventDefault();
    attachmentError = '';
    try {
      const saved = await Promise.all(files.map((file) => saveConversationClipboardImage(active.ownedId, file)));
      setConversationAttachments(active.ownedId, [...conversation.attachments, ...saved]);
    } catch (error) {
      attachmentError = error instanceof Error ? error.message : String(error);
    }
  }

  function removeAttachment(id: string): void {
    if (!active || !conversation) return;
    const found = conversation.attachments.find((item) => item.id === id);
    if (found) URL.revokeObjectURL(found.previewUrl);
    setConversationAttachments(active.ownedId, conversation.attachments.filter((item) => item.id !== id));
  }

  function chooseCommand(command: string): void {
    if (active) setConversationDraft(active.ownedId, command);
  }
</script>

<div class="conversation-shell">
  <div class:covered={structured} class="terminal-layer"><TerminalSurface {owned} {activeOwnedId} {registerHost} {onHostLayout} /></div>
  {#if structured && active && conversation}
    <section class="structured" aria-label={`${active.agent} conversation`}>
      <header>
        <div class="identity"><strong>{selectedChild?.label ?? active.title}</strong><span>{selectedChild ? `${active.agent} sub-agent transcript` : `${active.agent} session`}</span></div>
        <div class="header-actions"><button onclick={() => setConversationMode(active.ownedId, 'raw')}>Open raw terminal</button></div>
      </header>

      {#if conversation.children.length}
        <nav class="agent-tree" aria-label="Session agents">
          <button class:active={!conversation.selectedChildId} onclick={() => void selectChild(null)}><span class="agent-dot parent"></span>Parent</button>
          {#each conversation.children as child (child.childId)}
            <button class:active={conversation.selectedChildId === child.childId} onclick={() => void selectChild(child.childId)} title={child.childId}>
              <span class="tree-line">└</span><span class:working={child.state === 'active'} class:failed={child.state === 'failed'} class="agent-dot"></span>{child.label}<small>{child.state}</small>
            </button>
          {/each}
        </nav>
      {/if}

      <div class="messages" bind:this={messagesHost} onscroll={() => active && messagesHost && setConversationScrollTop(active.ownedId, messagesHost.scrollTop)}>
        {#if visibleTimeline.length === 0}<p class="empty">{conversation.selectedChildId ? 'This sub-agent transcript is not available yet.' : 'Start the conversation below.'}</p>{/if}
        {#each visibleTimeline as item (item.itemId)}
          {#if item.kind === 'user' || item.kind === 'assistant'}
            <ConversationMessage text={item.text} role={item.kind} label={item.kind === 'user' ? 'You' : selectedChild?.label ?? active.agent} />
          {:else if item.kind === 'approval'}
            <aside class="event approval"><span>{item.summary}</span>{#if item.state === 'requested'}<button onclick={() => respondToStructuredApproval(active.ownedId,item.requestId,'accept')}>Approve</button><button onclick={() => respondToStructuredApproval(active.ownedId,item.requestId,'decline')}>Decline</button>{/if}</aside>
          {:else if item.kind === 'tool'}<aside class="event">{item.name} · {item.state}{#if item.summary} · {item.summary}{/if}</aside>
          {:else if item.kind === 'error'}<aside class="event error">{item.message}</aside>{/if}
        {/each}
      </div>

      {#if !conversation.selectedChildId}
        <div class="composer-area">
          {#if conversation.attachments.length}
            <div class="attachments">{#each conversation.attachments as attachment (attachment.id)}<figure><img src={attachment.previewUrl} alt={attachment.name} /><button aria-label={`Remove ${attachment.name}`} onclick={() => removeAttachment(attachment.id)}>×</button></figure>{/each}</div>
          {/if}
          {#if attachmentError}<div class="attachment-error">{attachmentError}</div>{/if}
          {#if matchingCommands.length}
            <div class="command-menu" role="listbox">{#each matchingCommands as command}<button onclick={() => chooseCommand(command)}>{command}<small>{command === '/skills' ? 'Browse available skills in the real session' : 'Send through the existing session'}</small></button>{/each}</div>
          {/if}
          <div class="composer-row">
            <textarea aria-label="Message" placeholder={`Message ${active.agent}`} value={conversation.draft} onpaste={paste} oninput={(e) => setConversationDraft(active.ownedId,e.currentTarget.value)} onkeydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); } }}></textarea>
            <button class="send" disabled={!conversation.draft.trim() || conversation.sending} onclick={() => void send()}>{conversation.sending ? 'Working…' : 'Send'}</button>
          </div>
          <div class="session-status" aria-label="Session settings">
            <button title="Open the running provider's model picker" onclick={() => void openProviderPicker('/model')}>{conversation.metadata.model ?? 'Model unknown'}</button>
            <button title={active.agent === 'codex' ? "Open Codex's model and reasoning picker" : 'Reasoning effort reported by the running session'} disabled={active.agent !== 'codex' || !conversation.metadata.effort} onclick={() => void openProviderPicker('/model')}>{conversation.metadata.effort ? `${conversation.metadata.effort} effort` : 'Effort unknown'}</button>
            <button title="Open the running provider's approval picker" onclick={() => void openProviderPicker('/permissions')}>{conversation.metadata.approvalPolicy ?? 'Approval unknown'}</button>
            <span>{remainingContext == null ? 'Context unknown' : `${remainingContext}% context left`}</span>
            <span class="paste-hint">Paste screenshots with ⌘V</span>
          </div>
        </div>
      {:else}<div class="read-only-note">Read-only sub-agent transcript</div>{/if}
    </section>
  {:else if active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode === 'raw'}
    <button class="structured-toggle" onclick={() => setConversationMode(active.ownedId,'structured')}>Conversation</button>
  {/if}
</div>

<style>
  .conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.structured header{display:flex;justify-content:space-between;align-items:center;min-height:46px;padding:8px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 75%,transparent)}.identity{display:flex;align-items:baseline;gap:10px}.identity strong{font-size:13px}.identity span{color:var(--color-text-2);font-size:12px}button{border:1px solid color-mix(in srgb,var(--color-border) 82%,transparent);border-radius:7px;background:color-mix(in srgb,var(--color-surface) 80%,transparent);color:inherit;padding:6px 9px}button:hover:not(:disabled){background:color-mix(in srgb,var(--color-surface) 65%,var(--color-accent) 12%)}button:disabled{opacity:.55}.agent-tree{display:flex;gap:5px;overflow:auto;padding:7px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 55%,transparent)}.agent-tree button{display:flex;align-items:center;gap:6px;padding:5px 8px;border-color:transparent;background:transparent;white-space:nowrap;color:var(--color-text-2)}.agent-tree button.active{border-color:color-mix(in srgb,var(--color-accent) 35%,transparent);background:color-mix(in srgb,var(--color-accent) 11%,transparent);color:var(--color-text)}.agent-tree small{font-size:12px;opacity:.65}.tree-line{opacity:.35}.agent-dot{width:7px;height:7px;border-radius:50%;background:#7f8794}.agent-dot.parent{background:var(--color-accent)}.agent-dot.working{background:#52c7a8;box-shadow:0 0 0 3px color-mix(in srgb,#52c7a8 16%,transparent)}.agent-dot.failed{background:#e16f7a}.messages{flex:1;overflow:auto;padding:34px max(28px,calc((100% - 820px)/2)) 220px;display:flex;flex-direction:column;gap:30px;scrollbar-gutter:stable}.event{border-left:2px solid color-mix(in srgb,var(--color-accent) 45%,var(--color-border));padding:8px 12px;color:var(--color-text-2);user-select:text}.event button{margin-left:8px}.approval{background:color-mix(in srgb,var(--color-accent) 6%,transparent)}.error,.attachment-error{color:#ef8b92}.empty{margin:auto;color:var(--color-text-2)}.composer-area{position:absolute;left:50%;bottom:0;transform:translateX(-50%);width:min(820px,calc(100% - 44px));padding:12px 0 17px;background:linear-gradient(transparent,var(--color-bg) 20%,var(--color-bg))}.composer-row{display:flex;align-items:flex-end;gap:12px;min-height:76px;padding:12px 12px 10px 16px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:15px;background:color-mix(in srgb,var(--color-surface) 82%,var(--color-bg));box-shadow:0 8px 28px rgba(0,0,0,.18);transition:border-color 120ms ease,box-shadow 120ms ease}.composer-row:focus-within{border-color:color-mix(in srgb,var(--color-text) 28%,var(--color-border));box-shadow:0 8px 30px rgba(0,0,0,.22),0 0 0 1px color-mix(in srgb,var(--color-text) 5%,transparent)}.composer-row textarea{flex:1;min-height:52px;max-height:190px;padding:2px 0;resize:none;border:0!important;outline:0!important;box-shadow:none!important;background:transparent;color:inherit;font:14px/1.5 inherit;caret-color:var(--color-accent)}.send{align-self:flex-end;margin-bottom:2px}.session-status{display:flex;align-items:center;gap:5px;min-height:28px;color:var(--color-text-2);font-size:12px}.session-status button{border:0;background:transparent;padding:4px 6px;font-size:12px}.session-status span{padding:0 5px}.paste-hint{margin-left:auto}.attachments{display:flex;gap:8px}.attachments figure{position:relative;width:68px;height:52px;margin:0}.attachments img{width:100%;height:100%;object-fit:cover;border-radius:8px;border:1px solid var(--color-border)}.attachments button{position:absolute;right:-5px;top:-6px;width:20px;height:20px;padding:0;border-radius:50%}.command-menu{position:absolute;left:0;right:0;bottom:112px;display:flex;flex-direction:column;padding:6px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:0 12px 40px rgba(0,0,0,.28)}.command-menu button{display:flex;justify-content:space-between;border:0;background:transparent;text-align:left}.command-menu small{color:var(--color-text-2)}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}.structured-toggle{position:absolute;right:12px;top:12px;z-index:2}
</style>
