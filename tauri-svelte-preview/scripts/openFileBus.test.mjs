import assert from 'node:assert/strict';
import { onOpenFile, requestOpenFile } from '../src/lib/shell/openFileBus.ts';

// request before any subscriber is parked and delivered to the first subscriber
{
  requestOpenFile({ path: '/a.ts', line: 3 });
  const seen = [];
  const off = onOpenFile((request) => seen.push(request));
  assert.deepEqual(seen, [{ path: '/a.ts', line: 3 }], 'parked request delivered on subscribe');
  off();
}

// live delivery to all subscribers; unsubscribe stops delivery
{
  const a = [];
  const b = [];
  const offA = onOpenFile((request) => a.push(request.path));
  const offB = onOpenFile((request) => b.push(request.path));
  requestOpenFile({ path: '/b.ts' });
  assert.deepEqual(a, ['/b.ts']);
  assert.deepEqual(b, ['/b.ts']);
  offA();
  requestOpenFile({ path: '/c.ts' });
  assert.deepEqual(a, ['/b.ts'], 'unsubscribed listener not called');
  assert.deepEqual(b, ['/b.ts', '/c.ts']);
  offB();
}

// with a live subscriber nothing is parked: a later subscriber gets nothing old
{
  const first = [];
  const off = onOpenFile((request) => first.push(request.path));
  requestOpenFile({ path: '/d.ts' });
  off();
  const late = [];
  const offLate = onOpenFile((request) => late.push(request.path));
  assert.deepEqual(late, [], 'delivered requests are not replayed');
  offLate();
}

console.log('openFileBus: all tests passed');
