import { sessionWorkspaceRoot, parseRemoteWorkspacePath } from '../../workspacePaths';
/**
 * Controller for session selection, filesystem projections, and workspace expansion.
 */
import { rail, setActiveOwned } from '../stores/sessionRailStore.svelte';
import { SessionSelectionLayers, type SessionSelectionOwner } from '../sessionSelectionLayers.svelte';
import { canonicalPath } from '../explorer/explorerStore.svelte';
import { countInvoke } from '../devInvokeCounter.svelte';
import {
	readAgentConversationWorkspaceExpandedPathsFromTauri,
	writeAgentConversationWorkspaceExpandedPathsFromTauri,
	deleteAgentConversationSessionFromTauri,
	validateProjectRootFromTauri,
} from '../../tauriSource';
import type { OwnedSession } from '../ownedSessions';
import { removeOwnedSession } from '../stores/sessionRailStore.svelte';
import { shellPanels } from '../shellPanels';
import { NewSessionController } from './newSessionController.svelte';
import { EditorSessionController, type EditorPanelLifecycle } from './editorSessionController.svelte';
import {
	changeStructuredConversationCheckout,
	flushConversationSessionDraft,
} from '../conversation/conversationService';
import type { SessionWorkspaceSnapshot } from '../sessionWorkspaces';

export class SessionSelectionController {
	controlledSelectionOwnedId = $state<string | null>(null);
	expandedPathsByRoot = $state.raw<Readonly<Record<string, readonly string[]>>>({});
	sessionSelectionLayers = new SessionSelectionLayers();
	newSession = new NewSessionController();
	editorSessions = new EditorSessionController();
	activeRootAvailable = $state(true);
	activeRootRemote = $state(false);
	activeWorkspaceSnapshot = $state.raw<SessionWorkspaceSnapshot | null>(null);

	private workspaceWriteQueue: Promise<void> | null = null;
	private selectionGeneration = 0;
	private selectionAbort: AbortController | null = null;
	private pendingSelectionOwnedId: string | null = null;
	private selectionWork: Promise<void> | null = null;

	get activeOwnedId(): string | null {
		return this.controlledSelectionOwnedId ?? rail.activeOwnedId;
	}

	get railOwned(): OwnedSession[] {
		return rail.owned;
	}

	get hasTreeProjection(): boolean {
		return this.sessionSelectionLayers.hasTreeProjection;
	}

	get hasChatProjection(): boolean {
		return this.sessionSelectionLayers.hasChatProjection;
	}

	get filesProjectionRoot(): string {
		return this.sessionSelectionLayers.hasTreeProjection
			? this.sessionSelectionLayers.treeRoot
			: "";
	}

	get filesProjectionOwnedId(): string | null {
		return this.sessionSelectionLayers.hasTreeProjection
			? this.sessionSelectionLayers.treeOwnedId
			: null;
	}

	get controlledSession(): OwnedSession | null {
		if (!this.controlledSelectionOwnedId) return null;
		return rail.owned.find((s) => s.ownedId === this.controlledSelectionOwnedId) ?? null;
	}

	get durableSessionRoot(): string {
		const session = this.controlledSession ?? rail.owned.find((s) => s.ownedId === rail.activeOwnedId);
		if (!session) return "";
		return canonicalPath(sessionWorkspaceRoot(session));
	}

	get controlledEditorRootAvailable(): boolean {
		return (
			this.controlledSelectionOwnedId !== null &&
			Boolean(this.sessionSelectionLayers.treeRoot)
		);
	}

	get filesInspectionRoot(): string | null {
		return null;
	}

	get chatOwnedId(): string | null {
		return this.sessionSelectionLayers.chatOwnedId;
	}

	setEditorPanel(panel: EditorPanelLifecycle | null): void {
		this.editorSessions.setPanel(panel);
	}

	async selectSession(ownedId: string): Promise<void> {
		this.activeWorkspaceSnapshot = null;
		this.controlledSelectionOwnedId = ownedId;
		setActiveOwned(ownedId);
		this.pendingSelectionOwnedId = ownedId;
		const requestedSession = rail.owned.find((candidate) => candidate.ownedId === ownedId);
		this.activeRootRemote = requestedSession?.executionEnvironment === "remote";
		const requestedRoot = canonicalPath(
			requestedSession ? sessionWorkspaceRoot(requestedSession) : "",
		);
		if (canonicalPath(this.sessionSelectionLayers.treeRoot) !== requestedRoot) {
			this.sessionSelectionLayers.clearTreeView();
		}
		if (this.selectionWork) {
			this.selectionAbort?.abort();
			await this.selectionWork;
			return;
		}

		const work = this.drainSelections();
		this.selectionWork = work;
		try {
			await work;
		} finally {
			if (this.selectionWork === work) this.selectionWork = null;
		}
	}

