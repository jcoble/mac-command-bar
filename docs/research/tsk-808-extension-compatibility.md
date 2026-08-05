# TSK-808 extension/API compatibility register

**Preparation lane:** WP12A bounded preparation  
**Repository base:** `5eadb6da8cdfb16eb2ee816ebd3b51248a6f888c`  
**Date:** 2026-08-04  
**Native proof status:** **Passed for bounded internal adapters in the live `/next`
Tauri app.** App PID `51821` was launched by `tauri:dev:next` and remains running
for user inspection.

## Decision in one paragraph

Keep the one installed Monaco/VS Code service container and its one
`ExtensionHostKind.LocalWebWorker`. The native run proved the bounded internal
adapter path for commands, workspace roots/files/events, app-owned accessible
notification/progress status, the existing owned terminal service, the existing
Rust Git/GitService SCM adapter, and Roslyn CodeLens/Peek continuity. It did not
promote unavailable VS Code workbench projections into supported APIs: tree/view
projection and `vscode.scm` projection remain **Elevated host/service override
required/skipped**. The runtime/manifest attempt was removed, and no second
service container is allowed.

The user-selected policy releases only **Works now** and **Bounded adapter** rows.
A `Declarative only`, `Elevated host/service override required`, or `Rejected` row
is recorded precisely and does not block this wave.

## Evidence boundary

| Evidence slice | Verified native evidence | What is explicitly not claimed |
| --- | --- | --- |
| Live app identity | `/next` Tauri app PID `51821` from `tauri:dev:next`; app left running for user inspection. | This report does not claim a packaged/release app result. |
| C# workspace readiness | EdiPlatform C# reached ready state; lifecycle switch sequence completed `EdiPlatform -> rental-management -> EdiPlatform` at generation `3`. | No unrelated language server or workspace provider behavior is claimed. |
| Worker activation and guarded filesystem | Worker activation generation `1`; README fixture read returned `1478` bytes; outside-root and stale-generation paths were rejected. | No unbounded filesystem access, direct Tauri IPC, or external workspace access is enabled. |
| Terminal adapter | `show`, `write`, `resize`, and `dispose` passed through the existing owned terminal service; no lingering PTY process was found. | No arbitrary shell command, executable field, process API, or second terminal backend is exposed. |
| Internal Rust Git/GitService SCM adapter | Native read/refresh/dispose passed with `2` status groups; no lingering Git probe process was found. | This is not VS Code `vscode.scm` projection support and does not enable stage/commit/checkout mutation. |
| Roslyn semantic continuity | Exactly two warm Roslyn processes were present in distinct cwd roots; CodeLens lines `15/20/24/26/27/28` were visible. Clicking line `27` references populated Peek: `Found 28 symbols in 9 files`, including `2` in `EmailAlertDispatcherTests` and `19` in `EdiPlatformDbContext`; selected source was `public class AdminAlertConfiguration`; warm Peek was `46ms`. | No second C# language server, alternate semantic owner, or synthetic browser-only result is claimed. |
| Editor state preservation | Center tabs and selected file were preserved across the native acceptance flow. | This does not claim unrelated tab-strip or navigation behavior outside the tested flow. |
| Console cleanliness | Fresh DevTools console showed only Vite/Svelte warning/info messages. It showed no unsupported SCM/view, extension-host startup, `file:///src`, load-failed, or disposed-model errors. | Vite/Svelte warning/info output is not treated as a failure here. |
| Resource sample | Earlier sample recorded app idle `0.0%` CPU / `119,216` KiB RSS and peak `0.1%` / `119,232`; primary WebKit worker host idle `0.0%` / `889,728` KiB and peak `0.1%` / `889,744`; Roslyn EdiPlatform `0.0%` / `730,432`; Roslyn rental-management `0.0%` / `164,960`. | This is a sampled resource receipt, not a continuous profiler trace. |
| Disk footprint | Feature source total `70,486` bytes: browser probe `13,163`, contract `34,324`, controller `9,784`, bridge `4,094`, SCM adapter `4,380`, iframe `4,741`. Emitted probe asset `13,163` bytes. Build output `28,940` KiB, `.svelte-kit` `40,572` KiB, `src-tauri/target` `2,421,200` KiB. | These are measured disk sizes for the reported artifacts/directories, not an install/update/uninstall package budget. |
| Full native app disposal | Disposal samples were `106ms`, `104ms`, and `104ms`; exact app, WebContent, two Roslyn, and DevTools processes were gone; OS-level remaining app/worker/Roslyn/PTY/Git owners were `0`. | In-process listener/command/SCM owner enumeration after process death is not externally available; Node contracts assert cleanup. |
| Build/test verification | Controller ran the focused extension, SCM, terminal, CodeLens, and dockview contracts, `pnpm check`, `git diff --check`, and one serialized `pnpm build`; all passed. | The stale broad legacy `test:source-ui` suite and unrelated pre-existing Svelte accessibility/chunk-size warnings are not promoted into this lane's acceptance. |

