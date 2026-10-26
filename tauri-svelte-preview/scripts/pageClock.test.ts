import assert from 'node:assert/strict';
import { mock } from 'node:test';
import type { RestartCountdown } from '../src/lib/shell/providers/providerUpdateService.svelte.ts';

mock.timers.enable({ apis: ['setTimeout', 'Date'], now: 1_000_000 });
// Node's tick moves Date to the end before firing, so advance in small steps.
function advance(ms: number): void {
  for (let step = 0; step < ms; step += 250) mock.timers.tick(250);
}
let onVisibility: () => void = () => {};
const fakeDocument = {
  visibilityState: 'visible',
  addEventListener: (_type: string, listener: () => void) => { onVisibility = listener; }
};
(globalThis as unknown as { document: typeof fakeDocument }).document = fakeDocument;
function setHidden(hidden: boolean): void {
  fakeDocument.visibilityState = hidden ? 'hidden' : 'visible';
  onVisibility();
}

globalThis.$state = <T>(value: T): T => value;
const { pageClock, pageDeadline, pageWait, watchPageClock, pageClockArmed } = await import('../src/lib/pageClock.ts');
const { startRestartCountdown, stepRestartCountdown } = await import('../src/lib/shell/providers/providerUpdateService.svelte.ts');

// Subscribing refreshes the clock at once, then it advances at the watched resolution.
advance(5_000);
const stopSlow = watchPageClock(15_000);
assert.equal(pageClock.now, 1_005_000, 'subscribing refreshes now');
advance(1_000);
assert.equal(pageClock.now, 1_005_000, 'a 15 s watcher does not advance every second');

// The finest resolution wins while it is watched.
const stopFast = watchPageClock(1_000);
advance(1_000);
assert.equal(pageClock.now, 1_007_000, 'a 1 s watcher advances every second');
stopFast();
stopFast();
advance(1_000);
assert.equal(pageClock.now, 1_007_000, 'back to the 15 s resolution once the fast watcher stops');
advance(14_000);
assert.equal(pageClock.now, 1_022_000, 'the slow watcher still advances');

// A hidden window stops label ticks; becoming visible refreshes at once.
setHidden(true);
assert.equal(pageClockArmed(), false, 'hidden with only label watchers arms nothing');
advance(60_000);
const hiddenNow = pageClock.now;
setHidden(false);
assert.ok(pageClock.now >= hiddenNow + 60_000, 'becoming visible refreshes now');
stopSlow();
assert.equal(pageClockArmed(), false, 'the last watcher leaving disarms the clock');

// Deadlines fire once, also while hidden, and a cancelled one never fires.
const fired: string[] = [];
setHidden(true);
pageDeadline(Date.now() + 500, () => fired.push('a'));
const cancel = pageDeadline(Date.now() + 700, () => fired.push('b'));
cancel();
advance(1_000);
assert.deepEqual(fired, ['a'], 'a deadline fires while hidden; a cancelled one does not');
assert.equal(pageClockArmed(), false, 'nothing is armed once every deadline has fired');
setHidden(false);

// The restart warning re-arms one deadline per step and restarts exactly once.
let countdown: RestartCountdown | null = startRestartCountdown();
let restarts = 0;
let cancelStep = (): void => {};
function arm(): void {
  cancelStep();
  if (!countdown) return;
  cancelStep = pageDeadline(Date.now() + 1_000, () => {
    if (!countdown) return;
    const next = stepRestartCountdown(countdown, 'tick');
    countdown = next.phase === 'counting' ? next : null;
    if (next.phase === 'finished') restarts += 1;
    arm();
  });
}
arm();
advance(30_000);
assert.equal(restarts, 1, 'the countdown restarts exactly once');
countdown = startRestartCountdown();
arm();
advance(2_000);
countdown = stepRestartCountdown(countdown, 'cancel').phase === 'counting' ? countdown : null;
arm();
advance(30_000);
assert.equal(restarts, 1, 'a cancelled countdown never restarts');

// pageWait ends after its time, or at once on abort.
let waited = false;
const wait = pageWait(300, new AbortController().signal);
advance(250);
await Promise.resolve();
assert.equal(waited, false);
advance(250);
await wait;
waited = true;
const stopWait = new AbortController();
const abortedWait = pageWait(60_000, stopWait.signal);
stopWait.abort();
await abortedWait;
assert.equal(pageClockArmed(), false, 'an aborted wait leaves nothing armed');

mock.timers.reset();
console.log('page clock checks passed');
