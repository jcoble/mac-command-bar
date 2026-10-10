import assert from 'node:assert/strict';

import { resumeCommand } from '../src/lib/shell/resumeCommand.ts';
import { sessionRowMenuItems } from '../src/lib/shell/components/sessionRowMenu.ts';
import { sessionResumeCommand } from '../src/lib/shell/panels/history/sessionHistoryActions.ts';
import type { SessionLibraryRecord } from '../src/lib/shell/sessionLibrary/sessionLibraryModel.ts';

const ID = 'b1334810-309c-4e16-bc00-d1431b83ba4c';

// Claude has to be started in the session's folder to find the transcript.
assert.equal(
  resumeCommand('claude', ID, '/home/me/dev/EdiPlatform'),
  `cd /home/me/dev/EdiPlatform && claude --resume ${ID}`
);
// Codex: `codex resume [SESSION_ID]`, per `codex resume --help`.
assert.equal(resumeCommand('codex', ID, '/work/app'), `cd /work/app && codex resume ${ID}`);

// A folder with spaces or shell characters is single-quoted, quotes escaped.
assert.equal(
  resumeCommand('claude', ID, "/work/Bob's app"),
  `cd '/work/Bob'\\''s app' && claude --resume ${ID}`
);
assert.equal(resumeCommand('claude', ID, '/work/a$b'), `cd '/work/a$b' && claude --resume ${ID}`);

// Nothing to copy without the provider's id, or for an agent we have no command for.
assert.equal(resumeCommand('claude', null, '/work'), null);
assert.equal(resumeCommand('claude', '  ', '/work'), null);
assert.equal(resumeCommand('gemini', ID, '/work'), null);
// No folder: the bare command still resumes.
assert.equal(resumeCommand('codex', ID, ''), `codex resume ${ID}`);

// The rail's right-click menu offers the command and shows the id on hover.
const items = sessionRowMenuItems({
  status: 'working',
  sessionId: ID,
  resumeCommand: `cd /work && claude --resume ${ID}`,
  worktreePath: '/work',
  pinned: false
});
const copyId = items.find((item) => item.id === 'copy-session-id');
const copyResume = items.find((item) => item.id === 'copy-resume-command');
assert.equal(copyId?.hint, ID);
assert.equal(copyResume?.enabled, true);
assert.equal(copyResume?.label, 'Copy resume command');
assert.match(copyResume?.hint ?? '', /cd \/work && claude --resume/);
assert.match(copyResume?.hint ?? '', /not .*both/i);
assert.equal(
  items.findIndex((item) => item.id === 'copy-resume-command'),
  items.findIndex((item) => item.id === 'copy-session-id') + 1
);
const noResume = sessionRowMenuItems({
  status: 'working',
  sessionId: null,
  resumeCommand: null,
  worktreePath: null,
  pinned: false
}).find((item) => item.id === 'copy-resume-command');
assert.equal(noResume?.enabled, false);

// History: a session started in this app has no scan details, but its card
// still copies the same command the rail does.
const owned = {
  provider: 'claude',
  nativeSessionId: ID,
  canonicalCwd: '/work/My Project',
  available: null
} as unknown as SessionLibraryRecord;
assert.equal(sessionResumeCommand(owned), `cd '/work/My Project' && claude --resume ${ID}`);
const scannedCodex = {
  provider: 'codex',
  nativeSessionId: ID,
  canonicalCwd: '/work',
  available: { resumeCommands: [`codex resume ${ID}`] }
} as unknown as SessionLibraryRecord;
assert.equal(sessionResumeCommand(scannedCodex), `cd /work && codex resume ${ID}`);
// Agents the builder does not know keep the scanner's own command.
const gemini = {
  provider: 'cmux-gemini',
  nativeSessionId: ID,
  canonicalCwd: '/work',
  available: { resumeCommands: [`gemini --resume ${ID}`] }
} as unknown as SessionLibraryRecord;
assert.equal(sessionResumeCommand(gemini), `gemini --resume ${ID}`);

console.log('resumeCommand tests passed');
