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
} from '../../tauriSource';
import type { OwnedSession } from '../ownedSessions';
import { removeOwnedSession } from '../stores/sessionRailStore.svelte';

export class SessionSelectionController {
	controlledSelectionOwnedId = $state<string | null>(null);
	expandedPathsByRoot = $state.raw<Readonly<Record<string, readonly string[]>>>({});
	sessionSelectionLayers = new SessionSelectionLayers();
	activeRootAvailable = $state(true);
	activeRootRemote = $state(false);

	private workspaceWriteQueue: Promise<void> | null = null;
	private selectionGeneration = 0;
	private selectionAbort: AbortController | null = null;
	private selectionOwner: SessionSelectionOwner | null = null;

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
		return canonicalPath(session.cwd.trim() || (session.projectPath ?? "").trim());
	}

	get controlledEditorRootAvailable(): boolean {
		return (
			this.controlledSelectionOwnedId !== null &&
			Boolean(this.sessionSelectionLayers.treeRoot) &&
			!this.activeRootRemote
		);
	}

	get filesInspectionRoot(): string | null {
		return null;
	}

	get chatOwnedId(): string | null {
		return this.sessionSelectionLayers.chatOwnedId;
	}

	get selectionSignal(): AbortSignal | null {
		return this.selectionOwner?.signal ?? null;
	}

	async selectSession(ownedId: string): Promise<void> {
		const owner = this.beginSelection();
		this.controlledSelectionOwnedId = ownedId;
		setActiveOwned(ownedId);
		this.sessionSelectionLayers.clearTreeView();
		this.sessionSelectionLayers.clearChatHistory();

		const session = rail.owned.find((candidate) => candidate.ownedId === ownedId);
		if (session) {
			this.activeRootRemote = session.executionEnvironment === "remote";
			const root = canonicalPath(session.cwd.trim() || (session.projectPath ?? "").trim());
			this.expandedPathsByRoot = {};
			await this.materializeSelection(session, root, owner);
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

	dispose(): void {
		this.cancelSelection();
		this.controlledSelectionOwnedId = null;
		this.expandedPathsByRoot = {};
		this.sessionSelectionLayers.clearTreeView();
		this.sessionSelectionLayers.clearChatHistory();
	}

	private beginSelection(): SessionSelectionOwner {
		this.selectionAbort?.abort();
		const controller = new AbortController();
		this.selectionAbort = controller;
		const owner = {
			generation: ++this.selectionGeneration,
			signal: controller.signal,
		};
		this.selectionOwner = owner;
		return owner;
	}

	private cancelSelection(): void {
		this.selectionAbort?.abort();
		this.selectionAbort = null;
		this.selectionOwner = null;
		this.selectionGeneration += 1;
	}

	private isCurrent(owner: SessionSelectionOwner): boolean {
		return !owner.signal.aborted && owner.generation === this.selectionGeneration;
	}

	private async materializeSelection(
		session: OwnedSession,
		root: string,
		owner: SessionSelectionOwner,
	): Promise<void> {
		if (!this.isCurrent(owner)) return;

		await this.sessionSelectionLayers.selectSession(session, null, owner);
		if (!this.isCurrent(owner) || !root || this.activeRootRemote) return;
		await this.loadExpandedPaths(session.ownedId, root, owner);
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
