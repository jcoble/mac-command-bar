# SideX source study: what mac-command-bar should borrow

**Verified:** SideX is not a small custom Tauri workbench built from plain Monaco. It replaces Electron with Tauri while directly porting the Code OSS workbench, editor, SCM views, terminal UI, and service container; it does not use any `@codingame/monaco-vscode-*` package. (`sidex/README.md:28-30`, `sidex/README.md:94-106`, `sidex/ARCHITECTURE.md:18-24`, `sidex/package.json:22-44`)

**Verified:** The widely repeated “16.4 MB” is application size on disk, not idle memory. SideX's own README says real benchmarks have not been published and only states a target below 200 MB idle on macOS. (`sidex/docs/assets/compare.jpg`, embedded at `sidex/README.md:38-42`)

**Assumed recommendation:** Do not port SideX's workbench or SCM UI into mac-command-bar. The useful material is its bounded and explicit lifecycle machinery—then tighten it further for mac-command-bar's flat-growth requirement: validate the in-progress removal of the views/SCM service overrides, keep the custom Svelte panels, cap live xterm renderers, lower renderer scrollback, and retain the existing Monaco model LRU. Basis: SideX eagerly loads the workbench and retains hidden terminals, while mac-command-bar already owns its panel chrome and has bounded editor models. (`sidex/src/main.ts:28-49`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1719-1735`, `sidex/src/vs/workbench/browser/parts/editor/editor.ts:104-111`)

## Scope and evidence

**Verified:** I cloned `https://github.com/Sidenai/sidex.git` into this scratch folder and inspected commit `05d0710a2735d2a5d6d493f299381d5b6dd06a61`; the clone was clean before this report was written. This is a source audit, not a runtime benchmark: I did not build or launch SideX, and the repository itself says real benchmarks are still forthcoming. (`sidex/README.md:42`, `sidex/README.md:64-88`)

“Verified” below means directly established from the checked-out source, the checked-in comparison image, or an explicitly linked primary source. “Assumed” means an engineering inference or expected effect that must be tested in mac-command-bar.

## 1. Startup discipline

### What SideX actually is

**Verified:** There are no `@codingame` or `monaco-vscode` dependencies or imports in the SideX clone. Its declared frontend dependencies are Monaco itself, xterm and add-ons, TextMate/Oniguruma, Tauri, and a few VS Code support packages. A repository-wide `rg -n '@codingame|monaco-vscode' .` returned no matches. (`sidex/package.json:22-44`)

**Verified:** That does not mean SideX is “plain Monaco plus its own chrome.” The project describes the frontend as the same TypeScript workbench and a direct port of VS Code; the architecture document says the workbench, Monaco, and extension API are preserved, and says the workbench layout and eight visual parts are reused with modifications. (`sidex/README.md:28-30`, `sidex/README.md:94-106`, `sidex/ARCHITECTURE.md:18-24`, `sidex/ARCHITECTURE.md:108-123`)

**Verified:** The activity bar, side bar, panels, tabs, editor groups, title bar, SCM view, and terminal view are Code OSS workbench parts under `src/vs/workbench`, not Svelte components. The declared layering identifies `workbench/browser/parts` as the visual parts and `browser/layout.ts` as the layout engine. (`sidex/ARCHITECTURE.md:27-45`, `sidex/README.md:110-125`)

### Eager at application/workbench launch

**Verified:** Before creating the workbench, `boot()` loads locale data and then concurrently imports four large workbench barrels: common, web main, web dialog, and web services. Those imports execute module registration code for editor contributions, SCM, search, terminal, tasks, extensions, webviews, debug, and many other features. (`sidex/src/main.ts:28-49`, `sidex/src/vs/workbench/workbench.common.main.ts:6-26`, `sidex/src/vs/workbench/workbench.common.main.ts:210-229`, `sidex/src/vs/workbench/workbench.web.main.ts:30-70`, `sidex/src/vs/workbench/workbench.web.main.ts:112-159`)

**Verified:** SideX then constructs nine bridge/service objects in JavaScript—editor, syntax, Git, search, settings, theme, extensions, keymap, and filesystem—before calling the workbench factory. (`sidex/src/main.ts:51-77`, `sidex/src/main.ts:80-85`, `sidex/src/main.ts:208-213`)

**Verified:** The Rust application eagerly installs managed state for updater, TextMate, marketplace, two terminal/process implementations, debug/DAP, LSP, tasks, watchers, a trigram-capable index store, logging, extension supervision/diagnostics, settings, remote management, and a WASM extension runtime. It also opens `sidex_storage.db` and `sidex_state.db`, loads user settings if present, and initializes updater, profiles, and secrets during Tauri setup. (`sidex/src-tauri/src/lib.rs:370-395`, `sidex/src-tauri/src/lib.rs:443-486`)

**Assumed:** Many of those Rust stores begin mostly empty, so registering them is not equivalent to allocating their eventual workload. The startup shape is nevertheless broad rather than minimum-first, especially because `WasmExtensionRuntime::new()` and two database opens are executed eagerly. (`sidex/src-tauri/src/lib.rs:376-395`, `sidex/src-tauri/src/lib.rs:443-473`)

### Lazy or demand-driven

**Verified:** Importing a workbench service module does not necessarily instantiate its implementation. In the web service barrel, most shown singletons use `InstantiationType.Delayed`; the title service is explicitly eager. (`sidex/src/vs/workbench/workbench.web.main.ts:72-108`)

**Verified:** Git's Tauri API module is dynamically imported on first use and cached. The real Git contribution itself is registered at `WorkbenchPhase.BlockRestore`, checks only the first workspace folder, and creates a provider only if that folder is a Git repository. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:94-115`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1078-1137`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1489`)

**Verified:** Terminal processes and xterm instances are not created merely because the app launched. Each `TerminalInstance` creates its xterm when that terminal instance is constructed, and WebGL is loaded only when the xterm is attached to a DOM element. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:709-716`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1021-1059`, `sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:581-629`)

