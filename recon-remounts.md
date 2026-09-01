# `/next` session-switch remount recon

## Bottom line

- **[Verified]** `rail.activeOwnedId` is changed by `selectOwned`, then passed as a prop to the rail, right panel, and `ConversationSurface`; none of those three parent components is behind an active-session `{#key}` or keyed `{#each}`. The switch also restores per-session tabs and workspace data. `tauri-svelte-preview/src/routes/next/+page.svelte:868-925`
- **[Verified]** The rail rows, right-panel bodies, terminal surface, center tab roster, and the `ConversationSurface` outer component stay mounted during an ordinary session-to-session switch. Their props/stores rebind. `tauri-svelte-preview/src/routes/next/+page.svelte:1623-1689`; `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:172-186`
- **[Verified]** The largest normal structured-conversation churn is inside the stable `ConversationTimeline`: its visible `TimelineItem` children are keyed by `item.itemId`, so changing to another session replaces the visible item subtrees even though the timeline/virtualizer component itself remains. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:193-205,595-632`; `tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte:36-54`
- **[Assumed]** The first rebind investigation targets should be the visible timeline-item window, then the structured/raw conversation branch when it actually changes, then the native Git diff branch when the remembered center tab is Diff. This is a source-size ranking, not a measured allocation attribution. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:426-515`; `tauri-svelte-preview/src/lib/shell/components/GitDiffView.svelte:120-129`

## Scope and evidence

- **[Verified]** This is a read-only source recon of the current dirty worktree. The report file is the only new file written. Existing modifications were preserved.
- **[Verified]** The current profiling edits disable conversation loading, editor, Git History, Source Control, and terminal host creation. Those edits change what is instantiated in this A/B run, but the source still shows the intended component boundaries and keys. `tauri-svelte-preview/src/routes/next/+page.svelte:926-937,1692-1726`; `tauri-svelte-preview/src/lib/shell/components/RightPanel.svelte:45-72`; `tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:44-52`
- **[Assumed]** “Trivial”, “moderate”, and “heavy” below mean the likely DOM/component/native-resource footprint of the boundary, inferred from its children. No heap snapshot or allocation trace was run in this recon.
- **[Verified]** Every active-session claim below has a source line. “Assumed” is used for footprint and for a possible future rebind design, not for the existence of a Svelte boundary.

## What a switch does before rendering reacts

1. **[Verified]** `selectOwned` closes the draft surface first, records the previous ID, and calls `setActiveOwned(ownedId)`. `tauri-svelte-preview/src/routes/next/+page.svelte:868-883`
2. **[Verified]** It then shows the selected terminal, points session-scoped panels at the new selection, releases old editor resources, restores the new workspace and remembered center/right tabs, and sets the conversation mode. `tauri-svelte-preview/src/routes/next/+page.svelte:899-925`
3. **[Verified]** `restoreTabsFor` changes the single `centerTab` and `rightTab` values; it does not construct a per-session component tree. `tauri-svelte-preview/src/routes/next/+page.svelte:346-387`

## Render tree and stable parents

