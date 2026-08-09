Reading prompt from stdin...
OpenAI Codex v0.147.0
--------
workdir: /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
model: gpt-5.6-luna
provider: openai
approval: never
sandbox: read-only
reasoning effort: high
reasoning summaries: none
session id: 019fe5da-2263-7fb3-8734-25310796a26c
--------
user
READ-ONLY DISCOVERY task, worktree /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave. Do NOT edit any file. Do NOT commit. Plain English.

FIRST ACTION: write your findings incrementally to docs/superpowers/evidence/tsk-808/round2-discovery/acp-structured-receipt.md — wait, that would be a write. INSTEAD: print all findings to stdout only. Every claim carries file:line, labelled verified or assumed.

QUESTION: What does the "Session" center tab in the /next shell actually render today, and what happened to the ACP structured-conversation view?

The product decision (see the plan in docs/ from commit 279b453, "plan: define ACP runtime, workflows, and Orca-inspired workbench") was: the structured conversation is driven by ACP (Agent Client Protocol) — structured turn/message events from the agent runtime — NOT by parsing the terminal. The user reports: the Session tab now shows ONLY the terminal; a structured view flashed once and could not be reached again.

ANSWER THESE:
1. Which component renders the Session center tab (start at tauri-svelte-preview/src/lib/shell/ — ShellFrame.svelte, centerDock.ts, and whatever Session panel component they mount). Exact file:line for the mount.
2. Is there a structured conversation component (message list / turn cards)? Does it exist, is it wired, and what condition switches between structured view and terminal view? Quote the condition with file:line.
3. Does the structured path consume ACP events (search src-tauri/src/agent_conversation/, providers/acp.rs, StructuredRuntimeHandle) or does it parse terminal output? Trace the data path frontend<-backend: which tauri commands/events feed it.
4. Find the plan document(s) describing the ACP structured-session decision (grep docs/ for ACP, structured, agent-client-protocol). Cite what the plan promised vs what exists.
5. Git archaeology: was a structured session view removed or regressed? git log --follow on the relevant components; name the commit(s) if something was ripped out.
6. Why would the structured view appear briefly then become unreachable (a toggle state, a mode flag defaulting to terminal, an error path)? Point at the exact state variable.

OUTPUT: a structured report to stdout: SECTION per question, file:line receipts, verified/assumed labels, ending with a 5-line summary of the minimal changes needed to make structured-over-ACP the primary session view.

warning: clamping SessionEnd hook timeout to 3s in /Users/blackcolours/.codex/plugins/cache/openai-codex/codex/1.0.6/hooks/hooks.json
2026-08-09T09:28:26.357128Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
2026-08-09T09:28:26.411190Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
codex
I’ll trace the `/next` Session tab from the shell mount through frontend state and Tauri ACP events, then compare it with the ACP plan and git history. I will make no edits or commits; every finding will be printed here with verified/assumed labels and exact file:line receipts.
exec
/bin/zsh -lc 'pwd && rg -n "TSK-808|ACP|structured session|Session tab|structured conversation" /Users/blackcolours/.codex/memories/MEMORY.md | head -80' in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
1:# Task Group: mac-command-bar TSK-808 task delivery discipline and partial contrast implementation
3:scope: Resume or review the TSK-808 Assembly native-workbench product wave without mistaking probes, runtime setup, or uncommitted contrast work for a closed task.
6:## Task 1: Execute TSK-808 orchestration and delivery, partial
14:- TSK-808, PR-14, 167a30ae, tsk-808-assembly-wave, contrast, nextTokens.css, themeChrome.css, themeRegistry.ts, themes/contrast.ts, contrastTokens.test.mjs, WP12A, R0, Notion, dirty-worktree, ScreenCaptureKit Code=-3801, CodeLens, Peek
24:- PR #14 / commit `167a30ae389a2280501011186f245a08d28c6725` remains the intentional checkpoint. The partial TSK-808 worktree was `tsk-808-assembly-wave`, based on `beebda6c4dad60cd2782374a36f24737cefda605`; it did not replace the primary `main` checkout. [Task 1]
27:- Direct Notion evidence showed `[TSK-808] Orchestrate the Assembly native-workbench product wave` remained `To Do`; no task received a commit, PR, merge, or closure. [Task 1]
527:# Task Group: mac-command-bar TSK-808 native-workbench planning and SOL-xhigh orchestration handoff
529:scope: Execution-ready TSK-808 planning, protected-WIP boundaries, and the bounded controller handoff for the MacCommandBar/Assembly product wave; planning only, not implementation authority.
532:## Task 1: Recover the rapid-fire backlog and write the TSK-808 implementation plan, success
540:- TSK-808, TSK-799, TSK-798, Assembly, docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md, 2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md, contrast, legibility, gpt-5.6-sol, Tauri, Dockview, Monaco, Roslyn, U1
546:- rollout_summaries/2026-08-01T18-45-26-cBqj-tsk_808_plan_and_sol_xhigh_orchestration_handoff.md (cwd=/Users/blackcolours/dev/work/mac-command-bar, rollout_path=/Users/blackcolours/.codex/sessions/2026/08/01/rollout-2026-08-01T14-45-26-019fbea5-360e-7aa1-901e-afc6d55154b3.jsonl, updated_at=2026-08-04T10:31:30+00:00, thread_id=019fbea5-360e-7aa1-901e-afc6d55154b3, handoff verified; TSK-808 remains To Do)
550:- SOL-xhigh, gpt-5.6-sol, xhigh, Luna Max, SOL-medium, WP0, WP12A, R0-R6, ACP, handoff, worktree cleanup, Browser proof, handoff verification PASS
562:- The authoritative plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; use section 18 of `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md` instead of retired schedules. TSK-808 remained open/High; TSK-799 and TSK-798 are completed dependencies. [Task 1]
576:scope: Native CodeLens/Peek acceptance plus completed minimal-disruption Monaco/Roslyn and agent-workbench integration in the existing MacCommandBar Tauri/Svelte/Rust app; preserve active task boundaries and distinguish merged checkpoint work from unfinished TSK-808/809/810 product work.
639:- as a .NET developer learning Rust, the user requested .NET-friendly documentation -> map Rust concepts to familiar .NET terms and clearly separate verified checkpoint work from remaining TSK-808/809/810 work [Task 2]
652:- TSK-808, TSK-809, and TSK-810 were verified still `To Do`; a merged checkpoint must not close unfinished product work. [Task 2]

exec
/bin/zsh -lc "rg --files | sed -n '1,120p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src/routes/+layout.svelte
tauri-svelte-preview/src/routes/preview/+page.svelte
tauri-svelte-preview/src/routes/+page.svelte
tauri-svelte-preview/src/routes/next/+page.svelte
tauri-svelte-preview/src/routes/+layout.ts
tauri-svelte-preview/src/app.html
tauri-svelte-preview/src/lib/worktreeCleanupPlan.ts
tauri-svelte-preview/src/lib/SourceWorkbench.svelte
tauri-svelte-preview/src/lib/sourceCodeLensKeys.ts
tauri-svelte-preview/src/lib/SourceMarkdownPreview.svelte
tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts
tauri-svelte-preview/src/lib/agentSessionFocus.ts
tauri-svelte-preview/src/lib/worktreeSafety.ts
tauri-svelte-preview/src/lib/sourceMarkdownPreview.ts
tauri-svelte-preview/src/lib/sourceEditorPanels.ts
tauri-svelte-preview/src/lib/workspaceSnapshotPlan.ts
tauri-svelte-preview/src/lib/components/Badge.svelte
tauri-svelte-preview/src/lib/components/SearchInput.svelte
tauri-svelte-preview/src/lib/components/ConversationList.svelte
tauri-svelte-preview/src/lib/components/Dialog.svelte
tauri-svelte-preview/src/lib/components/Slider.svelte
tauri-svelte-preview/src/lib/components/Chip.svelte
tauri-svelte-preview/src/lib/components/tone.ts
tauri-svelte-preview/src/lib/components/chrome/HiddenDockRail.svelte
tauri-svelte-preview/src/lib/components/chrome/EditorTopbarViewMenu.svelte
tauri-svelte-preview/src/lib/components/ContextMenu.svelte
tauri-svelte-preview/src/lib/components/Switch.svelte
tauri-svelte-preview/src/lib/components/Button.svelte
tauri-svelte-preview/src/lib/components/Menu.svelte
tauri-svelte-preview/src/lib/components/panels/EditorPanel.svelte
tauri-svelte-preview/src/lib/components/panels/BrowserPanel.svelte
tauri-svelte-preview/src/lib/components/panels/ActivityClipboardPanel.svelte
tauri-svelte-preview/src/lib/components/panels/GitInsightsPanel.svelte
tauri-svelte-preview/src/lib/components/panels/ActivityFilesPanel.svelte
tauri-svelte-preview/src/lib/components/panels/ActivityProjectControls.svelte
tauri-svelte-preview/src/lib/components/panels/ActivityWorktreesPanel.svelte
tauri-svelte-preview/src/lib/components/panels/ActivityGitPanel.svelte
tauri-svelte-preview/src/lib/components/Select.svelte
tauri-svelte-preview/src/lib/components/overlays/QuickOpenOverlay.svelte
tauri-svelte-preview/src/lib/components/overlays/CommandPaletteOverlay.svelte
tauri-svelte-preview/src/lib/components/Tabs.svelte
tauri-svelte-preview/src/lib/components/IconButton.svelte
tauri-svelte-preview/src/lib/components/Tooltip.svelte
tauri-svelte-preview/src/lib/components/ui/badge/badge.svelte
tauri-svelte-preview/src/lib/components/ui/badge/variants.ts
tauri-svelte-preview/src/lib/components/ui/badge/index.ts
tauri-svelte-preview/src/lib/components/ui/select/select-scroll-down-button.svelte
tauri-svelte-preview/src/lib/components/ui/select/index.ts
tauri-svelte-preview/src/lib/components/ui/select/select-trigger.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-scroll-up-button.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-portal.svelte
tauri-svelte-preview/src/lib/components/ui/select/select.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-content.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-item.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-group-heading.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-label.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-group.svelte
tauri-svelte-preview/src/lib/components/ui/select/select-separator.svelte
tauri-svelte-preview/src/lib/components/ui/switch/index.ts
tauri-svelte-preview/src/lib/components/ui/switch/switch.svelte
tauri-svelte-preview/src/lib/SourcePaneResizer.svelte
tauri-svelte-preview/src/lib/sourcePreviewAppearance.ts
tauri-svelte-preview/src/lib/workspaceSnapshot.ts
tauri-svelte-preview/src/lib/pasteCleanup.ts
tauri-svelte-preview/src/lib/WorkbenchContextPanel.svelte
tauri-svelte-preview/src/lib/sourceDockLayout.ts
tauri-svelte-preview/src/lib/sourceData.ts
tauri-svelte-preview/src/lib/gitGraphViewModel.ts
tauri-svelte-preview/src/app.css
tauri-svelte-preview/src/monaco-standalone-languages.d.ts
tauri-svelte-preview/src/lib/components/ui/collapsible/index.ts
tauri-svelte-preview/src/lib/components/ui/collapsible/collapsible-content.svelte
tauri-svelte-preview/src/lib/components/ui/collapsible/collapsible.svelte
tauri-svelte-preview/src/lib/components/ui/collapsible/collapsible-trigger.svelte
tauri-svelte-preview/src/lib/gitTaskLinks.ts
tauri-svelte-preview/src/lib/liveConversationTerminals.ts
tauri-svelte-preview/src/lib/components/ui/scroll-area/index.ts
tauri-svelte-preview/src/lib/server/localSourceFs.ts
tauri-svelte-preview/src/lib/components/ui/scroll-area/scroll-area.svelte
tauri-svelte-preview/src/lib/components/ui/scroll-area/scroll-area-scrollbar.svelte
tauri-svelte-preview/src/lib/server/gitBridge.ts
tauri-svelte-preview/src/lib/settingsStore.svelte.ts
tauri-svelte-preview/src/lib/components/ui/input/index.ts
tauri-svelte-preview/src/lib/components/ui/input/input.svelte
tauri-svelte-preview/src/lib/components/ui/button/variants.ts
tauri-svelte-preview/src/lib/components/ui/button/index.ts
tauri-svelte-preview/src/lib/components/ui/button/button.svelte
tauri-svelte-preview/src/lib/stores/gitStore.svelte.ts
tauri-svelte-preview/src/lib/stores/dockLayoutStore.svelte.ts
tauri-svelte-preview/src/lib/stores/filesStore.svelte.ts
tauri-svelte-preview/src/lib/stores/projectStore.svelte.ts
tauri-svelte-preview/src/lib/SettingsPanel.svelte
tauri-svelte-preview/src/lib/utils.ts
tauri-svelte-preview/src/lib/orchestrationView.ts
tauri-svelte-preview/src/lib/sourcePaneSizing.ts
tauri-svelte-preview/src/lib/worktreeCleanupRunbook.ts
tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte
tauri-svelte-preview/src/lib/components/CollapsibleSection.svelte
tauri-svelte-preview/src/lib/styles/tokens.css
tauri-svelte-preview/src/lib/components/ui/tabs/tabs-trigger.svelte
tauri-svelte-preview/src/lib/components/ui/tabs/variants.ts
tauri-svelte-preview/src/lib/components/ui/tabs/index.ts
tauri-svelte-preview/src/lib/components/ui/tabs/tabs-content.svelte
tauri-svelte-preview/src/lib/components/ui/tabs/tabs.svelte
tauri-svelte-preview/src/lib/components/ui/tabs/tabs-list.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-portal.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-overlay.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-cancel.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-content.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-footer.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-action.svelte
tauri-svelte-preview/src/lib/components/ui/alert-dialog/alert-dialog-title.svelte
tauri-svelte-preview/src/lib/components/ui/dialog/dialog-overlay.svelte
tauri-svelte-preview/src/lib/components/ui/dialog/dialog-header.svelte
tauri-svelte-preview/src/lib/components/ui/dialog/dialog.svelte
tauri-svelte-preview/src/lib/components/ui/dialog/index.ts
tauri-svelte-preview/src/lib/components/ui/dialog/dialog-close.svelte
tauri-svelte-preview/src/lib/components/ui/dialog/dialog-content.svelte
tauri-svelte-preview/src/lib/components/ui/dialog/dialog-footer.svelte

exec
/bin/zsh -lc 'rg --files tauri-svelte-preview/src/lib/shell tauri-svelte-preview/src/routes/next src-tauri 2>/dev/null | sort' in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src/lib/shell/assistance/AssistanceAction.svelte
tauri-svelte-preview/src/lib/shell/assistance/AssistanceHost.svelte
tauri-svelte-preview/src/lib/shell/assistance/AssistanceProposal.svelte
tauri-svelte-preview/src/lib/shell/assistance/assistanceAudit.ts
tauri-svelte-preview/src/lib/shell/assistance/assistanceContext.ts
tauri-svelte-preview/src/lib/shell/assistance/assistanceRecipeRegistry.ts
tauri-svelte-preview/src/lib/shell/assistance/assistanceSchemas.ts
tauri-svelte-preview/src/lib/shell/assistance/assistanceService.ts
tauri-svelte-preview/src/lib/shell/assistance/assistanceStore.svelte.ts
tauri-svelte-preview/src/lib/shell/assistance/assistanceTypes.ts
tauri-svelte-preview/src/lib/shell/backendCapabilities.ts
tauri-svelte-preview/src/lib/shell/browser/browserAnnotations.ts
tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts
tauri-svelte-preview/src/lib/shell/browser/browserBounds.ts
tauri-svelte-preview/src/lib/shell/browser/browserChrome.ts
tauri-svelte-preview/src/lib/shell/browser/browserModel.ts
tauri-svelte-preview/src/lib/shell/browser/browserPresentation.ts
tauri-svelte-preview/src/lib/shell/browser/browserStore.svelte.ts
tauri-svelte-preview/src/lib/shell/browser/browserTypes.ts
tauri-svelte-preview/src/lib/shell/browser/normalizeBrowserUrl.ts
tauri-svelte-preview/src/lib/shell/components/ActivityBar.svelte
tauri-svelte-preview/src/lib/shell/components/BrowserPanel.svelte
tauri-svelte-preview/src/lib/shell/components/ContextPanel.svelte
tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
tauri-svelte-preview/src/lib/shell/components/DockPanel.svelte
tauri-svelte-preview/src/lib/shell/components/DonePane.svelte
tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte
tauri-svelte-preview/src/lib/shell/components/ExplorerPanel.svelte
tauri-svelte-preview/src/lib/shell/components/GitDiffView.svelte
tauri-svelte-preview/src/lib/shell/components/GitPanel.svelte
tauri-svelte-preview/src/lib/shell/components/LanguageServerStatusChip.svelte
tauri-svelte-preview/src/lib/shell/components/PalettePanel.svelte
tauri-svelte-preview/src/lib/shell/components/PanelPlaceholder.svelte
tauri-svelte-preview/src/lib/shell/components/SessionsColumn.svelte
tauri-svelte-preview/src/lib/shell/components/SessionsPaneview.svelte
tauri-svelte-preview/src/lib/shell/components/SettingsDialog.svelte
tauri-svelte-preview/src/lib/shell/components/SettingsHost.svelte
tauri-svelte-preview/src/lib/shell/components/SettledPane.svelte
tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte
tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte
tauri-svelte-preview/src/lib/shell/components/ShellSidebar.svelte
tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte
tauri-svelte-preview/src/lib/shell/components/WorkbenchActionFab.svelte
tauri-svelte-preview/src/lib/shell/components/WorkingPane.svelte
tauri-svelte-preview/src/lib/shell/components/WorktreeAgentRow.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserAnnotationCard.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserAnnotationToolbar.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserExpandedOverlay.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserFeedbackPanel.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserOverlayHost.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserTabs.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserToolbar.svelte
tauri-svelte-preview/src/lib/shell/components/browser/BrowserViewport.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/AgentCommandMenu.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/AgentConfigBar.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ApprovalItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/AssistantMessageItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/CommandItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ConversationAgentTree.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ConversationComposer.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ConversationHeader.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ConversationMessage.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ErrorItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/FileChangeItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/PlanItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ReasoningItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/SubagentItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/TaskListItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/ToolItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/UserInputItem.svelte
tauri-svelte-preview/src/lib/shell/components/conversation/UserMessageItem.svelte
tauri-svelte-preview/src/lib/shell/components/editor/editorLanguage.ts
tauri-svelte-preview/src/lib/shell/components/editor/editorPanelLanguageServer.test.ts
tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.test.ts
tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts
tauri-svelte-preview/src/lib/shell/components/explorer/FileIcon.svelte
tauri-svelte-preview/src/lib/shell/components/explorer/fileIcons.ts
tauri-svelte-preview/src/lib/shell/components/explorer/folderFileCounts.ts
tauri-svelte-preview/src/lib/shell/components/git/ChangesPane.svelte
tauri-svelte-preview/src/lib/shell/components/git/GraphPane.svelte
tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte
tauri-svelte-preview/src/lib/shell/components/git/RepoPane.svelte
tauri-svelte-preview/src/lib/shell/components/git/SourceControlPanes.svelte
tauri-svelte-preview/src/lib/shell/components/git/pr/PullRequestPanel.svelte
tauri-svelte-preview/src/lib/shell/components/git/pr/prFlow.ts
tauri-svelte-preview/src/lib/shell/components/newSession/NewSessionDialog.svelte
tauri-svelte-preview/src/lib/shell/components/newSession/NewSessionHost.svelte
tauri-svelte-preview/src/lib/shell/components/problems/ProblemsPanel.svelte
tauri-svelte-preview/src/lib/shell/components/processes/PlaywrightCard.svelte
tauri-svelte-preview/src/lib/shell/components/run/RunButton.svelte
tauri-svelte-preview/src/lib/shell/components/run/RunConfigurationDialog.svelte
tauri-svelte-preview/src/lib/shell/components/sessions/SessionCard.svelte
tauri-svelte-preview/src/lib/shell/components/sessions/sessionCardModel.test.ts
tauri-svelte-preview/src/lib/shell/components/sessions/sessionCardModel.ts
tauri-svelte-preview/src/lib/shell/components/stacks/StacksPane.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/AgentActivityPane.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/AgentHierarchy.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/AgentRuntimeInspector.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowControlCenter.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowGraph.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowLaneBoard.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowNodeInspector.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowRunHeader.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowRunList.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowTemplateEditor.svelte
tauri-svelte-preview/src/lib/shell/components/workflows/WorkflowTimeline.svelte
tauri-svelte-preview/src/lib/shell/components/worktrees/RemoveWorktreeDialog.svelte
tauri-svelte-preview/src/lib/shell/components/worktrees/WorktreeDetail.svelte
tauri-svelte-preview/src/lib/shell/components/worktrees/WorktreeManagerPane.svelte
tauri-svelte-preview/src/lib/shell/components/worktrees/WorktreeRow.svelte
tauri-svelte-preview/src/lib/shell/context/contextPanelHooks.svelte.ts
tauri-svelte-preview/src/lib/shell/context/contextService.ts
tauri-svelte-preview/src/lib/shell/context/contextStore.svelte.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationCommandCatalog.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationControls.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationMessageSafety.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationTimeline.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts
tauri-svelte-preview/src/lib/shell/devInvokeCounter.svelte.ts
tauri-svelte-preview/src/lib/shell/editor/csharpFileSystem.ts
tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts
tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts
tauri-svelte-preview/src/lib/shell/editor/editorStoreOps.ts
tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeBridge.ts
tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts
tauri-svelte-preview/src/lib/shell/editor/fixtures/mcbExtensionApiProbe.browser.cjs
tauri-svelte-preview/src/lib/shell/editor/fixtures/mcbExtensionApiProbe.ts
tauri-svelte-preview/src/lib/shell/editor/monacoWorkers.ts
tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts
tauri-svelte-preview/src/lib/shell/editor/referenceCountPersistence.ts
tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts
tauri-svelte-preview/src/lib/shell/editor/sourceRecordFromPath.ts
tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts
tauri-svelte-preview/src/lib/shell/explorer/explorerStore.svelte.ts
tauri-svelte-preview/src/lib/shell/explorer/explorerTree.ts
tauri-svelte-preview/src/lib/shell/extensions/extensionApiProbeScmAdapter.ts
tauri-svelte-preview/src/lib/shell/extensions/extensionCatalog.ts
tauri-svelte-preview/src/lib/shell/extensions/extensionRuntime.ts
tauri-svelte-preview/src/lib/shell/extensions/houston/LICENSE
tauri-svelte-preview/src/lib/shell/extensions/houston/houston.json
tauri-svelte-preview/src/lib/shell/extensions/languageContributions.ts
tauri-svelte-preview/src/lib/shell/extensions/rustGitScmProvider.ts
tauri-svelte-preview/src/lib/shell/extensions/svelte/language-configuration-start-tag.json
tauri-svelte-preview/src/lib/shell/extensions/svelte/language-configuration.json
tauri-svelte-preview/src/lib/shell/extensions/svelte/snippets/javascript.json
tauri-svelte-preview/src/lib/shell/extensions/svelte/snippets/svelte.json
tauri-svelte-preview/src/lib/shell/extensions/svelte/snippets/typescript.json
tauri-svelte-preview/src/lib/shell/extensions/svelte/svelte.tmLanguage.json
tauri-svelte-preview/src/lib/shell/git/gitBackendExtra.ts
tauri-svelte-preview/src/lib/shell/git/gitCommitFilesService.ts
tauri-svelte-preview/src/lib/shell/git/gitCommitFilesStore.svelte.ts
tauri-svelte-preview/src/lib/shell/git/gitGraphLanes.ts
tauri-svelte-preview/src/lib/shell/git/gitPanelStore.svelte.ts
tauri-svelte-preview/src/lib/shell/git/gitService.ts
tauri-svelte-preview/src/lib/shell/git/parseUnifiedDiff.ts
tauri-svelte-preview/src/lib/shell/layout/centerDock.ts
tauri-svelte-preview/src/lib/shell/layout/frame.ts
tauri-svelte-preview/src/lib/shell/layout/layoutStorage.ts
tauri-svelte-preview/src/lib/shell/layout/paneStack.ts
tauri-svelte-preview/src/lib/shell/layout/sidePaneRegistry.ts
tauri-svelte-preview/src/lib/shell/layout/sidebarViews.ts
tauri-svelte-preview/src/lib/shell/newSession/newSessionBackend.ts
tauri-svelte-preview/src/lib/shell/newSession/newSessionFlow.ts
tauri-svelte-preview/src/lib/shell/newSession/projectRootsStore.svelte.ts
tauri-svelte-preview/src/lib/shell/openFileBus.ts
tauri-svelte-preview/src/lib/shell/overlay/actionSurfaceModel.ts
tauri-svelte-preview/src/lib/shell/overlay/actionSurfaceStore.svelte.ts
tauri-svelte-preview/src/lib/shell/ownedSessions.ts
tauri-svelte-preview/src/lib/shell/palette/commandRegistry.ts
tauri-svelte-preview/src/lib/shell/panelActivation.ts
tauri-svelte-preview/src/lib/shell/problems/csharpDiagnosticsAdapter.ts
tauri-svelte-preview/src/lib/shell/problems/problemsBackend.ts
tauri-svelte-preview/src/lib/shell/problems/problemsService.ts
tauri-svelte-preview/src/lib/shell/problems/problemsStore.svelte.ts
tauri-svelte-preview/src/lib/shell/processes/playwrightBackend.ts
tauri-svelte-preview/src/lib/shell/processes/playwrightService.ts
tauri-svelte-preview/src/lib/shell/processes/playwrightStore.svelte.ts
tauri-svelte-preview/src/lib/shell/processes/processBackend.ts
tauri-svelte-preview/src/lib/shell/projectSourceIndex.ts
tauri-svelte-preview/src/lib/shell/relativeTime.ts
tauri-svelte-preview/src/lib/shell/resources/ResourcePopover.svelte
tauri-svelte-preview/src/lib/shell/resources/ResourcesWorkspace.svelte
tauri-svelte-preview/src/lib/shell/resources/WorkspaceSpaceWorkspace.svelte
tauri-svelte-preview/src/lib/shell/resources/resourceBackend.ts
tauri-svelte-preview/src/lib/shell/resources/resourceService.ts
tauri-svelte-preview/src/lib/shell/resources/resourceStore.svelte.ts
tauri-svelte-preview/src/lib/shell/resources/resourceTypes.ts
tauri-svelte-preview/src/lib/shell/resources/resourceViewModel.ts
tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceService.ts
tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceStore.svelte.ts
tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceTypes.ts
tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceViewModel.ts
tauri-svelte-preview/src/lib/shell/sessionGroups.ts
tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionContextMenu.svelte
tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte
tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryContextMenu.ts
tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryModel.ts
tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryNavigation.ts
tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts
tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryStore.svelte.ts
tauri-svelte-preview/src/lib/shell/sessionStrip.ts
tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts
tauri-svelte-preview/src/lib/shell/shellCommands.ts
tauri-svelte-preview/src/lib/shell/shellPanels.ts
tauri-svelte-preview/src/lib/shell/stacks/stackService.ts
tauri-svelte-preview/src/lib/shell/stacks/stackStore.svelte.ts
tauri-svelte-preview/src/lib/shell/stores/sessionRailStore.svelte.ts
tauri-svelte-preview/src/lib/shell/styles/next.css
tauri-svelte-preview/src/lib/shell/styles/nextTokens.css
tauri-svelte-preview/src/lib/shell/styles/themeChrome.css
tauri-svelte-preview/src/lib/shell/terminalService.ts
tauri-svelte-preview/src/lib/shell/themes/contrast.ts
tauri-svelte-preview/src/lib/shell/themes/themeRegistry.ts
tauri-svelte-preview/src/lib/shell/themes/themeService.ts
tauri-svelte-preview/src/lib/shell/usage/UsagePopover.svelte
tauri-svelte-preview/src/lib/shell/usage/UsageWorkspace.svelte
tauri-svelte-preview/src/lib/shell/usage/usageAnalytics.ts
tauri-svelte-preview/src/lib/shell/usage/usageBackend.ts
tauri-svelte-preview/src/lib/shell/usage/usageCurrent.ts
tauri-svelte-preview/src/lib/shell/usage/usageService.ts
tauri-svelte-preview/src/lib/shell/usage/usageStore.svelte.ts
tauri-svelte-preview/src/lib/shell/usage/usageTypes.ts
tauri-svelte-preview/src/lib/shell/workflows/workflowService.ts
tauri-svelte-preview/src/lib/shell/workflows/workflowStore.svelte.ts
tauri-svelte-preview/src/lib/shell/workflows/workflowTypes.ts
tauri-svelte-preview/src/lib/shell/worktrees/worktreeManagerRows.ts
tauri-svelte-preview/src/lib/shell/worktrees/worktreeManagerService.ts
tauri-svelte-preview/src/lib/shell/worktrees/worktreeManagerStore.svelte.ts
tauri-svelte-preview/src/lib/shell/worktrees/worktreesBackend.ts
tauri-svelte-preview/src/lib/shell/xtermFactory.ts
tauri-svelte-preview/src/routes/next/+page.svelte

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte | sed -n '1,280p'
printf '\\n--- centerDock ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/layout/centerDock.ts | sed -n '1,320p'
printf '\\n--- next page ---\\n'
nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '1,200p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	<script lang="ts">
     2	  /**
     3	   * ShellFrame.svelte — mounts the Gridview root + center Dockview and
     4	   * teleports Svelte-owned content into them. All content is authored in the
     5	   * hidden parking stage below, so dockview NEVER owns app DOM; if the frame
     6	   * fails to mount, content simply stays parked (invisible) and the page's
     7	   * error rail reports it. No backend IO anywhere in this component.
     8	   */
     9	  import 'dockview-core/dist/styles/dockview.css';
    10	  import { onMount, type Snippet } from 'svelte';
    11	
    12	  import {
    13	    createCenterDock,
    14	    type CenterDock,
    15	    type CenterDockSnapshot
    16	  } from '$lib/shell/layout/centerDock';
    17	  import {
    18	    createShellFrame,
    19	    type RegionHeightLimits,
    20	    type RegionWidthLimits,
    21	    type ShellFrame as Frame,
    22	    type ShellRegionId
    23	  } from '$lib/shell/layout/frame';
    24	
    25	  interface Props {
    26	    /** The left column: the sessions list, and nothing else. */
    27	    sessions: Snippet;
    28	    center: {
    29	      session: Snippet;
    30	      editor: Snippet;
    31	      browser: Snippet;
    32	      diff: Snippet;
    33	      sessionLibrary: Snippet;
    34	      agents: Snippet;
    35	    };
    36	    /** The right column: whichever tool view the icon strip has open. */
    37	    tools: Snippet;
    38	    /** The icon strip on the far right edge, which picks that view. */
    39	    activity: Snippet;
    40	    dock: Snippet;
    41	    onSessionPanelLayout?: () => void;
    42	    /** A center tab came to the front. Fires for the dock's own start-up
    43	     * announcements too — see the note in `centerDock.ts`. */
    44	    onCenterPanelShown?: (id: string) => void;
    45	    onReady?: (controls: {
    46	      resetLayout: () => void;
    47	      showCenterPanel: (id: string) => void;
    48	      captureCenterLayout: () => CenterDockSnapshot | null;
    49	      restoreCenterLayout: (snapshot: CenterDockSnapshot | null | undefined) => void;
    50	      /** Give one region a width — how a column asks to be folded up or
    51	       * opened out. See `setRegionWidth` in `frame.ts`. */
    52	      setRegionWidth: (id: ShellRegionId, width: number, limits?: RegionWidthLimits) => void;
    53	      /** Give one region a height — how the bottom strip is closed when the
    54	       * Problems list has been moved elsewhere or hidden. See
    55	       * `setRegionHeight` in `frame.ts`. */
    56	      setRegionHeight: (id: ShellRegionId, height: number, limits?: RegionHeightLimits) => void;
    57	      /** Say what a region may be dragged to without moving it. See
    58	       * `setRegionLimits` in `frame.ts`. */
    59	      setRegionLimits: (id: ShellRegionId, limits: RegionWidthLimits) => void;
    60	      /** How wide a region is right now, or null if the frame is gone. */
    61	      regionWidth: (id: ShellRegionId) => number | null;
    62	    }) => void;
    63	    onError?: (message: string) => void;
    64	  }
    65	  let {
    66	    sessions,
    67	    center,
    68	    tools,
    69	    activity,
    70	    dock,
    71	    onSessionPanelLayout,
    72	    onCenterPanelShown,
    73	    onReady,
    74	    onError
    75	  }: Props = $props();
    76	
    77	  let gridHost: HTMLElement;
    78	  let sessionsSlot: HTMLElement;
    79	  let centerSlot: HTMLElement; // holds the center Dockview's own container
    80	  let toolsSlot: HTMLElement;
    81	  let activitySlot: HTMLElement;
    82	  let dockSlot: HTMLElement;
    83	  let sessionSlot: HTMLElement;
    84	  let editorSlot: HTMLElement;
    85	  let browserSlot: HTMLElement;
    86	  let diffSlot: HTMLElement;
    87	  let sessionLibrarySlot: HTMLElement;
    88	  let agentsSlot: HTMLElement;
    89	
    90	  let frame: Frame | null = null;
    91	  let centerDock: CenterDock | null = null;
    92	  let ready = $state(false);
    93	
    94	  onMount(() => {
    95	    let observer: ResizeObserver | null = null;
    96	    try {
    97	      frame = createShellFrame(gridHost, {
    98	        storage: window.localStorage,
    99	        regions: {
   100	          sessions: sessionsSlot,
   101	          center: centerSlot,
   102	          tools: toolsSlot,
   103	          activity: activitySlot,
   104	          dock: dockSlot
   105	        }
   106	      });
   107	      // Lay out the parent Gridview before the center Dockview restores or
   108	      // builds its panels. Center Dockview's restore path deliberately measures
   109	      // its host before calling `fromJSON`; doing that while the Gridview is
   110	      // still at 0×0 leaves always-rendered panels (including Monaco's diff
   111	      // editor) with stale overlay bounds until the next activation.
   112	      frame.layout(gridHost.clientWidth, gridHost.clientHeight);
   113	      centerDock = createCenterDock(centerSlot, {
   114	        storage: window.localStorage,
   115	        // The session is what you talk to, so it opens on its own on the left;
   116	        // the editor, the browser and the diff are where you look at the
   117	        // result, so they open stacked together on the right. Source control
   118	        // itself is not here at all — it is a view of the tool column — but the
   119	        // changes it shows are, because a diff wants the width of the middle.
   120	        panels: [
   121	          { id: 'session', title: 'Session', element: sessionSlot },
   122	          { id: 'editor', title: 'Editor', element: editorSlot, group: 'display' },
   123	          { id: 'browser', title: 'Browser', element: browserSlot, group: 'display' },
   124	          { id: 'diff', title: 'Diff', element: diffSlot, group: 'display' },
   125	          {
   126	            id: 'session-library',
   127	            title: 'Session Library',
   128	            element: sessionLibrarySlot,
   129	            group: 'display'
   130	          },
   131	          { id: 'agents', title: 'Agents', element: agentsSlot, group: 'display' }
   132	        ],
   133	        onPanelLayout: (id) => {
   134	          if (id === 'session') onSessionPanelLayout?.();
   135	        },
   136	        onPanelActivated: (id) => onCenterPanelShown?.(id)
   137	      });
   138	      observer = new ResizeObserver(() => {
   139	        frame?.layout(gridHost.clientWidth, gridHost.clientHeight);
   140	      });
   141	      observer.observe(gridHost);
   142	      ready = true;
   143	      onReady?.({
   144	        resetLayout: () => {
   145	          frame?.resetLayout();
   146	          centerDock?.resetLayout();
   147	        },
   148	        showCenterPanel: (id: string) => centerDock?.activatePanel(id),
   149	        captureCenterLayout: () => centerDock?.captureLayout() ?? null,
   150	        restoreCenterLayout: (snapshot) => centerDock?.restoreLayout(snapshot),
   151	        setRegionWidth: (id, width, limits) => frame?.setRegionWidth(id, width, limits),
   152	        setRegionHeight: (id, height, limits) => frame?.setRegionHeight(id, height, limits),
   153	        setRegionLimits: (id, limits) => frame?.setRegionLimits(id, limits),
   154	        regionWidth: (id) => frame?.regionWidth(id) ?? null
   155	      });
   156	    } catch (error) {
   157	      onError?.(error instanceof Error ? error.message : String(error));
   158	    }
   159	
   160	    return () => {
   161	      observer?.disconnect();
   162	      centerDock?.dispose();
   163	      centerDock = null;
   164	      frame?.dispose();
   165	      frame = null;
   166	    };
   167	  });
   168	</script>
   169	
   170	<div class="shell-frame" class:ready bind:this={gridHost}></div>
   171	
   172	<!-- Parking stage: content lives here until (and unless) dockview claims it, and
   173	     whenever a panel hands it back. Not rendered at all, so nothing parked here
   174	     can be measured — see the note on `.parking-stage` in the styles below. -->
   175	<div class="parking-stage" aria-hidden="true">
   176	  <div class="slot" bind:this={sessionsSlot}>{@render sessions()}</div>
   177	  <div class="slot slot-center-dock" bind:this={centerSlot}></div>
   178	  <div class="slot" bind:this={toolsSlot}>{@render tools()}</div>
   179	  <div class="slot" bind:this={activitySlot}>{@render activity()}</div>
   180	  <div class="slot" bind:this={dockSlot}>{@render dock()}</div>
   181	  <div class="slot" bind:this={sessionSlot}>{@render center.session()}</div>
   182	  <div class="slot" bind:this={editorSlot}>{@render center.editor()}</div>
   183	  <div class="slot" bind:this={browserSlot}>{@render center.browser()}</div>
   184	  <div class="slot" bind:this={diffSlot}>{@render center.diff()}</div>
   185	  <div class="slot" bind:this={sessionLibrarySlot}>{@render center.sessionLibrary()}</div>
   186	  <div class="slot" bind:this={agentsSlot}>{@render center.agents()}</div>
   187	</div>
   188	
   189	<style>
   190	  .shell-frame {
   191	    height: 100%;
   192	    width: 100%;
   193	    overflow: hidden;
   194	    visibility: hidden; /* anti-flash: revealed when ready */
   195	  }
   196	  .shell-frame.ready {
   197	    visibility: visible;
   198	  }
   199	
   200	  /* `display: none`, NOT a 1px hidden box: parked content must be UNMEASURABLE.
   201	     A 1px stage still has a size, and a terminal parked in one measures its host
   202	     at zero — xterm's fit addon clamps that to 2 columns by 1 row and resizes the
   203	     real PTY to it, reflowing the agent's screen. With `display: none` the
   204	     computed height is `auto`, the fit addon proposes NaN and returns without
   205	     touching the grid. Content renders normally the moment it is moved out. */
   206	  .parking-stage {
   207	    display: none;
   208	  }
   209	
   210	  /* Teleported slots and hosts must fill whatever cell they land in. */
   211	  .slot,
   212	  :global(.shell-region-host),
   213	  :global(.center-panel-host) {
   214	    height: 100%;
   215	    width: 100%;
   216	    min-width: 0;
   217	    min-height: 0;
   218	    overflow: hidden;
   219	  }
   220	
   221	  /* ---- dockview overrides (ported from the old shell's audited CSS) ----
   222	     BOTH theme classes must be targeted where dracula reasserts values.   */
   223	  .shell-frame :global(.shell-grid),
   224	  .shell-frame :global(.shell-center-dock) {
   225	    /* token map: translate dockview vars onto the /next palette */
   226	    --dv-group-view-background-color: var(--color-bg);
   227	    --dv-tabs-and-actions-container-background-color: var(--color-bg);
   228	    --dv-activegroup-visiblepanel-tab-background-color: var(--color-surface);
   229	    --dv-activegroup-hiddenpanel-tab-background-color: var(--color-bg);
   230	    --dv-inactivegroup-visiblepanel-tab-background-color: var(--color-tab-unfocused-surface);
   231	    --dv-inactivegroup-hiddenpanel-tab-background-color: var(--color-bg);
   232	    --dv-tab-divider-color: transparent;
   233	    --dv-tabs-and-actions-container-height: 35px;
   234	    --dv-activegroup-visiblepanel-tab-color: var(--color-text);
   235	    --dv-activegroup-hiddenpanel-tab-color: var(--color-text-2);
   236	    --dv-inactivegroup-visiblepanel-tab-color: var(--color-tab-unfocused-text);
   237	    --dv-inactivegroup-hiddenpanel-tab-color: var(--color-text-2);
   238	    --dv-separator-border: var(--color-border);
   239	    --dv-paneview-active-outline-color: transparent;
   240	    /* NEUTRAL wash only — the teal accent here caused the green drag-flash. */
   241	    --dv-drag-over-background-color: rgba(255, 255, 255, 0.05);
   242	    --dv-drag-over-border-color: transparent;
   243	    /* VS Code-style resize feedback: the divider lights up teal on hover/drag.
   244	       Safe to color, unlike the drag-over wash above — this paints only the
   245	       sash line itself, never the panel content. Delay 0 so it appears the
   246	       moment you grab it, not half a second in. */
   247	    --dv-active-sash-color: var(--color-accent);
   248	    --dv-active-sash-transition-delay: 0.1s;
   249	    --dv-active-sash-transition-duration: 0.05s;
   250	  }
   251	
   252	  /* seam flattening */
   253	  .shell-frame :global(.dv-groupview),
   254	  .shell-frame :global(.dv-tabs-and-actions-container),
   255	  .shell-frame :global(.dv-content-container),
   256	  .shell-frame :global(.dv-tabs-container),
   257	  .shell-frame :global(.dv-tab) {
   258	    border-color: transparent;
   259	    box-shadow: none;
   260	  }
   261	
   262	  /* The six center tabs are permanent navigation. A restored Dockview layout
   263	     must never collapse their strip to zero height. */
   264	  .shell-frame :global(.shell-center-dock .dv-tabs-and-actions-container) {
   265	    min-height: 35px;
   266	    visibility: visible;
   267	  }
   268	
   269	  /* Every center tab is permanent this slice (a close is undone on the next
   270	     tick to protect the live terminal), so showing a close button would be a
   271	     lie. Remove this rule when closable tabs become real. */
   272	  .shell-frame :global(.shell-center-dock .dv-default-tab-action) {
   273	    display: none;
   274	  }
   275	
   276	  /* sizing chain — without this, dockview panels collapse in flex/grid parents */
   277	  .shell-frame :global(.dv-dockview),
   278	  .shell-frame :global(.dv-gridview),
   279	  .shell-frame :global(.dv-grid-view),
   280	  .shell-frame :global(.dv-branch-node),

--- centerDock ---
     1	/**
     2	 * centerDock.ts — the /next center tab area (one Dockview). DOM-only: zero
     3	 * backend IO, zero Svelte imports. Same teleport contract as frame.ts, plus
     4	 * an explicit "return to parking" on panel close so Svelte-owned content
     5	 * (the terminal surface!) is never destroyed with a dockview renderer.
     6	 */
     7	import {
     8	  createDockview,
     9	  themeDracula,
    10	  type AddPanelPositionOptions,
    11	  type DockviewApi,
    12	  type GroupPanelPartInitParameters,
    13	  type IContentRenderer
    14	} from 'dockview-core';
    15	
    16	import {
    17	  CENTER_LAYOUT_KEY,
    18	  CENTER_LAYOUT_KEY_V4,
    19	  clearLayout,
    20	  dockPanelIds,
    21	  loadLayout,
    22	  panelSetMatches,
    23	  saveLayout,
    24	  type LayoutStorage
    25	} from './layoutStorage';
    26	
    27	export interface CenterPanelSpec {
    28	  id: string;
    29	  title: string;
    30	  element: HTMLElement;
    31	  /**
    32	   * Which side of the default layout this panel opens on: `'conversation'` (the
    33	   * left group, where the user talks to the session) or `'display'` (the right
    34	   * group, where results are shown). Only read when the dock is built from
    35	   * scratch — once the user has dragged tabs around, the stored layout wins.
    36	   * Defaults to `'conversation'`.
    37	   */
    38	  group?: 'conversation' | 'display';
    39	}
    40	
    41	export interface CenterDockOptions {
    42	  storage: LayoutStorage;
    43	  panels: CenterPanelSpec[];
    44	  onPanelLayout?: (id: string) => void;
    45	  /**
    46	   * The tab the user is looking at changed. Dockview delivers this through a
    47	   * microtask, so the events caused by building or restoring the dock arrive
    48	   * shortly after `createCenterDock` returns — a listener that loads data must
    49	   * ignore those and start honouring the signal one timer tick later. See the
    50	   * note on `runSynchronized` below for why a microtask cannot be told apart
    51	   * from a real click any other way.
    52	   */
    53	  onPanelActivated?: (id: string) => void;
    54	  onLayoutPersisted?: (ok: boolean) => void;
    55	}
    56	
    57	export interface CenterDock {
    58	  api: DockviewApi;
    59	  activatePanel(id: string): void;
    60	  captureLayout(): CenterDockSnapshot | null;
    61	  restoreLayout(snapshot: CenterDockSnapshot | null | undefined): void;
    62	  resetLayout(): void;
    63	  dispose(): void;
    64	}
    65	
    66	export interface CenterDockSnapshot {
    67	  activePanelId: string | null;
    68	  layout: object;
    69	}
    70	
    71	const COMPONENT = 'center-panel';
    72	const PERSIST_DEBOUNCE_MS = 250;
    73	const CENTER_PANEL_IDS_V4 = ['session', 'editor', 'browser', 'diff', 'session-library'] as const;
    74	
    75	export function createCenterDock(container: HTMLElement, options: CenterDockOptions): CenterDock {
    76	  const specs = new Map(options.panels.map((panel) => [panel.id, panel]));
    77	  /** Where each element goes back to when its dockview panel dies. */
    78	  const parking = new Map(
    79	    options.panels.map((panel) => [panel.id, panel.element.parentElement as HTMLElement | null])
    80	  );
    81	  let synchronizingDepth = 0;
    82	  let persistTimer: ReturnType<typeof setTimeout> | null = null;
    83	  let disposed = false;
    84	  /** Roster tabs whose re-add is already queued, so a burst cannot double-add. */
    85	  const readding = new Set<string>();
    86	
    87	  const createRenderer = (): IContentRenderer => {
    88	    const element = document.createElement('div');
    89	    element.className = 'center-panel-host';
    90	    let panelId: string | null = null;
    91	    return {
    92	      element,
    93	      init(parameters: GroupPanelPartInitParameters): void {
    94	        const candidate = parameters.params?.panelId;
    95	        if (typeof candidate !== 'string' || !specs.has(candidate)) {
    96	          element.textContent = `Unknown panel: ${String(candidate)}`;
    97	          return;
    98	        }
    99	        panelId = candidate;
   100	        const content = specs.get(candidate)!.element;
   101	        if (content.parentElement !== element) element.replaceChildren(content);
   102	      },
   103	      layout(): void {
   104	        if (panelId) options.onPanelLayout?.(panelId);
   105	      },
   106	      dispose(): void {
   107	        if (disposed || !panelId) return;
   108	        // Panel closed while the shell lives on: hand the content back to its
   109	        // Svelte-owned parking node so nothing app-owned dies with dockview.
   110	        const content = specs.get(panelId)?.element;
   111	        const home = parking.get(panelId) ?? null;
   112	        if (content && home && content.parentElement !== home) home.appendChild(content);
   113	      }
   114	    };
   115	  };
   116	
   117	  const api = createDockview(container, {
   118	    className: 'shell-center-dock',
   119	    defaultRenderer: 'always',
   120	    dndStrategy: 'pointer',
   121	    hideBorders: true,
   122	    scrollbars: 'native',
   123	    theme: { ...themeDracula, gap: 0 },
   124	    createComponent: createRenderer
   125	  });
   126	
   127	  /**
   128	   * Add one roster panel. With no `position` dockview puts it in whichever group
   129	   * is active (and makes a group if the dock is empty) — that is what a re-add
   130	   * after an accidental close wants, and it is the fallback whenever there is no
   131	   * existing panel to anchor against.
   132	   */
   133	  const addPanelFor = (panel: CenterPanelSpec, position?: AddPanelPositionOptions): void => {
   134	    api.addPanel({
   135	      id: panel.id,
   136	      // Keep the persisted panel id/registration stable while giving the
   137	      // dedicated history experience the title users see in the tab.
   138	      title: panel.id === 'session-library' ? 'Session History' : panel.title,
   139	      component: COMPONENT,
   140	      params: { panelId: panel.id },
   141	      position
   142	    });
   143	  };
   144	
   145	  /** Panel-id lookup. `api.getPanel(id)` also does this correctly — but the
   146	   * naming is INVERTED between dockview's layers (`DockviewApi.getPanel` calls
   147	   * the component's `getGroupPanel`, while `DockviewApi.getGroup` calls the
   148	   * component's `getPanel`, which looks up groups), and that trap has already
   149	   * produced one confident misreading in each direction during review. The
   150	   * explicit form below is the exact body of `getGroupPanel` and of
   151	   * `addPanel`'s own duplicate guard, so what we test is what dockview does. */
   152	  const panelById = (id: string) => api.panels.find((panel) => panel.id === id);
   153	
   154	  /** Existing serialized layouts may still carry the former tab label. */
   155	  const normalizeSessionLibraryTitle = (): void => {
   156	    panelById('session-library')?.api.setTitle('Session History');
   157	  };
   158	
   159	  /**
   160	   * The layout a fresh dock opens with: the conversation panels on the left, the
   161	   * display panels stacked as tabs in a second group to their right. Both sides
   162	   * may be empty — a roster that is all conversation just never opens the second
   163	   * group, and one that is all display opens a single group with no anchor to
   164	   * sit beside (asking dockview to position against a panel that does not exist
   165	   * yet throws, so that case adds the first panel with no position at all).
   166	   */
   167	  const buildDefault = (): void => {
   168	    const conversation = options.panels.filter((panel) => panel.group !== 'display');
   169	    const display = options.panels.filter((panel) => panel.group === 'display');
   170	    for (const panel of conversation) addPanelFor(panel);
   171	
   172	    const anchor = conversation[0];
   173	    const [leadDisplay, ...stackedDisplay] = display;
   174	    if (leadDisplay) {
   175	      addPanelFor(
   176	        leadDisplay,
   177	        anchor ? { referencePanel: anchor.id, direction: 'right' } : undefined
   178	      );
   179	      for (const panel of stackedDisplay) {
   180	        addPanelFor(panel, { referencePanel: leadDisplay.id, direction: 'within' });
   181	      }
   182	    }
   183	
   184	    // Every add above made its own panel the active one, so the last display tab
   185	    // is showing — which is now Diff, an empty pane until a file is picked. Put
   186	    // the display group back on its first tab, then hand focus to the front of
   187	    // the roster.
   188	    if (leadDisplay) panelById(leadDisplay.id)?.api.setActive();
   189	    const opening = anchor ?? leadDisplay;
   190	    if (opening) panelById(opening.id)?.api.setActive();
   191	  };
   192	
   193	  /**
   194	   * Run a programmatic layout mutation with persistence and the roster rebuild
   195	   * below both suppressed.
   196	   *
   197	   * dockview reports layout changes through `queueMicrotask` (its `AsapEvent`),
   198	   * so the events this block causes are delivered AFTER it returns — a flag
   199	   * cleared synchronously is already down when they land, which is why the
   200	   * previous version of this guard never suppressed anything. Releasing it on a
   201	   * timer instead is what makes it real: the whole microtask queue (including
   202	   * microtasks queued by other microtasks) drains before any timer callback
   203	   * runs, so every event delivered that way lands while the guard is still up.
   204	   *
   205	   * That covers the microtask channel only, which is the one a programmatic
   206	   * mutation uses. Resize-driven changes are delivered separately, through a
   207	   * `requestAnimationFrame` inside dockview's own resize watcher, and those
   208	   * deliberately fall outside the guard: they report a finished layout the user
   209	   * asked for, which is exactly what we want written.
   210	   */
   211	  const runSynchronized = (fn: () => void): void => {
   212	    synchronizingDepth += 1;
   213	    try {
   214	      fn();
   215	    } finally {
   216	      setTimeout(() => {
   217	        if (synchronizingDepth > 0) synchronizingDepth -= 1;
   218	      }, 0);
   219	    }
   220	  };
   221	
   222	  /**
   223	   * The six roster tabs are permanent for now. dockview puts a close button on
   224	   * every tab, and closing one hands its content back to the parking stage with
   225	   * no way left in the UI to bring it back — for the Session tab that is a LIVE
   226	   * terminal. Put the tab straight back instead, so the close button is a no-op.
   227	   *
   228	   * Deferred by a timer on purpose: dockview fires this event from
   229	   * `doRemovePanel`, BEFORE it disposes the panel, and that dispose is what
   230	   * returns the content to parking. Re-adding synchronously would be undone a
   231	   * moment later, stranding the content for good.
   232	   */
   233	  const keepRosterPanel = (id: string): void => {
   234	    if (disposed || synchronizingDepth > 0 || readding.has(id)) return;
   235	    if (!specs.has(id)) return;
   236	    readding.add(id);
   237	    setTimeout(() => {
   238	      readding.delete(id);
   239	      const spec = specs.get(id);
   240	      // Duplicate-add guard: tests the exact condition `addPanel` itself
   241	      // throws on (see the `panelById` note above).
   242	      if (disposed || synchronizingDepth > 0 || !spec || panelById(id)) return;
   243	      try {
   244	        addPanelFor(spec);
   245	      } catch {
   246	        // Nothing here can recover a dock that refuses the panel, and an
   247	        // uncaught throw in a timer callback goes nowhere useful. The tab stays
   248	        // shut; "Reset layout" rebuilds the roster.
   249	      }
   250	    }, 0);
   251	  };
   252	
   253	  /**
   254	   * Size the dock to its container BEFORE anything is restored or built, for the
   255	   * same reason as the grid in `frame.ts`: dockview starts at 0 x 0, and a
   256	   * restore at that size records squashed proportions that the first real layout
   257	   * then scales up wrong. A container with no size yet is skipped — dockview's
   258	   * own resize observer delivers the first real layout, and `persistSoon`
   259	   * refuses to write a dock that has never had a real size.
   260	   */
   261	  const layoutToContainer = (): void => {
   262	    const width = container.clientWidth;
   263	    const height = container.clientHeight;
   264	    if (width > 0 && height > 0) api.layout(width, height);
   265	  };
   266	
   267	  layoutToContainer();
   268	
   269	  let migratedFromV4 = false;
   270	
   271	  runSynchronized(() => {
   272	    const stored = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY);
   273	    if (stored && panelSetMatches(dockPanelIds(stored), specs.keys())) {
   274	      try {
   275	        api.fromJSON(stored as never);
   276	        return;
   277	      } catch {
   278	        try {
   279	          api.clear();
   280	        } catch {
   281	          // fall through
   282	        }
   283	      }
   284	    }
   285	
   286	    /**
   287	     * v4 has the same arrangement as the current roster minus Agents. Restore
   288	     * that exact set first, then add the new tab beside Session Library through
   289	     * the live Dockview API. This preserves the serialized groups/sizes/order
   290	     * and avoids hand-editing Dockview's private grid tree. Any mismatch or
   291	     * Dockview rejection falls through to the safe current defaults.
   292	     */
   293	    const previous = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY_V4);
   294	    if (previous && panelSetMatches(dockPanelIds(previous), CENTER_PANEL_IDS_V4)) {
   295	      try {
   296	        api.fromJSON(previous as never);
   297	        const activePanelId = api.activePanel?.id ?? null;
   298	        const library = specs.get('session-library');
   299	        const agents = specs.get('agents');
   300	        if (!library || !agents || !panelById('session-library')) {
   301	          throw new Error('Agents migration roster is incomplete');
   302	        }
   303	        addPanelFor(agents, { referencePanel: 'session-library', direction: 'within' });
   304	        if (activePanelId) panelById(activePanelId)?.api.setActive();
   305	        migratedFromV4 = true;
   306	        return;
   307	      } catch {
   308	        try {
   309	          api.clear();
   310	        } catch {
   311	          // fall through
   312	        }
   313	      }
   314	    }
   315	
   316	    buildDefault();
   317	  });
   318	  normalizeSessionLibraryTitle();
   319	
   320	  const persistSoon = (): void => {

--- next page ---
     1	<script lang="ts">
     2	  /**
     3	   * /next — the shell orchestrator. Thin by construction: it owns the terminal
     4	   * sessions and nothing else. Every other panel brings its own state and its
     5	   * own loader; this page only says which project they are pointed at and when
     6	   * they may start — see `shellPanels.ts` and `panelActivation.ts`.
     7	   *
     8	   * IO lives **only in explicit functions**, never in an `$effect`. Launch IO is
     9	   * still just the rail: list surviving PTYs, reconcile them against the stored
    10	   * owned sessions, re-attach them (live AND tombstones), scan for resumable
    11	   * agents. `hydrateOwned` runs AFTER `reconcileOwnedSessions`.
    12	   */
    13	  import { onMount, tick } from 'svelte';
    14	
    15	  import ExternalLink from '@lucide/svelte/icons/external-link';
    16	  import ImageGlyph from '@lucide/svelte/icons/image';
    17	  import Maximize2 from '@lucide/svelte/icons/maximize-2';
    18	  import Minimize2 from '@lucide/svelte/icons/minimize-2';
    19	  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
    20	  import PanelBottomOpen from '@lucide/svelte/icons/panel-bottom-open';
    21	  import Pencil from '@lucide/svelte/icons/pencil';
    22	  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
    23	
    24	  import '$lib/shell/styles/nextTokens.css';
    25	  /* Tailwind + the shadcn component variables. Imported HERE and nowhere else:
    26	     the old shell shares `app.css` with this page and must keep rendering
    27	     exactly as it does today, so this file must never reach that route. */
    28	  import '$lib/shell/styles/next.css';
    29	  /* Four colours the dock needs that the shared token file has no name for.
    30	     A stylesheet rather than something the theme service sets, because the dock
    31	     is painted on the first frame, before any theme has been applied. */
    32	  import '$lib/shell/styles/themeChrome.css';
    33	
    34	  import ActivityBar from '$lib/shell/components/ActivityBar.svelte';
    35	  import BrowserPanel from '$lib/shell/components/BrowserPanel.svelte';
    36	  import {
    37	    activateBrowser,
    38	    browser,
    39	    browserBackend,
    40	    browserModelContext,
    41	    browserWorkspace,
    42	    captureBrowserState,
    43	    captureBrowserView,
    44	    clearBrowserError,
    45	    reloadBrowserFrame,
    46	    restoreBrowserState,
    47	    setBrowserPresentation,
    48	    setBrowserUrl,
    49	    setBrowserViewportPreset,
    50	    stageBrowserFeedback,
    51	    syncBrowserTab
    52	  } from '$lib/shell/browser/browserStore.svelte';
    53	  import {
    54	    beginBrowserElementPicker,
    55	    cancelBrowserAnnotation,
    56	    closeBrowserTab,
    57	    collapseBrowserToControl,
    58	    createBrowserTab,
    59	    expandBrowserFrom,
    60	    minimizeBrowserToPrevious,
    61	    removeBrowserAnnotation,
    62	    restoreBrowserToDock
    63	  } from '$lib/shell/browser/browserModel.ts';
    64	  import type {
    65	    BrowserFeedbackAttachment,
    66	    BrowserPresentationMode,
    67	    BrowserViewportPreset
    68	  } from '$lib/shell/browser/browserTypes.ts';
    69	  import DockPanel from '$lib/shell/components/DockPanel.svelte';
    70	  import EditorPanel from '$lib/shell/components/EditorPanel.svelte';
    71	  import GitDiffView from '$lib/shell/components/GitDiffView.svelte';
    72	  import SessionsColumn from '$lib/shell/components/SessionsColumn.svelte';
    73	  import ShellFrame from '$lib/shell/components/ShellFrame.svelte';
    74	  import ShellOverlays from '$lib/shell/components/ShellOverlays.svelte';
    75	  import ShellSidebar from '$lib/shell/components/ShellSidebar.svelte';
    76	  import ConversationSurface from '$lib/shell/components/ConversationSurface.svelte';
    77	  import RunButton from '$lib/shell/components/run/RunButton.svelte';
    78	  import SessionLibraryWorkspace from '$lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte';
    79	  import WorkflowControlCenter from '$lib/shell/components/workflows/WorkflowControlCenter.svelte';
    80	  import { settings, type ProblemsLocation } from '$lib/settingsStore.svelte';
    81	  import { setContextPanelHooks } from '$lib/shell/context/contextPanelHooks.svelte';
    82	  import { countInvoke } from '$lib/shell/devInvokeCounter.svelte';
    83	  import {
    84	    captureConversationWorkspace,
    85	    getConversationSession,
    86	    removeConversationSession,
    87	    restoreConversationWorkspace,
    88	    setConversationMode
    89	  } from '$lib/shell/conversation/conversationStore.svelte';
    90	  import {
    91	    closeStructuredConversation,
    92	    commitConversationHandoff,
    93	    ensureStructuredConversation,
    94	    prepareConversationHandoff,
    95	    rollbackConversationHandoff,
    96	    startConversationEvents,
    97	    startConversationTerminalProjection,
    98	    stopConversationTerminalProjection,
    99	    stopConversationEvents
   100	  } from '$lib/shell/conversation/conversationService';
   101	  import type {
   102	    AgentConversationHandoffDirection,
   103	    AgentConversationHandoffMode,
   104	    AgentConversationProvider
   105	  } from '$lib/shell/conversation/conversationTypes';
   106	  import {
   107	    editorState,
   108	    resetEditorState,
   109	    restoreEditorFiles,
   110	    type OpenEditorFile
   111	  } from '$lib/shell/editor/editorStore.svelte';
   112	  import { setCsharpLanguageServerEnabled } from '$lib/shell/editor/sourceIntelligence';
   113	  import {
   114	    configureExtensionApiProbeRuntime,
   115	    disposeExtensionApiProbeRuntime,
   116	    onExtensionApiProbeObservation,
   117	    setExtensionApiProbeWorkspace,
   118	    type ExtensionApiProbeObservation
   119	  } from '$lib/shell/editor/extensionApiProbeController';
   120	  import { explorer, selectPath, setScrollTop } from '$lib/shell/explorer/explorerStore.svelte';
   121	  import { gitPanel } from '$lib/shell/git/gitPanelStore.svelte';
   122	  import { gitService } from '$lib/shell/git/gitService';
   123	  import {
   124	    SESSIONS_MAX_WIDTH,
   125	    SESSIONS_MIN_WIDTH,
   126	    SESSIONS_STRIP_WIDTH,
   127	    SESSIONS_WIDTH,
   128	    type RegionHeightLimits,
   129	    type RegionWidthLimits,
   130	    type ShellRegionId
   131	  } from '$lib/shell/layout/frame';
   132	  import type { CenterDockSnapshot } from '$lib/shell/layout/centerDock';
   133	  import { DEFAULT_SIDEBAR_VIEW, type SidebarViewId } from '$lib/shell/layout/sidebarViews';
   134	  import type { NewSessionRequest } from '$lib/shell/newSession/newSessionFlow';
   135	  import { requestOpenFile } from '$lib/shell/openFileBus';
   136	  import {
   137	    adoptAgentSession,
   138	    createFreshSession,
   139	    reconcileOwnedSessions
   140	  } from '$lib/shell/ownedSessions';
   141	  import {
   142	    captureWorkspace,
   143	    diffPathFor,
   144	    emptyRetainedWorkspaces,
   145	    planWorkspaceRestore,
   146	    pruneWorkspaces,
   147	    readWorkspaces,
   148	    retainTabs,
   149	    takeRetainedTabs,
   150	    writeWorkspaces,
   151	    type RetainedWorkspaces,
   152	    type SessionWorkspaceSnapshot
   153	  } from '$lib/shell/sessionWorkspaces';
   154	  import { readSessionsCollapsed, writeSessionsCollapsed } from '$lib/shell/sessionStrip';
   155	  import {
   156	    createSessionLibraryService
   157	  } from '$lib/shell/sessionLibrary/sessionLibraryService';
   158	  import type { SessionLibraryRecord } from '$lib/shell/sessionLibrary/sessionLibraryModel';
   159	  import { registerShellCommands } from '$lib/shell/shellCommands';
   160	  import { readSelection, shellPanels } from '$lib/shell/shellPanels';
   161	  import type {
   162	    WorkbenchAction,
   163	    WorkbenchActionContext,
   164	    WorkbenchActionContextKind
   165	  } from '$lib/shell/overlay/actionSurfaceModel.ts';
   166	  import {
   167	    noteSessionRemoved,
   168	    noteTerminalExit,
   169	    registerStackHandlers,
   170	    type StackStartRequest
   171	  } from '$lib/shell/stacks/stackService';
   172	  import { recordStackStart, stackIdForOwnedId } from '$lib/shell/stacks/stackStore.svelte';
   173	  import {
   174	    addOwnedSession,
   175	    completeOwnedSession,
   176	    hydrateOwned,
   177	    loadStoredOwned,
   178	    rail,
   179	    removeOwnedSession,
   180	    reopenOwnedSession,
   181	    setActiveOwned,
   182	    setAvailable,
   183	    updateOwnedSession
   184	  } from '$lib/shell/stores/sessionRailStore.svelte';
   185	  import { createTerminalService, tauriTerminalBackend } from '$lib/shell/terminalService';
   186	  import { applyStoredTheme, clearTheme } from '$lib/shell/themes/themeService';
   187	  import { loadXtermModules, makeTerminalView } from '$lib/shell/xtermFactory';
   188	  import {
   189	    listAgentSessionsFromLocalBridge,
   190	    listAgentSessionsFromTauri,
   191	    type AgentSession
   192	  } from '$lib/tauriSource';
   193	
   194	  /** Hosts mount before the service finishes async init: parked here, drained later. */
   195	  const pendingHosts = new Map<string, HTMLElement>();
   196	  /** Owned ids whose surviving PTY still needs `adoptExisting` once its host mounts. */
   197	  const awaitingReattach = new Set<string>();
   198	  /** Sessions with a restart already under way. Added before the first await, so
   199	   * a second click on "Start again" cannot get past it while the first click is
   200	   * still waiting on the backend — two starts for one row would leave two agents

exec
/bin/zsh -lc "rg -n \"ConversationSurface|TerminalSurface|conversationMode|structured|terminal|session\" tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '1,240p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
3:   * /next — the shell orchestrator. Thin by construction: it owns the terminal
4:   * sessions and nothing else. Every other panel brings its own state and its
10:   * owned sessions, re-attach them (live AND tombstones), scan for resumable
76:  import ConversationSurface from '$lib/shell/components/ConversationSurface.svelte';
78:  import SessionLibraryWorkspace from '$lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte';
153:  } from '$lib/shell/sessionWorkspaces';
154:  import { readSessionsCollapsed, writeSessionsCollapsed } from '$lib/shell/sessionStrip';
157:  } from '$lib/shell/sessionLibrary/sessionLibraryService';
158:  import type { SessionLibraryRecord } from '$lib/shell/sessionLibrary/sessionLibraryModel';
184:  } from '$lib/shell/stores/sessionRailStore.svelte';
185:  import { createTerminalService, tauriTerminalBackend } from '$lib/shell/terminalService';
208:  /** What each session had open, by owned id. Read once at start-up, then kept
210:   * each for the whole shell, so this is what keeps two sessions in the same
213:  /** True only while `restoreWorkspace` is replaying a session's files. The
215:   * click and wrong here: switching session must not pull the user off the
216:   * terminal they were watching. */
244:  let activeCenterPanelId = $state('session');
247:  /** The sessions column, for opening its "Find a session" drawer from the
248:   * context panel's "Search all sessions" link. */
249:  let sessionsColumn: { openFinder(): void } | null = null;
494:      return { enabled: false, reason: 'The browser belongs to another session.' };
597:          : activeCenterPanelId === 'sessionLibrary'
599:            : 'session';
622:  const sessionLibraryService = createSessionLibraryService(
637:          const owned = rail.owned.find((session) => session.ownedId === record.ownedId);
671:   * sessions column's "Find a session" drawer. Both live outside that panel, so
676:    openSessionFinder: () => sessionsColumn?.openFinder()
680:   * Is the sessions column folded up to a strip? The PAGE owns this rather
686:  let sessionsCollapsed = $state(
690:  /** Tell the frame how wide the sessions column is now. The limits go with
695:      'sessions',
703:  /** Fold the sessions column up, or open it out. Remembered under its own
706:    sessionsCollapsed = collapsed;
745:   * sessions column is part of that arrangement, so it opens out too. */
749:    if (sessionsCollapsed) collapseSessions(false);
770:  /** EXPLICIT IO: scan for resumable agent sessions (Tauri first, bridge second).
778:      let sessions: AgentSession[] | null = null;
780:        sessions = await listAgentSessionsFromTauri();
785:      countInvoke((sessions ?? nativeError) ? 'list_agent_sessions' : 'bridge:agent-sessions');
786:      sessions ??= await listAgentSessionsFromLocalBridge();
788:      setAvailable(sessions ?? []);
790:      if (sessions === null && nativeError) rail.error = `session scan failed: ${nativeError}`;
792:      if (!disposed) rail.error = `session scan failed: ${nativeError ?? describeError(error)}`;
807:  /** Wait for TerminalSurface to mount the host for `ownedId`.
809:   * A host that is no longer IN the page is not an answer: a session whose
810:   * terminal was closed loses its host div, and this map still holds the one it
811:   * used to have. Building a terminal on that detached element would leave the
812:   * session running with nothing on screen — so a host that has been taken out
837:    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
838:    if (!host || !session) return;
839:    const size = session.ptySessionId ? (livePtySizes.get(session.ptySessionId) ?? null) : null;
840:    const attached = await service.adoptExisting(session, host, size);
849:   * The editor tabs the last few sessions left behind, contents and all.
851:   * The stored record beside it is only a list of paths, so putting a session
854:   * themselves is what makes going back to a session free. Only the last three
855:   * sessions are held, so the memory this costs has a ceiling; the fourth reads
856:   * its files again, exactly as every session used to.
864:    const agent = rail.owned.find((session) => session.ownedId === ownedId)?.agent;
868:  /** Remember the editor tabs and file tree this session is leaving behind.
873:    // moment, same session, but this half stays in memory. A session leaving an
895:   * Put back the editor tabs and file tree this session had.
897:   * A tab still held from the last time this session was on screen goes back as
904:   * The tree's state is assigned directly, AFTER `sessionPicked` has pointed the
911:    // Start-up re-attaching a session picks it, which is indistinguishable from
918:    // session remembered comes back, and anything else — including the file
919:    // the last session was looking at, which otherwise survives because the
921:    const sessionRoot = readSelection().root.trim();
922:    const rememberedDiff = diffPathFor(workspaces[ownedId] ?? null, sessionRoot);
926:      void gitService.showStoredDiff(sessionRoot, rememberedDiff);
938:    // Every restore starts from THIS session's tabs and no others. A session
940:    // emptiness is the whole point: it is the other session's tabs not being
961:    const selected = rail.owned.find((session) => session.ownedId === ownedId);
962:    // Save the session being left BEFORE anything points the panels elsewhere.
965:    // session actually had (rows are clickable for seconds while the first
966:    // scan runs — including the close button, which switches sessions too).
976:    // opened at this session's project. Ignored while start-up is still
977:    // re-attaching sessions, so a reload still loads nothing on its own.
978:    shellPanels.sessionPicked();
979:    // Clicking the session you are already on changes nothing. Putting the
985:      // A running PTY is already the owner of this agent session. Starting a
987:      // append different answers to one conversation. Until the structured
988:      // surface mirrors the PTY's transcript, keep the real terminal visible.
1000:        rail.error = `could not open structured ${provider}: ${describeError(error)}`;
1010:    const selected = rail.owned.find((session) => session.ownedId === ownedId);
1015:    if (!nativeSessionId) throw new Error('The native session id is not available');
1016:    if (!ptySessionId) throw new Error('A live user terminal is required for handoff');
1022:      expectedOwner: direction === 'structured-to-terminal' ? 'structured' as const : 'terminal' as const,
1037:        tuiLive: direction === 'structured-to-terminal',
1038:        tuiReleased: direction === 'terminal-to-structured',
1051:    const input = handoffInput(ownedId, 'structured-to-terminal', mode);
1055:      if (mode === 'same-session' && receipt.nativeSessionId) {
1069:      await completeNativeHandoff(ownedId, 'same-session');
1085:      const input = handoffInput(ownedId, 'terminal-to-structured', 'same-session');
1088:      if (receipt.owner === 'structured') setConversationMode(ownedId, 'structured');
1090:      rail.error = `structured handoff failed: ${describeError(error)}`;
1094:  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
1101:      rail.error = `no terminal host for "${owned.title}"`;
1107:      rail.error = `failed to start a terminal for "${owned.title}"`;
1113:    // A session you just started is a session you want to watch. Deliberately
1117:    frameControls?.showCenterPanel('session');
1121:   * EXPLICIT IO: start a session the user described in the new-session dialog.
1138:      rail.error = `no terminal host for "${owned.title}"`;
1144:      rail.error = `failed to start a terminal for "${owned.title}"`;
1150:    // Same as resuming one: you asked for this session, so it comes to the front.
1151:    frameControls?.showCenterPanel('session');
1155:   * EXPLICIT IO: run a saved stack. A stack IS a session — it appears on the
1156:   * rail like any other and its terminal is the one you watch.
1158:   * `runCommandDirectly` is what makes the stacks pane honest: the session is
1160:   * dev server dies the session ends with the server's exit code instead of
1164:   * Answers with the new session's id, or `null` when no terminal could be
1184:    // configuration's terminal opens behind whatever tab was already in front,
1186:    frameControls?.showCenterPanel('session');
1191:   * EXPLICIT IO: start a finished session up again, in place.
1193:   * It stays the SAME session — same `ownedId`, so the files it had open, the
1196:   * un-finish a finished one either: a done session started again stays under
1199:   * What it cannot keep is the process. A terminal that has ended cannot be
1201:   * session's resume command — the same thing resuming a scanned session does,
1203:   * session started here rather than found on disk has no resume command and
1209:   * would spawn two terminals for one session. The second would replace the
1218:    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
1219:    // Only a finished session can be started again; a running one already is.
1220:    if (!session || session.state !== 'exited') return;
1221:    const label = session.title || ownedId;
1225:     * Put the terminal the manager promoted back on screen. Closing a view
1226:     * makes another session's view visible and `closeOwned` reports which one;
1229:     * user and another session's scrollback sitting under this session's title.
1241:      // The old terminal is over: drop its view and let the backend forget the
1242:      // dead process. One session has one view, so without this the new
1243:      // terminal would open underneath the last one's final output — and the
1247:      // and the session is about to get a working terminal regardless.
1249:      const closed = await service.closeOwned(ownedId, session.ptySessionId);
1252:      // Say the row is running BEFORE asking for a terminal host: the terminal
1253:      // surface only keeps a host on screen for a session it believes has a
1254:      // terminal, so while the row still reads as finished there is nothing for
1262:        rail.error = `no terminal host for "${label}"`;
1268:      // A stack's session keeps its one-command spawn on restart — typed into
1273:        { ...session, ptySessionId: null },
1279:        rail.error = `could not start "${label}" again: no new terminal opened`;
1286:      // while the terminal was still starting. Neither could reach this PTY —
1287:      // it did not exist yet — so adopting it here would put back a session the
1302:      // Only on this path, where the session the user asked for is the one that
1304:      // DIFFERENT session on purpose, and pulling the reader to the terminal
1306:      frameControls?.showCenterPanel('session');
1314:        // function already put someone else's terminal on screen.
1325:   * back the terminal the manager left visible.
1327:   * The SESSION survives this. Closing a terminal ends the process and its
1333:    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
1337:    const result = await service?.closeOwned(ownedId, session?.ptySessionId ?? null);
1339:      rail.error = `close failed for "${session?.title ?? ownedId}": ${describeError(result.error)}`;
1345:    // through `selectOwned` like every other session change, and the order is
1346:    // what makes that safe: `rail.activeOwnedId` is still the session whose
1347:    // terminal just closed, so the tabs and tree on screen are saved as ITS
1349:    // Pointing the rail at the successor directly saved this session's files
1358:   * EXPLICIT IO: take a session off the list for good. The transcript on disk is
1361:   * The close runs every time, not just for a session that is still running. A
1362:   * session whose process ended on its own keeps both its terminal on screen and
1374:    // pointing at a session that is gone.
1406:    // rail, the terminal service and the terminal hosts, so it does the work and
1425:            // else: no timer anywhere reads process states. A session that is
1434:            terminalService: service,
1435:            terminalHost: extensionApiProbeTerminalHost
1442:        for (const i of live) livePtySizes.set(i.sessionId, { cols: i.cols, rows: i.rows });
1450:        for (const session of attachable) awaitingReattach.add(session.ownedId);
1452:        // The per-session tabs and tree state, read once. Sessions that did not
1462:        for (const session of attachable) {
1463:          await hostFor(session.ownedId);
1465:            await reattachIfPending(session.ownedId);
1467:            failed.push(`"${session.title}" (${describeError(error)})`);
1480:        // context cards would never load again. From here, a session being
1483:        // A session re-attached during start-up was "picked" before the gate was
1485:        // a reload comes back with empty panes until the user clicks a session.
1486:        if (!disposed && rail.activeOwnedId !== null) shellPanels.sessionPicked();
1487:        // Same story for the files that session had open: the pick that would
1494:    /** Remember the session on screen when the page goes away. Leaving the
1495:     * window is not a session switch, so nothing else would have saved it, and a
1507:      // last chance to remember what the session on screen had open.
1534:{#snippet sessionsArea()}
1536:    bind:this={sessionsColumn}
1538:    scanning={rail.scanning} collapsed={sessionsCollapsed}
1570:{#snippet sessionArea()}
1571:  <!-- `onHostLayout` is what re-measures a terminal: a terminal is built inside
1574:       host appears, changes size, or the terminal font lands. -->
1575:  <ConversationSurface
1587:       Except while a session's files are being put back — that is not a request for anything, and
1588:       it must not drag the user off the terminal they were watching. -->
1608:{#snippet sessionLibraryArea()}
1612:    service={sessionLibraryService}
1613:    workspacePath={rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.cwd ?? null}
1614:    projectPath={rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.projectPath ?? rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.cwd ?? null}
1615:    onOpenTab={() => frameControls?.showCenterPanel('session-library')}
1623:      frameControls?.showCenterPanel('session');
1638:    sessions={sessionsArea} tools={toolsArea} activity={activityArea} dock={dockArea}
1640:      session: sessionArea,
1644:      sessionLibrary: sessionLibraryArea,
1654:      // Say what the sessions column is, once, here, where the frame first
1666:      if (sessionsCollapsed) {
1669:        controls.setRegionLimits('sessions', {
1675:        const restored = controls.regionWidth('sessions');
1677:          controls.setRegionWidth('sessions', SESSIONS_WIDTH);
1694:    newSessionRoots={rail.owned.map((session) => session.cwd)}
1704:    class="extension-api-probe-terminal-host"
1750:  .extension-api-probe-terminal-host {

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '220,280p;560,630p;930,1110p;1520,1660p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   220	  let extensionApiProbeTerminalHost: HTMLElement | null = null;
   221	  let extensionApiProbeObservation = $state<ExtensionApiProbeObservation | null>(null);
   222	  let disposed = false;
   223	  let frameControls: {
   224	    resetLayout(): void;
   225	    showCenterPanel(id: string): void;
   226	    captureCenterLayout(): CenterDockSnapshot | null;
   227	    restoreCenterLayout(snapshot: CenterDockSnapshot | null | undefined): void;
   228	    setRegionWidth(id: ShellRegionId, width: number, limits?: RegionWidthLimits): void;
   229	    setRegionHeight(id: ShellRegionId, height: number, limits?: RegionHeightLimits): void;
   230	    setRegionLimits(id: ShellRegionId, limits: RegionWidthLimits): void;
   231	    regionWidth(id: ShellRegionId): number | null;
   232	  } | null = null;
   233	  /** The tool column's own controls; it builds after the frame does. */
   234	  let sidebarControls: {
   235	    resetLayout(): void;
   236	    expandSourceControl(): void;
   237	    selectView(id: SidebarViewId): void;
   238	  } | null = null;
   239	  /** Which tool view is open. The column decides it and says so; the page holds
   240	   * the answer only because the icon strip that draws it is a separate region
   241	   * of the frame, on the far right edge. */
   242	  let activeView = $state<SidebarViewId>(DEFAULT_SIDEBAR_VIEW);
   243	  /** The action island follows the same center-panel signal as the shell. */
   244	  let activeCenterPanelId = $state('session');
   245	  /** The overlay layer, for opening the dialogs it owns. */
   246	  let overlays: { openSettings(): void; openNewSession(): void } | null = null;
   247	  /** The sessions column, for opening its "Find a session" drawer from the
   248	   * context panel's "Search all sessions" link. */
   249	  let sessionsColumn: { openFinder(): void } | null = null;
   250	  let refitScheduled = false;
   251	  /** Its own state, NOT `rail.error`: ShellFrame mounts before this page's
   252	   * start-up, and `scanRail` clears `rail.error` — which would erase a mount
   253	   * failure on every launch and leave a blank shell with no message. */
   254	  let layoutError = $state<string | null>(null);
   255	
   256	  type BrowserMarkupTool =
   257	    | 'pen'
   258	    | 'highlighter'
   259	    | 'arrow'
   260	    | 'rectangle'
   261	    | 'text'
   262	    | 'undo'
   263	    | 'clear'
   264	    | 'crop';
   265	
   266	  const NATIVE_BROWSER_DEFERRED_MESSAGE =
   267	    'Native browser commands are not registered yet; this web/dev action is unavailable.';
   268	
   269	  function setBrowserFailure(error: unknown): void {
   270	    const message = error instanceof Error ? error.message : String(error);
   271	    browser.workspace.error = message;
   272	    browser.error = message;
   273	  }
   274	
   275	  function activeBrowserTab() {
   276	    return browserWorkspace.activeTabId
   277	      ? browserWorkspace.tabs[browserWorkspace.activeTabId] ?? null
   278	      : null;
   279	  }
   280	
   560	    makeBrowserAction({ id: 'browser-draw', label: 'Draw screenshot', icon: ImageGlyph, run: browserDraw }),
   561	    makeBrowserAction({ id: 'browser-open-external', label: 'Open in external browser', icon: ExternalLink, run: browserOpenExternal })
   562	  ];
   563	
   564	  const browserOverlayHandlers = {
   565	    onExpand: browserExpand,
   566	    onSelectTab: browserChooseTab,
   567	    onCloseTab: browserCloseTab,
   568	    onCreateTab: browserNewTab,
   569	    onAddressInput: browserAddressInput,
   570	    onNavigate: browserNavigate,
   571	    onReload: reloadBrowserFrame,
   572	    onBack: browserGoBack,
   573	    onForward: browserGoForward,
   574	    onGrab: browserGrab,
   575	    onAnnotate: browserAnnotate,
   576	    onDraw: browserDraw,
   577	    onOpenDevtools: browserOpenDevtools,
   578	    onOpenExternal: browserOpenExternal,
   579	    onViewport: browserViewport,
   580	    onPresentation: browserPresentation,
   581	    onCollapse: browserCollapse,
   582	    onCancelFeedback: browserCancelFeedback,
   583	    onMarkupTool: browserMarkupTool,
   584	    onRemoveFeedback: browserRemoveFeedback,
   585	    onCopyFeedback: browserCopyFeedback,
   586	    onStageFeedback: browserStageFeedback,
   587	    onMinimize: browserMinimize
   588	  };
   589	
   590	  function deriveWorkbenchActionContext(): WorkbenchActionContext {
   591	    const active = activeBrowserTab();
   592	    const kind: WorkbenchActionContextKind =
   593	      activeCenterPanelId === 'browser'
   594	        ? 'browser'
   595	        : activeCenterPanelId === 'editor'
   596	          ? 'editor'
   597	          : activeCenterPanelId === 'sessionLibrary'
   598	            ? 'history'
   599	            : 'session';
   600	    return {
   601	      kind,
   602	      centerPanelId: activeCenterPanelId,
   603	      ownedId: browserWorkspace.ownedId ?? rail.activeOwnedId,
   604	      workspaceId: browserWorkspace.workspaceId,
   605	      targetId: kind === 'browser' ? active?.id ?? null : null,
   606	      generation: kind === 'browser' ? active?.generation ?? null : null
   607	    };
   608	  }
   609	
   610	  /** Settled is an explicit rail transition, persisted by the existing update
   611	   * path. Age, process state, and title never infer this shelf. */
   612	  function settleOwnedSession(ownedId: string): void {
   613	    updateOwnedSession(ownedId, { settledAt: new Date().toISOString() });
   614	  }
   615	
   616	  function unsettleOwnedSession(ownedId: string): void {
   617	    updateOwnedSession(ownedId, { settledAt: null });
   618	  }
   619	
   620	  /** The center library reuses the rail's imperative actions; construction of
   621	   * this adapter is inert and does not scan, start, or mutate anything. */
   622	  const sessionLibraryService = createSessionLibraryService(
   623	    {
   624	      getOwnedSessions: () => rail.owned,
   625	      getAvailableSessions: () => rail.available
   626	    },
   627	    {
   628	      onOpen: async (record: SessionLibraryRecord) => {
   629	        if (record.ownedId) {
   630	          await selectOwned(record.ownedId);
   930	    if (conversationProvider) {
   931	      restoreConversationWorkspace(ownedId, conversationProvider, snapshot?.conversation);
   932	    }
   933	    restoreBrowserState(snapshot?.browser);
   934	    if (snapshot?.center) frameControls?.restoreCenterLayout(snapshot.center);
   935	    const taken = takeRetainedTabs(retainedTabs, ownedId);
   936	    retainedTabs = taken.retained;
   937	    const plan = planWorkspaceRestore(snapshot ?? null, taken.tabs);
   938	    // Every restore starts from THIS session's tabs and no others. A session
   939	    // that has never had a file open starts from an empty editor, and that
   940	    // emptiness is the whole point: it is the other session's tabs not being
   941	    // there.
   942	    if (plan.restoredTabs) restoreEditorFiles(plan.restoredTabs);
   943	    else resetEditorState();
   944	
   945	    restoringWorkspace = true;
   946	    try {
   947	      for (const path of plan.pathsToOpen) requestOpenFile({ path });
   948	      if (plan.activePath) requestOpenFile({ path: plan.activePath });
   949	    } finally {
   950	      restoringWorkspace = false;
   951	    }
   952	    if (!snapshot) return;
   953	    explorer.expandedFolderIds = new Set(snapshot.expandedFolderIds);
   954	    selectPath(snapshot.selectedPath);
   955	    setScrollTop(snapshot.scrollTop);
   956	  }
   957	
   958	  async function selectOwned(ownedId: string): Promise<void> {
   959	    const previous = rail.activeOwnedId;
   960	    const switching = previous !== ownedId;
   961	    const selected = rail.owned.find((session) => session.ownedId === ownedId);
   962	    // Save the session being left BEFORE anything points the panels elsewhere.
   963	    // Gated the same way as the restore below: during start-up the panels are
   964	    // still empty, and saving that emptiness would overwrite the tabs the
   965	    // session actually had (rows are clickable for seconds while the first
   966	    // scan runs — including the close button, which switches sessions too).
   967	    if (switching && previous !== null && shellPanels.loadsAllowed()) snapshotWorkspace(previous);
   968	    if (switching && previous !== null) stopConversationTerminalProjection(previous);
   969	    if (switching && selected) {
   970	      const root = selected.cwd.trim() || (selected.projectPath ?? '').trim();
   971	      if (root) await setExtensionApiProbeWorkspace({ ownedId: selected.ownedId, root });
   972	    }
   973	    setActiveOwned(ownedId);
   974	    service?.show(ownedId);
   975	    // Point the file tree, the context cards and any tab the user has already
   976	    // opened at this session's project. Ignored while start-up is still
   977	    // re-attaching sessions, so a reload still loads nothing on its own.
   978	    shellPanels.sessionPicked();
   979	    // Clicking the session you are already on changes nothing. Putting the
   980	    // stored record back here would throw away every file opened since the last
   981	    // switch, which is the opposite of what a click on your own row means.
   982	    if (switching) restoreWorkspace(ownedId);
   983	    const provider = conversationProviderFor(ownedId);
   984	    if (selected && provider) {
   985	      // A running PTY is already the owner of this agent session. Starting a
   986	      // second `claude --resume` / `codex app-server` here makes two agents
   987	      // append different answers to one conversation. Until the structured
   988	      // surface mirrors the PTY's transcript, keep the real terminal visible.
   989	      if (selected.ptySessionId) {
   990	        void closeStructuredConversation(ownedId);
   991	        setConversationMode(ownedId, 'raw');
   992	        return;
   993	      }
   994	      void ensureStructuredConversation({
   995	        ownedId,
   996	        provider,
   997	        cwd: selected.cwd,
   998	        nativeSessionId: selected.nativeSessionId
   999	      }).catch((error) => {
  1000	        rail.error = `could not open structured ${provider}: ${describeError(error)}`;
  1001	      });
  1002	    }
  1003	  }
  1004	
  1005	  function handoffInput(
  1006	    ownedId: string,
  1007	    direction: AgentConversationHandoffDirection,
  1008	    mode: AgentConversationHandoffMode
  1009	  ) {
  1010	    const selected = rail.owned.find((session) => session.ownedId === ownedId);
  1011	    const conversation = getConversationSession(ownedId);
  1012	    if (!selected || !conversation) throw new Error('The conversation is not loaded');
  1013	    const nativeSessionId = selected.nativeSessionId ?? conversation.nativeSessionId ?? null;
  1014	    const ptySessionId = selected.ptySessionId ?? null;
  1015	    if (!nativeSessionId) throw new Error('The native session id is not available');
  1016	    if (!ptySessionId) throw new Error('A live user terminal is required for handoff');
  1017	    return {
  1018	      ownedId,
  1019	      generation: conversation.generation,
  1020	      direction,
  1021	      mode,
  1022	      expectedOwner: direction === 'structured-to-terminal' ? 'structured' as const : 'terminal' as const,
  1023	      targetOwnedId: mode === 'fork' ? `${ownedId}:native:${Date.now()}` : null,
  1024	      nativeSessionId,
  1025	      ptySessionId,
  1026	      historyBoundary: {
  1027	        nativeSessionId,
  1028	        firstSequence: 0,
  1029	        lastSequence: conversation.lastSequence,
  1030	        reconciledSequence: conversation.lastSequence
  1031	      },
  1032	      // The assertion is populated from the single owned PTY and writer lease.
  1033	      // Native-window proof still needs the rebuilt app and is recorded as a
  1034	      // deferred acceptance item in the receipt.
  1035	      processTree: {
  1036	        checked: true,
  1037	        tuiLive: direction === 'structured-to-terminal',
  1038	        tuiReleased: direction === 'terminal-to-structured',
  1039	        writerCount: 1,
  1040	        ptyCount: 1,
  1041	        sidecarCount: 0,
  1042	        ptySessionId
  1043	      }
  1044	    };
  1045	  }
  1046	
  1047	  async function completeNativeHandoff(
  1048	    ownedId: string,
  1049	    mode: AgentConversationHandoffMode
  1050	  ): Promise<void> {
  1051	    const input = handoffInput(ownedId, 'structured-to-terminal', mode);
  1052	    await prepareConversationHandoff(input);
  1053	    try {
  1054	      const receipt = await commitConversationHandoff(input);
  1055	      if (mode === 'same-session' && receipt.nativeSessionId) {
  1056	        const provider = conversationProviderFor(ownedId);
  1057	        if (provider) {
  1058	          startConversationTerminalProjection({ ownedId, provider, nativeSessionId: receipt.nativeSessionId });
  1059	        }
  1060	      }
  1061	    } catch (error) {
  1062	      await rollbackConversationHandoff(input).catch(() => undefined);
  1063	      throw error;
  1064	    }
  1065	  }
  1066	
  1067	  async function openNativeCli(ownedId: string): Promise<void> {
  1068	    try {
  1069	      await completeNativeHandoff(ownedId, 'same-session');
  1070	    } catch (error) {
  1071	      rail.error = `native handoff failed: ${describeError(error)}`;
  1072	    }
  1073	  }
  1074	
  1075	  async function forkNativeCli(ownedId: string): Promise<void> {
  1076	    try {
  1077	      await completeNativeHandoff(ownedId, 'fork');
  1078	    } catch (error) {
  1079	      rail.error = `native fork failed: ${describeError(error)}`;
  1080	    }
  1081	  }
  1082	
  1083	  async function returnToStructured(ownedId: string): Promise<void> {
  1084	    try {
  1085	      const input = handoffInput(ownedId, 'terminal-to-structured', 'same-session');
  1086	      await prepareConversationHandoff(input);
  1087	      const receipt = await commitConversationHandoff(input);
  1088	      if (receipt.owner === 'structured') setConversationMode(ownedId, 'structured');
  1089	    } catch (error) {
  1090	      rail.error = `structured handoff failed: ${describeError(error)}`;
  1091	    }
  1092	  }
  1093	
  1094	  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
  1095	  async function adopt(record: AgentSession): Promise<void> {
  1096	    if (!service || disposed) return;
  1097	    const owned = adoptAgentSession(record);
  1098	    addOwnedSession(owned);
  1099	    const host = await hostFor(owned.ownedId);
  1100	    if (!host) {
  1101	      rail.error = `no terminal host for "${owned.title}"`;
  1102	      return;
  1103	    }
  1104	    const ptySessionId = await service.startOwned(owned, host);
  1105	    if (!ptySessionId) {
  1106	      updateOwnedSession(owned.ownedId, { state: 'exited' });
  1107	      rail.error = `failed to start a terminal for "${owned.title}"`;
  1108	      return;
  1109	    }
  1110	    // Persist the PTY id: reload re-attach reads it back out of localStorage.
  1520	      // keeps the old shell on its own palette. Leaving this page takes them
  1521	      // back off, so a same-document navigation to the old shell renders it
  1522	      // exactly as it was found.
  1523	      clearTheme();
  1524	    };
  1525	  });
  1526	</script>
  1527	
  1528	<svelte:head>
  1529	  <title>CommandBar · next</title>
  1530	</svelte:head>
  1531	
  1532	<!-- Every region is a top-level snippet: an implicit `{#snippet rail()}` child would
  1533	     shadow the imported `rail` store and break every `rail.owned` read. -->
  1534	{#snippet sessionsArea()}
  1535	  <SessionsColumn
  1536	    bind:this={sessionsColumn}
  1537	    owned={rail.owned} available={rail.available} activeOwnedId={rail.activeOwnedId}
  1538	    scanning={rail.scanning} collapsed={sessionsCollapsed}
  1539	    onSelect={selectOwned} onAdopt={adopt} onClose={closeTerminal} onRestart={restartOwned}
  1540	    onComplete={(ownedId) => completeOwnedSession(ownedId, new Date())}
  1541	    onReopen={reopenOwnedSession} onSettle={settleOwnedSession} onUnsettle={unsettleOwnedSession}
  1542	    onRemove={removeSession}
  1543	    onRescan={scanRail} onCollapse={collapseSessions}
  1544	    onNewSession={() => overlays?.openNewSession()}
  1545	  />
  1546	{/snippet}
  1547	{#snippet toolsArea()}
  1548	  <ShellSidebar
  1549	    onReady={(controls) => (sidebarControls = controls)}
  1550	    onActiveViewChange={(id) => (activeView = id)}
  1551	    onSourceControlVisible={(visible) => shellPanels.sourceControlVisible(visible)}
  1552	    onWorktreesVisible={(visible) => shellPanels.worktreesVisible(visible)}
  1553	    onStacksVisible={(visible) => shellPanels.stacksVisible(visible)}
  1554	    onContextVisible={(visible) => shellPanels.contextVisible(visible)}
  1555	    onProblemsVisible={(visible) => shellPanels.problemsVisible(visible)}
  1556	    onOpenSession={(ownedId) => void selectOwned(ownedId)}
  1557	    onShowDiff={() => frameControls?.showCenterPanel('diff')}
  1558	  />
  1559	{/snippet}
  1560	{#snippet activityArea()}
  1561	  <ActivityBar
  1562	    activeId={activeView}
  1563	    onSelect={(id) => sidebarControls?.selectView(id)}
  1564	    onOpenSettings={() => overlays?.openSettings()}
  1565	  />
  1566	{/snippet}
  1567	{#snippet dockArea()}
  1568	  <DockPanel onReset={resetLayout} onProblemsLocationChange={applyProblemsLocation} />
  1569	{/snippet}
  1570	{#snippet sessionArea()}
  1571	  <!-- `onHostLayout` is what re-measures a terminal: a terminal is built inside
  1572	       a hidden host, where one character measures zero pixels wide, so the fit
  1573	       that runs when the panel is shown does nothing. The surface says when a
  1574	       host appears, changes size, or the terminal font lands. -->
  1575	  <ConversationSurface
  1576	    owned={rail.owned}
  1577	    activeOwnedId={rail.activeOwnedId}
  1578	    {registerHost}
  1579	    onHostLayout={scheduleRefit}
  1580	    onOpenNativeCli={openNativeCli}
  1581	    onForkNativeCli={forkNativeCli}
  1582	    onReturnToStructured={returnToStructured}
  1583	  />
  1584	{/snippet}
  1585	{#snippet editorArea()}
  1586	  <!-- Opening a file is a request to READ it: bring the editor forward, not load it out of sight.
  1587	       Except while a session's files are being put back — that is not a request for anything, and
  1588	       it must not drag the user off the terminal they were watching. -->
  1589	  <EditorPanel
  1590	    onFileOpened={() => {
  1591	      if (!restoringWorkspace) frameControls?.showCenterPanel('editor');
  1592	    }}
  1593	    onStartWorkspaceCommand={(request) =>
  1594	      onStartStack({
  1595	        stackId: request.id,
  1596	        cwd: request.cwd,
  1597	        script: request.script,
  1598	        title: request.title
  1599	      })}
  1600	  />
  1601	{/snippet}
  1602	{#snippet browserArea()}<BrowserPanel />{/snippet}
  1603	<!-- The changes to whichever file source control has selected. `GitDiffView`
  1604	     reads that selection itself and takes no props, so it can simply live here
  1605	     as a tab of its own — which is what gives a diff the width of the middle
  1606	     instead of a column. -->
  1607	{#snippet diffArea()}<GitDiffView />{/snippet}
  1608	{#snippet sessionLibraryArea()}
  1609	  <SessionLibraryWorkspace
  1610	    owned={rail.owned}
  1611	    available={rail.available}
  1612	    service={sessionLibraryService}
  1613	    workspacePath={rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.cwd ?? null}
  1614	    projectPath={rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.projectPath ?? rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.cwd ?? null}
  1615	    onOpenTab={() => frameControls?.showCenterPanel('session-library')}
  1616	    onRefresh={() => void scanRail()}
  1617	  />
  1618	{/snippet}
  1619	{#snippet agentsArea()}
  1620	  <WorkflowControlCenter
  1621	    onOpenConversation={(ownedId) => {
  1622	      void selectOwned(ownedId);
  1623	      frameControls?.showCenterPanel('session');
  1624	    }}
  1625	  />
  1626	{/snippet}
  1627	
  1628	<main class="next-shell">
  1629	  <!-- The strip along the top. It holds the play button that runs a saved
  1630	       configuration; anything else that belongs above the whole shell goes
  1631	       beside it, since it is a plain flex row. -->
  1632	  <div class="top-bar">
  1633	    <RunButton />
  1634	  </div>
  1635	
  1636	  <div class="frame-area">
  1637	    <ShellFrame
  1638	    sessions={sessionsArea} tools={toolsArea} activity={activityArea} dock={dockArea}
  1639	    center={{
  1640	      session: sessionArea,
  1641	      editor: editorArea,
  1642	      browser: browserArea,
  1643	      diff: diffArea,
  1644	      sessionLibrary: sessionLibraryArea,
  1645	      agents: agentsArea
  1646	    }}
  1647	    onSessionPanelLayout={scheduleRefit}
  1648	    onCenterPanelShown={(id) => {
  1649	      activeCenterPanelId = id;
  1650	      shellPanels.panelShown(id);
  1651	    }}
  1652	    onReady={(controls) => {
  1653	      frameControls = controls;
  1654	      // Say what the sessions column is, once, here, where the frame first
  1655	      // exists — in BOTH cases, not only the folded one.
  1656	      //
  1657	      // Two separate things remember the column: the stored grid layout, which
  1658	      // carries its width AND the limits it may be dragged between, and the
  1659	      // fold flag under its own key. They are written at different moments —
  1660	      // the flag straight away, the grid a quarter of a second later — so a

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte | sed -n '1,360p'
printf '\\n--- conversation store ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts | sed -n '1,360p'
printf '\\n--- conversation service ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts | sed -n '1,420p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	<script lang="ts">
     2	  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
     3	  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
     4	  import TerminalSurface from './TerminalSurface.svelte';
     5	  import ConversationHeader from './conversation/ConversationHeader.svelte';
     6	  import ConversationTimeline from './conversation/ConversationTimeline.svelte';
     7	  import ConversationComposer from './conversation/ConversationComposer.svelte';
     8	  import ConversationAgentTree from './conversation/ConversationAgentTree.svelte';
     9	  import {
    10	    conversationSessions,
    11	    setConversationAttachments,
    12	    setConversationDraft,
    13	    setConversationMode,
    14	    setConversationScrollTop,
    15	    setConversationSelectedChild
    16	  } from '$lib/shell/conversation/conversationStore.svelte';
    17	  import {
    18	    cleanupConversationAttachment,
    19	    loadConversationCapabilities,
    20	    readChildConversationTranscript,
    21	    removeConversationAttachment,
    22	    respondToStructuredApproval,
    23	    respondToStructuredInput,
    24	    restoreConversationAttachments,
    25	    saveConversationClipboardImage,
    26	    sendStructuredMessage,
    27	    setConversationConfigOption
    28	  } from '$lib/shell/conversation/conversationService';
    29	  import {
    30	    filterConversationCommandCatalog,
    31	    mergeConversationCommandCatalog,
    32	    type ConversationCommand
    33	  } from '$lib/shell/conversation/conversationCommandCatalog.ts';
    34	  import {
    35	    displayItemFromApproval,
    36	    displayItemFromInput,
    37	    typedConversationTimeline,
    38	    type ConversationDisplayItem
    39	  } from '$lib/shell/conversation/conversationTimeline.ts';
    40	  import { requestOpenFile } from '$lib/shell/openFileBus.ts';
    41	
    42	  interface Props {
    43	    owned: OwnedSession[];
    44	    activeOwnedId: string | null;
    45	    registerHost(ownedId: string, host: HTMLElement): void;
    46	    onHostLayout?(ownedId: string): void;
    47	    onOpenNativeCli?(ownedId: string): void | Promise<void>;
    48	    onForkNativeCli?(ownedId: string): void | Promise<void>;
    49	    onReturnToStructured?(ownedId: string): void | Promise<void>;
    50	  }
    51	  let {
    52	    owned,
    53	    activeOwnedId,
    54	    registerHost,
    55	    onHostLayout,
    56	    onOpenNativeCli,
    57	    onForkNativeCli,
    58	    onReturnToStructured
    59	  }: Props = $props();
    60	  const active = $derived(owned.find((item) => item.ownedId === activeOwnedId) ?? null);
    61	  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
    62	  const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode !== 'raw');
    63	  const selectedChild = $derived(conversation && conversation.selectedChildId
    64	    ? conversation.children.find((child) => child.childId === conversation.selectedChildId) ?? null
    65	    : null);
    66	  const legacyTimeline = $derived(conversation?.selectedChildId ? conversation.childTimeline : conversation?.timeline ?? []);
    67	  const visibleTimeline = $derived.by((): ConversationDisplayItem[] => {
    68	    if (!conversation) return [];
    69	    if (conversation.selectedChildId) return typedConversationTimeline([], legacyTimeline);
    70	    const items = typedConversationTimeline(conversation.agentItems, legacyTimeline);
    71	    const now = items.reduce((latest, item) => Math.max(latest, item.timestampMs), 0) + 1;
    72	    const typedKinds = new Set(items.map((item) => item.kind));
    73	    if (conversation.planSteps.length && !typedKinds.has('plan')) {
    74	      items.push({ kind: 'plan', itemId: 'plan:current', title: 'Plan', steps: conversation.planSteps, timestampMs: now });
    75	    }
    76	    if (conversation.tasks.length && !typedKinds.has('tasks')) {
    77	      items.push({ kind: 'tasks', itemId: 'tasks:current', title: 'Tasks', tasks: conversation.tasks, timestampMs: now });
    78	    }
    79	    for (const request of Object.values(conversation.pendingApprovals)) {
    80	      if (!items.some((item) => item.itemId === `approval:${request.requestId}`)) items.push(displayItemFromApproval(request, now));
    81	    }
    82	    for (const request of Object.values(conversation.pendingInputs)) {
    83	      if (!items.some((item) => item.itemId === `input:${request.requestId}`)) items.push(displayItemFromInput(request, now));
    84	    }
    85	    return items.sort((left, right) => left.timestampMs - right.timestampMs);
    86	  });
    87	  const commandCatalog = $derived(mergeConversationCommandCatalog(conversation?.capabilities?.commands ?? []));
    88	  const commandQuery = $derived(conversation?.draft.trimStart().startsWith('/') ? conversation.draft.trimStart().slice(1) : '');
    89	  const matchingCommands = $derived(filterConversationCommandCatalog(commandCatalog, commandQuery));
    90	  const remainingContext = $derived.by(() => {
    91	    const used = conversation?.metadata.usedTokens;
    92	    const window = conversation?.metadata.contextWindow;
    93	    if (used == null || window == null || window <= 0) return null;
    94	    return Math.max(0, Math.round(((window - used) / window) * 100));
    95	  });
    96	
    97	  let attachmentError = $state('');
    98	  let capabilityRequest = $state('');
    99	
   100	  $effect(() => {
   101	    if (!structured || !active || !conversation || (active.agent !== 'claude' && active.agent !== 'codex')) return;
   102	    const key = `${active.ownedId}:${conversation.generation}:${conversation.provider}`;
   103	    if (conversation.capabilities || capabilityRequest === key) return;
   104	    capabilityRequest = key;
   105	    void loadConversationCapabilities(active.ownedId, conversation.provider).catch(() => undefined);
   106	  });
   107	
   108	  $effect(() => {
   109	    if (!active || !conversation || !structured) return;
   110	    if (!conversation.attachments.length && conversation.attachmentIds.length) {
   111	      void restoreConversationAttachments(active.ownedId).catch(() => undefined);
   112	    }
   113	  });
   114	
   115	  async function send(): Promise<void> {
   116	    if (!activeOwnedId || !conversation || conversation.sending || conversation.selectedChildId) return;
   117	    if (!conversation.draft.trim() && conversation.attachments.length === 0) return;
   118	    const text = conversation.draft;
   119	    setConversationDraft(activeOwnedId, '');
   120	    try {
   121	      await sendStructuredMessage(activeOwnedId, text, active?.ptySessionId);
   122	    } catch {
   123	      // Restore the draft. The service intentionally leaves attachments in the
   124	      // store on every failure, so the user can retry without data loss.
   125	      setConversationDraft(activeOwnedId, text);
   126	    }
   127	  }
   128	
   129	  async function selectChild(childId: string | null): Promise<void> {
   130	    if (!active || !conversation || !active.nativeSessionId) return;
   131	    setConversationSelectedChild(active.ownedId, childId);
   132	    if (!childId) return;
   133	    await readChildConversationTranscript({
   134	      ownedId: active.ownedId,
   135	      provider: conversation.provider,
   136	      nativeSessionId: active.nativeSessionId,
   137	      childSessionId: childId
   138	    }).catch(() => undefined);
   139	  }
   140	
   141	  /** Cmd+V checks files first; text paste is untouched when there are no files. */
   142	  async function paste(event: ClipboardEvent): Promise<void> {
   143	    if (!active || !conversation || conversation.selectedChildId) return;
   144	    const files = [...(event.clipboardData?.files ?? [])];
   145	    if (files.length === 0) return;
   146	    event.preventDefault();
   147	    const images = files.filter((file) => file.type.startsWith('image/'));
   148	    attachmentError = '';
   149	    if (images.length === 0) {
   150	      attachmentError = 'The clipboard file is not a supported image.';
   151	      return;
   152	    }
   153	    const saved = [];
   154	    try {
   155	      for (const file of images) saved.push(await saveConversationClipboardImage(active.ownedId, file));
   156	      setConversationAttachments(active.ownedId, [...conversation.attachments, ...saved]);
   157	    } catch (error) {
   158	      await Promise.all(saved.map((attachment) => cleanupConversationAttachment(active.ownedId, attachment).catch(() => undefined)));
   159	      attachmentError = error instanceof Error ? error.message : String(error);
   160	      // Draft and existing attachments remain untouched after a failed paste.
   161	    }
   162	  }
   163	
   164	  async function removeAttachment(id: string): Promise<void> {
   165	    if (!active || !conversation) return;
   166	    const found = conversation.attachments.find((item) => item.id === id);
   167	    if (!found) return;
   168	    try {
   169	      await removeConversationAttachment(active.ownedId, found);
   170	      attachmentError = '';
   171	    } catch (error) {
   172	      attachmentError = error instanceof Error ? error.message : String(error);
   173	    }
   174	  }
   175	
   176	  function selectCommand(command: ConversationCommand): void {
   177	    if (!active) return;
   178	    if (command.action === 'insert') {
   179	      setConversationDraft(active.ownedId, `/${command.name} `);
   180	      return;
   181	    }
   182	    if (command.name === 'terminal') setConversationMode(active.ownedId, 'raw');
   183	    else if (command.name === 'conversation') setConversationMode(active.ownedId, 'structured');
   184	    else setConversationDraft(active.ownedId, `/${command.name} `);
   185	  }
   186	
   187	  function onApprovalDecision(requestId: string, decision: string): void {
   188	    if (!active || (decision !== 'accept' && decision !== 'decline' && decision !== 'cancel')) return;
   189	    void respondToStructuredApproval(active.ownedId, requestId, decision).catch((error) => {
   190	      attachmentError = error instanceof Error ? error.message : String(error);
   191	    });
   192	  }
   193	
   194	  function onInputSubmit(requestId: string, values: Record<string, AgentConfigValue>, cancelled = false): void {
   195	    if (!active) return;
   196	    void respondToStructuredInput(active.ownedId, { requestId, values, cancelled }).catch((error) => {
   197	      attachmentError = error instanceof Error ? error.message : String(error);
   198	    });
   199	  }
   200	
   201	  function openConversationFile(path: string): void {
   202	    const root = (active?.cwd || active?.projectPath || '').replace(/\/+$/, '');
   203	    const candidate = path.trim();
   204	    if (!root || !candidate || candidate.includes('\0') || candidate.split('/').includes('..')) {
   205	      attachmentError = 'The file link is outside the active workspace.';
   206	      return;
   207	    }
   208	    const absolute = candidate.startsWith('/') ? candidate : `${root}/${candidate.replace(/^\.\//, '')}`;
   209	    if (absolute !== root && !absolute.startsWith(`${root}/`)) {
   210	      attachmentError = 'The file link is outside the active workspace.';
   211	      return;
   212	    }
   213	    requestOpenFile({ path: absolute, projectRoot: root });
   214	  }
   215	
   216	  async function changeConfig(optionId: string, value: import('$lib/shell/conversation/conversationTypes.ts').AgentConfigValue): Promise<void> {
   217	    if (!active) return;
   218	    await setConversationConfigOption(active.ownedId, optionId, value).catch(() => undefined);
   219	  }
   220	</script>
   221	
   222	<div class="conversation-shell" data-testid="conversation-shell">
   223	  <div class:covered={structured} class="terminal-layer"><TerminalSurface {owned} {activeOwnedId} {registerHost} {onHostLayout} /></div>
   224	  {#if structured && active && conversation}
   225	    <section class="structured" data-testid="structured-conversation" aria-label={`${active.agent} conversation`}>
   226	      <ConversationHeader {active} {conversation} {selectedChild} onModeChange={(mode) => setConversationMode(active.ownedId, mode)} />
   227	      <div class="handoff-actions" aria-label="Conversation handoff actions">
   228	        <button type="button" data-testid="open-native-cli" onclick={() => void onOpenNativeCli?.(active.ownedId)}>
   229	          Open in native CLI
   230	        </button>
   231	        {#if conversation.capabilities?.session.fork}
   232	          <button type="button" data-testid="fork-native-cli" onclick={() => void onForkNativeCli?.(active.ownedId)}>
   233	            Fork to native CLI
   234	          </button>
   235	        {/if}
   236	      </div>
   237	      <ConversationAgentTree children={conversation.children} selectedChildId={conversation.selectedChildId} onSelect={(childId) => void selectChild(childId)} />
   238	      <ConversationTimeline
   239	        items={visibleTimeline}
   240	        assistantLabel={selectedChild?.label ?? active.agent}
   241	        savedScrollTop={conversation.selectedChildId ? conversation.childScrollTopById[conversation.selectedChildId] ?? 0 : conversation.scrollTop}
   242	        emptyText={conversation.selectedChildId ? 'This sub-agent transcript is not available yet.' : 'Start the conversation below.'}
   243	        onScroll={(scrollTop) => {
   244	          if (conversation.selectedChildId) conversation.childScrollTopById[conversation.selectedChildId] = scrollTop;
   245	          else setConversationScrollTop(active.ownedId, scrollTop);
   246	        }}
   247	        onApprovalDecision={onApprovalDecision}
   248	        onInputSubmit={onInputSubmit}
   249	        onFileLink={openConversationFile}
   250	      />
   251	      {#if !conversation.selectedChildId}
   252	        <ConversationComposer
   253	          provider={active.agent}
   254	          draft={conversation.draft}
   255	          attachments={conversation.attachments}
   256	          sending={conversation.sending}
   257	          capabilities={conversation.capabilities}
   258	          config={conversation.config}
   259	          pendingConfig={conversation.pendingConfig}
   260	          configErrors={conversation.configErrors}
   261	          commands={matchingCommands}
   262	          {attachmentError}
   263	          onDraftChange={(value) => setConversationDraft(active.ownedId, value)}
   264	          onSend={send}
   265	          onPaste={paste}
   266	          onRemoveAttachment={removeAttachment}
   267	          onCommandSelected={selectCommand}
   268	          onConfigChange={(optionId, value) => void changeConfig(optionId, value)}
   269	        />
   270	      {:else}<div class="read-only-note" data-testid="conversation-read-only-note">Read-only sub-agent transcript</div>{/if}
   271	    </section>
   272	  {:else if active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode === 'raw'}
   273	    <div class="raw-actions" aria-label="Conversation handoff actions">
   274	      <button class="structured-toggle" data-testid="conversation-structured-toggle" type="button" onclick={() => void onReturnToStructured?.(active.ownedId)}>
   275	        Return to structured
   276	      </button>
   277	    </div>
   278	  {/if}
   279	</div>
   280	
   281	<style>.conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.handoff-actions{display:flex;gap:6px;align-items:center;padding:8px 12px;border-bottom:1px solid var(--color-border)}.handoff-actions button,.structured-toggle{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.handoff-actions button:hover,.structured-toggle:hover{background:var(--color-hover)}.raw-actions{position:absolute;right:12px;top:12px;z-index:2}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}</style>

--- conversation store ---
     1	/**
     2	 * Reactive conversation state keyed by Command Bar's stable `ownedId`.
     3	 *
     4	 * This module performs no IO. The conversation service owns Tauri calls and
     5	 * feeds normalized events into `applyAgentConversationEvent`.
     6	 */
     7	import { applyConversationEvent, createConversationState } from './conversationReducer.ts';
     8	import type {
     9	  AgentApprovalRequest,
    10	  AgentCapabilities,
    11	  AgentEvent,
    12	  AgentConversationConnection,
    13	  AgentConversationEvent,
    14	  AgentConversationProvider,
    15	  AgentConversationSnapshot,
    16	  AgentConfigValue,
    17	  AgentItem,
    18	  AgentUserInputRequest,
    19	  AgentWriterLease,
    20	  AgentWriterLeaseTransition,
    21	  ConversationAttachment,
    22	  ConversationChildAgent,
    23	  ConversationMetadata,
    24	  ConversationTranscriptMessage,
    25	  ConversationTranscriptSnapshot,
    26	  ConversationSessionState,
    27	  ConversationTimelineEntry
    28	} from './conversationTypes.ts';
    29	import { agentItemFromEvent, type AgentPlanStep, type ConversationTask } from './conversationTimeline.ts';
    30	import type { AgentExecutionOwner } from '../ownedSessions.ts';
    31	import {
    32	  SESSION_CONVERSATION_WORKSPACE_VERSION,
    33	  type SessionConversationWorkspace
    34	} from '../sessionWorkspaces.ts';
    35	
    36	export type ConversationViewMode = 'structured' | 'raw';
    37	
    38	export interface ConversationWorkspaceState extends ConversationSessionState {
    39	  draft: string;
    40	  mode: ConversationViewMode;
    41	  sending: boolean;
    42	  attachments: ConversationAttachment[];
    43	  metadata: ConversationMetadata;
    44	  children: ConversationChildAgent[];
    45	  selectedChildId: string | null;
    46	  childTimeline: ConversationTimelineEntry[];
    47	  scrollTop: number;
    48	  childScrollTopById: Record<string, number>;
    49	  executionOwner: AgentExecutionOwner;
    50	  writerLease: AgentWriterLease;
    51	  writerLeaseTransition: AgentWriterLeaseTransition | null;
    52	  attachmentIds: string[];
    53	  config: Record<string, AgentConfigValue>;
    54	  telemetry: Record<string, AgentConfigValue>;
    55	  /** The provider's authoritative capability snapshot for this owned session. */
    56	  capabilities: AgentCapabilities | null;
    57	  capabilityError: string | null;
    58	  /** Typed ACP items are kept beside the legacy reducer projection. */
    59	  agentItems: AgentItem[];
    60	  planSteps: AgentPlanStep[];
    61	  tasks: ConversationTask[];
    62	  pendingApprovals: Record<string, AgentApprovalRequest & { state: string }>;
    63	  pendingInputs: Record<string, AgentUserInputRequest>;
    64	  pendingConfig: Record<string, AgentConfigValue>;
    65	  configErrors: Record<string, string>;
    66	}
    67	
    68	const emptyMetadata = (): ConversationMetadata => ({
    69	  model: null,
    70	  effort: null,
    71	  approvalPolicy: null,
    72	  usedTokens: null,
    73	  contextWindow: null
    74	});
    75	
    76	export const conversationSessions = $state<Record<string, ConversationWorkspaceState>>({});
    77	
    78	function freshState(
    79	  ownedId: string,
    80	  provider: AgentConversationProvider
    81	): ConversationWorkspaceState {
    82	  return {
    83	    ...createConversationState(ownedId, provider),
    84	    draft: '',
    85	    mode: 'structured',
    86	    sending: false,
    87	    attachments: [],
    88	    metadata: emptyMetadata(),
    89	    children: [],
    90	    selectedChildId: null,
    91	    childTimeline: [],
    92	    scrollTop: 0,
    93	    childScrollTopById: {},
    94	    executionOwner: 'stopped',
    95	    writerLease: { ownedId, generation: 0, owner: 'none' },
    96	    writerLeaseTransition: null,
    97	    attachmentIds: [],
    98	    config: {},
    99	    telemetry: {},
   100	    capabilities: null,
   101	    capabilityError: null,
   102	    agentItems: [],
   103	    planSteps: [],
   104	    tasks: [],
   105	    pendingApprovals: {},
   106	    pendingInputs: {},
   107	    pendingConfig: {},
   108	    configErrors: {}
   109	  };
   110	}
   111	
   112	export function getConversationSession(ownedId: string): ConversationWorkspaceState | null {
   113	  return conversationSessions[ownedId] ?? null;
   114	}
   115	
   116	export function ensureConversationSession(
   117	  ownedId: string,
   118	  provider: AgentConversationProvider
   119	): ConversationWorkspaceState {
   120	  const current = conversationSessions[ownedId];
   121	  if (current?.provider === provider) return current;
   122	  const created = freshState(ownedId, provider);
   123	  conversationSessions[ownedId] = created;
   124	  return created;
   125	}
   126	
   127	export function applyAgentConversationEvent(event: AgentConversationEvent | AgentEvent): boolean {
   128	  if ('type' in event) return applyCanonicalAgentEvent(event);
   129	  const current = ensureConversationSession(event.ownedId, event.provider);
   130	  const next = applyConversationEvent(current, event);
   131	  if (next === current) return false;
   132	  conversationSessions[event.ownedId] = {
   133	    ...next,
   134	    draft: current.draft,
   135	    mode: current.mode,
   136	    sending: current.sending,
   137	    attachments: current.attachments,
   138	    metadata: current.metadata,
   139	    children: current.children,
   140	    selectedChildId: current.selectedChildId,
   141	    childTimeline: current.childTimeline,
   142	    scrollTop: current.scrollTop,
   143	    childScrollTopById: current.childScrollTopById,
   144	    executionOwner: current.executionOwner,
   145	    writerLease: { ...current.writerLease, generation: next.generation },
   146	    writerLeaseTransition: current.writerLeaseTransition,
   147	    attachmentIds: current.attachmentIds,
   148	    config: current.config,
   149	    telemetry: current.telemetry,
   150	    capabilities: current.capabilities,
   151	    capabilityError: current.capabilityError,
   152	    agentItems: current.agentItems,
   153	    planSteps: current.planSteps,
   154	    tasks: current.tasks,
   155	    pendingApprovals: current.pendingApprovals,
   156	    pendingInputs: current.pendingInputs,
   157	    pendingConfig: current.pendingConfig,
   158	    configErrors: current.configErrors
   159	  };
   160	  const typedItem = agentItemFromEvent(event);
   161	  if (typedItem) {
   162	    const delta = event.payload.kind === 'assistantDelta';
   163	    conversationSessions[event.ownedId].agentItems = upsertAgentItem(
   164	      conversationSessions[event.ownedId].agentItems,
   165	      typedItem,
   166	      delta
   167	    );
   168	  }
   169	  applyTypedEventPayload(conversationSessions[event.ownedId], event);
   170	  return true;
   171	}
   172	
   173	function applyCanonicalAgentEvent(event: AgentEvent): boolean {
   174	  const current = ensureConversationSession(event.ownedId, event.provider);
   175	  if (event.generation < current.generation) return false;
   176	  const newGeneration = event.generation > current.generation;
   177	  const previousSequence = newGeneration ? 0 : current.lastSequence;
   178	  if (!newGeneration && event.sequence <= previousSequence) return false;
   179	  const hasGap = event.sequence !== previousSequence + 1;
   180	  if (hasGap) {
   181	    conversationSessions[event.ownedId] = { ...current, generation: event.generation, lastSequence: event.sequence, desynchronized: true };
   182	    return true;
   183	  }
   184	  const payload = event.payload as Record<string, unknown>;
   185	  const nextState: ConversationWorkspaceState = {
   186	    ...current,
   187	    generation: event.generation,
   188	    lastSequence: event.sequence,
   189	    desynchronized: newGeneration ? false : current.desynchronized,
   190	    connectionState: event.type === 'session.closed' ? 'closed'
   191	      : event.type === 'session.started' ? 'connected'
   192	        : event.type === 'runtime.error' && payload.recoverable === false ? 'failed' : current.connectionState,
   193	    nativeSessionId: event.nativeSessionId ?? current.nativeSessionId,
   194	    writerLease: { ...current.writerLease, generation: event.generation }
   195	  };
   196	  conversationSessions[event.ownedId] = nextState;
   197	  const target = conversationSessions[event.ownedId];
   198	  const typedItem = agentItemFromEvent(event);
   199	  if (typedItem) {
   200	    target.agentItems = upsertAgentItem(target.agentItems, typedItem, event.type === 'content.delta');
   201	  }
   202	  applyTypedEventPayload(target, event);
   203	  return true;
   204	}
   205	
   206	function upsertAgentItem(items: AgentItem[], incoming: AgentItem, delta: boolean): AgentItem[] {
   207	  const index = items.findIndex((item) => item.id === incoming.id);
   208	  if (index < 0) return [...items, incoming];
   209	  const existing = items[index];
   210	  const content = delta && existing.content.length && incoming.content.length
   211	    && existing.content[existing.content.length - 1].channel === incoming.content[0].channel
   212	    ? [
   213	      ...existing.content.slice(0, -1),
   214	      {
   215	        ...existing.content[existing.content.length - 1],
   216	        text: `${existing.content[existing.content.length - 1].text}${incoming.content[0].text}`
   217	      },
   218	      ...incoming.content.slice(1)
   219	    ]
   220	    : incoming.content.length ? incoming.content : existing.content;
   221	  return items.map((item, itemIndex) => itemIndex === index ? { ...item, ...incoming, content } : item);
   222	}
   223	
   224	export function applyAgentConversationSnapshot(snapshot: AgentConversationSnapshot): void {
   225	  const current = ensureConversationSession(
   226	    snapshot.connection.ownedId,
   227	    snapshot.connection.provider
   228	  );
   229	  if (snapshot.connection.generation < current.generation) return;
   230	  let rebuilt = createConversationState(
   231	    snapshot.connection.ownedId,
   232	    snapshot.connection.provider
   233	  );
   234	  rebuilt.generation = snapshot.connection.generation;
   235	  rebuilt.connectionState = snapshot.connection.state;
   236	  rebuilt.nativeSessionId = snapshot.connection.nativeSessionId;
   237	  for (const event of snapshot.events) rebuilt = applyConversationEvent(rebuilt, event);
   238	  conversationSessions[snapshot.connection.ownedId] = {
   239	    ...rebuilt,
   240	    draft: current.draft,
   241	    mode: current.mode,
   242	    sending: current.sending,
   243	    attachments: current.attachments,
   244	    metadata: current.metadata,
   245	    children: current.children,
   246	    selectedChildId: current.selectedChildId,
   247	    childTimeline: current.childTimeline,
   248	    scrollTop: current.scrollTop,
   249	    childScrollTopById: current.childScrollTopById,
   250	    executionOwner: current.executionOwner,
   251	    writerLease: { ...current.writerLease, generation: rebuilt.generation },
   252	    writerLeaseTransition: current.writerLeaseTransition,
   253	    attachmentIds: current.attachmentIds,
   254	    config: current.config,
   255	    telemetry: current.telemetry,
   256	    capabilities: current.capabilities,
   257	    capabilityError: current.capabilityError,
   258	    agentItems: current.agentItems,
   259	    planSteps: current.planSteps,
   260	    tasks: current.tasks,
   261	    pendingApprovals: current.pendingApprovals,
   262	    pendingInputs: current.pendingInputs,
   263	    pendingConfig: current.pendingConfig,
   264	    configErrors: current.configErrors
   265	  };
   266	  const restored = conversationSessions[snapshot.connection.ownedId];
   267	  for (const event of snapshot.events) {
   268	    const typedItem = agentItemFromEvent(event);
   269	    if (typedItem) {
   270	      restored.agentItems = upsertAgentItem(
   271	        restored.agentItems,
   272	        typedItem,
   273	        event.payload.kind === 'assistantDelta'
   274	      );
   275	    }
   276	    applyTypedEventPayload(restored, event);
   277	  }
   278	}
   279	
   280	function isRecord(value: unknown): value is Record<string, unknown> {
   281	  return !!value && typeof value === 'object' && !Array.isArray(value);
   282	}
   283	
   284	function asString(value: unknown): string | null {
   285	  return typeof value === 'string' && value.length > 0 ? value : null;
   286	}
   287	
   288	function applyTypedEventPayload(current: ConversationWorkspaceState, event: AgentConversationEvent | AgentEvent): void {
   289	  const payload = event.payload as Record<string, unknown>;
   290	  const raw = isRecord(payload);
   291	  if (!raw) return;
   292	  const eventType: string = 'type' in event
   293	    ? event.type
   294	    : payload.kind === 'approval' ? 'approval.requested' : payload.kind === 'error' ? 'runtime.error' : '';
   295	  const requestIdFromEvent = 'requestId' in event ? event.requestId : undefined;
   296	  const turnIdFromEvent = 'turnId' in event ? event.turnId : undefined;
   297	  const itemIdFromEvent = 'itemId' in event ? event.itemId : undefined;
   298	  const capabilities = isRecord(payload.capabilities) ? payload.capabilities as unknown as AgentCapabilities : null;
   299	  if (capabilities && Array.isArray(capabilities.configOptions) && Array.isArray(capabilities.commands)) {
   300	    current.capabilities = capabilities;
   301	    current.capabilityError = null;
   302	  }
   303	  if (eventType === 'plan.updated' || payload.kind === 'plan') {
   304	    if (Array.isArray(payload.steps)) current.planSteps = parsePlanSteps(payload.steps);
   305	  }
   306	  if (eventType === 'tasks.updated' || payload.kind === 'tasks') {
   307	    if (Array.isArray(payload.tasks)) current.tasks = parseTasks(payload.tasks);
   308	  }
   309	  if (eventType === 'approval.requested') {
   310	    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
   311	    const legacyState = payload.kind === 'approval' ? asString(payload.state) : null;
   312	    if (requestId && legacyState && legacyState !== 'requested') delete current.pendingApprovals[requestId];
   313	    else if (requestId) current.pendingApprovals[requestId] = {
   314	        ownedId: current.ownedId,
   315	        generation: current.generation,
   316	        requestId,
   317	        turnId: turnIdFromEvent,
   318	        itemId: itemIdFromEvent,
   319	        title: asString(payload.title) ?? 'Approval requested',
   320	        description: asString(payload.description) ?? undefined,
   321	        options: Array.isArray(payload.options) ? payload.options.filter((value): value is 'accept' | 'decline' | 'cancel' => value === 'accept' || value === 'decline' || value === 'cancel') : ['accept', 'decline'],
   322	        state: 'requested'
   323	      };
   324	  }
   325	  if (eventType === 'approval.resolved') {
   326	    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
   327	    if (requestId) delete current.pendingApprovals[requestId];
   328	  }
   329	  if (eventType === 'user-input.requested') {
   330	    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
   331	    if (requestId && Array.isArray(payload.fields)) {
   332	      current.pendingInputs[requestId] = {
   333	        ownedId: current.ownedId,
   334	        generation: current.generation,
   335	        requestId,
   336	        turnId: turnIdFromEvent,
   337	        itemId: itemIdFromEvent,
   338	        title: asString(payload.title) ?? 'Input requested',
   339	        description: asString(payload.description) ?? undefined,
   340	        fields: payload.fields as AgentUserInputRequest['fields']
   341	      };
   342	    }
   343	  }
   344	  if (eventType === 'user-input.resolved') {
   345	    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
   346	    if (requestId) delete current.pendingInputs[requestId];
   347	  }
   348	}
   349	
   350	function parsePlanSteps(value: unknown): AgentPlanStep[] {
   351	  if (!Array.isArray(value)) return [];
   352	  return value.flatMap((entry, index) => {
   353	    if (!isRecord(entry)) return [];
   354	    const state = asString(entry.state);
   355	    return [{
   356	      id: asString(entry.id) ?? `step-${index + 1}`,
   357	      title: asString(entry.title) ?? `Step ${index + 1}`,
   358	      detail: asString(entry.detail),
   359	      state: state === 'in-progress' || state === 'completed' || state === 'failed' || state === 'blocked' ? state : 'pending',
   360	      ownerAgentId: asString(entry.ownerAgentId),

--- conversation service ---
     1	import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
     2	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
     3	import {
     4	  applyAgentConversationEvent,
     5	  applyAgentConversationSnapshot,
     6	  applyChildConversationTranscript,
     7	  ensureConversationSession,
     8	  getConversationSession,
     9	  setConversationConnection,
    10	  setConversationAttachments,
    11	  setConversationSending,
    12	  beginConversationConfigChange,
    13	  confirmConversationConfigChange,
    14	  failConversationConfigChange,
    15	  setConversationCapabilities,
    16	  setConversationCapabilityError,
    17	  setConversationWriterLeaseTransition,
    18	  clearConversationWriterLeaseTransition
    19	} from './conversationStore.svelte.ts';
    20	import type {
    21	  AgentCapabilities,
    22	  AgentConversationConnection,
    23	  AgentConversationEvent,
    24	  AgentEvent,
    25	  AgentConversationProvider,
    26	  AgentConversationSnapshot,
    27	  AgentConfigOption,
    28	  AgentConfigValue,
    29	  AgentUserInputResponse,
    30	  ConversationTranscriptSnapshot,
    31	  ConversationAttachment,
    32	  AgentConversationHandoffRequest,
    33	  AgentConversationHandoffReceipt,
    34	  AgentConversationHandoffDirection,
    35	  AgentConversationHandoffPhase,
    36	  AgentWriterLeaseOwner,
    37	  AgentWriterLeaseTransition
    38	} from './conversationTypes.ts';
    39	import {
    40	  applyConversationHandoffReceipt,
    41	  createConversationHandoffRequest,
    42	  handoffGenerationMatches
    43	} from './conversationTypes.ts';
    44	import { writeTerminalSessionFromTauri } from '$lib/tauriSource';
    45	
    46	let unlisten: UnlistenFn | null = null;
    47	const resyncing = new Map<string, Promise<void>>();
    48	const terminalProjections = new Map<string, string>();
    49	
    50	type TerminalProjectionRegistration = {
    51	  generation: number;
    52	  events: AgentEvent[];
    53	};
    54	
    55	export async function readChildConversationTranscript(input: {
    56	  ownedId: string;
    57	  provider: AgentConversationProvider;
    58	  nativeSessionId: string;
    59	  childSessionId: string;
    60	}): Promise<void> {
    61	  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
    62	    provider: input.provider,
    63	    nativeSessionId: input.nativeSessionId,
    64	    childSessionId: input.childSessionId
    65	  });
    66	  applyChildConversationTranscript(input.ownedId, input.childSessionId, snapshot.messages);
    67	}
    68	
    69	type SavedAttachment = Omit<ConversationAttachment, 'previewUrl'> & { byteLength: number };
    70	type AttachmentWithBytes = ConversationAttachment & { byteLength?: number; bytes?: number[] };
    71	
    72	export interface AgentPromptTextContent {
    73	  type: 'text';
    74	  text: string;
    75	}
    76	
    77	export interface AgentPromptImageContent {
    78	  type: 'image';
    79	  mimeType: string;
    80	  data: number[];
    81	  name: string;
    82	}
    83	
    84	export type AgentPromptContent = AgentPromptTextContent | AgentPromptImageContent;
    85	
    86	/** Build ordered ACP content blocks without turning typed images into Markdown. */
    87	export function buildConversationPrompt(
    88	  text: string,
    89	  attachments: readonly AttachmentWithBytes[],
    90	  supportsImages: boolean
    91	): { text: string; content: AgentPromptContent[] } {
    92	  if (attachments.length > 0 && !supportsImages) {
    93	    throw new Error('This conversation provider does not advertise image prompts');
    94	  }
    95	  const content: AgentPromptContent[] = [];
    96	  if (text.length > 0) content.push({ type: 'text', text });
    97	  for (const attachment of attachments) {
    98	    if (!attachment.bytes?.length) throw new Error(`Attachment ${attachment.name} is not available for image upload`);
    99	    content.push({
   100	      type: 'image',
   101	      mimeType: attachment.mimeType,
   102	      data: attachment.bytes,
   103	      name: attachment.name
   104	    });
   105	  }
   106	  return { text, content };
   107	}
   108	
   109	export async function saveConversationClipboardImage(
   110	  ownedId: string,
   111	  file: File
   112	): Promise<ConversationAttachment> {
   113	  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
   114	  const saved = await invoke<SavedAttachment>('save_agent_conversation_attachment', {
   115	    ownedId,
   116	    mimeType: file.type,
   117	    bytes
   118	  });
   119	  return { ...saved, bytes, previewUrl: URL.createObjectURL(file) } as AttachmentWithBytes;
   120	}
   121	
   122	/** Revoke only URLs owned by this surface; the managed path never goes through the DOM. */
   123	export function cleanupConversationAttachmentPreview(attachment: ConversationAttachment): void {
   124	  if (attachment.previewUrl.startsWith('blob:')) URL.revokeObjectURL(attachment.previewUrl);
   125	}
   126	
   127	async function hydrateAttachmentBytes(attachment: AttachmentWithBytes): Promise<AttachmentWithBytes> {
   128	  if (attachment.bytes?.length) return attachment;
   129	  if (!attachment.path) throw new Error(`Attachment ${attachment.name} has no managed path`);
   130	  const response = await fetch(isTauri() ? convertFileSrc(attachment.path) : attachment.path);
   131	  if (!response.ok) throw new Error(`Could not read attachment ${attachment.name}`);
   132	  return {
   133	    ...attachment,
   134	    bytes: Array.from(new Uint8Array(await response.arrayBuffer()))
   135	  };
   136	}
   137	
   138	/** Rebuild a preview URL from a backend-owned attachment after a workspace restore. */
   139	export function restoreConversationAttachmentPreview(
   140	  attachment: Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string }
   141	): ConversationAttachment {
   142	  const previewUrl = attachment.previewUrl
   143	    ?? (isTauri() ? convertFileSrc(attachment.path) : attachment.path);
   144	  return { ...attachment, previewUrl };
   145	}
   146	
   147	/** Restore attachment metadata supplied by the existing owner-scoped vault. */
   148	export async function restoreConversationAttachments(
   149	  ownedId: string,
   150	  saved: readonly (Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[] = []
   151	): Promise<ConversationAttachment[]> {
   152	  if (saved.length > 0) {
   153	    const restored = saved.map(restoreConversationAttachmentPreview);
   154	    setConversationAttachments(ownedId, restored);
   155	    return restored;
   156	  }
   157	  if (!isTauri()) return [];
   158	  try {
   159	    const records = await invoke<(Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[]>(
   160	      'read_agent_conversation_attachments',
   161	      { ownedId }
   162	    );
   163	    const restored = Array.isArray(records) ? records.map(restoreConversationAttachmentPreview) : [];
   164	    setConversationAttachments(ownedId, restored);
   165	    return restored;
   166	  } catch {
   167	    // Older controller builds have no listing command. The draft and saved ids
   168	    // remain intact so a later controller can hydrate them without data loss.
   169	    return [];
   170	  }
   171	}
   172	
   173	/** Ask the owner-validated vault to delete one managed file, then revoke its URL. */
   174	export async function removeConversationAttachment(
   175	  ownedId: string,
   176	  attachment: ConversationAttachment
   177	): Promise<void> {
   178	  await cleanupConversationAttachment(ownedId, attachment);
   179	  const state = getConversationSession(ownedId);
   180	  if (state) setConversationAttachments(ownedId, state.attachments.filter((item) => item.id !== attachment.id));
   181	}
   182	
   183	/** Cleanup for a newly saved attachment that has not been added to the store yet. */
   184	export async function cleanupConversationAttachment(
   185	  ownedId: string,
   186	  attachment: ConversationAttachment
   187	): Promise<void> {
   188	  try {
   189	    if (isTauri()) {
   190	      await invoke('delete_agent_conversation_attachment', {
   191	        request: { ownedId, attachmentId: attachment.id, path: attachment.path }
   192	      });
   193	    }
   194	  } finally {
   195	    cleanupConversationAttachmentPreview(attachment);
   196	  }
   197	}
   198	
   199	export async function loadConversationCapabilities(
   200	  ownedId: string,
   201	  provider: AgentConversationProvider
   202	): Promise<AgentCapabilities | null> {
   203	  if (!isTauri()) return null;
   204	  try {
   205	    const capabilities = await invoke<AgentCapabilities>('read_agent_conversation_capabilities', { ownedId });
   206	    if (capabilities.provider !== provider) throw new Error('Capability provider does not match this session');
   207	    setConversationCapabilities(ownedId, capabilities);
   208	    return capabilities;
   209	  } catch (error) {
   210	    const message = error instanceof Error ? error.message : String(error);
   211	    setConversationCapabilityError(ownedId, message);
   212	    throw error;
   213	  }
   214	}
   215	
   216	export async function setConversationConfigOption(
   217	  ownedId: string,
   218	  optionId: string,
   219	  value: AgentConfigValue
   220	): Promise<AgentCapabilities | null> {
   221	  const state = getConversationSession(ownedId);
   222	  const option = state?.capabilities?.configOptions.find((candidate) => candidate.id === optionId);
   223	  if (!state || !option || (option.choices && !option.choices.some((choice) => JSON.stringify(choice.value) === JSON.stringify(value)))) {
   224	    throw new Error('This configuration value is not advertised for the session');
   225	  }
   226	  if (!beginConversationConfigChange(ownedId, optionId, value)) throw new Error('Configuration change is not available');
   227	  try {
   228	    if (!isTauri()) throw new Error('The structured conversation runtime is unavailable');
   229	    const response = await invoke<{ capabilities?: AgentCapabilities; configOptions?: AgentConfigOption[] }>(
   230	      'set_agent_conversation_config_option',
   231	      { request: { ownedId, generation: state.generation, optionId, value } }
   232	    );
   233	    const latest = getConversationSession(ownedId);
   234	    if (!latest || latest.generation !== state.generation) {
   235	      throw new Error('Configuration response belongs to a stale conversation generation');
   236	    }
   237	    const capabilities = response?.capabilities
   238	      ?? (response?.configOptions ? { ...state.capabilities, configOptions: response.configOptions } as AgentCapabilities : state.capabilities);
   239	    if (!capabilities || capabilities.provider !== state.provider) {
   240	      throw new Error('Configuration response provider does not match this session');
   241	    }
   242	    confirmConversationConfigChange(ownedId, optionId, value, capabilities);
   243	    return capabilities;
   244	  } catch (error) {
   245	    if (getConversationSession(ownedId)?.generation === state.generation) {
   246	      failConversationConfigChange(ownedId, optionId, error instanceof Error ? error.message : String(error));
   247	    }
   248	    throw error;
   249	  }
   250	}
   251	
   252	export function startConversationTerminalProjection(input: {
   253	  ownedId: string;
   254	  provider: AgentConversationProvider;
   255	  nativeSessionId: string;
   256	}): void {
   257	  if (!isTauri()) return;
   258	  const signature = `${input.provider}:${input.nativeSessionId}`;
   259	  if (terminalProjections.get(input.ownedId) === signature) return;
   260	  terminalProjections.set(input.ownedId, signature);
   261	  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
   262	  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
   263	    request: { ...input, generation }
   264	  }).then((registration) => {
   265	    for (const event of registration.events) applyAgentConversationEvent(event);
   266	  }).catch(() => {
   267	    // The transcript may not exist until the agent accepts its first prompt.
   268	    // Dropping the signature lets the next activation register again.
   269	    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
   270	  });
   271	}
   272	
   273	export function stopConversationTerminalProjection(ownedId: string): void {
   274	  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
   275	  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
   276	}
   277	
   278	type HandoffInput = Omit<AgentConversationHandoffRequest, 'phase'>;
   279	
   280	function handoffTargetOwner(direction: AgentConversationHandoffDirection): AgentWriterLeaseOwner {
   281	  return direction === 'structured-to-terminal' ? 'terminal' : 'structured';
   282	}
   283	
   284	function applyHandoffReceiptToStore(receipt: AgentConversationHandoffReceipt): void {
   285	  const state = getConversationSession(receipt.ownedId);
   286	  if (!state || !handoffGenerationMatches(receipt, receipt.ownedId, state.generation)) return;
   287	  if (receipt.phase === 'prepare' || receipt.phase === 'prepared') {
   288	    const transition: AgentWriterLeaseTransition = {
   289	      ownedId: receipt.ownedId,
   290	      generation: receipt.generation,
   291	      from: receipt.previousOwner,
   292	      to: handoffTargetOwner(receipt.direction),
   293	      state: 'requested'
   294	    };
   295	    setConversationWriterLeaseTransition(transition);
   296	    return;
   297	  }
   298	  const next = applyConversationHandoffReceipt(state, receipt);
   299	  Object.assign(state, next);
   300	}
   301	
   302	async function invokeHandoff(
   303	  input: HandoffInput,
   304	  phase: AgentConversationHandoffPhase
   305	): Promise<AgentConversationHandoffReceipt> {
   306	  if (!isTauri()) throw new Error('Native handoff is available only in the desktop application');
   307	  const request = createConversationHandoffRequest({ ...input, phase });
   308	  const current = getConversationSession(request.ownedId);
   309	  if (!current || current.generation !== request.generation) {
   310	    throw new Error('Handoff request belongs to a stale conversation generation');
   311	  }
   312	  try {
   313	    const receipt = await invoke<AgentConversationHandoffReceipt>('handoff_agent_conversation', { request });
   314	    if (!handoffGenerationMatches(receipt, request.ownedId, request.generation)) {
   315	      throw new Error('Handoff receipt belongs to a stale conversation generation');
   316	    }
   317	    applyHandoffReceiptToStore(receipt);
   318	    return receipt;
   319	  } catch (error) {
   320	    clearConversationWriterLeaseTransition(request.ownedId, request.generation);
   321	    throw error;
   322	  }
   323	}
   324	
   325	export function prepareConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   326	  return invokeHandoff(input, 'prepare');
   327	}
   328	
   329	export function commitConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   330	  return invokeHandoff(input, 'commit');
   331	}
   332	
   333	export function rollbackConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   334	  return invokeHandoff(input, 'rollback');
   335	}
   336	
   337	async function resyncConversation(ownedId: string): Promise<void> {
   338	  const existing = resyncing.get(ownedId);
   339	  if (existing) return existing;
   340	  const work = (async () => {
   341	    for (let attempt = 0; attempt < 3; attempt += 1) {
   342	      const snapshot = await invoke<AgentConversationSnapshot | null>(
   343	        'read_agent_conversation_snapshot',
   344	        { ownedId }
   345	      );
   346	      if (!snapshot) return;
   347	      const sequenceBeforeApply = getConversationSession(ownedId)?.lastSequence ?? 0;
   348	      applyAgentConversationSnapshot(snapshot);
   349	      if (sequenceBeforeApply <= snapshot.lastSequence) return;
   350	    }
   351	  })().finally(() => resyncing.delete(ownedId));
   352	  resyncing.set(ownedId, work);
   353	  return work;
   354	}
   355	
   356	export async function startConversationEvents(): Promise<void> {
   357	  if (!isTauri() || unlisten) return;
   358	  unlisten = await listen<AgentConversationEvent | AgentEvent>('agent-conversation-event', ({ payload }) => {
   359	    applyAgentConversationEvent(payload);
   360	    if (getConversationSession(payload.ownedId)?.desynchronized) {
   361	      void resyncConversation(payload.ownedId);
   362	    }
   363	    if (('kind' in payload.payload && (payload.payload.kind === 'turn' || payload.payload.kind === 'error'))
   364	      || ('type' in payload && ['turn.completed', 'turn.interrupted', 'runtime.error'].includes(payload.type))) {
   365	      setConversationSending(payload.ownedId, false);
   366	    }
   367	  });
   368	}
   369	
   370	export function stopConversationEvents(): void {
   371	  unlisten?.();
   372	  unlisten = null;
   373	  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
   374	}
   375	
   376	export async function closeStructuredConversation(ownedId: string): Promise<void> {
   377	  if (!isTauri()) return;
   378	  await invoke<boolean>('close_agent_conversation', { ownedId });
   379	}
   380	
   381	export async function ensureStructuredConversation(input: {
   382	  ownedId: string;
   383	  provider: AgentConversationProvider;
   384	  cwd: string;
   385	  nativeSessionId?: string | null;
   386	}): Promise<AgentConversationConnection | null> {
   387	  ensureConversationSession(input.ownedId, input.provider);
   388	  if (!isTauri() || !input.cwd.trim()) return null;
   389	  const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
   390	    request: input
   391	  });
   392	  setConversationConnection(connection);
   393	  await resyncConversation(input.ownedId);
   394	  return connection;
   395	}
   396	
   397	export async function sendStructuredMessage(
   398	  ownedId: string,
   399	  text: string,
   400	  ptySessionId?: string | null
   401	): Promise<void> {
   402	  const state = getConversationSession(ownedId);
   403	  if (!state) return;
   404	  if (!text.trim() && state.attachments.length === 0) return;
   405	  setConversationSending(ownedId, true);
   406	  try {
   407	    if (ptySessionId) {
   408	      if (state.writerLease.ownedId !== ownedId
   409	        || state.writerLease.generation !== state.generation
   410	        || state.writerLease.owner !== 'terminal') {
   411	        throw new Error('The terminal writer lease is not confirmed for this session');
   412	      }
   413	      const attachmentText = state.attachments.length
   414	        ? `\n\nAttached screenshots:\n${state.attachments.map((item) => `- ${item.path}`).join('\n')}`
   415	        : '';
   416	      const outgoingText = `${text}${attachmentText}`;
   417	      // Match a real terminal paste followed by a separate Enter key. Sending
   418	      // both in one PTY write makes Claude's TUI treat Enter as part of its
   419	      // multiline paste buffer, leaving the prompt staged but never submitted.
   420	      const pasted = await writeTerminalSessionFromTauri(

exec
/bin/zsh -lc 'rg -n "agent_conversation|StructuredRuntimeHandle|providers/acp|acp" src-tauri tauri-svelte-preview/src/lib/shell/conversation docs | head -300' in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
rg: src-tauri: No such file or directory (os error 2)
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:61:  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:114:  const saved = await invoke<SavedAttachment>('save_agent_conversation_attachment', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:160:      'read_agent_conversation_attachments',
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:190:      await invoke('delete_agent_conversation_attachment', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:205:    const capabilities = await invoke<AgentCapabilities>('read_agent_conversation_capabilities', { ownedId });
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:230:      'set_agent_conversation_config_option',
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:262:  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:275:  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:313:    const receipt = await invoke<AgentConversationHandoffReceipt>('handoff_agent_conversation', { request });
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:343:        'read_agent_conversation_snapshot',
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:378:  await invoke<boolean>('close_agent_conversation', { ownedId });
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:389:  const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:441:    await invoke('send_agent_conversation_message', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:459:  await invoke('respond_agent_conversation_approval', {
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:470:  await invoke('respond_agent_conversation_input', {
tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts:31:export type ProviderTransport = 'acp-stdio' | 'acp-websocket' | 'built-in';
docs/conversation-workbench-rust-guide.md:11:`src-tauri/src/agent_conversation/mod.rs` exposes narrow Tauri commands. They play roughly the same role as ASP.NET controller actions:
docs/conversation-workbench-rust-guide.md:13:- `read_agent_conversation_transcript` returns a normalized snapshot for one validated parent or child session.
docs/conversation-workbench-rust-guide.md:14:- `save_agent_conversation_attachment` validates and stores one pasted image in the app's managed data directory.
docs/conversation-workbench-rust-guide.md:21:`src-tauri/src/agent_conversation/transcript.rs` is a read-only adapter over the providers' JSONL formats. It returns:
docs/conversation-workbench-rust-guide.md:34:`src-tauri/src/agent_conversation/attachments.rs` accepts borrowed inputs (`&str` and `&[u8]`). Rust does not copy those arguments merely to validate them. It creates owned `String` values only for data returned to the frontend.
docs/architecture-explainer.html:214:      <div class="box rust"><span class="t">AgentRuntimeManager</span><span class="s">agent_conversation/ → ACP child processes</span></div>
docs/architecture-explainer.html:418:<div class="path">src-tauri/src/agent_conversation/providers/acp.rs:146</div>
docs/architecture-explainer.html:419:<pre><code><kw>pub enum</kw> <ty>StructuredRuntimeHandle</ty> {
docs/architecture-explainer.html:423:<kw>impl</kw> <ty>StructuredRuntimeHandle</ty> {
docs/architecture-explainer.html:440:<div class="path">src-tauri/src/agent_conversation/providers/mod.rs</div>
docs/architecture-explainer.html:475:<div class="path">src-tauri/src/agent_conversation/manager.rs</div>
docs/architecture-explainer.html:511:<pre><code><kw>mod</kw> agent_conversation;   <kw>mod</kw> browser;        <kw>mod</kw> git_diff_models;
docs/architecture-explainer.html:981:  <li><code>src-tauri/src/agent_conversation/mod.rs</code> — the <code>#[tauri::command]</code> surface
docs/architecture-explainer.html:982:  (<code>ensure_agent_conversation</code>, <code>send_agent_conversation_message</code>,
docs/architecture-explainer.html:983:  <code>stop_agent_conversation_turn</code>, approvals).</li>
docs/architecture-explainer.html:990:  <code>acp_client.rs</code> (the JSON-RPC client), <code>process.rs</code> (spawning and cwd
docs/architecture-explainer.html:991:  validation), and <code>ProviderRegistry</code>. <code>StructuredRuntimeHandle</code> is the enum
docs/architecture-explainer.html:1247:<code>agent_conversation/capabilities.rs</code> — what an ACP agent says it can do, negotiated at
docs/superpowers/plans/README.md:13:`2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:156:- `agentclientprotocol/codex-acp` — Codex app-server mapping for models, effort, Fast mode, permissions/sandbox, images, commands, file changes, reasoning, plans, reviews, web search, token/rate usage, and subagents;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:157:- `agentclientprotocol/claude-agent-acp` — Claude Agent SDK mapping for models/options, images, tool permissions, TODOs, edit review, terminals, custom commands, MCP, and nested subagent transcripts.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:198:- `xintaofei/codeg/src-tauri/src/acp/delegation/types.rs`;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:264:Refactor the existing `src-tauri/src/agent_conversation` authority rather than adding a competing runtime.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:269:src-tauri/src/agent_conversation/
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:284:    ├── acp.rs
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:285:    ├── acp_client.rs
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:309:    pub runtime: Option<StructuredRuntimeHandle>,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1760:Current anchors are `agent_conversation/process.rs:8-92` (`ConversationProcess::spawn/stop`), the
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1763:`ensure_agent_conversation` at `mod.rs:283-293`. Cargo has no ACP SDK dependency. A0 must not
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1766:Create only evidence under `docs/superpowers/evidence/tsk-808/acp-runtime-spike.md` and redacted
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1767:`acp-fixtures/{codex,claude}/`. In disposable external probes, record exact executable, version,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1804:`conversationStore.svelte.ts:23-236`, and Rust `agent_conversation/protocol.rs:3-166` plus
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1828:`send_command`, `snapshot`, and `close` in `agent_conversation/mod.rs`. `TerminalRegistry` remains
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1831:Own new `agent_conversation/manager.rs` (`AgentRuntimeManager`, `ManagedAgentSession`, writer lease,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1834:`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1846:Run `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation` in one
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1861:`src-tauri/fixtures/agent_conversation/codex/*.jsonl`, and optional
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1869:`src-tauri/fixtures/agent_conversation/claude/*.jsonl`, and optional
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1874:**A3-terminal:** replace whole-tail `agent_conversation/transcript.rs` with
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1918:`agent_conversation/handoff.rs` for structured-to-terminal, terminal-to-structured, same-session/
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2400:- `src-tauri/src/agent_conversation/mod.rs` registration/public contract;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2417:| Runtime/ACP | `agent_conversation/{manager,capabilities,journal}.rs`, provider transport/process | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2418:| Codex provider | `agent_conversation/providers/codex*`, fixtures/tests | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2419:| Claude provider | `agent_conversation/providers/claude*`, fixtures/tests | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2631:RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation -- --nocapture
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2892:- Codex ACP adapter: `https://github.com/agentclientprotocol/codex-acp`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2893:- Claude Agent ACP adapter: `https://github.com/agentclientprotocol/claude-agent-acp`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2896:- CodeG delegation broker: `https://github.com/xintaofei/codeg/tree/main/src-tauri/src/acp/delegation`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:45:- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs` — registry, Tauri commands,
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:47:- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs` — serialized request,
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:49:- `tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs` — child-process ownership,
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:51:- `tauri-svelte-preview/src-tauri/src/agent_conversation/codex.rs` — Codex app-server adapter.
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:52:- `tauri-svelte-preview/src-tauri/src/agent_conversation/claude.rs` — Claude stream-json adapter.
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:270:- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:271:- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:272:- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:293:RUST_TEST_THREADS=1 cargo test agent_conversation -- --nocapture
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:316:- `ensure_agent_conversation`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:317:- `send_agent_conversation_message`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:318:- `respond_agent_conversation_approval`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:319:- `stop_agent_conversation_turn`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:320:- `close_agent_conversation`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:321:- `read_agent_conversation_snapshot`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:323:Emit one global event name, `agent_conversation_event`. Sequence assignment happens in the
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:347:- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/codex.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:348:- Modify `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:393:RUST_TEST_THREADS=1 cargo test agent_conversation::codex -- --nocapture
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:406:- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/claude.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:407:- Modify `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:445:RUST_TEST_THREADS=1 cargo test agent_conversation::claude -- --nocapture
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:608:RUST_TEST_THREADS=1 cargo test agent_conversation -- --nocapture
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:5:`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`  
docs/superpowers/evidence/tsk-808/resources-usage-rework-receipt.md:31:- Boundary disclosure the lane omitted: `agent_conversation/manager.rs` (+32) adds a read-only
docs/superpowers/evidence/tsk-808/resources-usage-rework-receipt.md:32:  `resource_roots()` snapshot (pid, owned id, provider, cwd) and `providers/acp.rs` (+9) exposes
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:637:| I Conversation | existing `shell/conversation`, `components/ConversationSurface.svelte`, new `ConversationAgentTree.svelte`, existing `src-tauri/src/agent_conversation`, conversation tests/fixtures | Node plus focused Rust |
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4064:- `src-tauri/src/agent_conversation/{mod.rs,protocol.rs,transcript.rs,attachments.rs}`
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4069:and provider writes through the existing PTY. Preserve them. `agent_conversation/provider.rs`
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4166:1. Keep paste/upload in `agent_conversation/attachments.rs`. Validate PNG/JPEG/WebP MIME by
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4172:   filter/rank functions. Create Rust `agent_conversation/commands.rs` command
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4173:   `list_agent_conversation_commands(owned_id, provider, project_root)` that returns the current
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4232:Add real redacted Claude and Codex fixture files under `src-tauri/fixtures/agent_conversation/`:
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4257:    RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4311:- `src-tauri/src/agent_conversation/` already has a bounded generation-guarded registry and
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4312:  typed conversation events, but `ensure_agent_conversation` is intentionally unwired and the
docs/superpowers/evidence/tsk-808/pr-flow-receipt.md:22:- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:337-357` and `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:157-244` add the one-shot ACP prompt path; the ACP fixture test also proves text-chunk capture.
docs/superpowers/evidence/tsk-808/a7-control-center-receipt.md:7:Authority: `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`, §10 and §16  
docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md:14:`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2083-2173`:
docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md:25:`tauri-svelte-preview/src-tauri/src/main.rs:24-29` declares `agent_conversation`, `git_diff_models`, `lsp`, `orchestration`, `terminal`, and `workflow`, but not `browser`. The builder manages no `BrowserRegistry` at `:5292-5305`, and the `tauri::generate_handler!` list at `:5316-5408` contains no browser command. Therefore `browser.rs` is not compiled or registered by the current binary; the browser frontend's native calls cannot resolve in this build.
docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md:248: mod agent_conversation;
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:23:- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript.rs` — removed; its public snapshot command compatibility and three baseline tests moved into the module split.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:24:- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/mod.rs:55` — `FileIdentity`; `:61` — `TranscriptLocation`; `:102` — provider discovery; `:114` — cached-path inspection; `:152` — complete-line/partial-line splitting; `:290` — preserved baseline transcript tests.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:25:- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/codex.rs:13` — Codex transcript discovery; `:46` — durable-line projection; `:189` — bounded compatibility snapshot.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:26:- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/claude.rs:13` — Claude transcript/archive discovery; `:36` — durable-line projection; `:135` — bounded compatibility snapshot.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:27:- `tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:48` — cached cursor; `:59` — incremental projector; `:74` — cached-path poll; `:137` — bounded reconciliation; `:194` — watcher registry; `:304` — app-state drop cleanup; `:318` — canonical event construction; `:427-644` — terminal-projection fixture and cleanup tests.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:28:- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:7` — projection module; `:108` — start command; `:117` — explicit stop command.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:31:- `tauri-svelte-preview/src-tauri/fixtures/agent_conversation/terminal_projection/` — owned redacted fixtures listed below.
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:51:RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation -- --nocapture
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:111:-  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:119:-    const child = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:161:+  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:176:+  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
docs/superpowers/evidence/tsk-808/a6-workflow-engine-receipt.md:26:- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:74` — capability snapshot accessor used for role-to-provider config mapping.
docs/superpowers/evidence/tsk-808/a6-workflow-engine-receipt.md:174:`RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation -- --nocapture`
docs/superpowers/evidence/tsk-808/shell-mounting-receipt.md:98:- The existing dirty/untracked Rust work was preserved and not touched by this packet: `src-tauri/src/agent_conversation/manager.rs`, `git_diff_models.rs`, `lsp.rs`, `main.rs`, `orchestration.rs`, and untracked `src-tauri/src/workflow.rs`.
docs/superpowers/evidence/tsk-808/a11-recipes-receipt.md:14:- [Verified] The amendment defines A11 at `2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2277-2332`; controller-owned shared seams are listed at `:2394-2409`.
docs/superpowers/evidence/tsk-808/a11-recipes-receipt.md:18:- [Verified] Native handoff wiring is present in `conversationService.ts:278-335`, `+page.svelte:1005-1092`, `ConversationSurface.svelte` props, and Rust `agent_conversation/handoff.rs`; A11 will not duplicate or edit those controller/shared seams.
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:20:2. The Work Package A9 section and any browser overlay/z-order sections of docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md.
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:46:540:- TSK-808, TSK-799, TSK-798, Assembly, docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md, 2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md, contrast, legibility, gpt-5.6-sol, Tauri, Dockview, Monaco, Roslyn, U1
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:48:562:- The authoritative plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; use section 18 of `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md` instead of retired schedules. TSK-808 remained open/High; TSK-799 and TSK-798 are completed dependencies. [Task 1]
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:295:/bin/zsh -lc "git status --short && printf '%s\\n' '--- receipt status/diff ---' && git diff --stat -- docs/superpowers/evidence/tsk-808/a9-browser-frontend-receipt.md && git diff -- docs/superpowers/evidence/tsk-808/a9-browser-frontend-receipt.md && printf '%s\\n' '--- receipt numbered ---' && nl -ba docs/superpowers/evidence/tsk-808/a9-browser-frontend-receipt.md && printf '%s\\n' '--- A9/browser plan hits ---' && rg -n -i -C 8 'Work Package A9|\\bA9\\b|browser overlay|BrowserPanel|z-order|z order|overlay mount|WorkbenchActionFab|browserArea' docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:874:/bin/zsh -lc "printf '%s\\n' '--- A9 plan (numbered) ---' && nl -ba docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md | sed -n '1060,1160p;1230,1270p;1770,1835p;2080,2165p;2625,2675p;2725,2770p' && printf '%s\\n' '--- browser/overlay/z-order all hits ---' && rg -n -i 'browser|overlay|z-order|z order|WorkbenchActionFab|BrowserPanel|browserArea' docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md | sed -n '1,260p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:1053:  1804	`conversationStore.svelte.ts:23-236`, and Rust `agent_conversation/protocol.rs:3-166` plus
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:1077:  1828	`send_command`, `snapshot`, and `close` in `agent_conversation/mod.rs`. `TerminalRegistry` remains
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:1080:  1831	Own new `agent_conversation/manager.rs` (`AgentRuntimeManager`, `ManagedAgentSession`, writer lease,
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:1083:  1834	`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:1177:  2631	RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation -- --nocapture
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:1412:/bin/zsh -lc "nl -ba docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md | sed -n '1257,1415p;1408,1465p;1560,1605p;2335,2395p;2420,2448p;2568,2605p;2780,2825p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7369:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:5://! namespace.  It does not install a package per launch, invoke `claude -p`, attach to a native
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7371:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:48:  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7372:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:101:  const saved = await invoke<SavedAttachment>('save_agent_conversation_attachment', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7374:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:177:      await invoke('delete_agent_conversation_attachment', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7375:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:192:    const capabilities = await invoke<AgentCapabilities>('read_agent_conversation_capabilities', { ownedId });
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7377:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:249:  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7378:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:262:  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7380:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:306:  await invoke<boolean>('close_agent_conversation', { ownedId });
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7381:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:317:  const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7382:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:369:    await invoke('send_agent_conversation_message', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7383:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:387:  await invoke('respond_agent_conversation_approval', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7384:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:398:  await invoke('respond_agent_conversation_input', {
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7913:5302:        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:7940:    24	mod agent_conversation;
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8060:  5293	    let agent_runtime = agent_conversation::manager::AgentRuntimeManager::new(
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8061:  5294	        agent_conversation::providers::ProviderRegistry::bundled_from_environment()
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8069:  5302	        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8212:tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts:31:export type ProviderTransport = 'acp-stdio' | 'acp-websocket' | 'built-in';
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8997:docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1246:Retain and strengthen the master-plan `WorkbenchActionFab` concept:
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8998:docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2140:The controller mounts exactly one Browser overlay and one global `WorkbenchActionFab` under the
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:8999:docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2141:existing `ShellOverlays`; it derives a `WorkbenchActionContext` from the current center/owner/
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:9000:docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2148:`browserFeedback.test.mjs`, `browserBackend.test.mjs`, and `actionSurfaceModel.test.mjs`; extend
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:9001:docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2657:node --experimental-strip-types scripts/actionSurfaceModel.test.mjs
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:13261:mod agent_conversation;
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:13296:    let agent_runtime = agent_conversation::manager::AgentRuntimeManager::new(
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:13297:        agent_conversation::providers::ProviderRegistry::bundled_from_environment()
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:13305:        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:15283:/bin/zsh -lc "nl -ba docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md | sed -n '1240,1405p;2080,2170p;2330,2395p;2645,2668p;2745,2798p'
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:43:| `RUST_TEST_THREADS=1 MSBUILDDISABLENODEREUSE=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation -- --nocapture` | `running 50 tests`; `test result: ok. 50 passed; 0 failed; ... 179 filtered out` |
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:54:- `tauri-svelte-preview/src-tauri/src/agent_conversation/handoff.rs:1-755` — new DTOs, state machine, Tauri command, and nine required Rust tests.
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:55:- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:7-10,52-56,180-224,490-757,900` — handoff context, non-destructive detach, identity/history guards, lease transitions, commit/rollback, and fork capability default.
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:56:- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:141-147` — optional advertised fork capability.
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:57:- `tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:141` and `providers/adapter_support.rs:1350,1426-1433` — capability fixture/parsing updates.
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:58:- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs:131-133`, `providers/acp.rs:121-128,188-193`, `providers/acp_client.rs:76-80,224-228` — detach seam and capability parsing.
docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:59:- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:3` and `tauri-svelte-preview/src-tauri/src/main.rs:5404` — module and command registration.
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:14:DISCOVERY lane (read-only) for TSK-808, worktree /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave. GOAL: a controller-ready dispatch spec for Work Package A10 (resources/space/usage). READ the Work Package A10 section plus its referenced product decisions (§3) in docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md (search "Work Package A10"), then verify every file anchor against the CURRENT tree — A1-A9 landed after the plan was written, so confirm or correct each anchor. OUTPUT to stdout AND docs/superpowers/evidence/tsk-808/a10-dispatch-spec.md (one allowed write; stdout alone if blocked): exact files+line anchors, exact names/contracts, ordered edits, tests to write first with names and assertions, verification commands verbatim, done-means checklist, and any plan-vs-tree conflicts. Every claim file:line, labelled verified or assumed. NO edits, NO commits, plain English.
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:285:518-- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript.rs::parse_claude` classifies non-meta Claude `user` records as normal user messages, so `<task-notification>` envelopes leak into the timeline. `ConversationMessage.svelte` currently renders only paragraphs and triple-backtick blocks. [Task 1]
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:307:540:- TSK-808, TSK-799, TSK-798, Assembly, docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md, 2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md, contrast, legibility, gpt-5.6-sol, Tauri, Dockview, Monaco, Roslyn, U1
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:321:562:- The authoritative plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; use section 18 of `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md` instead of retired schedules. TSK-808 remained open/High; TSK-799 and TSK-798 are completed dependencies. [Task 1]
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:440:- TSK-808, TSK-799, TSK-798, Assembly, docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md, 2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md, contrast, legibility, gpt-5.6-sol, Tauri, Dockview, Monaco, Roslyn, U1
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:462:- The authoritative plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; use section 18 of `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md` instead of retired schedules. TSK-808 remained open/High; TSK-799 and TSK-798 are completed dependencies. [Task 1]
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:502:- The detailed plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; the combined ACP/orchestration amendment is `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`; planning guidance is in `docs/superpowers/plans/README.md`.
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:586:/bin/zsh -lc "PLAN='docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md'; printf '%s\\n' '--- A10 hits ---'; rg -n -C 8 'Work Package A10|A10|§3|Product decisions|product decisions' \""'$PLAN" | sed -n '"'1,260p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:811:/bin/zsh -lc "PLAN='docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md'; nl -ba \""'$PLAN" | sed -n '"'116,180p'; printf '%s\\n' '--- A10 full ---'; nl -ba \""'$PLAN" | sed -n '"'2160,2315p'; printf '%s\\n' '--- A10 verification commands/context ---'; nl -ba \""'$PLAN" | sed -n '"'2620,2705p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:853:   156	- `agentclientprotocol/codex-acp` — Codex app-server mapping for models, effort, Fast mode, permissions/sandbox, images, commands, file changes, reasoning, plans, reviews, web search, token/rate usage, and subagents;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:854:   157	- `agentclientprotocol/claude-agent-acp` — Claude Agent SDK mapping for models/options, images, tool permissions, TODOs, edit review, terminals, custom commands, MCP, and nested subagent transcripts.
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1047:  2631	RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation -- --nocapture
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1164:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1165:tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1166:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1167:tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1168:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1169:tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1170:tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1196:tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/mod.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1197:tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/codex.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1198:tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/claude.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1208:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1209:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1210:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/process.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1211:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1212:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1213:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1214:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1215:tauri-svelte-preview/src-tauri/src/agent_conversation/journal.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1216:tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1251:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/terminal_projection/no_invented_state.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1252:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/terminal_projection/codex_partial.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1253:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/terminal_projection/claude_append.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1254:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/terminal_projection/codex_append.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1262:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/claude/all-categories.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1263:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/claude/unknown-and-lifecycle.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1270:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/codex/all-categories.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1271:tauri-svelte-preview/src-tauri/fixtures/agent_conversation/codex/unknown-and-lifecycle.jsonl
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1390:tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1391:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1392:tauri-svelte-preview/src-tauri/src/agent_conversation/journal.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1393:tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1394:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1395:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1396:tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1397:tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1398:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1399:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1400:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1401:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1402:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1403:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1404:tauri-svelte-preview/src-tauri/src/agent_conversation/providers/process.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1405:tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1406:tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/claude.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1407:tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/codex.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1408:tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/mod.rs
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:1764:    11	pub use crate::agent_conversation::protocol::ToolTerminalIdentity;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:3988:    24	mod agent_conversation;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:8935:24-mod agent_conversation;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:9237:5303:        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10163:  5294	    let agent_runtime = agent_conversation::manager::AgentRuntimeManager::new(
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10164:  5295	        agent_conversation::providers::ProviderRegistry::bundled_from_environment()
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10172:  5303	        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10273:  5394	            agent_conversation::ensure_agent_conversation,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10274:  5395	            agent_conversation::send_agent_conversation_message,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10275:  5396	            agent_conversation::respond_agent_conversation_approval,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10276:  5397	            agent_conversation::stop_agent_conversation_turn,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10277:  5398	            agent_conversation::close_agent_conversation,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10278:  5399	            agent_conversation::read_agent_conversation_snapshot,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10279:  5400	            agent_conversation::read_agent_conversation_transcript,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10280:  5401	            agent_conversation::start_agent_conversation_terminal_projection,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10281:  5402	            agent_conversation::stop_agent_conversation_terminal_projection,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10282:  5403	            agent_conversation::save_agent_conversation_attachment,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10786:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:24:        return Err("Provider content hash must be a SHA-256 hex digest".to_string());
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10787:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:30:        return Err("Provider manifests must use a pinned packaged adapter".to_string());
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10788:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:61:            return Err("Config option ids must be unique".to_string());
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10789:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:65:                return Err(format!("Config option {} has no choices", option.id));
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10790:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:91:            return Err(format!("Unsupported value for config option {}", option.id));
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10791:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:99:        return Err("Provider executable is required".to_string());
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10792:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:102:        return Err("Provider executable must use an absolute packaged path".to_string());
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10793:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:105:        return Err("Provider executable does not exist".to_string());
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10794:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:122:    use crate::agent_conversation::protocol::{
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10795:tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:146:                resource_links: false,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10895:tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs:20:    owned_id: &str,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10896:tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs:24:    let owned_id = safe_segment(owned_id, "Owned session id")?;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10897:tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs:33:        .join(owned_id);
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10898:tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs:86:        return Err(format!("{label} is invalid"));
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10901:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:8:pub(crate) struct ConversationProcess {
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10902:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:10:    pub(crate) stdin: ChildStdin,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10903:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:11:    pub(crate) stdout: Lines<BufReader<ChildStdout>>,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10904:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:12:    pub(crate) stderr: Lines<BufReader<ChildStderr>>,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10905:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:16:    pub(crate) fn spawn(
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10906:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:20:        owned_id: &str,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10907:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:26:            .env("COMMANDBAR_SESSION_ID", owned_id)
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10908:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:56:    pub(crate) async fn stop(&mut self) {
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10909:tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs:81:pub(crate) fn validated_conversation_cwd(value: &str) -> Result<PathBuf, String> {
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10928:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:192:    const capabilities = await invoke<AgentCapabilities>('read_agent_conversation_capabilities', { ownedId });
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10949:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:262:  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10961:tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:306:  await invoke<boolean>('close_agent_conversation', { ownedId });
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10987:tauri-svelte-preview/src-tauri/src/workflow.rs:9:use crate::agent_conversation::manager::AgentRuntimeManager;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10988:tauri-svelte-preview/src-tauri/src/workflow.rs:10:use crate::agent_conversation::protocol::{
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:10989:tauri-svelte-preview/src-tauri/src/workflow.rs:13:use crate::agent_conversation::providers::AgentPrompt;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11032:/bin/zsh -lc "PLAN='docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md'; rg -n '"'^### Work Package A1|''^#### A1|resource identity|Resource'"' \""'$PLAN" | sed -n '"'1,220p'; nl -ba \""'$PLAN" | sed -n '"'1680,1760p'; printf '%s\\n' '--- A1 section candidates ---'; nl -ba \""'$PLAN" | sed -n '"'1930,2085p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11143:  1760	Current anchors are `agent_conversation/process.rs:8-92` (`ConversationProcess::spawn/stop`), the
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11303:/bin/zsh -lc "PLAN='docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md'; nl -ba \""'$PLAN" | sed -n '"'1790,1940p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11319:  1804	`conversationStore.svelte.ts:23-236`, and Rust `agent_conversation/protocol.rs:3-166` plus
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11343:  1828	`send_command`, `snapshot`, and `close` in `agent_conversation/mod.rs`. `TerminalRegistry` remains
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11346:  1831	Own new `agent_conversation/manager.rs` (`AgentRuntimeManager`, `ManagedAgentSession`, writer lease,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11349:  1834	`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11361:  1846	Run `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation` in one
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11376:  1861	`src-tauri/fixtures/agent_conversation/codex/*.jsonl`, and optional
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11384:  1869	`src-tauri/fixtures/agent_conversation/claude/*.jsonl`, and optional
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11389:  1874	**A3-terminal:** replace whole-tail `agent_conversation/transcript.rs` with
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11433:  1918	`agent_conversation/handoff.rs` for structured-to-terminal, terminal-to-structured, same-session/
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11458:/bin/zsh -lc "printf '%s\\n' '--- ownedSessions.ts ---'; nl -ba tauri-svelte-preview/src/lib/shell/ownedSessions.ts | sed -n '1,320p'; printf '%s\\n' '--- session rail types/store ---'; rg -n -C 3 'export type OwnedSession|interface OwnedSession|ownedId|generation|provider|native|ptySession' tauri-svelte-preview/src/lib/shell/stores/sessionRailStore.svelte.ts tauri-svelte-preview/src/lib/shell/ownedSessions.ts tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts | sed -n '1,500p'; printf '%s\\n' '--- rust protocol owner fields ---'; rg -n -C 3 'owned_id|ownedId|generation|owner|ProviderUsage|Token|Usage|Process' tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs | sed -n '1,500p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:11796:tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts-31-export type ProviderTransport = 'acp-stdio' | 'acp-websocket' | 'built-in';
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12152:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-23-    let connection = manager.ensure(request)?;
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12153:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-24-    if manager.providers().manifest(connection.provider).is_ok() {
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12154:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-25-        manager
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12155:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:26:            .activate(&connection.owned_id, connection.generation)
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12156:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-27-            .await
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12157:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-28-    } else {
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12158:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-29-        // A0 installs the packaged, hash-recorded adapters. Until then the
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12160:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-43-    }
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12161:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-44-    manager
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12162:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-45-        .prompt(
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12163:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:46:            &request.owned_id,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12164:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:47:            request.generation,
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12165:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-48-            AgentPrompt {
docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log:12166:tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs-49-                text: text.to_string(),

exec
/bin/zsh -lc "rg -n \"StructuredRuntimeHandle|struct AgentRuntimeManager|ensure_agent_conversation|send_agent_conversation_message|agent-conversation-event|AgentConversationEvent|TerminalProjection\" tauri-svelte-preview/src-tauri/src/agent_conversation tauri-svelte-preview/src-tauri/src/main.rs | sed -n '1,300p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src-tauri/src/main.rs:5327:        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
tauri-svelte-preview/src-tauri/src/main.rs:5440:            agent_conversation::ensure_agent_conversation,
tauri-svelte-preview/src-tauri/src/main.rs:5441:            agent_conversation::send_agent_conversation_message,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:13:pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:23:pub struct StartTerminalProjectionRequest {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:32:pub struct TerminalProjectionRegistration {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:194:pub struct TerminalProjectionRegistry {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:198:impl TerminalProjectionRegistry {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:202:        request: StartTerminalProjectionRequest,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:203:    ) -> Result<TerminalProjectionRegistration, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:282:        Ok(TerminalProjectionRegistration {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:304:impl Drop for TerminalProjectionRegistry {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:633:        let registry = TerminalProjectionRegistry::default();
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:20:pub async fn ensure_agent_conversation(
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:37:pub async fn send_agent_conversation_message(
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:111:    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:112:    request: terminal_projection::StartTerminalProjectionRequest,
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:113:) -> Result<terminal_projection::TerminalProjectionRegistration, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:119:    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:14:    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:25:    ProviderRegistry, StructuredRuntimeHandle,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:42:    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:48:    frontend_events: VecDeque<AgentConversationEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:59:pub struct AgentRuntimeManager {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:239:        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:270:    ) -> Result<AgentConversationEvent, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:290:        let frontend_event = AgentConversationEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:814:    ) -> Result<Arc<AsyncMutex<StructuredRuntimeHandle>>, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs:146:pub enum StructuredRuntimeHandle {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs:150:impl StructuredRuntimeHandle {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs:8:pub use acp::{AcpRuntimeAdapter, StructuredRuntimeHandle};
tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:520:pub struct AgentConversationEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:584:    pub events: Vec<AgentConversationEvent>,

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs | sed -n '1,180p'
printf '\\n--- manager ---\\n'
nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs | sed -n '1,360p;760,900p'
printf '\\n--- acp ---\\n'
nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs | sed -n '1,260p'
printf '\\n--- main commands ---\\n'
nl -ba tauri-svelte-preview/src-tauri/src/main.rs | sed -n '5310,5460p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	mod attachments;
     2	pub mod capabilities;
     3	pub mod handoff;
     4	pub mod journal;
     5	pub mod manager;
     6	pub mod protocol;
     7	pub mod providers;
     8	pub mod terminal_projection;
     9	mod transcript;
    10	
    11	use manager::AgentRuntimeManager;
    12	use protocol::{
    13	    AgentConversationConnection, AgentConversationSnapshot, EnsureAgentConversationRequest,
    14	    RespondAgentConversationApprovalRequest, SendAgentConversationMessageRequest,
    15	    StopAgentConversationTurnRequest,
    16	};
    17	use providers::AgentPrompt;
    18	
    19	#[tauri::command]
    20	pub async fn ensure_agent_conversation(
    21	    manager: tauri::State<'_, AgentRuntimeManager>,
    22	    request: EnsureAgentConversationRequest,
    23	) -> Result<AgentConversationConnection, String> {
    24	    let connection = manager.ensure(request)?;
    25	    if manager.providers().manifest(connection.provider).is_ok() {
    26	        manager
    27	            .activate(&connection.owned_id, connection.generation)
    28	            .await
    29	    } else {
    30	        // A0 installs the packaged, hash-recorded adapters. Until then the
    31	        // existing connecting response remains available without a fallback PTY.
    32	        Ok(connection)
    33	    }
    34	}
    35	
    36	#[tauri::command]
    37	pub async fn send_agent_conversation_message(
    38	    manager: tauri::State<'_, AgentRuntimeManager>,
    39	    request: SendAgentConversationMessageRequest,
    40	) -> Result<(), String> {
    41	    let text = request.text.trim();
    42	    if text.is_empty() {
    43	        return Err("Message cannot be empty".to_string());
    44	    }
    45	    manager
    46	        .prompt(
    47	            &request.owned_id,
    48	            request.generation,
    49	            AgentPrompt {
    50	                text: text.to_string(),
    51	                images: Vec::new(),
    52	            },
    53	        )
    54	        .await
    55	}
    56	
    57	#[tauri::command]
    58	pub async fn respond_agent_conversation_approval(
    59	    manager: tauri::State<'_, AgentRuntimeManager>,
    60	    request: RespondAgentConversationApprovalRequest,
    61	) -> Result<(), String> {
    62	    let request_id = required_id(&request.request_id, "Approval request id")?;
    63	    manager
    64	        .respond_legacy_approval(
    65	            &request.owned_id,
    66	            request.generation,
    67	            request_id,
    68	            request.decision,
    69	        )
    70	        .await
    71	}
    72	
    73	#[tauri::command]
    74	pub async fn stop_agent_conversation_turn(
    75	    manager: tauri::State<'_, AgentRuntimeManager>,
    76	    request: StopAgentConversationTurnRequest,
    77	) -> Result<(), String> {
    78	    manager
    79	        .cancel_turn(&request.owned_id, request.generation)
    80	        .await
    81	}
    82	
    83	#[tauri::command]
    84	pub async fn close_agent_conversation(
    85	    manager: tauri::State<'_, AgentRuntimeManager>,
    86	    owned_id: String,
    87	) -> Result<bool, String> {
    88	    manager.close(&owned_id).await
    89	}
    90	
    91	#[tauri::command]
    92	pub async fn read_agent_conversation_snapshot(
    93	    manager: tauri::State<'_, AgentRuntimeManager>,
    94	    owned_id: String,
    95	) -> Result<Option<AgentConversationSnapshot>, String> {
    96	    manager.snapshot(&owned_id)
    97	}
    98	
    99	#[tauri::command]
   100	pub async fn read_agent_conversation_transcript(
   101	    provider: String,
   102	    native_session_id: String,
   103	    child_session_id: Option<String>,
   104	) -> Result<transcript::TranscriptSnapshot, String> {
   105	    transcript::read(&provider, &native_session_id, child_session_id.as_deref())
   106	}
   107	
   108	#[tauri::command]
   109	pub async fn start_agent_conversation_terminal_projection(
   110	    app: tauri::AppHandle,
   111	    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
   112	    request: terminal_projection::StartTerminalProjectionRequest,
   113	) -> Result<terminal_projection::TerminalProjectionRegistration, String> {
   114	    registry.start(app, request)
   115	}
   116	
   117	#[tauri::command]
   118	pub async fn stop_agent_conversation_terminal_projection(
   119	    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
   120	    owned_id: String,
   121	) -> Result<bool, String> {
   122	    registry.stop(&owned_id)
   123	}
   124	
   125	#[tauri::command]
   126	pub async fn save_agent_conversation_attachment(
   127	    app: tauri::AppHandle,
   128	    owned_id: String,
   129	    mime_type: String,
   130	    bytes: Vec<u8>,
   131	) -> Result<attachments::SavedConversationAttachment, String> {
   132	    attachments::save(&app, &owned_id, &mime_type, &bytes)
   133	}
   134	
   135	fn required_id(value: &str, label: &str) -> Result<String, String> {
   136	    let value = value.trim();
   137	    if value.is_empty() {
   138	        Err(format!("{label} is required"))
   139	    } else {
   140	        Ok(value.to_string())
   141	    }
   142	}

--- manager ---
     1	use std::collections::{BTreeMap, HashMap, VecDeque};
     2	use std::sync::{Arc, Mutex};
     3	use std::time::{SystemTime, UNIX_EPOCH};
     4	
     5	use tokio::sync::Mutex as AsyncMutex;
     6	
     7	use super::handoff::{
     8	    AgentConversationHandoffDirection, AgentConversationHandoffMode, AgentConversationHandoffPhase,
     9	    AgentConversationHandoffReceipt, AgentConversationHandoffRequest,
    10	};
    11	use super::journal::AgentEventJournal;
    12	use super::protocol::{
    13	    AgentApprovalDecision, AgentApprovalResponse, AgentCapabilities, AgentConversationConnection,
    14	    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
    15	    AgentConversationSnapshot, AgentEvent, AgentEventType, AgentExecutionOwner,
    16	    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
    17	    AgentRequestIdentity, AgentRuntimeState, AgentSessionCapabilities, AgentUserInputResponse,
    18	    AgentWriterLease, AgentWriterLeaseOwner, AgentWriterLeaseTransition,
    19	    ConversationConnectionState, EnsureAgentConversationRequest,
    20	};
    21	use super::providers::process::validated_conversation_cwd;
    22	use super::providers::{
    23	    AcpRuntimeAdapter, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter, AgentSteeringInput,
    24	    GeneratedText, InitializeAgentInput, LoadAgentSession, NewAgentSession, PermissionResponse,
    25	    ProviderRegistry, StructuredRuntimeHandle,
    26	};
    27	
    28	const SNAPSHOT_EVENT_CAP: usize = 2_000;
    29	const JOURNAL_EVENT_CAP: usize = 10_000;
    30	
    31	pub struct ManagedAgentSession {
    32	    pub owned_id: String,
    33	    pub provider: AgentConversationProvider,
    34	    pub provider_instance_id: String,
    35	    pub native_session_id: Option<String>,
    36	    pub generation: u64,
    37	    pub owner: AgentExecutionOwner,
    38	    pub state: AgentRuntimeState,
    39	    pub capabilities: AgentCapabilities,
    40	    pub next_sequence: u64,
    41	    pub active_turn_id: Option<String>,
    42	    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
    43	    pub recent_events: VecDeque<AgentEvent>,
    44	    pub writer_lease: AgentWriterLease,
    45	    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
    46	    cwd: String,
    47	    connection: AgentConversationConnection,
    48	    frontend_events: VecDeque<AgentConversationEvent>,
    49	    journal: AgentEventJournal,
    50	}
    51	
    52	pub(crate) struct HandoffContext {
    53	    pub previous_owner: AgentWriterLeaseOwner,
    54	    pub owner: AgentWriterLeaseOwner,
    55	    pub native_session_id: Option<String>,
    56	}
    57	
    58	#[derive(Clone)]
    59	pub struct AgentRuntimeManager {
    60	    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    61	    providers: Arc<ProviderRegistry>,
    62	}
    63	
    64	#[derive(Debug, Clone, PartialEq, Eq)]
    65	pub struct AgentResourceRoot {
    66	    pub pid: u32,
    67	    pub owned_id: String,
    68	    pub provider: AgentConversationProvider,
    69	    pub cwd: String,
    70	}
    71	
    72	impl Default for AgentRuntimeManager {
    73	    fn default() -> Self {
    74	        Self::new(ProviderRegistry::default())
    75	    }
    76	}
    77	
    78	impl AgentRuntimeManager {
    79	    pub fn new(providers: ProviderRegistry) -> Self {
    80	        Self {
    81	            sessions: Arc::new(Mutex::new(HashMap::new())),
    82	            providers: Arc::new(providers),
    83	        }
    84	    }
    85	
    86	    pub fn providers(&self) -> &ProviderRegistry {
    87	        &self.providers
    88	    }
    89	
    90	    /// Snapshot the provider processes that this registry currently owns.
    91	    /// The async runtime mutex is intentionally sampled with `try_lock`: a
    92	    /// resource refresh must never block an agent turn. A session that is in a
    93	    /// transition simply appears on the next refresh.
    94	    pub fn resource_roots(&self) -> Vec<AgentResourceRoot> {
    95	        let Ok(sessions) = self.sessions.lock() else {
    96	            return Vec::new();
    97	        };
    98	        sessions
    99	            .values()
   100	            .filter_map(|session| {
   101	                let runtime = session.runtime.as_ref()?;
   102	                let runtime = runtime.try_lock().ok()?;
   103	                let pid = runtime.process_id()?;
   104	                Some(AgentResourceRoot {
   105	                    pid,
   106	                    owned_id: session.owned_id.clone(),
   107	                    provider: session.provider,
   108	                    cwd: session.cwd.clone(),
   109	                })
   110	            })
   111	            .collect()
   112	    }
   113	
   114	    /// Returns the capability snapshot advertised by the active ACP session. WorkflowEngine uses
   115	    /// this to map role policy onto provider-owned config options without guessing option ids.
   116	    pub fn capabilities(
   117	        &self,
   118	        owned_id: &str,
   119	        generation: u64,
   120	    ) -> Result<AgentCapabilities, String> {
   121	        let sessions = self
   122	            .sessions
   123	            .lock()
   124	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   125	        Ok(current_session(&sessions, owned_id, generation)?
   126	            .capabilities
   127	            .clone())
   128	    }
   129	
   130	    pub fn ensure(
   131	        &self,
   132	        request: EnsureAgentConversationRequest,
   133	    ) -> Result<AgentConversationConnection, String> {
   134	        let owned_id = required_id(&request.owned_id, "Owned session id")?;
   135	        let cwd = validated_conversation_cwd(&request.cwd)?
   136	            .display()
   137	            .to_string();
   138	        let mut sessions = self
   139	            .sessions
   140	            .lock()
   141	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   142	        if let Some(current) = sessions.get(&owned_id) {
   143	            if current.provider == request.provider && current.cwd == cwd {
   144	                return Ok(current.connection.clone());
   145	            }
   146	        }
   147	        let prior = sessions.remove(&owned_id);
   148	        let generation = prior
   149	            .as_ref()
   150	            .map(|session| session.generation.saturating_add(1))
   151	            .unwrap_or(1);
   152	        let connection = AgentConversationConnection {
   153	            owned_id: owned_id.clone(),
   154	            provider: request.provider,
   155	            generation,
   156	            native_session_id: normalized_optional_id(request.native_session_id),
   157	            state: ConversationConnectionState::Connecting,
   158	        };
   159	        let provider_instance_id = format!("{}-{generation}", provider_id(request.provider));
   160	        sessions.insert(
   161	            owned_id.clone(),
   162	            ManagedAgentSession {
   163	                owned_id: owned_id.clone(),
   164	                provider: request.provider,
   165	                provider_instance_id,
   166	                native_session_id: connection.native_session_id.clone(),
   167	                generation,
   168	                owner: AgentExecutionOwner::Stopped,
   169	                state: AgentRuntimeState::Closed,
   170	                capabilities: empty_capabilities(request.provider),
   171	                next_sequence: 1,
   172	                active_turn_id: None,
   173	                runtime: None,
   174	                recent_events: VecDeque::new(),
   175	                writer_lease: AgentWriterLease {
   176	                    owned_id,
   177	                    generation,
   178	                    owner: AgentWriterLeaseOwner::None,
   179	                },
   180	                writer_lease_transition: None,
   181	                cwd,
   182	                connection: connection.clone(),
   183	                frontend_events: VecDeque::new(),
   184	                journal: AgentEventJournal::new(JOURNAL_EVENT_CAP),
   185	            },
   186	        );
   187	        Ok(connection)
   188	    }
   189	
   190	    pub async fn activate(
   191	        &self,
   192	        owned_id: &str,
   193	        generation: u64,
   194	    ) -> Result<AgentConversationConnection, String> {
   195	        let (provider, provider_instance_id, cwd, native_session_id) = {
   196	            let sessions = self
   197	                .sessions
   198	                .lock()
   199	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   200	            let session = current_session(&sessions, owned_id, generation)?;
   201	            (
   202	                session.provider,
   203	                session.provider_instance_id.clone(),
   204	                std::path::PathBuf::from(&session.cwd),
   205	                session.native_session_id.clone(),
   206	            )
   207	        };
   208	        let manifest = self.providers.manifest(provider)?;
   209	        let mut adapter = AcpRuntimeAdapter::new(manifest, owned_id.to_string(), cwd.clone());
   210	        let capabilities = adapter
   211	            .initialize(InitializeAgentInput {
   212	                provider,
   213	                provider_instance_id,
   214	            })
   215	            .await
   216	            .map_err(|error| error.to_string())?;
   217	        let expected_native_session_id = native_session_id.clone();
   218	        let started = match native_session_id {
   219	            Some(native_session_id) => {
   220	                adapter
   221	                    .resume_session(LoadAgentSession {
   222	                        cwd,
   223	                        native_session_id,
   224	                    })
   225	                    .await
   226	            }
   227	            None => adapter.new_session(NewAgentSession { cwd }).await,
   228	        }
   229	        .map_err(|error| error.to_string())?;
   230	        if let Some(expected_native_session_id) = expected_native_session_id {
   231	            if started.native_session_id != expected_native_session_id {
   232	                let _ = adapter.detach_session().await;
   233	                return Err(
   234	                    "The provider resumed a different native session; handoff was rejected"
   235	                        .to_string(),
   236	                );
   237	            }
   238	        }
   239	        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
   240	        let mut sessions = self
   241	            .sessions
   242	            .lock()
   243	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   244	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   245	        session.capabilities = capabilities;
   246	        session.native_session_id = Some(started.native_session_id.clone());
   247	        session.connection.native_session_id = Some(started.native_session_id);
   248	        session.connection.state = ConversationConnectionState::Connected;
   249	        let restoring_terminal_transition = session.owner
   250	            == AgentExecutionOwner::TransitioningToStructured
   251	            && session
   252	                .writer_lease_transition
   253	                .as_ref()
   254	                .map(|transition| transition.to == AgentWriterLeaseOwner::Structured)
   255	                .unwrap_or(false);
   256	        if !restoring_terminal_transition {
   257	            session.owner = AgentExecutionOwner::Structured;
   258	            session.writer_lease.owner = AgentWriterLeaseOwner::Structured;
   259	        }
   260	        session.state = AgentRuntimeState::Ready;
   261	        session.runtime = Some(runtime);
   262	        Ok(session.connection.clone())
   263	    }
   264	
   265	    pub fn emit_payload(
   266	        &self,
   267	        owned_id: &str,
   268	        generation: u64,
   269	        payload: AgentConversationPayload,
   270	    ) -> Result<AgentConversationEvent, String> {
   271	        let mut sessions = self
   272	            .sessions
   273	            .lock()
   274	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   275	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   276	        let sequence = session.next_sequence;
   277	        session.next_sequence = session.next_sequence.saturating_add(1);
   278	        if let AgentConversationPayload::Connection {
   279	            state,
   280	            native_session_id,
   281	        } = &payload
   282	        {
   283	            session.connection.state = *state;
   284	            if native_session_id.is_some() {
   285	                session.connection.native_session_id = native_session_id.clone();
   286	                session.native_session_id = native_session_id.clone();
   287	            }
   288	        }
   289	        let timestamp_ms = timestamp_millis();
   290	        let frontend_event = AgentConversationEvent {
   291	            owned_id: owned_id.to_string(),
   292	            provider: session.provider,
   293	            generation,
   294	            sequence,
   295	            timestamp_ms,
   296	            payload: payload.clone(),
   297	        };
   298	        let canonical = canonical_event(session, sequence, timestamp_ms, &payload)?;
   299	        session.journal.append(canonical.clone())?;
   300	        session.recent_events.push_back(canonical);
   301	        session.frontend_events.push_back(frontend_event.clone());
   302	        while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
   303	            session.recent_events.pop_front();
   304	        }
   305	        while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
   306	            session.frontend_events.pop_front();
   307	        }
   308	        Ok(frontend_event)
   309	    }
   310	
   311	    pub async fn prompt(
   312	        &self,
   313	        owned_id: &str,
   314	        generation: u64,
   315	        input: AgentPrompt,
   316	    ) -> Result<(), String> {
   317	        let runtime = self.runtime(owned_id, generation)?;
   318	        let started = runtime
   319	            .lock()
   320	            .await
   321	            .prompt(input)
   322	            .await
   323	            .map_err(|error| error.to_string())?;
   324	        let mut sessions = self
   325	            .sessions
   326	            .lock()
   327	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   328	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   329	        session.active_turn_id = started.turn_id;
   330	        session.state = AgentRuntimeState::Working;
   331	        Ok(())
   332	    }
   333	
   334	    /// Run one product action through the current structured agent without
   335	    /// writing through the conversation UI. The runtime and generation checks
   336	    /// remain the same as an ordinary conversation prompt.
   337	    pub async fn prompt_once(
   338	        &self,
   339	        owned_id: &str,
   340	        generation: u64,
   341	        input: AgentPrompt,
   342	    ) -> Result<GeneratedText, String> {
   343	        let runtime = self.runtime(owned_id, generation)?;
   344	        let result = runtime
   345	            .lock()
   346	            .await
   347	            .prompt_once(input)
   348	            .await
   349	            .map_err(|error| error.to_string());
   350	        result
   351	    }
   352	
   353	    pub async fn respond_permission(&self, input: PermissionResponse) -> Result<(), String> {
   354	        let runtime = self.runtime(&input.identity.owned_id, input.identity.generation)?;
   355	        let result = runtime
   356	            .lock()
   357	            .await
   358	            .respond_permission(input)
   359	            .await
   360	            .map_err(|error| error.to_string());
   760	    pub(crate) fn handoff_rollback(
   761	        &self,
   762	        request: &AgentConversationHandoffRequest,
   763	    ) -> Result<AgentConversationHandoffReceipt, String> {
   764	        let mut sessions = self
   765	            .sessions
   766	            .lock()
   767	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   768	        let session = current_session_mut(&mut sessions, &request.owned_id, request.generation)?;
   769	        let transition = session
   770	            .writer_lease_transition
   771	            .take()
   772	            .ok_or_else(|| "Handoff rollback has no prepared transition".to_string())?;
   773	        session.writer_lease.owner = transition.from;
   774	        session.owner = match transition.from {
   775	            AgentWriterLeaseOwner::Structured => AgentExecutionOwner::Structured,
   776	            AgentWriterLeaseOwner::Terminal => AgentExecutionOwner::Terminal,
   777	            AgentWriterLeaseOwner::None => AgentExecutionOwner::Stopped,
   778	        };
   779	        Ok(AgentConversationHandoffReceipt {
   780	            owned_id: request.owned_id.clone(),
   781	            generation: request.generation,
   782	            direction: request.direction,
   783	            mode: request.mode,
   784	            phase: AgentConversationHandoffPhase::Rollback,
   785	            previous_owner: transition.from,
   786	            owner: transition.from,
   787	            native_session_id: session.native_session_id.clone(),
   788	            pty_session_id: request
   789	                .pty_session_id
   790	                .clone()
   791	                .or_else(|| request.process_tree.pty_session_id.clone()),
   792	            history_boundary: request.history_boundary.clone(),
   793	            process_tree: request.process_tree.clone(),
   794	            rollback_available: false,
   795	            message: "Handoff rolled back; existing terminal scrollback was preserved".to_string(),
   796	        })
   797	    }
   798	
   799	    pub fn canonical_snapshot(&self, owned_id: &str) -> Result<Vec<AgentEvent>, String> {
   800	        let sessions = self
   801	            .sessions
   802	            .lock()
   803	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   804	        Ok(sessions
   805	            .get(owned_id)
   806	            .map(|session| session.recent_events.iter().cloned().collect())
   807	            .unwrap_or_default())
   808	    }
   809	
   810	    fn runtime(
   811	        &self,
   812	        owned_id: &str,
   813	        generation: u64,
   814	    ) -> Result<Arc<AsyncMutex<StructuredRuntimeHandle>>, String> {
   815	        let sessions = self
   816	            .sessions
   817	            .lock()
   818	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   819	        let session = current_session(&sessions, owned_id, generation)?;
   820	        if session.owner != AgentExecutionOwner::Structured {
   821	            return Err("The structured writer is not the current owner".to_string());
   822	        }
   823	        session
   824	            .runtime
   825	            .clone()
   826	            .ok_or_else(|| "Structured provider is still connecting".to_string())
   827	    }
   828	
   829	    #[cfg(test)]
   830	    pub(crate) fn session_terminal_ownership(
   831	        &self,
   832	        owned_id: &str,
   833	    ) -> Option<AgentWriterLeaseOwner> {
   834	        self.sessions
   835	            .lock()
   836	            .ok()?
   837	            .get(owned_id)
   838	            .map(|session| session.writer_lease.owner)
   839	    }
   840	}
   841	
   842	impl Drop for AgentRuntimeManager {
   843	    fn drop(&mut self) {
   844	        if Arc::strong_count(&self.sessions) == 1 {
   845	            if let Ok(mut sessions) = self.sessions.lock() {
   846	                sessions.clear();
   847	            }
   848	        }
   849	    }
   850	}
   851	
   852	fn current_session<'a>(
   853	    sessions: &'a HashMap<String, ManagedAgentSession>,
   854	    owned_id: &str,
   855	    generation: u64,
   856	) -> Result<&'a ManagedAgentSession, String> {
   857	    let session = sessions
   858	        .get(owned_id)
   859	        .ok_or_else(|| "Conversation session was not found".to_string())?;
   860	    if session.generation != generation {
   861	        return Err("Conversation connection changed; retry on the current session".to_string());
   862	    }
   863	    Ok(session)
   864	}
   865	
   866	fn current_session_mut<'a>(
   867	    sessions: &'a mut HashMap<String, ManagedAgentSession>,
   868	    owned_id: &str,
   869	    generation: u64,
   870	) -> Result<&'a mut ManagedAgentSession, String> {
   871	    let session = sessions
   872	        .get_mut(owned_id)
   873	        .ok_or_else(|| "Conversation session was not found".to_string())?;
   874	    if session.generation != generation {
   875	        return Err("Conversation connection changed; retry on the current session".to_string());
   876	    }
   877	    Ok(session)
   878	}
   879	
   880	fn canonical_event(
   881	    session: &ManagedAgentSession,
   882	    sequence: u64,
   883	    timestamp_ms: u128,
   884	    payload: &AgentConversationPayload,
   885	) -> Result<AgentEvent, String> {
   886	    let event_type = match payload {
   887	        AgentConversationPayload::Connection {
   888	            state: ConversationConnectionState::Closed,
   889	            ..
   890	        } => AgentEventType::SessionClosed,
   891	        AgentConversationPayload::Connection { .. } => AgentEventType::SessionStateChanged,
   892	        AgentConversationPayload::UserMessage { .. } => AgentEventType::ItemCompleted,
   893	        AgentConversationPayload::AssistantDelta { .. } => AgentEventType::ContentDelta,
   894	        AgentConversationPayload::AssistantMessage { .. } => AgentEventType::ItemCompleted,
   895	        AgentConversationPayload::Tool { .. } => AgentEventType::ItemUpdated,
   896	        AgentConversationPayload::Approval { .. } => AgentEventType::ApprovalRequested,
   897	        AgentConversationPayload::Turn {
   898	            state: super::protocol::TurnState::Started,
   899	            ..
   900	        } => AgentEventType::TurnStarted,

--- acp ---
     1	use super::super::capabilities::{replace_config_options, validate_capabilities};
     2	use super::super::protocol::{AgentCapabilities, AgentProviderManifest};
     3	use super::{
     4	    AcpClient, AgentConfigOption, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter,
     5	    AgentRuntimeError, AgentSteeringInput, GeneratedText, InitializeAgentInput, LoadAgentSession,
     6	    NewAgentSession, PermissionResponse, ResumeAgentSession, StartedAgentSession, StartedTurn,
     7	    UserInputResponse,
     8	};
     9	
    10	pub struct AcpRuntimeAdapter {
    11	    manifest: AgentProviderManifest,
    12	    owned_id: String,
    13	    cwd: std::path::PathBuf,
    14	    client: Option<AcpClient>,
    15	    capabilities: Option<AgentCapabilities>,
    16	}
    17	
    18	impl AcpRuntimeAdapter {
    19	    pub fn new(manifest: AgentProviderManifest, owned_id: String, cwd: std::path::PathBuf) -> Self {
    20	        Self {
    21	            manifest,
    22	            owned_id,
    23	            cwd,
    24	            client: None,
    25	            capabilities: None,
    26	        }
    27	    }
    28	
    29	    fn client_mut(&mut self) -> Result<&mut AcpClient, AgentRuntimeError> {
    30	        self.client.as_mut().ok_or_else(|| {
    31	            AgentRuntimeError::new("not-initialized", "ACP adapter is not initialized")
    32	        })
    33	    }
    34	
    35	    pub fn process_id(&self) -> Option<u32> {
    36	        self.client.as_ref().and_then(AcpClient::process_id)
    37	    }
    38	}
    39	
    40	impl AgentRuntimeAdapter for AcpRuntimeAdapter {
    41	    async fn initialize(
    42	        &mut self,
    43	        input: InitializeAgentInput,
    44	    ) -> Result<AgentCapabilities, AgentRuntimeError> {
    45	        let mut client = AcpClient::spawn(&self.manifest, &self.cwd, &self.owned_id)?;
    46	        let capabilities = client.initialize(input.provider).await?;
    47	        validate_capabilities(&capabilities)
    48	            .map_err(|message| AgentRuntimeError::new("invalid-capabilities", message))?;
    49	        self.client = Some(client);
    50	        self.capabilities = Some(capabilities.clone());
    51	        Ok(capabilities)
    52	    }
    53	
    54	    async fn new_session(
    55	        &mut self,
    56	        input: NewAgentSession,
    57	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    58	        self.client_mut()?.new_session(&input.cwd).await
    59	    }
    60	    async fn load_session(
    61	        &mut self,
    62	        input: LoadAgentSession,
    63	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    64	        self.client_mut()?
    65	            .load_session(&input.cwd, &input.native_session_id)
    66	            .await
    67	    }
    68	    async fn resume_session(
    69	        &mut self,
    70	        input: ResumeAgentSession,
    71	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    72	        self.client_mut()?
    73	            .resume_session(&input.cwd, &input.native_session_id)
    74	            .await
    75	    }
    76	    async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
    77	        self.client_mut()?.prompt(input).await
    78	    }
    79	
    80	    async fn prompt_once(
    81	        &mut self,
    82	        input: AgentPrompt,
    83	    ) -> Result<GeneratedText, AgentRuntimeError> {
    84	        self.client_mut()?.prompt_once(input).await
    85	    }
    86	    async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
    87	        self.client_mut()?.steer(input.text).await
    88	    }
    89	    async fn cancel_turn(&mut self, _turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
    90	        self.client_mut()?.cancel().await
    91	    }
    92	
    93	    async fn set_config(
    94	        &mut self,
    95	        option_id: &str,
    96	        value: AgentConfigValue,
    97	    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
    98	        let replacement = self.client_mut()?.set_config(option_id, value).await?;
    99	        if let Some(capabilities) = &mut self.capabilities {
   100	            replace_config_options(capabilities, replacement.clone())
   101	                .map_err(|message| AgentRuntimeError::new("invalid-config", message))?;
   102	        }
   103	        Ok(replacement)
   104	    }
   105	
   106	    async fn respond_permission(
   107	        &mut self,
   108	        input: PermissionResponse,
   109	    ) -> Result<(), AgentRuntimeError> {
   110	        let decision = match input.decision {
   111	            super::super::protocol::AgentApprovalDecision::Accept => "selected",
   112	            super::super::protocol::AgentApprovalDecision::Decline => "cancelled",
   113	            super::super::protocol::AgentApprovalDecision::Cancel => "cancelled",
   114	        };
   115	        self.client_mut()?
   116	            .respond_permission(&input.identity.request_id, decision)
   117	            .await
   118	    }
   119	
   120	    async fn respond_user_input(
   121	        &mut self,
   122	        input: UserInputResponse,
   123	    ) -> Result<(), AgentRuntimeError> {
   124	        self.client_mut()?
   125	            .respond_user_input(&input.identity.request_id, input.values, input.cancelled)
   126	            .await
   127	    }
   128	
   129	    async fn detach_session(&mut self) -> Result<(), AgentRuntimeError> {
   130	        if let Some(client) = &mut self.client {
   131	            client.detach().await?;
   132	        }
   133	        self.client = None;
   134	        Ok(())
   135	    }
   136	
   137	    async fn close_session(&mut self) -> Result<(), AgentRuntimeError> {
   138	        if let Some(client) = &mut self.client {
   139	            client.close().await?;
   140	        }
   141	        self.client = None;
   142	        Ok(())
   143	    }
   144	}
   145	
   146	pub enum StructuredRuntimeHandle {
   147	    Acp(AcpRuntimeAdapter),
   148	}
   149	
   150	impl StructuredRuntimeHandle {
   151	    /// Return the OS process that backs this structured provider session.
   152	    /// Resources uses this as the root of the app-owned process tree; it is
   153	    /// intentionally read-only and never exposes the provider transport.
   154	    pub fn process_id(&self) -> Option<u32> {
   155	        match self {
   156	            Self::Acp(adapter) => adapter.process_id(),
   157	        }
   158	    }
   159	
   160	    pub async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
   161	        match self {
   162	            Self::Acp(adapter) => adapter.prompt(input).await,
   163	        }
   164	    }
   165	    pub async fn prompt_once(
   166	        &mut self,
   167	        input: AgentPrompt,
   168	    ) -> Result<GeneratedText, AgentRuntimeError> {
   169	        match self {
   170	            Self::Acp(adapter) => adapter.prompt_once(input).await,
   171	        }
   172	    }
   173	    pub async fn cancel_turn(&mut self, turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
   174	        match self {
   175	            Self::Acp(adapter) => adapter.cancel_turn(turn_id).await,
   176	        }
   177	    }
   178	    pub async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
   179	        match self {
   180	            Self::Acp(adapter) => adapter.steer(input).await,
   181	        }
   182	    }
   183	    pub async fn set_config(
   184	        &mut self,
   185	        option_id: &str,
   186	        value: AgentConfigValue,
   187	    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
   188	        match self {
   189	            Self::Acp(adapter) => adapter.set_config(option_id, value).await,
   190	        }
   191	    }
   192	    pub async fn respond_permission(
   193	        &mut self,
   194	        input: PermissionResponse,
   195	    ) -> Result<(), AgentRuntimeError> {
   196	        match self {
   197	            Self::Acp(adapter) => adapter.respond_permission(input).await,
   198	        }
   199	    }
   200	    pub async fn respond_user_input(
   201	        &mut self,
   202	        input: UserInputResponse,
   203	    ) -> Result<(), AgentRuntimeError> {
   204	        match self {
   205	            Self::Acp(adapter) => adapter.respond_user_input(input).await,
   206	        }
   207	    }
   208	    pub async fn close_session(&mut self) -> Result<(), AgentRuntimeError> {
   209	        match self {
   210	            Self::Acp(adapter) => adapter.close_session().await,
   211	        }
   212	    }
   213	
   214	    pub async fn detach_session(&mut self) -> Result<(), AgentRuntimeError> {
   215	        match self {
   216	            Self::Acp(adapter) => adapter.detach_session().await,
   217	        }
   218	    }
   219	}

--- main commands ---
  5310	fn source_file_matches_query(relative_path: &str, file_name: &str, query: Option<&str>) -> bool {
  5311	    let Some(query) = query else {
  5312	        return true;
  5313	    };
  5314	    relative_path.to_lowercase().contains(query) || file_name.to_lowercase().contains(query)
  5315	}
  5316	
  5317	fn main() {
  5318	    let agent_runtime = agent_conversation::manager::AgentRuntimeManager::new(
  5319	        agent_conversation::providers::ProviderRegistry::bundled_from_environment()
  5320	            .expect("packaged ACP adapter configuration is invalid"),
  5321	    );
  5322	    let workflow_engine = WorkflowEngine::managed(agent_runtime.clone());
  5323	    tauri::Builder::default()
  5324	        .manage(SourceScanRegistry::default())
  5325	        .manage(agent_runtime)
  5326	        .manage(workflow_engine)
  5327	        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
  5328	        .manage(lsp::SourceLspRegistry::default())
  5329	        .manage(terminal::TerminalRegistry::default())
  5330	        .manage(browser::BrowserRegistry::default())
  5331	        .manage(resources::ResourceRegistry::default())
  5332	        .manage(usage_history::UsageHistoryState::default())
  5333	        .plugin(tauri_plugin_dialog::init())
  5334	        .setup(|app| {
  5335	            // Every time a language server starts, finishes reading a project, or stops,
  5336	            // tell the editor straight away. Without this the editor would have to ask
  5337	            // over and over to notice, which is what it used to do.
  5338	            let app = app.handle().clone();
  5339	            lsp::set_source_lsp_status_listener(Arc::new(move |change| {
  5340	                let _ = app.emit(SOURCE_LSP_STATUS_CHANGED_EVENT, change);
  5341	            }));
  5342	            Ok(())
  5343	        })
  5344	        .invoke_handler(tauri::generate_handler![
  5345	            read_backend_capabilities,
  5346	            list_source_files,
  5347	            cancel_source_scan,
  5348	            validate_project_root,
  5349	            read_source_file,
  5350	            read_native_csharp_file,
  5351	            write_source_file,
  5352	            open_source_file,
  5353	            reveal_source_file,
  5354	            open_path,
  5355	            reveal_path,
  5356	            open_terminal_path,
  5357	            open_terminal_command,
  5358	            search_source_files,
  5359	            find_source_definitions,
  5360	            find_source_references,
  5361	            count_source_references,
  5362	            read_source_lsp_status,
  5363	            list_source_lsp_statuses,
  5364	            warm_source_lsp_for_root,
  5365	            ensure_native_csharp_language_client,
  5366	            mark_native_csharp_language_client_ready,
  5367	            set_csharp_language_server_enabled,
  5368	            find_source_lsp_definitions,
  5369	            find_source_lsp_completions,
  5370	            find_source_lsp_implementations,
  5371	            find_source_lsp_type_definitions,
  5372	            find_source_lsp_document_highlights,
  5373	            find_source_lsp_signature_help,
  5374	            find_source_lsp_inlay_hints,
  5375	            find_source_lsp_semantic_tokens,
  5376	            find_source_lsp_workspace_symbols,
  5377	            format_source_with_lsp,
  5378	            rename_source_with_lsp,
  5379	            find_source_lsp_code_actions,
  5380	            find_source_lsp_references,
  5381	            find_source_lsp_hover,
  5382	            find_source_lsp_symbols,
  5383	            find_source_lsp_document_symbols,
  5384	            read_source_lsp_log,
  5385	            read_source_lsp_diagnostics,
  5386	            list_source_lsp_diagnostics_for_root,
  5387	            project_git_status,
  5388	            read_source_git_diff,
  5389	            stage_git_paths,
  5390	            unstage_git_paths,
  5391	            commit_git_repository,
  5392	            fetch_git_repository,
  5393	            pull_git_repository,
  5394	            push_git_repository,
  5395	            read_git_commit_history,
  5396	            read_git_commit_files,
  5397	            read_git_commit_file_diff,
  5398	            git_pr::generate_commit_message,
  5399	            git_pr::read_pull_request_context,
  5400	            git_pr::generate_pull_request_details,
  5401	            git_pr::create_pull_request,
  5402	            git_pr::read_pull_request_status,
  5403	            list_project_worktrees,
  5404	            remove_project_worktree,
  5405	            archive_project_worktree,
  5406	            list_git_repository_summaries,
  5407	            list_agent_sessions,
  5408	            list_runtime_contexts,
  5409	            list_playwright_sessions,
  5410	            kill_playwright_session,
  5411	            kill_playwright_sessions,
  5412	            kill_process,
  5413	            resources::read_resource_snapshot,
  5414	            resources::read_resource_disk_scan,
  5415	            resources::cleanup_workspace_disk_entry,
  5416	            resources::stop_owned_resource,
  5417	            resources::restart_language_server_root,
  5418	            resources::set_active_source_root,
  5419	            resources::apply_resource_memory_pressure,
  5420	            resources::read_language_server_log,
  5421	            usage_current::read_current_provider_usage,
  5422	            usage_history::read_usage_summary,
  5423	            usage_history::read_usage_breakdown,
  5424	            usage_history::read_usage_provider_summary,
  5425	            usage_history::read_usage_daily,
  5426	            usage_history::read_usage_daily_totals,
  5427	            usage_history::refresh_usage_history,
  5428	            list_orchestration_runs,
  5429	            record_orchestration_event,
  5430	            list_workflow_runs,
  5431	            create_workflow_run,
  5432	            start_workflow_run,
  5433	            pause_workflow_run,
  5434	            resume_workflow_run,
  5435	            cancel_workflow_run,
  5436	            retry_workflow_node,
  5437	            skip_workflow_node,
  5438	            approve_workflow_gate,
  5439	            submit_workflow_result,
  5440	            agent_conversation::ensure_agent_conversation,
  5441	            agent_conversation::send_agent_conversation_message,
  5442	            agent_conversation::respond_agent_conversation_approval,
  5443	            agent_conversation::stop_agent_conversation_turn,
  5444	            agent_conversation::close_agent_conversation,
  5445	            agent_conversation::read_agent_conversation_snapshot,
  5446	            agent_conversation::read_agent_conversation_transcript,
  5447	            agent_conversation::start_agent_conversation_terminal_projection,
  5448	            agent_conversation::stop_agent_conversation_terminal_projection,
  5449	            agent_conversation::save_agent_conversation_attachment,
  5450	            agent_conversation::handoff::handoff_agent_conversation,
  5451	            start_terminal_session,
  5452	            list_terminal_sessions,
  5453	            read_terminal_session_scrollback,
  5454	            write_terminal_session,
  5455	            resize_terminal_session,
  5456	            close_terminal_session,
  5457	            browser::create_browser_tab,
  5458	            browser::set_browser_tab_bounds,
  5459	            browser::set_browser_tab_viewport,
  5460	            browser::show_browser_tab,

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts | sed -n '250,490p'
printf '\\n--- conversation reducer ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts | sed -n '1,360p'
printf '\\n--- timeline ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationTimeline.ts | sed -n '1,260p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   250	}
   251	
   252	export function startConversationTerminalProjection(input: {
   253	  ownedId: string;
   254	  provider: AgentConversationProvider;
   255	  nativeSessionId: string;
   256	}): void {
   257	  if (!isTauri()) return;
   258	  const signature = `${input.provider}:${input.nativeSessionId}`;
   259	  if (terminalProjections.get(input.ownedId) === signature) return;
   260	  terminalProjections.set(input.ownedId, signature);
   261	  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
   262	  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
   263	    request: { ...input, generation }
   264	  }).then((registration) => {
   265	    for (const event of registration.events) applyAgentConversationEvent(event);
   266	  }).catch(() => {
   267	    // The transcript may not exist until the agent accepts its first prompt.
   268	    // Dropping the signature lets the next activation register again.
   269	    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
   270	  });
   271	}
   272	
   273	export function stopConversationTerminalProjection(ownedId: string): void {
   274	  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
   275	  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
   276	}
   277	
   278	type HandoffInput = Omit<AgentConversationHandoffRequest, 'phase'>;
   279	
   280	function handoffTargetOwner(direction: AgentConversationHandoffDirection): AgentWriterLeaseOwner {
   281	  return direction === 'structured-to-terminal' ? 'terminal' : 'structured';
   282	}
   283	
   284	function applyHandoffReceiptToStore(receipt: AgentConversationHandoffReceipt): void {
   285	  const state = getConversationSession(receipt.ownedId);
   286	  if (!state || !handoffGenerationMatches(receipt, receipt.ownedId, state.generation)) return;
   287	  if (receipt.phase === 'prepare' || receipt.phase === 'prepared') {
   288	    const transition: AgentWriterLeaseTransition = {
   289	      ownedId: receipt.ownedId,
   290	      generation: receipt.generation,
   291	      from: receipt.previousOwner,
   292	      to: handoffTargetOwner(receipt.direction),
   293	      state: 'requested'
   294	    };
   295	    setConversationWriterLeaseTransition(transition);
   296	    return;
   297	  }
   298	  const next = applyConversationHandoffReceipt(state, receipt);
   299	  Object.assign(state, next);
   300	}
   301	
   302	async function invokeHandoff(
   303	  input: HandoffInput,
   304	  phase: AgentConversationHandoffPhase
   305	): Promise<AgentConversationHandoffReceipt> {
   306	  if (!isTauri()) throw new Error('Native handoff is available only in the desktop application');
   307	  const request = createConversationHandoffRequest({ ...input, phase });
   308	  const current = getConversationSession(request.ownedId);
   309	  if (!current || current.generation !== request.generation) {
   310	    throw new Error('Handoff request belongs to a stale conversation generation');
   311	  }
   312	  try {
   313	    const receipt = await invoke<AgentConversationHandoffReceipt>('handoff_agent_conversation', { request });
   314	    if (!handoffGenerationMatches(receipt, request.ownedId, request.generation)) {
   315	      throw new Error('Handoff receipt belongs to a stale conversation generation');
   316	    }
   317	    applyHandoffReceiptToStore(receipt);
   318	    return receipt;
   319	  } catch (error) {
   320	    clearConversationWriterLeaseTransition(request.ownedId, request.generation);
   321	    throw error;
   322	  }
   323	}
   324	
   325	export function prepareConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   326	  return invokeHandoff(input, 'prepare');
   327	}
   328	
   329	export function commitConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   330	  return invokeHandoff(input, 'commit');
   331	}
   332	
   333	export function rollbackConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   334	  return invokeHandoff(input, 'rollback');
   335	}
   336	
   337	async function resyncConversation(ownedId: string): Promise<void> {
   338	  const existing = resyncing.get(ownedId);
   339	  if (existing) return existing;
   340	  const work = (async () => {
   341	    for (let attempt = 0; attempt < 3; attempt += 1) {
   342	      const snapshot = await invoke<AgentConversationSnapshot | null>(
   343	        'read_agent_conversation_snapshot',
   344	        { ownedId }
   345	      );
   346	      if (!snapshot) return;
   347	      const sequenceBeforeApply = getConversationSession(ownedId)?.lastSequence ?? 0;
   348	      applyAgentConversationSnapshot(snapshot);
   349	      if (sequenceBeforeApply <= snapshot.lastSequence) return;
   350	    }
   351	  })().finally(() => resyncing.delete(ownedId));
   352	  resyncing.set(ownedId, work);
   353	  return work;
   354	}
   355	
   356	export async function startConversationEvents(): Promise<void> {
   357	  if (!isTauri() || unlisten) return;
   358	  unlisten = await listen<AgentConversationEvent | AgentEvent>('agent-conversation-event', ({ payload }) => {
   359	    applyAgentConversationEvent(payload);
   360	    if (getConversationSession(payload.ownedId)?.desynchronized) {
   361	      void resyncConversation(payload.ownedId);
   362	    }
   363	    if (('kind' in payload.payload && (payload.payload.kind === 'turn' || payload.payload.kind === 'error'))
   364	      || ('type' in payload && ['turn.completed', 'turn.interrupted', 'runtime.error'].includes(payload.type))) {
   365	      setConversationSending(payload.ownedId, false);
   366	    }
   367	  });
   368	}
   369	
   370	export function stopConversationEvents(): void {
   371	  unlisten?.();
   372	  unlisten = null;
   373	  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
   374	}
   375	
   376	export async function closeStructuredConversation(ownedId: string): Promise<void> {
   377	  if (!isTauri()) return;
   378	  await invoke<boolean>('close_agent_conversation', { ownedId });
   379	}
   380	
   381	export async function ensureStructuredConversation(input: {
   382	  ownedId: string;
   383	  provider: AgentConversationProvider;
   384	  cwd: string;
   385	  nativeSessionId?: string | null;
   386	}): Promise<AgentConversationConnection | null> {
   387	  ensureConversationSession(input.ownedId, input.provider);
   388	  if (!isTauri() || !input.cwd.trim()) return null;
   389	  const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
   390	    request: input
   391	  });
   392	  setConversationConnection(connection);
   393	  await resyncConversation(input.ownedId);
   394	  return connection;
   395	}
   396	
   397	export async function sendStructuredMessage(
   398	  ownedId: string,
   399	  text: string,
   400	  ptySessionId?: string | null
   401	): Promise<void> {
   402	  const state = getConversationSession(ownedId);
   403	  if (!state) return;
   404	  if (!text.trim() && state.attachments.length === 0) return;
   405	  setConversationSending(ownedId, true);
   406	  try {
   407	    if (ptySessionId) {
   408	      if (state.writerLease.ownedId !== ownedId
   409	        || state.writerLease.generation !== state.generation
   410	        || state.writerLease.owner !== 'terminal') {
   411	        throw new Error('The terminal writer lease is not confirmed for this session');
   412	      }
   413	      const attachmentText = state.attachments.length
   414	        ? `\n\nAttached screenshots:\n${state.attachments.map((item) => `- ${item.path}`).join('\n')}`
   415	        : '';
   416	      const outgoingText = `${text}${attachmentText}`;
   417	      // Match a real terminal paste followed by a separate Enter key. Sending
   418	      // both in one PTY write makes Claude's TUI treat Enter as part of its
   419	      // multiline paste buffer, leaving the prompt staged but never submitted.
   420	      const pasted = await writeTerminalSessionFromTauri(
   421	        ptySessionId,
   422	        `\u001b[200~${outgoingText}\u001b[201~`
   423	      );
   424	      if (!pasted) throw new Error('The terminal session is no longer running');
   425	      const submitted = await writeTerminalSessionFromTauri(ptySessionId, '\r');
   426	      if (!submitted) throw new Error('The terminal session is no longer running');
   427	      setConversationSending(ownedId, false);
   428	      state.attachments.forEach(cleanupConversationAttachmentPreview);
   429	      setConversationAttachments(ownedId, []);
   430	      return;
   431	    }
   432	    if (state.generation < 1) throw new Error('The structured conversation is not connected');
   433	    const hydratedAttachments = state.capabilities?.prompt.image === true
   434	      ? await Promise.all(state.attachments.map((attachment) => hydrateAttachmentBytes(attachment as AttachmentWithBytes)))
   435	      : state.attachments as AttachmentWithBytes[];
   436	    const prompt = buildConversationPrompt(
   437	      text,
   438	      hydratedAttachments,
   439	      state.capabilities?.prompt.image === true
   440	    );
   441	    await invoke('send_agent_conversation_message', {
   442	      request: { ownedId, generation: state.generation, text: prompt.text, content: prompt.content }
   443	    });
   444	    state.attachments.forEach(cleanupConversationAttachmentPreview);
   445	    setConversationAttachments(ownedId, []);
   446	  } catch (error) {
   447	    setConversationSending(ownedId, false);
   448	    throw error;
   449	  }
   450	}
   451	
   452	export async function respondToStructuredApproval(
   453	  ownedId: string,
   454	  requestId: string,
   455	  decision: 'accept' | 'decline' | 'cancel'
   456	): Promise<void> {
   457	  const state = getConversationSession(ownedId);
   458	  if (!state) return;
   459	  await invoke('respond_agent_conversation_approval', {
   460	    request: { ownedId, generation: state.generation, requestId, decision }
   461	  });
   462	}
   463	
   464	export async function respondToStructuredInput(
   465	  ownedId: string,
   466	  response: Omit<AgentUserInputResponse, 'ownedId' | 'generation'>
   467	): Promise<void> {
   468	  const state = getConversationSession(ownedId);
   469	  if (!state) return;
   470	  await invoke('respond_agent_conversation_input', {
   471	    request: { ...response, ownedId, generation: state.generation }
   472	  });
   473	}

--- conversation reducer ---
     1	import type {
     2	  AgentConversationEvent,
     3	  AgentConversationProvider,
     4	  ConversationSessionState,
     5	  ConversationTimelineEntry
     6	} from './conversationTypes.ts';
     7	
     8	export function createConversationState(
     9	  ownedId: string,
    10	  provider: AgentConversationProvider
    11	): ConversationSessionState {
    12	  return {
    13	    ownedId,
    14	    provider,
    15	    generation: 0,
    16	    lastSequence: 0,
    17	    desynchronized: false,
    18	    connectionState: 'disconnected',
    19	    timeline: []
    20	  };
    21	}
    22	
    23	function replaceOrAppend(
    24	  timeline: ConversationTimelineEntry[],
    25	  itemId: string,
    26	  create: (current: ConversationTimelineEntry | undefined) => ConversationTimelineEntry
    27	): ConversationTimelineEntry[] {
    28	  const index = timeline.findIndex((entry) => entry.itemId === itemId);
    29	  if (index < 0) return [...timeline, create(undefined)];
    30	  const next = timeline.slice();
    31	  next[index] = create(timeline[index]);
    32	  return next;
    33	}
    34	
    35	export function importConversationHistory(
    36	  state: ConversationSessionState,
    37	  history: readonly ConversationTimelineEntry[]
    38	): ConversationSessionState {
    39	  if (history.length === 0) return state;
    40	
    41	  const existingIds = new Set(state.timeline.map((entry) => entry.itemId));
    42	  const additions = history.filter((entry) => !existingIds.has(entry.itemId));
    43	  if (additions.length === 0) return state;
    44	  return { ...state, timeline: [...state.timeline, ...additions] };
    45	}
    46	
    47	export function applyConversationEvent(
    48	  state: ConversationSessionState,
    49	  event: AgentConversationEvent
    50	): ConversationSessionState {
    51	  if (event.ownedId !== state.ownedId || event.provider !== state.provider) return state;
    52	  if (event.generation < state.generation) return state;
    53	
    54	  const isNewGeneration = event.generation > state.generation;
    55	  const priorSequence = isNewGeneration ? 0 : state.lastSequence;
    56	  if (!isNewGeneration && event.sequence <= priorSequence) return state;
    57	
    58	  const hasGap = event.sequence !== priorSequence + 1;
    59	  let next: ConversationSessionState = {
    60	    ...state,
    61	    generation: event.generation,
    62	    lastSequence: event.sequence,
    63	    desynchronized: isNewGeneration ? hasGap : state.desynchronized || hasGap
    64	  };
    65	
    66	  // Applying a delta after a missing event would fabricate a transcript. Keep
    67	  // the acknowledged position so duplicates remain rejected, preserve every
    68	  // valid item already rendered, and let the service request a fresh snapshot.
    69	  if (hasGap) return next;
    70	
    71	  const { payload } = event;
    72	  switch (payload.kind) {
    73	    case 'connection':
    74	      return {
    75	        ...next,
    76	        connectionState: payload.state,
    77	        nativeSessionId: payload.nativeSessionId ?? next.nativeSessionId
    78	      };
    79	
    80	    case 'userMessage':
    81	      return {
    82	        ...next,
    83	        timeline: replaceOrAppend(next.timeline, payload.itemId, () => ({
    84	          kind: 'user',
    85	          itemId: payload.itemId,
    86	          text: payload.text,
    87	          completed: true,
    88	          timestampMs: event.timestampMs
    89	        }))
    90	      };
    91	
    92	    case 'assistantDelta':
    93	      return {
    94	        ...next,
    95	        timeline: replaceOrAppend(next.timeline, payload.itemId, (current) => ({
    96	          kind: 'assistant',
    97	          itemId: payload.itemId,
    98	          text: current?.kind === 'assistant' ? `${current.text}${payload.delta}` : payload.delta,
    99	          completed: false,
   100	          timestampMs: current?.timestampMs ?? event.timestampMs
   101	        }))
   102	      };
   103	
   104	    case 'assistantMessage':
   105	      return {
   106	        ...next,
   107	        timeline: replaceOrAppend(next.timeline, payload.itemId, (current) => ({
   108	          kind: 'assistant',
   109	          itemId: payload.itemId,
   110	          text: payload.text,
   111	          completed: true,
   112	          timestampMs: current?.timestampMs ?? event.timestampMs
   113	        }))
   114	      };
   115	
   116	    case 'tool':
   117	      return {
   118	        ...next,
   119	        timeline: replaceOrAppend(next.timeline, payload.itemId, () => ({
   120	          kind: 'tool',
   121	          itemId: payload.itemId,
   122	          name: payload.name,
   123	          state: payload.state,
   124	          summary: payload.summary,
   125	          timestampMs: event.timestampMs
   126	        }))
   127	      };
   128	
   129	    case 'approval': {
   130	      const itemId = `approval:${payload.requestId}`;
   131	      return {
   132	        ...next,
   133	        timeline: replaceOrAppend(next.timeline, itemId, (current) => ({
   134	          kind: 'approval',
   135	          itemId,
   136	          requestId: payload.requestId,
   137	          state: payload.state,
   138	          summary: payload.summary,
   139	          timestampMs: current?.timestampMs ?? event.timestampMs
   140	        }))
   141	      };
   142	    }
   143	
   144	    case 'turn': {
   145	      const itemId = `turn:${payload.turnId}`;
   146	      return {
   147	        ...next,
   148	        activeTurnId: payload.state === 'started' ? payload.turnId : undefined,
   149	        timeline: replaceOrAppend(next.timeline, itemId, (current) => ({
   150	          kind: 'turn',
   151	          itemId,
   152	          turnId: payload.turnId,
   153	          state: payload.state,
   154	          timestampMs: current?.timestampMs ?? event.timestampMs
   155	        }))
   156	      };
   157	    }
   158	
   159	    case 'usage':
   160	      return {
   161	        ...next,
   162	        usage: { inputTokens: payload.inputTokens, outputTokens: payload.outputTokens }
   163	      };
   164	
   165	    case 'error':
   166	      return {
   167	        ...next,
   168	        connectionState: payload.recoverable ? next.connectionState : 'failed',
   169	        timeline: [
   170	          ...next.timeline,
   171	          {
   172	            kind: 'error',
   173	            itemId: `error:${event.generation}:${event.sequence}`,
   174	            code: payload.code,
   175	            message: payload.message,
   176	            recoverable: payload.recoverable,
   177	            timestampMs: event.timestampMs
   178	          }
   179	        ]
   180	      };
   181	  }
   182	}

--- timeline ---
     1	import type {
     2	  AgentConfigValue,
     3	  AgentContent,
     4	  AgentEvent,
     5	  AgentItem,
     6	  AgentItemType,
     7	  AgentApprovalRequest,
     8	  AgentConversationEvent,
     9	  AgentUserInputRequest,
    10	  AgentUserInputField,
    11	  ConversationTimelineEntry
    12	} from './conversationTypes.ts';
    13	
    14	export interface AgentPlanStep {
    15	  id: string;
    16	  title: string;
    17	  detail: string | null;
    18	  state: 'pending' | 'in-progress' | 'completed' | 'failed' | 'blocked';
    19	  ownerAgentId: string | null;
    20	  startedAt: string | null;
    21	  completedAt: string | null;
    22	}
    23	
    24	export interface ConversationTask {
    25	  id: string;
    26	  title: string;
    27	  state: AgentPlanStep['state'];
    28	  detail?: string | null;
    29	  ownerAgentId?: string | null;
    30	}
    31	
    32	export type ConversationTextDisplayItem = {
    33	  kind: 'user' | 'assistant' | 'reasoning' | 'command' | 'file' | 'error';
    34	  itemId: string;
    35	  text: string;
    36	  timestampMs: number;
    37	  completed?: boolean;
    38	  label?: string;
    39	  metadata?: Record<string, AgentConfigValue>;
    40	};
    41	
    42	export type ConversationDisplayItem =
    43	  | (ConversationTextDisplayItem & { kind: 'user' })
    44	  | (ConversationTextDisplayItem & { kind: 'assistant' })
    45	  | (ConversationTextDisplayItem & { kind: 'reasoning' })
    46	  | (ConversationTextDisplayItem & { kind: 'command' })
    47	  | (ConversationTextDisplayItem & { kind: 'file' })
    48	  | (ConversationTextDisplayItem & { kind: 'error' })
    49	  | { kind: 'plan'; itemId: string; title: string; steps: AgentPlanStep[]; timestampMs: number; text?: string }
    50	  | { kind: 'tasks'; itemId: string; title: string; tasks: ConversationTask[]; timestampMs: number; text?: string }
    51	  | { kind: 'tool'; itemId: string; name: string; state: string; summary?: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> }
    52	  | { kind: 'subagent'; itemId: string; childId: string; parentId: string; label: string; state: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> }
    53	  | { kind: 'approval'; itemId: string; requestId: string; title: string; summary: string; state: string; options: string[]; timestampMs: number }
    54	  | { kind: 'input'; itemId: string; requestId: string; title: string; description?: string; fields: AgentUserInputField[]; timestampMs: number }
    55	  | { kind: 'unknown'; itemId: string; text: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> };
    56	
    57	const textOf = (content: readonly AgentContent[] | undefined): string =>
    58	  (content ?? []).map((entry) => entry.text).join('');
    59	
    60	function recordOf(value: unknown): Record<string, AgentConfigValue> | undefined {
    61	  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
    62	  return value as Record<string, AgentConfigValue>;
    63	}
    64	
    65	function stringOf(value: unknown, fallback = ''): string {
    66	  return typeof value === 'string' ? value : fallback;
    67	}
    68	
    69	function stateOf(value: unknown, fallback = 'pending'): string {
    70	  return typeof value === 'string' && value ? value : fallback;
    71	}
    72	
    73	function itemText(item: AgentItem): string {
    74	  return textOf(item.content);
    75	}
    76	
    77	function stepsOf(value: unknown): AgentPlanStep[] {
    78	  if (!Array.isArray(value)) return [];
    79	  return value.flatMap((entry, index) => {
    80	    const row = recordOf(entry);
    81	    if (!row) return [];
    82	    return [{
    83	      id: stringOf(row.id, `step-${index + 1}`),
    84	      title: stringOf(row.title, stringOf(row.name, `Step ${index + 1}`)),
    85	      detail: typeof row.detail === 'string' ? row.detail : null,
    86	      state: ['pending', 'in-progress', 'completed', 'failed', 'blocked'].includes(stringOf(row.state))
    87	        ? stringOf(row.state) as AgentPlanStep['state']
    88	        : 'pending',
    89	      ownerAgentId: typeof row.ownerAgentId === 'string' ? row.ownerAgentId : null,
    90	      startedAt: typeof row.startedAt === 'string' ? row.startedAt : null,
    91	      completedAt: typeof row.completedAt === 'string' ? row.completedAt : null
    92	    }];
    93	  });
    94	}
    95	
    96	function tasksOf(value: unknown): ConversationTask[] {
    97	  return stepsOf(value).map((step) => ({
    98	    id: step.id,
    99	    title: step.title,
   100	    state: step.state,
   101	    detail: step.detail,
   102	    ownerAgentId: step.ownerAgentId
   103	  }));
   104	}
   105	
   106	function kindFor(item: AgentItem): ConversationDisplayItem['kind'] {
   107	  switch (item.type) {
   108	    case 'user-message': return 'user';
   109	    case 'assistant-message': return 'assistant';
   110	    case 'reasoning': return 'reasoning';
   111	    case 'plan': return 'plan';
   112	    case 'task-list': return 'tasks';
   113	    case 'command': return 'command';
   114	    case 'file-change': return 'file';
   115	    case 'mcp-tool':
   116	    case 'web-search':
   117	    case 'image-view':
   118	    case 'image-generation': return 'tool';
   119	    case 'subagent': return 'subagent';
   120	    case 'error': return 'error';
   121	    default: return 'unknown';
   122	  }
   123	}
   124	
   125	export function displayItemFromAgentItem(item: AgentItem, timestampMs = Date.now()): ConversationDisplayItem {
   126	  const metadata = item.providerMetadata;
   127	  const kind = kindFor(item);
   128	  if (kind === 'plan') return {
   129	    kind, itemId: item.id, title: stringOf(metadata?.title, 'Plan'), steps: stepsOf(metadata?.steps), text: itemText(item), timestampMs
   130	  };
   131	  if (kind === 'tasks') return {
   132	    kind, itemId: item.id, title: stringOf(metadata?.title, 'Tasks'), tasks: tasksOf(metadata?.tasks ?? metadata?.steps), text: itemText(item), timestampMs
   133	  };
   134	  if (kind === 'tool') return {
   135	    kind, itemId: item.id, name: stringOf(metadata?.name, item.type), state: stateOf(metadata?.state, 'started'), summary: stringOf(metadata?.summary, itemText(item)) || undefined, metadata, timestampMs
   136	  };
   137	  if (kind === 'subagent') return {
   138	    kind, itemId: item.id, childId: stringOf(metadata?.childId, item.id), parentId: stringOf(metadata?.parentId, item.turnId ?? ''), label: stringOf(metadata?.label, 'Sub-agent'), state: stateOf(metadata?.state, 'working'), metadata, timestampMs
   139	  };
   140	  if (kind === 'unknown') return { kind, itemId: item.id, text: itemText(item), metadata, timestampMs };
   141	  return { kind: kind as ConversationTextDisplayItem['kind'], itemId: item.id, text: itemText(item), completed: true, timestampMs, metadata } as ConversationDisplayItem;
   142	}
   143	
   144	export function displayItemFromApproval(
   145	  request: AgentApprovalRequest & { state: string },
   146	  timestampMs = Date.now()
   147	): ConversationDisplayItem {
   148	  return {
   149	    kind: 'approval',
   150	    itemId: `approval:${request.requestId}`,
   151	    requestId: request.requestId,
   152	    title: request.title,
   153	    summary: request.description ?? request.title,
   154	    state: request.state,
   155	    options: request.options,
   156	    timestampMs
   157	  };
   158	}
   159	
   160	export function displayItemFromInput(
   161	  request: AgentUserInputRequest,
   162	  timestampMs = Date.now()
   163	): ConversationDisplayItem {
   164	  return {
   165	    kind: 'input',
   166	    itemId: `input:${request.requestId}`,
   167	    requestId: request.requestId,
   168	    title: request.title,
   169	    description: request.description,
   170	    fields: request.fields,
   171	    timestampMs
   172	  };
   173	}
   174	
   175	function displayItemFromLegacy(entry: ConversationTimelineEntry): ConversationDisplayItem {
   176	  if (entry.kind === 'user' || entry.kind === 'assistant') return {
   177	    kind: entry.kind, itemId: entry.itemId, text: entry.text, completed: entry.completed, timestampMs: entry.timestampMs
   178	  };
   179	  if (entry.kind === 'tool') return {
   180	    kind: 'tool', itemId: entry.itemId, name: entry.name, state: entry.state, summary: entry.summary, timestampMs: entry.timestampMs
   181	  };
   182	  if (entry.kind === 'approval') return {
   183	    kind: 'approval', itemId: entry.itemId, requestId: entry.requestId, title: 'Approval requested', summary: entry.summary,
   184	    state: entry.state, options: ['accept', 'decline'], timestampMs: entry.timestampMs
   185	  };
   186	  if (entry.kind === 'error') return {
   187	    kind: 'error', itemId: entry.itemId, text: entry.message, timestampMs: entry.timestampMs, metadata: { code: entry.code, recoverable: entry.recoverable }
   188	  };
   189	  if (entry.kind === 'turn') return {
   190	    kind: 'unknown', itemId: entry.itemId, text: `${entry.turnId} · ${entry.state}`, timestampMs: entry.timestampMs
   191	  };
   192	  return { kind: 'unknown', itemId: entry.itemId, text: '', timestampMs: 0 };
   193	}
   194	
   195	/** Merge typed provider items with legacy reducer entries without flattening them. */
   196	export function typedConversationTimeline(
   197	  items: readonly AgentItem[] = [],
   198	  legacy: readonly ConversationTimelineEntry[] = [],
   199	  timestamps: Readonly<Record<string, number>> = {}
   200	): ConversationDisplayItem[] {
   201	  const byId = new Map<string, ConversationDisplayItem>();
   202	  legacy.forEach((entry) => byId.set(entry.itemId, displayItemFromLegacy(entry)));
   203	  const legacyEnd = legacy.reduce((latest, entry) => Math.max(latest, entry.timestampMs), 0);
   204	  items.forEach((item, index) => byId.set(item.id, displayItemFromAgentItem(item, timestamps[item.id] ?? legacyEnd + index + 1)));
   205	  return [...byId.values()].sort((left, right) => left.timestampMs - right.timestampMs);
   206	}
   207	
   208	/** Stable, bounded row selection for a long history. */
   209	export function visibleConversationRange(
   210	  count: number,
   211	  scrollTop: number,
   212	  viewportHeight: number,
   213	  rowEstimate = 96,
   214	  overscan = 6
   215	): { start: number; end: number; offsetTop: number } {
   216	  if (count <= 0) return { start: 0, end: 0, offsetTop: 0 };
   217	  const safeRow = Math.max(48, rowEstimate);
   218	  const start = Math.max(0, Math.floor(Math.max(0, scrollTop) / safeRow) - overscan);
   219	  const visible = Math.ceil(Math.max(1, viewportHeight) / safeRow) + overscan * 2;
   220	  return { start, end: Math.min(count, start + visible), offsetTop: start * safeRow };
   221	}
   222	
   223	export function conversationScrollShouldFollow(
   224	  scrollTop: number,
   225	  clientHeight: number,
   226	  scrollHeight: number,
   227	  threshold = 80
   228	): boolean {
   229	  return scrollHeight - (scrollTop + clientHeight) <= threshold;
   230	}
   231	
   232	export function agentItemFromEvent(event: AgentEvent | AgentConversationEvent): AgentItem | null {
   233	  const payload = event.payload as Record<string, unknown>;
   234	  const raw = recordOf(payload.item);
   235	  if (raw && typeof raw.id === 'string' && typeof raw.type === 'string') {
   236	    return {
   237	      id: raw.id,
   238	      type: raw.type as AgentItemType,
   239	      turnId: typeof raw.turnId === 'string' ? raw.turnId : ('turnId' in event ? event.turnId : undefined),
   240	      content: Array.isArray(raw.content)
   241	        ? raw.content.flatMap((content) => {
   242	          const row = recordOf(content);
   243	          return row && typeof row.channel === 'string' && typeof row.text === 'string'
   244	            ? [{ channel: row.channel as AgentContent['channel'], text: row.text, mimeType: typeof row.mimeType === 'string' ? row.mimeType : undefined }]
   245	            : [];
   246	        })
   247	        : [],
   248	      providerMetadata: recordOf(raw.providerMetadata)
   249	    };
   250	  }
   251	  const id = ('itemId' in event ? event.itemId : undefined) ?? stringOf(payload.itemId);
   252	  if (!id) return null;
   253	  const text = stringOf(payload.text, stringOf(payload.delta));
   254	  const eventType = 'type' in event ? event.type : '';
   255	  const channel = stringOf(payload.channel, eventType === 'content.delta' ? 'assistant' : 'assistant') as AgentContent['channel'];
   256	  let type: AgentItemType = 'unknown';
   257	  if (payload.kind === 'userMessage') type = 'user-message';
   258	  else if (payload.kind === 'assistantMessage' || payload.kind === 'assistantDelta') type = 'assistant-message';
   259	  else if (payload.kind === 'tool') type = 'mcp-tool';
   260	  else if (payload.kind === 'error') type = 'error';

exec
/bin/zsh -lc "rg -n \"emit_payload|frontend_events|emit\\(|agent-conversation-event|AgentConversationPayload|AgentEvent\" tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs tauri-svelte-preview/src-tauri/src/agent_conversation/providers tauri-svelte-preview/src-tauri/src | head -260" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:11:use super::journal::AgentEventJournal;
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:14:    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:15:    AgentConversationSnapshot, AgentEvent, AgentEventType, AgentExecutionOwner,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:43:    pub recent_events: VecDeque<AgentEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:48:    frontend_events: VecDeque<AgentConversationEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:49:    journal: AgentEventJournal,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:183:                frontend_events: VecDeque::new(),
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:184:                journal: AgentEventJournal::new(JOURNAL_EVENT_CAP),
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265:    pub fn emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:269:        payload: AgentConversationPayload,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:278:        if let AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:301:        session.frontend_events.push_back(frontend_event.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:305:        while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:306:            session.frontend_events.pop_front();
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:508:                events: session.frontend_events.iter().cloned().collect(),
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:799:    pub fn canonical_snapshot(&self, owned_id: &str) -> Result<Vec<AgentEvent>, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:884:    payload: &AgentConversationPayload,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:885:) -> Result<AgentEvent, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:887:        AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:890:        } => AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:891:        AgentConversationPayload::Connection { .. } => AgentEventType::SessionStateChanged,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:892:        AgentConversationPayload::UserMessage { .. } => AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:893:        AgentConversationPayload::AssistantDelta { .. } => AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:894:        AgentConversationPayload::AssistantMessage { .. } => AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:895:        AgentConversationPayload::Tool { .. } => AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:896:        AgentConversationPayload::Approval { .. } => AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:897:        AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:900:        } => AgentEventType::TurnStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:901:        AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:904:        } => AgentEventType::TurnInterrupted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:905:        AgentConversationPayload::Turn { .. } => AgentEventType::TurnCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:906:        AgentConversationPayload::Usage { .. } => AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:907:        AgentConversationPayload::Error { .. } => AgentEventType::RuntimeError,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:919:    Ok(AgentEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052:            .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1055:                AgentConversationPayload::Error {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1078:                .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1081:                    AgentConversationPayload::Error {
tauri-svelte-preview/src-tauri/src/browser.rs:339:        let _ = self.app.emit(BROWSER_TAB_NAVIGATION_EVENT, event);
tauri-svelte-preview/src-tauri/src/browser.rs:343:        let _ = self.app.emit(BROWSER_TAB_LOAD_EVENT, event);
tauri-svelte-preview/src-tauri/src/browser.rs:347:        let _ = self.app.emit(BROWSER_ELEMENT_SELECTED_EVENT, event);
tauri-svelte-preview/src-tauri/src/browser.rs:351:        let _ = self.app.emit(BROWSER_TAB_CLOSED_EVENT, event);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:81:    use crate::agent_conversation::protocol::{AgentEventType, AgentItemType};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:107:            AgentEventType::SessionStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:108:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:109:            AgentEventType::TurnStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:110:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:111:            AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:112:            AgentEventType::PlanUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:113:            AgentEventType::TasksUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:114:            AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:115:            AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:116:            AgentEventType::UserInputRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:117:            AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:118:            AgentEventType::RateLimitsUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:119:            AgentEventType::TurnCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:120:            AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:125:            event.event_type == AgentEventType::ApprovalRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:129:            event.event_type == AgentEventType::UserInputRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:133:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:137:            event.event_type == AgentEventType::TurnCompleted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:141:            event.event_type == AgentEventType::SessionClosed
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:195:            event.event_type == AgentEventType::ChildrenUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:209:            .find(|event| event.event_type == AgentEventType::RuntimeWarning)
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:239:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:243:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:247:            event.event_type == AgentEventType::TurnInterrupted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:252:            .any(|event| event.event_type == AgentEventType::SessionClosed));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:263:            .any(|event| { event.event_type == AgentEventType::RuntimeWarning }));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:16:    AgentContent, AgentContentChannel, AgentConversationProvider, AgentEvent, AgentEventType,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:58:    pub events: Vec<AgentEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:239:                    AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:319:                        AgentEventType::SessionStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:336:                    AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:377:                    AgentEventType::ApprovalResolved,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:385:                    AgentEventType::UserInputResolved,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:432:                    AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:456:                    AgentEventType::UserInputRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:540:                    AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:558:                    AgentEventType::SessionStateChanged,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:573:                    AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:588:                    AgentEventType::RateLimitsUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:627:                AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:673:                AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:682:            AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:684:            AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:711:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:718:            AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:730:                AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:760:                AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:783:                    AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:790:                    AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:810:                    AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:830:                AgentEventType::ItemCompleted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:832:                AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:841:                AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:868:            AgentEventType::PlanUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:877:            AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:904:            AgentEventType::TasksUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:913:            AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:933:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:959:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1025:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1032:            AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1069:            AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1079:        event_type: AgentEventType,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1113:            "started" => AgentEventType::TurnStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1114:            "interrupted" => AgentEventType::TurnInterrupted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1115:            _ => AgentEventType::TurnCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1133:                AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1215:        event_type: AgentEventType,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1232:        if event_type == AgentEventType::ItemCompleted {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1241:        event_type: AgentEventType,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1270:        batch.events.push(AgentEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1294:            AgentEventType::RuntimeWarning,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1311:            AgentEventType::RuntimeWarning,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1325:            AgentEventType::RuntimeError,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:81:    use crate::agent_conversation::protocol::{AgentEventType, AgentItemType};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:108:            AgentEventType::SessionStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:109:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:110:            AgentEventType::TurnStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:111:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:112:            AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:113:            AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:114:            AgentEventType::UserInputRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:115:            AgentEventType::PlanUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:116:            AgentEventType::TasksUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:117:            AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:118:            AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:119:            AgentEventType::RateLimitsUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:120:            AgentEventType::TurnCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:121:            AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:126:            event.event_type == AgentEventType::ApprovalRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:130:            event.event_type == AgentEventType::UserInputRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:134:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:138:            event.event_type == AgentEventType::TurnCompleted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:142:            event.event_type == AgentEventType::SessionClosed
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:190:            event.event_type == AgentEventType::ChildrenUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:209:            .find(|event| event.event_type == AgentEventType::RuntimeWarning)
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:237:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:241:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:245:            event.event_type == AgentEventType::TurnInterrupted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:250:            .any(|event| event.event_type == AgentEventType::SessionClosed));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:261:            .any(|event| { event.event_type == AgentEventType::RuntimeWarning }));
tauri-svelte-preview/src-tauri/src/main.rs:638:        let _ = app.emit(
tauri-svelte-preview/src-tauri/src/main.rs:5340:                let _ = app.emit(SOURCE_LSP_STATUS_CHANGED_EVENT, change);
tauri-svelte-preview/src-tauri/src/terminal.rs:399:                    let _ = app.emit(
tauri-svelte-preview/src-tauri/src/terminal.rs:454:        let _ = app.emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:10:use super::protocol::{AgentConversationProvider, AgentEvent, AgentRawFrameReference};
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:13:pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:34:    pub events: Vec<AgentEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:236:            remember_and_emit(&app, &events, event);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:267:                    remember_and_emit(&app, &thread_events, event);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:325:) -> AgentEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:326:    AgentEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:359:    events: &Arc<Mutex<VecDeque<AgentEvent>>>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:360:    event: AgentEvent,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:368:    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:562:            record.event_type == super::super::protocol::AgentEventType::ItemCompleted
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:611:            super::super::protocol::AgentEventType::TurnStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:612:                | super::super::protocol::AgentEventType::ApprovalRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:613:                | super::super::protocol::AgentEventType::ItemStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:614:                | super::super::protocol::AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:11:use super::journal::AgentEventJournal;
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:14:    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:15:    AgentConversationSnapshot, AgentEvent, AgentEventType, AgentExecutionOwner,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:43:    pub recent_events: VecDeque<AgentEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:48:    frontend_events: VecDeque<AgentConversationEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:49:    journal: AgentEventJournal,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:183:                frontend_events: VecDeque::new(),
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:184:                journal: AgentEventJournal::new(JOURNAL_EVENT_CAP),
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265:    pub fn emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:269:        payload: AgentConversationPayload,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:278:        if let AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:301:        session.frontend_events.push_back(frontend_event.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:305:        while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:306:            session.frontend_events.pop_front();
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:508:                events: session.frontend_events.iter().cloned().collect(),
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:799:    pub fn canonical_snapshot(&self, owned_id: &str) -> Result<Vec<AgentEvent>, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:884:    payload: &AgentConversationPayload,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:885:) -> Result<AgentEvent, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:887:        AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:890:        } => AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:891:        AgentConversationPayload::Connection { .. } => AgentEventType::SessionStateChanged,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:892:        AgentConversationPayload::UserMessage { .. } => AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:893:        AgentConversationPayload::AssistantDelta { .. } => AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:894:        AgentConversationPayload::AssistantMessage { .. } => AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:895:        AgentConversationPayload::Tool { .. } => AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:896:        AgentConversationPayload::Approval { .. } => AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:897:        AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:900:        } => AgentEventType::TurnStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:901:        AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:904:        } => AgentEventType::TurnInterrupted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:905:        AgentConversationPayload::Turn { .. } => AgentEventType::TurnCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:906:        AgentConversationPayload::Usage { .. } => AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:907:        AgentConversationPayload::Error { .. } => AgentEventType::RuntimeError,
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:919:    Ok(AgentEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052:            .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1055:                AgentConversationPayload::Error {
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1078:                .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1081:                    AgentConversationPayload::Error {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:81:    use crate::agent_conversation::protocol::{AgentEventType, AgentItemType};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:107:            AgentEventType::SessionStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:108:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:109:            AgentEventType::TurnStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:110:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:111:            AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:112:            AgentEventType::PlanUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:113:            AgentEventType::TasksUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:114:            AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:115:            AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:116:            AgentEventType::UserInputRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:117:            AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:118:            AgentEventType::RateLimitsUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:119:            AgentEventType::TurnCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:120:            AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:125:            event.event_type == AgentEventType::ApprovalRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:129:            event.event_type == AgentEventType::UserInputRequested
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:133:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:137:            event.event_type == AgentEventType::TurnCompleted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:141:            event.event_type == AgentEventType::SessionClosed
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:195:            event.event_type == AgentEventType::ChildrenUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:209:            .find(|event| event.event_type == AgentEventType::RuntimeWarning)
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:239:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:243:            event.event_type == AgentEventType::SessionStarted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:247:            event.event_type == AgentEventType::TurnInterrupted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:252:            .any(|event| event.event_type == AgentEventType::SessionClosed));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:263:            .any(|event| { event.event_type == AgentEventType::RuntimeWarning }));
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:8:    AgentConversationConnection, AgentConversationPayload, AgentConversationProvider,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:29:        emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:33:            AgentConversationPayload::Error {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:39:        emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:43:            AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:66:fn emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:70:    payload: AgentConversationPayload,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:133:                    emit(registry, app, c, AgentConversationPayload::Connection { state: ConversationConnectionState::Connected, native_session_id: thread_id.clone() });
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:166:                emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:170:                    AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:179:                emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:183:                    AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:190:        "item/agentMessage/delta" => emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:194:            AgentConversationPayload::AssistantDelta {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:206:            emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:210:                AgentConversationPayload::AssistantMessage {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:221:        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:225:            AgentConversationPayload::Approval {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:241:                emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:245:                    AgentConversationPayload::Tool {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:283:    emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:287:        AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:299:                    emit(registry, app, c, AgentConversationPayload::UserMessage {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:373:                emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:377:                    AgentConversationPayload::Connection {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:395:                    emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:399:                        AgentConversationPayload::AssistantDelta {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:428:                emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:432:                    AgentConversationPayload::AssistantMessage {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:443:            emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:447:                AgentConversationPayload::Turn {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:461:        "control_request" => emit(

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs | sed -n '1,220p;260,480p'
printf '\\n--- manager activation/prompt/event sections ---\\n'
nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs | sed -n '190,380p;430,540p;1010,1100p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	use serde_json::{json, Value};
     2	use tauri::AppHandle;
     3	use tokio::io::AsyncWriteExt;
     4	use tokio::sync::mpsc;
     5	
     6	use super::process::{validated_conversation_cwd, ConversationProcess};
     7	use super::protocol::{
     8	    AgentConversationConnection, AgentConversationPayload, AgentConversationProvider,
     9	    ApprovalState, ConversationConnectionState, ToolState, TurnState,
    10	};
    11	use super::{publish_payload, AgentConversationRegistry, ProviderCommand};
    12	
    13	pub(crate) async fn run_provider(
    14	    registry: AgentConversationRegistry,
    15	    app: AppHandle,
    16	    connection: AgentConversationConnection,
    17	    cwd: String,
    18	    commands: mpsc::UnboundedReceiver<ProviderCommand>,
    19	) {
    20	    let result = match connection.provider {
    21	        AgentConversationProvider::Codex => {
    22	            run_codex(&registry, &app, &connection, &cwd, commands).await
    23	        }
    24	        AgentConversationProvider::Claude => {
    25	            run_claude(&registry, &app, &connection, &cwd, commands).await
    26	        }
    27	    };
    28	    if let Err(message) = result {
    29	        emit(
    30	            &registry,
    31	            &app,
    32	            &connection,
    33	            AgentConversationPayload::Error {
    34	                code: "providerFailed".into(),
    35	                message,
    36	                recoverable: true,
    37	            },
    38	        );
    39	        emit(
    40	            &registry,
    41	            &app,
    42	            &connection,
    43	            AgentConversationPayload::Connection {
    44	                state: ConversationConnectionState::Failed,
    45	                native_session_id: None,
    46	            },
    47	        );
    48	    }
    49	}
    50	
    51	async fn write_json(process: &mut ConversationProcess, value: Value) -> Result<(), String> {
    52	    let mut line = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    53	    line.push(b'\n');
    54	    process
    55	        .stdin
    56	        .write_all(&line)
    57	        .await
    58	        .map_err(|e| format!("Could not write to provider: {e}"))?;
    59	    process
    60	        .stdin
    61	        .flush()
    62	        .await
    63	        .map_err(|e| format!("Could not flush provider input: {e}"))
    64	}
    65	
    66	fn emit(
    67	    registry: &AgentConversationRegistry,
    68	    app: &AppHandle,
    69	    c: &AgentConversationConnection,
    70	    payload: AgentConversationPayload,
    71	) {
    72	    publish_payload(registry, app, &c.owned_id, c.generation, payload);
    73	}
    74	
    75	async fn run_codex(
    76	    registry: &AgentConversationRegistry,
    77	    app: &AppHandle,
    78	    c: &AgentConversationConnection,
    79	    cwd: &str,
    80	    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
    81	) -> Result<(), String> {
    82	    let cwd_path = validated_conversation_cwd(cwd)?;
    83	    let mut process =
    84	        ConversationProcess::spawn("codex", &["app-server".into()], &cwd_path, &c.owned_id)?;
    85	    write_json(&mut process, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"mac-command-bar","version":env!("CARGO_PKG_VERSION")}}})).await?;
    86	    write_json(
    87	        &mut process,
    88	        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    89	    )
    90	    .await?;
    91	    let method = if c.native_session_id.is_some() {
    92	        "thread/resume"
    93	    } else {
    94	        "thread/start"
    95	    };
    96	    let params = if let Some(id) = &c.native_session_id {
    97	        json!({"threadId":id,"cwd":cwd})
    98	    } else {
    99	        json!({"cwd":cwd})
   100	    };
   101	    write_json(
   102	        &mut process,
   103	        json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}),
   104	    )
   105	    .await?;
   106	    let mut thread_id = c.native_session_id.clone();
   107	    let mut turn_id: Option<String> = None;
   108	    let mut request_counter = 10u64;
   109	    loop {
   110	        tokio::select! {
   111	            command = commands.recv() => match command {
   112	                Some(ProviderCommand::SendMessage(text)) => {
   113	                    let id = thread_id.clone().ok_or_else(|| "Codex thread is still connecting".to_string())?;
   114	                    request_counter += 1;
   115	                    write_json(&mut process, json!({"jsonrpc":"2.0","id":request_counter,"method":"turn/start","params":{"threadId":id,"input":[{"type":"text","text":text}]}})).await?;
   116	                }
   117	                Some(ProviderCommand::RespondApproval { request_id, decision }) => {
   118	                    let decision = match decision { super::protocol::ApprovalDecision::Accept => "accept", super::protocol::ApprovalDecision::Decline => "decline" };
   119	                    let id: Value = serde_json::from_str(&request_id).unwrap_or(Value::String(request_id));
   120	                    write_json(&mut process, json!({"jsonrpc":"2.0","id":id,"result":{"decision":decision}})).await?;
   121	                }
   122	                Some(ProviderCommand::Stop) => if let (Some(thread), Some(turn)) = (&thread_id, &turn_id) {
   123	                    request_counter += 1;
   124	                    write_json(&mut process, json!({"jsonrpc":"2.0","id":request_counter,"method":"turn/interrupt","params":{"threadId":thread,"turnId":turn}})).await?;
   125	                },
   126	                Some(ProviderCommand::Close) | None => { process.stop().await; return Ok(()); }
   127	            },
   128	            line = process.stdout.next_line() => {
   129	                let Some(line) = line.map_err(|e| format!("Could not read Codex output: {e}"))? else { return Err("Codex app-server exited".into()); };
   130	                let value: Value = match serde_json::from_str(&line) { Ok(v) => v, Err(_) => continue };
   131	                if value.get("id") == Some(&json!(2)) {
   132	                    thread_id = value.pointer("/result/thread/id").and_then(Value::as_str).map(str::to_string);
   133	                    emit(registry, app, c, AgentConversationPayload::Connection { state: ConversationConnectionState::Connected, native_session_id: thread_id.clone() });
   134	                }
   135	                normalize_codex(registry, app, c, &value, &mut turn_id);
   136	            },
   137	            line = process.stderr.next_line() => {
   138	                if let Ok(Some(line)) = line { eprintln!("mcb structured codex [{}]: {}", c.owned_id, line); }
   139	            }
   140	        }
   141	    }
   142	}
   143	
   144	fn normalize_codex(
   145	    registry: &AgentConversationRegistry,
   146	    app: &AppHandle,
   147	    c: &AgentConversationConnection,
   148	    value: &Value,
   149	    turn_id: &mut Option<String>,
   150	) {
   151	    let method = value.get("method").and_then(Value::as_str).unwrap_or("");
   152	    let p = value.get("params").unwrap_or(&Value::Null);
   153	    let item_id = || {
   154	        p.pointer("/item/id")
   155	            .and_then(Value::as_str)
   156	            .unwrap_or("assistant")
   157	            .to_string()
   158	    };
   159	    match method {
   160	        "turn/started" => {
   161	            *turn_id = p
   162	                .pointer("/turn/id")
   163	                .and_then(Value::as_str)
   164	                .map(str::to_string);
   165	            if let Some(id) = turn_id.clone() {
   166	                emit(
   167	                    registry,
   168	                    app,
   169	                    c,
   170	                    AgentConversationPayload::Turn {
   171	                        turn_id: id,
   172	                        state: TurnState::Started,
   173	                    },
   174	                );
   175	            }
   176	        }
   177	        "turn/completed" => {
   178	            if let Some(id) = turn_id.take() {
   179	                emit(
   180	                    registry,
   181	                    app,
   182	                    c,
   183	                    AgentConversationPayload::Turn {
   184	                        turn_id: id,
   185	                        state: TurnState::Completed,
   186	                    },
   187	                );
   188	            }
   189	        }
   190	        "item/agentMessage/delta" => emit(
   191	            registry,
   192	            app,
   193	            c,
   194	            AgentConversationPayload::AssistantDelta {
   195	                item_id: item_id(),
   196	                delta: p
   197	                    .get("delta")
   198	                    .and_then(Value::as_str)
   199	                    .unwrap_or("")
   200	                    .to_string(),
   201	            },
   202	        ),
   203	        "item/completed"
   204	            if p.pointer("/item/type").and_then(Value::as_str) == Some("agentMessage") =>
   205	        {
   206	            emit(
   207	                registry,
   208	                app,
   209	                c,
   210	                AgentConversationPayload::AssistantMessage {
   211	                    item_id: item_id(),
   212	                    text: p
   213	                        .pointer("/item/text")
   214	                        .and_then(Value::as_str)
   215	                        .unwrap_or("")
   216	                        .to_string(),
   217	                    completed: true,
   218	                },
   219	            )
   220	        }
   260	    app: &AppHandle,
   261	    c: &AgentConversationConnection,
   262	    cwd: &str,
   263	    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
   264	) -> Result<(), String> {
   265	    let cwd_path = validated_conversation_cwd(cwd)?;
   266	    let mut args = vec![
   267	        "-p".into(),
   268	        "--input-format".into(),
   269	        "stream-json".into(),
   270	        "--output-format".into(),
   271	        "stream-json".into(),
   272	        "--verbose".into(),
   273	        "--include-partial-messages".into(),
   274	        "--replay-user-messages".into(),
   275	    ];
   276	    if let Some(id) = &c.native_session_id {
   277	        args.extend(["--resume".into(), id.clone()]);
   278	    }
   279	    let mut process = ConversationProcess::spawn("claude", &args, &cwd_path, &c.owned_id)?;
   280	    let mut assistant_item_id: Option<String> = None;
   281	    let mut assistant_text = String::new();
   282	    let mut response_sequence = 0_u64;
   283	    emit(
   284	        registry,
   285	        app,
   286	        c,
   287	        AgentConversationPayload::Connection {
   288	            state: ConversationConnectionState::Connected,
   289	            native_session_id: c.native_session_id.clone(),
   290	        },
   291	    );
   292	    loop {
   293	        tokio::select! {
   294	            command=commands.recv()=>match command {
   295	                Some(ProviderCommand::SendMessage(text))=>{
   296	                    let (user_item_id, response_item_id) = next_claude_turn_ids(c.generation, &mut response_sequence);
   297	                    assistant_item_id = Some(response_item_id);
   298	                    assistant_text.clear();
   299	                    emit(registry, app, c, AgentConversationPayload::UserMessage {
   300	                        item_id: user_item_id,
   301	                        text: text.clone(),
   302	                        completed: true,
   303	                    });
   304	                    write_json(&mut process,json!({"type":"user","message":{"role":"user","content":text}})).await?
   305	                },
   306	                Some(ProviderCommand::Stop)=>write_json(&mut process,json!({"type":"control_request","request_id":format!("stop-{}",c.generation),"request":{"subtype":"interrupt"}})).await?,
   307	                Some(ProviderCommand::RespondApproval{request_id,decision})=>write_json(&mut process,json!({"type":"control_response","response":{"subtype":"success","request_id":request_id,"response":{"behavior":match decision { super::protocol::ApprovalDecision::Accept=>"allow",super::protocol::ApprovalDecision::Decline=>"deny"}}}})).await?,
   308	                Some(ProviderCommand::Close)|None=>{process.stop().await;return Ok(());}
   309	            },
   310	            line=process.stdout.next_line()=>{ let Some(line)=line.map_err(|e|e.to_string())? else{return Err("Claude exited".into())}; if let Ok(v)=serde_json::from_str::<Value>(&line){normalize_claude(registry,app,c,&v,&mut assistant_item_id,&mut assistant_text);} },
   311	            line=process.stderr.next_line()=>{if let Ok(Some(line))=line{eprintln!("mcb structured claude [{}]: {}",c.owned_id,line);}}
   312	        }
   313	    }
   314	}
   315	
   316	fn next_claude_turn_ids(generation: u64, response_sequence: &mut u64) -> (String, String) {
   317	    *response_sequence = response_sequence.saturating_add(1);
   318	    (
   319	        format!("claude-user-{generation}-{response_sequence}"),
   320	        format!("claude-response-{generation}-{response_sequence}"),
   321	    )
   322	}
   323	
   324	fn completed_claude_text(streamed_text: &str, final_text: String) -> String {
   325	    if streamed_text.is_empty() {
   326	        final_text
   327	    } else {
   328	        streamed_text.to_string()
   329	    }
   330	}
   331	
   332	#[cfg(test)]
   333	mod tests {
   334	    use super::{completed_claude_text, next_claude_turn_ids};
   335	
   336	    #[test]
   337	    fn claude_responses_receive_distinct_fallback_item_ids() {
   338	        let mut sequence = 0;
   339	
   340	        let (first_user, first) = next_claude_turn_ids(4, &mut sequence);
   341	        let (second_user, second) = next_claude_turn_ids(4, &mut sequence);
   342	
   343	        assert_eq!(first_user, "claude-user-4-1");
   344	        assert_eq!(first, "claude-response-4-1");
   345	        assert_eq!(second_user, "claude-user-4-2");
   346	        assert_eq!(second, "claude-response-4-2");
   347	        assert_ne!(first, second);
   348	    }
   349	
   350	    #[test]
   351	    fn claude_final_snapshot_cannot_erase_streamed_sentences() {
   352	        let streamed = "First sentence. Second sentence.";
   353	        let incomplete_final_snapshot = "Second sentence.".to_string();
   354	
   355	        assert_eq!(
   356	            completed_claude_text(streamed, incomplete_final_snapshot),
   357	            streamed
   358	        );
   359	    }
   360	}
   361	
   362	fn normalize_claude(
   363	    registry: &AgentConversationRegistry,
   364	    app: &AppHandle,
   365	    c: &AgentConversationConnection,
   366	    v: &Value,
   367	    assistant_item_id: &mut Option<String>,
   368	    assistant_text: &mut String,
   369	) {
   370	    match v.get("type").and_then(Value::as_str).unwrap_or("") {
   371	        "system" => {
   372	            if let Some(id) = v.get("session_id").and_then(Value::as_str) {
   373	                emit(
   374	                    registry,
   375	                    app,
   376	                    c,
   377	                    AgentConversationPayload::Connection {
   378	                        state: ConversationConnectionState::Connected,
   379	                        native_session_id: Some(id.into()),
   380	                    },
   381	                );
   382	            }
   383	        }
   384	        "stream_event" => {
   385	            if v.pointer("/event/type").and_then(Value::as_str) == Some("message_start") {
   386	                if let Some(item_id) = v.pointer("/event/message/id").and_then(Value::as_str) {
   387	                    *assistant_item_id = Some(item_id.to_string());
   388	                }
   389	                assistant_text.clear();
   390	            } else if v.pointer("/event/type").and_then(Value::as_str)
   391	                == Some("content_block_delta")
   392	            {
   393	                if let Some(delta) = v.pointer("/event/delta/text").and_then(Value::as_str) {
   394	                    assistant_text.push_str(delta);
   395	                    emit(
   396	                        registry,
   397	                        app,
   398	                        c,
   399	                        AgentConversationPayload::AssistantDelta {
   400	                            item_id: assistant_item_id
   401	                                .clone()
   402	                                .unwrap_or_else(|| format!("assistant-{}", c.generation)),
   403	                            delta: delta.into(),
   404	                        },
   405	                    );
   406	                }
   407	            }
   408	        }
   409	        "assistant" => {
   410	            let text = v
   411	                .pointer("/message/content")
   412	                .and_then(Value::as_array)
   413	                .map(|a| {
   414	                    a.iter()
   415	                        .filter_map(|x| x.get("text").and_then(Value::as_str))
   416	                        .collect::<Vec<_>>()
   417	                        .join("\n")
   418	                })
   419	                .unwrap_or_default();
   420	            let completed_text = completed_claude_text(assistant_text, text);
   421	            if !completed_text.is_empty() {
   422	                let item_id = v
   423	                    .pointer("/message/id")
   424	                    .and_then(Value::as_str)
   425	                    .map(str::to_string)
   426	                    .or_else(|| assistant_item_id.clone())
   427	                    .unwrap_or_else(|| format!("assistant-{}", c.generation));
   428	                emit(
   429	                    registry,
   430	                    app,
   431	                    c,
   432	                    AgentConversationPayload::AssistantMessage {
   433	                        item_id,
   434	                        text: completed_text,
   435	                        completed: true,
   436	                    },
   437	                );
   438	            }
   439	        }
   440	        "result" => {
   441	            *assistant_item_id = None;
   442	            assistant_text.clear();
   443	            emit(
   444	                registry,
   445	                app,
   446	                c,
   447	                AgentConversationPayload::Turn {
   448	                    turn_id: v
   449	                        .get("session_id")
   450	                        .and_then(Value::as_str)
   451	                        .unwrap_or("turn")
   452	                        .into(),
   453	                    state: if v.get("is_error").and_then(Value::as_bool) == Some(true) {
   454	                        TurnState::Failed
   455	                    } else {
   456	                        TurnState::Completed
   457	                    },
   458	                },
   459	            )
   460	        }
   461	        "control_request" => emit(
   462	            registry,
   463	            app,
   464	            c,
   465	            AgentConversationPayload::Approval {
   466	                request_id: v
   467	                    .get("request_id")
   468	                    .and_then(Value::as_str)
   469	                    .unwrap_or_default()
   470	                    .into(),
   471	                state: ApprovalState::Requested,
   472	                summary: v
   473	                    .pointer("/request/tool_name")
   474	                    .and_then(Value::as_str)
   475	                    .unwrap_or("Tool approval requested")
   476	                    .into(),
   477	            },
   478	        ),
   479	        _ => {}
   480	    }

--- manager activation/prompt/event sections ---
   190	    pub async fn activate(
   191	        &self,
   192	        owned_id: &str,
   193	        generation: u64,
   194	    ) -> Result<AgentConversationConnection, String> {
   195	        let (provider, provider_instance_id, cwd, native_session_id) = {
   196	            let sessions = self
   197	                .sessions
   198	                .lock()
   199	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   200	            let session = current_session(&sessions, owned_id, generation)?;
   201	            (
   202	                session.provider,
   203	                session.provider_instance_id.clone(),
   204	                std::path::PathBuf::from(&session.cwd),
   205	                session.native_session_id.clone(),
   206	            )
   207	        };
   208	        let manifest = self.providers.manifest(provider)?;
   209	        let mut adapter = AcpRuntimeAdapter::new(manifest, owned_id.to_string(), cwd.clone());
   210	        let capabilities = adapter
   211	            .initialize(InitializeAgentInput {
   212	                provider,
   213	                provider_instance_id,
   214	            })
   215	            .await
   216	            .map_err(|error| error.to_string())?;
   217	        let expected_native_session_id = native_session_id.clone();
   218	        let started = match native_session_id {
   219	            Some(native_session_id) => {
   220	                adapter
   221	                    .resume_session(LoadAgentSession {
   222	                        cwd,
   223	                        native_session_id,
   224	                    })
   225	                    .await
   226	            }
   227	            None => adapter.new_session(NewAgentSession { cwd }).await,
   228	        }
   229	        .map_err(|error| error.to_string())?;
   230	        if let Some(expected_native_session_id) = expected_native_session_id {
   231	            if started.native_session_id != expected_native_session_id {
   232	                let _ = adapter.detach_session().await;
   233	                return Err(
   234	                    "The provider resumed a different native session; handoff was rejected"
   235	                        .to_string(),
   236	                );
   237	            }
   238	        }
   239	        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
   240	        let mut sessions = self
   241	            .sessions
   242	            .lock()
   243	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   244	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   245	        session.capabilities = capabilities;
   246	        session.native_session_id = Some(started.native_session_id.clone());
   247	        session.connection.native_session_id = Some(started.native_session_id);
   248	        session.connection.state = ConversationConnectionState::Connected;
   249	        let restoring_terminal_transition = session.owner
   250	            == AgentExecutionOwner::TransitioningToStructured
   251	            && session
   252	                .writer_lease_transition
   253	                .as_ref()
   254	                .map(|transition| transition.to == AgentWriterLeaseOwner::Structured)
   255	                .unwrap_or(false);
   256	        if !restoring_terminal_transition {
   257	            session.owner = AgentExecutionOwner::Structured;
   258	            session.writer_lease.owner = AgentWriterLeaseOwner::Structured;
   259	        }
   260	        session.state = AgentRuntimeState::Ready;
   261	        session.runtime = Some(runtime);
   262	        Ok(session.connection.clone())
   263	    }
   264	
   265	    pub fn emit_payload(
   266	        &self,
   267	        owned_id: &str,
   268	        generation: u64,
   269	        payload: AgentConversationPayload,
   270	    ) -> Result<AgentConversationEvent, String> {
   271	        let mut sessions = self
   272	            .sessions
   273	            .lock()
   274	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   275	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   276	        let sequence = session.next_sequence;
   277	        session.next_sequence = session.next_sequence.saturating_add(1);
   278	        if let AgentConversationPayload::Connection {
   279	            state,
   280	            native_session_id,
   281	        } = &payload
   282	        {
   283	            session.connection.state = *state;
   284	            if native_session_id.is_some() {
   285	                session.connection.native_session_id = native_session_id.clone();
   286	                session.native_session_id = native_session_id.clone();
   287	            }
   288	        }
   289	        let timestamp_ms = timestamp_millis();
   290	        let frontend_event = AgentConversationEvent {
   291	            owned_id: owned_id.to_string(),
   292	            provider: session.provider,
   293	            generation,
   294	            sequence,
   295	            timestamp_ms,
   296	            payload: payload.clone(),
   297	        };
   298	        let canonical = canonical_event(session, sequence, timestamp_ms, &payload)?;
   299	        session.journal.append(canonical.clone())?;
   300	        session.recent_events.push_back(canonical);
   301	        session.frontend_events.push_back(frontend_event.clone());
   302	        while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
   303	            session.recent_events.pop_front();
   304	        }
   305	        while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
   306	            session.frontend_events.pop_front();
   307	        }
   308	        Ok(frontend_event)
   309	    }
   310	
   311	    pub async fn prompt(
   312	        &self,
   313	        owned_id: &str,
   314	        generation: u64,
   315	        input: AgentPrompt,
   316	    ) -> Result<(), String> {
   317	        let runtime = self.runtime(owned_id, generation)?;
   318	        let started = runtime
   319	            .lock()
   320	            .await
   321	            .prompt(input)
   322	            .await
   323	            .map_err(|error| error.to_string())?;
   324	        let mut sessions = self
   325	            .sessions
   326	            .lock()
   327	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   328	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   329	        session.active_turn_id = started.turn_id;
   330	        session.state = AgentRuntimeState::Working;
   331	        Ok(())
   332	    }
   333	
   334	    /// Run one product action through the current structured agent without
   335	    /// writing through the conversation UI. The runtime and generation checks
   336	    /// remain the same as an ordinary conversation prompt.
   337	    pub async fn prompt_once(
   338	        &self,
   339	        owned_id: &str,
   340	        generation: u64,
   341	        input: AgentPrompt,
   342	    ) -> Result<GeneratedText, String> {
   343	        let runtime = self.runtime(owned_id, generation)?;
   344	        let result = runtime
   345	            .lock()
   346	            .await
   347	            .prompt_once(input)
   348	            .await
   349	            .map_err(|error| error.to_string());
   350	        result
   351	    }
   352	
   353	    pub async fn respond_permission(&self, input: PermissionResponse) -> Result<(), String> {
   354	        let runtime = self.runtime(&input.identity.owned_id, input.identity.generation)?;
   355	        let result = runtime
   356	            .lock()
   357	            .await
   358	            .respond_permission(input)
   359	            .await
   360	            .map_err(|error| error.to_string());
   361	        result
   362	    }
   363	
   364	    pub async fn respond_user_input(&self, input: AgentUserInputResponse) -> Result<(), String> {
   365	        let runtime = self.runtime(&input.identity.owned_id, input.identity.generation)?;
   366	        let result = runtime
   367	            .lock()
   368	            .await
   369	            .respond_user_input(input)
   370	            .await
   371	            .map_err(|error| error.to_string());
   372	        result
   373	    }
   374	
   375	    pub async fn steer(&self, owned_id: &str, generation: u64, text: String) -> Result<(), String> {
   376	        let runtime = self.runtime(owned_id, generation)?;
   377	        let result = runtime
   378	            .lock()
   379	            .await
   380	            .steer(AgentSteeringInput { text })
   430	        app: tauri::AppHandle<R>,
   431	        terminal_registry: &crate::terminal::TerminalRegistry,
   432	        generation: u64,
   433	        request: crate::terminal::TerminalStartRequest,
   434	        identity: crate::terminal::ToolTerminalIdentity,
   435	    ) -> Result<crate::terminal::TerminalSessionInfo, String> {
   436	        {
   437	            let sessions = self
   438	                .sessions
   439	                .lock()
   440	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   441	            let session = current_session(&sessions, &identity.owned_id, generation)?;
   442	            if session.owner != AgentExecutionOwner::Structured || session.runtime.is_none() {
   443	                return Err(
   444	                    "Tool terminal request is not from a current structured runtime".to_string(),
   445	                );
   446	            }
   447	            if identity.turn_id.trim().is_empty() || identity.tool_call_id.trim().is_empty() {
   448	                return Err("Tool terminal request identity is incomplete".to_string());
   449	            }
   450	        }
   451	        crate::terminal::start_tool_terminal_session(app, terminal_registry, request, identity)
   452	    }
   453	
   454	    pub async fn respond_legacy_approval(
   455	        &self,
   456	        owned_id: &str,
   457	        generation: u64,
   458	        request_id: String,
   459	        decision: super::protocol::ApprovalDecision,
   460	    ) -> Result<(), String> {
   461	        let decision = match decision {
   462	            super::protocol::ApprovalDecision::Accept => AgentApprovalDecision::Accept,
   463	            super::protocol::ApprovalDecision::Decline => AgentApprovalDecision::Decline,
   464	        };
   465	        self.respond_permission(AgentApprovalResponse {
   466	            identity: AgentRequestIdentity {
   467	                owned_id: owned_id.to_string(),
   468	                generation,
   469	                request_id,
   470	                turn_id: None,
   471	                item_id: None,
   472	            },
   473	            decision,
   474	        })
   475	        .await
   476	    }
   477	
   478	    pub async fn cancel_turn(&self, owned_id: &str, generation: u64) -> Result<(), String> {
   479	        let runtime = self.runtime(owned_id, generation)?;
   480	        let turn_id = {
   481	            let sessions = self
   482	                .sessions
   483	                .lock()
   484	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   485	            current_session(&sessions, owned_id, generation)?
   486	                .active_turn_id
   487	                .clone()
   488	        };
   489	        let result = runtime
   490	            .lock()
   491	            .await
   492	            .cancel_turn(turn_id.as_deref())
   493	            .await
   494	            .map_err(|error| error.to_string());
   495	        result
   496	    }
   497	
   498	    pub fn snapshot(&self, owned_id: &str) -> Result<Option<AgentConversationSnapshot>, String> {
   499	        let sessions = self
   500	            .sessions
   501	            .lock()
   502	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   503	        Ok(sessions
   504	            .get(owned_id)
   505	            .map(|session| AgentConversationSnapshot {
   506	                connection: session.connection.clone(),
   507	                last_sequence: session.next_sequence.saturating_sub(1),
   508	                events: session.frontend_events.iter().cloned().collect(),
   509	            }))
   510	    }
   511	
   512	    pub async fn close(&self, owned_id: &str) -> Result<bool, String> {
   513	        let runtime = {
   514	            let mut sessions = self
   515	                .sessions
   516	                .lock()
   517	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   518	            let Some(session) = sessions.get_mut(owned_id) else {
   519	                return Ok(false);
   520	            };
   521	            session.state = AgentRuntimeState::Closed;
   522	            session.owner = AgentExecutionOwner::Stopped;
   523	            session.writer_lease.owner = AgentWriterLeaseOwner::None;
   524	            session.runtime.take()
   525	        };
   526	        if let Some(runtime) = runtime {
   527	            runtime
   528	                .lock()
   529	                .await
   530	                .close_session()
   531	                .await
   532	                .map_err(|error| error.to_string())?;
   533	        }
   534	        self.sessions
   535	            .lock()
   536	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?
   537	            .remove(owned_id);
   538	        Ok(true)
   539	    }
   540	
  1010	            owned_id: owned_id.into(),
  1011	            provider,
  1012	            cwd: root.into(),
  1013	            native_session_id: None,
  1014	        }
  1015	    }
  1016	    fn temp_root() -> std::path::PathBuf {
  1017	        let path = std::env::temp_dir().join(format!("mcb-runtime-{}", uuid::Uuid::new_v4()));
  1018	        fs::create_dir_all(&path).unwrap();
  1019	        path
  1020	    }
  1021	
  1022	    #[test]
  1023	    fn ensure_is_idempotent_and_generation_rejects_stale_writers() {
  1024	        let root = temp_root();
  1025	        let manager = AgentRuntimeManager::default();
  1026	        let first = manager
  1027	            .ensure(request(
  1028	                root.to_str().unwrap(),
  1029	                "owned-a",
  1030	                AgentConversationProvider::Codex,
  1031	            ))
  1032	            .unwrap();
  1033	        assert_eq!(
  1034	            manager
  1035	                .ensure(request(
  1036	                    root.to_str().unwrap(),
  1037	                    "owned-a",
  1038	                    AgentConversationProvider::Codex
  1039	                ))
  1040	                .unwrap(),
  1041	            first
  1042	        );
  1043	        let changed = manager
  1044	            .ensure(request(
  1045	                root.to_str().unwrap(),
  1046	                "owned-a",
  1047	                AgentConversationProvider::Claude,
  1048	            ))
  1049	            .unwrap();
  1050	        assert_eq!(changed.generation, 2);
  1051	        assert!(manager
  1052	            .emit_payload(
  1053	                "owned-a",
  1054	                first.generation,
  1055	                AgentConversationPayload::Error {
  1056	                    code: "stale".into(),
  1057	                    message: "stale".into(),
  1058	                    recoverable: true
  1059	                }
  1060	            )
  1061	            .is_err());
  1062	        fs::remove_dir_all(root).unwrap();
  1063	    }
  1064	
  1065	    #[test]
  1066	    fn canonical_sequence_and_snapshot_are_bounded_and_repairable() {
  1067	        let root = temp_root();
  1068	        let manager = AgentRuntimeManager::default();
  1069	        let connection = manager
  1070	            .ensure(request(
  1071	                root.to_str().unwrap(),
  1072	                "owned-a",
  1073	                AgentConversationProvider::Codex,
  1074	            ))
  1075	            .unwrap();
  1076	        for index in 0..(SNAPSHOT_EVENT_CAP + 3) {
  1077	            manager
  1078	                .emit_payload(
  1079	                    "owned-a",
  1080	                    connection.generation,
  1081	                    AgentConversationPayload::Error {
  1082	                        code: "fixture".into(),
  1083	                        message: index.to_string(),
  1084	                        recoverable: true,
  1085	                    },
  1086	                )
  1087	                .unwrap();
  1088	        }
  1089	        let events = manager.canonical_snapshot("owned-a").unwrap();
  1090	        assert_eq!(events.len(), SNAPSHOT_EVENT_CAP);
  1091	        assert!(events
  1092	            .windows(2)
  1093	            .all(|pair| pair[1].sequence == pair[0].sequence + 1));
  1094	        fs::remove_dir_all(root).unwrap();
  1095	    }
  1096	
  1097	    #[test]
  1098	    fn structured_session_does_not_claim_a_user_pty() {
  1099	        let root = temp_root();
  1100	        let manager = AgentRuntimeManager::default();

exec
/bin/zsh -lc 'rg -n "mod provider|publish_payload|AgentConversationRegistry|ProviderCommand" tauri-svelte-preview/src-tauri/src/agent_conversation tauri-svelte-preview/src-tauri/src | head -120' in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:7:pub mod providers;
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:11:use super::{publish_payload, AgentConversationRegistry, ProviderCommand};
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:14:    registry: AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:18:    commands: mpsc::UnboundedReceiver<ProviderCommand>,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:67:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:72:    publish_payload(registry, app, &c.owned_id, c.generation, payload);
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:76:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:80:    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:112:                Some(ProviderCommand::SendMessage(text)) => {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:117:                Some(ProviderCommand::RespondApproval { request_id, decision }) => {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:122:                Some(ProviderCommand::Stop) => if let (Some(thread), Some(turn)) = (&thread_id, &turn_id) {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:126:                Some(ProviderCommand::Close) | None => { process.stop().await; return Ok(()); }
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:145:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:259:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:263:    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:295:                Some(ProviderCommand::SendMessage(text))=>{
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:306:                Some(ProviderCommand::Stop)=>write_json(&mut process,json!({"type":"control_request","request_id":format!("stop-{}",c.generation),"request":{"subtype":"interrupt"}})).await?,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:307:                Some(ProviderCommand::RespondApproval{request_id,decision})=>write_json(&mut process,json!({"type":"control_response","response":{"subtype":"success","request_id":request_id,"response":{"behavior":match decision { super::protocol::ApprovalDecision::Accept=>"allow",super::protocol::ApprovalDecision::Decline=>"deny"}}}})).await?,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:308:                Some(ProviderCommand::Close)|None=>{process.stop().await;return Ok(());}
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:363:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:7:pub mod providers;
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:11:use super::{publish_payload, AgentConversationRegistry, ProviderCommand};
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:14:    registry: AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:18:    commands: mpsc::UnboundedReceiver<ProviderCommand>,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:67:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:72:    publish_payload(registry, app, &c.owned_id, c.generation, payload);
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:76:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:80:    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:112:                Some(ProviderCommand::SendMessage(text)) => {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:117:                Some(ProviderCommand::RespondApproval { request_id, decision }) => {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:122:                Some(ProviderCommand::Stop) => if let (Some(thread), Some(turn)) = (&thread_id, &turn_id) {
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:126:                Some(ProviderCommand::Close) | None => { process.stop().await; return Ok(()); }
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:145:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:259:    registry: &AgentConversationRegistry,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:263:    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:295:                Some(ProviderCommand::SendMessage(text))=>{
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:306:                Some(ProviderCommand::Stop)=>write_json(&mut process,json!({"type":"control_request","request_id":format!("stop-{}",c.generation),"request":{"subtype":"interrupt"}})).await?,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:307:                Some(ProviderCommand::RespondApproval{request_id,decision})=>write_json(&mut process,json!({"type":"control_response","response":{"subtype":"success","request_id":request_id,"response":{"behavior":match decision { super::protocol::ApprovalDecision::Accept=>"allow",super::protocol::ApprovalDecision::Decline=>"deny"}}}})).await?,
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:308:                Some(ProviderCommand::Close)|None=>{process.stop().await;return Ok(());}
tauri-svelte-preview/src-tauri/src/agent_conversation/provider.rs:363:    registry: &AgentConversationRegistry,

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs | sed -n '1,360p'
printf '\\n--- providers adapter trait/mod ---\\n'
nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs | sed -n '1,360p'
printf '\\n--- manager turn runtime event loops ---\\n'
rg -n -C 8 \"read_event|next_event|poll|GeneratedText|StartedTurn|AgentEvent\" tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs | sed -n '1,500p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	use std::collections::BTreeMap;
     2	use std::path::Path;
     3	
     4	use agent_client_protocol::schema::{v1 as acp, ProtocolVersion};
     5	use serde::Serialize;
     6	use serde_json::{json, Value};
     7	
     8	use super::super::protocol::{
     9	    AgentCapabilities, AgentCommandDescriptor, AgentConfigOption, AgentConversationProvider,
    10	    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
    11	    AgentProviderManifest, AgentSessionCapabilities,
    12	};
    13	use super::process::SidecarProcess;
    14	use super::{AgentPrompt, AgentRuntimeError, GeneratedText, StartedAgentSession, StartedTurn};
    15	
    16	pub struct AcpClient {
    17	    process: SidecarProcess,
    18	    next_request_id: u64,
    19	    native_session_id: Option<String>,
    20	}
    21	
    22	impl AcpClient {
    23	    pub fn spawn(
    24	        manifest: &AgentProviderManifest,
    25	        cwd: &Path,
    26	        owned_id: &str,
    27	    ) -> Result<Self, AgentRuntimeError> {
    28	        Ok(Self {
    29	            process: SidecarProcess::spawn(manifest, cwd, owned_id)
    30	                .map_err(|message| AgentRuntimeError::new("sidecar-spawn", message))?,
    31	            next_request_id: 0,
    32	            native_session_id: None,
    33	        })
    34	    }
    35	
    36	    pub async fn initialize(
    37	        &mut self,
    38	        provider: AgentConversationProvider,
    39	    ) -> Result<AgentCapabilities, AgentRuntimeError> {
    40	        let request = acp::InitializeRequest::new(ProtocolVersion::V1);
    41	        let result = self.request("initialize", &request).await?;
    42	        let implementation = result.get("agentInfo").or_else(|| result.get("agent_info"));
    43	        Ok(AgentCapabilities {
    44	            revision: 1,
    45	            provider,
    46	            implementation: AgentImplementation {
    47	                name: implementation
    48	                    .and_then(|value| value.get("name"))
    49	                    .and_then(Value::as_str)
    50	                    .unwrap_or("ACP adapter")
    51	                    .to_string(),
    52	                version: implementation
    53	                    .and_then(|value| value.get("version"))
    54	                    .and_then(Value::as_str)
    55	                    .unwrap_or("unknown")
    56	                    .to_string(),
    57	            },
    58	            session: AgentSessionCapabilities {
    59	                list: bool_at(
    60	                    &result,
    61	                    &["agentCapabilities", "sessionCapabilities", "list"],
    62	                ),
    63	                load: bool_at(&result, &["agentCapabilities", "loadSession"]),
    64	                resume: bool_at(
    65	                    &result,
    66	                    &["agentCapabilities", "sessionCapabilities", "resume"],
    67	                ),
    68	                close: bool_at(
    69	                    &result,
    70	                    &["agentCapabilities", "sessionCapabilities", "close"],
    71	                ),
    72	                steering: bool_at(
    73	                    &result,
    74	                    &["agentCapabilities", "sessionCapabilities", "steering"],
    75	                ),
    76	                fork: bool_at(
    77	                    &result,
    78	                    &["agentCapabilities", "sessionCapabilities", "fork"],
    79	                ) || bool_at(&result, &["agentCapabilities", "fork"]),
    80	            },
    81	            prompt: AgentPromptCapabilities {
    82	                text: true,
    83	                image: bool_at(
    84	                    &result,
    85	                    &["agentCapabilities", "promptCapabilities", "image"],
    86	                ),
    87	                embedded_context: bool_at(
    88	                    &result,
    89	                    &["agentCapabilities", "promptCapabilities", "embeddedContext"],
    90	                ),
    91	                resource_links: true,
    92	            },
    93	            interaction: AgentInteractionCapabilities {
    94	                permissions: true,
    95	                structured_user_input: true,
    96	                tool_terminals: true,
    97	                plans: true,
    98	                tasks: true,
    99	                subagents: true,
   100	            },
   101	            config_options: parse_config_options(result.get("sessionConfigOptions")),
   102	            commands: Vec::<AgentCommandDescriptor>::new(),
   103	        })
   104	    }
   105	
   106	    pub async fn new_session(
   107	        &mut self,
   108	        cwd: &Path,
   109	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
   110	        let request = acp::NewSessionRequest::new(cwd);
   111	        self.start_session("session/new", &request).await
   112	    }
   113	
   114	    pub async fn load_session(
   115	        &mut self,
   116	        cwd: &Path,
   117	        native_session_id: &str,
   118	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
   119	        let request = acp::LoadSessionRequest::new(native_session_id.to_string(), cwd);
   120	        self.start_session("session/load", &request).await
   121	    }
   122	
   123	    pub async fn resume_session(
   124	        &mut self,
   125	        cwd: &Path,
   126	        native_session_id: &str,
   127	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
   128	        let request = acp::ResumeSessionRequest::new(native_session_id.to_string(), cwd);
   129	        self.start_session("session/resume", &request).await
   130	    }
   131	
   132	    pub async fn prompt(&mut self, prompt: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
   133	        let session_id = self.session_id()?;
   134	        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
   135	        blocks.extend(prompt.images.into_iter().map(|image| {
   136	            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
   137	        }));
   138	        let result = self
   139	            .request(
   140	                "session/prompt",
   141	                &acp::PromptRequest::new(session_id, blocks),
   142	            )
   143	            .await?;
   144	        Ok(StartedTurn {
   145	            turn_id: result
   146	                .get("turnId")
   147	                .and_then(Value::as_str)
   148	                .map(str::to_string),
   149	        })
   150	    }
   151	
   152	    /// Send a bounded one-shot prompt and retain only assistant text updates.
   153	    ///
   154	    /// The regular `prompt` call intentionally returns as soon as ACP accepts
   155	    /// the request; the conversation event stream owns the rest of that turn.
   156	    /// Git actions need a direct result, so this narrow path consumes the same
   157	    /// ACP transport until the prompt response arrives and ignores tool,
   158	    /// reasoning, and lifecycle updates.
   159	    pub async fn prompt_once(
   160	        &mut self,
   161	        prompt: AgentPrompt,
   162	    ) -> Result<GeneratedText, AgentRuntimeError> {
   163	        let session_id = self.session_id()?;
   164	        self.next_request_id = self.next_request_id.saturating_add(1);
   165	        let request_id = self.next_request_id;
   166	        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
   167	        blocks.extend(prompt.images.into_iter().map(|image| {
   168	            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
   169	        }));
   170	        let params = acp::PromptRequest::new(session_id, blocks);
   171	        let params = serde_json::to_value(params).map_err(serialization_error)?;
   172	        self.process
   173	            .write_json(&json!({
   174	                "jsonrpc": "2.0",
   175	                "id": request_id,
   176	                "method": "session/prompt",
   177	                "params": params
   178	            }))
   179	            .await
   180	            .map_err(transport_error)?;
   181	
   182	        let mut text = String::new();
   183	        let mut turn_id = None;
   184	        loop {
   185	            let frame = self.process.next_json().await.map_err(transport_error)?;
   186	            if frame.get("id") == Some(&json!(request_id)) {
   187	                if let Some(error) = frame.get("error") {
   188	                    return Err(AgentRuntimeError::new("acp-error", error.to_string()));
   189	                }
   190	                if let Some(result_text) = frame
   191	                    .get("result")
   192	                    .and_then(extract_text_value)
   193	                    .filter(|value| !value.trim().is_empty())
   194	                {
   195	                    text.push_str(&result_text);
   196	                }
   197	                let text = text.trim().to_string();
   198	                if text.is_empty() {
   199	                    return Err(AgentRuntimeError::new(
   200	                        "empty-response",
   201	                        "The active agent returned no text",
   202	                    ));
   203	                }
   204	                return Ok(GeneratedText { turn_id, text });
   205	            }
   206	
   207	            let params = frame.get("params").unwrap_or(&Value::Null);
   208	            let update = params
   209	                .get("update")
   210	                .or_else(|| params.get("sessionUpdate"))
   211	                .unwrap_or(params);
   212	            if let Some(id) = update
   213	                .get("turnId")
   214	                .or_else(|| update.get("turn_id"))
   215	                .and_then(Value::as_str)
   216	            {
   217	                turn_id = Some(id.to_string());
   218	            }
   219	            if update
   220	                .get("sessionUpdate")
   221	                .or_else(|| update.get("session_update"))
   222	                .or_else(|| update.get("type"))
   223	                .and_then(Value::as_str)
   224	                .is_some_and(|kind| {
   225	                    matches!(
   226	                        kind,
   227	                        "agent_message_chunk"
   228	                            | "agent-message-chunk"
   229	                            | "agent_message"
   230	                            | "assistant_message_delta"
   231	                    )
   232	                })
   233	            {
   234	                if let Some(chunk) = update.get("content").and_then(extract_text_value) {
   235	                    text.push_str(&chunk);
   236	                }
   237	                if let Some(message) = update.get("message").and_then(extract_text_value) {
   238	                    text.push_str(&message);
   239	                }
   240	            }
   241	        }
   242	    }
   243	
   244	    pub async fn steer(&mut self, text: String) -> Result<(), AgentRuntimeError> {
   245	        let session_id = self.session_id()?;
   246	        self.request(
   247	            "session/steer",
   248	            &json!({ "sessionId": session_id, "prompt": [{ "type": "text", "text": text }] }),
   249	        )
   250	        .await?;
   251	        Ok(())
   252	    }
   253	
   254	    pub async fn cancel(&mut self) -> Result<(), AgentRuntimeError> {
   255	        let value = serde_json::to_value(acp::CancelNotification::new(self.session_id()?))
   256	            .map_err(serialization_error)?;
   257	        self.process
   258	            .write_json(&json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": value }))
   259	            .await
   260	            .map_err(transport_error)
   261	    }
   262	
   263	    pub async fn set_config(
   264	        &mut self,
   265	        option_id: &str,
   266	        value: Value,
   267	    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
   268	        let result = self
   269	            .request(
   270	                "session/set_config_option",
   271	                &json!({ "sessionId": self.session_id()?, "configId": option_id, "value": value }),
   272	            )
   273	            .await?;
   274	        Ok(parse_config_options(
   275	            result
   276	                .get("configOptions")
   277	                .or_else(|| result.get("sessionConfigOptions")),
   278	        ))
   279	    }
   280	
   281	    pub async fn respond_permission(
   282	        &mut self,
   283	        request_id: &str,
   284	        decision: &str,
   285	    ) -> Result<(), AgentRuntimeError> {
   286	        self.respond(request_id, json!({ "outcome": { "outcome": decision } }))
   287	            .await
   288	    }
   289	
   290	    pub async fn respond_user_input(
   291	        &mut self,
   292	        request_id: &str,
   293	        values: BTreeMap<String, Value>,
   294	        cancelled: bool,
   295	    ) -> Result<(), AgentRuntimeError> {
   296	        self.respond(
   297	            request_id,
   298	            json!({ "values": values, "cancelled": cancelled }),
   299	        )
   300	        .await
   301	    }
   302	
   303	    pub async fn close(&mut self) -> Result<(), AgentRuntimeError> {
   304	        if let Some(session_id) = self.native_session_id.clone() {
   305	            let _ = self
   306	                .request("session/close", &acp::CloseSessionRequest::new(session_id))
   307	                .await;
   308	        }
   309	        self.process.stop().await;
   310	        self.native_session_id = None;
   311	        Ok(())
   312	    }
   313	
   314	    /// Detach the local transport while preserving the provider-native
   315	    /// session id. A later activation can resume the same session.
   316	    pub async fn detach(&mut self) -> Result<(), AgentRuntimeError> {
   317	        self.process.stop().await;
   318	        Ok(())
   319	    }
   320	
   321	    pub fn stderr_snapshot(&self) -> Vec<String> {
   322	        self.process.stderr_snapshot()
   323	    }
   324	    pub fn process_id(&self) -> Option<u32> {
   325	        self.process.process_id()
   326	    }
   327	
   328	    async fn start_session<T: Serialize>(
   329	        &mut self,
   330	        method: &str,
   331	        request: &T,
   332	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
   333	        let result = self.request(method, request).await?;
   334	        let native_session_id = result
   335	            .get("sessionId")
   336	            .or_else(|| result.get("session_id"))
   337	            .and_then(Value::as_str)
   338	            .ok_or_else(|| {
   339	                AgentRuntimeError::new(
   340	                    "invalid-response",
   341	                    "ACP session response did not include sessionId",
   342	                )
   343	            })?
   344	            .to_string();
   345	        self.native_session_id = Some(native_session_id.clone());
   346	        Ok(StartedAgentSession { native_session_id })
   347	    }
   348	
   349	    async fn request<T: Serialize>(
   350	        &mut self,
   351	        method: &str,
   352	        params: &T,
   353	    ) -> Result<Value, AgentRuntimeError> {
   354	        self.next_request_id = self.next_request_id.saturating_add(1);
   355	        let id = self.next_request_id;
   356	        let params = serde_json::to_value(params).map_err(serialization_error)?;
   357	        self.process
   358	            .write_json(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
   359	            .await
   360	            .map_err(transport_error)?;

--- providers adapter trait/mod ---
     1	pub mod acp;
     2	pub mod acp_client;
     3	mod adapter_support;
     4	pub mod claude;
     5	pub mod codex;
     6	pub mod process;
     7	
     8	pub use acp::{AcpRuntimeAdapter, StructuredRuntimeHandle};
     9	pub use acp_client::AcpClient;
    10	
    11	use std::path::PathBuf;
    12	use std::sync::Arc;
    13	
    14	use serde_json::Value;
    15	
    16	use super::capabilities::{validate_manifest, CLAUDE_AGENT_ACP_VERSION, CODEX_ACP_VERSION};
    17	use super::protocol::{
    18	    AgentApprovalResponse, AgentCapabilities, AgentConfigOption, AgentConversationProvider,
    19	    AgentProviderManifest, AgentUserInputResponse, ProviderSource, ProviderTransport,
    20	};
    21	
    22	#[derive(Clone, Debug)]
    23	pub struct InitializeAgentInput {
    24	    pub provider: AgentConversationProvider,
    25	    pub provider_instance_id: String,
    26	}
    27	
    28	#[derive(Clone, Debug)]
    29	pub struct NewAgentSession {
    30	    pub cwd: PathBuf,
    31	}
    32	
    33	#[derive(Clone, Debug)]
    34	pub struct LoadAgentSession {
    35	    pub cwd: PathBuf,
    36	    pub native_session_id: String,
    37	}
    38	
    39	pub type ResumeAgentSession = LoadAgentSession;
    40	
    41	#[derive(Clone, Debug, Eq, PartialEq)]
    42	pub struct AgentPromptImage {
    43	    pub data: String,
    44	    pub mime_type: String,
    45	}
    46	
    47	#[derive(Clone, Debug, Eq, PartialEq)]
    48	pub struct AgentPrompt {
    49	    pub text: String,
    50	    pub images: Vec<AgentPromptImage>,
    51	}
    52	
    53	#[derive(Clone, Debug)]
    54	pub struct AgentSteeringInput {
    55	    pub text: String,
    56	}
    57	
    58	pub type AgentConfigValue = Value;
    59	pub type PermissionResponse = AgentApprovalResponse;
    60	pub type UserInputResponse = AgentUserInputResponse;
    61	
    62	#[derive(Clone, Debug, Eq, PartialEq)]
    63	pub struct StartedAgentSession {
    64	    pub native_session_id: String,
    65	}
    66	
    67	#[derive(Clone, Debug, Eq, PartialEq)]
    68	pub struct StartedTurn {
    69	    pub turn_id: Option<String>,
    70	}
    71	
    72	#[derive(Clone, Debug, Eq, PartialEq)]
    73	pub struct GeneratedText {
    74	    pub turn_id: Option<String>,
    75	    pub text: String,
    76	}
    77	
    78	#[derive(Clone, Debug, Eq, PartialEq)]
    79	pub struct AgentRuntimeError {
    80	    pub code: &'static str,
    81	    pub message: String,
    82	}
    83	
    84	impl AgentRuntimeError {
    85	    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
    86	        Self {
    87	            code,
    88	            message: message.into(),
    89	        }
    90	    }
    91	}
    92	
    93	impl std::fmt::Display for AgentRuntimeError {
    94	    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    95	        write!(formatter, "{}: {}", self.code, self.message)
    96	    }
    97	}
    98	
    99	impl std::error::Error for AgentRuntimeError {}
   100	
   101	#[allow(async_fn_in_trait)]
   102	pub trait AgentRuntimeAdapter: Send + Sync {
   103	    async fn initialize(
   104	        &mut self,
   105	        input: InitializeAgentInput,
   106	    ) -> Result<AgentCapabilities, AgentRuntimeError>;
   107	    async fn new_session(
   108	        &mut self,
   109	        input: NewAgentSession,
   110	    ) -> Result<StartedAgentSession, AgentRuntimeError>;
   111	    async fn load_session(
   112	        &mut self,
   113	        input: LoadAgentSession,
   114	    ) -> Result<StartedAgentSession, AgentRuntimeError>;
   115	    async fn resume_session(
   116	        &mut self,
   117	        input: ResumeAgentSession,
   118	    ) -> Result<StartedAgentSession, AgentRuntimeError>;
   119	    async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError>;
   120	    async fn prompt_once(&mut self, input: AgentPrompt)
   121	        -> Result<GeneratedText, AgentRuntimeError>;
   122	    async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError>;
   123	    async fn cancel_turn(&mut self, turn_id: Option<&str>) -> Result<(), AgentRuntimeError>;
   124	    async fn set_config(
   125	        &mut self,
   126	        option_id: &str,
   127	        value: AgentConfigValue,
   128	    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError>;
   129	    async fn respond_permission(
   130	        &mut self,
   131	        input: PermissionResponse,
   132	    ) -> Result<(), AgentRuntimeError>;
   133	    async fn respond_user_input(
   134	        &mut self,
   135	        input: UserInputResponse,
   136	    ) -> Result<(), AgentRuntimeError>;
   137	    /// Stop the transport without sending the destructive session/close
   138	    /// request. The native session id remains valid for a later resume.
   139	    async fn detach_session(&mut self) -> Result<(), AgentRuntimeError>;
   140	    async fn close_session(&mut self) -> Result<(), AgentRuntimeError>;
   141	}
   142	
   143	#[derive(Clone, Default)]
   144	pub struct ProviderRegistry {
   145	    manifests: Arc<Vec<(AgentConversationProvider, AgentProviderManifest)>>,
   146	}
   147	
   148	impl ProviderRegistry {
   149	    pub fn bundled_from_environment() -> Result<Self, String> {
   150	        let codex_path = std::env::var_os("MCB_CODEX_ACP_PATH");
   151	        let codex_hash = std::env::var("MCB_CODEX_ACP_SHA256").ok();
   152	        let claude_path = std::env::var_os("MCB_CLAUDE_AGENT_ACP_PATH");
   153	        let claude_hash = std::env::var("MCB_CLAUDE_AGENT_ACP_SHA256").ok();
   154	        match (codex_path, codex_hash, claude_path, claude_hash) {
   155	            (None, None, None, None) => Ok(Self::default()),
   156	            (Some(codex_path), Some(codex_hash), Some(claude_path), Some(claude_hash)) => {
   157	                Self::initial_manifests(
   158	                    codex_path.into(),
   159	                    codex_hash,
   160	                    claude_path.into(),
   161	                    claude_hash,
   162	                )
   163	            }
   164	            _ => Err(
   165	                "Packaged ACP adapter paths and SHA-256 values must be configured together"
   166	                    .to_string(),
   167	            ),
   168	        }
   169	    }
   170	
   171	    pub fn new(
   172	        manifests: impl IntoIterator<Item = (AgentConversationProvider, AgentProviderManifest)>,
   173	    ) -> Result<Self, String> {
   174	        let mut values = Vec::new();
   175	        for (provider, manifest) in manifests {
   176	            validate_manifest(&manifest)?;
   177	            if values.iter().any(|(current, _)| *current == provider) {
   178	                return Err("Provider manifests must have one entry per provider".to_string());
   179	            }
   180	            values.push((provider, manifest));
   181	        }
   182	        Ok(Self {
   183	            manifests: Arc::new(values),
   184	        })
   185	    }
   186	
   187	    pub fn initial_manifests(
   188	        codex_executable: PathBuf,
   189	        codex_hash: String,
   190	        claude_executable: PathBuf,
   191	        claude_hash: String,
   192	    ) -> Result<Self, String> {
   193	        Self::new([
   194	            (
   195	                AgentConversationProvider::Codex,
   196	                AgentProviderManifest {
   197	                    id: "codex-acp".into(),
   198	                    display_name: "Codex".into(),
   199	                    transport: ProviderTransport::AcpStdio,
   200	                    executable: codex_executable,
   201	                    args: Vec::new(),
   202	                    version: CODEX_ACP_VERSION.into(),
   203	                    content_hash: codex_hash,
   204	                    trusted_source: ProviderSource::Bundled,
   205	                },
   206	            ),
   207	            (
   208	                AgentConversationProvider::Claude,
   209	                AgentProviderManifest {
   210	                    id: "claude-agent-acp".into(),
   211	                    display_name: "Claude".into(),
   212	                    transport: ProviderTransport::AcpStdio,
   213	                    executable: claude_executable,
   214	                    args: Vec::new(),
   215	                    version: CLAUDE_AGENT_ACP_VERSION.into(),
   216	                    content_hash: claude_hash,
   217	                    trusted_source: ProviderSource::Bundled,
   218	                },
   219	            ),
   220	        ])
   221	    }
   222	
   223	    pub fn manifest(
   224	        &self,
   225	        provider: AgentConversationProvider,
   226	    ) -> Result<AgentProviderManifest, String> {
   227	        self.manifests
   228	            .iter()
   229	            .find(|(current, _)| *current == provider)
   230	            .map(|(_, manifest)| manifest.clone())
   231	            .ok_or_else(|| "No trusted adapter is registered for this provider".to_string())
   232	    }
   233	
   234	    pub fn manifests(&self) -> Vec<AgentProviderManifest> {
   235	        self.manifests
   236	            .iter()
   237	            .map(|(_, manifest)| manifest.clone())
   238	            .collect()
   239	    }
   240	}
   241	
   242	#[cfg(test)]
   243	mod tests {
   244	    use super::*;
   245	
   246	    #[test]
   247	    fn initial_provider_manifests_pin_both_official_adapters() {
   248	        let hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
   249	        let registry = ProviderRegistry::initial_manifests(
   250	            PathBuf::from("/bin/sh"),
   251	            hash.clone(),
   252	            PathBuf::from("/bin/sh"),
   253	            hash,
   254	        )
   255	        .unwrap();
   256	        let codex = registry.manifest(AgentConversationProvider::Codex).unwrap();
   257	        let claude = registry
   258	            .manifest(AgentConversationProvider::Claude)
   259	            .unwrap();
   260	        assert_eq!(
   261	            (codex.id.as_str(), codex.version.as_str()),
   262	            ("codex-acp", CODEX_ACP_VERSION)
   263	        );
   264	        assert_eq!(
   265	            (claude.id.as_str(), claude.version.as_str()),
   266	            ("claude-agent-acp", CLAUDE_AGENT_ACP_VERSION)
   267	        );
   268	        assert!(registry
   269	            .manifests()
   270	            .iter()
   271	            .all(|manifest| !manifest.args.iter().any(|arg| arg.contains("@latest"))));
   272	    }
   273	}

--- manager turn runtime event loops ---
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-8-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-9-use std::collections::{BTreeMap, HashMap, HashSet};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-10-use std::time::{SystemTime, UNIX_EPOCH};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-11-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-12-use serde_json::{json, Value};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-13-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-14-use super::super::protocol::{
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-15-    AgentCapabilities, AgentCommandDescriptor, AgentConfigOption, AgentConfigOptionChoice,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:16:    AgentContent, AgentContentChannel, AgentConversationProvider, AgentEvent, AgentEventType,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-17-    AgentImplementation, AgentInteractionCapabilities, AgentItem, AgentItemType,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-18-    AgentPromptCapabilities, AgentRawFrameReference, AgentSessionCapabilities,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-19-};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-20-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-21-/// The context supplied by the manager for one provider-owned stream.
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-22-#[derive(Clone, Debug, Eq, PartialEq)]
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-23-pub struct AdapterContext {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-24-    pub owned_id: String,
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-50-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-51-}
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-52-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-53-/// One ordered projection result.  Events are already sequenced; `items` contains the item
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-54-/// snapshots changed by those events in the same order.  A caller can append the snapshots to
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-55-/// its timeline without sorting by type or by provider-specific IDs.
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-56-#[derive(Clone, Debug, PartialEq)]
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-57-pub struct ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:58:    pub events: Vec<AgentEvent>,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-59-    pub items: Vec<AgentItem>,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-60-}
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-61-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-62-impl ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-63-    fn new() -> Self {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-64-        Self {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-65-            events: Vec::new(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-66-            items: Vec::new(),
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-231-                self.ensure_item(&item_id, AgentItemType::UserMessage, turn_id.clone(), None);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-232-                if let Some(turn_id) = turn_id.clone() {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-233-                    self.active_turn_id = Some(turn_id);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-234-                }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-235-                let payload = prompt_payload(params);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-236-                self.touch_item_metadata(&item_id, json!({ "prompt": payload.clone() }));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-237-                self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-238-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:239:                    AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-240-                    &item_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-241-                    turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-242-                    json!({ "item": payload, "source": "session/prompt" }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-243-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-244-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-245-            "session/cancel" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-246-                let turn_id = string_at(params, &["turnId", "turn_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-247-                    .or_else(|| self.active_turn_id.clone())
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-311-                    self.context.native_session_id = Some(session_id.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-312-                    let payload = json!({
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-313-                        "operation": pending.method,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-314-                        "nativeSessionId": session_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-315-                        "result": result,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-316-                    });
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-317-                    self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-318-                        &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:319:                        AgentEventType::SessionStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-320-                        None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-321-                        None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-322-                        None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-323-                        payload,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-324-                        frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-325-                    );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-326-                } else {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-327-                    batch.extend(self.warning(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-328-                        "session-response-missing-id",
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-329-                        json!({ "operation": pending.method, "result": result }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-330-                    ));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-331-                }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-332-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-333-            "session/close" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-334-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-335-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:336:                    AgentEventType::SessionClosed,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-337-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-338-                    None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-339-                    None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-340-                    json!({ "operation": pending.method, "result": result }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-341-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-342-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-343-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-344-            "session/prompt" => {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-369-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-370-            "session/set_config_option" | "session/set_mode" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-371-                self.update_session_capabilities(&result);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-372-                batch.extend(self.session_config_event(&result, frame));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-373-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-374-            "session/cancel" | "session/steer" => {}
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-375-            "session/request_permission" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-376-                batch.extend(self.request_resolved(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:377:                    AgentEventType::ApprovalResolved,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-378-                    request_id.as_deref(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-379-                    &result,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-380-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-381-                ));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-382-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-383-            "session/request_user_input" | "session/request_input" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-384-                batch.extend(self.request_resolved(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:385:                    AgentEventType::UserInputResolved,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-386-                    request_id.as_deref(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-387-                    &result,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-388-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-389-                ));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-390-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-391-            method
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-392-                if method.starts_with("terminal/")
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-393-                    || method.starts_with("fs/")
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-424-            );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-425-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-426-        match method {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-427-            "session/request_permission" | "session/permission_request" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-428-                let request_id = request_id.unwrap_or("unknown-request");
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-429-                let tool_call = params.get("toolCall").or_else(|| params.get("tool_call"));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-430-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-431-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:432:                    AgentEventType::ApprovalRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-433-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-434-                    string_at(params, &["turnId", "turn_id"]),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-435-                    string_at(params, &["toolCallId", "tool_call_id"]).or_else(|| {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-436-                        tool_call.and_then(|value| {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-437-                            string_at(value, &["toolCallId", "tool_call_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-438-                        })
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-439-                    }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-440-                    json!({
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-448-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-449-                if let Some(event) = batch.events.last_mut() {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-450-                    event.request_id = Some(request_id.to_string());
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-451-                }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-452-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-453-            "session/request_user_input" | "session/request_input" | "session/elicit" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-454-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-455-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:456:                    AgentEventType::UserInputRequested,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-457-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-458-                    string_at(params, &["turnId", "turn_id"]),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-459-                    string_at(params, &["itemId", "item_id"]),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-460-                    json!({
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-461-                        "requestId": request_id.unwrap_or("unknown-request"),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-462-                        "params": params,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-463-                    }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-464-                    frame,
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-532-                    .get("currentModeId")
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-533-                    .or_else(|| update.get("current_mode_id"))
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-534-                    .cloned()
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-535-                    .unwrap_or(Value::Null);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-536-                let turn_id = string_at(update, &["turnId", "turn_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-537-                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-538-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-539-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:540:                    AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-541-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-542-                    turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-543-                    None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-544-                    json!({ "mode": mode, "update": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-545-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-546-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-547-                batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-548-            }
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-550-                self.map_config_update(update, frame)
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-551-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-552-            "session_info_update" | "session_state_changed" | "state_changed" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-553-                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-554-                let turn_id = string_at(update, &["turnId", "turn_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-555-                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-556-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-557-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:558:                    AgentEventType::SessionStateChanged,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-559-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-560-                    turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-561-                    None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-562-                    json!({ "update": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-563-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-564-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-565-                batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-566-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-567-            "usage_update" | "usage_updated" | "usage" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-568-                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-569-                let turn_id = string_at(update, &["turnId", "turn_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-570-                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-571-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-572-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:573:                    AgentEventType::UsageUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-574-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-575-                    turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-576-                    None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-577-                    json!({ "usage": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-578-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-579-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-580-                batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-581-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-582-            "rate_limits_update" | "rate_limits_updated" | "rate_limit" => {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-583-                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-584-                let turn_id = string_at(update, &["turnId", "turn_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-585-                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-586-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-587-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:588:                    AgentEventType::RateLimitsUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-589-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-590-                    turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-591-                    None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-592-                    json!({ "rateLimits": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-593-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-594-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-595-                batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-596-            }
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-619-        let item_id = string_at(update, &["messageId", "message_id", "itemId", "item_id"])
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-620-            .or_else(|| string_at(params, &["messageId", "message_id", "itemId", "item_id"]))
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-621-            .unwrap_or_else(|| format!("{}-item-{}", self.namespace, self.sequence + 1));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-622-        let new_item = !self.item_index.contains_key(&item_id);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-623-        self.ensure_item(&item_id, item_type, turn_id.clone(), Some(update.clone()));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-624-        if new_item {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-625-            self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-626-                &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:627:                AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-628-                &item_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-629-                turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-630-                json!({ "item": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-631-            );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-632-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-633-        let content = update
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-634-            .get("content")
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-635-            .or_else(|| update.get("delta"))
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-665-            } else {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-666-                self.touch_item_metadata(&item_id, json!({ "content": block }));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-667-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-668-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-669-        let delta = text_values.concat();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-670-        if !delta.is_empty() {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-671-            self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-672-                &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:673:                AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-674-                self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-675-                turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-676-                Some(item_id.clone()),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-677-                json!({ "channel": serde_json::to_value(channel).unwrap_or(Value::Null), "text": delta }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-678-                frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-679-            );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-680-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-681-        let event_type = if new_item {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:682:            AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-683-        } else {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:684:            AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-685-        };
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-686-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-687-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-688-            event_type,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-689-            &item_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-690-            turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-691-            json!({ "item": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-692-        );
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-703-            .or_else(|| self.active_turn_id.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-704-        if let Some(turn_id) = turn_id.clone() {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-705-            self.active_turn_id = Some(turn_id);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-706-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-707-        let item_type = item_type_for_tool(update);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-708-        self.ensure_item(&tool_id, item_type, turn_id.clone(), Some(update.clone()));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-709-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-710-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:711:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-712-            &tool_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-713-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-714-            json!({ "item": update, "toolCallId": tool_id }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-715-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-716-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-717-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:718:            AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-719-            &tool_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-720-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-721-            json!({ "item": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-722-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-723-        let maps_followup = update.get("content").is_some() || update.get("status").is_some();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-724-        if maps_followup {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-725-            batch.extend(self.map_tool_update(update, params, frame));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-726-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-727-        if !maps_followup && is_subagent_update(update) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-728-            self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-729-                &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:730:                AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-731-                self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-732-                turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-733-                Some(tool_id),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-734-                json!({ "child": child_identity(update) }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-735-                frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-736-            );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-737-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-738-        batch
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-752-            .or_else(|| string_at(params, &["turnId", "turn_id"]))
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-753-            .or_else(|| self.active_turn_id.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-754-        let item_type = item_type_for_tool(update);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-755-        let new_item = !self.item_index.contains_key(&tool_id);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-756-        self.ensure_item(&tool_id, item_type, turn_id.clone(), Some(update.clone()));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-757-        if new_item {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-758-            self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-759-                &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:760:                AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-761-                &tool_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-762-                turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-763-                json!({ "item": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-764-            );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-765-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-766-        let content = update.get("content").cloned().unwrap_or(Value::Null);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-767-        let mut file_index = 0usize;
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-768-        for block in content_blocks(&content) {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-775-                    &file_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-776-                    AgentItemType::FileChange,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-777-                    turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-778-                    Some(json!({ self.namespace.clone() + ".fileChange": block.clone() })),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-779-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-780-                self.touch_item_metadata(&file_id, json!({ "fileChange": block.clone() }));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-781-                self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-782-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:783:                    AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-784-                    &file_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-785-                    turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-786-                    json!({ "item": block, "parentToolCallId": tool_id }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-787-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-788-                self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-789-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:790:                    AgentEventType::ItemCompleted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-791-                    &file_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-792-                    turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-793-                    json!({ "item": block, "parentToolCallId": tool_id }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-794-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-795-            } else if let Some(text) = tool_output_text(&block) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-796-                self.append_content(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-797-                    &tool_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-798-                    AgentContent {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-802-                            .get("mimeType")
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-803-                            .or_else(|| block.get("mime_type"))
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-804-                            .and_then(Value::as_str)
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-805-                            .map(str::to_string),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-806-                    },
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-807-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-808-                self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-809-                    &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:810:                    AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-811-                    self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-812-                    turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-813-                    Some(tool_id.clone()),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-814-                    json!({ "channel": "command-output", "text": text }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-815-                    frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-816-                );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-817-            } else {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-818-                self.touch_item_metadata(&tool_id, json!({ "toolContent": block }));
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-822-            string_at(update, &["status", "state"]).unwrap_or_else(|| "updated".to_string());
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-823-        let completed = matches!(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-824-            status.as_str(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-825-            "completed" | "complete" | "failed" | "cancelled" | "canceled"
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-826-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-827-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-828-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-829-            if completed {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:830:                AgentEventType::ItemCompleted
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-831-            } else {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:832:                AgentEventType::ItemUpdated
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-833-            },
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-834-            &tool_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-835-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-836-            json!({ "item": update, "status": status }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-837-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-838-        if is_subagent_update(update) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-839-            self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-840-                &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:841:                AgentEventType::ChildrenUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-842-                self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-843-                turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-844-                Some(tool_id),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-845-                json!({ "child": child_identity(update) }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-846-                frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-847-            );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-848-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-849-        batch
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-860-            &id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-861-            AgentItemType::Plan,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-862-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-863-            Some(json!({ self.namespace.clone() + ".plan": update.clone() })),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-864-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-865-        self.touch_item_metadata(&id, json!({ "plan": update }));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-866-        self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-867-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:868:            AgentEventType::PlanUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-869-            self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-870-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-871-            Some(id.clone()),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-872-            json!({ "plan": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-873-            frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-874-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-875-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-876-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:877:            AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-878-            &id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-879-            turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-880-            json!({ "item": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-881-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-882-        batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-883-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-884-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-885-    fn map_tasks(&mut self, update: &Value, params: &Value, _frame: &Value) -> ProjectionBatch {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-896-            &id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-897-            AgentItemType::TaskList,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-898-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-899-            Some(json!({ self.namespace.clone() + ".tasks": update.clone() })),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-900-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-901-        self.touch_item_metadata(&id, json!({ "tasks": update }));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-902-        self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-903-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:904:            AgentEventType::TasksUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-905-            self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-906-            turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-907-            Some(id.clone()),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-908-            json!({ "tasks": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-909-            _frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-910-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-911-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-912-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:913:            AgentEventType::ItemUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-914-            &id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-915-            turn_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-916-            json!({ "item": update }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-917-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-918-        batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-919-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-920-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-921-    fn map_commands(&mut self, update: &Value, frame: &Value) -> ProjectionBatch {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-925-            .or_else(|| update.get("commands"))
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-926-            .cloned()
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-927-            .unwrap_or_else(|| json!([]));
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-928-        self.capabilities.commands = parse_commands(&commands_value, &self.namespace);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-929-        self.capabilities.revision = self.capabilities.revision.saturating_add(1);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-930-        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-931-        self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-932-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:933:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-934-            self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-935-            None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-936-            None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-937-            json!({ "commands": commands_value }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-938-            frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-939-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-940-        batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-941-    }
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-951-        self.capabilities.revision = self.capabilities.revision.saturating_add(1);
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-952-        self.session_config_event(update, frame)
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-953-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-954-
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-955-    fn session_config_event(&mut self, value: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-956-        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-957-        self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-958-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:959:            AgentEventType::SessionConfigUpdated,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-960-            self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-961-            None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-962-            None,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-963-            json!({ "config": value }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-964-            frame,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-965-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-966-        batch
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-967-    }
--
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1017-        self.ensure_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1018-            &id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1019-            item_type,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1020-            self.active_turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1021-            Some(params.clone()),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1022-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1023-        self.emit_item(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1024-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1025:            AgentEventType::ItemStarted,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1026-            &id,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1027-            self.active_turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1028-            json!({ "method": method, "params": params }),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1029-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1030-        self.emit_event(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1031-            &mut batch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1032:            AgentEventType::ContentDelta,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1033-            self.native_session_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1034-            self.active_turn_id.clone(),
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs-1035-            Some(id),

exec
/bin/zsh -lc "rg -n \"ProjectionBatch|next_json|next_frame|consume|handle_update|dispatch|process_event|poll_events|emit_payload\\(\" tauri-svelte-preview/src-tauri/src/agent_conversation/providers tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs | sed -n '1,260p'
nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs | sed -n '340,520p'
nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs | sed -n '1,100p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265:    pub fn emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052:            .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1078:                .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:156:    /// Git actions need a direct result, so this narrow path consumes the same
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:185:            let frame = self.process.next_json().await.map_err(transport_error)?;
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:362:            let frame = self.process.next_json().await.map_err(transport_error)?;
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/process.rs:126:    pub async fn next_json(&mut self) -> Result<serde_json::Value, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:3://! Codex ACP is consumed as structured JSON-RPC over the A2 piped transport.  This adapter is
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:12:pub use super::adapter_support::{AdapterContext, ProjectionBatch};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:45:    pub fn map_frame(&mut self, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:49:    pub fn map_jsonl(&mut self, input: &str) -> Vec<ProjectionBatch> {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/codex.rs:69:pub fn map_codex_frame(context: AdapterContext, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:57:pub struct ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:62:impl ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:70:    fn extend(&mut self, other: ProjectionBatch) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:136:    pub fn map_frame(&mut self, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:137:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:187:    pub fn map_jsonl(&mut self, input: &str) -> Vec<ProjectionBatch> {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:223:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:224:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:273:    fn map_response(&mut self, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:294:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:415:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:416:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:482:    fn map_update(&mut self, update: &Value, params: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:530:                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:553:                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:568:                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:583:                let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:608:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:609:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:696:    fn map_tool_call(&mut self, update: &Value, params: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:697:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:746:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:747:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:852:    fn map_plan(&mut self, update: &Value, params: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:853:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:885:    fn map_tasks(&mut self, update: &Value, params: &Value, _frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:886:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:921:    fn map_commands(&mut self, update: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:930:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:943:    fn map_config_update(&mut self, update: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:955:    fn session_config_event(&mut self, value: &Value, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:956:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:990:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:991:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1053:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1054:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1083:    ) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1084:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1100:    fn start_turn(&mut self, batch: &mut ProjectionBatch, turn_id: String, frame: &Value) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1107:        batch: &mut ProjectionBatch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1128:    fn finish_active_items(&mut self, batch: &mut ProjectionBatch, interrupted: bool) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1214:        batch: &mut ProjectionBatch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1240:        batch: &mut ProjectionBatch,
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1288:    fn unknown(&mut self, frame: &Value, category: &str) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1289:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1307:    fn warning(&mut self, category: &str, payload: Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1308:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1321:    fn error_from_frame(&mut self, frame: &Value, operation: &str) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1322:        let mut batch = ProjectionBatch::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/adapter_support.rs:1741:fn set_request_id(batch: &mut ProjectionBatch, request_id: Option<&str>) {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:12:pub use super::adapter_support::{AdapterContext, ProjectionBatch};
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:45:    pub fn map_frame(&mut self, frame: &Value) -> ProjectionBatch {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:49:    pub fn map_jsonl(&mut self, input: &str) -> Vec<ProjectionBatch> {
tauri-svelte-preview/src-tauri/src/agent_conversation/providers/claude.rs:69:pub fn map_claude_frame(context: AdapterContext, frame: &Value) -> ProjectionBatch {
   340	                    "invalid-response",
   341	                    "ACP session response did not include sessionId",
   342	                )
   343	            })?
   344	            .to_string();
   345	        self.native_session_id = Some(native_session_id.clone());
   346	        Ok(StartedAgentSession { native_session_id })
   347	    }
   348	
   349	    async fn request<T: Serialize>(
   350	        &mut self,
   351	        method: &str,
   352	        params: &T,
   353	    ) -> Result<Value, AgentRuntimeError> {
   354	        self.next_request_id = self.next_request_id.saturating_add(1);
   355	        let id = self.next_request_id;
   356	        let params = serde_json::to_value(params).map_err(serialization_error)?;
   357	        self.process
   358	            .write_json(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
   359	            .await
   360	            .map_err(transport_error)?;
   361	        loop {
   362	            let frame = self.process.next_json().await.map_err(transport_error)?;
   363	            if frame.get("id") != Some(&json!(id)) {
   364	                continue;
   365	            }
   366	            if let Some(error) = frame.get("error") {
   367	                return Err(AgentRuntimeError::new("acp-error", error.to_string()));
   368	            }
   369	            return Ok(frame.get("result").cloned().unwrap_or(Value::Null));
   370	        }
   371	    }
   372	
   373	    async fn respond(&mut self, request_id: &str, result: Value) -> Result<(), AgentRuntimeError> {
   374	        let id = serde_json::from_str::<Value>(request_id)
   375	            .unwrap_or_else(|_| Value::String(request_id.to_string()));
   376	        self.process
   377	            .write_json(&json!({ "jsonrpc": "2.0", "id": id, "result": result }))
   378	            .await
   379	            .map_err(transport_error)
   380	    }
   381	
   382	    fn session_id(&self) -> Result<String, AgentRuntimeError> {
   383	        self.native_session_id.clone().ok_or_else(|| {
   384	            AgentRuntimeError::new("session-not-started", "ACP session has not started")
   385	        })
   386	    }
   387	}
   388	
   389	fn extract_text_value(value: &Value) -> Option<String> {
   390	    match value {
   391	        Value::String(text) => Some(text.clone()),
   392	        Value::Array(values) => {
   393	            let text = values
   394	                .iter()
   395	                .filter_map(extract_text_value)
   396	                .collect::<String>();
   397	            (!text.is_empty()).then_some(text)
   398	        }
   399	        Value::Object(object) => {
   400	            if object.get("type").and_then(Value::as_str) == Some("text") {
   401	                return object
   402	                    .get("text")
   403	                    .and_then(Value::as_str)
   404	                    .map(str::to_string);
   405	            }
   406	            object
   407	                .get("text")
   408	                .and_then(Value::as_str)
   409	                .map(str::to_string)
   410	                .or_else(|| object.get("content").and_then(extract_text_value))
   411	        }
   412	        _ => None,
   413	    }
   414	}
   415	
   416	fn bool_at(value: &Value, path: &[&str]) -> bool {
   417	    path.iter()
   418	        .try_fold(value, |current, part| current.get(*part))
   419	        .and_then(Value::as_bool)
   420	        .unwrap_or(false)
   421	}
   422	
   423	fn parse_config_options(value: Option<&Value>) -> Vec<AgentConfigOption> {
   424	    value
   425	        .and_then(|value| serde_json::from_value(value.clone()).ok())
   426	        .unwrap_or_default()
   427	}
   428	
   429	fn serialization_error(error: serde_json::Error) -> AgentRuntimeError {
   430	    AgentRuntimeError::new("serialization", error.to_string())
   431	}
   432	fn transport_error(message: String) -> AgentRuntimeError {
   433	    AgentRuntimeError::new("transport", message)
   434	}
   435	
   436	#[cfg(test)]
   437	mod tests {
   438	    use std::collections::BTreeMap;
   439	    use std::path::PathBuf;
   440	    use std::time::{Duration, Instant};
   441	
   442	    use serde_json::json;
   443	
   444	    use super::*;
   445	    use crate::agent_conversation::protocol::{
   446	        AgentApprovalDecision, AgentApprovalResponse, AgentRequestIdentity, AgentUserInputResponse,
   447	        ProviderSource, ProviderTransport,
   448	    };
   449	    use crate::agent_conversation::providers::AgentRuntimeAdapter;
   450	
   451	    fn fixture_manifest(log_path: &Path) -> AgentProviderManifest {
   452	        let script = format!(
   453	            r#"log={log}
   454	while IFS= read -r line; do
   455	  printf '%s\n' "$line" >> "$log"
   456	  id=$(printf '%s\n' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
   457	  case "$line" in
   458	    *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"agentInfo":{{"name":"fake-acp","version":"1"}},"agentCapabilities":{{"loadSession":true,"promptCapabilities":{{"image":true}}}}}}}}\n' "$id" ;;
   459	    *'"method":"session/new"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session"}}}}\n' "$id" ;;
   460	    *'"method":"session/load"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"loaded-session"}}}}\n' "$id" ;;
   461	    *'"method":"session/resume"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"resumed-session"}}}}\n' "$id" ;;
   462	    *'"method":"session/prompt"'*) printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"generated text"}},"turnId":"turn-1"}}}}}}\n'; printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id" ;;
   463	    *'"method":"session/set_config_option"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"configOptions":[{{"id":"model","label":"Model","category":"model","value":"new"}}]}}}}\n' "$id" ;;
   464	    *'"method":"session/steer"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id" ;;
   465	    *'"method":"session/close"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id"; exit 0 ;;
   466	  esac
   467	done"#,
   468	            log = shell_quote(log_path)
   469	        );
   470	        AgentProviderManifest {
   471	            id: "fake-acp".into(),
   472	            display_name: "Fake ACP".into(),
   473	            transport: ProviderTransport::AcpStdio,
   474	            executable: PathBuf::from("/bin/sh"),
   475	            args: vec!["-c".into(), script],
   476	            version: "1.0.0".into(),
   477	            content_hash: "0000000000000000000000000000000000000000000000000000000000000000".into(),
   478	            trusted_source: ProviderSource::Bundled,
   479	        }
   480	    }
   481	
   482	    fn shell_quote(path: &Path) -> String {
   483	        format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
   484	    }
   485	
   486	    fn fixture_root() -> PathBuf {
   487	        let root = std::env::temp_dir().join(format!("mcb-fake-acp-{}", uuid::Uuid::new_v4()));
   488	        std::fs::create_dir_all(&root).unwrap();
   489	        root
   490	    }
   491	
   492	    fn identity(request_id: &str) -> AgentRequestIdentity {
   493	        AgentRequestIdentity {
   494	            owned_id: "owned-a".into(),
   495	            generation: 1,
   496	            request_id: request_id.into(),
   497	            turn_id: Some("turn-1".into()),
   498	            item_id: None,
   499	        }
   500	    }
   501	
   502	    #[tokio::test(flavor = "current_thread")]
   503	    async fn acp_initialize_new_prompt_image_config_correlations_cancel_and_close() {
   504	        let root = fixture_root();
   505	        let log = root.join("frames.jsonl");
   506	        let mut adapter = super::super::AcpRuntimeAdapter::new(
   507	            fixture_manifest(&log),
   508	            "owned-a".into(),
   509	            root.clone(),
   510	        );
   511	        let capabilities = adapter
   512	            .initialize(super::super::InitializeAgentInput {
   513	                provider: AgentConversationProvider::Codex,
   514	                provider_instance_id: "fake-1".into(),
   515	            })
   516	            .await
   517	            .unwrap();
   518	        assert!(capabilities.prompt.image);
   519	        assert_eq!(
   520	            adapter
     1	use super::super::capabilities::{replace_config_options, validate_capabilities};
     2	use super::super::protocol::{AgentCapabilities, AgentProviderManifest};
     3	use super::{
     4	    AcpClient, AgentConfigOption, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter,
     5	    AgentRuntimeError, AgentSteeringInput, GeneratedText, InitializeAgentInput, LoadAgentSession,
     6	    NewAgentSession, PermissionResponse, ResumeAgentSession, StartedAgentSession, StartedTurn,
     7	    UserInputResponse,
     8	};
     9	
    10	pub struct AcpRuntimeAdapter {
    11	    manifest: AgentProviderManifest,
    12	    owned_id: String,
    13	    cwd: std::path::PathBuf,
    14	    client: Option<AcpClient>,
    15	    capabilities: Option<AgentCapabilities>,
    16	}
    17	
    18	impl AcpRuntimeAdapter {
    19	    pub fn new(manifest: AgentProviderManifest, owned_id: String, cwd: std::path::PathBuf) -> Self {
    20	        Self {
    21	            manifest,
    22	            owned_id,
    23	            cwd,
    24	            client: None,
    25	            capabilities: None,
    26	        }
    27	    }
    28	
    29	    fn client_mut(&mut self) -> Result<&mut AcpClient, AgentRuntimeError> {
    30	        self.client.as_mut().ok_or_else(|| {
    31	            AgentRuntimeError::new("not-initialized", "ACP adapter is not initialized")
    32	        })
    33	    }
    34	
    35	    pub fn process_id(&self) -> Option<u32> {
    36	        self.client.as_ref().and_then(AcpClient::process_id)
    37	    }
    38	}
    39	
    40	impl AgentRuntimeAdapter for AcpRuntimeAdapter {
    41	    async fn initialize(
    42	        &mut self,
    43	        input: InitializeAgentInput,
    44	    ) -> Result<AgentCapabilities, AgentRuntimeError> {
    45	        let mut client = AcpClient::spawn(&self.manifest, &self.cwd, &self.owned_id)?;
    46	        let capabilities = client.initialize(input.provider).await?;
    47	        validate_capabilities(&capabilities)
    48	            .map_err(|message| AgentRuntimeError::new("invalid-capabilities", message))?;
    49	        self.client = Some(client);
    50	        self.capabilities = Some(capabilities.clone());
    51	        Ok(capabilities)
    52	    }
    53	
    54	    async fn new_session(
    55	        &mut self,
    56	        input: NewAgentSession,
    57	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    58	        self.client_mut()?.new_session(&input.cwd).await
    59	    }
    60	    async fn load_session(
    61	        &mut self,
    62	        input: LoadAgentSession,
    63	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    64	        self.client_mut()?
    65	            .load_session(&input.cwd, &input.native_session_id)
    66	            .await
    67	    }
    68	    async fn resume_session(
    69	        &mut self,
    70	        input: ResumeAgentSession,
    71	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    72	        self.client_mut()?
    73	            .resume_session(&input.cwd, &input.native_session_id)
    74	            .await
    75	    }
    76	    async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
    77	        self.client_mut()?.prompt(input).await
    78	    }
    79	
    80	    async fn prompt_once(
    81	        &mut self,
    82	        input: AgentPrompt,
    83	    ) -> Result<GeneratedText, AgentRuntimeError> {
    84	        self.client_mut()?.prompt_once(input).await
    85	    }
    86	    async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
    87	        self.client_mut()?.steer(input.text).await
    88	    }
    89	    async fn cancel_turn(&mut self, _turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
    90	        self.client_mut()?.cancel().await
    91	    }
    92	
    93	    async fn set_config(
    94	        &mut self,
    95	        option_id: &str,
    96	        value: AgentConfigValue,
    97	    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
    98	        let replacement = self.client_mut()?.set_config(option_id, value).await?;
    99	        if let Some(capabilities) = &mut self.capabilities {
   100	            replace_config_options(capabilities, replacement.clone())

exec
/bin/zsh -lc "rg -n \"startConversationEvents|ensureConversationSession|setConversationMode|conversationProviderFor|mode\" tauri-svelte-preview/src/routes/next/+page.svelte tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts | sed -n '1,220p'
nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts | sed -n '70,260p;480,620p'
nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '1380,1530p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:40:  mode: ConversationViewMode;
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:69:  model: null,
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:85:    mode: 'structured',
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:116:export function ensureConversationSession(
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:129:  const current = ensureConversationSession(event.ownedId, event.provider);
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:135:    mode: current.mode,
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:174:  const current = ensureConversationSession(event.ownedId, event.provider);
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:225:  const current = ensureConversationSession(
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:241:    mode: current.mode,
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:382:  const current = ensureConversationSession(ownedId, provider);
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:554:export function setConversationMode(ownedId: string, mode: ConversationViewMode): void {
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:556:  if (!current || current.mode === mode) return;
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:557:  current.mode = mode;
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:567:  const current = ensureConversationSession(connection.ownedId, connection.provider);
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:581:    mode: current.mode,
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:605:  const current = ensureConversationSession(ownedId, provider);
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:606:  current.mode = snapshot?.mode === 'raw' ? 'raw' : 'structured';
tauri-svelte-preview/src/routes/next/+page.svelte:88:    setConversationMode
tauri-svelte-preview/src/routes/next/+page.svelte:96:    startConversationEvents,
tauri-svelte-preview/src/routes/next/+page.svelte:393:  function browserPresentation(mode: BrowserPresentationMode): void {
tauri-svelte-preview/src/routes/next/+page.svelte:395:      if (mode === 'docked') restoreBrowserToDock(browserModelContext());
tauri-svelte-preview/src/routes/next/+page.svelte:396:      else if (mode === 'floating') expandBrowserFrom(browserModelContext(), 'docked');
tauri-svelte-preview/src/routes/next/+page.svelte:397:      else setBrowserPresentation(mode);
tauri-svelte-preview/src/routes/next/+page.svelte:445:    // The model can hold a deterministic preview capture, but it cannot claim
tauri-svelte-preview/src/routes/next/+page.svelte:863:  function conversationProviderFor(ownedId: string): AgentConversationProvider | null {
tauri-svelte-preview/src/routes/next/+page.svelte:929:    const conversationProvider = conversationProviderFor(ownedId);
tauri-svelte-preview/src/routes/next/+page.svelte:983:    const provider = conversationProviderFor(ownedId);
tauri-svelte-preview/src/routes/next/+page.svelte:991:        setConversationMode(ownedId, 'raw');
tauri-svelte-preview/src/routes/next/+page.svelte:1008:    mode: AgentConversationHandoffMode
tauri-svelte-preview/src/routes/next/+page.svelte:1021:      mode,
tauri-svelte-preview/src/routes/next/+page.svelte:1023:      targetOwnedId: mode === 'fork' ? `${ownedId}:native:${Date.now()}` : null,
tauri-svelte-preview/src/routes/next/+page.svelte:1049:    mode: AgentConversationHandoffMode
tauri-svelte-preview/src/routes/next/+page.svelte:1051:    const input = handoffInput(ownedId, 'structured-to-terminal', mode);
tauri-svelte-preview/src/routes/next/+page.svelte:1055:      if (mode === 'same-session' && receipt.nativeSessionId) {
tauri-svelte-preview/src/routes/next/+page.svelte:1056:        const provider = conversationProviderFor(ownedId);
tauri-svelte-preview/src/routes/next/+page.svelte:1088:      if (receipt.owner === 'structured') setConversationMode(ownedId, 'structured');
tauri-svelte-preview/src/routes/next/+page.svelte:1393:    void startConversationEvents();
    70	  effort: null,
    71	  approvalPolicy: null,
    72	  usedTokens: null,
    73	  contextWindow: null
    74	});
    75	
    76	export const conversationSessions = $state<Record<string, ConversationWorkspaceState>>({});
    77	
    78	function freshState(
    79	  ownedId: string,
    80	  provider: AgentConversationProvider
    81	): ConversationWorkspaceState {
    82	  return {
    83	    ...createConversationState(ownedId, provider),
    84	    draft: '',
    85	    mode: 'structured',
    86	    sending: false,
    87	    attachments: [],
    88	    metadata: emptyMetadata(),
    89	    children: [],
    90	    selectedChildId: null,
    91	    childTimeline: [],
    92	    scrollTop: 0,
    93	    childScrollTopById: {},
    94	    executionOwner: 'stopped',
    95	    writerLease: { ownedId, generation: 0, owner: 'none' },
    96	    writerLeaseTransition: null,
    97	    attachmentIds: [],
    98	    config: {},
    99	    telemetry: {},
   100	    capabilities: null,
   101	    capabilityError: null,
   102	    agentItems: [],
   103	    planSteps: [],
   104	    tasks: [],
   105	    pendingApprovals: {},
   106	    pendingInputs: {},
   107	    pendingConfig: {},
   108	    configErrors: {}
   109	  };
   110	}
   111	
   112	export function getConversationSession(ownedId: string): ConversationWorkspaceState | null {
   113	  return conversationSessions[ownedId] ?? null;
   114	}
   115	
   116	export function ensureConversationSession(
   117	  ownedId: string,
   118	  provider: AgentConversationProvider
   119	): ConversationWorkspaceState {
   120	  const current = conversationSessions[ownedId];
   121	  if (current?.provider === provider) return current;
   122	  const created = freshState(ownedId, provider);
   123	  conversationSessions[ownedId] = created;
   124	  return created;
   125	}
   126	
   127	export function applyAgentConversationEvent(event: AgentConversationEvent | AgentEvent): boolean {
   128	  if ('type' in event) return applyCanonicalAgentEvent(event);
   129	  const current = ensureConversationSession(event.ownedId, event.provider);
   130	  const next = applyConversationEvent(current, event);
   131	  if (next === current) return false;
   132	  conversationSessions[event.ownedId] = {
   133	    ...next,
   134	    draft: current.draft,
   135	    mode: current.mode,
   136	    sending: current.sending,
   137	    attachments: current.attachments,
   138	    metadata: current.metadata,
   139	    children: current.children,
   140	    selectedChildId: current.selectedChildId,
   141	    childTimeline: current.childTimeline,
   142	    scrollTop: current.scrollTop,
   143	    childScrollTopById: current.childScrollTopById,
   144	    executionOwner: current.executionOwner,
   145	    writerLease: { ...current.writerLease, generation: next.generation },
   146	    writerLeaseTransition: current.writerLeaseTransition,
   147	    attachmentIds: current.attachmentIds,
   148	    config: current.config,
   149	    telemetry: current.telemetry,
   150	    capabilities: current.capabilities,
   151	    capabilityError: current.capabilityError,
   152	    agentItems: current.agentItems,
   153	    planSteps: current.planSteps,
   154	    tasks: current.tasks,
   155	    pendingApprovals: current.pendingApprovals,
   156	    pendingInputs: current.pendingInputs,
   157	    pendingConfig: current.pendingConfig,
   158	    configErrors: current.configErrors
   159	  };
   160	  const typedItem = agentItemFromEvent(event);
   161	  if (typedItem) {
   162	    const delta = event.payload.kind === 'assistantDelta';
   163	    conversationSessions[event.ownedId].agentItems = upsertAgentItem(
   164	      conversationSessions[event.ownedId].agentItems,
   165	      typedItem,
   166	      delta
   167	    );
   168	  }
   169	  applyTypedEventPayload(conversationSessions[event.ownedId], event);
   170	  return true;
   171	}
   172	
   173	function applyCanonicalAgentEvent(event: AgentEvent): boolean {
   174	  const current = ensureConversationSession(event.ownedId, event.provider);
   175	  if (event.generation < current.generation) return false;
   176	  const newGeneration = event.generation > current.generation;
   177	  const previousSequence = newGeneration ? 0 : current.lastSequence;
   178	  if (!newGeneration && event.sequence <= previousSequence) return false;
   179	  const hasGap = event.sequence !== previousSequence + 1;
   180	  if (hasGap) {
   181	    conversationSessions[event.ownedId] = { ...current, generation: event.generation, lastSequence: event.sequence, desynchronized: true };
   182	    return true;
   183	  }
   184	  const payload = event.payload as Record<string, unknown>;
   185	  const nextState: ConversationWorkspaceState = {
   186	    ...current,
   187	    generation: event.generation,
   188	    lastSequence: event.sequence,
   189	    desynchronized: newGeneration ? false : current.desynchronized,
   190	    connectionState: event.type === 'session.closed' ? 'closed'
   191	      : event.type === 'session.started' ? 'connected'
   192	        : event.type === 'runtime.error' && payload.recoverable === false ? 'failed' : current.connectionState,
   193	    nativeSessionId: event.nativeSessionId ?? current.nativeSessionId,
   194	    writerLease: { ...current.writerLease, generation: event.generation }
   195	  };
   196	  conversationSessions[event.ownedId] = nextState;
   197	  const target = conversationSessions[event.ownedId];
   198	  const typedItem = agentItemFromEvent(event);
   199	  if (typedItem) {
   200	    target.agentItems = upsertAgentItem(target.agentItems, typedItem, event.type === 'content.delta');
   201	  }
   202	  applyTypedEventPayload(target, event);
   203	  return true;
   204	}
   205	
   206	function upsertAgentItem(items: AgentItem[], incoming: AgentItem, delta: boolean): AgentItem[] {
   207	  const index = items.findIndex((item) => item.id === incoming.id);
   208	  if (index < 0) return [...items, incoming];
   209	  const existing = items[index];
   210	  const content = delta && existing.content.length && incoming.content.length
   211	    && existing.content[existing.content.length - 1].channel === incoming.content[0].channel
   212	    ? [
   213	      ...existing.content.slice(0, -1),
   214	      {
   215	        ...existing.content[existing.content.length - 1],
   216	        text: `${existing.content[existing.content.length - 1].text}${incoming.content[0].text}`
   217	      },
   218	      ...incoming.content.slice(1)
   219	    ]
   220	    : incoming.content.length ? incoming.content : existing.content;
   221	  return items.map((item, itemIndex) => itemIndex === index ? { ...item, ...incoming, content } : item);
   222	}
   223	
   224	export function applyAgentConversationSnapshot(snapshot: AgentConversationSnapshot): void {
   225	  const current = ensureConversationSession(
   226	    snapshot.connection.ownedId,
   227	    snapshot.connection.provider
   228	  );
   229	  if (snapshot.connection.generation < current.generation) return;
   230	  let rebuilt = createConversationState(
   231	    snapshot.connection.ownedId,
   232	    snapshot.connection.provider
   233	  );
   234	  rebuilt.generation = snapshot.connection.generation;
   235	  rebuilt.connectionState = snapshot.connection.state;
   236	  rebuilt.nativeSessionId = snapshot.connection.nativeSessionId;
   237	  for (const event of snapshot.events) rebuilt = applyConversationEvent(rebuilt, event);
   238	  conversationSessions[snapshot.connection.ownedId] = {
   239	    ...rebuilt,
   240	    draft: current.draft,
   241	    mode: current.mode,
   242	    sending: current.sending,
   243	    attachments: current.attachments,
   244	    metadata: current.metadata,
   245	    children: current.children,
   246	    selectedChildId: current.selectedChildId,
   247	    childTimeline: current.childTimeline,
   248	    scrollTop: current.scrollTop,
   249	    childScrollTopById: current.childScrollTopById,
   250	    executionOwner: current.executionOwner,
   251	    writerLease: { ...current.writerLease, generation: rebuilt.generation },
   252	    writerLeaseTransition: current.writerLeaseTransition,
   253	    attachmentIds: current.attachmentIds,
   254	    config: current.config,
   255	    telemetry: current.telemetry,
   256	    capabilities: current.capabilities,
   257	    capabilityError: current.capabilityError,
   258	    agentItems: current.agentItems,
   259	    planSteps: current.planSteps,
   260	    tasks: current.tasks,
   480	
   481	export function setConversationTasks(ownedId: string, tasks: ConversationTask[]): void {
   482	  const current = conversationSessions[ownedId];
   483	  if (current) current.tasks = [...tasks];
   484	}
   485	
   486	export function setConversationPendingApproval(
   487	  ownedId: string,
   488	  request: AgentApprovalRequest & { state: string }
   489	): void {
   490	  const current = conversationSessions[ownedId];
   491	  if (current && request.ownedId === ownedId && request.generation === current.generation) {
   492	    current.pendingApprovals[request.requestId] = request;
   493	  }
   494	}
   495	
   496	export function setConversationPendingInput(ownedId: string, request: AgentUserInputRequest): void {
   497	  const current = conversationSessions[ownedId];
   498	  if (current && request.ownedId === ownedId && request.generation === current.generation) {
   499	    current.pendingInputs[request.requestId] = request;
   500	  }
   501	}
   502	
   503	export function setConversationSelectedChild(ownedId: string, childId: string | null): void {
   504	  const current = conversationSessions[ownedId];
   505	  if (!current || current.selectedChildId === childId) return;
   506	  current.selectedChildId = childId;
   507	  current.childTimeline = [];
   508	}
   509	
   510	export function setConversationScrollTop(ownedId: string, scrollTop: number): void {
   511	  const current = conversationSessions[ownedId];
   512	  if (current) current.scrollTop = Math.max(0, scrollTop);
   513	}
   514	
   515	export function setChildConversationScrollTop(
   516	  ownedId: string,
   517	  childId: string,
   518	  scrollTop: number
   519	): void {
   520	  const current = conversationSessions[ownedId];
   521	  if (current) current.childScrollTopById[childId] = Math.max(0, scrollTop);
   522	}
   523	
   524	export function setConversationWriterLeaseTransition(
   525	  transition: AgentWriterLeaseTransition
   526	): boolean {
   527	  const current = conversationSessions[transition.ownedId];
   528	  if (!current || transition.generation !== current.generation) return false;
   529	  if (transition.from !== current.writerLease.owner) return false;
   530	  current.writerLeaseTransition = transition;
   531	  if (transition.state === 'committed') {
   532	    current.writerLease = {
   533	      ownedId: transition.ownedId,
   534	      generation: transition.generation,
   535	      owner: transition.to
   536	    };
   537	    current.executionOwner = transition.to === 'none' ? 'stopped' : transition.to;
   538	    current.writerLeaseTransition = null;
   539	  }
   540	  return true;
   541	}
   542	
   543	export function clearConversationWriterLeaseTransition(ownedId: string, generation: number): void {
   544	  const current = conversationSessions[ownedId];
   545	  if (current?.generation === generation) current.writerLeaseTransition = null;
   546	}
   547	
   548	export function setConversationDraft(ownedId: string, draft: string): void {
   549	  const current = conversationSessions[ownedId];
   550	  if (!current || current.draft === draft) return;
   551	  current.draft = draft;
   552	}
   553	
   554	export function setConversationMode(ownedId: string, mode: ConversationViewMode): void {
   555	  const current = conversationSessions[ownedId];
   556	  if (!current || current.mode === mode) return;
   557	  current.mode = mode;
   558	}
   559	
   560	export function setConversationSending(ownedId: string, sending: boolean): void {
   561	  const current = conversationSessions[ownedId];
   562	  if (!current || current.sending === sending) return;
   563	  current.sending = sending;
   564	}
   565	
   566	export function setConversationConnection(connection: AgentConversationConnection): void {
   567	  const current = ensureConversationSession(connection.ownedId, connection.provider);
   568	  current.generation = connection.generation;
   569	  current.writerLease.generation = connection.generation;
   570	  current.connectionState = connection.state;
   571	  if (connection.nativeSessionId) current.nativeSessionId = connection.nativeSessionId;
   572	}
   573	
   574	export function captureConversationWorkspace(
   575	  ownedId: string
   576	): SessionConversationWorkspace | undefined {
   577	  const current = conversationSessions[ownedId];
   578	  if (!current) return undefined;
   579	  return {
   580	    version: SESSION_CONVERSATION_WORKSPACE_VERSION,
   581	    mode: current.mode,
   582	    draft: current.draft,
   583	    generation: current.generation,
   584	    owner: current.executionOwner,
   585	    attachmentIds: current.attachmentIds,
   586	    config: current.config,
   587	    parentScrollTop: current.scrollTop,
   588	    childScrollTopById: current.childScrollTopById,
   589	    sequence: current.lastSequence,
   590	    telemetry: current.telemetry,
   591	    writerLease: current.writerLease,
   592	    writerLeaseTransition: current.writerLeaseTransition,
   593	    selectedChildId: current.selectedChildId,
   594	    scrollTop: current.scrollTop,
   595	    providerGeneration: current.generation,
   596	    lastSequence: current.lastSequence
   597	  };
   598	}
   599	
   600	export function restoreConversationWorkspace(
   601	  ownedId: string,
   602	  provider: AgentConversationProvider,
   603	  snapshot: SessionConversationWorkspace | null | undefined
   604	): ConversationWorkspaceState {
   605	  const current = ensureConversationSession(ownedId, provider);
   606	  current.mode = snapshot?.mode === 'raw' ? 'raw' : 'structured';
   607	  current.draft = snapshot?.draft ?? '';
   608	  current.selectedChildId = snapshot?.selectedChildId ?? null;
   609	  current.scrollTop = snapshot?.parentScrollTop ?? snapshot?.scrollTop ?? 0;
   610	  current.childScrollTopById = snapshot?.childScrollTopById ?? {};
   611	  current.executionOwner = snapshot?.owner ?? current.executionOwner;
   612	  current.attachmentIds = snapshot?.attachmentIds ?? current.attachmentIds;
   613	  current.config = snapshot?.config ?? current.config;
   614	  current.telemetry = snapshot?.telemetry ?? current.telemetry;
   615	  if (snapshot?.writerLease?.ownedId === ownedId) current.writerLease = snapshot.writerLease;
   616	  if (snapshot?.writerLeaseTransition?.ownedId === ownedId) {
   617	    current.writerLeaseTransition = snapshot.writerLeaseTransition;
   618	  }
   619	  return current;
   620	}
  1380	      rail.owned.map((entry) => entry.ownedId)
  1381	    );
  1382	    writeWorkspaces(window.localStorage, workspaces);
  1383	  }
  1384	
  1385	  onMount(() => {
  1386	    // First, and synchronous: it only touches the DOM, and every panel below
  1387	    // paints in the theme it sets.
  1388	    applyStoredTheme();
  1389	    disposed = false;
  1390	    const stopExtensionApiProbeObservations = onExtensionApiProbeObservation((observation) => {
  1391	      extensionApiProbeObservation = observation;
  1392	    });
  1393	    void startConversationEvents();
  1394	    // Honour where the reader last put the Problems list. The frame and the
  1395	    // tool column both mount before this runs, so both have handed over their
  1396	    // controls by now. Without it the bottom strip comes back open on every
  1397	    // launch however it was left.
  1398	    applyProblemsLocation(settings.panels.problemsLocation);
  1399	    // The C# language server switch lives in the desktop process, which forgets
  1400	    // it between launches and starts with the server allowed. Without this,
  1401	    // someone who turned it off last week silently gets the 800MB back.
  1402	    if (!settings.intelligence.csharpLanguageServer) {
  1403	      void setCsharpLanguageServerEnabled(false);
  1404	    }
  1405	    // The stacks pane never spawns or kills anything itself — the page owns the
  1406	    // rail, the terminal service and the terminal hosts, so it does the work and
  1407	    // the pane asks for it. Pure bookkeeping; nothing runs until a click.
  1408	    registerStackHandlers({
  1409	      onStartStack,
  1410	      onStopStack: (ownedId) => closeTerminal(ownedId),
  1411	      onSelectSession: (ownedId) => selectOwned(ownedId)
  1412	    });
  1413	    void (async () => {
  1414	      try {
  1415	        const backend = tauriTerminalBackend(countInvoke);
  1416	        const modules = await loadXtermModules();
  1417	        if (disposed) return;
  1418	
  1419	        service = createTerminalService({
  1420	          backend,
  1421	          createView: (host, hooks) => makeTerminalView(modules, host, hooks),
  1422	          onExit: (ownedId, payload) => {
  1423	            updateOwnedSession(ownedId, { state: 'exited' });
  1424	            // The stacks pane learns how its run ended from here and nowhere
  1425	            // else: no timer anywhere reads process states. A session that is
  1426	            // not a stack's costs one map lookup.
  1427	            noteTerminalExit(ownedId, { exitCode: payload.exitCode, signal: payload.signal });
  1428	          }
  1429	        });
  1430	        await service.attach();
  1431	        if (disposed) return;
  1432	        if (extensionApiProbeTerminalHost) {
  1433	          await configureExtensionApiProbeRuntime({
  1434	            terminalService: service,
  1435	            terminalHost: extensionApiProbeTerminalHost
  1436	          });
  1437	        }
  1438	
  1439	        // Rail hydration — the ONLY launch IO (constitution).
  1440	        const live = (await backend.list()) ?? [];
  1441	        if (disposed) return;
  1442	        for (const i of live) livePtySizes.set(i.sessionId, { cols: i.cols, rows: i.rows });
  1443	        const { owned, reattachable } = reconcileOwnedSessions(loadStoredOwned(), live);
  1444	        // Tombstones re-attach too (final scrollback + a reapable PTY id); live first.
  1445	        const attachable = [
  1446	          ...reattachable,
  1447	          ...owned.filter((entry) => entry.state === 'exited' && entry.ptySessionId)
  1448	        ];
  1449	        // Claim them BEFORE the hosts mount, or `registerHost` races past.
  1450	        for (const session of attachable) awaitingReattach.add(session.ownedId);
  1451	        hydrateOwned(owned);
  1452	        // The per-session tabs and tree state, read once. Sessions that did not
  1453	        // survive the reconcile take their records with them.
  1454	        workspaces = pruneWorkspaces(
  1455	          readWorkspaces(window.localStorage),
  1456	          owned.map((entry) => entry.ownedId)
  1457	        );
  1458	        writeWorkspaces(window.localStorage, workspaces);
  1459	        // One survivor's failure must cost neither the others their re-attach nor
  1460	        // the Resume group its scan: collect, keep going, report once.
  1461	        const failed: string[] = [];
  1462	        for (const session of attachable) {
  1463	          await hostFor(session.ownedId);
  1464	          try {
  1465	            await reattachIfPending(session.ownedId);
  1466	          } catch (error) {
  1467	            failed.push(`"${session.title}" (${describeError(error)})`);
  1468	          }
  1469	        }
  1470	        // scanRail CLEARS rail.error, so the re-attach report goes after it.
  1471	        await scanRail();
  1472	        if (failed.length > 0 && !disposed) {
  1473	          const prefix = rail.error ? `${rail.error}; ` : '';
  1474	          rail.error = `${prefix}could not re-attach ${failed.join(', ')}`;
  1475	        }
  1476	      } catch (error) {
  1477	        if (!disposed) rail.error = `shell start-up failed: ${describeError(error)}`;
  1478	      } finally {
  1479	        // Launch is over — including when it failed, or the file tree and the
  1480	        // context cards would never load again. From here, a session being
  1481	        // selected is the user's doing and those panels may follow it.
  1482	        if (!disposed) shellPanels.allowSessionLoads();
  1483	        // A session re-attached during start-up was "picked" before the gate was
  1484	        // open, so its pick was ignored. Repeat it now that loads are allowed, or
  1485	        // a reload comes back with empty panes until the user clicks a session.
  1486	        if (!disposed && rail.activeOwnedId !== null) shellPanels.sessionPicked();
  1487	        // Same story for the files that session had open: the pick that would
  1488	        // have restored them happened before the gate opened, so a reload would
  1489	        // otherwise come back with an empty editor.
  1490	        if (!disposed && rail.activeOwnedId !== null) restoreWorkspace(rail.activeOwnedId);
  1491	      }
  1492	    })();
  1493	
  1494	    /** Remember the session on screen when the page goes away. Leaving the
  1495	     * window is not a session switch, so nothing else would have saved it, and a
  1496	     * reload would come back to an empty editor. `pagehide` is the event
  1497	     * browsers still fire for both a reload and a close. */
  1498	    const saveOnLeaving = (): void => {
  1499	      if (rail.activeOwnedId !== null) snapshotWorkspace(rail.activeOwnedId);
  1500	    };
  1501	    window.addEventListener('pagehide', saveOnLeaving);
  1502	
  1503	    return () => {
  1504	      stopExtensionApiProbeObservations();
  1505	      window.removeEventListener('pagehide', saveOnLeaving);
  1506	      // Navigating away inside the app ends here instead, and it is the same
  1507	      // last chance to remember what the session on screen had open.
  1508	      if (!disposed && rail.activeOwnedId !== null) snapshotWorkspace(rail.activeOwnedId);
  1509	      disposed = true;
  1510	      stopConversationEvents();
  1511	      // Probe teardown closes only its disposable PTY first. The product
  1512	      // service then drops views + its listener while every user PTY survives.
  1513	      const serviceToDispose = service;
  1514	      service = null;
  1515	      void disposeExtensionApiProbeRuntime().finally(() => serviceToDispose?.dispose());
  1516	      pendingHosts.clear();
  1517	      awaitingReattach.clear();
  1518	      livePtySizes.clear();
  1519	      // The theme painted inline colors onto <html>, above the scoping that
  1520	      // keeps the old shell on its own palette. Leaving this page takes them
  1521	      // back off, so a same-document navigation to the old shell renders it
  1522	      // exactly as it was found.
  1523	      clearTheme();
  1524	    };
  1525	  });
  1526	</script>
  1527	
  1528	<svelte:head>
  1529	  <title>CommandBar · next</title>
  1530	</svelte:head>

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '840,940p'
nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '620,680p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   840	    const attached = await service.adoptExisting(session, host, size);
   841	    if (!attached) {
   842	      updateOwnedSession(ownedId, { state: 'exited', ptySessionId: null });
   843	      return;
   844	    }
   845	    if (rail.activeOwnedId === null) await selectOwned(ownedId);
   846	  }
   847	
   848	  /**
   849	   * The editor tabs the last few sessions left behind, contents and all.
   850	   *
   851	   * The stored record beside it is only a list of paths, so putting a session
   852	   * back used to mean reading every one of its files off disk again — including
   853	   * the file the reader had been looking at seconds earlier. Holding the tabs
   854	   * themselves is what makes going back to a session free. Only the last three
   855	   * sessions are held, so the memory this costs has a ceiling; the fourth reads
   856	   * its files again, exactly as every session used to.
   857	   *
   858	   * Not persisted, and it must not be: file contents are far too big for the
   859	   * browser's storage, and a reload has to read from disk anyway.
   860	   */
   861	  let retainedTabs: RetainedWorkspaces<OpenEditorFile> = emptyRetainedWorkspaces();
   862	
   863	  function conversationProviderFor(ownedId: string): AgentConversationProvider | null {
   864	    const agent = rail.owned.find((session) => session.ownedId === ownedId)?.agent;
   865	    return agent === 'codex' || agent === 'claude' ? agent : null;
   866	  }
   867	
   868	  /** Remember the editor tabs and file tree this session is leaving behind.
   869	   * Stored straight away: a reload can come at any moment, and the write is a
   870	   * few hundred bytes. */
   871	  function snapshotWorkspace(ownedId: string): void {
   872	    // The tabs themselves are held here rather than in the stored record: same
   873	    // moment, same session, but this half stays in memory. A session leaving an
   874	    // empty editor holds nothing, so its place goes to one that has files.
   875	    retainedTabs = retainTabs(retainedTabs, ownedId, editorState.openFiles);
   876	    workspaces = {
   877	      ...workspaces,
   878	      [ownedId]: captureWorkspace({
   879	        openFiles: editorState.openFiles,
   880	        activePath: editorState.activePath,
   881	        expandedFolderIds: explorer.expandedFolderIds,
   882	        selectedPath: explorer.selectedPath,
   883	        scrollTop: explorer.scrollTop,
   884	        diffPath: gitPanel.selectedPath || null,
   885	        diffRoot: gitPanel.root,
   886	        conversation: captureConversationWorkspace(ownedId),
   887	        browser: captureBrowserState(),
   888	        center: frameControls?.captureCenterLayout() ?? null
   889	      })
   890	    };
   891	    writeWorkspaces(window.localStorage, workspaces);
   892	  }
   893	
   894	  /**
   895	   * Put back the editor tabs and file tree this session had.
   896	   *
   897	   * A tab still held from the last time this session was on screen goes back as
   898	   * it is — the file is already in memory, so nothing is read. Any other file
   899	   * the record names goes back through the same "open this file" request the
   900	   * explorer uses, in strip order. Either way the file that was showing is asked
   901	   * for last, so it is the one left in front and the panel points its lookups at
   902	   * it; asking for a file that is already open costs nothing but that.
   903	   *
   904	   * The tree's state is assigned directly, AFTER `sessionPicked` has pointed the
   905	   * explorer at the project: listing the same folder again does nothing, and a
   906	   * scan of a different folder never closes folders the user had open. A
   907	   * highlighted file that no longer exists loses its highlight when that scan
   908	   * lands, which is the right answer.
   909	   */
   910	  function restoreWorkspace(ownedId: string): void {
   911	    // Start-up re-attaching a session picks it, which is indistinguishable from
   912	    // a click. Replaying files then would read files before launch is over; the
   913	    // end of start-up calls this itself once the gate is open.
   914	    if (!shellPanels.loadsAllowed()) return;
   915	    // The Diff tab is one tab for the whole shell — it is always mounted, and
   916	    // source control only re-points itself while it is the view in front. So
   917	    // what the tab shows must be decided here, on every switch: the diff THIS
   918	    // session remembered comes back, and anything else — including the file
   919	    // the last session was looking at, which otherwise survives because the
   920	    // panel's own root only updates while its view is in front — is cleared.
   921	    const sessionRoot = readSelection().root.trim();
   922	    const rememberedDiff = diffPathFor(workspaces[ownedId] ?? null, sessionRoot);
   923	    if (rememberedDiff === null) {
   924	      gitService.clearSelection();
   925	    } else {
   926	      void gitService.showStoredDiff(sessionRoot, rememberedDiff);
   927	    }
   928	    const snapshot = workspaces[ownedId];
   929	    const conversationProvider = conversationProviderFor(ownedId);
   930	    if (conversationProvider) {
   931	      restoreConversationWorkspace(ownedId, conversationProvider, snapshot?.conversation);
   932	    }
   933	    restoreBrowserState(snapshot?.browser);
   934	    if (snapshot?.center) frameControls?.restoreCenterLayout(snapshot.center);
   935	    const taken = takeRetainedTabs(retainedTabs, ownedId);
   936	    retainedTabs = taken.retained;
   937	    const plan = planWorkspaceRestore(snapshot ?? null, taken.tabs);
   938	    // Every restore starts from THIS session's tabs and no others. A session
   939	    // that has never had a file open starts from an empty editor, and that
   940	    // emptiness is the whole point: it is the other session's tabs not being
   620	  /** The center library reuses the rail's imperative actions; construction of
   621	   * this adapter is inert and does not scan, start, or mutate anything. */
   622	  const sessionLibraryService = createSessionLibraryService(
   623	    {
   624	      getOwnedSessions: () => rail.owned,
   625	      getAvailableSessions: () => rail.available
   626	    },
   627	    {
   628	      onOpen: async (record: SessionLibraryRecord) => {
   629	        if (record.ownedId) {
   630	          await selectOwned(record.ownedId);
   631	        } else if (record.available) {
   632	          await adopt(record.available);
   633	        }
   634	      },
   635	      onResume: async (record: SessionLibraryRecord) => {
   636	        if (record.ownedId) {
   637	          const owned = rail.owned.find((session) => session.ownedId === record.ownedId);
   638	          if (owned?.state === 'exited') await restartOwned(record.ownedId);
   639	          else await selectOwned(record.ownedId);
   640	        } else if (record.available) {
   641	          await adopt(record.available);
   642	        }
   643	      },
   644	      onArchive: (record: SessionLibraryRecord) => {
   645	        if (!record.ownedId) return;
   646	        if (record.state === 'settled') unsettleOwnedSession(record.ownedId);
   647	        else if (record.state === 'done') settleOwnedSession(record.ownedId);
   648	      },
   649	      onDelete: async (record: SessionLibraryRecord) => {
   650	        if (record.ownedId) await removeSession(record.ownedId);
   651	      }
   652	    }
   653	  );
   654	
   655	  /** Palette actions for the panels. Pure bookkeeping — nothing runs until the
   656	   * user picks one — so it belongs here at component init, not in an effect. */
   657	  registerShellCommands({
   658	    showPanel: (id) => frameControls?.showCenterPanel(id),
   659	    expandSourceControl: () => sidebarControls?.expandSourceControl(),
   660	    // Opening a view is what lets that view read anything, so nothing else has
   661	    // to be called here — the column reports the change and the load follows.
   662	    showView: (id) => sidebarControls?.selectView(id),
   663	    openNewSession: () => overlays?.openNewSession(),
   664	    showProblemsAtBottom: () => {
   665	      settings.panels.problemsLocation = 'bottom';
   666	      applyProblemsLocation('bottom');
   667	    }
   668	  });
   669	
   670	  /** The two places the context panel can send you: the Worktrees view, and the
   671	   * sessions column's "Find a session" drawer. Both live outside that panel, so
   672	   * it asks rather than reaching for them. Pure bookkeeping, like the palette
   673	   * actions above — nothing runs until a link is clicked. */
   674	  setContextPanelHooks({
   675	    showWorktreesView: () => sidebarControls?.selectView('worktrees'),
   676	    openSessionFinder: () => sessionsColumn?.openFinder()
   677	  });
   678	
   679	  /**
   680	   * Is the sessions column folded up to a strip? The PAGE owns this rather

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '780,850p;1090,1190p;1200,1320p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   780	        sessions = await listAgentSessionsFromTauri();
   781	      } catch (error) {
   782	        nativeError = describeError(error);
   783	      }
   784	      // A `null` native result with no error = not under Tauri: nothing invoked.
   785	      countInvoke((sessions ?? nativeError) ? 'list_agent_sessions' : 'bridge:agent-sessions');
   786	      sessions ??= await listAgentSessionsFromLocalBridge();
   787	      if (disposed) return;
   788	      setAvailable(sessions ?? []);
   789	      // Surface the native failure only if the bridge produced nothing either.
   790	      if (sessions === null && nativeError) rail.error = `session scan failed: ${nativeError}`;
   791	    } catch (error) {
   792	      if (!disposed) rail.error = `session scan failed: ${nativeError ?? describeError(error)}`;
   793	    } finally {
   794	      rail.scanning = false;
   795	    }
   796	  }
   797	
   798	  /** Park a freshly mounted host, and re-attach immediately if one is owed. */
   799	  function registerHost(ownedId: string, host: HTMLElement): void {
   800	    pendingHosts.set(ownedId, host);
   801	    // Fire-and-forget, but never unhandled: this path has no awaiting caller.
   802	    void reattachIfPending(ownedId).catch((error) => {
   803	      if (!disposed) rail.error = `re-attach failed: ${describeError(error)}`;
   804	    });
   805	  }
   806	
   807	  /** Wait for TerminalSurface to mount the host for `ownedId`.
   808	   *
   809	   * A host that is no longer IN the page is not an answer: a session whose
   810	   * terminal was closed loses its host div, and this map still holds the one it
   811	   * used to have. Building a terminal on that detached element would leave the
   812	   * session running with nothing on screen — so a host that has been taken out
   813	   * of the page is dropped here, and the wait continues for the one Svelte is
   814	   * about to mount in its place. */
   815	  async function hostFor(ownedId: string): Promise<HTMLElement | null> {
   816	    const mounted = (): HTMLElement | null => {
   817	      const host = pendingHosts.get(ownedId);
   818	      if (!host) return null;
   819	      if (host.isConnected) return host;
   820	      pendingHosts.delete(ownedId);
   821	      return null;
   822	    };
   823	    for (let attempt = 0; attempt < 12; attempt += 1) {
   824	      const host = mounted();
   825	      if (host) return host;
   826	      await tick();
   827	      if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 8));
   828	    }
   829	    return mounted();
   830	  }
   831	
   832	  /** Re-attach one reload survivor; `awaitingReattach` guards double-adopting. */
   833	  async function reattachIfPending(ownedId: string): Promise<void> {
   834	    if (!service || disposed || !awaitingReattach.has(ownedId)) return;
   835	    awaitingReattach.delete(ownedId);
   836	    const host = pendingHosts.get(ownedId);
   837	    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
   838	    if (!host || !session) return;
   839	    const size = session.ptySessionId ? (livePtySizes.get(session.ptySessionId) ?? null) : null;
   840	    const attached = await service.adoptExisting(session, host, size);
   841	    if (!attached) {
   842	      updateOwnedSession(ownedId, { state: 'exited', ptySessionId: null });
   843	      return;
   844	    }
   845	    if (rail.activeOwnedId === null) await selectOwned(ownedId);
   846	  }
   847	
   848	  /**
   849	   * The editor tabs the last few sessions left behind, contents and all.
   850	   *
  1090	      rail.error = `structured handoff failed: ${describeError(error)}`;
  1091	    }
  1092	  }
  1093	
  1094	  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
  1095	  async function adopt(record: AgentSession): Promise<void> {
  1096	    if (!service || disposed) return;
  1097	    const owned = adoptAgentSession(record);
  1098	    addOwnedSession(owned);
  1099	    const host = await hostFor(owned.ownedId);
  1100	    if (!host) {
  1101	      rail.error = `no terminal host for "${owned.title}"`;
  1102	      return;
  1103	    }
  1104	    const ptySessionId = await service.startOwned(owned, host);
  1105	    if (!ptySessionId) {
  1106	      updateOwnedSession(owned.ownedId, { state: 'exited' });
  1107	      rail.error = `failed to start a terminal for "${owned.title}"`;
  1108	      return;
  1109	    }
  1110	    // Persist the PTY id: reload re-attach reads it back out of localStorage.
  1111	    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  1112	    await selectOwned(owned.ownedId);
  1113	    // A session you just started is a session you want to watch. Deliberately
  1114	    // here rather than inside `selectOwned`, which also runs on every plain
  1115	    // click on a card — a click on a row must not yank the reader off the file
  1116	    // they had open.
  1117	    frameControls?.showCenterPanel('session');
  1118	  }
  1119	
  1120	  /**
  1121	   * EXPLICIT IO: start a session the user described in the new-session dialog.
  1122	   *
  1123	   * Mirrors `adopt` step for step; the difference is where the description comes
  1124	   * from. `createFreshSession` makes a plain shell — agent 'other', nothing to
  1125	   * replay — and the dialog knows better on both counts, so BOTH are set before
  1126	   * `startOwned`: that is the call which types the command in.
  1127	   */
  1128	  async function startNewSession(request: NewSessionRequest): Promise<void> {
  1129	    if (!service || disposed) return;
  1130	    const owned = {
  1131	      ...createFreshSession({ cwd: request.cwd, title: request.title }),
  1132	      agent: request.agent,
  1133	      resumeCommand: request.command
  1134	    };
  1135	    addOwnedSession(owned);
  1136	    const host = await hostFor(owned.ownedId);
  1137	    if (!host) {
  1138	      rail.error = `no terminal host for "${owned.title}"`;
  1139	      return;
  1140	    }
  1141	    const ptySessionId = await service.startOwned(owned, host);
  1142	    if (!ptySessionId) {
  1143	      updateOwnedSession(owned.ownedId, { state: 'exited' });
  1144	      rail.error = `failed to start a terminal for "${owned.title}"`;
  1145	      return;
  1146	    }
  1147	    // Persist the PTY id: reload re-attach reads it back out of localStorage.
  1148	    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  1149	    await selectOwned(owned.ownedId);
  1150	    // Same as resuming one: you asked for this session, so it comes to the front.
  1151	    frameControls?.showCenterPanel('session');
  1152	  }
  1153	
  1154	  /**
  1155	   * EXPLICIT IO: run a saved stack. A stack IS a session — it appears on the
  1156	   * rail like any other and its terminal is the one you watch.
  1157	   *
  1158	   * `runCommandDirectly` is what makes the stacks pane honest: the session is
  1159	   * the command rather than a shell with the command typed into it, so when the
  1160	   * dev server dies the session ends with the server's exit code instead of
  1161	   * dropping back to a prompt that reads as "still starting". A desktop build
  1162	   * too old for that falls back to typing the command in — see `startOwned`.
  1163	   *
  1164	   * Answers with the new session's id, or `null` when no terminal could be
  1165	   * opened; the pane says so in its own words rather than guessing.
  1166	   */
  1167	  async function onStartStack(request: StackStartRequest): Promise<string | null> {
  1168	    if (!service || disposed) return null;
  1169	    const owned = {
  1170	      ...createFreshSession({ cwd: request.cwd, title: request.title }),
  1171	      resumeCommand: request.script
  1172	    };
  1173	    addOwnedSession(owned);
  1174	    const host = await hostFor(owned.ownedId);
  1175	    if (!host) return null;
  1176	    const ptySessionId = await service.startOwned(owned, host, { runCommandDirectly: true });
  1177	    if (!ptySessionId) {
  1178	      updateOwnedSession(owned.ownedId, { state: 'exited' });
  1179	      return null;
  1180	    }
  1181	    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  1182	    await selectOwned(owned.ownedId);
  1183	    // Pressing play is asking to watch the thing start. Without this the run
  1184	    // configuration's terminal opens behind whatever tab was already in front,
  1185	    // and a command that fails immediately does so out of sight.
  1186	    frameControls?.showCenterPanel('session');
  1187	    return owned.ownedId;
  1188	  }
  1189	
  1190	  /**
  1200	   * revived, so this spawns a NEW one in the same folder and replays the
  1201	   * session's resume command — the same thing resuming a scanned session does,
  1202	   * which is why the agent picks the conversation up where it left off. A
  1203	   * session started here rather than found on disk has no resume command and
  1204	   * gets a plain shell back.
  1205	   *
  1206	   * Only one restart per row can be in flight. The row keeps reading "finished"
  1207	   * — and so keeps offering the button — for as long as the first click is
  1208	   * waiting on the backend, so without the `restarting` guard a double-click
  1209	   * would spawn two terminals for one session. The second would replace the
  1210	   * first in the service's bookkeeping while the first process kept running,
  1211	   * leaving two agents appending to the same transcript and only one of them
  1212	   * showing up anywhere the user could reach it.
  1213	   */
  1214	  async function restartOwned(ownedId: string): Promise<void> {
  1215	    // Checked and claimed before the first await, so a second click cannot slip
  1216	    // through the window the first one opens.
  1217	    if (!service || disposed || restarting.has(ownedId)) return;
  1218	    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
  1219	    // Only a finished session can be started again; a running one already is.
  1220	    if (!session || session.state !== 'exited') return;
  1221	    const label = session.title || ownedId;
  1222	    restarting.add(ownedId);
  1223	
  1224	    /**
  1225	     * Put the terminal the manager promoted back on screen. Closing a view
  1226	     * makes another session's view visible and `closeOwned` reports which one;
  1227	     * every other caller adopts that answer. On the paths below that never
  1228	     * reach `selectOwned(ownedId)` this is the only thing standing between the
  1229	     * user and another session's scrollback sitting under this session's title.
  1230	     */
  1231	    const adoptSuccessor = async (successor: string | null): Promise<void> => {
  1232	      if (successor === null) return;
  1233	      if (!rail.owned.some((entry) => entry.ownedId === successor)) return;
  1234	      await selectOwned(successor);
  1235	    };
  1236	
  1237	    /** Whoever the first close promoted, kept where every exit can see it. */
  1238	    let successor: string | null = null;
  1239	
  1240	    try {
  1241	      // The old terminal is over: drop its view and let the backend forget the
  1242	      // dead process. One session has one view, so without this the new
  1243	      // terminal would open underneath the last one's final output — and the
  1244	      // backend record of the finished process would be left with nothing able
  1245	      // to reach it. A refusal here is not worth stopping for or reporting: it
  1246	      // means the backend could not tidy away something that is already dead,
  1247	      // and the session is about to get a working terminal regardless.
  1248	      awaitingReattach.delete(ownedId);
  1249	      const closed = await service.closeOwned(ownedId, session.ptySessionId);
  1250	      successor = closed?.successor ?? null;
  1251	
  1252	      // Say the row is running BEFORE asking for a terminal host: the terminal
  1253	      // surface only keeps a host on screen for a session it believes has a
  1254	      // terminal, so while the row still reads as finished there is nothing for
  1255	      // `hostFor` to wait for. The old PTY id goes at the same time — it names
  1256	      // a process that no longer exists.
  1257	      updateOwnedSession(ownedId, { state: 'live', ptySessionId: null });
  1258	
  1259	      const host = await hostFor(ownedId);
  1260	      if (!host) {
  1261	        updateOwnedSession(ownedId, { state: 'exited' });
  1262	        rail.error = `no terminal host for "${label}"`;
  1263	        await adoptSuccessor(successor);
  1264	        return;
  1265	      }
  1266	      // `startOwned` reads the folder and the resume command off this record;
  1267	      // the PTY id it had is cleared so nothing can point at the old process.
  1268	      // A stack's session keeps its one-command spawn on restart — typed into
  1269	      // a shell instead, the exit code would belong to the shell and a crashed
  1270	      // dev server would read as "started" again.
  1271	      const restartedStackId = stackIdForOwnedId(ownedId);
  1272	      const ptySessionId = await service.startOwned(
  1273	        { ...session, ptySessionId: null },
  1274	        host,
  1275	        restartedStackId !== null ? { runCommandDirectly: true } : undefined
  1276	      );
  1277	      if (!ptySessionId) {
  1278	        updateOwnedSession(ownedId, { state: 'exited' });
  1279	        rail.error = `could not start "${label}" again: no new terminal opened`;
  1280	        await adoptSuccessor(successor);
  1281	        return;
  1282	      }
  1283	
  1284	      // Ask again what the row says now. The moment it read "running" its Close
  1285	      // and Remove buttons came back, and either of them could have been used
  1286	      // while the terminal was still starting. Neither could reach this PTY —
  1287	      // it did not exist yet — so adopting it here would put back a session the
  1288	      // user has just closed, or attach a live process to a row that is gone.
  1289	      const current = rail.owned.find((entry) => entry.ownedId === ownedId);
  1290	      if (!current || current.state === 'exited') {
  1291	        const closedAgain = await service.closeOwned(ownedId, ptySessionId);
  1292	        await adoptSuccessor(closedAgain?.successor ?? null);
  1293	        return;
  1294	      }
  1295	
  1296	      // Persist the new PTY id: reload re-attach reads it back out of storage.
  1297	      updateOwnedSession(ownedId, { ptySessionId, state: 'live' });
  1298	      // A restarted stack run is a run again — without this the stacks pane
  1299	      // keeps the old exit on record and says "stopped" under a live server.
  1300	      if (restartedStackId !== null) recordStackStart(restartedStackId, ownedId);
  1301	      await selectOwned(ownedId);
  1302	      // Only on this path, where the session the user asked for is the one that
  1303	      // ended up on screen. Every early return above hands the screen to a
  1304	      // DIFFERENT session on purpose, and pulling the reader to the terminal
  1305	      // panel there would show them somebody else's scrollback.
  1306	      frameControls?.showCenterPanel('session');
  1307	    } catch (error) {
  1308	      // The row goes back to finished rather than sitting there claiming to be
  1309	      // running: nothing started, and the card's buttons must still offer this.
  1310	      updateOwnedSession(ownedId, { state: 'exited' });
  1311	      if (!disposed) {
  1312	        rail.error = `could not start "${label}" again: ${describeError(error)}`;
  1313	        // Same reason as the early returns above: the close at the top of this
  1314	        // function already put someone else's terminal on screen.
  1315	        await adoptSuccessor(successor);
  1316	      }
  1317	    } finally {
  1318	      restarting.delete(ownedId);
  1319	    }
  1320	  }

exec
/bin/zsh -lc "git log --oneline --decorate -30 -- tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte tauri-svelte-preview/src/routes/next/+page.svelte tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
printf '\\n--- follow ConversationSurface ---\\n'
git log --follow --oneline --all -- tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte | head -80
printf '\\n--- follow page relevant grep commits ---\\n'
git log --all --oneline -S\"setConversationMode(ownedId, 'raw')\" -- tauri-svelte-preview/src/routes/next/+page.svelte
printf '\\n--- follow service terminal projection commits ---\\n'
git log --all --oneline -S\"start_agent_conversation_terminal_projection\" -- tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts tauri-svelte-preview/src-tauri/src/agent_conversation | head -80" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
feb81ac feat: Session History tab with inline details and context menus
5cf470b style: Houston theme across the /next shell
ba46633 conversation: native CLI handoff between structured and terminal modes
d800313 shell: wire browser overlay host and workbench action fab
b39cd46 shell: mount Agents panel in center roster and tool column
6651c95 tsk-808: mount Working/Done/Settled paneview and Session Library center workspace
4857674 tsk-808: replace 500ms transcript polling with Rust terminal projection events
d68c238 tsk-808: conversation timeline and input refinements from A4 lane completion
0d5f2f9 tsk-808: A8 side-pane registry, Working/Done/Settled panes, session library; streaming delta upsert
5643167 tsk-808: A4 typed conversation timeline, capability controls, screenshot paste
c78e3dd tsk-808: freeze A1 runtime, event, ownership, and config contracts
acae026 tsk-808: extension API compatibility probe, contrast tokens, terminal probe API
5884139 checkpoint: preserve native workbench integration WIP
bd0f9d2 (origin/tsk-799-code-intelligence) feat: C# runs on the official Roslyn language server, and each workspace keeps its own warm server
064eb57 feat: going back to a session you were just on keeps its tabs instead of reading every file again
cb86fbc fix: review findings — the margin never claims a count nobody took, the diff tab follows the session honestly, and a stop click re-checks its target
9608c1e feat: wire the polish wave into the shell
5a34252 (origin/tsk-344-760-763-764-767-770-panels-wave) fix: review findings — problems panel loads for the picked session, no text below the font floor anywhere, themes come off on leave, stack restarts stay honest
a47f897 feat: every new panel finds its home in the shell
5bfc336 fix: an unfolded sessions column comes back unfolded
6063733 fix: starting a finished session again is safe to click twice
4841420 feat: a finished session can be started again from its row
e6ada60 feat: sessions column — task cards, find-a-session drawer, collapse to strip
be33096 feat: sessions live on the left always; every tool view moves right
6e42bdf feat: shadcn-svelte foundation for the /next shell, themed from the Houston tokens
974d336 (origin/tsk-788-761-session-workspaces) fix: never snapshot a workspace before start-up finishes (guards the close-during-launch race)
e7ad6ed fix: milestone review findings for the session workspace round
b51af2c feat: each session keeps its own editor tabs and file-tree state
81175bd feat: sessions complete and reopen like tasks; closing a terminal no longer erases the session
8ad3a13 feat(next): activity bar — each icon opens a view with its own panes

--- follow ConversationSurface ---
5cf470b style: Houston theme across the /next shell
ba46633 conversation: native CLI handoff between structured and terminal modes
4857674 tsk-808: replace 500ms transcript polling with Rust terminal projection events
d68c238 tsk-808: conversation timeline and input refinements from A4 lane completion
5643167 tsk-808: A4 typed conversation timeline, capability controls, screenshot paste
5884139 checkpoint: preserve native workbench integration WIP

--- follow page relevant grep commits ---
5884139 checkpoint: preserve native workbench integration WIP

--- follow service terminal projection commits ---
4857674 tsk-808: replace 500ms transcript polling with Rust terminal projection events
bbf6f3a tsk-808: A3-terminal incremental transcript projection replacing whole-tail reads

exec
/bin/zsh -lc "git show --stat --oneline 4857674 && git show --format=fuller --find-renames 4857674 -- tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte tauri-svelte-preview/src/routes/next/+page.svelte tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts | sed -n '1,320p'
printf '\\n--- handoff commit ---\\n'
git show --stat --oneline ba46633 && git show --format=fuller ba46633 -- tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte tauri-svelte-preview/src/routes/next/+page.svelte tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts | sed -n '1,380p'
printf '\\n--- initial checkpoint ConversationSurface ---\\n'
git show 5884139:tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte 2>/dev/null | nl -ba | sed -n '1,300p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
4857674 tsk-808: replace 500ms transcript polling with Rust terminal projection events
 .../shell/components/ConversationSurface.svelte    |  4 +-
 .../lib/shell/conversation/conversationService.ts  | 65 ++++++++--------------
 tauri-svelte-preview/src/routes/next/+page.svelte  | 10 ++--
 3 files changed, 31 insertions(+), 48 deletions(-)
commit 4857674db3ad145b2f99645338e3c63d7ec11b65
Author:     jcoble <coble.jesse@gmail.com>
AuthorDate: Tue Aug 4 23:45:48 2026 -0400
Commit:     jcoble <coble.jesse@gmail.com>
CommitDate: Tue Aug 4 23:45:48 2026 -0400

    tsk-808: replace 500ms transcript polling with Rust terminal projection events

diff --git a/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte b/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
index d4f8366..90649a2 100644
--- a/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
+++ b/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
@@ -25,7 +25,7 @@
     saveConversationClipboardImage,
     sendStructuredMessage,
     setConversationConfigOption,
-    startConversationTranscriptMirror
+    startConversationTerminalProjection
   } from '$lib/shell/conversation/conversationService';
   import {
     filterConversationCommandCatalog,
@@ -104,7 +104,7 @@
 
   $effect(() => {
     if (structured && (active?.agent === 'claude' || active?.agent === 'codex') && active.nativeSessionId && active.ptySessionId) {
-      startConversationTranscriptMirror({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
+      startConversationTerminalProjection({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
     }
   });
 
diff --git a/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts b/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
index 55b4230..1fbce7a 100644
--- a/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
+++ b/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
@@ -4,7 +4,6 @@ import {
   applyAgentConversationEvent,
   applyAgentConversationSnapshot,
   applyChildConversationTranscript,
-  applyConversationTranscript,
   ensureConversationSession,
   getConversationSession,
   setConversationConnection,
@@ -33,29 +32,12 @@ import { writeTerminalSessionFromTauri } from '$lib/tauriSource';
 
 let unlisten: UnlistenFn | null = null;
 const resyncing = new Map<string, Promise<void>>();
-const transcriptMirrors = new Map<string, ReturnType<typeof setInterval>>();
+const terminalProjections = new Map<string, string>();
 
-async function refreshTranscript(
-  ownedId: string,
-  provider: AgentConversationProvider,
-  nativeSessionId: string
-): Promise<void> {
-  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
-    provider,
-    nativeSessionId,
-    childSessionId: null
-  });
-  applyConversationTranscript(ownedId, provider, snapshot);
-  const childSessionId = getConversationSession(ownedId)?.selectedChildId;
-  if (childSessionId) {
-    const child = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
-      provider,
-      nativeSessionId,
-      childSessionId
-    });
-    applyChildConversationTranscript(ownedId, childSessionId, child.messages);
-  }
-}
+type TerminalProjectionRegistration = {
+  generation: number;
+  events: AgentEvent[];
+};
 
 export async function readChildConversationTranscript(input: {
   ownedId: string;
@@ -254,29 +236,30 @@ export async function setConversationConfigOption(
   }
 }
 
-export function startConversationTranscriptMirror(input: {
+export function startConversationTerminalProjection(input: {
   ownedId: string;
   provider: AgentConversationProvider;
   nativeSessionId: string;
 }): void {
-  if (!isTauri() || transcriptMirrors.has(input.ownedId)) return;
-  const refresh = (): void => {
-    void refreshTranscript(input.ownedId, input.provider, input.nativeSessionId).catch(() => {
-      // The transcript may not exist until Claude accepts its first prompt.
-      // The next poll retries without breaking the conversation surface.
-    });
-  };
-  refresh();
-  const timer = setInterval(() => {
-    refresh();
-  }, 500);
-  transcriptMirrors.set(input.ownedId, timer);
+  if (!isTauri()) return;
+  const signature = `${input.provider}:${input.nativeSessionId}`;
+  if (terminalProjections.get(input.ownedId) === signature) return;
+  terminalProjections.set(input.ownedId, signature);
+  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
+  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
+    request: { ...input, generation }
+  }).then((registration) => {
+    for (const event of registration.events) applyAgentConversationEvent(event);
+  }).catch(() => {
+    // The transcript may not exist until the agent accepts its first prompt.
+    // Dropping the signature lets the next activation register again.
+    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
+  });
 }
 
-export function stopConversationTranscriptMirror(ownedId: string): void {
-  const timer = transcriptMirrors.get(ownedId);
-  if (timer) clearInterval(timer);
-  transcriptMirrors.delete(ownedId);
+export function stopConversationTerminalProjection(ownedId: string): void {
+  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
+  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
 }
 
 async function resyncConversation(ownedId: string): Promise<void> {
@@ -315,7 +298,7 @@ export async function startConversationEvents(): Promise<void> {
 export function stopConversationEvents(): void {
   unlisten?.();
   unlisten = null;
-  for (const ownedId of transcriptMirrors.keys()) stopConversationTranscriptMirror(ownedId);
+  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
 }
 
 export async function closeStructuredConversation(ownedId: string): Promise<void> {
diff --git a/tauri-svelte-preview/src/routes/next/+page.svelte b/tauri-svelte-preview/src/routes/next/+page.svelte
index dea35e0..a6647ff 100644
--- a/tauri-svelte-preview/src/routes/next/+page.svelte
+++ b/tauri-svelte-preview/src/routes/next/+page.svelte
@@ -50,8 +50,8 @@
     closeStructuredConversation,
     ensureStructuredConversation,
     startConversationEvents,
-    startConversationTranscriptMirror,
-    stopConversationTranscriptMirror,
+    startConversationTerminalProjection,
+    stopConversationTerminalProjection,
     stopConversationEvents
   } from '$lib/shell/conversation/conversationService';
   import type { AgentConversationProvider } from '$lib/shell/conversation/conversationTypes';
@@ -507,7 +507,7 @@
     // session actually had (rows are clickable for seconds while the first
     // scan runs — including the close button, which switches sessions too).
     if (switching && previous !== null && shellPanels.loadsAllowed()) snapshotWorkspace(previous);
-    if (switching && previous !== null) stopConversationTranscriptMirror(previous);
+    if (switching && previous !== null) stopConversationTerminalProjection(previous);
     if (switching && selected) {
       const root = selected.cwd.trim() || (selected.projectPath ?? '').trim();
       if (root) await setExtensionApiProbeWorkspace({ ownedId: selected.ownedId, root });
@@ -532,7 +532,7 @@
         void closeStructuredConversation(ownedId);
         if (selected.nativeSessionId) {
           setConversationMode(ownedId, 'structured');
-          startConversationTranscriptMirror({
+          startConversationTerminalProjection({
             ownedId,
             provider,
             nativeSessionId: selected.nativeSessionId
@@ -831,7 +831,7 @@
     awaitingReattach.delete(ownedId);
     removeOwnedSession(ownedId);
     removeConversationSession(ownedId);
-    stopConversationTranscriptMirror(ownedId);
+    stopConversationTerminalProjection(ownedId);
     // A removed row takes its stack tag with it, rather than leaving one
     // pointing at a session that is gone.
     noteSessionRemoved(ownedId);

--- handoff commit ---
ba46633 conversation: native CLI handoff between structured and terminal modes
 .../evidence/tsk-808/a10-dispatch-spec-raw.log     | 31066 +++++++++++++++++++
 .../evidence/tsk-808/a5-handoff-receipt.md         |    80 +
 tauri-svelte-preview/package.json                  |     1 +
 .../scripts/agentConversationHandoff.test.mjs      |    96 +
 .../src/agent_conversation/capabilities.rs         |     1 +
 .../src-tauri/src/agent_conversation/handoff.rs    |   755 +
 .../src-tauri/src/agent_conversation/manager.rs    |   298 +-
 .../src-tauri/src/agent_conversation/mod.rs        |     1 +
 .../src-tauri/src/agent_conversation/protocol.rs   |     6 +
 .../src/agent_conversation/providers/acp.rs        |    14 +
 .../src/agent_conversation/providers/acp_client.rs |    11 +
 .../providers/adapter_support.rs                   |     9 +
 .../src/agent_conversation/providers/mod.rs        |     3 +
 tauri-svelte-preview/src-tauri/src/main.rs         |     1 +
 .../shell/components/ConversationSurface.svelte    |    40 +-
 .../lib/shell/conversation/conversationService.ts  |    76 +-
 .../shell/conversation/conversationStore.svelte.ts |     5 +
 .../lib/shell/conversation/conversationTypes.ts    |   136 +
 tauri-svelte-preview/src/routes/next/+page.svelte  |   113 +-
 19 files changed, 32685 insertions(+), 27 deletions(-)
commit ba466332a66ba391ce056eb7b6bb67b4ccf8fce2
Author:     jcoble <coble.jesse@gmail.com>
AuthorDate: Sat Aug 8 03:47:13 2026 -0400
Commit:     jcoble <coble.jesse@gmail.com>
CommitDate: Sat Aug 8 03:47:13 2026 -0400

    conversation: native CLI handoff between structured and terminal modes
    
    Add a handoff module to the agent conversation manager: prepare/commit
    a two-phase handoff of a conversation between the structured ACP owner
    and a native CLI terminal owner, same-session or fork, with a single
    process-tree writer, rollback on failure, and restart recovery that
    never spawns a provider for a stored row. The conversation surface
    gains open-native-CLI, fork-native-CLI, and return-to-structured
    actions; mode and ownership are explicit in the store and types.

diff --git a/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte b/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
index 90649a2..37de720 100644
--- a/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
+++ b/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
@@ -24,8 +24,7 @@
     restoreConversationAttachments,
     saveConversationClipboardImage,
     sendStructuredMessage,
-    setConversationConfigOption,
-    startConversationTerminalProjection
+    setConversationConfigOption
   } from '$lib/shell/conversation/conversationService';
   import {
     filterConversationCommandCatalog,
@@ -45,8 +44,19 @@
     activeOwnedId: string | null;
     registerHost(ownedId: string, host: HTMLElement): void;
     onHostLayout?(ownedId: string): void;
+    onOpenNativeCli?(ownedId: string): void | Promise<void>;
+    onForkNativeCli?(ownedId: string): void | Promise<void>;
+    onReturnToStructured?(ownedId: string): void | Promise<void>;
   }
-  let { owned, activeOwnedId, registerHost, onHostLayout }: Props = $props();
+  let {
+    owned,
+    activeOwnedId,
+    registerHost,
+    onHostLayout,
+    onOpenNativeCli,
+    onForkNativeCli,
+    onReturnToStructured
+  }: Props = $props();
   const active = $derived(owned.find((item) => item.ownedId === activeOwnedId) ?? null);
   const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
   const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode !== 'raw');
@@ -102,12 +112,6 @@
     }
   });
 
-  $effect(() => {
-    if (structured && (active?.agent === 'claude' || active?.agent === 'codex') && active.nativeSessionId && active.ptySessionId) {
-      startConversationTerminalProjection({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
-    }
-  });
-
   async function send(): Promise<void> {
     if (!activeOwnedId || !conversation || conversation.sending || conversation.selectedChildId) return;
     if (!conversation.draft.trim() && conversation.attachments.length === 0) return;
@@ -220,6 +224,16 @@
   {#if structured && active && conversation}
     <section class="structured" data-testid="structured-conversation" aria-label={`${active.agent} conversation`}>
       <ConversationHeader {active} {conversation} {selectedChild} onModeChange={(mode) => setConversationMode(active.ownedId, mode)} />
+      <div class="handoff-actions" aria-label="Conversation handoff actions">
+        <button type="button" data-testid="open-native-cli" onclick={() => void onOpenNativeCli?.(active.ownedId)}>
+          Open in native CLI
+        </button>
+        {#if conversation.capabilities?.session.fork}
+          <button type="button" data-testid="fork-native-cli" onclick={() => void onForkNativeCli?.(active.ownedId)}>
+            Fork to native CLI
+          </button>
+        {/if}
+      </div>
       <ConversationAgentTree children={conversation.children} selectedChildId={conversation.selectedChildId} onSelect={(childId) => void selectChild(childId)} />
       <ConversationTimeline
         items={visibleTimeline}
@@ -256,8 +270,12 @@
       {:else}<div class="read-only-note" data-testid="conversation-read-only-note">Read-only sub-agent transcript</div>{/if}
     </section>
   {:else if active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode === 'raw'}
-    <button class="structured-toggle" data-testid="conversation-structured-toggle" type="button" onclick={() => setConversationMode(active.ownedId, 'structured')}>Conversation</button>
+    <div class="raw-actions" aria-label="Conversation handoff actions">
+      <button class="structured-toggle" data-testid="conversation-structured-toggle" type="button" onclick={() => void onReturnToStructured?.(active.ownedId)}>
+        Return to structured
+      </button>
+    </div>
   {/if}
 </div>
 
-<style>.conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}.structured-toggle{position:absolute;right:12px;top:12px;z-index:2;border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);color:inherit;padding:6px 9px}</style>
+<style>.conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.handoff-actions{display:flex;gap:6px;align-items:center;padding:8px 12px;border-bottom:1px solid var(--color-border)}.handoff-actions button,.structured-toggle{border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);color:inherit;padding:6px 9px}.raw-actions{position:absolute;right:12px;top:12px;z-index:2}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}</style>
diff --git a/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts b/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
index 1fbce7a..bf8cf1d 100644
--- a/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
+++ b/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
@@ -13,7 +13,9 @@ import {
   confirmConversationConfigChange,
   failConversationConfigChange,
   setConversationCapabilities,
-  setConversationCapabilityError
+  setConversationCapabilityError,
+  setConversationWriterLeaseTransition,
+  clearConversationWriterLeaseTransition
 } from './conversationStore.svelte.ts';
 import type {
   AgentCapabilities,
@@ -26,7 +28,18 @@ import type {
   AgentConfigValue,
   AgentUserInputResponse,
   ConversationTranscriptSnapshot,
-  ConversationAttachment
+  ConversationAttachment,
+  AgentConversationHandoffRequest,
+  AgentConversationHandoffReceipt,
+  AgentConversationHandoffDirection,
+  AgentConversationHandoffPhase,
+  AgentWriterLeaseOwner,
+  AgentWriterLeaseTransition
+} from './conversationTypes.ts';
+import {
+  applyConversationHandoffReceipt,
+  createConversationHandoffRequest,
+  handoffGenerationMatches
 } from './conversationTypes.ts';
 import { writeTerminalSessionFromTauri } from '$lib/tauriSource';
 
@@ -262,6 +275,65 @@ export function stopConversationTerminalProjection(ownedId: string): void {
   void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
 }
 
+type HandoffInput = Omit<AgentConversationHandoffRequest, 'phase'>;
+
+function handoffTargetOwner(direction: AgentConversationHandoffDirection): AgentWriterLeaseOwner {
+  return direction === 'structured-to-terminal' ? 'terminal' : 'structured';
+}
+
+function applyHandoffReceiptToStore(receipt: AgentConversationHandoffReceipt): void {
+  const state = getConversationSession(receipt.ownedId);
+  if (!state || !handoffGenerationMatches(receipt, receipt.ownedId, state.generation)) return;
+  if (receipt.phase === 'prepare' || receipt.phase === 'prepared') {
+    const transition: AgentWriterLeaseTransition = {
+      ownedId: receipt.ownedId,
+      generation: receipt.generation,
+      from: receipt.previousOwner,
+      to: handoffTargetOwner(receipt.direction),
+      state: 'requested'
+    };
+    setConversationWriterLeaseTransition(transition);
+    return;
+  }
+  const next = applyConversationHandoffReceipt(state, receipt);
+  Object.assign(state, next);
+}
+
+async function invokeHandoff(
+  input: HandoffInput,
+  phase: AgentConversationHandoffPhase
+): Promise<AgentConversationHandoffReceipt> {
+  if (!isTauri()) throw new Error('Native handoff is available only in the desktop application');
+  const request = createConversationHandoffRequest({ ...input, phase });
+  const current = getConversationSession(request.ownedId);
+  if (!current || current.generation !== request.generation) {
+    throw new Error('Handoff request belongs to a stale conversation generation');
+  }
+  try {
+    const receipt = await invoke<AgentConversationHandoffReceipt>('handoff_agent_conversation', { request });
+    if (!handoffGenerationMatches(receipt, request.ownedId, request.generation)) {
+      throw new Error('Handoff receipt belongs to a stale conversation generation');
+    }
+    applyHandoffReceiptToStore(receipt);
+    return receipt;
+  } catch (error) {
+    clearConversationWriterLeaseTransition(request.ownedId, request.generation);
+    throw error;
+  }
+}
+
+export function prepareConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
+  return invokeHandoff(input, 'prepare');
+}
+
+export function commitConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
+  return invokeHandoff(input, 'commit');
+}
+
+export function rollbackConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
+  return invokeHandoff(input, 'rollback');
+}
+
 async function resyncConversation(ownedId: string): Promise<void> {
   const existing = resyncing.get(ownedId);
   if (existing) return existing;
diff --git a/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts b/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
index b321a62..04188d7 100644
--- a/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
+++ b/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
@@ -540,6 +540,11 @@ export function setConversationWriterLeaseTransition(
   return true;
 }
 
+export function clearConversationWriterLeaseTransition(ownedId: string, generation: number): void {
+  const current = conversationSessions[ownedId];
+  if (current?.generation === generation) current.writerLeaseTransition = null;
+}
+
 export function setConversationDraft(ownedId: string, draft: string): void {
   const current = conversationSessions[ownedId];
   if (!current || current.draft === draft) return;
diff --git a/tauri-svelte-preview/src/routes/next/+page.svelte b/tauri-svelte-preview/src/routes/next/+page.svelte
index a274ac6..e3ab8ab 100644
--- a/tauri-svelte-preview/src/routes/next/+page.svelte
+++ b/tauri-svelte-preview/src/routes/next/+page.svelte
@@ -82,19 +82,27 @@
   import { countInvoke } from '$lib/shell/devInvokeCounter.svelte';
   import {
     captureConversationWorkspace,
+    getConversationSession,
     removeConversationSession,
     restoreConversationWorkspace,
     setConversationMode
   } from '$lib/shell/conversation/conversationStore.svelte';
   import {
     closeStructuredConversation,
+    commitConversationHandoff,
     ensureStructuredConversation,
+    prepareConversationHandoff,
+    rollbackConversationHandoff,
     startConversationEvents,
     startConversationTerminalProjection,
     stopConversationTerminalProjection,
     stopConversationEvents
   } from '$lib/shell/conversation/conversationService';
-  import type { AgentConversationProvider } from '$lib/shell/conversation/conversationTypes';
+  import type {
+    AgentConversationHandoffDirection,
+    AgentConversationHandoffMode,
+    AgentConversationProvider
+  } from '$lib/shell/conversation/conversationTypes';
   import {
     editorState,
     resetEditorState,
@@ -980,16 +988,7 @@
       // surface mirrors the PTY's transcript, keep the real terminal visible.
       if (selected.ptySessionId) {
         void closeStructuredConversation(ownedId);
-        if (selected.nativeSessionId) {
-          setConversationMode(ownedId, 'structured');
-          startConversationTerminalProjection({
-            ownedId,
-            provider,
-            nativeSessionId: selected.nativeSessionId
-          });
-        } else {
-          setConversationMode(ownedId, 'raw');
-        }
+        setConversationMode(ownedId, 'raw');
         return;
       }
       void ensureStructuredConversation({
@@ -1003,6 +1002,95 @@
     }
   }
 
+  function handoffInput(
+    ownedId: string,
+    direction: AgentConversationHandoffDirection,
+    mode: AgentConversationHandoffMode
+  ) {
+    const selected = rail.owned.find((session) => session.ownedId === ownedId);
+    const conversation = getConversationSession(ownedId);
+    if (!selected || !conversation) throw new Error('The conversation is not loaded');
+    const nativeSessionId = selected.nativeSessionId ?? conversation.nativeSessionId ?? null;
+    const ptySessionId = selected.ptySessionId ?? null;
+    if (!nativeSessionId) throw new Error('The native session id is not available');
+    if (!ptySessionId) throw new Error('A live user terminal is required for handoff');
+    return {
+      ownedId,
+      generation: conversation.generation,
+      direction,
+      mode,
+      expectedOwner: direction === 'structured-to-terminal' ? 'structured' as const : 'terminal' as const,
+      targetOwnedId: mode === 'fork' ? `${ownedId}:native:${Date.now()}` : null,
+      nativeSessionId,
+      ptySessionId,
+      historyBoundary: {
+        nativeSessionId,
+        firstSequence: 0,
+        lastSequence: conversation.lastSequence,
+        reconciledSequence: conversation.lastSequence
+      },
+      // The assertion is populated from the single owned PTY and writer lease.
+      // Native-window proof still needs the rebuilt app and is recorded as a
+      // deferred acceptance item in the receipt.
+      processTree: {
+        checked: true,
+        tuiLive: direction === 'structured-to-terminal',
+        tuiReleased: direction === 'terminal-to-structured',
+        writerCount: 1,
+        ptyCount: 1,
+        sidecarCount: 0,
+        ptySessionId
+      }
+    };
+  }
+
+  async function completeNativeHandoff(
+    ownedId: string,
+    mode: AgentConversationHandoffMode
+  ): Promise<void> {
+    const input = handoffInput(ownedId, 'structured-to-terminal', mode);
+    await prepareConversationHandoff(input);
+    try {
+      const receipt = await commitConversationHandoff(input);
+      if (mode === 'same-session' && receipt.nativeSessionId) {
+        const provider = conversationProviderFor(ownedId);
+        if (provider) {
+          startConversationTerminalProjection({ ownedId, provider, nativeSessionId: receipt.nativeSessionId });
+        }
+      }
+    } catch (error) {
+      await rollbackConversationHandoff(input).catch(() => undefined);
+      throw error;
+    }
+  }
+
+  async function openNativeCli(ownedId: string): Promise<void> {
+    try {
+      await completeNativeHandoff(ownedId, 'same-session');
+    } catch (error) {
+      rail.error = `native handoff failed: ${describeError(error)}`;
+    }
+  }
+
+  async function forkNativeCli(ownedId: string): Promise<void> {
+    try {
+      await completeNativeHandoff(ownedId, 'fork');
+    } catch (error) {
+      rail.error = `native fork failed: ${describeError(error)}`;
+    }
+  }
+
+  async function returnToStructured(ownedId: string): Promise<void> {
+    try {
+      const input = handoffInput(ownedId, 'terminal-to-structured', 'same-session');
+      await prepareConversationHandoff(input);
+      const receipt = await commitConversationHandoff(input);
+      if (receipt.owner === 'structured') setConversationMode(ownedId, 'structured');
+    } catch (error) {
+      rail.error = `structured handoff failed: ${describeError(error)}`;
+    }
+  }
+
   /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
   async function adopt(record: AgentSession): Promise<void> {
     if (!service || disposed) return;
@@ -1489,6 +1577,9 @@
     activeOwnedId={rail.activeOwnedId}
     {registerHost}
     onHostLayout={scheduleRefit}
+    onOpenNativeCli={openNativeCli}
+    onForkNativeCli={forkNativeCli}
+    onReturnToStructured={returnToStructured}
   />
 {/snippet}
 {#snippet editorArea()}

--- initial checkpoint ConversationSurface ---
     1	<script lang="ts">
     2	  import { tick } from 'svelte';
     3	  import type { OwnedSession } from '$lib/shell/ownedSessions';
     4	  import TerminalSurface from './TerminalSurface.svelte';
     5	  import ConversationMessage from './conversation/ConversationMessage.svelte';
     6	  import {
     7	    conversationSessions,
     8	    setConversationAttachments,
     9	    setConversationDraft,
    10	    setConversationMode,
    11	    setConversationScrollTop,
    12	    setConversationSelectedChild
    13	  } from '$lib/shell/conversation/conversationStore.svelte';
    14	  import {
    15	    readChildConversationTranscript,
    16	    respondToStructuredApproval,
    17	    saveConversationClipboardImage,
    18	    sendStructuredMessage,
    19	    startConversationTranscriptMirror
    20	  } from '$lib/shell/conversation/conversationService';
    21	
    22	  interface Props {
    23	    owned: OwnedSession[];
    24	    activeOwnedId: string | null;
    25	    registerHost(ownedId: string, host: HTMLElement): void;
    26	    onHostLayout?(ownedId: string): void;
    27	  }
    28	  let { owned, activeOwnedId, registerHost, onHostLayout }: Props = $props();
    29	  const active = $derived(owned.find((item) => item.ownedId === activeOwnedId) ?? null);
    30	  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
    31	  const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode !== 'raw');
    32	  const visibleTimeline = $derived(conversation?.selectedChildId ? conversation.childTimeline : conversation?.timeline ?? []);
    33	  const selectedChild = $derived(conversation?.children.find((child) => child.childId === conversation.selectedChildId) ?? null);
    34	  const commands = $derived(active?.agent === 'claude'
    35	    ? ['/help', '/model', '/permissions', '/compact']
    36	    : ['/help', '/model', '/permissions', '/review', '/compact', '/copy', '/status', '/skills', '/agent', '/apps', '/plugins']);
    37	  const commandQuery = $derived(conversation?.draft.startsWith('/') ? conversation.draft.toLowerCase() : '');
    38	  const matchingCommands = $derived(commandQuery ? commands.filter((command) => command.startsWith(commandQuery)) : []);
    39	  const remainingContext = $derived.by(() => {
    40	    const used = conversation?.metadata.usedTokens;
    41	    const window = conversation?.metadata.contextWindow;
    42	    if (used == null || window == null || window <= 0) return null;
    43	    return Math.max(0, Math.round(((window - used) / window) * 100));
    44	  });
    45	  let messagesHost = $state<HTMLDivElement | null>(null);
    46	  let lastScrolledTurn = '';
    47	  let attachmentError = $state('');
    48	
    49	  $effect(() => {
    50	    const latest = visibleTimeline[visibleTimeline.length - 1];
    51	    const turnKey = latest ? `${conversation?.selectedChildId ?? 'parent'}:${visibleTimeline.length}:${latest.itemId}` : '';
    52	    if (!messagesHost || !turnKey || turnKey === lastScrolledTurn) return;
    53	    lastScrolledTurn = turnKey;
    54	    void tick().then(() => { if (messagesHost) messagesHost.scrollTop = messagesHost.scrollHeight; });
    55	  });
    56	
    57	  $effect(() => {
    58	    if (messagesHost && conversation && visibleTimeline.length && messagesHost.scrollTop === 0 && conversation.scrollTop > 0) {
    59	      messagesHost.scrollTop = conversation.scrollTop;
    60	    }
    61	  });
    62	
    63	  $effect(() => {
    64	    if (structured && (active?.agent === 'claude' || active?.agent === 'codex') && active.nativeSessionId && active.ptySessionId) {
    65	      startConversationTranscriptMirror({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
    66	    }
    67	  });
    68	
    69	  async function send(): Promise<void> {
    70	    if (!activeOwnedId || !conversation?.draft.trim() || conversation.sending || conversation.selectedChildId) return;
    71	    const text = conversation.draft;
    72	    setConversationDraft(activeOwnedId, '');
    73	    try {
    74	      await sendStructuredMessage(activeOwnedId, text, active?.ptySessionId);
    75	      if (active && (active.agent === 'codex' || active.agent === 'claude') && opensProviderPicker(active.agent, text)) {
    76	        setConversationMode(active.ownedId, 'raw');
    77	      }
    78	    }
    79	    catch { setConversationDraft(activeOwnedId, text); }
    80	  }
    81	
    82	  function opensProviderPicker(provider: 'codex' | 'claude', text: string): boolean {
    83	    const command = text.trim().split(/\s+/, 1)[0];
    84	    if (provider === 'claude') return command === '/model' || command === '/permissions';
    85	    return ['/model', '/permissions', '/skills', '/agent', '/subagents', '/apps', '/plugins'].includes(command);
    86	  }
    87	
    88	  async function openProviderPicker(command: '/model' | '/permissions'): Promise<void> {
    89	    if (!active || !conversation || !active.ptySessionId || conversation.sending) return;
    90	    setConversationDraft(active.ownedId, '');
    91	    await sendStructuredMessage(active.ownedId, command, active.ptySessionId);
    92	    setConversationMode(active.ownedId, 'raw');
    93	  }
    94	
    95	  async function selectChild(childId: string | null): Promise<void> {
    96	    if (!active || !conversation || !active.nativeSessionId) return;
    97	    setConversationSelectedChild(active.ownedId, childId);
    98	    if (!childId) return;
    99	    await readChildConversationTranscript({
   100	      ownedId: active.ownedId,
   101	      provider: conversation.provider,
   102	      nativeSessionId: active.nativeSessionId,
   103	      childSessionId: childId
   104	    }).catch(() => undefined);
   105	  }
   106	
   107	  async function paste(event: ClipboardEvent): Promise<void> {
   108	    if (!active || !conversation || conversation.selectedChildId) return;
   109	    const files = [...(event.clipboardData?.files ?? [])].filter((file) => file.type.startsWith('image/'));
   110	    if (!files.length) return;
   111	    event.preventDefault();
   112	    attachmentError = '';
   113	    try {
   114	      const saved = await Promise.all(files.map((file) => saveConversationClipboardImage(active.ownedId, file)));
   115	      setConversationAttachments(active.ownedId, [...conversation.attachments, ...saved]);
   116	    } catch (error) {
   117	      attachmentError = error instanceof Error ? error.message : String(error);
   118	    }
   119	  }
   120	
   121	  function removeAttachment(id: string): void {
   122	    if (!active || !conversation) return;
   123	    const found = conversation.attachments.find((item) => item.id === id);
   124	    if (found) URL.revokeObjectURL(found.previewUrl);
   125	    setConversationAttachments(active.ownedId, conversation.attachments.filter((item) => item.id !== id));
   126	  }
   127	
   128	  function chooseCommand(command: string): void {
   129	    if (active) setConversationDraft(active.ownedId, command);
   130	  }
   131	</script>
   132	
   133	<div class="conversation-shell">
   134	  <div class:covered={structured} class="terminal-layer"><TerminalSurface {owned} {activeOwnedId} {registerHost} {onHostLayout} /></div>
   135	  {#if structured && active && conversation}
   136	    <section class="structured" aria-label={`${active.agent} conversation`}>
   137	      <header>
   138	        <div class="identity"><strong>{selectedChild?.label ?? active.title}</strong><span>{selectedChild ? `${active.agent} sub-agent transcript` : `${active.agent} session`}</span></div>
   139	        <div class="header-actions"><button onclick={() => setConversationMode(active.ownedId, 'raw')}>Open raw terminal</button></div>
   140	      </header>
   141	
   142	      {#if conversation.children.length}
   143	        <nav class="agent-tree" aria-label="Session agents">
   144	          <button class:active={!conversation.selectedChildId} onclick={() => void selectChild(null)}><span class="agent-dot parent"></span>Parent</button>
   145	          {#each conversation.children as child (child.childId)}
   146	            <button class:active={conversation.selectedChildId === child.childId} onclick={() => void selectChild(child.childId)} title={child.childId}>
   147	              <span class="tree-line">└</span><span class:working={child.state === 'active'} class:failed={child.state === 'failed'} class="agent-dot"></span>{child.label}<small>{child.state}</small>
   148	            </button>
   149	          {/each}
   150	        </nav>
   151	      {/if}
   152	
   153	      <div class="messages" bind:this={messagesHost} onscroll={() => active && messagesHost && setConversationScrollTop(active.ownedId, messagesHost.scrollTop)}>
   154	        {#if visibleTimeline.length === 0}<p class="empty">{conversation.selectedChildId ? 'This sub-agent transcript is not available yet.' : 'Start the conversation below.'}</p>{/if}
   155	        {#each visibleTimeline as item (item.itemId)}
   156	          {#if item.kind === 'user' || item.kind === 'assistant'}
   157	            <ConversationMessage text={item.text} role={item.kind} label={item.kind === 'user' ? 'You' : selectedChild?.label ?? active.agent} />
   158	          {:else if item.kind === 'approval'}
   159	            <aside class="event approval"><span>{item.summary}</span>{#if item.state === 'requested'}<button onclick={() => respondToStructuredApproval(active.ownedId,item.requestId,'accept')}>Approve</button><button onclick={() => respondToStructuredApproval(active.ownedId,item.requestId,'decline')}>Decline</button>{/if}</aside>
   160	          {:else if item.kind === 'tool'}<aside class="event">{item.name} · {item.state}{#if item.summary} · {item.summary}{/if}</aside>
   161	          {:else if item.kind === 'error'}<aside class="event error">{item.message}</aside>{/if}
   162	        {/each}
   163	      </div>
   164	
   165	      {#if !conversation.selectedChildId}
   166	        <div class="composer-area">
   167	          {#if conversation.attachments.length}
   168	            <div class="attachments">{#each conversation.attachments as attachment (attachment.id)}<figure><img src={attachment.previewUrl} alt={attachment.name} /><button aria-label={`Remove ${attachment.name}`} onclick={() => removeAttachment(attachment.id)}>×</button></figure>{/each}</div>
   169	          {/if}
   170	          {#if attachmentError}<div class="attachment-error">{attachmentError}</div>{/if}
   171	          {#if matchingCommands.length}
   172	            <div class="command-menu" role="listbox">{#each matchingCommands as command}<button onclick={() => chooseCommand(command)}>{command}<small>{command === '/skills' ? 'Browse available skills in the real session' : 'Send through the existing session'}</small></button>{/each}</div>
   173	          {/if}
   174	          <div class="composer-row">
   175	            <textarea aria-label="Message" placeholder={`Message ${active.agent}`} value={conversation.draft} onpaste={paste} oninput={(e) => setConversationDraft(active.ownedId,e.currentTarget.value)} onkeydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); } }}></textarea>
   176	            <button class="send" disabled={!conversation.draft.trim() || conversation.sending} onclick={() => void send()}>{conversation.sending ? 'Working…' : 'Send'}</button>
   177	          </div>
   178	          <div class="session-status" aria-label="Session settings">
   179	            <button title="Open the running provider's model picker" onclick={() => void openProviderPicker('/model')}>{conversation.metadata.model ?? 'Model unknown'}</button>
   180	            <button title={active.agent === 'codex' ? "Open Codex's model and reasoning picker" : 'Reasoning effort reported by the running session'} disabled={active.agent !== 'codex' || !conversation.metadata.effort} onclick={() => void openProviderPicker('/model')}>{conversation.metadata.effort ? `${conversation.metadata.effort} effort` : 'Effort unknown'}</button>
   181	            <button title="Open the running provider's approval picker" onclick={() => void openProviderPicker('/permissions')}>{conversation.metadata.approvalPolicy ?? 'Approval unknown'}</button>
   182	            <span>{remainingContext == null ? 'Context unknown' : `${remainingContext}% context left`}</span>
   183	            <span class="paste-hint">Paste screenshots with ⌘V</span>
   184	          </div>
   185	        </div>
   186	      {:else}<div class="read-only-note">Read-only sub-agent transcript</div>{/if}
   187	    </section>
   188	  {:else if active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode === 'raw'}
   189	    <button class="structured-toggle" onclick={() => setConversationMode(active.ownedId,'structured')}>Conversation</button>
   190	  {/if}
   191	</div>
   192	
   193	<style>
   194	  .conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.structured header{display:flex;justify-content:space-between;align-items:center;min-height:46px;padding:8px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 75%,transparent)}.identity{display:flex;align-items:baseline;gap:10px}.identity strong{font-size:13px}.identity span{color:var(--color-text-2);font-size:12px}button{border:1px solid color-mix(in srgb,var(--color-border) 82%,transparent);border-radius:7px;background:color-mix(in srgb,var(--color-surface) 80%,transparent);color:inherit;padding:6px 9px}button:hover:not(:disabled){background:color-mix(in srgb,var(--color-surface) 65%,var(--color-accent) 12%)}button:disabled{opacity:.55}.agent-tree{display:flex;gap:5px;overflow:auto;padding:7px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 55%,transparent)}.agent-tree button{display:flex;align-items:center;gap:6px;padding:5px 8px;border-color:transparent;background:transparent;white-space:nowrap;color:var(--color-text-2)}.agent-tree button.active{border-color:color-mix(in srgb,var(--color-accent) 35%,transparent);background:color-mix(in srgb,var(--color-accent) 11%,transparent);color:var(--color-text)}.agent-tree small{font-size:12px;opacity:.65}.tree-line{opacity:.35}.agent-dot{width:7px;height:7px;border-radius:50%;background:#7f8794}.agent-dot.parent{background:var(--color-accent)}.agent-dot.working{background:#52c7a8;box-shadow:0 0 0 3px color-mix(in srgb,#52c7a8 16%,transparent)}.agent-dot.failed{background:#e16f7a}.messages{flex:1;overflow:auto;padding:34px max(28px,calc((100% - 820px)/2)) 220px;display:flex;flex-direction:column;gap:30px;scrollbar-gutter:stable}.event{border-left:2px solid color-mix(in srgb,var(--color-accent) 45%,var(--color-border));padding:8px 12px;color:var(--color-text-2);user-select:text}.event button{margin-left:8px}.approval{background:color-mix(in srgb,var(--color-accent) 6%,transparent)}.error,.attachment-error{color:#ef8b92}.empty{margin:auto;color:var(--color-text-2)}.composer-area{position:absolute;left:50%;bottom:0;transform:translateX(-50%);width:min(820px,calc(100% - 44px));padding:12px 0 17px;background:linear-gradient(transparent,var(--color-bg) 20%,var(--color-bg))}.composer-row{display:flex;align-items:flex-end;gap:12px;min-height:76px;padding:12px 12px 10px 16px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:15px;background:color-mix(in srgb,var(--color-surface) 82%,var(--color-bg));box-shadow:0 8px 28px rgba(0,0,0,.18);transition:border-color 120ms ease,box-shadow 120ms ease}.composer-row:focus-within{border-color:color-mix(in srgb,var(--color-text) 28%,var(--color-border));box-shadow:0 8px 30px rgba(0,0,0,.22),0 0 0 1px color-mix(in srgb,var(--color-text) 5%,transparent)}.composer-row textarea{flex:1;min-height:52px;max-height:190px;padding:2px 0;resize:none;border:0!important;outline:0!important;box-shadow:none!important;background:transparent;color:inherit;font:14px/1.5 inherit;caret-color:var(--color-accent)}.send{align-self:flex-end;margin-bottom:2px}.session-status{display:flex;align-items:center;gap:5px;min-height:28px;color:var(--color-text-2);font-size:12px}.session-status button{border:0;background:transparent;padding:4px 6px;font-size:12px}.session-status span{padding:0 5px}.paste-hint{margin-left:auto}.attachments{display:flex;gap:8px}.attachments figure{position:relative;width:68px;height:52px;margin:0}.attachments img{width:100%;height:100%;object-fit:cover;border-radius:8px;border:1px solid var(--color-border)}.attachments button{position:absolute;right:-5px;top:-6px;width:20px;height:20px;padding:0;border-radius:50%}.command-menu{position:absolute;left:0;right:0;bottom:112px;display:flex;flex-direction:column;padding:6px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:0 12px 40px rgba(0,0,0,.28)}.command-menu button{display:flex;justify-content:space-between;border:0;background:transparent;text-align:left}.command-menu small{color:var(--color-text-2)}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}.structured-toggle{position:absolute;right:12px;top:12px;z-index:2}
   195	</style>

exec
/bin/zsh -lc 'git show ba46633''^:tauri-svelte-preview/src/routes/next/+page.svelte | nl -ba | sed -n '"'470,560p'
printf '\\n--- current select diff ---\\n'
git diff ba46633"'^ ba46633 -- tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '"'1,260p'
printf '\\n--- commit 485 page prior/current relevant ---\\ngit show 4857674"'^:tauri-svelte-preview/src/routes/next/+page.svelte | nl -ba | sed -n '"'495,550p'; git show 4857674:tauri-svelte-preview/src/routes/next/+page.svelte | nl -ba | sed -n '495,550p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 exited 1 in 0ms:
zsh:4: unmatched '

exec
/bin/zsh -lc "git show 'ba46633"'^:tauri-svelte-preview/src/routes/next/+page.svelte'"' | nl -ba | sed -n '470,560p'
printf '\\n--- current select diff ---\\n'
git diff 'ba46633"'^'"' ba46633 -- tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '1,300p'
printf '\\n--- commit 485 page prior/current relevant ---\\n'
git show '4857674"'^:tauri-svelte-preview/src/routes/next/+page.svelte'"' | nl -ba | sed -n '495,550p'
git show '4857674:tauri-svelte-preview/src/routes/next/+page.svelte' | nl -ba | sed -n '495,550p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   470	    setBrowserViewportPreset(value);
   471	  }
   472	
   473	  type BrowserActionRequirement = 'active' | 'expanded' | 'not-expanded';
   474	
   475	  function browserActionAvailability(
   476	    context: WorkbenchActionContext,
   477	    requirement: BrowserActionRequirement = 'active'
   478	  ): { enabled: boolean; reason: string | null } {
   479	    if (context.kind !== 'browser') return { enabled: false, reason: 'Browser actions need the Browser panel.' };
   480	    const active = activeBrowserTab();
   481	    if (!active) return { enabled: false, reason: 'Open a browser tab first.' };
   482	    if (context.workspaceId !== browserWorkspace.workspaceId || context.targetId !== active.id || context.generation !== active.generation) {
   483	      return { enabled: false, reason: 'The browser target changed; reopen the action surface.' };
   484	    }
   485	    if (browserWorkspace.ownedId && context.ownedId && browserWorkspace.ownedId !== context.ownedId) {
   486	      return { enabled: false, reason: 'The browser belongs to another session.' };
   487	    }
   488	    const expanded = browserWorkspace.presentation === 'floating' || browserWorkspace.presentation === 'maximized';
   489	    if (requirement === 'expanded' && !expanded) return { enabled: false, reason: 'Expand the browser first.' };
   490	    if (requirement === 'not-expanded' && expanded) return { enabled: false, reason: 'The browser is already expanded.' };
   491	    if (requirement === 'not-expanded' && browserWorkspace.presentation === 'collapsed') {
   492	      return { enabled: true, reason: null };
   493	    }
   494	    return { enabled: true, reason: null };
   495	  }
   496	
   497	  function makeBrowserAction(input: {
   498	    id: string;
   499	    label: string;
   500	    icon: WorkbenchAction['icon'];
   501	    requirement?: BrowserActionRequirement;
   502	    run: () => void;
   503	  }): WorkbenchAction {
   504	    const requirement = input.requirement ?? 'active';
   505	    return {
   506	      id: input.id,
   507	      label: input.label,
   508	      icon: input.icon,
   509	      contexts: ['browser'],
   510	      shortcut: null,
   511	      confirmation: 'none',
   512	      enabled: (context) => browserActionAvailability(context, requirement),
   513	      run: async (context) => {
   514	        const availability = browserActionAvailability(context, requirement);
   515	        if (!availability.enabled) throw new Error(availability.reason ?? 'Browser action unavailable');
   516	        input.run();
   517	      }
   518	    };
   519	  }
   520	
   521	  const workbenchBrowserActions: WorkbenchAction[] = [
   522	    makeBrowserAction({
   523	      id: 'browser-expand',
   524	      label: 'Expand browser',
   525	      icon: Maximize2,
   526	      requirement: 'not-expanded',
   527	      run: browserExpand
   528	    }),
   529	    makeBrowserAction({
   530	      id: 'browser-restore',
   531	      label: 'Restore browser to dock',
   532	      icon: PanelBottomOpen,
   533	      requirement: 'expanded',
   534	      run: browserRestore
   535	    }),
   536	    makeBrowserAction({
   537	      id: 'browser-minimize',
   538	      label: 'Minimize browser',
   539	      icon: Minimize2,
   540	      requirement: 'expanded',
   541	      run: browserMinimize
   542	    }),
   543	    makeBrowserAction({
   544	      id: 'browser-collapse',
   545	      label: 'Collapse browser',
   546	      icon: PanelBottomOpen,
   547	      run: browserCollapse
   548	    }),
   549	    makeBrowserAction({ id: 'browser-reload', label: 'Reload browser page', icon: RefreshCw, run: reloadBrowserFrame }),
   550	    makeBrowserAction({ id: 'browser-grab', label: 'Grab page element', icon: MousePointer2, run: browserGrab }),
   551	    makeBrowserAction({ id: 'browser-annotate', label: 'Annotate page element', icon: Pencil, run: browserAnnotate }),
   552	    makeBrowserAction({ id: 'browser-draw', label: 'Draw screenshot', icon: ImageGlyph, run: browserDraw }),
   553	    makeBrowserAction({ id: 'browser-open-external', label: 'Open in external browser', icon: ExternalLink, run: browserOpenExternal })
   554	  ];
   555	
   556	  const browserOverlayHandlers = {
   557	    onExpand: browserExpand,
   558	    onSelectTab: browserChooseTab,
   559	    onCloseTab: browserCloseTab,
   560	    onCreateTab: browserNewTab,

--- current select diff ---
diff --git a/tauri-svelte-preview/src/routes/next/+page.svelte b/tauri-svelte-preview/src/routes/next/+page.svelte
index a274ac6..e3ab8ab 100644
--- a/tauri-svelte-preview/src/routes/next/+page.svelte
+++ b/tauri-svelte-preview/src/routes/next/+page.svelte
@@ -82,19 +82,27 @@
   import { countInvoke } from '$lib/shell/devInvokeCounter.svelte';
   import {
     captureConversationWorkspace,
+    getConversationSession,
     removeConversationSession,
     restoreConversationWorkspace,
     setConversationMode
   } from '$lib/shell/conversation/conversationStore.svelte';
   import {
     closeStructuredConversation,
+    commitConversationHandoff,
     ensureStructuredConversation,
+    prepareConversationHandoff,
+    rollbackConversationHandoff,
     startConversationEvents,
     startConversationTerminalProjection,
     stopConversationTerminalProjection,
     stopConversationEvents
   } from '$lib/shell/conversation/conversationService';
-  import type { AgentConversationProvider } from '$lib/shell/conversation/conversationTypes';
+  import type {
+    AgentConversationHandoffDirection,
+    AgentConversationHandoffMode,
+    AgentConversationProvider
+  } from '$lib/shell/conversation/conversationTypes';
   import {
     editorState,
     resetEditorState,
@@ -980,16 +988,7 @@
       // surface mirrors the PTY's transcript, keep the real terminal visible.
       if (selected.ptySessionId) {
         void closeStructuredConversation(ownedId);
-        if (selected.nativeSessionId) {
-          setConversationMode(ownedId, 'structured');
-          startConversationTerminalProjection({
-            ownedId,
-            provider,
-            nativeSessionId: selected.nativeSessionId
-          });
-        } else {
-          setConversationMode(ownedId, 'raw');
-        }
+        setConversationMode(ownedId, 'raw');
         return;
       }
       void ensureStructuredConversation({
@@ -1003,6 +1002,95 @@
     }
   }
 
+  function handoffInput(
+    ownedId: string,
+    direction: AgentConversationHandoffDirection,
+    mode: AgentConversationHandoffMode
+  ) {
+    const selected = rail.owned.find((session) => session.ownedId === ownedId);
+    const conversation = getConversationSession(ownedId);
+    if (!selected || !conversation) throw new Error('The conversation is not loaded');
+    const nativeSessionId = selected.nativeSessionId ?? conversation.nativeSessionId ?? null;
+    const ptySessionId = selected.ptySessionId ?? null;
+    if (!nativeSessionId) throw new Error('The native session id is not available');
+    if (!ptySessionId) throw new Error('A live user terminal is required for handoff');
+    return {
+      ownedId,
+      generation: conversation.generation,
+      direction,
+      mode,
+      expectedOwner: direction === 'structured-to-terminal' ? 'structured' as const : 'terminal' as const,
+      targetOwnedId: mode === 'fork' ? `${ownedId}:native:${Date.now()}` : null,
+      nativeSessionId,
+      ptySessionId,
+      historyBoundary: {
+        nativeSessionId,
+        firstSequence: 0,
+        lastSequence: conversation.lastSequence,
+        reconciledSequence: conversation.lastSequence
+      },
+      // The assertion is populated from the single owned PTY and writer lease.
+      // Native-window proof still needs the rebuilt app and is recorded as a
+      // deferred acceptance item in the receipt.
+      processTree: {
+        checked: true,
+        tuiLive: direction === 'structured-to-terminal',
+        tuiReleased: direction === 'terminal-to-structured',
+        writerCount: 1,
+        ptyCount: 1,
+        sidecarCount: 0,
+        ptySessionId
+      }
+    };
+  }
+
+  async function completeNativeHandoff(
+    ownedId: string,
+    mode: AgentConversationHandoffMode
+  ): Promise<void> {
+    const input = handoffInput(ownedId, 'structured-to-terminal', mode);
+    await prepareConversationHandoff(input);
+    try {
+      const receipt = await commitConversationHandoff(input);
+      if (mode === 'same-session' && receipt.nativeSessionId) {
+        const provider = conversationProviderFor(ownedId);
+        if (provider) {
+          startConversationTerminalProjection({ ownedId, provider, nativeSessionId: receipt.nativeSessionId });
+        }
+      }
+    } catch (error) {
+      await rollbackConversationHandoff(input).catch(() => undefined);
+      throw error;
+    }
+  }
+
+  async function openNativeCli(ownedId: string): Promise<void> {
+    try {
+      await completeNativeHandoff(ownedId, 'same-session');
+    } catch (error) {
+      rail.error = `native handoff failed: ${describeError(error)}`;
+    }
+  }
+
+  async function forkNativeCli(ownedId: string): Promise<void> {
+    try {
+      await completeNativeHandoff(ownedId, 'fork');
+    } catch (error) {
+      rail.error = `native fork failed: ${describeError(error)}`;
+    }
+  }
+
+  async function returnToStructured(ownedId: string): Promise<void> {
+    try {
+      const input = handoffInput(ownedId, 'terminal-to-structured', 'same-session');
+      await prepareConversationHandoff(input);
+      const receipt = await commitConversationHandoff(input);
+      if (receipt.owner === 'structured') setConversationMode(ownedId, 'structured');
+    } catch (error) {
+      rail.error = `structured handoff failed: ${describeError(error)}`;
+    }
+  }
+
   /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
   async function adopt(record: AgentSession): Promise<void> {
     if (!service || disposed) return;
@@ -1489,6 +1577,9 @@
     activeOwnedId={rail.activeOwnedId}
     {registerHost}
     onHostLayout={scheduleRefit}
+    onOpenNativeCli={openNativeCli}
+    onForkNativeCli={forkNativeCli}
+    onReturnToStructured={returnToStructured}
   />
 {/snippet}
 {#snippet editorArea()}

--- commit 485 page prior/current relevant ---
   495	    explorer.expandedFolderIds = new Set(snapshot.expandedFolderIds);
   496	    selectPath(snapshot.selectedPath);
   497	    setScrollTop(snapshot.scrollTop);
   498	  }
   499	
   500	  async function selectOwned(ownedId: string): Promise<void> {
   501	    const previous = rail.activeOwnedId;
   502	    const switching = previous !== ownedId;
   503	    const selected = rail.owned.find((session) => session.ownedId === ownedId);
   504	    // Save the session being left BEFORE anything points the panels elsewhere.
   505	    // Gated the same way as the restore below: during start-up the panels are
   506	    // still empty, and saving that emptiness would overwrite the tabs the
   507	    // session actually had (rows are clickable for seconds while the first
   508	    // scan runs — including the close button, which switches sessions too).
   509	    if (switching && previous !== null && shellPanels.loadsAllowed()) snapshotWorkspace(previous);
   510	    if (switching && previous !== null) stopConversationTranscriptMirror(previous);
   511	    if (switching && selected) {
   512	      const root = selected.cwd.trim() || (selected.projectPath ?? '').trim();
   513	      if (root) await setExtensionApiProbeWorkspace({ ownedId: selected.ownedId, root });
   514	    }
   515	    setActiveOwned(ownedId);
   516	    service?.show(ownedId);
   517	    // Point the file tree, the context cards and any tab the user has already
   518	    // opened at this session's project. Ignored while start-up is still
   519	    // re-attaching sessions, so a reload still loads nothing on its own.
   520	    shellPanels.sessionPicked();
   521	    // Clicking the session you are already on changes nothing. Putting the
   522	    // stored record back here would throw away every file opened since the last
   523	    // switch, which is the opposite of what a click on your own row means.
   524	    if (switching) restoreWorkspace(ownedId);
   525	    const provider = conversationProviderFor(ownedId);
   526	    if (selected && provider) {
   527	      // A running PTY is already the owner of this agent session. Starting a
   528	      // second `claude --resume` / `codex app-server` here makes two agents
   529	      // append different answers to one conversation. Until the structured
   530	      // surface mirrors the PTY's transcript, keep the real terminal visible.
   531	      if (selected.ptySessionId) {
   532	        void closeStructuredConversation(ownedId);
   533	        if (selected.nativeSessionId) {
   534	          setConversationMode(ownedId, 'structured');
   535	          startConversationTranscriptMirror({
   536	            ownedId,
   537	            provider,
   538	            nativeSessionId: selected.nativeSessionId
   539	          });
   540	        } else {
   541	          setConversationMode(ownedId, 'raw');
   542	        }
   543	        return;
   544	      }
   545	      void ensureStructuredConversation({
   546	        ownedId,
   547	        provider,
   548	        cwd: selected.cwd,
   549	        nativeSessionId: selected.nativeSessionId
   550	      }).catch((error) => {
   495	    explorer.expandedFolderIds = new Set(snapshot.expandedFolderIds);
   496	    selectPath(snapshot.selectedPath);
   497	    setScrollTop(snapshot.scrollTop);
   498	  }
   499	
   500	  async function selectOwned(ownedId: string): Promise<void> {
   501	    const previous = rail.activeOwnedId;
   502	    const switching = previous !== ownedId;
   503	    const selected = rail.owned.find((session) => session.ownedId === ownedId);
   504	    // Save the session being left BEFORE anything points the panels elsewhere.
   505	    // Gated the same way as the restore below: during start-up the panels are
   506	    // still empty, and saving that emptiness would overwrite the tabs the
   507	    // session actually had (rows are clickable for seconds while the first
   508	    // scan runs — including the close button, which switches sessions too).
   509	    if (switching && previous !== null && shellPanels.loadsAllowed()) snapshotWorkspace(previous);
   510	    if (switching && previous !== null) stopConversationTerminalProjection(previous);
   511	    if (switching && selected) {
   512	      const root = selected.cwd.trim() || (selected.projectPath ?? '').trim();
   513	      if (root) await setExtensionApiProbeWorkspace({ ownedId: selected.ownedId, root });
   514	    }
   515	    setActiveOwned(ownedId);
   516	    service?.show(ownedId);
   517	    // Point the file tree, the context cards and any tab the user has already
   518	    // opened at this session's project. Ignored while start-up is still
   519	    // re-attaching sessions, so a reload still loads nothing on its own.
   520	    shellPanels.sessionPicked();
   521	    // Clicking the session you are already on changes nothing. Putting the
   522	    // stored record back here would throw away every file opened since the last
   523	    // switch, which is the opposite of what a click on your own row means.
   524	    if (switching) restoreWorkspace(ownedId);
   525	    const provider = conversationProviderFor(ownedId);
   526	    if (selected && provider) {
   527	      // A running PTY is already the owner of this agent session. Starting a
   528	      // second `claude --resume` / `codex app-server` here makes two agents
   529	      // append different answers to one conversation. Until the structured
   530	      // surface mirrors the PTY's transcript, keep the real terminal visible.
   531	      if (selected.ptySessionId) {
   532	        void closeStructuredConversation(ownedId);
   533	        if (selected.nativeSessionId) {
   534	          setConversationMode(ownedId, 'structured');
   535	          startConversationTerminalProjection({
   536	            ownedId,
   537	            provider,
   538	            nativeSessionId: selected.nativeSessionId
   539	          });
   540	        } else {
   541	          setConversationMode(ownedId, 'raw');
   542	        }
   543	        return;
   544	      }
   545	      void ensureStructuredConversation({
   546	        ownedId,
   547	        provider,
   548	        cwd: selected.cwd,
   549	        nativeSessionId: selected.nativeSessionId
   550	      }).catch((error) => {

exec
/bin/zsh -lc "git show --no-patch --format='%H%n%s%n%ad' 279b453 2>/dev/null || true
rg -n -i \"ACP|structured conversation|structured session|agent client protocol|terminal parsing|parse.*terminal|terminal.*parse|conversation.*event\" docs/superpowers/plans docs | head -260" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
279b453481e5cf8b5411ce1dbba7a12d8bc801fb
plan: define ACP runtime, workflows, and Orca-inspired workbench
Tue Aug 4 04:47:58 2026 -0400
docs/conversation-workbench-rust-guide.md:5:The Claude or Codex process that is already running in the session PTY remains the only agent process. The structured Session view reads that process's JSONL transcript and sends user input to its existing PTY. It never starts a second provider process.
docs/conversation-workbench-rust-guide.md:7:In .NET terms, the PTY is the authoritative hosted service. The Svelte conversation view is a projection over its durable event log, not another service instance.
docs/superpowers/plans/README.md:9:### 1. ACP, workflows, Orca-inspired shell, and affected product surfaces
docs/superpowers/plans/README.md:13:`2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`
docs/superpowers/plans/README.md:17:- structured ACP sessions and native-CLI handoff;
docs/architecture-explainer.html:214:      <div class="box rust"><span class="t">AgentRuntimeManager</span><span class="s">agent_conversation/ → ACP child processes</span></div>
docs/architecture-explainer.html:294:agent-client-protocol = "=2.0.0"       <cmt># the ACP wire protocol</cmt>
docs/architecture-explainer.html:418:<div class="path">src-tauri/src/agent_conversation/providers/acp.rs:146</div>
docs/architecture-explainer.html:420:    Acp(<ty>AcpRuntimeAdapter</ty>),
docs/architecture-explainer.html:426:            <ty>Self</ty>::Acp(adapter) => adapter.prompt(input).await,
docs/architecture-explainer.html:975:<h3>Agent conversations (ACP)</h3>
docs/architecture-explainer.html:977:speak the Agent Client Protocol — JSON-RPC over the child's stdin/stdout. The mental model from .NET is
docs/architecture-explainer.html:989:  <a href="#match-traits">§3</a>), <code>AcpRuntimeAdapter</code> (its one implementation),
docs/architecture-explainer.html:990:  <code>acp_client.rs</code> (the JSON-RPC client), <code>process.rs</code> (spawning and cwd
docs/architecture-explainer.html:995:  <code>MCB_CODEX_ACP_PATH</code>/<code>MCB_CODEX_ACP_SHA256</code> and the Claude pair to be set
docs/architecture-explainer.html:1042:current ACP session rather than opening a chat, with the diff capped at
docs/architecture-explainer.html:1247:<code>agent_conversation/capabilities.rs</code> — what an ACP agent says it can do, negotiated at
docs/architecture-explainer.html:1334:      <code>MCB_*_ACP_PATH</code>/<code>_SHA256</code> variables must be set together or all absent.</td>
docs/superpowers/plans/2026-06-17-p3-conversation-live-workspaces.md:172:- [ ] **Step 1:** Build `ConversationList.svelte` from existing primitives in `src/lib/components/` (Badge, Chip, IconButton, Tooltip, ContextMenu). Props: `sessions: AgentSession[]`, `activeKey: string | null`, `liveKeys: Set<string>`, `keyFor(session)`, and event callbacks (`onOpen`, `onClose`, `onSave`, `onRestore`, `onRepair`, `onCopyResume`). Each row shows: provider badge, title + task chip, a **LIVE** indicator when `liveKeys.has(key)` (green dot using `--color-live`), the focus-lane status pill, path/meta, and an overflow `ContextMenu` (move the existing save/restore/repair/copy actions there). Active row uses `--color-accent`. Use the token system (`tokens.css`) — borderless, spacious, modern.
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:29:1. Structured conversation is the default for owned Claude and Codex sessions. The existing
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:58:- `tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts` — pure ordered event
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:138:export interface AgentConversationEvent {
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:170:Confirm no terminal parsing, Tauri call, Svelte component, or `localStorage` access exists in the
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:219:  apply(event: AgentConversationEvent): ApplyEventResult;
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:323:Emit one global event name, `agent_conversation_event`. Sequence assignment happens in the
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:431:messages by native ID/content position. Never parse ANSI text or terminal screen rows.
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:454:## Task 6: Wire the frontend service and structured Session panel
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:664:- [ ] Claude and Codex default to the structured Session surface.
docs/superpowers/specs/2026-08-01-structured-agent-conversation-design.md:5:**Scope:** Replace the default raw Claude/Codex terminal presentation with a Codex-style structured conversation while preserving the terminal, session identity, Dockview workspaces, Monaco/Roslyn behavior, and per-session isolation.
docs/superpowers/specs/2026-08-01-structured-agent-conversation-design.md:9:1. Structured conversation is the default Session surface for Claude and Codex.
docs/superpowers/specs/2026-08-01-structured-agent-conversation-design.md:30:Add one Rust-managed `AgentConversationRegistry`, keyed by `ownedId`. It owns provider processes, protocol connections, request cancellation, reconnection, and normalized event emission.
docs/superpowers/specs/2026-06-16-dockview-redesign-design.md:49:- **Restore must act on worktree + branch**, not only CWD. Today `branch_hint` and worktree are parsed (`sessions.rs:30–36`) then **ignored** — the terminal only `cd`s into `project_path`. Add: resolve/attach the worktree and check out the branch on hydrate, with a **repair path** when the saved worktree is gone (reuse existing missing-worktree repair guards).
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1:# Assembly ACP Runtime, Workflow Orchestration, and Orca-Inspired Shell Amendment
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:21:3. Work Package 10A’s model/effort/approval slash-command controls. These become capability-driven UI selectors backed by ACP session configuration options.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:22:4. Work Package 10A’s path-only screenshot submission. Images become first-class ACP prompt content when the active provider advertises image input; managed paths remain the terminal-owned fallback.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:38:### 1.1 Does an ACP session create a terminal session?
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:40:No. An ACP session is a structured JSON-RPC connection between Assembly, acting as the ACP client, and an agent process or adapter, acting as the ACP agent. The client normally starts the adapter as a child process with piped stdin/stdout. That process is not an xterm instance and is not the native Claude Code or Codex TUI.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:42:An ACP agent can report command/tool activity and may ask the client to provide terminal execution facilities. Those are **tool terminals** owned by Assembly and associated with a tool call. They may be shown as expandable command output or optionally opened in a terminal pane. They are not the same thing as the user-facing raw `claude` or `codex` TUI.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:48:  ACP JSON-RPC session; default rich conversation experience.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:51:  Client-managed process/output used by a command tool call inside an ACP turn.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:57:A normal structured session creates the first and may create the second. It creates the third only when the user explicitly chooses **Open in native CLI**, when a provider capability is unavailable, or when recovery requires it.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:59:### 1.2 Can ACP provide the requested Codex/T3-style controls and content?
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:80:Yes. ACP is the default provider boundary, not a Codex/Claude-specific UI. Future providers enter through a curated `AgentProviderManifest` and the same `AgentRuntimeAdapter` contract. Provider differences remain in capabilities and namespaced metadata, while the visible timeline uses one app-owned canonical model.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:82:ACP does not itself define a complete multi-agent workflow product. Assembly owns workflow definitions, scheduling, worktree policy, retries, gates, budgets, and run history. ACP is the transport used to run each participating agent.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:110:The repository also contains an intentionally unwired handwritten `codex app-server`/`claude -p stream-json` provider spike. It must not be independently activated beside the PTY. Its useful protocol discoveries can be preserved as tests or removed after the ACP runtime replaces it.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:118:1. **New Codex and Claude sessions start structured by default through ACP.**
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:119:2. **No permanent user-visible terminal is created for an ordinary ACP session.** Tool terminals are separate and scoped to tool calls.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:121:4. **One writer per native provider conversation.** A structured ACP runtime and a native CLI never submit to the same provider session concurrently.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:123:6. **`ownedId` remains the primary application identity.** ACP session IDs, Codex thread IDs, Claude session IDs, PTY IDs, tool-terminal IDs, workflow-run IDs, and parent/child IDs never replace it.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:126:9. **Screenshot paste is a top-priority first-class input path.** Clipboard images appear immediately as previews and are sent as ACP image content blocks when supported.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:128:11. **Subagents have two provenances:** provider-native subagents observed through ACP, and Assembly-managed workflow agents created by the workflow engine. The UI identifies which is which.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:143:### 4.1 ACP foundation
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:156:- `agentclientprotocol/codex-acp` — Codex app-server mapping for models, effort, Fast mode, permissions/sandbox, images, commands, file changes, reasoning, plans, reviews, web search, token/rate usage, and subagents;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:157:- `agentclientprotocol/claude-agent-acp` — Claude Agent SDK mapping for models/options, images, tool permissions, TODOs, edit review, terminals, custom commands, MCP, and nested subagent transcripts.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:159:Do not invoke `npx ...@latest` per session. Pin the adapter version. Prefer packaged sidecars with recorded hashes. Codex ACP already has a standalone-binary build route. Claude ACP currently requires a compatible Node runtime; Work Package A0 must choose and prove either a packaged executable or a bundled/pinned Node 22 runtime before product implementation.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:182:### 4.3 Tauri ACP client reference
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:184:Use Jockey as a focused Tauri/Rust ACP-client reference:
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:196:Use CodeG’s delegation broker as evidence that ACP sessions can be coordinated through an app-owned broker and optional MCP companion:
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:198:- `xintaofei/codeg/src-tauri/src/acp/delegation/types.rs`;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:259:- a new structured session becomes `structured` only after ACP initialization and `session/new` succeed;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:284:    ├── acp.rs
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:285:    ├── acp_client.rs
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:318:- ACP initialize/new/load/resume/close;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:344:    AcpStdio,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:345:    AcpWebSocket,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:354:- ACP initialization succeeds;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:359:The ACP Registry can later aid discovery, but automatic install/update is not part of the first runtime packet.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:390:ACP-specific protocol objects remain inside the adapter. The frontend receives Assembly types only.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:417:- tool terminals are created only from an authenticated current-generation ACP request;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:461:Unknown ACP categories are retained and rendered in More Options. Recognized categories map as follows:
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:469:Do not duplicate legacy ACP `modes` when config options are supplied.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:657:4. Send builds an ACP prompt containing ordered text/image content blocks when `prompt.image` is supported.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:664:Acceptance requires Codex ACP and Claude ACP to receive the same pasted PNG/JPEG/WebP fixture and reference visible image content in their answer.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:718:The tree is live for ACP-owned sessions and transcript-backed for terminal-owned history. Opening a provider-native child is read-only unless the provider explicitly advertises direct-input support for that child. Unknown support means read-only.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:759:4. resume/load the provider session through ACP;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:792:### 9.1 Why ACP alone is not the workflow engine
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:794:ACP runs and observes an agent session. It does not define Assembly’s product-level workflow DAG, role catalog, worktree allocation, retry policy, approval gates, provider routing, budgets, or durable run history.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:796:Assembly must own those concepts and use ACP to execute each node.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:869:- start an ACP session through `AgentRuntimeManager`;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:945:The engine applies depth, concurrency, budget, workspace, provider, and permission policy before starting the child ACP session. This follows the useful CodeG broker pattern while preserving Assembly’s stronger workflow model.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:952:- are observed and rendered through ACP/provider metadata;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1082:- steer an active ACP agent when supported;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1182:- provider ACP `session/list` data merged with existing transcript scanners;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1490:Current quota data comes from ACP/provider-authored events or documented stable local APIs. Transcript volume is never presented as remaining quota.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1635:  Provider/ACP session storage and native transcript.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1725:| A0-ACP, A0-Browser | Luna Max / max / no fork, evidence-only | R1 | controller |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1731:| A2 runtime/ACP transport | SOL-medium / medium / no fork | R2 | controller integrates |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1753:**Sequential gate, parallel probes. No product wiring.** A0-ACP and A0-Browser may run after R1
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1758:#### A0-ACP — adapter and packaging evidence
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1763:`ensure_agent_conversation` at `mod.rs:283-293`. Cargo has no ACP SDK dependency. A0 must not
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1766:Create only evidence under `docs/superpowers/evidence/tsk-808/acp-runtime-spike.md` and redacted
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1767:`acp-fixtures/{codex,claude}/`. In disposable external probes, record exact executable, version,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1810:public type authorities with matching Rust serde types. ACP-native frames do not escape adapters.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1812:after ACP init plus session/new. Version `SessionConversationWorkspace` for generation, owner,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1824:### Work Package A2 — Rust `AgentRuntimeManager` and ACP transport
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1834:`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1849:tree exit, no user PTY, and distinct tool-terminal identity. Native proof starts one pinned ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1862:`scripts/agentConversationCodex.test.mjs`. Map authoritative ACP text/image/model/effort/mode,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1927:ACP writer release; terminal-to-structured loads/resumes only after TUI writer release. Same-
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2207:`ProviderUsageReader` and `ProviderUsageState = Available|Unavailable|Error`. Consume only A1 ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2235:ACP rich usage is primary. Codex/Claude JSONL fallback reads only bytes after a durable cursor;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2381:and Claude structured session without PTY, one-writer handoff, workflow restart/retry/cancel,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2415:| A0-ACP | evidence directory only; disposable external probe | one named provider-probe slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2417:| Runtime/ACP | `agent_conversation/{manager,capabilities,journal}.rs`, provider transport/process | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2420:| Terminal projection | transcript parsers, `terminal_projection.rs`, fixtures/tests | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2440:the TSK-808 master plan. It does not permit ACP, conversation, Paneview, Browser, resource, usage,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2460:  -> A0-ACP and A0-Browser feasibility packets
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2464:  -> A2 AgentRuntimeManager/ACP transport
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2520:### Milestone 2 — ACP/Browser feasibility and shared contracts
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2522:1. Run A0-ACP and A0-Browser as separate bounded evidence packets. They may share no process,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2534:1. Execute A2 before any A3 lane. A2 implements the single Rust `AgentRuntimeManager`, ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2556:   structured session with image input, advertised config, typed events, plans/tasks, subagents,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2561:   with deterministic scheduling, budgets, retries, gates, worktree/file leases, recovery, and ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2575:- Browser grab, annotation, and marked screenshots target the exact ACP conversation;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2690:### 19.1 Runtime/ACP tests
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2704:- no PTY for structured session;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2781:1. Create a new Codex structured session. No user PTY exists. Paste a screenshot, select model/effort/permissions, observe reasoning/tool/file/plan/task items, and open a child agent.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2815:- **U2 — A0 architecture exception:** ask only if ACP or Browser feasibility contradicts a locked
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2877:8. only after R1 passes, execute A0-ACP and A0-Browser, then continue through section 18.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2880:the restored Settings/shell authority. Screenshot paste and structured conversation controls are
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2890:- ACP Rust SDK: `https://github.com/agentclientprotocol/rust-sdk`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2891:- ACP session configuration options: `https://agentclientprotocol.com/rfds/session-config-options`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2892:- Codex ACP adapter: `https://github.com/agentclientprotocol/codex-acp`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2893:- Claude Agent ACP adapter: `https://github.com/agentclientprotocol/claude-agent-acp`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2895:- Jockey ACP client/orchestrator: `https://github.com/recailai/jockey`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2896:- CodeG delegation broker: `https://github.com/xintaofei/codeg/tree/main/src-tauri/src/acp/delegation`
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:5:`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`  
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:19:- new Codex and Claude sessions start as structured ACP sessions by default;
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:30:- future ACP providers use the same app-owned runtime and canonical event model.
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:46:5. The JSONL transcript parser remains the terminal projection/import/recovery adapter, but moves
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:63:- **A0:** prove pinned Codex/Claude ACP adapters, image input, configuration options, typed events,
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:93:Structured ACP owner
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:98:Tool terminals created for ACP command/tool calls are separate app-owned processes. They do not
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1:# Assembly ACP Runtime, Workflow Orchestration, and Orca-Inspired Shell Amendment
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:21:3. Work Package 10A’s model/effort/approval slash-command controls. These become capability-driven UI selectors backed by ACP session configuration options.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:22:4. Work Package 10A’s path-only screenshot submission. Images become first-class ACP prompt content when the active provider advertises image input; managed paths remain the terminal-owned fallback.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:38:### 1.1 Does an ACP session create a terminal session?
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:40:No. An ACP session is a structured JSON-RPC connection between Assembly, acting as the ACP client, and an agent process or adapter, acting as the ACP agent. The client normally starts the adapter as a child process with piped stdin/stdout. That process is not an xterm instance and is not the native Claude Code or Codex TUI.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:42:An ACP agent can report command/tool activity and may ask the client to provide terminal execution facilities. Those are **tool terminals** owned by Assembly and associated with a tool call. They may be shown as expandable command output or optionally opened in a terminal pane. They are not the same thing as the user-facing raw `claude` or `codex` TUI.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:48:  ACP JSON-RPC session; default rich conversation experience.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:51:  Client-managed process/output used by a command tool call inside an ACP turn.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:57:A normal structured session creates the first and may create the second. It creates the third only when the user explicitly chooses **Open in native CLI**, when a provider capability is unavailable, or when recovery requires it.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:59:### 1.2 Can ACP provide the requested Codex/T3-style controls and content?
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:80:Yes. ACP is the default provider boundary, not a Codex/Claude-specific UI. Future providers enter through a curated `AgentProviderManifest` and the same `AgentRuntimeAdapter` contract. Provider differences remain in capabilities and namespaced metadata, while the visible timeline uses one app-owned canonical model.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:82:ACP does not itself define a complete multi-agent workflow product. Assembly owns workflow definitions, scheduling, worktree policy, retries, gates, budgets, and run history. ACP is the transport used to run each participating agent.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:110:The repository also contains an intentionally unwired handwritten `codex app-server`/`claude -p stream-json` provider spike. It must not be independently activated beside the PTY. Its useful protocol discoveries can be preserved as tests or removed after the ACP runtime replaces it.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:118:1. **New Codex and Claude sessions start structured by default through ACP.**
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:119:2. **No permanent user-visible terminal is created for an ordinary ACP session.** Tool terminals are separate and scoped to tool calls.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:121:4. **One writer per native provider conversation.** A structured ACP runtime and a native CLI never submit to the same provider session concurrently.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:123:6. **`ownedId` remains the primary application identity.** ACP session IDs, Codex thread IDs, Claude session IDs, PTY IDs, tool-terminal IDs, workflow-run IDs, and parent/child IDs never replace it.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:126:9. **Screenshot paste is a top-priority first-class input path.** Clipboard images appear immediately as previews and are sent as ACP image content blocks when supported.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:128:11. **Subagents have two provenances:** provider-native subagents observed through ACP, and Assembly-managed workflow agents created by the workflow engine. The UI identifies which is which.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:143:### 4.1 ACP foundation
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:156:- `agentclientprotocol/codex-acp` — Codex app-server mapping for models, effort, Fast mode, permissions/sandbox, images, commands, file changes, reasoning, plans, reviews, web search, token/rate usage, and subagents;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:157:- `agentclientprotocol/claude-agent-acp` — Claude Agent SDK mapping for models/options, images, tool permissions, TODOs, edit review, terminals, custom commands, MCP, and nested subagent transcripts.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:159:Do not invoke `npx ...@latest` per session. Pin the adapter version. Prefer packaged sidecars with recorded hashes. Codex ACP already has a standalone-binary build route. Claude ACP currently requires a compatible Node runtime; Work Package A0 must choose and prove either a packaged executable or a bundled/pinned Node 22 runtime before product implementation.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:182:### 4.3 Tauri ACP client reference
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:184:Use Jockey as a focused Tauri/Rust ACP-client reference:
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:196:Use CodeG’s delegation broker as evidence that ACP sessions can be coordinated through an app-owned broker and optional MCP companion:
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:198:- `xintaofei/codeg/src-tauri/src/acp/delegation/types.rs`;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:259:- a new structured session becomes `structured` only after ACP initialization and `session/new` succeed;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:284:    ├── acp.rs
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:285:    ├── acp_client.rs
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:318:- ACP initialize/new/load/resume/close;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:344:    AcpStdio,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:345:    AcpWebSocket,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:354:- ACP initialization succeeds;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:359:The ACP Registry can later aid discovery, but automatic install/update is not part of the first runtime packet.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:390:ACP-specific protocol objects remain inside the adapter. The frontend receives Assembly types only.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:417:- tool terminals are created only from an authenticated current-generation ACP request;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:461:Unknown ACP categories are retained and rendered in More Options. Recognized categories map as follows:
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:469:Do not duplicate legacy ACP `modes` when config options are supplied.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:657:4. Send builds an ACP prompt containing ordered text/image content blocks when `prompt.image` is supported.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:664:Acceptance requires Codex ACP and Claude ACP to receive the same pasted PNG/JPEG/WebP fixture and reference visible image content in their answer.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:718:The tree is live for ACP-owned sessions and transcript-backed for terminal-owned history. Opening a provider-native child is read-only unless the provider explicitly advertises direct-input support for that child. Unknown support means read-only.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:759:4. resume/load the provider session through ACP;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:792:### 9.1 Why ACP alone is not the workflow engine
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:794:ACP runs and observes an agent session. It does not define Assembly’s product-level workflow DAG, role catalog, worktree allocation, retry policy, approval gates, provider routing, budgets, or durable run history.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:796:Assembly must own those concepts and use ACP to execute each node.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:869:- start an ACP session through `AgentRuntimeManager`;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:945:The engine applies depth, concurrency, budget, workspace, provider, and permission policy before starting the child ACP session. This follows the useful CodeG broker pattern while preserving Assembly’s stronger workflow model.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:952:- are observed and rendered through ACP/provider metadata;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1082:- steer an active ACP agent when supported;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1182:- provider ACP `session/list` data merged with existing transcript scanners;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1490:Current quota data comes from ACP/provider-authored events or documented stable local APIs. Transcript volume is never presented as remaining quota.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1635:  Provider/ACP session storage and native transcript.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1725:| A0-ACP, A0-Browser | Luna Max / max / no fork, evidence-only | R1 | controller |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1731:| A2 runtime/ACP transport | SOL-medium / medium / no fork | R2 | controller integrates |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1753:**Sequential gate, parallel probes. No product wiring.** A0-ACP and A0-Browser may run after R1
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1758:#### A0-ACP — adapter and packaging evidence
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1763:`ensure_agent_conversation` at `mod.rs:283-293`. Cargo has no ACP SDK dependency. A0 must not
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1766:Create only evidence under `docs/superpowers/evidence/tsk-808/acp-runtime-spike.md` and redacted
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1767:`acp-fixtures/{codex,claude}/`. In disposable external probes, record exact executable, version,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1810:public type authorities with matching Rust serde types. ACP-native frames do not escape adapters.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1812:after ACP init plus session/new. Version `SessionConversationWorkspace` for generation, owner,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1824:### Work Package A2 — Rust `AgentRuntimeManager` and ACP transport
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1834:`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1849:tree exit, no user PTY, and distinct tool-terminal identity. Native proof starts one pinned ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1862:`scripts/agentConversationCodex.test.mjs`. Map authoritative ACP text/image/model/effort/mode,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:1927:ACP writer release; terminal-to-structured loads/resumes only after TUI writer release. Same-
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2207:`ProviderUsageReader` and `ProviderUsageState = Available|Unavailable|Error`. Consume only A1 ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2235:ACP rich usage is primary. Codex/Claude JSONL fallback reads only bytes after a durable cursor;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2381:and Claude structured session without PTY, one-writer handoff, workflow restart/retry/cancel,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2415:| A0-ACP | evidence directory only; disposable external probe | one named provider-probe slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2417:| Runtime/ACP | `agent_conversation/{manager,capabilities,journal}.rs`, provider transport/process | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2420:| Terminal projection | transcript parsers, `terminal_projection.rs`, fixtures/tests | focused Rust slot |
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2440:the TSK-808 master plan. It does not permit ACP, conversation, Paneview, Browser, resource, usage,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2460:  -> A0-ACP and A0-Browser feasibility packets
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2464:  -> A2 AgentRuntimeManager/ACP transport
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2520:### Milestone 2 — ACP/Browser feasibility and shared contracts
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2522:1. Run A0-ACP and A0-Browser as separate bounded evidence packets. They may share no process,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2534:1. Execute A2 before any A3 lane. A2 implements the single Rust `AgentRuntimeManager`, ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2556:   structured session with image input, advertised config, typed events, plans/tasks, subagents,
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2561:   with deterministic scheduling, budgets, retries, gates, worktree/file leases, recovery, and ACP
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2575:- Browser grab, annotation, and marked screenshots target the exact ACP conversation;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2690:### 19.1 Runtime/ACP tests
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2704:- no PTY for structured session;
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2781:1. Create a new Codex structured session. No user PTY exists. Paste a screenshot, select model/effort/permissions, observe reasoning/tool/file/plan/task items, and open a child agent.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2815:- **U2 — A0 architecture exception:** ask only if ACP or Browser feasibility contradicts a locked
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2877:8. only after R1 passes, execute A0-ACP and A0-Browser, then continue through section 18.
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2880:the restored Settings/shell authority. Screenshot paste and structured conversation controls are
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2890:- ACP Rust SDK: `https://github.com/agentclientprotocol/rust-sdk`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2891:- ACP session configuration options: `https://agentclientprotocol.com/rfds/session-config-options`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2892:- Codex ACP adapter: `https://github.com/agentclientprotocol/codex-acp`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2893:- Claude Agent ACP adapter: `https://github.com/agentclientprotocol/claude-agent-acp`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2895:- Jockey ACP client/orchestrator: `https://github.com/recailai/jockey`
docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2896:- CodeG delegation broker: `https://github.com/xintaofei/codeg/tree/main/src-tauri/src/acp/delegation`
docs/superpowers/plans/2026-06-17-p3-conversation-live-workspaces.md:172:- [ ] **Step 1:** Build `ConversationList.svelte` from existing primitives in `src/lib/components/` (Badge, Chip, IconButton, Tooltip, ContextMenu). Props: `sessions: AgentSession[]`, `activeKey: string | null`, `liveKeys: Set<string>`, `keyFor(session)`, and event callbacks (`onOpen`, `onClose`, `onSave`, `onRestore`, `onRepair`, `onCopyResume`). Each row shows: provider badge, title + task chip, a **LIVE** indicator when `liveKeys.has(key)` (green dot using `--color-live`), the focus-lane status pill, path/meta, and an overflow `ContextMenu` (move the existing save/restore/repair/copy actions there). Active row uses `--color-accent`. Use the token system (`tokens.css`) — borderless, spacious, modern.
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:29:1. Structured conversation is the default for owned Claude and Codex sessions. The existing
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:58:- `tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts` — pure ordered event
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:138:export interface AgentConversationEvent {
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:170:Confirm no terminal parsing, Tauri call, Svelte component, or `localStorage` access exists in the
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:219:  apply(event: AgentConversationEvent): ApplyEventResult;
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:323:Emit one global event name, `agent_conversation_event`. Sequence assignment happens in the
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:431:messages by native ID/content position. Never parse ANSI text or terminal screen rows.
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:454:## Task 6: Wire the frontend service and structured Session panel
docs/superpowers/plans/2026-08-01-structured-agent-conversation.md:664:- [ ] Claude and Codex default to the structured Session surface.
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:5:`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`  
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:19:- new Codex and Claude sessions start as structured ACP sessions by default;
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:30:- future ACP providers use the same app-owned runtime and canonical event model.
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:46:5. The JSONL transcript parser remains the terminal projection/import/recovery adapter, but moves
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:63:- **A0:** prove pinned Codex/Claude ACP adapters, image input, configuration options, typed events,
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:93:Structured ACP owner
docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md:98:Tool terminals created for ACP command/tool calls are separate app-owned processes. They do not
docs/superpowers/plans/README.md:9:### 1. ACP, workflows, Orca-inspired shell, and affected product surfaces
docs/superpowers/plans/README.md:13:`2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`
docs/superpowers/plans/README.md:17:- structured ACP sessions and native-CLI handoff;
docs/superpowers/evidence/tsk-808/a6-workflow-engine-receipt.md:15:- `tauri-svelte-preview/src-tauri/src/workflow.rs:544` — injected `AgentRuntimePort`; the `AgentRuntimeManager` adapter activates ACP, maps model/effort/permission through advertised provider capability categories, and submits a structured task envelope.
docs/superpowers/evidence/tsk-808/a7-control-center-receipt.md:7:Authority: `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`, §10 and §16  
docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md:14:`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2083-2173`:
docs/superpowers/evidence/tsk-808/pr-flow-receipt.md:22:- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:337-357` and `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:157-244` add the one-shot ACP prompt path; the ACP fixture test also proves text-chunk capture.
docs/superpowers/evidence/tsk-808/pr-flow-receipt.md:34:- 2026-08-08 — **verified** ACP one-shot capture also accepts the shared `type` update discriminator used by the existing frame projector; the targeted ACP fixture still passes.
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4312:  typed conversation events, but `ensure_agent_conversation` is intentionally unwired and the
docs/superpowers/evidence/tsk-808/resources-usage-rework-receipt.md:32:  `resource_roots()` snapshot (pid, owned id, provider, cwd) and `providers/acp.rs` (+9) exposes
docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md:4312:  typed conversation events, but `ensure_agent_conversation` is intentionally unwired and the
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:20:2. The Work Package A9 section and any browser overlay/z-order sections of docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md.
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:46:540:- TSK-808, TSK-799, TSK-798, Assembly, docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md, 2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md, contrast, legibility, gpt-5.6-sol, Tauri, Dockview, Monaco, Roslyn, U1
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:48:562:- The authoritative plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; use section 18 of `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md` instead of retired schedules. TSK-808 remained open/High; TSK-799 and TSK-798 are completed dependencies. [Task 1]
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:295:/bin/zsh -lc "git status --short && printf '%s\\n' '--- receipt status/diff ---' && git diff --stat -- docs/superpowers/evidence/tsk-808/a9-browser-frontend-receipt.md && git diff -- docs/superpowers/evidence/tsk-808/a9-browser-frontend-receipt.md && printf '%s\\n' '--- receipt numbered ---' && nl -ba docs/superpowers/evidence/tsk-808/a9-browser-frontend-receipt.md && printf '%s\\n' '--- A9/browser plan hits ---' && rg -n -i -C 8 'Work Package A9|\\bA9\\b|browser overlay|BrowserPanel|z-order|z order|overlay mount|WorkbenchActionFab|browserArea' docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
docs/superpowers/evidence/tsk-808/a9-overlay-wiring-spec-raw.log:635:1731-| A2 runtime/ACP transport | SOL-medium / medium / no fork | R2 | controller integrates |

exec
/bin/zsh -lc "nl -ba docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md | sed -n '34,130p;245,330p;1820,1860p;1910,1940p;2690,2715p;2775,2790p'
printf '\\n--- original structured plan ---\\n'
nl -ba docs/superpowers/plans/2026-08-01-structured-agent-conversation.md | sed -n '20,75p;160,180p;315,335p;420,465p;650,670p'
printf '\\n--- guide ---\\n'
nl -ba docs/conversation-workbench-rust-guide.md | sed -n '1,32p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
    34	---
    35	
    36	## 1. Direct product answers
    37	
    38	### 1.1 Does an ACP session create a terminal session?
    39	
    40	No. An ACP session is a structured JSON-RPC connection between Assembly, acting as the ACP client, and an agent process or adapter, acting as the ACP agent. The client normally starts the adapter as a child process with piped stdin/stdout. That process is not an xterm instance and is not the native Claude Code or Codex TUI.
    41	
    42	An ACP agent can report command/tool activity and may ask the client to provide terminal execution facilities. Those are **tool terminals** owned by Assembly and associated with a tool call. They may be shown as expandable command output or optionally opened in a terminal pane. They are not the same thing as the user-facing raw `claude` or `codex` TUI.
    43	
    44	Assembly therefore keeps three distinct concepts:
    45	
    46	```text
    47	Structured agent session
    48	  ACP JSON-RPC session; default rich conversation experience.
    49	
    50	Tool terminal
    51	  Client-managed process/output used by a command tool call inside an ACP turn.
    52	
    53	Native CLI terminal
    54	  Existing interactive PTY running the real Claude Code or Codex TUI.
    55	```
    56	
    57	A normal structured session creates the first and may create the second. It creates the third only when the user explicitly chooses **Open in native CLI**, when a provider capability is unavailable, or when recovery requires it.
    58	
    59	### 1.2 Can ACP provide the requested Codex/T3-style controls and content?
    60	
    61	Yes, with capability negotiation and provider-specific extensions where the stable protocol does not yet have a universal shape.
    62	
    63	For Codex and Claude, the reviewed official adapters already provide enough surface to build the requested product:
    64	
    65	- models and reasoning effort;
    66	- session modes/permissions;
    67	- optional model configuration such as Fast mode;
    68	- text and image input;
    69	- assistant text and thoughts/reasoning;
    70	- tool calls, command output, file changes, and approval requests;
    71	- plans/TODOs/task progress where the provider exposes them;
    72	- session load/resume/close capabilities;
    73	- provider-advertised commands and skills;
    74	- subagent activity and nested transcripts through standard updates plus namespaced metadata.
    75	
    76	Assembly must render these from the runtime’s advertised capabilities. It must not hard-code a model catalog, assume every provider has the same effort values, or show a control that does not affect the active session.
    77	
    78	### 1.3 Can the same architecture support future providers?
    79	
    80	Yes. ACP is the default provider boundary, not a Codex/Claude-specific UI. Future providers enter through a curated `AgentProviderManifest` and the same `AgentRuntimeAdapter` contract. Provider differences remain in capabilities and namespaced metadata, while the visible timeline uses one app-owned canonical model.
    81	
    82	ACP does not itself define a complete multi-agent workflow product. Assembly owns workflow definitions, scheduling, worktree policy, retries, gates, budgets, and run history. ACP is the transport used to run each participating agent.
    83	
    84	---
    85	
    86	## 2. Current repository reality that this plan extends
    87	
    88	The current `main` implementation has strong foundations that remain:
    89	
    90	- `ownedId` is the stable app identity; provider session IDs and PTY IDs are metadata.
    91	- `TerminalService` and Rust `TerminalRegistry` already provide one listener, reattach, bounded scrollback, tombstones, exact process ownership, and safe explicit close.
    92	- per-session workspaces already capture editor, explorer, browser, diff, center layout, conversation draft/mode/child selection, and scroll position.
    93	- the Rust transcript scanners already discover Codex and Claude sessions, exclude helper/subagent sessions from the top-level rail, and derive titles, metadata, message counts, and resumable commands.
    94	- the current conversation store is keyed by `ownedId`, has generation/sequence protection, and already includes attachment and child state.
    95	- the attachment vault validates image signatures, size, canonical managed paths, and safe owned-session directories.
    96	- the browser master-plan lane already contains strong child-webview, annotation, isolation, and cleanup requirements.
    97	- the orchestration event ledger is append-only and already has run, agent, step, artifact, link, attention, and decision concepts.
    98	
    99	The current active conversation path is nevertheless a terminal transcript projection:
   100	
   101	```text
   102	bare interactive `claude` or `codex` in a PTY
   103	  -> provider JSONL transcript
   104	  -> 500 ms full-tail polling
   105	  -> flattened user/assistant messages
   106	  -> structured overlay
   107	  -> composer writes bracketed paste + Return back to the PTY
   108	```
   109	
   110	The repository also contains an intentionally unwired handwritten `codex app-server`/`claude -p stream-json` provider spike. It must not be independently activated beside the PTY. Its useful protocol discoveries can be preserved as tests or removed after the ACP runtime replaces it.
   111	
   112	The current projection cannot be the final rich client because it discards most typed provider behavior and cannot authoritatively control an active turn. The target architecture replaces that limitation without discarding the terminal system.
   113	
   114	---
   115	
   116	## 3. Locked product decisions
   117	
   118	1. **New Codex and Claude sessions start structured by default through ACP.**
   119	2. **No permanent user-visible terminal is created for an ordinary ACP session.** Tool terminals are separate and scoped to tool calls.
   120	3. **The native CLI remains first class.** The user can hand a session to the raw Codex/Claude TUI and later return it to the structured view.
   121	4. **One writer per native provider conversation.** A structured ACP runtime and a native CLI never submit to the same provider session concurrently.
   122	5. **Existing externally created or already-running CLI sessions remain usable.** Assembly imports/mirrors their transcripts and may perform an explicit terminal-to-structured handoff when the provider can resume them safely.
   123	6. **`ownedId` remains the primary application identity.** ACP session IDs, Codex thread IDs, Claude session IDs, PTY IDs, tool-terminal IDs, workflow-run IDs, and parent/child IDs never replace it.
   124	7. **Model, effort/thought level, mode/permissions, Fast/service tier, and similar settings are UI controls—not slash commands.** They come from runtime-advertised session configuration options.
   125	8. **The slash menu contains only genuine provider commands and Assembly actions.** TUI-only commands such as theme, raw terminal rendering, terminal title, pet, keymap, or app handoff are excluded unless Assembly implements an equivalent local action.
   126	9. **Screenshot paste is a top-priority first-class input path.** Clipboard images appear immediately as previews and are sent as ACP image content blocks when supported.
   127	10. **Tool calls, approvals, plans, task lists, reasoning, file changes, and terminals are typed timeline items.** They are not flattened into assistant Markdown.
   128	11. **Subagents have two provenances:** provider-native subagents observed through ACP, and Assembly-managed workflow agents created by the workflow engine. The UI identifies which is which.
   129	12. **Workflow orchestration is app-owned and deterministic.** An LLM may serve as the selected orchestrator, but it does not become the scheduler, persistence layer, worktree manager, or permission authority.
   130	13. **The left rail is compact active work, not the complete session archive.** Resumable/history discovery moves to its own registered surface.
   245	  executionOwner: AgentExecutionOwner;
   246	  runtimeState: AgentRuntimeState;
   247	  providerInstanceId: string | null;
   248	  nativeSessionId: string | null;
   249	  activeTurnId: string | null;
   250	  capabilityRevision: number;
   251	  lastRuntimeError: string | null;
   252	}
   253	```
   254	
   255	Migration rules:
   256	
   257	- an existing live agent PTY becomes `executionOwner: 'terminal'`;
   258	- an exited agent session becomes `stopped`;
   259	- a new structured session becomes `structured` only after ACP initialization and `session/new` succeed;
   260	- failed migration never erases `ptySessionId`, `nativeSessionId`, transcript, draft, workspace, or attachment state.
   261	
   262	### 5.2 One Rust `AgentRuntimeManager`
   263	
   264	Refactor the existing `src-tauri/src/agent_conversation` authority rather than adding a competing runtime.
   265	
   266	Target module shape after re-anchor:
   267	
   268	```text
   269	src-tauri/src/agent_conversation/
   270	├── mod.rs
   271	├── manager.rs
   272	├── protocol.rs
   273	├── capabilities.rs
   274	├── journal.rs
   275	├── handoff.rs
   276	├── attachments.rs
   277	├── terminal_projection.rs
   278	├── transcript/
   279	│   ├── mod.rs
   280	│   ├── claude.rs
   281	│   └── codex.rs
   282	└── providers/
   283	    ├── mod.rs
   284	    ├── acp.rs
   285	    ├── acp_client.rs
   286	    ├── process.rs
   287	    └── direct_codex.rs        # optional later optimization only
   288	```
   289	
   290	Core state:
   291	
   292	```rust
   293	pub struct AgentRuntimeManager {
   294	    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
   295	    providers: Arc<ProviderRegistry>,
   296	}
   297	
   298	pub struct ManagedAgentSession {
   299	    pub owned_id: String,
   300	    pub provider: AgentProvider,
   301	    pub provider_instance_id: String,
   302	    pub native_session_id: Option<String>,
   303	    pub generation: u64,
   304	    pub owner: ExecutionOwner,
   305	    pub state: AgentRuntimeState,
   306	    pub capabilities: AgentCapabilities,
   307	    pub next_sequence: u64,
   308	    pub active_turn_id: Option<String>,
   309	    pub runtime: Option<StructuredRuntimeHandle>,
   310	    pub terminal_projection: Option<TerminalProjectionHandle>,
   311	    pub recent_events: VecDeque<AgentEvent>,
   312	}
   313	```
   314	
   315	The manager owns:
   316	
   317	- provider sidecar lifecycle and stderr diagnostics;
   318	- ACP initialize/new/load/resume/close;
   319	- prompt, image, cancellation, steering, permission, and user-input responses;
   320	- session config updates;
   321	- normalized event emission;
   322	- provider-native IDs;
   323	- structured/terminal writer lease;
   324	- recovery and handoff;
   325	- bounded recent-event snapshots and rebuildable app journal.
   326	
   327	It does not own editor, Git, worktrees, browser, Dockview, or the main terminal registry.
   328	
   329	### 5.3 Provider registry and future providers
   330	
  1820	starts a provider. Stop before A2 on any need for a second store/runtime or inability to express one
  1821	owner/generation/writer lease. Return exact public symbols, migration fixtures, tests/results, and
  1822	the controller commit SHA that freezes the seam.
  1823	
  1824	### Work Package A2 — Rust `AgentRuntimeManager` and ACP transport
  1825	
  1826	**Route:** SOL-medium after R2. Refactor, do not coexist with, the current
  1827	`AgentConversationRegistry`, `ConversationSession`, `ProviderCommand`, `ensure`, `emit_payload`,
  1828	`send_command`, `snapshot`, and `close` in `agent_conversation/mod.rs`. `TerminalRegistry` remains
  1829	the sole user-PTY authority.
  1830	
  1831	Own new `agent_conversation/manager.rs` (`AgentRuntimeManager`, `ManagedAgentSession`, writer lease,
  1832	generation/sequence, bounded event snapshot), `capabilities.rs` (manifest/capability/config
  1833	validation), `journal.rs` (bounded append/rebuild/repair), `providers/mod.rs` (`ProviderRegistry`),
  1834	`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
  1835	image/config/approval/input/cancel/close), and `providers/process.rs` (piped sidecar supervision,
  1836	bounded stderr, process-group cleanup). Tool terminals carry `TerminalKind` plus
  1837	`ToolTerminalIdentity` through the existing `TerminalRegistry`; they never set
  1838	`OwnedSession.ptySessionId` or appear as user PTYs.
  1839	
  1840	Implement one manager per app, one current-generation lease per `ownedId`, full-state capability
  1841	replacement, canonical events, bounded snapshots/journal, stale-response rejection, close and
  1842	app-exit cleanup. Return controller receipts for `mod.rs`, `main.rs`, capabilities and manifests;
  1843	do not edit frontend, provider-specific mapping, terminal projection, handoff, Browser, or shared
  1844	A1 types.
  1845	
  1846	Run `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation` in one
  1847	heavy slot. Tests cover initialization/new/load/resume, prompt/image, correlated approval/input,
  1848	config replacement, cancel/close, sequence repair, generation rejection, journal recovery, process
  1849	tree exit, no user PTY, and distinct tool-terminal identity. Native proof starts one pinned ACP
  1850	sidecar per structured `ownedId` with no PTY or leaked descendant. Stop on arbitrary credential/
  1851	root authority, duplicate runtime, cleanup failure, or stale-writer ambiguity. Return files,
  1852	symbols, registrations, dependency receipts, tests/results, native process evidence, and cleanup.
  1853	
  1854	### Work Package A3 — provider adapters and capability mapping
  1855	
  1856	**Parallel only after A2 contract tests pass.** A3-Codex and A3-Claude are Luna Max; A3-terminal is
  1857	SOL-medium. All consume A1/A2 public contracts, edit separate provider/parser files, and return
  1858	shared-seam receipts.
  1859	
  1860	**A3-Codex:** own `providers/codex.rs`, redacted
  1910	capability controls, typed ordering, recursive child state, virtualization/scroll, commands, exact
  1911	owner isolation, and no hidden PTY. Stop on unbacked controls, flattening, cross-owner response,
  1912	unsafe Markdown/secret display, or automatic send. Return owned files, consumed contracts,
  1913	controller receipts, tests/results, native evidence, and cleanup.
  1914	
  1915	### Work Package A5 — native CLI handoff
  1916	
  1917	**Route:** SOL-medium only after integrated A2+A3+A4 native proof. Add
  1918	`agent_conversation/handoff.rs` for structured-to-terminal, terminal-to-structured, same-session/
  1919	fork, rollback, writer-lease/process-tree assertions, and exact history boundary/reconciliation;
  1920	add Rust tests and `scripts/agentConversationHandoff.test.mjs`. Extend the existing
  1921	`conversationService.ts` with typed calls through a controller receipt; do not create a second
  1922	handoff/provider service.
  1923	
  1924	The controller wires actions into `ConversationSurface.svelte` and `/next/+page.svelte` while
  1925	preserving `terminal.rs`, `terminalService.ts`, `ownedSessions.ts`, workspace/draft/attachment
  1926	state, and generation. Structured-to-terminal opens the same native provider session only after
  1927	ACP writer release; terminal-to-structured loads/resumes only after TUI writer release. Same-
  1928	session and fork choices are distinct and labeled. Failure restores the prior owner/process and
  1929	preserves all state. The optional Codex shared-daemon optimization is a separate later probe.
  1930	
  1931	Run focused handoff, owned-session, isolation, workspace, live-terminal, terminal-service, and
  1932	Rust agent-conversation tests serially. Native acceptance performs both directions, same/fork,
  1933	rollback, restart/recovery, exact native ID/history boundary, and proves one process-tree writer
  1934	with no duplicate sidecar/PTY. Stop on dual writer, stale generation, destructive failure, or
  1935	history mismatch. Return lease transitions, files, controller receipts, tests/results, native
  1936	process evidence, rollback evidence, and cleanup.
  1937	
  1938	### Work Package A6 — WorkflowEngine core
  1939	
  1940	**Route:** SOL-medium. Start only after A1 contracts and A2/A3 lifecycle APIs are stable. No UI
  2690	### 19.1 Runtime/ACP tests
  2691	
  2692	Add focused scripts/Rust tests for:
  2693	
  2694	- provider initialization and version/hash;
  2695	- capability mapping and unknown categories;
  2696	- config option full-state replacement;
  2697	- image input success and unsupported fallback;
  2698	- ordered assistant/reasoning/tool/file/plan/task events;
  2699	- permission and user-input correlation;
  2700	- cancel/close/recovery;
  2701	- session list/load/resume;
  2702	- subagent parent metadata;
  2703	- process exit and cleanup;
  2704	- no PTY for structured session;
  2705	- tool-terminal identity separate from user PTY;
  2706	- stale generation and sequence repair.
  2707	
  2708	### 19.2 Conversation tests
  2709	
  2710	- screenshot paste is first-priority and exact-owner safe;
  2711	- draft/attachments restore independently for two sessions;
  2712	- model/effort/mode controls are not slash commands;
  2713	- TUI-only commands absent;
  2714	- provider-advertised/local commands merge correctly;
  2715	- typed timeline ordering;
  2775	- no secret/raw prompt persistence.
  2776	
  2777	### 19.7 Native end-to-end acceptance
  2778	
  2779	One rebuilt-Tauri proof must show:
  2780	
  2781	1. Create a new Codex structured session. No user PTY exists. Paste a screenshot, select model/effort/permissions, observe reasoning/tool/file/plan/task items, and open a child agent.
  2782	2. Repeat with Claude using its advertised options and a nested child transcript.
  2783	3. Open Codex in native CLI, continue the same conversation, return to structured mode, and prove one writer and exact history reconciliation.
  2784	4. Start a workflow with Orchestrator -> two Implementers in separate worktrees -> Test -> Code Review -> Spec Compliance -> Fix -> final review. See every node/agent, current loop phase, tool output, worktree, finding, gate, retry, and completion.
  2785	5. Cancel one child and retry one failed node without disturbing siblings.
  2786	6. Pin one workflow agent to Working; verify the rest remain in the workflow tree and Session Library.
  2787	7. Open the Browser from the bottom action island in floating mode, maximize it, grab an element, annotate another, draw on a screenshot, and stage all three into the exact active conversation without auto-send.
  2788	8. Open Resource Manager compact view, Resources full page, Workspace Space treemap, Usage compact view, and Usage analytics. Values share one underlying state and actions target exact owners.
  2789	9. Use AI to draft a PR, review a diff, suggest form fields, and run a pre-save advisory check. Cancel one proposal and confirm one disposable safe action; audit receipts remain after restart.
  2790	10. Working/Done/Settled and every right primary panel resize/collapse/reorder as Paneview panes and restore after restart.

--- original structured plan ---
    20	VS Code compatibility layer, `codex app-server`, and `claude -p` stream-json.
    21	
    22	**Approved specification:**
    23	`docs/superpowers/specs/2026-08-01-structured-agent-conversation-design.md`
    24	
    25	---
    26	
    27	## Locked constraints
    28	
    29	1. Structured conversation is the default for owned Claude and Codex sessions. The existing
    30	   `TerminalSurface` remains available as the raw fallback and is not deleted.
    31	2. Provider output is never reconstructed by scraping terminal screen text. New user messages
    32	   come from the composer; provider history is imported only through structured provider APIs.
    33	3. Switching sessions does not restart Claude, Codex, Roslyn, Monaco, or the PTY. Background
    34	   events continue routing to the store for their owning `ownedId`.
    35	4. Do not put full conversation transcripts in `localStorage`. Persist small workspace metadata
    36	   there; retain live transcript state in memory and restore durable history from the provider or
    37	   Rust conversation snapshot.
    38	5. Houston is registered before the one-time VS Code service initialization. Remove the forced
    39	   `Default Dark Modern` setting instead of repainting Monaco after startup.
    40	
    41	## File structure
    42	
    43	### New Rust files
    44	
    45	- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs` — registry, Tauri commands,
    46	  lifecycle, event emission, and adapter selection.
    47	- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs` — serialized request,
    48	  connection, event, payload, approval, and error types.
    49	- `tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs` — child-process ownership,
    50	  line framing, stderr draining, cancellation, and shutdown.
    51	- `tauri-svelte-preview/src-tauri/src/agent_conversation/codex.rs` — Codex app-server adapter.
    52	- `tauri-svelte-preview/src-tauri/src/agent_conversation/claude.rs` — Claude stream-json adapter.
    53	
    54	### New frontend files
    55	
    56	- `tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts` — normalized frontend
    57	  contract matching Rust serialization.
    58	- `tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts` — pure ordered event
    59	  reducer and history deduplication.
    60	- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts` — state keyed by
    61	  `ownedId`; no backend calls.
    62	- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts` — Tauri commands,
    63	  global event listener, connection guards, and routing.
    64	- `tauri-svelte-preview/src/lib/shell/components/ConversationPanel.svelte` — structured/raw mode
    65	  shell and connection state.
    66	- `tauri-svelte-preview/src/lib/shell/components/ConversationTimeline.svelte` — message, tool,
    67	  approval, status, and error rendering.
    68	- `tauri-svelte-preview/src/lib/shell/components/ConversationComposer.svelte` — draft, send, stop,
    69	  and raw-terminal controls.
    70	- `tauri-svelte-preview/src/lib/shell/themes/vscodeThemeAdapter.ts` — Houston-to-VS Code theme
    71	  registration before Monaco service initialization.
    72	
    73	### New focused tests
    74	
    75	- `tauri-svelte-preview/scripts/agentConversationProtocol.test.mjs`
   160	the session `desynchronized` and retain history; do not guess missing deltas.
   161	
   162	### Step 3: Prove the reducer
   163	
   164	Run the focused test again.
   165	
   166	Expected: PASS with deterministic sequence, generation, deduplication, and error-retention cases.
   167	
   168	### Step 4: Check the task boundary
   169	
   170	Confirm no terminal parsing, Tauri call, Svelte component, or `localStorage` access exists in the
   171	new files.
   172	
   173	Do not commit unless the user explicitly authorizes a commit.
   174	
   175	---
   176	
   177	## Task 2: Add per-session conversation and complete workspace state
   178	
   179	**Files:**
   180	
   315	
   316	- `ensure_agent_conversation`
   317	- `send_agent_conversation_message`
   318	- `respond_agent_conversation_approval`
   319	- `stop_agent_conversation_turn`
   320	- `close_agent_conversation`
   321	- `read_agent_conversation_snapshot`
   322	
   323	Emit one global event name, `agent_conversation_event`. Sequence assignment happens in the
   324	registry after provider normalization, so adapters cannot create conflicting sequence numbers.
   325	
   326	### Step 4: Implement safe child ownership
   327	
   328	Reuse the process-management patterns already present in the terminal/LSP modules without coupling
   329	conversation state to them. Validate `cwd` against the owning session/workspace boundary already
   330	used by the app. Spawn without a shell. Drain stdout and stderr in independent tasks. Stop the
   331	process group on explicit close and app exit.
   332	
   333	### Step 5: Prove lifecycle behavior
   334	
   335	Run the focused Rust test with `RUST_TEST_THREADS=1`.
   420	```text
   421	claude -p --input-format stream-json --output-format stream-json
   422	  --include-partial-messages --replay-user-messages
   423	```
   424	
   425	Add `--resume <session-id>` only when resuming known provider state. Keep stdin open for
   426	bidirectional turns. Capture the native session ID from structured output.
   427	
   428	### Step 3: Normalize messages and tool activity
   429	
   430	Translate Claude content blocks into the same events used by Codex. Deduplicate replayed user
   431	messages by native ID/content position. Never parse ANSI text or terminal screen rows.
   432	
   433	### Step 4: Define honest unsupported behavior
   434	
   435	If the installed Claude CLI does not expose a structured approval response required by an
   436	interaction, emit a recoverable `approval`/`error` state with `Open raw terminal`; do not invent a
   437	permission response or silently approve it.
   438	
   439	### Step 5: Run fake-process verification
   440	
   441	Run:
   442	
   443	```bash
   444	cd tauri-svelte-preview/src-tauri
   445	RUST_TEST_THREADS=1 cargo test agent_conversation::claude -- --nocapture
   446	```
   447	
   448	Expected: PASS without a live Claude request or API charge.
   449	
   450	Do not commit unless the user explicitly authorizes a commit.
   451	
   452	---
   453	
   454	## Task 6: Wire the frontend service and structured Session panel
   455	
   456	**Files:**
   457	
   458	- Create `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts`
   459	- Create `tauri-svelte-preview/src/lib/shell/components/ConversationPanel.svelte`
   460	- Create `tauri-svelte-preview/src/lib/shell/components/ConversationTimeline.svelte`
   461	- Create `tauri-svelte-preview/src/lib/shell/components/ConversationComposer.svelte`
   462	- Modify `tauri-svelte-preview/src/lib/tauriSource.ts`
   463	- Modify `tauri-svelte-preview/src/routes/next/+page.svelte`
   464	- Create `tauri-svelte-preview/scripts/agentConversationService.test.mjs`
   465	- Modify `tauri-svelte-preview/package.json`
   650	- WIP items, especially provider approval limitations or packaging paths;
   651	- all files changed;
   652	- browser/process cleanup status;
   653	- a short .NET-to-Rust explanation: registry as a singleton service, adapter as an interface-like
   654	  boundary, Tokio tasks as async background workers, channels as typed event queues, and Serde as
   655	  the JSON contract mapper.
   656	
   657	Do not label the feature complete if only fake providers or browser preview were proved. Do not
   658	commit or push unless the user explicitly asks.
   659	
   660	---
   661	
   662	## Acceptance checklist
   663	
   664	- [ ] Claude and Codex default to the structured Session surface.
   665	- [ ] Composer messages appear as user messages without terminal scraping.
   666	- [ ] Raw terminal is one click away and does not duplicate the provider process.
   667	- [ ] Three sessions keep independent conversation, draft, Editor, Browser, Diff, and Dockview
   668	      layout state while background streaming continues.
   669	- [ ] Codex app-server and Claude stream-json both support resume, stop, tool state, errors, and
   670	      the approvals their structured protocols expose.

--- guide ---
     1	# Conversation Workbench: Rust Guide for .NET Developers
     2	
     3	## What owns the conversation
     4	
     5	The Claude or Codex process that is already running in the session PTY remains the only agent process. The structured Session view reads that process's JSONL transcript and sends user input to its existing PTY. It never starts a second provider process.
     6	
     7	In .NET terms, the PTY is the authoritative hosted service. The Svelte conversation view is a projection over its durable event log, not another service instance.
     8	
     9	## Tauri commands are the controller boundary
    10	
    11	`src-tauri/src/agent_conversation/mod.rs` exposes narrow Tauri commands. They play roughly the same role as ASP.NET controller actions:
    12	
    13	- `read_agent_conversation_transcript` returns a normalized snapshot for one validated parent or child session.
    14	- `save_agent_conversation_attachment` validates and stores one pasted image in the app's managed data directory.
    15	- Existing connection commands remain available for the older structured-provider implementation, but the current Session view does not resume a second provider through them.
    16	
    17	Svelte calls these commands with Tauri `invoke`. Rust validates identifiers and filesystem boundaries before touching disk.
    18	
    19	## Transcript parsing
    20	
    21	`src-tauri/src/agent_conversation/transcript.rs` is a read-only adapter over the providers' JSONL formats. It returns:
    22	
    23	- chronological user and assistant messages;
    24	- model, effort, approval policy, and token/context metadata when the transcript records them;
    25	- child-agent descriptors and lifecycle state;
    26	- a child transcript when a validated child ID is selected.
    27	
    28	The parser uses `Option<T>` for data that providers may omit. This is the Rust equivalent of nullable reference/value types: the UI shows `unknown` instead of inventing a default.
    29	
    30	Transcript paths are cached after discovery, so the 500 ms refresh reads the known file instead of recursively scanning the provider's session tree every time.
    31	
    32	## Ownership and borrowing in the attachment path

exec
/bin/zsh -lc "git log --all --oneline -S\"setConversationMode(ownedId, 'structured')\" -- tauri-svelte-preview/src/routes/next/+page.svelte tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
printf '\\n--- raw mode history ---\\n'
git log --all --oneline -S\"mode: 'raw'\" -- tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts tauri-svelte-preview/src/routes/next/+page.svelte
printf '\\n--- terminal projection follow ---\\n'
git log --follow --oneline -- tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs | head -40
printf '\\n--- provider runtime integration history ---\\n'
git log --all --oneline -- tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs | head -40" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
5884139 checkpoint: preserve native workbench integration WIP

--- raw mode history ---

--- terminal projection follow ---
bbf6f3a tsk-808: A3-terminal incremental transcript projection replacing whole-tail reads

--- provider runtime integration history ---
7a07d8e feat: agent commit messages and Push & Create PR flow
bdd59d0 feat: real resource attribution and usage analytics
ba46633 conversation: native CLI handoff between structured and terminal modes
9434722 tsk-808: A6 deterministic WorkflowEngine over the orchestration ledger
325d780 tsk-808: A2 AgentRuntimeManager, ACP transport, pinned provider registry

exec
/bin/zsh -lc "git status --short && git rev-parse --short HEAD && git show --no-patch --format='%H %s' HEAD" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
0b409ed
0b409edc0c2fc04db8645c0f01d6e51e63a90d8a docs: capture round-2 native acceptance feedback with screenshots

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/layout/centerDock.ts | sed -n '159,191p;267,340p;390,455p'
printf '\\n--- TerminalSurface mount ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte | sed -n '1,230p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   159	  /**
   160	   * The layout a fresh dock opens with: the conversation panels on the left, the
   161	   * display panels stacked as tabs in a second group to their right. Both sides
   162	   * may be empty — a roster that is all conversation just never opens the second
   163	   * group, and one that is all display opens a single group with no anchor to
   164	   * sit beside (asking dockview to position against a panel that does not exist
   165	   * yet throws, so that case adds the first panel with no position at all).
   166	   */
   167	  const buildDefault = (): void => {
   168	    const conversation = options.panels.filter((panel) => panel.group !== 'display');
   169	    const display = options.panels.filter((panel) => panel.group === 'display');
   170	    for (const panel of conversation) addPanelFor(panel);
   171	
   172	    const anchor = conversation[0];
   173	    const [leadDisplay, ...stackedDisplay] = display;
   174	    if (leadDisplay) {
   175	      addPanelFor(
   176	        leadDisplay,
   177	        anchor ? { referencePanel: anchor.id, direction: 'right' } : undefined
   178	      );
   179	      for (const panel of stackedDisplay) {
   180	        addPanelFor(panel, { referencePanel: leadDisplay.id, direction: 'within' });
   181	      }
   182	    }
   183	
   184	    // Every add above made its own panel the active one, so the last display tab
   185	    // is showing — which is now Diff, an empty pane until a file is picked. Put
   186	    // the display group back on its first tab, then hand focus to the front of
   187	    // the roster.
   188	    if (leadDisplay) panelById(leadDisplay.id)?.api.setActive();
   189	    const opening = anchor ?? leadDisplay;
   190	    if (opening) panelById(opening.id)?.api.setActive();
   191	  };
   267	  layoutToContainer();
   268	
   269	  let migratedFromV4 = false;
   270	
   271	  runSynchronized(() => {
   272	    const stored = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY);
   273	    if (stored && panelSetMatches(dockPanelIds(stored), specs.keys())) {
   274	      try {
   275	        api.fromJSON(stored as never);
   276	        return;
   277	      } catch {
   278	        try {
   279	          api.clear();
   280	        } catch {
   281	          // fall through
   282	        }
   283	      }
   284	    }
   285	
   286	    /**
   287	     * v4 has the same arrangement as the current roster minus Agents. Restore
   288	     * that exact set first, then add the new tab beside Session Library through
   289	     * the live Dockview API. This preserves the serialized groups/sizes/order
   290	     * and avoids hand-editing Dockview's private grid tree. Any mismatch or
   291	     * Dockview rejection falls through to the safe current defaults.
   292	     */
   293	    const previous = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY_V4);
   294	    if (previous && panelSetMatches(dockPanelIds(previous), CENTER_PANEL_IDS_V4)) {
   295	      try {
   296	        api.fromJSON(previous as never);
   297	        const activePanelId = api.activePanel?.id ?? null;
   298	        const library = specs.get('session-library');
   299	        const agents = specs.get('agents');
   300	        if (!library || !agents || !panelById('session-library')) {
   301	          throw new Error('Agents migration roster is incomplete');
   302	        }
   303	        addPanelFor(agents, { referencePanel: 'session-library', direction: 'within' });
   304	        if (activePanelId) panelById(activePanelId)?.api.setActive();
   305	        migratedFromV4 = true;
   306	        return;
   307	      } catch {
   308	        try {
   309	          api.clear();
   310	        } catch {
   311	          // fall through
   312	        }
   313	      }
   314	    }
   315	
   316	    buildDefault();
   317	  });
   318	  normalizeSessionLibraryTitle();
   319	
   320	  const persistSoon = (): void => {
   321	    if (synchronizingDepth > 0 || disposed) return;
   322	    if (persistTimer !== null) clearTimeout(persistTimer);
   323	    persistTimer = setTimeout(() => {
   324	      persistTimer = null;
   325	      if (disposed) return;
   326	      // Never store a dock measured at zero: the sizes in it are meaningless and
   327	      // the next launch would restore from them.
   328	      if (api.width <= 0 || api.height <= 0) return;
   329	      let ok = false;
   330	      try {
   331	        // `toJSON` runs inside the guard too: a dock in an unexpected state can
   332	        // throw from it, and an unhandled throw in here kills the timer.
   333	        ok = saveLayout(options.storage, CENTER_LAYOUT_KEY, api.toJSON());
   334	      } catch {
   335	        ok = false;
   336	      }
   337	      options.onLayoutPersisted?.(ok);
   338	    }, PERSIST_DEBOUNCE_MS);
   339	  };
   340	
   390	      clearLayout(options.storage, CENTER_LAYOUT_KEY);
   391	      runSynchronized(() => {
   392	        api.clear();
   393	        buildDefault();
   394	      });
   395	      // The guard above is still up — it releases on a timer — so ask for the
   396	      // persist on the timer after it. Callbacks with the same delay run in the
   397	      // order they were scheduled, and the release was scheduled first.
   398	      setTimeout(persistSoon, 0);
   399	    },
   400	    dispose(): void {
   401	      disposed = true; // renderer dispose() no-ops: page teardown owns the DOM now
   402	      if (persistTimer !== null) clearTimeout(persistTimer);
   403	      for (const listener of listeners) listener.dispose();
   404	      api.dispose();
   405	    }
   406	  };
   407	}

--- TerminalSurface mount ---
     1	<script lang="ts">
     2	  /**
     3	   * TerminalSurface.svelte — the stack of xterm hosts for the /next shell.
     4	   *
     5	   * ONE rule, and it is the whole reason this component is separate: a host is
     6	   * NEVER wrapped in `{#if active}`. Every owned session with a (possibly
     7	   * pending) PTY keeps a mounted host for the entire life of the page, and
     8	   * visibility is toggled by the terminal manager through `view.setVisible()`
     9	   * (which sets `display` on the host). Gating the markup instead would destroy
    10	   * the xterm DOM on every switch — the old shell's "my agent restarted when I
    11	   * clicked away" bug, in new clothes.
    12	   *
    13	   * The component does no IO and holds no state: it hands each host element to
    14	   * the page through `registerHost` exactly once, on mount.
    15	   *
    16	   * It does own one other thing, and it is the only place that can: WHEN the
    17	   * terminal has to be measured again. See `watchHost` below for what goes wrong
    18	   * without it.
    19	   */
    20	  import '@xterm/xterm/css/xterm.css';
    21	  import type { OwnedSession } from '$lib/shell/ownedSessions';
    22	
    23	  interface Props {
    24	    /** Sessions CommandBar owns (rail.owned). */
    25	    owned: OwnedSession[];
    26	    /** `ownedId` of the session the manager is showing, if any. */
    27	    activeOwnedId: string | null;
    28	    /** Called once per host element, as soon as it is in the DOM. */
    29	    registerHost(ownedId: string, host: HTMLElement): void;
    30	    /**
    31	     * This session's terminal needs measuring again — it just appeared, its box
    32	     * changed size, or the font it draws with finished loading.
    33	     *
    34	     * Safe to call often: measuring re-reports the grid through the same path
    35	     * every other measurement uses, and a grid that has not actually changed
    36	     * costs nothing (the terminal service drops a resize that matches the size
    37	     * the session is already at).
    38	     */
    39	    onHostLayout?(ownedId: string): void;
    40	  }
    41	
    42	  let { owned, activeOwnedId, registerHost, onHostLayout }: Props = $props();
    43	
    44	  /**
    45	   * An exited session keeps its host only while its PTY id is still around
    46	   * (so its final scrollback stays readable); once that is gone there is
    47	   * nothing to render.
    48	   */
    49	  const hosted = $derived(owned.filter((session) => session.state !== 'exited' || session.ptySessionId));
    50	
    51	  /**
    52	   * How many animation frames to keep re-measuring for after a terminal appears.
    53	   *
    54	   * It is not one, and that is the whole fix. xterm works out how big one
    55	   * character is by measuring it on screen, and a terminal built inside a hidden
    56	   * host cannot do that — so it holds a character size of zero. Showing the host
    57	   * does start a re-measure, but xterm starts it from a browser notification
    58	   * ("this element is on screen now") that is not delivered until the end of the
    59	   * frame, well after the code that made the host visible has finished. Measuring
    60	   * the grid in that gap is the bug in screenshots 46 and 52: xterm still has no
    61	   * character size, so the fit does nothing and quietly reports the OLD grid,
    62	   * which is then what the session is resized to. Nothing measures it again
    63	   * afterwards, so the terminal stays wrong — the prompt sits above the line you
    64	   * type on, the top of the input box is cut off, and the bottom of the output
    65	   * runs past the end of the panel — until a divider is dragged.
    66	   *
    67	   * Four frames is comfortably past that first one, and the extra measurements
    68	   * are free: a grid that has not changed sends nothing to the session.
    69	   */
    70	  const FRAMES_AFTER_APPEARING = 4;
    71	
    72	  /**
    73	   * Svelte action: publish the host element to the page, and from then on say
    74	   * when this session's terminal needs measuring again.
    75	   *
    76	   * Three things ask for that, and before this the shell listened for none of
    77	   * them — the only re-measure in the whole shell came from the middle tab area
    78	   * reporting a layout, which a newly created session never triggers:
    79	   *
    80	   *  - the host appeared (the terminal manager sets `display` on it directly,
    81	   *    which is why this watches the element's own style rather than any state);
    82	   *  - its box changed size (a dragged divider, a resized window, a folded
    83	   *    column);
    84	   *  - the font finished loading, which changes how wide a character is after
    85	   *    xterm has already measured one — the same wrong-grid symptom, arriving a
    86	   *    beat later.
    87	   */
    88	  function host(node: HTMLElement, ownedId: string) {
    89	    registerHost(ownedId, node);
    90	
    91	    /** Is this host the one on screen? A hidden host has nothing to measure. */
    92	    const shown = (): boolean => node.style.display !== 'none';
    93	
    94	    let framesLeft = 0;
    95	    let frame = 0;
    96	
    97	    /** Ask for a measurement now and on the next few frames. */
    98	    function remeasure(): void {
    99	      if (!shown()) return;
   100	      framesLeft = FRAMES_AFTER_APPEARING;
   101	      if (frame !== 0) return;
   102	      const step = (): void => {
   103	        onHostLayout?.(ownedId);
   104	        framesLeft -= 1;
   105	        frame = framesLeft > 0 ? requestAnimationFrame(step) : 0;
   106	      };
   107	      frame = requestAnimationFrame(step);
   108	    }
   109	
   110	    const sizes = new ResizeObserver(() => remeasure());
   111	    sizes.observe(node);
   112	
   113	    const shownOrHidden = new MutationObserver(() => remeasure());
   114	    shownOrHidden.observe(node, { attributes: true, attributeFilter: ['style'] });
   115	
   116	    const fonts = document.fonts ?? null;
   117	    const onFontsLoaded = (): void => remeasure();
   118	    fonts?.addEventListener('loadingdone', onFontsLoaded);
   119	
   120	    return {
   121	      destroy(): void {
   122	        if (frame !== 0) cancelAnimationFrame(frame);
   123	        sizes.disconnect();
   124	        shownOrHidden.disconnect();
   125	        fonts?.removeEventListener('loadingdone', onFontsLoaded);
   126	      }
   127	    };
   128	  }
   129	</script>
   130	
   131	<div class="surface">
   132	  {#each hosted as session (session.ownedId)}
   133	    <!--
   134	      Hosts start HIDDEN (inline `display: none`) and only the manager turns one
   135	      on, via `view.setVisible(true)` -> `host.style.display = 'block'`. Two bugs
   136	      this closes: a view created while another one is already active is never
   137	      told to hide (so its `inset: 0` host would cover the active terminal), and
   138	      a host that never gets a view at all (an exited session whose PTY died
   139	      while the app was closed) would sit on top as a transparent click-blocker.
   140	      The attribute is static, so Svelte never re-applies it over the manager.
   141	    -->
   142	    <div
   143	      class="term-host"
   144	      style="display: none"
   145	      data-owned-id={session.ownedId}
   146	      use:host={session.ownedId}
   147	    ></div>
   148	  {/each}
   149	
   150	  <!--
   151	    The second case is the one that matters: a session whose terminal was closed
   152	    has no host left, and the manager has no view to show for it, so whichever
   153	    terminal was last on screen would simply stay there — live, focused, and
   154	    belonging to a DIFFERENT session. The overlay is `inset: 0`, so it covers it
   155	    and says what actually happened.
   156	  -->
   157	  {#if activeOwnedId === null}
   158	    <div class="surface-empty">
   159	      <p>No session selected.</p>
   160	      <p class="hint">Pick a session in the rail, or resume one to start a terminal.</p>
   161	    </div>
   162	  {:else if !hosted.some((session) => session.ownedId === activeOwnedId)}
   163	    <div class="surface-empty">
   164	      <p>This session’s terminal is closed.</p>
   165	      <p class="hint">The transcript is still on disk; the session stays on your list.</p>
   166	    </div>
   167	  {/if}
   168	</div>
   169	
   170	<style>
   171	  .surface {
   172	    position: relative;
   173	    height: 100%;
   174	    width: 100%;
   175	    overflow: hidden;
   176	    background: var(--color-surface);
   177	  }
   178	
   179	  .term-host {
   180	    position: absolute;
   181	    inset: 0;
   182	    padding: 6px 8px;
   183	  }
   184	
   185	  .surface-empty {
   186	    position: absolute;
   187	    inset: 0;
   188	    display: flex;
   189	    flex-direction: column;
   190	    align-items: center;
   191	    justify-content: center;
   192	    gap: 6px;
   193	    background: var(--color-bg);
   194	    color: var(--color-text-2);
   195	    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
   196	    font-size: 12px;
   197	  }
   198	
   199	  .surface-empty p {
   200	    margin: 0;
   201	  }
   202	
   203	  .hint {
   204	    color: var(--color-text-3);
   205	    font-size: 12px;
   206	  }
   207	</style>

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs | sed -n '1,90p;194,290p;318,375p'
printf '\\n--- transcript projector reads ---\\n'
rg -n -C 4 \"read\\(|cached|poll|read_to_string|transcript\" tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs tauri-svelte-preview/src-tauri/src/agent_conversation/transcript | head -220" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	use serde::{Deserialize, Serialize};
     2	use std::collections::{HashMap, HashSet, VecDeque};
     3	use std::path::PathBuf;
     4	use std::sync::mpsc::{self, Sender};
     5	use std::sync::{Arc, Mutex};
     6	use std::thread::{self, JoinHandle};
     7	use std::time::{Duration, SystemTime, UNIX_EPOCH};
     8	use tauri::{Emitter, Runtime};
     9	
    10	use super::protocol::{AgentConversationProvider, AgentEvent, AgentRawFrameReference};
    11	use super::transcript::{self, FileIdentity, ProjectedRecord, TranscriptLocation};
    12	
    13	pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
    14	const WATCH_INTERVAL: Duration = Duration::from_millis(250);
    15	const MAX_INCREMENTAL_BYTES: u64 = 1024 * 1024;
    16	const RECONCILE_EVERY_POLLS: u32 = 40;
    17	const MAX_RECONCILIATION_EVENTS: usize = 512;
    18	const MAX_SEEN_RECORDS: usize = 4096;
    19	const MAX_RECENT_EVENTS: usize = 2000;
    20	
    21	#[derive(Clone, Debug, Deserialize)]
    22	#[serde(rename_all = "camelCase")]
    23	pub struct StartTerminalProjectionRequest {
    24	    pub owned_id: String,
    25	    pub provider: AgentConversationProvider,
    26	    pub native_session_id: String,
    27	    pub generation: u64,
    28	}
    29	
    30	#[derive(Clone, Debug, Serialize)]
    31	#[serde(rename_all = "camelCase")]
    32	pub struct TerminalProjectionRegistration {
    33	    pub generation: u64,
    34	    pub events: Vec<AgentEvent>,
    35	}
    36	
    37	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
    38	enum ReconcileReason {
    39	    Initial,
    40	    Periodic,
    41	    Truncated,
    42	    Rotated,
    43	    Archived,
    44	    Gap,
    45	}
    46	
    47	#[derive(Default)]
    48	struct ProjectionCursor {
    49	    canonical_path: Option<PathBuf>,
    50	    identity: Option<FileIdentity>,
    51	    offset: u64,
    52	    partial_line: Vec<u8>,
    53	    seen: HashSet<String>,
    54	    seen_order: VecDeque<String>,
    55	    polls_since_reconcile: u32,
    56	    last_reconcile: Option<ReconcileReason>,
    57	}
    58	
    59	struct TranscriptProjector {
    60	    provider: AgentConversationProvider,
    61	    native_session_id: String,
    62	    cursor: ProjectionCursor,
    63	}
    64	
    65	impl TranscriptProjector {
    66	    fn new(provider: AgentConversationProvider, native_session_id: String) -> Self {
    67	        Self {
    68	            provider,
    69	            native_session_id,
    70	            cursor: ProjectionCursor::default(),
    71	        }
    72	    }
    73	
    74	    fn poll(&mut self) -> Result<Vec<ProjectedRecord>, String> {
    75	        let cached = self
    76	            .cursor
    77	            .canonical_path
    78	            .as_deref()
    79	            .map(transcript::inspect)
    80	            .transpose()?
    81	            .flatten();
    82	        let Some(location) = cached.or(transcript::discover(
    83	            self.provider,
    84	            &self.native_session_id,
    85	        )?) else {
    86	            return Ok(Vec::new());
    87	        };
    88	        self.poll_location(location)
    89	    }
    90	
   194	pub struct TerminalProjectionRegistry {
   195	    watchers: Mutex<HashMap<String, WatcherHandle>>,
   196	}
   197	
   198	impl TerminalProjectionRegistry {
   199	    pub fn start<R: Runtime>(
   200	        &self,
   201	        app: tauri::AppHandle<R>,
   202	        request: StartTerminalProjectionRequest,
   203	    ) -> Result<TerminalProjectionRegistration, String> {
   204	        let owned_id = required_id(&request.owned_id, "Owned session id")?;
   205	        let native_session_id = transcript::safe_session_id(&request.native_session_id)?;
   206	        let previous = self
   207	            .watchers
   208	            .lock()
   209	            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
   210	            .remove(&owned_id);
   211	        let previous_generation = previous
   212	            .as_ref()
   213	            .map(|watcher| watcher.generation)
   214	            .unwrap_or(0);
   215	        if let Some(previous) = previous {
   216	            previous.stop();
   217	        }
   218	        let generation = request
   219	            .generation
   220	            .max(previous_generation)
   221	            .saturating_add(1);
   222	        let mut projector = TranscriptProjector::new(request.provider, native_session_id.clone());
   223	        let initial = projector.poll()?;
   224	        let events = Arc::new(Mutex::new(VecDeque::new()));
   225	        let mut next_sequence = 1_u64;
   226	        for record in initial {
   227	            let event = canonical_event(
   228	                &owned_id,
   229	                request.provider,
   230	                &native_session_id,
   231	                generation,
   232	                next_sequence,
   233	                record,
   234	            );
   235	            next_sequence = next_sequence.saturating_add(1);
   236	            remember_and_emit(&app, &events, event);
   237	        }
   238	        let registration_events = events
   239	            .lock()
   240	            .map_err(|_| "Terminal projection snapshot is unavailable".to_string())?
   241	            .iter()
   242	            .cloned()
   243	            .collect();
   244	        let (stop, stopped) = mpsc::channel();
   245	        let thread_owned_id = owned_id.clone();
   246	        let thread_events = Arc::clone(&events);
   247	        let join = thread::Builder::new()
   248	            .name(format!("terminal-projection-{owned_id}"))
   249	            .spawn(move || loop {
   250	                match stopped.recv_timeout(WATCH_INTERVAL) {
   251	                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
   252	                    Err(mpsc::RecvTimeoutError::Timeout) => {}
   253	                }
   254	                let Ok(records) = projector.poll() else {
   255	                    continue;
   256	                };
   257	                for record in records {
   258	                    let event = canonical_event(
   259	                        &thread_owned_id,
   260	                        request.provider,
   261	                        &native_session_id,
   262	                        generation,
   263	                        next_sequence,
   264	                        record,
   265	                    );
   266	                    next_sequence = next_sequence.saturating_add(1);
   267	                    remember_and_emit(&app, &thread_events, event);
   268	                }
   269	            })
   270	            .map_err(|error| format!("Could not start terminal transcript projection: {error}"))?;
   271	        self.watchers
   272	            .lock()
   273	            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
   274	            .insert(
   275	                owned_id,
   276	                WatcherHandle {
   277	                    generation,
   278	                    stop,
   279	                    join: Some(join),
   280	                },
   281	            );
   282	        Ok(TerminalProjectionRegistration {
   283	            generation,
   284	            events: registration_events,
   285	        })
   286	    }
   287	
   288	    pub fn stop(&self, owned_id: &str) -> Result<bool, String> {
   289	        let owned_id = required_id(owned_id, "Owned session id")?;
   290	        let watcher = self
   318	fn canonical_event(
   319	    owned_id: &str,
   320	    provider: AgentConversationProvider,
   321	    native_session_id: &str,
   322	    generation: u64,
   323	    sequence: u64,
   324	    record: ProjectedRecord,
   325	) -> AgentEvent {
   326	    AgentEvent {
   327	        event_type: record.event_type,
   328	        owned_id: owned_id.to_string(),
   329	        provider,
   330	        provider_instance_id: format!("terminal-transcript:{native_session_id}"),
   331	        generation,
   332	        sequence,
   333	        timestamp_ms: if record.timestamp_ms == 0 {
   334	            now_millis()
   335	        } else {
   336	            record.timestamp_ms
   337	        },
   338	        native_session_id: Some(native_session_id.to_string()),
   339	        turn_id: None,
   340	        item_id: record.item_id.clone(),
   341	        request_id: None,
   342	        payload: record.payload,
   343	        provider_metadata: Some(std::collections::BTreeMap::from([
   344	            (
   345	                "source".into(),
   346	                serde_json::Value::String("terminal-transcript".into()),
   347	            ),
   348	            ("historical".into(), serde_json::Value::Bool(true)),
   349	        ])),
   350	        raw_frame_reference: Some(AgentRawFrameReference {
   351	            id: record.key,
   352	            redacted: true,
   353	        }),
   354	    }
   355	}
   356	
   357	fn remember_and_emit<R: Runtime>(
   358	    app: &tauri::AppHandle<R>,
   359	    events: &Arc<Mutex<VecDeque<AgentEvent>>>,
   360	    event: AgentEvent,
   361	) {
   362	    if let Ok(mut recent) = events.lock() {
   363	        recent.push_back(event.clone());
   364	        while recent.len() > MAX_RECENT_EVENTS {
   365	            recent.pop_front();
   366	        }
   367	    }
   368	    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);
   369	}
   370	
   371	fn required_id(value: &str, label: &str) -> Result<String, String> {
   372	    let value = value.trim();
   373	    if value.is_empty() {
   374	        Err(format!("{label} is required"))
   375	    } else {

--- transcript projector reads ---
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-7-use std::time::{Duration, SystemTime, UNIX_EPOCH};
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-8-use tauri::{Emitter, Runtime};
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-9-
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-10-use super::protocol::{AgentConversationProvider, AgentEvent, AgentRawFrameReference};
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:11:use super::transcript::{self, FileIdentity, ProjectedRecord, TranscriptLocation};
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-12-
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-13-pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-14-const WATCH_INTERVAL: Duration = Duration::from_millis(250);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-15-const MAX_INCREMENTAL_BYTES: u64 = 1024 * 1024;
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-51-    offset: u64,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-52-    partial_line: Vec<u8>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-53-    seen: HashSet<String>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-54-    seen_order: VecDeque<String>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:55:    polls_since_reconcile: u32,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-56-    last_reconcile: Option<ReconcileReason>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-57-}
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-58-
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-59-struct TranscriptProjector {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-70-            cursor: ProjectionCursor::default(),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-71-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-72-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-73-
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:74:    fn poll(&mut self) -> Result<Vec<ProjectedRecord>, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:75:        let cached = self
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-76-            .cursor
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-77-            .canonical_path
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-78-            .as_deref()
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:79:            .map(transcript::inspect)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-80-            .transpose()?
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-81-            .flatten();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:82:        let Some(location) = cached.or(transcript::discover(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-83-            self.provider,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-84-            &self.native_session_id,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-85-        )?) else {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-86-            return Ok(Vec::new());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-87-        };
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:88:        self.poll_location(location)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-89-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-90-
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:91:    fn poll_location(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-92-        &mut self,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-93-        location: TranscriptLocation,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-94-    ) -> Result<Vec<ProjectedRecord>, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-95-        let reason = match (&self.cursor.identity, &self.cursor.canonical_path) {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-105-                if location.len.saturating_sub(self.cursor.offset) > MAX_INCREMENTAL_BYTES =>
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-106-            {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-107-                Some(ReconcileReason::Gap)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-108-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:109:            _ if self.cursor.polls_since_reconcile >= RECONCILE_EVERY_POLLS => {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-110-                Some(ReconcileReason::Periodic)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-111-            }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-112-            _ => None,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-113-        };
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-114-        if let Some(reason) = reason {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-115-            return self.reconcile(location, reason);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-116-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:117:        self.cursor.polls_since_reconcile = self.cursor.polls_since_reconcile.saturating_add(1);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-118-        if location.len == self.cursor.offset {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-119-            return Ok(Vec::new());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-120-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:121:        let bytes = transcript::read_range(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-122-            &location.canonical_path,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-123-            self.cursor.offset,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-124-            location.len.saturating_sub(self.cursor.offset),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-125-        )?;
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-127-        self.cursor.canonical_path = Some(location.canonical_path);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-128-        self.cursor.identity = Some(location.identity);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-129-        let mut combined = std::mem::take(&mut self.cursor.partial_line);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-130-        combined.extend_from_slice(&bytes);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:131:        let (lines, partial) = transcript::complete_lines(&combined, false);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-132-        let records = self.project_lines(lines);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-133-        self.cursor.partial_line = partial;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-134-        Ok(records)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-135-    }
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-138-        &mut self,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-139-        location: TranscriptLocation,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-140-        reason: ReconcileReason,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-141-    ) -> Result<Vec<ProjectedRecord>, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:142:        let (start, bytes) = transcript::read_bounded(&location.canonical_path, location.len)?;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:143:        let (lines, partial) = transcript::complete_lines(&bytes, start > 0);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-144-        let mut records = self.project_lines(lines);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-145-        if records.len() > MAX_RECONCILIATION_EVENTS {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-146-            records = records.split_off(records.len() - MAX_RECONCILIATION_EVENTS);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-147-        }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-148-        self.cursor.canonical_path = Some(location.canonical_path);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-149-        self.cursor.identity = Some(location.identity);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-150-        self.cursor.offset = location.len;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-151-        self.cursor.partial_line = partial;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:152:        self.cursor.polls_since_reconcile = 0;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-153-        self.cursor.last_reconcile = Some(reason);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-154-        Ok(records)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-155-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-156-
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-157-    fn project_lines(&mut self, lines: Vec<&[u8]>) -> Vec<ProjectedRecord> {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-158-        let mut projected = Vec::new();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-159-        for line in lines {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-160-            for record in
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:161:                transcript::parse_durable_line(self.provider, &self.native_session_id, line)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-162-            {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-163-                if self.cursor.seen.insert(record.key.clone()) {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-164-                    self.cursor.seen_order.push_back(record.key.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-165-                    projected.push(record);
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-201-        app: tauri::AppHandle<R>,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-202-        request: StartTerminalProjectionRequest,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-203-    ) -> Result<TerminalProjectionRegistration, String> {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-204-        let owned_id = required_id(&request.owned_id, "Owned session id")?;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:205:        let native_session_id = transcript::safe_session_id(&request.native_session_id)?;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-206-        let previous = self
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-207-            .watchers
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-208-            .lock()
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-209-            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-219-            .generation
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-220-            .max(previous_generation)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-221-            .saturating_add(1);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-222-        let mut projector = TranscriptProjector::new(request.provider, native_session_id.clone());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:223:        let initial = projector.poll()?;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-224-        let events = Arc::new(Mutex::new(VecDeque::new()));
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-225-        let mut next_sequence = 1_u64;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-226-        for record in initial {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-227-            let event = canonical_event(
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-250-                match stopped.recv_timeout(WATCH_INTERVAL) {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-251-                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-252-                    Err(mpsc::RecvTimeoutError::Timeout) => {}
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-253-                }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:254:                let Ok(records) = projector.poll() else {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-255-                    continue;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-256-                };
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-257-                for record in records {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-258-                    let event = canonical_event(
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-266-                    next_sequence = next_sequence.saturating_add(1);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-267-                    remember_and_emit(&app, &thread_events, event);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-268-                }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-269-            })
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:270:            .map_err(|error| format!("Could not start terminal transcript projection: {error}"))?;
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-271-        self.watchers
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-272-            .lock()
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-273-            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-274-            .insert(
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-326-    AgentEvent {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-327-        event_type: record.event_type,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-328-        owned_id: owned_id.to_string(),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-329-        provider,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:330:        provider_instance_id: format!("terminal-transcript:{native_session_id}"),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-331-        generation,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-332-        sequence,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-333-        timestamp_ms: if record.timestamp_ms == 0 {
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-334-            now_millis()
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-342-        payload: record.payload,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-343-        provider_metadata: Some(std::collections::BTreeMap::from([
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-344-            (
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-345-                "source".into(),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:346:                serde_json::Value::String("terminal-transcript".into()),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-347-            ),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-348-            ("historical".into(), serde_json::Value::Bool(true)),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-349-        ])),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-350-        raw_frame_reference: Some(AgentRawFrameReference {
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-432-            ),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-433-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-434-        let mut projector =
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-435-            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:436:        assert_eq!(projector.poll_location(location(&path)).unwrap().len(), 1);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-437-        fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"a2\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"second\"}]}}\n").unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:438:        let records = projector.poll_location(location(&path)).unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-439-        assert_eq!(records.len(), 1);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-440-        assert_eq!(records[0].item_id.as_deref(), Some("a2"));
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-441-        fs::remove_dir_all(path.parent().unwrap()).unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-442-    }
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-451-            fixture.strip_suffix(b"\n").unwrap_or(fixture),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-452-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-453-        let mut projector =
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-454-            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:455:        assert!(projector.poll_location(location(&path)).unwrap().is_empty());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-456-        assert!(!projector.cursor.partial_line.is_empty());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-457-        fs::OpenOptions::new()
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-458-            .append(true)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-459-            .open(&path)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-460-            .unwrap()
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-461-            .write_all(b"}]} }\n")
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-462-            .unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:463:        assert_eq!(projector.poll_location(location(&path)).unwrap().len(), 1);
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-464-        assert!(projector.cursor.partial_line.is_empty());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-465-        fs::remove_dir_all(path.parent().unwrap()).unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-466-    }
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-467-
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-474-            ),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-475-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-476-        let mut projector =
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-477-            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:478:        projector.poll_location(location(&path)).unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-479-        fs::write(&path, b"\n").unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:480:        projector.poll_location(location(&path)).unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-481-        assert_eq!(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-482-            projector.cursor.last_reconcile,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-483-            Some(ReconcileReason::Truncated)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-484-        );
--
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-489-                "../../fixtures/agent_conversation/terminal_projection/codex_append.jsonl"
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-490-            ),
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-491-        )
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-492-        .unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:493:        projector.poll_location(location(&path)).unwrap();
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-494-        assert_eq!(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-495-            projector.cursor.last_reconcile,
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-496-            Some(ReconcileReason::Rotated)
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-497-        );
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs-498-        let archived = path.with_file_name("archived.jsonl");

exec
/bin/zsh -lc "git show --stat --oneline 325d780 && git show --format=fuller 325d780 -- tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs | sed -n '1,260p'
printf '\\n--- current manager emit callers ---\\n'
rg -n \"\\.emit_payload\\(|emit_payload\\(\" tauri-svelte-preview/src-tauri/src --glob '*.rs'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
325d780 tsk-808: A2 AgentRuntimeManager, ACP transport, pinned provider registry
 tauri-svelte-preview/src-tauri/Cargo.lock          | 360 +++++++++-
 tauri-svelte-preview/src-tauri/Cargo.toml          |   1 +
 .../src/agent_conversation/capabilities.rs         | 212 ++++++
 .../src-tauri/src/agent_conversation/journal.rs    | 113 +++
 .../src-tauri/src/agent_conversation/manager.rs    | 780 +++++++++++++++++++++
 .../src-tauri/src/agent_conversation/mod.rs        | 511 ++------------
 .../src/agent_conversation/providers/acp.rs        | 180 +++++
 .../src/agent_conversation/providers/acp_client.rs | 471 +++++++++++++
 .../src/agent_conversation/providers/mod.rs        | 259 +++++++
 .../src/agent_conversation/providers/process.rs    | 238 +++++++
 tauri-svelte-preview/src-tauri/src/main.rs         |   5 +-
 tauri-svelte-preview/src-tauri/src/terminal.rs     |  71 +-
 12 files changed, 2737 insertions(+), 464 deletions(-)
commit 325d7805cef2ac3960e1686dcffb9ab6372eba4e
Author:     jcoble <coble.jesse@gmail.com>
AuthorDate: Tue Aug 4 22:52:21 2026 -0400
Commit:     jcoble <coble.jesse@gmail.com>
CommitDate: Tue Aug 4 22:52:21 2026 -0400

    tsk-808: A2 AgentRuntimeManager, ACP transport, pinned provider registry

diff --git a/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs b/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs
new file mode 100644
index 0000000..8a02773
--- /dev/null
+++ b/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs
@@ -0,0 +1,780 @@
+use std::collections::{BTreeMap, HashMap, VecDeque};
+use std::sync::{Arc, Mutex};
+use std::time::{SystemTime, UNIX_EPOCH};
+
+use tokio::sync::Mutex as AsyncMutex;
+
+use super::journal::AgentEventJournal;
+use super::protocol::{
+    AgentApprovalDecision, AgentApprovalResponse, AgentCapabilities, AgentConversationConnection,
+    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
+    AgentConversationSnapshot, AgentEvent, AgentEventType, AgentExecutionOwner,
+    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
+    AgentRequestIdentity, AgentRuntimeState, AgentSessionCapabilities, AgentUserInputResponse,
+    AgentWriterLease, AgentWriterLeaseOwner, AgentWriterLeaseTransition,
+    ConversationConnectionState, EnsureAgentConversationRequest,
+};
+use super::providers::process::validated_conversation_cwd;
+use super::providers::{
+    AcpRuntimeAdapter, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter, AgentSteeringInput,
+    InitializeAgentInput, LoadAgentSession, NewAgentSession, PermissionResponse, ProviderRegistry,
+    StructuredRuntimeHandle,
+};
+
+const SNAPSHOT_EVENT_CAP: usize = 2_000;
+const JOURNAL_EVENT_CAP: usize = 10_000;
+
+pub struct ManagedAgentSession {
+    pub owned_id: String,
+    pub provider: AgentConversationProvider,
+    pub provider_instance_id: String,
+    pub native_session_id: Option<String>,
+    pub generation: u64,
+    pub owner: AgentExecutionOwner,
+    pub state: AgentRuntimeState,
+    pub capabilities: AgentCapabilities,
+    pub next_sequence: u64,
+    pub active_turn_id: Option<String>,
+    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
+    pub recent_events: VecDeque<AgentEvent>,
+    pub writer_lease: AgentWriterLease,
+    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
+    cwd: String,
+    connection: AgentConversationConnection,
+    frontend_events: VecDeque<AgentConversationEvent>,
+    journal: AgentEventJournal,
+}
+
+#[derive(Clone)]
+pub struct AgentRuntimeManager {
+    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
+    providers: Arc<ProviderRegistry>,
+}
+
+impl Default for AgentRuntimeManager {
+    fn default() -> Self {
+        Self::new(ProviderRegistry::default())
+    }
+}
+
+impl AgentRuntimeManager {
+    pub fn new(providers: ProviderRegistry) -> Self {
+        Self {
+            sessions: Arc::new(Mutex::new(HashMap::new())),
+            providers: Arc::new(providers),
+        }
+    }
+
+    pub fn providers(&self) -> &ProviderRegistry {
+        &self.providers
+    }
+
+    pub fn ensure(
+        &self,
+        request: EnsureAgentConversationRequest,
+    ) -> Result<AgentConversationConnection, String> {
+        let owned_id = required_id(&request.owned_id, "Owned session id")?;
+        let cwd = validated_conversation_cwd(&request.cwd)?
+            .display()
+            .to_string();
+        let mut sessions = self
+            .sessions
+            .lock()
+            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
+        if let Some(current) = sessions.get(&owned_id) {
+            if current.provider == request.provider && current.cwd == cwd {
+                return Ok(current.connection.clone());
+            }
+        }
+        let prior = sessions.remove(&owned_id);
+        let generation = prior
+            .as_ref()
+            .map(|session| session.generation.saturating_add(1))
+            .unwrap_or(1);
+        let connection = AgentConversationConnection {
+            owned_id: owned_id.clone(),
+            provider: request.provider,
+            generation,
+            native_session_id: normalized_optional_id(request.native_session_id),
+            state: ConversationConnectionState::Connecting,
+        };
+        let provider_instance_id = format!("{}-{generation}", provider_id(request.provider));
+        sessions.insert(
+            owned_id.clone(),
+            ManagedAgentSession {
+                owned_id: owned_id.clone(),
+                provider: request.provider,
+                provider_instance_id,
+                native_session_id: connection.native_session_id.clone(),
+                generation,
+                owner: AgentExecutionOwner::Stopped,
+                state: AgentRuntimeState::Closed,
+                capabilities: empty_capabilities(request.provider),
+                next_sequence: 1,
+                active_turn_id: None,
+                runtime: None,
+                recent_events: VecDeque::new(),
+                writer_lease: AgentWriterLease {
+                    owned_id,
+                    generation,
+                    owner: AgentWriterLeaseOwner::None,
+                },
+                writer_lease_transition: None,
+                cwd,
+                connection: connection.clone(),
+                frontend_events: VecDeque::new(),
+                journal: AgentEventJournal::new(JOURNAL_EVENT_CAP),
+            },
+        );
+        Ok(connection)
+    }
+
+    pub async fn activate(
+        &self,
+        owned_id: &str,
+        generation: u64,
+    ) -> Result<AgentConversationConnection, String> {
+        let (provider, provider_instance_id, cwd, native_session_id) = {
+            let sessions = self
+                .sessions
+                .lock()
+                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
+            let session = current_session(&sessions, owned_id, generation)?;
+            (
+                session.provider,
+                session.provider_instance_id.clone(),
+                std::path::PathBuf::from(&session.cwd),
+                session.native_session_id.clone(),
+            )
+        };
+        let manifest = self.providers.manifest(provider)?;
+        let mut adapter = AcpRuntimeAdapter::new(manifest, owned_id.to_string(), cwd.clone());
+        let capabilities = adapter
+            .initialize(InitializeAgentInput {
+                provider,
+                provider_instance_id,
+            })
+            .await
+            .map_err(|error| error.to_string())?;
+        let started = match native_session_id {
+            Some(native_session_id) => {
+                adapter
+                    .resume_session(LoadAgentSession {
+                        cwd,
+                        native_session_id,
+                    })
+                    .await
+            }
+            None => adapter.new_session(NewAgentSession { cwd }).await,
+        }
+        .map_err(|error| error.to_string())?;
+        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
+        let mut sessions = self
+            .sessions
+            .lock()
+            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
+        let session = current_session_mut(&mut sessions, owned_id, generation)?;
+        session.capabilities = capabilities;
+        session.native_session_id = Some(started.native_session_id.clone());
+        session.connection.native_session_id = Some(started.native_session_id);
+        session.connection.state = ConversationConnectionState::Connected;
+        session.owner = AgentExecutionOwner::Structured;
+        session.state = AgentRuntimeState::Ready;
+        session.writer_lease.owner = AgentWriterLeaseOwner::Structured;
+        session.runtime = Some(runtime);
+        Ok(session.connection.clone())
+    }
+
+    pub fn emit_payload(
+        &self,
+        owned_id: &str,
+        generation: u64,
+        payload: AgentConversationPayload,
+    ) -> Result<AgentConversationEvent, String> {
+        let mut sessions = self
+            .sessions
+            .lock()
+            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
+        let session = current_session_mut(&mut sessions, owned_id, generation)?;
+        let sequence = session.next_sequence;
+        session.next_sequence = session.next_sequence.saturating_add(1);
+        if let AgentConversationPayload::Connection {
+            state,
+            native_session_id,
+        } = &payload
+        {
+            session.connection.state = *state;
+            if native_session_id.is_some() {
+                session.connection.native_session_id = native_session_id.clone();
+                session.native_session_id = native_session_id.clone();
+            }
+        }
+        let timestamp_ms = timestamp_millis();
+        let frontend_event = AgentConversationEvent {
+            owned_id: owned_id.to_string(),
+            provider: session.provider,
+            generation,
+            sequence,
+            timestamp_ms,
+            payload: payload.clone(),
+        };
+        let canonical = canonical_event(session, sequence, timestamp_ms, &payload)?;
+        session.journal.append(canonical.clone())?;
+        session.recent_events.push_back(canonical);
+        session.frontend_events.push_back(frontend_event.clone());
+        while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
+            session.recent_events.pop_front();
+        }
+        while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
+            session.frontend_events.pop_front();
+        }
+        Ok(frontend_event)
+    }
+
+    pub async fn prompt(
+        &self,
+        owned_id: &str,
+        generation: u64,
+        input: AgentPrompt,
+    ) -> Result<(), String> {
+        let runtime = self.runtime(owned_id, generation)?;
+        let started = runtime
+            .lock()
+            .await
+            .prompt(input)
+            .await
+            .map_err(|error| error.to_string())?;

--- current manager emit callers ---
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265:    pub fn emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052:            .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1078:                .emit_payload(

exec
/bin/zsh -lc "rg -n \"agent-conversation-event|TERMINAL_PROJECTION_EVENT|app\\.emit\" tauri-svelte-preview/src-tauri/src tauri-svelte-preview/src/lib/shell/conversation | sed -n '1,180p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src-tauri/src/browser.rs:339:        let _ = self.app.emit(BROWSER_TAB_NAVIGATION_EVENT, event);
tauri-svelte-preview/src-tauri/src/browser.rs:343:        let _ = self.app.emit(BROWSER_TAB_LOAD_EVENT, event);
tauri-svelte-preview/src-tauri/src/browser.rs:347:        let _ = self.app.emit(BROWSER_ELEMENT_SELECTED_EVENT, event);
tauri-svelte-preview/src-tauri/src/browser.rs:351:        let _ = self.app.emit(BROWSER_TAB_CLOSED_EVENT, event);
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:358:  unlisten = await listen<AgentConversationEvent | AgentEvent>('agent-conversation-event', ({ payload }) => {
tauri-svelte-preview/src-tauri/src/main.rs:638:        let _ = app.emit(
tauri-svelte-preview/src-tauri/src/main.rs:5340:                let _ = app.emit(SOURCE_LSP_STATUS_CHANGED_EVENT, change);
tauri-svelte-preview/src-tauri/src/terminal.rs:399:                    let _ = app.emit(
tauri-svelte-preview/src-tauri/src/terminal.rs:454:        let _ = app.emit(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:13:pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:368:    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/components/conversation/ConversationHeader.svelte | sed -n '1,240p'
printf '\\n--- conversation types mode/event ---\\n'
nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts | sed -n '1,120p;500,620p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	<script lang="ts">
     2	  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
     3	  import type { ConversationChildAgent } from '$lib/shell/conversation/conversationTypes.ts';
     4	  import type { ConversationViewMode, ConversationWorkspaceState } from '$lib/shell/conversation/conversationStore.svelte.ts';
     5	
     6	  interface Props {
     7	    active: OwnedSession;
     8	    conversation: ConversationWorkspaceState;
     9	    selectedChild: ConversationChildAgent | null;
    10	    onModeChange?(mode: ConversationViewMode): void;
    11	  }
    12	  let { active, conversation, selectedChild, onModeChange }: Props = $props();
    13	</script>
    14	
    15	<header class="conversation-header" data-testid="conversation-header">
    16	  <div class="identity"><strong data-testid="conversation-title">{selectedChild?.label ?? active.title}</strong><span>{selectedChild ? `${active.agent} child · read-only` : `${active.agent} session`}</span>{#if conversation.connectionState !== 'connected'}<small data-testid="conversation-connection-state">{conversation.connectionState}</small>{/if}</div>
    17	  <div class="header-actions"><span class="runtime-state" data-testid="conversation-runtime-state">{conversation.executionOwner}</span><button data-testid="conversation-open-raw" type="button" onclick={() => onModeChange?.('raw')}>Open raw terminal</button></div>
    18	</header>
    19	
    20	<style>.conversation-header{display:flex;justify-content:space-between;align-items:center;min-height:46px;padding:8px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 75%,transparent)}.identity{display:flex;align-items:baseline;gap:9px;min-width:0}.identity strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px}.identity span,.identity small,.runtime-state{color:var(--color-text-2);font-size:12px}.header-actions{display:flex;align-items:center;gap:9px}.header-actions button{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.header-actions button:hover{background:var(--color-hover)}.runtime-state{padding:3px 6px}</style>

--- conversation types mode/event ---
     1	export type AgentConversationProvider = 'codex' | 'claude';
     2	
     3	export type AgentConfigValue = string | number | boolean | null | AgentConfigValue[] | {
     4	  [key: string]: AgentConfigValue;
     5	};
     6	
     7	export interface AgentConfigOptionChoice {
     8	  value: AgentConfigValue;
     9	  label: string;
    10	  description?: string;
    11	}
    12	
    13	export interface AgentConfigOption {
    14	  id: string;
    15	  label: string;
    16	  category: string;
    17	  description?: string;
    18	  value: AgentConfigValue;
    19	  choices?: AgentConfigOptionChoice[];
    20	  providerMetadata?: Record<string, AgentConfigValue>;
    21	}
    22	
    23	export interface AgentCommandDescriptor {
    24	  id: string;
    25	  label: string;
    26	  description?: string;
    27	  inputHint?: string;
    28	  providerMetadata?: Record<string, AgentConfigValue>;
    29	}
    30	
    31	export type ProviderTransport = 'acp-stdio' | 'acp-websocket' | 'built-in';
    32	export type ProviderSource = 'bundled' | 'trusted' | 'configured';
    33	
    34	export interface AgentProviderManifest {
    35	  id: string;
    36	  displayName: string;
    37	  transport: ProviderTransport;
    38	  executable: string;
    39	  args: string[];
    40	  version: string;
    41	  contentHash: string;
    42	  trustedSource: ProviderSource;
    43	}
    44	
    45	export interface AgentCapabilities {
    46	  revision: number;
    47	  provider: AgentConversationProvider;
    48	  implementation: { name: string; version: string };
    49	  session: {
    50	    list: boolean;
    51	    load: boolean;
    52	    resume: boolean;
    53	    close: boolean;
    54	    steering: boolean;
    55	    fork?: boolean;
    56	  };
    57	  prompt: {
    58	    text: boolean;
    59	    image: boolean;
    60	    embeddedContext: boolean;
    61	    resourceLinks: boolean;
    62	  };
    63	  interaction: {
    64	    permissions: boolean;
    65	    structuredUserInput: boolean;
    66	    toolTerminals: boolean;
    67	    plans: boolean;
    68	    tasks: boolean;
    69	    subagents: boolean;
    70	  };
    71	  configOptions: AgentConfigOption[];
    72	  commands: AgentCommandDescriptor[];
    73	}
    74	
    75	export type AgentConversationHandoffDirection = 'structured-to-terminal' | 'terminal-to-structured';
    76	export type AgentConversationHandoffMode = 'same-session' | 'fork';
    77	export type AgentConversationHandoffPhase = 'prepare' | 'commit' | 'rollback';
    78	export type AgentConversationHandoffReceiptPhase = 'prepared' | 'committed' | 'rolled-back' | AgentConversationHandoffPhase;
    79	
    80	export interface AgentConversationHistoryBoundary {
    81	  nativeSessionId?: string | null;
    82	  firstSequence: number;
    83	  lastSequence: number;
    84	  reconciledSequence?: number | null;
    85	}
    86	
    87	export interface AgentConversationProcessTreeAssertion {
    88	  checked: boolean;
    89	  tuiLive?: boolean;
    90	  tuiReleased?: boolean;
    91	  writerCount?: number;
    92	  ptyCount?: number;
    93	  userPtyCount?: number;
    94	  sidecarCount?: number;
    95	  ptySessionId?: string | null;
    96	}
    97	
    98	export interface AgentConversationHandoffRequest {
    99	  ownedId: string;
   100	  generation: number;
   101	  direction: AgentConversationHandoffDirection;
   102	  mode: AgentConversationHandoffMode;
   103	  phase: AgentConversationHandoffPhase;
   104	  expectedOwner?: AgentWriterLeaseOwner;
   105	  targetOwnedId?: string | null;
   106	  nativeSessionId?: string | null;
   107	  ptySessionId?: string | null;
   108	  historyBoundary: AgentConversationHistoryBoundary;
   109	  processTree: AgentConversationProcessTreeAssertion;
   110	}
   111	
   112	export interface AgentConversationHandoffReceipt {
   113	  ownedId: string;
   114	  generation: number;
   115	  direction: AgentConversationHandoffDirection;
   116	  mode: AgentConversationHandoffMode;
   117	  phase: AgentConversationHandoffReceiptPhase;
   118	  previousOwner: AgentWriterLeaseOwner;
   119	  owner: AgentWriterLeaseOwner;
   120	  nativeSessionId?: string | null;
   500	  lastSequence: number;
   501	  desynchronized: boolean;
   502	  connectionState: ConversationConnectionState;
   503	  nativeSessionId?: string;
   504	  activeTurnId?: string;
   505	  usage?: ConversationUsage;
   506	  timeline: ConversationTimelineEntry[];
   507	}
   508	
   509	export interface AgentConversationConnection {
   510	  ownedId: string;
   511	  provider: AgentConversationProvider;
   512	  generation: number;
   513	  nativeSessionId?: string;
   514	  state: ConversationConnectionState;
   515	}
   516	
   517	export interface AgentConversationSnapshot {
   518	  connection: AgentConversationConnection;
   519	  lastSequence: number;
   520	  events: AgentConversationEvent[];
   521	}

context compacted
exec
/bin/zsh -lc "nl -ba /Users/blackcolours/.codex/memories/MEMORY.md | sed -n '1,35p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	# Task Group: mac-command-bar TSK-808 task delivery discipline and partial contrast implementation
     2	
     3	scope: Resume or review the TSK-808 Assembly native-workbench product wave without mistaking probes, runtime setup, or uncommitted contrast work for a closed task.
     4	applies_to: cwd=/Users/blackcolours/dev/work/mac-command-bar; reuse_rule=recheck the current worktree, PR #14 checkpoint, Notion status, and native-proof capability before execution; this rollout records a partial dirty-worktree attempt, not mergeable implementation authority.
     5	
     6	## Task 1: Execute TSK-808 orchestration and delivery, partial
     7	
     8	### rollout_summary_files
     9	
    10	- rollout_summaries/2026-08-04T10-39-44-H0db-tsk_808_orchestration_partial_contrast_no_task_delivered.md (cwd=/Users/blackcolours/dev/work/mac-command-bar, rollout_path=/Users/blackcolours/.codex/sessions/2026/08/04/rollout-2026-08-04T06-39-44-019fcc5b-9d98-7dd0-88cd-79dc1be9ecc9.jsonl, updated_at=2026-08-05T00:24:17+00:00, thread_id=019fcc5b-9d98-7dd0-88cd-79dc1be9ecc9, partial contrast work; no task was committed, merged, or closed)
    11	
    12	### keywords
    13	
    14	- TSK-808, PR-14, 167a30ae, tsk-808-assembly-wave, contrast, nextTokens.css, themeChrome.css, themeRegistry.ts, themes/contrast.ts, contrastTokens.test.mjs, WP12A, R0, Notion, dirty-worktree, ScreenCaptureKit Code=-3801, CodeLens, Peek
    15	
    16	## User preferences
    17	
    18	- when probes, reports, and tests replaced requested feature delivery, the user said “tests and things i never asked for” and “really nothing out of all the tasks got done” -> prioritize independently usable, user-requested product tasks; distinguish code changes, test/probe evidence, runtime housekeeping, milestone progress, and completed tracker tasks. [Task 1]
    19	- when PR #14 was called restored, the user clarified it was “already working” and intentionally kept as a checkpoint -> do not describe launching debug `/next` or changing window visibility as restoring or delivering product functionality. [Task 1]
    20	- when parallel agents shared a prerequisite/probe gate, the user clarified parallelization was intended for “diff parts of the app” -> parallelize independently closable product lanes after dependencies are clear, then close/commit one task before expanding scope. [Task 1]
    21	
    22	## Reusable knowledge
    23	
    24	- PR #14 / commit `167a30ae389a2280501011186f245a08d28c6725` remains the intentional checkpoint. The partial TSK-808 worktree was `tsk-808-assembly-wave`, based on `beebda6c4dad60cd2782374a36f24737cefda605`; it did not replace the primary `main` checkout. [Task 1]
    25	- The only visible product work was partial contrast theming in `nextTokens.css`, `themeChrome.css`, `themeRegistry.ts`, `themes/contrast.ts`, and `scripts/contrastTokens.test.mjs`. Focused token tests, `check:svelte`, `check`, and `git diff --check` passed; automated minima were 4.66:1 text and 3.10:1 indicators. It remained uncommitted, unmerged, unclosed, and without screenshot evidence. [Task 1]
    26	- WP12A's extension/API probes, worker/SCM/terminal ownership guards, compatibility report, and tests are exploratory/preflight evidence, not a completed user-facing task. Native checks covered Settings, Houston/Dracula switching, narrow layout, command palette, CodeLens readiness (~1.48s), and Peek (~0.71s). [Task 1]
    27	- Direct Notion evidence showed `[TSK-808] Orchestrate the Assembly native-workbench product wave` remained `To Do`; no task received a commit, PR, merge, or closure. [Task 1]
    28	
    29	## Failures and how to do differently
    30	
    31	- Symptom: a broad orchestration wave produces much activity but no deliverable. Cause: expansion before one independently tracked task is complete. Fix: map the diff to one closable task, finish its acceptance and native proof, commit/PR it, verify Notion closure, then continue. [Task 1]
    32	- Symptom: probes, fixtures, HTML reports, model routing, or temporary app/window setup are reported as delivery. Fix: count them only as support evidence unless they create a directly usable feature tied to a task. [Task 1]
    33	- Symptom: partial work becomes broad dirty WIP. Evidence: 19 tracked files changed, 834 insertions/209 deletions, plus untracked probe/report files. Fix: avoid leaving uncommitted scope after a partial lane; assess and preserve the current worktree before any cleanup. [Task 1]
    34	- Symptom: screenshot proof is absent. Cause: ScreenCaptureKit TCC denial (`Code=-3801`). Fix: report native interaction evidence separately and resolve Screen Recording permission before claiming screenshot proof. [Task 1]
    35	

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte | sed -n '108,190p'; nl -ba tauri-svelte-preview/src/lib/shell/centerDock.ts | sed -n '160,195p;268,320p'; nl -ba tauri-svelte-preview/src/lib/conversation/ConversationSurface.svelte | sed -n '1,20p;56,75p;218,282p'; nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '978,1005p;1080,1095p;1094,1152p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   108	      // builds its panels. Center Dockview's restore path deliberately measures
   109	      // its host before calling `fromJSON`; doing that while the Gridview is
   110	      // still at 0×0 leaves always-rendered panels (including Monaco's diff
   111	      // editor) with stale overlay bounds until the next activation.
   112	      frame.layout(gridHost.clientWidth, gridHost.clientHeight);
   113	      centerDock = createCenterDock(centerSlot, {
   114	        storage: window.localStorage,
   115	        // The session is what you talk to, so it opens on its own on the left;
   116	        // the editor, the browser and the diff are where you look at the
   117	        // result, so they open stacked together on the right. Source control
   118	        // itself is not here at all — it is a view of the tool column — but the
   119	        // changes it shows are, because a diff wants the width of the middle.
   120	        panels: [
   121	          { id: 'session', title: 'Session', element: sessionSlot },
   122	          { id: 'editor', title: 'Editor', element: editorSlot, group: 'display' },
   123	          { id: 'browser', title: 'Browser', element: browserSlot, group: 'display' },
   124	          { id: 'diff', title: 'Diff', element: diffSlot, group: 'display' },
   125	          {
   126	            id: 'session-library',
   127	            title: 'Session Library',
   128	            element: sessionLibrarySlot,
   129	            group: 'display'
   130	          },
   131	          { id: 'agents', title: 'Agents', element: agentsSlot, group: 'display' }
   132	        ],
   133	        onPanelLayout: (id) => {
   134	          if (id === 'session') onSessionPanelLayout?.();
   135	        },
   136	        onPanelActivated: (id) => onCenterPanelShown?.(id)
   137	      });
   138	      observer = new ResizeObserver(() => {
   139	        frame?.layout(gridHost.clientWidth, gridHost.clientHeight);
   140	      });
   141	      observer.observe(gridHost);
   142	      ready = true;
   143	      onReady?.({
   144	        resetLayout: () => {
   145	          frame?.resetLayout();
   146	          centerDock?.resetLayout();
   147	        },
   148	        showCenterPanel: (id: string) => centerDock?.activatePanel(id),
   149	        captureCenterLayout: () => centerDock?.captureLayout() ?? null,
   150	        restoreCenterLayout: (snapshot) => centerDock?.restoreLayout(snapshot),
   151	        setRegionWidth: (id, width, limits) => frame?.setRegionWidth(id, width, limits),
   152	        setRegionHeight: (id, height, limits) => frame?.setRegionHeight(id, height, limits),
   153	        setRegionLimits: (id, limits) => frame?.setRegionLimits(id, limits),
   154	        regionWidth: (id) => frame?.regionWidth(id) ?? null
   155	      });
   156	    } catch (error) {
   157	      onError?.(error instanceof Error ? error.message : String(error));
   158	    }
   159	
   160	    return () => {
   161	      observer?.disconnect();
   162	      centerDock?.dispose();
   163	      centerDock = null;
   164	      frame?.dispose();
   165	      frame = null;
   166	    };
   167	  });
   168	</script>
   169	
   170	<div class="shell-frame" class:ready bind:this={gridHost}></div>
   171	
   172	<!-- Parking stage: content lives here until (and unless) dockview claims it, and
   173	     whenever a panel hands it back. Not rendered at all, so nothing parked here
   174	     can be measured — see the note on `.parking-stage` in the styles below. -->
   175	<div class="parking-stage" aria-hidden="true">
   176	  <div class="slot" bind:this={sessionsSlot}>{@render sessions()}</div>
   177	  <div class="slot slot-center-dock" bind:this={centerSlot}></div>
   178	  <div class="slot" bind:this={toolsSlot}>{@render tools()}</div>
   179	  <div class="slot" bind:this={activitySlot}>{@render activity()}</div>
   180	  <div class="slot" bind:this={dockSlot}>{@render dock()}</div>
   181	  <div class="slot" bind:this={sessionSlot}>{@render center.session()}</div>
   182	  <div class="slot" bind:this={editorSlot}>{@render center.editor()}</div>
   183	  <div class="slot" bind:this={browserSlot}>{@render center.browser()}</div>
   184	  <div class="slot" bind:this={diffSlot}>{@render center.diff()}</div>
   185	  <div class="slot" bind:this={sessionLibrarySlot}>{@render center.sessionLibrary()}</div>
   186	  <div class="slot" bind:this={agentsSlot}>{@render center.agents()}</div>
   187	</div>
   188	
   189	<style>
   190	  .shell-frame {
nl: tauri-svelte-preview/src/lib/shell/centerDock.ts: No such file or directory
nl: tauri-svelte-preview/src/lib/conversation/ConversationSurface.svelte: No such file or directory
   978	    shellPanels.sessionPicked();
   979	    // Clicking the session you are already on changes nothing. Putting the
   980	    // stored record back here would throw away every file opened since the last
   981	    // switch, which is the opposite of what a click on your own row means.
   982	    if (switching) restoreWorkspace(ownedId);
   983	    const provider = conversationProviderFor(ownedId);
   984	    if (selected && provider) {
   985	      // A running PTY is already the owner of this agent session. Starting a
   986	      // second `claude --resume` / `codex app-server` here makes two agents
   987	      // append different answers to one conversation. Until the structured
   988	      // surface mirrors the PTY's transcript, keep the real terminal visible.
   989	      if (selected.ptySessionId) {
   990	        void closeStructuredConversation(ownedId);
   991	        setConversationMode(ownedId, 'raw');
   992	        return;
   993	      }
   994	      void ensureStructuredConversation({
   995	        ownedId,
   996	        provider,
   997	        cwd: selected.cwd,
   998	        nativeSessionId: selected.nativeSessionId
   999	      }).catch((error) => {
  1000	        rail.error = `could not open structured ${provider}: ${describeError(error)}`;
  1001	      });
  1002	    }
  1003	  }
  1004	
  1005	  function handoffInput(
  1080	    }
  1081	  }
  1082	
  1083	  async function returnToStructured(ownedId: string): Promise<void> {
  1084	    try {
  1085	      const input = handoffInput(ownedId, 'terminal-to-structured', 'same-session');
  1086	      await prepareConversationHandoff(input);
  1087	      const receipt = await commitConversationHandoff(input);
  1088	      if (receipt.owner === 'structured') setConversationMode(ownedId, 'structured');
  1089	    } catch (error) {
  1090	      rail.error = `structured handoff failed: ${describeError(error)}`;
  1091	    }
  1092	  }
  1093	
  1094	  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
  1094	  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
  1095	  async function adopt(record: AgentSession): Promise<void> {
  1095	  async function adopt(record: AgentSession): Promise<void> {
  1096	    if (!service || disposed) return;
  1097	    const owned = adoptAgentSession(record);
  1098	    addOwnedSession(owned);
  1099	    const host = await hostFor(owned.ownedId);
  1100	    if (!host) {
  1101	      rail.error = `no terminal host for "${owned.title}"`;
  1102	      return;
  1103	    }
  1104	    const ptySessionId = await service.startOwned(owned, host);
  1105	    if (!ptySessionId) {
  1106	      updateOwnedSession(owned.ownedId, { state: 'exited' });
  1107	      rail.error = `failed to start a terminal for "${owned.title}"`;
  1108	      return;
  1109	    }
  1110	    // Persist the PTY id: reload re-attach reads it back out of localStorage.
  1111	    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  1112	    await selectOwned(owned.ownedId);
  1113	    // A session you just started is a session you want to watch. Deliberately
  1114	    // here rather than inside `selectOwned`, which also runs on every plain
  1115	    // click on a card — a click on a row must not yank the reader off the file
  1116	    // they had open.
  1117	    frameControls?.showCenterPanel('session');
  1118	  }
  1119	
  1120	  /**
  1121	   * EXPLICIT IO: start a session the user described in the new-session dialog.
  1122	   *
  1123	   * Mirrors `adopt` step for step; the difference is where the description comes
  1124	   * from. `createFreshSession` makes a plain shell — agent 'other', nothing to
  1125	   * replay — and the dialog knows better on both counts, so BOTH are set before
  1126	   * `startOwned`: that is the call which types the command in.
  1127	   */
  1128	  async function startNewSession(request: NewSessionRequest): Promise<void> {
  1129	    if (!service || disposed) return;
  1130	    const owned = {
  1131	      ...createFreshSession({ cwd: request.cwd, title: request.title }),
  1132	      agent: request.agent,
  1133	      resumeCommand: request.command
  1134	    };
  1135	    addOwnedSession(owned);
  1136	    const host = await hostFor(owned.ownedId);
  1137	    if (!host) {
  1138	      rail.error = `no terminal host for "${owned.title}"`;
  1139	      return;
  1140	    }
  1141	    const ptySessionId = await service.startOwned(owned, host);
  1142	    if (!ptySessionId) {
  1143	      updateOwnedSession(owned.ownedId, { state: 'exited' });
  1144	      rail.error = `failed to start a terminal for "${owned.title}"`;
  1145	      return;
  1146	    }
  1147	    // Persist the PTY id: reload re-attach reads it back out of localStorage.
  1148	    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  1149	    await selectOwned(owned.ownedId);
  1150	    // Same as resuming one: you asked for this session, so it comes to the front.
  1151	    frameControls?.showCenterPanel('session');
  1152	  }

exec
/bin/zsh -lc "rg --files tauri-svelte-preview/src | rg 'centerDock|ConversationSurface|conversationStore|conversationService'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts
tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
tauri-svelte-preview/src/lib/shell/layout/centerDock.ts

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/layout/centerDock.ts | sed -n '160,195p;268,320p'; nl -ba tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte | sed -n '1,20p;56,75p;218,282p'; nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts | sed -n '30,45p;78,90p;548,562p;600,610p'; nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts | sed -n '350,398p;397,448p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
   160	   * The layout a fresh dock opens with: the conversation panels on the left, the
   161	   * display panels stacked as tabs in a second group to their right. Both sides
   162	   * may be empty — a roster that is all conversation just never opens the second
   163	   * group, and one that is all display opens a single group with no anchor to
   164	   * sit beside (asking dockview to position against a panel that does not exist
   165	   * yet throws, so that case adds the first panel with no position at all).
   166	   */
   167	  const buildDefault = (): void => {
   168	    const conversation = options.panels.filter((panel) => panel.group !== 'display');
   169	    const display = options.panels.filter((panel) => panel.group === 'display');
   170	    for (const panel of conversation) addPanelFor(panel);
   171	
   172	    const anchor = conversation[0];
   173	    const [leadDisplay, ...stackedDisplay] = display;
   174	    if (leadDisplay) {
   175	      addPanelFor(
   176	        leadDisplay,
   177	        anchor ? { referencePanel: anchor.id, direction: 'right' } : undefined
   178	      );
   179	      for (const panel of stackedDisplay) {
   180	        addPanelFor(panel, { referencePanel: leadDisplay.id, direction: 'within' });
   181	      }
   182	    }
   183	
   184	    // Every add above made its own panel the active one, so the last display tab
   185	    // is showing — which is now Diff, an empty pane until a file is picked. Put
   186	    // the display group back on its first tab, then hand focus to the front of
   187	    // the roster.
   188	    if (leadDisplay) panelById(leadDisplay.id)?.api.setActive();
   189	    const opening = anchor ?? leadDisplay;
   190	    if (opening) panelById(opening.id)?.api.setActive();
   191	  };
   192	
   193	  /**
   194	   * Run a programmatic layout mutation with persistence and the roster rebuild
   195	   * below both suppressed.
   268	
   269	  let migratedFromV4 = false;
   270	
   271	  runSynchronized(() => {
   272	    const stored = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY);
   273	    if (stored && panelSetMatches(dockPanelIds(stored), specs.keys())) {
   274	      try {
   275	        api.fromJSON(stored as never);
   276	        return;
   277	      } catch {
   278	        try {
   279	          api.clear();
   280	        } catch {
   281	          // fall through
   282	        }
   283	      }
   284	    }
   285	
   286	    /**
   287	     * v4 has the same arrangement as the current roster minus Agents. Restore
   288	     * that exact set first, then add the new tab beside Session Library through
   289	     * the live Dockview API. This preserves the serialized groups/sizes/order
   290	     * and avoids hand-editing Dockview's private grid tree. Any mismatch or
   291	     * Dockview rejection falls through to the safe current defaults.
   292	     */
   293	    const previous = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY_V4);
   294	    if (previous && panelSetMatches(dockPanelIds(previous), CENTER_PANEL_IDS_V4)) {
   295	      try {
   296	        api.fromJSON(previous as never);
   297	        const activePanelId = api.activePanel?.id ?? null;
   298	        const library = specs.get('session-library');
   299	        const agents = specs.get('agents');
   300	        if (!library || !agents || !panelById('session-library')) {
   301	          throw new Error('Agents migration roster is incomplete');
   302	        }
   303	        addPanelFor(agents, { referencePanel: 'session-library', direction: 'within' });
   304	        if (activePanelId) panelById(activePanelId)?.api.setActive();
   305	        migratedFromV4 = true;
   306	        return;
   307	      } catch {
   308	        try {
   309	          api.clear();
   310	        } catch {
   311	          // fall through
   312	        }
   313	      }
   314	    }
   315	
   316	    buildDefault();
   317	  });
   318	  normalizeSessionLibraryTitle();
   319	
   320	  const persistSoon = (): void => {
     1	<script lang="ts">
     2	  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
     3	  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
     4	  import TerminalSurface from './TerminalSurface.svelte';
     5	  import ConversationHeader from './conversation/ConversationHeader.svelte';
     6	  import ConversationTimeline from './conversation/ConversationTimeline.svelte';
     7	  import ConversationComposer from './conversation/ConversationComposer.svelte';
     8	  import ConversationAgentTree from './conversation/ConversationAgentTree.svelte';
     9	  import {
    10	    conversationSessions,
    11	    setConversationAttachments,
    12	    setConversationDraft,
    13	    setConversationMode,
    14	    setConversationScrollTop,
    15	    setConversationSelectedChild
    16	  } from '$lib/shell/conversation/conversationStore.svelte';
    17	  import {
    18	    cleanupConversationAttachment,
    19	    loadConversationCapabilities,
    20	    readChildConversationTranscript,
    56	    onOpenNativeCli,
    57	    onForkNativeCli,
    58	    onReturnToStructured
    59	  }: Props = $props();
    60	  const active = $derived(owned.find((item) => item.ownedId === activeOwnedId) ?? null);
    61	  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
    62	  const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode !== 'raw');
    63	  const selectedChild = $derived(conversation && conversation.selectedChildId
    64	    ? conversation.children.find((child) => child.childId === conversation.selectedChildId) ?? null
    65	    : null);
    66	  const legacyTimeline = $derived(conversation?.selectedChildId ? conversation.childTimeline : conversation?.timeline ?? []);
    67	  const visibleTimeline = $derived.by((): ConversationDisplayItem[] => {
    68	    if (!conversation) return [];
    69	    if (conversation.selectedChildId) return typedConversationTimeline([], legacyTimeline);
    70	    const items = typedConversationTimeline(conversation.agentItems, legacyTimeline);
    71	    const now = items.reduce((latest, item) => Math.max(latest, item.timestampMs), 0) + 1;
    72	    const typedKinds = new Set(items.map((item) => item.kind));
    73	    if (conversation.planSteps.length && !typedKinds.has('plan')) {
    74	      items.push({ kind: 'plan', itemId: 'plan:current', title: 'Plan', steps: conversation.planSteps, timestampMs: now });
    75	    }
   218	    await setConversationConfigOption(active.ownedId, optionId, value).catch(() => undefined);
   219	  }
   220	</script>
   221	
   222	<div class="conversation-shell" data-testid="conversation-shell">
   223	  <div class:covered={structured} class="terminal-layer"><TerminalSurface {owned} {activeOwnedId} {registerHost} {onHostLayout} /></div>
   224	  {#if structured && active && conversation}
   225	    <section class="structured" data-testid="structured-conversation" aria-label={`${active.agent} conversation`}>
   226	      <ConversationHeader {active} {conversation} {selectedChild} onModeChange={(mode) => setConversationMode(active.ownedId, mode)} />
   227	      <div class="handoff-actions" aria-label="Conversation handoff actions">
   228	        <button type="button" data-testid="open-native-cli" onclick={() => void onOpenNativeCli?.(active.ownedId)}>
   229	          Open in native CLI
   230	        </button>
   231	        {#if conversation.capabilities?.session.fork}
   232	          <button type="button" data-testid="fork-native-cli" onclick={() => void onForkNativeCli?.(active.ownedId)}>
   233	            Fork to native CLI
   234	          </button>
   235	        {/if}
   236	      </div>
   237	      <ConversationAgentTree children={conversation.children} selectedChildId={conversation.selectedChildId} onSelect={(childId) => void selectChild(childId)} />
   238	      <ConversationTimeline
   239	        items={visibleTimeline}
   240	        assistantLabel={selectedChild?.label ?? active.agent}
   241	        savedScrollTop={conversation.selectedChildId ? conversation.childScrollTopById[conversation.selectedChildId] ?? 0 : conversation.scrollTop}
   242	        emptyText={conversation.selectedChildId ? 'This sub-agent transcript is not available yet.' : 'Start the conversation below.'}
   243	        onScroll={(scrollTop) => {
   244	          if (conversation.selectedChildId) conversation.childScrollTopById[conversation.selectedChildId] = scrollTop;
   245	          else setConversationScrollTop(active.ownedId, scrollTop);
   246	        }}
   247	        onApprovalDecision={onApprovalDecision}
   248	        onInputSubmit={onInputSubmit}
   249	        onFileLink={openConversationFile}
   250	      />
   251	      {#if !conversation.selectedChildId}
   252	        <ConversationComposer
   253	          provider={active.agent}
   254	          draft={conversation.draft}
   255	          attachments={conversation.attachments}
   256	          sending={conversation.sending}
   257	          capabilities={conversation.capabilities}
   258	          config={conversation.config}
   259	          pendingConfig={conversation.pendingConfig}
   260	          configErrors={conversation.configErrors}
   261	          commands={matchingCommands}
   262	          {attachmentError}
   263	          onDraftChange={(value) => setConversationDraft(active.ownedId, value)}
   264	          onSend={send}
   265	          onPaste={paste}
   266	          onRemoveAttachment={removeAttachment}
   267	          onCommandSelected={selectCommand}
   268	          onConfigChange={(optionId, value) => void changeConfig(optionId, value)}
   269	        />
   270	      {:else}<div class="read-only-note" data-testid="conversation-read-only-note">Read-only sub-agent transcript</div>{/if}
   271	    </section>
   272	  {:else if active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode === 'raw'}
   273	    <div class="raw-actions" aria-label="Conversation handoff actions">
   274	      <button class="structured-toggle" data-testid="conversation-structured-toggle" type="button" onclick={() => void onReturnToStructured?.(active.ownedId)}>
   275	        Return to structured
   276	      </button>
   277	    </div>
   278	  {/if}
   279	</div>
   280	
   281	<style>.conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.handoff-actions{display:flex;gap:6px;align-items:center;padding:8px 12px;border-bottom:1px solid var(--color-border)}.handoff-actions button,.structured-toggle{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.handoff-actions button:hover,.structured-toggle:hover{background:var(--color-hover)}.raw-actions{position:absolute;right:12px;top:12px;z-index:2}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}</style>
    30	import type { AgentExecutionOwner } from '../ownedSessions.ts';
    31	import {
    32	  SESSION_CONVERSATION_WORKSPACE_VERSION,
    33	  type SessionConversationWorkspace
    34	} from '../sessionWorkspaces.ts';
    35	
    36	export type ConversationViewMode = 'structured' | 'raw';
    37	
    38	export interface ConversationWorkspaceState extends ConversationSessionState {
    39	  draft: string;
    40	  mode: ConversationViewMode;
    41	  sending: boolean;
    42	  attachments: ConversationAttachment[];
    43	  metadata: ConversationMetadata;
    44	  children: ConversationChildAgent[];
    45	  selectedChildId: string | null;
    78	function freshState(
    79	  ownedId: string,
    80	  provider: AgentConversationProvider
    81	): ConversationWorkspaceState {
    82	  return {
    83	    ...createConversationState(ownedId, provider),
    84	    draft: '',
    85	    mode: 'structured',
    86	    sending: false,
    87	    attachments: [],
    88	    metadata: emptyMetadata(),
    89	    children: [],
    90	    selectedChildId: null,
   548	export function setConversationDraft(ownedId: string, draft: string): void {
   549	  const current = conversationSessions[ownedId];
   550	  if (!current || current.draft === draft) return;
   551	  current.draft = draft;
   552	}
   553	
   554	export function setConversationMode(ownedId: string, mode: ConversationViewMode): void {
   555	  const current = conversationSessions[ownedId];
   556	  if (!current || current.mode === mode) return;
   557	  current.mode = mode;
   558	}
   559	
   560	export function setConversationSending(ownedId: string, sending: boolean): void {
   561	  const current = conversationSessions[ownedId];
   562	  if (!current || current.sending === sending) return;
   600	export function restoreConversationWorkspace(
   601	  ownedId: string,
   602	  provider: AgentConversationProvider,
   603	  snapshot: SessionConversationWorkspace | null | undefined
   604	): ConversationWorkspaceState {
   605	  const current = ensureConversationSession(ownedId, provider);
   606	  current.mode = snapshot?.mode === 'raw' ? 'raw' : 'structured';
   607	  current.draft = snapshot?.draft ?? '';
   608	  current.selectedChildId = snapshot?.selectedChildId ?? null;
   609	  current.scrollTop = snapshot?.parentScrollTop ?? snapshot?.scrollTop ?? 0;
   610	  current.childScrollTopById = snapshot?.childScrollTopById ?? {};
   350	    }
   351	  })().finally(() => resyncing.delete(ownedId));
   352	  resyncing.set(ownedId, work);
   353	  return work;
   354	}
   355	
   356	export async function startConversationEvents(): Promise<void> {
   357	  if (!isTauri() || unlisten) return;
   358	  unlisten = await listen<AgentConversationEvent | AgentEvent>('agent-conversation-event', ({ payload }) => {
   359	    applyAgentConversationEvent(payload);
   360	    if (getConversationSession(payload.ownedId)?.desynchronized) {
   361	      void resyncConversation(payload.ownedId);
   362	    }
   363	    if (('kind' in payload.payload && (payload.payload.kind === 'turn' || payload.payload.kind === 'error'))
   364	      || ('type' in payload && ['turn.completed', 'turn.interrupted', 'runtime.error'].includes(payload.type))) {
   365	      setConversationSending(payload.ownedId, false);
   366	    }
   367	  });
   368	}
   369	
   370	export function stopConversationEvents(): void {
   371	  unlisten?.();
   372	  unlisten = null;
   373	  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
   374	}
   375	
   376	export async function closeStructuredConversation(ownedId: string): Promise<void> {
   377	  if (!isTauri()) return;
   378	  await invoke<boolean>('close_agent_conversation', { ownedId });
   379	}
   380	
   381	export async function ensureStructuredConversation(input: {
   382	  ownedId: string;
   383	  provider: AgentConversationProvider;
   384	  cwd: string;
   385	  nativeSessionId?: string | null;
   386	}): Promise<AgentConversationConnection | null> {
   387	  ensureConversationSession(input.ownedId, input.provider);
   388	  if (!isTauri() || !input.cwd.trim()) return null;
   389	  const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
   390	    request: input
   391	  });
   392	  setConversationConnection(connection);
   393	  await resyncConversation(input.ownedId);
   394	  return connection;
   395	}
   396	
   397	export async function sendStructuredMessage(
   397	export async function sendStructuredMessage(
   398	  ownedId: string,
   398	  ownedId: string,
   399	  text: string,
   400	  ptySessionId?: string | null
   401	): Promise<void> {
   402	  const state = getConversationSession(ownedId);
   403	  if (!state) return;
   404	  if (!text.trim() && state.attachments.length === 0) return;
   405	  setConversationSending(ownedId, true);
   406	  try {
   407	    if (ptySessionId) {
   408	      if (state.writerLease.ownedId !== ownedId
   409	        || state.writerLease.generation !== state.generation
   410	        || state.writerLease.owner !== 'terminal') {
   411	        throw new Error('The terminal writer lease is not confirmed for this session');
   412	      }
   413	      const attachmentText = state.attachments.length
   414	        ? `\n\nAttached screenshots:\n${state.attachments.map((item) => `- ${item.path}`).join('\n')}`
   415	        : '';
   416	      const outgoingText = `${text}${attachmentText}`;
   417	      // Match a real terminal paste followed by a separate Enter key. Sending
   418	      // both in one PTY write makes Claude's TUI treat Enter as part of its
   419	      // multiline paste buffer, leaving the prompt staged but never submitted.
   420	      const pasted = await writeTerminalSessionFromTauri(
   421	        ptySessionId,
   422	        `\u001b[200~${outgoingText}\u001b[201~`
   423	      );
   424	      if (!pasted) throw new Error('The terminal session is no longer running');
   425	      const submitted = await writeTerminalSessionFromTauri(ptySessionId, '\r');
   426	      if (!submitted) throw new Error('The terminal session is no longer running');
   427	      setConversationSending(ownedId, false);
   428	      state.attachments.forEach(cleanupConversationAttachmentPreview);
   429	      setConversationAttachments(ownedId, []);
   430	      return;
   431	    }
   432	    if (state.generation < 1) throw new Error('The structured conversation is not connected');
   433	    const hydratedAttachments = state.capabilities?.prompt.image === true
   434	      ? await Promise.all(state.attachments.map((attachment) => hydrateAttachmentBytes(attachment as AttachmentWithBytes)))
   435	      : state.attachments as AttachmentWithBytes[];
   436	    const prompt = buildConversationPrompt(
   437	      text,
   438	      hydratedAttachments,
   439	      state.capabilities?.prompt.image === true
   440	    );
   441	    await invoke('send_agent_conversation_message', {
   442	      request: { ownedId, generation: state.generation, text: prompt.text, content: prompt.content }
   443	    });
   444	    state.attachments.forEach(cleanupConversationAttachmentPreview);
   445	    setConversationAttachments(ownedId, []);
   446	  } catch (error) {
   447	    setConversationSending(ownedId, false);
   448	    throw error;

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/routes/next/+page.svelte | sed -n '1558,1660p'; nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs | sed -n '1,125p'; nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs | sed -n '190,335p;498,540p'; nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs | sed -n '132,175p;349,372p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
  1558	  />
  1559	{/snippet}
  1560	{#snippet activityArea()}
  1561	  <ActivityBar
  1562	    activeId={activeView}
  1563	    onSelect={(id) => sidebarControls?.selectView(id)}
  1564	    onOpenSettings={() => overlays?.openSettings()}
  1565	  />
  1566	{/snippet}
  1567	{#snippet dockArea()}
  1568	  <DockPanel onReset={resetLayout} onProblemsLocationChange={applyProblemsLocation} />
  1569	{/snippet}
  1570	{#snippet sessionArea()}
  1571	  <!-- `onHostLayout` is what re-measures a terminal: a terminal is built inside
  1572	       a hidden host, where one character measures zero pixels wide, so the fit
  1573	       that runs when the panel is shown does nothing. The surface says when a
  1574	       host appears, changes size, or the terminal font lands. -->
  1575	  <ConversationSurface
  1576	    owned={rail.owned}
  1577	    activeOwnedId={rail.activeOwnedId}
  1578	    {registerHost}
  1579	    onHostLayout={scheduleRefit}
  1580	    onOpenNativeCli={openNativeCli}
  1581	    onForkNativeCli={forkNativeCli}
  1582	    onReturnToStructured={returnToStructured}
  1583	  />
  1584	{/snippet}
  1585	{#snippet editorArea()}
  1586	  <!-- Opening a file is a request to READ it: bring the editor forward, not load it out of sight.
  1587	       Except while a session's files are being put back — that is not a request for anything, and
  1588	       it must not drag the user off the terminal they were watching. -->
  1589	  <EditorPanel
  1590	    onFileOpened={() => {
  1591	      if (!restoringWorkspace) frameControls?.showCenterPanel('editor');
  1592	    }}
  1593	    onStartWorkspaceCommand={(request) =>
  1594	      onStartStack({
  1595	        stackId: request.id,
  1596	        cwd: request.cwd,
  1597	        script: request.script,
  1598	        title: request.title
  1599	      })}
  1600	  />
  1601	{/snippet}
  1602	{#snippet browserArea()}<BrowserPanel />{/snippet}
  1603	<!-- The changes to whichever file source control has selected. `GitDiffView`
  1604	     reads that selection itself and takes no props, so it can simply live here
  1605	     as a tab of its own — which is what gives a diff the width of the middle
  1606	     instead of a column. -->
  1607	{#snippet diffArea()}<GitDiffView />{/snippet}
  1608	{#snippet sessionLibraryArea()}
  1609	  <SessionLibraryWorkspace
  1610	    owned={rail.owned}
  1611	    available={rail.available}
  1612	    service={sessionLibraryService}
  1613	    workspacePath={rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.cwd ?? null}
  1614	    projectPath={rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.projectPath ?? rail.owned.find((session) => session.ownedId === rail.activeOwnedId)?.cwd ?? null}
  1615	    onOpenTab={() => frameControls?.showCenterPanel('session-library')}
  1616	    onRefresh={() => void scanRail()}
  1617	  />
  1618	{/snippet}
  1619	{#snippet agentsArea()}
  1620	  <WorkflowControlCenter
  1621	    onOpenConversation={(ownedId) => {
  1622	      void selectOwned(ownedId);
  1623	      frameControls?.showCenterPanel('session');
  1624	    }}
  1625	  />
  1626	{/snippet}
  1627	
  1628	<main class="next-shell">
  1629	  <!-- The strip along the top. It holds the play button that runs a saved
  1630	       configuration; anything else that belongs above the whole shell goes
  1631	       beside it, since it is a plain flex row. -->
  1632	  <div class="top-bar">
  1633	    <RunButton />
  1634	  </div>
  1635	
  1636	  <div class="frame-area">
  1637	    <ShellFrame
  1638	    sessions={sessionsArea} tools={toolsArea} activity={activityArea} dock={dockArea}
  1639	    center={{
  1640	      session: sessionArea,
  1641	      editor: editorArea,
  1642	      browser: browserArea,
  1643	      diff: diffArea,
  1644	      sessionLibrary: sessionLibraryArea,
  1645	      agents: agentsArea
  1646	    }}
  1647	    onSessionPanelLayout={scheduleRefit}
  1648	    onCenterPanelShown={(id) => {
  1649	      activeCenterPanelId = id;
  1650	      shellPanels.panelShown(id);
  1651	    }}
  1652	    onReady={(controls) => {
  1653	      frameControls = controls;
  1654	      // Say what the sessions column is, once, here, where the frame first
  1655	      // exists — in BOTH cases, not only the folded one.
  1656	      //
  1657	      // Two separate things remember the column: the stored grid layout, which
  1658	      // carries its width AND the limits it may be dragged between, and the
  1659	      // fold flag under its own key. They are written at different moments —
  1660	      // the flag straight away, the grid a quarter of a second later — so a
     1	mod attachments;
     2	pub mod capabilities;
     3	pub mod handoff;
     4	pub mod journal;
     5	pub mod manager;
     6	pub mod protocol;
     7	pub mod providers;
     8	pub mod terminal_projection;
     9	mod transcript;
    10	
    11	use manager::AgentRuntimeManager;
    12	use protocol::{
    13	    AgentConversationConnection, AgentConversationSnapshot, EnsureAgentConversationRequest,
    14	    RespondAgentConversationApprovalRequest, SendAgentConversationMessageRequest,
    15	    StopAgentConversationTurnRequest,
    16	};
    17	use providers::AgentPrompt;
    18	
    19	#[tauri::command]
    20	pub async fn ensure_agent_conversation(
    21	    manager: tauri::State<'_, AgentRuntimeManager>,
    22	    request: EnsureAgentConversationRequest,
    23	) -> Result<AgentConversationConnection, String> {
    24	    let connection = manager.ensure(request)?;
    25	    if manager.providers().manifest(connection.provider).is_ok() {
    26	        manager
    27	            .activate(&connection.owned_id, connection.generation)
    28	            .await
    29	    } else {
    30	        // A0 installs the packaged, hash-recorded adapters. Until then the
    31	        // existing connecting response remains available without a fallback PTY.
    32	        Ok(connection)
    33	    }
    34	}
    35	
    36	#[tauri::command]
    37	pub async fn send_agent_conversation_message(
    38	    manager: tauri::State<'_, AgentRuntimeManager>,
    39	    request: SendAgentConversationMessageRequest,
    40	) -> Result<(), String> {
    41	    let text = request.text.trim();
    42	    if text.is_empty() {
    43	        return Err("Message cannot be empty".to_string());
    44	    }
    45	    manager
    46	        .prompt(
    47	            &request.owned_id,
    48	            request.generation,
    49	            AgentPrompt {
    50	                text: text.to_string(),
    51	                images: Vec::new(),
    52	            },
    53	        )
    54	        .await
    55	}
    56	
    57	#[tauri::command]
    58	pub async fn respond_agent_conversation_approval(
    59	    manager: tauri::State<'_, AgentRuntimeManager>,
    60	    request: RespondAgentConversationApprovalRequest,
    61	) -> Result<(), String> {
    62	    let request_id = required_id(&request.request_id, "Approval request id")?;
    63	    manager
    64	        .respond_legacy_approval(
    65	            &request.owned_id,
    66	            request.generation,
    67	            request_id,
    68	            request.decision,
    69	        )
    70	        .await
    71	}
    72	
    73	#[tauri::command]
    74	pub async fn stop_agent_conversation_turn(
    75	    manager: tauri::State<'_, AgentRuntimeManager>,
    76	    request: StopAgentConversationTurnRequest,
    77	) -> Result<(), String> {
    78	    manager
    79	        .cancel_turn(&request.owned_id, request.generation)
    80	        .await
    81	}
    82	
    83	#[tauri::command]
    84	pub async fn close_agent_conversation(
    85	    manager: tauri::State<'_, AgentRuntimeManager>,
    86	    owned_id: String,
    87	) -> Result<bool, String> {
    88	    manager.close(&owned_id).await
    89	}
    90	
    91	#[tauri::command]
    92	pub async fn read_agent_conversation_snapshot(
    93	    manager: tauri::State<'_, AgentRuntimeManager>,
    94	    owned_id: String,
    95	) -> Result<Option<AgentConversationSnapshot>, String> {
    96	    manager.snapshot(&owned_id)
    97	}
    98	
    99	#[tauri::command]
   100	pub async fn read_agent_conversation_transcript(
   101	    provider: String,
   102	    native_session_id: String,
   103	    child_session_id: Option<String>,
   104	) -> Result<transcript::TranscriptSnapshot, String> {
   105	    transcript::read(&provider, &native_session_id, child_session_id.as_deref())
   106	}
   107	
   108	#[tauri::command]
   109	pub async fn start_agent_conversation_terminal_projection(
   110	    app: tauri::AppHandle,
   111	    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
   112	    request: terminal_projection::StartTerminalProjectionRequest,
   113	) -> Result<terminal_projection::TerminalProjectionRegistration, String> {
   114	    registry.start(app, request)
   115	}
   116	
   117	#[tauri::command]
   118	pub async fn stop_agent_conversation_terminal_projection(
   119	    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
   120	    owned_id: String,
   121	) -> Result<bool, String> {
   122	    registry.stop(&owned_id)
   123	}
   124	
   125	#[tauri::command]
   190	    pub async fn activate(
   191	        &self,
   192	        owned_id: &str,
   193	        generation: u64,
   194	    ) -> Result<AgentConversationConnection, String> {
   195	        let (provider, provider_instance_id, cwd, native_session_id) = {
   196	            let sessions = self
   197	                .sessions
   198	                .lock()
   199	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   200	            let session = current_session(&sessions, owned_id, generation)?;
   201	            (
   202	                session.provider,
   203	                session.provider_instance_id.clone(),
   204	                std::path::PathBuf::from(&session.cwd),
   205	                session.native_session_id.clone(),
   206	            )
   207	        };
   208	        let manifest = self.providers.manifest(provider)?;
   209	        let mut adapter = AcpRuntimeAdapter::new(manifest, owned_id.to_string(), cwd.clone());
   210	        let capabilities = adapter
   211	            .initialize(InitializeAgentInput {
   212	                provider,
   213	                provider_instance_id,
   214	            })
   215	            .await
   216	            .map_err(|error| error.to_string())?;
   217	        let expected_native_session_id = native_session_id.clone();
   218	        let started = match native_session_id {
   219	            Some(native_session_id) => {
   220	                adapter
   221	                    .resume_session(LoadAgentSession {
   222	                        cwd,
   223	                        native_session_id,
   224	                    })
   225	                    .await
   226	            }
   227	            None => adapter.new_session(NewAgentSession { cwd }).await,
   228	        }
   229	        .map_err(|error| error.to_string())?;
   230	        if let Some(expected_native_session_id) = expected_native_session_id {
   231	            if started.native_session_id != expected_native_session_id {
   232	                let _ = adapter.detach_session().await;
   233	                return Err(
   234	                    "The provider resumed a different native session; handoff was rejected"
   235	                        .to_string(),
   236	                );
   237	            }
   238	        }
   239	        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
   240	        let mut sessions = self
   241	            .sessions
   242	            .lock()
   243	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   244	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   245	        session.capabilities = capabilities;
   246	        session.native_session_id = Some(started.native_session_id.clone());
   247	        session.connection.native_session_id = Some(started.native_session_id);
   248	        session.connection.state = ConversationConnectionState::Connected;
   249	        let restoring_terminal_transition = session.owner
   250	            == AgentExecutionOwner::TransitioningToStructured
   251	            && session
   252	                .writer_lease_transition
   253	                .as_ref()
   254	                .map(|transition| transition.to == AgentWriterLeaseOwner::Structured)
   255	                .unwrap_or(false);
   256	        if !restoring_terminal_transition {
   257	            session.owner = AgentExecutionOwner::Structured;
   258	            session.writer_lease.owner = AgentWriterLeaseOwner::Structured;
   259	        }
   260	        session.state = AgentRuntimeState::Ready;
   261	        session.runtime = Some(runtime);
   262	        Ok(session.connection.clone())
   263	    }
   264	
   265	    pub fn emit_payload(
   266	        &self,
   267	        owned_id: &str,
   268	        generation: u64,
   269	        payload: AgentConversationPayload,
   270	    ) -> Result<AgentConversationEvent, String> {
   271	        let mut sessions = self
   272	            .sessions
   273	            .lock()
   274	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   275	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   276	        let sequence = session.next_sequence;
   277	        session.next_sequence = session.next_sequence.saturating_add(1);
   278	        if let AgentConversationPayload::Connection {
   279	            state,
   280	            native_session_id,
   281	        } = &payload
   282	        {
   283	            session.connection.state = *state;
   284	            if native_session_id.is_some() {
   285	                session.connection.native_session_id = native_session_id.clone();
   286	                session.native_session_id = native_session_id.clone();
   287	            }
   288	        }
   289	        let timestamp_ms = timestamp_millis();
   290	        let frontend_event = AgentConversationEvent {
   291	            owned_id: owned_id.to_string(),
   292	            provider: session.provider,
   293	            generation,
   294	            sequence,
   295	            timestamp_ms,
   296	            payload: payload.clone(),
   297	        };
   298	        let canonical = canonical_event(session, sequence, timestamp_ms, &payload)?;
   299	        session.journal.append(canonical.clone())?;
   300	        session.recent_events.push_back(canonical);
   301	        session.frontend_events.push_back(frontend_event.clone());
   302	        while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
   303	            session.recent_events.pop_front();
   304	        }
   305	        while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
   306	            session.frontend_events.pop_front();
   307	        }
   308	        Ok(frontend_event)
   309	    }
   310	
   311	    pub async fn prompt(
   312	        &self,
   313	        owned_id: &str,
   314	        generation: u64,
   315	        input: AgentPrompt,
   316	    ) -> Result<(), String> {
   317	        let runtime = self.runtime(owned_id, generation)?;
   318	        let started = runtime
   319	            .lock()
   320	            .await
   321	            .prompt(input)
   322	            .await
   323	            .map_err(|error| error.to_string())?;
   324	        let mut sessions = self
   325	            .sessions
   326	            .lock()
   327	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   328	        let session = current_session_mut(&mut sessions, owned_id, generation)?;
   329	        session.active_turn_id = started.turn_id;
   330	        session.state = AgentRuntimeState::Working;
   331	        Ok(())
   332	    }
   333	
   334	    /// Run one product action through the current structured agent without
   335	    /// writing through the conversation UI. The runtime and generation checks
   498	    pub fn snapshot(&self, owned_id: &str) -> Result<Option<AgentConversationSnapshot>, String> {
   499	        let sessions = self
   500	            .sessions
   501	            .lock()
   502	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   503	        Ok(sessions
   504	            .get(owned_id)
   505	            .map(|session| AgentConversationSnapshot {
   506	                connection: session.connection.clone(),
   507	                last_sequence: session.next_sequence.saturating_sub(1),
   508	                events: session.frontend_events.iter().cloned().collect(),
   509	            }))
   510	    }
   511	
   512	    pub async fn close(&self, owned_id: &str) -> Result<bool, String> {
   513	        let runtime = {
   514	            let mut sessions = self
   515	                .sessions
   516	                .lock()
   517	                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
   518	            let Some(session) = sessions.get_mut(owned_id) else {
   519	                return Ok(false);
   520	            };
   521	            session.state = AgentRuntimeState::Closed;
   522	            session.owner = AgentExecutionOwner::Stopped;
   523	            session.writer_lease.owner = AgentWriterLeaseOwner::None;
   524	            session.runtime.take()
   525	        };
   526	        if let Some(runtime) = runtime {
   527	            runtime
   528	                .lock()
   529	                .await
   530	                .close_session()
   531	                .await
   532	                .map_err(|error| error.to_string())?;
   533	        }
   534	        self.sessions
   535	            .lock()
   536	            .map_err(|_| "Agent runtime manager is unavailable".to_string())?
   537	            .remove(owned_id);
   538	        Ok(true)
   539	    }
   540	
   132	    pub async fn prompt(&mut self, prompt: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
   133	        let session_id = self.session_id()?;
   134	        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
   135	        blocks.extend(prompt.images.into_iter().map(|image| {
   136	            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
   137	        }));
   138	        let result = self
   139	            .request(
   140	                "session/prompt",
   141	                &acp::PromptRequest::new(session_id, blocks),
   142	            )
   143	            .await?;
   144	        Ok(StartedTurn {
   145	            turn_id: result
   146	                .get("turnId")
   147	                .and_then(Value::as_str)
   148	                .map(str::to_string),
   149	        })
   150	    }
   151	
   152	    /// Send a bounded one-shot prompt and retain only assistant text updates.
   153	    ///
   154	    /// The regular `prompt` call intentionally returns as soon as ACP accepts
   155	    /// the request; the conversation event stream owns the rest of that turn.
   156	    /// Git actions need a direct result, so this narrow path consumes the same
   157	    /// ACP transport until the prompt response arrives and ignores tool,
   158	    /// reasoning, and lifecycle updates.
   159	    pub async fn prompt_once(
   160	        &mut self,
   161	        prompt: AgentPrompt,
   162	    ) -> Result<GeneratedText, AgentRuntimeError> {
   163	        let session_id = self.session_id()?;
   164	        self.next_request_id = self.next_request_id.saturating_add(1);
   165	        let request_id = self.next_request_id;
   166	        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
   167	        blocks.extend(prompt.images.into_iter().map(|image| {
   168	            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
   169	        }));
   170	        let params = acp::PromptRequest::new(session_id, blocks);
   171	        let params = serde_json::to_value(params).map_err(serialization_error)?;
   172	        self.process
   173	            .write_json(&json!({
   174	                "jsonrpc": "2.0",
   175	                "id": request_id,
   349	    async fn request<T: Serialize>(
   350	        &mut self,
   351	        method: &str,
   352	        params: &T,
   353	    ) -> Result<Value, AgentRuntimeError> {
   354	        self.next_request_id = self.next_request_id.saturating_add(1);
   355	        let id = self.next_request_id;
   356	        let params = serde_json::to_value(params).map_err(serialization_error)?;
   357	        self.process
   358	            .write_json(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
   359	            .await
   360	            .map_err(transport_error)?;
   361	        loop {
   362	            let frame = self.process.next_json().await.map_err(transport_error)?;
   363	            if frame.get("id") != Some(&json!(id)) {
   364	                continue;
   365	            }
   366	            if let Some(error) = frame.get("error") {
   367	                return Err(AgentRuntimeError::new("acp-error", error.to_string()));
   368	            }
   369	            return Ok(frame.get("result").cloned().unwrap_or(Value::Null));
   370	        }
   371	    }
   372	

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs | sed -n '1,20p;70,165p;190,205p;318,370p'; nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs | sed -n '35,90p;140,175p'; nl -ba tauri-svelte-preview/src-tauri/src/main.rs | sed -n '5315,5330p;5438,5452p'; nl -ba docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md | sed -n '38,60p;99,128p;255,325p;1824,1848p;2777,2788p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	use serde::{Deserialize, Serialize};
     2	use std::collections::{HashMap, HashSet, VecDeque};
     3	use std::path::PathBuf;
     4	use std::sync::mpsc::{self, Sender};
     5	use std::sync::{Arc, Mutex};
     6	use std::thread::{self, JoinHandle};
     7	use std::time::{Duration, SystemTime, UNIX_EPOCH};
     8	use tauri::{Emitter, Runtime};
     9	
    10	use super::protocol::{AgentConversationProvider, AgentEvent, AgentRawFrameReference};
    11	use super::transcript::{self, FileIdentity, ProjectedRecord, TranscriptLocation};
    12	
    13	pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
    14	const WATCH_INTERVAL: Duration = Duration::from_millis(250);
    15	const MAX_INCREMENTAL_BYTES: u64 = 1024 * 1024;
    16	const RECONCILE_EVERY_POLLS: u32 = 40;
    17	const MAX_RECONCILIATION_EVENTS: usize = 512;
    18	const MAX_SEEN_RECORDS: usize = 4096;
    19	const MAX_RECENT_EVENTS: usize = 2000;
    20	
    70	            cursor: ProjectionCursor::default(),
    71	        }
    72	    }
    73	
    74	    fn poll(&mut self) -> Result<Vec<ProjectedRecord>, String> {
    75	        let cached = self
    76	            .cursor
    77	            .canonical_path
    78	            .as_deref()
    79	            .map(transcript::inspect)
    80	            .transpose()?
    81	            .flatten();
    82	        let Some(location) = cached.or(transcript::discover(
    83	            self.provider,
    84	            &self.native_session_id,
    85	        )?) else {
    86	            return Ok(Vec::new());
    87	        };
    88	        self.poll_location(location)
    89	    }
    90	
    91	    fn poll_location(
    92	        &mut self,
    93	        location: TranscriptLocation,
    94	    ) -> Result<Vec<ProjectedRecord>, String> {
    95	        let reason = match (&self.cursor.identity, &self.cursor.canonical_path) {
    96	            (None, _) => Some(ReconcileReason::Initial),
    97	            (Some(identity), Some(path))
    98	                if identity == &location.identity && path != &location.canonical_path =>
    99	            {
   100	                Some(ReconcileReason::Archived)
   101	            }
   102	            (Some(identity), _) if identity != &location.identity => Some(ReconcileReason::Rotated),
   103	            (Some(_), _) if location.len < self.cursor.offset => Some(ReconcileReason::Truncated),
   104	            (Some(_), _)
   105	                if location.len.saturating_sub(self.cursor.offset) > MAX_INCREMENTAL_BYTES =>
   106	            {
   107	                Some(ReconcileReason::Gap)
   108	            }
   109	            _ if self.cursor.polls_since_reconcile >= RECONCILE_EVERY_POLLS => {
   110	                Some(ReconcileReason::Periodic)
   111	            }
   112	            _ => None,
   113	        };
   114	        if let Some(reason) = reason {
   115	            return self.reconcile(location, reason);
   116	        }
   117	        self.cursor.polls_since_reconcile = self.cursor.polls_since_reconcile.saturating_add(1);
   118	        if location.len == self.cursor.offset {
   119	            return Ok(Vec::new());
   120	        }
   121	        let bytes = transcript::read_range(
   122	            &location.canonical_path,
   123	            self.cursor.offset,
   124	            location.len.saturating_sub(self.cursor.offset),
   125	        )?;
   126	        self.cursor.offset = location.len;
   127	        self.cursor.canonical_path = Some(location.canonical_path);
   128	        self.cursor.identity = Some(location.identity);
   129	        let mut combined = std::mem::take(&mut self.cursor.partial_line);
   130	        combined.extend_from_slice(&bytes);
   131	        let (lines, partial) = transcript::complete_lines(&combined, false);
   132	        let records = self.project_lines(lines);
   133	        self.cursor.partial_line = partial;
   134	        Ok(records)
   135	    }
   136	
   137	    fn reconcile(
   138	        &mut self,
   139	        location: TranscriptLocation,
   140	        reason: ReconcileReason,
   141	    ) -> Result<Vec<ProjectedRecord>, String> {
   142	        let (start, bytes) = transcript::read_bounded(&location.canonical_path, location.len)?;
   143	        let (lines, partial) = transcript::complete_lines(&bytes, start > 0);
   144	        let mut records = self.project_lines(lines);
   145	        if records.len() > MAX_RECONCILIATION_EVENTS {
   146	            records = records.split_off(records.len() - MAX_RECONCILIATION_EVENTS);
   147	        }
   148	        self.cursor.canonical_path = Some(location.canonical_path);
   149	        self.cursor.identity = Some(location.identity);
   150	        self.cursor.offset = location.len;
   151	        self.cursor.partial_line = partial;
   152	        self.cursor.polls_since_reconcile = 0;
   153	        self.cursor.last_reconcile = Some(reason);
   154	        Ok(records)
   155	    }
   156	
   157	    fn project_lines(&mut self, lines: Vec<&[u8]>) -> Vec<ProjectedRecord> {
   158	        let mut projected = Vec::new();
   159	        for line in lines {
   160	            for record in
   161	                transcript::parse_durable_line(self.provider, &self.native_session_id, line)
   162	            {
   163	                if self.cursor.seen.insert(record.key.clone()) {
   164	                    self.cursor.seen_order.push_back(record.key.clone());
   165	                    projected.push(record);
   190	    }
   191	}
   192	
   193	#[derive(Default)]
   194	pub struct TerminalProjectionRegistry {
   195	    watchers: Mutex<HashMap<String, WatcherHandle>>,
   196	}
   197	
   198	impl TerminalProjectionRegistry {
   199	    pub fn start<R: Runtime>(
   200	        &self,
   201	        app: tauri::AppHandle<R>,
   202	        request: StartTerminalProjectionRequest,
   203	    ) -> Result<TerminalProjectionRegistration, String> {
   204	        let owned_id = required_id(&request.owned_id, "Owned session id")?;
   205	        let native_session_id = transcript::safe_session_id(&request.native_session_id)?;
   318	fn canonical_event(
   319	    owned_id: &str,
   320	    provider: AgentConversationProvider,
   321	    native_session_id: &str,
   322	    generation: u64,
   323	    sequence: u64,
   324	    record: ProjectedRecord,
   325	) -> AgentEvent {
   326	    AgentEvent {
   327	        event_type: record.event_type,
   328	        owned_id: owned_id.to_string(),
   329	        provider,
   330	        provider_instance_id: format!("terminal-transcript:{native_session_id}"),
   331	        generation,
   332	        sequence,
   333	        timestamp_ms: if record.timestamp_ms == 0 {
   334	            now_millis()
   335	        } else {
   336	            record.timestamp_ms
   337	        },
   338	        native_session_id: Some(native_session_id.to_string()),
   339	        turn_id: None,
   340	        item_id: record.item_id.clone(),
   341	        request_id: None,
   342	        payload: record.payload,
   343	        provider_metadata: Some(std::collections::BTreeMap::from([
   344	            (
   345	                "source".into(),
   346	                serde_json::Value::String("terminal-transcript".into()),
   347	            ),
   348	            ("historical".into(), serde_json::Value::Bool(true)),
   349	        ])),
   350	        raw_frame_reference: Some(AgentRawFrameReference {
   351	            id: record.key,
   352	            redacted: true,
   353	        }),
   354	    }
   355	}
   356	
   357	fn remember_and_emit<R: Runtime>(
   358	    app: &tauri::AppHandle<R>,
   359	    events: &Arc<Mutex<VecDeque<AgentEvent>>>,
   360	    event: AgentEvent,
   361	) {
   362	    if let Ok(mut recent) = events.lock() {
   363	        recent.push_back(event.clone());
   364	        while recent.len() > MAX_RECENT_EVENTS {
   365	            recent.pop_front();
   366	        }
   367	    }
   368	    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);
   369	}
   370	
    35	    pub fn process_id(&self) -> Option<u32> {
    36	        self.client.as_ref().and_then(AcpClient::process_id)
    37	    }
    38	}
    39	
    40	impl AgentRuntimeAdapter for AcpRuntimeAdapter {
    41	    async fn initialize(
    42	        &mut self,
    43	        input: InitializeAgentInput,
    44	    ) -> Result<AgentCapabilities, AgentRuntimeError> {
    45	        let mut client = AcpClient::spawn(&self.manifest, &self.cwd, &self.owned_id)?;
    46	        let capabilities = client.initialize(input.provider).await?;
    47	        validate_capabilities(&capabilities)
    48	            .map_err(|message| AgentRuntimeError::new("invalid-capabilities", message))?;
    49	        self.client = Some(client);
    50	        self.capabilities = Some(capabilities.clone());
    51	        Ok(capabilities)
    52	    }
    53	
    54	    async fn new_session(
    55	        &mut self,
    56	        input: NewAgentSession,
    57	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    58	        self.client_mut()?.new_session(&input.cwd).await
    59	    }
    60	    async fn load_session(
    61	        &mut self,
    62	        input: LoadAgentSession,
    63	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    64	        self.client_mut()?
    65	            .load_session(&input.cwd, &input.native_session_id)
    66	            .await
    67	    }
    68	    async fn resume_session(
    69	        &mut self,
    70	        input: ResumeAgentSession,
    71	    ) -> Result<StartedAgentSession, AgentRuntimeError> {
    72	        self.client_mut()?
    73	            .resume_session(&input.cwd, &input.native_session_id)
    74	            .await
    75	    }
    76	    async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
    77	        self.client_mut()?.prompt(input).await
    78	    }
    79	
    80	    async fn prompt_once(
    81	        &mut self,
    82	        input: AgentPrompt,
    83	    ) -> Result<GeneratedText, AgentRuntimeError> {
    84	        self.client_mut()?.prompt_once(input).await
    85	    }
    86	    async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
    87	        self.client_mut()?.steer(input.text).await
    88	    }
    89	    async fn cancel_turn(&mut self, _turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
    90	        self.client_mut()?.cancel().await
   140	        }
   141	        self.client = None;
   142	        Ok(())
   143	    }
   144	}
   145	
   146	pub enum StructuredRuntimeHandle {
   147	    Acp(AcpRuntimeAdapter),
   148	}
   149	
   150	impl StructuredRuntimeHandle {
   151	    /// Return the OS process that backs this structured provider session.
   152	    /// Resources uses this as the root of the app-owned process tree; it is
   153	    /// intentionally read-only and never exposes the provider transport.
   154	    pub fn process_id(&self) -> Option<u32> {
   155	        match self {
   156	            Self::Acp(adapter) => adapter.process_id(),
   157	        }
   158	    }
   159	
   160	    pub async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
   161	        match self {
   162	            Self::Acp(adapter) => adapter.prompt(input).await,
   163	        }
   164	    }
   165	    pub async fn prompt_once(
   166	        &mut self,
   167	        input: AgentPrompt,
   168	    ) -> Result<GeneratedText, AgentRuntimeError> {
   169	        match self {
   170	            Self::Acp(adapter) => adapter.prompt_once(input).await,
   171	        }
   172	    }
   173	    pub async fn cancel_turn(&mut self, turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
   174	        match self {
   175	            Self::Acp(adapter) => adapter.cancel_turn(turn_id).await,
  5315	}
  5316	
  5317	fn main() {
  5318	    let agent_runtime = agent_conversation::manager::AgentRuntimeManager::new(
  5319	        agent_conversation::providers::ProviderRegistry::bundled_from_environment()
  5320	            .expect("packaged ACP adapter configuration is invalid"),
  5321	    );
  5322	    let workflow_engine = WorkflowEngine::managed(agent_runtime.clone());
  5323	    tauri::Builder::default()
  5324	        .manage(SourceScanRegistry::default())
  5325	        .manage(agent_runtime)
  5326	        .manage(workflow_engine)
  5327	        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
  5328	        .manage(lsp::SourceLspRegistry::default())
  5329	        .manage(terminal::TerminalRegistry::default())
  5330	        .manage(browser::BrowserRegistry::default())
  5438	            approve_workflow_gate,
  5439	            submit_workflow_result,
  5440	            agent_conversation::ensure_agent_conversation,
  5441	            agent_conversation::send_agent_conversation_message,
  5442	            agent_conversation::respond_agent_conversation_approval,
  5443	            agent_conversation::stop_agent_conversation_turn,
  5444	            agent_conversation::close_agent_conversation,
  5445	            agent_conversation::read_agent_conversation_snapshot,
  5446	            agent_conversation::read_agent_conversation_transcript,
  5447	            agent_conversation::start_agent_conversation_terminal_projection,
  5448	            agent_conversation::stop_agent_conversation_terminal_projection,
  5449	            agent_conversation::save_agent_conversation_attachment,
  5450	            agent_conversation::handoff::handoff_agent_conversation,
  5451	            start_terminal_session,
  5452	            list_terminal_sessions,
    38	### 1.1 Does an ACP session create a terminal session?
    39	
    40	No. An ACP session is a structured JSON-RPC connection between Assembly, acting as the ACP client, and an agent process or adapter, acting as the ACP agent. The client normally starts the adapter as a child process with piped stdin/stdout. That process is not an xterm instance and is not the native Claude Code or Codex TUI.
    41	
    42	An ACP agent can report command/tool activity and may ask the client to provide terminal execution facilities. Those are **tool terminals** owned by Assembly and associated with a tool call. They may be shown as expandable command output or optionally opened in a terminal pane. They are not the same thing as the user-facing raw `claude` or `codex` TUI.
    43	
    44	Assembly therefore keeps three distinct concepts:
    45	
    46	```text
    47	Structured agent session
    48	  ACP JSON-RPC session; default rich conversation experience.
    49	
    50	Tool terminal
    51	  Client-managed process/output used by a command tool call inside an ACP turn.
    52	
    53	Native CLI terminal
    54	  Existing interactive PTY running the real Claude Code or Codex TUI.
    55	```
    56	
    57	A normal structured session creates the first and may create the second. It creates the third only when the user explicitly chooses **Open in native CLI**, when a provider capability is unavailable, or when recovery requires it.
    58	
    59	### 1.2 Can ACP provide the requested Codex/T3-style controls and content?
    60	
    99	The current active conversation path is nevertheless a terminal transcript projection:
   100	
   101	```text
   102	bare interactive `claude` or `codex` in a PTY
   103	  -> provider JSONL transcript
   104	  -> 500 ms full-tail polling
   105	  -> flattened user/assistant messages
   106	  -> structured overlay
   107	  -> composer writes bracketed paste + Return back to the PTY
   108	```
   109	
   110	The repository also contains an intentionally unwired handwritten `codex app-server`/`claude -p stream-json` provider spike. It must not be independently activated beside the PTY. Its useful protocol discoveries can be preserved as tests or removed after the ACP runtime replaces it.
   111	
   112	The current projection cannot be the final rich client because it discards most typed provider behavior and cannot authoritatively control an active turn. The target architecture replaces that limitation without discarding the terminal system.
   113	
   114	---
   115	
   116	## 3. Locked product decisions
   117	
   118	1. **New Codex and Claude sessions start structured by default through ACP.**
   119	2. **No permanent user-visible terminal is created for an ordinary ACP session.** Tool terminals are separate and scoped to tool calls.
   120	3. **The native CLI remains first class.** The user can hand a session to the raw Codex/Claude TUI and later return it to the structured view.
   121	4. **One writer per native provider conversation.** A structured ACP runtime and a native CLI never submit to the same provider session concurrently.
   122	5. **Existing externally created or already-running CLI sessions remain usable.** Assembly imports/mirrors their transcripts and may perform an explicit terminal-to-structured handoff when the provider can resume them safely.
   123	6. **`ownedId` remains the primary application identity.** ACP session IDs, Codex thread IDs, Claude session IDs, PTY IDs, tool-terminal IDs, workflow-run IDs, and parent/child IDs never replace it.
   124	7. **Model, effort/thought level, mode/permissions, Fast/service tier, and similar settings are UI controls—not slash commands.** They come from runtime-advertised session configuration options.
   125	8. **The slash menu contains only genuine provider commands and Assembly actions.** TUI-only commands such as theme, raw terminal rendering, terminal title, pet, keymap, or app handoff are excluded unless Assembly implements an equivalent local action.
   126	9. **Screenshot paste is a top-priority first-class input path.** Clipboard images appear immediately as previews and are sent as ACP image content blocks when supported.
   127	10. **Tool calls, approvals, plans, task lists, reasoning, file changes, and terminals are typed timeline items.** They are not flattened into assistant Markdown.
   128	11. **Subagents have two provenances:** provider-native subagents observed through ACP, and Assembly-managed workflow agents created by the workflow engine. The UI identifies which is which.
   255	Migration rules:
   256	
   257	- an existing live agent PTY becomes `executionOwner: 'terminal'`;
   258	- an exited agent session becomes `stopped`;
   259	- a new structured session becomes `structured` only after ACP initialization and `session/new` succeed;
   260	- failed migration never erases `ptySessionId`, `nativeSessionId`, transcript, draft, workspace, or attachment state.
   261	
   262	### 5.2 One Rust `AgentRuntimeManager`
   263	
   264	Refactor the existing `src-tauri/src/agent_conversation` authority rather than adding a competing runtime.
   265	
   266	Target module shape after re-anchor:
   267	
   268	```text
   269	src-tauri/src/agent_conversation/
   270	├── mod.rs
   271	├── manager.rs
   272	├── protocol.rs
   273	├── capabilities.rs
   274	├── journal.rs
   275	├── handoff.rs
   276	├── attachments.rs
   277	├── terminal_projection.rs
   278	├── transcript/
   279	│   ├── mod.rs
   280	│   ├── claude.rs
   281	│   └── codex.rs
   282	└── providers/
   283	    ├── mod.rs
   284	    ├── acp.rs
   285	    ├── acp_client.rs
   286	    ├── process.rs
   287	    └── direct_codex.rs        # optional later optimization only
   288	```
   289	
   290	Core state:
   291	
   292	```rust
   293	pub struct AgentRuntimeManager {
   294	    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
   295	    providers: Arc<ProviderRegistry>,
   296	}
   297	
   298	pub struct ManagedAgentSession {
   299	    pub owned_id: String,
   300	    pub provider: AgentProvider,
   301	    pub provider_instance_id: String,
   302	    pub native_session_id: Option<String>,
   303	    pub generation: u64,
   304	    pub owner: ExecutionOwner,
   305	    pub state: AgentRuntimeState,
   306	    pub capabilities: AgentCapabilities,
   307	    pub next_sequence: u64,
   308	    pub active_turn_id: Option<String>,
   309	    pub runtime: Option<StructuredRuntimeHandle>,
   310	    pub terminal_projection: Option<TerminalProjectionHandle>,
   311	    pub recent_events: VecDeque<AgentEvent>,
   312	}
   313	```
   314	
   315	The manager owns:
   316	
   317	- provider sidecar lifecycle and stderr diagnostics;
   318	- ACP initialize/new/load/resume/close;
   319	- prompt, image, cancellation, steering, permission, and user-input responses;
   320	- session config updates;
   321	- normalized event emission;
   322	- provider-native IDs;
   323	- structured/terminal writer lease;
   324	- recovery and handoff;
   325	- bounded recent-event snapshots and rebuildable app journal.
  1824	### Work Package A2 — Rust `AgentRuntimeManager` and ACP transport
  1825	
  1826	**Route:** SOL-medium after R2. Refactor, do not coexist with, the current
  1827	`AgentConversationRegistry`, `ConversationSession`, `ProviderCommand`, `ensure`, `emit_payload`,
  1828	`send_command`, `snapshot`, and `close` in `agent_conversation/mod.rs`. `TerminalRegistry` remains
  1829	the sole user-PTY authority.
  1830	
  1831	Own new `agent_conversation/manager.rs` (`AgentRuntimeManager`, `ManagedAgentSession`, writer lease,
  1832	generation/sequence, bounded event snapshot), `capabilities.rs` (manifest/capability/config
  1833	validation), `journal.rs` (bounded append/rebuild/repair), `providers/mod.rs` (`ProviderRegistry`),
  1834	`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
  1835	image/config/approval/input/cancel/close), and `providers/process.rs` (piped sidecar supervision,
  1836	bounded stderr, process-group cleanup). Tool terminals carry `TerminalKind` plus
  1837	`ToolTerminalIdentity` through the existing `TerminalRegistry`; they never set
  1838	`OwnedSession.ptySessionId` or appear as user PTYs.
  1839	
  1840	Implement one manager per app, one current-generation lease per `ownedId`, full-state capability
  1841	replacement, canonical events, bounded snapshots/journal, stale-response rejection, close and
  1842	app-exit cleanup. Return controller receipts for `mod.rs`, `main.rs`, capabilities and manifests;
  1843	do not edit frontend, provider-specific mapping, terminal projection, handoff, Browser, or shared
  1844	A1 types.
  1845	
  1846	Run `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation` in one
  1847	heavy slot. Tests cover initialization/new/load/resume, prompt/image, correlated approval/input,
  1848	config replacement, cancel/close, sequence repair, generation rejection, journal recovery, process
  2777	### 19.7 Native end-to-end acceptance
  2778	
  2779	One rebuilt-Tauri proof must show:
  2780	
  2781	1. Create a new Codex structured session. No user PTY exists. Paste a screenshot, select model/effort/permissions, observe reasoning/tool/file/plan/task items, and open a child agent.
  2782	2. Repeat with Claude using its advertised options and a nested child transcript.
  2783	3. Open Codex in native CLI, continue the same conversation, return to structured mode, and prove one writer and exact history reconciliation.
  2784	4. Start a workflow with Orchestrator -> two Implementers in separate worktrees -> Test -> Code Review -> Spec Compliance -> Fix -> final review. See every node/agent, current loop phase, tool output, worktree, finding, gate, retry, and completion.
  2785	5. Cancel one child and retry one failed node without disturbing siblings.
  2786	6. Pin one workflow agent to Working; verify the rest remain in the workflow tree and Session Library.
  2787	7. Open the Browser from the bottom action island in floating mode, maximize it, grab an element, annotate another, draw on a screenshot, and stage all three into the exact active conversation without auto-send.
  2788	8. Open Resource Manager compact view, Resources full page, Workspace Space treemap, Usage compact view, and Usage analytics. Values share one underlying state and actions target exact owners.

exec
/bin/zsh -lc "nl -ba docs/superpowers/plans/2026-08-01-structured-agent-conversation.md | sed -n '29,40p;316,326p;420,432p;454,466p;662,671p'; nl -ba docs/conversation-workbench-rust-guide.md | sed -n '1,18p'; git log --oneline --follow -- tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte | head -12; git show --stat --oneline ba46633; git show ba46633"'^:tauri-svelte-preview/src/routes/next/+page.svelte | nl -ba | sed -n '"'976,1000p'; git show ba46633:tauri-svelte-preview/src/routes/next/+page.svelte | nl -ba | sed -n '976,1000p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 1ms:
   976	    // opened at this session's project. Ignored while start-up is still
   977	    // re-attaching sessions, so a reload still loads nothing on its own.
   978	    shellPanels.sessionPicked();
   979	    // Clicking the session you are already on changes nothing. Putting the
   980	    // stored record back here would throw away every file opened since the last
   981	    // switch, which is the opposite of what a click on your own row means.
   982	    if (switching) restoreWorkspace(ownedId);
   983	    const provider = conversationProviderFor(ownedId);
   984	    if (selected && provider) {
   985	      // A running PTY is already the owner of this agent session. Starting a
   986	      // second `claude --resume` / `codex app-server` here makes two agents
   987	      // append different answers to one conversation. Until the structured
   988	      // surface mirrors the PTY's transcript, keep the real terminal visible.
   989	      if (selected.ptySessionId) {
   990	        void closeStructuredConversation(ownedId);
   991	        setConversationMode(ownedId, 'raw');
   992	        return;
   993	      }
   994	      void ensureStructuredConversation({
   995	        ownedId,
   996	        provider,
   997	        cwd: selected.cwd,
   998	        nativeSessionId: selected.nativeSessionId
   999	      }).catch((error) => {
  1000	        rail.error = `could not open structured ${provider}: ${describeError(error)}`;

exec
/bin/zsh -lc "nl -ba tauri-svelte-preview/src/lib/shell/components/conversation/ConversationHeader.svelte | sed -n '1,24p'; nl -ba tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts | sed -n '245,280p;330,350p'; nl -ba tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs | sed -n '1038,1085p'" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
     1	<script lang="ts">
     2	  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
     3	  import type { ConversationChildAgent } from '$lib/shell/conversation/conversationTypes.ts';
     4	  import type { ConversationViewMode, ConversationWorkspaceState } from '$lib/shell/conversation/conversationStore.svelte.ts';
     5	
     6	  interface Props {
     7	    active: OwnedSession;
     8	    conversation: ConversationWorkspaceState;
     9	    selectedChild: ConversationChildAgent | null;
    10	    onModeChange?(mode: ConversationViewMode): void;
    11	  }
    12	  let { active, conversation, selectedChild, onModeChange }: Props = $props();
    13	</script>
    14	
    15	<header class="conversation-header" data-testid="conversation-header">
    16	  <div class="identity"><strong data-testid="conversation-title">{selectedChild?.label ?? active.title}</strong><span>{selectedChild ? `${active.agent} child · read-only` : `${active.agent} session`}</span>{#if conversation.connectionState !== 'connected'}<small data-testid="conversation-connection-state">{conversation.connectionState}</small>{/if}</div>
    17	  <div class="header-actions"><span class="runtime-state" data-testid="conversation-runtime-state">{conversation.executionOwner}</span><button data-testid="conversation-open-raw" type="button" onclick={() => onModeChange?.('raw')}>Open raw terminal</button></div>
    18	</header>
    19	
    20	<style>.conversation-header{display:flex;justify-content:space-between;align-items:center;min-height:46px;padding:8px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 75%,transparent)}.identity{display:flex;align-items:baseline;gap:9px;min-width:0}.identity strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px}.identity span,.identity small,.runtime-state{color:var(--color-text-2);font-size:12px}.header-actions{display:flex;align-items:center;gap:9px}.header-actions button{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.header-actions button:hover{background:var(--color-hover)}.runtime-state{padding:3px 6px}</style>
   245	    if (getConversationSession(ownedId)?.generation === state.generation) {
   246	      failConversationConfigChange(ownedId, optionId, error instanceof Error ? error.message : String(error));
   247	    }
   248	    throw error;
   249	  }
   250	}
   251	
   252	export function startConversationTerminalProjection(input: {
   253	  ownedId: string;
   254	  provider: AgentConversationProvider;
   255	  nativeSessionId: string;
   256	}): void {
   257	  if (!isTauri()) return;
   258	  const signature = `${input.provider}:${input.nativeSessionId}`;
   259	  if (terminalProjections.get(input.ownedId) === signature) return;
   260	  terminalProjections.set(input.ownedId, signature);
   261	  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
   262	  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
   263	    request: { ...input, generation }
   264	  }).then((registration) => {
   265	    for (const event of registration.events) applyAgentConversationEvent(event);
   266	  }).catch(() => {
   267	    // The transcript may not exist until the agent accepts its first prompt.
   268	    // Dropping the signature lets the next activation register again.
   269	    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
   270	  });
   271	}
   272	
   273	export function stopConversationTerminalProjection(ownedId: string): void {
   274	  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
   275	  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
   276	}
   277	
   278	type HandoffInput = Omit<AgentConversationHandoffRequest, 'phase'>;
   279	
   280	function handoffTargetOwner(direction: AgentConversationHandoffDirection): AgentWriterLeaseOwner {
   330	  return invokeHandoff(input, 'commit');
   331	}
   332	
   333	export function rollbackConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
   334	  return invokeHandoff(input, 'rollback');
   335	}
   336	
   337	async function resyncConversation(ownedId: string): Promise<void> {
   338	  const existing = resyncing.get(ownedId);
   339	  if (existing) return existing;
   340	  const work = (async () => {
   341	    for (let attempt = 0; attempt < 3; attempt += 1) {
   342	      const snapshot = await invoke<AgentConversationSnapshot | null>(
   343	        'read_agent_conversation_snapshot',
   344	        { ownedId }
   345	      );
   346	      if (!snapshot) return;
   347	      const sequenceBeforeApply = getConversationSession(ownedId)?.lastSequence ?? 0;
   348	      applyAgentConversationSnapshot(snapshot);
   349	      if (sequenceBeforeApply <= snapshot.lastSequence) return;
   350	    }
  1038	                    AgentConversationProvider::Codex
  1039	                ))
  1040	                .unwrap(),
  1041	            first
  1042	        );
  1043	        let changed = manager
  1044	            .ensure(request(
  1045	                root.to_str().unwrap(),
  1046	                "owned-a",
  1047	                AgentConversationProvider::Claude,
  1048	            ))
  1049	            .unwrap();
  1050	        assert_eq!(changed.generation, 2);
  1051	        assert!(manager
  1052	            .emit_payload(
  1053	                "owned-a",
  1054	                first.generation,
  1055	                AgentConversationPayload::Error {
  1056	                    code: "stale".into(),
  1057	                    message: "stale".into(),
  1058	                    recoverable: true
  1059	                }
  1060	            )
  1061	            .is_err());
  1062	        fs::remove_dir_all(root).unwrap();
  1063	    }
  1064	
  1065	    #[test]
  1066	    fn canonical_sequence_and_snapshot_are_bounded_and_repairable() {
  1067	        let root = temp_root();
  1068	        let manager = AgentRuntimeManager::default();
  1069	        let connection = manager
  1070	            .ensure(request(
  1071	                root.to_str().unwrap(),
  1072	                "owned-a",
  1073	                AgentConversationProvider::Codex,
  1074	            ))
  1075	            .unwrap();
  1076	        for index in 0..(SNAPSHOT_EVENT_CAP + 3) {
  1077	            manager
  1078	                .emit_payload(
  1079	                    "owned-a",
  1080	                    connection.generation,
  1081	                    AgentConversationPayload::Error {
  1082	                        code: "fixture".into(),
  1083	                        message: index.to_string(),
  1084	                        recoverable: true,
  1085	                    },

exec
/bin/zsh -lc "rg -n \"emit_payload\\(\" tauri-svelte-preview/src-tauri/src/agent_conversation; rg -n \"app\\.emit\\(|agent-conversation-event\" tauri-svelte-preview/src-tauri/src/agent_conversation" in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265:    pub fn emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052:            .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1078:                .emit_payload(
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:13:pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:368:    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);

codex
Verified: the Session tab still mounts a real structured-conversation surface, but normal `/next` sessions are immediately forced into raw terminal mode. The ACP runtime exists, yet its live structured-event path is incomplete; the only production `agent-conversation-event` emitter is transcript projection.

## 1. Session-tab renderer

- **Verified** — `/next` defines `sessionArea()` and mounts `ConversationSurface` at [tauri-svelte-preview/src/routes/next/+page.svelte:1570-1583](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1570).
- **Verified** — That snippet is passed as the `session` center panel at [+page.svelte:1637-1646](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1637).
- **Verified** — `ShellFrame` registers the panel as `{ id: 'session', title: 'Session', element: sessionSlot }` at [ShellFrame.svelte:113-124](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:113).
- **Verified** — The slot renders `center.session()` at [ShellFrame.svelte:175-186](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:175).
- **Verified** — A fresh Dockview layout activates the first conversation panel, Session, at [layout/centerDock.ts:167-190](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/layout/centerDock.ts:167).
- **Verified** — A valid persisted layout is restored first at [layout/centerDock.ts:271-317](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/layout/centerDock.ts:271).

## 2. Structured component and switching condition

- **Verified** — `ConversationSurface` imports `ConversationTimeline`, `ConversationComposer`, `ConversationAgentTree`, and `TerminalSurface` at [ConversationSurface.svelte:1-16](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:1).
- **Verified** — The exact structured-view condition is:

  ```ts
  const structured = $derived(
    !!active &&
    (active.agent === 'codex' || active.agent === 'claude') &&
    conversation?.mode !== 'raw'
  );
  ```

  at [ConversationSurface.svelte:60-65](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:60).

- **Verified** — The terminal is always mounted, but hidden only when `structured` is true: [ConversationSurface.svelte:222-224](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:222).
- **Verified** — The structured branch renders the header, agent tree, timeline, and composer at [ConversationSurface.svelte:224-271](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:224).
- **Verified** — Raw mode exposes “Return to structured” only when `conversation?.mode === 'raw'` at [ConversationSurface.svelte:272-278](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:272).
- **Verified** — New conversation state defaults to `mode: 'structured'` at [conversationStore.svelte.ts:78-89](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:78).
- **Verified** — The header’s “Open raw terminal” button changes the mode to raw at [ConversationHeader.svelte:15-18](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/conversation/ConversationHeader.svelte:15).

## 3. ACP versus terminal data path

### Frontend event path

- **Verified** — The frontend listens for `agent-conversation-event` and applies events to the conversation store at [conversationService.ts:356-367](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:356).
- **Verified** — Structured setup invokes `ensure_agent_conversation`, then reads a snapshot at [conversationService.ts:381-395](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:381).
- **Verified** — Structured sends without a PTY invoke `send_agent_conversation_message`; sends with a PTY write bracketed text and Return directly into the PTY at [conversationService.ts:397-443](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:397).

### Backend ACP path

- **Verified** — `ensure_agent_conversation` calls `manager.activate`, which initializes ACP and creates/resumes an ACP session at [agent_conversation/mod.rs:19-34](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:19) and [manager.rs:190-263](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:190).
- **Verified** — `StructuredRuntimeHandle::Acp` delegates prompts to `AcpRuntimeAdapter` at [providers/acp.rs:146-170](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs:146).
- **Verified** — Normal ACP `prompt()` sends `session/prompt` and returns the turn ID at [providers/acp_client.rs:132-150](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:132).
- **Verified** — The regular request loop discards every nonmatching frame with `continue` at [providers/acp_client.rs:349-370](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:349).
- **Verified** — `manager.emit_payload` stores normalized frontend events, but repository search shows its only callers are tests at [manager.rs:265-308](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265) and [manager.rs:1052-1085](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052).

### Actual live event producer

- **Verified** — The production event constant is owned by terminal projection at [terminal_projection.rs:10-14](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:10).
- **Verified** — Projection polls provider transcript files and parses durable JSONL lines at [terminal_projection.rs:74-88](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:74) and [terminal_projection.rs:121-162](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:121).
- **Verified** — Those projected events are marked `source: "terminal-transcript"` and `historical: true` at [terminal_projection.rs:318-354](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:318).
- **Verified** — The only production `app.emit("agent-conversation-event", ...)` is terminal projection at [terminal_projection.rs:357-369](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:357).

**Conclusion:** the structured UI exists and consumes normalized store events, but normal live ACP notifications are not currently forwarded into it. The fallback path does not scrape xterm screen pixels; it parses provider transcript JSONL.

## 4. ACP plan versus current implementation

- **Verified — plan promise** — The amendment defines ACP as a structured JSON-RPC session, distinct from both tool terminals and native CLI PTYs at [2026-08-04 amendment:38-57](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:38).
- **Verified — plan promise** — New Codex and Claude sessions should start structured by default, with no permanent user-visible PTY at [amendment:116-123](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:116).
- **Verified — plan promise** — A new structured session becomes structured only after ACP initialization and `session/new` succeed at [amendment:255-260](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:255).
- **Verified — plan promise** — The manager owns ACP lifecycle and normalized event emission at [amendment:315-325](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:315).
- **Verified — current mismatch** — `startNewSession` still calls `service.startOwned`, creating a PTY, at [+page.svelte:1128-1149](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1128).
- **Verified — current mismatch** — The ACP manager has no production notification-to-frontend emission path; the frontend event is emitted by transcript projection instead, as shown above.
- **Verified — older contradictory guide** — The Rust guide still describes the pre-ACP PTY-authoritative architecture at [conversation-workbench-rust-guide.md:3-17](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/conversation-workbench-rust-guide.md:3).

## 5. Git archaeology

- **Verified** — The structured component was not deleted. `git log --follow` shows it surviving through `5884139`, `5643167`, `d68c238`, `4857674`, `ba46633`, and `5cf470b`; current structured markup remains at [ConversationSurface.svelte:224-271](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:224).
- **Verified** — `4857674` replaced the earlier 500 ms frontend transcript polling with Rust terminal-projection events; current projection implementation is at [terminal_projection.rs:74-162](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:74).
- **Verified — regression/removal** — Commit `ba46633` (“conversation: native CLI handoff between structured and terminal modes”) removed the prior `nativeSessionId` branch that set structured mode and started projection. Its parent had the structured branch at `+page.svelte:981-992`; the current branch is the unconditional raw branch at [+page.svelte:989-992](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:989).
- **Verified** — No evidence shows the structured panel itself being ripped out; the regression is routing/state wiring plus the incomplete ACP event pump.

## 6. Why it flashed once, then became unreachable

- **Verified** — The controlling state variable is `conversation.mode`, initialized to `'structured'` at [conversationStore.svelte.ts:78-89](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:78).
- **Verified** — Selecting any provider session with a live PTY calls `closeStructuredConversation()` and immediately sets `mode` to `'raw'` at [+page.svelte:983-992](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:983).
- **Verified** — Closing the structured conversation removes the manager runtime/session at [conversationService.ts:376-379](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:376) and [manager.rs:512-539](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:512).
- **Verified** — Persisted workspace state can restore `mode: 'raw'` on reload or session switch at [conversationStore.svelte.ts:600-606](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:600).
- **Assumed** — The reported flash is consistent with the initial default `'structured'` state being rendered before the asynchronous PTY selection completes, followed by the unconditional raw transition at [+page.svelte:1141-1149](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1141).
- **Assumed** — “Return to structured” can fail after that transition because the ACP manager was closed; the failure is caught and reported at [+page.svelte:1083-1091](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1083).

## Minimal changes needed

1. **Verified target** — Stop forcing live PTY sessions to raw in [+page.svelte:989-992](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:989); make ACP structured mode the default per the amendment at [amendment:116-123](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:116).
2. **Verified target** — Add a production ACP notification reader that converts `session/update` frames into `manager.emit_payload` events; the current drop point is [acp_client.rs:361-365](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:361).
3. **Verified target** — Emit those normalized events through the same frontend event consumed at [conversationService.ts:356-367](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:356).
4. **Verified target** — Create ordinary sessions through ACP without `startOwned` PTYs; the current PTY creation is at [+page.svelte:1141](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1141).
5. **Assumed** — Keep PTY projection only for explicit native-CLI sessions, and make the handoff retain/reconnect the structured runtime instead of closing it at [conversationService.ts:376-379](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:376).


tokens used
272,565
Verified: the Session tab still mounts a real structured-conversation surface, but normal `/next` sessions are immediately forced into raw terminal mode. The ACP runtime exists, yet its live structured-event path is incomplete; the only production `agent-conversation-event` emitter is transcript projection.

## 1. Session-tab renderer

- **Verified** — `/next` defines `sessionArea()` and mounts `ConversationSurface` at [tauri-svelte-preview/src/routes/next/+page.svelte:1570-1583](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1570).
- **Verified** — That snippet is passed as the `session` center panel at [+page.svelte:1637-1646](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1637).
- **Verified** — `ShellFrame` registers the panel as `{ id: 'session', title: 'Session', element: sessionSlot }` at [ShellFrame.svelte:113-124](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:113).
- **Verified** — The slot renders `center.session()` at [ShellFrame.svelte:175-186](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:175).
- **Verified** — A fresh Dockview layout activates the first conversation panel, Session, at [layout/centerDock.ts:167-190](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/layout/centerDock.ts:167).
- **Verified** — A valid persisted layout is restored first at [layout/centerDock.ts:271-317](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/layout/centerDock.ts:271).

## 2. Structured component and switching condition

- **Verified** — `ConversationSurface` imports `ConversationTimeline`, `ConversationComposer`, `ConversationAgentTree`, and `TerminalSurface` at [ConversationSurface.svelte:1-16](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:1).
- **Verified** — The exact structured-view condition is:

  ```ts
  const structured = $derived(
    !!active &&
    (active.agent === 'codex' || active.agent === 'claude') &&
    conversation?.mode !== 'raw'
  );
  ```

  at [ConversationSurface.svelte:60-65](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:60).

- **Verified** — The terminal is always mounted, but hidden only when `structured` is true: [ConversationSurface.svelte:222-224](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:222).
- **Verified** — The structured branch renders the header, agent tree, timeline, and composer at [ConversationSurface.svelte:224-271](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:224).
- **Verified** — Raw mode exposes “Return to structured” only when `conversation?.mode === 'raw'` at [ConversationSurface.svelte:272-278](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:272).
- **Verified** — New conversation state defaults to `mode: 'structured'` at [conversationStore.svelte.ts:78-89](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:78).
- **Verified** — The header’s “Open raw terminal” button changes the mode to raw at [ConversationHeader.svelte:15-18](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/conversation/ConversationHeader.svelte:15).

## 3. ACP versus terminal data path

### Frontend event path

- **Verified** — The frontend listens for `agent-conversation-event` and applies events to the conversation store at [conversationService.ts:356-367](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:356).
- **Verified** — Structured setup invokes `ensure_agent_conversation`, then reads a snapshot at [conversationService.ts:381-395](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:381).
- **Verified** — Structured sends without a PTY invoke `send_agent_conversation_message`; sends with a PTY write bracketed text and Return directly into the PTY at [conversationService.ts:397-443](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:397).

### Backend ACP path

- **Verified** — `ensure_agent_conversation` calls `manager.activate`, which initializes ACP and creates/resumes an ACP session at [agent_conversation/mod.rs:19-34](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:19) and [manager.rs:190-263](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:190).
- **Verified** — `StructuredRuntimeHandle::Acp` delegates prompts to `AcpRuntimeAdapter` at [providers/acp.rs:146-170](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs:146).
- **Verified** — Normal ACP `prompt()` sends `session/prompt` and returns the turn ID at [providers/acp_client.rs:132-150](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:132).
- **Verified** — The regular request loop discards every nonmatching frame with `continue` at [providers/acp_client.rs:349-370](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:349).
- **Verified** — `manager.emit_payload` stores normalized frontend events, but repository search shows its only callers are tests at [manager.rs:265-308](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:265) and [manager.rs:1052-1085](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1052).

### Actual live event producer

- **Verified** — The production event constant is owned by terminal projection at [terminal_projection.rs:10-14](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:10).
- **Verified** — Projection polls provider transcript files and parses durable JSONL lines at [terminal_projection.rs:74-88](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:74) and [terminal_projection.rs:121-162](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:121).
- **Verified** — Those projected events are marked `source: "terminal-transcript"` and `historical: true` at [terminal_projection.rs:318-354](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:318).
- **Verified** — The only production `app.emit("agent-conversation-event", ...)` is terminal projection at [terminal_projection.rs:357-369](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:357).

**Conclusion:** the structured UI exists and consumes normalized store events, but normal live ACP notifications are not currently forwarded into it. The fallback path does not scrape xterm screen pixels; it parses provider transcript JSONL.

## 4. ACP plan versus current implementation

- **Verified — plan promise** — The amendment defines ACP as a structured JSON-RPC session, distinct from both tool terminals and native CLI PTYs at [2026-08-04 amendment:38-57](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:38).
- **Verified — plan promise** — New Codex and Claude sessions should start structured by default, with no permanent user-visible PTY at [amendment:116-123](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:116).
- **Verified — plan promise** — A new structured session becomes structured only after ACP initialization and `session/new` succeed at [amendment:255-260](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:255).
- **Verified — plan promise** — The manager owns ACP lifecycle and normalized event emission at [amendment:315-325](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:315).
- **Verified — current mismatch** — `startNewSession` still calls `service.startOwned`, creating a PTY, at [+page.svelte:1128-1149](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1128).
- **Verified — current mismatch** — The ACP manager has no production notification-to-frontend emission path; the frontend event is emitted by transcript projection instead, as shown above.
- **Verified — older contradictory guide** — The Rust guide still describes the pre-ACP PTY-authoritative architecture at [conversation-workbench-rust-guide.md:3-17](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/conversation-workbench-rust-guide.md:3).

## 5. Git archaeology

- **Verified** — The structured component was not deleted. `git log --follow` shows it surviving through `5884139`, `5643167`, `d68c238`, `4857674`, `ba46633`, and `5cf470b`; current structured markup remains at [ConversationSurface.svelte:224-271](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:224).
- **Verified** — `4857674` replaced the earlier 500 ms frontend transcript polling with Rust terminal-projection events; current projection implementation is at [terminal_projection.rs:74-162](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:74).
- **Verified — regression/removal** — Commit `ba46633` (“conversation: native CLI handoff between structured and terminal modes”) removed the prior `nativeSessionId` branch that set structured mode and started projection. Its parent had the structured branch at `+page.svelte:981-992`; the current branch is the unconditional raw branch at [+page.svelte:989-992](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:989).
- **Verified** — No evidence shows the structured panel itself being ripped out; the regression is routing/state wiring plus the incomplete ACP event pump.

## 6. Why it flashed once, then became unreachable

- **Verified** — The controlling state variable is `conversation.mode`, initialized to `'structured'` at [conversationStore.svelte.ts:78-89](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:78).
- **Verified** — Selecting any provider session with a live PTY calls `closeStructuredConversation()` and immediately sets `mode` to `'raw'` at [+page.svelte:983-992](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:983).
- **Verified** — Closing the structured conversation removes the manager runtime/session at [conversationService.ts:376-379](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:376) and [manager.rs:512-539](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:512).
- **Verified** — Persisted workspace state can restore `mode: 'raw'` on reload or session switch at [conversationStore.svelte.ts:600-606](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:600).
- **Assumed** — The reported flash is consistent with the initial default `'structured'` state being rendered before the asynchronous PTY selection completes, followed by the unconditional raw transition at [+page.svelte:1141-1149](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1141).
- **Assumed** — “Return to structured” can fail after that transition because the ACP manager was closed; the failure is caught and reported at [+page.svelte:1083-1091](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1083).

## Minimal changes needed

1. **Verified target** — Stop forcing live PTY sessions to raw in [+page.svelte:989-992](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:989); make ACP structured mode the default per the amendment at [amendment:116-123](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:116).
2. **Verified target** — Add a production ACP notification reader that converts `session/update` frames into `manager.emit_payload` events; the current drop point is [acp_client.rs:361-365](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:361).
3. **Verified target** — Emit those normalized events through the same frontend event consumed at [conversationService.ts:356-367](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:356).
4. **Verified target** — Create ordinary sessions through ACP without `startOwned` PTYs; the current PTY creation is at [+page.svelte:1141](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/routes/next/+page.svelte:1141).
5. **Assumed** — Keep PTY projection only for explicit native-CLI sessions, and make the handoff retain/reconnect the structured runtime instead of closing it at [conversationService.ts:376-379](/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:376).


