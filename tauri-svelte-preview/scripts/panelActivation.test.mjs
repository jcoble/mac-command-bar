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
      context: (selection) => calls.push(['context', selection.root]),
      worktrees: (selection) => calls.push(['worktrees', selection.root]),
      stacks: (root) => calls.push(['stacks', root]),
      problems: (root) => calls.push(['problems', root])
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
// on the Source control view or the Context view, so neither of those is one of
// them.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(
    calls,
    [['explorer', '/repo/one']],
    'a view nobody is looking at costs nothing on a pick'
  );
  assert.deepEqual(panels.loadedPanels(), ['explorer']);
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
  assert.deepEqual(panels.loadedPanels(), ['explorer', 'git']);

  panels.sourceControlVisible(false);
  panels.sourceControlVisible(true);
  assert.equal(calls.length, 1, 'coming back to the same folder reads nothing again');
}

// The context cards follow the same rule: they are a view of the tool column,
// and they read the machine only while you can see them.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sessionPicked();
  calls.length = 0;

  panels.contextVisible(true);
  assert.deepEqual(calls, [['context', '/repo/one']], 'opening the context view loads the cards');
  assert.deepEqual(panels.loadedPanels(), ['explorer', 'context']);

  panels.contextVisible(false);
  panels.contextVisible(true);
  assert.equal(calls.length, 1, 'coming back to the same folder reads nothing again');
}

// Opening the context view BEFORE any session is picked loads nothing — there is
// nothing to point the cards at yet. The pick that follows is what loads them.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.contextVisible(true);
  assert.deepEqual(calls, [], 'the context view open with no session picked loads nothing');
  assert.deepEqual(panels.loadedPanels(), []);

  panels.sessionPicked();
  assert.deepEqual(calls, [
    ['explorer', '/repo/one'],
    ['context', '/repo/one']
  ]);
}

// Nothing about the context view can load anything before launch is over.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.contextVisible(true);
  panels.sessionPicked();
  panels.contextVisible(true);
  assert.deepEqual(calls, [], 'launch loads nothing, in view or not');
}

// With the context view open, changing session re-points the cards; with it
// closed, a session change costs nothing and coming back reads the new folder.
{
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowSessionLoads();
  panels.sessionPicked();
  panels.contextVisible(true);
  calls.length = 0;

  root = '/repo/two';
  panels.sessionPicked();
  assert.deepEqual(calls, [
    ['explorer', '/repo/two'],
    ['context', '/repo/two']
  ]);

  panels.contextVisible(false);
  calls.length = 0;
  root = '/repo/three';
  panels.sessionPicked();
  assert.ok(
    !calls.some(([name]) => name === 'context'),
    'a session change while the context view is closed reads nothing'
  );
  panels.contextVisible(true);
  assert.deepEqual(calls.at(-1), ['context', '/repo/three'], 'coming back reads the new folder');
}

// The two visible-only views are independent: opening one does not load the
// other.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sessionPicked();
  calls.length = 0;

  panels.sourceControlVisible(true);
  assert.deepEqual(calls, [['git', '/repo/one']], 'source control alone');
  panels.contextVisible(true);
  assert.deepEqual(calls.at(-1), ['context', '/repo/one'], 'the context cards alone');
  assert.deepEqual(panels.loadedPanels(), ['explorer', 'git', 'context']);
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
    ['git', '/repo/one']
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
// tree.
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
    ['git', '/repo/two']
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

// A session with no project folder has no folder to list files from. In view,
// source control is still told there is no repository to show, and the context
// cards still load — they are machine-wide, not folder-wide.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(''));
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(calls, [], 'no folder means no file listing');

  panels.sourceControlVisible(true);
  assert.deepEqual(calls.at(-1), ['git', null], 'and no repository either');

  panels.contextVisible(true);
  assert.deepEqual(calls.at(-1), ['context', ''], 'the cards still have a machine to look at');
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
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowPanelLoads();
  panels.panelShown('editor');
  panels.sessionPicked();
  assert.deepEqual(calls, [['editor', '/repo/one']], 'session picks are still switched off');
  panels.allowSessionLoads();
  root = '/repo/two';
  panels.sessionPicked();
  assert.deepEqual(calls.slice(1), [
    ['explorer', '/repo/two'],
    ['editor', '/repo/two']
  ]);
}