| Area | Current boundary | What `activeOwnedId` does | Remount verdict |
|---|---|---|---|
| Session rail | `SessionsColumn` receives `activeOwnedId`; expanded `SessionRail` receives it as a prop. `tauri-svelte-preview/src/routes/next/+page.svelte:1623-1641`; `tauri-svelte-preview/src/lib/shell/components/SessionsColumn.svelte:377-388` | Changes each row’s `active` value. | **[Verified] Rebind.** No active ID in the group/session keys. |
| Rail rows | Groups keyed by `group.key`; rows keyed by `session.ownedId`; `active` is a boolean prop. `tauri-svelte-preview/src/lib/shell/components/SessionRail.svelte:188-241` | The old row loses `active`; the new row gains it. | **[Verified] Rebind.** Row `onDestroy` is for actual row removal, not selection. `tauri-svelte-preview/src/lib/shell/components/WorktreeAgentRow.svelte:350-362` |
| Right column | `RightPanel` is one child of the page and receives `ownedId` as data. `tauri-svelte-preview/src/routes/next/+page.svelte:1643-1649` | Panel stores/effects read the new root/owned ID; CSS selects the visible body. | **[Verified] Rebind.** No parent key. |
| Center tabs | `CenterCornerTabs` receives only `centerTab`; its tab list is keyed by fixed tab IDs. `tauri-svelte-preview/src/routes/next/+page.svelte:1651-1653`; `tauri-svelte-preview/src/lib/shell/components/CenterCornerTabs.svelte:87-117` | Changes the active tab and a short-lived hold class. | **[Verified] Rebind.** The editor-only language controls are a tab-state branch, not a session branch. `tauri-svelte-preview/src/lib/shell/components/CenterCornerTabs.svelte:87-95` |
| Session center | `ConversationSurface` is one child of the page; `TerminalSurface` is inside it. `tauri-svelte-preview/src/routes/next/+page.svelte:1657-1676`; `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:424-425` | Derived `active`, `conversation`, `structured`, timeline data, and composer props change. | **[Verified] Outer components rebind.** The structured branch has conditional descendants described below. |
| Dock/shell | Snippets are rendered into fixed parking slots; Dockview moves the supplied element into a host and returns it to parking on renderer disposal. `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:95-186`; `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts:88-114` | Panel activation changes which existing center element is in front. | **[Verified] DOM move, not Svelte remount.** |

## Remount boundaries, in detail

### 1. Visible transcript item subtrees — highest normal structured-switch churn

- **[Verified boundary]** `ConversationTimeline` stays mounted and receives a new `items`, `conversationId`, and `renderWindowId`. Its session identity is in `renderWindowId`, not in a component key. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:444-467`
- **[Verified trigger]** On a new `renderWindowId`, the timeline effect clears the anchored item, fold state, expanded-turn map, and restores the target session’s scroll policy. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:193-205`
- **[Verified remount]** The virtualized outer rows are keyed by `row.key`, while each visible group’s items are keyed by `item.itemId`; an item ID that exists only in the old session is destroyed and an item ID only in the new session is created. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:595-632`
- **[Verified depth]** Each `TimelineItem` selects one kind-specific child (`UserMessageItem`, `AssistantMessageItem`, tool, file, approval, and so on) behind an `item.kind` branch. `tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte:36-54`
- **[Assumed footprint]** **Moderate to heavy per visible item window.** A tool, assistant, file-edit, or message item can contain substantially more DOM than the timeline wrapper; virtualization limits this to the visible/overscanned rows rather than the full transcript. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:142-147,595-632`
- **[Assumed rebind result]** The parent timeline is already the correct stable rebind point. Removing the `item.itemId` key would reuse old item components for different transcript identities and can lose per-item state or leave the wrong kind-specific subtree in place. A no-loss fix should preserve item identity semantics; simply making the inner each unkeyed is not safe. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:606-621`; `tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte:32-52`
- **[Verified lifecycle]** Session changes are handled by the timeline effect and virtualizer/observer effects; the timeline itself is not destroyed on an ordinary structured-to-structured switch. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:182-205`

### 2. The structured conversation section — largest conditional subtree

- **[Verified boundary]** `ConversationSurface` has one structured section at lines 426-508 and a raw-actions fallback at lines 509-515. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:424-516`
- **[Verified trigger]** The structured condition is derived from the active session, agent kind, origin, and conversation mode. A switch can therefore flip it when the target is raw, when the target has no conversation record yet, or when the target is not an applicable structured agent. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:90-98`
- **[Verified remount]** When the condition goes from true to false, the section containing `ConversationAgentTree`, `ConversationTimeline`, and (for a parent conversation) `ConversationComposer` is destroyed; when it becomes true again, those child components are created again. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:426-508`
- **[Assumed footprint]** **Heavy.** This is the whole structured transcript surface, including the visible timeline and composer. The terminal is deliberately outside the branch, so its DOM is not part of this remount. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:424-469`
- **[Assumed rebind result]** A stable mode host could data-rebind structured/raw state, but retaining a hidden structured transcript for raw sessions would retain its DOM and local state. Behavioral parity would require explicit mode gating for handoff buttons, read-only state, and composer visibility; deleting this condition mechanically is not a no-loss change. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:426-515`
- **[Verified lifecycle]** The surface’s active-session effects already perform the needed per-session cleanup/rebind: viewed-session presence is cleared for the old ID, and config/capability effects use new session keys. Those effects must remain if the parent is kept mounted. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:203-250`

