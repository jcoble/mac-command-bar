# Tree-sitter Navigation Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the slow whole-project text-grep + cold-LSP-blocking code-navigation path with a fast, persistent, tree-sitter-backed Rust symbol index that powers Monaco's go-to-definition / find-all-references / peek / cmd-click / code-lens reference-counts, and stop Monaco from eagerly reading every result file.

**Architecture:** Two independent fixes that together kill the ~30s freeze (the "two costs in series" finding in `.git/sdd/design-devils-advocate.md`): **(Cost B, frontend)** Monaco providers return `Location[]` only and materialize target models lazily via a resolver shim — no eager fan-out reads; **(Cost A, backend)** a Rust `RepoIndex` (interned-name inverted maps `defs_by_name`/`refs_by_name`, built once in the background after the existing `list_source_files` scan, persisted to a per-repo bincode cache, kept fresh by mtime/hash diff + a `notify` watcher) answers every lookup as an O(1) in-memory map hit instead of grepping the repo. The semantic LSP is demoted to **diagnostics only** (background, never on the nav hot path); a precise-when-warm overlay is a later phase.

**Tech Stack:** Rust (Tauri 2, `src-tauri/src/{main.rs,lsp.rs}`), new crates `tree-sitter` + `tree-sitter-c-sharp` (+ later `-rust`/`-typescript`), `bincode` + `serde` (serde already present), `rustc-hash` (FxHashMap), `notify` (file watcher), `twox-hash` (content hash); Svelte 5 + `monaco-editor@0.55.1` (`src/lib/MonacoSourceEditor.svelte`, `src/routes/+page.svelte`).

## Global Constraints

- **Verification:** Rust logic is gated by **cargo unit tests** run **serially** — NEVER launch parallel `cargo build`/`cargo test` (memory-critical, per CLAUDE.md); set `MSBUILDDISABLENODEREUSE=1` n/a, but do run one build/test at a time and `cargo build` once before batches. End-to-end navigation **speed + accuracy is verified ONLY by NATIVE co-verification with the user** — the web preview mocks `read_source_file`/`list_source_files`/`find_source_*`, so it cannot exercise the index. Web preview is still used to verify frontend changes compile + don't regress (Vite transform 200 + tsc + playwright screenshot).
- **The output structs are frozen:** `SourceReferenceTarget` (`main.rs:136`) and `SourceDefinitionTarget` (`main.rs:121`) must be produced unchanged by the index so existing Monaco wiring needs no shape change.
- **Reuse, do not duplicate:** the file source for the index is the existing path scan `collect_source_files`/`list_source_files_sync_with_cancellation` (`main.rs:1260`/`:1110`) with its `skip_dir_reason` ignore set (`:4153`) and `detect_language`/`is_source_file` (`:3877`/`:3816`). Reuse `spawn_blocking` + the `SourceScanRegistry`/`AtomicBool` cancellation shape (`:353`/`:389`).
- **Accuracy is honestly syntactic:** name + structure (tree-sitter), NOT type-resolved. Over/under-match on identically-named symbols is accepted and labelled. **Rename must be gated on a warm semantic LSP only** — never offered on index (approximate) data.
- **Never block the gesture:** providers enforce their own deadline (~200ms) + honor the `CancellationToken`; the index answers synchronously in-memory; the bounded grep is only a first-paint bridge while the index builds, capped (≤300 files / ≤8 MB / ≤150 ms).
- **One `+page.svelte` editor + one committer at a time** (concurrent commits corrupt the index — observed). Branch carries task ids per CLAUDE.md.
- **Cache parallelism:** the cold index build runs in **one** background task; if parallelized, cap at `num_cpus/2` — never spawn multiple concurrent index builders.

---

## File Structure

