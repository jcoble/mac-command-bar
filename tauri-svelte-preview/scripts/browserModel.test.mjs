import assert from 'node:assert/strict';
import { createBrowserWorkspace } from '../src/lib/shell/browser/browserTypes.ts';
import {
  BrowserModelError,
  createBrowserModel
} from '../src/lib/shell/browser/browserModel.ts';

const workspace = createBrowserWorkspace({ workspaceId: 'workspace-1', ownedId: 'owned-1' });
const model = createBrowserModel({ workspace, now: () => '2026-08-05T00:00:00.000Z' });
model.activateBrowserWorkspace();
const tab = model.createBrowserTab({ url: 'https://example.com/start' });
const identity = tab.id;
const initialGeneration = tab.generation;

assert.equal(model.workspace.presentation, 'docked');
model.setBrowserPresentationMode('floating');
model.setBrowserPresentationMode('maximized');
assert.equal(model.workspace.tabs[identity].id, identity, 'presentation never recreates the native page');
model.minimizeBrowserToPrevious();
assert.equal(model.workspace.presentation, 'floating');
model.restoreBrowserToDock();
model.collapseBrowserToControl();
assert.equal(model.workspace.presentation, 'collapsed');
model.expandBrowserFrom('collapsed');
assert.equal(model.workspace.presentation, 'floating');
model.restoreBrowserToDock();

const navigated = model.navigateActiveBrowserTab('https://example.com/next');
assert.equal(navigated.id, identity);
assert.equal(navigated.generation, initialGeneration + 1);
assert.throws(
  () => model.navigateActiveBrowserTab('file:///Users/me/private.txt'),
  BrowserModelError
);
assert.throws(
  () => model.navigateActiveBrowserTab('https://user:password@example.com/'),
  /sign-in information/
);
assert.equal(model.workspace.tabs[identity].url, 'https://example.com/next');

const second = model.createBrowserTab({ url: ':5177', title: 'Local app' });
assert.notEqual(second.id, identity);
model.selectBrowserTab(identity);
model.setBrowserViewport('mobile-l');
assert.deepEqual(model.workspace.tabs[identity].viewport, {
  preset: 'mobile-l',
  width: 425,
  height: 812
});
model.closeBrowserTab(second.id);
assert.deepEqual(model.workspace.tabOrder, [identity]);

assert.throws(() => model.createBrowserTab({ url: 'javascript:alert(1)' }), /http and https|http or https/);
model.deactivateBrowserWorkspace();
assert.equal(model.workspace.activated, false);

console.log('browserModel: all tests passed');
