# Assembly

**A desktop workspace for AI-assisted software development.**

Assembly brings coding-agent conversations, project files, Git changes, pull requests, and browser previews into one macOS app. Work with Codex, Claude, or Antigravity, keep conversations organized by project, and review the results alongside the code.

Projects can live on your Mac or on a remote development machine. Assembly keeps the desktop interface local while the remote backend runs agents and repository operations where the project lives.

> Assembly is under active development. The repository retains its original name, `mac-command-bar`; the current desktop app is built with Tauri, Svelte, TypeScript, and Rust.

[Releases](https://github.com/jcoble/mac-command-bar/releases) · [Issues](https://github.com/jcoble/mac-command-bar/issues) · [Architecture guide](main-architecture-explained.html)

## Features

### Coding-agent conversations

- Use Codex, Claude, and Antigravity through provider adapters.
- Resume conversations with their original provider sessions, with model and effort controls where supported.
- Send screenshots and image attachments, inspect tool activity, and respond to permission requests in the conversation.
- Browse saved history, collapse completed work, and inspect session context and changed files.

### Editing and project navigation

- Explore project files and edit code in the integrated CodeMirror editor.
- Read rendered Markdown and compare file versions in a dedicated diff view.
- Browse file history and use language-server features for supported project environments.
- Open web previews in the built-in browser while keeping the conversation and code close at hand.

### Git and pull requests

- Inspect working-tree changes, stage files, commit, and publish branches from Source Control.
- Navigate Git history and manage worktrees for separate streams of work.
- Find GitHub pull requests, read their discussions and checks, and review changed files in the main workspace.
- Open PR diffs in the full-width diff viewer, draft review comments with the Helper, and edit the text before confirming a post.
- Merge pull requests from the app, subject to GitHub permissions, checks, and repository rules.

### Local and remote work

- Connect to development machines over SSH and work with their projects from the same desktop interface.
- Install and update the remote backend from Settings. The packaged remote installer currently targets Linux x86-64.
- Cache remote conversation history in the local session database and retrieve missing or newer history as needed.
- Retain cached history across app restarts and temporary connection failures. Explicit disconnect, machine removal, or successful backend uninstall clears that machine's local history cache.

### Tasks, helpers, and resource visibility

- Browse Notion tasks, search and filter the list, read task details, and update task status.
- Use the Helper for session titles, Inspect answers, and editable PR review drafts. The selected Codex or Claude CLI is the default route; API use is an explicit Settings choice.
- Inspect app resource use, session context, and provider usage from the workspace.

## Getting started

For packaged builds, see [GitHub Releases](https://github.com/jcoble/mac-command-bar/releases). To run the current source, follow the development steps below.

1. Set up and sign in to the coding provider you want to use. Codex and Claude integrations use the corresponding installed CLI; Antigravity handles sign-in through its adapter.
2. Open a project and start or resume a conversation.
3. Use the editor, Source Control, and diff views to inspect the agent's changes.
4. For remote projects, add the machine in **Settings → Connections** and connect its backend.
5. Connect Notion from the Tasks panel if you want task tracking in the workspace.

GitHub features require an authenticated **GitHub CLI (`gh`), version 2.63 or newer**, on the machine that owns the repository. Remote agent sessions likewise use provider installations and credentials on the remote machine. Provider access and billing remain with your chosen provider.

## Development

### Requirements

- macOS with Xcode Command Line Tools.
- Node.js 22.18 or newer, with TypeScript stripping support.
- pnpm 10.28.2, matching the frontend's `packageManager` field.
- Stable Rust and Cargo.
- Bun, used to prepare the bundled provider adapters.
- Git, plus the provider CLIs and GitHub CLI for the integrations you use.

### Clone and run

```sh
git clone https://github.com/jcoble/mac-command-bar.git
cd mac-command-bar
pnpm --dir tauri-svelte-preview install --frozen-lockfile
pnpm app:dev
```

The development launcher prepares missing adapter payloads, starts the frontend server, builds the Rust app, and opens Assembly. Initial setup requires network access to download dependencies and provider payloads. Keep the terminal open while using the app; **Ctrl+C** stops the development run.

The dev app uses its normal local session database. Starting it is not a disposable test environment.

### Update main and launch

From the main checkout:

```sh
pnpm app:update
```

This command pulls `origin/main` with fast-forward only, installs the locked frontend dependencies, and starts the dev app. It preserves local edits and includes them in the build. Conflicting changes or diverged commit history stop the update; the script does not stash, discard, or merge your work automatically.

On a feature branch, use `pnpm app:dev` to run the current checkout without pulling main.

### Checks

Run these from the repository root:

```sh
# TypeScript and Rust checks
pnpm --dir tauri-svelte-preview check

# Svelte component diagnostics
pnpm --dir tauri-svelte-preview exec svelte-check --tsconfig ./tsconfig.json

# Production frontend build
pnpm --dir tauri-svelte-preview build

# Focused conversation logic tests
pnpm --dir tauri-svelte-preview test:conversation-timeline
pnpm --dir tauri-svelte-preview test:agent-conversation-store
```

Additional focused checks are listed in [the frontend package scripts](tauri-svelte-preview/package.json). A frontend build alone does not package the macOS app. Release packaging and updater signing are defined in [the release workflow](.github/workflows/release.yml).

## Repository guide

| Path | Purpose |
| --- | --- |
| [`tauri-svelte-preview/`](tauri-svelte-preview/) | Current Assembly desktop application; the folder retains its early development name. |
| [`tauri-svelte-preview/src/lib/shell/`](tauri-svelte-preview/src/lib/shell/) | Conversations, editor, workspace panels, and frontend state. |
| [`tauri-svelte-preview/src-tauri/`](tauri-svelte-preview/src-tauri/) | Rust backend, Tauri configuration, storage, provider integration, and remote operations. |
| [`scripts/`](scripts/) | Repository tooling, including the update-and-launch command. |
| [`docs/`](docs/) | Feature plans, design proposals, and supporting documentation. |
| [`infrastructure/`](infrastructure/) | Supporting service infrastructure, including Notion OAuth. |
| [`Sources/`](Sources/) and [`core/`](core/) | Earlier Swift app and Rust helper code retained in the repository. |

The [architecture guide](main-architecture-explained.html) is the canonical explanation of app boundaries, session lifecycles, storage, provider adapters, and remote execution. Open the HTML file in a browser to read it. The earlier Swift build script, `scripts/build-app.sh`, builds the original MacCommandBar app; use the Tauri commands above for Assembly.

## Contributing and reporting issues

Read [AGENTS.md](AGENTS.md) before making changes. Keep pull requests focused, include relevant test results, and check user-visible behavior in the running app. Changes to the architecture described in the guide must update that guide in the same commit.

When reporting an issue, include the app version or commit, macOS version, provider, whether the project is local or remote, and the steps needed to reproduce it. Remove credentials and private conversation or repository content from logs and screenshots before sharing them.
