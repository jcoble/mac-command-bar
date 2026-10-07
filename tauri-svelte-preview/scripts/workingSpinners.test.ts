import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { SPINNER_IDS, pickSpinner } from '../src/lib/shell/components/conversation/workingSpinners.ts';

const spinnerDir = fileURLToPath(new URL('../src/lib/shell/components/conversation/spinners/', import.meta.url));

// Exactly the owner's 16 kept designs, each with its SVG file.
{
  assert.equal(SPINNER_IDS.length, 16);
  assert.equal(new Set(SPINNER_IDS).size, 16);
  const files = readdirSync(spinnerDir).filter((name) => name.endsWith('.svg')).map((name) => name.slice(0, -4)).sort();
  assert.deepEqual(files, [...SPINNER_IDS].sort());
}

// A session's seed always gives the same design, so its rail row and working row match.
{
  for (const seed of ['owned-a', 'owned-b', '0192f3c1-7a2e-7c11-9d1b-2f6a1c0e9b44', '']) {
    assert.equal(pickSpinner(seed), pickSpinner(seed));
  }
}

// Different sessions spread over the set.
{
  const picked = new Set(Array.from({ length: 20 }, (_, index) => pickSpinner(`session-${index}`)));
  assert.ok(picked.size >= 8, `20 seeds reached only ${picked.size} designs`);
}

// Without a seed the pick is random but always a real design.
{
  for (let index = 0; index < 50; index += 1) assert.ok(SPINNER_IDS.includes(pickSpinner()));
}

console.log('working spinners: ok');
