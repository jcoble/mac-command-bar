# Reactive cycle bug hunt

The freeze has one verified reactive cycle. The cycle is the retained-draft callback inside Monaco's preview effect; the new session restore path in `c980f59` makes it reliably reachable, and restored-view-state consumption supplies the first re-run that enters the loop.

The smallest fix direction is to prevent `applyPreview()` from calling `onContentChange` when the parent already holds the model's text—for example, guard the call at `MonacoSourceEditor.svelte:1952` with `content !== model.getValue()`. No code was changed.

## Scope and evidence

- **[verified]** `git show --stat c980f59` succeeded and listed six touched files: `tauri-svelte-preview/scripts/sessionWorkspaces.test.ts`, `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte`, `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte`, `tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts`, `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts`, and `tauri-svelte-preview/src/routes/next/+page.svelte`. The runtime editor paths are therefore `src/lib/MonacoSourceEditor.svelte` and `src/lib/shell/components/EditorPanel.svelte`; the tentative nested paths do not exist in this change.
- **[verified]** The test file contains no `$state`, `$derived`, `$effect`, or store subscription and cannot be the runtime cycle (`tauri-svelte-preview/scripts/sessionWorkspaces.test.ts:1`).
- **[verified]** `sessionWorkspaces.ts` is pure capture/normalization/planning code and contains no runes or subscriptions (`tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts:158-199`, `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts:471-487`).
- **[verified]** The installed Svelte runtime throws this exact error after repeated effect invalidation (`tauri-svelte-preview/node_modules/.pnpm/svelte@5.56.3/node_modules/svelte/src/internal/client/errors.js:239-247`). A read-only probe using that installed runtime confirmed that a synchronous callback invoked by an effect remains inside the effect's dependency tracking: the callback read and rewrote one source, producing the same update-depth error after 1,001 runs.

## Ranked candidates

### 1. Retained draft writes back through the effect that is applying it — verified-cycle

**Reactive read.** **[verified]** Monaco's `$effect` calls `applyPreview()` (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2974-2977`). `applyPreview()` reads the reactive `preview` and `content` props (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1933-1937`) and detects a retained draft whenever the live model differs from the last disk content (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1944-1946`).

**Reactive write.** **[verified]** In that retained-draft branch, the effect synchronously calls `onContentChange(model.getValue())` (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1951-1952`). The bound callback reads the active editor file (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:747-750`), which reads `editorState.activePath` and `editorState.openFiles` (`tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:73-80`), then rewrites `editorState.openFiles` (`tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:129-135`). `patchOpenFile` always constructs a new array and a new active-file object even when the text and dirty flag are unchanged (`tauri-svelte-preview/src/lib/shell/editor/editorStoreOps.ts:101-106`). Because those reads happen synchronously under the Monaco effect, the store is in that effect's dependency set; the write schedules it again.

**Trigger sequence.** **[verified]** The sequence introduced or made reachable by `c980f59` is:

1. A session switch snapshots dirty text and Monaco view state (`tauri-svelte-preview/src/routes/next/+page.svelte:782-799`; `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts:167-185`).
2. The switch releases the departing models, then restores the arriving session's lightweight file descriptors and eagerly requests its active file (`tauri-svelte-preview/src/routes/next/+page.svelte:906-915`; `tauri-svelte-preview/src/routes/next/+page.svelte:848-860`).
3. `restoreEditorFiles` reinstates `draftContent` before the disk preview exists (`tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:164-173`). When disk content arrives, `setEditorFilePreview` deliberately retains that draft (`tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:116-125`).
4. The first `applyPreview()` creates a model from the draft (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1936-1949`) but records the disk preview as its saved value (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1980`; `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1450-1452`). Thus the model is immediately classified as a retained draft on the next effect run.
5. If a saved view state exists, that same first run reads it and synchronously consumes it (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2001-2004`), deleting reactive parent state (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:872-873`). That mutation schedules the next preview-effect run.
6. On the next run, `retainedDraft` remains true, so line 1952 calls the parent callback. The callback reads and rewrites `editorState.openFiles`; the new run sees the same retained draft and repeats indefinitely (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1944-1952`; `tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:129-135`).

**Why `c980f59` is implicated.** **[verified]** The raw Monaco retained-draft callback predates `c980f59`, but `c980f59` added descriptor-based draft restoration, retained drafts on disk arrival, model release/recreation, and restored-view-state consumption (`tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:116-125`, `tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:164-173`, `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:862-892`). Those changes assemble the full trigger matching repeated session switches with open editors.

**Fix direction.** **[verified direction, not implemented]** Make the write at `MonacoSourceEditor.svelte:1952` conditional on the parent content actually being stale, such as `content !== model.getValue()`, or remove that callback now that disk arrival preserves the draft in the store (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1951-1952`; `tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:116-125`). The one-line equality guard is the smallest change.

