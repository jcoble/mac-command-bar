Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

The spec to review:
${{ steps.spec.summary }}

This review covers only what the owner will SEE. If the spec changes no visible UI (no `.svelte` markup, styles, layout, copy or interaction), call report_review with PASS and no findings right away.

Otherwise judge the planned UI against the task, its screenshots and mockup, `tauri-svelte-preview/src/lib/components/ui/DESIGN.md` (binding), and the repository rules: stock shadcn-svelte components, no CSS `:has()`, no native `<select>`. Check where each new element sits and whether it fits. The rail and drawers are already crowded, so a plan that adds rows, chips or badges must say where the space comes from. Check that empty, loading, error and long-text states are covered, and that the plan follows the patterns the app already uses for the same thing. Flag visual scope the task does not ask for. Do not raise matters of taste the task and DESIGN.md leave open.

Last round's findings, if this is a later round: ${{ redo.findings }}
If one of your findings is the same problem as one of those and it is still not fixed, start it with `REPEAT:`. A repeated finding stops the run for the owner; a new finding sends the work back for another round.

The run works without the owner. Settle how the code behaves by reading it, never by asking. Small choices the task leaves open belong to the author; flag a choice only when it is wrong, and say what is right. Every finding must be a fix the author can make alone. Never write a finding that asks the owner to decide.

Call report_review exactly once: PASS with no findings, or a list of actionable findings, each naming the file and the fix.
