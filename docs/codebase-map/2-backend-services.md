# Rust backend services

## What this portion is

This portion is the native backend behind the Assembly desktop window. `main.rs` registers the Tauri commands and creates the process-wide registries; the modules behind it provide source discovery, language-server access, terminals, Git and pull-request operations, browser tabs, process and disk resources, provider usage, orchestration, and workflow execution. The `core` crate contains the bounded disk scanners, source preview logic, one-pass reference counter, dispatcher, and AES-GCM helper used by those services. (tauri-svelte-preview/src-tauri/src/main.rs:1-45; core/src/lib.rs:1-7)

## Files and what each one does

| File | What it does |
| --- | --- |
| `tauri-svelte-preview/src-tauri/src/main.rs` | Defines the Tauri-facing data shapes, command wrappers, source-scan registry, launch setup, capability list, and generated command handler. (tauri-svelte-preview/src-tauri/src/main.rs:47-219, 613-5913) |
| `tauri-svelte-preview/src-tauri/src/terminal.rs` | Starts and owns native PTYs, keeps session metadata and bounded scrollback, forwards output events, resizes sessions, and closes them. (tauri-svelte-preview/src-tauri/src/terminal.rs:13-91, 93-475) |
| `tauri-svelte-preview/src-tauri/src/lsp.rs` | Discovers language-server executables, starts one session per language and canonical workspace root, speaks framed LSP, exposes status and editor requests, and manages native Roslyn C# endpoints. (tauri-svelte-preview/src-tauri/src/lsp.rs:27-66, 1588-1774, 4516-4746) |
| `tauri-svelte-preview/src-tauri/src/git_workspace.rs` | Builds and runs direct `git` and `gh` argument vectors for branches, stashes, worktree changes, pull-request summaries, and destructive path operations. (tauri-svelte-preview/src-tauri/src/git_workspace.rs:1-14, 79-197, 355-616) |
| `tauri-svelte-preview/src-tauri/src/git_diff_models.rs` | Reads bounded commit blobs and returns the file-change and diff models used by the Git views. (tauri-svelte-preview/src-tauri/src/git_diff_models.rs:1-86) |
| `tauri-svelte-preview/src-tauri/src/git_pr.rs` | Generates commit and pull-request text through the supplied conversation manager, reads GitHub pull-request state with `gh`, and creates a pull request after pushing. (tauri-svelte-preview/src-tauri/src/git_pr.rs:1-6, 61-160, 200-236) |
| `tauri-svelte-preview/src-tauri/src/browser.rs` | Owns one native browser child view per workspace and tab, navigation and bounds, bounded element inspection, annotation picking, viewport capture, and browser profile data. (tauri-svelte-preview/src-tauri/src/browser.rs:1-8, 395-445, 479-1129) |
| `tauri-svelte-preview/src-tauri/src/resources.rs` | Samples CPU and memory, joins owned process trees, keeps short history, scans protected disk entries, and performs validated process/resource actions. (tauri-svelte-preview/src-tauri/src/resources.rs:1-35, 37-156, 325-442, 906-1368) |
| `tauri-svelte-preview/src-tauri/src/resources_disk.rs` | Finds bounded workspace and application-store disk entries, caches the report, and revalidates a selected reclaim operation. (tauri-svelte-preview/src-tauri/src/resources_disk.rs:1-41, 80-141, 177-239, 391-499) |
| `tauri-svelte-preview/src-tauri/src/usage_current.rs` | Converts provider-authored live quota data into an available, unavailable, or error snapshot. (tauri-svelte-preview/src-tauri/src/usage_current.rs:5-34, 40-103, 150-169) |
| `tauri-svelte-preview/src-tauri/src/usage_db.rs` | Opens the usage-history SQLite database, applies migrations, stores events and cursors, and runs filtered summary, breakdown, provider, and daily aggregate queries. (tauri-svelte-preview/src-tauri/src/usage_db.rs:15-201, 203-369, 372-795) |
| `tauri-svelte-preview/src-tauri/src/usage_history.rs` | Defines the usage-history query and command surface and selects the database path. (tauri-svelte-preview/src-tauri/src/usage_history.rs:12-119) |
| `tauri-svelte-preview/src-tauri/src/usage_indexer.rs` | Incrementally tails the local provider files, carries session context across passes, inserts new events and cursors transactionally, and rebuilds rollups. (tauri-svelte-preview/src-tauri/src/usage_indexer.rs:9-117) |
| `tauri-svelte-preview/src-tauri/src/usage_remote.rs` | Fetches remote quota data with a five-minute success cache and a retry gate. (tauri-svelte-preview/src-tauri/src/usage_remote.rs:9-18, 20-46, 93-157) |
| `tauri-svelte-preview/src-tauri/src/usage_sources.rs` | Discovers Claude and Codex JSONL sources, reads complete newline records incrementally, parses token/context usage, and derives stable file identity. (tauri-svelte-preview/src-tauri/src/usage_sources.rs:10-45, 47-151, 154-342, 621-654) |
| `tauri-svelte-preview/src-tauri/src/claude_quota.rs` | Reads Claude credentials, refreshes OAuth credentials when required, calls the usage endpoint, normalizes provider windows, and persists refreshed credentials atomically. (tauri-svelte-preview/src-tauri/src/claude_quota.rs:15-24, 26-207, 306-408, 612-871) |
| `tauri-svelte-preview/src-tauri/src/orchestration.rs` | Appends and reduces orchestration events from `sessions.db`, imports the former JSONL ledger once, and returns run projections with explicit opt-in demo rows. |
| `tauri-svelte-preview/src-tauri/src/workflow.rs` | Defines workflow documents and contracts, validates graphs and policies, schedules ready nodes, and persists deterministic lifecycle events through the orchestration ledger. (tauri-svelte-preview/src-tauri/src/workflow.rs:16-447, 766-995, 997-1844) |
| `tauri-svelte-preview/src-tauri/src/debug_log.rs` | Mirrors stderr to a bounded rotating backend log without panicking when the output pipe is closed. (tauri-svelte-preview/src-tauri/src/debug_log.rs:7-68, 70-144) |
| `tauri-svelte-preview/src-tauri/src/product_identity.rs` | Supplies the product name, terminal/LSP identity, and startup failure text. (tauri-svelte-preview/src-tauri/src/product_identity.rs:1-4) |
| `core/src/scanners/mod.rs` | Exports the disk, process, resource, session, and worktree scanners. (core/src/scanners/mod.rs:1-5) |
| `core/src/scanners/disk.rs` | Measures explicit roots, classifies protection, and returns bounded disk entries and totals. (core/src/scanners/disk.rs:6-185) |
| `core/src/scanners/processes.rs` | Parses `lsof` listeners and supplements them with process working directories and arguments. (core/src/scanners/processes.rs:5-117) |
| `core/src/scanners/resources.rs` | Joins process trees to registered app-owned roots and returns resource snapshots. (core/src/scanners/resources.rs:6-93, 247-413) |
| `core/src/scanners/sessions.rs` | Scans bounded Claude, Codex, and cmux session sources and returns merged `AgentSessionRecord` rows. (core/src/scanners/sessions.rs:37-94, 189-334) |
| `core/src/scanners/worktrees.rs` | Reads Git worktree porcelain, status, branch reachability, size, and activity into cleanup-eligibility records. (core/src/scanners/worktrees.rs:5-25, 32-81, 220-258) |
| `core/src/dispatcher.rs` | Dispatches named scanner and health actions and returns JSON values or a non-executing kill plan. (core/src/dispatcher.rs:9-23, 26-175) |
| `core/src/source.rs` | Previews UTF-8 source, recursively lists source files, detects languages and syntax roles, and returns bounded diagnostics. (core/src/source.rs:12-103, 105-192, 194-390) |
| `core/src/reference_counts.rs` | Counts requested symbol mentions by line in one bounded, parallel pass over source files. (core/src/reference_counts.rs:1-10, 17-33, 88-233) |
| `core/src/crypto.rs` | Encrypts and decrypts byte blobs with AES-256-GCM and a random nonce. (core/src/crypto.rs:9-52) |
| `core/src/lib.rs` | Exports the core modules. (core/src/lib.rs:1-7) |