- **Create** `src-tauri/src/symbol_index/mod.rs` — public API: `SymbolIndexRegistry`, `RepoIndex`, lookup fns, `IndexState`. One responsibility: own the in-memory index + its lifecycle.
- **Create** `src-tauri/src/symbol_index/model.rs` — data types: `RepoIndex`, `FileEntry`, `Occurrence`, `SymKind`, `Interner`, `NameId`/`FileId`, `IndexState`. Pure data + (de)serialize.
- **Create** `src-tauri/src/symbol_index/tokenizer.rs` — tree-sitter parse → `Vec<Occurrence>` per file, per language; the `.scm` queries embedded as `&str` consts. The only file that knows tree-sitter.
- **Create** `src-tauri/src/symbol_index/cache.rs` — bincode load/save, repo fingerprint, dirty-set diff.
- **Create** `src-tauri/src/symbol_index/watcher.rs` — `notify` watcher → debounced changed-path set → incremental re-index calls.
- **Create** `src-tauri/tests/symbol_index_fixtures/` — small `.cs`/`.rs` fixture files for unit tests.
- **Modify** `src-tauri/src/main.rs` — register the new module + `.manage(SymbolIndexRegistry)`; re-back `find_source_references`/`find_source_definitions` (`:650`/`:637`) onto the index; add commands `build_symbol_index`, `symbol_index_status`; re-index one file inside `write_source_file_sync` (`:1371`).
- **Modify** `src-tauri/Cargo.toml` — add deps.
- **Modify** `src/lib/MonacoSourceEditor.svelte` — lazy model resolver shim; drop eager `ensureSourceTargetModels`; deadline+token in providers; codelens cache LRU + `onDidChange`.
- **Modify** `src/routes/+page.svelte` — per-path preview memo; route nav lookups at the index (fast tier); demote LSP to diagnostics-only on the nav path.

---

## Milestone A — Monaco lazy model resolution (Cost B). Frontend-only, web-verifiable NOW, no index needed.

This milestone is independent of the Rust index and delivers immediate value: it removes the eager ×50 file-read fan-out (`ensureSourceTargetModels`) that is half the freeze, and is fully verifiable on the web preview (find-refs/goto still work, far fewer reads). Land it first.

### Task A1: Memoize external previews by path + in-flight dedup

**Files:**
- Modify: `src/routes/+page.svelte` — `loadEditorExternalSourcePreview` (~`:8698`), `syncSourcePreviewContent` (~`:9879`), `writeSourceToTauri` save path (invalidate).
- Modify: `src/lib/MonacoSourceEditor.svelte` — `ensureSourceTargetModel` (~`:763`) in-flight `Map`.

**Interfaces:**
- Produces: a per-path preview cache so a repeated `onExternalPreviewLookup(path)` is a map hit, never another `read_source_file`; invalidated on write/scan.

- [ ] **Step 1: Add a per-path preview memo in the page.** In `+page.svelte`, near `loadEditorExternalSourcePreview`, add `const externalPreviewCache = new Map<string, SourcePreview>();`. At the top of `loadEditorExternalSourcePreview(record)`, after the existing draft/saved short-circuit, `const cached = externalPreviewCache.get(record.path); if (cached) return cached;`. After a successful `readSourceFromTauri`, `externalPreviewCache.set(record.path, sourcePreview);`. In the save path (`writeSourceToTauri` success) and in `scanProject` start, `externalPreviewCache.delete(path)` / `.clear()` respectively.
- [ ] **Step 2: In-flight dedup in `ensureSourceTargetModel`** (re-add the reverted map): `const inFlightTargetModels = new Map<string, Promise<Monaco.editor.ITextModel | null>>();` declared by `ownedModels`; in `ensureSourceTargetModel`, after the `getModel(uri)` check, share a pending promise per `target.path` (see `.git/sdd/design-monaco.md` §4.4 + the prior implementation pattern), with a `raced = monaco.editor.getModel(uri)` check inside the loaded body and a `.finally(() => inFlightTargetModels.delete(target.path))`.
- [ ] **Step 3: Verify compile + no regression.** `node_modules/.bin/tsc --noEmit` (clean); `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:5177/src/routes/+page.svelte` and `…/MonacoSourceEditor.svelte` → both `200`.
- [ ] **Step 4: Web-verify find-refs still works** (playwright-cli, see Verification Recipe below): open a `.cs` file, confirm codelens "N references" renders and find-refs/peek returns results.
- [ ] **Step 5: Commit** `git add … && git commit -m "perf: memoize external source previews + dedup target-model reads"` (footer per CLAUDE.md).

### Task A2: Lazy target-model resolver — delete the eager fan-out

**Files:**
- Modify: `src/lib/MonacoSourceEditor.svelte` — providers `provideDefinition`/`provideReferences`/`provideImplementation`/`provideTypeDefinition` (`:438/453/530/549`), `ensureSourceTargetModels` (`:780`), `showCodeLensReferences` (`:1495`), the editor-create resolver registration (near `registerEditorOpener` `:1552`).

