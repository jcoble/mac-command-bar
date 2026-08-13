# Center transcript rebuild — T3-grade, two lanes

Owner call 2026-08-13: the session conversation surface is super slow and duplicates content.
Rebuild it against the T3 renderer study (`docs/superpowers/specs/2026-08-13-t3-center-ui-study.html`,
T3 commit 1e59b4c) so we implement what T3 verifiably does well instead of chasing symptoms.

## What T3 does well (source-verified — this is the target, do not re-litigate)

1. **Identity is the fast path.** Derived timeline rows keep stable ids and reuse previous row
   objects after per-kind shallow comparison; the list renders from stable identity so only the
   changing tool/timer/markdown subtree updates (T3 MessagesTimeline.tsx:124-159, 549-638,
   .logic.ts:643-713). Nothing does O(transcript) work per streamed delta.
2. **Asymmetry does the labeling.** User prompts are quiet right-side bubbles (80% cap, 16px
   radius, 12px inset); assistant prose is unboxed and full measure; no avatars or role headings;
   one 768px reading plane shared with the composer.
3. **Compression by default.** Consecutive tool work collapses to ONE newest row plus a
   "+N previous tool calls" disclosure that inserts older rows on demand; each row starts
   collapsed with a one-line heading, chevron, and terminal status; expanded output is an 11px
   pre-wrapped region capped at 16rem with no nested "show more".
4. **Streaming text stays in the normal plane.** No completion chrome (copy/timestamp) until the
   message settles. The working clock updates ONE DOM text node, never a component tree.
5. **Code blocks are utility chrome.** Filename/language title, wrap toggle, copy, 12px mono,
   bounded (500-entry/50MB LRU) highlight cache; streaming blocks bypass the cache.
6. **Metadata appears on intent.** Hover/focus reveals per-turn actions; nothing is displayed
   by default under every turn.
7. **One 4px geometry grammar.** 2/4/6/8/10/12/16/20px spacing, radius derived from 10px,
   28px control height, 22px composer dock.

Explicitly NOT copied (conflicts with our zero-idle-work rule): T3's infinite status pulse,
its always-mounted 1s interval, and backdrop blur/saturation. Scroll: we KEEP our reducer
(`conversationScrollAnchor.ts`) — send anchors the user row, streaming is inert unless the
user pins the bottom.

## Our defects being fixed (receipts)

- `ConversationTimeline.svelte:51-55` — `contentRevision` maps and joins EVERY item's text
  lengths into one string on every items change: O(transcript) per streamed delta.
- `conversationStore.svelte.ts:167-295` — every event replaces the whole session object
  (`conversationSessions[id] = {...current, ...}`), invalidating every reader of the session on
  every delta; `agentItems` is re-merged per event.
- `conversationStore.svelte.ts:303-318` — `idempotentSnapshotEvents` only blanks CONSECUTIVE
  identical assistant deltas; replay overlap in any other shape renders twice (owner sees
  duplicated content).
- Transcripts load as a 2,000-event tail and render as one flat keyed list with no offscreen
  strategy; selection of a large session blocks the main thread for seconds and stays slow.

## Lane split

- **Lane A (SOL, behavior — runs first): store + timeline identity, dedup, bounded per-event
  work.** No visual changes. Acceptance is measured: hover/App stays fast with a 2k-event
  session selected; applying one streamed delta touches O(1) rows; a replayed snapshot renders
  every logical item exactly once.
- **Lane B (Opus 5, visual — after A merges): the T3 look.** Bubble/plain asymmetry, 768px
  measure, tool-row compression UI, code-block chrome, hover metadata, 4px geometry — on
  shadcn kit components, opaque surfaces, all animation finite.

Both lanes carry the animation rule (every animation ends; no loops at rest), TS-only,
plain-English naming, and the perf gates (`check:svelte` 0/0, script tests, perf harness once
merged).
