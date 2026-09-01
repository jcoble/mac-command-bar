import { hydrateSettings } from '$lib/settingsStore.svelte';
import { applyStoredFonts, clearFonts } from '../themes/fontService';
import { applyStoredTheme, clearTheme } from '../themes/themeService';
import { hydrateOwned, rail } from '../stores/sessionRailStore.svelte';
import {
	listAgentConversationSessionsFromTauri,
	listRemoteAgentConversationSessionsFromTauri
} from '$lib/tauriSource';
import { ownedSessionFromBackend } from '../ownedSessions';
import { startConversationEvents, stopConversationEvents } from '../conversation/conversationService';
import { shellPanels } from '../shellPanels';

export interface ShellStartupOptions {
	onSelectInitial(ownedId: string): Promise<void>;
}

let shellGeneration = 0;
let shellAbort: AbortController | null = null;

export async function startShell(options: ShellStartupOptions): Promise<void> {
	shellAbort?.abort();
	const controller = new AbortController();
	shellAbort = controller;
	const generation = ++shellGeneration;
	document.documentElement.classList.add('next-shell-document');
	applyStoredTheme();
	applyStoredFonts();

	void hydrateShellSettings(generation, controller.signal);

	try {
		const storedSessions = (await listAgentConversationSessionsFromTauri()) ?? [];
		if (!shellActive(generation, controller.signal)) return;
		const projected = storedSessions.map(ownedSessionFromBackend);

		hydrateOwned(projected);

		const initial = projected[0] ?? null;
		if (initial) {
			await options.onSelectInitial(initial.ownedId);
			if (!shellActive(generation, controller.signal)) return;
		}

		void startConversationEventsForOwner(generation, controller.signal);
		void hydrateRemoteSessionsForOwner(generation, controller.signal);
	} catch (error) {
		if (!shellActive(generation, controller.signal)) return;
		rail.error = `shell start-up failed: ${error instanceof Error ? error.message : String(error)}`;
	} finally {
		if (shellActive(generation, controller.signal)) shellPanels.allowSessionLoads();
	}
}

export function stopShell(): void {
	shellAbort?.abort();
	shellAbort = null;
	shellGeneration += 1;
	stopConversationEvents();
	clearTheme();
	clearFonts();
	document.documentElement.classList.remove('next-shell-document');
}

async function hydrateShellSettings(generation: number, signal: AbortSignal): Promise<void> {
	try {
		await hydrateSettings();
		if (!shellActive(generation, signal)) return;
		applyStoredTheme();
		applyStoredFonts();
	} catch {
		// Stored settings are optional; startup continues with defaults.
	}
}

function shellActive(generation: number, signal: AbortSignal): boolean {
	return !signal.aborted && generation === shellGeneration;
}

async function startConversationEventsForOwner(generation: number, signal: AbortSignal): Promise<void> {
	try {
		await startConversationEvents();
	} catch (error) {
		if (!shellActive(generation, signal)) return;
		rail.error = `shell event setup failed: ${error instanceof Error ? error.message : String(error)}`;
	}
}

async function hydrateRemoteSessionsForOwner(generation: number, signal: AbortSignal): Promise<void> {
	try {
		const storedSessions = (await listRemoteAgentConversationSessionsFromTauri(signal)) ?? [];
		if (!shellActive(generation, signal) || storedSessions.length === 0) return;
		const projected = storedSessions.map(ownedSessionFromBackend);
		const remoteIds = new Set(projected.map((session) => session.ownedId));
		hydrateOwned([...rail.owned.filter((session) => !remoteIds.has(session.ownedId)), ...projected]);
	} catch (error) {
		if (!shellActive(generation, signal)) return;
		rail.error = `remote session refresh failed: ${error instanceof Error ? error.message : String(error)}`;
	}
}
