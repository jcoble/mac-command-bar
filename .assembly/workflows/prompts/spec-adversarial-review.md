Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

The spec to review:
${{ steps.spec.summary }}

Your job is to break this spec. Read the code it touches and look for ways the planned change fails: wrong assumptions about how the code behaves (check each one), unhandled states (empty, many, slow, offline, remote machine, restart, a session closed mid-way), races and ordering, stale data, cancellation and cleanup (the async lifecycle rule), error paths, and the same requirement read two ways. Each finding must name a concrete case that goes wrong, show it from the code, and give the fix. Do not raise far-fetched hypotheticals or ask for scope the task does not need.

Last round's findings, if this is a later round: ${{ redo.findings }}
If one of your findings is the same problem as one of those and it is still not fixed, start it with `REPEAT:`. A repeated finding stops the run for the owner; a new finding sends the work back for another round.

The run works without the owner. Settle how the code behaves by reading it, never by asking. Small choices the task leaves open belong to the author; flag a choice only when it is wrong, and say what is right. Every finding must be a fix the author can make alone. Never write a finding that asks the owner to decide. A real problem that is outside this task, or that only the owner can settle, is not a finding: name it in your final reply as a follow-up, and pass or fail on the rest.

Call report_review exactly once: PASS with no findings, or a list of actionable findings, each naming the file and the fix. Only a recorded call counts: if the call is rejected or fails, read the reason and call it again.

Run commands with full paths or `git -C <dir>`; never start a command with `cd`, because that stops for approval.
