# mac-command-bar

## ⛔ Every UI dispatch carries the Notion task and the screenshot paths (2026-08-17, owner)

A sub-agent starts cold. It cannot see the conversation, the owner's messages, or any image the
owner pasted — images live in the controller's context, not on the agent's disk. Whatever the
controller retypes from memory is the entire world the agent gets, and a paraphrase silently drops
requirements. That is exactly how a composer restyle shipped with no `+` button, a send button on an
empty field, and unstyled menus, when all three were written down.

Do not paraphrase the owner's decisions. Hand over the two artifacts that already hold them:

1. **The Notion task id**, with instructions to `notion-fetch` it and read the relevant sections
   verbatim. The Command Center task is where the owner's decisions accumulate, in their words,
   including later corrections the controller has forgotten.
2. **Absolute paths to the reference screenshots**, with instructions to `Read` each one before
   touching a file. Screenshots attached to a Notion task are also saved under
   `~/dev/work/reports/tsk-ui-base/screenshots/` — check there first. Recent pastes live in
   `~/.claude/image-cache/<session-id>/`, but that cache is evicted, so a task's images must be
   copied somewhere durable and the path recorded in the task body.

The dispatch then requires the agent to reply with a requirement-by-requirement checklist —
`already done` / `will fix` / `blocked` — BEFORE it edits anything, so a misread surfaces in one
message instead of in a diff the owner has to review by eye.

Corrections go to the SAME agent via `SendMessage` when the fix depends on what it already built.
A fresh agent re-reads everything and repeats the mistakes it cannot see.

## ⛔ Never write a CSS `:has()` selector (purged 2026-08-13, commit `ccd3cc1`)

Not a style preference. Profiling the live app's startup freeze found the WebContent main
thread spending SECONDS inside `:has()` invalidation while the transcript inserted its DOM.
Once any `:has` rule is registered anywhere, WebKit re-checks it on every DOM mutation in the
whole document — so one careless selector taxes every keystroke and every streamed token in
the app, not just the component that declared it.

Every relational selector was replaced with a state class set where the state originates:
`.menu-open` set by whatever owns the open state, `.next-shell-document` toggled by the route
lifecycle, explicit props (`iconPosition`, `hasFooter`, `hasMedia`, `hasKbd`) instead of
descendant inspection. Third-party CSS was not exempt: a vite transform rewrites dockview's
and Monaco's selectors and patches the paired class toggles, keyed to exact source markers so
a dependency upgrade FAILS THE BUILD rather than quietly restoring the cost.

Write the state instead: a class on the element that owns it, a custom property inherited
down, or a pseudo-class scoped to the element that actually has it (`.group:focus-within`,
never `.region:has(:focus)`). Tailwind's `has-[...]` variants compile to `:has()` and are
banned by the same rule. `src` currently greps zero `:has(` — keep it that way.

## ⛔ No UI regression tests until the owner says otherwise (2026-08-17, owner)

Do not write, restore, or re-add any test that asserts on component structure, layout, panels,
menus, styling, colour tokens or contrast. The UI is being reshaped daily and those tests only go
red and get "fixed" to match whatever the UI happens to be, which proves nothing and wastes the
owner's time. Thirty-eight of them were deleted on 2026-08-17 for exactly that reason.

This applies to every lane and sub-agent — paste it into every dispatch. If a change seems to need
one, say so and move on without writing it; the owner will lift this when the UI settles.

Still expected: tests for logic and data — parsing, reducers, view models, stores, protocol
contracts, Rust backend behaviour. A test that would survive a total restyle is fine.

## ⛔ Performance is the reason this app exists

This app is being built because existing tools are slow and heavy. Low CPU, low RAM, low GPU, and a
snappy UI are the product — every feature is secondary to that. Anything added must prove it does not
slow the app down or add idle resource cost.

Rules for all work in this repo:

- **Animations are welcome — every one must END.** Finite, interaction-driven animation (hover
  fades, menu pops, 150-250ms slides on transform/opacity) costs nothing once finished; use it
  generously to make the app feel alive. What is forbidden is animation without an end: infinite
  time-based loops at rest (pulsing dots, shimmer, spinner loops, animated gradients) force the
  compositor to draw every frame forever. A loop is allowed only while tied to real activity (a
  working indicator during an actual turn) and pauses when offscreen or hidden
  (`content-visibility`, IntersectionObserver, or removing the class). Animate transform/opacity,
  never layout properties (width/height/top).
