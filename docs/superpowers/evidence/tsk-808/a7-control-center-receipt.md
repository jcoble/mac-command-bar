# Work Package A7 — Agent Control Center receipt

Date: 2026-08-05  
Repository: `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave`  
Branch: `tsk-808-assembly-wave`  
Frontend scope: `tauri-svelte-preview/`  
Authority: `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`, §10 and §16  
Backend contract: A6 WorkflowEngine at base commit `9434722`

## Result

A7 is implemented as a frontend-only Agent Control Center. The UI consumes complete `WorkflowRunRecord` snapshots, keeps only selection/filter state locally, sends every control through `workflowService.ts`, preserves the engine's typed loop phase, and keeps `workflowRunId` separate from provider-owned `ownedId`.

The controller-owned shell files were intentionally not edited. Their exact wiring diffs are recorded below for the controller/A8 lane. The branch already contains the shared A7 baseline commit `b8dad57`; this agent did not create or amend that commit. The gate/artifact polish and this receipt remain uncommitted per the explicit no-commit instruction.

The optional MCP companion (`src-tauri/src/workflow_mcp.rs` and its test) is deferred exactly as directed by the approved scope.

## Owned files and symbols

### Typed workflow contracts — `src/lib/shell/workflows/workflowTypes.ts`

- Exact serialized engine records: `WorkflowRunRecord`, `WorkflowNodeRunRecord`, `WorkflowGateRecord`, `WorkflowArtifactRef`, `WorkflowFailure`.
- Exact serialized definitions and policies: `WorkflowDefinitionV1`, `AgentRoleDefinition`, `WorkflowNodeDefinition`, `WorkflowEdge`, provider/model/effort/permission/workspace/retry/budget/concurrency/completion policies.
- Engine enums preserved as typed unions: `WorkflowRunState`, `WorkflowNodeState`, `WorkflowLoopPhase`, and `WorkflowOutputContract`.
- Structured receipts: `ImplementationReceipt`, `ReviewReceipt`, `SpecComplianceReceipt`, `VerificationReceipt`, `PlanReceipt`, and `WorkflowResultReceipt`.
- Presentation views: `WorkflowRunView`, `WorkflowNodeView`, `WorkflowAgentView`, `WorkflowGateView`, `WorkflowArtifactView`.
- Provenance is the exact union `'provider-native' | 'workflow'`, with `provenanceLabel` and stable display labels.
- `toWorkflowRunView` and related helpers add presentation identity only. `workflowRunId` is copied from the engine run id; each agent retains its independent `ownedId`. Loop phase is copied from `record.phase` and is never inferred from text.
- Provider-native children use `providerNativeAgentView` with `workflowRunId: null` and `provenance: 'provider-native'`.
- Typed command DTOs: `CreateWorkflowRunDto`, `WorkflowRunIdempotencyDto`, `WorkflowNodeIdempotencyDto`, `ApproveWorkflowGateDto`, `SubmitWorkflowResultDto`, and the start/pause/resume/cancel/retry/skip aliases plus `WorkflowCommandDto`.

### Tauri service boundary — `src/lib/shell/workflows/workflowService.ts`

The service is the only workflow UI boundary. It contains no reducer, ledger reader, phase inference, or optimistic state. It calls the ten authorized wrappers and exposes:

```text
listWorkflowRuns
createWorkflowRun
startWorkflowRun
pauseWorkflowRun
resumeWorkflowRun
cancelWorkflowRun
retryWorkflowNode
skipWorkflowNode
approveWorkflowGate
submitWorkflowResult
```

`subscribeWorkflowSnapshots` is a passive Tauri event subscription (`workflow-run-updated`) with a browser/older-controller no-op fallback. Command-returned snapshots remain authoritative. Components never import Tauri APIs and never call `invoke`.

### Snapshot cache — `src/lib/shell/workflows/workflowStore.svelte.ts`

The store contains one `$state` snapshot cache and ephemeral UI state only:

```text
resetWorkflowStore
beginWorkflowLoad
applyWorkflowSnapshots
failWorkflowLoad
upsertWorkflowSnapshot
selectWorkflowRun
selectWorkflowNode
setWorkflowQuery
setWorkflowFilter
selectedWorkflowRun
selectedWorkflowNode
filteredWorkflowRuns
workflowFilterOptions
```

It does not call Tauri, read event files, replay/reduce a ledger, or invent a phase.

### Authorized `tauriSource.ts` additions

`src/lib/tauriSource.ts` contains the exact ten A6 wrapper additions from `a6-workflow-engine-receipt.md` §“Exact tauriSource.ts wrapper additions”, with type-only imports adapted to `./shell/workflows/workflowTypes`:

```text
listWorkflowRunsFromTauri
createWorkflowRunFromTauri
startWorkflowRunFromTauri
pauseWorkflowRunFromTauri
resumeWorkflowRunFromTauri
cancelWorkflowRunFromTauri
retryWorkflowNodeFromTauri
skipWorkflowNodeFromTauri
approveWorkflowGateFromTauri
submitWorkflowResultFromTauri
```

No other `tauriSource.ts` code was changed for A7.

### Control-center components

All are under `src/lib/shell/components/workflows/`:

- `WorkflowControlCenter.svelte` — owns service orchestration, snapshot landing, selection, idempotency keys, and intent callbacks.
- `WorkflowRunList.svelte` — run list, search/filter, typed state/phase/provenance labels, refresh, and new-template entry point.
- `WorkflowRunHeader.svelte` — typed phase/metrics and real start/pause/resume/cancel controls plus save-as-template.
- `WorkflowGraph.svelte` — definition DAG with dependency edges and engine node states.
- `WorkflowLaneBoard.svelte` — role lanes, provider labels, provenance, and node state.
- `AgentHierarchy.svelte` — workflow root, workflow agents, optional provider-native children, `ownedId`, conversation/worktree/pin intent callbacks.
- `WorkflowTimeline.svelte` — snapshot timeline, attempts, output contracts, failures, and receipt sequence.
- `WorkflowNodeInspector.svelte` — node identity, gate JSON response, approve/reject, retry, skip, structured result submission, artifacts, and reviewer comparison.
- `AgentRuntimeInspector.svelte` — conversation/tools/plan/tasks/output/worktree/usage slots, provider/runtime identity, and intent callbacks.
- `WorkflowTemplateEditor.svelte` — template definition and per-role provider/model/effort/permission fields, JSON input, create draft, and create-and-start.
- `AgentActivityPane.svelte` — compact progress, active-agent, waiting-approval, failure, conversation, and pin sections.

Workflow children are not added to the left rail by these components. Pinning is emitted through `onPinToWorking(ownedId)` only.

## Service DTO and command payload receipt

The UI sends the exact backend parameter names and an idempotency key on every mutating command:

```ts
interface CreateWorkflowRunDto {
  definition: WorkflowDefinitionV1;
  input: Record<string, unknown>;
  idempotencyKey: string;
}

interface WorkflowRunIdempotencyDto {
  runId: string;
  idempotencyKey: string;
}

interface WorkflowNodeIdempotencyDto extends WorkflowRunIdempotencyDto {
  nodeId: string;
}

interface ApproveWorkflowGateDto extends WorkflowNodeIdempotencyDto {
  approval: { approved: boolean; [key: string]: unknown };
}

interface SubmitWorkflowResultDto extends WorkflowNodeIdempotencyDto {
  result: WorkflowResultReceipt;
}
```

The wrappers serialize these as `definition`, `input`, `runId`, `nodeId`, `approval`, `result`, and `idempotencyKey`. The gate editor validates a JSON object and sends `{ ...structuredInput, approved }`, matching the engine's structured gate contract.

## Controller wiring receipt — exact diffs, intentionally not applied

The following are the exact controller-side changes required after the A7 lane. The target files are forbidden by the A7 ownership boundary, so these hunks are evidence/instructions only.

### 1. Center layout key v4 → v5 exact-set migration

`src/lib/shell/layout/layoutStorage.ts`:

```diff
- export const CENTER_LAYOUT_KEY = 'mac-command-bar.next.center-layout-v4';
+ export const CENTER_LAYOUT_KEY_V4 = 'mac-command-bar.next.center-layout-v4';
+ export const CENTER_LAYOUT_KEY = 'mac-command-bar.next.center-layout-v5';
```

`src/lib/shell/layout/centerDock.ts`:

```diff
 import {
   CENTER_LAYOUT_KEY,
+  CENTER_LAYOUT_KEY_V4,
   CENTER_LAYOUT_KEY_V3,
   ...
 } from './layoutStorage';
 
+const CENTER_PANEL_IDS_V4 = [
+  'session',
+  'editor',
+  'browser',
+  'diff',
+  'session-library'
+] as const;
 const CENTER_PANEL_IDS_V3 = ['session', 'editor', 'browser', 'diff'] as const;
```

Add the Agents center panel to the panel specification and migrate only an exact v4 roster before the existing v3 migration:

```diff
   panels: [
     { id: 'session', title: 'Session', element: sessionSlot },
     { id: 'editor', title: 'Editor', element: editorSlot, group: 'display' },
     { id: 'browser', title: 'Browser', element: browserSlot, group: 'display' },
     { id: 'diff', title: 'Diff', element: diffSlot, group: 'display' },
     {
       id: 'session-library',
       title: 'Session Library',
       element: sessionLibrarySlot,
       group: 'display'
     },
+    { id: 'agents', title: 'Agents', element: agentsSlot, group: 'display' }
   ]
```

