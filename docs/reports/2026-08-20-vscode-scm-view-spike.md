# SCM embedding feasibility spike

## 1. Verdict

**MOUNTS-WITH-CAVEATS.** The Playwright session that previously failed to start (npm cache / daemon EPERM) worked on this continuation. `MonacoVscodeApiWrapper` starts, `attachPart(Parts.SIDEBAR_PART, scmHost)` succeeds, `workbench.view.scm` executes, and the fake SCM data (commit input box, `Merge Changes` / `Staged Changes` / `Changes` resource groups, and every fake file row) is present and correctly structured in the DOM — verified via `element.innerText` on the mounted host:

```text
Source Control
Drag a view here to display.
Message (press Cmd+Enter to commit)
Merge Changes
1
conflicted.ts ~/src
Staged Changes
2
README.md
app.ts ~/src
Changes
3
new-file.ts ~/src
old.ts ~/src
client.ts ~/src/git
```

The mount button's status output (`[data-testid=mount-state]`) reads `"Mounted"`, not `"Failed"`.

**The caveat:** none of that content actually paints. The screenshot (`scm-spike.png`) shows only the pane header ("Source Control") and an unrelated empty-pane placeholder string, "Drag a view here to display." — no commit box, no resource groups, no file rows visible on screen, even though they exist in the DOM with real, non-zero layout boxes (e.g. the "Changes" group header measured at `x:41, y:280, w:307, h:22`). Root cause, found by walking the ancestor chain of a rendered row: the immediate clipping ancestor, `.split-view-container`, has a computed height of **0px**, while the `.pane` element it contains reports **865px**. The SplitView's own layout never ran — normal VS Code startup drives that via the full workbench grid/layout machinery, which this spike's minimal `attachPart` call does not invoke. One fix attempt was tried (dispatching a `window` `resize` event) and made no difference; per the 2-attempt budget for obvious wiring mistakes, no further fix-looping was done. This is a real, reproducible caveat for anyone building on this approach: `attachPart` alone gets you a correctly wired, correctly populated view that is invisible until something drives the SplitView's `layout()`.

## 2. What was required

- Existing `@codingame/monaco-vscode-api` version: `25.1.2`.
- `vscode` is already installed as the exact-version alias `npm:@codingame/monaco-vscode-extension-api@25.1.2`; this spike did not need to introduce that alias.
- Added direct dependencies `@codingame/monaco-vscode-views-service-override@25.1.2` and `@codingame/monaco-vscode-scm-service-override@25.1.2`.
- The installed `vscode` alias was sufficient; no new extension-API package was required.
- The preview selects `viewsConfig: { $type: "ViewsService" }`, passes `...getScmServiceOverride()` in the same wrapper configuration, then calls `attachPart(Parts.SIDEBAR_PART, scmHost)`. This works without activity-bar/full-sidebar chrome in the sense that it mounts and populates (see §1); it does **not** paint without a SplitView layout pass, which nothing in this minimal path provides.
- Menu experiment mechanism: `registerExtension(manifest, ExtensionHostKind.LocalProcess, { system: true })`, with one `scm/resourceState/context` item in group `inline` and one `scm/title` item in group `navigation`; commands are registered through `await registration.getApi()`. **Both actions render into the DOM.** Queried by `aria-label`: the title action shows up as two `"Refresh"`-labelled elements (one is the accessible label, one the tooltip host) with group `navigation`; the inline resource action shows up as three `"Stage"`-labelled elements, one per row in the `Changes` (working-tree) group, matching the `when: scmResourceGroup == workingTree` clause. Total action-item count under the SCM host: 7. Because the containing SplitView is 0-height (§1 caveat), these elements are structurally correct but not visually clickable in this spike's current state — a live hover/click test was not attempted for that reason.

### Current shell bootstrap

The current shell uses `MonacoVscodeApiWrapper` with `viewsConfig: { $type: "EditorService" }`. Its explicit overrides are `@codingame/monaco-vscode-files-service-override` and `@codingame/monaco-vscode-keybindings-service-override`; the wrapper also installs its extended highlighting services and its standalone editor service. The `/next` compatibility run is done — see §7.

## 3. Numbers

### Installed payload

The exact required command returned zero because pnpm exposes these entries as symlinks:

