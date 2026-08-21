import assert from 'node:assert/strict';
import test from 'node:test';

import { createPanelActivation } from '../src/lib/shell/panelActivation.ts';

test('files_panel_hidden_skips_explorer_on_session_pick', () => {
  let root = '/repo/one';
  const explorerCalls: string[] = [];
  const panels = createPanelActivation(
    {
      editor: () => {},
      git: () => {},
      browser: () => {},
      explorer: (selectedRoot) => explorerCalls.push(selectedRoot),
      worktrees: () => {},
      stacks: () => {},
      problems: () => {}
    },
    () => ({ root, projects: [] })
  );

  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(explorerCalls, [], 'a hidden Files panel costs nothing on a session pick');

  panels.filesVisible(true);
  assert.deepEqual(explorerCalls, ['/repo/one'], 'showing Files loads the selected session root');

  panels.sessionPicked();
  assert.deepEqual(explorerCalls, ['/repo/one'], 'the same root is not loaded twice');

  root = '/repo/two';
  panels.sessionPicked();
  assert.deepEqual(explorerCalls, ['/repo/one', '/repo/two'], 'a new visible root is loaded');
});
