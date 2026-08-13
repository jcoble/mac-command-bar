# centeritems21 — T3 visual pass on transcript item components

Branch `lane/centeritems21`, worktree `/Users/blackcolours/dev/work/worktrees/mac-command-bar/centeritems21`.
Visual only: no store reads added, no timeline/container files touched. Nothing committed or staged.

Every claim below is labelled **verified** (command output or a file:line I wrote) or **assumed**.

## What changed, per file

All paths under `tauri-svelte-preview/src/lib/shell/components/conversation/`.

### Asymmetry replaces role chrome

- **ConversationMessage.svelte** — removed the `label` prop and the `.turn-label` heading
  element entirely; the `conversation-message-label` testid no longer exists. The user bubble is
  now `max-width:80%; margin-left:auto; padding:12px; border-radius:16px` with a flat tinted
  surface and no border (`ConversationMessage.svelte:74`); the assistant article is unboxed at
  `max-width:768px` (`ConversationMessage.svelte:52`). Inline code radius 5px → 6px to join the
  radius family (`ConversationMessage.svelte:70`). **verified** (file contents).
- **UserMessageItem.svelte:8-11** / **AssistantMessageItem.svelte:9-13** — wrapper now carries
  `class="group"` and renders `TurnMetadata` beneath the turn. The assistant metadata is gated on
  `item.completed`, so streaming text has no completion chrome. **verified**.

### New: turn metadata on intent

- **TurnMetadata.svelte** (new, 71 lines) — time plus one copy action under a settled turn, at
  zero opacity until `group-hover`/`group-focus-within`. Built on the kit `HoverActions` +
  `HoverActionButton` (bare icon button, per-button hover color, no backdrop pill). The row is
  always in the page, so revealing it shifts nothing. No token counts.
  Measured mid-hover: the hovered turn's time reported `opacity: "1"` while the other turn's
  reported `"0"`. **verified** (playwright eval over both `conversation-turn-time` nodes).
  It uses one self-clearing `setTimeout` for the 1.4s "Copied" state, cleared on destroy
  (`TurnMetadata.svelte:34-45`) — the same one-shot pattern `CodeBlock.svelte` already used. This
  is a deliberate reading of the "do not add timers" rule as being about idle/recurring timers;
  flag it if you disagree, it is 4 lines to change.

### Tool rows compress

- **ToolItem.svelte** — a collapsed row is now a bare 28px row (`border:1px solid transparent`,
  no fill); the box and background appear only on `[open]` (`ToolItem.svelte:89-90`), so a run of
  tool calls no longer reads as a stack of cards. Summary line is chevron, kind icon, mono title,
  one-line preview, terminal glyph. **verified** by screenshot 01 (four consecutive rows, one line
  each).
- Status is now a glyph, not a labelled chip: check / X / minus / dot for completed, failed,
  queued, running (`ToolItem.svelte:67-72`), colored good/bad/accent (`:104-107`).
- The running **spinner was removed** (previously `animation:spin .85s linear infinite`). A
  running tool is a static accent `CircleDot`. This follows the study's own "use a static mark,
  not T3's infinite pulse" and is what makes the at-rest census reachable. **verified**.
- Expanded output is one bounded region: `max-height:16rem; overflow:auto; white-space:pre-wrap`,
  12px mono, **no nested "show more"** (`ToolItem.svelte:112-113`). Measured computed
  `max-height: "224px"` — 16rem against this app's 14px root size — and
  `timeline-tool-output-toggle` absent from the DOM. **verified** (playwright eval).
- The body no longer repeats the summary that is already the collapsed preview line; only real
  payload (output or a diff) makes a row expandable (`ToolItem.svelte:21-23`), so a summary-only
  row shows no chevron. **verified** by screenshot 04.

### Code block chrome

- **CodeBlock.svelte** — header is now a 28px bar: language/filename name at 12px mono taking the
  width, then a **wrap toggle** and the copy button with its confirmation
  (`CodeBlock.svelte:80-95`). Body stays 12px mono, horizontal scroll by default;
  `pre.wrapped` switches to `pre-wrap` (`:108`).
  Toggling reported `aria-pressed` `false → true` with computed `white-space` `pre → pre-wrap`.
  **verified** (playwright eval).
