Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

The spec to review:
${{ steps.spec.summary }}

Check it against the task and the repository rules (CLAUDE.md, AGENTS.md). Check that its file and line claims are true by reading them. Flag added scope or complexity the task does not need. Do not raise far-fetched hypotheticals.

Call report_review exactly once: PASS with no findings, or a list of actionable findings, each naming the file and the fix.
