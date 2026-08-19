# Editor memory spike — passive measurements (T0, part 1)

Date: 2026-08-19. Machine: macOS 26.5.2 (25F84), arm64.

This covers only the **passive** half of the T0 brief: a read-only breakdown of the app
that was already running, the production bundle composition, and the on-disk dependency
weight. The interactive scenarios (brief steps 1–3), the release build (step 4), the
extension-host branch (step 5) and the heap snapshot (step 6) were deliberately **not**
run — the owner was using the app and a second instance corrupts the session database.
Section D is the script for that run.

Nothing here **proves** what the editor costs. It narrows where to look, and it produced
two unplanned but useful results: the reload finding (A.6) and the cold-start baseline
(A.7).

---

## A. Live process breakdown (read-only)

### A.1 Which processes

The app was already running as a **debug** build, launched 06:47:59, measured from 08:45.

| role | pid | how identified |
|---|---|---|
| App main | 26605 | `pgrep -fl mac-command-bar-webview-preview` → `26605 target/debug/mac-command-bar-webview-preview` |
| WebKit GPU helper | 26654 | `ps -axo pid,ppid,rss,comm \| grep -i WebKit`, started 06:48:00 |
| WebKit Networking helper | 26655 | same; `lsof -p 26655` has 16 open paths under `mac-command-bar-webview-preview` |
| WebKit **WebContent** helper | 26656 | same; `lsof -p 26656` shows `~/Library/Caches/mac-command-bar-webview-preview/WebKit/NetworkCache/…` |

`launchctl procinfo <pid>` printed nothing for every pid (needs root), so the helpers were
matched the way the app itself matches them (`src-tauri/src/resources.rs:661-679`:
helper start time ≥ app start time, and the responsible pid must agree). All three helpers
started at 06:48:00, one second after the app — and three other WebKit helper triples on
the box (pids 1601/1603/1604, 6201/6202/6203, 73543/73544/73545) belong to other apps and
were excluded. **Verified.**

Note on method: `ps` RSS is misleading here — WebContent showed `RSS 44 MB` while its real
`phys_footprint` was 307 MB, because ~195 MB of it is compressed. Every number below is
`phys_footprint` from `footprint -p <pid>`, which is what Activity Monitor's "Memory"
column shows.

### A.2 Three samples, 60 s apart — footprint in MB

Command per cell: `footprint -p <pid>` (the `phys_footprint` line).

| process | pid | S1 08:45:44 | S2 08:46:48 | S3 08:47:51 | peak so far |
|---|---|---|---|---|---|
| App main | 26605 | 41 | 41 | 41 | 44 |
| WebKit GPU | 26654 | 17 | 85 | 85 | 134 |
| WebKit Networking | 26655 | 28 | 28 | 28 | 39 |
| **WebKit WebContent** | 26656 | **307** | **309** | **313** | **470** |
| **total** | | **393** | **463** | **467** | **687** |

### A.3 WebContent (26656) by category — MB, dirty

Command per column: `footprint -p 26656`.

| category | S1 | S2 | S3 | post-reload 08:52 |
|---|---|---|---|---|
| WebKit malloc | 225 | 228 | **231** | **287** |
| Owned physical footprint (unmapped) (graphics) | 57 | 57 | 57 | 64 |
| MALLOC_SMALL | 8.8 | 8.8 | 8.8 | 16 |
| JS VM Gigacage | 6.8 | 6.8 | 6.8 | 7.1 |
| JS JIT generated code | 2.1 | 2.1 | 2.1 | 5.3 |
| untagged (VM_ALLOCATE) | 2.3 | 2.3 | 2.3 | 2.3 |
| everything else (sum) | ~5 | ~5 | ~5 | ~5 |
| **phys_footprint total** | **307** | **309** | **313** | **387** |

Two things to read carefully here:

- **"WebKit malloc" is 74% of the process and it is not the JS-tagged memory.** The
  regions the kernel tags as JavaScript — `JS VM Gigacage` (6.8 MB) plus `JS JIT generated
  code` (2.1 MB) — total **8.9 MB**. WebKit routes JavaScriptCore's garbage-collected
  object heap *and* all of WebCore's DOM/layout/render allocations through bmalloc, which
  is what shows up as `WebKit malloc`. So 231 MB is "JS objects + DOM + render tree
  together", not a pure JS heap number. Splitting those two apart needs the Safari heap
  snapshot (brief step 6), which was not run.
