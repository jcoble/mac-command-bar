Reading prompt from stdin...
OpenAI Codex v0.147.0
--------
workdir: /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
model: gpt-5.6-luna
provider: openai
approval: never
sandbox: read-only
reasoning effort: high
reasoning summaries: none
session id: 019fe600-fa26-7a62-bf48-f84850deceed
--------
user
WEB RECON task (read-only sandbox, web search enabled). You research; you do NOT edit files or commit. Print a structured report to stdout. Plain English, no invented jargon.

SUBJECT: T3 Code (the agent coding app from Theo / t3.gg, 2026) — how its UX works, in enough concrete detail that another team can borrow its best patterns for a Tauri-based agent workbench.

Research via web search: official site/docs, Theo's videos and posts (titles/descriptions/transcripts), Reddit/HN/X threads, reviews.

COVER, each with concrete mechanics:
1. Thread list / sidebar: projects → threads, per-thread live status ("Working 4h 3m" with spinner, Monitoring badge), PR number chips, worktree name chips, how done/needs-attention states are shown, and what notification is fired when a thread finishes.
2. The conversation view: how tool calls / file changes / command runs are rendered as collapsed rows ("+3 previous tool calls"), narration text between them, "Working for Xm Ys" live footer.
3. The composer: "@tag files/folders, $use skills, / for commands", model picker, context-size picker (High · 1M), permission mode (Full access), Stash feature, worktree/branch/PR chips under the input.
4. Worktree workflow: "Setup Worktree" and "Commit & push" buttons, how each thread binds to a worktree/branch/PR.
5. Multi-agent/monitoring features: anything about running many threads at once, the Monitoring state, usage view.
6. Anything publicly known about its architecture (Electron/Tauri? how it drives the agents underneath — CLI wrapping, SDK, ACP?).

OUTPUT: SECTION per topic. For each pattern: what it does, edge behavior, one-line "worth copying: yes/no + why". Label every claim with source URL. End with a top-10 list of patterns most worth adopting.

warning: clamping SessionEnd hook timeout to 3s in /Users/blackcolours/.codex/plugins/cache/openai-codex/codex/1.0.6/hooks/hooks.json
2026-08-09T10:10:51.858293Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
2026-08-09T10:10:51.901263Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
codex
I’ll produce a source-labeled, sectioned report focused on observable UX mechanics and clearly separate confirmed facts from uncertain or anecdotal architecture claims. I’ll search official T3 Code materials first, then triangulate with Theo’s public media and community/review sources.
web search: 
web search: T3 Code agent coding app Theo t3.gg official docs threads worktree Monitoring ...
web search: 
web search: github pingdotgg t3code "Working for" "previous tool calls" ...
web search: 
web search: site:github.com/pingdotgg/t3code "Working" "Monitoring" ...
web search: 
web search: 
web search: 
web search: "Setup Worktree" "T3 Code" ...
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/provider-architecture.md
web search: 
web search: https://api.github.com/repos/pingdotgg/t3code/git/trees/main?recursive=1
web search: 
web search: https://github.com/pingdotgg/t3code/search?q=Monitoring&type=code
web search: 
web search: https://github.com/pingdotgg/t3code/tree/main/apps/web/src
web search: 
web search: https://github.com/pingdotgg/t3code/tree/main/apps/web/src/components/sidebar
web search: 
web search: 
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ChatComposer.tsx
web search: 
web search: 'Stash'
web search: 
web search: 'LiveElapsed'
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts
web search: 
web search: 'onlyToolEntries'
web search: 
web search: 
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ComposerPromptEditor.tsx
web search: 
web search: 
web search: 
web search: https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx
web search: 
web search: '@'
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts
web search: 
web search: site:github.com/pingdotgg/t3code "1M" context ...
context compacted
web search: 
web search: site:github.com/pingdotgg/t3code "Working for" sidebar ...
web search: 
web search: https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/LegacySidebar.tsx
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.tsx
web search: 
web search: 'ThreadStatusPill'
web search: 
web search: 
web search: 
web search: 'ThreadWorktreeIndicator'
web search: 
web search: 
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/ThreadStatusIndicators.tsx
web search: 
web search: 
web search: 
web search: github pingdotgg t3code "desktop notification" ...
web search: 
web search: site:github.com/pingdotgg/t3code/blob/main/apps "notification" ...
web search: 
web search: https://api.github.com/repos/pingdotgg/t3code/git/trees/main?recursive=1
web search: 
web search: https://relay.t3.codes/
web search: 
web search: site:relay.t3.codes/docs t3 code notification activity ...
web search: 
web search: site:github.com/pingdotgg/t3code "agent-activity" notification ...
web search: 
web search: github pingdotgg t3code "stash" composer ...
web search: 
web search: https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/chat/ComposerStashMenu.tsx?plain=1
web search: 
web search: https://github.com/pingdotgg/t3code/tree/main/apps/web/src/components/chat
web search: 
web search: 
web search: 
web search: https://raw.githubusercontent.com/pingdotgg/t3code/refs/heads/main/apps/web/src/components/chat/ComposerStashMenu.tsx
web search: 
web search: "ComposerStashMenu.tsx" ...
web search: 
web search: T3 Code stash composer feature ...
web search: 
web search: site:github.com/pingdotgg/t3code/commit stash composer ...
web search: 
web search: site:github.com/pingdotgg/t3code "Usage" "tokens" ...
web search: 
web search: https://github.com/pingdotgg/t3code/issues/228
web search: 
web search: https://github.com/pingdotgg/t3code/tree/main/apps/web/src/components/chat
web search: 
web search: 'Setup Worktree'
context compacted
codex
T3 Code’s strongest reusable idea is a durable project → thread inbox tied directly to a Git worktree, branch, and pull request. Its public implementation is an Electron/Node control plane—not Tauri—with unusually explicit attention states and transcript folding.

