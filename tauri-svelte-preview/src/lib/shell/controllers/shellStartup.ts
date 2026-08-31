import { hydrateSettings } from '$lib/settingsStore.svelte';
import { applyStoredFonts, clearFonts } from '../themes/fontService';
import { applyStoredTheme, clearTheme } from '../themes/themeService';
import {
	hydrateOwned,
	rail,
} from '../stores/sessionRailStore.svelte';
import {
	listAgentConversationSessionsFromTauri,
} from '$lib/tauriSource';
import { ownedSessionFromBackend } from '../ownedSessions';
import { startConversationEvents, stopConversationEvents } from '../conversation/conversationService';
import { shellPanels } from '../shellPanels';

export interface ShellStartupOptions {
	onSelectInitial(ownedId: string): Promise<void>;
}

export async function refreshRailSessions(): Promise<void> {
	try {
		const storedSessions = (await listAgentConversationSessionsFromTauri()) ?? [];
		const projected = storedSessions.map(ownedSessionFromBackend);
		hydrateOwned(projected);
	} catch (error) {
		rail.error = `failed to refresh sessions: ${error instanceof Error ? error.message : String(error)}`;
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

	try {
		const storedSessions = (await listAgentConversationSessionsFromTauri()) ?? [];
		const projected = storedSessions.map(ownedSessionFromBackend);

		hydrateOwned(projected);

		const initial = projected[0] ?? null;
		if (initial) {
			await options.onSelectInitial(initial.ownedId);
		}

		void startConversationEvents();
	} catch (error) {
		rail.error = `shell start-up failed: ${error instanceof Error ? error.message : String(error)}`;
	} finally {
		shellPanels.allowSessionLoads();
	}
}

export function stopShell(): void {
	stopConversationEvents();
	clearTheme();
	clearFonts();
	document.documentElement.classList.remove('next-shell-document');
}