- **It grew while completely idle: 225 → 228 → 231 MB over 2 minutes, about +3 MB/min**,
  with nobody touching the app. `vmmap --summary 26656` shows the allocation count in the
  malloc zones climbing in step: 625,426 → 632,111 → 638,796 (+~6,700/min).

### A.4 `vmmap --summary 26656`, selected rows (S1 / S2 / S3)

| region | VIRTUAL | RESIDENT | DIRTY | SWAPPED |
|---|---|---|---|---|
| WebKit Malloc | 529.2M | 30.1M / 34.7M / 36.1M | 29.8M / 34.4M / 34.6M | **194.8M / 193.1M / 196.0M** |
| JS VM Gigacage | 2.0G (+66G reserved) | 448K / 464K / 480K | same | 6336K / 6320K / 6304K |
| JS JIT generated code | 512.0M | 1344K / 1344K / 1312K | same | 768K / 768K / 816K |
| owned unmapped (graphics) | 143.9M | 56.6M / 71.6M / 56.6M | 56.6M | 0 |
| `__TEXT` | 838.3M | 231.8M / 234.1M / 234.4M | 0 | 0 |
| `__LINKEDIT` | 573.0M | 30.3M / 25.3M / 25.3M | 0 | 0 |

`__TEXT` at ~234 MB resident is WebKit's own framework code — clean, file-backed, shared
with every other WebKit process on the machine, and **not** counted in `phys_footprint`.
It is not the app's cost and no editor choice changes it.

### A.5 What the owner had open

Could not be established without touching the app, and it is **not** inferable from these
numbers. What can be said: the WebContent peak of **470 MB** means the process has been
substantially larger at some point in this 2-hour session than the 307–313 MB it was
sitting at, and ~195 MB of its 231 MB `WebKit malloc` was compressed out to swap — the
profile of a large heap that has gone cold, not of a heap actively in use. The WebKit
network cache on disk holds 1,154 blobs containing the string `monaco` and 233 containing
`vs/editor` (`grep -rl` over `~/Library/Caches/mac-command-bar-webview-preview/WebKit/
NetworkCache/Version 17/Blobs`), but that cache is cumulative across app runs and is
**not** evidence about this session. The two largest cached blobs are 43 MB each and are
the Microsoft-copyright TypeScript compiler payload.

### A.6 A hot reload during the run — the most interesting number in this document

Between 08:47:51 and 08:52:07 the app hot-reloaded. **What triggered it cannot be pinned
down**, and I will not claim otherwise: my `pnpm build` (section B) ran at 08:48:11–08:48:47
and regenerates `.svelte-kit/generated`, which the dev server on 127.0.0.1:5187 watches —
but several other lanes were editing Svelte files in this same worktree throughout, and
each of those edits triggers the same hot reload. Either could be the cause. No second
build was run regardless.

Whatever triggered it, the reload is a clean before/after on one process:

| | before (S3, 08:47:51) | after (08:52:07) | delta |
|---|---|---|---|
| WebContent phys_footprint | 313 MB | **387 MB** | **+74 MB** |
| WebContent peak | 470 MB | **587 MB** | **+117 MB** |
| WebKit malloc | 231 MB | 287 MB | +56 MB |
| JS JIT generated code | 2.1 MB | 5.3 MB | +3.2 MB |
| WebKit GPU helper | 85 MB | 91 MB | +6 MB |

Three and a half minutes later none of it had come back. **One reload of this frontend
costs ~75 MB of resident footprint and ~117 MB of peak, and the previous document's memory
is not reclaimed.** Caveats: a dev-server hot reload is not the same event as a user
action, and one observation is not a trend. This is exactly what D.2's added reload
scenario is for.

### A.7 The instance was replaced twice — a fresh-start baseline, and a warning

The app under measurement restarted twice in ten minutes:

| time | app pid | what happened |
|---|---|---|
| 06:47:59 | 26605 | the instance sections A.2–A.6 measure |
| 08:55:00 | 51742 | `tauri dev` supervisor (pid 26273) rebuilt and relaunched |
| 08:57:0x | 54729 | rebuilt and relaunched again |
| by 08:58:36 | *(none)* | no instance running; supervisor and dev server still up |

