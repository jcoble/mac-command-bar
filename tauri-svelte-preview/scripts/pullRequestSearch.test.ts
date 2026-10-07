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

// A plain word matches the title or body, the head branch (by prefix), or the
// author, so a branch name or a login typed on its own finds its pull requests.
assert.equal(
  pullRequestSearchQuery('open', 'mine', '  adapter   jcoble \n'),
  'is:open author:@me (adapter OR head:adapter OR author:adapter) (jcoble OR head:jcoble OR author:jcoble) sort:updated-desc'
);
assert.equal(
  pullRequestSearchQuery('all', 'everyone', 'cdx/selected-provider'),
  '(cdx/selected-provider OR head:cdx/selected-provider OR author:cdx/selected-provider) sort:updated-desc'
);

// Anything else is GitHub search syntax and passes through untouched: a
// qualifier, or every word of a quoted phrase.
assert.equal(pullRequestSearchQuery('all', 'everyone', 'head:fix label:bug'), 'head:fix label:bug sort:updated-desc');
assert.equal(
  pullRequestSearchQuery('all', 'everyone', '"selected adapter" show'),
  '"selected adapter" (show OR head:show OR author:show) sort:updated-desc'
);

// A number names one pull request, so it is found whatever its state or author.
assert.equal(pullRequestSearchQuery('open', 'mine', '111'), '111 sort:updated-desc');
assert.equal(pullRequestSearchQuery('open', 'needs-review', ' #111 '), '111 sort:updated-desc');

console.log('pullRequestSearch: ok');
