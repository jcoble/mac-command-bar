<script lang="ts">
  import type { Snippet } from 'svelte';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import ImageUp from '@lucide/svelte/icons/image-up';
  import Mic from '@lucide/svelte/icons/mic';
  import Plus from '@lucide/svelte/icons/plus';
  import Square from '@lucide/svelte/icons/square';
  import X from '@lucide/svelte/icons/x';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import type { ConversationAttachment, AgentConfigValue, AgentPermissionRequest, AgentUserInputRequest } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import type { AgentConversationConfigField, AgentConversationConfigState } from '$lib/shell/conversation/conversationConfig.ts';
  import type { ConversationCommand } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import { contextMeterPopover, draftAfterSlashCommand, moveSlashMenuIndex, ringDash, slashCommandQuery, slashMenuState, snapshotConversationCommands, type ContextMeterState } from '$lib/shell/conversation/composerSlashCommands.ts';
  import { agentDisplayName } from '$lib/shell/agentIcons.ts';
  import type { AgentKind } from '$lib/shell/ownedSessions.ts';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import AgentCommandMenu from './AgentCommandMenu.svelte';
  import AttachmentLightbox from './AttachmentLightbox.svelte';
  import ComposerBannerStack, { type ComposerBannerItem } from './ComposerBannerStack.svelte';
  import ComposerConfigMenu from './ComposerConfigMenu.svelte';
  import PlanChip, { type PlanFileChanges } from './PlanChip.svelte';
  import ComposerPendingApprovalPanel from './ComposerPendingApprovalPanel.svelte';
  import ComposerPendingUserInputPanel from './ComposerPendingUserInputPanel.svelte';
  import CompactComposerControlsMenu from './CompactComposerControlsMenu.svelte';

  type ComposerAttachment = ConversationAttachment & { byteLength?: number };
  interface Props {
    provider: string;
    draft: string;
    attachments: readonly ComposerAttachment[];
    sending: boolean;
    supportsSteering?: boolean;
    configState: AgentConversationConfigState;
    pendingConfig: Partial<Record<AgentConversationConfigField, string>>;
    configError?: string | null;
    commands: readonly ConversationCommand[];
    contextMeter?: ContextMeterState | null;
    /** The plan the session is working to, drawn as a chip above the capsule. */
    plan?: Extract<ConversationDisplayItem, { kind: 'plan' }> | null;
    /** What the running turn has changed on disk so far, shown beside the step
     * count. Left out when the turn has touched no files. */
    planFileChanges?: PlanFileChanges | null;
    attachmentError?: string;
    /** Clears the attachment failure. Given only where the failure is
     * per-session state that can be cleared; the draft composer has none. */
    onDismissAttachmentError?(): void;
    sendError?: string;
    /** Clears the send failure. Given only where the failure is per-session
     * state that can be cleared; the draft composer has none. */
    onDismissSendError?(): void;
    /** What this session's provider will not do. Said once, then dismissed. */
    providerNotice?: string;
    onDismissProviderNotice?(): void;
    pendingApproval?: AgentPermissionRequest | null;
    pendingApprovalCount?: number;
    pendingInputs?: readonly AgentUserInputRequest[];
    respondingRequestIds?: readonly string[];
    /** Extra controls for the left of the footer, ahead of Attach. The draft
     * session puts its project and branch pickers here, so a session being set
     * up reads as the same composer as one already running. */
    leadingControls?: Snippet;
    onDraftChange?(value: string): void;
    onDraftBlur?(): void | Promise<void>;
    onSend?(): void | Promise<void>;
    onStop?(): void | Promise<void>;
    onPaste?(event: ClipboardEvent): void | Promise<void>;
    onDropFiles?(files: File[]): void | Promise<void>;
    onRemoveAttachment?(id: string): void | Promise<void>;
    onAnnotateAttachment?(id: string): void;
    onConfigChange?(field: AgentConversationConfigField, value: string): void | Promise<void>;
    onCommandSelected?(command: ConversationCommand): void;
    onApprovalDecision?(requestId: string, optionId: string): void | Promise<void>;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void | Promise<void>;
    onHeightChange?(height: number): void;
  }

  let {
    provider,
    draft,
    attachments,
    sending,
    supportsSteering = false,
    configState,
    pendingConfig,
    configError = null,
    commands,
    contextMeter = null,
    plan = null,
    planFileChanges = null,
    attachmentError = '',
    onDismissAttachmentError,
    sendError = '',
    onDismissSendError,
    providerNotice = '',
    onDismissProviderNotice,
    pendingApproval = null,
    pendingApprovalCount = 1,
    pendingInputs = [],
    respondingRequestIds = [],
    leadingControls,
    onDraftChange,
    onDraftBlur,
    onSend,
    onStop,
    onPaste,
    onDropFiles,
    onRemoveAttachment,
    onAnnotateAttachment,
    onConfigChange,
    onCommandSelected,
    onApprovalDecision,
    onInputSubmit,
    onHeightChange
  }: Props = $props();

  let activeIndex = $state(0);
  let addMenuOpen = $state(false);
  let dismissedDraft = $state<string | null>(null);
  let dragging = $state(false);
  let inputDraft = $state('');
  let observedDraft = $state('');
  let promptHost = $state<HTMLTextAreaElement | null>(null);
  let composerArea = $state<HTMLDivElement | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);
  let commandSnapshot = $state<ConversationCommand[]>([]);

  /** Put the caret in the prompt box. The only way in from outside — a panel
   * that hands the composer an attachment wants the reader typing next to it. */
  export function focus(): void {
    promptHost?.focus();
  }

  let planExpanded = $state(false);
  /* Whether the turn on screen has actually started. A turn that has not begun
     yet is not a turn that just ended, and without this the chip would shut
     itself the moment it opened while the session sat idle. */
  let planTurnRunning = false;

  /** Open the plan chip. The transcript's "Plan updated" line calls this, so
   * the plan is read in one place wherever the reader asks for it from. */
  export function expandPlan(): void {
    if (plan) planExpanded = true;
  }

  /* The plan is what the session is doing now, so the panel shuts when the
     session stops doing it. */
  $effect(() => {
    if (sending) {
      planTurnRunning = true;
      return;
    }
    if (planTurnRunning) {
      planTurnRunning = false;
      planExpanded = false;
    }
  });

  function onWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && planExpanded) planExpanded = false;
  }

  $effect(() => {
    if (draft === observedDraft) return;
    if (slashCommandQuery(inputDraft) === null && slashCommandQuery(draft) !== null) {
      commandSnapshot = snapshotConversationCommands(commands);
    }
    observedDraft = draft;
    inputDraft = draft;
    activeIndex = 0;
    dismissedDraft = null;
    resizePrompt();
  });

  const menu = $derived(slashMenuState(inputDraft, commandSnapshot, {
    activeIndex,
    dismissed: dismissedDraft === inputDraft
  }));
  /* The exact numbers behind the ring, named for the provider that reported
     them so an absent figure reads as that provider's silence. */
  const meterPopover = $derived(
    contextMeter ? contextMeterPopover(contextMeter, agentDisplayName(provider as AgentKind)) : null
  );
  const composerLocked = $derived(Boolean(pendingApproval || pendingInputs.length));
  const hasSendableContent = $derived(Boolean(inputDraft.trim() || attachments.length));
  /**
   * Empty, the composer is a single capsule line: add button, placeholder,
   * the settings chips and the mic. Once it is carrying something the box
   * relaxes into a rounded rectangle and the message takes a line of its own
   * above the control row. A turn in flight, an approval to answer and a file
   * being dragged over all need that second line too, so they open the box the
   * same way typing does.
   *
   * Focus deliberately does NOT open it. Focus arrives from more places than
   * anyone can hold in their head — a click landing anywhere in the capsule, a
   * menu handing focus back as it closes, the window itself being returned to —
   * so an opening keyed to it appears to fire at random. Content is the only
   * signal a person can predict, so content is the only one that moves the box.
   */
  const opening = $derived(
    Boolean(hasSendableContent || sending || composerLocked || dragging)
  );
  /**
   * The box holds the height its message needed while that message is being
   * written: clearing the text by hand, or backspacing to nothing on the way
   * to rewording it, leaves the box where it was rather than snapping shut
   * under the cursor. Sending is the one thing that puts it back — the message
   * has gone, so the box goes back to the line it starts as.
   *
   * Only content latches it. A turn in flight, an approval and a drag all open
   * the box through `opening` for as long as they last, and none of them is a
   * reason for it to stay open afterwards.
   */
  let hasOpened = $state(false);
  $effect(() => {
    if (hasSendableContent) hasOpened = true;
  });
  const relaxed = $derived(opening || hasOpened);
  const bannerItems = $derived.by((): ComposerBannerItem[] => {
    const items: ComposerBannerItem[] = [];
    if (sendError) items.push({ id: 'send-error', variant: 'error', title: 'Message not sent', description: sendError, dismissLabel: 'Dismiss send failure', onDismiss: onDismissSendError });
    if (attachmentError) items.push({ id: 'attachment-error', variant: 'error', title: 'Attachment unavailable', description: attachmentError, dismissLabel: 'Dismiss attachment failure', onDismiss: onDismissAttachmentError });
    if (configError) items.push({ id: 'config-error', variant: 'warning', title: 'Settings unavailable', description: configError });
    if (providerNotice) items.push({ id: 'provider-notice', variant: 'info', title: providerNotice, dismissLabel: 'Dismiss provider notice', onDismiss: onDismissProviderNotice });
    return items;
  });

  $effect(() => {
    if (!composerArea || !onHeightChange) return;
    let publishedHeight = -1;
    const publish = (): void => {
      const nextHeight = Math.ceil(composerArea?.getBoundingClientRect().height ?? 0);
      if (nextHeight === publishedHeight) return;
      publishedHeight = nextHeight;
      onHeightChange(nextHeight);
    };
    const observer = new ResizeObserver(publish);
    observer.observe(composerArea);
    publish();
    return () => {
      observer.disconnect();
    };
  });

  /** One line while the message is short, growing with it up to a ceiling. */
  function resizePrompt(): void {
    if (!promptHost) return;
    promptHost.style.height = 'auto';
    promptHost.style.height = `${Math.min(190, promptHost.scrollHeight)}px`;
  }

  function chooseCommand(command: ConversationCommand | undefined): void {
    if (!command) return;
    activeIndex = 0;
    const nextDraft = draftAfterSlashCommand(inputDraft, command);
    inputDraft = nextDraft;
    onDraftChange?.(nextDraft);
    resizePrompt();
    onCommandSelected?.(command);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (menu.open) {
      if (event.key === 'Escape') {
        event.preventDefault();
        dismissedDraft = inputDraft;
        return;
      }
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        activeIndex = moveSlashMenuIndex(menu.activeIndex, menu.commands.length, event.key);
        return;
      }
      if (event.key === 'Enter' && !event.shiftKey && menu.commands.length) {
        event.preventDefault();
        chooseCommand(menu.commands[menu.activeIndex]);
        return;
      }
    }
    if (event.key === 'Enter' && !event.shiftKey && !composerLocked) {
      event.preventDefault();
      submitPrompt();
    }
  }

  /** Hand the message off and let the box return to its resting height. */
  function submitPrompt(): void {
    if (composerLocked) return;
    hasOpened = false;
    void onSend?.();
  }

  function changeDraft(value: string): void {
    if (slashCommandQuery(inputDraft) === null && slashCommandQuery(value) !== null) {
      commandSnapshot = snapshotConversationCommands(commands);
    }
    inputDraft = value;
    activeIndex = 0;
    dismissedDraft = null;
    onDraftChange?.(value);
    resizePrompt();
  }

  function imageFiles(transfer: DataTransfer | null): File[] {
    return [...(transfer?.files ?? [])];
  }

  function onDragOver(event: DragEvent): void {
    if (!onDropFiles || !event.dataTransfer?.types.includes('Files')) return;
    event.preventDefault();
    dragging = true;
  }

  function onDrop(event: DragEvent): void {
    if (!onDropFiles) return;
    const files = imageFiles(event.dataTransfer);
    dragging = false;
    if (!files.length) return;
    event.preventDefault();
    void onDropFiles(files);
  }

  function chooseFiles(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    input.value = '';
    if (files.length) void onDropFiles?.(files);
  }

  function byteLabel(attachment: ComposerAttachment): string {
    const size = attachment.byteLength;
    if (typeof size !== 'number') return attachment.mimeType;
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
    return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="composer-area" data-testid="conversation-composer-area" bind:this={composerArea}>
  {#if bannerItems.length}<ComposerBannerStack items={bannerItems} />{/if}
  <!-- A draft session's own pickers sit ABOVE the capsule, not inside it.
       They set up the session rather than the message, and three of them on
       the control row left the message nowhere to go. -->
  {#if leadingControls}<div class="leading-controls" data-testid="composer-leading-controls">{@render leadingControls()}</div>{/if}
  {#if (plan && plan.steps.length) || planFileChanges}
    <div class="plan-chip-slot">
      <PlanChip {plan} fileChanges={planFileChanges} expanded={planExpanded} running={sending} onToggle={() => (planExpanded = !planExpanded)} />
    </div>
  {/if}
  <form class="composer-form" onsubmit={(event) => { event.preventDefault(); submitPrompt(); }}>
    <div
      class:dragging
      class:relaxed
      class:locked={composerLocked}
      class="composer-box"
      data-testid="conversation-composer-box"
      role="presentation"
      ondragover={onDragOver}
      ondragleave={() => (dragging = false)}
      ondrop={onDrop}
    >
      {#if pendingApproval || pendingInputs.length}
        <div class="composer-panels">
          {#if pendingApproval}
            <ComposerPendingApprovalPanel
              approval={pendingApproval}
              pendingCount={pendingApprovalCount}
              responding={respondingRequestIds.includes(pendingApproval.requestId)}
              onDecision={onApprovalDecision}
            />
          {:else}
            <ComposerPendingUserInputPanel
              requests={pendingInputs}
              responding={respondingRequestIds.length > 0}
              onSubmit={onInputSubmit}
            />
          {/if}
        </div>
      {/if}

      <div class="composer-input-zone" data-testid="conversation-composer">
        {#if attachments.length}
          <div class="attachments" data-testid="conversation-attachment-previews">
            {#each attachments as attachment (attachment.id)}
              <figure data-testid="conversation-attachment-preview">
                <div class="attachment-preview">
                  <AttachmentLightbox
                    src={attachment.previewUrl}
                    fullPath={attachment.path}
                    name={attachment.name}
                    variant="composer"
                  />
                  <button class="attachment-remove" data-testid="conversation-attachment-remove" aria-label={`Remove ${attachment.name}`} type="button" onclick={() => void onRemoveAttachment?.(attachment.id)}><X size={13} /></button>
                </div>
                {#if onAnnotateAttachment}<button class="attachment-annotate" data-testid="conversation-attachment-annotate" type="button" onclick={() => onAnnotateAttachment?.(attachment.id)}>Annotate</button>{/if}
                <figcaption><strong>{attachment.name}</strong><small>{byteLabel(attachment)}</small></figcaption>
              </figure>
            {/each}
          </div>
        {/if}
        {#if menu.open && !composerLocked}<AgentCommandMenu commands={menu.commands} activeIndex={menu.activeIndex} emptyText={menu.emptyText} onSelect={chooseCommand} />{/if}
        <div class="prompt-row">
          <textarea
            bind:this={promptHost}
            data-testid="conversation-composer-input"
            aria-label="Message"
            spellcheck="false"
            autocapitalize="off"
            placeholder={pendingApproval ? 'Resolve the approval above to continue' : pendingInputs.length ? 'Complete the requested input above' : `Message ${provider}`}
            value={inputDraft}
            disabled={composerLocked}
            onpaste={(event) => void onPaste?.(event)}
            oninput={(event) => changeDraft(event.currentTarget.value)}
            onblur={() => void onDraftBlur?.()}
            onkeydown={onKeydown}
          ></textarea>
        </div>
      </div>

      {#if !composerLocked}
        <div class="composer-footer" data-testid="conversation-composer-footer">
          <div class="footer-left">
            <input bind:this={fileInput} class="file-input" type="file" accept="image/*" multiple onchange={chooseFiles} />
            <!-- The add button. It carries a menu, so it turns into a close
                 cross while that menu is on screen. -->
            <DropdownMenu.Root bind:open={addMenuOpen}>
              <DropdownMenu.Trigger>
                {#snippet child({ props })}
                  <button
                    {...props}
                    class:open={addMenuOpen}
                    class="round-control add-control"
                    type="button"
                    aria-label={addMenuOpen ? 'Close the add menu' : 'Add to this message'}
                  >
                    {#if addMenuOpen}<X size={18} />{:else}<Plus size={18} />{/if}
                  </button>
                {/snippet}
              </DropdownMenu.Trigger>
              <DropdownMenu.Content
                side="top"
                align="start"
                sideOffset={8}
                avoidCollisions
                collisionPadding={12}
                class="w-auto! max-w-[320px] min-w-[var(--menu-sheet-min-width)]"
              >
                <DropdownMenu.Item onSelect={() => fileInput?.click()}>
                  <ImageUp aria-hidden="true" />
                  <span class="menu-row-copy">
                    <span>Attach images</span>
                    <span class="menu-row-description">Or paste and drop them in</span>
                  </span>
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          </div>
          <div class="footer-right">
            <!-- How full the context window is, as a ring: the arc is what has
                 been used, so a fresh session shows an empty circle and a
                 session near its limit shows a full one. A window barely
                 touched still paints a small sliver rather than nothing. The
                 exact numbers are one hover away and the remaining percent is
                 in the hint line below; the ring is for the glance. Nothing is
                 shown when the usage isn't trustworthy enough to turn into a
                 percentage. -->
            {#if contextMeter?.kind === 'percent' && meterPopover}
              <Tooltip.Provider delayDuration={0}>
                <Tooltip.Root>
                  <Tooltip.Trigger>
                    {#snippet child({ props })}
                      <span
                        {...props}
                        class="context-ring"
                        class:context-ring-warm={contextMeter.warm}
                        class:context-ring-hot={contextMeter.hot}
                        data-testid="conversation-context-remaining"
                        role="img"
                        aria-label={`${contextMeter.remaining}% context left`}
                        style={`--arc:${ringDash(contextMeter.arc, 100)}`}
                      ><svg viewBox="0 0 20 20" aria-hidden="true"><circle class="ring-track" cx="10" cy="10" r="7.5" /><circle class="ring-fill" cx="10" cy="10" r="7.5" pathLength="100" /></svg></span>
                    {/snippet}
                  </Tooltip.Trigger>
                  <Tooltip.Content side="top" sideOffset={6} class="p-0!">
                    <span class="context-popover" data-testid="conversation-context-popover">
                      <span class="context-popover-headline">{meterPopover.headline}</span>
                      {#each meterPopover.rows as row (row.label)}
                        <span class="context-popover-row">
                          <span class="context-popover-label">{row.label}</span>
                          <span class="context-popover-value">{row.value}</span>
                        </span>
                      {/each}
                    </span>
                  </Tooltip.Content>
                </Tooltip.Root>
              </Tooltip.Provider>
            {/if}
            <div class="wide-controls"><ComposerConfigMenu {provider} state={configState} pending={pendingConfig} error={configError} onChange={onConfigChange} /></div>
            <div class="compact-controls"><CompactComposerControlsMenu {provider} state={configState} pending={pendingConfig} onChange={onConfigChange} /></div>
            <!-- The mic keeps its place in every state; send joins it to the
                 right the moment there is something to send. -->
            <button class="round-control mic" type="button" disabled title="Voice input is not available yet" aria-label="Voice input, not available yet"><Mic size={17} /></button>
            {#if sending && (!hasSendableContent || !supportsSteering)}
              <button class="round-control send stop" data-testid="conversation-stop" type="button" aria-label="Stop generation" onclick={() => void onStop?.()}><Square size={13} fill="currentColor" /></button>
            {/if}
            {#if hasSendableContent && (!sending || supportsSteering)}
              <button class="round-control send" data-testid="conversation-send" type="submit" aria-label={sending ? 'Steer current turn' : 'Send message'}><ArrowUp size={17} strokeWidth={2.2} /></button>
            {/if}
          </div>
        </div>
      {/if}
      {#if dragging}<p class="drop-hint" data-testid="conversation-drop-hint">Drop images to attach them</p>{/if}
    </div>
  </form>
  <div class="composer-hint" data-testid="conversation-paste-hint"><span>Paste or drop images · type / for commands</span>{#if contextMeter?.kind === 'percent'}<span aria-hidden="true">·</span><span>{contextMeter.remaining}% left</span>{/if}</div>
</div>

<style>
  /* The backdrop runs the full width of the panel so the transcript dissolves
     into it everywhere, not only behind the box. The box keeps its own width
     and stays centred. The whole reserved space above the box is the ramp, and
     the ramp holds off before it converges, so lines thin out over the length
     of it instead of meeting an edge. It is opaque by the time it reaches the
     box, or the transcript would read through beside the capsule. */
  .composer-area { position: absolute; left: 0; right: 0; bottom: 0; z-index: 2; padding: var(--composer-fade) 0 6px; container-type: inline-size; container-name: composer; background: linear-gradient(to bottom, transparent, color-mix(in srgb, var(--color-bg) 45%, transparent) calc(var(--composer-fade) * 0.55), var(--color-bg) var(--composer-fade)); }
  .composer-form { width: min(820px, calc(100% - 44px)); margin: 0 auto; }
  /* The chip is centred on the capsule and takes the same width, so its
     panel opens inside the composer's own column rather than the panel's. */
  .plan-chip-slot { width: min(820px, calc(100% - 44px)); margin: 0 auto; }

  /* The box is one grid in two shapes.
     Empty, it is a single capsule line — add button, message, controls.
     Carrying something, the message takes the whole width on its own line and
     the two control groups sit on the line below it. Only the named areas
     change between the two; nothing moves in the markup. */
  .composer-box {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    grid-template-areas: 'panels panels panels' 'lead prompt trail' 'hint hint hint';
    align-items: center;
    overflow: visible;
    padding: var(--composer-inset);
    /* No outline at rest: the surface and its shadow are what separate the box
       from the page. The width stays so that dragging and a locked composer can
       colour it without the box changing size underneath them. */
    border: 1px solid transparent;
    border-radius: var(--composer-radius-capsule);
    /* A step lighter again than the sheet colour the token names. The box floats
       on the backdrop's own near-black, where a drop shadow has nothing to fall
       on, so the fill is the whole of what lifts it — and at the token's value
       the difference was small enough that the capsule read as a hole rather
       than a control. */
    background: color-mix(in srgb, var(--color-text) 3%, var(--composer-surface));
    box-shadow: var(--shadow-md);
  }
  /* Shut, the box is as tall as the controls sitting in it and no taller, which
     came out shorter than the thing it is: the one place a conversation starts.
     A floor gives it presence without changing the shape. */
  .composer-box:not(.relaxed) { min-height: 70px; }
  .composer-box.relaxed { grid-template-areas: 'panels panels panels' 'prompt prompt prompt' 'lead . trail' 'hint hint hint'; border-radius: var(--composer-radius-relaxed); }
  .composer-box:focus-within { border-color: var(--composer-border-focus); box-shadow: var(--shadow-lg); }
  .composer-box.dragging { border-color: var(--color-accent); background: var(--composer-surface-drop); }
  .composer-box.locked { border-color: var(--composer-border-locked); }
  .composer-panels { grid-area: panels; }
  .composer-input-zone { grid-area: prompt; position: relative; padding: var(--composer-prompt-inset-capsule); }
  .composer-box.relaxed .composer-input-zone { padding: var(--composer-prompt-inset); }
  /* Three lines of room once the box is open. The height a textarea is given
     follows what is typed into it, which at one line left a box barely taller
     than the capsule it just grew out of — opening that looked like nothing
     happened. This is the floor; the text still grows past it to 190px. */
  .composer-box.relaxed textarea { min-height: 63px; }
  .prompt-row { display: flex; }
  textarea { width: 100%; max-height: 190px; padding: 2px 0; resize: none; border: 0; outline: 0; background: transparent; color: var(--color-text); caret-color: var(--color-accent); font: 14px/1.5 inherit; }
  /* The prompt line is an invitation, not a label. At the tertiary text colour
     it sat at the same weight as the settings beside it and the box read as
     three competing pieces of writing; quieter, the capsule reads as one thing
     with a waiting cursor in it. */
  textarea::placeholder { color: color-mix(in srgb, var(--color-text-3) 70%, transparent); }
  /* Shut, the message shares its row with the + button and the setting pills,
     so the box is as tall as they are while the text in it is one line. The
     line was landing against the top of that box and reading as floating above
     the controls beside it.

     `align-content` centres the text block inside the box it already has. It is
     not only a flex and grid property any more — it aligns the contents of a
     block container too, which is what a textarea is. Stretching the line height
     to fill the box was the old way of faking this, and it lied about the
     leading: a second line would have inherited it. */
  .composer-box:not(.relaxed) textarea { padding: 0; align-content: center; }
  textarea:disabled { cursor: not-allowed; opacity: .6; }

  /* The footer is only a bracket around the two control groups: it hands them
     to the grid above so each can take its own cell in either shape. */
  .composer-footer { display: contents; }
  .footer-left, .footer-right { display: flex; min-width: 0; align-items: center; gap: var(--composer-row-gap); }
  .footer-left { grid-area: lead; }
  /* The right of the row is one cluster, not four separate controls: the pills
     already carry their own padding, so the gap between them only has to keep
     them from touching. The meter is the one thing there that is read rather
     than pressed, so it keeps its own space ahead of the group. */
  .footer-right { grid-area: trail; justify-self: end; gap: 2px; }
  .file-input { position: absolute; width: 1px; height: 1px; overflow: hidden; opacity: 0; pointer-events: none; }
  .leading-controls { display: flex; width: min(820px, calc(100% - 44px)); min-width: 0; margin: 0 auto var(--composer-row-gap); align-items: center; gap: var(--composer-row-gap); }
  .wide-controls { display: flex; min-width: 0; }
  .compact-controls { display: none; }

  /* Every round control on the row — add, mic, send — is the same capsule. */
  .round-control { display: inline-grid; place-items: center; flex: none; width: var(--composer-control-size); height: var(--composer-control-size); padding: 0; border: 0; border-radius: var(--radius-pill); background: var(--composer-pill-surface); color: var(--composer-pill-text); cursor: pointer; }
  .round-control:hover:not(:disabled) { background: var(--composer-pill-surface-hover); color: var(--composer-pill-text-open); }
  .round-control:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: 2px; }
  .add-control.open { background: var(--composer-pill-surface-open); color: var(--composer-pill-text-open); }
  .mic:disabled { cursor: default; opacity: .55; }
  .send { background: var(--color-accent); color: var(--color-on-accent); box-shadow: var(--shadow-sm); }
  .send:hover:not(:disabled) { background: var(--color-accent); color: var(--color-on-accent); filter: brightness(1.06); transform: translateY(-1px); }
  .send.stop { background: var(--color-bad); }
  .send.stop:hover:not(:disabled) { background: var(--color-bad); }

  /* The ring is the size of the icons beside it and drawn in the same quiet
     colour, warming as the window empties so the last stretch is noticed
     without being read. `pathLength` makes the circle 100 units round, so the
     dash is the remaining percentage itself — the ring empties as the window
     fills, agreeing with the "N% left" text beside it. The margin is the space
     the meter keeps ahead of the control group beside it. */
  .context-ring { display: grid; place-items: center; flex: none; margin-inline-end: 6px; width: var(--composer-control-size); height: var(--composer-control-size); color: var(--color-text-2); }
  .context-ring svg { width: 18px; height: 18px; transform: rotate(-90deg); }
  .context-ring circle { fill: none; stroke-width: 2.2; }
  .ring-track { stroke: color-mix(in srgb, currentColor 22%, transparent); }
  .ring-fill { stroke: currentColor; stroke-linecap: round; stroke-dasharray: var(--arc) 100; }
  /* The popup the ring opens on hover. It says the numbers the ring can only
     gesture at, in the shell's own type sizes rather than the tooltip's. */
  .context-popover { display: flex; flex-direction: column; gap: var(--space-1); padding: var(--space-2) var(--space-3); text-align: left; }
  .context-popover-headline { color: var(--color-text); font-size: 13px; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .context-popover-row { display: flex; justify-content: space-between; gap: var(--space-4); white-space: nowrap; }
  .context-popover-label { color: var(--color-text-3); font-size: 12px; }
  .context-popover-value { color: var(--color-text-2); font-size: 12px; font-variant-numeric: tabular-nums; }
  .context-ring-warm { color: var(--color-attention); }
  .context-ring-hot { color: var(--color-bad); }
  .menu-row-copy { display: flex; min-width: 0; flex-direction: column; gap: var(--menu-row-description-gap); }
  .menu-row-description { color: var(--secondary-label); font-size: 13px; line-height: 1.4; white-space: normal; }

  .attachments { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 10px; }
  .attachments figure { display: grid; grid-template-columns: 60px minmax(0, 1fr); column-gap: 8px; min-width: 210px; max-width: 280px; margin: 0; padding: var(--composer-attachment-inset); border: 1px solid var(--composer-attachment-border); border-radius: var(--radius-md); background: var(--composer-attachment-surface); }
  .attachment-preview { position: relative; grid-row: span 2; width: 60px; height: 52px; }
  .attachment-remove { position: absolute; top: 3px; right: 3px; display: grid; place-items: center; width: 20px; height: 20px; border: 0; border-radius: var(--radius-pill); background: var(--composer-attachment-scrim); color: var(--color-text); cursor: pointer; }
  .attachment-remove:hover { background: var(--color-bad-bg); color: var(--color-bad); }
  .attachment-remove:focus-visible, .attachment-annotate:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: 1px; }
  .attachments figcaption { display: grid; align-content: center; min-width: 0; gap: 2px; }
  .attachments figcaption strong { overflow: hidden; color: var(--color-text); font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .attachments figcaption small { color: var(--color-text-2); font-size: 12px; }
  .attachment-annotate { grid-column: 2; justify-self: start; border: 0; background: transparent; color: var(--color-accent); padding: 1px 0; font-size: 12px; cursor: pointer; }
  .drop-hint { grid-area: hint; margin: 0; padding: var(--composer-prompt-inset); color: var(--color-accent); font-size: 13px; }
  /* Two things nobody needs to read twice: how to attach a file, and a number
     the ring above already draws. It stays because it is where a newcomer finds
     the slash menu, but at the weight of the message it was competing with the
     one line in the box that matters. */
  .composer-hint { display: flex; justify-content: flex-end; gap: 8px; width: min(820px, calc(100% - 44px)); margin: 5px auto 0; color: color-mix(in srgb, var(--color-text-3) 78%, transparent); font-size: 12px; }
  /* The settings row is a fixed 211px — approval on the left, the model pill on
     the right — and the add button beside it needs about 80px more. Measured in
     the browser, the pair still sits unclipped in a 438px composer, so
     collapsing them into the overflow menu at 680px hid controls that had room
     to spare in any window narrower than about 1100px. Collapse only when they
     genuinely stop fitting. */
  @container composer (max-width: 440px) { .wide-controls { display: none; } .compact-controls { display: flex; } }

  /* Short, one-shot transitions. The box eases between its two corners as the
     message grows, and the message box itself eases its own height; the send
     button pops in the moment there is something to send. */
  @keyframes send-pop { from { transform: scale(.6); opacity: 0; } to { transform: scale(1); opacity: 1; } }
  @media (prefers-reduced-motion: no-preference) {
    .composer-box { transition: border-color .14s ease, border-radius .18s ease, background-color .14s ease, box-shadow .14s ease; }
    .ring-fill { transition: stroke-dasharray .4s ease; }
    .context-ring { transition: color .3s ease; }
    /* Commented out while we chase a UI freeze. Height is a layout property, so
       animating it makes the ResizeObserver on the composer fire every frame for
       160ms; each fire republishes `--composer-height`, which is the transcript's
       bottom padding, so the scroll box and every mounted turn re-measure with
       it. Snapping settles in one frame instead of ten. */
    /* textarea { transition: height .16s ease; } */
    .round-control { transition: background-color .14s ease, color .14s ease, filter .14s ease, transform .14s ease; }
    .send { animation: send-pop .16s ease-out; }
  }
</style>
