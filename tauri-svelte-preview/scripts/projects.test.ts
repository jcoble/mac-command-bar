import assert from 'node:assert/strict';

import {
  defaultDraftProjectId,
  filterProjects,
  isNewFolderPath,
  projectBadge,
  projectCountsByMachine,
  projectLastUsedLabel,
  projectMachineLabel,
  shortProjectPath,
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

// A typed path names a folder to create when its parent is the listed folder
// and nothing of that name is listed there; anything else is not new.
{
  const listing = { path: '/Users/me/dev', directories: ['app', 'site'], truncated: false };
  assert.equal(isNewFolderPath('/Users/me/dev/new-app', listing), true);
  assert.equal(isNewFolderPath(' /Users/me/dev/new-app ', listing), true);
  assert.equal(isNewFolderPath('/Users/me/dev/app', listing), false);
  assert.equal(isNewFolderPath('/Users/me/dev/new-app/', listing), true);
  assert.equal(isNewFolderPath('/Users/me/dev/app/', listing), false);
  assert.equal(isNewFolderPath('/Users/me/dev/', listing), false);
  assert.equal(isNewFolderPath('/Users/me/dev/..', listing), false);
  assert.equal(isNewFolderPath('/Users/me/other/new-app', listing), false);
  assert.equal(isNewFolderPath('/Users/me/dev/new-app', null), false);
  assert.equal(isNewFolderPath('/fresh', { path: '/', directories: ['Users'], truncated: false }), true);
}

// The Projects screen narrows the list by the chosen machine and by a search
// that matches the name or the folder, ignoring case.
{
  const projects = [
    { ...project('EdiPlatform', 'workbox'), rootPath: '/home/me/dev/EdiPlatform' },
    { ...project('rental', 'workbox'), rootPath: '/home/me/dev/rental-management' },
    { ...project('assembly', 'local'), rootPath: '/Users/me/dev/mac-command-bar' }
  ];
  const ids = (rows: ProjectRecord[]) => rows.map((row) => row.id);
  assert.deepEqual(ids(filterProjects(projects, null, '')), ['EdiPlatform', 'rental', 'assembly']);
  assert.deepEqual(ids(filterProjects(projects, 'workbox', '')), ['EdiPlatform', 'rental']);
  assert.deepEqual(ids(filterProjects(projects, 'local', '')), ['assembly']);
  assert.deepEqual(ids(filterProjects(projects, null, 'edi')), ['EdiPlatform']);
  assert.deepEqual(ids(filterProjects(projects, null, ' MANAGEMENT ')), ['rental']);
  assert.deepEqual(ids(filterProjects(projects, 'local', 'edi')), []);
}

// Each machine's count of projects; a machine with none has no entry.
{
  const projects = [project('a', 'workbox'), project('b', 'workbox'), project('c', 'local')];
  assert.deepEqual(projectCountsByMachine(projects), { workbox: 2, local: 1 });
  assert.deepEqual(projectCountsByMachine([]), {});
}

// A folder under a home directory is shown from `~`; anything else is unchanged.
{
  assert.equal(shortProjectPath('/Users/me/dev/mac-command-bar'), '~/dev/mac-command-bar');
  assert.equal(shortProjectPath('/home/blackcolours/dev/work/EdiPlatform'), '~/dev/work/EdiPlatform');
  assert.equal(shortProjectPath('/home/me'), '~');
  assert.equal(shortProjectPath('/srv/app'), '/srv/app');
  assert.equal(shortProjectPath('/homework/app'), '/homework/app');
}

// Last used reads as the app's short age; a project never used shows nothing.
{
  const now = new Date('2026-10-10T12:00:00Z');
  assert.equal(projectLastUsedLabel(null, now), '');
  assert.equal(projectLastUsedLabel(now.getTime() - 30_000, now), 'now');
  assert.equal(projectLastUsedLabel(now.getTime() - 2 * 3_600_000, now), '2h');
  assert.equal(projectLastUsedLabel(now.getTime() - 3 * 86_400_000, now), '3d');
}

console.log('projects.test.ts passed');
