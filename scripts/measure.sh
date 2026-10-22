#!/usr/bin/env bash
# The memory check for Assembly: proportional set size (PSS) and private dirty
# memory from /proc/<pid>/smaps_rollup, summed over the app's own process tree
# (the main process, the web process and every child). Linux only; it runs on
# the workbox against a real app, before merge, with the numbers in the PR body.
#
#   measure.sh sample <app-pid>
#       One row per process: pid, PSS MB, private dirty MB, name; then a
#       `total` row.
#   measure.sh check <before-MB> <after-MB>...
#       The pass rule: each after-idle value is no more than 15 MB above the
#       before value, and no more than 15 MB above the repeat before it.
#       Exits 0 on pass, 1 on fail, 2 on bad input.
#   measure.sh run <app-pid> <repeats> <idle-seconds> <cycle command>...
#       Samples, then for each repeat runs the cycle command, waits the idle
#       time and samples again; applies the rule to PSS and to private dirty.
set -euo pipefail

LIMIT_MB=15

usage() {
  sed -n '7,16p' "$0" | sed 's/^# \{0,1\}//' >&2
  exit 2
}

# The pid and every descendant, from the kernel's own child lists.
process_tree() {
  local pid=$1 child
  [ -d "/proc/$pid" ] || return 0
  echo "$pid"
  for child in $(cat /proc/"$pid"/task/*/children 2>/dev/null); do
    process_tree "$child"
  done
}

sample() {
  local root=$1 pid name pss dirty total_pss=0 total_dirty=0
  [ -d "/proc/$root" ] || { echo "measure.sh: no process $root" >&2; exit 2; }
  for pid in $(process_tree "$root"); do
    # A process can exit between the listing and the read; it then counts nothing.
    read -r pss dirty < <(awk '/^Pss:/ { p = $2 } /^Private_Dirty:/ { d = $2 } END { print p + 0, d + 0 }' \
      "/proc/$pid/smaps_rollup" 2>/dev/null || echo "0 0")
    name=$(cat "/proc/$pid/comm" 2>/dev/null || echo exited)
    printf '%s %.1f %.1f %s\n' "$pid" "$(mb "$pss")" "$(mb "$dirty")" "$name"
    total_pss=$((total_pss + pss))
    total_dirty=$((total_dirty + dirty))
  done
  printf 'total %.1f %.1f\n' "$(mb "$total_pss")" "$(mb "$total_dirty")"
}

mb() { awk -v kb="$1" 'BEGIN { printf "%.1f", kb / 1024 }'; }

check() {
  [ $# -ge 2 ] || { echo "measure.sh: check needs a before value and at least one repeat" >&2; exit 2; }
  local value
  for value in "$@"; do
    [[ $value =~ ^[0-9]+([.][0-9]+)?$ ]] || { echo "measure.sh: not a number of MB: $value" >&2; exit 2; }
  done
  awk -v limit="$LIMIT_MB" 'BEGIN {
    start = ARGV[1]; previous = start; failed = 0
    for (i = 2; i < ARGC; i++) {
      value = ARGV[i]; repeat = i - 1
      if (value - start > limit) {
        printf "repeat %d: %s MB is more than %d MB above the start (%s MB)\n", repeat, value, limit, start; failed = 1
      } else if (value - previous > limit) {
        printf "repeat %d: %s MB is more than %d MB above repeat %d (%s MB)\n", repeat, value, limit, repeat - 1, previous; failed = 1
      } else {
        printf "repeat %d: %s MB (start %s MB) ok\n", repeat, value, start
      }
      previous = value
    }
    exit failed
  }' "$@"
}

run() {
  [ $# -ge 4 ] || usage
  local pid=$1 repeats=$2 idle=$3 repeat totals
  shift 3
  [[ $repeats =~ ^[1-9][0-9]*$ && $idle =~ ^[0-9]+$ ]] || usage
  local -a pss dirty
  totals=$(sample "$pid" | tail -1)
  read -r _ pss[0] dirty[0] <<<"$totals"
  echo "before: PSS ${pss[0]} MB, private dirty ${dirty[0]} MB"
  for repeat in $(seq 1 "$repeats"); do
    "$@"
    sleep "$idle"
    totals=$(sample "$pid" | tail -1)
    read -r _ pss[repeat] dirty[repeat] <<<"$totals"
    echo "repeat $repeat: PSS ${pss[repeat]} MB, private dirty ${dirty[repeat]} MB"
  done
  local status=0
  echo "PSS:"
  check "${pss[@]}" || status=1
  echo "Private dirty:"
  check "${dirty[@]}" || status=1
  if [ "$status" -eq 0 ]; then echo "PASS"; else echo "FAIL"; fi
  return "$status"
}

case "${1:-}" in
  sample) [ $# -eq 2 ] || usage; sample "$2" ;;
  check) shift; check "$@" ;;
  run) shift; run "$@" ;;
  *) usage ;;
esac
