# Lane railv320 — session rail v3

Worktree `/Users/blackcolours/dev/work/worktrees/mac-command-bar/railv320`, branch `lane/railv320`.
Nothing was committed, staged, or pushed. All paths below are relative to `tauri-svelte-preview/`
unless stated otherwise. Every claim carries a receipt and is marked **verified** (I ran or read it
in this worktree) or **assumed**.

Built from the approved mockup `docs/superpowers/specs/2026-08-13-session-rail-v3-mockup.html`
with every owner amendment folded in: hover buttons respond one by one with no backdrop pill and
now live in the kit as the app-wide pattern; the row's time never disappears and coarsens as it
ages; and the model chip in the row is replaced by the provider glyph, with the model name moved
to the hover card.

## What the rail is now

One scroll area holding sections, not three resizable panes. The three-line row is the whole click
target, hover reveals three bare buttons that overlay the right-side metadata without moving the
title, and right-click opens the kit context menu. Every row carries its age, which stays put when
the work stops and coarsens from seconds to minutes to hours. It answers the pointer — the fill,
the selection bar, the chevron, the hover cluster and the card all move — but every one of those
motions ends. The only loop is one spinner per genuinely working, on-screen row, and the rail's one
shared clock ticks per second only while a row needs seconds, drops to once a minute otherwise, and
stops when no row is watching.

## Files added

| File | What it is |
| --- | --- |
| `src/lib/components/ui/hover-actions/hover-actions.svelte` | **Kit**: the row-hover action cluster, bare and CSS-revealed |
| `src/lib/components/ui/hover-actions/hover-action-button.svelte` | **Kit**: one button in that cluster, with a `tone` for where it leads |
| `src/lib/components/ui/hover-actions/index.ts` | Exports `HoverActions` and `HoverActionButton` |
| `src/lib/shell/agentIcons.ts` | The one provider glyph map and display name, shared by the strip and the rows |
| `src/lib/shell/components/SessionRail.svelte` | The list: one scroll area, sticky collapsible section headings, keyed rows |
| `src/lib/shell/components/railElapsedTicker.ts` | The rail's one clock, its cadence, and the age reading |
| `src/lib/shell/components/railRowVisibility.ts` | One shared observer telling each row whether it is on screen |
| `src/lib/shell/components/sessionRowMenu.ts` | The right-click roster and which items have a command behind them |
| `scripts/railElapsedTicker.test.ts` | The ticker lifecycle, cadence, and format test |

## Files changed

| File | Change |
| --- | --- |
| `src/lib/shell/components/WorktreeAgentRow.svelte` | Rebuilt as the v3 three-line row with hover actions and the context menu |
| `src/lib/shell/components/SessionsColumn.svelte` | Hosts `SessionRail`; header matches the mockup; four dead props removed |
| `src/lib/shell/components/SessionHoverCard.svelte` | Restyled onto the same surface as the row menu; gained an error line and an agent-and-model line |
| `src/lib/components/ui/DESIGN.md` | Documents the hover-action pattern and adds it to the job table |
| `src/lib/shell/ownedSessions.ts` | Carries the stored record's start stamp through as `startedAtMs` |
| `src/lib/shell/conversation/sessionPresence.ts` | Deleted the unused always-on `sessionPresenceNow` clock |
| `src/lib/shell/styles/nextTokens.css` | Row inset retuned to the mockup; two orphaned tokens deleted |
| `src/routes/next/+page.svelte` | Stopped passing the four props the column no longer takes |
| `scripts/sessionRowActions.test.ts` | Rewrote the assertions whose behavior moved; added menu and motion checks |
| `scripts/errorPresentation.test.mjs` → `.ts` | Renamed per the TypeScript rule; split the row and card expectations |
| `package.json` | `test:session-rail` points at the renamed test and runs the new ticker test |

## Files deleted (the old rail, removed in the same wave)

`SessionsPaneview.svelte`, `WorkingPane.svelte`, `DonePane.svelte`, `SettledPane.svelte`,
`MyWorkSessionList.svelte`. **Verified** — `git status --short` lists all five as deleted, and
nothing references them: `grep -rn "SessionsPaneview\|MyWorkSessionList\|WorkingPane\|DonePane\|SettledPane" scripts src package.json docs` returns nothing.

The splitter machinery is gone from the rail: `grep -rn "paneStack" src/lib/shell/components/`
returns only `ShellSidebar.svelte:43`, which is the right-hand tool column and not part of this
lane. **Verified.**

## Receipts

**The row (`src/lib/shell/components/WorktreeAgentRow.svelte`)** — all **verified** by reading the
file at these lines:

- Three lines in the mockup's order: project, status and age (`:402-435`), title and provider glyph
  (`:437-444`), branch (`:446-449`).
- The whole row is one button, edge to edge: `:397` `class="session-row"`, styled at `:549-564`
  with `min-height: 75px` and `padding: var(--rail-row-content-inset)`.
- Selected is a quiet fill plus a 2px accent bar that sweeps in: `:568-584`.
- Status by state: amber permission badge `:407-409`, error summary `:411-416`, dim static
  "Suspended" or "Stopped" `:417-419`, and the age at `:423-435` outside all of them.
- Hover actions are the kit cluster (`:476-518`), positioned by the row and nothing more; the
  metadata under it fades rather than moves (`:713-720`), so the title and branch never shift.
- Rows stay cheap offscreen: `content-visibility: auto` and `contain-intrinsic-size: auto 75px`
  at `:545-546`.
- 13px floor holds: `grep -n "font-size: 1[0-2]" WorktreeAgentRow.svelte SessionRail.svelte SessionHoverCard.svelte` returns nothing. **Verified.**
- No raw hex in any file this lane wrote or changed under `src/lib/shell/components/`:
  `grep -rn "#[0-9a-fA-F]\{3,8\}\b"` over the seven files returns nothing. **Verified.**

**The kit hover-action pattern** — **verified**:

- `hover-actions.svelte:38-49` is the cluster: `opacity-0 pointer-events-none` at rest,
  `group-hover:`/`group-focus-within:` to reveal, and no background, ring, shadow or radius of its
  own. The row supplies the trigger by carrying `class="row group"` (`WorktreeAgentRow.svelte:384`)
  and says only where the cluster sits (`:476-480`).
- `hover-action-button.svelte:57-71` wraps the kit `IconButton`, so a hover action still owes its
  label as tooltip and accessible name, and `:49-55` holds the four tones. `tone` names the kind of
  destination — `primary` for the session, `info` for an editor surface, `success` for source
  control — which is what the rail passes at `WorktreeAgentRow.svelte:483-515`.
- Documented for the next surface to adopt: `src/lib/components/ui/DESIGN.md`, the "Row actions on
  hover" paragraph plus a row in the job table. The two tones that read shell status tokens are
  called out there as the deliberate exception.

**The age, and the motion rules** — **verified**:

- The age is its own element outside every status branch (`:423-435`), so it is on the row whether
  or not anything is working. It counts from `session.startedAtMs` (`:171-178`), which
  `ownedSessions.ts:314` now carries off the stored record's `createdAtMs`; a session with no
  recorded start falls back to its last activity stamp, and a row with neither shows no age.
- The reading coarsens with age — `38s`, then `11m`, then `2h` — in `formatRailElapsed`
  (`railElapsedTicker.ts:97-106`), pinned by `scripts/railElapsedTicker.test.ts:89-99`.
- The spinner only animates through the `spinning` class (`:641`), bound to
  `spinning = isWorking && onScreen` (`:164`, applied at `:431`). A row that is not working, or is
  scrolled out of the list, has no animation at all. Reduced motion switches it off (`:644`).
- One interval for the whole rail, and it runs no faster than the rows need: the row subscribes
  only while it is on screen and has an age (`:193-202`), asking for `second` while it is working
  or under a minute old and `minute` after that (`railElapsedCadenceFor`, `railElapsedTicker.ts:87-89`).
  The ticker retunes to the fastest cadence anyone still needs and stops outright when nobody does
  (`:42-49`). The cadence is a two-value derived (`:191`), so the subscription is replaced at the
  one-minute mark rather than on every tick.
- `grep -rn "setInterval"` across `WorktreeAgentRow.svelte`, `SessionRail.svelte`,
  `SessionsColumn.svelte`, `SessionHoverCard.svelte`, `railRowVisibility.ts`, `sessionRowMenu.ts`
  finds none; the only hits are `railElapsedTicker.ts:16` (a type) and `:29` (the one timer).
- The old always-on rail clock `sessionPresenceNow` was deleted from
  `src/lib/shell/conversation/sessionPresence.ts` — it had no readers
  (`grep -rn "sessionPresenceNow" src scripts` returns nothing). **Verified.**
- Two other intervals remain in the shell and are outside this lane, none of them in the rail:
  `resources/BottomBar.svelte:44` (3s resource sample), `sessionLibrary/SessionLibraryWorkspace.svelte:272`
  (60s clock), `conversation/SessionPresenceIndicator.svelte:62` (one instance in the conversation
  header, already gated to a working session). **Verified** by `grep -rn "setInterval" src`.

