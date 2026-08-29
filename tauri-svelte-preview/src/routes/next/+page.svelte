<script lang="ts">
	/**
	 * /next — the shell orchestrator.
	 *
	 * Thin shell component: directly composes the real workbench components
	 * (SessionsColumn, RightPanel, DockPanel, GitDiffView, GitHistoryView)
	 * and uses controllers for state management.
	 */
	import { onMount } from "svelte";

	import { PRODUCT_DOCUMENT_TITLE } from "$lib/productIdentity";

	import "$lib/shell/styles/nextTokens.css";
	import "$lib/shell/styles/next.css";
	import "$lib/shell/styles/themeChrome.css";

	import CenterCornerTabs from "$lib/shell/components/CenterCornerTabs.svelte";
	import DockPanel from "$lib/shell/components/DockPanel.svelte";
	import GitDiffView from "$lib/shell/components/GitDiffView.svelte";
	import GitHistoryView from "$lib/shell/components/git/GitHistoryView.svelte";
	import RightPanel from "$lib/shell/components/RightPanel.svelte";
	import SessionsColumn from "$lib/shell/components/SessionsColumn.svelte";
	import ShellFrame from "$lib/shell/components/ShellFrame.svelte";
	import ShellOverlays from "$lib/shell/components/ShellOverlays.svelte";
	import UtilityStrip from "$lib/shell/components/UtilityStrip.svelte";
	import type { UtilityId } from "$lib/shell/components/utilityStrip";

	import { SessionSelectionController } from "$lib/shell/controllers/sessionSelectionController.svelte";
	import { WorkbenchController } from "$lib/shell/controllers/workbenchController.svelte";
	import { TerminalController } from "$lib/shell/controllers/terminalController.svelte";
	import { scanRail, startShell, stopShell } from "$lib/shell/controllers/shellStartup";

	const selection = new SessionSelectionController();
	const workbench = new WorkbenchController();
	const terminal = new TerminalController();

	let sessionsColumn = $state<SessionsColumn | null>(null);
	let overlays = $state<ShellOverlays | null>(null);
	let openUtility = $state<UtilityId | null>(null);

	onMount(() => {
		void startShell({
			terminal,
			onSelectInitial: async (ownedId) => {
				await selection.selectSession(ownedId);
			},
		});

		return () => {
			stopShell({ terminal });
		};
	});

	function handleAskRemoveSession(ownedId: string): void {
		void selection.removeSession(ownedId);
	}
</script>

<svelte:head>
	<title>{PRODUCT_DOCUMENT_TITLE}</title>
</svelte:head>

{#snippet sessionsArea()}
	<div class="sessions-region">
		<div class="sessions-list">
			<SessionsColumn
				bind:this={sessionsColumn}
				owned={selection.railOwned}
				activeOwnedId={selection.activeOwnedId}
				collapsed={workbench.sessionsCollapsed}
				onCollapse={(collapsed) => workbench.collapseSessions(collapsed)}
				onNewSession={() => {}}
				onSelectSession={(ownedId) => {
					void selection.selectSession(ownedId);
				}}
				onAskRemove={handleAskRemoveSession}
			/>
		</div>
	</div>
{/snippet}

{#snippet toolsArea()}
	<RightPanel
		activeId={selection.controlledSelectionOwnedId === null ? workbench.rightTab : "files"}
		onSelect={(id) => {
			if (selection.controlledSelectionOwnedId === null || id === "files") workbench.selectRightTab(id);
		}}
		root={selection.controlledSelectionOwnedId === null ? selection.durableSessionRoot : ""}
		rootAvailable={selection.controlledSelectionOwnedId === null ? selection.activeRootAvailable : false}
		ownedId={selection.controlledSelectionOwnedId === null ? selection.activeOwnedId : null}
		filesRoot={selection.activeRootRemote ? "" : selection.filesProjectionRoot}
		filesOwnedId={selection.filesProjectionOwnedId}
		expandedPathsByRoot={selection.expandedPathsByRoot}
		onExpandedPathsChange={(root, paths) => selection.rememberExpandedPaths(root, paths)}
		filesInspectionRoot={null}
		sourceControlInspectionRoot={null}
	/>
{/snippet}

{#snippet centerTabsArea()}
	<CenterCornerTabs
		activeId={workbench.centerTab}
		onSelect={(id) => workbench.selectCenterTab(id)}
	/>
{/snippet}

{#snippet dockArea()}
	<DockPanel onReset={() => workbench.resetLayout()} />
{/snippet}

{#snippet sessionArea()}
	<!-- Conversation isolated/disabled during baseline measurement -->
{/snippet}

{#snippet editorArea()}
	<!-- Editor isolated/disabled during baseline measurement -->
{/snippet}

{#snippet diffArea()}
	{#if workbench.centerTab === "diff"}
		<GitDiffView
			showing={true}
			rootAvailable={selection.activeRootAvailable}
			mode={workbench.diffMode}
			onModeChange={(mode) => workbench.setDiffMode(mode)}
		/>
	{/if}
{/snippet}

{#snippet gitHistoryArea()}
	{#if workbench.centerTab === "git-history"}
		<GitHistoryView
			root={selection.activeRootAvailable ? selection.durableSessionRoot : ""}
			rootAvailable={selection.activeRootAvailable}
		/>
	{/if}
{/snippet}

<main
	class="next-shell"
	oncontextmenu={(event) => {
		const target = event.target instanceof Element ? event.target : null;
		if (target?.closest('input, textarea, [contenteditable="true"]')) return;
		event.preventDefault();
	}}
>
	<div class="frame-area">
		<ShellFrame
			sessions={sessionsArea}
			tools={toolsArea}
			centerTabs={centerTabsArea}
			dock={dockArea}
			center={{
				session: sessionArea,
				editor: editorArea,
				diff: diffArea,
				gitHistory: gitHistoryArea,
			}}
			onReady={(controls) => workbench.onFrameReady(controls)}
			onCenterPanelShown={(id) => workbench.handleCenterPanelShown(id)}
		/>
	</div>

	<UtilityStrip
		{openUtility}
		onOpenUtility={(id, anchor) => overlays?.openUtility(id, anchor)}
		onOpenSettings={() => overlays?.openSettings()}
	/>

	<ShellOverlays
		bind:this={overlays}
		onResetLayout={() => workbench.resetLayout()}
		onRescanSessions={() => scanRail()}
		message={null}
		onUtilityStateChange={(id, open) => {
			openUtility = open ? id : null;
		}}
	/>
</main>

<style>
	.next-shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		background: var(--color-bg);
		color: var(--color-text);
	}

	.frame-area {
		flex: 1 1 auto;
		min-height: 0;
		position: relative;
	}

	.sessions-region {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
		overflow: hidden;
	}

	.sessions-list {
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}
</style>