- I initially added filename detection for multi-word fence info strings and **reverted it**:
  `conversationMessageSafety.ts:93` matches `/^\s*(```+|~~~+)\s*([^\s]*)\s*$/`, so an info string
  can never contain a space and the code was unreachable. See follow-ups. **verified** (that line).

### Everything else in scope

- **ReasoningItem.svelte** — spinner removed, chevron/preview row retuned to the 8px gap and 28px
  row height, radius 9 → 8, label prop drop threaded through.
- **SubagentSection.svelte** — same collapsed-row treatment as ToolItem (transparent border until
  open), spinner replaced with a static `CircleDot`, state chip is now bare colored text rather
  than a pill.
- **CommandItem.svelte** — card chrome removed; command output reads in the plain plane behind a
  quiet caption and a hairline rule.
- **ErrorItem.svelte** — left-border strip → a full 8px-radius bordered block (an error is the one
  thing that should keep its box).
- **ApprovalItem.svelte** — hand-rolled buttons replaced with kit `Button`
  (`default` for allow, `ghost` for deny), padding/radius on the 4px scale.
- **UserInputItem.svelte** — rewritten from one 1,400-character line into readable markup; text
  fields now use the kit `Input`, actions use kit `Button`.
- **TaskListItem.svelte** / **PlanItem.svelte** — same card geometry (12px inset, 10px radius,
  8/12px rhythm), uppercase accent kind-label demoted to quiet 12px.
- **FileChangeItem.svelte** — radius 7 → 8, diff capped at 16rem to match tool output.
- **TimelineItem.svelte:47-58** — work rows (tool, reasoning, subagent, command, file, plan,
  tasks) give back 8px of the container's 16px gap so a run reads as one turn; message turns keep
  the full 16px. Done with per-kind `margin-bottom`, no `:has()` and no sibling selectors.

### Deleted (one-way rule)

- `collapseToolOutput.ts`, `scripts/collapseToolOutput.test.mjs`, and the
  `test:collapse-tool-output` entry in `package.json` — the nested "show more" they served is gone.
- `SubagentItem.svelte` — dead file, no importer (`grep -rn "SubagentItem" src scripts` returned
  nothing). **verified**.

## The one place I diverged from the spec

The brief and the T3 study both call for **11px** metadata and tool output. The repo gate rejects
it: `scripts/checkSvelteNext.mjs:132-174` fails the build on any `font-size` under 12px in owned
files, and it flagged twelve of my declarations. I raised everything to **12px** rather than slip
under the check by writing the size in a `font:` shorthand, which the regex does not read. If 11px
is genuinely wanted here, the gate has to change first — that is a controller call, not mine.

## Follow-ups for the container lane (files I was not allowed to touch)

1. **Cross-row grouping.** Each tool row now collapses individually. "One newest row plus
   *+N previous tool calls*" needs `ConversationTimeline.svelte`, which I could not edit.
2. **`assistantLabel` is dead.** Nothing renders a role heading any more. I left the prop accepted
   on `TimelineItem.svelte:24-27` with a comment purely so the timeline that still passes it keeps
   type-checking. Drop it from both files in the container pass.
3. **The 768px measure** is asserted per-article in `ConversationMessage.svelte`; the reading plane
   itself belongs to the container.
4. **Fence info strings are single-token**, so a code block can only ever be named by its language,
   never by a filename. Giving code blocks file identity means changing the fence regex at
   `conversationMessageSafety.ts:93` — a parser change, out of a visual lane's scope.
5. **Turn metadata for streaming assistant turns** appears the moment `completed` flips, which adds
   a 24px row. If that shift is unwanted, the container should reserve the row.

## Gates

```
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```
`pnpm run check:svelte`, run last **after** the fixture route was deleted. **verified**.

Script tests — no test in `scripts/` pins any testid I touched
(`grep -rl` over the changed testids returned nothing). Related suites all pass: **verified**
```
conversationTimeline.test.ts: PASS
conversationTimelineIdentity.test.ts: PASS
conversationMessageSafety.test.mjs: PASS
codeHighlight.test.mjs: PASS
contrastTokens.test.mjs: PASS
```

`nextTokens.test.mjs` **FAILS, and it fails identically on the untouched baseline** — I stashed my
whole diff and re-ran it to confirm. The extra tokens it rejects are
`--floating-content-inset`, `--floating-row-gap`, `--rail-content-inset`,
`--rail-row-content-inset`, `--row-selected`, `--secondary-label`, none of which I added. Someone
else's lane owns that. **verified** (`git stash` → run → `git stash pop`).

## Animation census

```
{ "vp": "1710x990", "anims": 0 }
```
Zero at rest, re-confirmed after expanding a tool row and after toggling wrap. **verified**.

Getting there required removing the `body-in` entrance keyframes from ToolItem, SubagentSection and
ReasoningItem. The first census read **4**, all finished `body-in` CSSAnimations on `.tool-body`
elements inside *collapsed* `<details>` — Chrome keeps a finished CSS animation in
`getAnimations()` while its `animation-name` still applies, and closed `<details>` children still
compute style. Dropping `animation-fill-mode` was not enough; the declaration had to go. The
remaining motion is `transition` only (chevron rotate, row background), which leaves the census as
soon as it finishes. **verified**.

## Screenshots

Viewport **set and verified at 1710x990** before every capture
(`eval "() => window.innerWidth + 'x' + window.innerHeight"` → `"1710x990"`, re-verified after the
vite restart). Under `tauri-svelte-preview/output/playwright/centeritems21/`:

- `01-transcript-collapsed.png` — user bubble right, unboxed assistant prose, highlighted code
  block with header, four collapsed tool rows with check/minus/X/dot glyphs.
- `02-tool-expanded-and-hover-meta.png` — the failed tool row open with its bounded scrolling
  output, and the hovered user turn showing its revealed time and copy action.
- `03-code-block-header.png` — the code block header close up: `ts`, Wrap, Copy.
- `04-tool-row-collapsed.png` — one hovered collapsed row (no chevron: summary only, nothing to
  open).

These were rendered through a throwaway fixture route, `src/routes/next-item-check/+page.svelte`,
which mounted real `TimelineItem` instances over fixture display items — the /next shell itself
shows "No session selected" in a browser because a transcript needs the Tauri backend. **The route
has been deleted** and `check:svelte` re-run afterwards; `git status` shows no stray route.
**verified**.

## Cleanup

```
Browser cleanup: stopped centeritems21 (daemon + Chrome helper tree)
Vite cleanup: stopped 127.0.0.1:5183 (lsof -ti :5183 → none)
```
Both confirmed empty by `ps aux | grep -i chrom | grep centeritems21` → none and
`lsof -ti :5183` → none. The worktree stays — the controller owns reviewing and committing it.
**verified**.