**The list (`src/lib/shell/components/SessionRail.svelte`)** — **verified**:

- One scroll area for every section: `:55`, styled `:106-123` with `scrollbar-gutter: stable`, a
  thin native scrollbar that stays visible, and its own gutter so no row or heading is ever
  covered.
- Sticky section headings with name and count: `:59-75`, `position: sticky` at `:126`. A shut
  section drops its heading out of sticky so it reads as part of the list (`:154-158`).
- Sections and their order come from the existing view options through `buildMyWorkGroups` (`:43`),
  which is what keeps the Group-by-project and Sort options in the header working. Working, Done
  and Settled are the default because `DEFAULT_MY_WORK_VIEW_OPTIONS.groupBy` is `status`
  (`myWorkViewOptions.ts:26-30`).
- Rows are keyed by `session.ownedId` (`:78`).

**Data sources** — no new invokes were added. The row reads the same backend list record and live
presence it already read, and the age comes from a field that was already on the stored record and
was simply being dropped in projection (`ownedSessions.ts:314`). **Verified** by reading the
imports; `grep -n "invoke(" WorktreeAgentRow.svelte` returns nothing.

## Kit components used

`ContextMenu` (`Root`, `Trigger`, `Content`, `Item`, `Separator`) from
`src/lib/components/ui/context-menu` at `WorktreeAgentRow.svelte:390-472`, and the new
`HoverActions` / `HoverActionButton` from `src/lib/components/ui/hover-actions` at `:476-518` —
which are themselves built on the kit's `IconButton`, so the row no longer styles a button by hand.
`SessionsColumn` keeps its existing kit controls (`IconButton`, `DropdownMenu`, `SegmentedControl`,
`Select`, `Switch`, `Button`, `Tooltip`, `AlertDialog`). **Verified.**

## The context menu, and what is switched off

The roster lives in `sessionRowMenu.ts` and is drawn in row order. Enabled items and their wiring
(`WorktreeAgentRow.svelte:342-353`):

| Item | Wired to |
| --- | --- |
| Archive (Settle) | `onSettle` → `+page.svelte:283` `settleOwnedSession` |
| Copy session id | `navigator.clipboard`, using `nativeSessionId` or the owned id |
| Copy worktree path | `navigator.clipboard`, using the row's canonical worktree |
| Open in editor | `sessionRowJump(ownedId, 'editor')` |
| Delete | `onAskRemove` → the column's existing remove confirmation → `+page.svelte:1147` `removeSession` |

Switched off, with the reason shown as the item's tooltip. Each needs a backend command that does
not exist; none were invented in this lane:

- **Rename** — `// follow-up:` no command writes a session title. `update_agent_conversation_session_meta`
  (`src/lib/tauriSource.ts:1278-1289`) carries model, effort and meta only. **Verified.**
- **Move to project** — `// follow-up:` nothing reassigns a session's project.
- **Continue in new session** — `// follow-up:` no command forks a session into a new one.
- **Reveal in Finder** — `// follow-up:` no shell-open command exists; `grep -n "invoke<" src/lib/tauriSource.ts`
  has no reveal or open-path command. **Verified.**

## Decisions the amendments needed

1. **The hover pattern is kit-level, and named for the sweep.** `HoverActions` (the cluster) and
   `HoverActionButton` (one button, with a `tone`) live in
   `src/lib/components/ui/hover-actions/` and are exported under those names, so the later lane
   sweeping other surfaces imports them rather than copying CSS. The cluster carries no position of
   its own — the host row places it — because rows differ in height and in which metadata is being
   covered.
2. **The reveal is CSS-only, through Tailwind's `group`.** No JavaScript state, no mount on hover:
   the host row carries `class="group"` and the cluster is `opacity-0` until `group-hover` or
   `group-focus-within`. That keeps the perf rule (controls always in the page) inside the kit
   rather than restating it per surface.
3. **The tone names say where the action leads, not which color to use.** `info` and `success` are
   the only place a kit component reads the shell's status tokens directly, since the registry has
   no slot for "a live surface" or "source control". Called out in DESIGN.md beside the other
   documented divergences.
4. **The age needed a field the projection was dropping.** The stored record already had
   `createdAtMs`; `ownedSessionFromBackend` was discarding it, so `OwnedSession` now carries
   `startedAtMs` (`ownedSessions.ts:118-123`, `:314`). No new backend call. A session with no
   recorded start falls back to its last-activity stamp, and one with neither shows no age at all —
   there is nothing truthful to put there, and I did not want to invent a start time.