	async refreshRemoteConnection(profileId: string): Promise<void> {
		const selected = rail.owned.find((session) => session.ownedId === this.activeOwnedId);
		if (selected?.executionEnvironment !== 'remote' || selected.remoteProfileId !== profileId) return;
		await flushConversationSessionDraft(selected.ownedId);
		this.sessionSelectionLayers.clearTreeView();
		this.sessionSelectionLayers.clearChatHistory();
		await this.selectSession(selected.ownedId);
	}

	private async drainSelections(): Promise<void> {
		while (this.pendingSelectionOwnedId !== null) {
			const ownedId = this.pendingSelectionOwnedId;
			this.pendingSelectionOwnedId = null;
			await this.selectSessionOnce(ownedId);
		}
	}

	private async selectSessionOnce(ownedId: string): Promise<void> {
		const owner = this.beginSelection();

		const session = rail.owned.find((candidate) => candidate.ownedId === ownedId);
		if (session) {
			this.activeRootRemote = session.executionEnvironment === "remote";
			const root = canonicalPath(sessionWorkspaceRoot(session));
			const rootChanged = canonicalPath(this.sessionSelectionLayers.treeRoot) !== root;
			const needsWorkspaceState =
				rootChanged || !Object.prototype.hasOwnProperty.call(this.expandedPathsByRoot, root);
			if (rootChanged) this.expandedPathsByRoot = {};
			await this.materializeSelection(session, root, owner, this.chatOwnedId, needsWorkspaceState);
		} else {
			this.expandedPathsByRoot = {};
			this.sessionSelectionLayers.abandonSelection(owner);
			this.sessionSelectionLayers.clearTreeView();
			this.sessionSelectionLayers.clearChatHistory();
		}
	}

	rememberExpandedPaths(root: string, paths: readonly string[]): void {
		const ownedId = this.controlledSelectionOwnedId;
		const projectRoot = canonicalPath(root);
		if (!ownedId || !projectRoot) return;

		this.expandedPathsByRoot = { [projectRoot]: [...paths] };
		const previousWrite = this.workspaceWriteQueue;
		const write = this.writeExpandedPathsAfter(previousWrite, ownedId, projectRoot, paths);
		this.workspaceWriteQueue = write;
		void this.ignoreWorkspaceWriteFailure(write);
	}

	rememberWorkspaceState(patch: Partial<SessionWorkspaceSnapshot>): void {
		this.activeWorkspaceSnapshot = this.editorSessions.rememberWorkspaceState(patch);
	}

	/** One removable seam for changing every checkout-backed session surface. */
	async useSessionCheckout(requestedRoot: string): Promise<boolean> {
		const ownedId = this.activeOwnedId;
		const selected = rail.owned.find((session) => session.ownedId === ownedId);
		if (!ownedId || !selected || selected.agent !== 'codex' || selected.origin !== 'app') {
			rail.error = 'Only the active Codex Assembly session can change checkout.';
			return false;
		}

		const root = canonicalPath(requestedRoot);
		if (!root) {
			rail.error = 'A checkout folder is required.';
			return false;
		}

		const remoteRoot = parseRemoteWorkspacePath(root);
		if ((selected.executionEnvironment === 'remote' && remoteRoot?.profileId !== selected.remoteProfileId)
			|| (selected.executionEnvironment !== 'remote' && remoteRoot)) {
			rail.error = 'Choose a folder on this session’s machine.';
			return false;
		}
		const owner = this.beginSelection();
		try {
			countInvoke('validate_project_root');
			const validation = await validateProjectRootFromTauri(root, owner.signal);
			if (!this.isCurrent(owner)) return false;
			if (validation && (!validation.exists || !validation.isDirectory)) {
				rail.error = 'That checkout folder is not available.';
				return false;
			}

			await flushConversationSessionDraft(ownedId);
			if (!this.isCurrent(owner)) return false;
			const record = await changeStructuredConversationCheckout(ownedId, parseRemoteWorkspacePath(root)?.path ?? root);
			if (!record) return false;
			if (!this.isCurrent(owner)) return true;

			await this.editorSessions.resetForCheckoutChange(owner.signal);
			if (!this.isCurrent(owner)) return true;
			this.expandedPathsByRoot = {};
			this.sessionSelectionLayers.clearTreeView();
			const updated = rail.owned.find((session) => session.ownedId === ownedId);
			if (!updated) return true;
			this.activeRootRemote = updated.executionEnvironment === "remote";
			await this.materializeSelection(updated, canonicalPath(sessionWorkspaceRoot(updated)), owner, this.chatOwnedId, true);
			return true;
		} catch (error) {
			if (this.isCurrent(owner)) {
				rail.error = `Checkout change failed: ${error instanceof Error ? error.message : String(error)}`;
			}
			return false;
		}
	}

