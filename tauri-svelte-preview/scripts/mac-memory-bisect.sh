#!/usr/bin/env bash
# Finds which part of the chat transcript holds memory (TSK-1299). macOS only.
#
#   mac-memory-bisect.sh [--app <built binary>] [--dry-run]
#
# Builds the app once (or uses --app), then for each case starts it with ASSEMBLY_DEBUG_OFF=<case>
# on a fresh copy of this Mac's sessions database (the real one is only read), lets it switch
# between the longest local chats 40 times, and reads the web process's physical footprint at the
# start, after the switches, and after 60 s idle. The case whose growth collapses names the part.
# --dry-run prints what each case would run and starts nothing.
set -euo pipefail

CASES=(none tools tool-output markdown code icons effects virtual activity all)
SESSIONS=5
IDLE_S=60
PER_CASE_S=155 # start, settle, 40 switches 1.5 s apart, idle, stop
PREVIEW="$(cd "$(dirname "$0")/.." && pwd)"
IDENTIFIER=dev.blackcolours.MacCommandBarWebviewPreview
REAL_DATA="$HOME/Library/Application Support/$IDENTIFIER"
APP="${CARGO_TARGET_DIR:-$PREVIEW/src-tauri/target}/release/mac-command-bar-webview-preview"
DRY=0
BUILD=1
while [ $# -gt 0 ]; do
  case "$1" in
    --app) APP="$2"; BUILD=0; shift 2 ;;
    --dry-run) DRY=1; shift ;;
    *) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2 ;;
  esac
done
WORK="${TMPDIR:-/tmp}/assembly-memory-bisect-$(date +%Y%m%d-%H%M%S)"
SEED="$WORK/seed"
RESULTS="$WORK/results.txt"

if [ "$DRY" = 0 ]; then
  [ "$(uname -s)" = Darwin ] || { echo "This script measures the Mac app; run it on the Mac (or use --dry-run)." >&2; exit 1; }
  for tool in sqlite3 python3 lsof; do command -v "$tool" >/dev/null || { echo "Needs $tool" >&2; exit 1; }; done
  [ -f "$REAL_DATA/sessions.db" ] || { echo "No sessions database at $REAL_DATA" >&2; exit 1; }
fi
run() { if [ "$DRY" = 1 ]; then printf '[dry-run] %s\n' "$*"; else "$@"; fi; }

echo "Cases: ${CASES[*]}"
echo "About $(( ${#CASES[@]} * PER_CASE_S / 60 )) minutes for the cases, plus the build. Leave the Mac alone until the table prints."

if [ "$BUILD" = 1 ]; then
  echo "Building the app once (the switches are read when it starts, so one build serves every case)."
  (cd "$PREVIEW" && run pnpm install --frozen-lockfile && run pnpm exec tauri build --no-bundle)
fi
[ "$DRY" = 1 ] || [ -x "$APP" ] || { echo "No built app at $APP" >&2; exit 1; }

# The copy: remote machines and their cached chats are removed so the test app signs in nowhere.
run mkdir -p "$SEED"
run sqlite3 -readonly "$REAL_DATA/sessions.db" ".backup '$SEED/sessions.db'"
run sqlite3 "$SEED/sessions.db" "PRAGMA foreign_keys=ON;
DELETE FROM sessions WHERE cached_remote_profile_id IS NOT NULL;
DELETE FROM app_settings WHERE setting_key LIKE 'remote-assembly.%';
DELETE FROM projects WHERE machine <> 'local';"
if [ "$DRY" = 1 ]; then
  ids="<the $SESSIONS longest local chats>"
