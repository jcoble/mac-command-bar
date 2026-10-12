# Writing an Assembly workflow

Write YAML and prompt files that the owner can review, edit and version in git.
Read this reference through `workflow_format()` in any Assembly session.

## Files and validation

Write `.assembly/workflows/<name>.yml` in the calling session's project repository
on the machine that will run it. Use `.yml`, not `.yaml`. The picker lists only
files directly in that folder. Put prompt files beside the YAML, usually under
`.assembly/workflows/prompts/`. Prompt paths are relative to the YAML's folder.
Files must exist. Reject absolute paths and `../`; the YAML must stay inside the
project and prompts inside its resolved parent folder, including through symlinks.
Pass a project-relative path to `check_workflow(path)`.
Use one YAML document, unique mapping keys, and step IDs containing only ASCII
letters, digits, `_` or `-`. Include at least one step. Dependencies must exist,
must not refer to the step itself, and must not form a cycle.
Unknown top-level and step-level keys are rejected: the error names the key and,
for a step-level key, the step ID. The only allowed keys at these two levels
are those in the tables below. Agent and review steps require a string provider;
reviews require `of`. Both `review.of` and `on_fail`, when present, must name a
direct or transitive dependency of the current step. `run` must be a non-empty
list whose entries are strings.

`check_workflow` uses the same loader that starts a run, so a file that checks
`ok` will load when it is started. A check does not run commands or prove that a
provider or model is installed.

## Top-level fields

| Field | Type | Required / default | What the engine does |
|---|---|---|---|
| `name` | string | Required; no default | Display name saved with the definition. Use a nonempty descriptive name. |
| `inputs` | mapping from input names to definitions | Optional; empty | Saves definitions with the snapshot; values are supplied when starting the run. |
| `limits` | mapping | Optional; defaults below | Saves limits and uses session/timeout values when constructing nodes. |
| `steps` | mapping from step IDs to step objects | Required; no default | Builds the dependency graph and saves each step. YAML order does not set execution order. |

Do not write internal `version`, `roles`, `nodes`, `edges` or `concurrency` fields.
The loader constructs these. Unknown keys at this level fail validation.

| Input definition field | Type | Required / default | Meaning |
|---|---|---|---|
| `description` | string | Optional; no description | Explains the input to the owner. |
| `required` | boolean | Optional; no loader default | Declares that the caller must supply the input. |
| `default` | YAML value | Optional; no default | Declares the value to use when the caller omits it. |

Task 7 saves input definitions without checking `required` or filling `default`.
Supply needed values explicitly until the start caller applies them. `task` is a
special input: start supplies `{key, title, body}` as a snapshot and the loader
inserts it into run input, replacing a supplied `task` value. The backend needs
no Notion token. `i_merge` is a boolean; use `false` unless the owner selects
“I'll merge”. The loader also inserts the canonical project root as `cwd`.

| Limit field | Type | Required / default | Meaning |
|---|---|---|---|
| `max_sessions` | nonnegative integer | Optional; 3 | Maximum active agents shared across runs on the owning backend. Use a positive value. |
| `step_timeout_minutes` | nonnegative integer | Optional; 60 | Per-step timeout, converted to seconds; excessive values are rejected. Use a positive value. |
| `max_rounds` | integer | Optional; write 5 explicitly | Review/check loop ceiling. Task 7 saves it without applying a default; the loop contract caps rounds at 5. |

Task 7 also falls back to 3/60 for wrongly typed values; write proper integers.

## Step fields and the four kinds

Each step has exactly one of `agent`, `run`, `review`, `gate`.

| Field | Type | Required / default | What the engine does |
|---|---|---|---|
| `needs` | list of step ID strings | Optional; `[]` | Waits for every dependency to pass or be skipped. |
| `agent` | mapping | Required for an agent step | Starts a worker with the provider/model below. |
| `prompt` | relative file path string | Required for an agent step; no default | Reads and snapshots the prompt text; fills template values at execution. |
| `run` | non-empty list of shell command strings | Required for a command step; no default | Runs every command separately from the run worktree root; all must exit 0. Saves exit codes/output. |
| `review` | mapping | Required for a review step | Starts a fresh reviewer; requires exactly one valid `report_review` tool call. |
| `gate` | YAML value (opaque to Task 7) | Required for a gate step; no default | Pauses for an owner answer. Task 7 preserves this value; its nested format is not yet defined, so do not invent gate fields. |
| `on_fail` | step ID string | Optional; no redo target | Must name a direct/transitive dependency; a failed command step sends it back to redo with output. |
| `expect_files` | list of file pattern strings | Optional; no expected files | Checks agent output files; missing files fail the step. |
| `if` | template condition string | Optional; no condition | False skips the step without dispatching it. |
| `repeat` | boolean | Optional; not marked safe | Only `true` makes a command step safe to rerun after restart. Omit for side effects such as PR creation and merge. |