### 3. Native Git diff editor — conditional heavy native child, not a remounted `GitDiffView`

- **[Verified parent]** `GitDiffView` is a singleton center snippet. `tauri-svelte-preview/src/routes/next/+page.svelte:1715-1719`; `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:183-186`
- **[Verified boundary]** `NativeGitDiffEditor` exists only when the Diff tab is showing, the selected diff has full models, and a repository root exists. `tauri-svelte-preview/src/lib/shell/components/GitDiffView.svelte:99-129`
- **[Verified active-session trigger]** On a switch, workspace restoration can clear or load the target session’s remembered diff and tab restoration can change `showing`. `tauri-svelte-preview/src/routes/next/+page.svelte:820-847,384-387`
- **[Verified remount]** If that condition changes from true to false, the Monaco diff component is destroyed; returning to true mounts it and runs its asynchronous Monaco/service setup. `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:97-147`
- **[Assumed footprint]** **Heavy but conditional.** The child creates a Monaco diff editor and two text models. `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:67-86,110-125`
- **[Verified rebind]** If the native child remains mounted while the selected diff changes, it already rebinds through an effect, but that effect disposes the two old Monaco models and creates two new ones for the new file/content. `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:59-95`
- **[Verified lifecycle]** Model disposal on data replacement and editor disposal on actual component destruction are explicit resource ownership. Removing those disposals would trade churn for leaks, not produce a safe rebind. `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:59-65,140-147`
- **[Assumed rebind result]** The no-loss rebind target here is the editor component/host, which already stays stable when the condition stays true. Reusing models across sessions would need a separate, explicit ownership/cache design and is not implied by this recon.

### 4. Composer branch when the target is a selected child transcript

- **[Verified boundary]** The composer is inside `{#if !conversation.selectedChildId}`. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:468-507`
- **[Verified trigger]** `selectedChildId` is derived from the active conversation, so switching from a parent session to a selected-child view (or back) destroys/creates the composer component. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:93-98,468-507`
- **[Assumed footprint]** **Moderate.** The composer includes the prompt, menus, attachments, plan and approval UI, but not the transcript. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:469-506`
- **[Assumed rebind result]** A stable composer could be given an explicit read-only/hidden mode, but keeping the current component alive without that contract would expose a prompt in a child transcript that is intentionally read-only. The current branch is behaviorally load-bearing. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:468-507`; `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationComposer.svelte:161-171`
- **[Verified lifecycle]** Its draft-sync effect resets input/command state when the supplied draft changes, and its resize observer is cleaned up when the composer is actually removed. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationComposer.svelte:161-171,226-233`

### 5. Conversation-agent tree child nodes

- **[Verified boundary]** The tree component itself is stable, but its root and nested nodes are keyed by `childId`; an old session’s child nodes are replaced by the target session’s child nodes. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationAgentTree.svelte:16-34`
- **[Assumed footprint]** **Trivial to moderate.** These are navigation buttons and labels, not transcript bodies. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationAgentTree.svelte:16-33`
- **[Assumed rebind result]** Reusing nodes across different child IDs would lose the identity that drives selection and nested tree structure. Keep the keys; this is not a first rebind target. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationAgentTree.svelte:20-32`

### 6. Draft session surface — intentional teardown on every selection

