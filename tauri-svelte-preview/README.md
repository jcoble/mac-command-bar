# MacCommandBar Tauri/SvelteKit Preview

Small visual prototype for comparing a WebKit/Tauri source preview against the native Swift/AppKit implementation.

The SvelteKit app follows Tauri's current SvelteKit guidance: static adapter, SPA mode, `build/` output, and a Tauri `devUrl` pointed at Vite.

## Run

```bash
npm install
npm run dev
```

Open `http://127.0.0.1:5177` for the browser preview.

For the Tauri shell:

```bash
npm run tauri:dev
```

The browser preview uses embedded demo source. The Tauri shell can call `read_source_file` from Rust to load the listed local files.

## Verify

```bash
npm run check
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Shiki gives the source surface TextMate-style highlighting. The current build intentionally accepts Shiki's large syntax chunks for this visual comparison; production would narrow loaded languages/themes or move highlighting to the Rust helper.
