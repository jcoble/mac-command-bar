/**
 * Controller for session selection, filesystem projections, and workspace expansion.
 */
import { rail, setActiveOwned } from '../stores/sessionRailStore.svelte';
import { SessionSelectionLayers } from '../sessionSelectionLayers.svelte';
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

	private workspaceWriteQueue: Promise<void> = Promise.resolve();
	private selectionGeneration = 0;

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
			: this.durableSessionRoot;
	}

	get filesProjectionOwnedId(): string | null {
		return this.sessionSelectionLayers.hasTreeProjection
			? this.sessionSelectionLayers.treeOwnedId
			: this.activeOwnedId;
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

	get sourceControlInspectionRoot(): string | null {
		return null;
	}

	async selectSession(ownedId: string): Promise<void> {
		this.controlledSelectionOwnedId = ownedId;
		setActiveOwned(ownedId);
		this.expandedPathsByRoot = {};

		const session = rail.owned.find((candidate) => candidate.ownedId === ownedId);
		if (session) {
			this.activeRootRemote = session.executionEnvironment === "remote";
			await this.sessionSelectionLayers.selectTreeOnly(session);
		} else {
			this.sessionSelectionLayers.clearTreeView();
		}
	}

	rememberExpandedPaths(root: string, paths: readonly string[]): void {
		const ownedId = this.controlledSelectionOwnedId;
		const projectRoot = canonicalPath(root);
		if (!ownedId || !projectRoot) return;

		this.expandedPathsByRoot = { [projectRoot]: [...paths] };
		const write = this.workspaceWriteQueue.then(async () => {
			countInvoke("write_agent_conversation_workspace_expanded_paths");
			await writeAgentConversationWorkspaceExpandedPathsFromTauri(ownedId, projectRoot, paths);
		});
		this.workspaceWriteQueue = write.catch(() => undefined);
	}

	async removeSession(ownedId: string): Promise<void> {
		await deleteAgentConversationSessionFromTauri(ownedId);
		removeOwnedSession(ownedId);
		if (this.controlledSelectionOwnedId === ownedId) {
			this.controlledSelectionOwnedId = null;
			this.sessionSelectionLayers.clearTreeView();
		}
	}
}
