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

The browser preview uses embedded demo source. The Tauri shell can call `read_source_file` from Rust to load the listed local files.

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

`pnpm test:native-lsp` exercises the Rust LSP bridge directly. It uses TypeScript and C# scratch workspaces and verifies symbols, hover, definition, references, and diagnostics against the same registry used by the Tauri commands.
