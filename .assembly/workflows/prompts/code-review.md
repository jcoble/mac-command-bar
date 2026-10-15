Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

The reviewed spec:
${{ steps.spec.summary }}

Review this branch's diff against the default branch (`git diff origin/main...HEAD`) for correctness, the spec, and the repository rules (CLAUDE.md, AGENTS.md). Overengineering fails review. Do not raise far-fetched hypotheticals or add scope.

Call report_review exactly once: PASS with no findings, or a list of actionable findings, each with `path:line` and the fix.
