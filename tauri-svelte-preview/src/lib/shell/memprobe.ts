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
  }),
  /**
   * UTF-16 character bytes reachable from each live conversation projection.
   * Large strings are allocated from bmalloc and report under "WebKit malloc"
   * rather than the JS heap, so a transcript can weigh hundreds of megabytes
   * while the heap and the DOM both look small.
   */
  bytes: () => {
    const seen = new Set<object>();
    const perSession: Record<string, { events: number; eventBytes: number; sessionBytes: number }> = {};
    for (const [ownedId, session] of Object.entries(conversationSessions)) {
      perSession[ownedId] = {
        events: session.loadedEvents.length,
        eventBytes: characterBytes(session.loadedEvents, seen),
        sessionBytes: characterBytes(session, seen)
      };
    }
    return perSession;
  }
};

function characterBytes(value: unknown, seen: Set<object>): number {
  if (typeof value === 'string') return value.length * 2;
  if (value === null || typeof value !== 'object') return 0;
  if (seen.has(value)) return 0;
  seen.add(value);
  let total = 0;
  for (const entry of Object.values(value as Record<string, unknown>)) total += characterBytes(entry, seen);
  return total;
}

type CounterKey = Exclude<keyof typeof memprobe, 'selectBailEarly' | 'sizes' | 'bytes'>;

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
