import assert from 'node:assert/strict';
import { groupResourceProcesses, formatBytes } from '../src/lib/shell/resources/resourceViewModel.ts';

const process = (pid, projectId, workspaceId, sessionName, rssBytes, cpuPercent) => ({
  pid, ppid: pid - 1, pgid: 1, cpuPercent, rssBytes, elapsedSeconds: 1, user: 'test', command: `/bin/tool-${pid}`,
  listeningPorts: [], owner: 'owned-session', ownerId: sessionName, root: '/tmp', projectId, workspaceId, sessionName,
  registryGeneration: 1, canStop: true
});

const groups = groupResourceProcesses([
  process(1, 'repo-a', 'workspace-a', 'Terminal 1', 2048, 2),
  process(2, 'repo-a', 'workspace-a', 'Terminal 1', 1024, 1),
  process(3, 'repo-a', 'workspace-b', 'Agent 1', 4096, 3)
]);
assert.equal(groups.length, 1);
assert.equal(groups[0].workspaces.length, 2);
assert.equal(groups[0].workspaces[0].sessions[0].processes.length, 2);
assert.equal(groups[0].totalRssBytes, 7168);
assert.equal(groups[0].totalCpuPercent, 6);
assert.equal(formatBytes(1024 * 1024), '1.0 MB');
console.log('resource view-model tests passed');