**Verified:** The Rust search index object is created at launch, but its maps are empty until `index_build` is called. Building explicitly clears the previous index, walks the requested root, and indexes files in parallel; no call to `index_build`, `index_search`, or `index_update` exists in the TypeScript source, so this index appears wired as a backend capability rather than an active frontend path in this commit. (`sidex/src-tauri/src/lib.rs:385-387`, `sidex/src-tauri/src/commands/index.rs:114-149`, `sidex/src-tauri/src/commands/index.rs:671-713`)

### Meaning for mac-command-bar

**Verified:** The mac-command-bar worktree changed concurrently during this read-only research lane. Its current uncommitted experiment now selects `EditorService` and retains only file and keybinding overrides; the Git UI remains custom Svelte. The tracked baseline visible in `git diff` had selected `ViewsService` and added the SCM override. I did not make that worktree change. (`/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:361-396`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/git/SourceControlPanes.svelte:1`)

**Assumed:** SideX supports the current Codingame views/SCM removal experiment only as proof that Tauri can bridge Monaco/Code OSS to Rust without those packages. It is not evidence that SideX's direct Code OSS workbench has a lower JavaScript baseline than mac-command-bar's custom Svelte shell. The smallest experiment is the change now present in the shared worktree: A/B `EditorService` without the SCM override against the tracked baseline in the same native idle scenario. (`sidex/src/main.ts:28-49`, `sidex/ARCHITECTURE.md:116-123`)

## 2. SCM and Git UI

### Component ownership

**Verified:** Source Control is not SideX's own component tree. SideX imports the unchanged Code OSS SCM contribution and a SideX Git contribution; its high-level `SideXSCMProvider` even says it is a simplified bridge and that a full `ISCMProvider` adapter was still a TODO. The separate `git.contribution.ts` is that practical adapter and implements the Code OSS SCM interfaces. (`sidex/src/vs/workbench/workbench.common.main.ts:220-222`, `sidex/src/vs/platform/sidex/browser/sidexSCMProvider.ts:1-10`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:18-67`)

**Verified:** The Rust side shells out to the installed `git` CLI. The Tauri commands validate paths and delegate status, diff, log, stage, and other operations to `sidex-git`; the crate states that all operations use `std::process::Command`. (`sidex/src-tauri/src/commands/git.rs:75-145`, `sidex/crates/sidex-git/src/lib.rs:1-4`, `sidex/crates/sidex-git/src/lib.rs:20-31`)

### Diff behavior

