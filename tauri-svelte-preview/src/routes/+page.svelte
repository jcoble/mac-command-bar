<script lang="ts">
	/**
	 * Primary Assembly shell orchestrator.
	 *
	 * Thin shell component: directly composes the real workbench components
	 * (SessionsColumn, RightPanel, DockPanel, GitDiffView, GitHistoryView)
	 * and uses controllers for state management.
	 */
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button/index.js";

	import { PRODUCT_DOCUMENT_TITLE } from "$lib/productIdentity";

	import "$lib/shell/styles/nextTokens.css";
	import "$lib/shell/styles/next.css";
	import "$lib/shell/styles/themeChrome.css";

	import CenterCornerTabs from "$lib/shell/components/CenterCornerTabs.svelte";
	import { registerSessionRowJumpTarget } from "$lib/shell/components/sessionRowJump.ts";
	import PendingFirstMessage from "$lib/shell/components/conversation/PendingFirstMessage.svelte";
	import ConversationSurface from "$lib/shell/components/ConversationSurface.svelte";
	import { sendStructuredMessage } from "$lib/shell/conversation/conversationService";
	import {
		getConversationSession,
		setConversationAttachments,
	} from "$lib/shell/conversation/conversationStore.svelte";
	import DockPanel from "$lib/shell/components/DockPanel.svelte";
	import EditorPanel from "$lib/shell/components/EditorPanel.svelte";
	import GitDiffView from "$lib/shell/components/GitDiffView.svelte";
	import GitHistoryView from "$lib/shell/components/git/GitHistoryView.svelte";
	import RightPanel from "$lib/shell/components/RightPanel.svelte";
	import RightPanelTabs from "$lib/shell/components/RightPanelTabs.svelte";
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
	import { gitPanel } from "$lib/shell/git/gitPanelStore.svelte";
	import { diffPathFor } from "$lib/shell/sessionWorkspaces";
	import { registerSessionHistoryHost } from "$lib/shell/history/sessionHistoryHost";
	import DraftSessionSurface from "$lib/shell/newSession/DraftSessionSurface.svelte";
	import type { ThreadStartRequest } from "$lib/shell/newSession/threadStartFlow";
	import {
		clearWorkbenchNavigation,
		registerWorkbenchNavigation,
	} from "$lib/shell/workbenchNavigation";

	const selection = new SessionSelectionController();
	const workbench = new WorkbenchController();
	// Register before the right-panel children are constructed. History reads
	// this route-owned bridge during its own construction, while the scan itself
	// remains an on-mount async operation with an AbortSignal below.
	const historyHost = registerSessionHistoryHost({
		openOwned: selectSession,
		deleteOwned: (ownedId) => selection.removeSession(ownedId),
	});

	let sessionsColumn = $state<SessionsColumn | null>(null);
	let editorPanel = $state<EditorPanelLifecycle | null>(null);
	let overlays = $state<ShellOverlays | null>(null);
	let openUtility = $state<UtilityId | null>(null);
	let sessionsRailWidth = $state(360);
	let toolsRailWidth = $state(320);
	let routeDisposal: Promise<void> | null = null;

	if (import.meta.hot) {
		import.meta.hot.dispose(() => {
			void disposeRoute();
		});
	}

	$effect(() => {
		selection.setEditorPanel(editorPanel);
	});

	$effect(() => {
		if (typeof document === "undefined") return;
		document.documentElement.style.setProperty("--sessions-rail-width", `${sessionsRailWidth}px`);
		return () => document.documentElement.style.removeProperty("--sessions-rail-width");
	});

	onMount(() => {
		const historyStop = new AbortController();
		const releaseSessionRowJump = registerSessionRowJumpTarget({
			selectSession,
			showCenterPanel: (_ownedId, id) => selectCenterTab(id),
			showSidebarView: (_ownedId, id) => selectRightTab(id),
		});
		registerWorkbenchNavigation({
			showCenterTab: selectCenterTab,
			showRightTab: selectRightTab,
			openDiff: async (request) => {
				if (!selection.activeRootAvailable) return;
				await gitService.showStoredDiff(request.projectRoot, request.relativePath);
			},
			openFileTimeline: async (request) => {
				const ownedId = selection.activeOwnedId;
				const sessionRoot = selection.durableSessionRoot;
				if (!selection.activeRootAvailable || !ownedId || !sessionRoot) return false;
				selectCenterTab("git-history");
				await gitService.showFileHistory(request.projectRoot, request.relativePath);
				if (
					!selection.activeRootAvailable ||
					selection.activeOwnedId !== ownedId ||
					selection.durableSessionRoot !== sessionRoot
				)
					return false;
				return true;
			},
			sendToSession: async (request) => {
				const conversation = getConversationSession(request.ownedId);
				if (!conversation) throw new Error("The selected conversation is not ready");
				if (request.attachments?.length) {
					setConversationAttachments(request.ownedId, [
						...conversation.attachments,
						...request.attachments,
					]);
				}
				await sendStructuredMessage(request.ownedId, request.text);
			},
		});
		void startShell({
			onSelectInitial: async (ownedId, stopSignal) => {
				if (stopSignal.aborted) return;
				await selectSession(ownedId);
				if (stopSignal.aborted) return;
			},
		});
		void historyHost.rescan(historyStop.signal);

		return () => {
			historyStop.abort();
			historyHost.release();
			releaseSessionRowJump();
			clearWorkbenchNavigation();
			void disposeRoute();
		};
	});

	function disposeRoute(): Promise<void> {
		if (routeDisposal) return routeDisposal;
		routeDisposal = disposeRouteOnce();
		return routeDisposal;
	}

	async function disposeRouteOnce(): Promise<void> {
		selection.rememberWorkspaceState(workbench.captureSessionState());
		const selectionDisposal = selection.dispose();
		stopShell();
		await selectionDisposal;
	}

	function handleAskRemoveSession(ownedId: string): void {
		void selection.removeSession(ownedId);
	}

	async function selectSession(ownedId: string): Promise<void> {
		if (selection.activeOwnedId === ownedId && selection.activeWorkspaceSnapshot !== null) return;
		if (selection.activeOwnedId !== null) {
			selection.rememberWorkspaceState(workbench.captureSessionState());
		}
		workbench.beginSessionSwitch();
		await selection.selectSession(ownedId);
		if (selection.activeOwnedId === ownedId) {
			await restoreSelectedWorkbench();
		}
	}

	async function connectSelectedRemote(): Promise<void> {
		const profileId = await selection.connectSelectedRemote();
		if (profileId) await refreshRemoteConnection(profileId);
	}

	async function refreshRemoteConnection(profileId: string): Promise<void> {
		if (!(await selection.refreshRemoteConnection(profileId))) return;
		await restoreSelectedWorkbench();
	}

	async function restoreSelectedWorkbench(): Promise<void> {
		workbench.restoreSessionState(selection.activeWorkspaceSnapshot);
		const storedDiffPath = diffPathFor(selection.activeWorkspaceSnapshot, selection.durableSessionRoot);
		if (storedDiffPath && selection.activeWorkspaceSnapshot?.center?.activePanelId === 'diff') {
			await gitService.showStoredDiff(selection.durableSessionRoot, storedDiffPath);
		}
	}

	function selectCenterTab(id: Parameters<WorkbenchController["selectCenterTab"]>[0]): void {
		workbench.selectCenterTab(id);
		const ownedId = selection.activeOwnedId;
		if (ownedId) selection.persistWorkspaceState(ownedId, workbench.captureSessionState());
	}

	function selectRightTab(id: Parameters<WorkbenchController["selectRightTab"]>[0]): void {
		workbench.selectRightTab(id);
		const ownedId = selection.activeOwnedId;
		if (ownedId) selection.persistWorkspaceState(ownedId, workbench.captureSessionState());
	}

	function openNewSession(): void {
		selection.newSession.open();
		selectCenterTab("session");
	}

	async function startNewSession(request: ThreadStartRequest): Promise<void> {
		await selection.newSession.start(
			request,
			async (ownedId) => {
				await selectSession(ownedId);
			},
			() => {
				selectCenterTab("session");
				selectRightTab("files");
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
		visible={workbench.rightPanelOpen}
		activeId={workbench.rightTab}
		root={selection.durableSessionRoot}
		rootAvailable={selection.activeRootAvailable}
		ownedId={selection.activeOwnedId}
		filesRoot={selection.filesProjectionRoot || selection.durableSessionRoot}
		filesOwnedId={selection.filesProjectionOwnedId}
		expandedPathsByRoot={selection.expandedPathsByRoot}
		onExpandedPathsChange={(root, paths) => selection.rememberExpandedPaths(root, paths)}
		filesInspectionRoot={selection.activeWorkspaceSnapshot?.filesInspectionRoot ?? null}
		sourceControlInspectionRoot={selection.activeWorkspaceSnapshot?.sourceControlInspectionRoot ?? null}
		onFilesInspectionRootChange={(root) => selection.rememberWorkspaceState({ filesInspectionRoot: root })}
		onSourceControlInspectionRootChange={(root) => selection.rememberWorkspaceState({ sourceControlInspectionRoot: root })}
		sourceControlWorkspace={selection.activeWorkspaceSnapshot?.sourceControl}
		historyWorkspace={selection.activeWorkspaceSnapshot?.history}
		onSourceControlWorkspaceChange={(ownedId, sourceControl) => {
			if (selection.activeOwnedId === ownedId) selection.rememberWorkspaceState({ sourceControl });
		}}
		onHistoryWorkspaceChange={(ownedId, history) => {
			if (selection.activeOwnedId === ownedId) selection.rememberWorkspaceState({ history });
		}}
		onBrowserWorkspaceChange={(ownedId, browser) => {
			selection.persistWorkspaceState(ownedId, { browser });
		}}
		checkoutDiscoveryRoots={selection.durableSessionRoot ? [selection.durableSessionRoot] : []}
		onUseSessionCheckout={selection.controlledSession?.agent === "codex" && selection.controlledSession.origin === "app"
			? async (root) => {
					await selection.useSessionCheckout(root);
				}
			: undefined}
	/>
{/snippet}

{#snippet centerTabsArea()}
	<CenterCornerTabs
		activeId={workbench.centerTab}
		onSelect={selectCenterTab}
		rightPanelOpen={workbench.rightPanelOpen}
		onToggleRightPanel={() => workbench.toggleRightPanel()}
	/>
{/snippet}

{#snippet dockArea()}
	<DockPanel
		root={selection.activeRootAvailable ? selection.durableSessionRoot : ""}
		onReset={() => workbench.resetLayout()}
		onProblemsLocationChange={(location) => workbench.applyProblemsLocation(location)}
	/>
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
					showing={workbench.centerTab === "session"}
					pendingFirstMessage={selection.newSession.pendingFirstMessage?.ownedId === selection.chatOwnedId ? selection.newSession.pendingFirstMessage.text : null}
					rootAvailable={selection.activeRootAvailable}
				/>
			</div>
		{/if}
		{#if selection.activeOwnedId && (!selection.hasChatProjection || selection.chatOwnedId !== selection.activeOwnedId)}
			<div class="conversation-data-isolation" class:pending-first-send={selection.newSession.pendingFirstMessage?.ownedId === selection.activeOwnedId} aria-label={selection.selectionError ? "Conversation unavailable" : "Loading conversation"}>
                {#if selection.newSession.pendingFirstMessage?.ownedId === selection.activeOwnedId}
                  <PendingFirstMessage text={selection.newSession.pendingFirstMessage.text} />
				{:else if selection.selectionError}
					<p>{selection.selectionError}</p>
					{#if selection.disconnectedRemoteProfileId || selection.connectingRemote}
						<Button disabled={selection.connectingRemote} onclick={() => void connectSelectedRemote()}>{selection.connectingRemote ? 'Connecting…' : 'Connect'}</Button>
					{/if}
                {:else}<p>Loading conversation…</p>{/if}
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
		ownedId={selection.activeOwnedId}
		showing={workbench.centerTab === "editor"}
		rootAvailable={selection.controlledEditorRootAvailable}
		onCloseAllEditors={() => selection.editorSessions.clearActiveEditors()}
		onFileOpened={() => selectCenterTab("editor")}
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
	<GitHistoryView
		root={selection.activeRootAvailable ? selection.durableSessionRoot : ""}
		rootAvailable={selection.activeRootAvailable}
		historyPath={gitPanel.historyPath}
		showing={workbench.centerTab === "git-history"}
	/>
{/snippet}

<main
	class="next-shell"
	style={`--sessions-rail-width:${sessionsRailWidth}px;--tools-rail-width:${toolsRailWidth}px`}
	oncontextmenu={(event) => {
		const target = event.target instanceof Element ? event.target : null;
		if (target?.closest('input, textarea, [contenteditable="true"]')) return;
		event.preventDefault();
	}}
>
	<div class="window-chrome" data-tauri-drag-region>
		<div class="window-sessions-cap" data-tauri-drag-region></div>
		<div class="window-center-tabs" data-tauri-drag-region>
			{@render centerTabsArea()}
		</div>
		<div class="window-right-tabs" class:open={workbench.rightPanelOpen} data-tauri-drag-region>
			{#if workbench.rightPanelOpen}
				<RightPanelTabs activeId={workbench.rightTab} onSelect={selectRightTab} />
			{/if}
		</div>
	</div>
	<div class="frame-area">
		<ShellFrame
			sessions={sessionsArea}
			tools={toolsArea}
			dock={dockArea}
			center={{
				session: sessionArea,
				editor: editorArea,
				diff: diffArea,
				gitHistory: gitHistoryArea,
			}}
			onSessionsWidthChange={(width) => (sessionsRailWidth = width)}
			onToolsWidthChange={(width) => (toolsRailWidth = width)}
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
		onRemoteConnected={(profileId) => void refreshRemoteConnection(profileId)}
		onResetLayout={() => workbench.resetLayout()}
		message={null}
		onProblemsLocationChange={(location) => workbench.applyProblemsLocation(location)}
		onShowBottomDock={() => workbench.showBottomDock()}
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

	.window-chrome {
		display: grid;
		grid-template-columns: var(--sessions-rail-width) minmax(0, 1fr) var(--tools-rail-width);
		flex: 0 0 calc(var(--center-head-row-height) + 6px);
		align-items: center;
		min-height: 0;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg);
	}

	.window-sessions-cap {
		height: 100%;
		border-right: 1px solid var(--color-border);
	}

	.window-center-tabs {
		min-width: 0;
		justify-self: center;
	}

	.window-right-tabs {
		display: flex;
		width: 100%;
		height: 100%;
		min-width: 0;
		align-items: center;
		box-sizing: border-box;
	}

	.window-right-tabs.open {
		border-left: 1px solid var(--color-border);
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

	.conversation-data-isolation.pending-first-send { align-items: flex-start; justify-content: flex-end; }
</style>
