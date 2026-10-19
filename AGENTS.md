Assembly
Assembly is a minimal GUI for coding agents. A Rust server wraps provider ACP adapters and agents (Codex, Claude Code, Antigravity) and serves desktop, and mobile clients.

You can think of Assembly as an open source "bring-your-own-subscription" alternative to apps like Claude Desktop, Codex App, Cursor Glass and Conductor.

What makes Assembly special?
These are the things that make Assembly different from other agent harnesses. They are the foundation of the project, and the reason we are building it. They are also a
list of the things we can never compromise on.

1. Open at the core
Assembly is truly open. We share our roadmap, we share how we think about things, and of course we share all our code. We work in the open, and should strive to stay that way.

2. Performance and low memory usage without compromise

This app is being built because existing tools are slow and heavy. Low CPU, low RAM, low GPU, and a
snappy UI are the product — every feature is secondary to that. Anything added must prove it does not
slow the app down or add idle resource cost.

Lots of apps have gotten bogged down with bad tech decisions and "slop". We have not, and we're proud of the performance of Assembly. We regularly audit for performance regressions, often caused by sending too much data over taouri commands, websockets, css animations causing gpu spikes, ticks and timers, not disposing things correctly, no stopping running processes when cancelled (abort signals) and more. Make sure all changes are considerate of performance and memory impact.

3. Remote ready
The architecture of Assembly's IPC processes, JSON-RPC, and websocket layer enables a lot of awesome remote features. These have become core to the product. Whether users are connecting directly over their local network to a remote backend, using tailscale to connect to the mobile app using Tailscale, we need to make sure new features are properly supported.

4. Multi-surface
Assembly has 2 key app surfaces: desktop, and mobile.

Desktop is kind of two surfaces, as we have the public facing "app.t3.codes" as well as locally hosting the web app through the npx t3 command. Both need to be supported by all new features where reasonable.

Desktop is the main surface most users install first. It's a full Tauri/Sveltekit/Russt app that bundles the server runner as well. The desktop app can also be used as the host server, allowing remote connections from the remote backend or the mobile app.

Mobile is a Flutter app for Android now and iOS in the future, available on the App Store and Google Play soon. The mobile app allows for connecting to any Assembly server to control work remotely.

A note from Jesse
I like ambitious ideas, simple systems, and software that feels obvious. Do not preserve complexity just because it already exists. Do not introduce machinery because it looks architecturally impressive. Understand the real constraint, then fight for the smallest model that makes the correct behavior unsurprising.

Channel both "measure twice, cut once" and "yagni". Fight scope creep. Try to honor the dev's intent in both a minimal and realistic fashion.

The rest of this document is meant to help you navigate the codebase and make changes effectively. Think of these instructions less as "hard rules", more as "good defaults". The developer's preferences should be able to override anything here.

A small glossary
We need to be on the same page with terminology. When communicating, use this language:

you means the agent reading this file and changing Assembly.
we, us, and maintainer/owner mean Jesse. This is who you are talking to now.
user means the person using Assembly to direct coding agents.
agent means the coding agent a user runs inside Assembly. Depending on context, that may also include you.
provider means the agent runtime or harness Assembly talks to, such as Codex, Claude, or Antigravity.
client means the desktop, or mobile UI.
environment means one running Assembly server and the machine, filesystem, provider credentials, and state it owns.
project means an environment-local workspace record rooted at a directory.
session means the durable conversation (chat history), project, and files and work history for a project.
turn means one user-to-agent cycle, including follow-up work such as checkpointing.
Assembly home means the base data directory. Runtime state normally lives below its userdata directory.
The three ways to hurt yourself
Killing by pattern. Never pkill -f, pgrep | kill, or kill a PID you found by matching a name, path, or worktree string. Your own agent process has this worktree's path in its argv, and this machine runs several other dev servers at once. Kill only a PID you captured at spawn, or the owner of your port from ss -H -ltnp after confirming /proc/<pid>/cwd is your worktree.
 
 The owners sqlite db, Reading it and copying from it are fine, and a good way to get real test data (see Test data). Never start a server against it, never open it read-write, never clean it up.
URemote scenarios run on the test machine "Workbox Test" (SSH alias `workbox-test`, see `tools/workbox-test/README.md`), added in the proof app's Settings → Connections with **Install and connect**. Lanes never connect a proof app to the owner's machines ("Agent Workbox" or any other saved machine), because their sessions land in the owner's session list.  All testing must contain the 4 parts of the testingn matrix before every PR, or it will not be merged.  Both Claude Code and Codex must test what was just implmemented against the local test app, and then both Claude Code and Codex must test what was just implemnted against the remote server test box on the workbox in the container. 

