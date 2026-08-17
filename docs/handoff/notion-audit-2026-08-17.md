# Notion audit — MacCommandBar open tasks vs. code on `tsk-808-assembly-wave`

Read-only audit, 2026-08-17. Worktree `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave`, HEAD `b5a03c0`, 355 commits ahead of `main`.
Board read with `list-tasks.sh "mac-command-bar"` — 64 open tasks. Paths below are relative to `tauri-svelte-preview/` unless prefixed `core/`.
Commits marked `(in main)` are already merged; the rest are branch-only. All were confirmed ancestors of HEAD.

| Task | Title (short) | Notion status | Verdict | Receipt |
|------|---------------|---------------|---------|---------|
| TSK-307 | Reusable Git graph view-model helper | To Do | DONE | commit `c7453a1` (in main); `src/lib/gitGraphViewModel.ts` (600 lines) |
| TSK-378 | Reusable overlay components (bits-ui v2) | Doing | DONE | `package.json:166` `bits-ui ^2.18.1`; `src/lib/components/ui/{dropdown-menu,context-menu,tooltip}/` |
| TSK-759 | Rebuild frame in Codex-app shape | To Do | DONE | commit `dd4c8c9`; `src/lib/shell/components/{SessionRail,CenterCornerTabs,RightPanelTabs}.svelte` |
| TSK-760 | Project picker + new-session flow | To Do | DONE | commits `2942949`, `4043a5a`; `src/lib/shell/newSession/{DraftSessionSurface.svelte,newSessionFlow.ts,projectRootsStore.svelte.ts,threadStartFlow.ts}` |
| TSK-763 | Worktree manager pane | To Do | DONE | commit `d132ec5` (in main) + `dd4c8c9`; `src/lib/shell/worktrees/worktreeManagerService.ts:163` remove, `:226` forceRemove, `:250` archive; `src/lib/shell/panels/worktrees/WorktreesPanel.svelte` |
| TSK-767 | Stack runner | To Do | DONE | commit `e977960` (in main); `src/lib/shell/stacks/stackService.ts:138` refresh, `:184` start, `:219` stop; `src/lib/shell/panels/run/RunPanel.svelte` |
| TSK-768 | Format document via LSP | To Do | DONE | commit `9bcd20a` (in main); `src/lib/tauriSource.ts:1635` `formatSourceWithLspFromTauri`; editor action `src/lib/MonacoSourceEditor.svelte:2693-2694` "Format Document" |
| TSK-771 | Helper filter before the 512-file budget | To Do | DONE | commit `1d1243b` (in main); `core/src/scanners/sessions.rs:198-200`, `:255` |
| TSK-780 | Right pane becomes a tabbed surface | To Do | DONE | commit `dd4c8c9`; `src/lib/shell/components/RightPanelTabs.svelte`; eight panels under `src/lib/shell/panels/` |
| TSK-784 | Language-server status in the editor | To Do | DONE | commit `9655014` (in main); `src/lib/shell/components/LanguageServerStatusChip.svelte`; test `src/lib/shell/components/editor/editorPanelLanguageServer.test.ts` |
| TSK-795 | Rename / rebrand the app | To Do | DONE | commit `4b12764`; `src-tauri/tauri.conf.json:3` `"productName": "Assembly"`, `:16` window title |
| TSK-802 | Per-method reply to server-initiated LSP requests | To Do | DONE | commit `6acb2db` (in main); `src-tauri/src/lsp.rs:883-895` `answer_the_servers_own_question` — `workspace/configuration` returns a real per-setting result, other methods null by design |
| TSK-871 | Multiplex sessions over one shared adapter per provider | To Do | DONE | commit `b638155`; `src-tauri/src/agent_conversation/manager.rs:118` `pool_key`, `:266` `adapter_pools`, `:379` `release_pool_scope` |
| TSK-872 | Session records into SQLite as source of truth | To Do | DONE | commits `7276816`, `b638155`; `core/src/session_store.rs` |
| TSK-891 | Remove the session header bar | To Do | DONE | commit `54d06b3`; `rg -i "session-header|SessionHeader" src` returns nothing |
| TSK-892 | New session becomes a real session surface | To Do | DONE | commit `4043a5a`; `src/lib/shell/newSession/DraftSessionSurface.svelte` (draft opens from the rail, not a modal) |
| TSK-894 | Browser panel native view blocks app / ignores bounds | To Do | DONE | commits `17a502c`, `c65922f`, `e52f751`, `0bd136f`; `src/lib/shell/panels/browser/browserPanelBounds.ts` |
| TSK-909 | Usage popover: used vs. remaining toggle | To Do | DONE | commit `27679e9`; `src/lib/shell/usage/UsagePopover.svelte:32` |
| TSK-915 | Browser annotate 1:1 Codex parity rebuild | Doing | DONE | commits `3226bdc`, `f62b03e`, `13c53a2`; `src/lib/shell/panels/browser/{AnnotationCanvas,AnnotationBadges,BrowserMiniComposer}.svelte`, `annotationList.ts`. Task's own done-criterion is owner confirmation in the release build — code is complete, sign-off is not |
| TSK-344 | VS Code-style Git source control panel | Doing | PARTIAL | commits `8e65a64`, `9361670`, `dd4c8c9`; `src/lib/shell/panels/sourceControl/{SourceControlPanel,ChangedFileRow,CommitTimeline}.svelte`. Missing: the Graph pane and the center "Git graph" tab — `gitGraphViewModel.ts` still has no UI home in the next shell (only `gitCommitRefSummary` is imported, `CommitTimeline.svelte:31`); `buildGitGraphViewModel` is called only from the legacy `src/routes/+page.svelte:1476` |
| TSK-761 | Session lifecycle Working/Done/Settled | Doing | PARTIAL | commits `8f4f127`, `6651c95`, `a5d7ace`; `src/lib/shell/ownedSessions.ts:84` `settledAt`, `src/lib/shell/components/sessionRowMenu.ts:66-69` Settle/Unsettle. Not verified: the two-surface split ("My work" vs. "Find a session") and restart-from-Done |
| TSK-765 | Theme system / Houston / VSIX import | To Do | PARTIAL | commit `3e2817f` (in main); `src/lib/shell/themes/themeRegistry.ts:190` houston, `:352` dracula; applied to Monaco (`MonacoSourceEditor.svelte:28`), xterm (`xtermFactory.ts:23`), shell (`routes/next/+page.svelte:185`). File-type icons exist (`src/lib/shell/components/explorer/{FileIcon.svelte,fileIcons.ts}`). Missing: VSIX theme/grammar import — `rg -i vsix` over `src` and `src-tauri` returns nothing |
| TSK-770 | Problems panel | To Do | PARTIAL | commit `e96fe14` (in main); `src/lib/shell/components/problems/ProblemsPanel.svelte`, mounted `src/lib/shell/components/DockPanel.svelte:42`. Missing: the build-errors half — no dotnet/cargo/vite output parsing anywhere in `src/lib/shell/problems/`. Matches the task's own 2026-07-29 note |
| TSK-773 | svelte-check in the check pipeline | To Do | PARTIAL | `package.json:188` `svelte-check ^4.7.4` installed; `:13` `check:svelte` script exists. Missing: `:11` `"check"` runs `tsc --noEmit && pnpm check:rust` only — `check:svelte` is not in the pipeline |
| TSK-779 | Top-bar quick-open for center tabs | To Do | PARTIAL | commits `dd4c8c9`, `1574198`; `src/lib/shell/components/CenterCornerTabs.svelte:45-47`. Tabs are Session / Editor / Diff — no Browser entry (the browser became a right panel, commits `d92cb0c`, `1a9b63f`), so the task as written is not literally met |
| TSK-809 | Composer: attachments, commands, model controls | To Do | PARTIAL | commits `7fc49ed`, `5643167`, `69d42a1`, `62e356e`; `src/lib/shell/components/conversation/ConversationComposer.svelte:13` slash menu, `:26` attachments, `ComposerConfigMenu.svelte` model/effort/approval. Acceptance not met while TSK-906 (attachments), TSK-897 (model switch), TSK-899/901 (slash menu) are open against it |
| TSK-879 | Sub-agent visibility: live children strip | To Do | PARTIAL | commits `e53af76`, `706c1a4`, `07ffbf7`, `0499762`, `b3fa85e`; `src/lib/shell/components/ConversationSurface.svelte:391` `ConversationAgentTree`, `src/lib/shell/panels/agents/AgentsPanel.svelte`. Missing: the rail child-count badge — `rg "childCount\|subagent" SessionRail.svelte sessions/*.svelte` returns nothing |
| TSK-889 | Right-click in the right panel freezes the app | To Do | PARTIAL | commit `458fa12`; `src/routes/next/+page.svelte:1694` `oncontextmenu` suppression. The webview's own menu is suppressed; the reported freeze itself is not verified fixed |
| TSK-897 | Mid-session model switch fails | To Do | PARTIAL | commits `998621a`, `076abe7` (capabilities now populate on resume, generation/revision persisted). The mid-session switch path itself is not verified |
| TSK-899 | Slash-command menu rows overlap | To Do | PARTIAL | commit `27679e9` ("slash menu placement"). Row-overlap CSS not verified |
| TSK-903 | Worktrees panel CSS overhaul | To Do | PARTIAL | commits `f14b58b`, `27679e9` (panel stops painting black, content stays inside the panel). Blank session chips not verified |
| TSK-904 | Source Control freezes on commit clicks | To Do | PARTIAL | commits `9863772`, `61c0182`, `9361670` (body mounts only while the tab shows; snapshot-on-open menus; per-file staging restored). The 10s freeze is not verified gone |
| TSK-923 | Resume a past session as a real Assembly session | To Do | PARTIAL | commits `8aab513`, `30ce778`, `95b32f1`, `7dba43c`, `92e5ee5`, `65c2353`, `06147a5`, `076abe7`; `src-tauri/src/agent_conversation/transcript_import.rs`. The Notion page's own verified build-status section is current: transcript still renders empty on screen; `extendAgentConversationImportFromTauri` has zero callers (no Load More); virtualization not built |
| TSK-924 | Base UI pass: alignment, status bar, center tabs, rail | To Do | PARTIAL | commits `890a920`, `ab44932`, `bffc9d0`, `44a59a2`, `1574198`, `021005c`, `19477f2`, `aced966`; global status bar at `src/routes/next/+page.svelte:1770`. Broad umbrella — remaining scope not verifiable from code alone |
| TSK-898 | Rail drag reorder, hover time, Settle/Revert | To Do | PARTIAL | `src/lib/shell/components/SessionRail.svelte:163-169` reorder + persist; Settle in `sessionRowMenu.ts:66-69`. Hover-time display not verified |
| TSK-758 | Minimize panels to an edge strip | To Do | NOT DONE | only browser minimize exists (`src/lib/shell/browser/browserModel.ts:528`); no panel edge-strip code found |
| TSK-762 | Rail correctness (top-level only, file/session click) | To Do | NOT DONE / unverified | no implementing symbol found; not individually verified |
| TSK-764 | Playwright card: stale sessions, kill the killable | To Do | NOT DONE | `rg -i stale src/lib/shell/processes/` returns nothing; only `killPlaywrightSession` / `killAllPlaywrightSessions` exist |
| TSK-766 | Research CMUX agent orchestration | To Do | NOT DONE / unverified | research task; no receipt found |
| TSK-769 | DB browser panel | To Do | NOT DONE | `rg -i "dbBrowser\|database browser" src src-tauri` returns nothing |
| TSK-783 | Editor paints file text instantly | To Do | NOT DONE / unverified | `47889f3`, `b4bdf21` touch editor boot and skeleton paint, but no plain-text-first-frame warm path found |
| TSK-789 | Cancel superseded LSP reference lookups | Doing | NOT DONE / unverified | not individually verified |
| TSK-808 | Orchestrate the Assembly workbench wave | To Do | NOT DONE | umbrella task for this whole branch; open by construction |
| TSK-870 | Agents phase: cross-model handoffs, supervisor | To Do | NOT DONE | only `src-tauri/src/orchestration.rs` matches; no handoff/supervisor surface |
| TSK-874 | Database browser with AI query assist | To Do | NOT DONE | same search as TSK-769 — nothing |
| TSK-876 | Split main.rs into domain command modules | To Do | NOT DONE | `src-tauri/src/main.rs` is 9034 lines (grew from the 8.7k in the title) |
| TSK-878 | Unify hover-action buttons to the kit pattern | To Do | NOT DONE / unverified | `src/lib/components/ui/hover-actions/` exists but the sweep is not verified |
| TSK-888 | Editor syntax highlighting independent of LS | To Do | NOT DONE | no shiki/TextMate/basic-languages registration; `src/lib/shell/editor/monacoWorkers.ts` wires standalone JSON/TS workers only |
| TSK-893 | New session wires up Files panel and Editor | To Do | NOT DONE / unverified | commit `4043a5a` says "bind panels on first send", but no `bindPanels`-style symbol found in `newSessionFlow.ts` |
| TSK-896 | User message bubbles grey background | To Do | NOT DONE | no background rule in `src/lib/shell/components/conversation/UserMessageItem.svelte` |
| TSK-900 | Surface custom provider slash commands for codex | To Do | NOT DONE | `src/lib/shell/conversation/composerSlashCommands.ts` has no provider-custom command path |
| TSK-901 | Slash-menu selection freezes the UI | To Do | NOT DONE / unverified | no fix commit identified |
| TSK-902 | Recurring vite cache corruption | To Do | NOT DONE / unverified | no fix commit identified |
| TSK-905 | Run panel Add Action dialog overflows | To Do | NOT DONE / unverified | `src/lib/shell/panels/run/AddActionDialog.svelte` exists; overflow fix not verified |
| TSK-906 | Image attachments broken | To Do | NOT DONE / unverified | `0d73861`, `69d42a1`, `e3eca0e` touch the pipeline, but the task's four reported shapes are not verifiably resolved |
| TSK-907 | Claude sessions fail resume | To Do | NOT DONE / unverified | superseded in scope by TSK-923, which is still failing |
| TSK-908 | Codex history resume broken, orphan rows | To Do | NOT DONE / unverified | superseded in scope by TSK-923, which is still failing |
| TSK-910 | Mirror resume ensure failures into backend log | To Do | NOT DONE / unverified | `debug_log::stderr_log!` calls exist in `manager.rs` but not confirmed on the resume-ensure path |
| TSK-911 | Composer file picker → async dialog | To Do | NOT DONE | no `plugin-dialog` / async open-dialog call in the composer or conversation code |
| TSK-912 | Attachment persistence cleanup and wire format | To Do | NOT DONE / unverified | `src-tauri/src/agent_conversation/attachments.rs` exists but is **uncommitted-modified** in the worktree |
| TSK-913 | 5.4s main-thread block at shell startup | To Do | NOT DONE / unverified | several perf commits, none naming this block |
| TSK-922 | Comment pass over the agent_conversation backend | To Do | NOT DONE | `manager.rs` carries 27 `//` lines, `mod.rs` 35 — far short of a reader-oriented pass |
| TSK-925 | Hide the CLI / raw-terminal entry points | To Do | NOT DONE | all three buttons still render: `src/lib/shell/components/ConversationSurface.svelte:378` "Open in native CLI", `:383` "Fork to native CLI", `:387` "Open raw terminal" |
| TSK-926 | Bottom dock hosts Problems and Terminal as tabs | To Do | NOT DONE | `src/lib/shell/components/DockPanel.svelte:42` hosts `ProblemsPanel` only; no tab strip, no Terminal tenant |

