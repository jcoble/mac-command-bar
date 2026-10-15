Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Inspect the open pull request for this branch (`gh pr view`, `gh pr diff`, `gh pr checks`) and the proof it links. Check that the diff does what the task asks, the checks pass, and the proof shows the change.

Call report_review exactly once: PASS with no findings, or a list of actionable findings.
