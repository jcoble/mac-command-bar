Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

Recon found:
${{ steps.recon.summary }}

On redo, fix these review findings: ${{ redo.findings }}

The run works without the owner. Answer questions about how the code works by reading it. For small decisions the task leaves open, make the best choice, state it in one line, and keep going. Mark something `blocked` only when you truly cannot do it without the owner.

Open the spec with a requirement-by-requirement checklist taken from the task and its screenshots, each requirement marked `already done`, `will fix` or `blocked`, so a misread shows up before anything is built. Then write a small implementation spec in your reply: the exact files and line anchors, the change in each, the test to write first and what it asserts, the verification commands, and the expected diff size. Keep it the smallest change that meets the task (KISS, YAGNI: no new abstractions, options or fallbacks the task does not ask for). Do not edit files.

Run commands with full paths or `git -C <dir>`; never start a command with `cd`, because that stops for approval.