## How it works today

### App launch and the Tauri command contract

`main` installs the panic hook, creates the Tauri builder, registers the source-scan, terminal projection, LSP, terminal, browser, resource, and usage-history registries, and installs the dialog and notification plugins. During setup it creates the application-data directory, installs the `logs/backend.log` mirror, opens the agent runtime with bundled provider configuration and `sessions.db`, validates the initial session list, creates the workflow engine, collects live owned-session identifiers, starts the startup reaper, installs the `agent-conversation-event` emitter, stores the workflow engine and runtime, and installs the LSP status event listener. (tauri-svelte-preview/src-tauri/src/main.rs:5681-5735)

The generated handler is the exact backend contract. Each name below is a separate command registered by `tauri::generate_handler!`; the `agent_conversation` implementations are outside this report's scope. (tauri-svelte-preview/src-tauri/src/main.rs:5737-5905)

**Capabilities** (tauri-svelte-preview/src-tauri/src/main.rs:1429-1505)

- `read_backend_capabilities` returns the capability-name list.

**Source files and source actions** (tauri-svelte-preview/src-tauri/src/main.rs:613-832)

- `list_source_files` scans a root with an optional limit, query, and scan identifier.
- `cancel_source_scan` cancels a registered source scan.
- `validate_project_root` validates a project root.
- `read_source_file` reads a bounded UTF-8 source preview.
- `read_native_csharp_file` reads a C# file after canonical-root containment checks.
- `write_source_file` writes source text and returns the resulting preview.
- `open_source_file` opens a source file with the platform action.
- `reveal_source_file` reveals a source file.
- `open_path` opens an arbitrary validated path action.
- `reveal_path` reveals an arbitrary validated path action.
- `open_terminal_path` opens a path in a selected terminal.
- `open_terminal_command` opens a path and runs one command in a selected terminal.
- `search_source_files` searches the supplied source records.
- `find_source_definitions` finds definition targets in the supplied records.
- `find_source_references` finds reference targets in the supplied records.
- `count_source_references` counts requested symbols in one project pass.

The source scan result carries `records`, the applied `limit`, a `truncated` flag, and scan statistics; each record carries the absolute path, relative path, file name, language, and byte count. The reference-count result carries counts, an `approximate` flag, scanned-file count, elapsed milliseconds, and a skipped-file count; the approximate flag means a deadline or file ceiling was reached, while zero means the symbol was not seen in the scanned set. (tauri-svelte-preview/src-tauri/src/main.rs:74-219)

All heavy source wrappers use `spawn_blocking`; source scans can emit `source_scan_progress` with the scan identifier, visited-entry count, and matched-file count. (tauri-svelte-preview/src-tauri/src/main.rs:613-681)

**Language-server lifecycle and editor requests** (tauri-svelte-preview/src-tauri/src/main.rs:834-1010, 1109-1355)

- `read_source_lsp_status` reads one root/language status.
- `list_source_lsp_statuses` lists readiness statuses for a root.
- `warm_source_lsp_for_root` re-points already-running servers while a root is selected.
- `ensure_native_csharp_language_client` returns the native C# endpoint when the root contains a C# marker and full mode is on.
- `mark_native_csharp_language_client_ready` records that the native editor client is ready.
- `set_csharp_language_server_enabled` turns the process-wide C# server permission on or off and stops running C# servers when disabled.
- `read_workspace_language_intelligence` reads one workspace's read/full mode and server process identifiers.
- `set_workspace_language_intelligence` records that mode, stops servers when turning it off, and optionally starts the selected language.
- `find_source_lsp_definitions` asks the server for definitions.
- `find_source_lsp_completions` asks for completion items.
- `find_source_lsp_implementations` asks for implementations.
- `find_source_lsp_type_definitions` asks for type definitions.
- `find_source_lsp_document_highlights` asks for document highlights.
- `find_source_lsp_signature_help` asks for signature help.
- `find_source_lsp_inlay_hints` asks for inlay hints.
- `find_source_lsp_semantic_tokens` asks for semantic tokens.
- `find_source_lsp_workspace_symbols` asks for workspace symbols.
- `format_source_with_lsp` asks for text edits that format a source document.
- `rename_source_with_lsp` asks for a workspace rename edit.
- `find_source_lsp_code_actions` asks for code actions.
- `find_source_lsp_references` asks for references.
- `find_source_lsp_hover` asks for hover information.
- `find_source_lsp_symbols` asks for the generic symbol lookup.
- `find_source_lsp_document_symbols` asks for a bounded document-symbol list.
- `read_source_lsp_log` reads the retained language-server log lines.
- `read_source_lsp_diagnostics` reads diagnostics for one file through the LSP registry.
- `list_source_lsp_diagnostics_for_root` reads diagnostics for a whole root.

