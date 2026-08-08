# TSK-808 A9 Rust Review Receipt

Review lane: Rust browser bridge and inspector boundary.

Scope constraints: review and tests only; no source edits; no commit; at most one cargo process at a time.

## Receipt log

- [assumed] Receipt created before repository inspection, per lane instructions.

## A9 contract baseline (verified)

The required Rust-side contract is taken from the A9 plan section at
`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2083-2173`:

- one native child view per `(workspace_id, tab_id)`, reused while hidden, measured, moved, and shown; no second iframe/WebView (`:2085-2089`, `:2111-2114`);
- bounded Rust commands: `create_browser_tab`, `set_browser_tab_bounds`, `show_browser_tab`, `hide_browser_workspace`, `navigate_browser_tab`, `reload_browser_tab`, `go_back_browser_tab`, `go_forward_browser_tab`, `close_browser_tab`, release-gated `open_browser_tab_devtools`, `open_browser_tab_external`, `clear_browser_workspace_data`, `arm_browser_element_picker`, `cancel_browser_element_picker`, and only A0-approved viewport capture (`:2111-2116`);
- events include workspace, tab, and monotonic generation; stale events are dropped (`:2115-2116`);
- reject non-HTTP/S, userinfo, `file:`, `data:`, `javascript:`, and custom protocols; profiles are opaque and workspace-isolated; cookie import stays disabled (`:2115-2117`);
- inspector/feedback metadata is bounded (selector <=2 KB, snippet <=500 chars, <=32 classes; UI contract also caps annotations at 100/tab) and excludes cookies, storage, form values, full HTML, tokens, and provider screenshots (`:2098-2101`, `:2117-2121`);
- Draw requires a native visible-viewport snapshot and restores the same live view without reload (`:2119-2120`); unsafe snapshot/inspector, broad capability, profile-isolation, owner/generation, z-order/input, or cleanup gaps are explicit stop conditions (`:2165-2173`).

## Initial wiring finding (verified)

`tauri-svelte-preview/src-tauri/src/main.rs:24-29` declares `agent_conversation`, `git_diff_models`, `lsp`, `orchestration`, `terminal`, and `workflow`, but not `browser`. The builder manages no `BrowserRegistry` at `:5292-5305`, and the `tauri::generate_handler!` list at `:5316-5408` contains no browser command. Therefore `browser.rs` is not compiled or registered by the current binary; the browser frontend's native calls cannot resolve in this build.

## Required browser-filtered test (verified)

Command (exact):

```text
RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml browser
```

Exit: `0` (verified). Output tail:

```text
warning: `mac-command-bar-webview-preview` (bin "mac-command-bar-webview-preview" test) generated 22 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 13.49s
     Running unittests src/main.rs (tauri-svelte-preview/src-tauri/target/debug/deps/mac_command_bar_webview_preview-7fbf27f85a481383)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 216 filtered out; finished in 0.00s
```

The command is green only because no browser module/tests are compiled or selected. Zero browser tests ran (verified).

## Full existing Rust suite (verified)

Command (exact):

```text
RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml
```

Exit: `101` (verified). The full suite ran `216` tests: `214 passed`, `1 failed`, `1 ignored` (verified). Output tail:

```text
---- lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint stdout ----

thread 'lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint' (113158) panicked at src/lsp.rs:6171:51:
called `Result::unwrap()` on an `Err` value: "Could not bind native C# bridge: Operation not permitted (os error 1)"

failures:
    lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint

test result: FAILED. 214 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 8.98s

error: test failed, to rerun `--bin mac-command-bar-webview-preview`
```

The failing test is unrelated to A9 by file/symbol evidence: it is `lsp.rs:6150-6176`, and the bind error originates at `lsp.rs:1550-1551`; this review does not assign a root cause beyond the observed sandbox `Operation not permitted` (environmental explanation is assumed, not proven).

## File-by-file review

### `tauri-svelte-preview/src-tauri/src/browser.rs`

Verdict: **keep with listed fixes**. This is not shippable as-is: the file is untracked, ends at line 1057, has no command wrappers, and references helpers/factory definitions that do not exist in the file (all verified).

Line review against the contract:

