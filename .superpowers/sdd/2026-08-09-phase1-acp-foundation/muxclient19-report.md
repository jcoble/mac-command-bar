# Track C: per-session ACP client routing

Status: complete and verified on 2026-08-12.

## Scope

- Verified: Rust changes are limited to `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs`.
- Verified: no manager, module, protocol, process, or frontend file changed.

## Result

- Verified: `AcpTransport` and `AcpClient` share one `HashMap<SessionId, AcpSessionState>`; each state owns its native ID, commands, config protocol/config, active prompt route, and completed-turn history (`acp_client.rs:40-68`, `acp_client.rs:438-479`).
- Verified: update routing selects `params.sessionId`, then a matching turn, then the sole active prompt/single session for adapters that omit `sessionId` (`acp_client.rs:162-288`, `acp_client.rs:1088-1142`).
- Verified: `available_commands_update` and current-mode changes mutate only the resolved session state (`acp_client.rs:272-287`, `acp_client.rs:1144-1172`).
- Verified: the former `native_session_id: Option<String>`, global `PromptUpdateRoute`, and global conversation-config state are gone. `primary_session_id` is only an alias into the session map and is assigned only when no primary exists (`acp_client.rs:438-479`, `acp_client.rs:884-912`).
- Assumed by contract: Track B decides when multiplexing is allowed. This client contains no capability policy.

## Track B API mapping

| Existing single-session API | Explicit multi-session API | Behavior |
|---|---|---|
| `new_session(cwd) -> StartedAgentSession` | `new_session_multi(cwd) -> SessionId` | Adds a session; the first session remains primary (`acp_client.rs:574-586`). |
| `resume_session(cwd, native_id) -> StartedAgentSession` | `resume_session_multi(cwd, native_id) -> SessionId` | Resumes into the same map without replacing an existing primary (`acp_client.rs:588-612`). |
| `prompt_once(prompt)` | `prompt_on(session_id, prompt)` | Aggregates updates only for the selected session (`acp_client.rs:628-691`). |
| `set_conversation_config(update)` | `set_conversation_config_on(session_id, update)` | Uses the selected session's negotiated config protocol (`acp_client.rs:705-748`). |
| primary model selection through conversation config | `set_model_on(session_id, model_id)` | Thin session-targeted config call (`acp_client.rs:776-793`). |
| `close()` | `close_session(session_id)` | Closes/removes one session without stopping the shared process (`acp_client.rs:795-837`). |
| initial commands from `StartedAgentSession` | `commands_on(session_id)` | Returns the latest captured commands for that session (`acp_client.rs:957-970`). |

The multi-session methods have narrow `dead_code` allowances because Track B is intentionally forbidden from wiring them into `manager.rs` in this lane. Tests exercise every new entry point.

## Tests

- Verified red-first receipt: the initial focused command exited 101 because `new_session_multi`, `prompt_on`, `commands_on`, and `close_session` did not exist in the singleton implementation.
- Verified: `two_sessions_on_one_process_route_independently` interleaves two prompt responses into `ALPHA` and `BRAVO` without cross-talk (`acp_client.rs:1907-1941`).
- Verified: `second_session_new_does_not_clobber_primary` proves the legacy prompt path remains on session 1 (`acp_client.rs:1943-1970`).
- Verified: `per_session_command_capture` proves a session-2 command update does not alter session 1 (`acp_client.rs:1972-2003`).
- Verified: `close_one_session_keeps_the_other_alive` closes session 1 and successfully prompts session 2 (`acp_client.rs:2005-2038`).

## Evidence and verification

- Verified: `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml acp_client` — exit 0; 18 passed, 0 failed, 336 filtered out.
- Verified: `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation` — exit 0; 89 passed, 0 failed, 265 filtered out.
- Verified: `RUSTFLAGS="-D warnings" cargo check --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml` — exit 0.
- Verified: `cargo fmt --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml -- tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs` — exit 0.
- Verified: cargo commands ran serially, one at a time.

## Files touched

- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs`
- `.superpowers/sdd/2026-08-09-phase1-acp-foundation/muxclient19-report.md`

No files were staged, committed, or pushed.