- **[Verified boundary]** The page mounts `DraftSessionSurface` only while `draftOpen` is true. `tauri-svelte-preview/src/routes/next/+page.svelte:1677-1689`
- **[Verified trigger]** `selectOwned` sets `draftOpen = false` before changing the active ID. `tauri-svelte-preview/src/routes/next/+page.svelte:868-876`
- **[Assumed footprint]** **Moderate.** It contains the new-session surface and a composer, but it is present only while a draft is open. `tauri-svelte-preview/src/routes/next/+page.svelte:1677-1688`; `tauri-svelte-preview/src/lib/shell/newSession/DraftSessionSurface.svelte:1-13`
- **[Verified lifecycle]** Mounting hydrates state, sets roots, chooses defaults, loads refs, and focuses the composer; the component’s own documentation says abandoning a draft discards its state. `tauri-svelte-preview/src/lib/shell/newSession/DraftSessionSurface.svelte:10-13,228-235`
- **[Verified load-bearing verdict]** This remount should stay unless the product deliberately changes draft-abandonment semantics. It is not the explanation for ordinary switches when no draft is open. `tauri-svelte-preview/src/routes/next/+page.svelte:872-876`; `tauri-svelte-preview/src/lib/shell/newSession/DraftSessionSurface.svelte:10-13`

## Areas that only rebind

### Rail

- **[Verified]** `SessionRail` groups are keyed by stable group keys and rows by stable `ownedId`; `activeOwnedId` is used only to compute the row’s `active` prop. `tauri-svelte-preview/src/lib/shell/components/SessionRail.svelte:188-241`
- **[Verified]** `WorktreeAgentRow` renders active styling and `aria-current` from that prop; its cleanup is for row destruction, not active selection. `tauri-svelte-preview/src/lib/shell/components/WorktreeAgentRow.svelte:350-362,465-507`

### Terminal

- **[Verified intended structure]** `TerminalSurface` explicitly avoids `{#if active}` and keys hosts by `session.ownedId`; active selection controls visibility/empty-state text, not host identity. `tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:3-18,134-170`
- **[Verified intended manager behavior]** The terminal manager idempotently returns an existing view, hides other views, shows/fits/focuses the target, and disposes only a closed view. `tauri-svelte-preview/src/lib/liveConversationTerminals.ts:128-168,175-202,273-293`
- **[Verified side effect]** Switching terminal visibility releases/acquires the WebGL addon, but `xterm` itself is disposed only by `dispose()`. `tauri-svelte-preview/src/lib/shell/xtermFactory.ts:245-279`
- **[Current-WIP caveat]** The dirty profiling edit makes `hosted` an empty array, so this worktree’s current run creates no terminal hosts; the intended no-remount contract is still explicit in the component and manager source. `tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:44-52`

### Right panel

- **[Verified intended structure]** `RightPanel` documents eight panels mounted for the whole shell and uses CSS display switching rather than `{#if}` around the panel components. `tauri-svelte-preview/src/lib/shell/components/RightPanel.svelte:3-15,42-72,101-110`
- **[Current-WIP caveat]** Source Control is commented out for the memory A/B, so the current dirty runtime has seven direct panel instances; the eight-panel claim is the surrounding architecture contract. `tauri-svelte-preview/src/lib/shell/components/RightPanel.svelte:19-26,45-72`
- **[Verified rebind examples]** Agents derives rows from the new `ownedId` and keys only child rows; Session Context switches its empty/content branch only when `ownedId` becomes null/non-null; Browser snapshots and restores state in an effect keyed by `ownedId`. `tauri-svelte-preview/src/lib/shell/panels/agents/AgentsPanel.svelte:41-76,97-118`; `tauri-svelte-preview/src/lib/shell/panels/context/SessionContextPanel.svelte:56-112,125-156`; `tauri-svelte-preview/src/lib/shell/panels/browser/BrowserPanel.svelte:698-735`
- **[Verified verdict]** Nothing above `RightPanel` remounts it per active session. Internal list rows or empty-state DOM can change with panel data, but the panel component instances and their local state remain. `tauri-svelte-preview/src/routes/next/+page.svelte:1643-1649`; `tauri-svelte-preview/src/lib/shell/components/RightPanel.svelte:42-72`

### Center editor and history snippets

