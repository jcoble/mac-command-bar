import assert from 'node:assert/strict';
import test from 'node:test';

import { matchNotionProject } from '../src/lib/shell/notionProjectMatch.ts';
import { normalizeTasksView, projectToShow } from '../src/lib/shell/panels/tasks/tasksView.ts';

const PROJECTS = ['MacCommandBar', 'EdiPlatform'];

test("the session's Notion project wins, otherwise the saved project is kept", () => {
  const shown = (root: string) => projectToShow(matchNotionProject(root, PROJECTS), 'EdiPlatform');

  assert.equal(shown('/home/me/dev/work/mac-command-bar'), 'MacCommandBar');
  assert.equal(shown('/home/me/dev/work/TestEmptyFolderWGit'), 'EdiPlatform');
  assert.equal(shown(''), 'EdiPlatform');
});

test('junk in a stored tasks view becomes its default', () => {
  assert.deepEqual(
    normalizeTasksView({
      search: 42,
      filters: { status: ['Doing', 7, 'Doing'], priority: 'High' },
      projectFilter: null,
      sortBy: 'createdAt',
      sortDirection: 'sideways'
    }),
    {
      search: '',
      filters: { status: ['Doing'], priority: [] },
      projectFilter: '',
      sortBy: 'taskNumber',
      sortDirection: 'desc'
    }
  );
  assert.equal(normalizeTasksView('all'), null);
});