- [verified, contract-aligned] Lines 1-7 document one native child per workspace/tab and deliberately bounded inspector access; line 21 embeds a fixed script and lines 22-25 poll with fixed expressions/timeouts. No user string is interpolated into these three script strings (verified by lines 21-25 and 721-742).
- [verified, contract-aligned] Lines 32-43 define bounded identity/URL/title/selector/text/class/geometry limits; lines 168-185 carry workspace/tab/generation in selection events and lines 187-193 do so for close events.
- [verified] Lines 414-503 enforce nonzero generations, workspace generation monotonicity, idempotent same-generation retries, identity validation calls, and one registry entry per tab. However, `input.profile_id` is only validated at lines 424-426 and then ignored; the workspace always gets `BrowserProfile::new()` at lines 463-471. Profile A/B isolation therefore cannot be proven (the constructor is absent; the ignored value is verified).
- [verified] URL safety is not implementable/provable in this file: `normalize_browser_url` is called at lines 422, 612, 884, and 1021 but has no definition before EOF line 1057. The same is true of `clamp_browser_bounds` (line 423), `validate_identity`/`validate_profile_id` (lines 420-425), `TauriBrowserViewFactory` (lines 370 and 476), `BrowserProfile::new` (line 466), `title_from_url` (line 487), `open_external_url` (line 813), `parse_picker_payload` (line 977), `bounded_chars`/`bounded_utf8_bytes`/`sanitize_rect`/`browser_source_hash` (lines 931-1050). This is a compile blocker if `mod browser;` is added; the current cargo suites miss it because it is not registered.
- [verified, implementation gap] `BROWSER_INSPECTOR_SCRIPT` is only defined at line 21 and never consumed; the imported `WebviewBuilder`/`WebviewUrl` at line 18 have no factory implementation. Thus `browser_inspector.js` is not injected into any native child in this tree, and the fixed arm/cancel calls at lines 721-742 would see an absent API if the registry were called. The missing factory must install the script as a restricted initialization script and bind page-load/title callbacks to the same child identity (the intended binding is assumed from the unused imports).
- [verified, security gap] Because the URL helper is absent, the code does not establish the A9 rejection of `file:`, `data:`, `javascript:`, custom protocols, or userinfo. `create_tab` passes `true` to the missing normalizer at line 422, which also leaves the intended blank/about:blank policy unknown. Treat arbitrary URL/file access as **unproven**, not safe (impact is assumed until the helper is implemented and tested).
- [verified, security-positive] The current visible eval sites (lines 721, 742, and 769-770) use fixed literal arm/cancel/poll scripts; no public generic page-evaluation command is present. The `BrowserView::eval` trait at lines 273-290 is nevertheless a broad internal seam, so command wrappers must never forward frontend strings (future exposure risk is assumed).
- [verified, contract gap] `BrowserViewCallbacks` at lines 265-269 and `callbacks` at lines 395-412 identify only `(workspace_id, tab_id)`, not a navigation generation. `handle_page_load` lines 871-918 synthesizes the *current* tab generation and only compares URL text at lines 883-891. A stale callback for a repeated URL can therefore pass the URL check and be emitted with the newer generation; stale-event dropping is not proven (the repeated-URL race is a code-path inference, labelled assumed impact).
- [verified, contract gap] `navigate` lines 611-656 mutates generation, URL, title, history flags, and picker state before `view.navigate` returns. If native navigation fails, the registry retains the new identity; no rollback exists. This is a verified state-ordering defect.
- [verified, contract gap] `go_back`/`go_forward` lines 664-685 call the native view but never update `tab.url` or title. `handle_page_load` lines 883-891 rejects a load whose URL differs from the stale `tab.url`, so normal back/forward loads can be discarded; verified from the two code paths.
- [verified, cleanup gap] `close_tab` removes the tab at lines 688-702 before calling `view.close` at line 702. A close error loses the registry reference while the native view may remain; the error path emits no cleanup or closed event. `hide_workspace` lines 591-608 and `shutdown` lines 825-840 clear Rust picker state but never call the inspector's fixed `cancel` script. The inspector's document listeners can survive hide, and shutdown is not wired from `main.rs`; both are verified lifecycle gaps.
- [verified, cleanup limitation] Picker polling spawns an untracked thread at lines 751-784. It stops only after the next 80 ms check when the tab/picker disappears, with no join handle; a native callback that does not return can outlive close/shutdown (the latter is a bounded-risk inference, labelled assumed).
- [verified, resource gap] `capture` lines 816-823 always returns `Unsupported`. That avoids an unsafe guessed snapshot, but it does not satisfy A9 Draw's required native visible-viewport capture/restoration contract; acceptance remains blocked until an A0-approved implementation exists.
- [verified, contract gap] No viewport command/state mutation exists in this Rust file. The `BrowserViewport` input (lines 55-61, 70-72) is only handed to the missing factory; later frontend `set_browser_tab_viewport` cannot reach Rust.
- [verified, cleanup gap] `show_tab` lines 563-589 hides other views but does not disarm their active inspector picker; `hide_workspace` likewise only sets `picker = None`. A hidden tab can retain document listeners until a page action or close (the listener effect is verified in inspector.js:149-179).
- [verified, identity-positive] The registry stores one `Arc<dyn BrowserView>` per tab (`browser.rs:331-355`) and idempotent retries return before factory creation (`:439-449`), so the intended one-view-per-tab invariant is represented in Rust state. The actual same-page child-webview implementation is not proven because `TauriBrowserViewFactory` is absent (`:370,473-476`).
- [verified, z-order/input gap] `BrowserView` exposes only bounds/show/hide/navigation/eval (`browser.rs:271-290`); `show_tab` only hides peers then calls `show` (`:563-589`). There is no focus, raise/reorder, parent-window, or input-routing operation in this file. A child-view z-order/input acceptance therefore remains unproven (impact is assumed until the missing factory is reviewed on the target platform).