| Agent/review mapping field | Type | Required / default | What the engine does |
|---|---|---|---|
| `agent.provider` | string | Required; missing/non-string provider is rejected | Selects the provider; use `codex` or `claude`. |
| `agent.model` | string | Optional; no selection (`null` internally) | Selects an advertised provider model when given; omission keeps provider selection. |
| `review.of` | step ID string | Required; missing `of` is rejected | Findings redo this step. It must be a direct or transitive dependency. |
| `review.agent` | mapping with `provider`, optional `model` | Required for usable dispatch; no default | Selects the reviewer provider/model. |
| `review.prompt` | relative file path string | Required; no default | Reads the review prompt beside the YAML and snapshots its contents. |

In a review, put `agent` and `prompt` inside `review`, not at step level.
`report_review` carries PASS with zero findings, or findings that the engine
counts. A final reply saying “looks good” is not a verdict. The shared loader enforces the validation rules above and prompt containment.
It does not validate gate contents, file patterns, conditions or repeat flags,
or nested unknown keys. No schema framework or installed-provider/model check
is part of format validation.

## Template values

Use `${{ ... }}` in prompt text and supported command/file/condition fields.
These values come from the engine; do not invent values or evaluate arbitrary code.

| Value | Meaning |
|---|---|
| `${{ inputs.<name> }}` | A run input value. |
| `${{ task.key }}`, `${{ task.title }}`, `${{ task.body }}` | The exact starting task snapshot. |
| `${{ steps.<id>.summary }}` | Final agent reply, or last 200 lines of command output. |
| `${{ steps.<id>.files }}` | Files recorded for that step. |
| `${{ redo.findings }}`, `${{ redo.output }}` | Findings or failing output provided on redo. |
| `${{ run.proof_dir }}` | Engine-owned proof folder outside the repository, linked in history. |
| `${{ run.pr_body }}` | Engine-written PR body file containing task, summaries, final review and proof links. |
| `${{ !inputs.i_merge }}` | Negated boolean condition used to skip automatic merge when the owner will merge. |

## Loop rules

The first findings allow a redo. Later rounds continue only while the finding
count strictly decreases; equal/increased counts or the five-round ceiling pause
for the owner. Commands count failures per command; reviews count reported
findings. Redo reuses the original worker session, creates fresh reviewers, and
invalidates every direct/transitive dependent so checks, reviews and proof run
again. Another reviewer is another `review` step with the same `of`; never add a
`reviewers` list. Findings from either reviewer redo the shared target.

## Merge rules

The workflow's merge rule is the owner's standing authorization for that run:
merge only after checks pass, final review passes and proof is attached, then
send a summary. With `i_merge: true`, stop with the PR open. Use the condition
below and keep merge non-repeatable so restart uncertainty pauses for the owner.
These are slice-4 execution rules, not guarantees supplied by the Task 7 parser.
The PR step must attach reachable proof before the final review; merge must use
the slice-4 guarded command that checks the current PR head and D6 facts. The
short commands below illustrate the graph and require that runtime guard; raw
`gh pr merge` alone does not enforce it. Refresh the engine-written PR body with
the final verdict before merge or the “I'll merge” handoff.
An authoring request or agent message alone does not authorize a merge.

## Example 1: worker then checks

Write `.assembly/workflows/worker-checks.yml`:

```yaml
name: Worker and checks
limits: { max_sessions: 1, step_timeout_minutes: 60, max_rounds: 5 }
steps:
  build: { agent: { provider: codex }, prompt: prompts/build.md }
  checks:
    needs: [build]
    run: ["git diff --check"]
    repeat: true
    on_fail: build
```

Write `prompts/build.md`: “Implement ${{ task.body }}. Fix ${{ redo.output }} on
redo. Commit only your changes and summarize them.” Replace the whitespace check
with the project's real checks.

## Example 2: dev pipeline

Write `.assembly/workflows/dev-pipeline.yml` (spec §5.1, with D6 final review):
Model strings reproduce the spec; select installed models before running it.

