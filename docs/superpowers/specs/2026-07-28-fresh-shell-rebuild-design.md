# Fresh Shell Rebuild — Design Spec

- Date: 2026-07-28
- Status: **Approved** (user-confirmed via brainstorming, 2026-07-28)
- Relates to: TSK-127 and the layout/session cluster (TSK-324 / TSK-344 / TSK-346 / TSK-360 / TSK-369 / TSK-378)
- Builds on: `2026-06-16-dockview-redesign-design.md` (visual/product direction — still valid) and
  `2026-06-17-live-agent-sessions-design.md` (session model — still valid, §5 carries over)
- Supersedes: the **incremental refactor route** (Phase 0 / Phase 0b decomposition of the existing
  `+page.svelte`). That route consumed significant time, shipped a string of performance regressions
  (scan storms, eager read fan-outs, code-lens whole-project scans, ghost double-scans), and left the
  app slower and less reliable than before it started. **Decision: stop refactoring the old shell;
  build a fresh shell and port deliberately.**

## Why (diagnosis, in brief)

The lag is a **regression, not a platform ceiling**. The app was fast before the refactor; Tauri +
Monaco + xterm in a webview can feel instant (VS Code is webview-class). The slowdown lives in the
frontend layer: a ~19k-line reactive monolith where extractions re-wired state and created cascades —
eager hydration at launch (e.g. auto-restoring the EdiPlatform workspace spawns `csharp-ls` against a
full .NET solution before the user touches anything), `$effect` chains triggering backend IO, and
repeated full re-scans. The Rust backend is **not** the slow part and carries over wholesale.

## Product goal (unchanged from prior specs)

An agent-first command center: **switch between live agent sessions fast, talk to the agent, and read
code like a real IDE** (go-to-def, peek references, code lens). Editing works (Monaco), but fluent
*reading/following* of code is the primary IDE requirement. Terminal, browser, git, file tree — the
things you'd expect from an IDE crossed with the Codex app.

## Decisions

| Question | Decision |
|---|---|
| Route | **Fresh shell rebuild** — not bisect-and-stabilize, not continue-the-refactor |
| Stack | **Svelte 5 + Dockview (dockview-core) + Bits UI + Monaco + xterm**, same as before — the problem was structure, not stack |
| Where it lives | **Same repo, same SvelteKit project, new route** (e.g. `/next`). Shares vite config, deps, `$lib`, and `src-tauri`. Tauri window flips to the new route when it becomes the better daily driver |
| Backend | `src-tauri` + `core/` **untouched and reused wholesale** |
| Old shell | **Frozen** — zero further refactor investment. Kept launchable at the old route as a reference until parity, then deleted |
| Old `$lib` modules/components | **Audit-then-adopt** — reused only after checking for the two poisons (eager IO on mount; `$effect` chains that trigger backend calls). `+page.svelte` is a reference quarry, never imported from |

## The constitution (architecture rules — what keeps the new shell from becoming the old one)

1. **Thin orchestrator.** The shell page stays ~300 lines (hard cap). Every panel is a component.
   All state lives in `.svelte.ts` rune modules with explicit interfaces.
2. **Nothing hydrates at launch except the session rail.** A workspace (source scan, git, LSP)
   hydrates on **first activation of its session**. LSP spawns only when a code view is actually
   opened by user action — never for a background preview.
3. **Effects never do IO.** Backend calls are explicit service functions invoked imperatively;
   results land in stores. Reactivity renders state; it never *fetches* state.
4. **Perf gates at every merge:** cold launch → interactive rail is fast; session switch feels
   instant; a dev-mode **invoke counter** makes any backend call storm visible immediately.

## Sequencing

1. **Autopsy (half-day, timeboxed).** Profile the old shell just enough to write the concrete
   checklist of patterns that made it slow. Not to fix it — to arm the audit in the reuse policy.
2. **Slice 1 — Live session core** (the heart; spec §5 of live-agent-sessions carries over).
   Backend first: `terminal.rs` — don't kill owned PTYs on webview teardown/reload, tombstone
   terminated sessions (`closed`/`exited` flag + final scrollback), add `COMMANDBAR_SESSION_ID` to
   the spawn env. Then frontend: Owned/Resume rail, N live xterm views fed via
   `feedSession` (bind ptySessionId→ownedId before subscribe), Background = instance alive but
   hidden with WebGL addon released, nothing killed on switch.
   **Success:** start 2–3 agents inside the app, switch instantly, talk to each, no TUI corruption,
   hidden agents keep progressing.
3. **Slice 2 — Walking skeleton**, immediately after. The single Gridview-rooted Dockview frame:
   left rail, center tabs, right context, bottom dock — with shallow editor / git / browser panel
   slots. From here on, everything lands as a feature on a stable frame.
4. **Feature lanes on the skeleton**, in priority order:
   1. **Code reading** — Monaco + LSP go-to-def / peek references / find refs / code lens
      (tree-sitter nav index plan feeds this). The #1 IDE requirement.
   2. Git panel (VS Code-style source control).
   3. Browser panel.
   4. Settings & theming (tokens, live-apply Monaco/xterm appearance).

## Carried-over invariants (from the live-agent-sessions spec — still binding)

- Identity: minted **ownedId** is primary; provider ids are mutable; normalize `cmux-*` providers.
- Switching a view never mutates another session's cwd/branch/worktree; missing worktree → copyable
  repair plan, never an auto `git worktree add`; warn on same-worktree collisions.
- LSP: one server per (language), reroot-in-place across worktree roots (already implemented in
  `lsp.rs`); keep servers warm across switches; re-point debounced + cancel-in-flight.
- Lifecycle: Live / Background / Hibernated three-state model; idle-timer hibernate is the RAM
  lever; Background is for fidelity and instant switch.
- Cold restart = cold-resume from snapshot + native resume command; SerializeAddon for cold-path
  rendering.

## Out of scope

Fixing or further decomposing the old shell; split-view; orchestration loops / sub-agent visibility;
scratchpad; rebrand; mobile; multi-window.

## Risks

- **Rewrite gravity:** the skeleton tempts breadth-first porting. Mitigation: slices 1–2 are the only
  structural work; everything else must arrive as an independent feature lane with its own perf gate.
- **Reimporting the disease** via reused `$lib` modules. Mitigation: the autopsy checklist + the
  audit-then-adopt rule.
- **Two shells in one project** (old route + new route) briefly doubles surface. Mitigation: old
  shell is frozen and read-only; delete at parity.
