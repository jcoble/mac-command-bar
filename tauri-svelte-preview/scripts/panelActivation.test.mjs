import assert from 'node:assert/strict';

import { createPanelActivation } from '../src/lib/shell/panelActivation.ts';

/** Records every loader call so a test can say exactly what ran. */
function recorder() {
  const calls = [];
  return {
    calls,
    activators: {
      editor: (root) => calls.push(['editor', root]),
      git: (root) => calls.push(['git', root]),
      browser: () => calls.push(['browser', null]),
      explorer: (root) => calls.push(['explorer', root]),
      context: (selection) => calls.push(['context', selection.root])
    }
  };
}

function selection(root, projects = []) {
  return { root, projects };
}

// Launch: the dock announcing a restored tab, and start-up re-attaching a
// session, must both load nothing at all.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.panelShown('browser');
  panels.panelShown('editor');
  panels.sessionPicked();
  assert.deepEqual(calls, [], 'nothing loads before the shell says launch is over');
  assert.deepEqual(panels.loadedPanels(), [], 'and nothing counts as opened');
}

// A tab brought to the front after launch loads that tab, and only that tab.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.panelShown('editor');
  assert.deepEqual(calls, [['editor', '/repo/one']], 'only the tab the user opened loads');
  panels.panelShown('browser');
  assert.deepEqual(calls.at(-1), ['browser', null], 'the browser panel takes no project');
  assert.deepEqual(panels.loadedPanels(), ['editor', 'browser']);
}

// The terminal tab is never re-loaded from here, and an unknown id is ignored.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.panelShown('session');
  panels.panelShown('something-else');
  assert.deepEqual(calls, [], 'the terminal tab and unknown tabs load nothing');
}

// Source control is a section of the left column, not a tab: nothing can bring
// it to the front, so a tab activation named "git" loads nothing and does not
// count as an opened tab either.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.panelShown('git');
  assert.deepEqual(calls, [], 'source control has no tab to be shown');
  assert.deepEqual(panels.loadedPanels(), []);
}

// With no session picked yet, a tab still opens — with no project.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(''));
  panels.allowPanelLoads();
  panels.panelShown('editor');
  panels.panelShown('browser');
  assert.deepEqual(calls, [
    ['editor', null],
    ['browser', null]
  ]);
}

// Picking a session loads the panels that come with it. The shell does not open
// on the Source control view, so source control is NOT one of them.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(
    calls,
    [
      ['explorer', '/repo/one'],
      ['context', '/repo/one']
    ],
    'source control out of view costs nothing on a pick'
  );
  assert.deepEqual(panels.loadedPanels(), ['explorer', 'context']);
}

// Bringing source control into view after a session is picked loads it once, for
// the folder that session is in. Looking away and back does not read the
// repository again.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sessionPicked();
  calls.length = 0;

  panels.sourceControlVisible(true);
  assert.deepEqual(calls, [['git', '/repo/one']], 'bringing it into view loads it');
  assert.deepEqual(panels.loadedPanels(), ['explorer', 'context', 'git']);

  panels.sourceControlVisible(false);
  panels.sourceControlVisible(true);
  assert.equal(calls.length, 1, 'coming back to the same folder reads nothing again');
}

// Bringing it into view BEFORE any session is picked loads nothing — there is no
// folder to read yet. The pick that follows is what loads it.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sourceControlVisible(true);
  assert.deepEqual(calls, [], 'source control in view with no session picked loads nothing');
  assert.deepEqual(panels.loadedPanels(), []);

  panels.sessionPicked();
  assert.deepEqual(calls, [
    ['explorer', '/repo/one'],
    ['git', '/repo/one'],
    ['context', '/repo/one']
  ]);
}

// Nothing about the view can load anything before launch is over, whatever order
// the two arrive in.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.sourceControlVisible(true);
  panels.sessionPicked();
  panels.sourceControlVisible(true);
  assert.deepEqual(calls, [], 'launch loads nothing, in view or not');
}

// With source control in view, changing session re-points it along with the file
// tree and the context cards.
{
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowSessionLoads();
  panels.sessionPicked();
  panels.sourceControlVisible(true);
  calls.length = 0;

  root = '/repo/two';
  panels.sessionPicked();
  assert.deepEqual(calls, [
    ['explorer', '/repo/two'],
    ['git', '/repo/two'],
    ['context', '/repo/two']
  ]);
}

// Looking away, changing session, then coming back: the session change costs
// nothing while source control is out of view, and coming back reads the new
// folder rather than showing the old one.
{
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowSessionLoads();
  panels.sessionPicked();
  panels.sourceControlVisible(true);
  panels.sourceControlVisible(false);
  calls.length = 0;

  root = '/repo/two';
  panels.sessionPicked();
  assert.ok(
    !calls.some(([name]) => name === 'git'),
    'a session change while it is out of view reads no repository'
  );

  panels.sourceControlVisible(true);
  assert.deepEqual(calls.at(-1), ['git', '/repo/two'], 'coming back reads the new folder');
}

// A session with no project folder still loads the context cards, which are
// machine-wide, but has no folder to list files from. In view it still tells
// source control there is no repository to show.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(''));
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(calls, [['context', '']], 'no folder means no file listing');

  panels.sourceControlVisible(true);
  assert.deepEqual(calls.at(-1), ['git', null], 'and no repository either');
}

// Changing session re-points the tabs the user has opened, and leaves the rest.
{
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.panelShown('editor');
  calls.length = 0;

  root = '/repo/two';
  panels.sessionPicked();
  assert.deepEqual(calls, [
    ['explorer', '/repo/two'],
    ['context', '/repo/two'],
    ['editor', '/repo/two']
  ]);
  assert.ok(
    !calls.some(([name]) => name === 'browser'),
    'a tab the user never opened is not loaded by a session change'
  );
}

// The two gates are independent: opening tabs is allowed while start-up is
// still finishing, and picking a session then still works.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.panelShown('editor');
  panels.sessionPicked();
  assert.deepEqual(calls, [['editor', '/repo/one']], 'session picks are still switched off');
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(calls.slice(1), [
    ['explorer', '/repo/one'],
    ['context', '/repo/one'],
    ['editor', '/repo/one']
  ]);
}

// A repeated tab activation is passed on every time: the loaders are the ones
// that decide a repeat costs nothing, and this keeps a retry after a failure
// possible.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.panelShown('editor');
  panels.panelShown('editor');
  assert.equal(calls.length, 2);
  assert.deepEqual(panels.loadedPanels(), ['editor'], 'still one opened tab');
}

// Anything that wants to know whether a session pick would be honoured can ask.
// The page uses it to keep start-up from re-opening a session's files before
// launch is over; opening tabs is a different gate and does not answer for it.
{
  const { activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  assert.equal(panels.loadsAllowed(), false, 'session picks are ignored during launch');
  panels.allowPanelLoads();
  assert.equal(panels.loadsAllowed(), false, 'opening tabs says nothing about session picks');
  panels.allowSessionLoads();
  assert.equal(panels.loadsAllowed(), true, 'launch is over, so a pick is the user speaking');
}

console.log('panelActivation: all tests passed');