5. **The clock slows instead of stopping outright.** The amendment allowed either. A row past its
   first minute still needs a tick a minute to roll "59m" over to "1h", so the ticker retunes to
   60s rather than stopping; it stops entirely when no row is watching, which is the case whenever
   the rail is scrolled away or every row is offscreen.
6. **A working row keeps the fast cadence at any age.** Working is the state a person watches, so
   its seconds stay live even when the reading itself is in minutes.
7. **The provider glyph is the existing one, moved to a shared home.** `AGENT_ICONS` was a private
   constant inside `SessionsColumn`; it now lives in `src/lib/shell/agentIcons.ts` and both the
   collapsed strip and the rows read it, so the two can never drift. The name beside it is the
   provider id read back as words, matching how `sessionHistoryViewModel.ts:124` already does it —
   no new hard-coded product names anywhere.
8. **The hover card is now a snapshot, not a live view** — the follow-up the perf lane left in these
   files. What the card shows is read once on the closed-to-open transition into a typed plain-data
   view (`HoverCardView` at `WorktreeAgentRow.svelte:204-227`, taken by `takeCardView` at `:243-259`
   as the card is placed, `:264`), the card renders from that alone
   (`{#if cardPlacement && cardView}` at `:522`, `<SessionHoverCard {...cardView} />` at `:530`),
   and closing drops it (`:293`). Nothing the card draws reads a store while it is open, so a
   transcript event can no longer repaint an open floating surface — the two fields that used to
   couple it to the live conversation, the model and the usage total, are captured values now.
   I implemented the pattern locally rather than importing the wave branch's
   `src/lib/shell/floatingSurface.ts` and its typed snapshot helpers, because this worktree branched
   before that merge and those files are not here. The shape is the same, so the merge should be a
   matter of swapping `takeCardView` onto the shared helper — worth doing in the resolving lane,
   since a local copy of a shared pattern is exactly the split the one-way rule is against.
   Pinned by `scripts/sessionRowActions.test.ts:211-238`, which also fails if any of the four live
   props comes back. **Verified.**
9. **Finite motion added back, now that the rule encourages it.** I had kept the rail deliberately
   still beyond two opacity fades. Under the refined rule the rail now answers the pointer in four
   places, all of them transitions or one-shot keyframes that finish and then cost nothing:
   the row fill on hover (`WorktreeAgentRow.svelte:742`, 140ms `background-color`); the selection
   bar, which is always in the page and sweeps up from the row's top edge on selection rather than
   blinking into existence (`:570-584` with `transform: scaleY()`, 180ms at `:743`); the hover card
   fading in on open (`card-in`, 160ms, `:744`); and in the rail, the section chevron turning a
   quarter turn (`SessionRail.svelte:144` and `:166`) with its rows settling in behind it
   (`section-in`, 160ms, `:168` and `:171-174`). The kit cluster fades and settles in from the right over
   150ms (`hover-actions.svelte:49-54`).
   Three rules held while adding them: nothing loops except the working spinner, which is unchanged
   and still pauses offscreen; nothing animates a layout property, so opening a section is a settle
   rather than a height animation and no row is ever laid out twice; and every rule sits inside
   `prefers-reduced-motion: no-preference` or a `motion-safe:` variant.
   `scripts/sessionRowActions.test.ts:414-478` pins all three — it counts the loops in the row and
   fails if there is ever more than the one, rejects a `transition` on any layout property in these
   files, and checks each animation is actually present. **Verified.**
   Two notes for the merge. The context menu already animated (the kit's shared
   `context-menu-content` fades and zooms at `duration-100`); I left that alone rather than
   retuning a class that git panes and the source-control menu also use — worth a look in the
   sweep lane if 100ms reads too quick beside the rail's 150-160ms. And my `AGENTS.md` still
   carries the old wording, since the refined rule landed on the wave branch after I forked; I did
   not edit it here, to keep the merge clean.
10. **The model moved to the hover card as agent-plus-model on one line**, the agent in words and
   the model as a quiet chip beside it (`SessionHoverCard.svelte:70-86`). The row's glyph carries
   an `aria-label` with the provider name, so nothing is lost to a screen reader.

## Deviations from the mockup and the spec, for your call

1. **Three lifecycle items were added to the menu.** The approved roster has Archive (Settle) only,
   but the old row also offered Mark done, Move back to Working, and Move back to Done. Dropping
   them would have made Working → Done impossible and Settled a one-way trip, so the roster shows
   exactly one of them per row depending on where the session sits: Mark done on a working row,
   Move back to Working on a done row, Move back to Done on a settled row (`sessionRowMenu.ts:58-69`).
   Say the word and I will cut them back to the mockup's list.
