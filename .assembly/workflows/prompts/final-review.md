Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

Inspect the open pull request for this branch (`gh pr view`, `gh pr diff`, `gh pr checks`) and the proof it links. Check that the diff does what the task asks, the checks pass, and the proof shows the change.

Last round's findings, if this is a later round: ${{ redo.findings }}
If one of your findings is the same problem as one of those and it is still not fixed, start it with `REPEAT:`. A repeated finding stops the run for the owner; a new finding sends the work back for another round.

The run works without the owner. Settle how the code behaves by reading it, never by asking. Small choices the task leaves open belong to the author; flag a choice only when it is wrong, and say what is right. Every finding must be a fix the author can make alone. Never write a finding that asks the owner to decide.

Call report_review exactly once: PASS with no findings, or a list of actionable findings. Only a recorded call counts: if the call is rejected or fails, read the reason and call it again.

Run commands with full paths or `git -C <dir>`; never start a command with `cd`, because that stops for approval.