**Interfaces:**
- Consumes: A1's memoized `onExternalPreviewLookup`.
- Produces: peek/jump materialize models lazily; `ensureSourceTargetModels` (bulk) is gone.

- [ ] **Step 1: Install the lazy resolver (Route 1, `design-monaco.md` §4.2).** At editor create, override the standalone text-model resolver so `createModelReference(uri)` on a miss `await`s `onExternalPreviewLookup` once, `createModel`s, tracks in a bounded LRU (§4.4), and returns the reference — instead of rejecting. If overriding proves brittle on this Monaco build, fall back to **Route 2** (pre-create only the first peek group + navigated target; lazily fill a group on first expand via the peek tree's expand event) — documented in `design-monaco.md` §4.2.
- [ ] **Step 2: Remove `ensureSourceTargetModels` from the four providers** — they return `targets.map(sourceXTargetToLocation)` only. Keep `ensureSourceTargetModel` (single) as the resolver's on-miss body.
- [ ] **Step 3: `showCodeLensReferences`** (`:1502`): drop the bulk `ensureSourceTargetModels`; rely on the lazy resolver; keep `peekLocations`.
- [ ] **Step 4: Bounded LRU for external models** — replace dispose-on-`onDidChangeModel` (`:1990` model-dispose path) with an LRU (cap ~24); `onDestroy` still disposes all.
- [ ] **Step 5: Verify** tsc + Vite 200 + playwright: open a `.cs` file, **Find All References** → peek opens; expand a file group → its preview loads; confirm via the running app's stdout that opening the peek emits **0** `read_source_file` and expanding one group emits ~the files in that group (Native co-verify for the read-count claim; web-verify that peek still renders + no crash).
- [ ] **Step 6: Commit** `perf: lazy on-demand target models (drop eager fan-out read storm)`.

### Task A3: Non-blocking providers — deadline + cancellation token

**Files:**
- Modify: `src/lib/MonacoSourceEditor.svelte` — the four nav providers; add `withDeadline` helper.

- [ ] **Step 1: Add `withDeadline(p, ms, token)`** = `Promise.race([p, sleep(ms)→null, tokenCancelled→null])` (`design-monaco.md` §2.3).
- [ ] **Step 2: Wrap each nav provider's lookup** in `withDeadline(onXLookup(request), 200, token)`; `if (token.isCancellationRequested) return null; if (!result) return [];` — never throw, never hang. Honor the token (defs/refs currently ignore it).
- [ ] **Step 3: Verify** tsc + Vite 200 + playwright: rapid re-trigger of find-refs does not paint stale peeks; a slow/empty lookup returns fast (no spinner hang).
- [ ] **Step 4: Commit** `perf: non-blocking nav providers (deadline + cancellation)`.

---

## Milestone B — Rust symbol index core. cargo-unit-testable; no native UI needed to verify the engine.

Build the engine with **C# first** (the hard case). Each task ends with a `cargo test` gate (run serially).

### Task B1: Dependencies + data model

**Files:**
- Modify: `src-tauri/Cargo.toml`.
- Create: `src-tauri/src/symbol_index/model.rs`, `src-tauri/src/symbol_index/mod.rs`.
- Modify: `src-tauri/src/main.rs` (add `mod symbol_index;`).

**Interfaces:**
- Produces: `RepoIndex`, `FileEntry`, `Occurrence`, `SymKind`, `Interner`, `NameId(u32)`, `FileId(u32)`, `IndexState` — exactly the shapes in `.git/sdd/design-rust.md` §2.2.

- [ ] **Step 1: Add deps** to `Cargo.toml` `[dependencies]`: `tree-sitter = "0.22"`, `tree-sitter-c-sharp = "0.21"`, `bincode = "1.3"`, `rustc-hash = "2"`, `twox-hash = "1.6"`, `notify = "6"`, `notify-debouncer-mini = "0.4"`. (Pin exact resolvable versions during impl; `cargo build` once to confirm they compile.)
- [ ] **Step 2: Write the model** in `model.rs` per `design-rust.md` §2.2 — `Occurrence { file: FileId, line: u32, column: u32, len: u32, kind: SymKind, container: NameId }`, `FileEntry { path, relative_path, language: u8, mtime_ns: i128, size: u64, content_hash: u64, occurrences: Vec<Occurrence> }`, `Interner { map: FxHashMap<Box<str>, NameId>, names: Vec<Box<str>> }` with `intern(&mut self,&str)->NameId` + `get(&self,&str)->Option<NameId>` + `resolve(NameId)->&str`, `RepoIndex { root, names, files, path_to_file, defs_by_name, refs_by_name, state }`, `IndexState { Empty, Building{done,total}, Ready, Stale }`. Derive `Serialize`/`Deserialize` on the persisted parts.
- [ ] **Step 3: Unit test the interner** — `tests` mod in `model.rs`: interning the same string twice returns the same `NameId`; `get` misses an unknown name; `resolve` round-trips.
- [ ] **Step 4: Run** `cargo test -p mac-command-bar-webview-preview symbol_index::model` → PASS. (Serial; one test run.)
- [ ] **Step 5: Commit** `feat(index): symbol-index data model + interner`.

### Task B2: tree-sitter C# tokenizer

**Files:**
- Create: `src-tauri/src/symbol_index/tokenizer.rs`.
- Create: `src-tauri/tests/symbol_index_fixtures/Sample.cs`.

**Interfaces:**
- Consumes: `Occurrence`, `SymKind`, `Interner` (B1).
- Produces: `fn tokenize(language: Language, src: &str, names: &mut Interner) -> Vec<Occurrence>` — definitions captured with their `SymKind`, references captured as `SymKind::Ref`; comments/strings excluded by tree-sitter node types; columns/lines 1-based.

- [ ] **Step 1: Embed the C# queries** as `const CSHARP_DEFS_SCM: &str` (capture `class_declaration`/`interface_declaration`/`method_declaration`/`property_declaration`/`field_declaration`/`enum_declaration`/`namespace_declaration` name nodes with kinds) and `const CSHARP_REFS_SCM: &str` (capture `identifier` use nodes). Start from `tree-sitter-c-sharp`'s shipped `tags.scm` for the defs side; reference `design-rust.md` §2.5.
- [ ] **Step 2: Write `tokenize`** — build a `Parser` with the C# language, parse `src`, run the two `Query`s via `QueryCursor`, map each capture node to an `Occurrence` (intern the node text, line/col from `node.start_position()` +1, len from byte range). Definitions set the declaration `SymKind`; references set `SymKind::Ref`. Set `container` from the nearest enclosing type/namespace capture (0 if none).
- [ ] **Step 3: Write the fixture** `Sample.cs` with a class `FormatResolver`, a method `Resolve`, a field, and a call site `resolver.Resolve()` plus the word `Resolve` inside a `// comment` and a `"string"`.
- [ ] **Step 4: Unit test** `tokenize` on the fixture: `Resolve` appears as exactly one `Method` def + one `Ref` (the call), and the comment/string occurrences are **excluded**; `FormatResolver` appears as one `Class` def.
- [ ] **Step 5: Run** `cargo test … symbol_index::tokenizer` → PASS.
- [ ] **Step 6: Commit** `feat(index): tree-sitter C# tokenizer (defs + refs, comment/string-free)`.

### Task B3: Build RepoIndex from a file list

**Files:**
- Modify: `src-tauri/src/symbol_index/mod.rs`.

**Interfaces:**
- Consumes: tokenizer (B2), model (B1).
- Produces: `fn build_index(root: &Path, files: &[(PathBuf, String /*rel*/, u8 /*lang*/)], cancel: &AtomicBool) -> RepoIndex` — reads each file, tokenizes, merges into `defs_by_name`/`refs_by_name` + per-file `FileEntry.occurrences`; sets `state = Ready`.

- [ ] **Step 1: Implement `build_index`** — for each file: `std::fs::read` (skip > `MAX_PREVIEW_BYTES`), compute `content_hash` (twox), `tokenize`, push occurrences into the file's `FileEntry` and into the inverted maps (def kinds → `defs_by_name`, `Ref` → `refs_by_name`), honoring `cancel`. Cap parallelism per Global Constraints (single task, or `num_cpus/2`).
- [ ] **Step 2: Unit test** — build over a 2-file fixture set (`Sample.cs` + a second file that calls `Resolve`): `refs_by_name[Resolve].len() == 2` (both call sites), `defs_by_name[Resolve].len() == 1`.
- [ ] **Step 3: Run** `cargo test … symbol_index::build` → PASS.
- [ ] **Step 4: Commit** `feat(index): build RepoIndex from file list`.

### Task B4: Lookups returning the frozen target structs

**Files:**
- Modify: `src-tauri/src/symbol_index/mod.rs`.

**Interfaces:**
- Produces: `fn lookup_references(&RepoIndex, symbol: &str, limit: usize) -> Vec<SourceReferenceTarget>`, `fn lookup_definitions(&RepoIndex, symbol: &str, limit: usize) -> Vec<SourceDefinitionTarget>`, `fn reference_count(&RepoIndex, symbol: &str) -> Option<usize>` — O(1) name lookup, miss → empty/`None` (NEVER a repo scan). Excerpts from a small per-file line cache (`design-rust.md` §2.3), bounded by `limit`.

- [ ] **Step 1: Implement the three lookups** — `names.get(symbol)?`, index the map, `take(limit)`, map each `Occurrence` to the frozen `SourceReferenceTarget`/`SourceDefinitionTarget` (`main.rs:136`/`:121`), pulling `excerpt`/`detail` from a bounded line cache. `reference_count` = `refs_by_name[name].len()`.
- [ ] **Step 2: Unit test** — on the B3 fixture index, `lookup_references("Resolve", 50)` returns 2 targets with correct paths+lines; `lookup_definitions("FormatResolver", 20)` returns the class decl; `reference_count("Resolve")` == 2; an unknown symbol returns empty/`None` and touches no filesystem (assert by building the index from in-memory strings, then deleting the fixture dir before lookup — lookups must still succeed from memory).
- [ ] **Step 3: Run** `cargo test … symbol_index::lookup` → PASS.
- [ ] **Step 4: Commit** `feat(index): O(1) reference/definition/count lookups`.

### Task B5: On-disk bincode cache + dirty-set diff

**Files:**
- Create: `src-tauri/src/symbol_index/cache.rs`.

**Interfaces:**
- Produces: `fn save(&RepoIndex, dir: &Path)`, `fn load(root: &Path, dir: &Path) -> Option<RepoIndex>` (version + fingerprint checked), `fn dirty_set(&RepoIndex, scanned: &[(PathBuf, mtime_ns, size)]) -> DirtySet { added, removed, changed }` (`design-rust.md` §2.4 + §4).

- [ ] **Step 1: Implement save/load** — bincode-serialize `RepoIndex` to `<dir>/<hash(root)>.idx` with a `u32` format-version header; `load` rejects on version mismatch or fingerprint mismatch (returns `None` → cold build).
- [ ] **Step 2: Implement `dirty_set`** — diff scanned `(path,mtime,size)` against `FileEntry` → added/removed/changed.
- [ ] **Step 3: Unit test** — build an index, `save` to a temp dir, `load` back → equal lookups; bump the format version → `load` returns `None`; change one file's mtime in the scanned list → `dirty_set.changed` contains exactly that path.
- [ ] **Step 4: Run** `cargo test … symbol_index::cache` → PASS.
- [ ] **Step 5: Commit** `feat(index): bincode on-disk cache + dirty-set diff`.

### Task B6: Registry + Tauri commands + background build + re-back the existing commands

**Files:**
- Modify: `src-tauri/src/symbol_index/mod.rs` (`SymbolIndexRegistry`), `src-tauri/src/main.rs` (`.manage`, command registration `:4204`, re-back `find_source_references`/`find_source_definitions` `:650`/`:637`, re-index in `write_source_file_sync` `:1371`).

**Interfaces:**
- Consumes: build/lookup/cache (B3-B5), the path scan `collect_source_files`.
- Produces: `SymbolIndexRegistry { map: Mutex<HashMap<RepoKey, Arc<RwLock<RepoIndex>>>> }` (tauri-managed); commands `build_symbol_index(root)` (spawn_blocking, cancellable, loads cache then diffs then background-builds dirty set), `symbol_index_status(root) -> IndexState`. `find_source_references`/`find_source_definitions` bodies become `registry.get(root).read()` + the B4 lookups (with the bounded grep — Task B7-bridge — only while `state != Ready`).

- [ ] **Step 1: Implement `SymbolIndexRegistry`** + `build_symbol_index` (load cache → `dirty_set` from a fresh path scan → background re-index dirty files → save) reusing `spawn_blocking` + `AtomicBool` cancel.
- [ ] **Step 2: Re-back `find_source_references`/`find_source_definitions`** onto `lookup_references`/`lookup_definitions`; keep a **capped** grep (≤300 files/≤8MB/≤150ms) only when `state != Ready` (build-gap bridge, `design-rust.md` §6).
- [ ] **Step 3: Re-index one file in `write_source_file_sync`** — after a successful save, re-tokenize that file and swap its occurrences (incremental update, `design-rust.md` §4).
- [ ] **Step 4: Register** all new commands in the `invoke_handler` list (`main.rs:4204`).
- [ ] **Step 5: Unit test** the registry + incremental swap (build, save a changed file's new occurrences, assert the inverted maps updated O(file) not O(repo)); `cargo build` once to confirm the whole crate compiles with the new commands.
- [ ] **Step 6: Run** `cargo test … symbol_index` (all) → PASS; `cargo build` → OK.
- [ ] **Step 7: Commit** `feat(index): registry + tauri commands + re-back find-refs/defs on the index`.

### Task B7: notify file-watcher + incremental freshness

**Files:**
- Create: `src-tauri/src/symbol_index/watcher.rs`.
- Modify: `src-tauri/src/symbol_index/mod.rs` (start/stop watcher with the index).

- [ ] **Step 1: Implement the watcher** — `notify-debouncer-mini` watching `root`, applying the same `skip_dir_reason` ignore set, debounced ~200ms, coalescing events → for each changed/added path re-tokenize + swap occurrences; removed path drops its `FileId`.
- [ ] **Step 2: Unit test the incremental apply** (not the OS watcher itself): feed a synthetic "changed path + new content" through the swap function → inverted maps reflect the new occurrences and drop the old ones.
- [ ] **Step 3: Run** `cargo test … symbol_index::watcher` → PASS; `cargo build` → OK.
- [ ] **Step 4: Commit** `feat(index): notify file-watcher + incremental re-index`.

---

## Milestone C — Integrate + demote LSP + NATIVE co-verify.

### Task C1: Route Monaco nav lookups at the index (fast tier)

**Files:**
- Modify: `src/routes/+page.svelte` — `findSourceReferenceTargetsForEditor` (`:8603`), `findSourceDefinitionTargetsForEditor` (`:8563`), `countSourceReferencesForCodeLens` (`:8643`).

- [ ] **Step 1:** Make these call the index commands first (instant), drop the LSP-first-blocking + the unbounded whole-project grep from the hot path (`design-monaco.md` §2.2/§6). Keep the bounded grep only as the build-gap bridge (already capped in Rust, B6-step2). Return targets immediately.
- [ ] **Step 2: Verify** tsc + Vite 200 + playwright web smoke (still renders; mock returns nothing for the index command, so web shows the bounded-grep/demo path — confirm no crash + counts still render on web).
- [ ] **Step 3: Commit** `feat: wire Monaco navigation to the Rust symbol index`.

### Task C2: Demote the LSP to diagnostics-only on the nav path

**Files:**
- Modify: `src/routes/+page.svelte` (nav lookup handlers), `src/lib/MonacoSourceEditor.svelte` (provider wiring). Diagnostics push path (`lsp.rs` + `applyExternalDiagnostics`) stays untouched.

- [ ] **Step 1:** Remove the synchronous LSP reference/definition calls from the nav hot path; keep `loadSourceLspDiagnostics` (diagnostics) + syntax highlighting (Monaco-native) intact. (Precise-when-warm overlay is Milestone D — not now.)
- [ ] **Step 2: Verify** tsc + Vite 200; web smoke: diagnostics squiggles still appear (mock), nav uses the index/grep path.
- [ ] **Step 3: Commit** `refactor: LSP is diagnostics-only; navigation comes from the index`.

### Task C3: NATIVE co-verification (with the user)

- [ ] **Step 1: Build + launch native** — `cd tauri-svelte-preview && node_modules/.bin/tauri dev --config src-tauri/tauri.dev.attach.conf.json` (cargo build serial; attaches to Vite 5177).
- [ ] **Step 2: With the user**, open the EdiPlatform C# repo, wait for `symbol_index_status` → `Ready` (watch stdout for the background build completing in a few seconds), then: cmd-click a symbol, Find All References, peek, code-lens count. **Confirm:** first lookup after Ready is sub-second; opening a peek emits 0 `read_source_file` and expanding a group emits only that group's files; no 6s freeze; counts render (with `~` if still building). Capture the stdout read counts as evidence.
- [ ] **Step 3:** If the user confirms the speedup + acceptable accuracy, the milestone is done. Record the result in `.git/sdd/progress.md`. (No commit gate beyond C1/C2 — this is a verification step.)

---

## Milestone D — Future phases (NOT in v1; plan-level only)

- **More grammars:** add `tree-sitter-rust`, `tree-sitter-typescript` (+ Svelte via the embedded `<script lang=ts>` region, `design-rust.md` §2.5); each is a `tokenize` arm + a fixture test, behind the same `RepoIndex` API.
- **Precise-when-warm LSP overlay** (`design-monaco.md` §3, `design-rust.md` §7): fix the csharp-ls client (answer `window/workDoneProgress/create` + `client/registerCapability` + `workspace/configuration`, advertise capabilities, wait-for-`$/progress`, raise cold-load budget, stop `reroot` killing the warm process — `.git/sdd/research-csharp-lsp.md` §5 / `code-lsp-map.md`); add `CodeLensProvider.onDidChange` + an `onSemanticIndexWarmed` signal to upgrade approximate→precise in place; enable **rename** only when the LSP is warm.
- **Phase-2 scope/binding** (`design-rust.md` §2.2 `bindings`, §9): per-file scope trees + C# namespace/using/type-member resolution so refs bind to the right symbol (kills same-name over-counting). Measure-first.

---

## Self-Review

**1. Spec coverage:**
- design-rust.md index (interned inverted maps, bincode cache, mtime/hash freshness, notify watcher, built after the scan) → B1-B7. ✓
- design-monaco.md (non-blocking providers, drop eager `ensureSourceTargetModels` for lazy `createModelReference` resolver, codelens cache/`onDidChange`, per-path memo) → A1-A3 + (codelens `onDidChange` upgrade is D, since it only matters with the precise overlay). ✓ (codelens cache-LRU / stop-clear-on-switch folded into A2-step4 + noted; if the implementer wants it standalone, split — but it's a small change.)
- devil's-advocate two-costs-in-series (Cost A grep + Cost B fan-out, fix BOTH) → A (Cost B) + B/C (Cost A). ✓
- research-csharp-lsp (LSP = background, the abort fix) → D (overlay) + C2 (demote). ✓
- Frozen output structs, reuse the scan, syntactic-honest accuracy, rename-gated-on-LSP, never-block, serial cargo, native co-verify → Global Constraints + per-task gates. ✓

**2. Placeholder scan:** Tree-sitter `.scm` query *contents* are described by capture intent + "start from the grammar's shipped `tags.scm`" rather than inlined verbatim — this is the one place exact code is deferred to implementation because the precise S-expressions depend on the installed grammar version; every other code step is concrete. Flagged honestly, not a TODO.

**3. Type consistency:** `SourceReferenceTarget`/`SourceDefinitionTarget` are the frozen existing structs throughout (B4/B6/C1). `RepoIndex`/`Occurrence`/`FileEntry`/`Interner`/`IndexState`/`NameId`/`FileId` are defined once (B1) and consumed unchanged (B2-B7). Lookup fn names (`lookup_references`/`lookup_definitions`/`reference_count`) are stable B4→B6. ✓

---

## Execution Handoff

Verify before merge per Global Constraints. Two execution options:

1. **Subagent-Driven (recommended)** — fresh implementer per task, task review between tasks, fast iteration. Milestone A is web-verifiable and can run start-to-finish; Milestones B's cargo tests run **serially** (one builder at a time); Milestone C ends in a **native co-verification with the user** (cannot be done by a subagent — the controller schedules it with the user).
2. **Inline Execution** — batch with checkpoints.

**Recommended order:** Milestone A first (immediate, web-verifiable freeze relief), then B (engine, unit-tested), then C (integrate + native co-verify with the user). D is a follow-up.