	async removeSession(ownedId: string): Promise<void> {
		await deleteAgentConversationSessionFromTauri(ownedId);
		removeOwnedSession(ownedId);
		if (this.controlledSelectionOwnedId === ownedId) {
			this.cancelSelection();
			this.controlledSelectionOwnedId = null;
			this.sessionSelectionLayers.clearTreeView();
			this.sessionSelectionLayers.clearChatHistory();
		}
	}

	async dispose(): Promise<void> {
		this.cancelSelection();
		this.newSession.dispose();
		const editorDisposal = this.editorSessions.dispose();
		this.controlledSelectionOwnedId = null;
		this.expandedPathsByRoot = {};
		this.activeWorkspaceSnapshot = null;
		this.sessionSelectionLayers.clearTreeView();
		this.sessionSelectionLayers.clearChatHistory();
		await editorDisposal;
	}

	private beginSelection(): SessionSelectionOwner {
		this.selectionAbort?.abort();
		const controller = new AbortController();
		this.selectionAbort = controller;
		const owner = {
			generation: ++this.selectionGeneration,
			signal: controller.signal,
		};
		return owner;
	}

	private cancelSelection(): void {
		this.selectionAbort?.abort();
		this.selectionAbort = null;
		this.pendingSelectionOwnedId = null;
		this.selectionGeneration += 1;
	}

	private isCurrent(owner: SessionSelectionOwner): boolean {
		return !owner.signal.aborted && owner.generation === this.selectionGeneration;
	}

	private async materializeSelection(
		session: OwnedSession,
		root: string,
		owner: SessionSelectionOwner,
		displayedChatOwnedId: string | null,
		needsWorkspaceState: boolean,
	): Promise<void> {
		if (!this.isCurrent(owner)) return;

		this.newSession.abandonDraftForSessionSwitch(session.ownedId);
		await this.sessionSelectionLayers.selectSession(session, displayedChatOwnedId, owner);
		if (!this.isCurrent(owner)) return;
		this.activeRootAvailable = Boolean(this.sessionSelectionLayers.treeRoot);
		shellPanels.sessionPicked(this.activeRootAvailable);
		const snapshot = await this.editorSessions.restoreEditorWorkspaceForSession(
			session.ownedId,
			this.sessionSelectionLayers.treeRoot,
			Boolean(this.sessionSelectionLayers.treeRoot),
			owner.signal,
		);
		if (!this.isCurrent(owner)) return;
		this.activeWorkspaceSnapshot = snapshot;
		if (!root) return;
		if (needsWorkspaceState) await this.loadExpandedPaths(session.ownedId, root, owner);
	}

	private async loadExpandedPaths(
		ownedId: string,
		root: string,
		owner: SessionSelectionOwner,
	): Promise<void> {
		try {
			countInvoke("read_agent_conversation_workspace_expanded_paths");
			const paths = await readAgentConversationWorkspaceExpandedPathsFromTauri(ownedId, root, owner.signal);
			if (!this.isCurrent(owner)) return;
			this.expandedPathsByRoot = { [root]: paths };
		} catch {
			// The session may have changed while the uncancellable invoke was in flight.
		}
	}

	private async writeExpandedPathsAfter(
		previousWrite: Promise<void> | null,
		ownedId: string,
		projectRoot: string,
		paths: readonly string[],
	): Promise<void> {
		try {
			if (previousWrite) await previousWrite;
		} catch {
			// Keep later writes ordered even if an earlier persistence call failed.
		}
		countInvoke("write_agent_conversation_workspace_expanded_paths");
		await writeAgentConversationWorkspaceExpandedPathsFromTauri(ownedId, projectRoot, paths);
	}

	private async ignoreWorkspaceWriteFailure(write: Promise<void>): Promise<void> {
		try {
			await write;
		} catch {
			// Persistence is best-effort; the current projection remains in memory.
		}
	}
}