The LSP status-change event is `source-lsp-status-changed`; it carries the root, language, state, and optional detail, and is emitted when a server starts, finishes project loading, or stops. (tauri-svelte-preview/src-tauri/src/lsp.rs:282-325; tauri-svelte-preview/src-tauri/src/main.rs:5728-5734)

**Git repository operations** (tauri-svelte-preview/src-tauri/src/main.rs:1357-1427, 1507-1580)

- `project_git_status` reads repository status.
- `read_source_git_diff` reads the bounded working-tree diff.
- `stage_git_paths` stages exact paths.
- `unstage_git_paths` unstages exact paths.
- `commit_git_repository` creates a commit.
- `fetch_git_repository` fetches repository refs.
- `pull_git_repository` pulls repository changes.
- `push_git_repository` pushes repository changes.
- `read_git_commit_history` reads bounded commit history.
- `read_git_commit_files` lists files changed by one commit.
- `read_git_commit_file_diff` reads one commit-file diff.
- `git_pr::generate_commit_message` asks the supplied conversation session for a commit message.
- `git_pr::read_pull_request_context` reads branch, base, commits, and bounded diff context.
- `git_pr::generate_pull_request_details` asks the supplied conversation session for pull-request JSON details.
- `git_pr::create_pull_request` pushes and creates a GitHub pull request.
- `git_pr::read_pull_request_status` reads pull-request state and checks.
- `git_workspace::discard_git_paths` discards selected tracked and untracked paths.
- `git_workspace::discard_all_git_changes` discards all tracked and untracked changes under the validated root.
- `git_workspace::list_git_branches` lists local branches.
- `git_workspace::create_git_branch` creates a branch.
- `git_workspace::switch_git_branch` switches branches.
- `git_workspace::stash_git_changes` creates a stash.
- `git_workspace::pop_git_stash` applies and removes a stash.
- `git_workspace::list_git_stashes` lists stashes.
- `git_workspace::amend_git_commit` amends `HEAD`.
- `git_workspace::list_open_pull_requests` lists open pull requests through `gh`.
- `list_project_worktrees` lists worktrees.
- `list_project_git_refs` lists refs.
- `remove_project_worktree` removes one worktree, with an optional force flag.
- `archive_project_worktree` archives one worktree.
- `list_git_repository_summaries` returns repository summaries for runtime projects.

**Sessions, processes, and resource actions** (tauri-svelte-preview/src-tauri/src/main.rs:1582-1635, 5819-5830)

- `list_agent_sessions` scans the local session sources and returns merged session records.
- `list_runtime_contexts` returns runtime context rows for supplied projects.
- `list_playwright_sessions` lists detected Playwright sessions.
- `kill_playwright_session` stops one Playwright process group.
- `kill_playwright_sessions` stops all detected Playwright process groups.
- `kill_process` sends the polite stop signal to one displayed process.
- `resources::read_resource_sample` returns the current resource sample.
- `resources::read_resource_snapshot` returns a grouped resource snapshot.
- `resources::read_resource_disk_scan` returns protected disk-scan entries.
- `resources::cleanup_workspace_disk_entry` removes one revalidated disk entry.
- `resources::stop_owned_resource` stops one registered owned resource.
- `resources::stop_resource_process_tree` stops one validated process tree.
- `resources_disk::read_resource_disk_usage` reads the cached resource-disk report.
- `resources_disk::reclaim_resource_disk_entry` revalidates and removes one reclaimable entry.
- `resources::restart_language_server_root` is the language-server restart action endpoint.
- `resources::set_active_source_root` sets the active source root for resource grouping.
- `resources::apply_resource_memory_pressure` is the memory-pressure action endpoint.
- `resources::read_language_server_log` is the resource-panel language-server log endpoint.

**Provider usage and usage history** (tauri-svelte-preview/src-tauri/src/main.rs:5550-5654, 5831-5840)

- `usage_current::read_current_provider_usage` returns authoritative current provider windows or an unavailable state.
- `usage_history::read_usage_summary` reads filtered usage summary rows.
- `usage_history::read_usage_breakdown` reads filtered breakdown rows.
- `usage_history::read_usage_provider_summary` reads provider summary rows.
- `usage_history::read_usage_daily` reads daily usage rows.
- `usage_history::read_usage_daily_totals` reads daily totals.
- `read_usage_token_breakdown` reads token-breakdown rows.
- `read_usage_provider_daily_totals` reads provider/day totals.
- `read_usage_cost_inputs` reads cost-input rows.
- `usage_history::refresh_usage_history` runs the incremental local indexer.

**Orchestration and workflow** (tauri-svelte-preview/src-tauri/src/main.rs:1637-1798, 5841-5852)

- `list_orchestration_runs` reduces the orchestration ledger for the supplied projects.
- `record_orchestration_event` appends one legacy orchestration event and returns its run projection.
- `list_workflow_runs` lists workflow projections from the managed engine.
- `create_workflow_run` validates and creates a workflow run.
- `start_workflow_run` starts a run.
- `pause_workflow_run` pauses a run.
- `resume_workflow_run` resumes a run.
- `cancel_workflow_run` cancels a run.
- `retry_workflow_node` retries one node.
- `skip_workflow_node` skips one node.
- `approve_workflow_gate` records a gate approval.
- `submit_workflow_result` submits one node result.

Each workflow command emits `workflow-run-updated` with the resulting run record. (tauri-svelte-preview/src-tauri/src/main.rs:1660-1798)

**Agent-conversation commands registered by `main.rs`** (implementation excluded from this report) (tauri-svelte-preview/src-tauri/src/main.rs:5853-5882)