### `tauri-svelte-preview/src-tauri/src/browser_inspector.js`

Verdict: **keep with listed fixes**. The inspector is intentionally metadata-only, but its global API is mutable by page script and its DOM work/payload handoff are not bounded enough for the A9 security boundary.

Line review:

- [verified, contract-aligned] Lines 2-5 state and implement the no-cookie/storage/form/full-HTML/page-code bridge boundary; a token scan found no reads of those APIs. Lines 69-100 read only bounded text/accessibility attributes, and lines 121-138 return bounded selector/text/name/class/rect/URL/title metadata.
- [verified, security gap] Lines 6 and 12-13 use a predictable writable global: `if (window[KEY]) return`, and line 182 assigns `window[KEY] = Object.freeze(...)`. `Object.freeze` freezes the API object, not the `window` property descriptor; page code can replace/delete the property before Rust polls it. Rust's fixed poll at `browser.rs:22-23` would then call page-controlled `take`. Metadata poisoning/denial-of-service is the verified surface; privilege impact is assumed because the missing parser/factory prevents runtime proof.
- [verified, bound gap] `browser.rs:766-782` forwards the callback string to `parse_picker_payload` without a raw byte/character cap; inspector.js only bounds fields after DOM collection. A page-controlled replacement API can return an arbitrarily large string before parsing (verified code path; resource impact assumed).
- [verified, bound gap] `boundedText` lines 69-79 caps accumulated text but can traverse an unbounded number of empty/irrelevant text nodes before reaching 500 characters. `selectorFor` lines 38-67 scans all preceding same-tag siblings for each ancestor. These are bounded output sizes but not bounded traversal work (verified; denial-of-service severity is assumed).
- [verified, lifecycle gap] `disarm` lines 149-155 removes listeners and clears hover, but Rust's `hide_workspace`/`shutdown` do not call `cancel`; hidden/closing views can retain listeners until their page is destroyed.
- [verified, contract-aligned] Lines 112-119 refuse cross-origin frame metadata when `contentDocument` is unavailable; no cookies, storage, form values, HTML, or arbitrary page code are read. This is compatible with a metadata-only boundary but does not by itself prove A9's same/cross-origin acceptance flow.

### `tauri-svelte-preview/src-tauri/Cargo.toml`

Verdict: **keep as-is** for the requested child-webview work. The feature change at line 20 is required, not gratuitous: the locked Tauri version is 2.11.2 (`Cargo.lock:3782-3785`), Tauri's normalized manifest defines `unstable = ["tauri-runtime-wry?/unstable"]` (`/Users/blackcolours/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.2/Cargo.toml:123-130`), and `Window::add_child` is compiled only under `cfg(any(test, all(desktop, feature = "unstable")))` (`.../tauri-2.11.2/src/window/mod.rs:1126-1129`). `test` alone only makes the child API visible during test builds; a normal desktop child webview needs `unstable` (verified by source inspection, no Cargo/source edit used).

The feature does not make the incomplete/unregistered bridge safe or functional; it only removes the compile-time gate for the Tauri API the missing factory intends to call (assumed intent from imports at `browser.rs:18`).

## Browser command-name match table