Anything that changes the chat (history, turn folds, tool calls and their grouping, live versus stored rows, the composer) must work the same for Claude Code and Codex, each on a local and a remote session. Verify all four in a running app before calling it done, and report a four-row table (provider × local/remote: pass/fail + evidence). A scenario you cannot run is reported as blocked with the exact error, never skipped silently.

Hit every surface
The most common defect in this repo is a change that works on the path you tested and is missing everywhere else. Before calling frontend work done, walk this list and say which entries applied:

Entry points. A behavior reachable from the chat view is usually also reachable from Settings. Fixing one is not fixing the feature.
Clients, desktop (wraps svelte web, adds Tauri shell/IPC), and mobile (Flutter, separate navigation). Shared logic lives in packages/client-runtime
Providers. Codex, Claude, Agy each have an adapter. They get downloaded and packeged/released/signed at 7:30 AM Eastern everyday, if the upstream adapter is a newer version. They get packaged with provider specific configurations. Provider-shaped features need a decision per adapter, even if the decision is "not supported here".
Agents. A capability a user can trigger is usually one an agent should reach through tools, and scheduled tasks run the same paths. That only works when it is a service method, not handler code.
Contracts. Anything crossing the wire is typed in packages/contracts. Change the schema and the server, mobile, and desktop all follow.
Reverse states. If you added a way in, add the way out and the way to see it. Snooze needs unsnooze. Close needs reopen. A one-way door is a bug.
Connection modes. Local, remote/relay, and tunnel behave differently. Multi-device and multi-environment cases are real.

Stop what you started, by the PID you tracked. See rule 1.
Test data
An empty database is a bad test. Seed your worktree's db with a copy of real data instead of pointing at live state:

Data flows one way: into your sandbox, never back out.
Verifying
Pre-commit checks staged source file sizes (600 lines, or a non-growing existing baseline) and provider branch counts outside provider adapters; commit-msg requires fix commits to change a test or give a No-test reason. Existing counts live in `tauri-svelte-preview/scripts/commitChecks.baseline.json`; lower a file size entry when its file shrinks. The existing async lifecycle scanner still runs.
Smallest proof that the change works. svelte check test run <files> for the tests you touched, targeted backend tests and typecheck for the scope you changed.
Test meaningful logic or observable behavior. Do not render components to static markup to assert props or attributes, or add tests that merely assert callback wiring or mirror the implementation.
Do not run repo-wide checks. CI owns the full suite.
Backend behavior changes ship with focused tests for that behavior.
The server is event-sourced, and side effects run after the command commits. Never wait on sleeps or polling. A test that needs a timeout to pass is wrong.
Upon request, user-visible frontend changes should get one integrated pass in a real client: test-assembly-app for desktop, test-assembly-mobile for mobile. The primary agent does this once after integrating. Subagents do not launch their own dev servers.

Pull requests
Never make a PR unless the developer explicitly asks you to do so.
Conventional commit titles, plain language: fix(web): new threads no longer spike CPU.
Body: the problem in a sentence or two, then how you fixed it. End with the model and harness that did the work.
UI changes need before/after images. Motion or timing needs a short video.
Upload PR evidence to Google Drive. Never commit PR-only screenshots or assets such as .github/pr-assets/.
One request is one PR. Split it only when the maintainer asks. Outside contributions follow the stricter one problem per PR rule.
When babysitting: poll checks and comments newer than the last push, verify each bot finding against the source, fix real ones, dismiss false positives with a written reason. Stay quiet when nothing is new. Stop when the bots are green on the latest commit.
Documentation
## ⛔ `main-architecture-explained.html` is the architecture source of truth (2026-08-19, owner)
The page at the repository root, `main-architecture-explained.html`, is the single description of how
the app works: boundaries, provider adapters, session/prompt/suspend lifecycle, storage, wire formats,
frontend projection, native surfaces. It is the only architecture description that is kept current;
there is no second copy anywhere in the tree (not under `docs/`, not in a plan).


All new frontend/tooling code is TypeScript — including one-off scripts, test scripts, and probes
(`node --experimental-strip-types` runs `.ts` directly, so there is no excuse for `.mjs`).

main-architecture-explained.html is for architectural decisions and their reasons, constraints that span components, and implementation traps that are hard to discover from the source. Before adding a paragraph, ask what a maintainer would get wrong without it. 
Do not document every feature, enumerate fields or methods, narrate control flow, maintain file catalogs, or append PR summaries. Types, tests, and code already record the implementation. The glossary defines shared vocabulary; it is not a feature index.
Keep a local implementation explanation in a nearby code comment. Use an internal doc when the reasoning crosses boundaries or needs context the code cannot carry well. Link to the relevant source instead of copying it.
When a documented decision or constraint changes, rewrite or remove the affected text. Do not append another account of the new behavior. A new internal page needs a distinct, durable reason to exist.

