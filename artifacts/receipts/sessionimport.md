lane sessionimport — import a past transcript into sessions.db

## Test-first receipt

Verified — the five storage tests were added before the implementation.

Command:

```text
MSBUILDDISABLENODEREUSE=1 RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test transcript_import
```

Exit code: 101

```text
   Compiling mac-command-bar-webview-preview v0.1.0 (/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri)
error[E0425]: cannot find type `ImportCursor` in this scope
   --> src/agent_conversation/transcript_import.rs:111:63
    |
111 |     fn stored_cursor(store: &SessionStore, owned_id: &str) -> ImportCursor {
    |                                                               ^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `import_session` in this scope
   --> src/agent_conversation/transcript_import.rs:130:24
    |
130 |         let owned_id = import_session(
    |                        ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `import_session` in this scope
   --> src/agent_conversation/transcript_import.rs:158:24
    |
158 |         let owned_id = import_session(
    |                        ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `import_session` in this scope
   --> src/agent_conversation/transcript_import.rs:178:24
    |
178 |         let owned_id = import_session(
    |                        ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `extend_session` in this scope
   --> src/agent_conversation/transcript_import.rs:189:21
    |
189 |         let added = extend_session(&store, &owned_id, fixture.line_len() * 3, usize::MAX)
    |                     ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `import_session` in this scope
   --> src/agent_conversation/transcript_import.rs:203:24
    |
203 |         let owned_id = import_session(
    |                        ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `extend_session` in this scope
   --> src/agent_conversation/transcript_import.rs:218:21
    |
218 |         let added = extend_session(&store, &owned_id, fixture.len(), usize::MAX)
    |                     ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `import_session` in this scope
   --> src/agent_conversation/transcript_import.rs:235:24
    |
235 |         let owned_id = import_session(
    |                        ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `extend_session` in this scope
   --> src/agent_conversation/transcript_import.rs:257:9
    |
257 |         extend_session(&store, &owned_id, fixture.line_len() * 2, usize::MAX)
    |         ^^^^^^^^^^^^^^ not found in this scope

For more information about this error, try `rustc --explain E0425`.
error: could not compile `mac-command-bar-webview-preview` (bin "mac-command-bar-webview-preview" test) due to 9 previous errors
```

## Final verification

Verified — formatting check passed.

Command:

```text
rustfmt --check --edition 2021 src/agent_conversation/transcript_import.rs
```

Exit code: 0

```text
<no output>
```

Verified — all transcript import tests passed with warnings denied.

Command:

```text
MSBUILDDISABLENODEREUSE=1 RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test transcript_import
```

Exit code: 0

```text
   Compiling mac-command-bar-webview-preview v0.1.0 (/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.84s
     Running unittests src/main.rs (target/debug/deps/mac_command_bar_webview_preview-8a7d2c4b8a5bfb82)

running 10 tests
test agent_conversation::transcript_import::tests::a_window_with_no_usable_line_still_moves_the_cutoff_back ... ok
test agent_conversation::transcript_import::tests::extending_a_session_that_reached_the_start_adds_nothing ... ok
test agent_conversation::transcript_import::tests::extending_prepends_older_records_before_the_existing_ones ... ok
test agent_conversation::transcript_import::tests::import_of_a_short_transcript_reports_it_reached_the_start ... ok
test agent_conversation::transcript_import::tests::import_preserves_other_extra_json_content ... ok
test agent_conversation::transcript_import::tests::import_writes_the_tail_and_records_where_it_stopped ... ok
test agent_conversation::transcript_import::tests::reading_again_from_the_cutoff_walks_backwards_without_gaps ... ok
test agent_conversation::transcript_import::tests::tail_honours_the_record_cap ... ok
test agent_conversation::transcript_import::tests::tail_reaching_the_start_reports_it ... ok
test agent_conversation::transcript_import::tests::tail_stops_at_the_byte_budget ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 405 filtered out; finished in 0.01s
```

Verified — the required build failed, so the lane stopped without a commit.

Command:

```text
RUSTFLAGS="-D warnings" cargo build
```

Exit code: 101

```text
   Compiling mac-command-bar-webview-preview v0.1.0 (/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src-tauri)
error: struct `ImportCursor` is never constructed
  --> src/agent_conversation/transcript_import.rs:18:12
   |
18 | pub struct ImportCursor {
   |            ^^^^^^^^^^^^
   |
   = note: `-D dead-code` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(dead_code)]`

error: struct `ImportedTail` is never constructed
  --> src/agent_conversation/transcript_import.rs:24:12
   |
24 | pub struct ImportedTail {
   |            ^^^^^^^^^^^^

error: function `import_session` is never used
  --> src/agent_conversation/transcript_import.rs:33:8
   |
33 | pub fn import_session(
   |        ^^^^^^^^^^^^^^

error: function `extend_session` is never used
   --> src/agent_conversation/transcript_import.rs:106:8
    |
106 | pub fn extend_session(
    |        ^^^^^^^^^^^^^^

error: function `append_imported_record` is never used
   --> src/agent_conversation/transcript_import.rs:173:4
    |
173 | fn append_imported_record(
    |    ^^^^^^^^^^^^^^^^^^^^^^

error: function `projection_payload` is never used
   --> src/agent_conversation/transcript_import.rs:198:4
    |
198 | fn projection_payload(
    |    ^^^^^^^^^^^^^^^^^^

error: function `import_cursor` is never used
   --> src/agent_conversation/transcript_import.rs:222:4
    |
222 | fn import_cursor(extra_json: &str) -> Result<ImportCursor, String> {
    |    ^^^^^^^^^^^^^

error: function `merge_import_cursor` is never used
   --> src/agent_conversation/transcript_import.rs:234:4
    |
234 | fn merge_import_cursor(extra_json: &str, cursor: &ImportCursor) -> Result<String, String> {
    |    ^^^^^^^^^^^^^^^^^^^

error: function `enum_storage_value` is never used
   --> src/agent_conversation/transcript_import.rs:249:4
    |
249 | fn enum_storage_value<T: Serialize>(value: T) -> Result<String, String> {
    |    ^^^^^^^^^^^^^^^^^^

error: function `record_timestamp` is never used
   --> src/agent_conversation/transcript_import.rs:257:4
    |
257 | fn record_timestamp(record: &ProjectedRecord) -> Option<i64> {
    |    ^^^^^^^^^^^^^^^^

error: function `now_millis` is never used
   --> src/agent_conversation/transcript_import.rs:261:4
    |
261 | fn now_millis() -> i64 {
    |    ^^^^^^^^^^

error: function `read_tail` is never used
   --> src/agent_conversation/transcript_import.rs:603:8
    |
603 | pub fn read_tail(
    |        ^^^^^^^^^

error: could not compile `mac-command-bar-webview-preview` (bin "mac-command-bar-webview-preview") due to 12 previous errors
```

## Current diff

Verified — captured before the required build failure.

```text
 .../src/agent_conversation/transcript_import.rs    | 435 ++++++++++++++++++++-
 1 file changed, 434 insertions(+), 1 deletion(-)
```

Assumed — no claim is made that this lane is complete because the required build failed and no commit was created.