- **[Current-WIP caveat]** The current profiling source comments out `EditorPanel` and `GitHistoryView`, replacing each with an empty `<div>`/empty snippet. `tauri-svelte-preview/src/routes/next/+page.svelte:1692-1726`
- **[Verified intended structure]** When restored, both are supplied as fixed snippets to `ShellFrame`’s fixed parking slots; no active-session key appears around either slot. `tauri-svelte-preview/src/routes/next/+page.svelte:1746-1753`; `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:172-186`
- **[Verified resource distinction]** `selectOwned` explicitly releases old editor session resources before restoring the target workspace. That is model/file-resource replacement inside a stable editor component, not a Svelte subtree remount. `tauri-svelte-preview/src/routes/next/+page.svelte:907-916`

## Rank and rebind targets

The ranking combines likely footprint with the condition occurring during a switch. It is not a heap measurement.

1. **[Assumed rank 1] Visible `TimelineItem` descendants.** This is the common structured-to-structured path and can replace many visible message/tool/file subtrees at once. Target the stable timeline/data window first; do not remove `item.itemId` keys without preserving item identity and per-item state. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:595-632`; `tauri-svelte-preview/src/lib/shell/components/conversation/TimelineItem.svelte:36-54`
2. **[Assumed rank 2] `ConversationSurface` structured branch.** When the target changes structured/raw mode or has a transiently missing conversation, the entire transcript-plus-composer branch can be replaced. A stable mode host is worth considering only with explicit raw/read-only behavior and the existing active-session effects preserved. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:90-98,426-515`
3. **[Assumed rank 3] Native Git Diff when Diff is the remembered center tab.** The component branch can create/destroy Monaco, and a same-component data change still disposes/recreates two Monaco models. Investigate this only for switches that actually restore Diff; it is not evidence that the center `GitDiffView` parent remounts. `tauri-svelte-preview/src/lib/shell/components/GitDiffView.svelte:120-129`; `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:59-95,97-147`

Lower-ranked boundaries are the selected-child composer branch (moderate), the agent-tree child nodes (trivial/moderate), and the draft surface (moderate but intentional and infrequent). `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:468-507`; `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationAgentTree.svelte:16-34`; `tauri-svelte-preview/src/routes/next/+page.svelte:1677-1689`

## Boundaries that should remain load-bearing

- **[Verified]** Keep transcript item keys by `itemId`; they encode item identity and kind-specific state. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:606-621`
- **[Verified]** Keep terminal hosts keyed by `ownedId` and keep the manager’s hide/show model; removing a host on selection would destroy the xterm DOM and violate the documented live-session behavior. `tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:3-18,134-151`; `tauri-svelte-preview/src/lib/liveConversationTerminals.ts:5-9`
- **[Verified]** Keep draft teardown on selection while draft abandonment means discard. `tauri-svelte-preview/src/routes/next/+page.svelte:872-876`; `tauri-svelte-preview/src/lib/shell/newSession/DraftSessionSurface.svelte:10-13`
- **[Verified]** Keep the selected-child composer exclusion unless a future component contract explicitly makes the composer read-only/hidden. `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:468-507`
- **[Verified]** Keep Monaco model disposal when a diff really changes and editor disposal when the native component leaves; otherwise the proposed rebind would leak native resources. `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:59-65,140-147`

## Final conclusion

**[Verified]** The source does not show a keyed “only active session” wrapper around the rail, right panel, terminal surface, center dock, or `ConversationSurface`. The hypothesis is therefore only partly supported by structure: normal session switching rebinds the major parents, but it still replaces the visible keyed transcript items and can replace entire conditional structured/raw, composer, draft, or native-diff children under specific session-state changes. `tauri-svelte-preview/src/routes/next/+page.svelte:1623-1726`; `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:424-516`

**[Assumed]** If rapid switching is mostly between ordinary structured parent conversations with the Session center tab selected, the visible transcript item window is the first place to attribute and measure. If switches cross raw sessions or remembered Diff tabs, the structured branch and native diff editor become conditional high-cost paths. The current source recon cannot prove which path owns the reported 1.1–1.4 GB ceiling. `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:595-632`; `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:426-515`; `tauri-svelte-preview/src/lib/shell/components/git/NativeGitDiffEditor.svelte:97-147`
