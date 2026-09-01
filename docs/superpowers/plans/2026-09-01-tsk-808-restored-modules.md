# TSK-808 restored modules integration contract

## Owner outcome

Create a new integration worktree from `tsk-808-async-lifecycle-cleanup` and
restore the product capabilities that were implemented on other TSK-808
worktrees but never reached this branch, including the large Notion-task wave
and remote development.

This is a recovery and modularization lane, not a rollback. Historical commits
are evidence and implementation sources; they are not blanket permission to
restore diagnostics, experiments, generated evidence, obsolete architecture,
or code deliberately superseded by the current lifecycle contract.

## Fixed decisions

- Integration worktree:
  `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-restored-modules`
- Integration branch: `tsk-808-restored-modules`
- Exact base: `04b625ac577b88ef6a6dcdd031556f024028a8a4`
  (`tsk-808-async-lifecycle-cleanup` when the worktree was created)
- Restore independently useful product modules and their required tests.
- Do not restore memory probes, experiment toggles, screenshots, generated
  reports, temporary fixtures, or retired duplicate implementations unless a
  production module directly requires them.
- Keep unrelated behavior out of the session-selection path. Each restored
  subsystem gets one narrow, named integration call so commenting out that call
  removes the subsystem from session switching without editing its internals.
- Put restored behavior in focused TypeScript modules and Svelte components.
  Do not rebuild a large route-level controller or mix unrelated product areas.
- Lifecycle-owned TypeScript asynchronous code uses `async` functions and
  `await`, not promise chains.
- Session-owned or supersedable asynchronous work receives and checks the
  existing stop-signal abstraction. Stopping one selection must not cancel a
  running agent turn that is allowed to continue in the background.
- Preserve the accepted explicit teardown order. Do not use a whole-session
  `{#key}` remount.
- Never introduce CSS `:has()`.

## Recovery sources under verification

1. `tsk-808-workbench-completion`: the large product/task implementation wave.
2. `tsk-808-assembly-wave`: later integrated product work, remote execution,
   memory-lifecycle changes, and current modular shell controllers.
3. `1591be58985c6c38bebb274e2cdf9e39b78a811a` and its prerequisite commits:
   generic remote Assembly execution.
4. Individual TSK-808 branches and historical commits where a product module
   was removed or disabled during memory isolation.

The inventory must distinguish current product code from historical probes and
must verify content equivalence rather than treating an absent commit hash as
an absent capability.

## Implementation sequence

1. Build a capability ledger with source commits, current presence, dependencies,
   and deliberate-removal evidence.
2. Integrate the latest coherent product baseline without importing generated
   evidence and diagnostic-only history.
3. Recover missing or disabled product modules one subsystem at a time.
4. Extract session-selection wiring into focused controller modules where the
   current source still mixes unrelated behavior.
5. Convert restored lifecycle-owned promise chains to `async`/`await` and add
   stop-signal ownership at their entry points.
6. Run focused TypeScript/Svelte tests after each subsystem, then serialized
   repository gates and native lifecycle proof.

## Completion evidence

- Capability ledger accounts for the Notion/task wave, remote development, and
  features removed during memory experiments.
- Restored features are present and reachable in the integration worktree.
- Session-selection integrations are individually identifiable and removable.
- No production promise chains remain in restored lifecycle-owned code.
- Superseded session work stops through the existing stop-signal abstraction.
- Focused tests, Svelte/TypeScript gates, diff checks, and current native proof
  pass, with exact receipts recorded before the branch is called complete.