No database is touched by this lane. No SQL, persistence, migration, aggregation,
or data-access code is introduced. The project hard rule still applies: all SQL
aggregation, filtering, joins, sorting, and paging must remain server-side in one
translated statement or database view.

## Classification register

Every row uses exactly one policy classification. “Works now” means the existing
implementation has repository evidence. “Bounded adapter” means the native run
proved a narrow adapter over an existing app-owned service without creating a new
host, process owner, or user-state owner.

| API/capability | Classification | Repository evidence / adapter boundary | Native claim |
| --- | --- | --- | --- |
| Houston theme contribution | **Works now** | `extensionRuntime.ts` registers the pinned JSON contribution once; `extensionIntegration.test.mjs` checks the asset and the prior report records the browser-preview result. | Existing baseline preserved. |
| Common language grammar/snippet contributions | **Declarative only** | Pinned `@codingame/monaco-vscode-*` packages and official Svelte assets are registered without a Node entrypoint or language-server process. | No activation probe is required for the declarative path. |
| Native Monaco DiffEditor | **Works now** | `NativeGitDiffEditor.svelte` owns paired `file://` models; existing Git data supplies bounded original/modified content. | Existing implementation evidence preserved; not retested by this report writer. |
| `extensions.getExtension`, activation, `whenReady`, `dispose` | **Bounded adapter** | Fixture binds to the existing registration handle and LocalWebWorker readiness. Dispose removes only this fixture's registrations and permits reactivation without duplicates. | Passed: worker activation generation `1`; no extension-host startup errors in fresh DevTools console. |
| `commands.registerCommand` / `executeCommand` | **Bounded adapter** | Fixture registers deterministic commands in the shared command service; command results carry extension ID, host tier, owned ID, and generation. No second palette/registry is permitted. | Passed through the active native worker generation. |
| `workspace.workspaceFolders` | **Bounded adapter** | Fixture accepts only canonical roots from the active owned session. Generation changes are required for root changes; stale or foreign owned IDs are ignored. | Passed: lifecycle `EdiPlatform -> rental-management -> EdiPlatform`, generation `3`; no stale unsupported workspace-folder console error in the fresh run. |
| `workspace.fs` read/stat/readDirectory | **Bounded adapter** | Fixture checks active-root membership, path boundary, exact allow-list, and generation before forwarding to the existing native C#/workspace filesystem bridge. | Passed: EdiPlatform README read returned `1478` bytes; outside-root and stale-generation paths rejected. |
| Workspace-folder and file events | **Bounded adapter** | One listener of each kind, generation-tagged event ledger, stale-event counter, and disposal. No listener is registered per editor or workspace. | Passed across the generation `3` lifecycle. |
| `window.showInformationMessage` | **Bounded adapter** | Forwards app-owned accessible plain-text status through the existing workbench surface; no fixture toast stack. | Passed as an accessible app-owned status receipt. No native VS Code dialog behavior is claimed. |
| `window.withProgress` | **Bounded adapter** | Forwards bounded progress invocation through the worker and app-owned status surface; no background process or duplicate progress UI. | Passed as worker invocation plus app-owned accessible status receipt. No native VS Code progress dialog behavior is claimed. |
| Tree/view projection | **Elevated host/service override required** | EditorService reported no registered view. Runtime/manifest attempt was removed; no second service container was added. | Skipped without blocking the bounded adapter wave. |
| Terminal API | **Bounded adapter** | `McbProbeTerminalAdapter` has only request/show/write/resize/dispose and requires exact `ownedId`, generation, active root, bounded grid, and bounded input. It maps to `nextTerminalService`/`TerminalService`; there is no command, shell, executable, or process field. | Passed: show/write/resize/dispose completed; no lingering PTY process. |
| Internal Rust Git/GitService adapter | **Bounded adapter** | `McbProbeScmAdapter` exposes read/select/refresh/dispose only over `rustGitScmProvider.ts`/`gitService.ts`; it rejects root/generation mismatch and bounds groups/resources. | Passed: read/refresh/dispose completed with `2` groups; no lingering Git probe process; mutation remains disabled. |
| VS Code `vscode.scm` projection | **Elevated host/service override required** | The VS Code SCM service projection is not supplied by the current host without an elevated service override. The internal Rust Git/GitService adapter remains the supported bounded path. | Skipped without blocking the bounded adapter wave. |
| Language-server semantic navigation / CodeLens / Peek | **Bounded adapter** | Existing Roslyn client remains the sole semantic owner and workspace-scoped process. The probe must not register a second language server or change CodeLens/Peek ownership. | Passed: exactly two warm Roslyn processes in distinct cwd roots; CodeLens lines `15/20/24/26/27/28` visible; line `27` Peek populated `28` symbols in `9` files; selected source preserved; warm Peek `46ms`. |
| Webview/custom editor APIs | **Elevated host/service override required** | CSP, z-order, resource roots, disposal, and untrusted content were not proven by the current editor-service host. No fixture or product enablement is added. | Skipped without blocking. |
| Network/auth/secrets | **Elevated host/service override required** | LocalWebWorker does not by itself establish outbound-network denial or credential isolation. The fixture receives no Tauri IPC or secret handle. | Skipped; measure the actual worker/CSP boundary before any third-party executable extension. |
| GitLens/full workbench | **Elevated host/service override required** | Browser activation alone does not supply its expected views, storage, repository/auth, and workbench services. This is a separate approved spike, not a blanket rejection. | Skipped without blocking. |
| Generic VSIX import/update/uninstall | **Elevated host/service override required** | No exact package/hash was selected. Hash consent, traversal/size/symlink defenses, rollback, and cleanup remain deferred graduation work. | Skipped without blocking. |
| Git Graph extension | **Rejected** | Prior repository review found a Node/workspace-oriented extension with no browser entry, Git-executable dependency, and unsuitable redistribution constraints. Native Rust history plus Svelte graph remains the owner. | Not a native probe candidate. |

