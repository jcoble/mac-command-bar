# Performance harness 20 report

## Usage

1. `cd tauri-svelte-preview` from the repository root.
2. Run `node --experimental-strip-types scripts/perfHarness.ts` (Vite uses strict port 5181).
3. Read the printed measurement tables; a threshold failure exits nonzero.

## Stall root cause

Verified in the browser harness with one selected 2,000-event snapshot. `applyAgentConversationSnapshot`
published the half-built state before replaying its events, so every replay mutation crossed Svelte's
deep reactive proxy. In the same loop, `mergeAgentItem` linearly searched and copied the growing typed
item array and `appendRecentEvent` copied its capped array for every event. The CDP CPU profile ranked
`mergeAgentItem` first (113 samples), then Svelte proxy `get`/`tag_proxy`, `appendRecentEvent` (60), and
`finishReasoningItems` (49). That produced a 2,214 ms main-thread long task.

The row CSS and hover-card snapshot were not the blocker: hover-highlight p95 was 31.7 ms before the
snapshot and 31.2 ms after it at HEAD.

## Fixes and before/after measurements

- `conversationStore.svelte.ts`: rebuilds the complete snapshot in an unpublished plain object,
  creates the bounded 200-event inspector tail once, then publishes the finished state once.
- `conversationTimeline.ts`: snapshot/batch replay uses an item-id index and in-place updates on its
  private array. Live event updates keep the existing immutable `mergeAgentItem` contract.
- `agentConversationStore.test.ts`: covers a 2,000-event replay, 400 legacy items, 400 typed items,
  the 200-event cap, and idempotent second load.
- Production backend seam changes: none. The Tauri mock exists only under `scripts/`.

| Measurement | HEAD before | Fixed after |
|---|---:|---:|
| Click to 2k-event transcript interactive | 2,639.4 ms | 323.9 ms |
| Largest snapshot long task | 2,214 ms | 83 ms |
| Hover highlight p95 before snapshot | 31.7 ms | 32.4 ms |
| Hover highlight p95 after snapshot | 31.2 ms | 32.3 ms |
| Hover buttons p95 after snapshot | 134.5 ms | 132.5 ms |
| Idle long tasks, 15 seconds | 0 | 0 |
| Animations at rest | 0 | 0 |

The before/after browser runs used the same strict port 5181 and viewport 1710x990. Because another
active lane owned IPv4 `127.0.0.1:5181`, these diagnostic runs bound IPv6 loopback `::1:5181`; the
permanent harness has been restored to the required `127.0.0.1:5181`. Exact-host final run pending
release of the other lane's process.

Verification completed:

- `pnpm run check:svelte` -> `Files the /next shell owns: 0 error(s), 0 warning(s).`
- `pnpm test:agent-conversation-store` -> passed.
- `node --experimental-strip-types scripts/conversationTimeline.test.ts` -> passed.
- Harness and mock TypeScript syntax checks -> passed.

## Cleanup

Browser cleanup: stopped perfharness20-chrome (Chrome helper tree exited)

Browser cleanup: stopped perfharness20-vite (5181 process tree exited)

Exact-host final run remains pending; no perfharness20 process is left running.