The frontend names below are verified at `tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts:49-65` (interface) and `:100-210` (in-memory recorder). That file is a fake/deterministic backend; it contains no `invoke` call (verified by the file contents). The Rust entries are verified by `browser.rs` symbol locations and the absence of any `#[tauri::command]` attribute.

| Frontend command name | Rust-side symbol/status | Match |
|---|---|---|
| `create_browser_tab` (`browserBackend.ts:50,100-101`) | private `BrowserRegistry::create_tab` (`browser.rs:414-503`); no command wrapper/registration | **No** — internal name differs and is not callable |
| `set_browser_tab_bounds` (`:51,119-123`) | private `set_bounds` (`browser.rs:556-561`); no wrapper | **No** |
| `set_browser_tab_viewport` optional (`:52,125-129`) | no Rust symbol/command | **No** |
| `show_browser_tab` (`:53,131-135`) | private `show_tab` (`browser.rs:563-589`); no wrapper | **No** |
| `hide_browser_workspace` (`:54,137-142`) | private `hide_workspace` (`browser.rs:591-609`); no wrapper | **No** |
| `navigate_browser_tab` (`:55,144-152`) | private `navigate` (`browser.rs:611-656`); no wrapper | **No** |
| `reload_browser_tab` (`:56,154-156`) | private `reload` (`browser.rs:658-662`); no wrapper | **No** |
| `go_back_browser_tab` (`:57,158-165`) | private `go_back` (`browser.rs:664-674`); no wrapper | **No** |
| `go_forward_browser_tab` (`:58,167-174`) | private `go_forward` (`browser.rs:676-686`); no wrapper | **No** |
| `close_browser_tab` (`:59,176-179`) | private `close_tab` (`browser.rs:688-709`); no wrapper | **No** |
| `arm_browser_element_picker` (`:60,181-183`) | private `arm_picker` (`browser.rs:711-737`); no wrapper | **No** |
| `cancel_browser_element_picker` (`:61,185-187`) | private `cancel_picker` (`browser.rs:739-749`); no wrapper | **No** |
| `capture_browser_viewport` (`:62,189-202`) | private `capture` (`browser.rs:816-823`), always returns `Unsupported`; no wrapper | **No** |
| `open_browser_tab_devtools` optional (`:63,204-206`) | private `open_devtools` (`browser.rs:787-802`), debug-only; no wrapper | **No** |
| `open_browser_tab_external` optional (`:64,208-210`) | private `open_external` (`browser.rs:804-814`); helper `open_external_url` is absent | **No** |
| `clear_browser_workspace_data` (required by plan `:2115-2117`) | no frontend method and no Rust symbol | **No / missing on both sides** |

Every row is **verified**. There is no matching callable Rust command today.

## `main.rs` registration and shutdown check

- [verified] `tauri-svelte-preview/src-tauri/src/main.rs:24-29` has no `mod browser;` declaration.
- [verified] `main.rs:5292-5305` creates the builder and manages other registries, but not `browser::BrowserRegistry`.
- [verified] `main.rs:5316-5408` has no browser command names from the table; because `browser.rs` has no `#[tauri::command]` functions, there is nothing valid to add today.
- [verified] `main.rs:5409-5410` calls `.run(...)` directly. No `on_window_event`/`WindowEvent::Destroyed` hook calls `BrowserRegistry::shutdown`, so the cleanup method at `browser.rs:825-840` is unreachable from this application.

## Proposed exact diffs (not applied)

The following are review patches only. They are intentionally recorded here instead of applied, per the lane's no-source-edit rule.

### 1. Enforce the URL boundary before any native view receives a URL

```diff
diff --git a/tauri-svelte-preview/src-tauri/src/browser.rs b/tauri-svelte-preview/src-tauri/src/browser.rs
@@
 use tauri::{Emitter, Manager, WebviewBuilder, WebviewUrl};
@@
 const MAX_BROWSER_URL_BYTES: usize = 8 * 1024;
@@
+fn normalize_browser_url(raw: &str, allow_blank: bool) -> Result<Option<String>, BrowserCommandError> {
+    let value = raw.trim();
+    if value.is_empty() {
+        return if allow_blank {
+            Ok(None)
+        } else {
+            Err(BrowserCommandError::new(BrowserErrorCode::InvalidUrl, "Browser URL is required"))
+        };
+    }
+    if value.len() > MAX_BROWSER_URL_BYTES {
+        return Err(BrowserCommandError::new(BrowserErrorCode::InvalidUrl, "Browser URL is too long"));
+    }
+    if value.eq_ignore_ascii_case("about:blank") && allow_blank {
+        return Ok(None);
+    }
+    let parsed = tauri::Url::parse(value)
+        .map_err(|_| BrowserCommandError::new(BrowserErrorCode::InvalidUrl, "Browser URL is invalid"))?;
+    if !matches!(parsed.scheme(), "http" | "https")
+        || !parsed.username().is_empty()
+        || parsed.password().is_some()
+    {
+        return Err(BrowserCommandError::new(
+            BrowserErrorCode::InvalidUrl,
+            "Only http and https URLs without userinfo are allowed",
+        ));
+    }
+    Ok(Some(parsed.to_string()))
+}
```