The trigger is other lanes editing Rust in this shared worktree — `git status` shows
`src-tauri/src/agent_conversation/manager.rs` and
`src-tauri/src/agent_conversation/providers/process.rs` modified — which the `tauri dev`
watcher rebuilds and relaunches on. Only ever one instance ran
(`pgrep -fl mac-command-bar-webview-preview` returned a single pid at every check) and no
second instance was started by this work.

The replacements are useful anyway: they are **freshly launched apps**, same sampler, same
on-disk session state.

| | pid 51742 @08:56, ~1 min old | pid 54729 @08:57, ~30 s | pid 54729 @08:57:44 | pid 26605 @08:47, ~2 h |
|---|---|---|---|---|
| WebContent footprint | 235 MB | 175 MB | **157 MB** | **313 MB** |
| WebContent peak | 323 MB | 254 MB | 254 MB | 470 MB |
| WebKit malloc | 181 MB | 124 MB | 106 MB | 231 MB |
| graphics (owned unmapped) | 37 MB | 37 MB | 37 MB | 57 MB |
| JS VM Gigacage | 2.9 MB | 1.5 MB | 1.5 MB | 6.8 MB |
| JS JIT generated code | 3.8 MB | 2.8 MB | 2.8 MB | 2.1 MB |
| App main | 37 MB | 38 MB | 38 MB | 41 MB |

Two readings, both tentative on this few samples. **A cold start is already ~157–235 MB in
WebContent before any editor is opened** — most of the app's memory is there before Monaco
is asked to do anything. And a fresh instance **settles downward** (175 → 157 MB in 16 s)
as startup allocations are freed, which is the opposite of the +3 MB/min climb the 2-hour
instance showed in A.3. That contrast — new process shrinks, old process grows — is what
the D.2 reload scenario is meant to pin down.

A planned 9-sample growth series at 30 s intervals could not be completed: the app was
restarted out from under it and then stayed down. **Longitudinal memory measurement is not
possible in this worktree while other lanes are writing to it**, which is why D.0 now leads
with quiescing them.

---

## B. Bundle composition

`pnpm build` (vite 8.0.16, SvelteKit + `adapter-static`, output `build/`). Exit code 0,
"✓ built in 13.45s". `build/` (`.gitignore:9`) and `.svelte-kit/` (`.gitignore:6`) are both
gitignored, so this left the repository clean.

| category | total | monaco / vscode / codingame | share | everything else |
|---|---|---|---|---|
| JS | 22.92 MB | **20.17 MB** | **88.0%** | 2.74 MB |
| CSS | 0.83 MB | 0.27 MB | 32.3% | 0.56 MB |
| other assets (grammars, wasm, fonts) | 4.99 MB | 4.47 MB | 89.8% | 0.51 MB |
| **all** | **28.73 MB** | **24.92 MB** | **86.7%** | 3.81 MB |

Raw bytes: total 30,122,811; monaco/vscode 26,126,014; other 3,996,797. JS only: total
24,029,125; monaco/vscode 21,154,235; other 2,874,890.

**How chunks were attributed.** Production chunk names are hashed and minification strips
module paths, so filename and identifier heuristics do not work (a first attempt scored the
6.8 MB TypeScript worker as "app code"). Each file was classified by (a) the
`.svelte-kit/output/client/.vite/manifest.json` source path when the manifest names one,
and (b) string literals that survive minification — `vs/editor|base|platform|workbench/`
module-path literals, `editor.action.*` and `workbench.*` command ids, TypeScript compiler
diagnostic text, `MonacoEnvironment`/`IStandaloneCodeEditor`, `vscode-uri|textmate|
oniguruma`. Six large chunks the fingerprint missed were read directly and classified by
hand (listed below). This is a careful attribution, not an exact one — a mixed chunk counts
wholly to one side.

### Largest 15 files

