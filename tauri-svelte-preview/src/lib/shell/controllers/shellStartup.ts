import { hydrateSettings } from '$lib/settingsStore.svelte';
import { applyStoredFonts, clearFonts } from '../themes/fontService';
import { applyStoredTheme, clearTheme } from '../themes/themeService';
import {
	ACTIVE_OWNED_SESSION_SETTING_KEY,
	hydrateOwned,
	rail
} from '../stores/sessionRailStore.svelte';
import {
	listAgentConversationSessionsFromTauri,
	listRemoteAgentConversationSessionsFromTauri,
	readAssemblySettingFromTauri
} from '$lib/tauriSource';
import { ownedSessionFromBackend } from '../ownedSessions';
import { startConversationEvents, stopConversationEvents } from '../conversation/conversationService';
import { shellPanels } from '../shellPanels';

export interface ShellStartupOptions {
	onSelectInitial(ownedId: string, stopSignal: AbortSignal): Promise<void>;
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
		if (!shellActive(generation, controller.signal)) return;
		const [storedSessions, storedRemoteSessions, storedActiveOwnedId] = await Promise.all([
			listAgentConversationSessionsFromTauri(),
			listRemoteAgentConversationSessionsFromTauri(controller.signal).catch(() => []),
			readAssemblySettingFromTauri(ACTIVE_OWNED_SESSION_SETTING_KEY).catch(() => null)
		]);
		if (!shellActive(generation, controller.signal)) return;
		const projected = (storedSessions ?? []).map(ownedSessionFromBackend);
		const projectedRemote = (storedRemoteSessions ?? []).map(ownedSessionFromBackend);
		const remoteIds = new Set(projectedRemote.map((session) => session.ownedId));
		const combined = [...projected.filter((session) => !remoteIds.has(session.ownedId)), ...projectedRemote];

		hydrateOwned(combined);

		const rememberedOwnedId = typeof storedActiveOwnedId === 'string' ? storedActiveOwnedId : null;
		const initial = combined.find((session) => session.ownedId === rememberedOwnedId) ?? combined[0] ?? null;
		if (initial) {
			if (!shellActive(generation, controller.signal)) return;
			await options.onSelectInitial(initial.ownedId, controller.signal);
			if (!shellActive(generation, controller.signal)) return;
		}

		void startConversationEventsForOwner(generation, controller.signal);
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

async function hydrateShellSettings(generation: number, stopSignal: AbortSignal): Promise<void> {
	try {
		if (!shellActive(generation, stopSignal)) return;
		await hydrateSettings();
		if (!shellActive(generation, stopSignal)) return;
		applyStoredTheme();
		applyStoredFonts();
	} catch {
		// Stored settings are optional; startup continues with defaults.
	}
}

function shellActive(generation: number, stopSignal: AbortSignal): boolean {
	return !stopSignal.aborted && generation === shellGeneration;
}

async function startConversationEventsForOwner(generation: number, stopSignal: AbortSignal): Promise<void> {
	try {
		if (!shellActive(generation, stopSignal)) return;
		await startConversationEvents();
		if (!shellActive(generation, stopSignal)) return;
	} catch (error) {
		if (!shellActive(generation, stopSignal)) return;
		rail.error = `shell event setup failed: ${error instanceof Error ? error.message : String(error)}`;
	}
}