// Picking a session that is in the SAME folder tells the tabs nothing they do
// not already know, so none of them is loaded again. This is what keeps
// switching between two sessions in one project from throwing away and
// rebuilding everything on screen.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.panelShown('editor');
  panels.sessionPicked();
  calls.length = 0;

  panels.sessionPicked();
  assert.deepEqual(
    calls,
    [['explorer', '/repo/one']],
    'only the file tree is told again, and it refuses the repeat itself'
  );
}

// The same folder twice over, with everything the user could have open: nothing
// reads anything a second time.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.panelShown('editor');
  panels.panelShown('browser');
  panels.sessionPicked();
  panels.sourceControlVisible(true);
  panels.contextVisible(true);
  panels.worktreesVisible(true);
  panels.stacksVisible(true);
  panels.problemsVisible(true);
  calls.length = 0;

  panels.sessionPicked();
  assert.deepEqual(
    calls.map(([name]) => name),
    ['explorer'],
    'a session pick that changes no folder loads no panel'
  );

  assert.deepEqual(
    panels.loadedPanels(),
    ['editor', 'browser', 'explorer', 'git', 'context', 'worktrees', 'stacks', 'problems'],
    'and every panel still counts as opened'
  );
}

// A pick that DOES change the folder loads every panel that is open, tab or
// view — this is the switch that has to be honoured.
{
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.panelShown('editor');
  panels.sessionPicked();
  panels.sourceControlVisible(true);
  panels.stacksVisible(true);
  calls.length = 0;

  root = '/repo/two';
  panels.sessionPicked();
  assert.deepEqual(calls, [
    ['explorer', '/repo/two'],
    ['git', '/repo/two'],
    ['stacks', '/repo/two'],
    ['editor', '/repo/two']
  ]);
}

// The browser panel shows a web page, not a project, so once it has loaded no
// session pick can tell it anything — including a pick that changes folder.
{
  let root = '/repo/one';
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(root));
  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.panelShown('browser');
  calls.length = 0;

  root = '/repo/two';
  panels.sessionPicked();
  assert.ok(
    !calls.some(([name]) => name === 'browser'),
    'the browser panel is not reloaded by a change of project'
  );
}

// A tab the dock puts back at launch announces itself before either gate is
// open, so it loads nothing then — but it IS on screen, and the session picked
// next has to point it at that session's project. Forgetting the announcement
// altogether is what left a restored editor tab never being told which project
// its file is in, so nothing in the margin could be counted for the rest of the
// session.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.panelShown('editor');
  assert.deepEqual(calls, [], 'the tab the dock put back loads nothing at launch');
  assert.deepEqual(panels.loadedPanels(), [], 'and it has not loaded');

  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.deepEqual(
    calls,
    [
      ['explorer', '/repo/one'],
      ['editor', '/repo/one']
    ],
    'the first pick points the restored editor tab at the project'
  );
}

// The same announcement for the browser tab: it shows a web page rather than a
// project, so a session pick is nothing to it whether it has loaded or not.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.panelShown('browser');
  panels.allowPanelLoads();
  panels.allowSessionLoads();
  panels.sessionPicked();
  assert.ok(
    !calls.some(([name]) => name === 'browser'),
    'picking a session never loads the browser tab'
  );
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

