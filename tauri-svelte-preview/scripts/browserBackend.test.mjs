import assert from 'node:assert/strict';
import { createFakeBrowserBackend } from '../src/lib/shell/browser/browserBackend.ts';

const backend = createFakeBrowserBackend();
const target = { workspaceId: 'workspace-1', tabId: 'tab-1', generation: 1 };
backend.create_browser_tab({ ...target, url: 'https://example.com/' });
backend.navigate_browser_tab({ ...target, url: 'https://example.com/next' });
backend.go_back_browser_tab(target);
backend.go_forward_browser_tab(target);
backend.set_browser_tab_viewport({
  ...target,
  viewport: { preset: 'mobile-m', width: 375, height: 667 }
});
const capture = backend.capture_browser_viewport(target);

assert.deepEqual(
  backend.calls.map((call) => call.command),
  [
    'create_browser_tab',
    'navigate_browser_tab',
    'go_back_browser_tab',
    'go_forward_browser_tab',
    'set_browser_tab_viewport',
    'capture_browser_viewport'
  ]
);
assert.equal(backend.tabs.get('tab-1').url, 'https://example.com/next');
assert.equal(backend.tabs.get('tab-1').viewport.preset, 'mobile-m');
assert.deepEqual(capture.bytes.slice(0, 4), [137, 80, 78, 71]);
assert.equal(capture.sourceHash, 'fake-capture:tab-1:1');

backend.hide_browser_workspace({ workspaceId: 'workspace-1' });
assert.equal(backend.tabs.get('tab-1').visible, false);
backend.show_browser_tab(target);
assert.equal(backend.tabs.get('tab-1').visible, true);
backend.close_browser_tab(target);
assert.equal(backend.tabs.has('tab-1'), false);

console.log('browserBackend: all tests passed');
