/**
 * DEV-only lifecycle counters read by the leak-sweep Playwright script via
 * `window.__memprobe`, to check whether session-switch resources are created
 * without being released. Never runs outside `import.meta.env.DEV`.
 */
import { conversationSessions } from './conversation/conversationStore.svelte.ts';
import { resourceDiagnostics } from './resourceDiagnostics.svelte.ts';

export const memprobe = {
  conversationCreates: 0,
  conversationEvicts: 0,
  hydrationsStarted: 0,
  hydrationsLandedStale: 0,
  selectEntries: 0,
  selectBailEarly: {} as Record<string, number>,
  editorReleases: 0,
  blobUrlsCreated: 0,
  blobUrlsRevoked: 0,
  sizes: () => ({
    conversationSessions: Object.keys(conversationSessions).length,
    editorStates: resourceDiagnostics.codeMirrorEditorStates
  })
};

type CounterKey = Exclude<keyof typeof memprobe, 'selectBailEarly' | 'sizes'>;

export function bump(key: CounterKey): void {
  memprobe[key] += 1;
}

export function bumpBail(label: string): void {
  memprobe.selectBailEarly[label] = (memprobe.selectBailEarly[label] ?? 0) + 1;
}

export function memprobeSnapshot(): typeof memprobe {
  return { ...memprobe, selectBailEarly: { ...memprobe.selectBailEarly } };
}

if (import.meta.env.DEV) {
  (window as Window & { __memprobe?: typeof memprobe }).__memprobe = memprobe;
}
