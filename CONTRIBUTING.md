# Contributing to Assembly

Thanks for helping. Assembly is a small project with a strong opinion: it should stay fast and light while doing the work of an agent chat app and an IDE. This guide covers what you need to make a change that can be merged.

## Set up

Follow [Build from source](README.md#build-from-source) in the README. In short, on macOS with Node.js 22, pnpm 10.28.2, stable Rust, and Bun:

```sh
git clone https://github.com/jcoble/mac-command-bar.git
cd mac-command-bar
git config core.hooksPath .githooks
pnpm --dir tauri-svelte-preview install --frozen-lockfile
pnpm app:dev
```

The `core.hooksPath` line turns on the repository's commit-message hook (see [Commit messages](#commit-messages)). Run it once per clone.

The dev app uses your real local session database. Expect it to see your actual provider history and projects.

Before you change anything, read:

- [`main-architecture-explained.html`](main-architecture-explained.html), which explains how the app is put together.
- [`AGENTS.md`](AGENTS.md), the project's working rules. The sections below are the ones that matter most to contributors.

## Branches and pull requests

- Fork the repository, or create a branch if you have access. Use a short descriptive name such as `fix-diff-scroll` or `editor-rust-hover`. Maintainers also use `tsk-<id>-<slug>` for work tracked in their task board, but you do not need to.
- Keep one problem per pull request. Unrelated clean-ups belong in their own PR.
- In the PR description, say what changed, why, and how you checked it. For anything visible, include a screenshot or a short clip (at most four screenshots and one video of two minutes or less).

## Checks to run before a pull request

From the repository root:

```sh
pnpm --dir tauri-svelte-preview check   # svelte-kit sync, tsc --noEmit, cargo check with warnings as errors
pnpm --dir tauri-svelte-preview build   # production frontend build
```

Then run the tests for the area you touched:

```sh
cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml   # Rust backend
(cd core && cargo test)                                                 # mcb-core crate
pnpm --dir tauri-svelte-preview test:<name>                             # focused frontend logic tests
pnpm --dir tauri-svelte-preview test:async-lifecycle                    # if you added a timer or interval
```

The full list of `test:*` scripts is in [`tauri-svelte-preview/package.json`](tauri-svelte-preview/package.json). If you changed something visible, check it in the running app (`pnpm app:dev`) as well.

## Rules that matter

**Memory and speed are features.** The app exists because similar tools are slow and heavy. A correct change that makes Assembly feel slower, or adds CPU or memory cost at idle, is a defect. In practice:

- Prefer event-driven updates to polling. Any interval must be small in scope and stop when its element is not visible.
- Animations must end. A looping animation is allowed only while real work is happening, and it stops when hidden. Animate `transform` and `opacity`, not layout properties.
- Keep long lists cheap offscreen with virtualization or `content-visibility: auto`.
- Do heavy work (Git, file system, processes) in the Rust backend, not in the web view.
- Judge memory in Activity Monitor, not in the Web Inspector, which inflates it.

**Never use CSS `:has()`.** Once any `:has()` rule exists, WebKit re-checks it on every DOM change in the document, and profiling showed seconds lost to it. Tailwind's `has-[...]` variants compile to `:has()` and are banned too. Set a state class on the element that owns the state instead.

**No UI regression tests for now.** Do not add tests that assert on component structure, layout, styling, or colours; the UI is still changing too fast for them to mean anything. Tests for logic and data are welcome: parsers, reducers, view models, stores, protocol contracts, and Rust backend behaviour.

**Follow the UI design rules.** [`tauri-svelte-preview/src/lib/components/ui/DESIGN.md`](tauri-svelte-preview/src/lib/components/ui/DESIGN.md) is binding for interface work.

**Keep it simple, and keep one way of doing things.** Build the smallest change that works. Do not add abstractions, options, or fallback paths for needs that do not exist yet. When you replace a mechanism, delete the old one in the same change; never leave two paths for the same job.

**Write TypeScript.** New frontend and tooling code is TypeScript, including scripts and tests. Do not add new `.js` or `.mjs` files.

**Keep the architecture guide true.** If your change alters anything `main-architecture-explained.html` describes, such as a Tauri command, an event, a table, a provider protocol call, or a lifecycle rule, update the guide in the same commit and add a row to its change log.

## Commit messages

The hook in `.githooks/commit-msg` checks every commit except merges:

- The last line of the message must be `Committed-by: <your name>`.
- The message must not contain a `Co-Authored-By:` trailer.
- The commit must either stage `main-architecture-explained.html`, or include the line `Architecture: unchanged` in the body when the change does not affect anything the guide describes (styling, copy, tests, tooling).
- On a branch named `tsk-<number>-…`, the message must also include `Task: TSK-<number> https://app.notion.com/p/<page-id>`. Outside contributors can ignore this by not using `tsk-` branch names.

Example:

```text
Fix the diff view losing its scroll position on refresh

The view now keeps the first visible hunk when new changes arrive.

Architecture: unchanged

Committed-by: Your Name
```

## Reporting bugs

Open a [GitHub issue](https://github.com/jcoble/mac-command-bar/issues) and include:

- the app version or commit, and your macOS version
- the provider (Codex, Claude, or Antigravity) and whether the project is local or remote
- the steps to reproduce, what you expected, and what happened
- for slowness or memory problems, the numbers from Activity Monitor and what you were doing at the time

Remove credentials, tokens, and private conversation or repository content from logs and screenshots before you share them.

## Conduct

Be respectful and constructive. Assume good intent, keep criticism about the code, and help newcomers find their way.
