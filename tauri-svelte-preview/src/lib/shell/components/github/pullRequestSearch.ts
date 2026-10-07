/**
 * The GitHub search terms behind the Pull Requests tab's filters. The backend
 * adds `is:pr` and the known repositories; everything the person chooses or
 * types is built here, so the filters are plain data a test can check.
 *
 * A plain word typed on its own matches the title or body, the head branch
 * (GitHub matches `head:` by prefix) or the author, so a branch name or a login
 * finds its pull requests. Anything else is GitHub search syntax and passes
 * through untouched: `label:bug`, `author:someone`, a quoted phrase. A number,
 * with or without `#`, names one pull request and is found in any state.
 * GitHub has no partial-word match, so "adapt" does not find "adapter".
 *
 * The backend sends this as GitHub's advanced search, which reads `OR` and
 * parentheses.
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

const PLAIN_WORD = /^[\w./-]+$/;

/** Newest activity first, whatever the text matches. */
export function pullRequestSearchQuery(state: PullRequestState, scope: PullRequestScope, text: string): string {
  const trimmed = text.trim();
  const number = /^#?(\d+)$/.exec(trimmed);
  if (number) return `${number[1]} sort:updated-desc`;
  let quoted = false;
  const words = trimmed.split(/\s+/).filter(Boolean).map((word) => {
    const plain = !quoted && PLAIN_WORD.test(word);
    if ((word.match(/"/g) ?? []).length % 2 === 1) quoted = !quoted;
    return plain ? `(${word} OR head:${word} OR author:${word})` : word;
  });
  return [STATE_TERMS[state], SCOPE_TERMS[scope], ...words, 'sort:updated-desc'].filter(Boolean).join(' ');
}
