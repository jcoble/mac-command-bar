Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

The reviewed spec:
${{ steps.spec.summary }}

Review this branch's diff against the default branch (`git diff origin/main...HEAD`) for correctness, the spec, and the repository rules (CLAUDE.md, AGENTS.md). Overengineering fails review. Do not raise far-fetched hypotheticals or add scope.

Call report_review exactly once: PASS with no findings, or a list of actionable findings, each with `path:line` and the fix.
