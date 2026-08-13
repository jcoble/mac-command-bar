# haspurge21 verification report

Date: 2026-08-13  
Worktree: `/Users/blackcolours/dev/work/worktrees/mac-command-bar/haspurge21`  
Branch: `lane/haspurge21`  
Scope: verification, receipts, and receipt-driven corrections only. No files were staged or committed.

## Decision

**Verified:** the application source, live `/next` CSS, and production build CSS contain zero `:has(` selectors. The production build succeeds, the `/next` Svelte-owned check is 0 errors / 0 warnings, the menu and markdown affected tests pass, and the lane performance run shows no idle animation, timer, or long-task regression against the sequential baseline run.

**Verified exception:** `pnpm test:next-tokens` reaches an inherited token-allowlist failure after its stale `:root:has(.next-shell)` expectation was corrected. The untouched baseline fails at the same six-token assertion, so this report does not misclassify that existing theme-contract mismatch as a purge regression.

**Assumed, not proved:** post-change screenshots show the four requested states at the required viewport, but there are no pre-change captures in this lane. Functional state parity is visually plausible; pixel-identical before/after parity is not proved.

## Receipt-driven corrections

1. The stale test assertion now checks the explicit route-state selector `:root.next-shell-document` (`tauri-svelte-preview/scripts/nextTokens.test.mjs:130`). This was changed only after the affected test failed on the removed selector.
2. The production Vite configuration now permits both the worktree and the real path of its symlinked dependency directory (`tauri-svelte-preview/vite.config.ts:291-293`). This was changed only after the lane harness reproduced a Vite 403 for dependency files; the second lane run completed.
3. The temporary `tauri-svelte-preview/vite.haspurge21-validation.config.ts` was deleted. The load-bearing production transform remains registered at `tauri-svelte-preview/vite.config.ts:40-105,278`.

## Selector inventory and replacements

Command receipt:

```text
$ rg -n ':has\(|has-\[' src
0 matches
```

| Original rule family | Replacement | Evidence | Status |
|---|---|---|---|
| Activity worktree row/actions conditioned on an open descendant menu | `class:menu-open` on the row and action owner; ordinary `.menu-open` selectors | `src/lib/components/panels/ActivityWorktreesPanel.svelte:191,257`; `src/app.css:278-286` | Verified |
| Repository, task-ledger, and commit rows/actions conditioned on an open menu | Existing open-state functions drive `class:menu-open`; ordinary grouped selectors | `src/lib/components/panels/ActivityGitPanel.svelte:374,402,488,541,648,666,681,1747-1776` | Verified |
| Old-shell run, snapshot, runtime, and agent rows/actions conditioned on an open menu | Existing open-state functions drive `class:menu-open`; ordinary grouped selectors | `src/routes/+page.svelte:15219,15375,15530,15560,15715,15729,15826,15870,18122-18157` | Verified |
| `/next` document/root descendant detection | Route lifecycle adds/removes `.next-shell-document`; CSS uses `:is(:root.next-shell-document, .next-shell)` | `src/routes/next/+page.svelte:1169,1345`; `src/lib/shell/styles/nextTokens.css:44`; `src/lib/shell/styles/next.css:178,259-261`; `src/lib/shell/styles/themeChrome.css:32` | Verified |
| Alert-dialog header/title conditioned on media | Explicit `hasMedia` composition prop | `src/lib/components/ui/alert-dialog/alert-dialog-header.svelte:9-22`; `src/lib/components/ui/alert-dialog/alert-dialog-title.svelte:8-21` | Verified |
| Badge, button, and tab padding conditioned on leading/trailing icon | Explicit `iconPosition` prop | `src/lib/components/ui/badge/badge.svelte:12-30`; `src/lib/components/ui/button/button.svelte:10-23`; `src/lib/components/ui/tabs/tabs-trigger.svelte:8-22` | Verified |
| Card spacing/layout conditioned on footer, leading image, action, or description | Explicit `hasFooter`, `hasLeadingImage`, `hasAction`, and `hasDescription` props | `src/lib/components/ui/card/card.svelte:9-28`; `src/lib/components/ui/card/card-header.svelte:9-25` | Verified |
| Tooltip padding conditioned on keyboard hint | Explicit `hasKbd` prop | `src/lib/components/ui/tooltip/tooltip-content.svelte:16-34` | Verified |
| Dependency dock resize container conditioned on a group view | Build transform rewrites CSS to `.dv-resize-container-with-groupview`; dependency JavaScript toggles the paired class | `vite.config.ts:52-65,68-74` | Verified by live/build CSS receipts |
| Dependency dock tab chip conditioned on empty label | Build transform rewrites CSS to `.dv-tab-group-chip--empty`; dependency JavaScript toggles the paired class | `vite.config.ts:58-61,77-83` | Verified by live/build CSS receipts |
| Dependency markdown list item conditioned on a checkbox | Build transform rewrites CSS to `.rendered-markdown-checkbox-item`; renderer adds the paired class | `vite.config.ts:86-100` | Verified by live/build CSS receipts |

The explicit kit props are currently dormant where those state-bearing compositions are not present. This preserves current rendered behavior, but future compositions must set the matching prop instead of relying on descendant inspection. This is an implementation-contract observation, not a claim of testing future call sites.

## CSS purge receipts

