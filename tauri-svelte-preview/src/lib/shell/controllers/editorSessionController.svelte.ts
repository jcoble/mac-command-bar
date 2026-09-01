/** Saves and restores only the editor portion of each session workspace. */
import {
	editorState,
	resetEditorState,
	restoreEditorFiles,
	setEditorProjectRoot,
} from '../editor/editorStore.svelte';
import {
	captureWorkspace,
	planWorkspaceRestore,
	type SessionWorkspaceSnapshot,
} from '../sessionWorkspaces';
import {
	readAgentConversationWorkspaceFromTauri,
	writeAgentConversationWorkspaceFromTauri,
} from '../../tauriSource';

export type EditorPanelLifecycle = {
	captureViewStates(paths: readonly string[]): Record<string, object>;
	workspaceOwnedPaths(): string[];
	restoreViewStates(files: readonly { path: string; viewState?: object }[]): void;
	releaseSessionResources(paths: readonly string[]): void;
};

export class EditorSessionController {
	private panel: EditorPanelLifecycle | null = null;
	private activeOwnedId: string | null = null;
	private activeSnapshot: SessionWorkspaceSnapshot | null = null;

	setPanel(panel: EditorPanelLifecycle | null): void {
		this.panel = panel;
	}

	/** One session-selection seam: commenting out this call disables editor switching. */
	async restoreEditorWorkspaceForSession(
		ownedId: string,
		projectRoot: string,
		rootAvailable: boolean,
		stopSignal: AbortSignal,
	): Promise<void> {
		if (stopSignal.aborted) return;
		if (this.activeOwnedId === ownedId) {
			setEditorProjectRoot(rootAvailable ? projectRoot : null);
			return;
		}

		await this.checkpointActiveWorkspace(stopSignal);
		if (stopSignal.aborted) return;
		this.releaseActiveEditorResources();

		if (stopSignal.aborted) return;
		const snapshot = await readAgentConversationWorkspaceFromTauri(ownedId);
		if (stopSignal.aborted) return;
		this.activeOwnedId = ownedId;
		this.activeSnapshot = snapshot;
		setEditorProjectRoot(rootAvailable ? projectRoot : null);
		if (!rootAvailable) {
			this.panel?.restoreViewStates([]);
			resetEditorState();
			return;
		}

		const plan = planWorkspaceRestore(snapshot);
		this.panel?.restoreViewStates(plan.openFiles);
		if (plan.openFiles.length > 0) restoreEditorFiles(plan.openFiles, plan.activePath);
		else resetEditorState();
	}

	async dispose(): Promise<void> {
		const stopSignal = new AbortController().signal;
		const checkpoint = this.checkpointActiveWorkspace(stopSignal);
		this.releaseActiveEditorResources();
		this.activeOwnedId = null;
		this.activeSnapshot = null;
		await checkpoint;
		if (stopSignal.aborted) return;
	}

	private async checkpointActiveWorkspace(stopSignal: AbortSignal): Promise<void> {
		const ownedId = this.activeOwnedId;
		if (!ownedId || stopSignal.aborted) return;
		const panel = this.panel;
		const ownedPaths = panel?.workspaceOwnedPaths()
			?? editorState.openFiles.map((file) => file.path);
		const ownedPathSet = new Set(ownedPaths);
		const openFiles = editorState.openFiles.filter((file) => ownedPathSet.has(file.path));
		const activePath = editorState.activePath && ownedPathSet.has(editorState.activePath)
			? editorState.activePath
			: openFiles.at(-1)?.path ?? null;
		const viewStates = panel?.captureViewStates(ownedPaths);

		if (stopSignal.aborted) return;
		const latest = await readAgentConversationWorkspaceFromTauri(ownedId);
		if (stopSignal.aborted) return;
		const previous = latest ?? this.activeSnapshot;
		const editorCapture = captureWorkspace({
			openFiles,
			activePath,
			viewStates,
			selectedPath: previous?.selectedPath ?? null,
			scrollTop: previous?.scrollTop ?? 0,
			rightTab: previous?.rightTab ?? 'files',
		});
		const snapshot: SessionWorkspaceSnapshot = {
			...(previous ?? editorCapture),
			openPaths: editorCapture.openPaths,
			activePath: editorCapture.activePath,
		};
		if (editorCapture.fileStates) snapshot.fileStates = editorCapture.fileStates;
		else delete snapshot.fileStates;

		if (stopSignal.aborted) return;
		await writeAgentConversationWorkspaceFromTauri(ownedId, snapshot);
		if (stopSignal.aborted) return;
		this.activeSnapshot = snapshot;
	}

	private releaseActiveEditorResources(): void {
		const paths = editorState.openFiles.map((file) => file.path);
		this.panel?.releaseSessionResources(paths);
		if (!this.panel) resetEditorState();
		setEditorProjectRoot(null);
	}
}
