# Assembly: guide for agents working in this repo

Assembly is a desktop workbench for coding agents: chat sessions beside an editor, terminal, Git
and a browser, on the user's own provider subscriptions. The Tauri app (SvelteKit in
`tauri-svelte-preview/src`, Rust in `tauri-svelte-preview/src-tauri`) runs Codex, Claude Code and
Antigravity through ACP adapters it packages itself. The same Rust backend runs headless on remote
machines over SSH, and the Flutter app in `mobile/` drives sessions from an Android phone.

Jesse (the owner) wrote this file to set defaults; what he says in a task wins over it. When a rule
here gets in the way of the task, stop and tell him rather than quietly working around it.

## What we don't trade away

- **Fast and light.** The app exists because the other tools are slow and heavy. Low CPU, RAM and
  GPU and a responsive UI come before any feature; a correct feature that makes the app slower is a
  defect. The regressions we keep finding: too much data per Tauri command or websocket message,
  animations that never stop, timers and listeners left running, objects not disposed, and processes
  that keep running after the user cancels. Filtering, counting and paging over SQLite happen in
  the SQL query, not in Rust or TypeScript after loading rows.
- **Animations end.** Short animations tied to an interaction (hover fades, menu pops, 150-250ms
  transform or opacity slides) are welcome. Loops at rest (pulsing dots, shimmer, spinning
  gradients) repaint every frame forever: a loop may run only while real work is running.
- **Every timer and async call has an owner that stops it.** `async`/`await` with an `AbortSignal`
  instead of `.then` chains; the owner clears its timers on destroy; real backend work can be
  cancelled. The pre-commit hook runs `tauri-svelte-preview/scripts/scanSchedulers.ts`, which also
  rejects CSS `:has()` and Tailwind `has-*`. Never add an allow-list entry without a stated reason.
- **Remote is not second class.** Desktop to a remote backend over SSH, and phone to desktop or
  remote over Tailscale, are everyday use. A feature that only works locally is unfinished.
- **Open source.** The code, the roadmap and the reasoning are public.

From Jesse: I want the simplest system that makes the right behaviour obvious. If code is more
complicated than its problem, simplify it rather than build around it. Don't add structure to look
thorough. Find the real constraint first, then change only what the task needs.

## Words we use

- **You**: the agent reading this. **Jesse** or **the owner**: who you are working for.
- **User**: a person using Assembly. **Provider**: Codex, Claude Code or Antigravity.
- **Adapter**: the provider's ACP process Assembly launches. It is disposable: once a turn and
  everything it started are idle, `suspend_if_quiescent` detaches it and stops the process; the next
  send relaunches it and resumes the same provider session. Never keep idle adapters alive for
  quicker switching, and never call a suspended session "live".
- **Session**: two records. The Assembly session (`ownedId`) is the SQLite record the UI shows:
  journal, draft, attachments, workspace state. The provider session (`nativeSessionId`) owns the
  model's transcript and context; Assembly stores the ID and does not replay its journal as context.
- **Turn**: one prompt and all the work it causes. **Machine**: where a backend runs, local or remote.

## Things that do real damage

1. **Killing by name.** Stop only processes whose PID you saved when you started them. `pkill -f`,
   `pgrep | kill` and anything else that matches a name or path can hit your own agent (its command
   line holds the worktree path) or another lane's servers.
2. **Touching the owner's data.** The real app data folder (holding `sessions.db`; on the Mac
   `/Users/blackcolours/Library/Application Support/dev.blackcolours.MacCommandBarWebviewPreview`) and
   `~/.local/share/assembly` on any machine are in use. You may read or copy from them. Never open
   them read-write, start a server against them, or clean them up. Copy test data in; never symlink.
3. **Connecting to the owner's machines.** Remote tests use only the "Workbox Test" container (SSH
   alias `workbox-test`, see `tools/workbox-test/README.md`). **Install and connect** is fine from
   the test app, against Workbox Test only. Never connect a test app to "Agent Workbox", "SD" or any
   other saved machine: its sessions land in the owner's session list.
4. **`git reset --hard`.** It has destroyed uncommitted work. Use `git stash push -m`,
   `git restore -- <path>` after reading its diff, or `git reset --soft HEAD~1`. Before anything that
   can discard changes, run `git status --short` and say what would be lost.

## Providers behave the same