- **No idle timers doing visual work.** Intervals that re-render (sparklines, tickers) must be
  single-flight, scoped to the smallest component (one `<span>`, not a row), and stopped when the
  element is not visible. Prefer event-driven updates over polling.
- **GPU cost is real cost.** `backdrop-filter`, large `blur()`, big `box-shadow` stacks, and
  always-on `transform` layers keep the compositor busy. Use sparingly; never on many elements in a
  scrolling list.
- **Lists must be cheap offscreen.** Long lists (session rail, transcripts) use
  `content-visibility: auto` with `contain-intrinsic-size`, or virtualization.
- **Measure before shipping anything animated or periodic.** Minimum receipt: with the feature idle
  on screen, the WebView content process and app process sit near 0% CPU (`ps -o %cpu -p <pid>`
  sampled over ~10s) and no animation is running (`document.getAnimations().length` at rest,
  long-task count from PerformanceObserver). Interaction latency: clicking a session must stay
  under ~350ms to interactive.
- A correct feature that makes the app feel slower is a defect. Fix or remove it.

## ⛔ One way only — never old and new side by side

A change is finished only when the old way is deleted. Never leave two mechanisms for the same job in
the tree: no compatibility bridges, no "legacy" fallback paths, no adapters kept just in case, no
feature flags that keep both alive, no half-migrated stores where a reader can't tell which one is the
truth. A migration cuts over and removes the old path in the same wave — if the old way can't be
deleted yet, the change isn't done, and the work does not merge. When you find an existing old/new
split, deleting one side is part of whatever task touched it.

## ⛔ KISS and YAGNI are load-bearing rules

- **Keep it simple.** The simplest design that does the job wins. One store, one path, one obvious
  place for each responsibility. If explaining a design takes more than a few sentences, simplify the
  design before writing more code.
- **You aren't gonna need it.** Do not build for imagined futures: no speculative abstraction layers,
  no config for things nobody configures, no generalizing a function that has one caller. Add
  flexibility on the day it is actually needed, not before.

## Explain in HTML, not walls of text

Anything beyond a simple explanation gets a self-contained HTML page — a diagram, a mockup, a
before/after, an annotated flow — instead of paragraphs. Plans and especially specs should be written
as (or paired with) an HTML page under `docs/`; they are far easier to read and review. A mockup HTML
page is the approval artifact for any visual change.

## TypeScript over JavaScript, always

All new frontend/tooling code is TypeScript — including one-off scripts, test scripts, and probes
(`node --experimental-strip-types` runs `.ts` directly, so there is no excuse for `.mjs`). Existing
`.mjs` files under `tauri-svelte-preview/scripts/` convert to `.ts` when a task touches them (one-way
rule: the rename and the change land together). Never add a new `.js`/`.mjs` file.
`any` is a last resort: type things properly — real interfaces, unions, generics, `unknown` with
narrowing — and reach for `any` only when a correct type is genuinely impractical (say why in the
one place it's used).

## Browser viewport — 1710x990, every Playwright/browser session, no exceptions

The desktop testing viewport is 1710x990 (this Mac's 2880x1864 Retina in "More Space" = 1710x1107
points, minus ~117 for menu bar + browser chrome). Half-width screenshots have invalidated whole
test runs; every browser mechanism must pin and VERIFY it:

- playwright-cli: `open <url>`, then `resize 1710 990`, then verify with
  `eval "() => window.innerWidth + 'x' + window.innerHeight"` — must print "1710x990"; resize
  again if not, and re-run after any browser restart.
- Raw playwright scripts: `chromium.launch(...)` + context/page viewport `{ width: 1710, height: 990 }`,
  then the same innerWidth verification before any screenshot or measurement.
- Playwright test-runner configs: pin `viewport: { width: 1710, height: 990 }` INSIDE each
  project's `use` block AFTER any `...devices[...]` spread — the spread otherwise silently drops
  it to 1280x720.