## Recommend closing

Safely closable — the feature exists, is committed, and the receipt is in the table:

TSK-307, TSK-378, TSK-759, TSK-760, TSK-763, TSK-767, TSK-768, TSK-771, TSK-780, TSK-784, TSK-795, TSK-802, TSK-871, TSK-872, TSK-891, TSK-892, TSK-894, TSK-909

TSK-915 is code-complete but its written done-criterion is owner confirmation in the release build — close it after that look, not before.

## Needs owner decision (PARTIAL)

- **TSK-344** — Graph pane and the center "Git graph" tab are missing; `gitGraphViewModel.ts` still has no UI home in the next shell.
- **TSK-761** — Settled state and row menu exist; the "My work" / "Find a session" two-surface split and restart-from-Done are not verified.
- **TSK-765** — Themes are wired end to end and file icons exist; VSIX theme/grammar import is entirely absent.
- **TSK-770** — LSP half shipped; build-error parsing (dotnet/cargo/vite) never started. The task's own note already says this.
- **TSK-773** — `svelte-check` is installed and `check:svelte` exists, but `pnpm check` does not call it, so the blind spot is still open.
- **TSK-779** — Session/Editor/Diff shipped; no Browser center tab, because the browser deliberately became a right panel. Decide whether that closes the task.
- **TSK-809** — Composer features shipped, but four open defects (TSK-906, 897, 899, 901) sit on top of its acceptance list.
- **TSK-879** — Children tree on the session view and the Agents roster exist; the rail child-count badge does not.
- **TSK-889 / 897 / 899 / 903 / 904** — plausible fix commits landed for each, but none can be confirmed against the reported symptom without running the app. These are the cheapest wins to verify by hand.
- **TSK-923** — Import, resume, capabilities and replay all verified; the transcript still renders empty, Load More has zero callers, and virtualization is unbuilt.
- **TSK-924** — A large amount of the base UI pass landed (status bar, panel corners, pill tabs, rail rows); the remaining scope is not decidable from code.
