/**
 * Shell startup and teardown orchestrator.
 */
import { hydrateSettings, settings } from '$lib/settingsStore.svelte';
import { applyStoredFonts, clearFonts } from '../themes/fontService';
import { applyStoredTheme, clearTheme } from '../themes/themeService';
import {
	hydrateOwned,
	rail,
	setAvailable,
} from '../stores/sessionRailStore.svelte';
import {
	listAgentConversationSessionsFromTauri,
	listAgentSessionsFromTauri,
	listAgentSessionsFromLocalBridge,
	type AgentSession,
} from '$lib/tauriSource';
import { ownedSessionFromBackend, reconcileOwnedSessions } from '../ownedSessions';
import { shellPanels } from '../shellPanels';
import { tauriTerminalBackend } from '../terminalService';
import { countInvoke } from '../devInvokeCounter.svelte';
import type { TerminalController } from './terminalController.svelte';

export interface ShellStartupOptions {
	terminal: TerminalController;
	onSelectInitial(ownedId: string): Promise<void>;
}

export async function scanRail(): Promise<void> {
	rail.scanning = true;
	try {
		countInvoke('list_agent_sessions');
		const sessions: AgentSession[] =
			(await listAgentSessionsFromTauri()) ??
			(await listAgentSessionsFromLocalBridge()) ??
			[];
		setAvailable(sessions);
	} catch (error) {
		rail.error = `failed to scan available agents: ${error instanceof Error ? error.message : String(error)}`;
	} finally {
		rail.scanning = false;
	}
}

export async function startShell(options: ShellStartupOptions): Promise<void> {
	document.documentElement.classList.add('next-shell-document');
	applyStoredTheme();
	applyStoredFonts();

	void hydrateSettings().then(() => {
		applyStoredTheme();
		applyStoredFonts();
	}).catch(() => undefined);

	await options.terminal.initialize(options.onSelectInitial);

	try {
		const backend = tauriTerminalBackend(countInvoke);
		const live = (await backend.list()) ?? [];
		const storedSessions = (await listAgentConversationSessionsFromTauri()) ?? [];
		const projected = storedSessions.map(ownedSessionFromBackend);
		const { owned, reattachable } = reconcileOwnedSessions(projected, live);

		const attachable = [
			...reattachable,
			...owned.filter((entry) => entry.state === 'exited' && entry.ptySessionId),
		];

		hydrateOwned(owned);

		for (const session of attachable) {
			if (options.terminal.service) {
				options.terminal.service.trackExisting(session, null);
			}
		}

		const initial = attachable[0] ?? null;
		if (initial) {
			await options.onSelectInitial(initial.ownedId);
		}

		await scanRail();
	} catch (error) {
		rail.error = `shell start-up failed: ${error instanceof Error ? error.message : String(error)}`;
	} finally {
		shellPanels.allowSessionLoads();
	}
}

export function stopShell(options: { terminal: TerminalController }): void {
	options.terminal.dispose();
	clearTheme();
	clearFonts();
	document.documentElement.classList.remove('next-shell-document');
}
