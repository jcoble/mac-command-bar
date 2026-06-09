import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const pageSource = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');

function blockFor(selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`, 'm').exec(pageSource);
  assert.ok(match?.groups?.body, `Missing style block for ${selector}`);
  return match.groups.body;
}

function assertDeclaration(selector, declaration) {
  assert.match(blockFor(selector), new RegExp(`(^|\\n)\\s*${declaration.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*;`), `${selector} should include ${declaration}`);
}

assertDeclaration('.source-browser-stack', 'overflow: hidden');
assertDeclaration('.source-list-panel', 'overflow: hidden');
assertDeclaration('.file-tree', 'overflow-y: auto');
assertDeclaration('.file-tree', 'overflow-x: hidden');
assertDeclaration('.file-tree', 'scrollbar-gutter: stable');
assertDeclaration('.file-tree', 'scrollbar-width: thin');
assert.ok(pageSource.includes('.file-tree::-webkit-scrollbar'), 'Tree view should style WebKit scrollbars');
assert.ok(pageSource.includes('function cancelSourceScan()'), 'Source scans should expose a cancel action');
assert.ok(
  pageSource.includes('onclick={scanning ? cancelSourceScan : () => scanProject(selectedProject, undefined, { force: true })}'),
  'Scan button should become a cancel button while a scan is running'
);
assert.ok(pageSource.includes("aria-label={scanning ? 'Stop source scan' : 'Scan source files'}"), 'Scan button should announce stop state');
assert.ok(pageSource.includes("<span>{scanning ? 'Stop' : 'Scan'}</span>"), 'Scan button label should switch to Stop while scanning');