// The worktree manager, the stacks pane and the Problems panel take the same
// route as source control and the context cards: in view plus a session picked.
// Out of view they cost nothing, and coming back to the same folder reads
// nothing again.
for (const panel of [
  { report: 'worktreesVisible', name: 'worktrees' },
  { report: 'stacksVisible', name: 'stacks' },
  { report: 'problemsVisible', name: 'problems' }
]) {
  {
    const { calls, activators } = recorder();
    const panels = createPanelActivation(activators, () => selection('/repo/one'));
    panels.allowSessionLoads();
    panels.sessionPicked();
    calls.length = 0;

    panels[panel.report](true);
    assert.deepEqual(
      calls,
      [[panel.name, '/repo/one']],
      `bringing ${panel.name} into view loads it`
    );
    assert.deepEqual(panels.loadedPanels(), ['explorer', panel.name]);

    panels[panel.report](false);
    panels[panel.report](true);
    assert.equal(calls.length, 1, `${panel.name}: coming back to the same folder reads nothing`);
  }

  // In view before any session is picked: nothing to point it at, so nothing
  // loads. The pick that follows is what loads it.
  {
    const { calls, activators } = recorder();
    const panels = createPanelActivation(activators, () => selection('/repo/one'));
    panels.allowSessionLoads();
    panels[panel.report](true);
    assert.deepEqual(calls, [], `${panel.name} in view with no session picked loads nothing`);
    assert.deepEqual(panels.loadedPanels(), []);

    panels.sessionPicked();
    assert.deepEqual(calls, [
      ['explorer', '/repo/one'],
      [panel.name, '/repo/one']
    ]);
  }

  // Launch loads nothing, in view or not.
  {
    const { calls, activators } = recorder();
    const panels = createPanelActivation(activators, () => selection('/repo/one'));
    panels[panel.report](true);
    panels.sessionPicked();
    panels[panel.report](true);
    assert.deepEqual(calls, [], `${panel.name}: launch loads nothing, in view or not`);
  }

  // Changing session re-points it while it is in view, costs nothing while it is
  // not, and coming back afterwards reads the new folder rather than the old.
  {
    let root = '/repo/one';
    const { calls, activators } = recorder();
    const panels = createPanelActivation(activators, () => selection(root));
    panels.allowSessionLoads();
    panels.sessionPicked();
    panels[panel.report](true);
    calls.length = 0;

    root = '/repo/two';
    panels.sessionPicked();
    assert.deepEqual(calls, [
      ['explorer', '/repo/two'],
      [panel.name, '/repo/two']
    ]);

    panels[panel.report](false);
    calls.length = 0;
    root = '/repo/three';
    panels.sessionPicked();
    assert.ok(
      !calls.some(([name]) => name === panel.name),
      `a session change while ${panel.name} is out of view reads nothing`
    );
    panels[panel.report](true);
    assert.deepEqual(
      calls.at(-1),
      [panel.name, '/repo/three'],
      `${panel.name}: coming back reads the new folder`
    );
  }
}

// A session with no folder still tells the stacks pane and the Problems panel
// there is nothing to look at, and still gives the worktree manager the empty
// selection — none of them may be left showing the last project's answer.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection(''));
  panels.allowSessionLoads();
  panels.sessionPicked();

  panels.worktreesVisible(true);
  assert.deepEqual(calls.at(-1), ['worktrees', '']);
  panels.stacksVisible(true);
  assert.deepEqual(calls.at(-1), ['stacks', null]);
  panels.problemsVisible(true);
  assert.deepEqual(calls.at(-1), ['problems', null]);
}

// The view-gated panels are independent of each other: opening one loads that
// one and nothing else.
{
  const { calls, activators } = recorder();
  const panels = createPanelActivation(activators, () => selection('/repo/one'));
  panels.allowSessionLoads();
  panels.sessionPicked();
  calls.length = 0;

  panels.worktreesVisible(true);
  assert.deepEqual(calls, [['worktrees', '/repo/one']], 'the worktree manager alone');
  panels.stacksVisible(true);
  assert.deepEqual(calls.at(-1), ['stacks', '/repo/one'], 'the stacks pane alone');
  panels.problemsVisible(true);
  assert.deepEqual(calls.at(-1), ['problems', '/repo/one'], 'the Problems panel alone');
  assert.deepEqual(panels.loadedPanels(), [
    'explorer',
    'worktrees',
    'stacks',
    'problems'
  ]);
}

console.log('panelActivation: all tests passed');