- The Assembly session (`ownedId`) is the durable SQLite/UI record: normalized journal, draft,
  attachments, workspace state and rail metadata.
- The provider-native session (`nativeSessionId`) is the resumable conversation/context authority.
  The provider owns its transcript and context; Assembly stores the ID and does not replay its
  SQLite journal as model context on every prompt.
- The ACP runtime is the expendable adapter transport and process. An active turn may continue when
  the user selects another Assembly session, but once that turn and all related prompts,
  permissions, inputs, tools and background work are quiescent, `suspend_if_quiescent` detaches the
  ACP session and stops the sidecar process group when its pool has no other members. The next send
  launches the adapter and resumes or loads the same provider-native ID.

  ## ⛔ One way only — never old and new side by side

A change is finished only when the old way is deleted. Never leave two mechanisms for the same job in
the tree: no compatibility bridges, no "legacy" fallback paths, no adapters kept just in case, no
feature flags that keep both alive, no half-migrated stores where a reader can't tell which one is the
truth. A migration cuts over and removes the old path in the same wave — if the old way can't be
deleted yet, the change isn't done, and the work does not merge. When you find an existing old/new
split, deleting one side is part of whatever task touched it.

  Do not write, restore, or re-add any test that asserts on component structure, layout, panels,
menus, styling, colour tokens or contrast. The UI is being reshaped daily and those tests only go red and get "fixed" to match whatever the UI happens to be, which proves nothing and wastes the owner's time.

Never describe suspended/disconnected rows as live ACP sessions or leave idle ACP runtimes resident
for quick switching. Session selection changes only the large frontend projection; it neither
interrupts a live turn nor pins an idle adapter.

A sub-agent starts cold. It cannot see the conversation, the owner's messages, or any image the
owner pasted — images live in the controller's context, not on the agent's disk. Whatever the
controller retypes from memory is the entire world the agent gets, and a paraphrase silently drops
requirements.


# ⛔ KISS and YAGNI are load-bearing rules

- **Keep it simple.** The simplest design that does the job wins. One store, one path, one obvious
  place for each responsibility. If explaining a design takes more than a few sentences, simplify the
  design before writing more code.
- **You aren't gonna need it.** Do not build for imagined futures: no speculative abstraction layers,
  no config for things nobody configures, no generalizing a function that has one caller. Add
  flexibility on the day it is actually needed, not before.

  - **Animations are welcome — every one must END.** Finite, interaction-driven animation (hover
  fades, menu pops, 150-250ms slides on transform/opacity) costs nothing once finished; use it
  generously to make the app feel alive. What is forbidden is animation without an end: infinite
  time-based loops at rest (pulsing dots, shimmer, spinner loops, animated gradients) force the
  compositor to draw every frame forever. A loop is allowed only while tied to real activit

  ## Notion task delivery record

For implementation of a Notion-tracked task, use the shared `work-tasks` skill at
`.claude/skills/work-tasks/SKILL.md` (also exposed at `.codex/skills/work-tasks`).
Keep the small plan, Eastern-dated owner/location and commit updates, direct proof,
and one combined code/spec review in the task page. Review the plan before editing.
Link that task and its proof from the PR. The commit hook checks the task link on
`tsk-*` branches, a final `Committed-by:` line, and no `Co-Authored-By` trailer. -->


## Explain in HTML, not walls of text

Anything beyond a simple explanation gets a self-contained HTML page — a diagram, a mockup, a
before/after, an annotated flow — instead of paragraphs.

docs/user/ helps users accomplish tasks. Give each major feature a concise section explaining what it does, how to start, and anything unintuitive. A settings path is useful; descriptions of visible buttons, icons, layouts, animations, or every UI state are not. Before adding text, ask what task or decision it helps the user with.
Keep user docs in the shipped product's voice, without implementation details or contributor tooling. Update the relevant feature section when how to use it changes. A UI tweak does not need a documentation entry, and a new control does not need its own page.
docs/operations/ holds maintainer setup, release, and debugging procedures. Keep instructions for operating an installed Assembly server in the user guides.
Plans and work artifacts
Do not commit implementation plans, research notes, or agent scratch files. Keep temporary working material outside the worktree. .plans/ is gitignored only as a safety net for legacy tooling.
Track active maintainer work in the GitHub issue or project item that owns it. External proposals follow CONTRIBUTING.md and belong in Ideas discussions.
A merged PR is the implementation record. Close or update its tracking item when the work lands; do not preserve a second checklist in the repository.

Taste
Complexity belongs at the adapter boundary. Orchestration stays pure, UI stays dumb.


