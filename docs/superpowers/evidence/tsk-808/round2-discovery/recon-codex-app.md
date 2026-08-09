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
session id: 019fe600-ebd7-7f23-980a-7869dd4822b6
--------
user
WEB RECON task (read-only sandbox, web search enabled). You research; you do NOT edit files or commit. Print a structured report to stdout. Plain English, no invented jargon.

SUBJECT: The OpenAI Codex desktop app (the native Mac app for Codex, current 2026 version) — how its UX works, in enough concrete detail that another team can borrow its best patterns for a Tauri-based agent workbench.

Research via web search: official docs/changelogs, OpenAI announcements, reviews, YouTube walkthroughs' descriptions/transcripts, Reddit/HN threads, blog posts.

COVER, each with concrete mechanics (not vibes):
1. Conversation-centric layout: how the conversation owns the center; what lives in the left sidebar (projects → threads grouping, pinned, "Show more"); what lives in the right rail (Plan, Outputs, Subagents, Sources); how panels open/close/resize.
2. The composer/prompt input: model + effort picker in the bar, approval/permission mode control, attach button, mic/dictation, pasting screenshots, dragging files from Finder, slash commands, how attachments and annotations render as chips.
3. The embedded browser: how it docks as a right panel, expand-to-almost-fullscreen behavior, tab strip, and especially ANNOTATION mode — numbered pins dropped on page elements, inline comment box, annotation count chip on the composer, Send N; what data the annotation carries into the turn (screenshot region? element/selector info? coordinates?).
4. Turn rendering: user vs agent message styling, tool-call/file-edit cards ("Edited 5 files +53 -15" with Undo/Review), "Worked for Xm Ys" collapsible activity, streaming behavior.
5. Working-state signals: how active/running threads are indicated in the sidebar, notifications when a turn completes.
6. Anything on its underlying architecture that is publicly known (how the UI talks to the agent runtime, protocols).

OUTPUT: SECTION per topic. For each pattern: what it does, how it behaves at edges (resize, multiple threads, long content), and a one-line "worth copying: yes/no + why". Label every claim with its source URL. End with a top-10 list of the patterns most worth adopting.

warning: clamping SessionEnd hook timeout to 3s in /Users/blackcolours/.codex/plugins/cache/openai-codex/codex/1.0.6/hooks/hooks.json
2026-08-09T10:10:48.201020Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
2026-08-09T10:10:48.247109Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
codex
I’ll produce a source-labeled, sectioned research report covering all seven UX areas, including edge behavior and a copy/no-copy verdict for each pattern. I’ll rely on current official material first, then triangulate with walkthroughs, reviews, and community reports; no files or repo state will be changed.
web search: 
web search: site:openai.com Codex desktop app Mac 2026 ...
web search: 
web search: site:openai.com/index/introducing-the-codex-app projects threads sidebar plan outputs subagents sources ...
web search: 
web search: "Plan" "Outputs" "Subagents" "Sources" Codex app ...
web search: 
web search: site:help.openai.com/en/articles/ Codex app model picker effort picker approval mode attachment ...
web search: 
web search: 
web search: 
web search: 
web search: 
web search: 'annotation'
web search: 
web search: Codex desktop app walkthrough sidebar right panel plan outputs subagents sources screenshot ...
web search: 
web search: site:developers.openai.com/codex/app "right panel" ...
web search: 
web search: 
web search: 
web search: 'JSON-RPC'
web search: 
web search: '/plan'
web search: 
web search: site:learn.chatgpt.com/docs "drag" "composer" Codex ...
web search: 
web search: 
web search: 
web search: "Outputs" "Subagents" "Sources" "Codex" ...
web search: 
web search: "Worked for" "Codex" desktop app ...
web search: 
web search: Codex in-app browser right panel expand fullscreen tab strip ...
web search: 
web search: openai codex browser annotation payload selector coordinates screenshot ...
web search: 
web search: 
web search: 
web search: Codex app composer attach file drag screenshot paste microphone model reasoning effort ...
web search: 
web search: 'Command'
context compacted
codex
Bottom line: Codex is a conversation-first command center: projects and threads organize work, the center shows the active turn, and secondary rails expose plans, artifacts, sources, and agents. The strongest reusable ideas are structured agent state, reviewable file changes, and a browser that turns visual comments into actionable context. Some current UI details—exact rail labels, resize gestures, chip text, and annotation payload fields—are only described by community reports, so they should be verified against the live Mac build.