```yaml
name: Dev pipeline
inputs:
  task: { description: Notion task, required: true }
  i_merge: { description: "I'll merge", default: false }
limits: { max_sessions: 3, step_timeout_minutes: 60, max_rounds: 5 }
steps:
  recon: { agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/recon.md }
  spec: { needs: [recon], agent: { provider: claude, model: opus }, prompt: prompts/spec.md }
  spec-review:
    needs: [spec]
    review: { of: spec, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/spec-review.md }
  build: { needs: [spec-review], agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/build.md }
  checks:
    needs: [build]
    run:
      - cd tauri-svelte-preview && npx tsc --noEmit
      - cd tauri-svelte-preview && pnpm run check:svelte
      - cd core && cargo test
    repeat: true
    on_fail: build
  code-review:
    needs: [checks]
    review: { of: build, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/code-review.md }
  proof:
    needs: [code-review]
    agent: { provider: claude, model: opus }
    prompt: prompts/proof.md
    expect_files: ["${{ run.proof_dir }}/*.png"]
  pr:
    needs: [proof]
    run:
      - git push -u origin HEAD
      - gh pr create --fill --body-file "${{ run.pr_body }}"
  final-review:
    needs: [pr]
    review: { of: build, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/final-review.md }
  merge:
    needs: [final-review]
    if: "${{ !inputs.i_merge }}"
    run: ["ALLOW_MERGE=1 gh pr merge --merge"]
```

Create these complete starter prompt files under `.assembly/workflows/prompts/`;
adapt their content to the project and its rules before using the workflow:

| File | Contents |
|---|---|
| `recon.md` | Read ${{ task.body }} and the project rules. Locate relevant files and report their responsibilities. |
| `spec.md` | Use ${{ steps.recon.summary }} to write a small implementation spec for ${{ task.body }}. |
| `spec-review.md` | Review ${{ steps.spec.summary }} against ${{ task.body }}. Call report_review exactly once with PASS and no findings, or actionable findings. |
| `build.md` | Implement the reviewed spec for ${{ task.body }}. Fix ${{ redo.findings }} and ${{ redo.output }} on redo. Commit only your changes on the run branch. |
| `code-review.md` | Review the build diff against ${{ task.body }} and the reviewed spec. Call report_review exactly once with PASS and no findings, or actionable findings. |
| `proof.md` | Verify the completed change in the running app. Save direct proof in ${{ run.proof_dir }}, register and publish media with the existing evidence tool, and summarize what it shows. For nonvisual work, attach readable command receipts to the PR. |
| `final-review.md` | Inspect the current PR diff, checks and attached proof against ${{ task.body }}. Call report_review exactly once with PASS and no findings, or actionable findings. |

## Example 3: dev pipeline with two code reviewers

Write `.assembly/workflows/dev-pipeline-two-reviewers.yml`; use all seven prompt
files above plus `prompts/code-review-2.md` containing: “Independently review the
build diff against ${{ task.body }}. Call report_review exactly once with PASS
and zero findings, or actionable findings.” Both reviewers must pass before proof.

```yaml
name: Dev pipeline with two code reviewers
inputs:
  task: { description: Notion task, required: true }
  i_merge: { description: "I'll merge", default: false }
limits: { max_sessions: 3, step_timeout_minutes: 60, max_rounds: 5 }
steps:
  recon: { agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/recon.md }
  spec: { needs: [recon], agent: { provider: claude, model: opus }, prompt: prompts/spec.md }
  spec-review:
    needs: [spec]
    review: { of: spec, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/spec-review.md }
  build: { needs: [spec-review], agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/build.md }
  checks:
    needs: [build]
    run:
      - cd tauri-svelte-preview && npx tsc --noEmit
      - cd tauri-svelte-preview && pnpm run check:svelte
      - cd core && cargo test
    repeat: true
    on_fail: build
  code-review:
    needs: [checks]
    review: { of: build, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/code-review.md }
  code-review-2:
    needs: [code-review]
    review: { of: build, agent: { provider: claude, model: opus }, prompt: prompts/code-review-2.md }
  proof:
    needs: [code-review-2]
    agent: { provider: claude, model: opus }
    prompt: prompts/proof.md
    expect_files: ["${{ run.proof_dir }}/*.png"]
  pr:
    needs: [proof]
    run:
      - git push -u origin HEAD
      - gh pr create --fill --body-file "${{ run.pr_body }}"
  final-review:
    needs: [pr]
    review: { of: build, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/final-review.md }
  merge:
    needs: [final-review]
    if: "${{ !inputs.i_merge }}"
    run: ["ALLOW_MERGE=1 gh pr merge --merge"]
```

## How to write one

1. Call `workflow_format()` and read the current reference.
2. Turn the owner's request into step IDs, dependencies and real project checks.
3. Use a separate review step per reviewer, with the same `of` for shared work.
4. Write the YAML and every referenced prompt inside `.assembly/workflows/`.
5. Keep side-effect commands non-repeatable; state any runtime feature still pending.
6. call `check_workflow(path)` and fix every error before showing the owner