```text
$ du -sh node_modules/@codingame/monaco-vscode-views-service-override node_modules/@codingame/monaco-vscode-scm-service-override
  0B node_modules/@codingame/monaco-vscode-views-service-override
  0B node_modules/@codingame/monaco-vscode-scm-service-override
```

Dereferencing those symlinks gives the package payload:

```text
$ du -shL node_modules/@codingame/monaco-vscode-views-service-override node_modules/@codingame/monaco-vscode-scm-service-override node_modules/vscode
 68K node_modules/@codingame/monaco-vscode-views-service-override
620K node_modules/@codingame/monaco-vscode-scm-service-override
 72K node_modules/vscode
```

The two overrides total **688K** installed. `vscode` was already a direct dependency and the views override was already present transitively; pnpm reported `Packages: +1`, so the incremental store addition for this spike was the SCM package.

### Built route chunk delta

Both baseline and spike `pnpm build` commands succeeded. The baseline was built with the spike route temporarily moved out and restored by a shell trap.

```text
$ find build -type f -exec stat -f %z {} + | awk '{s+=$1} END {print s}'
baseline: 30147128 bytes
spike:    30504779 bytes
delta:      357651 bytes (349.3 KiB, uncompressed)

$ find .svelte-kit/output/client -type f -exec stat -f %z {} + | awk '{s+=$1} END {print s}'
baseline: 30259117 bytes
spike:    30622252 bytes
delta:      363135 bytes (354.6 KiB, uncompressed)
```

The spike route node is 5,428 bytes; the rest of the delta is its lazy VS Code service chunks and CSS. The second build completed in 33.47 seconds.

### Renderer memory

Rough numbers — a single headless Chromium renderer process, measured with `ps -o rss= -p <pid>` immediately before clicking "Mount SCM view" and again ~5 seconds after (state reached `"Mounted"`):

```text
before mount: 134,064 KB  (~131 MB)
after mount:  256,112 KB  (~250 MB)
delta:        ~122,048 KB (~119 MB)
```

An `about:blank` navigation in the same Playwright session did not produce a directly comparable number: Chromium recycled the tab into a fresh browser process/profile on that navigation rather than reusing the existing renderer, so "before mount, same page, real DOM present" was used as the baseline instead of a true `about:blank` number. Labeling this rough per the task's own instruction — it is one sample, not an average, and includes whatever else was resident in that renderer (Svelte/Vite HMR client, the app shell).

## 4. Out-of-the-box look

