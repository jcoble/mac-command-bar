// Waits for the Codex review bot to finish reviewing this branch's pull request at its current head,
// then prints every unresolved bot thread. Exit 0: reviewed with nothing open (or the bot never came).
// Exit 1: open findings, printed with the commands to reply to and resolve each one.
// Run from the branch's worktree: node --experimental-strip-types tauri-svelte-preview/scripts/prBotReview.ts
import { execFileSync } from 'node:child_process';

const BOT = 'chatgpt-codex-connector';
const WAIT_MINUTES = 30;
const POLL_SECONDS = 30;

type ReviewState = { commit_sha: string | null; status: string | null };
type Thread = { id: string; isResolved: boolean; comments: { nodes: Array<{ databaseId: number; author: { login: string } | null; path: string; line: number | null; originalLine: number | null; body: string }> } };

function gh(args: string[]): string {
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

export function openBotThreads(threads: Thread[]): Thread[] {
  return threads.filter((thread) => !thread.isResolved && thread.comments.nodes[0]?.author?.login.startsWith(BOT));
}

function main(): void {
  const repo = gh(['repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner']).trim();
  const [owner, name] = repo.split('/');
  const pr = JSON.parse(gh(['pr', 'view', '--json', 'number,headRefOid'])) as { number: number; headRefOid: string };
  const head = pr.headRefOid;
  const botComments = () => JSON.parse(gh(['api', '--paginate', '--slurp', `repos/${repo}/issues/${pr.number}/comments`]))
    .flat()
    .filter((comment: { user: { login: string } }) => comment.user.login.startsWith(BOT))
    .map((comment: { body: string }) => comment.body) as string[];

  // A new PR is reviewed on its own; a later push needs an explicit request.
  const before = reviewState(botComments());
  if (before && before.commit_sha !== head) {
    gh(['pr', 'comment', String(pr.number), '--body', '@codex review']);
    console.log(`Asked the review bot to review ${head.slice(0, 9)}.`);
  }

  const deadline = Date.now() + WAIT_MINUTES * 60_000;
  for (;;) {
    const state = reviewState(botComments());
    if (state?.commit_sha === head && state.status === 'completed') break;
    if (Date.now() > deadline) {
      console.log(`The review bot did not review ${head.slice(0, 9)} within ${WAIT_MINUTES} minutes; continuing without it.`);
      return;
    }
    execFileSync('sleep', [String(POLL_SECONDS)]);
  }

  const query = `query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){pullRequest(number:$number){reviewThreads(first:100){nodes{id isResolved comments(first:1){nodes{databaseId author{login} path line originalLine body}}}}}}}`;
  const threads = JSON.parse(gh(['api', 'graphql', '-f', `query=${query}`, '-F', `owner=${owner}`, '-F', `name=${name}`, '-F', `number=${pr.number}`]))
    .data.repository.pullRequest.reviewThreads.nodes as Thread[];
  const open = openBotThreads(threads);
  if (open.length === 0) {
    console.log(`The review bot reviewed ${head.slice(0, 9)} and has no open findings.`);
    return;
  }
  console.log(`The review bot has ${open.length} open finding(s) on PR #${pr.number}:\n`);
  for (const thread of open) {
    const comment = thread.comments.nodes[0];
    console.log(`--- ${comment.path}:${comment.line ?? comment.originalLine}`);
    console.log(comment.body.trim());
    console.log(`Reply:   gh api -X POST repos/${repo}/pulls/${pr.number}/comments/${comment.databaseId}/replies -f body='<what you did>'`);
    console.log(`Resolve: gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"${thread.id}"}){thread{isResolved}}}'\n`);
  }
  process.exitCode = 1;
}

if (process.argv[1]?.endsWith('prBotReview.ts')) main();
