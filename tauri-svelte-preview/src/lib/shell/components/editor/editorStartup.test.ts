/**
 * Whether the code editor's placeholder bars can ever be the last thing shown.
 *
 * Run it with:
 *   node --experimental-strip-types \
 *     src/lib/shell/components/editor/editorStartup.test.ts
 *
 * The reported bug was an editor tab that opened with the right title and then
 * showed shimmering bars for good. Both halves of that are pinned here:
 *  1. the host element the editor is built into is off the document for a
 *     stretch of frames at launch, and the wait has to outlast that — the
 *     previous single-frame wait returned "give up" and said nothing;
 *  2. every way of failing produces a sentence naming the file, so the bars
 *     are replaced by something the reader can act on.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  editorStartFailureMessage,
  retryRejectedStart,
  waitForRegisteredStart,
  waitForConnectedHost,
  EDITOR_START_LIMIT_MS
} from './editorStartup.ts';

/** A host that joins the document on a given frame. */
function hostConnectingOnFrame(target: number) {
  let frame = 0;
  return {
    frames: () => frame,
    wait: {
      isConnected: () => frame >= target,
      isWanted: () => true,
      nextFrame: async () => {
        frame += 1;
      }
    }
  };
}

test('a host already in the document needs no waiting at all', async () => {
  const host = hostConnectingOnFrame(0);
  assert.equal(await waitForConnectedHost(host.wait), true);
  assert.equal(host.frames(), 0);
});

test('the wait outlasts a host that only joins the document several frames in', async () => {
  // The shell moves the center surfaces into their dock hosts after the panels
  // inside them have mounted. One frame was not enough, and that is the bug.
  const host = hostConnectingOnFrame(6);
  assert.equal(await waitForConnectedHost(host.wait), true);
  assert.equal(host.frames(), 6);
});

test('an editor nobody wants any more stops waiting, and says so', async () => {
  let frame = 0;
  const answer = await waitForConnectedHost({
    isConnected: () => false,
    isWanted: () => frame < 3,
    nextFrame: async () => {
      frame += 1;
    }
  });
  assert.equal(answer, false);
  assert.equal(frame, 3);
});

test('a host that connects only after the editor was given up on does not count', async () => {
  // Both true at once: the element is back in the document, but the component
  // has been torn down. Building an editor into it now would leak one.
  const answer = await waitForConnectedHost({
    isConnected: () => true,
    isWanted: () => false,
    nextFrame: async () => {}
  });
  assert.equal(answer, false);
});

test('a failure names the file that did not open', () => {
  const message = editorStartFailureMessage(
    'RentalCommand.Api/Controllers/AccountingController.cs',
    new Error('Failed to fetch dynamically imported module')
  );
  assert.match(message, /RentalCommand\.Api\/Controllers\/AccountingController\.cs/);
  assert.match(message, /Failed to fetch dynamically imported module/);
});

test('running out of time reads as running out of time, not as an empty error', () => {
  const message = editorStartFailureMessage('src/app.ts');
  assert.match(message, /src\/app\.ts/);
  assert.match(message, /in time/);
  assert.doesNotMatch(message, /undefined/);
});

test('a reason with nothing in it adds nothing to the sentence', () => {
  assert.equal(
    editorStartFailureMessage('src/app.ts', new Error('   ')),
    'The code editor could not be started, so src/app.ts cannot be shown.'
  );
});

test('with no file known the sentence still stands on its own', () => {
  const message = editorStartFailureMessage(null, new Error('boom'));
  assert.equal(message, 'The code editor could not be started. boom');
});

test('the time limit is a real bound, so the bars always end', () => {
  assert.ok(EDITOR_START_LIMIT_MS > 0 && Number.isFinite(EDITOR_START_LIMIT_MS));
});

test('a rejected services start is retried', async () => {
  let attempts = 0;
  const start = retryRejectedStart(async () => {
    attempts += 1;
    if (attempts === 1) throw new Error('first start failed');
  });

  await assert.rejects(start(), /first start failed/);
  await start();
  assert.equal(attempts, 2);
});

test('a resolved services start stays cached', async () => {
  let attempts = 0;
  const start = retryRejectedStart(async () => {
    attempts += 1;
  });

  await Promise.all([start(), start()]);
  await start();
  assert.equal(attempts, 1);
});

test('waitReturnsTrueWhenAStartRegistersAndFinishes', async () => {
  let start: (() => Promise<void>) | null = null;
  let finished = false;
  const waiting = waitForRegisteredStart(100, () => start);

  setTimeout(() => {
    start = async () => {
      finished = true;
    };
  }, 0);

  assert.equal(await waiting, true);
  assert.equal(finished, true);
});

test('waitReturnsFalseAfterTimeoutWithNoStart', async () => {
  assert.equal(await waitForRegisteredStart(5, () => null), false);
});

test('waitReturnsFalseWhenStartRejects', async () => {
  const start = async () => {
    throw new Error('services start failed');
  };
  assert.equal(await waitForRegisteredStart(100, () => start), false);
});