Research reflects public material available through August 9, 2026. “Official source” means T3’s repository/docs; “marketing” means t3.codes; community and review claims are labeled.

## 1. Thread list and sidebar

### Mechanics

- Projects contain repositories or workspace paths, and each project contains durable threads. A thread stores its conversation, activities, checkpoints, session state, branch, and worktree path. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)

- The sidebar can group by repository, path, or separate project. Threads can be sorted by recent user message, creation time, or manually. Activity does not constantly reorder the list; status is carried by the row instead. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Active rows have explicit states:
  - `Working`: animated indicator and live duration such as `Working 4h 3m`.
  - `Connecting`: animated connection state.
  - `Monitoring`: calm, steady state for background observation.
  - `Pending Approval`: the agent needs permission.
  - `Awaiting Input`: the agent needs a user answer.
  - `Plan Ready`: a plan is waiting for approval.
  - `Failed`: error state.
  - `Completed`/`Done`: green completion state when completion happened after the user last visited.
  - `Woke`: amber, dismissible notification after a snoozed thread becomes active again. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx), [official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Working duration is formatted compactly: seconds, minutes, or hours plus minutes. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Status priority is intentional: approval and input requests outrank active work, which outranks monitoring and completion. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Rows that are actively running visually recede unless selected or unread. Attention is reserved for completed-unread, failed, woke, and other unsettled states. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- Pull requests appear as clickable `#number` badges. Open, closed, and merged states use different colors and tooltips. A PR is matched only when the thread’s branch matches the current Git branch. [Official status indicators](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/ThreadStatusIndicators.tsx), [official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- The worktree is represented by a folder/Git indicator with a tooltip containing the worktree path and branch. In the newer sidebar, the branch is also shown as text below the thread title. The current source does not consistently show a literal worktree-name chip. [Official status indicators](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/ThreadStatusIndicators.tsx), [official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- The sidebar supports pinning, settling/unsettling, snoozing/waking, renaming, marking unread, copying the path or branch, deleting, and starting a new thread on the same branch. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

### Edge behavior

- A completed thread becomes green `Done` only when completion is newer than the last visit; visiting it clears the unread completion emphasis. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- A thread can be snoozed and later return as an amber `Woke` notification with a dismiss action. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- A known bug showed why branch identity matters: T3 created the correct worktree but projected the wrong `thread.branch`, so the PR badge disappeared. [GitHub issue #2640](https://github.com/pingdotgg/t3code/issues/2640)

- Public source does not expose a named operating-system notification or exact notification text fired when a thread finishes. The confirmed web behavior is green `Completed`/`Done`; T3’s relay service documents mobile push and Live Activity registration, but not the completion payload. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx), [relay documentation](https://relay.t3.codes/docs)

**Worth copying: Yes.** The combination of stable ordering, live status, unread completion, and explicit attention states makes many concurrent threads manageable without turning the sidebar into a noisy activity feed.

## 2. Conversation view

### Mechanics

- Tool calls, shell commands, file edits, searches, MCP actions, and subagent activity are grouped into work-log rows rather than rendered as a long uninterrupted transcript. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

- Settled work is folded behind labels such as `+3 previous tool calls` or `+N previous log entries`. The user can expand the group and then use `Show fewer tool calls` or `Show fewer log entries`. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- T3 normally shows only the latest work-log entry while keeping the assistant’s final narration visible. Tool-only groups are labeled `1 tool call` or `N tool calls`; mixed groups use `Work Log`. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- Individual collapsed rows include a short preview: command text, action detail, changed-file names, or a count such as `+3 more`. Expanding reveals command details, MCP JSON, raw data, or full file paths. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- Tool type is communicated through icons: terminal, file view, file edit, web search, image work, MCP, dynamic tools, and collaboration agents. Errors and warnings receive separate visual indicators. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- While a turn is active, the footer shows three pulsing dots and `Working for <duration>`. The timer updates every second. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- When a turn settles, the fold reads `Worked for <duration>`. If the user interrupts it, the label becomes `You stopped after <duration>`. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- Subagent activity is summarized separately: live text such as `Kicked off N subagents` with a working count, then `Ran N subagents` with completed or failed counts. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

### Edge behavior

- The current work remains visible and expanded; older work is what gets folded. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

- The agent-spawn summary is deliberately not hidden, so background work remains discoverable. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

- The final assistant message remains the primary readable artifact, while the tool history remains recoverable on demand. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

**Worth copying: Yes.** It preserves auditability without forcing users to scroll through every low-level command.

## 3. Composer

### Mechanics

- `@` opens workspace path search for files and folders. Selected paths become inline, non-editable mention chips showing the basename and retaining the full path in a tooltip. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts), [official prompt editor](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx)

- `$` opens workspace skill search. Selected skills render as inline skill chips and serialize back to `$skill-name`. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts), [official prompt editor](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx)

- `/` opens slash commands. Results are grouped into built-in commands, provider commands, skills, and paths. Empty states distinguish “no matching files,” “no matching command,” and “no skills found.” [Official command menu](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ComposerCommandMenu.tsx)

- The model picker shows the active provider and model; selecting a model closes the picker. [Official model picker](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ProviderModelPicker.tsx)

- A traits picker exposes provider capabilities such as effort, context window, agent mode, fast mode, and thinking. It joins selections into compact labels, so a provider can display something like `High · 1M`; that exact combination is provider-dependent rather than hard-coded. [Official traits picker](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/TraitsPicker.tsx)

- Permission modes include supervised approval, automatic edit acceptance, automatic operation, and `Full access`. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md), [official introduction](https://pingdotgg-t3code.mintlify.app/introduction)

- The context meter is a compact circular control. Its popover shows percentage used, maximum context, total processed, and an auto-compaction note when supported. It turns red above 90%. [Official context meter](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ContextWindowMeter.tsx)

- The current source tree contains dedicated `ComposerStashBadge` and `ComposerStashMenu` components, plus a stash store. Public source fetched for this report does not document the exact stash actions or lifecycle, so its precise behavior should be verified in the running app. [Official composer source tree](https://github.com/pingdotgg/t3code/tree/main/apps/web/src/components/chat)

- Branch and worktree selection is exposed through the workspace/branch toolbar. The current source uses labels such as `local`, `worktree`, and `previous worktree`; a server-backed thread can lock the environment mode to its bound worktree. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx)

### Edge behavior

- Trigger recognition distinguishes standalone `/plan` and `/default` commands from ordinary text. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts)

- The public source does not prove that every branch, worktree, and PR chip appears directly underneath the input in the exact layout described in the question. The confirmed implementation has those controls in the branch/workspace toolbar and Git controls. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx), [official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx)