- Anything an ACP adapter or provider CLI supports must work the same for every provider that is
  installed and signed in. Don't ship a feature for one provider and leave the rest for later. When a
  provider really can't do something, decide that per provider and show it, rather than hiding it.
- A provider-specific branch lives under `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/`
  and carries a comment citing the evidence that the provider differs: an adapter source line, a
  recorded response, or a test. The pre-commit check counts branches on provider names in every
  other non-test file and fails a file that goes above its baseline (zero for new files).
- Defaults never name a provider. The Helper, a default model or agent, or anything that picks one
  chooses among the providers this user has set up, and says so when none are.
- Provider quirks stay in `providers/` and `normalization/`, so the session manager and the UI never
  ask which provider they are talking to. The UI shows state; it doesn't decide it.
- Adapters are repackaged daily by `.github/workflows/provider-updates.yml` when upstream releases.

## Where code lives

Rust (`tauri-svelte-preview/src-tauri/src/`): `agent_conversation/` holds `manager` (ordering, commit
and lifecycle), `normalization/` (the only raw ACP parsing), `history/` (the only transcript readers),
`providers/`, `remote/` (`wire.rs` is the only definition of the remote frames; `server/` owns
dispatch) and `phone/` (phone routes, which write through the shared commands). Also `git/`,
`workflow/`, `files/`, `terminal/`, `browser/`, `helper/`.

Frontend (`tauri-svelte-preview/src/lib/shell/`): one folder per feature, such as `conversation/`,
`messages/`, `git/`, `files/`, `terminal/`, `browser/`, `providers/`, `settings/`, `workflows/`.
Import a feature's public files only. The pre-commit check rejects imports of another feature's
internals (timeline row components, `terminal/ghosttyFactory`, browser annotation and bounds files)
and new files at paths the module refactor (TSK-1512) removed.

New frontend and tooling code, one-off scripts included, is TypeScript; no new `.mjs` files.

## Check every place the change reaches

Changes here usually fail by being incomplete rather than wrong: the path you tried works and the
one next to it doesn't. Before calling work done, name which of these you covered:

- **Every way in.** The chat view, Settings, the command palette and the phone often reach the same
  feature. Cover them all, not only the one you found.
- **Every provider.** See above. Chat changes are proved for Claude Code and Codex at minimum.
- **Local and remote.** A remote session goes through `remote/wire.rs` to a backend that may be a
  different build (it is released on every merge to main). Change the frames on both sides at once,
  and a `phone/` route together with the Flutter app.
- **Agents and workflows.** If a user can trigger it, an agent or a workflow will want to. Put the
  behaviour in a service function, not in the UI handler.
- **The way back out.** Anything you can open, start or hide needs its close, stop or show, and a
  way to see which state it is in.

## Testing

- **Test first** for any behaviour change: write the test, watch it fail, then make it pass.
- The commit-msg hook rejects a commit whose subject starts with `fix` unless it stages a test file
  (`*.test.ts`, `*_test.dart`, a `tests/` folder) or a Rust `#[test]`, or the body has a line
  `No-test: <reason>`.
- Run only what you touched: `pnpm --dir tauri-svelte-preview test:<name>` (or `node
  --experimental-strip-types <file>.test.ts`), `cargo test --manifest-path
  tauri-svelte-preview/src-tauri/Cargo.toml <name>`, `check:svelte`. Full suites belong to CI.
- Test behaviour and logic. No tests that mirror the implementation or only check that a callback
  is wired. No tests on component structure, layout, panels, menus, styling, colour tokens or
  contrast: the UI changes daily and those tests only get rewritten to match it.
- A test that needs a sleep or polling to pass is wrong; wait on the event that marks completion.
- **Every change gets the four-scenario check** (`docs/operations/four-scenario-check.md`), with
  no exemption for changes that look small: Claude Code and Codex, each on a local session and on
  Workbox Test. Several changes may share one run if it exercises each of them. Report the four-row
  table with evidence; a scenario you can't run is reported blocked with the exact error.
- That check and any UI proof use the real native app with real test data, driven the way a
  person uses it. On Linux the app runs under Xvfb at 1710x990, driven with `assembly-native-ui`
  (screenshots, clicks, keys); Codex may use its computer-use tool. A browser preview is not proof:
  it has no app data. Use a headless browser at 1710x990 only for real web pages.