**Verified:** Changed resources expose an original URI using a read-only `git-original:` filesystem scheme and the working file as the modified URI. Reading an original is demand-driven: it invokes `git show <ref>:<path>` and falls back to `git_show`. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:117-178`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:187-220`)

**Verified:** Opening a change runs the Code OSS `vscode.diff` command with the original and modified URIs, so the visual diff is the workbench's Monaco diff editor, not a custom SideX diff renderer. “Open All Changes” loops over every current change and opens a file or diff editor for each one; there is no cap in that command. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1182-1235`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1294-1319`)

### State placement and refresh behavior

**Verified:** JavaScript retains the current branch, observable counts/actions/status-bar commands, three resource arrays (merge, staged, working changes), lazily built resource trees, history refs, file decorations, and a Monaco model for the commit input. Each successful refresh replaces the resource arrays with newly mapped objects. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:260-305`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:681-778`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:797-851`)

**Verified:** Rust/Git supplies payloads on demand. A refresh fetches status, then a one-entry log for HEAD, then upstream ahead/behind data; history pages default to 50 items, history-file changes are fetched when asked for, and original file bytes are fetched when a diff model reads them. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:797-886`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:452-555`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:557-608`)

**Verified:** After initial refresh, SideX runs `provider.refresh()` every ten seconds for the contribution's entire lifetime. The timer is not conditional on the SCM view being visible; it is cleared only when the contribution is disposed. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1167-1179`)

**Assumed recommendation:** Borrow the narrow contracts—paged history, original contents fetched only for an opened diff, and small status summaries—but not the Code OSS SCM view or ten-second poller. mac-command-bar already has the correct product boundary: its Svelte tree over Rust Git. Prefer invalidation from the existing scanner/watcher and fetch the visible pane's data on demand. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:117-178`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:452-461`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1169-1179`)

## 3. Terminal lifecycle

### Number of xterm instances and buffers

**Verified:** SideX has one xterm object per live Code OSS `TerminalInstance`. The group service simply flattens all instances across all terminal groups, and no maximum terminal count or renderer eviction is present in the inspected lifecycle. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:709-716`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalGroupService.ts:26-33`)

**Verified:** xterm scrollback defaults to 1,000 lines. The configuration text explicitly warns that memory is preallocated from this value, and the xterm constructor receives that configured value. (`sidex/src/vs/workbench/contrib/terminal/common/terminalConfiguration.ts:544-551`, `sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:292-304`)

**Verified:** The active SideX terminal adapter uses `terminal_spawn`/`terminal-data`, not the separate `term_*` process API. Rust stores the live PTY handles and streams 8 KiB chunks directly as Tauri events; it removes the handle on process exit or explicit kill. Therefore the active integrated path has xterm's bounded scrollback but no independent replayable backend scrollback ring. (`sidex/src/vs/workbench/contrib/terminal/browser/tauriTerminalBackend.ts:136-178`, `sidex/src-tauri/src/commands/terminal.rs:258-329`, `sidex/src-tauri/src/commands/terminal.rs:383-397`)

**Verified:** SideX also eagerly manages an alternative `ProcessStore` whose implementation has a 10,000-entry ring and a bounded 1,000-message channel, but no TypeScript call to `term_spawn` or `term_read` exists in this commit. It should not be counted as the integrated terminal's live memory. (`sidex/src-tauri/src/lib.rs:379-381`, `sidex/src-tauri/src/commands/process.rs:26-36`, `sidex/src-tauri/src/commands/process.rs:208-255`, `sidex/src-tauri/src/commands/process.rs:756-798`)

### Hidden, switched, and disposed terminals

**Verified:** Switching terminal groups only calls `setVisible(false)` on inactive groups. `TerminalInstance.setVisible` toggles an `active` CSS class and performs resize/open work when becoming visible; hiding does not detach or dispose xterm. Hidden terminal instances therefore retain their xterm object and buffer and continue to receive process output. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalGroupService.ts:512-521`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1719-1735`, `sidex/src/vs/workbench/contrib/terminal/browser/tauriTerminalBackend.ts:139-147`)

**Verified:** Even `detachFromElement()` only removes the wrapper element and clears its container pointer. Full terminal disposal is a separate path that disposes xterm, then the process manager, then the instance's disposables. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1316-1340`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1582-1641`)

