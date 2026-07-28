# Old-shell autopsy — what made it slow (audit checklist for reuse)

Source audited: `tauri-svelte-preview/src/routes/+page.svelte` (~19.4k lines, read-only).
Method: read-only greps + targeted reads, 60-minute timebox. Depth over breadth — 5
well-evidenced poison instances below, plus the known regression commits for cross-reference.

## Poison patterns found (file:line each)

### 1. Unguarded backend fan-out in `onMount` — the "eager hydration" burn

`onMount` (`tauri-svelte-preview/src/routes/+page.svelte:14642`) does all of its localStorage
restores synchronously, then — with no snapshot restore in play — fires **six independent
backend calls in parallel plus a three-step sequential chain**, none gated by any user action:

```
tauri-svelte-preview/src/routes/+page.svelte:14808  void loadProjectGitStatus(startupProject)
tauri-svelte-preview/src/routes/+page.svelte:14809  void loadGitCommitHistory(startupProject)
tauri-svelte-preview/src/routes/+page.svelte:14810  void loadRuntimeContexts(startupProjectOptions)
tauri-svelte-preview/src/routes/+page.svelte:14811  void loadProjectWorktrees(startupProject)
tauri-svelte-preview/src/routes/+page.svelte:14812  void loadAgentSessions()
tauri-svelte-preview/src/routes/+page.svelte:14813  void loadOrchestrationRuns(startupProjectOptions)
tauri-svelte-preview/src/routes/+page.svelte:14814-14820
    void (async () => {
        await loadGitRepositorySummaries(startupProjectOptions);
        await scanProject(startupProject, storedSelectedSourcePaths[startupProject.id], { limit: expandedSourceScanLimit });
        await indexProjectsInBackground(startupProjectOptions);
    })();
```

Each of these `load*` helpers (defined `:3776`–`:3990`) is a `*FromTauri` invoke wrapped in its
own loading/error state — none share a request coalescer, none check "is this data already
warm," and all of them fire before Svelte has painted a single frame the user asked for. This is
the direct analog of the `cad76d3` "eager fan-out read storm" and `e606033` localStorage-quota
failure: work the user never asked for, done at the worst possible time (cold start).

### 2. Background project indexing — unbounded parallel full-tree walks

`indexProjectsInBackground` (`tauri-svelte-preview/src/routes/+page.svelte:3700-3715`) loops
every *non-selected* project and fires `indexProjectInBackground` for each with no concurrency
cap:

```
tauri-svelte-preview/src/routes/+page.svelte:3712-3714
    for (const project of projectsToIndex) {
        void indexProjectInBackground(project);
    }
```

`indexProjectInBackground` (`:3717`) calls `listSourceFilesFromTauri` (`:3724`) — a full
native filesystem walk of that project's tree — for every project simultaneously. This is
called from the mount chain above (`:14819`) and also after every project activation
(`:14462`, inside `scanCompletion.then(...)`), so switching projects re-triggers a full
parallel walk of every *other* known project in the background, unconditionally.

### 3. `scanProject` reachable from 8+ independent call sites — the "ghost double-scan" shape

`scanProject` (`tauri-svelte-preview/src/routes/+page.svelte:3224`) is invoked from at least
eight distinct reactive/user paths with no shared in-flight guard beyond a generation counter
that discards *results* (not the in-flight backend work) — exactly the bug `cd2f525` had to
patch after the fact by adding `cancelSourceScanFromTauri` for the superseded scan
(`:3294-3297`):

```
tauri-svelte-preview/src/routes/+page.svelte:1742   command-palette "scan-project" action
tauri-svelte-preview/src/routes/+page.svelte:1750   command-palette "scan-project-expanded" action
tauri-svelte-preview/src/routes/+page.svelte:3333   self-recursive repair call inside scanProject
tauri-svelte-preview/src/routes/+page.svelte:10738  reactive void call in a preview/reload path
tauri-svelte-preview/src/routes/+page.svelte:14455  project-activation flow
tauri-svelte-preview/src/routes/+page.svelte:14503  fallback-project-after-delete flow
tauri-svelte-preview/src/routes/+page.svelte:14816  onMount startup chain (no snapshot case)
tauri-svelte-preview/src/routes/+page.svelte:15050  UI button/action binding
tauri-svelte-preview/src/routes/+page.svelte:15099  UI button/action binding
```

Notably, `scanProject` still ships a **leftover temp diagnostic** from debugging that exact
duplication — a `console.warn` that builds a full stack trace on *every* scan call and was
never removed:

```
tauri-svelte-preview/src/routes/+page.svelte:3233-3237
    // TEMP DIAGNOSTIC (remove after pinning the double-scan): log every
    // scanProject invocation + its caller stack so we can see the two triggers.
    console.warn(
        `[mcb scan] project=${project.name} force=${Boolean(options.force)} limit=${options.limit ?? "default"} gen=${generation}\n${new Error().stack ?? "(no stack)"}`,
    );
```
This is a small thing on its own, but it is evidence the module still carries unresolved
debug scaffolding from a past perf incident — a sign to read surrounding code with suspicion,
not copy it verbatim.

### 4. `$effect` that does backend IO on every keystroke (best- and worst-case example in one)

Of the file's 12 `$effect` blocks (`grep -n '\$effect'` → lines 3047, 3072, 3080, 3103, 3111,
3139, 3162, 3184, 3201, 13073, 13079), exactly **one** reaches backend IO — the quick-open
workspace-symbol search:

```
tauri-svelte-preview/src/routes/+page.svelte:3080-3101
    $effect(() => {
        const query = quickOpenWorkspaceSymbolQuery;
        if (!quickOpenVisible || !quickOpenWorkspaceSymbolMode) { ...; return; }
        if (!query || !files.preview || !sourceIntelligenceAvailable) { ...; return; }
        const timer = window.setTimeout(() => {
            void loadSourceLspWorkspaceSymbols(query);   // -> findSourceLspWorkspaceSymbolsFromTauri (invoke)
        }, 180);
        return () => window.clearTimeout(timer);
    });
```
`loadSourceLspWorkspaceSymbols` (`:10054`) calls `findSourceLspWorkspaceSymbolsFromTauri`
(`:10069`), a Tauri `invoke`. This is actually the **positive** example: visibility-gated,
mode-gated, query-gated, 180ms debounced, and cleans up its own timer — worth copying the
*shape* of. The negative lesson is elsewhere (findings 1–3): none of those apply the same
discipline to mount-time or project-switch-time backend calls, even though they are far more
expensive (full filesystem walks vs. a single LSP query) than anything gated behind this
effect.

### 5. Snapshot-restore path duplicates the same chain under a different name

`restoreConversationWorkspaceSnapshot` (`tauri-svelte-preview/src/routes/+page.svelte:6567`),
reached from the `onMount` startup path when a workspace snapshot exists
(`:14802-14806`, `void restoreConversationWorkspaceSnapshot(startupWorkspaceSnapshot).then(() => indexProjectsInBackground(startupProjectOptions))`),
independently calls `activateWorkspaceSnapshotProject` (`:6592`), `selectRecord` (`:6596`), and
`restoreWorkspaceEmbeddedTerminal` (`:6601`) — a second, differently-shaped fan-out of backend
calls that duplicates most of what finding #1's non-snapshot branch does (git status, scan,
terminal session lookup), just reached through a different startup branch. Two branches with
independently-maintained IO fan-out are twice the surface for the same class of bug.

## The audit checklist (apply to ANY old `$lib` module before reuse in `/next`)
- [ ] No IO at module import time or component mount
- [ ] No `$effect` that calls `invoke`/`*FromTauri`/`fetch`
- [ ] No hidden singleton state that assumes one instance
- [ ] Pure logic separated from DOM/runtime access

Additional checks earned by this audit specifically:
- [ ] No function reachable from more than one call site is allowed to start backend work
      without an in-flight guard that cancels/dedupes the superseded call (not just discards
      its result) — see finding #3 / commit `cd2f525`.
- [ ] Any loop that fires `void someAsyncCall(x)` per item in a collection has an explicit
      concurrency cap or sequencing — see finding #2.
- [ ] No leftover `console.warn`/`console.log` diagnostic scaffolding with stack-trace capture
      on a hot path — see finding #3.
- [ ] Backend calls gated behind reactive UI state (search boxes, popovers) follow the
      visibility-gate + debounce + cleanup shape in finding #4, not the ungated mount-time shape
      in finding #1.

## Known regression commits (from git log, for reference)
- `cad76d3` — perf: lazy on-demand target models (eager fan-out read storm from Monaco
  navigation providers reading every peek-tree target eagerly)
- `cd2f525` — perf: cancel the superseded source scan's backend walk (kills the ghost
  double-scan — the generation counter discarded *results* but the Rust-side walk kept running)
- `52ad343` / `0593c20` — perf: stop code-lens reference counts from scanning the whole project,
  then revert (fix broke the web/cold-LSP fallback; native fix deferred separately)
- `e606033` — fix: never fail the source scan on a localStorage quota error (scan cache write on
  a 5,781-file repo exceeded browser storage quota and looped the scan as "failed")