This exact helper is required before accepting the current call sites at `browser.rs:422,612,884,1021`; add tests for blank, `about:blank`, userinfo, file/data/javascript/custom schemes, overlong input, and canonical http/https URLs.

### 2. Make the inspector API immutable at the window boundary

```diff
diff --git a/tauri-svelte-preview/src-tauri/src/browser_inspector.js b/tauri-svelte-preview/src-tauri/src/browser_inspector.js
--- a/tauri-svelte-preview/src-tauri/src/browser_inspector.js
+++ b/tauri-svelte-preview/src-tauri/src/browser_inspector.js
@@
-  window[KEY] = Object.freeze({
+  const api = Object.freeze({
     arm,
     cancel: () => disarm(true),
     take: () => {
       const result = state.picked;
       state.picked = null;
       return result;
     }
   });
+  Object.defineProperty(window, KEY, {
+    configurable: false,
+    enumerable: false,
+    writable: false,
+    value: api
+  });
 })();
```

This closes the replacement/deletion surface identified at `browser_inspector.js:6,12-13,182-190`; Rust must still treat all returned metadata as hostile.

### 3. Cap the raw inspector callback before JSON parsing

```diff
diff --git a/tauri-svelte-preview/src-tauri/src/browser.rs b/tauri-svelte-preview/src-tauri/src/browser.rs
@@
 const MAX_CLASS_NAME_BYTES: usize = 128;
+const MAX_INSPECTOR_PAYLOAD_BYTES: usize = 64 * 1024;
@@
     fn handle_picker_payload(&self, key: &BrowserTabKey, generation: u64, epoch: u64, raw: &str) {
+        if raw.len() > MAX_INSPECTOR_PAYLOAD_BYTES {
+            self.emit_picker_unavailable(key, generation, epoch, "inspector-payload-too-large");
+            return;
+        }
         let payload = match parse_picker_payload(raw) {
```

The cap belongs before `parse_picker_payload` at `browser.rs:976-983`, not only on individual fields.

### 4. Register callable commands and wire window teardown after the wrappers exist

```diff
diff --git a/tauri-svelte-preview/src-tauri/src/main.rs b/tauri-svelte-preview/src-tauri/src/main.rs
@@
 mod agent_conversation;
+mod browser;
 mod git_diff_models;
@@
         .manage(terminal::TerminalRegistry::default())
+        .manage(browser::BrowserRegistry::default())
         .plugin(tauri_plugin_dialog::init())
@@
             close_terminal_session
+            ,browser::create_browser_tab
+            ,browser::set_browser_tab_bounds
+            ,browser::show_browser_tab
+            ,browser::hide_browser_workspace
+            ,browser::navigate_browser_tab
+            ,browser::reload_browser_tab
+            ,browser::go_back_browser_tab
+            ,browser::go_forward_browser_tab
+            ,browser::close_browser_tab
+            ,browser::arm_browser_element_picker
+            ,browser::cancel_browser_element_picker
+            ,browser::capture_browser_viewport
+            ,browser::open_browser_tab_devtools
+            ,browser::open_browser_tab_external
         ])
+        .on_window_event(|window, event| {
+            if matches!(event, tauri::WindowEvent::Destroyed) {
+                window.state::<browser::BrowserRegistry>().shutdown();
+            }
+        })
```

This is an exact registration shape, but it is deliberately not claimed sufficient: `browser.rs` must first add the corresponding `#[tauri::command] pub fn` wrappers (and implement the missing helpers/factory). `clear_browser_workspace_data` is omitted from this diff because neither side currently defines its contract or storage semantics; it must be added only after that contract is written. The comma style should be normalized when the real wrapper patch lands.

### 5. Preserve state and disarm the inspector around native failures/close