```diff
   let migratedFromV3 = false;
+  let migratedFromV4 = false;
 
   runSynchronized(() => {
     const stored = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY);
     if (stored && panelSetMatches(dockPanelIds(stored), specs.keys())) {
       ...
     }
 
+    const previousV4 = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY_V4);
+    if (previousV4 && panelSetMatches(dockPanelIds(previousV4), CENTER_PANEL_IDS_V4)) {
+      try {
+        api.fromJSON(previousV4 as never);
+        const activePanelId = api.activePanel?.id ?? null;
+        const agents = specs.get('agents');
+        if (!agents || !panelById('session-library')) {
+          throw new Error('Agents migration roster is incomplete');
+        }
+        addPanelFor(agents, { referencePanel: 'session-library', direction: 'within' });
+        if (activePanelId) panelById(activePanelId)?.api.setActive();
+        migratedFromV4 = true;
+        return;
+      } catch {
+        try { api.clear(); } catch { /* fall through */ }
+      }
+    }

     const previous = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY_V3);
     if (previous && panelSetMatches(dockPanelIds(previous), CENTER_PANEL_IDS_V3)) {
       try {
         api.fromJSON(previous as never);
         const activePanelId = api.activePanel?.id ?? null;
         const library = specs.get('session-library');
+        const agents = specs.get('agents');
-        if (!library || !panelById('diff')) throw new Error('Session Library migration roster is incomplete');
+        if (!library || !agents || !panelById('diff')) throw new Error('Center migration roster is incomplete');
         addPanelFor(library, { referencePanel: 'diff', direction: 'within' });
+        addPanelFor(agents, { referencePanel: 'session-library', direction: 'within' });
         if (activePanelId) panelById(activePanelId)?.api.setActive();
         migratedFromV3 = true;
         return;
       ...
 
-  if (migratedFromV3) {
+  if (migratedFromV4 || migratedFromV3) {
     saveLayout(options.storage, CENTER_LAYOUT_KEY, api.toJSON());
   }
```

The current-key restore remains an exact `specs.keys()` match. A v4 layout is not hand-edited; it is restored through Dockview, then the single `agents` tab is added and the migrated v5 snapshot is persisted.

### 2. `ShellFrame.svelte` center snippet and parking slot

```diff
   center: {
     session: Snippet;
     editor: Snippet;
     browser: Snippet;
     diff: Snippet;
     sessionLibrary: Snippet;
+    agents: Snippet;
   };
@@
   let diffSlot: HTMLElement;
   let sessionLibrarySlot: HTMLElement;
+  let agentsSlot: HTMLElement;
@@
           {
             id: 'session-library',
             title: 'Session Library',
             element: sessionLibrarySlot,
             group: 'display'
           },
+          { id: 'agents', title: 'Agents', element: agentsSlot, group: 'display' }
@@
   <div class="slot" bind:this={diffSlot}>{@render center.diff()}</div>
   <div class="slot" bind:this={sessionLibrarySlot}>{@render center.sessionLibrary()}</div>
+  <div class="slot" bind:this={agentsSlot}>{@render center.agents()}</div>
```

### 3. Right-side `AgentActivityPane` registration

The current legacy right-column roster needs an `agents` view (the A8 side-pane registry can express the same registration without changing the behavior). `sidebarViews.ts`:

```diff
 export type SidebarViewId =
   | 'explorer'
   | 'source-control'
   | 'worktrees'
   | 'stacks'
   | 'context'
+  | 'agents'
   | 'problems';
@@
   { id: 'context', title: 'Context' },
+  { id: 'agents', title: 'Agent activity' },
   PROBLEMS_SIDEBAR_VIEW
 ];
```

Because `ActivityBar.svelte` exhaustively maps the sidebar roster to icons, its controller-side companion hunk is:

```diff
 import {
   Activity,
+  Bot,
   Files,
@@
   context: Activity,
+  agents: Bot,
   problems: TriangleAlert
 };
```

`ShellSidebar.svelte`:

```diff
 import ContextPanel from './ContextPanel.svelte';
+import AgentActivityPane from './workflows/AgentActivityPane.svelte';
@@
     onProblemsVisible?: (visible: boolean) => void;
+    onOpenWorkflow?(workflowRunId: string): void;
+    onOpenAgentConversation?(ownedId: string): void;
+    onPinAgentToWorking?(ownedId: string): void;
@@
    problems: PROBLEMS_SIDEBAR_PANE,
+   agents: { id: 'agents', title: 'Agent activity' }
   };
@@
   <div class="slot" bind:this={bodies.context}><ContextPanel /></div>
+  <div class="slot" bind:this={bodies.agents}>
+    <AgentActivityPane
+      onOpenWorkflow={onOpenWorkflow}
+      onOpenConversation={onOpenAgentConversation}
+      onPinToWorking={onPinAgentToWorking}
+    />
+  </div>
   <div class="slot" bind:this={bodies.problems}><ProblemsPanel /></div>
```

