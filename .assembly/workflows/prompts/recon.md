Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

Read the whole task before anything else: `notion-fetch` ${{ task.key }} and read its sections verbatim; the owner's decisions and later corrections are there. If the task has reference screenshots, `Read` each one at the absolute path recorded in the task body (attached screenshots are also saved under `~/dev/work/reports/tsk-ui-base/screenshots/`) and judge the work against them.

Read the repository rules first (CLAUDE.md, AGENTS.md, and tauri-svelte-preview/CLAUDE.md). Then find the files this task touches. Do not edit anything.

Report: each relevant file with `path:line` and what it does for this task, the current behaviour, and any rule that constrains the change. Mark every claim verified (you read it) or assumed.
