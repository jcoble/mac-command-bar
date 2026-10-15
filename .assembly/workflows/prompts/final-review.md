Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

Inspect the open pull request for this branch (`gh pr view`, `gh pr diff`, `gh pr checks`) and the proof it links. Check that the diff does what the task asks, the checks pass, and the proof shows the change.

Call report_review exactly once: PASS with no findings, or a list of actionable findings.
