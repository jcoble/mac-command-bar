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

// The native registry refuses a new tab unless it advances the workspace's
// highest generation, so selecting or closing an older tab must not lower it.
const tabsModel = createBrowserModel({ workspace: createBrowserWorkspace({ workspaceId: 'workspace-2' }) });
tabsModel.activateBrowserWorkspace();
const tabA = tabsModel.createBrowserTab({ url: 'https://example.com/' });
const tabB = tabsModel.createBrowserTab({ url: 'https://www.google.com/' });
tabsModel.selectBrowserTab(tabA.id);
const tabC = tabsModel.createBrowserTab({ url: 'https://example.org/' });
assert.ok(tabC.generation > tabB.generation, 'a tab opened after selecting an older tab advances the workspace generation');
tabsModel.closeBrowserTab(tabC.id);
const tabD = tabsModel.createBrowserTab({ url: 'https://example.net/' });
assert.ok(tabD.generation > tabC.generation, 'a tab opened after closing the newest tab still advances the workspace generation');

// Restoring saved IDs must not make the next plus click select an old tab.
const restoredModel = createBrowserModel({ workspace: createBrowserWorkspace({ workspaceId: 'restored' }) });
const seedTab = restoredModel.createBrowserTab();
const seedSequence = Number(seedTab.id.split('-').at(-1));
restoredModel.closeBrowserTab(seedTab.id);
const restoredIds = [1, 2, 3].map(offset => `browser-tab-${seedSequence + offset}`);
for (const tabId of restoredIds) restoredModel.createBrowserTab({ tabId, url: 'https://example.com/' });
restoredModel.selectBrowserTab(restoredIds[0]);
const fresh = restoredModel.createBrowserTab();
assert.deepEqual(restoredModel.workspace.tabOrder, [...restoredIds, fresh.id]);
assert.equal(fresh.url, '');
assert.ok(!restoredIds.includes(fresh.id));
restoredModel.closeBrowserTab(fresh.id);
const replacement = restoredModel.createBrowserTab();
assert.notEqual(replacement.id, fresh.id);
assert.deepEqual(restoredModel.workspace.tabOrder, [...restoredIds, replacement.id]);
const switchedModel = createBrowserModel({ workspace: createBrowserWorkspace({ workspaceId: 'switched' }) });
switchedModel.createBrowserTab({ tabId: 'browser-tab-200', url: 'https://example.org/' });
assert.notEqual(switchedModel.createBrowserTab().id, 'browser-tab-200');

console.log('browserModel: all tests passed');
