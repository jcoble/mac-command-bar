import assert from 'node:assert/strict';

import {
  SESSION_HISTORY_ROW_WINDOW,
  buildSessionHistoryViewModel,
  createSessionHistoryWindowState,
  extendSessionHistoryWindow,
  resetSessionHistoryWindowOnFilterChange
} from '../src/lib/shell/history/sessionHistoryViewModel.ts';
import type { SessionLibraryRecord } from '../src/lib/shell/sessionLibrary/sessionLibraryModel.ts';

function record(index: number): SessionLibraryRecord {
  const stamp = new Date(Date.UTC(2026, 0, 1, 0, 0, index)).toISOString();
  return {
    key: `history-${index}`,
    source: 'provider',
    ownedId: null,
    provider: 'alpha',
    nativeSessionId: `native-${index}`,
    canonicalCwd: '/Users/dev/work/worktrees/atlas/historybound',
    title: `Matching history ${index}`,
    description: null,
    projectPath: '/Users/dev/work/worktrees/atlas/historybound',
    model: null,
    state: 'resumable',
    runtimeState: null,
    lastActivity: stamp,
    updatedAt: stamp,
    messageCount: null,
    firstPrompt: null,
    latestTurns: [],
    owned: null,
    available: null
  };
}

const records = Array.from({ length: 4_000 }, (_, index) => record(index));
const initialWindow = createSessionHistoryWindowState();

{
  const view = buildSessionHistoryViewModel(records, { windowState: initialWindow });
  const worktree = view.projects[0]?.worktrees[0];
  assert.ok(worktree);
  assert.equal(worktree.rows.length, SESSION_HISTORY_ROW_WINDOW);
  assert.equal(worktree.olderCount, 4_000 - SESSION_HISTORY_ROW_WINDOW);
  assert.equal(worktree.rows[0]?.record.key, 'history-3999', 'the newest row renders first');

  const extended = extendSessionHistoryWindow(initialWindow, worktree.key);
  const disclosed = buildSessionHistoryViewModel(records, { windowState: extended });
  const disclosedWorktree = disclosed.projects[0]?.worktrees[0];
  assert.ok(disclosedWorktree);
  assert.equal(disclosedWorktree.rows.length, SESSION_HISTORY_ROW_WINDOW * 2);
  assert.equal(disclosedWorktree.olderCount, 4_000 - SESSION_HISTORY_ROW_WINDOW * 2);
}

// A text search is an explicit request to see every matching result. It filters
// the full record set and bypasses the browsing window for matching groups.
{
  const searched = buildSessionHistoryViewModel(records, {
    query: 'matching history',
    windowState: initialWindow
  });
  const worktree = searched.projects[0]?.worktrees[0];
  assert.ok(worktree);
  assert.equal(worktree.rows.length, 4_000);
  assert.equal(worktree.olderCount, 0);
}

// Any filter change starts browsing from the first window again instead of
// carrying a disclosure count across a different result set.
{
  const key = 'worktree:/Users/dev/work/worktrees/atlas/historybound';
  const extended = extendSessionHistoryWindow(initialWindow, key);
  const reset = resetSessionHistoryWindowOnFilterChange(extended, { provider: 'beta' });
  assert.notEqual(reset, extended);
  assert.deepEqual(reset.visibleRowsByWorktree, {});
  assert.equal(reset.provider, 'beta');
}

console.log('sessionHistoryWindow: all tests passed');
