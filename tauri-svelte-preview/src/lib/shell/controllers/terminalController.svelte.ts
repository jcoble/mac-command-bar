/**
 * Controller for terminal service, PTY tracking, and stacks execution.
 */
import { createTerminalService, tauriTerminalBackend } from '../terminalService';
import { loadXtermModules, makeTerminalView } from '../xtermFactory';
import { countInvoke } from '../devInvokeCounter.svelte';
import { updateOwnedSession } from '../stores/sessionRailStore.svelte';
import { noteTerminalExit, registerStackHandlers, clearStackHandlers, type StackStartRequest } from '../stacks/stackService';
import { recordStackStart } from '../stacks/stackStore.svelte';

export class TerminalController {
	service: ReturnType<typeof createTerminalService> | null = null;
	private disposed = false;
	private pendingHosts = new Map<string, (host: HTMLElement) => void>();

	registerHost(ownedId: string, host: HTMLElement): void {
		const pending = this.pendingHosts.get(ownedId);
		if (pending) {
			this.pendingHosts.delete(ownedId);
			pending(host);
		}
	}

	hostFor(ownedId: string): Promise<HTMLElement> {
		return new Promise<HTMLElement>((resolve) => {
			this.pendingHosts.set(ownedId, resolve);
		});
	}

	handleLayout(ownedId: string): void {
		this.service?.refit();
	}

	async initialize(onSelectSession: (ownedId: string) => void | Promise<void>): Promise<void> {
		const backend = tauriTerminalBackend(countInvoke);
		const modules = await loadXtermModules();
		if (this.disposed) return;

		this.service = createTerminalService({
			backend,
			createView: (host, hooks) => makeTerminalView(modules, host, hooks),
			onExit: (ownedId, payload) => {
				updateOwnedSession(ownedId, { state: 'exited' });
				noteTerminalExit(ownedId, { exitCode: payload.exitCode, signal: payload.signal });
			},
		});

		await this.service.attach();

		registerStackHandlers({
			onStartStack: async (req) => this.startStack(req),
			onStopStack: async (ownedId) => { await this.closeTerminal(ownedId); },
			onSelectSession: (ownedId) => { void onSelectSession(ownedId); },
		});
	}

	async startStack(request: StackStartRequest): Promise<string | null> {
		if (!this.service) return null;
		recordStackStart(request.stackId, request.stackId);
		return request.stackId;
	}

	async closeTerminal(ownedId: string): Promise<void> {
		if (!this.service) return;
		await this.service.closeOwned(ownedId);
	}

	dispose(): void {
		this.disposed = true;
		this.pendingHosts.clear();
		clearStackHandlers();
		this.service?.dispose();
		this.service = null;
	}
}
