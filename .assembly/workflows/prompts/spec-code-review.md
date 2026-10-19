Task ${{ task.key }}: ${{ task.title }}

The reviewed spec:

${{ task.body }}

Read the whole Notion task too: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there.

Review this branch's diff against the default branch (`git diff origin/main...HEAD`) for correctness, the spec, and the repository rules (CLAUDE.md, AGENTS.md). Check that every step of the spec is done and nothing beyond it was added. Overengineering fails review: a new abstraction, flag, fallback, compatibility path or drive-by refactor the spec didn't ask for is a finding. SQL that loads rows and filters or groups them in code is a finding. Do not raise far-fetched hypotheticals or add scope.

What the builder reported:
${{ steps.build.summary }}

Last round's findings, if this is a later round: ${{ redo.findings }}
If one of your findings is the same problem as one of those and it is still not fixed, start it with `REPEAT:`. A repeated finding stops the run for the owner; a new finding sends the work back for another round.

The run works without the owner. Settle how the code behaves by reading it, never by asking. Every finding must be a fix the author can make alone, with `path:line` and the fix. Never write a finding that asks the owner to decide. A real problem that is outside this task, or that only the owner can settle, is not a finding: name it in your final reply as a follow-up, and pass or fail on the rest.

Call report_review exactly once: PASS with no findings, or a list of actionable findings. Only a recorded call counts: if the call is rejected or fails, read the reason and call it again.

Run commands with full paths or `git -C <dir>`; never start a command with `cd`, because that stops for approval.
