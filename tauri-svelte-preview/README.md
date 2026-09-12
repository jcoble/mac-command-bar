# MacCommandBar Tauri/SvelteKit Preview

Small visual prototype for comparing a WebKit/Tauri source preview against the native Swift/AppKit implementation.

The SvelteKit app follows Tauri's current SvelteKit guidance: static adapter, SPA mode, `build/` output, and a Tauri `devUrl` pointed at Vite.

## Run

```bash
pnpm install
pnpm dev
```

Open `http://127.0.0.1:5177` for the browser preview.

For the Tauri shell:

```bash
pnpm tauri:dev
```

If Vite is already running on `127.0.0.1:5177`, use attach mode so Tauri does
not try to start a second dev server:

```bash
pnpm tauri:dev:attach
```

The browser preview uses the local Vite source bridge for filesystem-backed source scans, reads,
writes, and basic search. The Tauri shell uses native Rust commands for the same source operations
plus native Git, LSP, terminal, worktree, process, and session actions.

## Appearance

Source preview font and Monaco theme settings live in `src/lib/sourcePreviewAppearance.ts`.
The `theme.id` value is registered with Monaco before the editor is created, so changing the
theme colors or font stack in that file is reflected by the web viewer after Vite reloads.

## Verify

```bash
pnpm check
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
pnpm test:native-lsp
```

Monaco is the default source surface, so the prototype now exercises a real editor viewport instead of static highlighted HTML. The current build intentionally accepts Monaco's large editor chunks for this visual comparison; production would lazy-load the editor route and add language intelligence only where needed.

`pnpm test:native-lsp` exercises the Rust LSP bridge directly. It verifies the legacy
TypeScript, Svelte, and Rust request path plus the native C# bridge lifecycle. C# editor
intelligence runs through Monaco's VS Code-compatible language client and is verified in
the desktop app rather than by the retired direct-stdio client.

## Orchestration Events

Long-running agent loops can append visual run events through the repo-level
bridge:

```bash
../scripts/mcb-orch --run-id run-tsk-127 --preset scenario-started --scenario "Trading partner setup"
../scripts/mcb-orch --run-id run-tsk-127 --preset issue-found --issue-id AUTH-7 --issue-count 1
../scripts/mcb-orch --run-id run-tsk-127 --preset batch-delegated --agent-role fix-agent --fix-count 2
../scripts/mcb-orch --run-id run-tsk-127 --preset ui-verified --scenario "Trading partner setup" --verified-count 1
```

The Source Browser command palette can copy ready-to-edit versions of those
commands for the selected project: run-started, scenario-started, issue-found,
fix-batch, UI-verified, and approval-required.
