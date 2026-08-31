/**
 * DEV-only lifecycle counters read by the leak-sweep Playwright script via
 * `window.__memprobe`, to check whether session-switch resources are created
 * without being released. Never runs outside `import.meta.env.DEV`.
 */
export const memprobe = {
  conversationCreates: 0,
  conversationEvicts: 0,
  hydrationsStarted: 0,
  hydrationsLandedStale: 0,
  selectEntries: 0,
  selectBailEarly: {} as Record<string, number>,
  editorReleases: 0,
  blobUrlsCreated: 0,
  blobUrlsRevoked: 0
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

if (import.meta.env?.DEV && typeof window !== 'undefined') {
  (window as Window & { __memprobe?: typeof memprobe }).__memprobe = memprobe;
}