No workflow service is called from `ShellSidebar`; the pane reads the shared A7 snapshot store.

### 4. Command palette entry — `shellCommands.ts`

`showPanel` already accepts a center-panel id, so no new command-hook type is needed:

```diff
     {
       id: 'show-session-library',
       label: 'Show the Session Library',
       detail: 'Open the complete session and history library in the center',
       perform: () => hooks.showPanel('session-library')
     },
+    {
+      id: 'show-agents',
+      label: 'Show agent control center',
+      detail: 'Open workflow runs, agents, gates, and activity',
+      perform: () => hooks.showPanel('agents')
+    },
     {
       id: 'new-session',
```

### 5. Page mount and intent callbacks — `routes/next/+page.svelte`

Add the component import and the shared selection helper in the controller lane:

```diff
 import SessionLibraryWorkspace from '$lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte';
+import WorkflowControlCenter from '$lib/shell/components/workflows/WorkflowControlCenter.svelte';
+import { selectWorkflowRun } from '$lib/shell/workflows/workflowStore.svelte';
```

Add the center snippet, keeping all existing shell internals behind callback props:

```diff
 {#snippet sessionLibraryArea()}
   <SessionLibraryWorkspace
     owned={rail.owned}
     available={rail.available}
     service={sessionLibraryService}
     onRefresh={() => void scanRail()}
   />
 {/snippet}
+{#snippet agentsArea()}
+  <WorkflowControlCenter
+    onOpenConversation={(ownedId) => void selectOwned(ownedId)}
+    onOpenWorktree={(ownedId) => {
+      sidebarControls?.selectView('worktrees');
+      // hand ownedId to the existing worktree surface's intent callback
+    }}
+    onPinToWorking={(ownedId) => {
+      // hand ownedId to the existing Working intent; do not create a second rail
+    }}
+    onCompareReviewers={(workflowRunId) => {
+      selectWorkflowRun(workflowRunId);
+      frameControls?.showCenterPanel('agents');
+    }}
+  />
+{/snippet}
@@
     center={{
       session: sessionArea,
       editor: editorArea,
       browser: browserArea,
       diff: diffArea,
-      sessionLibrary: sessionLibraryArea
+      sessionLibrary: sessionLibraryArea,
+      agents: agentsArea
     }}
```

Pass the right-pane intent callbacks through the existing `ShellSidebar` registration at the same controller boundary:

```diff
   <ShellSidebar
@@
     onOpenSession={(ownedId) => void selectOwned(ownedId)}
     onShowDiff={() => frameControls?.showCenterPanel('diff')}
+    onOpenWorkflow={(workflowRunId) => {
+      selectWorkflowRun(workflowRunId);
+      frameControls?.showCenterPanel('agents');
+    }}
+    onOpenAgentConversation={(ownedId) => void selectOwned(ownedId)}
+    onPinAgentToWorking={(ownedId) => {
+      // forward the explicit pin intent to the existing Working authority
+    }}
   />
```

The comments above mark existing controller intents, not new A7 state or a new rail/store. The controller owner supplies the already-existing Working/worktree callbacks for the `ownedId` intent.

## Verification receipts

All checks were run serially after the final A7 edits:

```text
node --experimental-strip-types scripts/agentControlCenter.test.mjs
PASS — agent control center tests passed

pnpm test:orchestration-event
PASS — process exited 0

pnpm check:svelte
PASS — Files the /next shell owns: 0 error(s), 0 warning(s)

pnpm check
PASS — svelte-kit sync and tsc --noEmit exited 0
```

The Svelte checker reports an existing 16-error backlog outside the `/next` shell; A7-owned files have zero errors and zero warnings. The Node runtime prints only its standard experimental strip-types warning.

## Boundary, cleanup, and deviations

- No Rust file, manifest, package file, centerDock, ShellFrame, ShellSidebar, sidebarViews, shellCommands, or `+page.svelte` was edited for A7. An unrelated concurrent dirty change to `src-tauri/Cargo.toml` was observed and left untouched.
- No browser/Playwright session was opened.
- No worktree was created by A7, so no worktree cleanup was required.
- No second workflow store, ledger, reducer, runtime, or direct JSONL/event-reader path was added.
- Optional `workflow_mcp.rs` remains deferred as required.
- No deviations.
