import { tick } from 'svelte';
import { createSubscriber } from 'svelte/reactivity';
import { deferred } from './deferred.ts';

/**
 * The page's one clock, and the only place it schedules anything. Labels that turn a known start time into "Working for
 * 1m 12s" or "5m" read `pageClock.now` and ask it to advance only while they are
 * shown; one-shot waits (a debounce, an auto-close, a countdown step, a snooze
 * wake) go through `pageDeadline`. Between them they own the app's only timer,
 * armed for whichever is due first. Label ticks stop while the window is hidden
 * and catch up the moment it is visible; deadlines keep their time while hidden,
 * because a restart countdown or a snoozed card must still act.
 *
 * Two waits that are not clocks live here too, so nothing else in the page
 * schedules work: `nextFrame` (a frame boundary, used only by the two WebKit
 * width workarounds in ShellFrame and layout/frame.ts) and `afterRender`
 * (Svelte's render wait, used only by ConversationTimeline to place the chat
 * after its virtual rows draw). Neither may be used per row or per item.
 */
let nowMs = Date.now();
let notify = (): void => {};
const subscribe = createSubscriber((update) => {
  notify = update;
  return () => (notify = () => {});
});
/** Reading `now` inside an effect or template re-runs it each time the clock advances. */
export const pageClock = {
  get now(): number {
    subscribe();
    return nowMs;
  }
};

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
  nowMs = now;
  notify();
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
    if (resolutions.length > 0) nextTickAtMs = nowMs + Math.min(...resolutions);
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

/** Wait `ms` through the page clock; aborting `signal` ends the wait at once. */
export async function pageWait(ms: number, signal: AbortSignal): Promise<void> {
  if (signal.aborted) return;
  const done = deferred();
  const finish = (): void => {
    cancel();
    signal.removeEventListener('abort', finish);
    done.resolve();
  };
  const cancel = pageDeadline(Date.now() + ms, finish);
  signal.addEventListener('abort', finish, { once: true });
  await done.promise;
}

/** Run `fn` at the next frame, unless the returned cancel is called first. WebKit width workarounds only. */
export function nextFrame(fn: () => void): () => void {
  const frame = requestAnimationFrame(fn);
  return () => cancelAnimationFrame(frame);
}

/** Wait until Svelte has drawn pending updates. ConversationTimeline's chat placement only. */
export async function afterRender(): Promise<void> {
  await tick();
}
