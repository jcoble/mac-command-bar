Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Recon found:
${{ steps.recon.summary }}

On redo, fix these review findings: ${{ redo.findings }}

Write a small implementation spec in your reply: the exact files and line anchors, the change in each, the test to write first and what it asserts, the verification commands, and the expected diff size. Keep it the smallest change that meets the task (KISS, YAGNI: no new abstractions, options or fallbacks the task does not ask for). Do not edit files.