```diff
diff --git a/tauri-svelte-preview/src-tauri/src/browser.rs b/tauri-svelte-preview/src-tauri/src/browser.rs
@@
         for tab in workspace.tabs.values_mut() {
+            let _ = tab.view.eval("window.__mcbBrowserInspector?.cancel?.();");
             tab.picker = None;
             tab.view.hide()?;
         }
@@
-        let tab = workspace.tabs.remove(&target.tab_id).ok_or_else(|| {
+        let view = workspace.tabs.get(&target.tab_id).ok_or_else(|| {
             BrowserCommandError::new(
                 BrowserErrorCode::UnknownTab,
                 "That browser tab is no longer available",
             )
-        })?;
-        tab.view.close()?;
+        })?.view.clone();
+        let _ = view.eval("window.__mcbBrowserInspector?.cancel?.();");
+        view.close()?;
+        let tab = workspace.tabs.remove(&target.tab_id).expect("validated browser tab disappeared while locked");
         tab.sink.closed(BrowserTabClosedEvent {
```

The same cancel call belongs in `shutdown` before each `close` (`browser.rs:834-837`). The close-first ordering keeps a failed native close recoverable instead of dropping the registry reference (`browser.rs:688-703`).

### 6. Make navigation generation and native failure atomic

```diff
diff --git a/tauri-svelte-preview/src-tauri/src/browser.rs b/tauri-svelte-preview/src-tauri/src/browser.rs
@@
-        tab.generation = target.generation;
-        workspace.last_generation = workspace.last_generation.max(target.generation);
-        tab.url = url.clone();
-        tab.title = title_from_url(Some(&url));
-        tab.can_go_back = true;
-        tab.can_go_forward = false;
-        tab.picker = None;
-        tab.view.navigate(&url)
+        let previous = (tab.generation, tab.url.clone(), tab.title.clone(), tab.can_go_back, tab.can_go_forward, tab.picker);
+        tab.generation = target.generation;
+        workspace.last_generation = workspace.last_generation.max(target.generation);
+        tab.url = url.clone();
+        tab.title = title_from_url(Some(&url));
+        tab.can_go_back = true;
+        tab.can_go_forward = false;
+        tab.picker = None;
+        if let Err(error) = tab.view.navigate(&url) {
+            (tab.generation, tab.url, tab.title, tab.can_go_back, tab.can_go_forward, tab.picker) = previous;
+            return Err(error);
+        }
+        Ok(())
```

The native callback API must also carry the generation captured by the navigation request; compare that value to `tab.generation` before emitting at `browser.rs:871-918`. URL equality alone is insufficient for same-URL stale callbacks.

## Working-tree and security summary

- [verified] The requested partial-work state is present: `tauri-svelte-preview/src-tauri/src/browser.rs` and `browser_inspector.js` are untracked, while `tauri-svelte-preview/src-tauri/Cargo.toml` is modified; the receipt itself is the only additional file created by this lane. No source file was edited by this review, and no commit was created.
- [verified] Current runtime exposure is effectively absent because `browser.rs` is not declared/registered (`main.rs:24-29,5292-5408`). This means no demonstrated live arbitrary-file navigation or script injection path exists in the current binary, but it also means A9 is not wired.
- [verified] If the missing factory/commands are later wired, URL/file access remains unsafe until the helper at the proposed diff is implemented and tested (`browser.rs:422,612,884,1021`).
- [verified] The fixed arm/cancel/poll strings are not user-interpolated (`browser.rs:721,742,769-770`), and the inspector does not read cookies/storage/form values/HTML (`browser_inspector.js:2-5,69-139`).
- [verified] The mutable `window.__mcbBrowserInspector` property (`browser_inspector.js:6,12-13,182-190`), unbounded raw callback (`browser.rs:766-782,976-983`), absent parser/factory, missing generation token, and uncalled shutdown are the security/identity/cleanup blockers to resolve before native acceptance.

## Final verdict

**A9 Rust review: reject for acceptance in the current tree; preserve the files only as an implementation draft with the listed fixes.** `Cargo.toml`'s `unstable` feature should stay. `browser.rs` needs its missing helpers/factory/command wrappers, strict URL handling, generation-safe callbacks, atomic navigation, profile binding, viewport/capture decision, and close/shutdown wiring. `browser_inspector.js` needs immutable global installation, raw-result bounding, and explicit lifecycle cancellation. The requested browser tests currently execute zero tests; the full suite has one unrelated observed bind failure (`214 passed, 1 failed, 1 ignored`).