| size | class | file |
|---|---|---|
| 6819K | monaco/vscode | `_app/immutable/workers/ts.worker-CGEMzsuR.js` |
| 2196K | monaco/vscode | `_app/immutable/chunks/qjNfNQMh.js` |
| 1942K | monaco/vscode | `_app/immutable/chunks/DOpd91vJ.js` |
| 1695K | monaco/vscode | `_app/immutable/workers/extensionHost.worker-C4HmvSMr.js` |
| 1342K | monaco/vscode | `_app/immutable/chunks/DiSHbXe02.js` |
| 939K | monaco/vscode | `_app/immutable/chunks/1PpjM7sw.js` |
| 938K | monaco/vscode | `_app/immutable/chunks/DwTb-qXl.js` |
| 871K | monaco/vscode | `_app/immutable/chunks/46VC-Wyz.js` |
| 548K | monaco/vscode | `_app/immutable/assets/platform.tmLanguage.BtxvkeXe.json` |
| 485K | monaco/vscode | `_app/immutable/workers/json.worker-DeDyREUx.js` |
| 473K | monaco/vscode | `_app/immutable/assets/cuda-cpp.tmLanguage.CLki45UZ.json` |
| 459K | app/other | `_app/immutable/nodes/3.gk-7mrPY.js` (a Svelte route page) |
| 456K | monaco/vscode | `_app/immutable/assets/onig.Du5pRr7Y.wasm` |
| 439K | app/other | `_app/immutable/nodes/2.BMtRiwLa.js` (a Svelte route page) |
| 412K | monaco/vscode | `_app/immutable/assets/cpp.tmLanguage.C7xWZ90S.json` |

Hand-classified after reading the file contents:

| size | file | what it is |
|---|---|---|
| 358K | `workers/editor.worker-BgQFvTF1.js` | monaco `TextModel` worker → monaco |
| 333K | `workers/chunks/7VEr93PG.js` | jschardet, worker copy → vscode |
| 333K | `chunks/BAyiS2gQ2.js` | jschardet, main-thread copy → vscode |
| 297K | `workers/chunks/Cxw6X6_M.js` | iconv-lite-umd, worker copy → vscode |
| 297K | `chunks/v_TcpS82.js` | iconv-lite-umd, main-thread copy → vscode |
| 252K | `workers/worker-BWRFFbYs.js` | monaco `LineTokens` worker → monaco |
| 332K | `chunks/DooSxjI52.js` | xterm (94 `xterm` literals) → app/other |
| 276K | `chunks/BviiPFBE2.js` | dockview (`DockviewComponent`) → app/other |

Two details worth naming. **The TypeScript compiler is 6.8 MB of the 22.9 MB of JS**, one
file, shipped so Monaco can offer TS/JS language features. And **jschardet and iconv-lite
are bundled twice** — once for the main thread and once for the extension-host worker —
1.26 MB of duplicated character-encoding code that exists because vscode wants to guess
file encodings.

---

## C. Dependency weight on disk

pnpm symlinks, so `du -sh node_modules/@codingame` reports `0B`; the real sizes come from
the `node_modules/.pnpm` store.

| measure | value | command |
|---|---|---|
| `@codingame/*` on disk | **88 MB** | `du -ch node_modules/.pnpm/@codingame+* \| tail -1` |
| of which `monaco-vscode-api` alone | 40 MB | `du -sh node_modules/.pnpm/@codingame+monaco-vscode-api@25.1.2` |
| of which `standalone-typescript-language-features` | 12 MB | same pattern |
| `monaco-editor` package | 648 KB | it is an **alias**: `package.json:173` reads `"monaco-editor": "npm:@codingame/monaco-vscode-editor-api@25.1.2"` |
| `@codingame` packages, total | 34 | `ls node_modules/@codingame \| wc -l` |
| of which `*-default-extension` | **27** | `ls node_modules/@codingame \| grep -c -- "-default-extension"` |
| `monaco-vscode-language-pack-*` in the store | 14 | `ls node_modules/.pnpm \| grep -c language-pack` (transitive; ru/ja/it/fr/es/de/tr/qps-ploc and more, ~1.5–2.2 MB each) |
| `.wasm` files under `@codingame` | **1** — `onig.wasm`, 456 KB | `find -L node_modules/@codingame -name "*.wasm"` |
| `.tmLanguage.json` grammars | **46** | `find -L node_modules/@codingame -name "*.tmLanguage.json" \| wc -l` |
| largest grammars | `platform.tmLanguage.json` 552K, `cuda-cpp` 476K, `cpp` 412K — all C++ | `find -L … \| xargs du -h \| sort -rh` |
| whole `node_modules` | 364 MB | `du -sh node_modules` |

`find` without `-L` reports zero for all of these, because it will not follow pnpm's
symlinks — worth knowing before anyone re-runs this.

---

## What this suggests (suggests — the interactive scenarios decide)

**The bundle is overwhelmingly Monaco's, and that part is settled: 88% of shipped
JavaScript and 87% of all shipped bytes are monaco/vscode/codingame.** Removing the
`monaco-vscode-api` layer would be a very large download and parse reduction. That is a
measured fact about the bundle, and it is the strongest number in this document.