## 1. Conversation-centric layout

### Center: the conversation is the workspace

- The Mac app is designed to manage several agents and long-running tasks at once. Each agent runs in its own thread, and threads are organized under projects. The active thread remains the primary workspace for messages, progress, approvals, and review. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))
- Reviewers describe a three-pane layout: project/thread navigation on the left, the active conversation in the center, and contextual results or review content on the right. ([AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/), [VibeCoding review](https://vibecoding.app/blog/openai-codex-app-review))

**Edge behavior**

- Long-running work persists as a thread that can be resumed or forked; the public protocol treats a thread as the durable conversation and a turn as one request plus its agent work. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Multiple agents can work in separate worktrees, so their changes do not overwrite one another. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))

**Worth copying: yes.** A conversation should own the center; project navigation and diagnostics should remain secondary.

### Left sidebar: projects, threads, pinning, and “Show more”

- Projects group related threads; the sidebar is the main navigation surface for switching between workstreams. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/), [VibeCoding review](https://vibecoding.app/blog/openai-codex-app-review))
- The app-server protocol exposes thread metadata for pinning and unpinning, so pinned threads are a first-class state rather than a visual-only shortcut. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Current builds use a “Show more” or expansion path for additional threads, but users have reported that it can show only a capped or already-loaded candidate set. ([OpenAI Community thread](https://community.openai.com/t/search-or-show-all-threads-in-codex-ide-or-app/1375338), [GitHub issue #27314](https://github.com/openai/codex/issues/27314))
- Worktree-created threads have also been reported as failing to group correctly under the expected project. ([GitHub issue #10522](https://github.com/openai/codex/issues/10522))

**Edge behavior**

- The important implementation requirement is true project-scoped pagination or search. Expanding a global list is not sufficient for large histories. ([GitHub issue #27314](https://github.com/openai/codex/issues/27314))

**Worth copying: yes, with robust loading.** Projects, pins, and thread history are useful; a misleading “Show more” is not.

### Right rail: Plan, Outputs, Subagents, Sources

- A 2026 workspace update added a task/sidebar view that surfaces the agent’s plan, tracked sources, and generated artifacts. ([Codex Workspace Knowledge Base](https://codex.danielvaughan.com/2026/04/17/codex-app-workspace-pr-review-task-sidebar-artifact-viewer/), [TechRadar report](https://www.techradar.com/pro/codex-can-now-operate-your-computer-alongside-you-with-major-workplace-updates))
- “Plan” is also represented as structured progress state. The composer can invoke `/plan` and `/goal`, and the protocol streams plan updates. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands), [App-server documentation](https://developers.openai.com/codex/app-server))
- “Outputs” corresponds to generated artifacts and previews such as documents, spreadsheets, slides, and other files. ([Codex Workspace Knowledge Base](https://codex.danielvaughan.com/2026/04/17/codex-app-workspace-pr-review-task-sidebar-artifact-viewer/), [TechRadar report](https://www.techradar.com/pro/codex-can-now-operate-your-computer-alongside-you-with-major-workplace-updates))
- The Subagents view exposes each child agent’s status, allows opening its individual thread, and supports stopping agents. ([Subagents documentation](https://developers.openai.com/codex/subagents))
- Sources track files, URLs, and other context used by the task. ([Codex Workspace Knowledge Base](https://codex.danielvaughan.com/2026/04/17/codex-app-workspace-pr-review-task-sidebar-artifact-viewer/))

**Opening, closing, and resizing**

- Community users report a right-side browser/context panel opened with a toolbar control or `Cmd+Option+B`, with an expand control that makes it nearly full-screen while leaving a floating chat surface. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
- I found no authoritative public documentation specifying a resize gesture or exact minimum/maximum rail width. That behavior should be treated as release-sensitive.

**Worth copying: yes.** A persistent contextual rail prevents the conversation from becoming a dumping ground, but it needs predictable collapse and width behavior.

## 2. Composer and prompt input

### Model, effort, approval, and sandbox controls

- The public runtime accepts model, reasoning effort, approval policy, and sandbox settings when a thread starts. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The composer exposes related controls through model/reasoning commands such as `/model`, `/reasoning`, and `/fast`; exact picker labels vary by release. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))
- Codex’s documented approval modes include asking before actions, allowing edits automatically, and fully automatic execution; the desktop app uses the same underlying permission model. ([OpenAI Codex usage guidance](https://help.openai.com/en/articles/11096431), [OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))

**Worth copying: yes.** Keep execution policy visible at the point where the user sends work.

### Attachments, screenshots, and voice

- The desktop app supports voice input: select the Voice control or its shortcut, grant microphone access, speak, and interrupt naturally while live text appears. ([ChatGPT Work and Codex help](https://help.openai.com/en/articles/20001275))
- Appshots capture the frontmost Mac window as an image plus text the app exposes, behave like attachments, and are stored locally with the session. Pressing both Command keys is the default capture gesture. ([Appshots documentation](https://developers.openai.com/codex/appshots))
- The built-in browser supports visual comments and sends the resulting comment context back through chat. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app), [Codex browser documentation](https://developers.openai.com/codex/app/browser))

**Not fully documented**

- I found no authoritative public text that specifies Finder drag-and-drop, generic screenshot paste, the exact attach-button layout, or the exact chip styling. These should be validated in the live Mac application rather than assumed from the protocol.

**Worth copying: yes.** Show every attachment as a removable, inspectable chip before send; preserve the source image and any extracted text.

### Slash commands

- Typing `/` opens a searchable command menu. `$` explicitly selects a skill. Available commands include `/model`, `/reasoning`, `/plan`, `/goal`, `/review`, `/fork`, `/project`, `/worktree`, `/local`, and `/cloud`. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))

**Edge behavior**

- Goal state can remain visible above the composer with pause, resume, edit, and clear actions. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))

**Worth copying: yes.** Slash commands reduce hidden mode switches and make advanced actions discoverable without adding permanent toolbar clutter.

## 3. Embedded browser and annotation mode

### Docked browser

- The desktop app has a built-in browser with its own browser profile and state, separate from Chrome. It supports multiple tabs, downloads, sign-in flows, and permission prompts. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app))
- The browser can be opened from the desktop toolbar; the documented Mac shortcut is `Cmd+Shift+B`. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app))
- Community reports describe the browser as a right-side panel with an expand button that makes it nearly full-screen while preserving a floating chat control. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))

**Edge behavior**

- Official documentation confirms multiple tabs, but does not specify the exact tab-strip appearance or tab-reordering behavior. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app))

**Worth copying: yes.** A docked browser keeps visual review beside the agent conversation; near-fullscreen expansion is useful for dense pages.

### Annotation mode

- Officially, the user can select a page element or drag over an area, write a comment, save it, and send that feedback through chat. ([Codex browser documentation](https://developers.openai.com/codex/app/browser))
- Community reports describe numbered pins placed on the page, an inline comment box, and the browser returning to chat with the comments attached. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
- A community answer reports that an annotation contains a screenshot with the selected area marked, the page URL, a selector/path such as `html > body … > h1`, and the target text. This is not confirmed by official protocol documentation. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
- I found no public source confirming that raw pixel coordinates are part of the stable annotation contract. Screenshot-coordinate bugs have occurred because of zoom and device-pixel-ratio differences. ([GitHub issue #19429](https://github.com/openai/codex/issues/19429))
- Current UI reports mention an annotation-count chip in the composer and a “Send N” action. A later issue reports that pressing Enter began submitting the first comment immediately, breaking multi-comment batching. ([Codex issue mirror #22719](https://codexissues.com/issue/22719-in-app-browser-comments-submit-immediately-on-enter-instead-of-allowing-multiple))

**Worth copying: yes.** Combine visual evidence with semantic identity (URL, selector, text); do not rely on coordinates alone.

## 4. Turn rendering

### Messages and streaming

- The public protocol distinguishes user messages, agent messages, commands, file changes, and tool calls as separate items. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Agent text streams as incremental `agentMessage/delta` notifications, while items emit started and completed events. ([App-server documentation](https://developers.openai.com/codex/app-server))
- This gives the UI enough structure to render user text, agent text, tool activity, and edits differently instead of treating everything as one transcript.

**Worth copying: yes.** Render from typed events, not from a single string stream.

### File-edit and tool cards

- File changes have an explicit lifecycle: proposed change, approval request, client decision, then completed, failed, or declined. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Review-oriented desktop guides describe file-change cards with changed-file counts, line deltas, inline annotations, review, rollback, and staging actions. ([AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/))
- The exact phrase “Edited 5 files +53 -15” is a current UI string, not a documented API contract. A Tauri workbench should treat the counts and actions as structured fields, not parse display text.

**Worth copying: yes.** “Review” and “Undo” at the point of change make agent work reversible.

### Activity and elapsed-time rows

- The protocol exposes turn start/completion and tool-progress events, so the client can show an activity section and elapsed time. ([App-server documentation](https://developers.openai.com/codex/app-server))
- I found no official documentation that guarantees the exact “Worked for Xm Ys” wording or whether the activity section is always collapsed. Treat that label as release-specific.

**Worth copying: yes.** Collapse noisy activity by default while keeping the full event history available.

## 5. Working-state signals

### Active threads and subagents

- Thread status includes `active`, `idle`, `systemError`, and `notLoaded`; active flags can include waiting for approval. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The protocol emits `thread/status/changed` events, allowing the sidebar to update without polling. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The Subagents panel shows active child agents, their status, stop controls, and links to their individual threads. ([Subagents documentation](https://developers.openai.com/codex/subagents))

**Edge behavior**

- A sidebar indicator should distinguish running, waiting for approval, completed, failed, and disconnected states. The public protocol supplies the state; the exact iconography is a UI choice. ([App-server documentation](https://developers.openai.com/codex/app-server))

**Worth copying: yes.** State-aware navigation is essential when several agents run concurrently.

### Completion notifications

- Codex supports desktop completion notifications, but an open issue reports that notification settings can be ignored in app-server mode. ([GitHub issue #13019](https://github.com/openai/codex/issues/13019))

**Worth copying: yes, with fallback.** Provide an in-app completion marker even when operating-system notifications are unavailable or suppressed.

## 6. Publicly known architecture

- OpenAI describes the app-server as the interface used by rich clients such as IDE extensions. It handles authentication, history, approvals, and streamed agent events. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The protocol is bidirectional JSON-RPC 2.0. Documented transports include stdio with JSONL, WebSocket text frames, and Unix sockets. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Core objects are Thread, Turn, and Item. Items include messages, commands, file changes, and tool calls. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The client performs an initialization handshake, starts or resumes threads, starts or steers turns, receives streamed notifications, and answers approval requests. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Thread metadata supports resume, fork, pin/unpin, archive, and runtime status. ([App-server documentation](https://developers.openai.com/codex/app-server))
- OpenAI’s launch post says the desktop app uses isolated worktrees and carries over CLI/IDE history and configuration. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))

**Implication for a Tauri workbench**

Use a typed event bridge between the Tauri UI and an agent service. Keep conversation state, approvals, tool calls, file changes, and subagent status as separate event types. Do not make the UI infer state from rendered text.

## Top 10 patterns worth adopting

1. Project → thread navigation with durable resume and fork. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))
2. A center conversation that remains the primary surface while rails stay contextual. ([AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/))
3. Visible model, reasoning, approval, and sandbox controls at send time. ([App-server documentation](https://developers.openai.com/codex/app-server))
4. Searchable slash commands for advanced operations. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))
5. Attachment chips that preserve screenshots, extracted text, and source identity. ([Appshots documentation](https://developers.openai.com/codex/appshots))
6. A docked browser with near-fullscreen expansion. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app), [Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
7. Annotations carrying both visual evidence and semantic target data. ([Codex browser documentation](https://developers.openai.com/codex/app/browser), [Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
8. Structured file-change cards with review and undo. ([App-server documentation](https://developers.openai.com/codex/app-server), [AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/))
9. Collapsible activity details backed by streamed events. ([App-server documentation](https://developers.openai.com/codex/app-server))
10. Sidebar state indicators plus in-app and OS-level completion notifications. ([App-server documentation](https://developers.openai.com/codex/app-server), [GitHub issue #13019](https://github.com/openai/codex/issues/13019))

No files were edited or committed.
tokens used
217,001
Bottom line: Codex is a conversation-first command center: projects and threads organize work, the center shows the active turn, and secondary rails expose plans, artifacts, sources, and agents. The strongest reusable ideas are structured agent state, reviewable file changes, and a browser that turns visual comments into actionable context. Some current UI details—exact rail labels, resize gestures, chip text, and annotation payload fields—are only described by community reports, so they should be verified against the live Mac build.

## 1. Conversation-centric layout

### Center: the conversation is the workspace

- The Mac app is designed to manage several agents and long-running tasks at once. Each agent runs in its own thread, and threads are organized under projects. The active thread remains the primary workspace for messages, progress, approvals, and review. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))
- Reviewers describe a three-pane layout: project/thread navigation on the left, the active conversation in the center, and contextual results or review content on the right. ([AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/), [VibeCoding review](https://vibecoding.app/blog/openai-codex-app-review))

**Edge behavior**

- Long-running work persists as a thread that can be resumed or forked; the public protocol treats a thread as the durable conversation and a turn as one request plus its agent work. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Multiple agents can work in separate worktrees, so their changes do not overwrite one another. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))

**Worth copying: yes.** A conversation should own the center; project navigation and diagnostics should remain secondary.

### Left sidebar: projects, threads, pinning, and “Show more”

- Projects group related threads; the sidebar is the main navigation surface for switching between workstreams. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/), [VibeCoding review](https://vibecoding.app/blog/openai-codex-app-review))
- The app-server protocol exposes thread metadata for pinning and unpinning, so pinned threads are a first-class state rather than a visual-only shortcut. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Current builds use a “Show more” or expansion path for additional threads, but users have reported that it can show only a capped or already-loaded candidate set. ([OpenAI Community thread](https://community.openai.com/t/search-or-show-all-threads-in-codex-ide-or-app/1375338), [GitHub issue #27314](https://github.com/openai/codex/issues/27314))
- Worktree-created threads have also been reported as failing to group correctly under the expected project. ([GitHub issue #10522](https://github.com/openai/codex/issues/10522))

**Edge behavior**

- The important implementation requirement is true project-scoped pagination or search. Expanding a global list is not sufficient for large histories. ([GitHub issue #27314](https://github.com/openai/codex/issues/27314))

**Worth copying: yes, with robust loading.** Projects, pins, and thread history are useful; a misleading “Show more” is not.

### Right rail: Plan, Outputs, Subagents, Sources

- A 2026 workspace update added a task/sidebar view that surfaces the agent’s plan, tracked sources, and generated artifacts. ([Codex Workspace Knowledge Base](https://codex.danielvaughan.com/2026/04/17/codex-app-workspace-pr-review-task-sidebar-artifact-viewer/), [TechRadar report](https://www.techradar.com/pro/codex-can-now-operate-your-computer-alongside-you-with-major-workplace-updates))
- “Plan” is also represented as structured progress state. The composer can invoke `/plan` and `/goal`, and the protocol streams plan updates. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands), [App-server documentation](https://developers.openai.com/codex/app-server))
- “Outputs” corresponds to generated artifacts and previews such as documents, spreadsheets, slides, and other files. ([Codex Workspace Knowledge Base](https://codex.danielvaughan.com/2026/04/17/codex-app-workspace-pr-review-task-sidebar-artifact-viewer/), [TechRadar report](https://www.techradar.com/pro/codex-can-now-operate-your-computer-alongside-you-with-major-workplace-updates))
- The Subagents view exposes each child agent’s status, allows opening its individual thread, and supports stopping agents. ([Subagents documentation](https://developers.openai.com/codex/subagents))
- Sources track files, URLs, and other context used by the task. ([Codex Workspace Knowledge Base](https://codex.danielvaughan.com/2026/04/17/codex-app-workspace-pr-review-task-sidebar-artifact-viewer/))

**Opening, closing, and resizing**

- Community users report a right-side browser/context panel opened with a toolbar control or `Cmd+Option+B`, with an expand control that makes it nearly full-screen while leaving a floating chat surface. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
- I found no authoritative public documentation specifying a resize gesture or exact minimum/maximum rail width. That behavior should be treated as release-sensitive.

**Worth copying: yes.** A persistent contextual rail prevents the conversation from becoming a dumping ground, but it needs predictable collapse and width behavior.

## 2. Composer and prompt input

### Model, effort, approval, and sandbox controls

- The public runtime accepts model, reasoning effort, approval policy, and sandbox settings when a thread starts. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The composer exposes related controls through model/reasoning commands such as `/model`, `/reasoning`, and `/fast`; exact picker labels vary by release. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))
- Codex’s documented approval modes include asking before actions, allowing edits automatically, and fully automatic execution; the desktop app uses the same underlying permission model. ([OpenAI Codex usage guidance](https://help.openai.com/en/articles/11096431), [OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))

**Worth copying: yes.** Keep execution policy visible at the point where the user sends work.

### Attachments, screenshots, and voice

- The desktop app supports voice input: select the Voice control or its shortcut, grant microphone access, speak, and interrupt naturally while live text appears. ([ChatGPT Work and Codex help](https://help.openai.com/en/articles/20001275))
- Appshots capture the frontmost Mac window as an image plus text the app exposes, behave like attachments, and are stored locally with the session. Pressing both Command keys is the default capture gesture. ([Appshots documentation](https://developers.openai.com/codex/appshots))
- The built-in browser supports visual comments and sends the resulting comment context back through chat. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app), [Codex browser documentation](https://developers.openai.com/codex/app/browser))

**Not fully documented**

- I found no authoritative public text that specifies Finder drag-and-drop, generic screenshot paste, the exact attach-button layout, or the exact chip styling. These should be validated in the live Mac application rather than assumed from the protocol.

**Worth copying: yes.** Show every attachment as a removable, inspectable chip before send; preserve the source image and any extracted text.

### Slash commands

- Typing `/` opens a searchable command menu. `$` explicitly selects a skill. Available commands include `/model`, `/reasoning`, `/plan`, `/goal`, `/review`, `/fork`, `/project`, `/worktree`, `/local`, and `/cloud`. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))

**Edge behavior**

- Goal state can remain visible above the composer with pause, resume, edit, and clear actions. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))

**Worth copying: yes.** Slash commands reduce hidden mode switches and make advanced actions discoverable without adding permanent toolbar clutter.

## 3. Embedded browser and annotation mode

### Docked browser

- The desktop app has a built-in browser with its own browser profile and state, separate from Chrome. It supports multiple tabs, downloads, sign-in flows, and permission prompts. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app))
- The browser can be opened from the desktop toolbar; the documented Mac shortcut is `Cmd+Shift+B`. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app))
- Community reports describe the browser as a right-side panel with an expand button that makes it nearly full-screen while preserving a floating chat control. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))

**Edge behavior**

- Official documentation confirms multiple tabs, but does not specify the exact tab-strip appearance or tab-reordering behavior. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app))

**Worth copying: yes.** A docked browser keeps visual review beside the agent conversation; near-fullscreen expansion is useful for dense pages.

### Annotation mode

- Officially, the user can select a page element or drag over an area, write a comment, save it, and send that feedback through chat. ([Codex browser documentation](https://developers.openai.com/codex/app/browser))
- Community reports describe numbered pins placed on the page, an inline comment box, and the browser returning to chat with the comments attached. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
- A community answer reports that an annotation contains a screenshot with the selected area marked, the page URL, a selector/path such as `html > body … > h1`, and the target text. This is not confirmed by official protocol documentation. ([Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
- I found no public source confirming that raw pixel coordinates are part of the stable annotation contract. Screenshot-coordinate bugs have occurred because of zoom and device-pixel-ratio differences. ([GitHub issue #19429](https://github.com/openai/codex/issues/19429))
- Current UI reports mention an annotation-count chip in the composer and a “Send N” action. A later issue reports that pressing Enter began submitting the first comment immediately, breaking multi-comment batching. ([Codex issue mirror #22719](https://codexissues.com/issue/22719-in-app-browser-comments-submit-immediately-on-enter-instead-of-allowing-multiple))

**Worth copying: yes.** Combine visual evidence with semantic identity (URL, selector, text); do not rely on coordinates alone.

## 4. Turn rendering

### Messages and streaming

- The public protocol distinguishes user messages, agent messages, commands, file changes, and tool calls as separate items. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Agent text streams as incremental `agentMessage/delta` notifications, while items emit started and completed events. ([App-server documentation](https://developers.openai.com/codex/app-server))
- This gives the UI enough structure to render user text, agent text, tool activity, and edits differently instead of treating everything as one transcript.

**Worth copying: yes.** Render from typed events, not from a single string stream.

### File-edit and tool cards

- File changes have an explicit lifecycle: proposed change, approval request, client decision, then completed, failed, or declined. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Review-oriented desktop guides describe file-change cards with changed-file counts, line deltas, inline annotations, review, rollback, and staging actions. ([AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/))
- The exact phrase “Edited 5 files +53 -15” is a current UI string, not a documented API contract. A Tauri workbench should treat the counts and actions as structured fields, not parse display text.

**Worth copying: yes.** “Review” and “Undo” at the point of change make agent work reversible.

### Activity and elapsed-time rows

- The protocol exposes turn start/completion and tool-progress events, so the client can show an activity section and elapsed time. ([App-server documentation](https://developers.openai.com/codex/app-server))
- I found no official documentation that guarantees the exact “Worked for Xm Ys” wording or whether the activity section is always collapsed. Treat that label as release-specific.

**Worth copying: yes.** Collapse noisy activity by default while keeping the full event history available.

## 5. Working-state signals

### Active threads and subagents

- Thread status includes `active`, `idle`, `systemError`, and `notLoaded`; active flags can include waiting for approval. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The protocol emits `thread/status/changed` events, allowing the sidebar to update without polling. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The Subagents panel shows active child agents, their status, stop controls, and links to their individual threads. ([Subagents documentation](https://developers.openai.com/codex/subagents))

**Edge behavior**

- A sidebar indicator should distinguish running, waiting for approval, completed, failed, and disconnected states. The public protocol supplies the state; the exact iconography is a UI choice. ([App-server documentation](https://developers.openai.com/codex/app-server))

**Worth copying: yes.** State-aware navigation is essential when several agents run concurrently.

### Completion notifications

- Codex supports desktop completion notifications, but an open issue reports that notification settings can be ignored in app-server mode. ([GitHub issue #13019](https://github.com/openai/codex/issues/13019))

**Worth copying: yes, with fallback.** Provide an in-app completion marker even when operating-system notifications are unavailable or suppressed.

## 6. Publicly known architecture

- OpenAI describes the app-server as the interface used by rich clients such as IDE extensions. It handles authentication, history, approvals, and streamed agent events. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The protocol is bidirectional JSON-RPC 2.0. Documented transports include stdio with JSONL, WebSocket text frames, and Unix sockets. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Core objects are Thread, Turn, and Item. Items include messages, commands, file changes, and tool calls. ([App-server documentation](https://developers.openai.com/codex/app-server))
- The client performs an initialization handshake, starts or resumes threads, starts or steers turns, receives streamed notifications, and answers approval requests. ([App-server documentation](https://developers.openai.com/codex/app-server))
- Thread metadata supports resume, fork, pin/unpin, archive, and runtime status. ([App-server documentation](https://developers.openai.com/codex/app-server))
- OpenAI’s launch post says the desktop app uses isolated worktrees and carries over CLI/IDE history and configuration. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))

**Implication for a Tauri workbench**

Use a typed event bridge between the Tauri UI and an agent service. Keep conversation state, approvals, tool calls, file changes, and subagent status as separate event types. Do not make the UI infer state from rendered text.

## Top 10 patterns worth adopting

1. Project → thread navigation with durable resume and fork. ([OpenAI launch post](https://openai.com/index/introducing-the-codex-app/))
2. A center conversation that remains the primary surface while rails stay contextual. ([AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/))
3. Visible model, reasoning, approval, and sandbox controls at send time. ([App-server documentation](https://developers.openai.com/codex/app-server))
4. Searchable slash commands for advanced operations. ([Slash-command reference](https://developers.openai.com/codex/reference/slash-commands))
5. Attachment chips that preserve screenshots, extracted text, and source identity. ([Appshots documentation](https://developers.openai.com/codex/appshots))
6. A docked browser with near-fullscreen expansion. ([Built-in browser help](https://help-lb.openai.com/en/articles/20001277-using-the-built-in-browser-in-the-chatgpt-desktop-app), [Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
7. Annotations carrying both visual evidence and semantic target data. ([Codex browser documentation](https://developers.openai.com/codex/app/browser), [Reddit browser discussion](https://www.reddit.com/r/codex/comments/1svaj3v/most_slept_on_feature_in_the_codex_app/))
8. Structured file-change cards with review and undo. ([App-server documentation](https://developers.openai.com/codex/app-server), [AI Docs Hub desktop overview](https://ai-docs-hub.soarx.cc/en/guides/codex/07-desktop-app/))
9. Collapsible activity details backed by streamed events. ([App-server documentation](https://developers.openai.com/codex/app-server))
10. Sidebar state indicators plus in-app and OS-level completion notifications. ([App-server documentation](https://developers.openai.com/codex/app-server), [GitHub issue #13019](https://github.com/openai/codex/issues/13019))

No files were edited or committed.