**Verified:** Persistent terminal sessions are disabled in SideX's default configuration, and the Tauri child-process adapter declares `shouldPersist = false`; shutdown kills the Rust PTY and unregisters both event listeners. (`sidex/src/main.ts:164-180`, `sidex/src/vs/workbench/contrib/terminal/browser/tauriTerminalBackend.ts:69-89`, `sidex/src/vs/workbench/contrib/terminal/browser/tauriTerminalBackend.ts:194-206`)

### WebGL

**Verified:** WebGL is demand-loaded at first DOM attachment, normally in `auto` mode. Before replacing a WebGL addon SideX disposes the previous addon specifically to avoid leaking contexts, and it also disposes on context loss or terminal disposal. (`sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:581-592`, `sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:694-699`, `sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:966-1007`, `sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:1084-1099`)

**Verified:** SideX does not release the WebGL addon merely because a terminal becomes hidden. Its visibility path never calls `_disposeOfWebglRenderer`. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1719-1735`, `sidex/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts:1084-1099`)

**Verified:** mac-command-bar is currently heavier per session: it keeps one mounted hidden xterm per owned session, writes output even while hidden, configures 20,000 lines, and its own source estimates roughly 20 MB per 120-column view. It already does one thing better than SideX by releasing a hidden view's WebGL addon. Its Rust backend separately retains up to 16 MiB of authoritative scrollback per session. (`/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:3-14`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/liveConversationTerminals.ts:128-168`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/liveConversationTerminals.ts:245-282`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/xtermFactory.ts:138-166`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/xtermFactory.ts:179-210`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri/src/terminal.rs:13-23`)

**Assumed recommendation:** Treat xterm as an evictable visible projection, not the session. Keep a small hot set—ideally active plus one recent view—dispose other xterm instances completely, keep PTY/transcript state in Rust, and recreate/replay the bounded tail on selection. SideX supplies a clean full-dispose boundary but does not implement this hot-set policy itself; mac-command-bar's backend scrollback makes the policy feasible. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1582-1641`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalConfiguration.ts:544-551`)

## 4. Editor and tab lifecycle

### Editor controls versus models

**Verified:** SideX follows Code OSS's editor-pane reuse. Each editor group caches at most one pane per editor descriptor/type; changing tabs clears the old input and sets the new input on that pane rather than constructing one Monaco editor for every tab. Hidden panes are removed from the DOM but remain cached for reuse. (`sidex/src/vs/workbench/browser/parts/editor/editorPanes.ts:114-120`, `sidex/src/vs/workbench/browser/parts/editor/editorPanes.ts:448-459`, `sidex/src/vs/workbench/browser/parts/editor/editorPanes.ts:480-539`, `sidex/src/vs/workbench/browser/parts/editor/editorPanes.ts:544-569`)

**Verified:** Text models are reference-counted. Resolving a resource obtains or creates a shared model; when the last reference is destroyed, the resolver waits for file/untitled disposal conditions and calls `model.dispose()`. Dirty file models are deliberately retained until they become clean so edits are not lost. (`sidex/src/vs/workbench/services/textmodelResolver/common/textModelResolverService.ts:52-111`, `sidex/src/vs/workbench/services/textmodelResolver/common/textModelResolverService.ts:122-158`, `sidex/src/vs/workbench/services/textfile/common/textFileEditorModelManager.ts:631-659`)

### View-state persistence and caps

**Verified:** Before an input is cleared, closed, or shut down, text editors compute state with Monaco's `saveViewState()`. The workbench keeps those states in an LRU memento capped at 100 resources, scoped to the workspace and machine, and restores state by default. (`sidex/src/vs/workbench/browser/parts/editor/textCodeEditor.ts:72-92`, `sidex/src/vs/workbench/browser/parts/editor/editorWithViewState.ts:52-89`, `sidex/src/vs/workbench/browser/parts/editor/editorWithViewState.ts:91-149`, `sidex/src/vs/workbench/browser/parts/editor/editorPane.ts:200-220`, `sidex/src/vs/workbench/browser/parts/editor/editorPane.ts:431-455`)

