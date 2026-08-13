# sqlstore19 Track A report

Status: complete

## Files touched

- `.superpowers/sdd/2026-08-09-phase1-acp-foundation/sqlstore19-report.md`
- `core/Cargo.lock`
- `core/Cargo.toml`
- `core/src/lib.rs`
- `core/src/session_store.rs`

## Contract deviations

- `rusqlite` is pinned to stable `0.37.0` instead of latest `0.40.2` because the app already links `0.37.0`; using `0.40.2` creates an unsatisfiable duplicate native SQLite link without a forbidden app-crate change.

## Verification

- `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path core/Cargo.toml session_store` — passed: 10 passed, 0 failed; all unrelated test targets selected 0 tests.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path core/Cargo.toml` — passed.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml` — passed.
- `cargo fmt --manifest-path core/Cargo.toml` — completed; only the touched Rust files remain formatted, and unrelated formatter changes were removed.
- WAL test proof: an independent read transaction prevents close-time checkpointing, the WAL remains nonempty after the writer is dropped, and a newly opened store reads the committed session and event.