- `agent_conversation::ensure_agent_conversation` ensures a conversation session.
- `agent_conversation::send_agent_conversation_message` sends a conversation message.
- `agent_conversation::agent_conversation_set_session_draft` stores a session draft.
- `agent_conversation::agent_conversation_get_session_draft` reads a session draft.
- `agent_conversation::agent_conversation_clear_session_draft` clears a session draft.
- `agent_conversation::respond_agent_conversation_approval` responds to an approval request.
- `agent_conversation::respond_agent_conversation_permission` responds to a permission request.
- `agent_conversation::respond_agent_conversation_input` responds to an input request.
- `agent_conversation::stop_agent_conversation_turn` stops a turn.
- `agent_conversation::set_agent_conversation_config` sets conversation configuration.
- `agent_conversation::set_agent_conversation_config_option` sets one configuration option.
- `agent_conversation::read_agent_conversation_config` reads conversation configuration.
- `agent_conversation::read_agent_conversation_capabilities` reads conversation capabilities.
- `agent_conversation::close_agent_conversation` closes a conversation.
- `agent_conversation::read_agent_conversation_snapshot` reads a conversation snapshot.
- `agent_conversation::list_agent_conversation_sessions` lists conversation sessions.
- `agent_conversation::list_agent_conversation_events` lists conversation events.
- `agent_conversation::update_agent_conversation_session_meta` updates session metadata.
- `agent_conversation::read_agent_conversation_transcript` reads a transcript.
- `agent_conversation::import_agent_conversation_transcript` imports a transcript.
- `agent_conversation::extend_agent_conversation_import` extends an import.
- `agent_conversation::start_agent_conversation_terminal_projection` starts terminal projection.
- `agent_conversation::stop_agent_conversation_terminal_projection` stops terminal projection.
- `agent_conversation::save_agent_conversation_attachment` saves an attachment.
- `agent_conversation::read_agent_conversation_attachments` reads attachments.
- `agent_conversation::delete_agent_conversation_attachment` deletes an attachment.
- `agent_conversation::agent_conversation_add_session_annotation` adds a session annotation.
- `agent_conversation::agent_conversation_list_session_annotations` lists session annotations.
- `agent_conversation::agent_conversation_delete_session_annotation` deletes a session annotation.
- `agent_conversation::handoff::handoff_agent_conversation` performs conversation handoff.

**Terminals** (tauri-svelte-preview/src-tauri/src/main.rs:1801-1850)

- `start_terminal_session` starts a PTY session.
- `list_terminal_sessions` lists terminal metadata.
- `read_terminal_session_scrollback` reads retained output.
- `write_terminal_session` writes input.
- `resize_terminal_session` resizes the PTY.
- `close_terminal_session` closes the session.

**Browser tabs** (tauri-svelte-preview/src-tauri/src/main.rs:5889-5905)

- `browser::create_browser_tab` creates or reuses a native tab.
- `browser::set_browser_tab_bounds` sets the child-view bounds.
- `browser::set_browser_tab_viewport` sets the viewport size.
- `browser::show_browser_tab` shows one tab.
- `browser::hide_browser_workspace` hides the browser workspace.
- `browser::navigate_browser_tab` navigates a tab.
- `browser::reload_browser_tab` reloads a tab.
- `browser::go_back_browser_tab` goes back.
- `browser::go_forward_browser_tab` goes forward.
- `browser::close_browser_tab` closes a tab.
- `browser::clear_browser_workspace_data` removes a browser profile after closing its tabs.
- `browser::arm_browser_element_picker` arms bounded element picking.
- `browser::cancel_browser_element_picker` cancels element picking.
- `browser::inspect_browser_rect` inspects a bounded page rectangle.
- `browser::capture_browser_viewport` captures a PNG page snapshot.
- `browser::open_browser_tab_devtools` opens native developer tools.
- `browser::open_browser_tab_external` opens the tab URL externally.

When the window is destroyed, `main` shuts down the browser registry; the Tauri run call uses the product-specific startup error text from `product_identity.rs`. (tauri-svelte-preview/src-tauri/src/main.rs:5907-5913; tauri-svelte-preview/src-tauri/src/product_identity.rs:1-4)

### Core source discovery and past-session discovery

`core::source` rejects non-files, files above the 512 KiB preview ceiling, invalid UTF-8, and unsupported syntax; it parses C#, TypeScript, JavaScript, and JSX with tree-sitter where a grammar is present. A source preview contains the path, language, byte count, full text, and syntax-role spans. (core/src/source.rs:12-22, 105-141)

The source list validates the directory, changes a zero limit to 10,000, clamps the maximum to 25,000, lowercases the optional query, recursively collects source extensions while skipping the configured directories, sorts records, and returns records plus truncation and diagnostics. Plain text files are excluded from the source-language list. (core/src/source.rs:143-192, 258-390)

The session scanner reads the home directory. It looks in Codex session and archive directories, excludes subagent paths before the file budget, reads an index and bounded head/tail windows, reads the cmux hook source, reads bounded Claude project files, merges by provider plus session identifier, sorts by activity, truncates to the combined result limit, and derives branch, task, pull-request, and source labels. An `AgentSessionRecord` carries provider, identifier, title, optional description/model/project/activity, resume commands, optional transcript path, derived metadata, a bounded message-count floor, and a latest-turn preview. (core/src/scanners/sessions.rs:37-94, 189-334)

The scanner's message count is a floor over the bounded window. User turns and agent prose count; tool calls and their results do not. Consecutive Codex prose records count as one agent turn so the field has the same meaning as the Claude row. (core/src/scanners/sessions.rs:68-91)

The dispatcher accepts named actions for worktree, process, session, health, source-list, and source-preview operations. Its process action returns a confirmable kill plan and does not execute a kill. Unknown actions return an error. (core/src/dispatcher.rs:9-23, 26-175)

### Terminal and PTY model

`TerminalRegistry` stores a map of session identifier to a PTY master, child killer, writer, session metadata, and scrollback under an `Arc<Mutex<...>>`. `TerminalKind` distinguishes `user-pty`, `agent-tool`, `run-configuration`, and `browser-automation`. A start request contains the working directory, optional shell, optional columns and rows, optional owned identifier, and optional one-shot command. (tauri-svelte-preview/src-tauri/src/terminal.rs:25-91)

Starting a session canonicalizes the working directory, selects the requested shell or the environment/default shell, clamps the size to 20–300 columns and 4–100 rows, chooses the supplied tool identity or a collision-resistant generated identifier, opens a native PTY, starts the shell as a login shell, sets `TERM`, color support, `TERM_PROGRAM=Assembly`, and `COMMANDBAR_SESSION_ID`, and inserts the session before starting the reader and waiter threads. A non-empty `command` is passed as one command and the child exits with its real status; an omitted command leaves an interactive shell. (tauri-svelte-preview/src-tauri/src/terminal.rs:93-198, 445-551)

The reader reads up to 8,192 bytes at a time, appends bytes to the session scrollback, and emits `terminal_output` events that carry the session identifier, output text, and a non-terminal marker. Scrollback is capped at 16 MiB; once over the cap one trim drains to 75 percent, finding a UTF-8 boundary by jumping to the target and nudging at most three bytes. This keeps reattach output in the backend even while a view is hidden. (tauri-svelte-preview/src-tauri/src/terminal.rs:13-23, 361-415)

The waiter records exit code or signal, marks the session as an exited tombstone, and emits a terminal output event with the terminated marker. Listing retains tombstones until explicit close; closing a live session kills first and removes only after the kill succeeds, while closing a tombstone only removes its record. Invalid identifiers return no-op or false results as defined by each command. (tauri-svelte-preview/src-tauri/src/terminal.rs:201-358, 417-443, 560-685)

