# Assembly phone app — design and build plan (TSK-1416 slice 5)

Date: 2026-10-12. Status: **draft for owner review**. Written overnight from the owner's standing
direction; every choice the owner did not make is marked **Decision** with its reason, and the
five that most need the owner are in §9.
Task: TSK-1416 (Workflows and orchestration) — https://app.notion.com/p/3f5394b0689d81e5b2aef3bb8bddfea8

Parent spec: `docs/superpowers/specs/2026-10-10-workflows-orchestration-design.md` (latest on
branch `tsk-1416-authoring-docs`), especially D10, D11, D13, §4.5, §6.3, §6.4 and §8.
Desktop plan this builds on: `docs/superpowers/plans/2026-10-11-tsk-1416-workflows-slices-3-4.md`
(branch `tsk-1416-slices-3-4-plan`), Tasks 12, 17, 18, 21, 23. Task numbers here continue at 27.
Visual mockup: `2026-10-12-tsk-1416-phone-app-mockup.html` beside this file; screenshot
`~/Workbox/screenshots/tsk-1416-phone-mockup.png`.

`RC/` below means `~/dev/work/rental-management/mobile/` (Rental Command's Flutter app, read only).
`backend/` means `tauri-svelte-preview/src-tauri/src/`. Line numbers were read on 2026-10-12 at
Assembly `db7930054` and Rental Command's current working tree; re-find symbols before editing.

## 1. Goal and scope

The owner can leave the Mac closed and still do three things from an Android phone (D11):

1. **Chat** with the main orchestrator on the home machine (default the workbox, §4.5).
2. **Runs**: see every run on a paired machine, open one, and pause, resume or stop it (§6.3).
3. **Inbox**: answer everything waiting for the owner — run pauses, gates, `ask_owner` confirms
   and agent permission requests (§6.4) — and get a phone notification when a new one appears.

Not in this version (YAGNI, matches parent §8 "Later"): starting runs from the phone, messaging
arbitrary sessions, the full live run timeline, file/diff viewing, images in chat, iPhone builds,
Play Store release, more than one owner.

## 2. Where the app lives and what it is called

- **Decision: a new folder `mobile/` in the Assembly repo**, not a separate repo. Reasons: the
  parent spec §8 already says so; the phone API (Rust) and its only client (Dart) change together
  in one PR and one review; the Notion branch sync and `tsk-1416-*` branches already cover this repo.
- Flutter project name `assembly_phone`; Android application id **`dev.blackcolours.assembly_phone`**
  (same `dev.blackcolours` prefix as the desktop identifier in
  `tauri-svelte-preview/src-tauri/tauri.conf.json:5`); launcher label **Assembly**.
- **Decision: Android only.** Create with `flutter create --org dev.blackcolours
  --project-name assembly_phone --platforms=android mobile`. No `ios/` folder: it cannot be
  compiled on the workbox, so "keep it compiling" is not free. Adding iOS later is one
  `flutter create --platforms=ios .`.
- **Decision: no build flavors.** Rental Command needs dev/prod flavors because each flavor has a
  fixed API URL (`RC/android/app/build.gradle.kts:51-63`, `RC/lib/core/config/app_config.dart`).
  This app has no fixed URL: every machine URL comes from pairing.
- Toolchain on the workbox: Flutter 3.44.9 stable at `~/develop/flutter` (its
  `bin/cache/flutter.version.json`), Android SDK at `~/Android/Sdk`. Same Dart constraint as
  Rental Command: `sdk: ^3.12.0` (`RC/pubspec.yaml:7-8`).

## 3. What we copy from Rental Command, and what we drop

Rental Command's mobile app: Riverpod 3 hand-written providers, Dio REST, go_router, secure
storage, SignalR change notices, FCM push, an M3 Expressive dark theme (`RC/pubspec.yaml:10-58`).

### 3.1 Copied (file by file)

| Rental Command source | Assembly destination | How |
|---|---|---|
| `RC/lib/core/theme/app_tokens.dart` (colors `M3Colors` 29-122, `M3Shape` 205-223, `M3Motion`, `AppTokens`) | `mobile/lib/core/theme/app_tokens.dart` | Copy whole. |
| `RC/lib/core/theme/app_theme.dart` (`AppTheme.light/dark`, button/card/chip/nav themes 66-265) | `mobile/lib/core/theme/app_theme.dart` | Copy whole. |
| `RC/lib/core/theme/app_typography.dart` (`M3Type`, Funnel Display / Funnel Sans via google_fonts) | `mobile/lib/core/theme/app_typography.dart` | Copy whole. |
| `RC/lib/core/theme/app_recipes.dart` `M3TonalFamily` (19), `M3TonalCard` (223), `M3MorphNavItem` (270) | `mobile/lib/core/theme/app_recipes.dart` | Copy these three only. Drop `M3ArtSurface` / `M3ArtBand` (they need `assets/patterns/`). |
| `RC/lib/core/widgets/mobile_m3_list.dart` (`MobileM3ListItem`, `MobileM3ListDivider`, `MobileM3LeadingIcon`) | `mobile/lib/core/widgets/mobile_m3_list.dart` | Copy whole; used by runs list, step list, machine list. |
| `RC/lib/core/api/api_exception.dart` (`ApiException.fromDioException`) | `mobile/lib/core/api/api_exception.dart` | Copy whole. |
| `RC/lib/core/api/dio_client.dart` (`dioProvider`, timeouts) | `mobile/lib/core/api/machine_client.dart` | Adapt: one Dio per paired machine, base URL from the machine record, `Authorization: Bearer <device token>`. Drop the self-signed-certificate bypass (Tailscale serve gives a real certificate, §5.1). |
| `RC/lib/core/auth/token_store.dart` (`FlutterSecureStorage` keys) | `mobile/lib/core/machines/machine_store.dart` | Adapt: stores the list of paired machines `{id, name, url, deviceToken}` under one key `assembly_machines`, plus the selected machine id. |
| `RC/lib/core/realtime/realtime_providers.dart` `realtimeWatcherProvider` (64) invalidate-on-notice pattern | `mobile/lib/core/live/live_providers.dart` | Same pattern: a notice invalidates the matching provider, which refetches. |
| `RC/lib/core/realtime/signalr_service.dart` `RealtimeEvent` shape and reconnect delays 0/2/5/10/30/60 s | `mobile/lib/core/live/live_socket.dart` | Replace SignalR with a plain WebSocket (`web_socket_channel`, already in `RC/pubspec.lock:1344` as a transitive dependency). The Rust backend speaks no SignalR. |
| `RC/lib/core/push/push_service.dart` (`PushService.init` 75, local channel 114-146, handlers 148-177, `_registerDevice` 218) | `mobile/lib/core/push/push_service.dart` | Adapt: channel id `assembly_inbox`, register with `POST /phone/v1/push-token` per machine, tap opens `/inbox/<id>`. Keep its fail-soft Firebase init. |
| `RC/lib/features/messages/message_detail_screen.dart` `_MessageThread` (375), `_MessageBubble` (416), `_ComposeBar` (512), `_SendButton` (632) | `mobile/lib/features/chat/chat_widgets.dart` | Copy and trim: drop channel chips and `SendingChannelsLine`; bubble "mine" = owner. Add a relayed-message variant and a confirm card. |
| `RC/lib/features/messages/messages_repository.dart` `conversationProvider` (291, Notifier with `refresh()` / `sendMessage()`) | `mobile/lib/features/chat/chat_repository.dart` | Same Notifier pattern over the phone chat endpoints. |
| `RC/lib/features/home/home_shell.dart` `_MorphNavBar` (1118) | `mobile/lib/app_shell.dart` | Copy the nav bar only (3 tabs, no FAB gap). Do not copy `HomeShell` (3,431 lines of landlord logic). |
| `RC/lib/core/router/app_router.dart` redirect guard (103-148) | `mobile/lib/core/router.dart` | Same shape, one rule: no paired machine → `/pair`. |
| `RC/lib/main.dart` `_FriendlyErrorWidget` (33-100) and `themeMode: ThemeMode.dark` | `mobile/lib/main.dart` | Copy; fix its surface color to the theme's `#121217`. |
| `RC/android/app/build.gradle.kts:7` and `RC/android/settings.gradle.kts:25` (google-services plugin 4.4.2), `RC/android/app/src/main/AndroidManifest.xml:5,67` (POST_NOTIFICATIONS, default notification channel) | `mobile/android/...` | Same lines with the new channel id. |
| Test patterns: `RC/test/message_detail_composer_test.dart` (fake repository via `ProviderScope(overrides:)`), `RC/test/app_theme_test.dart` (`GoogleFonts.config.allowRuntimeFetching = false`) | `mobile/test/` | Same patterns. |

### 3.2 Dropped

All 36 feature folders except the message-widget patterns above; email/password, Google and
biometric login (`RC/lib/core/auth/*` except the token-store idea); the refresh-token interceptor
(`RC/lib/core/auth/auth_interceptor.dart`; device tokens do not expire, they are revoked); SignalR;
voice, TTS, recording, app links; scanning/PDF/image/file pickers; flavors; `assets/patterns/`;
the idempotency helper (the send button is disabled while a send is in flight).

### 3.3 Dependencies (`mobile/pubspec.yaml`)

`flutter_riverpod`, `dio`, `go_router`, `flutter_secure_storage`, `web_socket_channel`,
`firebase_core`, `firebase_messaging`, `flutter_local_notifications`, `google_fonts`,
`material_symbols_icons` — all at Rental Command's versions — plus one new package,
**`mobile_scanner`** for the pairing QR code (Rental Command has no QR package). Dev:
`flutter_test`, `flutter_lints`.

## 4. Screens

Bottom navigation with three tabs (copied `_MorphNavBar` + `M3MorphNavItem`): **Chat**,
**Runs**, **Inbox** (badge = pending count). The app bar on every tab shows the selected machine
as a chip; tapping it opens the machine picker. Dark theme only, as Rental Command
(`RC/lib/main.dart` `themeMode: ThemeMode.dark`).

1. **Chat** (`mobile/lib/features/chat/chat_screen.dart`). The orchestrator conversation on the
   selected machine. Owner bubbles right (primary), orchestrator bubbles left
   (surfaceContainerHighest), copied from `_MessageBubble`. A message another session sent to the
   orchestrator shows as a labelled card "Message from `tsk-1412/build`" (`M3TonalCard`, sky).
   Orchestrator tool calls show as one grey line ("start_run — 2 runs started"). Pending
   `ask_owner` items show inline at their time as a **confirm card** (`M3TonalCard`, amber) with the
   item's option buttons; tapping one answers the inbox item (§6). A thin "Working…" line shows
   while the orchestrator's turn runs. Composer copied from `_ComposeBar`. If the selected machine
   does not host the orchestrator: empty state "The orchestrator lives on <home machine>" with a
   button to switch.
2. **Runs list** (`mobile/lib/features/runs/runs_screen.dart`). Two sections: **Active**
   (running, paused, waiting-for-owner) and **Recent** (last 20 done/stopped/failed). Each row
   (`MobileM3ListItem`): task key and title, current step, round, elapsed, state chip. Pull to
   refresh; live notices refresh it.
3. **Run detail** (`mobile/lib/features/runs/run_screen.dart`). Header: task, state chip, machine,
   "I'll merge" on/off, PR link if any. Buttons: **Pause** / **Resume** (whichever applies) and
   **Stop** (asks "Stop this run? Running turns are cancelled." first). A waiting-for-owner banner
   links to its inbox item. Step list: each step's state and attempt count; the current/paused step
   is highlighted; tapping a step opens a bottom sheet with its latest attempt summary, failing
   command output tail or review findings. "Recent activity": last 20 timeline lines.
4. **Inbox** (`mobile/lib/features/inbox/inbox_screen.dart`). Pending items, newest first, one
   card each: kind label, run/task and step, what happened, evidence lines, an optional reply box,
   and the **buttons the backend sends** (the phone has no decision logic of its own). Permission
   requests show the session name, provider, tool and command, and the provider's own option
   labels ("Allow once", "Reject"). A "Loop stalled" card shows the round counts. After an
   answer the card disappears; a stale answer shows "Already answered elsewhere".
5. **Machines and pairing** (`mobile/lib/features/machines/pair_screen.dart`,
   `machines_screen.dart`). First launch opens pairing: camera view for the QR code and a fallback
   "Enter code" form (URL + code). The machines screen lists paired machines (home machine marked),
   lets the owner select one, pair another, or remove one from the phone.

## 5. Backend API for the phone

### 5.1 Transport and network

Today the remote backend is one axum server with one route, `GET /assembly`, a WebSocket guarded
by a single bearer token (`backend/agent_conversation/remote.rs:2985-3002`
`upgrade_remote_socket`), bound to `127.0.0.1:7777` (`run_server_from_environment`,
`remote.rs:2900-2976`, `ASSEMBLY_SERVER_BIND`). The desktop reaches it only through an SSH
tunnel (`open_remote_tunnel`, `remote.rs:2500-2569`) and reads the token over SSH from
`~/.config/assembly/server.env` (`read_remote_token`, `remote.rs:2578-2587`). That token gives full
control (terminals, workspace commands), so it must never reach a phone.

- **Decision: a second, phone-only listener in the same server process.** When
  `ASSEMBLY_PHONE_BIND` is set, `run_server_from_environment` also serves a separate
  `Router` with only the `/phone/v1/*` routes below, sharing `ServerState` (the same
  `AgentRuntimeManager`, store and `events` broadcast, `remote.rs:229-239`). The phone listener
  never serves `/assembly`, and the desktop listener never serves `/phone`. Unset means no phone
  API (the default).
- **Decision: reach it through `tailscale serve`**, which gives a tailnet-only HTTPS URL with a
  real certificate, so Android needs no cleartext or self-signed exceptions. HTTPS certificates
  are already enabled on the tailnet (the workbox already serves several ports on
  `https://agent-workbox.chimp-map.ts.net`, `tailscale serve status`). Ports 8777 and 8778 are
  unused today.
  - Workbox (owner's real backend): `ASSEMBLY_PHONE_BIND=127.0.0.1:7778`, then once
    `tailscale serve --bg --https=8777 http://127.0.0.1:7778`.
  - Workbox Test (proof only): `ASSEMBLY_PHONE_BIND=0.0.0.0:7778` inside the container, add
    `-p 127.0.0.1:7788:7778` to the `docker run` in `tools/workbox-test/README.md`, then
    `tailscale serve --bg --https=8778 http://127.0.0.1:7788` on the workbox host.
- Settings live in a new optional file `~/.config/assembly/phone.env`
  (`ASSEMBLY_PHONE_BIND`, `ASSEMBLY_PHONE_URL` = the public tailnet URL put into QR codes,
  `ASSEMBLY_FCM_SERVICE_ACCOUNT` = path to the push key). The installer rewrites `server.env` on
  every install (`tauri-svelte-preview/scripts/remoteBackendPackage.ts:52-59`), so the unit file
  gets one extra line `EnvironmentFile=-$HOME/.config/assembly/phone.env` (the `-` makes it
  optional) next to the existing one at `remoteBackendPackage.ts:68`.
- The desktop app's own local backend ("This Mac") has no server, so the phone cannot pair with
  it. Runs on This Mac are not visible on the phone. This matches §4.5: long runs and the
  orchestrator live on the workbox.

### 5.2 Pairing and auth

1. Desktop: Settings → Connections → a connected remote machine → **Pair phone**
   (`tauri-svelte-preview/src/lib/shell/components/RemoteConnections.svelte`). The desktop sends
   new `RemoteCommand::CreatePhonePairing` over the existing authenticated socket (enum at
   `remote.rs:80-165`; bump `PROTOCOL_VERSION`, `remote.rs:54`).
2. The backend creates a **pairing code**: 10 random base32 characters, single use, valid 10
   minutes, held in memory only, 5 wrong tries kill it. It returns
   `{url: ASSEMBLY_PHONE_URL, machineName, code, expiresAt}`; with `ASSEMBLY_PHONE_URL` unset it
   returns the error "Phone access is not set up on this machine" naming `phone.env`.
3. The desktop shows a QR code of the JSON `{"v":1,"url":…,"machine":…,"code":…}` (npm
   `qrcode`, rendered as an `<img>` data URL) and the URL and code as text.
4. The phone scans (or types) it and calls `POST /phone/v1/pair {code, deviceName}`. The backend
   returns `{deviceId, deviceToken, machine:{id, name}}`. The device token is 32 random bytes
   (base64url). Only its SHA-256 is stored, in a new `phone_devices` table in the backend's
   `sessions.db` (`id, name, token_hash, fcm_token, created_at, last_seen_at, revoked_at`), via
   `core/src/session_store.rs` like the other tables.
5. Every other `/phone/v1` call needs `Authorization: Bearer <deviceToken>`; the server
   checks the hash against non-revoked rows in one SQL lookup. A missing, unknown or revoked token
   returns 401 and the phone shows "This phone was removed — pair again".
6. The same desktop card lists paired phones (`RemoteCommand::ListPhoneDevices`) with **Remove**
   (`RemoteCommand::RevokePhoneDevice`). Removing sets `revoked_at` and closes that device's live
   socket.

The machine id in pairing is the backend's own stable id (the `phone_devices` table lives in that
backend's database, so ids are per machine); the phone keys everything by `(machineId, …)`.

### 5.3 Endpoints

All JSON, all under `/phone/v1`, all on the phone listener. "Same path as" names the function the
desktop route already calls, so the phone gets no second implementation of any rule.

| Method and path | Request → response | Same path as |
|---|---|---|
| `POST /pair` | `{code, deviceName}` → `{deviceId, deviceToken, machine}` | new (§5.2) |
| `GET /me` | → `{deviceId, machine:{id,name}, hostsOrchestrator}`; also updates `last_seen_at` | Task 21 orchestrator designation |
| `POST /push-token` | `{fcmToken}` → 204 | new |
| `GET /chat?after=<seq>` | → `{items:[ChatItem], cursor, turn:"idle"|"running", ownerItems:[InboxItem]}`; no `after` = latest 100 items | `manager.list_events_before` / `list_events_after` (`remote.rs:3459-3472`) on the designated orchestrator's owned id |
| `POST /chat` | `{text}` → `{itemId}` | `RemoteCommand::Send` handler (`remote.rs:3561`) after Task 21's ensure/resume of the orchestrator; server fills the current generation |
| `GET /runs` | → `{active:[RunRow], recent:[RunRow]}` | Task 12 `list_runs` / Task 17 bounded SQL list |
| `GET /runs/{id}` | → `RunDetail` (steps, attempts, last 20 timeline lines, pending item id) | Task 12 `get_run` + Task 17 `get_run_history(limit 20)` |
| `POST /runs/{id}/pause`, `/resume`, `/stop` | → `RunRow` | the engine pause/resume/stop the desktop uses (Task 12) |
| `GET /inbox` | → `{items:[InboxItem]}` pending only | Task 18 `list_owner_inbox` (includes Task 23 permission requests) |
| `POST /inbox/{id}/answer` | `{optionId, reply?, version}` → 204, or 409 `{error:"stale"}` | Task 18 `answer_owner_item`; permission items go through Task 23's single owner permission path, which ends in `respond_permission_option` (`remote.rs:3620`) |
| `GET /live` (WebSocket) | server → phone frames, below | `ServerState.events` broadcast (`remote.rs:3046`) plus the run/inbox change signal Tasks 12/17/18 add |

Shapes (field names final; extra fields allowed later only when a screen needs them):

```text
ChatItem   {id, seq, kind: owner|agent|relayed|tool|error, from?, text, at, done}
RunRow     {id, taskKey, taskTitle, workflow, state, currentStep, round, startedAt, updatedAt, iMerge, prUrl?, waitingItemId?}
RunDetail  RunRow + {branch, steps:[{id, kind, state, attempts:[{n, state, startedAt, endedAt?, summary?, findings?:[string], failingCommands?:[{command, exit, tail}]}]}], timeline:[{at, text}]}
InboxItem  {id, kind: loop_stalled|round_limit|invalid_review|timeout|gate|ask_owner|restart|permission,
            runId?, taskKey?, stepId?, sessionName?, title, detail, evidence:[{label, text?, url?}],
            options:[{id, label, style: primary|neutral|danger}], allowsReply, createdAt, version}
```

Chat reduction (server side, so the phone never parses the provider event stream): from
`AgentConversationPayload` (`backend/agent_conversation/protocol.rs:453-575`) keep
`UserMessage` (→ `owner`, or `relayed` with `from` when it is a delivered message — use the sender
field slice 1 Task 4 stores; fall back to the `[message from <name>]` prefix),
`AssistantMessage` (→ `agent`), `Tool` (→ `tool`, `summary` or name only), `Error` (→ `error`),
and `Turn` (→ the `turn` field). Everything else is dropped.

Live frames on `GET /live` (the phone reconnects with Rental Command's delays and refetches on
reconnect; nothing is lost because every list is refetchable):

```text
{type:"chat", item: ChatItem}                 orchestrator item added or completed
{type:"chatDelta", itemId, delta}             streaming text for the open agent bubble
{type:"turn", state:"idle"|"running"}
{type:"changed", entity:"runs"|"run"|"inbox", id?}   phone invalidates and refetches (Rental Command's EntityUpdated pattern)
```

### 5.4 Push notifications

- **Decision: Firebase Cloud Messaging**, as Rental Command (`RC/lib/core/push/push_service.dart`,
  server `~/dev/work/rental-management/RentalCommand.Engine/Services/FcmPushSender.cs`, HTTP v1).
  It is the only path that wakes a closed Android app without a battery-draining background
  connection, and the client code is already written.
- **Decision: a new Firebase project "assembly"** (free), not the existing `rental-command` project
  (its `google-services.json` declares only `com.rentalcommand.rental_command`). Needs the
  owner, see §9 Q1: create the project, add Android app `dev.blackcolours.assembly_phone`, put
  `google-services.json` in `mobile/android/app/`, and create a service-account key (role
  "Firebase Cloud Messaging API Admin") saved on each backend machine at
  `~/.config/assembly/fcm-service-account.json` (mode 600), named by `ASSEMBLY_FCM_SERVICE_ACCOUNT`.
- Backend sender: new `backend/agent_conversation/phone_push.rs`. It signs a service-account JWT
  (RS256, new crate `jsonwebtoken`), exchanges it at Google's token endpoint, caches the access
  token until it expires, and POSTs `https://fcm.googleapis.com/v1/projects/<id>/messages:send`
  with the existing `reqwest` (`tauri-svelte-preview/src-tauri/Cargo.toml:27`).
- Trigger: once per new inbox item (the same signal that sends `changed: inbox`), one message per
  device with an `fcm_token`. No retry; failures are logged. An `UNREGISTERED` answer clears that
  token.
- **Decision: no agent text in the notification.** Title "Assembly — <machine>", body is the item
  kind and task key only ("Permission request · tsk-1412/build", "Loop stalled · TSK-1415"). Data:
  `{machineId, itemId}`; tapping opens that item. No action buttons on the notification itself.
- Missing Firebase files or key: the app runs without push and the backend skips sending (both
  fail soft, as Rental Command, `push_service.dart:75-92`).

## 6. D13: only the owner approves

- An answer or run control only comes from `POST /phone/v1/inbox/{id}/answer` or
  `/runs/{id}/pause|resume|stop`, on the phone listener, with a device token. Device tokens are
  issued only by the pairing flow, which needs a code shown on the owner's own desktop.
- Device tokens and session MCP tokens (slice 1 Task 2) are different credentials in different
  stores on different listeners; neither is accepted where the other belongs. Tests below prove
  both directions.
- The phone sends the item id, the chosen **option id from the backend's own list**, and the
  item `version`; the backend rejects unknown options and stale versions (409). The answer is
  recorded through Task 18's path with provenance `{by:"owner", via:"phone", deviceId}`.
- Chat text is never an answer. Typing "yes" sends text to the orchestrator; only the confirm
  card's buttons, which call the inbox endpoint, answer an `ask_owner` item. A relayed message
  from an agent is shown as a labelled message, never as a button.
- The notification has no approve action; the owner opens the app and taps.
- The orchestrator has no tool that answers inbox items (slices 3-4 plan Task 22).

## 7. Build tasks

Rules for every task: KISS/YAGNI block from the slices 3-4 plan applies unchanged (smallest diff;
no new abstraction, flag, shim or factory; ~2x the expected diff or an unlisted new file means
stop and send a one-paragraph proposal). Test first, record the failing run, then the minimum edit.
One heavy job at a time; `free -g` must show more than 6 GB available before Rust or Flutter
builds; `MSBUILDDISABLENODEREUSE=1`. Commits end `Committed-by: <actual committer>`, no co-author
trailer, body `Task: TSK-1416 <url>`. Backend architecture changes update
`main-architecture-explained.html` in the same commit. Everything Flutter runs as
`~/develop/flutter/bin/flutter` from `mobile/`.

Dependencies: Tasks 27, 30 and 31 can start as soon as this spec is approved (they need only
today's code). Tasks 28-29 need slices 3-4 Tasks 12, 17, 18, 21 and 23 merged. Tasks 32-34 need
Task 28's endpoints (they can be built against a fake repository first). Order:
**27 → 31 → 30 → 28 → 29 → 32 → 33 → 34 → 35**.

### Task 27: Phone listener, pairing and device tokens

**Files:** create `backend/agent_conversation/phone_api.rs` (router, pairing, auth; declare in
`agent_conversation/mod.rs`); modify `remote.rs` `run_server_from_environment` (2900) to start the
second listener when `ASSEMBLY_PHONE_BIND` is set, `RemoteCommand` (80) + dispatcher
(`execute_remote_command`, 3389) for `CreatePhonePairing`, `ListPhoneDevices`, `RevokePhoneDevice`,
`PROTOCOL_VERSION` (54); `core/src/session_store.rs` `phone_devices` table and three queries;
`tauri-svelte-preview/scripts/remoteBackendPackage.ts:68` optional `phone.env` line (+ its test).
**Lane:** backend logic.

**Interface:** §5.2 exactly. `POST /phone/v1/pair`, `GET /phone/v1/me`. Pairing codes in a
`Mutex<HashMap>` on `ServerState`; tokens hashed with the existing `sha2` crate (`Cargo.toml:35`).

- [ ] **Red tests** (in `phone_api.rs`): `pairing_code_is_single_use_and_expires`,
  `pairing_code_dies_after_five_wrong_tries`, `phone_routes_reject_missing_unknown_and_revoked_tokens`,
  `phone_listener_does_not_serve_desktop_socket` (GET `/assembly` on the phone router → 404),
  `desktop_server_token_is_not_a_phone_token`. Extend `remoteBackendPackage.test.ts` for the
  optional `EnvironmentFile=-` line.
- [ ] **Verify failure:** `cd tauri-svelte-preview/src-tauri && CARGO_BUILD_JOBS=3 ~/.cargo/bin/cargo test phone_` fails to compile/assert before the module exists.
- [ ] **Minimum edit:** one router, one table, three remote commands. No refresh tokens, no
  device permissions model, no rate limiter beyond the five-tries rule.
- [ ] **Green:** same command passes; `cd core && ~/.cargo/bin/cargo test`; `cd tauri-svelte-preview && node --experimental-strip-types scripts/remoteBackendPackage.test.ts`. Manual: on Workbox Test with `phone.env` set, `curl -s -X POST https://agent-workbox.chimp-map.ts.net:8778/phone/v1/pair -d '{"code":"<code>","deviceName":"curl"}' -H 'content-type: application/json'` returns a token; `curl -H "Authorization: Bearer <token>" …/phone/v1/me` returns the machine.

**Done means:** a phone can pair with a remote backend and authenticate; the desktop socket is unreachable from the phone listener. **Expected diff:** 250-350 lines including tests, plus 5-15 architecture lines.

### Task 28: Phone chat, runs, inbox endpoints and live socket

**Files:** `phone_api.rs` (handlers); `remote.rs` only if the run/inbox change signal is not
already reachable from `ServerState`. No other new file. **Lane:** backend logic.
**Dependencies:** slices 3-4 Tasks 12, 17, 18, 21, 23 merged; re-read their final symbols first.

**Interface:** §5.3 table and shapes. Every handler calls the function the desktop route calls;
no rule is reimplemented. `GET /chat` reduction as §5.3.

- [ ] **Red tests:** `phone_chat_keeps_owner_agent_relayed_tool_and_drops_the_rest` (event list in, ChatItems out),
  `phone_answer_records_owner_device_provenance_once`, `phone_answer_with_unknown_option_or_stale_version_is_rejected`,
  `session_mcp_token_cannot_call_phone_routes`, `phone_run_controls_match_desktop_engine_calls`,
  `phone_live_sends_changed_inbox_when_item_created`.
- [ ] **Verify failure:** `cargo test phone_` (same directory as Task 27) fails on the new tests.
- [ ] **Minimum edit:** handlers plus the chat reducer. Inbox and run lists come from the
  existing single-SQL queries (Tasks 17/18); no per-row lookups. No pagination beyond "latest 100
  chat items" and "last 20 recent runs".
- [ ] **Green:** `cargo test phone_` and backend gates. Manual with curl on Workbox Test: send a chat
  message, list a run, pause and resume it, answer a demo pause; the desktop shows the same state.

**Done means:** every phone screen has its data and actions over the phone listener, with owner provenance. **Expected diff:** 300-450 lines including tests, plus 10-20 architecture lines.

### Task 29: Push on new inbox items

**Files:** create `backend/agent_conversation/phone_push.rs`; `phone_api.rs` (`POST /push-token`,
trigger hook); `Cargo.toml` adds `jsonwebtoken`. **Lane:** backend logic.
**Dependencies:** Task 28; owner's Firebase project and key (§9 Q1) for the real send only.

**Interface:** `send_inbox_push(devices, item)`; message JSON per §5.4; token cache in memory.

- [ ] **Red tests:** `inbox_push_body_has_kind_and_task_key_but_no_agent_text`,
  `one_push_per_device_with_token`, `unregistered_response_clears_token`,
  `missing_service_account_skips_without_error` (fake HTTP server via a test base URL constant).
- [ ] **Verify failure:** `cargo test inbox_push` fails before the module exists.
- [ ] **Minimum edit:** one function, one cached token, no retry queue, no topics.
- [ ] **Green:** `cargo test inbox_push`; with the owner's key on Workbox Test, a demo pause makes the paired phone ring.

**Done means:** a new inbox item notifies each paired phone once, with no agent text. **Expected diff:** 150-220 lines including tests.

### Task 30: Desktop "Pair phone" card

**Files:** `tauri-svelte-preview/src/lib/shell/components/RemoteConnections.svelte` (per-machine
card), `src/lib/tauriSource.ts` (three wrappers), `backend/commands/` remote command passthrough
where the other remote calls live, `package.json` adds `qrcode` + `@types/qrcode`; add
`tauri-svelte-preview/scripts/phonePairing.test.ts`. **Lane:** plumbing, then **visible UI: Opus lane**.
**Dependencies:** Task 27.

**Interface:** "Pair phone" button on a connected remote machine opens a small panel: QR image,
URL and code text, expiry countdown, paired phones list with Remove. Closing the panel aborts the
request (existing `AbortSignal` rule).

- [ ] **Red test:** `pairing_qr_payload_is_versioned_json_with_url_machine_and_code`, `closing_panel_aborts_pairing_request`.
- [ ] **Verify failure:** `cd tauri-svelte-preview && node --experimental-strip-types scripts/phonePairing.test.ts`.
- [ ] **Minimum edit:** stock shadcn-svelte controls; no new settings section. Not shown for This Mac.
- [ ] **Green:** script and frontend gates (`pnpm run check:svelte && npx tsc --noEmit -p . && pnpm test:async-lifecycle`); headless 1710x990 screenshot of the panel against Workbox Test via `workbox-native-ui`, registered with `evidence add`.

**Done means:** the owner can show a pairing QR for a remote machine and remove a phone. **Expected diff:** 120-200 lines.

### Task 31: Flutter shell: theme, machines, pairing, navigation

**Files:** create `mobile/` with `flutter create` (§2), then the copied files of §3.1 (theme,
list widgets, api exception, error widget), plus `lib/main.dart`, `lib/app_shell.dart`,
`lib/core/router.dart`, `lib/core/machines/machine_store.dart`,
`lib/core/api/machine_client.dart`, `lib/features/machines/{pair_screen,machines_screen}.dart`,
`mobile/README.md` (build and install steps, §8). Tests in `mobile/test/`. **Lane:** **visible UI: Opus lane** (plumbing may be split out).
**Dependencies:** Task 27 for real pairing; build against a fake client first.

**Interface:** `PairingPayload.parse(String) -> PairingPayload` (rejects other versions and
non-https URLs); `MachineStore` read/write/select/remove; `machineClientProvider(machineId)` Dio
with bearer header; router redirect to `/pair` when no machine.

- [ ] **Red tests:** `pairing_payload_parses_v1_and_rejects_others`, `machine_store_round_trips_and_selects`,
  `machine_client_sends_device_token`, `no_machine_redirects_to_pair`.
- [ ] **Verify failure:** `cd mobile && ~/develop/flutter/bin/flutter test` fails before the code exists.
- [ ] **Minimum edit:** copy, rename, trim. No flavors, no iOS, no login screens.
- [ ] **Green:** `flutter analyze` clean, `flutter test` passes, `flutter build apk --debug` succeeds.

**Done means:** an installable Assembly app that pairs with a machine and shows three empty tabs in Rental Command's look. **Expected diff:** 500-700 new lines plus ~1,300 copied lines (theme ~1,000, list widgets ~180, exception ~140), counted separately.

### Task 32: Chat screen

**Files:** `mobile/lib/features/chat/{chat_screen,chat_widgets,chat_repository}.dart`,
`mobile/lib/core/live/{live_socket,live_providers}.dart`. **Lane:** **visible UI: Opus lane**.
**Dependencies:** Tasks 28, 31.

**Interface:** `ChatNotifier` (Rental Command `conversationProvider` pattern) with `load()`,
`send(text)`, live `chat` / `chatDelta` / `turn` frames applied in order by `seq`; owner items
rendered as confirm cards that call the inbox answer.

- [ ] **Red tests:** `chat_applies_live_items_in_seq_order_and_ignores_duplicates`,
  `chat_delta_appends_to_open_agent_item_only`, `confirm_card_answers_inbox_not_chat`,
  `typed_yes_sends_chat_text_and_answers_nothing`, `reconnect_refetches_after_cursor`.
- [ ] **Verify failure / Green:** `cd mobile && ~/develop/flutter/bin/flutter test test/chat_*` red then green; `flutter analyze` clean.

**Done means:** the owner can chat with the orchestrator and confirm its `ask_owner` items. **Expected diff:** 400-550 lines including tests.

### Task 33: Runs list and run detail

**Files:** `mobile/lib/features/runs/{runs_screen,run_screen,runs_repository}.dart`. **Lane:** **visible UI: Opus lane**.
**Dependencies:** Tasks 28, 31.

- [ ] **Red tests:** `runs_split_active_and_recent`, `run_controls_show_pause_or_resume_by_state`,
  `stop_requires_confirmation`, `changed_run_notice_refetches_only_that_run`.
- [ ] **Verify failure / Green:** `flutter test test/runs_*` red then green; `flutter analyze` clean.

**Done means:** runs can be seen, opened, paused, resumed and stopped from the phone. **Expected diff:** 400-550 lines including tests.

### Task 34: Inbox and push

**Files:** `mobile/lib/features/inbox/{inbox_screen,inbox_repository}.dart`,
`mobile/lib/core/push/push_service.dart` (adapted copy), `mobile/android/` Firebase plugin lines,
`AndroidManifest.xml` channel and POST_NOTIFICATIONS. **Lane:** **visible UI: Opus lane** for the
screen; push plumbing may be split. **Dependencies:** Tasks 28, 29, 31; owner's `google-services.json`.

- [ ] **Red tests:** `inbox_renders_backend_options_only`, `stale_answer_shows_already_answered`,
  `notification_tap_opens_item_on_its_machine`, `push_init_fails_soft_without_firebase`.
- [ ] **Verify failure / Green:** `flutter test test/inbox_* test/push_*` red then green; `flutter analyze` clean; `flutter build apk --debug`.

**Done means:** the owner gets a notification for a new item and can answer it in the app. **Expected diff:** 350-500 lines including tests.

### Task 35: Slice 5 proof on the owner's phone

**Files:** none in the repo except fixes to Tasks 27-34. **Lane:** QA with the owner present for
the phone's wireless-debugging pairing.

Parent §9 proof: **answering a paused run and a permission request from the phone.** On Workbox
Test (never the owner's real machines for test sessions): pair the phone, chat with the
orchestrator, start the slice 2 demo workflow from the desktop, receive the push for its loop
pause, answer it on the phone, see the run continue on the desktop; trigger a permission request
in a Workbox Test session, answer it on the phone, confirm the provider got exactly one response;
send "approve" from an agent and show the item stays pending. Chat with the orchestrator once
with it on Claude Code and once on Codex (the project's four-scenario rule; the two "This Mac"
rows are reported as not applicable because the phone cannot reach a local backend, §5.1). The
desktop chat code is not changed by this slice. Screenshots with
`adb exec-out screencap -p > ~/Workbox/screenshots/TSK-1416-phone-<what>-after.png` and
`evidence add … --task TSK-1416`, at most four plus one short video. Stop owned processes; remove
the test device from the paired list afterwards.

**Done means:** slice 5 proven end to end on a real Android phone. **Expected diff:** 0-30 lines.

## 8. Build, install and release

- Build: `cd mobile && ~/develop/flutter/bin/flutter build apk --debug` (proof) or `--release`
  (daily use). Release signing copies Rental Command's `key.properties` pattern
  (`RC/android/app/build.gradle.kts:16-22,65-75`) with a new upload key; without it the build is
  debug-signed, as in Rental Command.
- Install: the workbox has no emulator (`~/dev/work/rental-management/Docs/mobile-testing-on-workbox.md`:
  no `/dev/kvm`). Use the owner's phone over Tailscale with wireless debugging, as documented there:
  `adb pair <phone>:<port> <code>`, `adb connect <phone>:<port>`, `adb install -r
  build/app/outputs/flutter-apk/app-debug.apk`.
- **Decision: no Play Store release in this slice.** Sideloaded APK only; Play internal testing
  (Rental Command's `Docs/legal/play-store-internal-testing-steps.md`) can follow later.

## 9. Open questions for the owner (each has a recommended answer)

1. **Firebase project for push.** Create a new free Firebase project "assembly" with Android app
   `dev.blackcolours.assembly_phone`, or add that app to the existing `rental-command` project?
   **Recommended: new project.** Then drop `google-services.json` and the service-account key
   into `~/Workbox/to-box/` (about 10 minutes in the Firebase console). Until then everything
   except the real push works.
2. **Extra check before approving from the phone** (fingerprint prompt, as Rental Command's
   `local_auth`)? **Recommended: no for now.** The phone's own lock screen plus revocable device
   tokens are enough; add a fingerprint prompt for permission requests later if wanted.
3. **What a notification may say.** **Recommended: kind and task key only**, no agent text, since
   notification text passes through Google.
4. **Phone proof needs you at the phone** for wireless-debugging pairing (its port rotates).
   **Recommended:** everything up to Task 34 is built and unit-tested without you; schedule ~15
   minutes together for Task 35.
5. **App id and name** `dev.blackcolours.assembly_phone`, shown as "Assembly". **Recommended: yes.**
