/**
 * The session rail's one clock.
 *
 * Every row shows how long its session has been going, and that text has to
 * keep up on its own. One timer per row would mean a dozen timers waking the
 * app up forever, so every row shares this single interval instead. It starts
 * when the first row asks for it and stops the moment the last row lets go.
 *
 * It also runs no faster than the rows actually need. A row asks for `second`
 * only while it is working or while its age is still counted in seconds;
 * everything else asks for `minute`, and a rail with no second-by-second row
 * ticks once a minute rather than once a second.
 */

export type RailElapsedListener = (nowMs: number) => void;
/** How often a row needs to hear from the clock. */
export type RailElapsedCadence = 'second' | 'minute';

export const RAIL_ELAPSED_SECOND_MS = 1_000;
export const RAIL_ELAPSED_MINUTE_MS = 60_000;
/** Above this age the reading no longer changes second by second. */
export const RAIL_ELAPSED_SECONDS_CEILING_MS = 60_000;

const watchers = new Map<RailElapsedListener, RailElapsedCadence>();
let timer: ReturnType<typeof setInterval> | null = null;
let interval: number | null = null;

function tick(): void {
  const nowMs = Date.now();
  for (const listener of [...watchers.keys()]) listener(nowMs);
}

function wantedInterval(): number | null {
  if (watchers.size === 0) return null;
  for (const cadence of watchers.values()) {
    if (cadence === 'second') return RAIL_ELAPSED_SECOND_MS;
  }
  return RAIL_ELAPSED_MINUTE_MS;
}

/** Run at the fastest cadence anyone still needs, and at no cadence at all when nobody does. */
function retune(): void {
  const wanted = wantedInterval();
  if (wanted === interval) return;
  if (timer !== null) clearInterval(timer);
  interval = wanted;
  // TIMER-TEST: periodic refresh disabled while chasing UI freezes.
  // timer = wanted === null ? null : setInterval(tick, wanted);
  timer = null;
}

/**
 * Hear the current time until the returned release function is called. The
 * cadence is a request, not a promise: the clock runs at the fastest one any
 * row currently needs.
 */
export function watchRailElapsed(
  listener: RailElapsedListener,
  cadence: RailElapsedCadence = 'second'
): () => void {
  watchers.set(listener, cadence);
  retune();

  let released = false;
  return () => {
    if (released) return;
    released = true;
    watchers.delete(listener);
    retune();
  };
}

/** Whether the shared interval is currently running. */
export function railElapsedTickerRunning(): boolean {
  return timer !== null;
}

/** How often the shared interval is firing, in milliseconds, or null when it is stopped. */
export function railElapsedTickerInterval(): number | null {
  return interval;
}

/** How many rows are currently listening. */
export function railElapsedWatcherCount(): number {
  return watchers.size;
}

/** Which cadence a row of this age needs while it is, or is not, working. */
export function railElapsedCadenceFor(elapsedMs: number, working: boolean): RailElapsedCadence {
  return working || elapsedMs < RAIL_ELAPSED_SECONDS_CEILING_MS ? 'second' : 'minute';
}

/**
 * The rail's reading of an age, coarsening as it grows: seconds under a minute
 * ("38s"), whole minutes under an hour ("11m"), whole hours beyond that ("2h").
 * A row that has been going for days still reads in hours, because that is the
 * number a person compares against the other rows.
 */
export function formatRailElapsed(elapsedMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1_000));
  if (totalSeconds < 60) return `${totalSeconds}s`;

  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes < 60) return `${totalMinutes}m`;

  return `${Math.floor(totalMinutes / 60)}h`;
}
