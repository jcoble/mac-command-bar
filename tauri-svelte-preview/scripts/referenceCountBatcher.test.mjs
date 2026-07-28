/**
 * Covers the batching and remembering behind the editor's "N references"
 * margin numbers. The rules live in `referenceCountBatcher.ts` (no Svelte
 * state, no backend calls) precisely so this test can run under plain Node —
 * `sourceIntelligence.ts` only wires them to the real backend.
 */
import assert from 'node:assert/strict';
import {
  countFromResult,
  createCountMemory,
  createReferenceCountBatcher
} from '../src/lib/shell/editor/referenceCountBatcher.ts';

const resultFor = (counts, approximate = false) => ({
  counts,
  approximate,
  scannedFiles: 10,
  elapsedMs: 5
});

const settle = () => new Promise((resolve) => setTimeout(resolve, 20));

// every symbol asked about while the window is open travels in one request
{
  const asked = [];
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences(symbolNames) {
      asked.push(symbolNames);
      return Promise.resolve(resultFor({ Alpha: 3, Beta: 7, Gamma: 0 }));
    }
  });

  const counts = await Promise.all([
    batcher.count('Alpha'),
    batcher.count('Beta'),
    batcher.count('Gamma')
  ]);

  assert.deepEqual(counts, [3, 7, 0]);
  assert.equal(asked.length, 1, 'three symbols must cost one request, not three');
  assert.deepEqual(asked[0], ['Alpha', 'Beta', 'Gamma']);
}

// the same symbol asked about twice in one window is asked about once
{
  const asked = [];
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences(symbolNames) {
      asked.push(symbolNames);
      return Promise.resolve(resultFor({ Alpha: 4 }));
    }
  });

  const counts = await Promise.all([batcher.count('Alpha'), batcher.count('Alpha')]);

  assert.deepEqual(counts, [4, 4]);
  assert.deepEqual(asked, [['Alpha']]);
}

// a counted symbol is remembered, so typing does not re-ask for it
{
  let requests = 0;
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences() {
      requests += 1;
      return Promise.resolve(resultFor({ Alpha: 2 }));
    }
  });

  assert.equal(await batcher.count('Alpha'), 2);
  assert.equal(await batcher.count('Alpha'), 2);
  assert.equal(requests, 1);

  // …until the project changes underneath it
  batcher.forget();
  assert.equal(await batcher.count('Alpha'), 2);
  assert.equal(requests, 2);
}

// counts above the margin's ceiling are reported as the ceiling ("50+")
{
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences: () => Promise.resolve(resultFor({ Alpha: 812 }))
  });

  assert.equal(await batcher.count('Alpha'), 50);
}

// a pass that ran out of time reports "not counted", not "no references"
{
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences: () => Promise.resolve(resultFor({ Alpha: 6, Beta: 0 }, true))
  });

  const [alpha, beta] = await Promise.all([batcher.count('Alpha'), batcher.count('Beta')]);
  assert.equal(alpha, 6, 'a symbol the pass did reach still gets its number');
  assert.equal(beta, null, 'a zero from a pass that gave up early means "unknown"');
}

// nothing was counted at all: every waiting symbol hears "unknown"
{
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences: () => Promise.resolve(null)
  });

  assert.deepEqual(await Promise.all([batcher.count('Alpha'), batcher.count('Beta')]), [
    null,
    null
  ]);
}

// a failed request answers "unknown" rather than leaving the margin hanging
{
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences: () => Promise.reject(new Error('backend is gone'))
  });

  assert.equal(await batcher.count('Alpha'), null);
}

// an unknown answer is not remembered, so the next look can still find one
{
  let requests = 0;
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences() {
      requests += 1;
      return Promise.resolve(requests === 1 ? null : resultFor({ Alpha: 9 }));
    }
  });

  assert.equal(await batcher.count('Alpha'), null);
  assert.equal(await batcher.count('Alpha'), 9);
  assert.equal(requests, 2);
}

// a symbol asked about after the window closed opens a new one
{
  const asked = [];
  const batcher = createReferenceCountBatcher({
    windowMs: 5,
    cacheMs: 30_000,
    maxCount: 50,
    countReferences(symbolNames) {
      asked.push(symbolNames);
      return Promise.resolve(resultFor({ Alpha: 1, Beta: 2 }));
    }
  });

  assert.equal(await batcher.count('Alpha'), 1);
  await settle();
  assert.equal(await batcher.count('Beta'), 2);
  assert.deepEqual(asked, [['Alpha'], ['Beta']]);
}

// a remembered count is let go once it is old enough
{
  let clock = 1_000;
  const memory = createCountMemory({ cacheMs: 30_000, now: () => clock });

  memory.remember('Alpha', 5);
  assert.equal(memory.get('Alpha'), 5);

  clock += 29_999;
  assert.equal(memory.get('Alpha'), 5, 'still good just before it expires');

  clock += 1;
  assert.equal(memory.get('Alpha'), undefined, 'let go once it is old enough');
  assert.equal(memory.size, 0, 'and dropped rather than kept around');
}

// forgetting empties the memory
{
  const memory = createCountMemory({ cacheMs: 30_000 });
  memory.remember('Alpha', 5);
  memory.forget();
  assert.equal(memory.get('Alpha'), undefined);
}

// reading one symbol out of a pass's result
{
  assert.equal(countFromResult(resultFor({ Alpha: 3 }), 'Alpha', 50), 3);
  assert.equal(countFromResult(resultFor({ Alpha: 3 }), 'Beta', 50), null);
  assert.equal(countFromResult(null, 'Alpha', 50), null);
  assert.equal(countFromResult(resultFor({ Alpha: 0 }), 'Alpha', 50), 0);
  assert.equal(countFromResult(resultFor({ Alpha: 0 }, true), 'Alpha', 50), null);
  assert.equal(countFromResult(resultFor({ Alpha: 4 }, true), 'Alpha', 50), 4);
}

console.log('referenceCountBatcher tests passed');