### Language servers and native C#

The backend recognizes C#, TypeScript, TSX, JavaScript, JSX, Rust, and Svelte. Ordinary LSP requests have a six-second timeout; diagnostics have a 1.2-second timeout; cancelled requests retry the server cancel after 180 milliseconds. Document symbols are limited to 500 symbols and 4 MiB, and server logs retain 200 lines. A language keeps at most five workspace sessions. (tauri-svelte-preview/src-tauri/src/lsp.rs:27-66)

Read mode is the default. Opening a file in read mode performs colouring and does not start a server; turning full language-intelligence mode on records the workspace and may start the selected language, while turning it off stops that workspace's server processes. The process-wide C# switch is separate because status reads can occur before the registry exists; turning it off stops existing C# servers and leaves plain search and reference counts available. (tauri-svelte-preview/src-tauri/src/lsp.rs:68-128; tauri-svelte-preview/src-tauri/src/main.rs:898-1009)

For C#, the server specification is `roslyn-language-server --stdio --autoLoadProjects --telemetryLevel off`; TypeScript and TSX use `typescript-language-server --stdio`, JavaScript and JSX use the same executable with the JavaScript language identifier, Rust uses `rust-analyzer`, and Svelte uses `svelteserver --stdio`. Other language identifiers have no server specification. (tauri-svelte-preview/src-tauri/src/lsp.rs:4516-4555)

Executable discovery checks the app's `node_modules/.bin`, a cached login-shell `PATH`, the inherited `PATH`, `$HOME/.cargo/bin`, `.dotnet/tools`, `.local/bin`, nvm bins, Homebrew and system bin directories, and requires an executable file. C# startup first performs a bounded shallow marker scan for `.sln`, `.slnx`, or `.csproj`. (tauri-svelte-preview/src-tauri/src/lsp.rs:4558-4746)

Each `(language, canonical root)` has a registry slot. The connection sends LSP initialize data with Assembly's client name, root URI, workspace folders, and declared capabilities, then sends `initialized`; a reader task routes responses by request identifier so hover, references, semantic tokens, and other questions can be outstanding together. Requests use framed `Content-Length` messages, document synchronization follows the LSP open/change rules, and timeout cancellation sends `$/cancelRequest`. (tauri-svelte-preview/src-tauri/src/lsp.rs:1038-1283, 1388-1448; commit `6acb2db`)

Status readings include language identifier, availability, server name and command, an optional reason, state, and a plain detail sentence; the request supplies the root. A status listener emits `source-lsp-status-changed` instead of requiring polling. Diagnostics are read per file or per root, and `read_source_lsp_log` returns the retained server stderr lines; its root parameter is accepted but the implementation uses one log for the language server instance. (tauri-svelte-preview/src-tauri/src/lsp.rs:282-325, 495-511, 1285-1386, 1588-1601, 2172-2303; tauri-svelte-preview/src-tauri/src/main.rs:1301-1317)

The native C# path starts a Roslyn endpoint per canonical root, coalesces concurrent ensure calls into one slot, exposes the endpoint to the native editor, and stops the whole process tree after a 1.5-second shutdown grace. Workspace warming only re-points live existing sessions; it does not start a server for an unused language. (tauri-svelte-preview/src-tauri/src/lsp.rs:1680-1774, 2172-2303, 2435-2620; commit `bd0f9d2`; commit `99fba29`)

The one failing LSP test recorded for this portion is `lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint`. Its assertions unwrap two concurrent endpoint results and then require the same workspace URL, the same canonical root, and one native-session map entry; the recorded serial test receipt failed earlier at the endpoint unwrap with `Could not bind native C# bridge: Operation not permitted (os error 1)`. (tauri-svelte-preview/src-tauri/src/lsp.rs:6713-6743; `docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md:57-68`)

### Git operations

The Git workspace module builds argument arrays and passes them directly to `Command`; it does not invoke a shell. Discard operations validate the repository root and exact paths, split tracked and untracked paths, use `git restore --source=HEAD --staged --worktree --` for tracked paths and `git clean -f -d --` for untracked paths, and do not use `-x`. The all-changes path applies the same two operations to the validated root. (tauri-svelte-preview/src-tauri/src/git_workspace.rs:1-14, 79-118, 199-343)

Branch and stash operations call `git branch`, `git switch`, `git stash push`, `git stash pop`, and `git stash list` with explicit arguments. Amend checks `HEAD` before calling `git commit --amend`; open pull requests call `gh pr list` with a limit of 30 and parse the returned fields. Git status, history, commit file lists, and file diffs are similarly direct commands with bounded result models. (tauri-svelte-preview/src-tauri/src/git_workspace.rs:355-616; tauri-svelte-preview/src-tauri/src/git_diff_models.rs:10-86)

The diff model reads `git show` output and disk content into two text blobs bounded at 512 KiB. New or deleted blobs can be empty; NUL bytes, invalid UTF-8, and over-limit content are rejected. (tauri-svelte-preview/src-tauri/src/git_diff_models.rs:1-86)

The pull-request module receives an ACP session identifier and uses `manager.prompt_once` to generate commit-message or pull-request-detail text; it does not fall back to a separate chat path. Context is assembled from branch/base, commits, and staged/working diffs with a 160,000-character bound. Creation pushes first and then invokes `gh pr create`; status invokes `gh pr view` and parses checks. (tauri-svelte-preview/src-tauri/src/git_pr.rs:1-6, 61-160, 238-283, 292-450)

### Browser tabs

The browser registry keeps one native child view per `(workspace, tab)` and tracks a generation number, URL, bounds, viewport, profile, and picker state. Creation validates workspace/tab identity, URL, bounds, and profile; a repeated request with the same generation and URL reuses the tab. The native factory accepts HTTP or HTTPS URLs, creates profile data under `<app_data>/browser-profiles/<uuid>`, starts the view hidden, and attaches page-load/title/navigation callbacks. (tauri-svelte-preview/src-tauri/src/browser.rs:1-8, 395-587, 1851-1936)

Bounds and navigation operations require the current workspace/tab/generation identity. Showing one tab hides siblings and cancels an inspector; navigation advances the generation and clears the picker; reload, back, forward, close, and profile-data clearing use the same registry. Clearing data closes the tabs and removes the profile directory. (tauri-svelte-preview/src-tauri/src/browser.rs:589-875)

