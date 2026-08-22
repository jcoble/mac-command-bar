<script lang="ts">
	/**
	 * /next — the shell orchestrator. Thin by construction: it owns the terminal
	 * sessions and nothing else. Every other panel brings its own state and its
	 * own loader; this page only says which project they are pointed at and when
	 * they may start — see `shellPanels.ts` and `panelActivation.ts`.
	 *
	 * IO lives **only in explicit functions**, never in an `$effect`. Launch IO is
	 * still just the rail: list surviving PTYs, reconcile them against the stored
	 * owned sessions, re-attach them (live AND tombstones), scan for resumable
	 * agents. `hydrateOwned` runs AFTER `reconcileOwnedSessions`.
	 */
	import { onMount, tick } from "svelte";

	import { PRODUCT_DOCUMENT_TITLE } from "$lib/productIdentity";

	import "$lib/shell/styles/nextTokens.css";
	/* Tailwind + the shadcn component variables. Imported HERE and nowhere else:
     the old shell shares `app.css` with this page and must keep rendering
     exactly as it does today, so this file must never reach that route. */
	import "$lib/shell/styles/next.css";
	/* Four colours the dock needs that the shared token file has no name for.
     A stylesheet rather than something the theme service sets, because the dock
     is painted on the first frame, before any theme has been applied. */
	import "$lib/shell/styles/themeChrome.css";

	import { hydrateSettings, settings, type ProblemsLocation } from "$lib/settingsStore.svelte";
	import {
		captureBrowserState,
		clearBrowserUrl,
		openBrowserUrl,
		releaseBrowserWorkspace,
		restoreBrowserState,
	} from "$lib/shell/browser/browserStore.svelte.ts";
	import CenterCornerTabs from "$lib/shell/components/CenterCornerTabs.svelte";
	import ConversationSurface from "$lib/shell/components/ConversationSurface.svelte";
	import DockPanel from "$lib/shell/components/DockPanel.svelte";
	import EditorPanel from "$lib/shell/components/EditorPanel.svelte";
	import GitHistoryView from "$lib/shell/components/git/GitHistoryView.svelte";
	import GitDiffView from "$lib/shell/components/GitDiffView.svelte";
	import RightPanel from "$lib/shell/components/RightPanel.svelte";
	import { registerSessionRowJumpTarget } from "$lib/shell/components/sessionRowJump";
	import SessionsColumn from "$lib/shell/components/SessionsColumn.svelte";
	import ShellFrame from "$lib/shell/components/ShellFrame.svelte";
	import ShellOverlays from "$lib/shell/components/ShellOverlays.svelte";
	import type { UtilityId } from "$lib/shell/components/utilityStrip";
	import UtilityStrip from "$lib/shell/components/UtilityStrip.svelte";
	import { rememberedAgentConfigChoice } from "$lib/shell/conversation/agentConfigMemory";
	import {
		commitConversationHandoff,
		changeStructuredConversationCheckout,
		ensureStructuredConversation,
		flushConversationSessionDraft,
		loadConversationForRead,
		loadConversationSessionDraft,
		prepareConversationHandoff,
		releaseConversationForRead,
		rollbackConversationHandoff,
		sendStructuredMessage,
		startConversationEvents,
		startConversationTerminalProjection,
		stopConversationEvents,
		stopConversationTerminalProjection,
	} from "$lib/shell/conversation/conversationService";
	import {
		captureConversationWorkspace,
		conversationSessions,
		ensureConversationSession,
		getConversationSession,
		removeConversationSession,
		restoreConversationWorkspace,
		setConversationAttachments,
		setConversationDraft,
		setConversationMode,
	} from "$lib/shell/conversation/conversationStore.svelte";
	import type {
		AgentConversationHandoffDirection,
		AgentConversationHandoffMode,
		AgentConversationProvider,
	} from "$lib/shell/conversation/conversationTypes";
	import { countInvoke } from "$lib/shell/devInvokeCounter.svelte";
	import { editorState, resetEditorState, restoreEditorFiles } from "$lib/shell/editor/editorStore.svelte";
	import {
		configureExtensionApiProbeRuntime,
		disposeExtensionApiProbeResources,
		disposeExtensionApiProbeRuntime,
		onExtensionApiProbeObservation,
		setExtensionApiProbeWorkspace,
		type ExtensionApiProbeObservation,
	} from "$lib/shell/editor/extensionApiProbeController";
	import {
		setLanguageServerEnabled,
		setLanguageServersEnabled,
		sourceIntelligence,
	} from "$lib/shell/editor/sourceIntelligence";
	import { canonicalPath, explorer, selectPath, setScrollTop } from "$lib/shell/explorer/explorerStore.svelte";
	import { gitPanel } from "$lib/shell/git/gitPanelStore.svelte";
	import { gitCommitFilesService } from "$lib/shell/git/gitCommitFilesService";
	import { gitService } from "$lib/shell/git/gitService";
	import type { CenterDockSnapshot } from "$lib/shell/layout/centerDock";
	import {
		CENTER_MIN_WIDTH,
		DOCK_HEIGHT,
		SESSIONS_MAX_WIDTH,
		SESSIONS_MIN_WIDTH,
		SESSIONS_STRIP_WIDTH,
		SESSIONS_WIDTH,
		TOOLS_MAX_WIDTH,
		TOOLS_MIN_WIDTH,
		type RegionHeightLimits,
		type RegionWidthLimits,
		type ShellRegionId,
	} from "$lib/shell/layout/frame";
	import { isSidebarViewId, type SidebarViewId } from "$lib/shell/layout/sidebarViews";
	import {
		DEFAULT_CENTER_TAB,
		DEFAULT_RIGHT_TAB,
		isCenterTabId as isStoredCenterTabId,
	} from "$lib/shell/layout/workbenchTabs";
	import DraftSessionSurface from "$lib/shell/newSession/DraftSessionSurface.svelte";
	import { rememberLastUsed } from "$lib/shell/newSession/projectRootsStore.svelte";
	import type { ThreadStartProviderConfig, ThreadStartRequest } from "$lib/shell/newSession/threadStartFlow.ts";
	import { deriveThreadStartProjects } from "$lib/shell/newSession/threadStartFlow.ts";
	import {
		adoptAgentSession,
		createFreshSession,
		ownedSessionFromBackend,
		ownedSessionMetaForBackend,
		reconcileOwnedSessions,
	} from "$lib/shell/ownedSessions";
	import { setUnavailableOpenFileRoot } from "$lib/shell/openFileBus";
	import type { SessionLibraryRecord } from "$lib/shell/sessionLibrary/sessionLibraryModel";
	import {
		createSessionLibraryService,
		registerSessionLibraryHost,
	} from "$lib/shell/sessionLibrary/sessionLibraryService";
	import {
		captureWorkspace,
		DEFAULT_DIFF_MODE,
		diffPathFor,
		planWorkspaceRestore,
		type CheckoutScope,
		type DiffMode,
		type SessionWorkspaceSnapshot,
	} from "$lib/shell/sessionWorkspaces";
	import { registerShellCommands } from "$lib/shell/shellCommands";
	import { readSelection, shellPanels } from "$lib/shell/shellPanels";
	import {
		clearStackHandlers,
		noteSessionRemoved,
		noteTerminalExit,
		registerStackHandlers,
		type StackStartRequest,
	} from "$lib/shell/stacks/stackService";
	import { recordStackStart, stackIdForOwnedId } from "$lib/shell/stacks/stackStore.svelte";
	import {
		addOwnedSession,
		completeOwnedSession,
		hydrateOwned,
		rail,
		removeOwnedSession,
		reopenOwnedSession,
		setActiveOwned,
		setAvailable,
		updateOwnedSession,
		type SessionProjection,
	} from "$lib/shell/stores/sessionRailStore.svelte";
	import { createTerminalService, tauriTerminalBackend } from "$lib/shell/terminalService";
	import { applyStoredFonts, clearFonts } from "$lib/shell/themes/fontService";
	import { applyStoredTheme, clearTheme } from "$lib/shell/themes/themeService";
	import {
		clearWorkbenchNavigation,
		registerWorkbenchNavigation,
		type CenterTabId,
		type RightTabId,
	} from "$lib/shell/workbenchNavigation";
	import { loadXtermModules, makeTerminalView } from "$lib/shell/xtermFactory";
	import {
		clearAgentConversationWorkspaceEditorsFromTauri,
		clearAgentConversationWorkspaceTabsFromTauri,
		deleteAgentConversationSessionFromTauri,
		readAgentConversationWorkspaceFromTauri,
		listAgentConversationSessionsFromTauri,
		listAgentSessionsForProjectFromTauri,
		listAgentSessionsFromLocalBridge,
		listAgentSessionsFromTauri,
		openMainDevtoolsFromTauri,
		readAssemblySettingFromTauri,
		updateAgentConversationSessionMetaFromTauri,
		validateProjectRootFromTauri,
		writeAssemblySettingFromTauri,
		writeAgentConversationWorkspaceFromTauri,
		type AgentSession,
	} from "$lib/tauriSource";

	/** Hosts mount before the service finishes async init: parked here, drained later. */
	const pendingHosts = new Map<string, HTMLElement>();
	/** Sessions with a restart already under way. Added before the first await, so
	 * a second click on "Start again" cannot get past it while the first click is
	 * still waiting on the backend — two starts for one row would leave two agents
	 * resuming the same conversation, with only one of them reachable. */
	const restarting = new Set<string>();
	/** ptySessionId -> the PTY's REAL grid (launch `backend.list()`), fed to
	 * `adoptExisting`: a HIDDEN host cannot be measured, so without it a survivor's
	 * view keeps 80x24 and wraps its replay wrong. */
	const livePtySizes = new Map<string, { cols: number; rows: number }>();

	let activeWorkspaceSnapshot = $state<SessionWorkspaceSnapshot | null>(null);
	let diffMode = $state<DiffMode>(DEFAULT_DIFF_MODE);
	let workspaceRestoreGeneration = 0;
	let sessionSelectionGeneration = 0;
	let sessionProjectionOwner: "selection" | "checkout" | null = null;
	let workspaceSaveTimer: ReturnType<typeof setTimeout> | null = null;
	let workspaceWriteQueue: Promise<void> = Promise.resolve();
	let workspaceAutosaveEnabled = false;
	let clearAllEditorsInFlight = false;
	const sessionProjection = $derived<SessionProjection>({
		activeOwnedId: rail.activeOwnedId,
		rail: rail.owned,
		activeConversation: rail.activeOwnedId === null
			? null
			: conversationSessions[rail.activeOwnedId] ?? null,
		activeWorkspace: activeWorkspaceSnapshot,
	});
	const checkoutScope = $derived.by<CheckoutScope>(() => {
		const durableSessionRoot = activeRootAvailable ? readSelection().root : "";
		const filesInspectionRoot = sessionProjection.activeWorkspace?.filesInspectionRoot ?? null;
		const gitInspectionRoot = sessionProjection.activeWorkspace?.sourceControlInspectionRoot ?? null;
		const canonicalSessionRoot = canonicalPath(durableSessionRoot);
		return {
			durableSessionRoot,
			filesInspectionRoot,
			gitInspectionRoot,
			filesReadOnly: Boolean(filesInspectionRoot)
				&& canonicalPath(filesInspectionRoot ?? "") !== canonicalSessionRoot,
			gitReadOnly: Boolean(gitInspectionRoot)
				&& canonicalPath(gitInspectionRoot ?? "") !== canonicalSessionRoot,
			workspaceRestoreGeneration,
			sessionSelectionGeneration,
			conversationGeneration: sessionProjection.activeConversation?.generation ?? null,
		};
	});
	let service: ReturnType<typeof createTerminalService> | null = null;
	let extensionApiProbeTerminalHost: HTMLElement | null = null;
	let extensionApiProbeObservation = $state<ExtensionApiProbeObservation | null>(null);
	let disposed = false;
	let activeRootAvailable = $state(true);
	let frameControls: {
		resetLayout(): void;
		showCenterPanel(id: string): void;
		captureCenterLayout(): CenterDockSnapshot | null;
		restoreCenterLayout(snapshot: CenterDockSnapshot | null | undefined): void;
		setRegionWidth(id: ShellRegionId, width: number, limits?: RegionWidthLimits): void;
		setRegionHeight(id: ShellRegionId, height: number, limits?: RegionHeightLimits): void;
		setDockPresent(present: boolean): void;
		setRegionLimits(id: ShellRegionId, limits: RegionWidthLimits): void;
		regionWidth(id: ShellRegionId): number | null;
	} | null = null;
	let editorPanel: {
		captureViewStates(paths: readonly string[]): Record<string, object>;
		workspaceOwnedPaths(): string[];
		restoreViewStates(files: readonly { path: string; viewState?: object }[]): void;
		releaseSessionResources(paths: readonly string[]): void;
		requestCloseActive(): void;
	} | null = null;
	/** Which panel the right column is showing, and which surface the center
	 * pane is on. The PAGE owns both, because both are remembered per session and
	 * the sessions are the page's. Read for the session on screen, written the
	 * moment either is clicked. */
	let rightTab = $state<RightTabId>(DEFAULT_RIGHT_TAB);
	let centerTab = $state<CenterTabId>(DEFAULT_CENTER_TAB);
	let restoringTabs = false;
	/** Which of the two bottom-strip surfaces is open, so its button reads as on. */
	let openUtility = $state<UtilityId | null>(null);
	/** The overlay layer, for opening the dialogs and surfaces it owns. */
	let overlays: {
		openSettings(): void;
		openUtility(id: UtilityId, anchor: { left: number; top: number; width: number; height: number }): void;
	} | null = null;
	/**
	 * The draft session, if one is open. It is NOT a session: no row on the rail,
	 * no conversation, nothing in the backend. It is the Session tab showing an
	 * empty transcript and a composer, and it becomes a session on the first send
	 * and at no other moment. Switching to another session throws it away.
	 */
	let draftOpen = $state(false);
	let draftProjectPath = $state<string | null>(null);
	/** The conversation surface, for putting the caret in its prompt box when a
	 * panel hands the composer something. */
	let conversationSurface: { focusComposer(): void } | null = null;

	function mostRecentProjectPath(): string | undefined {
		const active = rail.owned.find((session) => session.ownedId === rail.activeOwnedId);
		if (active) return active.projectPath?.trim() || active.cwd.trim() || undefined;
		const activityTime = (value: string | null): number => {
			const parsed = Date.parse(value ?? "");
			return Number.isFinite(parsed) ? parsed : 0;
		};
		const recent = [...rail.owned].sort(
			(left, right) => activityTime(right.lastActivity) - activityTime(left.lastActivity),
		)[0];
		return recent?.projectPath?.trim() || recent?.cwd.trim() || undefined;
	}

	/** Open a draft in the Session tab, ready to type in. Nothing is created. */
	function openNewSessionForProject(projectPath?: string): void {
		draftProjectPath = projectPath?.trim() || mostRecentProjectPath() || null;
		draftOpen = true;
		selectCenterTab("session");
	}

	function openNewSession(): void {
		openNewSessionForProject();
	}
	/** Provider settings already fetched for existing structured sessions. A
	 * fresh pane consumes these snapshots without starting a hidden session just
	 * to populate its menus. */
	function providerConfigsForNewSession(): ThreadStartProviderConfig[] {
		return (["codex", "claude", "antigravity"] as const).map((provider) => {
			const existing = Object.values(conversationSessions).find((session) => session.provider === provider);
			const config = existing?.agentConfig;
			// The lists come from any session that has them; the choice is the
			// reader's last one, not whichever session happened to be found.
			const chosen = rememberedAgentConfigChoice(provider);
			return {
				provider,
				model: chosen?.model ?? config?.model ?? null,
				availableModels: config?.availableModels ?? [],
				reasoningEffort: chosen?.reasoningEffort ?? config?.reasoningEffort ?? null,
				availableEfforts: config?.availableEfforts ?? [],
				approvalPolicy: chosen?.approvalPolicy ?? config?.approvalPolicy ?? null,
				availableApprovalPolicies: config?.availableApprovalPolicies ?? [],
			};
		});
	}
	/** The sessions column, for opening its "Find a session" drawer from the
	 * context panel's "Search all sessions" link. */
	let sessionsColumn: { openFinder(): void } | null = null;
	let refitScheduled = false;
	/** Its own state, NOT `rail.error`: ShellFrame mounts before this page's
	 * start-up, and `scanRail` clears `rail.error` — which would erase a mount
	 * failure on every launch and leave a blank shell with no message. */
	let layoutError = $state<string | null>(null);

	/**
	 * Dockview says a center surface came to the front — because the corner tabs
	 * asked for it, or because the dock restored one at launch. Either way the
	 * page adopts the answer, so the tabs and the dock can never disagree.
	 */
	function handleCenterPanelShown(id: string): void {
		if (isCenterTabId(id)) adoptCenterTab(id);
		shellPanels.panelShown(id);
		syncGitSurfaceVisibility();
	}

	function isCenterTabId(value: string): value is CenterTabId {
		return value === "session" || value === "editor" || value === "diff" || value === "git-history";
	}

	function adoptCenterTab(id: CenterTabId): void {
		const previous = centerTab;
		centerTab = id;
		if (previous === "diff" && id !== "diff") {
			gitService.clearSelection();
			gitCommitFilesService.clearSelection();
		}
	}

	/** Show a center surface. The dock owns which panel is active, so the tab
	 * state follows its announcement rather than being set here twice. */
	function applyCenterTab(id: CenterTabId): void {
		adoptCenterTab(id);
		frameControls?.showCenterPanel(id);
		syncGitSurfaceVisibility();
	}

	/** A center tab the user clicked: shown, and remembered for this session. */
	function selectCenterTab(id: CenterTabId): void {
		applyCenterTab(id);
	}

	/**
	 * Show a right panel, and say so.
	 *
	 * Four of the eight read something project-scoped, and all four follow the
	 * same rule they always have: they load only while the user can see them.
	 * "Visible" here means simply "this is the open tab" — the column no longer
	 * has folding panes, so there is no second half to the question.
	 */
	function applyRightTab(id: RightTabId): void {
		rightTab = id;
		shellPanels.filesVisible(id === "files");
		shellPanels.worktreesVisible(id === "worktrees");
		shellPanels.stacksVisible(id === "run");
		if (id === "browser") shellPanels.panelShown("browser");
		syncGitSurfaceVisibility();
	}

	function syncGitSurfaceVisibility(): void {
		if (restoringTabs) return;
		const graphVisible = centerTab === "git-history" || rightTab === "source-control";
		shellPanels.sourceControlVisible(graphVisible);
		if (graphVisible) {
			return;
		}
		gitService.releaseHistorySurface();
		gitCommitFilesService.release();
		// A commit-file click intentionally moves from History to Diff. That visible
		// diff owns its payload until the Diff tab itself is left.
		if (centerTab !== "diff") gitService.clearSelection();
	}

	/** A right tab the user clicked: shown, and remembered for this session. */
	function selectRightTab(id: RightTabId): void {
		applyRightTab(id);
	}

	/** Put both columns back on the tabs this session was left on. */
	function restoreTabsFor(snapshot: SessionWorkspaceSnapshot | null): void {
		restoringTabs = true;
		try {
			const storedCenter = snapshot?.center?.activePanelId;
			applyCenterTab(isStoredCenterTabId(storedCenter) ? storedCenter : DEFAULT_CENTER_TAB);
			applyRightTab(snapshot?.rightTab ?? DEFAULT_RIGHT_TAB);
		} finally {
			restoringTabs = false;
		}
		syncGitSurfaceVisibility();
	}

	/** The tool column used to be the vocabulary the palette, the rail rows and
	 * the context panel spoke; the right panel's tabs are that vocabulary now. A
	 * view with no tab of its own — the Problems list, which lives in the strip
	 * along the bottom — comes back null and is ignored. */
	function rightTabForView(id: SidebarViewId): RightTabId | null {
		if (id === "explorer") return "files";
		if (id === "stacks") return "run";
		if (id === "problems") return null;
		return id;
	}

	/** What the palette and the rail rows mean by a surface name. Session, Editor
	 * and Diff are center tabs; the browser and the session history became panels
	 * of the right column, and their old names still have to lead somewhere. */
	function showSurface(id: string): void {
		if (isCenterTabId(id)) {
			selectCenterTab(id);
			return;
		}
		if (id === "browser") selectRightTab("browser");
		else if (id === "session-library") selectRightTab("history");
	}

	async function persistOwnedMetadata(ownedId: string): Promise<void> {
		const session = rail.owned.find((entry) => entry.ownedId === ownedId);
		if (!session) return;
		await updateAgentConversationSessionMetaFromTauri({
			ownedId,
			model: session.model ?? null,
			effort: getConversationSession(ownedId)?.agentConfig.reasoningEffort ?? null,
			meta: ownedSessionMetaForBackend(session),
		});
	}

	/** Settled is an explicit rail transition. Age, process state, and title never infer this shelf. */
	function settleOwnedSession(ownedId: string): void {
		updateOwnedSession(ownedId, { settledAt: new Date().toISOString() });
		void persistOwnedMetadata(ownedId);
	}

	function unsettleOwnedSession(ownedId: string): void {
		updateOwnedSession(ownedId, { settledAt: null });
		void persistOwnedMetadata(ownedId);
	}

	function completeOwned(ownedId: string): void {
		completeOwnedSession(ownedId, new Date());
		void persistOwnedMetadata(ownedId);
	}

	function reopenOwned(ownedId: string): void {
		reopenOwnedSession(ownedId);
		void persistOwnedMetadata(ownedId);
	}

	/** The center library reuses the rail's imperative actions; construction of
	 * this adapter is inert and does not scan, start, or mutate anything. */
	const sessionLibraryService = createSessionLibraryService(
		{
			getOwnedSessions: () => rail.owned,
			listProviderSessions: async (projectPath?: string) => {
				try {
					const nativeSessions = projectPath
						? await listAgentSessionsForProjectFromTauri(projectPath)
						: await listAgentSessionsFromTauri();
					return nativeSessions ?? (await listAgentSessionsFromLocalBridge()) ?? [];
				} catch {
					return (await listAgentSessionsFromLocalBridge()) ?? [];
				}
			},
		},
		{
			onOpen: async (record: SessionLibraryRecord) => {
				if (record.ownedId) {
					await selectOwned(record.ownedId);
				} else if (record.available) {
					await adopt(record.available);
				}
			},
			onResume: async (record: SessionLibraryRecord) => {
				if (record.ownedId) {
					const owned = rail.owned.find((session) => session.ownedId === record.ownedId);
					if (owned?.state === "exited") await restartOwned(record.ownedId);
					else await selectOwned(record.ownedId);
				} else if (record.available) {
					await adopt(record.available);
				}
			},
			onArchive: (record: SessionLibraryRecord) => {
				if (!record.ownedId) return;
				if (record.state === "settled") unsettleOwnedSession(record.ownedId);
				else if (record.state === "done") settleOwnedSession(record.ownedId);
			},
			onDelete: async (record: SessionLibraryRecord) => {
				if (record.ownedId) await removeSession(record.ownedId);
			},
		},
	);

	/** Palette actions for the panels. Pure bookkeeping — nothing runs until the
	 * user picks one — so it belongs here at component init, not in an effect. */
	const releaseShellCommands = registerShellCommands({
		showPanel: (id) => showSurface(id),
		expandSourceControl: () => selectRightTab("source-control"),
		// Opening a panel is what lets that panel read anything, so nothing else
		// has to be called here — showing it reports the change and the load
		// follows.
		showView: (id) => {
			const tab = rightTabForView(id);
			if (tab) selectRightTab(tab);
		},
		openNewSession: () => openNewSession(),
		closeActiveEditor: () => editorPanel?.requestCloseActive(),
		showProblemsAtBottom: () => {
			settings.panels.problemsLocation = "bottom";
			applyProblemsLocation("bottom");
		},
	});

	/** The rail rows' quick-jump buttons. A jump is a session AND a surface, and
	 * this page is the only place that can do both — the center tabs belong to
	 * the frame and the views to the tool column. Bookkeeping like the palette
	 * actions above; nothing runs until a row button is clicked. */
	const releaseSessionRowJumpTarget = registerSessionRowJumpTarget({
		selectSession: async (ownedId) => {
			if (rail.activeOwnedId === ownedId) return;
			await selectOwned(ownedId);
		},
		showCenterPanel: (_ownedId, id) => {
			if (isCenterTabId(id)) selectCenterTab(id);
		},
		showSidebarView: (_ownedId, id) => {
			if (!isSidebarViewId(id)) return;
			const tab = rightTabForView(id);
			if (tab) selectRightTab(tab);
		},
	});

	/**
	 * The one way a panel reaches anything outside itself.
	 *
	 * Pure bookkeeping like the registrations above: it stores functions and
	 * calls none of them. Every panel in the right column and every surface in
	 * the middle goes through these six, so no panel ever imports another.
	 */
	registerWorkbenchNavigation({
		showCenterTab: (id) => selectCenterTab(id),
		showRightTab: (id) => selectRightTab(id),
		openDiff: async (request) => {
			if (!activeRootAvailable) return;
			await gitService.showStoredDiff(request.projectRoot, request.relativePath);
		},
		openFileTimeline: async (request) => {
			const ownedId = rail.activeOwnedId;
			const root = readSelection().root.trim();
			const generation = workspaceRestoreGeneration;
			if (!activeRootAvailable || !ownedId || !root) return false;
			selectCenterTab("git-history");
			await gitService.showFileHistory(request.projectRoot, request.relativePath);
			if (
				disposed ||
				rail.activeOwnedId !== ownedId ||
				workspaceRestoreGeneration !== generation ||
				canonicalPath(readSelection().root) !== canonicalPath(root)
			) return false;
			return true;
		},
		openUrl: (request) => {
			if (!activeRootAvailable || !readSelection().root.trim()) return;
			openBrowserUrl(request.url);
		},
		focusComposer: async (handoff) => {
			if (handoff.attachments) {
				// Add to what the composer is already holding. Replacing dropped a
				// screenshot the reader had just pasted, with nothing said about it.
				const existing = getConversationSession(handoff.ownedId)?.attachments ?? [];
				setConversationAttachments(handoff.ownedId, [...existing, ...handoff.attachments]);
			}
			if (handoff.appendText) {
				const draft = getConversationSession(handoff.ownedId)?.draft ?? "";
				setConversationDraft(handoff.ownedId, draft ? `${draft}\n${handoff.appendText}` : handoff.appendText);
			}
			// The composer may not be the surface on screen yet, so wait for Svelte
			// to have applied the draft before asking for the caret.
			await tick();
			conversationSurface?.focusComposer();
		},
		sendToSession: async (request) => {
			// The same two steps the composer takes: the attachments go on the
			// session, then the send picks them up. A send that fails leaves them
			// there, which is what the service does for the composer too.
			if (request.attachments?.length) {
				const existing = getConversationSession(request.ownedId)?.attachments ?? [];
				setConversationAttachments(request.ownedId, [...existing, ...request.attachments]);
			}
			await sendStructuredMessage(request.ownedId, request.text);
		},
		startSession: async (request) => {
			const provider = request.provider ?? "codex";
			const config = providerConfigsForNewSession().find((entry) => entry.provider === provider);
			try {
				return await startNewSession({
					prompt: request.prompt,
					provider,
					model: config?.model ?? null,
					reasoningEffort: config?.reasoningEffort ?? null,
					// Same reason as the draft's own request: Antigravity's adapter has
					// no approval control, so a remembered choice must not be sent to it.
					approvalPolicy: provider === "antigravity" ? null : (config?.approvalPolicy ?? null),
					projectPath: request.projectPath,
					cwd: request.cwd,
					branch: "",
					createNewWorktree: false,
					title: request.title,
				});
			} catch {
				// `startNewSession` has already put the failure on the rail in words a
				// person can read; the caller only needs to know it did not happen.
				return null;
			}
		},
	});

	/** The History panel's actions are the page's, because only the page owns the
	 * rail and the terminal service. Registered once, read by the panel. */
	const releaseSessionLibraryHost = registerSessionLibraryHost({
		service: sessionLibraryService,
		rescan: () => scanRail(),
	});

	/**
	 * Is the sessions column folded up to a strip? The PAGE owns this rather
	 * than the column, because folding is a WIDTH: the column says it wants to
	 * fold, and the frame is what actually makes the region 52px wide.
	 *
	 * SQLite owns the durable choice; this is only the live projection.
	 */
	const SESSIONS_COLLAPSED_SETTING_KEY = "shell.sessions-collapsed";
	let sessionsCollapsed = $state(false);
	let sessionsCollapsedVersion = 0;

	/** Tell the frame how wide the sessions column is now. The limits go with
	 * the width: folded, the column is fixed at strip width so the divider
	 * beside it cannot be dragged; open, it can be dragged again. */
	function applySessionsWidth(collapsed: boolean): void {
		frameControls?.setRegionWidth(
			"sessions",
			collapsed ? SESSIONS_STRIP_WIDTH : SESSIONS_WIDTH,
			collapsed
				? { minimumWidth: SESSIONS_STRIP_WIDTH, maximumWidth: SESSIONS_STRIP_WIDTH }
				: { minimumWidth: SESSIONS_MIN_WIDTH, maximumWidth: SESSIONS_MAX_WIDTH },
		);
	}

	/**
	 * How wide the tool column was before the browser asked to be widened. There
	 * is one width, and it is the grid's — the browser page has none of its own —
	 * so widening the page is this and nothing else, and narrowing it again puts
	 * the seam back where the user had dragged it rather than at a default.
	 */
	/** Fold the sessions column up, or open it out. SQLite remembers the choice. */
	function collapseSessions(collapsed: boolean): void {
		sessionsCollapsedVersion += 1;
		sessionsCollapsed = collapsed;
		applySessionsWidth(collapsed);
		void writeAssemblySettingFromTauri(SESSIONS_COLLAPSED_SETTING_KEY, collapsed).catch(() => undefined);
	}

	/**
	 * Put the Problems list where the setting says it goes.
	 *
	 * "In the strip along the bottom" keeps that strip open; hiding the list
	 * closes it to nothing, and the maximum height goes with the minimum so a
	 * divider cannot be dragged to reopen a strip holding nothing. Asked for once
	 * at start-up as well as on every change, or a choice made last week would
	 * only take effect when the buttons were pressed again.
	 */
	function applyProblemsLocation(location: ProblemsLocation): void {
		const atBottom = location === "bottom";
		// Shut means gone from the grid, not zero pixels tall. A region sized to
		// zero keeps its divider and the room the grid leaves around it, which is
		// the band that sat above the status bar with the strip closed.
		frameControls?.setDockPresent(atBottom);
		if (atBottom) {
			frameControls?.setRegionHeight("dock", DOCK_HEIGHT, {
				minimumHeight: 96,
				maximumHeight: Number.MAX_SAFE_INTEGER,
			});
		}
	}

	/** "Reset layout" means ALL of it: the grid regions (so both side columns go
	 * back to their default widths), the center surfaces, and every session's
	 * remembered pair of tabs. A folded sessions column is part of that
	 * arrangement, so it opens out too. */
	function resetLayout(): void {
		frameControls?.resetLayout();
		restoreTabsFor(null);
		void clearAllWorkspaceTabRecords();
		collapseSessions(false);
		// A reset builds the default arrangement, which has the bottom strip open.
		// Where the Problems list goes is a setting rather than part of the
		// arrangement, so it is said again here — a reset must not quietly undo it.
		applyProblemsLocation(settings.panels.problemsLocation);
	}

	/** Coalesce dockview's layout bursts into one refit per frame. */
	function scheduleRefit(): void {
		if (refitScheduled) return;
		refitScheduled = true;
		requestAnimationFrame(() => {
			refitScheduled = false;
			service?.refit();
		});
	}

	function describeError(error: unknown): string {
		return error instanceof Error ? error.message : String(error);
	}

	/** EXPLICIT IO: scan for resumable agent sessions (Tauri first, bridge second).
	 * ONE counted call per rescan, named for the transport that actually ran. */
	async function scanRail(): Promise<void> {
		if (rail.scanning) return;
		rail.scanning = true;
		rail.error = null;
		let nativeError: string | null = null;
		try {
			let sessions: AgentSession[] | null = null;
			try {
				sessions = await listAgentSessionsFromTauri();
			} catch (error) {
				nativeError = describeError(error);
			}
			// A `null` native result with no error = not under Tauri: nothing invoked.
			countInvoke((sessions ?? nativeError) ? "list_agent_sessions" : "bridge:agent-sessions");
			sessions ??= await listAgentSessionsFromLocalBridge();
			if (disposed) return;
			setAvailable(sessions ?? []);
			// Surface the native failure only if the bridge produced nothing either.
			if (sessions === null && nativeError) rail.error = `session scan failed: ${nativeError}`;
		} catch (error) {
			if (!disposed) rail.error = `session scan failed: ${nativeError ?? describeError(error)}`;
		} finally {
			rail.scanning = false;
		}
	}

	/** Park the active session's freshly mounted terminal host. */
	function registerHost(ownedId: string, host: HTMLElement): void {
		pendingHosts.set(ownedId, host);
	}

	/** Wait for TerminalSurface to mount the host for `ownedId`.
	 *
	 * A host that is no longer IN the page is not an answer: a session whose
	 * terminal was closed loses its host div, and this map still holds the one it
	 * used to have. Building a terminal on that detached element would leave the
	 * session running with nothing on screen — so a host that has been taken out
	 * of the page is dropped here, and the wait continues for the one Svelte is
	 * about to mount in its place. */
	async function hostFor(ownedId: string): Promise<HTMLElement | null> {
		const mounted = (): HTMLElement | null => {
			const host = pendingHosts.get(ownedId);
			if (!host) return null;
			if (host.isConnected) return host;
			pendingHosts.delete(ownedId);
			return null;
		};
		for (let attempt = 0; attempt < 12; attempt += 1) {
			const host = mounted();
			if (host) return host;
			await tick();
			if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 8));
		}
		return mounted();
	}

	function conversationProviderFor(ownedId: string): AgentConversationProvider | null {
		const agent = rail.owned.find((session) => session.ownedId === ownedId)?.agent;
		return agent === "codex" || agent === "claude" || agent === "antigravity" ? agent : null;
	}

	function rememberFileTreeExpandedPaths(root: string, paths: readonly string[]): void {
		const ownedId = rail.activeOwnedId;
		const projectRoot = canonicalPath(root);
		if (!ownedId || !projectRoot) return;
		const expandedPathsByRoot = {
			...(activeWorkspaceSnapshot?.expandedPathsByRoot ?? {}),
			[projectRoot]: [...paths],
		};
		activeWorkspaceSnapshot = activeWorkspaceSnapshot
			? { ...activeWorkspaceSnapshot, expandedPathsByRoot }
			: captureWorkspace({
				openFiles: [],
				activePath: null,
				selectedPath: explorer.selectedPath,
				scrollTop: explorer.scrollTop,
				rightTab,
				expandedPathsByRoot,
			});
		scheduleWorkspaceAutosave(
			ownedId,
			editorState.openFiles,
			editorState.activePath,
			centerTab,
			rightTab,
			captureBrowserState(),
		);
	}

	function rememberInspectionRoot(kind: 'files' | 'source-control', root: string | null): void {
		const ownedId = rail.activeOwnedId;
		const sessionRoot = canonicalPath(readSelection().root);
		if (!ownedId || !sessionRoot) return;
		const selectedRoot = canonicalPath(root ?? '');
		const inspectionRoot = selectedRoot && selectedRoot !== sessionRoot ? selectedRoot : null;
		const filesInspectionRoot =
			kind === 'files' ? inspectionRoot : (activeWorkspaceSnapshot?.filesInspectionRoot ?? null);
		const sourceControlInspectionRoot =
			kind === 'source-control'
				? inspectionRoot
				: (activeWorkspaceSnapshot?.sourceControlInspectionRoot ?? null);
		activeWorkspaceSnapshot = activeWorkspaceSnapshot
			? { ...activeWorkspaceSnapshot, filesInspectionRoot, sourceControlInspectionRoot }
			: captureWorkspace({
					openFiles: [],
					activePath: null,
					selectedPath: explorer.selectedPath,
					scrollTop: explorer.scrollTop,
					rightTab,
					filesInspectionRoot,
					sourceControlInspectionRoot,
				});
		scheduleWorkspaceAutosave(
			ownedId,
			editorState.openFiles,
			editorState.activePath,
			centerTab,
			rightTab,
			captureBrowserState(),
		);
	}

	/** Remember the editor tabs and file tree this session is leaving behind.
	 * Stored straight away: a reload can come at any moment, and the write is a
	 * few hundred bytes. */
	async function snapshotWorkspace(ownedId: string): Promise<boolean> {
		if (clearAllEditorsInFlight) return false;
		const ownedPaths = editorPanel?.workspaceOwnedPaths()
			?? editorState.openFiles.map((file) => file.path);
		const ownedPathSet = new Set(ownedPaths);
		const openFiles = editorState.openFiles.filter((file) => ownedPathSet.has(file.path));
		const activePath = editorState.activePath && ownedPathSet.has(editorState.activePath)
			? editorState.activePath
			: openFiles.at(-1)?.path ?? null;
		const snapshot = captureWorkspace({
				openFiles,
				activePath,
				viewStates: editorPanel?.captureViewStates(ownedPaths),
				selectedPath: explorer.selectedPath,
				scrollTop: explorer.scrollTop,
				diffPath: gitPanel.selectedPath || null,
				diffRoot: gitPanel.root,
				diffMode,
				conversation: captureConversationWorkspace(ownedId),
				browser: captureBrowserState(),
				center: frameControls?.captureCenterLayout() ?? null,
				rightTab,
				expandedPathsByRoot: activeWorkspaceSnapshot?.expandedPathsByRoot,
				filesInspectionRoot: activeWorkspaceSnapshot?.filesInspectionRoot,
				sourceControlInspectionRoot: activeWorkspaceSnapshot?.sourceControlInspectionRoot,
		});
		activeWorkspaceSnapshot = snapshot;
		try {
			const write = workspaceWriteQueue.then(async () => {
				if (!(await writeAgentConversationWorkspaceFromTauri(ownedId, snapshot))) {
					throw new Error("workspace checkpoint was refused");
				}
			});
			workspaceWriteQueue = write.catch(() => undefined);
			await write;
			return true;
		} catch (error) {
			if (!disposed && !clearAllEditorsInFlight) rail.error = `workspace checkpoint failed: ${describeError(error)}`;
			return false;
		}
	}

	function handleDiffModeChange(mode: DiffMode): void {
		diffMode = mode;
		const ownedId = rail.activeOwnedId;
		if (workspaceAutosaveEnabled && ownedId !== null) void snapshotWorkspace(ownedId);
	}

	/** Move one active Codex session to another checkout after preserving and
	 * releasing every projection rooted in the departing folder. The backend
	 * owns the quiescent gate and durable event; this page only re-points the
	 * checkout-backed surfaces once that transaction succeeds. */
	async function changeCodexCheckout(ownedId: string, requestedRoot: string): Promise<boolean> {
		const selected = rail.owned.find((session) => session.ownedId === ownedId);
		const conversation = getConversationSession(ownedId);
		if (
			!selected
			|| selected.origin !== "app"
			|| rail.activeOwnedId !== ownedId
			|| conversationProviderFor(ownedId) !== 'codex'
			|| !conversation
		) {
			rail.error = 'Only the active Codex session can change checkout.';
			return false;
		}
		const conversationGeneration = conversation.generation;
		const restoreGeneration = workspaceRestoreGeneration;
		const selectionGeneration = sessionSelectionGeneration;
		const checkoutStillCurrent = (): boolean =>
			!disposed
			&& rail.activeOwnedId === ownedId
			&& workspaceRestoreGeneration === restoreGeneration
			&& sessionSelectionGeneration === selectionGeneration
			&& getConversationSession(ownedId)?.generation === conversationGeneration
			&& conversationProviderFor(ownedId) === 'codex';
		const root = requestedRoot.trim();
		if (!root) {
			rail.error = 'A checkout folder is required.';
			return false;
		}
		if (sessionProjectionOwner !== null || clearAllEditorsInFlight) {
			rail.error = 'Wait for the current session change to finish.';
			return false;
		}
		sessionProjectionOwner = "checkout";
		try {
			countInvoke('validate_project_root');
			const validation = await validateProjectRootFromTauri(root);
			if (!checkoutStillCurrent()) return false;
			if (validation && (!validation.exists || !validation.isDirectory)) {
				rail.error = 'That checkout folder is not available.';
				return false;
			}
			workspaceAutosaveEnabled = false;
			cancelWorkspaceAutosave();
			await flushConversationSessionDraft(ownedId);
			if (!checkoutStillCurrent()) return false;
			if (!(await snapshotWorkspace(ownedId))) return false;
			if (!checkoutStillCurrent()) return false;
			const oldPaths = editorState.openFiles.map((file) => file.path);
			const record = await changeStructuredConversationCheckout(ownedId, root);
			if (!record) return false;
			if (!checkoutStillCurrent()) return false;
			// The backend has committed the new CWD. Replace the old-root workspace
			// checkpoint before any stale exit or unrelated probe work can fail.
			stopConversationTerminalProjection(ownedId);
			service?.releaseView(ownedId);
			releaseBrowserWorkspace();
			clearBrowserUrl();
			editorPanel?.releaseSessionResources(oldPaths);
			gitService.releaseHistorySurface();
			gitCommitFilesService.release();
			gitService.clearSelection();
			shellPanels.sourceControlVisible(false);
			resetEditorState();
			selectPath(null);
			setScrollTop(0);

			updateOwnedSession(ownedId, { cwd: record.cwd, runtimeState: record.state });
			activeRootAvailable = true;
			setUnavailableOpenFileRoot(null);
			activeWorkspaceSnapshot = null;
			shellPanels.sessionPicked(true);
			syncGitSurfaceVisibility();
			if (!(await snapshotWorkspace(ownedId))) return false;
			if (!checkoutStillCurrent()) return true;

			await disposeExtensionApiProbeResources();
			if (!checkoutStillCurrent()) return true;
			await setExtensionApiProbeWorkspace({ ownedId, root: record.cwd });
			if (!checkoutStillCurrent()) return true;
			return true;
		} catch (error) {
			if (!disposed) rail.error = `Checkout change failed: ${describeError(error)}`;
			return false;
		} finally {
			if (sessionProjectionOwner === "checkout") sessionProjectionOwner = null;
			workspaceAutosaveEnabled = shellPanels.loadsAllowed();
		}
	}

	function cancelWorkspaceAutosave(): void {
		if (workspaceSaveTimer !== null) clearTimeout(workspaceSaveTimer);
		workspaceSaveTimer = null;
	}

	function scheduleWorkspaceAutosave(
		ownedId: string,
		_openFiles: unknown,
		_activePath: unknown,
		_centerTab: CenterTabId,
		_rightTab: RightTabId,
		_browser: unknown,
	): void {
		cancelWorkspaceAutosave();
		workspaceSaveTimer = setTimeout(() => {
			workspaceSaveTimer = null;
			if (workspaceAutosaveEnabled && rail.activeOwnedId === ownedId) void snapshotWorkspace(ownedId);
		}, 30_000);
	}

	$effect(() => {
		const ownedId = rail.activeOwnedId;
		const openFiles = editorState.openFiles;
		const activePath = editorState.activePath;
		const browserState = captureBrowserState();
		if (workspaceAutosaveEnabled && ownedId !== null) {
			scheduleWorkspaceAutosave(ownedId, openFiles, activePath, centerTab, rightTab, browserState);
		}
	});

	async function clearAllEditorWorkspaceRecords(): Promise<boolean> {
		const ownedId = rail.activeOwnedId;
		if (ownedId === null || clearAllEditorsInFlight || sessionProjectionOwner === "checkout") return false;
		clearAllEditorsInFlight = true;
		workspaceAutosaveEnabled = false;
		cancelWorkspaceAutosave();
		const clear = workspaceWriteQueue.then(clearAgentConversationWorkspaceEditorsFromTauri);
		workspaceWriteQueue = clear.catch(() => undefined);
		let cleared = false;
		try {
			await clear;
			cleared = true;
			if (activeWorkspaceSnapshot !== null) {
				activeWorkspaceSnapshot = {
					...activeWorkspaceSnapshot,
					openPaths: [],
					activePath: null,
					fileStates: undefined,
				};
			}
		} catch (error) {
			if (!disposed) rail.error = `workspace checkpoint failed: ${describeError(error)}`;
		} finally {
			workspaceAutosaveEnabled = shellPanels.loadsAllowed();
			clearAllEditorsInFlight = false;
		}
		return cleared;
	}

	async function clearAllWorkspaceTabRecords(): Promise<void> {
		workspaceAutosaveEnabled = false;
		cancelWorkspaceAutosave();
		const clear = workspaceWriteQueue.then(clearAgentConversationWorkspaceTabsFromTauri);
		workspaceWriteQueue = clear.catch(() => undefined);
		try {
			await clear;
			if (activeWorkspaceSnapshot !== null) {
				activeWorkspaceSnapshot = {
					...activeWorkspaceSnapshot,
					rightTab: DEFAULT_RIGHT_TAB,
					center: activeWorkspaceSnapshot.center
						? { ...activeWorkspaceSnapshot.center, activePanelId: DEFAULT_CENTER_TAB }
						: activeWorkspaceSnapshot.center,
				};
			}
		} catch (error) {
			rail.error = `workspace checkpoint failed: ${describeError(error)}`;
		} finally {
			workspaceAutosaveEnabled = shellPanels.loadsAllowed();
		}
	}

	/**
	 * Put back the editor tabs and file tree this session had.
	 *
	 * The strip is rebuilt from lightweight descriptors without reading files.
	 * Only the active path goes through the open-file request immediately; the
	 * other tabs read from disk when selected.
	 *
	 * The tree's state is assigned directly, AFTER `sessionPicked` has pointed the
	 * explorer at the project: listing the same folder again does nothing, and a
	 * scan of a different folder never closes folders the user had open. A
	 * highlighted file that no longer exists loses its highlight when that scan
	 * lands, which is the right answer.
	 */
	async function restoreWorkspace(ownedId: string): Promise<void> {
		// Start-up re-attaching a session picks it, which is indistinguishable from
		// a click. Replaying files then would read files before launch is over; the
		// end of start-up calls this itself once the gate is open.
		if (!shellPanels.loadsAllowed()) return;
		const generation = ++workspaceRestoreGeneration;
		let snapshot: SessionWorkspaceSnapshot | null = null;
		try {
			snapshot = await readAgentConversationWorkspaceFromTauri(ownedId);
		} catch (error) {
			if (!disposed && generation === workspaceRestoreGeneration && rail.activeOwnedId === ownedId && !clearAllEditorsInFlight) {
				rail.error = `workspace restore failed: ${describeError(error)}`;
			}
		}
		if (disposed || generation !== workspaceRestoreGeneration || rail.activeOwnedId !== ownedId || clearAllEditorsInFlight) return;
		activeWorkspaceSnapshot = snapshot;
		diffMode = snapshot?.diffMode ?? DEFAULT_DIFF_MODE;
		restoreBrowserState(snapshot?.browser);
		// A remembered diff is read only when this session is returning to the Diff
		// surface. Restoring it before the remembered center tab is applied would do
		// hidden status and diff work for sessions returning to Chat or Editor.
		const sessionRoot = activeRootAvailable ? readSelection().root.trim() : "";
		const rememberedDiff = diffPathFor(snapshot, sessionRoot);
		const restoringDiff = snapshot?.center?.activePanelId === "diff";
		if (snapshot?.center) frameControls?.restoreCenterLayout(snapshot.center);
		if (!restoringDiff || rememberedDiff === null) {
			gitService.clearSelection();
		} else {
			void gitService.showStoredDiff(sessionRoot, rememberedDiff);
		}
		const conversationProvider = conversationProviderFor(ownedId);
		if (conversationProvider) {
			restoreConversationWorkspace(ownedId, conversationProvider, snapshot?.conversation);
			if (rail.owned.find((session) => session.ownedId === ownedId)?.origin === "app") {
				setConversationMode(ownedId, "structured");
			}
		}
		if (!activeRootAvailable) {
			editorPanel?.restoreViewStates([]);
			resetEditorState();
			selectPath(null);
			setScrollTop(0);
			return;
		}
		const plan = planWorkspaceRestore(snapshot ?? null);
		// Every restore starts from THIS session's tabs and no others. A session
		// that has never had a file open starts from an empty editor, and that
		// emptiness is the whole point: it is the other session's tabs not being
		// there.
		editorPanel?.restoreViewStates(plan.openFiles);
		if (plan.openFiles.length > 0) restoreEditorFiles(plan.openFiles, plan.activePath);
		else resetEditorState();
		if (!snapshot) return;
		selectPath(snapshot.selectedPath);
		setScrollTop(snapshot.scrollTop);
	}

	async function selectOwned(ownedId: string, propagateStructuredFailure = false): Promise<void> {
		if (sessionProjectionOwner === "checkout" || clearAllEditorsInFlight) return;
		const selectionGeneration = ++sessionSelectionGeneration;
		sessionProjectionOwner = "selection";
		try {
			await selectOwnedCurrent(ownedId, propagateStructuredFailure, selectionGeneration);
		} finally {
			if (selectionGeneration === sessionSelectionGeneration && sessionProjectionOwner === "selection") {
				sessionProjectionOwner = null;
			}
		}
	}

	async function selectOwnedCurrent(ownedId: string, propagateStructuredFailure: boolean, selectionGeneration: number): Promise<void> {
		const selectionIsCurrent = (): boolean => !disposed && !clearAllEditorsInFlight && selectionGeneration === sessionSelectionGeneration;
		// A draft is discarded the moment another session takes the Session tab.
		// It never existed anywhere but this flag, so there is nothing to clean up.
		draftOpen = false;
		const previous = rail.activeOwnedId;
		const switching = previous !== ownedId;
		const selected = rail.owned.find((session) => session.ownedId === ownedId);
		let selectedRootAvailable = true;
		if (selected && shellPanels.loadsAllowed()) {
			const root = selected.cwd.trim() || (selected.projectPath ?? "").trim();
			if (root) {
				countInvoke("validate_project_root");
				const validation = await validateProjectRootFromTauri(root);
				if (!selectionIsCurrent()) return;
				selectedRootAvailable = validation === null || (validation.exists && validation.isDirectory);
			}
		}
		let workspaceCaptured = previous === null || !switching;
		// The strip along the bottom reports the last thing that went wrong. Left
		// up, a start that failed in one session was still being reported while
		// the reader worked in another; moving on is what puts it down.
		if (switching) rail.error = null;
		// Save the session being left BEFORE anything points the panels elsewhere.
		// Gated the same way as the restore below: during start-up the panels are
		// still empty, and saving that emptiness would overwrite the tabs the
		// session actually had (rows are clickable for seconds while the first
		// scan runs — including the close button, which switches sessions too).
		if (switching && previous !== null) {
			workspaceAutosaveEnabled = false;
			cancelWorkspaceAutosave();
			try {
				await flushConversationSessionDraft(previous);
			} catch (error) {
				if (!selectionIsCurrent()) return;
				rail.error = `conversation draft checkpoint failed: ${describeError(error)}`;
				workspaceAutosaveEnabled = true;
				return;
			}
			if (!selectionIsCurrent()) return;
			workspaceCaptured = shellPanels.loadsAllowed();
			if (workspaceCaptured && !(await snapshotWorkspace(previous))) {
				if (!selectionIsCurrent()) return;
				workspaceAutosaveEnabled = true;
				return;
			}
			if (!selectionIsCurrent()) return;
			if (workspaceCaptured) releaseBrowserWorkspace();
		}
		if (!selectionIsCurrent()) return;
		if (switching && previous !== null) {
			stopConversationTerminalProjection(previous);
			service?.releaseView(previous);
			await disposeExtensionApiProbeResources();
			if (!selectionIsCurrent()) return;
			releaseConversationForRead(previous);
			gitService.releaseHistorySurface();
			gitCommitFilesService.release();
			gitService.clearSelection();
			shellPanels.sourceControlVisible(false);
		}
		const selectedRoot = selected ? selected.cwd.trim() || (selected.projectPath ?? "").trim() : "";
		if (switching && selected && selectedRoot && selectedRootAvailable) {
			await setExtensionApiProbeWorkspace({ ownedId: selected.ownedId, root: selectedRoot });
			if (!selectionIsCurrent()) return;
		}
		if (!selectionIsCurrent()) return;
		// Publish the new owner only after every async precondition is current. A
		// second click during the probe setup must still checkpoint the session that
		// is actually on screen, not this one whose setup has not finished.
		// Do not expose the departing session's tree projection while the arriving
		// session's SQLite workspace is still being restored.
		if (switching) activeWorkspaceSnapshot = null;
		setActiveOwned(ownedId);
		activeRootAvailable = selectedRootAvailable;
		setUnavailableOpenFileRoot(activeRootAvailable ? null : selectedRoot);
		const provider = conversationProviderFor(ownedId);
		let structuredHydration: Promise<void> | null = null;
		if (selected && provider) {
			ensureConversationSession(ownedId, provider);
			if (selected.origin === "external" && selected.ptySessionId) {
				setConversationMode(ownedId, "raw");
			} else {
				setConversationMode(ownedId, "structured");
				if (switching) {
					structuredHydration = Promise.all([
						loadConversationForRead(ownedId),
						loadConversationSessionDraft(ownedId),
					]).then(() => undefined).catch((error) => {
						if (!selectionIsCurrent()) return;
						const message = describeError(error);
						updateOwnedSession(ownedId, { lastError: message });
						if (propagateStructuredFailure) throw error;
					});
				}
			}
		}
		if (!activeRootAvailable) {
			await handleActiveRootUnavailable(selectedRoot, true);
			if (!selectionIsCurrent()) return;
		}
		// Clicking the session you are already on changes nothing. Putting the
		// stored record back here would throw away every file opened since the last
		// switch, which is the opposite of what a click on your own row means.
		if (switching) {
			// Departing models must be gone before replay can create the arriving session's model.
			if (previous !== null && workspaceCaptured) {
				editorPanel?.releaseSessionResources(editorState.openFiles.map((file) => file.path));
			}
			await restoreWorkspace(ownedId);
			if (!selectionIsCurrent()) return;
			workspaceAutosaveEnabled = shellPanels.loadsAllowed();
			// Both columns go back to the tabs this session was left on. After the
			// workspace restore, which may have brought the editor forward for a file
			// it re-opened — the session's own remembered tab wins.
			restoreTabsFor(activeWorkspaceSnapshot);
		}
		// Point only the surfaces restored for the arriving session at its root.
		// Doing this before restore made a departing blank Editor tab wake editor
		// and language-service ownership for an arriving chat-only session.
		if (activeRootAvailable) shellPanels.sessionPicked(true);
		if (switching && selectedRootAvailable && selected?.ptySessionId && service) {
			const host = await hostFor(ownedId);
			if (!selectionIsCurrent()) return;
			if (!host) {
				rail.error = `no terminal host for "${selected.title}"`;
			} else {
				const size = livePtySizes.get(selected.ptySessionId) ?? null;
				try {
					await service.adoptExisting(selected, host, size);
					if (!selectionIsCurrent()) return;
					service.show(ownedId);
				} catch (error) {
					if (!selectionIsCurrent()) return;
					rail.error = `re-attach failed for "${selected.title}": ${describeError(error)}`;
				}
			}
		}
		if (!switching && activeRootAvailable) service?.show(ownedId);
		if (selected && provider) {
			const mode = selected.origin === "external" && selected.ptySessionId ? "raw" : "structured";
			setConversationMode(ownedId, mode);
		}
		if (structuredHydration) {
			await structuredHydration;
			if (!selectionIsCurrent()) return;
		}
	}

	async function handleActiveRootUnavailable(root: string, force = false): Promise<void> {
		const selectedRoot = readSelection().root.trim();
		const ownedId = rail.activeOwnedId;
		if (
			(!force && !activeRootAvailable) ||
			!selectedRoot ||
			canonicalPath(root) !== canonicalPath(selectedRoot)
		) return;

		activeRootAvailable = false;
		setUnavailableOpenFileRoot(selectedRoot);
		releaseBrowserWorkspace();
		if (ownedId) {
			stopConversationTerminalProjection(ownedId);
			service?.releaseView(ownedId);
		}
		const probeCleanup = disposeExtensionApiProbeResources();
		editorPanel?.releaseSessionResources(editorState.openFiles.map((file) => file.path));
		resetEditorState();
		gitService.releaseHistorySurface();
		gitCommitFilesService.release();
		gitService.clearSelection();
		shellPanels.sessionPicked(false);
		try {
			await probeCleanup;
		} catch (error) {
			if (
				rail.activeOwnedId === ownedId &&
				canonicalPath(readSelection().root) === canonicalPath(selectedRoot)
			) rail.error = `extension probe cleanup failed: ${describeError(error)}`;
		}
	}

	function handoffInput(
		ownedId: string,
		direction: AgentConversationHandoffDirection,
		mode: AgentConversationHandoffMode,
	) {
		const selected = rail.owned.find((session) => session.ownedId === ownedId);
		const conversation = getConversationSession(ownedId);
		if (!selected || !conversation) throw new Error("The conversation is not loaded");
		const nativeSessionId = selected.nativeSessionId ?? conversation.nativeSessionId ?? null;
		const ptySessionId = selected.ptySessionId ?? null;
		if (!nativeSessionId) throw new Error("The native session id is not available");
		if (!ptySessionId) throw new Error("A live user terminal is required for handoff");
		return {
			ownedId,
			generation: conversation.generation,
			direction,
			mode,
			expectedOwner: direction === "structured-to-terminal" ? ("structured" as const) : ("terminal" as const),
			targetOwnedId: mode === "fork" ? `${ownedId}:native:${Date.now()}` : null,
			nativeSessionId,
			ptySessionId,
			historyBoundary: {
				nativeSessionId,
				firstSequence: 0,
				lastSequence: conversation.lastSequence,
				reconciledSequence: conversation.lastSequence,
			},
			// The assertion is populated from the single owned PTY and writer lease.
			// Native-window proof still needs the rebuilt app and is recorded as a
			// deferred acceptance item in the receipt.
			processTree: {
				checked: true,
				tuiLive: direction === "structured-to-terminal",
				tuiReleased: direction === "terminal-to-structured",
				writerCount: 1,
				ptyCount: 1,
				sidecarCount: 0,
				ptySessionId,
			},
		};
	}

	async function completeNativeHandoff(ownedId: string, mode: AgentConversationHandoffMode): Promise<void> {
		const input = handoffInput(ownedId, "structured-to-terminal", mode);
		await prepareConversationHandoff(input);
		try {
			const receipt = await commitConversationHandoff(input);
			if (mode === "same-session" && receipt.nativeSessionId) {
				const provider = conversationProviderFor(ownedId);
				if (provider) {
					startConversationTerminalProjection({ ownedId, provider, nativeSessionId: receipt.nativeSessionId });
				}
			}
		} catch (error) {
			await rollbackConversationHandoff(input).catch(() => undefined);
			throw error;
		}
	}

	async function openNativeCli(ownedId: string): Promise<void> {
		try {
			await completeNativeHandoff(ownedId, "same-session");
		} catch (error) {
			rail.error = `native handoff failed: ${describeError(error)}`;
		}
	}

	async function forkNativeCli(ownedId: string): Promise<void> {
		try {
			await completeNativeHandoff(ownedId, "fork");
		} catch (error) {
			rail.error = `native fork failed: ${describeError(error)}`;
		}
	}

	async function returnToStructured(ownedId: string): Promise<void> {
		try {
			const input = handoffInput(ownedId, "terminal-to-structured", "same-session");
			await prepareConversationHandoff(input);
			const receipt = await commitConversationHandoff(input);
			if (receipt.owner === "structured") setConversationMode(ownedId, "structured");
		} catch (error) {
			rail.error = `structured handoff failed: ${describeError(error)}`;
		}
	}

	/** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
	async function adopt(record: AgentSession): Promise<void> {
		if (!service || disposed) return;
		// A scan that never found the session's folder cannot be resumed into one.
		// Checked before the row is added, because a row added here and refused by
		// the backend a moment later is an orphan nothing can start or clear.
		if (!record.projectPath?.trim()) {
			rail.error = `no working folder was recorded for "${record.title}", so it cannot be resumed`;
			return;
		}
		const owned = adoptAgentSession(record);
		addOwnedSession(owned);
		const provider = conversationProviderFor(owned.ownedId);
		if (!provider) {
			rail.error = `could not adopt "${owned.title}" into the session database`;
			return;
		}
		await ensureStructuredConversation({
			ownedId: owned.ownedId,
			provider,
			cwd: owned.cwd,
			nativeSessionId: owned.nativeSessionId,
			nativeSessionMode: "load",
		});
		await persistOwnedMetadata(owned.ownedId);
		await selectOwned(owned.ownedId);
		const host = await hostFor(owned.ownedId);
		if (!host) {
			rail.error = `no terminal host for "${owned.title}"`;
			return;
		}
		const ptySessionId = await service.startOwned(owned, host);
		if (!ptySessionId) {
			updateOwnedSession(owned.ownedId, { state: "exited" });
			rail.error = `failed to start a terminal for "${owned.title}"`;
			return;
		}
		updateOwnedSession(owned.ownedId, { ptySessionId, state: "live" });
		await persistOwnedMetadata(owned.ownedId);
		await selectOwned(owned.ownedId);
		// A session you just started is a session you want to watch. Deliberately
		// here rather than inside `selectOwned`, which also runs on every plain
		// click on a card — a click on a row must not yank the reader off the file
		// they had open.
		selectCenterTab("session");
	}

	/**
	 * EXPLICIT IO: turn the thread-first draft into one app-owned conversation.
	 * Opening the pane does nothing; this function runs only after the first
	 * prompt is sent. The existing ensure/select/send path remains the single
	 * session creation authority.
	 */
	async function startNewSession(request: ThreadStartRequest): Promise<string> {
		console.warn("mcb next: thread-start submit", {
			provider: request.provider,
			cwd: request.cwd,
			branch: request.branch,
			disposed,
		});
		if (disposed) throw new Error("the shell is closing");
		const owned = {
			...createFreshSession({ cwd: request.cwd, title: request.title }),
			agent: request.provider,
			projectPath: request.projectPath,
			branch: request.branch,
			resumeCommand: null,
			origin: "app" as const,
		};
		addOwnedSession(owned);
		updateOwnedSession(owned.ownedId, {
			state: "live",
			executionOwner: "structured",
			runtimeState: "starting",
			lastError: null,
		});
		// Starting a session is the user's doing, so the panels may follow it — even
		// if start-up has not finished opening that gate yet. Without this a session
		// started in the first seconds after launch got a file tree and an editor
		// that were never pointed at its folder, and nothing came along later to
		// point them: the pick that would have done it was the one being ignored.
		shellPanels.allowSessionLoads();

		try {
			await selectOwned(owned.ownedId, true);
			// Said again here rather than relied upon: the pick above is what loads
			// the file tree and re-points the open tabs, and this is the one place
			// that must be certain it happened for the folder the user chose.
			shellPanels.sessionPicked(activeRootAvailable);
			// Only now — both tabs are remembered under the session that is actually
			// active, and a new session opens on its own transcript and its own files.
			selectCenterTab("session");
			selectRightTab("files");
			await sendStructuredMessage(owned.ownedId, request.prompt, {
				reasoningEffort: request.reasoningEffort,
				model: request.model,
				approvalPolicy: request.approvalPolicy,
			});
			await persistOwnedMetadata(owned.ownedId);
			rememberLastUsed(request.projectPath);
			updateOwnedSession(owned.ownedId, { runtimeState: "ready", lastError: null });
			return owned.ownedId;
		} catch (error) {
			const detail = describeError(error);
			const message = `could not start ${owned.agent} session: ${detail}`;
			console.warn("mcb next: thread-start failed", detail);
			updateOwnedSession(owned.ownedId, {
				state: "exited",
				executionOwner: "stopped",
				runtimeState: "failed",
				lastError: detail,
			});
			rail.error = message;
			throw new Error(message);
		}
	}

	/**
	 * EXPLICIT IO: run a saved stack. A stack IS a session — it appears on the
	 * rail like any other and its terminal is the one you watch.
	 *
	 * `runCommandDirectly` is what makes the stacks pane honest: the session is
	 * the command rather than a shell with the command typed into it, so when the
	 * dev server dies the session ends with the server's exit code instead of
	 * dropping back to a prompt that reads as "still starting". A desktop build
	 * too old for that falls back to typing the command in — see `startOwned`.
	 *
	 * Answers with the new session's id, or `null` when no terminal could be
	 * opened; the pane says so in its own words rather than guessing.
	 */
	async function onStartStack(request: StackStartRequest): Promise<string | null> {
		if (!service || disposed) return null;
		const owned = {
			...createFreshSession({ cwd: request.cwd, title: request.title }),
			resumeCommand: request.script,
		};
		addOwnedSession(owned);
		await selectOwned(owned.ownedId);
		const host = await hostFor(owned.ownedId);
		if (!host) return null;
		const ptySessionId = await service.startOwned(owned, host, { runCommandDirectly: true });
		if (!ptySessionId) {
			updateOwnedSession(owned.ownedId, { state: "exited" });
			return null;
		}
		updateOwnedSession(owned.ownedId, { ptySessionId, state: "live" });
		await selectOwned(owned.ownedId);
		// Pressing play is asking to watch the thing start. Without this the run
		// configuration's terminal opens behind whatever tab was already in front,
		// and a command that fails immediately does so out of sight.
		selectCenterTab("session");
		return owned.ownedId;
	}

	/**
	 * EXPLICIT IO: start a finished session up again, in place.
	 *
	 * It stays the SAME session — same `ownedId`, so the files it had open, the
	 * branch and task the scanner gave it, and the day it was marked done all
	 * survive. Being picked back up is not a new piece of work, and it does not
	 * un-finish a finished one either: a done session started again stays under
	 * Done until the user reopens it.
	 *
	 * What it cannot keep is the process. A terminal that has ended cannot be
	 * revived, so this spawns a NEW one in the same folder and replays the
	 * session's resume command — the same thing resuming a scanned session does,
	 * which is why the agent picks the conversation up where it left off. A
	 * session started here rather than found on disk has no resume command and
	 * gets a plain shell back.
	 *
	 * Only one restart per row can be in flight. The row keeps reading "finished"
	 * — and so keeps offering the button — for as long as the first click is
	 * waiting on the backend, so without the `restarting` guard a double-click
	 * would spawn two terminals for one session. The second would replace the
	 * first in the service's bookkeeping while the first process kept running,
	 * leaving two agents appending to the same transcript and only one of them
	 * showing up anywhere the user could reach it.
	 */
	async function restartOwned(ownedId: string): Promise<void> {
		// Checked and claimed before the first await, so a second click cannot slip
		// through the window the first one opens.
		if (disposed || restarting.has(ownedId)) return;
		const session = rail.owned.find((entry) => entry.ownedId === ownedId);
		// Only a finished session can be started again; a running one already is.
		if (!session || session.state !== "exited") return;
		const label = session.title || ownedId;
		restarting.add(ownedId);

		if (session.origin === "app") {
			const provider = conversationProviderFor(ownedId);
			if (!provider) {
				restarting.delete(ownedId);
				return;
			}
			updateOwnedSession(ownedId, {
				state: "background",
				executionOwner: "structured",
				runtimeState: "suspended",
				ptySessionId: null,
				lastError: null,
			});
			try {
				await selectOwned(ownedId);
				selectCenterTab("session");
			} catch (error) {
				updateOwnedSession(ownedId, {
					state: "exited",
					executionOwner: "stopped",
					runtimeState: "failed",
					lastError: describeError(error),
				});
				if (!disposed) rail.error = `could not retry ${provider} session: ${describeError(error)}`;
			} finally {
				restarting.delete(ownedId);
			}
			return;
		}

		if (!service) {
			restarting.delete(ownedId);
			rail.error = `could not start "${label}" again: the terminal service is not ready`;
			return;
		}

		/**
		 * Put the terminal the manager promoted back on screen. Closing a view
		 * makes another session's view visible and `closeOwned` reports which one;
		 * every other caller adopts that answer. On the paths below that never
		 * reach `selectOwned(ownedId)` this is the only thing standing between the
		 * user and another session's scrollback sitting under this session's title.
		 */
		const adoptSuccessor = async (successor: string | null): Promise<void> => {
			if (successor === null) return;
			if (!rail.owned.some((entry) => entry.ownedId === successor)) return;
			await selectOwned(successor);
		};

		/** Whoever the first close promoted, kept where every exit can see it. */
		let successor: string | null = null;

		try {
			// The old terminal is over: drop its view and let the backend forget the
			// dead process. One session has one view, so without this the new
			// terminal would open underneath the last one's final output — and the
			// backend record of the finished process would be left with nothing able
			// to reach it. A refusal here is not worth stopping for or reporting: it
			// means the backend could not tidy away something that is already dead,
			// and the session is about to get a working terminal regardless.
			const closed = await service.closeOwned(ownedId, session.ptySessionId);
			successor = closed?.successor ?? null;

			// Say the row is running BEFORE asking for a terminal host: the terminal
			// surface only keeps a host on screen for a session it believes has a
			// terminal, so while the row still reads as finished there is nothing for
			// `hostFor` to wait for. The old PTY id goes at the same time — it names
			// a process that no longer exists.
			updateOwnedSession(ownedId, { state: "live", ptySessionId: null });
			await persistOwnedMetadata(ownedId);
			await selectOwned(ownedId);

			const host = await hostFor(ownedId);
			if (!host) {
				updateOwnedSession(ownedId, { state: "exited" });
				rail.error = `no terminal host for "${label}"`;
				await adoptSuccessor(successor);
				return;
			}
			// `startOwned` reads the folder and the resume command off this record;
			// the PTY id it had is cleared so nothing can point at the old process.
			// A stack's session keeps its one-command spawn on restart — typed into
			// a shell instead, the exit code would belong to the shell and a crashed
			// dev server would read as "started" again.
			const restartedStackId = stackIdForOwnedId(ownedId);
			const ptySessionId = await service.startOwned(
				{ ...session, ptySessionId: null },
				host,
				restartedStackId !== null ? { runCommandDirectly: true } : undefined,
			);
			if (!ptySessionId) {
				updateOwnedSession(ownedId, { state: "exited" });
				rail.error = `could not start "${label}" again: no new terminal opened`;
				await adoptSuccessor(successor);
				return;
			}

			// Ask again what the row says now. The moment it read "running" its Close
			// and Remove buttons came back, and either of them could have been used
			// while the terminal was still starting. Neither could reach this PTY —
			// it did not exist yet — so adopting it here would put back a session the
			// user has just closed, or attach a live process to a row that is gone.
			const current = rail.owned.find((entry) => entry.ownedId === ownedId);
			if (!current || current.state === "exited") {
				const closedAgain = await service.closeOwned(ownedId, ptySessionId);
				await adoptSuccessor(closedAgain?.successor ?? null);
				return;
			}

			// Persist the new PTY id: reload re-attach reads it back out of storage.
			updateOwnedSession(ownedId, { ptySessionId, state: "live" });
			await persistOwnedMetadata(ownedId);
			// A restarted stack run is a run again — without this the stacks pane
			// keeps the old exit on record and says "stopped" under a live server.
			if (restartedStackId !== null) await recordStackStart(restartedStackId, ownedId);
			await selectOwned(ownedId);
			// Only on this path, where the session the user asked for is the one that
			// ended up on screen. Every early return above hands the screen to a
			// DIFFERENT session on purpose, and pulling the reader to the terminal
			// panel there would show them somebody else's scrollback.
			selectCenterTab("session");
		} catch (error) {
			// The row goes back to finished rather than sitting there claiming to be
			// running: nothing started, and the card's buttons must still offer this.
			// updateOwnedSession(ownedId, { state: "exited" });
			if (!disposed) {
				rail.error = `could not start "${label}" again: ${describeError(error)}`;
				// Same reason as the early returns above: the close at the top of this
				// function already put someone else's terminal on screen.
				await adoptSuccessor(successor);
			}
		} finally {
			restarting.delete(ownedId);
		}
	}

	/**
	 * EXPLICIT IO: the ONLY path that kills a PTY. `service.closeOwned` never
	 * rejects — it reports `{ successor, error }` — so a failed close still hands
	 * back the terminal the manager left visible.
	 *
	 * The SESSION survives this. Closing a terminal ends the process and its
	 * screen; it does not end the piece of work, which stays on the list as a
	 * finished row until the user marks it done and removes it. `removeSession`
	 * is the only thing that takes a row off the list.
	 */
	async function closeTerminal(ownedId: string): Promise<void> {
		const session = rail.owned.find((entry) => entry.ownedId === ownedId);
		const result = await service?.closeOwned(ownedId, session?.ptySessionId ?? null);
		if (result?.error) {
			rail.error = `close failed for "${session?.title ?? ownedId}": ${describeError(result.error)}`;
		}
		// The PTY id is cleared with the state: it names a process that is gone, and
		// leaving it stored would have the next launch try to re-attach to it.
		updateOwnedSession(ownedId, { state: "exited", ptySessionId: null });
		await persistOwnedMetadata(ownedId);
		// Picked BEFORE the await: adopt it only while it still exists. It goes
		// through `selectOwned` like every other session change, and the order is
		// what makes that safe: `rail.activeOwnedId` is still the session whose
		// terminal just closed, so the tabs and tree on screen are saved as ITS
		// workspace, and only then does the successor's own state come back.
		// Pointing the rail at the successor directly saved this session's files
		// into the successor's record on the next switch.
		const successor = result?.successor ?? null;
		if (successor !== null && rail.owned.some((entry) => entry.ownedId === successor)) {
			await selectOwned(successor);
		}
	}

	/**
	 * EXPLICIT IO: take a session off the list for good. The transcript on disk is
	 * untouched; only CommandBar's record of it goes.
	 *
	 * The close runs every time, not just for a session that is still running. A
	 * session whose process ended on its own keeps both its terminal on screen and
	 * its record in the backend, and dropping the row is the last chance to clear
	 * either — the row is what the ids were reachable through.
	 */
	async function removeSession(ownedId: string): Promise<void> {
		await closeTerminal(ownedId);
		// The row is what the next launch rebuilds the rail from. Left in the
		// store, a removed session came back every time the app opened.
		await deleteAgentConversationSessionFromTauri(ownedId);
		pendingHosts.delete(ownedId);
		removeOwnedSession(ownedId);
		releaseConversationForRead(ownedId);
		removeConversationSession(ownedId);
		stopConversationTerminalProjection(ownedId);
		// A removed row takes its stack tag with it, rather than leaving one
		// pointing at a session that is gone.
		noteSessionRemoved(ownedId);
		if (rail.activeOwnedId === ownedId) activeWorkspaceSnapshot = null;
	}

	onMount(() => {
		const disposers: Array<() => void> = [
			releaseShellCommands,
			releaseSessionRowJumpTarget,
			clearWorkbenchNavigation,
			clearStackHandlers,
			releaseSessionLibraryHost,
			() => sourceIntelligence.dispose(),
			stopConversationEvents,
		];
		// First, and synchronous: it only touches the DOM, and every panel below
		// paints in the theme it sets.
		document.documentElement.classList.add("next-shell-document");
		applyStoredTheme();
		applyStoredFonts();
		disposed = false;
		void hydrateSettings()
			.then(() => {
				if (disposed) return;
				applyStoredTheme();
				applyStoredFonts();
				applyProblemsLocation(settings.panels.problemsLocation);
				if (!settings.intelligence.languageServers) {
					void setLanguageServersEnabled(false);
				}
				for (const [language, enabled] of Object.entries(settings.intelligence.languageServerEnabled)) {
					if (!enabled) void setLanguageServerEnabled(language as 'csharp' | 'typescript' | 'rust', false);
				}
			})
			.catch(() => undefined);
		const collapsedRestoreVersion = sessionsCollapsedVersion;
		void readAssemblySettingFromTauri(SESSIONS_COLLAPSED_SETTING_KEY)
			.then((stored) => {
				if (disposed || sessionsCollapsedVersion !== collapsedRestoreVersion) return;
				const collapsed = stored === true;
				sessionsCollapsed = collapsed;
				if (collapsed) applySessionsWidth(true);
			})
			.catch(() => undefined);
		// A file dropped anywhere but a drop zone would otherwise navigate the
		// window to that file and take the whole shell with it. Anything a zone
		// has already claimed arrives here with its default prevented.
		const swallowStrayDrop = (event: DragEvent): void => {
			if (!event.defaultPrevented) event.preventDefault();
		};
		window.addEventListener("dragover", swallowStrayDrop);
		window.addEventListener("drop", swallowStrayDrop);
		disposers.push(() => {
			window.removeEventListener("dragover", swallowStrayDrop);
			window.removeEventListener("drop", swallowStrayDrop);
		});
		// ⌥⌘I opens the inspector, the same chord every browser uses. WebKit offers
		// this itself, but only in a build the inspector was compiled into, and only
		// when the shortcut has not been swallowed on its way through — asking the
		// desktop app outright is the version that keeps working.
		const openDevtoolsOnChord = (event: KeyboardEvent): void => {
			if (!event.metaKey || !event.altKey || event.ctrlKey || event.code !== "KeyI") return;
			event.preventDefault();
			void openMainDevtoolsFromTauri().catch(() => undefined);
		};
		window.addEventListener("keydown", openDevtoolsOnChord);
		disposers.push(() => window.removeEventListener("keydown", openDevtoolsOnChord));
		const stopExtensionApiProbeObservations = onExtensionApiProbeObservation((observation) => {
			extensionApiProbeObservation = observation;
		});
		disposers.push(stopExtensionApiProbeObservations);
		void startConversationEvents();
		// Honour where the reader last put the Problems list. The frame and the
		// tool column both mount before this runs, so both have handed over their
		// controls by now. Without it the bottom strip comes back open on every
		// launch however it was left.
		applyProblemsLocation(settings.panels.problemsLocation);
		// The stacks pane never spawns or kills anything itself — the page owns the
		// rail, the terminal service and the terminal hosts, so it does the work and
		// the pane asks for it. Pure bookkeeping; nothing runs until a click.
		registerStackHandlers({
			onStartStack,
			onStopStack: (ownedId) => closeTerminal(ownedId),
			onSelectSession: (ownedId) => selectOwned(ownedId),
		});
		void (async () => {
			try {
				const backend = tauriTerminalBackend(countInvoke);
				const modules = await loadXtermModules();
				if (disposed) return;

				service = createTerminalService({
					backend,
					createView: (host, hooks) => makeTerminalView(modules, host, hooks),
					onExit: (ownedId, payload) => {
						updateOwnedSession(ownedId, { state: "exited" });
						// The stacks pane learns how its run ended from here and nowhere
						// else: no timer anywhere reads process states. A session that is
						// not a stack's costs one map lookup.
						noteTerminalExit(ownedId, { exitCode: payload.exitCode, signal: payload.signal });
					},
				});
				await service.attach();
				if (disposed) return;
				if (extensionApiProbeTerminalHost) {
					await configureExtensionApiProbeRuntime({
						terminalService: service,
						terminalHost: extensionApiProbeTerminalHost,
					});
				}

				// The rail is whatever SQLite holds, and nothing else.
				//
				// An empty database used to mean "this must be an old install", and the
				// shell re-imported a session list kept in the webview's own storage.
				// That could not be turned off: the code deleted the key after reading
				// it, but WebKit writes local storage lazily, so quitting the app threw
				// the deletion away and the very same list came back on the next launch.
				// Wiping sessions was therefore impossible — they returned every time,
				// as empty rows, along with the failures of the ones that could not be
				// carried over. The import was a one-time migration from a build that is
				// long gone, so it is gone too.
				const live = (await backend.list()) ?? [];
				if (disposed) return;
				for (const i of live) livePtySizes.set(i.sessionId, { cols: i.cols, rows: i.rows });
				const storedSessions = (await listAgentConversationSessionsFromTauri()) ?? [];
				const projected = storedSessions.map(ownedSessionFromBackend);
				const { owned, reattachable } = reconcileOwnedSessions(projected, live);
				// Tombstones re-attach too (final scrollback + a reapable PTY id); live first.
				const attachable = [
					...reattachable,
					...owned.filter((entry) => entry.state === "exited" && entry.ptySessionId),
				];
				hydrateOwned(owned);
				// Keep every PTY routable for exit/close events without creating its
				// xterm view. Only the selected survivor is hydrated below.
				for (const session of attachable) {
					const size = session.ptySessionId ? (livePtySizes.get(session.ptySessionId) ?? null) : null;
					service.trackExisting(session, size);
				}
				const initial = attachable[0] ?? null;
				if (initial) await selectOwned(initial.ownedId);
				const activationError = rail.error;
				await scanRail();
				if (activationError) {
					const prefix = rail.error ? `${rail.error}; ` : "";
					rail.error = `${prefix}${activationError}`;
				}
			} catch (error) {
				if (!disposed) rail.error = `shell start-up failed: ${describeError(error)}`;
			} finally {
				// Launch is over — including when it failed, or the file tree and the
				// context cards would never load again. From here, a session being
				// selected is the user's doing and those panels may follow it.
				if (!disposed) shellPanels.allowSessionLoads();
				// A session re-attached during start-up was "picked" before the gate was
				// open, so its pick was ignored. Repeat it now that loads are allowed, or
				// a reload comes back with empty panes until the user clicks a session.
				if (!disposed && rail.activeOwnedId !== null) {
					const selected = rail.owned.find((session) => session.ownedId === rail.activeOwnedId);
					const root = selected ? selected.cwd.trim() || (selected.projectPath ?? "").trim() : "";
					if (root) {
						countInvoke("validate_project_root");
						const validation = await validateProjectRootFromTauri(root);
						activeRootAvailable = validation === null || (validation.exists && validation.isDirectory);
					}
					if (!activeRootAvailable && root) await handleActiveRootUnavailable(root, true);
					else {
						setUnavailableOpenFileRoot(null);
						shellPanels.sessionPicked(true);
					}
				}
				// Same story for the files that session had open: the pick that would
				// have restored them happened before the gate opened, so a reload would
				// otherwise come back with an empty editor.
				if (!disposed && rail.activeOwnedId !== null) await restoreWorkspace(rail.activeOwnedId);
				if (!disposed) workspaceAutosaveEnabled = shellPanels.loadsAllowed();
			}
		})();

		/** Remember the session on screen when the page goes away. Leaving the
		 * window is not a session switch, so nothing else would have saved it, and a
		 * reload would come back to an empty editor. `pagehide` is the event
		 * browsers still fire for both a reload and a close. */
		const saveOnLeaving = (): void => {
			if (rail.activeOwnedId !== null && sessionProjectionOwner !== "checkout") void snapshotWorkspace(rail.activeOwnedId);
		};
		window.addEventListener("pagehide", saveOnLeaving);
		disposers.push(() => window.removeEventListener("pagehide", saveOnLeaving));

		return () => {
			cancelWorkspaceAutosave();
			// Navigating away inside the app ends here instead, and it is the same
			// last chance to remember what the session on screen had open.
			if (!disposed && rail.activeOwnedId !== null && sessionProjectionOwner !== "checkout") void snapshotWorkspace(rail.activeOwnedId);
			disposed = true;
			for (const dispose of disposers.splice(0)) dispose();
			// Probe teardown closes only its disposable PTY first. The product
			// service then drops views + its listener while every user PTY survives.
			const serviceToDispose = service;
			service = null;
			void disposeExtensionApiProbeRuntime().finally(() => serviceToDispose?.dispose());
			pendingHosts.clear();
			livePtySizes.clear();
			// The theme painted inline colors onto <html>, above the scoping that
			// keeps the old shell on its own palette. Leaving this page takes them
			// back off, so a same-document navigation to the old shell renders it
			// exactly as it was found.
			clearTheme();
			clearFonts();
			document.documentElement.classList.remove("next-shell-document");
		};
	});