**Whether that bundle is what the owner's 1.3–1.6 GB is made of, these measurements do not
say, and three observations argue it is not the whole story.** First, the WebContent
process sat at 307–313 MB with a 470 MB peak after two hours of real use — a fifth of what
the owner reported — so the 1.3–1.6 GB state is something this session had not reached.
Second, that process grew ~3 MB/min while nothing was happening, and a hot reload added
74 MB of footprint and 117 MB of peak that were still not reclaimed minutes later; an
accumulation of ~75–120 MB per reload across a long session reaches 1.3–1.6 GB without any
editor being involved. Third, a **cold start is already 157–235 MB** with no editor open,
so the editor cannot be responsible for the larger part of even the baseline. Bundle size
and retained memory are different problems, and the evidence so far points at retention at
least as much as at Monaco's size.

**The category split argues the same way.** The JS-tagged regions are 8.9 MB. The 231 MB of
`WebKit malloc` mixes the JavaScriptCore object heap with WebCore's DOM and render tree,
and nothing measured here separates them. Choosing an editor on this data would be choosing
on the bundle number alone while the 231 MB — the number that actually decides whether the
app hits 1.3 GB — is still unattributed.

**So this document does not recommend among the three options yet, and the brief's step 7
recommendation should not be written until steps 1–3 and step 6 are done.** What it does
change is the shape of the remaining work: the heap snapshot (step 6) is now the highest-
value step, not the last one, because it is the only measurement that splits `WebKit
malloc` into "Monaco's editor state" and "our transcript DOM". And one scenario should be
added to the brief: **reload the app 5 times with the editor closed and sample after each**.
If footprint climbs ~75 MB per reload with no editor open, the editor is not the defect
being chased, and T24–T26 should not be scoped as a memory fix.

Two smaller findings that need no further measurement. 27 `*-default-extension` packages
are installed, of which C++ alone contributes the three largest grammars (1.4 MB); the
project is a C#/TypeScript workbench and most of those 27 are dead weight in every build.
And `monaco-editor` is an npm alias onto `@codingame/monaco-vscode-editor-api`
(`package.json:173`), so the option "Monaco without the vscode-api layer" is not a matter
of deleting imports — the real `monaco-editor` package is not installed at all and would
have to be added back.

---

## D. Script for the interactive run

Run this when the owner is not using the app. It replaces the brief's steps 1–4 and takes
about 15 minutes of wall time, most of it the step-3 idle wait.

### D.0 Before anything — stop every other lane in this worktree

This is the precondition that matters most, and it is not in the brief. While the passive
measurements were being taken, the app restarted **twice in ten minutes** (26605 → 51742 at
08:55, → 54729 at 08:57) because other lanes were editing Rust under `src-tauri/src/` and
the `tauri dev` watcher rebuilds and relaunches on every such edit. Frontend edits do the
same thing more cheaply, as a hot reload. **A 10-minute idle scenario cannot be measured in
a worktree where other agents are writing files.**

```sh
# 1. No other lane may be running against this worktree. Confirm the tree is quiet:
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
git status --short          # note it, then re-run after 60 s — it must be identical
# 2. The owner's app must be closed. Confirm exactly zero instances:
pgrep -fl mac-command-bar-webview-preview        # must print nothing
# 3. A dev server may still be up; leave it, it is fine:
lsof -nP -iTCP:5187 -sTCP:LISTEN
```

During the run, re-check `pgrep -fl mac-command-bar-webview-preview` at each sample: the
sampler records the pid in every row precisely so a restart is visible. **If the pid changes
mid-scenario, discard that scenario and start it again.**

**Do not run `pnpm build` or `pnpm exec tauri build` while an app instance is open** — it
rewrites `.svelte-kit/generated`, the dev server hot-reloads the window, and the
measurement is destroyed (see A.6).

### D.1 Install the sampler (paste once, writes to /private/tmp, touches no repo file)

This script was written and **run against the live app** while preparing this document, so
it is known to work — the numbers in A.7 came out of it.