The element picker installs a bounded script, polls at 80-millisecond intervals, times out after 250 milliseconds, and caps its payload at 64 KiB. Rectangle inspection validates the current page URL and uses bounded data. Viewport capture waits for a native PNG snapshot for up to five seconds and returns image bytes, width, height, and source hash. (tauri-svelte-preview/src-tauri/src/browser.rs:877-1111; commit `3ee06a5`; commit `53cdc4c`)

The browser interface does not expose generic script evaluation, cookies, or storage bridges; its inspection path is bounded to picker and rectangle operations. (tauri-svelte-preview/src-tauri/src/browser.rs:1-8)

### Resource sampling, disk usage, and process actions

`ResourceRegistry` keeps a 60-sample ring with a three-second cadence, which covers about three minutes, and prunes idle resource roots after five minutes. A sample refreshes one reused `sysinfo::System`, reads process data, joins it with one `lsof` pass, and stores CPU, RSS, process, terminal, and agent-root data. A snapshot groups the result by project, workspace, terminal, and agent. (tauri-svelte-preview/src-tauri/src/resources.rs:24-35, 37-156, 158-228, 325-442)

The core resource scanner returns a `ResourceSnapshot` with generation, capture time, process rows, total CPU percent, and total physical-memory bytes. Each `ResourceProcess` carries PID, parent and group IDs, CPU, memory, elapsed time, user, command, listening ports, ownership, optional project/workspace/session identity, registry generation, and a stop permission. It keeps only process trees rooted at registered app-owned PTY or agent process identifiers; unrelated system processes are dropped. It validates process generation, process group, and owner identity before actions. (core/src/scanners/resources.rs:28-93, 247-413)

The resource disk scan takes explicit active-terminal and agent roots plus application stores and delegates directory measurement to `mcb_core`. Workspace disk kinds are worktree, build output, dependency cache, agent data, and other; only `SafeCandidate` entries are reclaimable. A selected cleanup revalidates identity and byte count, routes worktree cleanup through Git safety, and removes other entries only after symlink checks. (tauri-svelte-preview/src-tauri/src/resources.rs:906-1005; core/src/scanners/disk.rs:6-72; tauri-svelte-preview/src-tauri/src/resources_disk.rs:1-17, 43-78, 391-499)

The explicit resource-disk module scans `target`, `node_modules`, `.git`, worktrees, transcript stores, and usage stores with depth and entry caps. Cargo `target` and `node_modules` are the reclaimable categories; the report is cached for five minutes. (tauri-svelte-preview/src-tauri/src/resources_disk.rs:1-41, 143-239)

`stop_owned_resource` revalidates the generation, owner, root, and process group and sends `SIGTERM`. `stop_resource_process_tree` selects a validated tree, sends `SIGTERM` deepest-first, waits up to five seconds, and sends `SIGKILL` only when a still-existing process has not exited. (tauri-svelte-preview/src-tauri/src/resources.rs:1025-1082, 1084-1329)

### Core disk, process, and worktree scanners

The disk scanner accepts explicit roots and clamps depth to eight and entries to 10,000. It canonicalizes roots, measures sorted descendants, returns top-level item sizes, marks only `SafeCandidate` bytes as reclaimable, and reports capture time, entries, scanned bytes, reclaimable bytes, and truncation. (core/src/scanners/disk.rs:28-72, 74-185)

The process scanner invokes `lsof -nP -iTCP -sTCP:LISTEN`; if that command fails it returns no listeners. It groups listener rows by PID and supplements each process with `lsof` working-directory output and `ps` arguments. A `ProcessRecord` carries name, PID, user, optional current directory, listening ports, and arguments. (core/src/scanners/processes.rs:5-117)

The worktree scanner parses `git worktree list --porcelain`, reads status, checks for commits reachable from local branches but not remotes, measures `du -sk`, and records recent activity. A `WorktreeRecord` carries repository and worktree paths, branch, task identifier, dirty/unmerged/prunable/locked flags, last activity, disk bytes, and deletion eligibility. Eligibility distinguishes dirty, untracked, unmerged, locked, prunable, and confirmation-required states. (core/src/scanners/worktrees.rs:5-25, 32-81, 220-258)

### Usage sampling and storage

Current usage is authoritative provider data only. The current snapshot carries provider, account, instance, state, windows, capture time, source, version, and reason; a window carries label, used percent, reset time, and duration in minutes. Claude uses remote provider-authored windows; Codex uses its latest local quota source; providers without an authoritative source return unavailable rather than inferred transcript usage. (tauri-svelte-preview/src-tauri/src/usage_current.rs:5-34, 40-103, 150-169)

The local indexer discovers `.claude/projects`, `.codex/sessions`, and archived JSONL sources, bounded to depth eight and JSONL files. It tracks byte offsets and file identity, reads only complete newline records, re-reads session context on each pass, parses Claude assistant usage and Codex `token_count`/context records, deduplicates events, and writes new cursors and events in one transaction before rebuilding rollups. (tauri-svelte-preview/src-tauri/src/usage_sources.rs:47-151, 154-342; tauri-svelte-preview/src-tauri/src/usage_indexer.rs:54-117)

Usage history is stored in SQLite at `MAC_COMMAND_BAR_USAGE_DB` when that environment variable is set, otherwise under the application-data directory as `usage-history.sqlite3`. The database stores source-file identity and cursors, usage events, and rollups; summary, breakdown, provider, daily, and cost-input queries apply SQL filters and aggregate rows in the database, with a maximum result limit of 200. (tauri-svelte-preview/src-tauri/src/usage_history.rs:110-119; tauri-svelte-preview/src-tauri/src/usage_db.rs:15-201, 360-365, 372-795)

Remote quota requests use a five-minute success cache and a 60-second retry gate. Claude quota reads credentials from the supported file or keychain stores, checks OAuth scope and expiry, refreshes an expired credential or one 401 response, persists refreshed credentials atomically, and converts returned model windows into deduplicated provider windows with bounded values. (tauri-svelte-preview/src-tauri/src/usage_remote.rs:9-18, 93-157; tauri-svelte-preview/src-tauri/src/claude_quota.rs:15-24, 26-207, 306-408, 612-871)

### Orchestration and workflow ledger

Orchestration events use a schema version, stable run and event identifiers, sequence, timestamp, kind, status, project/workspace fields, and additive workflow fields. They live in the `orchestration_events` table in `sessions.db`; listing reduces SQLite rows and returns no demo runs unless `MCB_DEMO_RUNS=1` is explicitly set. Startup imports the former JSONL ledger transactionally and deletes it only after success. (`core/src/session_store.rs`; `tauri-svelte-preview/src-tauri/src/orchestration.rs`)

