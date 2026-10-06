<script lang="ts">
	/**
	 * Primary Assembly shell orchestrator.
	 *
	 * Thin shell component: directly composes the real workbench components
	 * (SessionsColumn, TopTabRow, the tab pane's surfaces, the right drawer,
	 * DockPanel) and uses controllers for state management. The top tab row's
	 * active tab (`topTabs`) decides what the pane shows; one effect below
	 * hands that choice to the editor and browser stores.
	 */
	import { onMount, untrack } from "svelte";
	import { Button } from "$lib/components/ui/button/index.js";

	import { PRODUCT_DOCUMENT_TITLE } from "$lib/productIdentity";

	import "$lib/shell/styles/nextTokens.css";
	import "$lib/shell/styles/next.css";
	import "$lib/shell/styles/themeChrome.css";

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
	import PullRequestWorkspace from "$lib/shell/components/github/PullRequestWorkspace.svelte";
	import RightPanel from "$lib/shell/components/RightPanel.svelte";
	import RightPanelTabs from "$lib/shell/components/RightPanelTabs.svelte";
	import TopTabRow, { type TopTabView } from "$lib/shell/components/TopTabRow.svelte";
	import BrowserPanel from "$lib/shell/panels/browser/BrowserPanel.svelte";
	import { pullRequestSelection } from "$lib/shell/components/github/pullRequestSelection.svelte";
	import { topTabs } from "$lib/shell/layout/topTabs.svelte";
	import { parseTopTabKey } from "$lib/shell/layout/topTabsOps";
	import { editorState, pinEditorFile } from "$lib/shell/editor/editorStore.svelte";
	import {
		browser,
		captureBrowserState,
		closeBrowserPageTab,
		createBrowserPageTab,
		queueBrowserPageTab,
		selectBrowserPageTab,
	} from "$lib/shell/browser/browserStore.svelte";
	import SessionsColumn from "$lib/shell/components/SessionsColumn.svelte";
	import ShellFrame from "$lib/shell/components/ShellFrame.svelte";
	import ShellOverlays from "$lib/shell/components/ShellOverlays.svelte";
	import { parseRemoteWorkspacePath } from "$lib/workspacePaths";
	import UtilityStrip from "$lib/shell/components/UtilityStrip.svelte";
	import type { UtilityId } from "$lib/shell/components/utilityStrip";

	import { SessionSelectionController } from "$lib/shell/controllers/sessionSelectionController.svelte";
	import { ownedSessionMetaForBackend } from "$lib/shell/ownedSessions";
	import { setOwnedSessionStatus, updateOwnedSession } from "$lib/shell/stores/sessionRailStore.svelte";
	import { updateAgentConversationSessionMetaFromTauri } from "$lib/tauriSource";
	import { WorkbenchController } from "$lib/shell/controllers/workbenchController.svelte";
	import { startShell, stopShell } from "$lib/shell/controllers/shellStartup";
	import { gitService } from "$lib/shell/git/gitService";
	import { gitPanel } from "$lib/shell/git/gitPanelStore.svelte";
	import type { CenterTabId, OpenPullRequestDiffRequest } from "$lib/shell/workbenchNavigation";
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
	let editorPanel = $state<EditorPanel | null>(null);
	let overlays = $state<ShellOverlays | null>(null);
	let openUtility = $state<UtilityId | null>(null);
	let sessionsRailWidth = $state(360);
	// Zero until the frame reports the pane's laid-out width: the browser may
	// place its native view only once the pane really has a size.
	let toolsRailWidth = $state(0);
	let pullRequestDiff = $state<OpenPullRequestDiffRequest | null>(null);
	let routeDisposal: Promise<void> | null = null;

	async function changeSessionStatus(ownedId: string, status: 'working' | 'done' | 'settled'): Promise<void> {
		const before = selection.railOwned.find((session) => session.ownedId === ownedId);
		const updated = setOwnedSessionStatus(ownedId, status, new Date());
		if (!before || !updated) return;
		try {
			await updateAgentConversationSessionMetaFromTauri({
				ownedId,
				model: null,
				effort: null,
				meta: ownedSessionMetaForBackend(updated)
			});
		} catch (error) {
			updateOwnedSession(ownedId, { completedAt: before.completedAt, settledAt: before.settledAt });
			console.error('Could not change session status', error);
		}
	}

	if (import.meta.hot) {
		import.meta.hot.dispose(() => {
			void disposeRoute();
		});
	}

	$effect(() => {
		selection.setEditorPanel(editorPanel);
	});

	/** The tab pane is in the frame whenever the top row has a tab. */
	const paneShowing = $derived(topTabs.row.length > 0);
	const browserRoot = $derived(selection.newSession.draftOpen ? "" : selection.durableSessionRoot);

	const topTabViews = $derived.by(() => {
		const browserTabs = captureBrowserState().tabs;
		return topTabs.row.flatMap((key): TopTabView[] => {
			const ref = parseTopTabKey(key);
			if (!ref) return [];
			if (ref.kind === "editor") {
				const file = editorState.openFiles.find((entry) => entry.path === ref.id);
				if (!file) return [];
				const detail = [file.relativePath, file.path, file.dirty ? "Unsaved changes" : ""];
				return [{ key, kind: "editor", label: file.fileName, detail: detail.filter(Boolean).join("\n"), path: file.path, dirty: Boolean(file.dirty), preview: Boolean(file.previewTab) }];
			}
			if (ref.kind === "browser") {
				const tab = browserTabs.find((entry) => entry.id === ref.id);
				const label = tab?.title || "New browser tab";
				return [{ key, kind: "browser", label, detail: [label, tab?.url ?? ""].filter(Boolean).join("\n") }];
			}
			if (ref.kind === "diff") return [{ key, kind: "diff", label: "Changes", detail: gitPanel.selectedPath || "Working tree changes", additions: gitPanel.status?.additions ?? 0, deletions: gitPanel.status?.deletions ?? 0 }];
			if (ref.kind === "git-history") return [{ key, kind: "git-history", label: "History", detail: gitPanel.historyPath || "Whole repository" }];
			const pullRequest = pullRequestSelection.selected;
			return [
				pullRequest
					? { key, kind: "pull-requests", label: `PR #${pullRequest.number}`, detail: `PR #${pullRequest.number} — ${pullRequest.title}\n${pullRequest.headBranch} → ${pullRequest.baseBranch}` }
					: { key, kind: "pull-requests", label: "Pull requests", detail: "Pull requests" },
			];
		});
	});

	// The pane comes and goes with its tabs, but not mid-switch: the next
	// session's tabs arrive in `restoreSessionState`, and removing then re-adding
	// the region in between would flash.
	$effect(() => {
		const present = paneShowing;
		if (workbench.switching) return;
		untrack(() => workbench.setPanePresent(present));
	});

	$effect(() => {
		const kind = paneShowing ? topTabs.activeKind : "session";
		untrack(() => workbench.paneShown(kind ?? "session"));
	});

	// The active top tab is the truth for what is showing. Whatever made it
	// change (a click, a close, a restore), the owning store follows.
	$effect(() => {
		if (workbench.switching) return;
		const ref = topTabs.activeKey ? parseTopTabKey(topTabs.activeKey) : null;
		const panel = editorPanel;
		if (!ref) return;
		untrack(() => {
			if (ref.kind === "editor" && editorState.activePath !== ref.id) panel?.selectFile(ref.id);
			else if (ref.kind === "browser" && captureBrowserState().activeTabId !== ref.id) selectBrowserPageTab(ref.id);
		});
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
				pullRequestDiff = null;
				await gitService.showStoredDiff(request.projectRoot, request.relativePath);
			},
			openPullRequestDiff: (request) => {
				pullRequestDiff = request;
				workbench.setDiffMode('side-by-side');
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
		pullRequestDiff = null;
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
		if (storedDiffPath && selection.activeWorkspaceSnapshot?.topTabs?.order.includes('diff')) {
			await gitService.showStoredDiff(selection.durableSessionRoot, storedDiffPath);
		}
	}

	function persistTabs(): void {
		const ownedId = selection.activeOwnedId;
		if (ownedId) selection.persistWorkspaceState(ownedId, workbench.captureSessionState());
	}

	function selectTopTab(key: string): void {
		topTabs.select(key);
		persistTabs();
	}

	function closeTopTab(key: string): void {
		const ref = parseTopTabKey(key);
		if (!ref) return;
		if (ref.kind === "editor") {
			if (editorState.openFiles.find((file) => file.path === ref.id)?.dirty) {
				// The unsaved-changes dialog must not open over a live browser page,
				// so the file comes to the front first. The row drops its tab once
				// the dialog really closes the file.
				topTabs.select(key);
				editorPanel?.selectFile(ref.id);
				editorPanel?.requestCloseFile(ref.id);
				persistTabs();
				return;
			}
			topTabs.forget(key);
			editorPanel?.requestCloseFile(ref.id);
		} else {
			topTabs.forget(key);
			if (ref.kind === "browser") closeBrowserPageTab(ref.id);
			else if (ref.kind === "diff") {
				pullRequestDiff = null;
				gitService.clearSelection();
			} else if (ref.kind === "pull-requests") pullRequestSelection.selected = null;
		}
		persistTabs();
	}

	function openNewBrowserTab(): void {
		if (selection.activeOwnedId) browser.workspace.ownedId = selection.activeOwnedId;
		const id = browser.workspace.activated ? createBrowserPageTab() : queueBrowserPageTab();
		if (!id) return;
		topTabs.open(`browser:${id}`);
		persistTabs();
	}

	function selectCenterTab(id: CenterTabId): void {
		if (id === "session") {
			if (workbench.expanded) workbench.setExpanded(false);
			return;
		}
		if (id === "editor") {
			if (editorState.activePath) topTabs.open(`editor:${editorState.activePath}`);
		} else if (id === "browser") {
			const active = captureBrowserState().activeTabId;
			if (!active) return openNewBrowserTab();
			topTabs.open(`browser:${active}`);
		} else {
			topTabs.open(id);
		}
		persistTabs();
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

	async function startNewSession(request: ThreadStartRequest, images: File[]): Promise<void> {
		await selection.newSession.start(
			request,
			images,
			async (ownedId) => {
				await selectSession(ownedId);
			},
			() => {
				selectCenterTab("session");
				workbench.rightTab = "files";
			},
			(ownedId, ids) => selection.persistConversationAttachmentIds(ownedId, ids),
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
				onComplete={(ownedId) => void changeSessionStatus(ownedId, 'done')}
				onReopen={(ownedId) => void changeSessionStatus(ownedId, 'working')}
				onSettle={(ownedId) => void changeSessionStatus(ownedId, 'settled')}
				onUnsettle={(ownedId) => void changeSessionStatus(ownedId, 'done')}
			/>
		</div>
	</div>
{/snippet}

{#snippet drawerArea()}
	<RightPanel
		visible={workbench.rightPanelOpen}
		activeId={workbench.rightTab}
		root={selection.newSession.draftOpen ? "" : selection.durableSessionRoot}
		rootAvailable={selection.activeRootAvailable}
		ownedId={selection.activeOwnedId}
		filesRoot={selection.filesProjectionRoot}
		filesOwnedId={selection.filesProjectionOwnedId}
		expandedPathsByRoot={selection.expandedPathsByRoot}
		onExpandedPathsChange={(ownedId, root, paths) => selection.rememberExpandedPaths(ownedId, root, paths)}
		filesInspectionRoot={selection.activeWorkspaceSnapshot?.filesInspectionRoot ?? null}
		sourceControlInspectionRoot={selection.activeWorkspaceSnapshot?.sourceControlInspectionRoot ?? null}
		onFilesInspectionRootChange={(root) => selection.rememberWorkspaceState({ filesInspectionRoot: root })}
		onSourceControlInspectionRootChange={(root) => selection.rememberWorkspaceState({ sourceControlInspectionRoot: root })}
		sourceControlWorkspace={selection.activeWorkspaceSnapshot?.sourceControl}
		onSourceControlWorkspaceChange={(ownedId, sourceControl) => {
			if (selection.activeOwnedId === ownedId) selection.rememberWorkspaceState({ sourceControl });
		}}
		tasksView={selection.activeWorkspaceSnapshot?.tasksView}
		onTasksViewChange={(ownedId, tasksView) => selection.persistWorkspaceState(ownedId, { tasksView })}
		checkoutDiscoveryRoots={selection.filesProjectionRoot ? [selection.filesProjectionRoot] : []}
		onUseSessionCheckout={selection.controlledSession?.agent === "codex" && selection.controlledSession.origin === "app"
			? async (root) => {
					await selection.useSessionCheckout(root);
				}
			: undefined}
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
					pendingFirstMessage={selection.newSession.pendingFirstMessage?.ownedId === selection.chatOwnedId ? selection.newSession.pendingFirstMessage.text : null}
					rootAvailable={selection.activeRootAvailable}
					onPersistAttachmentIds={(ownedId, ids) => selection.persistConversationAttachmentIds(ownedId, ids)}
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
				stopSignal={selection.newSession.stopSignal}
				onSend={startNewSession}
				onClose={() => selection.newSession.close()}
			/>
		{/if}
	</div>
{/snippet}

{#snippet toolsArea()}
	<!-- The tab pane: one surface per kind, only the active one displayed.
	     Editor and browser stay mounted (their lifecycles are per session);
	     History and Pull requests mount while their tab exists; the diff only
	     while it is in front. -->
	<div class="pane-stack">
		<div class="pane-body" class:showing={paneShowing && topTabs.activeKind === "editor"}>
			<EditorPanel
				bind:this={editorPanel}
				ownedId={selection.activeOwnedId}
				showing={paneShowing && topTabs.activeKind === "editor"}
				rootAvailable={selection.controlledEditorRootAvailable}
				onCloseAllEditors={() => selection.editorSessions.clearActiveEditors()}
				onFileOpened={() => selectCenterTab("editor")}
			/>
		</div>
		<div class="pane-body" class:showing={paneShowing && topTabs.activeKind === "browser"}>
			<!-- The page is a native view painted above every DOM layer, so it
			     steps aside (hidden, still live) while Settings covers the window. -->
			<BrowserPanel
				visible={paneShowing && topTabs.activeKind === "browser" && toolsRailWidth > 0 && !overlays?.settingsOpen()}
				panelOpen={paneShowing}
				root={browserRoot}
				ownedId={selection.activeOwnedId}
				onWorkspaceChange={(ownedId, browser) => {
					selection.persistWorkspaceState(ownedId, { browser });
				}}
			/>
		</div>
		{#if topTabs.row.includes("git-history")}
			<div class="pane-body" class:showing={topTabs.activeKind === "git-history"}>
				<GitHistoryView
					root={selection.activeRootAvailable ? selection.durableSessionRoot : ""}
					rootAvailable={selection.activeRootAvailable}
					historyPath={gitPanel.historyPath}
					showing={topTabs.activeKind === "git-history"}
				/>
			</div>
		{/if}
		{#if topTabs.row.includes("pull-requests")}
			<div class="pane-body" class:showing={topTabs.activeKind === "pull-requests"}>
				<PullRequestWorkspace showing={topTabs.activeKind === "pull-requests"} />
			</div>
		{/if}
		{#if topTabs.activeKind === "diff"}
			<div class="pane-body showing">
				<GitDiffView
					showing={true}
					rootAvailable={pullRequestDiff !== null || selection.activeRootAvailable}
					pullRequestDiff={pullRequestDiff}
					mode={workbench.diffMode}
					onModeChange={(mode) => workbench.setDiffMode(mode)}
				/>
			</div>
		{/if}
	</div>
{/snippet}

<main
	class="next-shell"
	style:--sessions-rail-width={`${sessionsRailWidth}px`}
	style:--tools-rail-width={`${toolsRailWidth}px`}
	style:--drawer-width={workbench.rightPanelOpen ? "326px" : "0px"}
	oncontextmenu={(event) => {
		const target = event.target instanceof Element ? event.target : null;
		if (target?.closest('input, textarea, [contenteditable="true"]')) return;
		event.preventDefault();
	}}
>
	<div class="window-chrome" data-tauri-drag-region>
		<div class="window-sessions-cap" data-tauri-drag-region></div>
		<TopTabRow
			tabs={topTabViews}
			activeKey={topTabs.activeKey}
			expanded={workbench.expanded}
			paneOpen={paneShowing && toolsRailWidth > 0}
			chatTitle={selection.railOwned.find((s) => s.ownedId === selection.activeOwnedId)?.title || "Chat"}
			drawerOpen={workbench.rightPanelOpen}
			canOpenBrowser={Boolean(browserRoot)}
			onSelect={selectTopTab}
			onClose={closeTopTab}
			onSelectChat={() => selectCenterTab("session")}
			onNewBrowserTab={openNewBrowserTab}
			onToggleExpanded={() => workbench.setExpanded(!workbench.expanded)}
			onToggleDrawer={() => workbench.toggleRightPanel()}
			editorActions={{
				pin: pinEditorFile,
				closeOthers: (path) => editorPanel?.closeOtherFiles(path),
				closeSaved: () => editorPanel?.closeSavedFiles(),
				timeline: (path) => editorPanel?.openTimeline(path),
			}}
		/>
	</div>
	<!-- The drawer pushes rather than floats: the frame narrows beside it, so
	     nothing is ever drawn over the native browser page. -->
	<div class="frame-area">
		<div class="frame-main">
			<ShellFrame
				sessions={sessionsArea}
				center={sessionArea}
				tools={toolsArea}
				dock={dockArea}
				onSessionsWidthChange={(width) => (sessionsRailWidth = width)}
				onToolsWidthChange={(width) => (toolsRailWidth = width)}
				onReady={(controls) => workbench.onFrameReady(controls)}
			/>
		</div>
		<!-- Closing hides the drawer rather than unmounting it (measured: unmounting
		     saved no memory and made reopening slower). Hidden, it takes no width. -->
		<aside class="right-drawer" class:open={workbench.rightPanelOpen} aria-label="Tools" aria-hidden={!workbench.rightPanelOpen}>
			<RightPanelTabs
				activeId={workbench.rightTab}
				extraId={workbench.rightExtraTab}
				onSelect={selectRightTab}
				onClose={() => workbench.setRightPanelOpen(false)}
			/>
			<div class="right-drawer-body">
				{@render drawerArea()}
			</div>
		</aside>
	</div>

	<UtilityStrip
		{openUtility}
		onOpenUtility={(id, anchor) => overlays?.openUtility(id, anchor)}
		onOpenSettings={() => overlays?.openSettings()}
		location={browserRoot ? (parseRemoteWorkspacePath(browserRoot) ? "Remote" : "Local Mac") : null}
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
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
		position: relative;
	}

	.frame-main {
		flex: 1 1 auto;
		min-width: 0;
		height: 100%;
	}

	/* The drawer is a card in the same gutter as the frame's own cards; its
	   320px plus the 6px gutter is the --drawer-width the chrome row adds. The
	   column takes its full width at once (an animated width would lay the
	   whole frame out again every frame); only the card slides in, and the
	   animation ends. Closed, it is display:none and releases its width. */
	.right-drawer {
		display: none;
		flex: 0 0 320px;
		flex-direction: column;
		min-height: 0;
		margin: 6px 6px 6px 0;
		border-radius: var(--radius-sm);
		overflow: hidden;
		background: var(--panel-fade), var(--color-surface);
		background-repeat: no-repeat;
	}

	.right-drawer.open {
		display: flex;
		animation: right-drawer-in 160ms ease-out;
	}

	@keyframes right-drawer-in {
		from {
			opacity: 0;
			transform: translateX(16px);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.right-drawer.open {
			animation: none;
		}
	}

	.right-drawer-body {
		flex: 1 1 auto;
		min-height: 0;
	}

	.pane-stack {
		position: relative;
		height: 100%;
		width: 100%;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}

	.pane-body {
		position: absolute;
		inset: 0;
		display: none;
		overflow: hidden;
	}

	.pane-body.showing {
		display: block;
	}

	.window-chrome {
		display: grid;
		grid-template-columns: var(--sessions-rail-width) minmax(0, 1fr);
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
		/* Covers the card it sits in, so it repaints the card's own face. */
		background: var(--panel-fade), var(--color-surface);
		background-repeat: no-repeat;
	}

	.conversation-data-isolation.pending-first-send { align-items: flex-start; justify-content: flex-end; }
</style>