</script>

<svelte:head>
	<title>{PRODUCT_DOCUMENT_TITLE}</title>
</svelte:head>

<!-- Every region is a top-level snippet: an implicit `{#snippet rail()}` child would
     shadow the imported `rail` store and break every `rail.owned` read. -->
{#snippet sessionsArea()}
	<!-- The session list. Settings used to sit in a strip of its own under it;
       it now lives at the left end of the status bar, where a single row along
       the bottom of the window carries all three. -->
	<div class="sessions-region">
		<div class="sessions-list">
			<SessionsColumn
				bind:this={sessionsColumn}
				owned={rail.owned}
				activeOwnedId={sessionProjection.activeOwnedId}
				collapsed={sessionsCollapsed}
				onSelect={selectOwned}
				onRestart={restartOwned}
				onComplete={completeOwned}
				onReopen={reopenOwned}
				onSettle={settleOwnedSession}
				onUnsettle={unsettleOwnedSession}
				onRemove={removeSession}
				onCollapse={collapseSessions}
				onNewSession={openNewSession}
			/>
		</div>
	</div>
{/snippet}
{#snippet toolsArea()}
		<RightPanel
			activeId={rightTab}
			onSelect={selectRightTab}
			root={checkoutScope.durableSessionRoot}
			rootAvailable={activeRootAvailable}
			{checkoutScope}
			ownedId={sessionProjection.activeOwnedId}
			onRootUnavailable={handleActiveRootUnavailable}
			expandedPathsByRoot={sessionProjection.activeWorkspace?.expandedPathsByRoot ?? {}}
			onExpandedPathsChange={rememberFileTreeExpandedPaths}
			filesInspectionRoot={checkoutScope.filesInspectionRoot}
			sourceControlInspectionRoot={checkoutScope.gitInspectionRoot}
			onFilesInspectionRootChange={(root) => rememberInspectionRoot('files', root)}
			onSourceControlInspectionRootChange={(root) => rememberInspectionRoot('source-control', root)}
			onUseSessionCheckout={async (root) => {
				const ownedId = sessionProjection.activeOwnedId;
				if (ownedId !== null) await changeCodexCheckout(ownedId, root);
			}}
		/>
{/snippet}
{#snippet centerTabsArea()}
	<CenterCornerTabs activeId={centerTab} onSelect={selectCenterTab} />
{/snippet}
{#snippet dockArea()}
	<DockPanel onReset={resetLayout} onProblemsLocationChange={applyProblemsLocation} />
{/snippet}
{#snippet sessionArea()}
	<!-- `onHostLayout` is what re-measures a terminal: a terminal is built inside
       a hidden host, where one character measures zero pixels wide, so the fit
       that runs when the panel is shown does nothing. The surface says when a
       host appears, changes size, or the terminal font lands. -->
	<!-- A draft is LAYERED over the conversation rather than replacing it: the
       active terminal host lives inside this component, and unmounting it to
       show a draft would take the running session off screen. -->
	<div class="session-area">
			<ConversationSurface
			bind:this={conversationSurface}
			owned={rail.owned}
			activeOwnedId={sessionProjection.activeOwnedId}
			activeOrigin={rail.owned.find((session) => session.ownedId === sessionProjection.activeOwnedId)?.origin}
				rootAvailable={activeRootAvailable}
			{registerHost}
			onHostLayout={scheduleRefit}
			onOpenNativeCli={openNativeCli}
			onForkNativeCli={forkNativeCli}
			onReturnToStructured={returnToStructured}
		/>
		{#if draftOpen}
			<DraftSessionSurface
				sessionRoots={deriveThreadStartProjects(rail.owned.map((session) => session.projectPath ?? session.cwd)).map(
					(project) => project.path,
				)}
				presetProjectPath={draftProjectPath}
				providerConfigs={providerConfigsForNewSession()}
				onSend={async (request) => {
					await startNewSession(request);
				}}
				onClose={() => (draftOpen = false)}
			/>
		{/if}
	</div>
{/snippet}
{#snippet editorArea()}
	<!-- Opening a file is a request to READ it: bring the editor forward, not load it out of sight. -->
		<EditorPanel
		bind:this={editorPanel}
			showing={centerTab === "editor"}
			rootAvailable={activeRootAvailable}
		onCloseAllEditors={clearAllEditorWorkspaceRecords}
		onStartWorkspaceCommand={(request) => {
			void onStartStack({ stackId: request.id, ...request });
		}}
		onFileOpened={() => {
			selectCenterTab("editor");
		}}
	/>
{/snippet}
<!-- The changes to whichever file source control has selected. `GitDiffView`
     reads the selected file itself and lives here as a tab of its own — which
     is what gives a diff the width of the middle instead of a column. -->
	{#snippet diffArea()}{#if centerTab === "diff"}<GitDiffView showing={true} rootAvailable={activeRootAvailable} mode={diffMode} onModeChange={handleDiffModeChange} />{/if}{/snippet}

<!-- The whole commit history as a table, given the width of the middle. It reads
     `gitPanel` itself; the active root only tells its lazy surface when to point
     the existing service at a new repository. -->
	{#snippet gitHistoryArea()}{#if centerTab === "git-history"}<GitHistoryView root={activeRootAvailable ? readSelection().root : ""} rootAvailable={activeRootAvailable} />{/if}{/snippet}

<!-- The webview's own right-click menu runs a native tracking loop that stalls
     the whole window for seconds, which reads as a freeze. Surfaces with a menu
     of their own already prevent the default; this catches everywhere else.
     Text fields keep the native menu — that one is the editing menu people
     expect, and it does not stall. -->
<main
	class="next-shell"
	oncontextmenu={(event) => {
		const target = event.target instanceof Element ? event.target : null;
		if (target?.closest('input, textarea, [contenteditable="true"]')) return;
		event.preventDefault();
	}}
>
	<!-- No strip along the top any more. The last thing left up there was the
       project's language-server switch, and it cost a row of window height on
       every screen to say one word; it now travels with the centre pane's pill
       tabs, which appear only when they are wanted. -->
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
			onSessionPanelLayout={scheduleRefit}
			onCenterPanelShown={handleCenterPanelShown}
			onReady={(controls) => {
				frameControls = controls;
				// Say how far the tool column's seam may travel, every launch. A stored
				// layout carries the limits it was saved with, so a shell that ran while
				// the ceiling was lower comes back unable to be dragged — or widened —
				// past it. The center's floor is said in the same breath: it is what
				// stops the seam before the conversation is squeezed out.
				controls.setRegionLimits("tools", {
					minimumWidth: TOOLS_MIN_WIDTH,
					maximumWidth: TOOLS_MAX_WIDTH,
				});
				controls.setRegionLimits("center", { minimumWidth: CENTER_MIN_WIDTH });
				// Say what the sessions column is, once, here, where the frame first
				// exists — in BOTH cases, not only the folded one.
				//
				// The grid layout remembers width and drag limits while SQLite remembers
				// whether the column is folded. Re-state both cases so an interrupted save
				// cannot leave open content pinned into the folded 52px geometry.
				if (sessionsCollapsed) {
					applySessionsWidth(true);
				} else {
					controls.setRegionLimits("sessions", {
						minimumWidth: SESSIONS_MIN_WIDTH,
						maximumWidth: SESSIONS_MAX_WIDTH,
					});
					// Only rescue a column that came back narrower than it is allowed to
					// be. Any other width is one the user dragged, and it survives.
					const restored = controls.regionWidth("sessions");
					if (restored !== null && restored < SESSIONS_MIN_WIDTH) {
						controls.setRegionWidth("sessions", SESSIONS_WIDTH);
					}
				}
				// One timer tick later: the tab area announces the tab it restored
				// through a microtask, and those all arrive before any timer. Waiting
				// means a restored tab loads nothing, while a real click still does.
				setTimeout(() => shellPanels.allowPanelLoads(), 0);
				// Open on the tabs the shell was left on. The frame is what shows a
				// center surface, so this cannot happen any earlier than here.
				restoreTabsFor(sessionProjection.activeWorkspace);
			}}
			onError={(message) => (layoutError = `layout failed: ${message}`)}
		/>
	</div>

	<!-- One bar across the whole window, below every panel. The Resources and
       Usage readouts used to sit inside the right column; they are about the
       machine and the account rather than that column, and a single bar the
       width of the window is the honest shape for them. It is also the only
       element that reaches the window edges, which is what lets every panel
       above it float with all four corners rounded. -->
	<div class="status-bar">
		<UtilityStrip
			onOpenSettings={() => overlays?.openSettings()}
			{openUtility}
			onOpenUtility={(id, anchor) => overlays?.openUtility(id, anchor)}
		/>
	</div>

	<ShellOverlays
		bind:this={overlays}
		onResetLayout={resetLayout}
		onRescanSessions={scanRail}
		message={[layoutError, centerTab === "session" ? rail.error : null].filter(Boolean).join("; ") || null}
		onProblemsLocationChange={applyProblemsLocation}
		onUtilityStateChange={(id, open) => {
			if (open) openUtility = id;
			else if (openUtility === id) openUtility = null;
		}}
	/>
	<div bind:this={extensionApiProbeTerminalHost} class="extension-api-probe-terminal-host" aria-hidden="true"></div>
	{#if extensionApiProbeObservation}
		<div class="extension-api-probe-observation" role="status" aria-live="polite">
			<strong>{extensionApiProbeObservation.summary}</strong>
			<span>{extensionApiProbeObservation.detail}</span>
		</div>
	{/if}
</main>

<style>
	/* ShellFrame owns the geometry now, but it still needs a definite height to
     measure against — at 0x0 the grid mounts and renders nothing.
     `position: relative` is what ShellOverlays positions its message strip and
     the development call counter against; both are absolute, so neither becomes
     a flex item of the column below. */
	.next-shell {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		background: var(--color-bg);
		color: var(--color-text);
	}

	/* The draft layer is absolute inside this, so the conversation underneath
     keeps its own size and its terminals keep their hosts. */
	.session-area {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 0;
	}

	.sessions-region {
		display: grid;
		height: 100%;
		width: 100%;
		min-width: 0;
		min-height: 0;
		grid-template-rows: minmax(0, 1fr) auto;
		overflow: hidden;
	}

	.sessions-list {
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}

	/* ShellFrame's own root is `height: 100%`, so it needs a parent whose height
     is already settled. A flex child with `min-height: 0` has one, and the
     status bar below it is the other child of the same column. */
	.frame-area {
		flex: 1 1 auto;
		min-height: 0;
	}

	/* The shell's one full-bleed element. Every panel above it is inset and
     rounded; this reaches both window edges so the layout has a floor. */
	.status-bar {
		flex: 0 0 auto;
		background: var(--color-bg);
		/* No rule along the top. The panels already stop short of the window edge,
       so the backdrop runs behind this bar and the two read as one floor; a
       border drew a line across that gap and made it look like a second bar. */
	}

	.extension-api-probe-terminal-host {
		position: fixed;
		left: -10000px;
		top: -10000px;
		width: 800px;
		height: 480px;
		pointer-events: none;
	}

	.extension-api-probe-observation {
		position: absolute;
		right: 16px;
		bottom: 16px;
		z-index: 120;
		display: grid;
		max-width: min(560px, calc(100vw - 32px));
		gap: 4px;
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
		border-radius: 6px;
		background: color-mix(in srgb, var(--color-bg) 96%, transparent);
		color: var(--color-text);
		box-shadow: 0 12px 28px rgb(0 0 0 / 38%);
		font-size: 12px;
		line-height: 1.35;
	}

	.extension-api-probe-observation span {
		color: var(--color-text-2);
	}
</style>