**Verified:** A separate open-editor LRU limiter exists, but it is disabled by default. Its nominal value is ten and it can close least-recent editors when enabled; both Code OSS defaults and SideX's Rust defaults leave `workbench.editor.limit.enabled` false. Open tabs can therefore retain model references without a default count ceiling. (`sidex/src/vs/workbench/browser/parts/editor/editor.ts:95-111`, `sidex/src/vs/workbench/browser/parts/editor/editorsObserver.ts:337-377`, `sidex/crates/sidex-settings/src/defaults.rs:300-305`)

### Workspace and window changes

**Verified:** Opening a different workspace folder rewrites the URL and reloads the webview. That destroys the old JavaScript heap rather than migrating live Monaco/xterm objects. Workbench layout and editor-part state are persisted separately through VS Code storage; editor view-state mementos are workspace-scoped, so persisted state can return after reload without retaining the old models in memory. (`sidex/src/main.ts:22-25`, `sidex/src/main.ts:105-115`, `sidex/crates/sidex-db/src/window_state.rs:1-5`, `sidex/src/vs/workbench/browser/parts/editor/editorPane.ts:200-220`)

**Assumed:** A separate SideX `WebviewWindow` should be treated as a separate frontend heap for accounting. The clone documents the `BrowserWindow` to `WebviewWindow` mapping but does not provide a cross-window model-sharing mechanism in the inspected editor lifecycle. (`sidex/README.md:96-104`, `sidex/ARCHITECTURE.md:10-24`)