2. **"Close terminal" has no rail affordance any more.** It lived in the old expanded row detail,
   which v3 replaces. Delete still closes the terminal on its way out (`+page.svelte:1148`), and the
   page keeps `closeTerminal` for other callers. Not in the approved roster, so I did not add it.
   Follow-up if you want it back.
3. **A failed session now shows its error summary as the row's status text**, with the technical
   detail as that element's tooltip and the summary repeated on the hover card
   (`WorktreeAgentRow.svelte:411-416`, `SessionHoverCard.svelte:91-96`). The old row had a
   `<details>` disclosure inside its expanded area, which v3 does not have. `SessionCard.svelte`
   still keeps the full disclosure, and the test now checks each surface for what it actually does.
4. **The rail's own reading is its own formatter.** `formatRailElapsed` prints "38s" / "11m" / "2h"
   per the amendment; `formatPresenceElapsed` in `sessionPresence.ts:137` prints "01:05" for the
   conversation header, which is a live turn timer rather than a session age. Two readings, two
   surfaces — not a duplicated mechanism, but worth knowing about.
5. **`--rail-row-content-inset` changed value**, from `8px 10px 9px 12px` to the mockup's
   `5px 11px 5px 13px` (`nextTokens.css:71`). The row was its only reader. `--row-hover` and
   `--row-active` lost their only reader when the old row went, so I deleted them.
6. **A session started in the app before this change has no start stamp until it round-trips.**
   `startedAtMs` comes off the stored record, so an already-running session shows its
   last-activity-based age until the backend record is read again. Nothing is lost, but the first
   rows a person sees after this ships may read older than they are.

## Verification

All commands were run in `tauri-svelte-preview/`.

- `pnpm run check:svelte` → `Files the /next shell owns: 0 error(s), 0 warning(s).` **Verified.**
  (It also prints the old shell's untouched backlog of 16 errors, which this gate does not check.)
- `pnpm run test:session-rail` → passes end to end: `sessionRowActions: ok`, `ownedSessions`,
  `centerDock`, `myWorkViewOptions`, `errorPresentation` (23 tests, 0 fail), `railElapsedTicker: ok`,
  `paneLayout: all tests passed`. **Verified.**
- Individually green: `railBackendCutover.test.ts`, `sessionPresence.test.mjs`,
  `sessionGroups.test.mjs`, `contrastTokens.test.mjs`, `sidePaneRegistry.test.mjs`,
  `railUtilityEvents.test.mjs`, `agentSessionFocus.test.mjs`, `sessionContextMenuPlacement.test.ts`,
  `sessionLibrary.test.mjs`, `sessionHistoryViewModel.test.mjs`, `sourceUi.test.mjs`,
  `extensionApiCompatibility.test.mjs`, `workspaceCodeLens.test.mjs`,
  `newSessionSubmission.test.ts`, `sessionScanFilter.test.mjs`. **Verified.**
- The typing complaints on the new test files are fixed and checked, not just eyeballed:
  `npx tsc --noEmit --ignoreConfig --strict --skipLibCheck --target es2022 --module esnext
  --moduleResolution bundler --allowImportingTsExtensions` over `scripts/sessionRowActions.test.ts`,
  `scripts/railElapsedTicker.test.ts`, `scripts/errorPresentation.test.ts`, `sessionRowMenu.ts`,
  `railElapsedTicker.ts` and `railRowVisibility.ts` reports nothing beyond the `node:` module
  lookups, which are tsconfig scope noise and were left alone as instructed. The recording target
  now has an explicit `RecordingTarget` interface with typed callback parameters
  (`scripts/sessionRowActions.test.ts:51-73`), and the jump loop narrows `plan` with a `throw`
  before using it (`:121`). No `any` was added anywhere in this lane. **Verified.**
- Two tests fail, and **both fail identically on the branch base** — I confirmed by stashing the
  whole lane (`git stash -u`), running them, and restoring:
  - `nextTokens.test.mjs` — "the /next palette may add only the six approved semantic theme tokens";
    the extra names are the pre-existing inset, row and label tokens. My change touched one value,
    not the name list (`git diff nextTokens.css` shows a single value line plus the two deletions
    above). **Verified.**
  - `conversationActivation.test.mjs` — expects a console line in `+page.svelte` that is not there.
    Nothing in this lane touches conversation activation. **Verified.**

The app was never launched and no browser was opened, so nothing was checked visually — the mockup
match is **assumed** from reading both files side by side. Port 5177 was never bound. No browser or
worktree cleanup applies: this lane started none.