**Worth copying: Yes for `@`, `$`, `/`, model, permission, and context controls.** They turn prompt construction into visible, reversible objects. Copy the stash concept only after confirming its intended semantics.

## 4. Worktree workflow

### Mechanics

- T3’s marketing promise is one branch per agent thread. When the result is ready, one action can open a GitHub pull request with generated title, body, and changelog. [Official marketing site](https://t3.codes/)

- Internally, a worktree is an isolated Git checkout associated with a thread. The thread’s worktree path and branch drive terminals, checkpoints, diff calculation, Git status, and cleanup. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md), [GitHub issue #3753](https://github.com/pingdotgg/t3code/issues/3753)

- The current branch toolbar offers local, worktree, and previous-worktree choices. Draft composer context remembers branch, worktree path, and environment mode. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx)

- Commit controls show changed files and can accept a manual message or generate one. A chained action can commit, push, and open a pull request. [Official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx), [Better Stack review](https://betterstack.com/community/guides/ai/t3-code/)

- “Setup Worktree” is not the literal label in the current public source; the current UI uses workspace/worktree terminology. The underlying behavior—creating an isolated checkout and optionally running setup commands—is documented by a contemporary review. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx), [Better Stack review](https://betterstack.com/community/guides/ai/t3-code/)

### Edge behavior

- Commit, push, and PR actions are blocked for detached HEADs, dirty working trees, missing remotes, missing commits, branches behind upstream, or already-open PRs. [Official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx)

- Manually creating a worktree outside T3 does not reliably re-point an existing thread; the thread binding must be established by T3. [GitHub issue #3753](https://github.com/pingdotgg/t3code/issues/3753)

**Worth copying: Yes.** Thread-owned worktrees plus Git preflight prevent agents from silently colliding in the same checkout.

## 5. Multi-agent, monitoring, and usage

### Mechanics

- T3 supports many concurrent threads through the project/thread sidebar, with each thread isolated by its own worktree. A community-maintained guide describes running three agents in parallel in separate directories. [Community documentation](https://t3codedocs.com/)

- The conversation view exposes subagent counts and links to inspect them. Live states show how many subagents are working; settled states show completed and failed counts. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- `Monitoring` is a distinct background-liveness state, not a synonym for active tool execution. Active execution is `Working`; monitoring is steady and visually quieter. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts), [official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- T3 currently exposes context-window usage through a meter in the composer. [Official context meter](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ContextWindowMeter.tsx)

- A full provider quota or cost dashboard is not confirmed as shipped. One open feature request asks for per-thread token usage and account limits; another closed request says T3 currently has model selection, elapsed metadata, and a context meter but not a complete per-turn statistics footer. [GitHub issue #228](https://github.com/pingdotgg/t3code/issues/228), [GitHub issue #2518](https://github.com/pingdotgg/t3code/issues/2518)

- T3’s relay service documents mobile push notifications and Live Activity registration for agent activity. [Relay documentation](https://relay.t3.codes/docs)

### Edge behavior

- Monitoring rows intentionally recede so the sidebar does not become a list of perpetual background jobs. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- A community comparison describes T3 as human-in-the-loop, with visual review of reasoning, tool calls, proposed file changes, and diffs before committing in supervised mode. [Community comparison](https://github.com/ComposioHQ/agent-orchestrator/discussions/526)

**Worth copying: Yes for the state model; partial for usage.** Separate `Working` from `Monitoring`, and show attention states prominently. Do not claim to have a usage dashboard until token, cost, and quota data are actually available.

## 6. Publicly known architecture

### Mechanics

- T3 Code is Electron-based, not Tauri. Its desktop app bundles or supervises a backend and can serve web, desktop, and mobile clients. [Official repository guidance](https://github.com/pingdotgg/t3code/blob/main/AGENTS.md), [official workspace layout](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/workspace-layout.md)

- A Node WebSocket server is the control plane. It wraps provider CLIs including Codex, Claude Code, Cursor, Grok, and OpenCode. [Official repository guidance](https://github.com/pingdotgg/t3code/blob/main/AGENTS.md)

- The browser or desktop client sends typed WebSocket requests to the server. The server talks to providers through subprocesses and provider adapters. Codex is driven through its `codex app-server` JSON-RPC interface over stdio; other providers can use ACP-based adapters. [Official introduction](https://pingdotgg-t3code.mintlify.app/introduction), [official workspace layout](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/workspace-layout.md)

- The server records events, derives a current read model, and uses queued reactors for follow-up work. Each turn creates a hidden Git checkpoint ref so changes can be compared or restored. [Official repository guidance](https://github.com/pingdotgg/t3code/blob/main/AGENTS.md), [official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)

- Mobile clients connect through a relay for discovery and notifications, then use direct traffic after connection. [Official workspace layout](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/workspace-layout.md), [relay documentation](https://relay.t3.codes/docs)

### Edge behavior

- Provider authentication is generally BYO CLI subscription/authentication rather than a T3-managed model account. [Official introduction](https://pingdotgg-t3code.mintlify.app/introduction), [official marketing site](https://t3.codes/)

- The architecture is portable to Tauri: the provider-adapter boundary, WebSocket/API contract, event log, and Git checkpoint model are the important pieces. The Electron shell itself is not.

**Worth copying: Yes for the separation; no for the shell choice.** Keep the backend/provider boundary and durable Git/event model, but use Tauri if that is your workbench’s platform requirement.

## Top 10 patterns worth adopting

1. **Project → thread hierarchy** — makes many tasks navigable without flattening everything into one inbox. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)
2. **Stable sidebar ordering with status on each row** — prevents active threads from constantly jumping position. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)
3. **Live `Working for …` duration** — communicates progress without pretending to know completion time. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)
4. **Distinct `Working` and `Monitoring` states** — separates active execution from background observation. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)
5. **Unread completion and dismissible wake notifications** — turns finished work into actionable attention rather than permanent noise. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)
6. **Collapsed grouped tool history with `+N previous …`** — preserves evidence while keeping the conversation readable. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)
7. **Always-visible assistant narration and final diff evidence** — keeps the user oriented after tools finish. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)
8. **Inline `@` file, `$` skill, and `/` command chips** — makes context selection explicit and editable. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts), [official prompt editor](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx)
9. **Thread-bound worktree, branch, and PR identity** — prevents agents from modifying the wrong checkout. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)
10. **One-button commit/push/PR with strict Git preflight** — compresses delivery while blocking unsafe repository states. [Official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx), [official marketing site](https://t3.codes/)
tokens used
428,278
T3 Code’s strongest reusable idea is a durable project → thread inbox tied directly to a Git worktree, branch, and pull request. Its public implementation is an Electron/Node control plane—not Tauri—with unusually explicit attention states and transcript folding.

Research reflects public material available through August 9, 2026. “Official source” means T3’s repository/docs; “marketing” means t3.codes; community and review claims are labeled.

## 1. Thread list and sidebar

### Mechanics

- Projects contain repositories or workspace paths, and each project contains durable threads. A thread stores its conversation, activities, checkpoints, session state, branch, and worktree path. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)

- The sidebar can group by repository, path, or separate project. Threads can be sorted by recent user message, creation time, or manually. Activity does not constantly reorder the list; status is carried by the row instead. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Active rows have explicit states:
  - `Working`: animated indicator and live duration such as `Working 4h 3m`.
  - `Connecting`: animated connection state.
  - `Monitoring`: calm, steady state for background observation.
  - `Pending Approval`: the agent needs permission.
  - `Awaiting Input`: the agent needs a user answer.
  - `Plan Ready`: a plan is waiting for approval.
  - `Failed`: error state.
  - `Completed`/`Done`: green completion state when completion happened after the user last visited.
  - `Woke`: amber, dismissible notification after a snoozed thread becomes active again. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx), [official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Working duration is formatted compactly: seconds, minutes, or hours plus minutes. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Status priority is intentional: approval and input requests outrank active work, which outranks monitoring and completion. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- Rows that are actively running visually recede unless selected or unread. Attention is reserved for completed-unread, failed, woke, and other unsettled states. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- Pull requests appear as clickable `#number` badges. Open, closed, and merged states use different colors and tooltips. A PR is matched only when the thread’s branch matches the current Git branch. [Official status indicators](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/ThreadStatusIndicators.tsx), [official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- The worktree is represented by a folder/Git indicator with a tooltip containing the worktree path and branch. In the newer sidebar, the branch is also shown as text below the thread title. The current source does not consistently show a literal worktree-name chip. [Official status indicators](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/ThreadStatusIndicators.tsx), [official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- The sidebar supports pinning, settling/unsettling, snoozing/waking, renaming, marking unread, copying the path or branch, deleting, and starting a new thread on the same branch. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

### Edge behavior

- A completed thread becomes green `Done` only when completion is newer than the last visit; visiting it clears the unread completion emphasis. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)

- A thread can be snoozed and later return as an amber `Woke` notification with a dismiss action. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- A known bug showed why branch identity matters: T3 created the correct worktree but projected the wrong `thread.branch`, so the PR badge disappeared. [GitHub issue #2640](https://github.com/pingdotgg/t3code/issues/2640)

- Public source does not expose a named operating-system notification or exact notification text fired when a thread finishes. The confirmed web behavior is green `Completed`/`Done`; T3’s relay service documents mobile push and Live Activity registration, but not the completion payload. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx), [relay documentation](https://relay.t3.codes/docs)

**Worth copying: Yes.** The combination of stable ordering, live status, unread completion, and explicit attention states makes many concurrent threads manageable without turning the sidebar into a noisy activity feed.

## 2. Conversation view

### Mechanics

- Tool calls, shell commands, file edits, searches, MCP actions, and subagent activity are grouped into work-log rows rather than rendered as a long uninterrupted transcript. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

- Settled work is folded behind labels such as `+3 previous tool calls` or `+N previous log entries`. The user can expand the group and then use `Show fewer tool calls` or `Show fewer log entries`. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- T3 normally shows only the latest work-log entry while keeping the assistant’s final narration visible. Tool-only groups are labeled `1 tool call` or `N tool calls`; mixed groups use `Work Log`. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- Individual collapsed rows include a short preview: command text, action detail, changed-file names, or a count such as `+3 more`. Expanding reveals command details, MCP JSON, raw data, or full file paths. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- Tool type is communicated through icons: terminal, file view, file edit, web search, image work, MCP, dynamic tools, and collaboration agents. Errors and warnings receive separate visual indicators. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- While a turn is active, the footer shows three pulsing dots and `Working for <duration>`. The timer updates every second. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- When a turn settles, the fold reads `Worked for <duration>`. If the user interrupts it, the label becomes `You stopped after <duration>`. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- Subagent activity is summarized separately: live text such as `Kicked off N subagents` with a working count, then `Ran N subagents` with completed or failed counts. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

### Edge behavior

- The current work remains visible and expanded; older work is what gets folded. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

- The agent-spawn summary is deliberately not hidden, so background work remains discoverable. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

- The final assistant message remains the primary readable artifact, while the tool history remains recoverable on demand. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)

**Worth copying: Yes.** It preserves auditability without forcing users to scroll through every low-level command.

## 3. Composer

### Mechanics

- `@` opens workspace path search for files and folders. Selected paths become inline, non-editable mention chips showing the basename and retaining the full path in a tooltip. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts), [official prompt editor](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx)

- `$` opens workspace skill search. Selected skills render as inline skill chips and serialize back to `$skill-name`. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts), [official prompt editor](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx)

- `/` opens slash commands. Results are grouped into built-in commands, provider commands, skills, and paths. Empty states distinguish “no matching files,” “no matching command,” and “no skills found.” [Official command menu](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ComposerCommandMenu.tsx)

- The model picker shows the active provider and model; selecting a model closes the picker. [Official model picker](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ProviderModelPicker.tsx)

- A traits picker exposes provider capabilities such as effort, context window, agent mode, fast mode, and thinking. It joins selections into compact labels, so a provider can display something like `High · 1M`; that exact combination is provider-dependent rather than hard-coded. [Official traits picker](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/TraitsPicker.tsx)

- Permission modes include supervised approval, automatic edit acceptance, automatic operation, and `Full access`. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md), [official introduction](https://pingdotgg-t3code.mintlify.app/introduction)

- The context meter is a compact circular control. Its popover shows percentage used, maximum context, total processed, and an auto-compaction note when supported. It turns red above 90%. [Official context meter](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ContextWindowMeter.tsx)

- The current source tree contains dedicated `ComposerStashBadge` and `ComposerStashMenu` components, plus a stash store. Public source fetched for this report does not document the exact stash actions or lifecycle, so its precise behavior should be verified in the running app. [Official composer source tree](https://github.com/pingdotgg/t3code/tree/main/apps/web/src/components/chat)

- Branch and worktree selection is exposed through the workspace/branch toolbar. The current source uses labels such as `local`, `worktree`, and `previous worktree`; a server-backed thread can lock the environment mode to its bound worktree. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx)

### Edge behavior

- Trigger recognition distinguishes standalone `/plan` and `/default` commands from ordinary text. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts)

- The public source does not prove that every branch, worktree, and PR chip appears directly underneath the input in the exact layout described in the question. The confirmed implementation has those controls in the branch/workspace toolbar and Git controls. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx), [official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx)

