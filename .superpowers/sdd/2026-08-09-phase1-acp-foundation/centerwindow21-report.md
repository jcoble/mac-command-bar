# centerwindow21 — bounded transcript render window

## Outcome

Verified. A 2,000-display-item stored transcript now materializes 120 timeline rows initially, exposes one explicit kit-button disclosure row, and adds 120 earlier rows per click without moving the reader's prior first row. Root and child transcripts share the same `ConversationTimeline` code path.

## Implementation receipts

- `tauri-svelte-preview/src/lib/shell/conversation/conversationTimeline.ts:85-136` — `CONVERSATION_RENDER_WINDOW = 120`, identity-preserving `slice`, conversation reset, disclosure anchor, and 120-item backward extension.
- `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:58-79` — local disclosure state and reset keyed by the active root/child transcript.
- `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:147-167` — measure the previously first row before prepend and restore its viewport offset after `tick()`, instantly.
- `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:269-283` — only windowed items enter the keyed DOM loop; the single disclosure row uses the kit `Button` with `ghost` variant.
- `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:351-359` — root and selected-child transcript identities feed the same component window.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationScrollAnchor.ts` — unchanged (`git diff --exit-code -- <file>` exited 0).

## DOM and browser receipt

The controlled before run used the same 2,000-item mock and same harness with the render-window constant temporarily raised above the fixture size. The constant was restored to 120 before the after run and final checks.

| 2,000-display-item transcript | Before | After |
| --- | ---: | ---: |
| Timeline DOM rows | 2,000 | 120 |
| Disclosure rows | 0 | 1 |
| Rows after one disclosure click | n/a | 240 |
| Prior-first-row viewport delta after prepend | n/a | -0.5 px |
| Verified viewport | 1710×990 | 1710×990 |

Selector: `[data-testid="conversation-timeline-item"]`.

Screenshot: `.superpowers/sdd/2026-08-09-phase1-acp-foundation/centerwindow21-show-earlier.png` (visually inspected; the row reads `Show earlier — 1,880 more`).

Browser command: `node --experimental-strip-types scripts/perfHarness.ts`, Vite strict port 5181.

## Performance harness before/after

| Metric | Unbounded before | Bounded after | Result |
| --- | ---: | ---: | --- |
| Click to transcript interactive | 399.1 ms | 229.3 ms | 169.8 ms faster (42.5%) |
| Snapshot-load long tasks >50 ms | 1 × 184 ms | 1 × 82 ms | 102 ms shorter (55.4%) |
| Idle long tasks over 15 s | 0 | 0 | held |
| Animations at rest | 0 | 0 | held |
| Post-load row-hover highlight p95 | 32.0 ms | 33.0 ms | +1.0 ms run-to-run variance; below the 100 ms harness gate |
| Post-load hover-actions-visible p95 | 131.7 ms | 132.9 ms | +1.2 ms run-to-run variance |

Final after-run tables:

```text
Click to transcript interactive
events  latency ms
2,000   229.3

Transcript DOM count
events  rows  show-earlier rows
2,000   120   1

Show-earlier scroll anchor
rows after click  anchored item  offset delta px
240               user-1880     -0.5

Snapshot-load long tasks >50ms
start   duration  name
5967.1  82        self

Idle long tasks >50ms (15s)
start  duration  name
-      0         none

Animation census at rest
animations
0
```

The timer census held the same three pre-existing interval callsites in both runs: one visible presence tick fired 10 times during the 10-second window; the Vite connection and session-library intervals fired 0 times.

## Tests

Fail-first receipt:

```text
SyntaxError: conversationTimeline.ts does not provide an export named 'CONVERSATION_RENDER_WINDOW'
```

Final command:

```text
pnpm run check:svelte && pnpm test:menu-open-sweep && pnpm test:session-rail && pnpm test:agent-conversation-store && node --experimental-strip-types scripts/conversationTimeline.test.ts && node --experimental-strip-types scripts/conversationTimelineIdentity.test.ts && node --experimental-strip-types scripts/conversationRenderWindow.test.ts
```

Final receipts:

```text
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
menu open sweep tests passed
sessionRowActions: ok
ownedSessions tests passed
centerDock: ... verified
myWorkViewOptions: ... passed
tests 23; pass 23; fail 0
railElapsedTicker: ok
paneLayout: all tests passed
agent conversation store tests passed
conversationTimeline.test.ts passed
conversation timeline identity tests passed
conversation render window tests passed
```

`tauri-svelte-preview/scripts/conversationRenderWindow.test.ts:8-36` covers 2,000→120, +120 disclosure, projection-object identity, live append retaining disclosed objects, and reset on conversation switch.

## Cleanup and repository state

```text
Browser cleanup: stopped perfharness20-chrome (Chrome helper tree exited)
Browser cleanup: stopped perfharness20-vite (5181 process tree exited)
```

No commit, staging, or push was performed. The pre-existing untracked `tauri-svelte-preview/node_modules` path was preserved.
