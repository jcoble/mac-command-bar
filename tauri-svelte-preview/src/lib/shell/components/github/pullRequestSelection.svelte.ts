/**
 * The pull request the PR tab is showing. Shared so the top tab row can label
 * the tab "PR #N" without a second copy of the selection.
 */
import type { GithubPullRequestSummary } from '$lib/tauriSource';

export const pullRequestSelection = $state<{ selected: GithubPullRequestSummary | null }>({ selected: null });