## Internal probe contract

`src/lib/shell/editor/fixtures/mcbExtensionApiProbe.ts` is intentionally an adapter
contract, not a registration point. The controller supplies seams after the existing
singleton has started:

1. `vscode.commands` and `vscode.workspace` adapters, including one
   workspace-folder listener and one file listener.
2. `vscode.window.showInformationMessage` and `vscode.window.withProgress`
   forwarding to app-owned accessible status/progress surfaces.
3. A terminal adapter backed by the existing `nextTerminalService`/`TerminalService`
   mapping. It may request/show/write/resize/dispose one exact `ownedId` and
   generation and must not create a shell or process independently.
4. A read-only SCM adapter backed by `rustGitScmProvider.ts` and `gitService.ts`.
   It may read/select/refresh/dispose the active-root projection; the Rust Git
   service remains the repository authority and no extension command may run Git.
5. Roslyn semantic checks stay observational. The fixture must not register a
   second C# language server or change CodeLens/Peek ownership.

The fixture state is observable through a deterministic snapshot: registration
counts, accepted/stale events, active generation, terminal request/write/resize/
dispose counts, SCM read/refresh/dispose counts, and notification/progress counts.
Activation is idempotent for the same host/context; reactivation is possible only
after disposal. A workspace switch changes generation and does not add a worker,
command, PTY, SCM owner, listener, service container, or Roslyn process owner.

## Exact comparison metrics

The HTML companion at `output/tsk-808-extension-compatibility/index.html` uses the
same metric vocabulary and records measured native values where the run contains
evidence. After this report refresh, the controller ran the focused contracts,
`pnpm check`, `git diff --check`, and one serialized `pnpm build`; all passed.