The workflow engine validates workflow definitions, node roles, DAG edges, policies, budgets, output contracts, provider ports, and gates. Creating a run canonicalizes and hashes the input, honors an idempotency key, creates a draft run and blocked node records, and appends `workflow.run.created`. Scheduling marks nodes ready only when all dependencies are completed or skipped, in deterministic order. (tauri-svelte-preview/src-tauri/src/workflow.rs:16-447, 766-995, 997-1091)

Start, pause, resume, cancel, retry, skip, approve-gate, and submit-result operations append events containing run/node identifiers, attempts and depth, input hash, output contract, artifacts, gate, lease, provider instance, idempotency, provenance, and workflow payload. The reducer orders events by run and sequence and keeps the latest run projection; the engine dispatches ready nodes within concurrency, depth, budget, and timeout policies. (tauri-svelte-preview/src-tauri/src/workflow.rs:766-824, 1093-1844)

### Debug logging, identity, and encryption

The debug logger writes each line to stderr and, when installed, to a file mirror. The mirror creates its parent directory and rotates the current file to `.1` before appending when it exceeds 5 MiB. Closed-pipe writes are discarded without a panic. (tauri-svelte-preview/src-tauri/src/debug_log.rs:7-68, 70-144)

The product identity strings are `Assembly` for the product, terminal program, and LSP client, and `failed to run Assembly` for the Tauri startup error. (tauri-svelte-preview/src-tauri/src/product_identity.rs:1-4)

The core crypto helper stores a 12-byte nonce and ciphertext in `EncryptedBlob`; `SecretBox` requires a 32-byte AES-256-GCM key, generates a random nonce for encryption, and base64-encodes the nonce and ciphertext. (core/src/crypto.rs:9-52)

## Why it is the way it is

- Heavy Tauri commands are asynchronous because synchronous command bodies run on the UI thread; the module states this at its top. (tauri-svelte-preview/src-tauri/src/main.rs:1)
- Reference counting lives in the optimized `mcb-core` crate because an unoptimized desktop crate made a full-project pass take seconds instead of a fraction of one. (core/src/reference_counts.rs:1-10)
- The reference counter has its own 4 MiB file ceiling instead of reusing the 512 KiB preview ceiling because the preview limit is for what a reader sees, while the counter walks bytes; the comment records a 660 KiB file previously making every project total a floor. (core/src/reference_counts.rs:20-29; commit `6acb2db`)
- One-pass counting replaced one full-project reread per symbol because 120 concurrent per-symbol requests froze the editor. (tauri-svelte-preview/src-tauri/src/main.rs:813-820)
- Terminal scrollback was raised from 256 KiB to 16 MiB because hidden views retain no history and the backend is the reattach source. Hysteresis trims to 75 percent so each PTY read does not rescan and move the entire buffer; the same commit changed close ordering to kill before removing the registry entry. (tauri-svelte-preview/src-tauri/src/terminal.rs:13-23; commit `e7a8479`)
- Exited terminals remain as tombstones so final output, exit code, and signal remain visible until explicit close. (tauri-svelte-preview/src-tauri/src/terminal.rs:48-67, 417-443; commit `f732d19`)
- Shell sessions use login-shell arguments because a plain PTY shell omitted login-only Homebrew, pnpm, nvm, and local-bin paths and caused resumed commands to be reported as not found. (tauri-svelte-preview/src-tauri/src/terminal.rs:464-475; commit `0997509`)
- `COMMANDBAR_SESSION_ID` and collision-resistant identifiers associate child work with the owning terminal. (tauri-svelte-preview/src-tauri/src/terminal.rs:93-116; commit `6fb8ee5`)
- LSP startup now reports ordinary no-server, missing-program, C#-native, C#-disabled, non-C#-project, and read-mode reasons because the previous language switch recorded a setting while lazy startup stayed silent. (tauri-svelte-preview/src-tauri/src/main.rs:983-1006; commit `543eeb5`)
- Roslyn replaced `csharp-ls`, and sessions are kept per workspace with a five-workspace-per-language bound so switching roots reuses warm servers and evicts the least-recently-used session after the bound. (tauri-svelte-preview/src-tauri/src/lsp.rs:41-48, 4516-4555; commit `bd0f9d2`)
- A reader task routes LSP answers by request identifier and sends cancellation after a timeout because serial request handling made a slow reference request block hover and other questions. (tauri-svelte-preview/src-tauri/src/lsp.rs:773-799, 1140-1283; commit `6acb2db`)
- Login-shell `PATH`, Cargo, local-bin, nvm, Homebrew, and system paths are probed because Finder-launched applications receive a narrower environment than a login shell. (tauri-svelte-preview/src-tauri/src/lsp.rs:4558-4746; commit `561747e`)
- Existing language servers are warmed on project switch so cold re-indexing happens before the first file request, while unused languages are not spawned. (tauri-svelte-preview/src-tauri/src/lsp.rs:2510-2568; commit `99fba29`)
- The browser snapshot path was added because annotation tools needed a still image and the native view previously refused every capture; the five-second channel wait makes a non-answer a readable failure instead of an indefinite wait. (tauri-svelte-preview/src-tauri/src/browser.rs:877-1111; commit `3ee06a5`)
- Browser capture releases the registry lock before waiting for the native snapshot because the earlier error path returned an unreadable object and the snapshot seam needed to wait without holding the registry lock. (tauri-svelte-preview/src-tauri/src/browser.rs:877-1111; commit `53cdc4c`)
- Resource inventory and usage history were added together as bounded native services with protected cleanup, provider-authoritative current usage, tail-only JSONL ingestion, stable file identity, and SQL daily rollups. (commit `e794abc`)
- Resource attribution walks process trees rooted at app-owned PTYs and agent processes so system daemons are outside the default project/workspace view; local usage indexing rereads session context so tail increments retain attribution. (commit `bdd59d0`)
- The workflow engine is a deterministic layer over the orchestration ledger, and empty run lists remain empty unless sample runs are explicitly requested. (commit `9434722`; commit `e72e40d`)

## What is stored, and where

