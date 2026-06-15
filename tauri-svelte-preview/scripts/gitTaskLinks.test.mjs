import assert from 'node:assert/strict';

import {
  buildGitTaskLink,
  buildGitTaskLinksFromText,
  extractGitTaskIDs,
  normalizeGitTaskID
} from '../src/lib/gitTaskLinks.ts';

assert.equal(normalizeGitTaskID('TSK-127'), 'TSK-127');
assert.equal(normalizeGitTaskID('tsk-127'), 'TSK-127');
assert.equal(normalizeGitTaskID('[tsk-127]'), 'TSK-127');
assert.equal(normalizeGitTaskID('cdx/tsk-127-layout'), null);
assert.equal(normalizeGitTaskID('not-a-task'), null);

assert.deepEqual(extractGitTaskIDs('TSK-127'), ['TSK-127']);
assert.deepEqual(extractGitTaskIDs('ship [tsk-127] then TSK-128'), ['TSK-127', 'TSK-128']);
assert.deepEqual(extractGitTaskIDs('cdx/tsk-127-layout'), ['TSK-127']);
assert.deepEqual(extractGitTaskIDs('tsk-127 tsk-128 TSK-127'), ['TSK-127', 'TSK-128']);
assert.deepEqual(
  extractGitTaskIDs('TSK-127, tsk-128; cdx/tsk-129-layout [TSK-130]'),
  ['TSK-127', 'TSK-128', 'TSK-129', 'TSK-130']
);

assert.deepEqual(
  extractGitTaskIDs('xtsk-127 tsk-128x task_tsk-129 tsk-130_extra nottsk-131'),
  []
);
assert.deepEqual(extractGitTaskIDs('feature/tsk-127-layout fix: tsk-128.followup'), [
  'TSK-127',
  'TSK-128'
]);

assert.deepEqual(buildGitTaskLink('tsk-127'), {
  id: 'TSK-127',
  label: 'TSK-127'
});
assert.deepEqual(buildGitTaskLink('not-a-task'), null);
assert.deepEqual(buildGitTaskLink('tsk-127', { baseURL: 'https://tasks.example.test/items' }), {
  id: 'TSK-127',
  label: 'TSK-127',
  href: 'https://tasks.example.test/items/TSK-127'
});
assert.deepEqual(buildGitTaskLink('TSK-128', { baseURL: 'https://tasks.example.test/items/' }), {
  id: 'TSK-128',
  label: 'TSK-128',
  href: 'https://tasks.example.test/items/TSK-128'
});
assert.deepEqual(buildGitTaskLink('TSK-129', { baseURL: '' }), {
  id: 'TSK-129',
  label: 'TSK-129'
});

assert.deepEqual(
  buildGitTaskLinksFromText(
    ['cdx/tsk-127-layout', 'feat: [TSK-128] TSK-127 cleanup'],
    { baseURL: 'https://tasks.example.test/tasks' }
  ),
  [
    {
      id: 'TSK-127',
      label: 'TSK-127',
      href: 'https://tasks.example.test/tasks/TSK-127'
    },
    {
      id: 'TSK-128',
      label: 'TSK-128',
      href: 'https://tasks.example.test/tasks/TSK-128'
    }
  ]
);