| Metric | Native evidence captured |
| --- | --- |
| Cold app/host launch | Measured failure/limit: dirty-HMR attempt showed window/top bar but `MY WORK` failed within `99,000ms`; label discarded as an inconclusive stale-dev-runtime attempt, not a product cold benchmark. Current correct app later hydrated under `/next` PID `51821`; worker activation generation `1`. |
| Warm app/host launch | Warm state measured exactly two Roslyn processes in distinct cwd roots: EdiPlatform `0.0%` CPU / `730,432` KiB RSS and rental-management `0.0%` CPU / `164,960` KiB RSS. |
| Command latency | Measured failure/limit: AX automation produced `0` valid samples, so no command percentile is claimed. Functional native command/adapters passed. Warm Peek interaction measured `46ms`. |
| Workspace switch | Native timing from the restored `/next` runtime: selecting the EdiPlatform session made Source Control available in `936.2ms` and reached `C#: ready` in `1,177.3ms`. The earlier warm sequence `EdiPlatform -> rental-management -> EdiPlatform` also succeeded, worker reached generation `3`, no hang occurred, and stale/outside requests rejected. Exact small-versus-large percentile samples remain a measurement limit. |
| CPU/RSS | Earlier sample: app idle `0.0%` CPU / `119,216` KiB RSS, peak `0.1%` / `119,232`; primary WebKit worker host idle `0.0%` / `889,728` KiB, peak `0.1%` / `889,744`; Roslyn EdiPlatform `0.0%` / `730,432`; Roslyn rental-management `0.0%` / `164,960`. |
| Disk | Feature source total `70,486` bytes: browser probe `13,163`, contract `34,324`, controller `9,784`, bridge `4,094`, SCM adapter `4,380`, iframe `4,741`. Emitted probe asset `13,163` bytes. Build output `28,940` KiB, `.svelte-kit` `40,572` KiB, `src-tauri/target` `2,421,200` KiB. |
| Lifecycle | Terminal show/write/resize/dispose passed; internal Rust Git SCM read/refresh/dispose passed with `2` groups; no lingering PTY/Git probe process. Full native app disposal measured `106ms`, `104ms`, `104ms`; exact app/WebContent/two Roslyn/DevTools processes all gone; OS-level remaining app/worker/Roslyn/PTY/Git owners `0`. Measurement limit: in-process listener/command/SCM owner enumeration after process death is not externally available; Node contracts assert cleanup. |
| Editor safety | CodeLens `15/20/24/26/27/28` visible; clicked `27` references; Peek found `28` symbols in `9` files, including `2` in `EmailAlertDispatcherTests` and `19` in `EdiPlatformDbContext`; selected source was `public class AdminAlertConfiguration`; fresh console had no unsupported SCM/view, extension-host startup, `file:///src`, load-failed, or disposed-model errors. |

## Acceptance sequence: performed versus deferred

| Step | Status | Evidence / deferral |
| --- | --- | --- |
| Verify app identity and launch `/next` | **Performed** | PID `51821` from `tauri:dev:next`; app remains running for user. |
| Activate worker and command/workspace adapters | **Performed** | Worker activation generation `1`; workspace lifecycle reached generation `3`. |
| Read in-root fixture and reject unsafe paths | **Performed** | EdiPlatform README read `1478` bytes; outside-root and stale-generation rejected. |
| Notification/progress | **Performed** | Worker invocation and app-owned accessible status receipt passed; no VS Code dialog behavior claimed. |
| Tree/view projection | **Deferred/skipped** | EditorService reports no registered view; elevated host/service override required. Runtime/manifest attempt removed; no second service container. |
| Terminal adapter | **Performed** | Show/write/resize/dispose passed; no lingering PTY process. |
| Internal Rust Git/GitService SCM adapter | **Performed** | Read/refresh/dispose passed with `2` groups; no lingering Git probe process. |
| VS Code `vscode.scm` projection | **Deferred/skipped** | Elevated host/service override required; kept separate from the internal Rust Git/GitService adapter. |
| Workspace lifecycle and disposal guard | **Performed** | `EdiPlatform -> rental-management -> EdiPlatform`, generation `3`; center tabs and selected file preserved. |
| Roslyn CodeLens/Peek continuity | **Performed** | Two warm Roslyn processes in distinct cwd roots; CodeLens `15/20/24/26/27/28` visible; clicked `27`; Peek populated `28` symbols in `9` files; selected `public class AdminAlertConfiguration`; warm Peek `46ms`. |
| Fresh DevTools console check | **Performed** | Only Vite/Svelte warning/info messages; no unsupported SCM/view, extension-host startup, `file:///src`, load-failed, or disposed-model errors. |
| Build/test verification | **Performed** | Focused extension/SCM/terminal/CodeLens/dockview contracts, `pnpm check`, `git diff --check`, and serialized `pnpm build` all passed. Existing unrelated build warnings were reported but did not fail the build. |

## Stop conditions

Stop and return a bounded SOL/controller decision if future integration requires a
second service container, a new LSP/PTY/Git owner, direct Tauri IPC, arbitrary
process execution, unbounded filesystem/network access, or a new manifest/capability/
package change outside the approved receipt. Do not relabel skipped elevated
workbench projections as supported because the bounded internal adapters passed.
