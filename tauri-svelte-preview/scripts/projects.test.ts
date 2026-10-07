import assert from 'node:assert/strict';

import {
  defaultDraftProjectId,
  projectBadge,
  projectMachineLabel,
  visibleProjects
} from '../src/lib/shell/projects/projects.ts';
import type { ProjectRecord } from '../src/lib/tauriSource.ts';

const project = (id: string, machine: string): ProjectRecord => ({
  id,
  machine,
  rootPath: `/work/${id}`,
  title: id,
  repoKey: '',
  createdAtMs: 0
});

// The badge is the first letters of the first and last word, or the first and
// last letter of a single word.
{
  assert.equal(projectBadge('mac-command-bar'), 'MB');
  assert.equal(projectBadge('rental-management'), 'RM');
  assert.equal(projectBadge('EdiPlatform'), 'EM');
  assert.equal(projectBadge('HealthAggregator'), 'HR');
  assert.equal(projectBadge('x'), 'X');
}

// Projects on a machine that is no longer saved are hidden, not deleted.
{
  const projects = [project('a', 'workbox'), project('b', 'local'), project('c', 'gone')];
  assert.deepEqual(visibleProjects(projects, ['workbox']).map((row) => row.id), ['a', 'b']);
}

// A new draft starts in the active session's project, else the most recent
// session's, else in no project.
{
  const projects = visibleProjects([project('a', 'local'), project('b', 'local'), project('hidden', 'gone')], []);
  const owned = [
    { ownedId: 'one', projectId: 'a', lastActivity: '2026-10-01T10:00:00.000Z' },
    { ownedId: 'two', projectId: 'b', lastActivity: '2026-10-02T10:00:00.000Z' },
    { ownedId: 'three', projectId: 'hidden', lastActivity: '2026-10-03T10:00:00.000Z' },
    { ownedId: 'four', projectId: null, lastActivity: '2026-10-04T10:00:00.000Z' }
  ];
  assert.equal(defaultDraftProjectId(projects, owned, 'one'), 'a');
  assert.equal(defaultDraftProjectId(projects, owned, 'three'), 'b');
  assert.equal(defaultDraftProjectId(projects, owned, null), 'b');
  assert.equal(defaultDraftProjectId(projects, [owned[3]], null), null);
  assert.equal(defaultDraftProjectId([], owned, 'one'), null);
}

{
  const profiles = [{ id: 'workbox', name: 'Workbox' }];
  assert.equal(projectMachineLabel('local', profiles), 'This Mac');
  assert.equal(projectMachineLabel('workbox', profiles), 'Workbox');
  assert.equal(projectMachineLabel('gone', profiles), 'Remote machine');
}

console.log('projects.test.ts passed');