else
  [ -d "$REAL_DATA/conversation-attachments" ] && cp -cR "$REAL_DATA/conversation-attachments" "$SEED/" 2>/dev/null || true
  ids=$(sqlite3 "$SEED/sessions.db" "SELECT s.owned_id FROM sessions s JOIN events e ON e.owned_id = s.owned_id
    WHERE s.parent_owned_id IS NULL GROUP BY s.owned_id ORDER BY sum(length(e.payload)) DESC LIMIT $SESSIONS;" | paste -sd, -)
  [ "$(tr ',' '\n' <<<"$ids" | grep -c .)" -ge 2 ] || { echo "Fewer than two local chats to switch between." >&2; exit 1; }
  sqlite3 "$SEED/sessions.db" "SELECT count(*) || ' events in the ' || count(DISTINCT owned_id) || ' chats switched between' FROM events
    WHERE owned_id IN ('$(sed "s/,/','/g" <<<"$ids")');"
fi

# Physical footprint (Activity Monitor's Memory) of the WebContent processes this app started.
footprint_mb() { python3 - "$@" <<'PY'
import ctypes, subprocess, sys
app, before = int(sys.argv[1]), set(sys.argv[2].split(','))
lib = ctypes.CDLL(None)
owner = lib.responsibility_get_pid_responsible_for_pid
info = (ctypes.c_uint64 * 32)()
total = count = 0
for line in subprocess.run(['ps', '-axo', 'pid=,comm='], capture_output=True, text=True).stdout.splitlines():
    pid, _, command = line.strip().partition(' ')
    if not command.endswith('com.apple.WebKit.WebContent') or pid in before: continue
    if owner(int(pid)) not in (app, owner(app)) or lib.proc_pid_rusage(int(pid), 0, info) != 0: continue
    total += info[9]  # rusage_info_v0.ri_phys_footprint
    count += 1
if not count: sys.exit('no WebContent process found for the test app')
print(f'{total / 1048576:.0f}')
PY
}
wait_for() { # <log> <text> <seconds>
  for _ in $(seq "$3"); do grep -q "assembly-debug: $2" "$1" 2>/dev/null && return 0; kill -0 "$pid" 2>/dev/null || break; sleep 1; done
  echo "The app never logged '$2'; see $1" >&2; return 1
}
pid=""
stop_app() { if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi; pid=""; }
trap 'stop_app; rm -rf "$WORK"/home-* "$SEED"' EXIT # the copies of the database go; results and logs stay

[ "$DRY" = 1 ] || printf '%-12s %10s %16s %15s %8s\n' case start_MB after_switch_MB after_idle_MB delta | tee "$RESULTS"
for case in "${CASES[@]}"; do
  home="$WORK/home-$case"
  data="$home/Library/Application Support/$IDENTIFIER"
  log="$WORK/$case.log"
  run mkdir -p "$data"
  run cp -cR "$SEED/" "$data/"
  if [ "$DRY" = 1 ]; then
    run env HOME="$home" ASSEMBLY_DEBUG_OFF="$case" ASSEMBLY_DEBUG_SWITCH_SESSIONS="$ids" "$APP" ">$log 2>&1 &"
    echo "[dry-run] wait for 'assembly-debug: ready', +10 s footprint; wait for 'assembly-debug: switches done', footprint; +${IDLE_S} s footprint; stop the app"
    continue
  fi
  before=$(ps -axo pid=,comm= | awk '/com\.apple\.WebKit\.WebContent$/ {print $1}' | paste -sd, -)
  env HOME="$home" ASSEMBLY_DEBUG_OFF="$case" ASSEMBLY_DEBUG_SWITCH_SESSIONS="$ids" "$APP" >"$log" 2>&1 &
  pid=$!
  wait_for "$log" ready 120
  if lsof -p "$pid" -Fn 2>/dev/null | grep -qF "$REAL_DATA"; then echo "The test app opened the real data folder; stopping." >&2; exit 1; fi
  sleep 10
  start=$(footprint_mb "$pid" "$before")
  wait_for "$log" "switches done" 180
  after=$(footprint_mb "$pid" "$before")
  sleep "$IDLE_S"
  idle=$(footprint_mb "$pid" "$before")
  stop_app
  printf '%-12s %10s %16s %15s %+8d\n' "$case" "$start" "$after" "$idle" "$((idle - start))" | tee -a "$RESULTS"
done
[ "$DRY" = 1 ] && exit 0
echo "Saved: $RESULTS (logs beside it). Compare each delta with 'none': the case where it drops to near zero is the part holding memory."