- Save screenshots as `~/Workbox/screenshots/TSK-<id>-<what>-<before|after>.png`
  and run `evidence add <file> --task TSK-<id> --desc "<what it proves>"`; `evidence publish
  TSK-<id>` gives the Drive links for the PR and the task. Close every browser and app you opened.

## The workbox

- Jesse works in Assembly's own Terminal tab on the workbox: the shell is started by the Assembly
  backend there and reached over the app's one SSH tunnel, and it ends whenever that backend
  restarts. How it works and what can go wrong: `docs/remote-terminal.md`.
- Run `free -g` before a Rust build, a Tauri build or an app launch. If `available` is 6 GB or less,
  wait. One heavy job at a time per lane; prefer `cargo test <name>` to whole crates.
- `/tmp` is a RAM disk: never put a cargo target folder or other big build output there.
- Worktrees go in `/home/blackcolours/dev/work/worktrees/mac-command-bar/<branch>` (on the Mac, the
  same path under `/Users/blackcolours`). Remove yours with `git worktree remove` once it is merged
  or abandoned; report a dirty one instead of deleting it.

## Commits and pull requests

- Hooks live in `.githooks` (`git config core.hooksPath .githooks` once per clone). Pre-commit
  rejects a source file over 600 lines or over its entry in
  `tauri-svelte-preview/scripts/commitChecks.baseline.json`. Never raise a baseline; when a
  baselined file shrinks, lower its entry. New tests for a baselined file go in a small new file.
- Branch names: `tsk-<id>-<short-slug>`, lowercase. On those branches the message needs
  `Task: TSK-<id> https://app.notion.com/p/<page-id>`.
- Every message needs `Architecture: unchanged`, or a staged `main-architecture-explained.html` with
  a change-log row. The last line is `Committed-by: <who did it>`. No `Co-Authored-By` lines.
- Subject in plain language, conventional style: `fix(chat): reopened sessions keep their order`.
  Body: what was wrong, then what changed. Read `git diff --cached` first; commit only your work.
- Don't open a PR unless asked. Never merge on your own: a merge happens only when Jesse asks for
  that PR, as `ALLOW_MERGE=1 gh pr merge <n> --merge`.
- A PR is one request. UI changes need before and after screenshots; motion needs a short video.
  Evidence goes to Drive and is linked from the PR and the task; never commit screenshots.
- End the PR body with the model and harness that did the work (for example `Model: Claude Opus ·
  Harness: Claude Code`). Code, comments and commit messages don't name AI tools.
- Looking after a PR means handling what is new since your last push: confirm each reported
  problem in the code, fix it if real, otherwise reply with why not.

## Scope: KISS and YAGNI

- The smallest change that does the job wins. If two designs work, ship the one with fewer parts. If
  a design needs more than a few sentences to explain, simplify it first.
- Build nothing for imagined futures: no new abstraction, config option, flag, fallback path or
  wrapper the task didn't ask for, and no drive-by refactors. One problem per change.
- **One way only.** A change is done when the old mechanism is deleted. No compatibility bridges,
  legacy fallbacks, flags that keep both alive, or half-migrated stores. If the old way can't go yet,
  the change isn't finished. When you find an old/new split, removing one side is part of your task.

## Tasks, plans and docs

- Tasks live in Notion. For a Notion-tracked task, follow `.claude/skills/work-tasks/SKILL.md`
  (also `.codex/skills/work-tasks`): the plan, dated updates, proof and review go on the task page,
  and the PR links the task and its proof.
- A sub-agent sees none of the conversation, the owner's messages or pasted images. Its prompt
  must carry every requirement; a paraphrase from memory drops some.
- `main-architecture-explained.html` is the one description of how the app works. Change it when a
  boundary, lifecycle, storage or wire decision changes, rewriting the affected text rather than
  adding a second account. Leave out what the code already says: field lists, control flow, file
  catalogues, PR summaries. A local reason belongs in a comment next to the code.
- Explain anything long with a self-contained HTML page (diagram, mockup, before and after).
- `docs/operations/` holds maintainer procedures. Plans, recon notes and lane reports that are
  committed go in `docs/plans/<YYYY-MM-DD>/`, never the repo root. Scratch stays out of the worktree.
- Plain English in everything a person reads. Spell terms out; no invented shorthand.
