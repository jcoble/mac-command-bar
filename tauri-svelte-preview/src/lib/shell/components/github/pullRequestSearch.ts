/**
 * The GitHub search terms behind the Pull Requests tab's filters. The backend
 * adds `is:pr` and the known repositories; everything the person chooses or
 * types is built here, so the filters are plain data a test can check.
 *
 * The typed text is passed through as GitHub search syntax rather than quoted:
 * `head:my-branch`, `author:someone`, `label:bug` and `#123` all work the way
 * they do on github.com.
 */

export type PullRequestState = 'open' | 'merged' | 'closed' | 'all';
export type PullRequestScope = 'everyone' | 'mine' | 'needs-review';

const STATE_TERMS: Record<PullRequestState, string> = {
  open: 'is:open',
  merged: 'is:merged',
  closed: 'is:closed is:unmerged',
  all: ''
};

const SCOPE_TERMS: Record<PullRequestScope, string> = {
  everyone: '',
  mine: 'author:@me',
  'needs-review': 'review-requested:@me'
};

/** Newest activity first, whatever the text matches. */
export function pullRequestSearchQuery(state: PullRequestState, scope: PullRequestScope, text: string): string {
  return [STATE_TERMS[state], SCOPE_TERMS[scope], text.replace(/\s+/g, ' ').trim(), 'sort:updated-desc']
    .filter(Boolean)
    .join(' ');
}
