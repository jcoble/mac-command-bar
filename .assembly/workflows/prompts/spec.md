Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

Recon found:
${{ steps.recon.summary }}

On redo, fix these review findings: ${{ redo.findings }}

Write a small implementation spec in your reply: the exact files and line anchors, the change in each, the test to write first and what it asserts, the verification commands, and the expected diff size. Keep it the smallest change that meets the task (KISS, YAGNI: no new abstractions, options or fallbacks the task does not ask for). Do not edit files.