**Worth copying: Yes for `@`, `$`, `/`, model, permission, and context controls.** They turn prompt construction into visible, reversible objects. Copy the stash concept only after confirming its intended semantics.

## 4. Worktree workflow

### Mechanics

- T3’s marketing promise is one branch per agent thread. When the result is ready, one action can open a GitHub pull request with generated title, body, and changelog. [Official marketing site](https://t3.codes/)

- Internally, a worktree is an isolated Git checkout associated with a thread. The thread’s worktree path and branch drive terminals, checkpoints, diff calculation, Git status, and cleanup. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md), [GitHub issue #3753](https://github.com/pingdotgg/t3code/issues/3753)

- The current branch toolbar offers local, worktree, and previous-worktree choices. Draft composer context remembers branch, worktree path, and environment mode. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx)

- Commit controls show changed files and can accept a manual message or generate one. A chained action can commit, push, and open a pull request. [Official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx), [Better Stack review](https://betterstack.com/community/guides/ai/t3-code/)

- “Setup Worktree” is not the literal label in the current public source; the current UI uses workspace/worktree terminology. The underlying behavior—creating an isolated checkout and optionally running setup commands—is documented by a contemporary review. [Official branch toolbar](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/BranchToolbar.tsx), [Better Stack review](https://betterstack.com/community/guides/ai/t3-code/)

### Edge behavior

- Commit, push, and PR actions are blocked for detached HEADs, dirty working trees, missing remotes, missing commits, branches behind upstream, or already-open PRs. [Official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx)

- Manually creating a worktree outside T3 does not reliably re-point an existing thread; the thread binding must be established by T3. [GitHub issue #3753](https://github.com/pingdotgg/t3code/issues/3753)

**Worth copying: Yes.** Thread-owned worktrees plus Git preflight prevent agents from silently colliding in the same checkout.

## 5. Multi-agent, monitoring, and usage

### Mechanics

- T3 supports many concurrent threads through the project/thread sidebar, with each thread isolated by its own worktree. A community-maintained guide describes running three agents in parallel in separate directories. [Community documentation](https://t3codedocs.com/)

- The conversation view exposes subagent counts and links to inspect them. Live states show how many subagents are working; settled states show completed and failed counts. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)

- `Monitoring` is a distinct background-liveness state, not a synonym for active tool execution. Active execution is `Working`; monitoring is steady and visually quieter. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts), [official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- T3 currently exposes context-window usage through a meter in the composer. [Official context meter](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/ContextWindowMeter.tsx)

- A full provider quota or cost dashboard is not confirmed as shipped. One open feature request asks for per-thread token usage and account limits; another closed request says T3 currently has model selection, elapsed metadata, and a context meter but not a complete per-turn statistics footer. [GitHub issue #228](https://github.com/pingdotgg/t3code/issues/228), [GitHub issue #2518](https://github.com/pingdotgg/t3code/issues/2518)

- T3’s relay service documents mobile push notifications and Live Activity registration for agent activity. [Relay documentation](https://relay.t3.codes/docs)

### Edge behavior

- Monitoring rows intentionally recede so the sidebar does not become a list of perpetual background jobs. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)

- A community comparison describes T3 as human-in-the-loop, with visual review of reasoning, tool calls, proposed file changes, and diffs before committing in supervised mode. [Community comparison](https://github.com/ComposioHQ/agent-orchestrator/discussions/526)

**Worth copying: Yes for the state model; partial for usage.** Separate `Working` from `Monitoring`, and show attention states prominently. Do not claim to have a usage dashboard until token, cost, and quota data are actually available.

## 6. Publicly known architecture

### Mechanics

- T3 Code is Electron-based, not Tauri. Its desktop app bundles or supervises a backend and can serve web, desktop, and mobile clients. [Official repository guidance](https://github.com/pingdotgg/t3code/blob/main/AGENTS.md), [official workspace layout](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/workspace-layout.md)

- A Node WebSocket server is the control plane. It wraps provider CLIs including Codex, Claude Code, Cursor, Grok, and OpenCode. [Official repository guidance](https://github.com/pingdotgg/t3code/blob/main/AGENTS.md)

- The browser or desktop client sends typed WebSocket requests to the server. The server talks to providers through subprocesses and provider adapters. Codex is driven through its `codex app-server` JSON-RPC interface over stdio; other providers can use ACP-based adapters. [Official introduction](https://pingdotgg-t3code.mintlify.app/introduction), [official workspace layout](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/workspace-layout.md)

- The server records events, derives a current read model, and uses queued reactors for follow-up work. Each turn creates a hidden Git checkpoint ref so changes can be compared or restored. [Official repository guidance](https://github.com/pingdotgg/t3code/blob/main/AGENTS.md), [official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)

- Mobile clients connect through a relay for discovery and notifications, then use direct traffic after connection. [Official workspace layout](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/workspace-layout.md), [relay documentation](https://relay.t3.codes/docs)

### Edge behavior

- Provider authentication is generally BYO CLI subscription/authentication rather than a T3-managed model account. [Official introduction](https://pingdotgg-t3code.mintlify.app/introduction), [official marketing site](https://t3.codes/)

- The architecture is portable to Tauri: the provider-adapter boundary, WebSocket/API contract, event log, and Git checkpoint model are the important pieces. The Electron shell itself is not.

**Worth copying: Yes for the separation; no for the shell choice.** Keep the backend/provider boundary and durable Git/event model, but use Tauri if that is your workbench’s platform requirement.

## Top 10 patterns worth adopting

1. **Project → thread hierarchy** — makes many tasks navigable without flattening everything into one inbox. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)
2. **Stable sidebar ordering with status on each row** — prevents active threads from constantly jumping position. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)
3. **Live `Working for …` duration** — communicates progress without pretending to know completion time. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)
4. **Distinct `Working` and `Monitoring` states** — separates active execution from background observation. [Official sidebar logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/Sidebar.logic.ts)
5. **Unread completion and dismissible wake notifications** — turns finished work into actionable attention rather than permanent noise. [Official sidebar UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/SidebarV2.tsx)
6. **Collapsed grouped tool history with `+N previous …`** — preserves evidence while keeping the conversation readable. [Official timeline UI](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.tsx)
7. **Always-visible assistant narration and final diff evidence** — keeps the user oriented after tools finish. [Official timeline logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/components/chat/MessagesTimeline.logic.ts)
8. **Inline `@` file, `$` skill, and `/` command chips** — makes context selection explicit and editable. [Official composer logic](https://raw.githubusercontent.com/pingdotgg/t3code/main/apps/web/src/composer-logic.ts), [official prompt editor](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/ComposerPromptEditor.tsx)
9. **Thread-bound worktree, branch, and PR identity** — prevents agents from modifying the wrong checkout. [Official glossary](https://raw.githubusercontent.com/pingdotgg/t3code/main/docs/internals/glossary.md)
10. **One-button commit/push/PR with strict Git preflight** — compresses delivery while blocking unsafe repository states. [Official Git actions](https://github.com/pingdotgg/t3code/blob/main/apps/web/src/components/GitActionsControl.tsx), [official marketing site](https://t3.codes/)
