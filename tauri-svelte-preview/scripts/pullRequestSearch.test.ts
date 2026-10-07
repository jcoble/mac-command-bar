import assert from 'node:assert/strict';

import { pullRequestSearchQuery } from '../src/lib/shell/components/github/pullRequestSearch.ts';

// Every state reaches GitHub as its own qualifier; "all" adds none, so merged
// and closed pull requests are searchable too.
assert.equal(pullRequestSearchQuery('open', 'everyone', ''), 'is:open sort:updated-desc');
assert.equal(pullRequestSearchQuery('merged', 'everyone', ''), 'is:merged sort:updated-desc');
assert.equal(pullRequestSearchQuery('closed', 'everyone', ''), 'is:closed is:unmerged sort:updated-desc');
assert.equal(pullRequestSearchQuery('all', 'everyone', ''), 'sort:updated-desc');

// The scope narrows by who, independent of the state.
assert.equal(pullRequestSearchQuery('all', 'mine', ''), 'author:@me sort:updated-desc');
assert.equal(pullRequestSearchQuery('merged', 'needs-review', ''), 'is:merged review-requested:@me sort:updated-desc');

// Typed text is GitHub search syntax, not a quoted phrase, so qualifiers work.
assert.equal(
  pullRequestSearchQuery('all', 'everyone', 'head:cdx/selected-provider-progress'),
  'head:cdx/selected-provider-progress sort:updated-desc'
);
assert.equal(pullRequestSearchQuery('open', 'mine', '  adapter   selected \n'), 'is:open author:@me adapter selected sort:updated-desc');
assert.equal(pullRequestSearchQuery('all', 'everyone', '#111'), '#111 sort:updated-desc');

console.log('pullRequestSearch: ok');