### Live development CSS, `/next`

The browser was opened against strict port `5181`, resized, and verified before inspection:

```text
window.innerWidth + 'x' + window.innerHeight = 1710x990
```

All eight loaded CSS resources reported zero occurrences:

```text
/src/app.css                                                   0
/src/lib/shell/styles/nextTokens.css                           0
/src/lib/shell/styles/next.css                                 0
/src/lib/shell/styles/themeChrome.css                          0
/src/routes/next/+page.svelte?...                              0
/src/lib/shell/components/GitDiffView.svelte?...               0
/node_modules/.../dockview-core/dist/styles/dockview.css?...   0
/src/lib/shell/components/ShellFrame.svelte?...                0
TOTAL                                                          0
```

**Verified:** no justified survivor exists in live `/next` CSS.

### Production build CSS

```text
$ find build -type f -name '*.css' | wc -l
22
$ rg -nF ':has(' build --glob '*.css'
0 matches
```

**Verified:** no justified survivor exists in production CSS.

## Gate receipts

| Gate | Receipt | Result |
|---|---|---|
| Svelte ownership check | `pnpm run check:svelte` → `Files the /next shell owns: 0 error(s), 0 warning(s).` The same command reports `16 error(s)` in the pre-existing old-shell backlog. | `/next` PASS; repository-wide backlog disclosed |
| Menu-open sweep | `pnpm test:menu-open-sweep` → `menu open sweep tests passed` | PASS |
| Markdown preview | `pnpm test:source-markdown-preview` → exit 0 | PASS |
| Next token test | `pnpm test:next-tokens` → fails only on six extra tokens: `--floating-content-inset`, `--floating-row-gap`, `--rail-content-inset`, `--rail-row-content-inset`, `--row-selected`, `--secondary-label` | INHERITED FAILURE |
| Baseline next token test | The same command in the untouched assembly-wave baseline fails at the identical six-token allowlist assertion | Confirms inherited failure |
| Production build | `pnpm build` → exit 0; `built in 34.44s` | PASS |
| Patch hygiene | `git diff --check` → exit 0, no output | PASS |

## Performance receipts

The two harnesses were run sequentially, baseline first. The baseline worktree was not modified.

| Metric | Assembly-wave baseline | haspurge21 lane | Difference |
|---|---:|---:|---:|
| Hover highlight p95, initial snapshot | 81.4 ms | 97.2 ms | +15.8 ms |
| Hover buttons p95, initial snapshot | 181.5 ms | 197.3 ms | +15.8 ms |
| Hover highlight p95, after 2k transcript rows | 32.9 ms | 32.4 ms | -0.5 ms |
| Hover buttons p95, after 2k transcript rows | 132.9 ms | 131.9 ms | -1.0 ms |
| Session click to transcript | 224.0 ms | 201.5 ms | -22.5 ms |
| Snapshot long tasks | 2 (`61`, `70` ms) | 1 (`65` ms) | -1 event |
| Idle long tasks | 0 | 0 | no change |
| Resting animation census | 0 | 0 | no change |
| Presence interval firings | 10 | 10 | no change |
| Vite interval firings | 0 | 0 | no change |
| Session-library interval firings | 0 | 0 | no change |

Command: `node --experimental-strip-types scripts/perfHarness.ts` in each worktree. **Verified:** the steady 2,000-row hover measurements are effectively unchanged, click latency remains below the repository's 350 ms target, and no idle work regression was observed. **Assumed:** a single run is evidence for this snapshot, not a causal benchmark or distribution-level performance guarantee.

## Visual receipts

All full-page captures were made from Vite strict port `5181` after `window.innerWidth + 'x' + window.innerHeight` returned `1710x990`.

| Required state | Artifact | Evidence status |
|---|---|---|
| `/next` session rail hover | `haspurge21-screenshots/after-next-session-hover.png` (1710×990) | Verified post-change state |
| Old-shell activity worktree row with action menu open | `haspurge21-screenshots/after-old-shell-worktree-menu.png` (1710×990) | Verified post-change state |
| Kit card | `haspurge21-screenshots/after-kit-card.png` (287×76 element crop taken within the verified browser session) | Verified post-change component state; crop is not a viewport receipt by itself |
| Alert dialog | `haspurge21-screenshots/after-alert-dialog.png` (1710×990) | Verified post-change state |

No pre-change screenshots were present. Therefore these artifacts prove the requested states render after the purge, but not pixel-identical before/after parity.

## Cleanup and repository state

```text
Browser cleanup: stopped haspurge21b, haspurge21b-capture, and haspurge21b-old-shell (sessions and owned helper trees)
Browser cleanup: stopped haspurge21b-vite (strict 5181 server tree)
Port receipt: lsof -nP -iTCP:5181 -sTCP:LISTEN -> no output
```

`git status --short` still shows the existing uncommitted implementation, the receipt-driven test/config edits, the screenshot directory, and the existing untracked `tauri-svelte-preview/node_modules` symlink. The temporary validation config is absent. Nothing was staged, committed, or pushed.

## Final disposition

The `:has()` purge itself is verified complete with no source, live CSS, or build CSS survivors and no reproduced performance or visual-state defect. The lane is suitable for review with two explicit limitations: the inherited next-token allowlist gate remains red, and pixel-identical visual parity cannot be claimed without pre-change captures.