```sh
cat > /private/tmp/t0-sample.sh <<'SH'
#!/bin/bash
# usage: t0-sample.sh <label>   -> appends one row per process to /private/tmp/t0-memory.csv
LABEL="$1"
OUT=/private/tmp/t0-memory.csv
APP=$(pgrep -f "mac-command-bar-webview-preview" | head -1)
[ -z "$APP" ] && { echo "no app instance running"; exit 1; }
APP_START=$(ps -o lstart= -p "$APP")
APP_EPOCH=$(date -j -f '%a %b %e %T %Y' "$APP_START" +%s 2>/dev/null)
[ -f "$OUT" ] || echo "label,time,role,pid,footprint_mb,peak_mb,webkit_malloc_mb,graphics_mb,js_gigacage_mb,js_jit_mb" > "$OUT"
row() {
  local role=$1 pid=$2
  local f
  f=$(footprint -p "$pid" 2>/dev/null) || return
  [ -z "$f" ] && return
  # every value normalised to MB
  local vals
  vals=$(echo "$f" | awk '
    function mb(n,u){ if(u=="KB")return n/1024; if(u=="GB")return n*1024; if(u=="B")return n/1048576; return n }
    /^ *phys_footprint:/      {fp=$2}
    /^ *phys_footprint_peak:/ {pk=$2}
    /WebKit malloc$/                    {wm=mb($1,$2)}
    /unmapped\) \(graphics\)$/          {gx=mb($1,$2)}
    /JS VM Gigacage$/                   {gg=mb($1,$2)}
    /JS JIT generated code$/            {jj=mb($1,$2)}
    END{printf "%s,%s,%.1f,%.1f,%.1f,%.1f", fp+0, pk+0, wm+0, gx+0, gg+0, jj+0}')
  echo "$LABEL,$(date '+%H:%M:%S'),$role,$pid,$vals" >> "$OUT"
}
row app "$APP"
# WebKit helpers started at or after the app (src-tauri/src/resources.rs:661-679)
for p in $(pgrep -f "com.apple.WebKit"); do
  s=$(ps -o lstart= -p "$p" 2>/dev/null) || continue
  e=$(date -j -f '%a %b %e %T %Y' "$s" +%s 2>/dev/null) || continue
  [ -n "$e" ] && [ -n "$APP_EPOCH" ] && [ "$e" -ge "$APP_EPOCH" ] || continue
  n=$(ps -o comm= -p "$p" | sed 's#.*/##' | sed 's/com.apple.WebKit.//')
  row "$n" "$p"
done
column -s, -t "$OUT" | tail -6
SH
chmod +x /private/tmp/t0-sample.sh
rm -f /private/tmp/t0-memory.csv
```

Every `t0-sample.sh <label>` below writes one labelled row per process. At the end,
`/private/tmp/t0-memory.csv` is the brief's step-7 table with no transcription by hand.

### D.2 Steps 1–3, debug build (about 14 min)

```sh
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview
./scripts/dev-next.sh            # leave running in its own terminal; wait for the window
pgrep -fl mac-command-bar-webview-preview   # must show exactly ONE pid
```

Then, in a second terminal — the app is at one session open, **editor tab not opened**:

```sh
# STEP 1  baseline, editor closed        (3 min)
/private/tmp/t0-sample.sh baseline-1; sleep 60
/private/tmp/t0-sample.sh baseline-2; sleep 60
/private/tmp/t0-sample.sh baseline-3

# --- in the app: open ONE .cs file ---
# STEP 2a  one C# file                    (3 min)
/private/tmp/t0-sample.sh cs-1; sleep 60
/private/tmp/t0-sample.sh cs-2; sleep 60
/private/tmp/t0-sample.sh cs-3

# --- in the app: open ONE .html file as well ---
# STEP 2b  C# + HTML                      (3 min)
/private/tmp/t0-sample.sh cs-html-1; sleep 60
/private/tmp/t0-sample.sh cs-html-2; sleep 60
/private/tmp/t0-sample.sh cs-html-3

# --- in the app: close the editor tab ---
# STEP 2c  does it come back?             (3 min)
/private/tmp/t0-sample.sh editor-closed-1; sleep 60
/private/tmp/t0-sample.sh editor-closed-2; sleep 60
/private/tmp/t0-sample.sh editor-closed-3

# --- in the app: select the longest Codex session, then do not touch it ---
# STEP 3  long transcript idle            (10 min)
for i in 1 2 3 4 5 6 7 8 9 10; do /private/tmp/t0-sample.sh "idle-$i"; sleep 60; done
```

**Add this — it is not in the brief and A.6 says it matters most** (about 5 min):

