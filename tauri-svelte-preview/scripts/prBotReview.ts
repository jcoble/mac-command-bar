// Waits for the Codex review bot to finish reviewing this branch's pull request at its current head,
// then prints every open bot thread (see openThreads). Exit 0: the head is reviewed and nothing is open.
// Exit 1: open findings, printed with the command to reply to each one and its thread id.
// Exit 2: nothing is open, but the bot did not review the head, even after one fresh request.
// Run from the branch's worktree: node --experimental-strip-types tauri-svelte-preview/scripts/prBotReview.ts [--accepted <file>]
// --accepted names a file of accepted thread ids, one per line.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const BOT = 'chatgpt-codex-connector[bot]';
// Two waits of 15 minutes keep the whole run near 30 minutes, well inside the workflow step timeouts.
const WAIT_MINUTES = 15;
const POLL_SECONDS = 30;
const POLLS = (WAIT_MINUTES * 60) / POLL_SECONDS;

type ReviewState = { commit_sha: string | null; status: string | null };
export type Thread = { id: string; isResolved: boolean; isOutdated: boolean; comments: { nodes: Array<{ databaseId: number; author: { __typename: string; login: string } | null; path: string; line: number | null; originalLine: number | null; body: string }> } };

export function gh(args: string[]): string {
  return execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
}

// The bot keeps one summary comment whose hidden marker carries the commit it last reviewed.
export function reviewState(commentBodies: string[]): ReviewState | null {
  for (const body of commentBodies.toReversed()) {
    const marker = body.match(/codex-review-state:v1 (\{.*?\}) -->/s);
    if (marker) return JSON.parse(marker[1]).code_review as ReviewState;
  }
  return null;
}

// A bot thread stays open until the diff moved the line it cites (outdated) or its id is in the accepted list.
// GitHub's resolved flag is ignored: agents resolve threads with the owner's login. The bot never resolves
// its own threads, and GitHub records only a user as the resolver, so there is no bot resolution to read.
// GraphQL reports a bot's login without the "[bot]" suffix REST adds, so check the account type as well.
export function openThreads(threads: Thread[], accepted: string[]): Thread[] {
  return threads.filter((thread) => {
    const author = thread.comments.nodes[0]?.author;
    return author?.__typename === 'Bot' && `${author.login}[bot]` === BOT && !thread.isOutdated && !accepted.includes(thread.id);
  });
}

export function acceptedIds(argv: string[]): string[] {
  const at = argv.indexOf('--accepted');
  return at < 0 ? [] : readFileSync(argv[at + 1], 'utf8').split('\n').map((line) => line.trim()).filter(Boolean);
}

export function botCommentBodies(repo: string, number: number): string[] {
  return JSON.parse(gh(['api', '--paginate', '--slurp', `repos/${repo}/issues/${number}/comments`]))
    .flat()
    .filter((comment: { user: { login: string } }) => comment.user.login === BOT)
    .map((comment: { body: string }) => comment.body);
}

export function reviewThreads(repo: string, number: number): Thread[] {
  const [owner, name] = repo.split('/');
  const query = `query($owner:String!,$name:String!,$number:Int!,$endCursor:String){repository(owner:$owner,name:$name){pullRequest(number:$number){reviewThreads(first:100,after:$endCursor){pageInfo{hasNextPage endCursor} nodes{id isResolved isOutdated comments(first:1){nodes{databaseId author{__typename login} path line originalLine body}}}}}}}`;
  const pages = JSON.parse(gh(['api', 'graphql', '--paginate', '--slurp', '-f', `query=${query}`, '-F', `owner=${owner}`, '-F', `name=${name}`, '-F', `number=${number}`])) as Array<{ data: { repository: { pullRequest: { reviewThreads: { nodes: Thread[] } } } } }>;
  return pages.flatMap((page) => page.data.repository.pullRequest.reviewThreads.nodes);
}

function main(): void {
  const accepted = acceptedIds(process.argv);
  const repo = gh(['repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner']).trim();
  const pr = JSON.parse(gh(['pr', 'view', '--json', 'number,headRefOid'])) as { number: number; headRefOid: string };
  const head = pr.headRefOid;
  const botComments = () => botCommentBodies(repo, pr.number);

  const askForReview = () => {
    gh(['pr', 'comment', String(pr.number), '--body', '@codex review']);
    console.log(`Asked the review bot to review ${head.slice(0, 9)}.`);
  };
  const waitForReview = () => {
    for (let poll = 0; poll < POLLS; poll++) {
      const state = reviewState(botComments());
      if (state?.commit_sha === head && state.status === 'completed') return true;
      execFileSync('sleep', [String(POLL_SECONDS)]);
    }
    return false;
  };

  // A new PR is reviewed on its own; a later push needs an explicit request.
  const before = reviewState(botComments());
  if (before && before.commit_sha !== head) askForReview();
  let reviewed = waitForReview();
  // The bot sometimes stays "running" on a head and never posts; one fresh request usually gets it going.
  if (!reviewed) {
    console.log(`The review bot did not finish ${head.slice(0, 9)} within ${WAIT_MINUTES} minutes.`);
    askForReview();
    reviewed = waitForReview();
  }

  const open = openThreads(reviewThreads(repo, pr.number), accepted);
  if (open.length === 0) {
    if (reviewed) {
      console.log(`The review bot reviewed ${head.slice(0, 9)} and has no open findings.`);
    } else {
      console.log(`The review bot did not review ${head.slice(0, 9)} after a second request and ${WAIT_MINUTES} more minutes. Nothing is open, but the head is unreviewed.`);
      process.exitCode = 2;
    }
    return;
  }
  console.log(`The review bot has ${open.length} open finding(s) on PR #${pr.number}:\n`);
  for (const thread of open) {
    const comment = thread.comments.nodes[0];
    console.log(`--- ${comment.path}:${comment.line ?? comment.originalLine}`);
    console.log(comment.body.trim());
    console.log(`Reply:   gh api -X POST repos/${repo}/pulls/${pr.number}/comments/${comment.databaseId}/replies -f body='<what you did>'`);
    console.log(`Thread:  ${thread.id} (open until a push changes the line or the run accepts the reply)\n`);
  }
  process.exitCode = 1;
}

if (process.argv[1]?.endsWith('prBotReview.ts')) main();
