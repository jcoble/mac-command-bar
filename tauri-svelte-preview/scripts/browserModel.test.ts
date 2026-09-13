import assert from 'node:assert/strict';

globalThis.$state = (value) => value;

const { createBrowserWorkspace } = await import('../src/lib/shell/browser/browserTypes.ts');
const { createBrowserModel } = await import(
  '../src/lib/shell/browser/browserModel.ts'
);
const { hostRectFitsPanel } = await import(
  '../src/lib/shell/panels/browser/browserPanelBounds.ts'
);

assert.equal(
  hostRectFitsPanel(
    { x: 1200, y: 100, width: 300, height: 700 },
    { x: 1197, y: 3, width: 306, height: 894 },
    { x: 363, y: 3, width: 828, height: 894 }
  ),
  true,
  'a page contained by the tools region may be placed'
);
assert.equal(
  hostRectFitsPanel(
    { x: 3, y: 100, width: 1497, height: 700 },
    { x: 3, y: 3, width: 1500, height: 894 },
    { x: 363, y: 3, width: 828, height: 894 }
  ),
  false,
  'a transient tools rectangle overlapping the center may not cover the shell'
);

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
const localFile = model.navigateActiveBrowserTab('file:///Users/me/My Report.html');
assert.equal(localFile?.url, 'file:///Users/me/My%20Report.html');
assert.equal(localFile?.title, 'My Report.html');
assert.throws(
  () => model.navigateActiveBrowserTab('https://user:password@example.com/'),
  /http, https, or file/
);
assert.equal(model.workspace.tabs[identity].url, 'file:///Users/me/My%20Report.html');

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

assert.throws(() => model.createBrowserTab({ url: 'javascript:alert(1)' }), /http, https, or file/);
model.deactivateBrowserWorkspace();
assert.equal(model.workspace.activated, false);

console.log('browserModel: all tests passed');
