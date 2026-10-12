# How the Terminal tab works for a remote session

TSK-1454. This covers what happens when you open the right-panel Terminal tab on a session that lives on another machine (for example Workbox Test), what can go wrong, and where to look.

## The short version

Nothing connects back to the Mac, and no second SSH login happens.

There is one connection, and the Mac opens it:

```
Mac (Assembly app)                                   Remote machine
──────────────────                                   ──────────────
ssh -L <local port>:127.0.0.1:7777 <machine>  ───►   sshd
WebSocket ws://127.0.0.1:<local port>/assembly ═══►  Assembly backend (listens on 127.0.0.1:7777)
   (inside that SSH tunnel)                              │
                                                         └─ spawns the shell (bash -l) in a PTY
```

- The **SSH tunnel** is a plain port forward the app starts when you connect to the machine. Its only job is to carry the WebSocket. Chats, history, files, git and now terminals all travel over it.
- The **Assembly backend** is the server already running on the remote machine (the same one that runs the Claude and Codex sessions there).
- The **shell** is started *by that backend*, as a child process on the remote machine, in the session's folder there. It is not an SSH session. The backend just runs `bash -l` (or whatever `$SHELL` is) in a pseudo-terminal, the same way the Mac does for a local session.

Your keystrokes go Mac → tunnel → backend → shell. The shell's output goes shell → backend → tunnel → Mac → the Terminal tab. Both directions use the one WebSocket.

## Step by step

1. **You open the Terminal tab.** The tab asks the Mac's Rust side to start a terminal for the session folder. For a remote session that folder looks like `assembly-remote://<machine id>/home/you/project`.
2. **The Mac sees the `assembly-remote://` prefix** (`commands/terminal.rs`). Instead of starting a local shell, it strips the prefix and sends a `TerminalStart` request with the real path (`/home/you/project`) over that machine's WebSocket. If you leave the tab before the answer comes back, the app cancels that request and the backend skips the start.
3. **The backend starts the shell** (`remote.rs`, `execute_server_command`). It checks that the folder exists, then starts a login shell there. If this session already has a live shell, it reuses that one instead of starting a second. It answers with the shell's id.
4. **The Mac renames the id** to `assembly-remote-terminal://<machine id>/<shell id>` and gives it to the tab. Every later call (typing, resize, reading scrollback, close) carries that id, and that prefix is how the Mac knows to send the call to the machine rather than to a local shell.
5. **Typing.** Each write goes over the WebSocket as a `TerminalWrite` request. The tab keeps only one write in flight at a time and batches keys typed in the meantime, so letters can't arrive out of order.
6. **Output.** The backend reads the shell's output and pushes it to every connected Mac as a `TerminalOutput` frame. The Mac puts it into the same output stream a local terminal uses, under the renamed id, and the tab draws it.
7. **Closing the tab** only drops the view. The shell keeps running on the machine, and its last 16 MB of output stays in memory there. Reopening the tab finds the same shell and replays the last 4 MB of output, so running commands, variables and history are still there.
8. **Deleting the session** closes its shells on the machine.

## Limitations

- **The shell lives inside the backend.** When the backend restarts (an update, a provider-update restart, a crash or a deploy), every remote shell ends with it. The tab will then show an error on the next keystroke. Close and reopen the tab to get a fresh shell.
- **It runs as the backend's user, with the backend's environment.** It is a login shell (`-l`), so `~/.bash_profile` / `~/.profile` are read. It is still not identical to what you get from `ssh machine`: it inherits whatever environment the backend service was started with.
- **Losing the connection.** The shell keeps running on the machine. Output produced while the Mac is disconnected doesn't appear live in the open tab, but it is kept in the machine's scrollback; reopen the tab to see it. Typing while disconnected fails with an error.
- **A very slow link can drop live output.** The backend buffers about 1,000 output chunks per connection. If the Mac falls further behind than that (for example a huge `cat` over a bad network), the missed chunks are skipped live. Reopening the tab replays them from scrollback.
- **Every connected Mac receives every remote shell's output.** A second Mac connected to the same backend gets the frames too, and ignores the ones it didn't open. This only matters for bandwidth.
- **Each request has a 15-second timeout.** A start, write or resize that takes longer than that (a frozen machine or a hung network) fails with a timeout error.
- **Remote shells don't appear in the Mac's local terminal list** (for example the Resource Manager's terminal count). They are counted on the machine, not on the Mac.
- **The Mac app and the remote backend must speak the same protocol version** (16 as of this change). An older backend can't run remote terminals, and the app refuses to connect to a mismatched backend.
- **When a shell has exited**, reopening the tab starts a fresh one. Unlike a local terminal, it does not first replay the dead shell's last output.

## When it goes wrong

| What you see | Likely cause | Where to look |
|---|---|---|
| "This session's folder no longer exists: /path" | The session's folder was moved or deleted on that machine. No shell is started anywhere else. | Recreate the folder, or move the session to a project that exists. `terminal.rs`, `terminal_cwd_from_request` |
| The shell opens on the Mac, or you get a path error mentioning `assembly-remote://` | The start request didn't go through the remote branch: the folder lost its `assembly-remote://` prefix somewhere in the frontend, or this build predates TSK-1454. | `commands/terminal.rs`, `start_terminal_session` |
| "Terminal command needs the server's terminals" | A terminal request reached the backend's plain command handler without the server's terminal registry. This is a code bug, not a setup problem. | `remote.rs`, `execute_server_command` and `execute_remote_command` |
| An error mentioning a protocol version or an incompatible response | The app and the backend are on different versions. | Update the backend on that machine. |
| The tab opened but output stopped | The backend restarted (the shell is gone), or the connection dropped. | Check the machine's connection status in the app; close and reopen the tab. |
| Letters arrive scrambled | Writes are going out more than one at a time. | `terminalService.ts`, `createTerminalInput`, and its test in `scripts/nextTerminalService.test.ts` |

## Code map

| Part | File |
|---|---|
| Terminal tab (the view) | `tauri-svelte-preview/src/lib/shell/components/WorkspaceTerminal.svelte` |
| Ordered typing | `tauri-svelte-preview/src/lib/shell/terminalService.ts` (`createTerminalInput`) |
| Mac commands that choose local or remote | `tauri-svelte-preview/src-tauri/src/commands/terminal.rs` |
| Mac side of the connection: start, write and the output sink | `tauri-svelte-preview/src-tauri/src/agent_conversation/remote.rs` (`start_terminal`, `write_terminal`, `terminal_sink`) |
| Wire messages | `remote.rs`: `RemoteCommand::Terminal*`, `RemoteResponse::TerminalSession` / `TerminalScrollback`, `ServerFrame::TerminalOutput` |
| Backend side | `remote.rs`: `RemoteTerminals`, the `Terminal*` arms of `execute_server_command`, and the `live_terminals` branch in `serve_remote_socket` |
| The shell and its scrollback (shared by the Mac and the backend) | `tauri-svelte-preview/src-tauri/src/terminal.rs` |
| Tests | `remote.rs` `terminal_tests`, `terminal.rs` `terminal_cwd_*`, `scripts/nextTerminalService.test.ts` |

The architecture guide (`main-architecture-explained.html`, section 14, "A remote session's shell runs on its machine") has the same design in short form.