**Verified:** mac-command-bar already has a stronger default model-growth policy than SideX: clean closed-tab models are disposable, tab models use a 24-entry LRU that protects dirty models, external peek/jump models have a separate 24-entry LRU, and component teardown disposes owned models. (`/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/editor/editorStoreOps.ts:126-155`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1363-1420`, `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2978-3023`)

**Assumed recommendation:** Keep mac-command-bar's existing model LRUs. Borrow only SideX's bounded per-resource `saveViewState()` memento if tab recreation currently loses cursor/fold/scroll state. Do not copy SideX's disabled open-tab limit as a memory policy. (`sidex/src/vs/workbench/browser/parts/editor/editorWithViewState.ts:52-89`, `sidex/src/vs/workbench/browser/parts/editor/editor.ts:104-111`)

## 5. Memory methodology and the 16.4 MB claim

### What the screenshot measures

**Verified:** `docs/assets/compare.jpg` is a macOS Finder “Get Info” comparison. It literally shows SideX `Size: 16,355,217 bytes (16.4 MB on disk)` and VS Code `Size: 797,790,918 bytes (812 MB on disk)`. It is embedded between README paragraphs about RAM, but it contains no process-memory measurement. (`sidex/docs/assets/compare.jpg`, embedding and surrounding claim at `sidex/README.md:34-42`)

**Verified:** The current README explicitly calls under 200 MB idle a target and says real benchmarks will be published later. Therefore the clone contains no evidence for an honest SideX idle-RAM number, and 16.4 MB must not be used as one. (`sidex/README.md:36-42`)

### WKWebView accounting

**Verified:** SideX's own architecture places its TypeScript frontend in a Tauri webview. Modern WKWebView is multi-process: WebKit documents an application/UI process plus WebContent, Networking, and Storage processes. An app-process-only number therefore omits memory required to run the frontend. (`sidex/ARCHITECTURE.md:18-24`; [WebKit process architecture](https://docs.webkit.org/Getting%20Started/Introduction.html); [WebKit debugging process table](https://webkit.org/debugging-webkit/))

**Verified:** The README's statement that WKWebView is “shared with Safari” and costs almost nothing extra is a project assertion, not substantiated by a benchmark in this repository. WebKit documents sharing a networking session among WebContent processes in one browsing session; that is not a license to count an app's WebContent heap, JavaScript heap, DOM, canvases, or GPU resources as zero. (`sidex/README.md:34-42`; [WebKit2 multi-process architecture](https://docs.webkit.org/Deep%20Dive/Architecture/WebKit2.html))

**Verified:** Apple defines physical footprint as the relevant charged memory metric and provides `proc_pid_rusage`/`ri_phys_footprint`; its tooling guidance uses `vmmap` to separate dirty, compressed/swapped, and clean mapped regions. ([Apple WWDC22 memory methodology](https://developer.apple.com/videos/play/wwdc2022/10106/?time=1012); [Apple WWDC21 memory diagnosis](https://developer.apple.com/videos/play/wwdc2021/10180/?time=1572))

### The honest comparable idle number

**Verified:** No honest comparable number can be calculated from this source tree. The answer today is “unknown, and definitely not the 16.4 MB shown.” (`sidex/README.md:38-42`, `sidex/docs/assets/compare.jpg`)

**Assumed measurement protocol:** On the same Mac and OS, compare release builds with the same small workspace, same window count, no extensions/LSP/terminals, and the same cold-versus-warm cache condition. After a fixed settle period, record several samples and report both median and peak physical footprint for: (1) the app/UI process; (2) its WebContent, Networking, and Storage auxiliaries; and (3) spawned extension hosts, language servers, and PTY children. For shared auxiliary processes, report both the full observed process footprint and the incremental delta from a matched baseline rather than silently assigning it to neither app. Basis: SideX uses WKWebView and may spawn sidecars/PTYs, while WebKit and Apple require process-aware footprint accounting. (`sidex/ARCHITECTURE.md:10-24`, `sidex/README.md:96-106`; [WebKit process architecture](https://docs.webkit.org/Getting%20Started/Introduction.html); [Apple physical-footprint guidance](https://developer.apple.com/videos/play/wwdc2022/10106/?time=1012))

### Memory-specific code in SideX

**Verified:** The release profile uses optimization level 3, thin LTO, one codegen unit, symbol stripping, and abort-on-panic. Those settings explain small/optimized binaries more directly than low live heap; they are not cache or lifecycle controls. (`sidex/Cargo.toml:123-128`, `sidex/src-tauri/Cargo.toml:144-149`)

**Verified:** A repository-wide manifest/source search found no jemalloc, mimalloc, or other custom global allocator declaration, and the inspected startup contains no heap priming or memory benchmark harness. The concrete memory controls found are local bounds and disposal paths—not allocator tuning. (`sidex/Cargo.toml:35-103`, `sidex/src-tauri/Cargo.toml:22-124`, `sidex/src/main.ts:28-77`, `sidex/src-tauri/src/lib.rs:370-395`)

**Assumed:** SideX may still use less total idle memory than Electron VS Code because it does not bundle/run Electron's Chromium process set, but neither the screenshot nor this source inspection quantifies that difference. (`sidex/README.md:28-42`, `sidex/README.md:94-106`)

## 6. Other transferable patterns

### Search indexing

**Verified:** SideX's index excludes common generated directories by default, skips files over 1 MiB, checks the first 8 KiB for NUL bytes, rejects invalid UTF-8, caps search results at 1,000 by default, follows no symlinks, and limits traversal depth to 50. (`sidex/src-tauri/src/commands/index.rs:20-47`, `sidex/src-tauri/src/commands/index.rs:189-207`, `sidex/src-tauri/src/commands/index.rs:610-668`)

**Verified:** The index itself is not memory-bounded. It retains word-to-occurrence vectors, a word set per file, path maps, and—because the launch store enables it—a trigram-to-word index. It tracks an estimated byte count and supports explicit clear, but there is no hard byte ceiling or eviction policy; removing a file removes postings but does not decrement the estimate. (`sidex/src-tauri/src/commands/index.rs:107-149`, `sidex/src-tauri/src/commands/index.rs:189-248`, `sidex/src-tauri/src/commands/index.rs:251-267`, `sidex/src-tauri/src/commands/index.rs:477-497`, `sidex/src-tauri/src/lib.rs:385-387`)

**Assumed recommendation:** Borrow the file-size, result-count, binary, symlink, depth, and generated-directory bounds. Do not copy the in-memory trigram index without a hard workspace byte budget and a proven query need; for flat growth, a Rust streaming search or disk-backed index is safer. (`sidex/src-tauri/src/commands/index.rs:20-47`, `sidex/src-tauri/src/commands/index.rs:114-149`)

### File watching

**Verified:** The Rust watcher defaults to 100 ms debounce, filters ignores/extensions before enqueueing, drains events, and keeps only the latest event per path before sending one batch. A stopped session aborts the debounce task and drops the native watcher. (`sidex/src-tauri/src/commands/watch.rs:1-26`, `sidex/src-tauri/src/commands/watch.rs:247-325`, `sidex/src-tauri/src/commands/watch.rs:455-505`, `sidex/src-tauri/src/commands/watch.rs:513-526`)

**Verified:** The frontend uses one shared Tauri event listener with reference counting and unregisters it after the final watch is disposed. Each filesystem-provider watch starts and stops its matching Rust watch ID. (`sidex/src/vs/platform/files/browser/tauriFileSystemProvider.ts:59-70`, `sidex/src/vs/platform/files/browser/tauriFileSystemProvider.ts:101-121`, `sidex/src/vs/platform/files/browser/tauriFileSystemProvider.ts:247-284`)

**Verified:** The debounce input channel and pending-event vector are unbounded. A continuous event storm can grow them until a quiet debounce interval is reached, so the exact queue design is not suitable for a strict flat-growth target. (`sidex/src-tauri/src/commands/watch.rs:247-286`)

**Assumed recommendation:** Borrow reference-counted listener teardown and per-path coalescing, but implement coalescing directly into a bounded map/queue with a maximum batch size and overflow-rescan signal. (`sidex/src-tauri/src/commands/watch.rs:282-325`, `sidex/src/vs/platform/files/browser/tauriFileSystemProvider.ts:247-284`)

### SQLite

**Verified:** The newer SideX database wrapper enables WAL and foreign keys, runs migrations on open, and uses cached prepared statements for scoped state. Workspace state is keyed by workspace and key rather than loaded wholesale. (`sidex/crates/sidex-db/src/db.rs:17-36`, `sidex/crates/sidex-db/src/state.rs:17-54`, `sidex/crates/sidex-db/src/state.rs:89-123`)

**Verified:** Both storage implementations enforce a 256-byte key cap and a 1 MiB value cap. The legacy Tauri storage uses one mutex-protected SQLite connection and query-by-prefix; the newer store uses `prepare_cached`. (`sidex/src-tauri/src/commands/storage.rs:5-9`, `sidex/src-tauri/src/commands/storage.rs:31-50`, `sidex/src-tauri/src/commands/storage.rs:53-104`, `sidex/crates/sidex-db/src/storage_kv.rs:14-65`)

**Verified:** SideX opens two state databases at startup and retains both a legacy storage implementation and the newer `sidex-db` path. That duplication is migration/compatibility structure, not a memory pattern mac-command-bar should copy. (`sidex/src-tauri/src/lib.rs:443-473`, `sidex/crates/sidex-db/src/storage_kv.rs:1-7`)

**Assumed recommendation:** Borrow payload-size guards, workspace-scoped keys, WAL, and prepared statements where mac-command-bar lacks them; keep one database abstraction rather than SideX's parallel legacy/new stores. (`sidex/src-tauri/src/commands/storage.rs:5-9`, `sidex/crates/sidex-db/src/db.rs:17-36`, `sidex/crates/sidex-db/src/state.rs:89-123`)

## Ranked shortlist for mac-command-bar

**Assumed:** All expected wins, effort sizes, and risks in this shortlist are estimates to validate in mac-command-bar; the cited SideX receipts establish the source pattern, not the size of mac-command-bar's measured change.

| Rank | What to borrow or do | Expected win | Rough effort | Risk |
|---:|---|---|---|---|
| 1 | **Finish the native A/B of the uncommitted `EditorService`/no-SCM-override experiment; retain the custom Svelte SCM/files/history panels if it passes.** **Assumed:** This is the cleanest test of today's fixed idle-baseline suspect. SideX proves the packages are not intrinsic to Tauri/Monaco/Rust Git, but not that its much larger direct workbench is cheap. (`sidex/package.json:22-44`, `sidex/ARCHITECTURE.md:116-123`) | **Assumed:** potentially large fixed idle reduction; measurement required | Small–medium | Medium: extension API probes or SCM registrations may depend on services indirectly |
| 2 | **Cap live xterm views to a small hot set and fully dispose cold renderers while PTYs continue in Rust.** **Assumed:** SideX clearly separates hide from full dispose; mac-command-bar's replayable Rust ring supplies the missing rehydration layer. (`sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1316-1340`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1582-1641`) | **Assumed:** highest flat-growth win after real use; removes a renderer, DOM tree, buffer, addons, and possible GPU resources per cold session | Medium | Medium: exact ANSI screen restoration, selection, and resize behavior need native proof |
| 3 | **Lower xterm frontend scrollback from 20,000 toward a measured 1,000–5,000 while keeping the Rust transcript/ring authoritative.** **Verified basis:** SideX defaults to 1,000 and warns allocation grows with the setting. (`sidex/src/vs/workbench/contrib/terminal/common/terminalConfiguration.ts:544-551`) | **Assumed:** large per-live-view reduction; mac-command-bar's own source estimates about 20 MB at its current setting | Small | Low–medium: users may expect more immediate local scrollback; replay/search must cover it |
| 4 | **Keep and audit the existing Monaco model LRUs; add a bounded `saveViewState()` cache only if recreation loses UX state.** **Verified basis:** SideX's 100-entry view-state LRU is good, but its open-tab limiter is disabled; mac-command-bar already caps tab and external models at 24. (`sidex/src/vs/workbench/browser/parts/editor/editorWithViewState.ts:52-89`, `sidex/src/vs/workbench/browser/parts/editor/editor.ts:104-111`) | **Assumed:** prevents model growth without paying for Code OSS editor services; small additional memory win because the main cap already exists | Small | Low: protect dirty models and test diff/peek references |
| 5 | **Keep SCM payloads demand-driven and replace unconditional polling with watcher invalidation/visibility-aware refresh.** **Verified basis:** SideX's 50-item history and on-read originals are useful; its ten-second lifetime poll is not. (`sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:452-461`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:557-608`, `sidex/src/vs/workbench/contrib/scm/browser/git.contribution.ts:1169-1179`) | **Assumed:** modest idle CPU/allocations; meaningful heap restraint for large histories/diffs | Small–medium | Low: invalidation must cover external Git changes |
| 6 | **Use watcher debounce, per-path coalescing, and reference-counted teardown, but put a hard bound on the pending queue.** **Verified basis:** SideX has clean teardown/coalescing but an unbounded channel/vector. (`sidex/src-tauri/src/commands/watch.rs:247-325`, `sidex/src-tauri/src/commands/watch.rs:513-526`, `sidex/src/vs/platform/files/browser/tauriFileSystemProvider.ts:247-284`) | **Assumed:** low idle win; high protection against file-event storms and workspace churn | Medium | Low: overflow must trigger a safe rescan rather than silently lose truth |
| 7 | **Adopt hard search/storage bounds, not SideX's whole in-memory trigram index.** **Verified basis:** 1 MiB files, 1,000 results, no symlinks, depth 50, and 1 MiB SQLite values are useful limits; the index has no byte cap. (`sidex/src-tauri/src/commands/index.rs:20-47`, `sidex/src-tauri/src/commands/index.rs:610-668`, `sidex/src-tauri/src/commands/storage.rs:5-9`) | **Assumed:** guards against large-repository spikes and oversized persisted state; little normal-idle change | Medium | Medium: search completeness and truncation must be visible to users |

## Bottom line

**Verified:** SideX's source invalidates the marketing shorthand that a 16.4 MB Tauri bundle demonstrates a 16.4 MB editor process. It also invalidates the idea that SideX achieved its UI by replacing VS Code chrome with a tiny custom component tree: it directly ports that chrome and eagerly imports broad workbench barrels. (`sidex/docs/assets/compare.jpg`, `sidex/README.md:28-42`, `sidex/src/main.ts:28-49`, `sidex/ARCHITECTURE.md:116-123`)

**Assumed decision:** For mac-command-bar, the right lesson is lifecycle discipline, not stack replacement. Finish the narrow native A/B of the views/SCM-override removal now present in the shared worktree; then cap/dispose inactive xterm renderers for flat growth. Keep the custom Svelte workbench, current Rust Git/SQLite/TextMate stack, and existing Monaco model LRUs. (`sidex/src/vs/workbench/contrib/terminal/common/terminalConfiguration.ts:544-551`, `sidex/src/vs/workbench/contrib/terminal/browser/terminalInstance.ts:1582-1641`, `sidex/src/vs/workbench/browser/parts/editor/editor.ts:104-111`)
