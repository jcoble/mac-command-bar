# lane tailreader — bounded transcript tail reader

- Verified — `git status --short` showed unrelated tracked and untracked work before this lane began; this lane will stage only the new reader, its parent module declaration, and this receipt.
- Verified — `src/agent_conversation/transcript/mod.rs` already provides `read_range`, `complete_lines`, `parse_durable_line`, and `ProjectedRecord`; no duplicate reader primitives were added.
- Assumed — Codex message records are sufficient fixtures for provider-independent byte-window and record-cap behavior because the reader delegates provider-specific parsing to the existing parser.

## Tests before implementation

- Verified — all four named tests were written before the implementation and observed failing only at the intentional `unimplemented!` placeholder.

```text
$ MSBUILDDISABLENODEREUSE=1 RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test transcript_import
    Blocking waiting for file lock on artifact directory
   Compiling mac-command-bar-webview-preview v0.1.0 (/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 20.54s
     Running unittests src/main.rs (target/debug/deps/mac_command_bar_webview_preview-8a7d2c4b8a5bfb82)

running 4 tests
test agent_conversation::transcript_import::tests::reading_again_from_the_cutoff_walks_backwards_without_gaps ... FAILED
test agent_conversation::transcript_import::tests::tail_honours_the_record_cap ... FAILED
test agent_conversation::transcript_import::tests::tail_reaching_the_start_reports_it ... FAILED
test agent_conversation::transcript_import::tests::tail_stops_at_the_byte_budget ... FAILED

failures:

---- agent_conversation::transcript_import::tests::reading_again_from_the_cutoff_walks_backwards_without_gaps stdout ----

thread 'agent_conversation::transcript_import::tests::reading_again_from_the_cutoff_walks_backwards_without_gaps' (271188) panicked at src/agent_conversation/transcript_import.rs:155:5:
not implemented: bounded transcript tail reader
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- agent_conversation::transcript_import::tests::tail_honours_the_record_cap stdout ----

thread 'agent_conversation::transcript_import::tests::tail_honours_the_record_cap' (271189) panicked at src/agent_conversation/transcript_import.rs:155:5:
not implemented: bounded transcript tail reader

---- agent_conversation::transcript_import::tests::tail_reaching_the_start_reports_it stdout ----

thread 'agent_conversation::transcript_import::tests::tail_reaching_the_start_reports_it' (271190) panicked at src/agent_conversation/transcript_import.rs:155:5:
not implemented: bounded transcript tail reader

---- agent_conversation::transcript_import::tests::tail_stops_at_the_byte_budget stdout ----

thread 'agent_conversation::transcript_import::tests::tail_stops_at_the_byte_budget' (271191) panicked at src/agent_conversation/transcript_import.rs:155:5:
not implemented: bounded transcript tail reader

failures:
    agent_conversation::transcript_import::tests::reading_again_from_the_cutoff_walks_backwards_without_gaps
    agent_conversation::transcript_import::tests::tail_honours_the_record_cap
    agent_conversation::transcript_import::tests::tail_reaching_the_start_reports_it
    agent_conversation::transcript_import::tests::tail_stops_at_the_byte_budget

test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 405 filtered out; finished in 0.00s

error: test failed, to rerun pass `--bin mac-command-bar-webview-preview`
```
