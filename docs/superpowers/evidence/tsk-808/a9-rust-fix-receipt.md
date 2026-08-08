# TSK-808 A9 Rust Fix Receipt

Status: [verified] implementation and requested source checks complete; the full suite has the single permitted sandbox bind failure recorded below.

Scope: [verified] only `src-tauri/src/browser.rs`, `src-tauri/src/browser_inspector.js`, `src-tauri/src/main.rs`, and this receipt are allowed to change. No commit or push will be made.

## Work log

- Receipt created before source inspection (verified).
- Completed review receipt `docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md` read in full before source edits (verified).
- Rust formatting was run on the two Rust files after the final edit (verified).
- The three required cargo checks were run sequentially after the final edit (verified).

## Changed files and line ranges

- [verified] `tauri-svelte-preview/src-tauri/src/browser.rs:1-2157` was completed. The changed regions cover the URL and payload limits (`:9-45`, `:1097-1199`, `:1543-1807`), serde request/response types and registry seams (`:48-400`), registry lifecycle and generation handling (`:420-952`), the native factory (`:1339-1533`), all command wrappers (`:1203-1337`), and the pure-logic tests (`:1810-2157`).
- [verified] `tauri-svelte-preview/src-tauri/src/browser_inspector.js:182-196` now installs the frozen inspector API through a non-configurable, non-enumerable, non-writable property descriptor.
- [verified] `tauri-svelte-preview/src-tauri/src/main.rs:21,24-25,5306,5410-5431` imports the manager trait, declares the browser module, manages `BrowserRegistry`, registers the browser commands, and shuts the registry down on `WindowEvent::Destroyed`.
- [verified] This receipt (`docs/superpowers/evidence/tsk-808/a9-rust-fix-receipt.md:1-125`) is the only file created by this lane. No commit or push was made. Pre-existing edits in `Cargo.toml`, `git_diff_models.rs`, `lsp.rs`, the frontend receipt, and overlay/review evidence were preserved.

## Implemented contract

- [verified] `normalize_browser_url` trims input, permits blank/about:blank only when requested, caps URLs at 8 KiB, and accepts only HTTP(S) without userinfo. The Tauri navigation callback applies the same allowlist before a native child receives a URL.
- [verified] The registry keeps one native child view per workspace/tab, makes same-generation create retries idempotent, validates opaque profiles and identities, validates bounds and viewport values, and carries the generation into page-load/title callbacks so stale callbacks are dropped.
- [verified] Navigation calls the native view before changing registry identity; close calls native close before removing the tab; hide/show/shutdown cancel the fixed inspector listener before hiding or closing; window teardown calls `BrowserRegistry::shutdown`.
- [verified] The raw inspector callback is rejected above 64 KiB before JSON parsing. Parsed metadata is bounded, unsafe page URLs and page changes become unavailable events, and the reason field is capped before emission.
- [verified] The native factory creates a uniquely labelled Tauri child webview under the main window, injects `browser_inspector.js`, uses the workspace profile directory, and binds page-load/title callbacks to the tab generation.
- [verified] `capture_browser_viewport` has the required typed response (`mimeType`, `bytes`, `width`, `height`, `sourceHash`) but returns a typed `Unsupported` error because this Tauri/wry version has no safe snapshot wrapper; no guessed image bytes were added.
- [assumed] The frontend currently exposes an in-memory backend and contains no Tauri `invoke` adapter, so live IPC invocation was not available for proof. The Rust structs use `camelCase` serde names and the wrapper signatures match the frontend input and result interfaces by static inspection.

## Command-name match table re-verified after the changes

[verified] The frontend names and input/result shapes were re-read from `tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts:9-65,100-210`. Every frontend command has a Rust `#[tauri::command]` wrapper and a `generate_handler!` entry; the plan-required cleanup command is included as an additional row.

