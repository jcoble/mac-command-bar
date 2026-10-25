import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';
import ts from 'typescript';

const source = await readFile(
  new URL('../src/lib/shell/components/UtilityStrip.svelte', import.meta.url), 'utf8'
);
const script = source.match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];
const parsed = ts.createSourceFile('UtilityStrip.ts', script, ts.ScriptTarget.Latest, true);
const mount = parsed.statements.find((statement) => ts.isExpressionStatement(statement)
  && ts.isCallExpression(statement.expression) && statement.expression.expression.getText(parsed) === 'onMount');
const callback = mount.expression.arguments[0].getText(parsed);
const compiled = ts.transpileModule(`const mount = ${callback}; mount();`, {
  compilerOptions: { target: ts.ScriptTarget.ESNext }
}).outputText;
let refreshes = 0;
const intervals = new Set();
const listeners = new Map();
const document = {
  visibilityState: 'visible',
  addEventListener(name, listener) { listeners.set(name, listener); },
  removeEventListener(name, listener) {
    assert.equal(listeners.get(name), listener);
    listeners.delete(name);
  }
};
const cleanup = runInNewContext(compiled, {
  refreshResourceTotals() { refreshes += 1; },
  document,
  window: {
    setInterval(callback) { intervals.add(callback); return callback; },
    clearInterval(callback) { intervals.delete(callback); }
  }
});
assert.equal(refreshes, 1, 'mount reads the resource totals once');
assert.equal(intervals.size, 0, 'mount must not start repeating resource scans');
document.visibilityState = 'hidden';
listeners.get('visibilitychange')();
assert.equal(refreshes, 1, 'hiding the window must not scan resources');
document.visibilityState = 'visible';
listeners.get('visibilitychange')();
assert.equal(refreshes, 2, 'returning to the window refreshes the displayed totals');
cleanup();
assert.equal(listeners.size, 0, 'unmount removes the visibility listener');
assert.equal(intervals.size, 0, 'unmount leaves no interval');
console.log('utility strip resource refresh checks passed');
