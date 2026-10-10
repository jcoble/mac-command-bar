#!/usr/bin/env bash
# One-time back-fill (TSK-1434): files a remote machine's older sessions, the
# ones its server holds with no project, under the project this Mac registered
# for that machine. Run it on the Mac. The Mac's next connect copies each
# project onto its own cached rows.
#
#   tools/backfill-remote-session-projects.sh <mac sessions.db> <machine id> <ssh host> [server sessions.db]
#
# Machine ids:  sqlite3 <mac sessions.db> "SELECT DISTINCT machine FROM projects WHERE machine <> 'local'"
# The server database defaults to ~/.local/share/assembly/sessions.db on the host.
# It is copied to <server db>.before-tsk-1434-<time> first. Only sessions with no
# project are changed: the deepest project root at or above their folder,
# else the project that shares their git repository (worktrees).
set -euo pipefail
[ $# -ge 3 ] || { sed -n 6,13p "$0"; exit 2; }
mac_db=$1 machine=$2 host=$3 server_db=${4:-.local/share/assembly/sessions.db}
[[ $machine =~ ^[A-Za-z0-9._:-]+$ ]] || { echo "Unexpected machine id: $machine" >&2; exit 2; }
projects=$(sqlite3 -readonly -separator $'\t' "$mac_db" \
  "SELECT id, root_path FROM projects WHERE machine = '$machine' ORDER BY created_at, id" | base64 | tr -d '\n')
[ -n "$projects" ] || { echo "No projects for machine $machine in $mac_db" >&2; exit 1; }

ssh "$host" bash -s -- "$(printf %q "$server_db")" "$projects" <<'REMOTE'
set -euo pipefail
db=$1
command -v sqlite3 >/dev/null || { echo "sqlite3 is not installed on $(hostname)" >&2; exit 1; }
q() { printf "'%s'" "${1//\'/\'\'}"; }
common() { git -C "$1" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true; }
sqlite3 "$db" ".backup $(q "$db.before-tsk-1434-$(date +%Y%m%d%H%M%S)")"
{
  echo ".timeout 5000"
  echo "CREATE TEMP TABLE p(id TEXT, root TEXT, common TEXT); CREATE TEMP TABLE c(cwd TEXT, common TEXT);"
  while IFS=$'\t' read -r id root; do
    echo "INSERT INTO p VALUES($(q "$id"), $(q "$root"), $(q "$(common "$root")"));"
  done < <(printf %s "$2" | base64 -d)
  while IFS= read -r cwd; do
    echo "INSERT INTO c VALUES($(q "$cwd"), $(q "$(common "$cwd")"));"
  done < <(sqlite3 "$db" "SELECT DISTINCT cwd FROM sessions WHERE project_id IS NULL AND cached_remote_profile_id IS NULL AND cwd <> ''")
  cat <<'SQL'
CREATE TEMP TABLE b AS SELECT count(*) AS n FROM sessions WHERE project_id IS NULL AND cached_remote_profile_id IS NULL;
UPDATE sessions SET project_id = COALESCE(
  (SELECT p.id FROM p WHERE sessions.cwd = p.root OR substr(sessions.cwd, 1, length(p.root) + 1) = p.root || '/'
   ORDER BY length(p.root) DESC, p.rowid LIMIT 1),
  (SELECT p.id FROM c JOIN p ON p.common = c.common WHERE c.cwd = sessions.cwd AND c.common <> '' ORDER BY p.rowid LIMIT 1))
WHERE project_id IS NULL AND cached_remote_profile_id IS NULL AND cwd <> '';
SELECT ((SELECT n FROM b) - count(*)) || ' sessions filed under a project, ' || count(*) || ' still without one'
FROM sessions WHERE project_id IS NULL AND cached_remote_profile_id IS NULL;
SQL
} | sqlite3 "$db"
REMOTE
