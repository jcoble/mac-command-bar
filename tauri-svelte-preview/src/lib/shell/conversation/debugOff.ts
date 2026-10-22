/* Memory hunt (TSK-1299): switch parts of the chat transcript off, one at a time, to find the part
   that holds memory. `ASSEMBLY_DEBUG_OFF=tools,markdown` (or `all`) is read once at startup, and the
   transcript checks `debugOff(name)` where that part mounts. `ASSEMBLY_DEBUG_SWITCH_SESSIONS=<ids>`
   makes the app switch between those chats 40 times, so a script can measure the same run each time.
   Neither is set in normal use, and then nothing changes. */
import { invoke } from '@tauri-apps/api/core';

export const DEBUG_OFF_NAMES = ['tools', 'tool-output', 'markdown', 'code', 'icons', 'effects', 'virtual', 'activity'] as const;
export type DebugOffName = (typeof DEBUG_OFF_NAMES)[number];

const SWITCHES = 40;
const SETTLE_MS = 20_000;
const SWITCH_MS = 1_500;

let off: ReadonlySet<DebugOffName> = new Set();
let sessions: string[] = [];

const list = (value: string): string[] => value.split(',').map((part) => part.trim()).filter(Boolean);

export function parseDebugOff(value: string): ReadonlySet<DebugOffName> {
  const names = list(value.toLowerCase());
  if (names.includes('all')) return new Set(DEBUG_OFF_NAMES);
  return new Set(DEBUG_OFF_NAMES.filter((name) => names.includes(name)));
}

export const parseDebugSessions = list;

export function debugOff(name: DebugOffName): boolean {
  return off.has(name);
}

/** Reads the switches once, before the first chat mounts. Outside the app there are none. */
export async function loadDebugOff(): Promise<void> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
  const read = await invoke<{ off: string; switchSessions: string }>('debug_off', { mark: null });
  off = parseDebugOff(read.off);
  sessions = parseDebugSessions(read.switchSessions);
  if (off.size) console.warn(`Chat parts switched off for the memory hunt: ${[...off].join(', ')}`);
}

/** The measured run: settle, switch between the chats, then say so in the backend log. */
export async function runDebugSwitches(select: (ownedId: string) => Promise<void>, signal: AbortSignal): Promise<void> {
  if (!sessions.length) return;
  await invoke('debug_off', { mark: 'ready' });
  await pause(SETTLE_MS, signal);
  for (let index = 0; index < SWITCHES && !signal.aborted; index += 1) {
    await select(sessions[index % sessions.length]);
    await pause(SWITCH_MS, signal);
  }
  if (!signal.aborted) await invoke('debug_off', { mark: 'switches done' });
}

async function pause(ms: number, signal: AbortSignal): Promise<void> {
  await new Promise<void>((resolve) => { const timer = setTimeout(resolve, ms); signal.addEventListener('abort', () => { clearTimeout(timer); resolve(); }, { once: true }); });
}