```sh
# STEP 3b  reload accumulation, editor CLOSED. Cmd-R in the app window between samples.
/private/tmp/t0-sample.sh reload-0
# reload the window, wait ~20 s for it to settle, then:
/private/tmp/t0-sample.sh reload-1
# repeat four more times: reload, wait, sample as reload-2 … reload-5
```

If footprint climbs per reload with no editor open, the editor is not the cause of the
1.3–1.6 GB and T24–T26 must not be scoped as a memory fix.

Then quit the app and confirm: `pgrep -fl mac-command-bar-webview-preview` prints nothing.

### D.3 Step 4, release build

The brief's `cargo build --release` alone is **not enough**, for two reasons found while
reading the configs. `cargo build` does not run `beforeBuildCommand`, so `build/` must
already exist. And the release window opens `frontendDist` at `/`
(`src-tauri/tauri.conf.json` → `"frontendDist": "../build"`, no window `url` set), which is
the **old** shell — `/next` is only reached in dev because `tauri.dev.next.conf.json` sets
`devUrl` to `http://127.0.0.1:5177/next`. Measuring `/` would measure the wrong app.
`adapter-static` is configured with `fallback: 'index.html'` (`svelte.config.js`), so `/next`
does resolve at runtime; the start URL just has to be set. One command does all of it:

```sh
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview
pnpm exec tauri build --no-bundle \
  --config '{"app":{"windows":[{"label":"main","url":"/next"}]}}'
```

`--no-bundle` skips the .dmg/.app packaging; both flags are present in this CLI (verified
with `pnpm exec tauri build --help`). It runs `pnpm build` first via `beforeBuildCommand`
and leaves the binary at `src-tauri/target/release/mac-command-bar-webview-preview`. Then
run it with the environment `scripts/dev-next.sh` exports — the ACP adapter paths, without
which every structured session fails to activate (`scripts/dev-next.sh:8-130`):

```sh
export MCB_ALLOW_KEYCHAIN_CREDENTIALS=1
export MCB_CLAUDE_AGENT_ACP_PATH="$HOME/.mac-command-bar/claude-acp-wrapper.sh"
export MCB_CLAUDE_AGENT_ACP_SHA256="$(shasum -a 256 "$MCB_CLAUDE_AGENT_ACP_PATH" | awk '{print $1}')"
export MCB_CODEX_ACP_PATH="$HOME/.mac-command-bar/codex-acp-bridge.sh"
export MCB_CODEX_ACP_SHA256="$(shasum -a 256 "$MCB_CODEX_ACP_PATH" | awk '{print $1}')"
# optional, only if the Antigravity adapter is built:
[ -x "$HOME/.mac-command-bar/agy-acp-wrapper.sh" ] && {
  export MCB_AGY_ACP_PATH="$HOME/.mac-command-bar/agy-acp-wrapper.sh"
  export MCB_AGY_ACP_SHA256="$(shasum -a 256 "$MCB_AGY_ACP_PATH" | awk '{print $1}')"
}
./src-tauri/target/release/mac-command-bar-webview-preview
```

Those three wrapper scripts are written by `scripts/dev-next.sh` on every dev launch, so
they already exist on this machine; if any is missing, run `./scripts/dev-next.sh` once and
quit it. Then repeat D.2 steps 1 and 2 with labels prefixed `rel-`:

```sh
/private/tmp/t0-sample.sh rel-baseline-1; sleep 60
/private/tmp/t0-sample.sh rel-baseline-2; sleep 60
/private/tmp/t0-sample.sh rel-baseline-3
# --- open one .cs file ---
/private/tmp/t0-sample.sh rel-cs-1; sleep 60
/private/tmp/t0-sample.sh rel-cs-2; sleep 60
/private/tmp/t0-sample.sh rel-cs-3
```

### D.4 Step 6, heap snapshot — do this one, it is the deciding measurement

With the editor loaded (right after the `cs-3` sample): Safari → Develop → the app's
WebContent → Timelines → Memory → take a heap snapshot, sort by retained size, and record
the top 10 class names with bytes. This is the only step that splits the 231 MB of
`WebKit malloc` into Monaco's editor state versus the transcript DOM, and A.5 shows every
other measurement leaves that unattributed.

### D.5 Finish

```sh
column -s, -t /private/tmp/t0-memory.csv          # the step-7 table
git branch -D spike/extension-host-cost 2>/dev/null   # after step 5, never merge it
pgrep -fl mac-command-bar-webview-preview             # must print nothing
```
