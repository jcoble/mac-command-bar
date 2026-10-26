/**
 * The page's one clock. Labels that turn a known start time into "Working for
 * 1m 12s" or "5m" read `pageClock.now` and ask it to advance only while they are
 * shown; one-shot waits (a debounce, an auto-close, a countdown step, a snooze
 * wake) go through `pageDeadline`. Between them they own the app's only timer,
 * armed for whichever is due first. Label ticks stop while the window is hidden
 * and catch up the moment it is visible; deadlines keep their time while hidden,
 * because a restart countdown or a snoozed card must still act.
 */
export const pageClock = $state({ now: Date.now() });

type Deadline = { atMs: number; fn: () => void };
const resolutions: number[] = [];
const deadlines = new Set<Deadline>();
let timer: ReturnType<typeof setTimeout> | undefined;
let nextTickAtMs = 0;
let listening = false;

function hidden(): boolean {
  return typeof document !== 'undefined' && document.visibilityState === 'hidden';
}

function ticking(): boolean {
  return resolutions.length > 0 && !hidden();
}

function refresh(): void {
  const now = Date.now();
  pageClock.now = now;
  nextTickAtMs = now + Math.min(...resolutions);
}

function arm(): void {
  clearTimeout(timer);
  timer = undefined;
  let atMs = ticking() ? nextTickAtMs : Infinity;
  for (const deadline of deadlines) atMs = Math.min(atMs, deadline.atMs);
  if (atMs !== Infinity) timer = setTimeout(fire, Math.max(0, atMs - Date.now()));
}

function fire(): void {
  timer = undefined;
  const now = Date.now();
  if (ticking() && now >= nextTickAtMs) refresh();
  const due = [...deadlines].filter((deadline) => deadline.atMs <= now);
  for (const deadline of due) deadlines.delete(deadline);
  // Re-arm before running callbacks, so one that throws cannot stop the clock.
  arm();
  for (const deadline of due) deadline.fn();
}

function listenForVisibility(): void {
  if (listening || typeof document === 'undefined') return;
  listening = true;
  document.addEventListener('visibilitychange', () => {
    if (ticking()) refresh();
    arm();
  });
}

/** Keep `pageClock.now` advancing at least every `resolutionMs` while the window is visible, until the returned stop is called. */
export function watchPageClock(resolutionMs: number): () => void {
  listenForVisibility();
  resolutions.push(resolutionMs);
  refresh();
  arm();
  let stopped = false;
  return () => {
    if (stopped) return;
    stopped = true;
    resolutions.splice(resolutions.indexOf(resolutionMs), 1);
    if (resolutions.length > 0) nextTickAtMs = pageClock.now + Math.min(...resolutions);
    arm();
  };
}

/** Call `fn` once at `atMs`, also while the window is hidden, unless the returned cancel is called first. */
export function pageDeadline(atMs: number, fn: () => void): () => void {
  listenForVisibility();
  const deadline: Deadline = { atMs, fn };
  deadlines.add(deadline);
  arm();
  return () => {
    if (deadlines.delete(deadline)) arm();
  };
}

/** Whether the clock's timer is armed; nothing should be when no label or deadline needs it. */
export function pageClockArmed(): boolean {
  return timer !== undefined;
}
