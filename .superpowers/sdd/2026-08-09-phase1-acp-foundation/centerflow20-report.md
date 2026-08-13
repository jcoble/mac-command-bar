# Lane centerflow20 report

Status: implementation complete; source and script verification passed. The real app was not launched, per lane constraint.

## Root cause

Verified from the pre-change `ConversationTimeline.svelte` source and the final diff:

- The scroll owner used `height:100%` with 236px of vertical padding but no border-box sizing. Its actual box extended beyond the flex viewport and could be clipped by the structured conversation surface. The corrected scroll owner is `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:232-234`.
- Every revision of the last streaming item called `jumpToLatest()` while the viewport was within 80px of the bottom (pre-change `ConversationTimeline.svelte:61-66`). That bottom chase could immediately overwrite wheel/trackpad movement.
- The transcript sliced variable-height markdown/tool rows using one fixed 96px estimate and synthetic spacers (pre-change `ConversationTimeline.svelte:38-41,79-85`). The estimate could not represent real turn heights, so the scroll range and rendered window diverged.

The old revision-driven bottom chase and fixed-height spacer window are deleted. Rows now remain in one DOM list and use browser-native offscreen containment at `tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte:43-48`.

## Behavior contract

- A submitted user turn anchors at the transcript viewport top.
- Streaming assistant growth does not move that anchor.
- Direct user scrolling cancels programmatic movement immediately.
- Jump to latest explicitly pins the transcript bottom; later stream growth follows only that explicit pin.
- Reduced motion uses an instant jump.

Verified implementation:

- Send records the previous user-row identity and emits a session-scoped request at `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:167-180`; the request is wired at `ConversationSurface.svelte:340-345`.
- The pure state/action contract is at `tauri-svelte-preview/src/lib/shell/conversation/conversationScrollAnchor.ts:59-122`.
- The timeline resolves the next user row once at `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:148-167`; stream growth is separately inert unless explicitly pinned at `ConversationTimeline.svelte:169-178`.
- Smooth movement is a bounded 180ms `requestAnimationFrame` operation at `ConversationTimeline.svelte:65-108`. Wheel, touch, and scroll-key input cancel it at `ConversationTimeline.svelte:186-209`. Reduced motion is read at `ConversationTimeline.svelte:56-58`.
- End-of-list anchoring works with all semantic rows present and browser-native offscreen containment; the target is remeasured after skipped content is revealed at `ConversationTimeline.svelte:70-130`.

## Files touched

- `.superpowers/sdd/2026-08-09-phase1-acp-foundation/centerflow20-report.md`
- `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte`
- `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte`
- `tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte`
- `tauri-svelte-preview/src/lib/shell/conversation/conversationScrollAnchor.ts`
- `tauri-svelte-preview/src/lib/shell/conversation/conversationTimeline.ts` (deleted obsolete scroll helpers)
- `tauri-svelte-preview/scripts/conversationScrollAnchor.test.ts`
- `tauri-svelte-preview/scripts/conversationTimeline.test.ts` (renamed from `.mjs` because it was touched)

## Verification receipts

Verified from a fresh run in `tauri-svelte-preview`:

```text
$ node --experimental-strip-types scripts/conversationScrollAnchor.test.ts
conversationScrollAnchor.test.ts passed

$ node --experimental-strip-types scripts/conversationTimeline.test.ts
conversationTimeline.test.ts passed

$ node --experimental-strip-types scripts/conversationSendRecovery.test.ts
conversationSendRecovery.test.ts passed

$ pnpm run check:svelte
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.

$ git diff --check
(exit 0, no output)
```

The decision cases are asserted at `tauri-svelte-preview/scripts/conversationScrollAnchor.test.ts:7-43`: send anchors the next user row, stream growth is inert after send, user input cancels smooth movement, Jump to latest pins/follows the bottom, and reduced motion is instant.

Assumed/not runtime-verified: native WebView wheel/trackpad behavior. The lane explicitly prohibited launching the real app; no app or browser process was started and port 5177 was not bound.
