Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

The spec to review:
${{ steps.spec.summary }}

Check it against the task and the repository rules (CLAUDE.md, AGENTS.md). Check that its file and line claims are true by reading them. Flag added scope or complexity the task does not need. Do not raise far-fetched hypotheticals.

Call report_review exactly once: PASS with no findings, or a list of actionable findings, each naming the file and the fix.
