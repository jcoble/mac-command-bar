/**
 * devInvokeCounter.svelte.ts — dev-only tally of backend `invoke` calls.
 *
 * A runes `$state` counter the /next shell bumps on every Tauri command it
 * issues, so the dev HUD can show "are we hammering the backend?" at a glance.
 * Pure bookkeeping: no IO, no persistence, no backend calls. Reads/writes are
 * tracked by Svelte's runes runtime.
 */

/**
 * Live invoke tallies. `total` is every counted call; `byCommand` breaks that
 * down per Tauri command name.
 */
export const invokeCounts = $state<{ total: number; byCommand: Record<string, number> }>({
  total: 0,
  byCommand: {}
});

/** Record one backend invoke of `command`. */
export function countInvoke(command: string): void {
  invokeCounts.total += 1;
  invokeCounts.byCommand[command] = (invokeCounts.byCommand[command] ?? 0) + 1;
}