### 2. Restored view state is read and deleted by the same effect — unlikely as a standalone cycle; verified amplifier

**Read and write.** **[verified]** `applyPreview()` reads `restoredViewStates[preview.path]` and calls the consume callback (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2001-2004`); the callback deletes that exact reactive property (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:872-873`).

**Why it does not loop alone.** **[verified]** Before consuming the state, `applyPreview()` assigns `currentPath = preview.path` (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1997-2000`). On the re-run, `pathChanged` is false (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1975`, `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1997`), so the consume branch is skipped. It is a bounded one-run invalidation, but it is the direct kick that enters candidate 1 when the restored file has a draft.

**Fix direction.** **[verified direction, not implemented]** Do not change this first; removing or guarding the retained-draft write at `MonacoSourceEditor.svelte:1952` breaks the actual loop while preserving one-time view-state consumption (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2001-2004`).

### 3. Generation counters — unlikely

**[verified]** Both generation counters are plain local variables, not `$state`: Monaco declares its counter at `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:389` and increments it only in the exported release function at `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1431-1433`; EditorPanel declares its counter at `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:152` and increments it only in its exported release function at `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:876-878`. Neither increment occurs inside an effect, so neither can invalidate a Svelte effect.

### 4. EditorPanel state-copy effects — unlikely

- **[verified]** The intelligence-sync effect reads editor store fields (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:704-708`) but writes only plain fields inside the non-reactive `sourceIntelligence` service (`tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:1204-1219`). There is no reactive return edge.
- **[verified]** The language-bar effect reads panel state (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:959-967`) and writes a separate mirror store (`tauri-svelte-preview/src/lib/shell/editor/languageIntelligenceBar.svelte.ts:48-60`). That mirror's switch handler is invoked only by a user change (`tauri-svelte-preview/src/lib/shell/editor/languageIntelligenceBar.svelte.ts:64-72`), so publishing does not write any dependency of the publishing effect.
- **[verified]** The lazy-editor effect reads only `showing` and `editorState.activePath` (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:947-949`). `ensureCodeEditor` changes component/loading state, not either dependency; no closing edge was found (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:136-144`).

### 5. Other Monaco effects and subscriptions — unlikely

- **[verified]** The native-language-mode effect reads `nativeCsharpLanguageClient` and reconciles Monaco provider registrations (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2981-2986`); it does not write that prop.
- **[verified]** The appearance effect reads appearance props and applies options to Monaco (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2992-2998`); it does not write those props.
- **[verified]** Monaco model, scroll, and marker subscriptions call editor callbacks or mutate non-rune caches (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2930-2945`). The only subscription path that closes into reactive editor content is the retained-draft callback already ranked first (`tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1951-1952`).
- **[verified]** EditorPanel's native-diagnostics subscription writes `diagnosticsByPath` only when an external native event arrives (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:538-549`); assigning that state does not emit another native event. The open-file subscription similarly runs from the explicit open-file bus, not from a reactive effect (`tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:970-1001`).

### 6. `+page.svelte` switching block — unlikely as the effect owner, verified as the trigger path

**[verified]** `+page.svelte` contains no `$effect` or `$derived`. `selectOwned` is an explicit async selection handler (`tauri-svelte-preview/src/routes/next/+page.svelte:868-935`), and `restoreWorkspace` is an explicit restore function (`tauri-svelte-preview/src/routes/next/+page.svelte:820-866`). These functions create the state sequence that exposes candidate 1, but they are not themselves reactive effects and cannot directly raise an effect self-update loop.

## Uncommitted experiments

- **Terminal `hosted = []`: ruled out — unlikely. [verified]** The experiment is a constant `$derived` with no reactive input and no write (`tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:49-52`). It removes terminal hosts from the DOM; it cannot read and write the same state. The remaining host action reacts only to browser observers and local animation-frame counters (`tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:91-130`).
- **Empty Git History snippet: ruled out — unlikely. [verified]** The experiment replaces `<GitHistoryView />` with a static `<div>` (`tauri-svelte-preview/src/routes/next/+page.svelte:1714-1719`). It removes a component and introduces no rune, subscription, callback, or write.

## Conclusion

**[verified-cycle]** The freeze is caused by `MonacoSourceEditor.svelte:1952` writing editor draft state from inside the `$effect` at `MonacoSourceEditor.svelte:2974-2977`. The write path reads and replaces `editorState.openFiles` at `EditorPanel.svelte:747-750` and `editorStore.svelte.ts:129-135`, so Svelte schedules the same effect forever while the model remains a retained draft.

**[verified direction, not implemented]** The one-line-level correction is to call `onContentChange` only when `content !== model.getValue()` at `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1952`. The restored-view-state deletion is a one-shot invalidation and should not be treated as the root cycle.