- Application data is created at launch. `logs/backend.log` is the rotating backend log mirror, and `sessions.db` is opened by the agent runtime. (tauri-svelte-preview/src-tauri/src/main.rs:5693-5715; tauri-svelte-preview/src-tauri/src/debug_log.rs:52-68)
- Terminal sessions, PTY handles, exit tombstones, scrollback, LSP registry slots, browser registry state, resource samples, and usage-history state are held in process memory through the managed registries. (tauri-svelte-preview/src-tauri/src/main.rs:5683-5690; tauri-svelte-preview/src-tauri/src/terminal.rs:80-91; tauri-svelte-preview/src-tauri/src/resources.rs:37-156; tauri-svelte-preview/src-tauri/src/lsp.rs:1588-1601)
- Browser profile files are under `<app_data>/browser-profiles/<uuid>`. (tauri-svelte-preview/src-tauri/src/browser.rs:1851-1936)
- Orchestration and workflow events are durable rows in `sessions.db.orchestration_events`; the former JSONL path is import-only. (`core/src/session_store.rs`; `tauri-svelte-preview/src-tauri/src/orchestration.rs`)
- Usage history is SQLite at `MAC_COMMAND_BAR_USAGE_DB` or the application-data `usage-history.sqlite3` path. (tauri-svelte-preview/src-tauri/src/usage_history.rs:110-119)
- Local usage input remains in Claude and Codex session JSONL files under their home-directory source trees; the SQLite database stores cursors, identities, events, and rollups rather than replacing those files. (tauri-svelte-preview/src-tauri/src/usage_sources.rs:47-151; tauri-svelte-preview/src-tauri/src/usage_db.rs:15-201)
- Claude credentials are read from the supported credential file or macOS keychain stores and refreshed credentials are persisted atomically through the selected store. (tauri-svelte-preview/src-tauri/src/claude_quota.rs:26-207, 612-695)
- Disk scans and process scans return in-memory reports; the scanner modules do not write a database or cache. (core/src/scanners/disk.rs:64-185; core/src/scanners/processes.rs:5-117)

## Contracts other portions rely on

- Tauri command names are the generated-handler list in `main.rs`; `read_backend_capabilities` exposes the capability strings used before offering optional arguments or features. (tauri-svelte-preview/src-tauri/src/main.rs:1429-1505, 5737-5905)
- `source_scan_progress` carries `scanId`, `visitedEntries`, and `matchedFiles`; `source-lsp-status-changed` carries root, language, state, and detail; `terminal_output` carries session identifier, output, and termination state; `workflow-run-updated` carries the updated workflow run. (tauri-svelte-preview/src-tauri/src/main.rs:71-72, 658-681; tauri-svelte-preview/src-tauri/src/lsp.rs:312-325; tauri-svelte-preview/src-tauri/src/terminal.rs:70-78; tauri-svelte-preview/src-tauri/src/main.rs:1660-1662)
- `agent-conversation-event` is emitted from the runtime manager for live conversation events. (tauri-svelte-preview/src-tauri/src/main.rs:5722-5725)
- Browser events are `browser-tab-navigation`, `browser-tab-load`, `browser-element-selected`, and `browser-tab-closed`; their payloads identify workspace, tab, and generation, with navigation/load metadata or selected-element metadata as appropriate. (tauri-svelte-preview/src-tauri/src/browser.rs:36-39, 169-244)
- `SourceRecord`, `SourceScanResult`, `SourcePreview`, `AgentSessionRecord`, `WorkspaceDiskEntry`, `ProcessRecord`, `ResourceSnapshot`, usage snapshots/rows, and workflow records are the serialized boundary shapes consumed by frontend commands. (tauri-svelte-preview/src-tauri/src/main.rs:74-219; core/src/source.rs:12-65; core/src/scanners/sessions.rs:37-94; core/src/scanners/disk.rs:28-72; core/src/scanners/processes.rs:5-14; tauri-svelte-preview/src-tauri/src/resources.rs:158-228; tauri-svelte-preview/src-tauri/src/usage_current.rs:5-34; tauri-svelte-preview/src-tauri/src/workflow.rs:321-447)
- `COMMANDBAR_SESSION_ID` identifies the owning terminal to child processes, and `MCB_TIMING=1` enables one-line LSP and reference-count timing output. (tauri-svelte-preview/src-tauri/src/terminal.rs:93-116; tauri-svelte-preview/src-tauri/src/lsp.rs:262-269)
- `MAC_COMMAND_BAR_USAGE_DB` overrides the usage database path; the orchestration CLI uses `MAC_COMMAND_BAR_ORCHESTRATION_DB` for an isolated `sessions.db`. Legacy orchestration path overrides are read only while importing the former JSONL ledger. (tauri-svelte-preview/src-tauri/src/usage_history.rs:110-119; tauri-svelte-preview/src-tauri/src/orchestration.rs; tauri-svelte-preview/scripts/orchestrationEvent.mjs)
- Git operations rely on direct executable argument vectors for `git` and `gh`, not shell command text. (tauri-svelte-preview/src-tauri/src/git_workspace.rs:1-14; tauri-svelte-preview/src-tauri/src/git_pr.rs:1-6)
- The core dispatcher action names are `scan.worktrees`, `scan.processes`, `scan.sessions`, `health.snapshot`, `source.list`, `source.preview`, and `plan.killProcess`; the process action returns a plan without executing it. (core/src/dispatcher.rs:9-23, 26-175)
- `EncryptedBlob` is the crypto boundary: a base64 nonce plus base64 ciphertext, decrypted with the caller's 32-byte AES-GCM key. (core/src/crypto.rs:9-52)

## Known gaps

- `server_spec_for_language` returns no configured server for Python, Python files, or Go; the `python_and_go_lsp_adapters_are_deferred` test asserts an unavailable status with the reason `No language server configured for this file type`. (tauri-svelte-preview/src-tauri/src/lsp.rs:4516-4555, 5273-5293)
- The recorded native-C# concurrency test receipt has one failure: `concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint` reaches an `unwrap()` error while binding the native bridge with `Operation not permitted (os error 1)`, before its same-URL, same-root, one-session assertions can complete. (tauri-svelte-preview/src-tauri/src/lsp.rs:6713-6743; `docs/superpowers/evidence/tsk-808/a9-rust-review-receipt.md:57-68`)
- `restart_language_server_root`, `apply_resource_memory_pressure`, and the resource-panel `read_language_server_log` endpoint return unavailable results in the resource module. (tauri-svelte-preview/src-tauri/src/resources.rs:1331-1368)
- Current usage returns unavailable when the provider does not expose an authoritative current quota source; local transcript data is used for history indexing, not substituted into the live quota snapshot. (tauri-svelte-preview/src-tauri/src/usage_current.rs:40-103, 150-169)
- Browser operations are limited to HTTP/HTTPS native tabs and bounded picker, rectangle, and PNG snapshot operations; generic evaluation, cookie access, and storage access are not exposed by this module. (tauri-svelte-preview/src-tauri/src/browser.rs:1-8, 1942-2187)
