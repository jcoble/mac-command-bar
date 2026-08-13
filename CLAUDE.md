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
