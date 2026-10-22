# Four-scenario check

Every change is proved this way, however small it looks: in the running native app with real test data, driven the way a person uses it, for Claude Code and Codex, each on a local session and a remote session on Workbox Test. Several changes may share one run if the run exercises each of them. A browser preview is not proof: it has no app data. The script owns setup, isolation and teardown; you drive the app.

| # | Provider | Machine |
|---|---|---|
| 1 | Claude Code | local |
| 2 | Codex | local |
| 3 | Claude Code | remote (Workbox Test) |
| 4 | Codex | remote (Workbox Test) |

## The script

Run from the worktree under test, in the background (the first build takes 10+ minutes), and poll the log:

```
S="node --experimental-strip-types tauri-svelte-preview/scripts/fourScenarios.ts"
nohup $S start > /tmp/four.log 2>&1 &
```

- `start [--worktree <path>] [--data <existing data dir>] [--seed <source data folder>]` refuses when `free -g` shows 6 GB or less available. It makes `~/assembly-proof/<timestamp>/` with a fresh copy of `~/assembly-proof/template` as `data/` (the template has Workbox Test saved), starts Xvfb (1710x990), Vite and the native app detached, saves `state.json`, links `~/assembly-proof/current`, and screenshots `00-started.png`. It prints whether files landed in the private data folder and whether the owner's `~/.local/share/assembly` stayed untouched; if either says NO, stop and report it.
- `start` launches the app only once the Vite port is served by this run's own Vite (its process group, in this worktree's folder). If another lane takes the port first, it moves to the next port; if anything else serves the port, before or while the app starts, it stops everything it started and says which process holds the port. Vite logs are `vite-<port>.log`.
- `start` compiles the app through the workbox build lock (`/var/tmp/tsk-1582-lanes/build-lock.sh`: two builds at once, memory capped) into `CARGO_TARGET_DIR`. Export it from `cargo-slot claim <lane>` first; if it isn't set, `start` claims a cache named after the worktree folder and prints the `cargo-slot release` command for the end of the lane.
- `start --seed <source data folder>` replaces the template with a copy of real data: it copies `<source>/sessions.db` (and `conversation-attachments/`, if present) with `sqlite3 -readonly ... ".backup"` (the source is only read) and, on the copy, deletes cached remote chats, every saved machine except Workbox Test, remote projects and phone devices. It stops if any remote entry is left. The workbox source is `~/.local/share/dev.blackcolours.MacCommandBarWebviewPreview`. Do not combine it with `--data`.
- On the Mac, seed the test app's data folder by hand: `.backup` the real `sessions.db` into it, then run this on the copy before the first launch (skip the last line if the copy has no `phone_devices` table):

  ```
  PRAGMA foreign_keys=ON;
  DELETE FROM sessions WHERE cached_remote_profile_id IS NOT NULL;
  DELETE FROM app_settings WHERE setting_key IN ('remote-assembly.profile.v1','remote-assembly.session-projections.v1');
  UPDATE app_settings SET value_json=(SELECT json_group_array(json(value)) FROM json_each(value_json) WHERE json_extract(value,'$.sshTarget')='workbox-test') WHERE setting_key='remote-assembly.profiles.v1';
  DELETE FROM projects WHERE machine <> 'local';
  DELETE FROM phone_devices;
  ```

- Never sign out (`codex logout`, `claude logout`) to test sign-in. Signing out revokes the login on the provider's side, and restoring the old file does not bring it back.
- `stop` stops exactly the process groups in `current/state.json` and prints a `ps` check. Run it when finished, and also after a failed start.
- `offline-ssh` prints a folder whose `ssh` exits 255. Start with `PATH="$($S offline-ssh):$PATH" $S start --data <dir>` to make the remote unreachable.

To relaunch: `stop`, then `start --data <dir>` to reuse that data (local scenarios, offline and reconnect steps), or plain `start` for a fresh copy of the template (the remote "never saved this session" reopen). The run folder keeps every data copy.

Never point the app at `~/.local/share`, never use `--assembly-server`, and connect only to Workbox Test (SSH alias `workbox-test`), never to any other machine.

## Driving the app

On Linux the app runs under Xvfb and you drive it with `assembly-native-ui`. Codex may use its computer-use tool instead.

```
DISPLAY=:<n> assembly-native-ui shot <file.png>
DISPLAY=:<n> assembly-native-ui click <x> <y>
DISPLAY=:<n> assembly-native-ui key <key> [Modifier]
```

Open every screenshot after each action; do not assume a click worked. Fixtures: add a git repo with one commit as a local project; for remote, a folder on Workbox Test (Settings -> Connections lists Workbox Test; **Install and connect** is fine from the test app, against Workbox Test only). A remote backend older or newer than the app speaks another protocol: use the dev-only "Deploy from this checkout" for Workbox Test when the change touches the backend.

## Steps per scenario

1. Start a session for the provider and machine. Send: "Run the shell command `seq 1 20000` in this folder, then reply DONE." Approve if asked. Wait for DONE. Send "Reply with the word SECOND." and wait. Screenshot the chat.
2. Quit: `stop`, then `ps` shows nothing left.
3. Reopen. Local: `start --data <same dir>`. Remote: plain `start` (fresh template copy), connect, open the session. Expect both prompts, the command output fold, DONE and SECOND in order with no duplicate rows. Scroll to the very top to confirm the first prompt is there.
4. Remote only, offline: `stop`, then `start --data <that dir>` with `offline-ssh` on PATH. Confirm the saved chat still shows.
5. Remote only, reconnect: `stop`, `start --data <that dir>` normally, wait for reconnect, send "Reply with the word THIRD." Confirm it arrives once and nothing is duplicated.
6. Note anything that errors, spins forever, shows a raw error, duplicates, loses or reorders rows, or differs between providers or machines. If Codex rejects its default model, pick GPT-5.6 Luna and say so.

A scenario that cannot run (provider not signed in, session will not start, Workbox Test unreachable) is reported blocked with the exact error and a screenshot. Never substitute a result.

## Evidence

Save screenshots to `~/Workbox/screenshots/TSK-<id>-<scenario>-<what>.png`. After each, from the task worktree: `evidence add <file> --task TSK-<id> --desc "<what it proves>"` (`--key` on the best reopen proof per scenario). At the end `evidence publish TSK-<id>` and paste the links. Stop everything with `stop` and show its output.

## Required report table

One row per scenario; each cell is pass, FAIL, blocked or n/a, with the screenshot name:

| # | Provider / machine | Chat after send | Reopen (fresh copy for remote) | Offline view | Reconnect + new message | No duplicates |
|---|---|---|---|---|---|---|
| 1 | Claude Code / local | | | n/a | n/a | |
| 2 | Codex / local | | | n/a | n/a | |
| 3 | Claude Code / remote | | | | | |
| 4 | Codex / remote | | | | | |

Under the table list bugs: title, scenario, steps, expected versus actual, screenshot.
