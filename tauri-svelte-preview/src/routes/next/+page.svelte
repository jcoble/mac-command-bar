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
	import { registerSessionRowJumpTarget } from "$lib/shell/components/sessionRowJump.ts";
	import ConversationSurface from "$lib/shell/components/ConversationSurface.svelte";
	import DockPanel from "$lib/shell/components/DockPanel.svelte";
	import EditorPanel from "$lib/shell/components/EditorPanel.svelte";
	import GitDiffView from "$lib/shell/components/GitDiffView.svelte";
	import GitHistoryView from "$lib/shell/components/git/GitHistoryView.svelte";
	import RightPanel from "$lib/shell/components/RightPanel.svelte";
	import SessionsColumn from "$lib/shell/components/SessionsColumn.svelte";
	import ShellFrame from "$lib/shell/components/ShellFrame.svelte";
	import ShellOverlays from "$lib/shell/components/ShellOverlays.svelte";
	import UtilityStrip from "$lib/shell/components/UtilityStrip.svelte";
	import type { UtilityId } from "$lib/shell/components/utilityStrip";

	import { SessionSelectionController } from "$lib/shell/controllers/sessionSelectionController.svelte";
	import type { EditorPanelLifecycle } from "$lib/shell/controllers/editorSessionController.svelte";
	import { WorkbenchController } from "$lib/shell/controllers/workbenchController.svelte";
	import { startShell, stopShell } from "$lib/shell/controllers/shellStartup";
	import { gitService } from "$lib/shell/git/gitService";
	import DraftSessionSurface from "$lib/shell/newSession/DraftSessionSurface.svelte";
	import type { ThreadStartRequest } from "$lib/shell/newSession/threadStartFlow";
	import {
		clearWorkbenchNavigation,
		registerWorkbenchNavigation,
	} from "$lib/shell/workbenchNavigation";

	const selection = new SessionSelectionController();
	const workbench = new WorkbenchController();

	let sessionsColumn = $state<SessionsColumn | null>(null);
	let editorPanel = $state<EditorPanelLifecycle | null>(null);
	let overlays = $state<ShellOverlays | null>(null);
	let openUtility = $state<UtilityId | null>(null);

	$effect(() => {
		selection.setEditorPanel(editorPanel);
	});

	onMount(() => {
		const releaseSessionRowJump = registerSessionRowJumpTarget({
			selectSession,
			showCenterPanel: (_ownedId, id) => workbench.selectCenterTab(id),
			showSidebarView: (_ownedId, id) => workbench.selectRightTab(id),
		});
		registerWorkbenchNavigation({
			showCenterTab: (id) => workbench.selectCenterTab(id),
			showRightTab: (id) => workbench.selectRightTab(id),
			openDiff: async (request) => {
				if (!selection.activeRootAvailable) return;
				await gitService.showStoredDiff(request.projectRoot, request.relativePath);
			},
			openFileTimeline: async (request) => {
				const ownedId = selection.activeOwnedId;
				const sessionRoot = selection.durableSessionRoot;
				if (!selection.activeRootAvailable || !ownedId || !sessionRoot) return false;
				workbench.selectCenterTab("git-history");
				await gitService.showFileHistory(request.projectRoot, request.relativePath);
				if (
					!selection.activeRootAvailable ||
					selection.activeOwnedId !== ownedId ||
					selection.durableSessionRoot !== sessionRoot
				)
					return false;
				return true;
			},
		});
		void startShell({
			onSelectInitial: async (ownedId, stopSignal) => {
				if (stopSignal.aborted) return;
				await selection.selectSession(ownedId);
				if (stopSignal.aborted) return;
			},
		});

		return () => {
			releaseSessionRowJump();
			clearWorkbenchNavigation();
			void disposeRoute();
		};
	});

	async function disposeRoute(): Promise<void> {
		const selectionDisposal = selection.dispose();
		stopShell();
		await selectionDisposal;
	}

	function handleAskRemoveSession(ownedId: string): void {
		void selection.removeSession(ownedId);
	}

	async function selectSession(ownedId: string): Promise<void> {
		await selection.selectSession(ownedId);
	}

	function openNewSession(): void {
		selection.newSession.open();
		workbench.selectCenterTab("session");
	}

	async function startNewSession(request: ThreadStartRequest): Promise<void> {
		await selection.newSession.start(
			request,
			async (ownedId) => {
				await selection.selectSession(ownedId);
			},
			() => {
				workbench.selectCenterTab("session");
				workbench.selectRightTab("files");
			},
		);
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
				onNewSession={openNewSession}
				onSelectSession={(ownedId) => {
					void selectSession(ownedId);
				}}
				onAskRemove={handleAskRemoveSession}
			/>
		</div>
	</div>
{/snippet}

{#snippet toolsArea()}
	<RightPanel
		activeId={workbench.rightTab}
		onSelect={(id) => workbench.selectRightTab(id)}
		root={selection.durableSessionRoot}
		rootAvailable={selection.activeRootAvailable}
		ownedId={selection.activeOwnedId}
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
	<div class="session-area">
		{#if selection.hasChatProjection && selection.chatOwnedId}
			<div
				class="conversation-surface-shell"
				hidden={selection.chatOwnedId !== selection.activeOwnedId}
				aria-hidden={selection.chatOwnedId !== selection.activeOwnedId ? "true" : undefined}
				inert={selection.chatOwnedId !== selection.activeOwnedId}
			>
				<ConversationSurface
					owned={selection.railOwned}
					activeOwnedId={selection.chatOwnedId}
					rootAvailable={selection.activeRootAvailable}
				/>
			</div>
		{/if}
		{#if selection.activeOwnedId && (!selection.hasChatProjection || selection.chatOwnedId !== selection.activeOwnedId)}
			<div class="conversation-data-isolation" aria-label="Loading conversation">
				<p>Loading conversation…</p>
			</div>
		{:else if !selection.activeOwnedId}
			<div class="conversation-data-isolation" aria-label="No session selected">
				<p>Select a session to view conversation.</p>
			</div>
		{/if}
		{#if selection.newSession.draftOpen}
			<DraftSessionSurface
				sessionRoots={selection.newSession.sessionRoots}
				presetProjectPath={selection.newSession.draftProjectPath}
				providerConfigs={selection.newSession.providerConfigs}
				stopSignal={selection.newSession.stopSignal}
				onSend={startNewSession}
				onClose={() => selection.newSession.close()}
			/>
		{/if}
	</div>
{/snippet}

{#snippet editorArea()}
	<EditorPanel
		bind:this={editorPanel}
		showing={workbench.centerTab === "editor"}
		rootAvailable={selection.controlledEditorRootAvailable}
		onCloseAllEditors={() => selection.editorSessions.clearActiveEditors()}
		onFileOpened={() => workbench.selectCenterTab("editor")}
	/>
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

	.session-area {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
		min-height: 0;
		overflow: hidden;
		position: relative;
	}

	.conversation-surface-shell {
		display: flex;
		flex: 1;
		height: 100%;
		min-height: 0;
	}

	.conversation-surface-shell[hidden] {
		display: none;
	}

	.conversation-data-isolation {
		display: flex;
		flex: 1;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-2);
		font-size: 13px;
		gap: 10px;
		inset: 0;
		position: absolute;
		z-index: 1;
		background: var(--color-bg);
	}

</style>
