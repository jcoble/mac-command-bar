const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } = require("node:fs");
const { tmpdir } = require("node:os");
const path = require("node:path");
const { test } = require("node:test");

const script = path.join(__dirname, "capture-footprint.sh");

function capture(appPid: string, helpers: string, footprintStatus = "0") {
  const root = mkdtempSync(path.join(tmpdir(), "capture-footprint-test-"));
  const bin = path.join(root, "bin");
  const output = path.join(root, "output with spaces");
  const calls = path.join(root, "calls");
  mkdirSync(bin);
  writeFileSync(path.join(bin, "pgrep"), `#!/bin/sh
if [ "$1" = "-of" ] && [ "$2" = '^(.*/)?(test-app)([[:space:]]|$)' ]; then
  [ -n "$APP_PID" ] || exit 1
  printf '%s\\n' "$APP_PID"
elif [ "$1" = "-f" ] && [ "$2" = 'com\\.apple\\.WebKit' ]; then
  [ -n "$HELPERS" ] || exit 1
  printf '%s\\n' "$HELPERS"
else
  exit 2
fi
`, { mode: 0o755 });
  writeFileSync(path.join(bin, "footprint"), `#!/bin/sh
printf '%s\\n' "$*" >> "$CALLS"
[ "$#" -eq 2 ] && [ "$1" = "-p" ] && [ -n "$2" ] || exit 64
printf 'Footprint for %s\\n' "$2"
printf 'diagnostic for %s\\n' "$2" >&2
exit "$FOOTPRINT_STATUS"
`, { mode: 0o755 });
  const result = spawnSync("bash", [script, output, "test-app"], {
    encoding: "utf8",
    env: {
      ...process.env, PATH: `${bin}:${process.env.PATH}`,
      APP_PID: appPid, HELPERS: helpers, CALLS: calls, FOOTPRINT_STATUS: footprintStatus,
    },
  });
  return {
    root, output, calls, result,
    cleanup: () => rmSync(root, { recursive: true, force: true }),
  };
}

test("missing app fails clearly without calling footprint or creating output", () => {
  const run = capture("", "101\n102");
  try {
    assert.equal(run.result.status, 1);
    assert.match(run.result.stderr, /App process not found/);
    assert.deepEqual(readdirSync(run.root), ["bin"]);
  } finally { run.cleanup(); }
});

test("captures the oldest matching app and each WebKit PID, including diagnostics", () => {
  const run = capture("100", "101\n102");
  try {
    assert.equal(run.result.status, 0, run.result.stderr);
    const folder = run.result.stdout.trim();
    assert.equal(path.dirname(folder), run.output);
    assert.deepEqual(readdirSync(folder).sort(), ["app-100.txt", "webkit-101.txt", "webkit-102.txt"]);
    assert.equal(readFileSync(run.calls, "utf8"), "-p 100\n-p 101\n-p 102\n");
    assert.match(readFileSync(path.join(folder, "app-100.txt"), "utf8"), /diagnostic for 100/);
  } finally { run.cleanup(); }
});

test("no WebKit helpers still captures the app successfully", () => {
  const run = capture("100", "");
  try {
    assert.equal(run.result.status, 0, run.result.stderr);
    assert.deepEqual(readdirSync(run.result.stdout.trim()), ["app-100.txt"]);
  } finally { run.cleanup(); }
});

test("a failed footprint capture returns failure and retains its diagnostic", () => {
  const run = capture("100", "", "7");
  try {
    assert.equal(run.result.status, 7);
    const folder = path.join(run.output, readdirSync(run.output)[0]);
    assert.match(readFileSync(path.join(folder, "app-100.txt"), "utf8"), /diagnostic for 100/);
  } finally { run.cleanup(); }
});
