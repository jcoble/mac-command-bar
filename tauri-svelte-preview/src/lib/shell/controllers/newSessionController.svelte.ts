/** Owns the draft surface and the side effects of its first send. */
import { sessionWorkspaceRoot } from '../../workspacePaths';
import {
	getConversationSession,
	setConversationDraft,
	setConversationSendError,
} from '../conversation/conversationStore.svelte';
import { flushConversationSessionDraft, persistConversationSessionDraft, sendStructuredMessage } from '../conversation/conversationService';
import { rememberLastUsed } from '../newSession/projectRootsStore.svelte';
import {
	deriveThreadStartProjects,
	type ThreadStartRequest,
} from '../newSession/threadStartFlow';
import {
	createFreshSession,
	ownedSessionMetaForBackend,
} from '../ownedSessions';
import { shellPanels } from '../shellPanels';
import {
	addOwnedSession,
	rail,
	updateOwnedSession,
} from '../stores/sessionRailStore.svelte';
import { updateAgentConversationSessionMetaFromTauri } from '../../tauriSource';

export class NewSessionController {
	draftOpen = $state(false);
	draftProjectPath = $state<string | null>(null);

	private draftWork = new AbortController();
	private startingOwnedId: string | null = null;
	pendingFirstMessage = $state<{ ownedId: string; text: string } | null>(null);

	get stopSignal(): AbortSignal {
		return this.draftWork.signal;
	}

	get sessionRoots(): string[] {
		return deriveThreadStartProjects(
			rail.owned.filter((session) => session.executionEnvironment !== 'remote').map((session) => session.projectPath ?? session.cwd),
		).map((project) => project.path);
	}

	open(): void {
		this.stopDraftWork();
		this.draftWork = new AbortController();
		this.draftProjectPath = this.mostRecentProjectPath() ?? null;
		this.draftOpen = true;
	}

	close(): void {
		this.draftOpen = false;
		this.stopDraftWork();
	}

	/** One session-selection seam: commenting out this call leaves drafts alone. */
	abandonDraftForSessionSwitch(ownedId: string): void {
		this.draftOpen = false;
		if (this.startingOwnedId !== ownedId) this.stopDraftWork();
	}

	async start(
		request: ThreadStartRequest,
		selectSession: (ownedId: string) => Promise<void>,
		showSession: () => void,
	): Promise<string> {
		const stopSignal = this.stopSignal;
		if (stopSignal.aborted) throw new Error('the new session draft was closed');

		const owned = {
			...createFreshSession({
				cwd: request.cwd,
				title: request.title,
				executionEnvironment: request.executionEnvironment,
				remoteProfileId: request.remoteProfileId,
			}),
			agent: request.provider,
			projectPath: request.projectPath,
			branch: request.branch,
			resumeCommand: null,
			origin: 'app' as const,
		};
		this.startingOwnedId = owned.ownedId;
		this.pendingFirstMessage = { ownedId: owned.ownedId, text: request.prompt };
		this.draftOpen = false;
		addOwnedSession(owned);
		updateOwnedSession(owned.ownedId, {
			state: 'live',
			executionOwner: 'structured',
			runtimeState: 'starting',
			lastError: null,
		});
		shellPanels.allowSessionLoads();
		let promptAccepted = false;

		try {
			await selectSession(owned.ownedId);
			if (stopSignal.aborted) {
				updateOwnedSession(owned.ownedId, {
					state: 'background',
					executionOwner: 'stopped',
					runtimeState: 'closed',
				});
				return owned.ownedId;
			}

			showSession();
			await sendStructuredMessage(owned.ownedId, request.prompt, {
				reasoningEffort: request.reasoningEffort,
				model: request.model,
				approvalPolicy: request.approvalPolicy,
			});
			promptAccepted = true;
			// A switch may stop draft ownership, but it must not cancel the agent turn
			// that sendStructuredMessage has already handed to the background runtime.
			const stillPresented = !stopSignal.aborted;
			await this.persistOwnedMetadata(owned.ownedId);
			if (!stopSignal.aborted && request.executionEnvironment !== 'remote') rememberLastUsed(request.projectPath);
			updateOwnedSession(owned.ownedId, { runtimeState: 'ready', lastError: null });
			if (stillPresented && !stopSignal.aborted) showSession();
			return owned.ownedId;
		} catch (error) {
			const detail = this.describeError(error);
			if (!promptAccepted) {
				const currentDraft = getConversationSession(owned.ownedId)?.draft ?? '';
				const draft = currentDraft ? `${request.prompt}\n\n${currentDraft}` : request.prompt;
				setConversationDraft(owned.ownedId, draft);
				setConversationSendError(owned.ownedId, detail);
				persistConversationSessionDraft(owned.ownedId, draft);
				try { await flushConversationSessionDraft(owned.ownedId); } catch {
					// Keep the draft in memory and report the original send failure.
				}
			}
			updateOwnedSession(owned.ownedId, {
				state: 'exited',
				executionOwner: 'stopped',
				runtimeState: 'failed',
				lastError: detail,
			});
			rail.error = `could not start ${owned.agent} session: ${detail}`;
			throw error;
		} finally {
			if (this.startingOwnedId === owned.ownedId) this.startingOwnedId = null;
			if (this.pendingFirstMessage?.ownedId === owned.ownedId) this.pendingFirstMessage = null;
		}
	}

	dispose(): void {
		this.draftOpen = false;
		this.startingOwnedId = null;
		this.stopDraftWork();
	}

	private stopDraftWork(): void {
		this.draftWork.abort();
		this.pendingFirstMessage = null;
	}

	private mostRecentProjectPath(): string | undefined {
		const active = rail.owned.find((session) => session.ownedId === rail.activeOwnedId);
		if (active) return sessionWorkspaceRoot(active) || undefined;
		const activityTime = (value: string | null): number => {
			const parsed = Date.parse(value ?? '');
			return Number.isFinite(parsed) ? parsed : 0;
		};
		const recent = [...rail.owned].sort(
			(left, right) => activityTime(right.lastActivity) - activityTime(left.lastActivity),
		)[0];
		return recent ? sessionWorkspaceRoot(recent) || undefined : undefined;
	}

	private async persistOwnedMetadata(ownedId: string): Promise<void> {
		const session = rail.owned.find((entry) => entry.ownedId === ownedId);
		if (!session) return;
		await updateAgentConversationSessionMetaFromTauri({
			ownedId,
			model: session.model ?? null,
			effort: getConversationSession(ownedId)?.agentConfig.reasoningEffort ?? null,
			meta: ownedSessionMetaForBackend(session),
		});
	}

	private describeError(error: unknown): string {
		return error instanceof Error ? error.message : String(error);
	}
}
