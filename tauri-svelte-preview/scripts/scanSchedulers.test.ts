import assert from 'node:assert/strict';
import test from 'node:test';
import { scanSource } from './scanSchedulers.ts';

test('rejects prohibited async and scheduler patterns', () => {
  const source = `
setTimeout(work, 1);
task.then(done).catch(failed).finally(cleanup);
const first = new Promise<void>(() => {});
const fastest = Promise.race(tasks);
const deferred = Promise.withResolvers<void>();
void (async () => work());
function load(): Promise<string> { return first; }
`;
  const categories = scanSource('src/fixture.ts', source).map((hit) => hit.category);

  assert.deepEqual(categories, [
    'scheduler',
    'promise-chain',
    'promise-chain',
    'promise-chain',
    'promise-wrapper',
    'promise-wrapper',
    'promise-wrapper',
    'anonymous-async-iife',
    'non-async-promise-implementation'
  ]);
});

test('rejects CSS :has selectors and Tailwind has variants', () => {
  const source = `
.region:HAS (.selected) { color: red; }
<div class="has-[:checked]:bg-blue-500"></div>
<div class="group-has-[a]:block"></div>
<div class="peer-has-[input]:hidden"></div>
<div class="has-checked:bg-red-500"></div>
<div class="group-has-checked/card:block"></div>
<div class="peer-has-checked:hidden"></div>
`;
  const hits = scanSource('src/fixture.svelte', source);

  assert.equal(hits.filter((hit) => hit.category === 'css-has-selector').length, 1);
  assert.equal(hits.filter((hit) => hit.category === 'tailwind-has-variant').length, 6);
});

test('allows async implementations, Promise interfaces and ordinary CSS', () => {
  const source = `
interface Loader { load(): Promise<string>; }
async function load(): Promise<string> { return await read(); }
const transform = async (): Promise<string> => await read();
.region .selected { color: green; }
<!-- removed has-[…] class -->
`;
  assert.deepEqual(scanSource('src/fixture.ts', source), []);
});

test('CSS files receive only CSS checks', () => {
  const ordinaryCss = '.timer::before { content: "setTimeout(() => task.then(done))"; }';
  assert.deepEqual(scanSource('src/fixture.css', ordinaryCss), []);
  assert.equal(scanSource('src/fixture.css', '.region:has(.selected) {}')[0]?.category, 'css-has-selector');
});
