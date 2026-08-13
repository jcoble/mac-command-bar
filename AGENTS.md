# mac-command-bar

## ⛔ Performance is the reason this app exists

This app is being built because existing tools are slow and heavy. Low CPU, low RAM, low GPU, and a
snappy UI are the product — every feature is secondary to that. Anything added must prove it does not
slow the app down or add idle resource cost.

Rules for all work in this repo:

- **No looping CSS animations at rest.** Pulsing dots, shimmer, spinner loops, and animated gradients
  burn CPU/GPU continuously. Animations run only during a transition, then stop. Anything that must
  loop (a working indicator) pauses when offscreen or when its panel is hidden
  (`content-visibility`, IntersectionObserver, or removing the class).
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