Screenshot: `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike/scm-spike.png` (1710x990 viewport, headless Chromium). It does not look VS Code-dark: the mounted `Source Control` panel renders in VS Code's default **light** theme (an off-white `#f6f6f6`-ish panel background with dark text), because the spike page passes no `userConfiguration` theme override. That panel sits directly against the app shell's dark background (`#111318` from the page's own `<style>` block), producing a hard light-panel-on-dark-app seam that would clash badly if this were shipped as-is. Only the panel header and the placeholder line are actually visible on screen (see the §1 SplitView-height-0 caveat) — the commit box, resource groups, and file rows that are confirmed present in the DOM do not appear in the screenshot.

## 5. Wiring effort to replace `SourceControlPanel`

Preliminary source comparison: an adapter would map `gitService.state.status.files` into SCM resource groups, map `commitMessage`/`commit()` to the SCM input box, and register commands that call `selectFile`, `stagePaths`, `unstagePaths`, `discardPaths`, `refresh`, and remote actions. The existing panel additionally owns worktree-scope selection and read-only enforcement, discard confirmation, stage-all, branch/stash/amend operations, fetch/pull/push enablement explanations, commit timeline/history paging, diffstat, loading/error/status copy, and per-session diff restoration. Final gaps will be stated only after the native SCM view is rendered and exercised.

## 6. Blockers verbatim (historical — resolved on this continuation)

All of the EPERM/daemon failures below were transient to the earlier session's environment. On this continuation, `pnpm dev --port 5201`, `playwright-cli -s=scmspike open ...`, `resize`, `eval`, and `screenshot` all worked without modification — see §1–§4 and §7 for the resulting evidence. Kept verbatim as historical record only, not current status:

The continuation successfully started the preview on port 5201:

```text
$ pnpm dev --port 5201
VITE v8.0.16  ready in 1854 ms
Local: http://127.0.0.1:5201/
```

The required Playwright wrapper first failed because the default npm cache is not writable:

```text
npm error code EPERM
npm error syscall open
npm error path /Users/blackcolours/.npm/_cacache/tmp/02a9567b
npm error errno EPERM
npm error
npm error Your cache folder contains root-owned files, due to a bug in
npm error previous versions of npm which has since been addressed.
```

Retrying with a task-scoped npm cache reached Playwright but failed before browser launch:

```text
Error: EPERM: operation not permitted, mkdir '/Users/blackcolours/Library/Caches/ms-playwright/daemon/81643611a6cd3a9c'
```

The already-installed global CLI failed at the existing daemon directory:

```text
$ playwright-cli -s=scmspike open about:blank
EPERM: operation not permitted, open '/Users/blackcolours/Library/Caches/ms-playwright/daemon/442fc19945d84153/scmspike.session'
```

The required renderer RSS command path is independently blocked by the execution environment:

```text
zsh:5: operation not permitted: ps
```

The original run's port-5199 blocker remains historical evidence only:

```text
$ pnpm dev --port 5199
error when starting dev server:
Error: Port 5199 is already in use
    at httpServerStart (file:///Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike/tauri-svelte-preview/node_modules/.pnpm/vite@8.0.16_@types+node@25.9.2_jiti@2.7.0/node_modules/vite/dist/node/chunks/node.js:10721:10)
    at process.processTicksAndRejections (node:internal/process/task_queues:104:5)
    at async startServer (file:///Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike/tauri-svelte-preview/node_modules/.pnpm/vite@8.0.16_@types+node@25.9.2_jiti@2.7.0/node_modules/vite/dist/node/chunks/node.js:26503:30)
    at async Object.listen (file:///Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike/tauri-svelte-preview/node_modules/.pnpm/vite@8.0.16_@types+node@25.9.2_jiti@2.7.0/node_modules/vite/dist/node/chunks/node.js:26320:4)
    at async CAC.<anonymous> (file:///Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike/tauri-svelte-preview/node_modules/.pnpm/vite@8.0.16_@types+node@25.9.2_jiti@2.7.0/node_modules/vite/dist/node/cli.js:721:3)
ELIFECYCLE Command failed with exit code 1.
```

`lsof -nP -iTCP:5199 -sTCP:LISTEN` identified PID 11064. Its cwd is `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview`, and its `/preview/scm-spike` route returns HTTP 404. It is another worktree's server, so this spike did not stop it.

## 7. Bootstrap-compatibility test (make-or-break)

The real shell's `MonacoVscodeApiWrapper` config lives in `tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts` (around line 372), not in `vscodeServices.ts` — that file only wraps a generic `ensureVscodeServices(config)` caller with retry logic and does not itself construct the config object. The edit was made there instead, in this worktree only:

- Added `import getScmServiceOverride from "@codingame/monaco-vscode-scm-service-override";`
- Changed `viewsConfig: { $type: "EditorService" }` to `viewsConfig: { $type: "ViewsService" }`
- Added `...getScmServiceOverride()` alongside the existing `...getFileServiceOverride()` and `...getKeybindingsServiceOverride()` in `serviceOverrides`.

With that change live (Vite HMR picked it up), a fresh page open of `http://127.0.0.1:5201/next` in the same Playwright session:

- **Shell painted, no blank screen.** `document.title` read `"Assembly"`; `document.body.innerText` showed the full three-pane layout copy (`Sessions`, `Files`, `Resources`, `Usage`) rather than an empty document.
- **No console fatal.** No `console-*.log` artifact was produced for this navigation at all, meaning zero console errors/warnings were captured by Playwright's collector for the whole load.
- **Editor/syntax-highlighting check not reached.** The shell showed `"No session selected. Pick a session in the rail, or resume one to start a terminal."` — per the task's own instruction not to start a live agent session, this was not pushed further. Reported as far as it got: the `ViewsService` + SCM-override config change does not break the shell's initial paint, but whether it breaks (or is compatible with) the actual Monaco editor once a file is open remains unverified.

**Works** for the part that could be tested (shell paints, no fatal error) — **inconclusive** for the editor/syntax-highlighting part, which needs a live session to reach.

**Reverted.** `git -C /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike checkout -- tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts` (the file actually touched, since `vscodeServices.ts` was never edited — see above). `git status --short` afterward shows only this spike's own pre-existing changes (`tauri-svelte-preview/package.json`, `tauri-svelte-preview/pnpm-lock.yaml`, the untracked spike route, this report, and the screenshot) — `csharpLanguageClient.ts` is clean.

## Command receipts

- Fresh setup: `pnpm install --prefer-offline` and `npx svelte-kit sync` exited 0.
- Dependency install: `pnpm add @codingame/monaco-vscode-views-service-override@25.1.2 @codingame/monaco-vscode-scm-service-override@25.1.2` exited 0.
- Full `svelte-check` is not a clean project-wide gate: it reports 16 pre-existing errors and 24 warnings outside the spike route. Filtering its output found no diagnostic under `src/routes/preview/scm-spike`.
- Baseline and spike `pnpm build` runs exited 0.
- Continuation dev server: `pnpm dev --port 5201` reached Vite ready state and was stopped with Ctrl-C after the Playwright failure.
- No `scmspike` browser session or Chrome helper tree was created; cleanup therefore had no browser process to stop.

---

# Round 3 — demoable state

Both known bugs are fixed. The editor works under `ViewsService` and the SCM pane paints, dark. **Zero additional override packages were needed** — both bugs had the same shape: the workbench container was wrong.

## 1. Editor under ViewsService — WORKS, 0 packages added

### The reported error was a symptom, not the cause

The prior round reported `MarkdownRendererService.setDefaultCodeBlockRenderer is not supported` and read it as a missing service override. It is not. Chasing it in `node_modules` first showed the stub and the real implementation:

- Stub that throws: `@codingame/monaco-vscode-api/missing-services.js:7710-7722` (`MarkdownRendererService`, both members assigned `unsupported`).
- Real implementation is registered by `@codingame/monaco-vscode-base-service-override/index.js:69` — **already installed**, so "install the missing package" was never the fix.
- Call site: `vscode/src/vs/editor/standalone/browser/standaloneCodeEditor.js:66`, in the `StandaloneCodeEditor` constructor.

The actual cause is stated in the app's own code, `tauri-svelte-preview/src/lib/shell/editor/vscodeServices.ts:14-19`: creating a Monaco editor before the real services are registered hits the stub. So the MarkdownRenderer error means *service init failed earlier*.

It failed here — `monaco-languageclient/lib/vscode/apiWrapper.js:74-77`:

```js
if (this.apiConfig.viewsConfig.htmlContainer === undefined) {
    this.performErrorHandling(`View Service Type "${viewsConfigType}" requires a HTMLElement.`);
}
```

`ViewsService` **requires** `htmlContainer`; `EditorService` does not. The switch to `ViewsService` left the config without one.

Reproduced verbatim on the smoke page before the fix:

```json
{ "state": "Failed",
  "err": "Error: View Service Type \"ViewsService\" requires a HTMLElement.",
  "flag": "failed" }
```

### The fix — one line

`tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:373`:

```diff
-        viewsConfig: { $type: "EditorService" },
+        viewsConfig: { $type: "ViewsService", htmlContainer: document.body },
```

`document.body` is the equivalent of what `EditorService` mode used implicitly — `initialize(overrides, container = document.body, …)` at `@codingame/monaco-vscode-api/services.js:441`. The container is used for `injectCss(container)` and as the workbench container reference; `injectCss` targets `target.ownerDocument` (`css.js:33-46`), so styles stay document-wide.

**Override packages added to make the editor work: 0.** Well under the 6 cap, and `@codingame/monaco-vscode-workbench-service-override` (the full workbench) is **not** used — the CAP condition was never hit.

### Smoke result — PASS

New page `tauri-svelte-preview/src/routes/preview/editor-smoke/+page.svelte` calls the shell's own `prepareNativeCsharpEditorServices()` (the exact path `csharpLanguageClient.ts:423` exposes), then `monaco.editor.create` on a hardcoded C# string.

| Check | Result |
|---|---|
| Services start | `"state": "Editor created"`, `"flag": "ok"` |
| Editor renders | 15 `.view-line` elements |
| Text visible | `["using System;", "", "namespace Assembly.Smoke", "{"]` |
| **TextMate colors** | **YES** — 67 `span.mtk*` tokens, **10 distinct computed colors** (`rgb(84,185,255)`, `rgb(255,215,0)`, `rgb(218,112,214)`, `rgb(75,243,200)`, …) |
| Console errors | 1, non-fatal (below) |

`/next` after the fix: **0 console errors, 0 warnings** (`Total messages: 2 (Errors: 0, Warnings: 0)`), `document.title === "Assembly"`, full three-pane shell copy present.

`pnpm build` exits 0 — `✓ built in 24.21s`, `Wrote site to "build"`.

### One new non-fatal console error, cause identified

On the smoke page (which loads the curated extensions):

```
TypeError: Cannot read properties of undefined (reading 'extensionId')
    at ViewsExtensionHandler.addViews
```

Source: `@codingame/monaco-vscode-view-common-service-override/…/viewsExtensionPoint.js:470`:

```js
const order = ExtensionIdentifier.equals(extension.description.identifier, container.extensionId)
```

`container` is `viewContainer || this.getDefaultViewContainer()` (line 458). The bundled `npm` default extension contributes `"views": { "explorer": [...] }` (`@codingame/monaco-vscode-npm-default-extension/resources/package.json`), the `explorer` container does not exist, and the default (Explorer) container does not exist either — so `container` is `undefined`.

This only appears in `ViewsService` mode, because the views extension point does not run under `EditorService`. It does not stop the editor. Registering an Explorer view container — which is on the roadmap anyway — would resolve it.

## 2. SCM pane paints, dark — FIXED

### Root cause: CSS scoping, not layout

Attempts, in order (the first two were diagnostics that ruled out the layout hypothesis):

1. **Nudge the container to fire the `ResizeObserver`** installed by `attachPart` (`views-service-override/index.js:94-101`). Result: `{"splitViewH": 0, "compositeH": 0}` — no change.
2. **Force a second `part.layout(360, 900, 0, 0)`**. Result: `{"splitViewH": 0, "compositeH": 0}` — no change.
3. **Nest the attach host inside the `htmlContainer`.** Worked.

Live inspection showed every JS-side value was already correct, which is what ruled out layout:

```json
{ "isVisible": true, "dimension": "360x900", "contentAreaSize": "359x865",
  "activeComposite": "workbench.view.scm", "pvSize": 865, "paneCount": 1, "didLayout": true }
```

The paneview knew it was 865px tall while the DOM measured 0. That is a styling failure. Walking the ancestry found why:

```json
"ancestors": ["DIV|part monaco-workbench-part sidebar pane-composite-part left",
              "DIV|scm-host …", "DIV|stage …", "MAIN|…", "DIV|", "BODY|", "HTML|scm-spike-page"],
"workbenchEls": ["DIV|service-root … monaco-workbench mac web chromium … vs vscode-theme-defaults-themes-light_modern-json"]
```

`monaco-workbench` (and the theme classes) are put on the **`htmlContainer`** — which was a 1px×1px `.service-root`. The SCM part was attached to `.scm-host`, a **sibling**. VS Code's rules are all scoped `.monaco-workbench .part > .content > .composite { height: 100% }`, so the `height: 100%` chain never matched and every element from `.composite.viewlet` down computed to `0px`. `.content` was 865px only because the part's own inline layout set it.

### The fix

Make the attach host a **descendant** of the `htmlContainer`, in `src/routes/preview/scm-spike/+page.svelte`:

```svelte
<div bind:this={workbenchRoot} class="workbench-root">
  <div bind:this={scmHost} class="scm-host" data-testid="scm-host"></div>
</div>
```
```css
.workbench-root { position: relative; width: 100%; height: 100%; }
.scm-host       { position: relative; width: 100%; height: 100%; overflow: hidden; }
```
```diff
- viewsConfig: { $type: 'ViewsService', htmlContainer: serviceRoot },
+ viewsConfig: { $type: 'ViewsService', htmlContainer: workbenchRoot },
```

This is the same shape the library's own `defaultViewsHtml` uses (`monaco-languageclient/lib/vscode/viewsService.js`): one `#workbench-container` with the part hosts as children.

`.split-view-container` height: **0px → 863px**.

### Dark theme — done, via `userConfiguration`

```js
userConfiguration: {
  json: JSON.stringify({ 'workbench.colorTheme': 'Default Dark Modern' })
}
```

Verified by the class list on the workbench root, which flipped from `vs vscode-theme-defaults-themes-light_modern-json` to:

```
vs-dark vscode-theme-defaults-themes-dark_modern-json
```

### What is on screen

Screenshot: **`/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-935-scm-spike/scm-spike-fixed.png`** (headless, viewport verified `"1710x990"`).

`SOURCE CONTROL` title with the refresh + overflow actions (the `scm/title` navigation contribution), the commit message box (`Message (press Cmd+Enter to commit)`), and all three groups with counts — `Merge Changes 1`, `Staged Changes 2`, `Changes 3` — over 10 list rows carrying `~/src` paths, `M`/`A`/`U`/`D` status letters, and strikethrough on the deleted `old.ts`. 2 title actions, 8 action items. This is the real VS Code SCM view, dark.

Remaining console output on the spike page is fixture-specific, not a platform limit: no worker factory is configured on this standalone page (the real shell passes `configureMonacoWorkers`), and `/workspace/assembly` is a fake path that does not resolve.

## 3. Numbers

### Renderer memory (rough)

`ps -o rss=` on the page renderer of a **fresh** headless session, measured in forward order. Chromium keeps spare renderers and RSS is dominated by shared framework mappings, so the **deltas** are the signal, not the absolute values.

| Stage | Page renderer RSS (rough) | JS heap used |
|---|---|---|
| `about:blank` | 107 MB | — |
| `/next` loaded | 117 MB | 39 MB |
| `/preview/scm-spike` mounted | **187 MB** | **77 MB** |

Hosting the SCM view costs roughly **+70 MB renderer RSS and +38 MB JS heap** on top of the shell. A first pass that measured across a reused renderer gave inflated figures (338–477 MB summed across three renderers); those are discarded as polluted.

### Package sizes

`du -shL`, unpacked. **Both were installed in the prior round; this round installed nothing.**

| Package | Size |
|---|---|
| `@codingame/monaco-vscode-views-service-override@25.1.2` | 68K |
| `@codingame/monaco-vscode-scm-service-override@25.1.2` | 620K |

### Override package count

- **5** declared in `package.json`: `files`, `keybindings`, `scm`, `textmate`, `views`.
- **25** present in `node_modules` once transitive dependencies are resolved: base, bulk-edit, configuration, editor, environment, extensions, files, host, keybindings, languages, layout, localization, log, model, monarch, quickaccess, scm, textmate, theme, view-banner, view-common, view-status-bar, view-title-bar, views, workbench.

`workbench-service-override` is present as a transitive dependency (the wrapper dynamic-imports it for `WorkbenchService` mode) but **is not used** by this configuration.

## Verdict

**Worth it.** The feasibility question is answered yes, and more cheaply than the earlier rounds suggested. Both blockers turned out to be the same one-line mistake — not giving the views host a properly-placed, properly-sized workbench container — rather than the open-ended "register another override, hit the next error" chain that the cap was written to guard against. Nothing was installed this round, the full-workbench escape hatch was never needed, the real editor keeps its TextMate colors, `/next` is clean, and the production build passes. The SCM view that paints is genuinely VS Code's: real groups, real counts, real per-resource status and inline menu contributions, themed by a stock VS Code theme id.

The signals pointing the other way are real but bounded, and worth stating plainly. Roughly +70 MB of renderer RSS for one view is not nothing on a desktop app that already carries Monaco, and Explorer/Timeline/Search will not each be free — this number should be re-measured, not extrapolated, as views are added. Adopting `ViewsService` also turns on VS Code's views extension point, which immediately surfaced a crash from a *bundled* default extension contributing into an Explorer container that does not exist; that specific one is harmless and disappears once Explorer is registered, but it is the class of problem to expect — the more of VS Code's contribution model is switched on, the more of VS Code's assumptions about a full workbench must be satisfied. And the CSS-scoping trap is a standing constraint on the design, not a one-off bug: every future view has to be attached *inside* an element carrying `monaco-workbench`, which constrains how freely these panes can be placed inside Assembly's own layout.

The honest recommendation: adopt the views host, and take Explorer next as the second data point — it registers the container that the current console error wants, and it is the view most likely to expose whatever assumption SCM did not. Re-measure memory after it lands before committing to Timeline and Search.
