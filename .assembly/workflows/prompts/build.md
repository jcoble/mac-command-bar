Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

Implement this reviewed spec:
${{ steps.spec.summary }}

On redo, fix these review findings: ${{ redo.findings }}
and this failing check output: ${{ redo.output }}

Follow the repository rules (CLAUDE.md, AGENTS.md): test first, the smallest diff, SQL work in SQL, the async lifecycle rule, no CSS :has(). Run the targeted tests yourself before committing. Commit only your changes on this branch; the commit body ends with `Task: ${{ task.key }}`, `Architecture: unchanged` (or the staged architecture page), and `Committed-by: Codex`, and has no Co-Authored-By line.

Report: the commit hash, files changed, and each command you ran with its result.