| Command | Frontend contract | Rust wrapper | Registered | Result |
|---|---:|---:|---:|---|
| `create_browser_tab` | `:50,100-116` | `browser.rs:1203` | `main.rs:5410` | [verified] match; returns `{tabId,generation}` |
| `set_browser_tab_bounds` | `:51,119-123` | `browser.rs:1216` | `main.rs:5411` | [verified] match; void |
| `set_browser_tab_viewport` (optional) | `:52,125-129` | `browser.rs:1231` | `main.rs:5412` | [verified] match; void |
| `show_browser_tab` | `:53,131-135` | `browser.rs:1239` | `main.rs:5413` | [verified] match; void |
| `hide_browser_workspace` | `:54,137-142` | `browser.rs:1247` | `main.rs:5414` | [verified] match; void |
| `navigate_browser_tab` | `:55,144-152` | `browser.rs:1255` | `main.rs:5415` | [verified] match; void |
| `reload_browser_tab` | `:56,154-156` | `browser.rs:1263` | `main.rs:5416` | [verified] match; void |
| `go_back_browser_tab` | `:57,158-165` | `browser.rs:1271` | `main.rs:5417` | [verified] match; void |
| `go_forward_browser_tab` | `:58,167-174` | `browser.rs:1279` | `main.rs:5418` | [verified] match; void |
| `close_browser_tab` | `:59,176-179` | `browser.rs:1287` | `main.rs:5419` | [verified] match; void |
| `clear_browser_workspace_data` (plan-required) | `:2115-2117` in the review plan | `browser.rs:1295` | `main.rs:5420` | [verified] wrapper and registration present |
| `arm_browser_element_picker` | `:60,181-183` | `browser.rs:1303` | `main.rs:5421` | [verified] match; void |
| `cancel_browser_element_picker` | `:61,185-187` | `browser.rs:1311` | `main.rs:5422` | [verified] match; void |
| `capture_browser_viewport` | `:62,189-202` | `browser.rs:1319` | `main.rs:5423` | [verified] typed capture result or `Unsupported` |
| `open_browser_tab_devtools` (optional) | `:63,204-206` | `browser.rs:1327` | `main.rs:5424` | [verified] match; void/typed unsupported in release |
| `open_browser_tab_external` (optional) | `:64,208-210` | `browser.rs:1335` | `main.rs:5425` | [verified] match; void |

[verified] The static command set comparison reported no frontend command without a Rust wrapper and no wrapper without a registration.

## Pure-logic tests

- [verified] `browser.rs:1957-1978` rejects `file:`, `javascript:`, `data:`, custom schemes, userinfo, and overlong URLs while checking blank/about:blank and canonical HTTP(S).
- [verified] `browser.rs:1981-2029` covers insert, close/remove, native close, window-shutdown cleanup, and empty registry state.
- [verified] `browser.rs:2030-2110` covers idempotent retry, stale generation, unknown identity, generation monotonicity, identity mismatch, and stale page-load callbacks.
- [verified] `browser.rs:2111-2157` covers the 64 KiB inspector payload cap, unavailable reason, and picker-state cleanup without creating a real webview.

## Verification commands and output tails

[verified] All commands below were run from `tauri-svelte-preview/` and each cargo process was allowed to finish before the next started.

### Build

Command (verbatim):

```text
cargo build --manifest-path src-tauri/Cargo.toml
```

- [verified] Exit code: `0`.
- [verified] Output tail:

```text
warning: `mac-command-bar-webview-preview` (bin "mac-command-bar-webview-preview") generated 75 warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.12s
```

### Browser-filtered tests

Command (verbatim):

```text
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml browser
```

- [verified] Exit code: `0`; four browser tests ran and passed.
- [verified] Output tail:

```text
running 4 tests
test browser::tests::generation_and_identity_guards_reject_stale_targets_and_retries_are_idempotent ... ok
test browser::tests::inspector_payload_cap_emits_unavailable_without_parsing_large_input ... ok
test browser::tests::registry_insert_remove_and_shutdown_cleanup_on_window_close ... ok
test browser::tests::url_boundary_rejects_unsafe_schemes_and_userinfo ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 216 filtered out; finished in 0.00s
```

### Full Rust suite

Command (verbatim):

```text
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml
```

- [verified] Exit code: `101` with `218 passed`, `1 failed`, and `1 ignored`.
- [verified] The only failure is the permitted environmental bind error, exactly:

```text
---- lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint stdout ----
called `Result::unwrap()` on an `Err` value: "Could not bind native C# bridge: Operation not permitted (os error 1)"
test result: FAILED. 218 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 10.56s
```

- [assumed] The bind failure is sandbox/environmental, as stated by the review receipt; no A9 browser source path appears in that failure.

## Final state

- [verified] `git diff --check` reported no whitespace errors.
- [verified] No commit or push was made; the worktree remains dirty only with the requested edits plus pre-existing unrelated files listed above.
