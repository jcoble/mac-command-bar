import assert from 'node:assert/strict';
import { createBrowserWorkspace } from '../src/lib/shell/browser/browserTypes.ts';
import { createBrowserModel } from '../src/lib/shell/browser/browserModel.ts';

const workspace = createBrowserWorkspace({ workspaceId: 'workspace-1', ownedId: 'owned-1' });
const model = createBrowserModel({ workspace, now: () => '2026-08-05T00:00:00.000Z' });
model.createBrowserTab({ url: 'https://example.com/' });
model.beginBrowserElementPicker('annotation');
model.acceptBrowserElementSelection({
  selector: '#save',
  accessibleName: 'Save',
  textSnippet: 'Save changes'
});
const queued = model.queueBrowserAnnotation({ note: 'Make this button easier to find', intent: 'change' });

let draft = 'Please review this page.';
const stagedAttachments = [];
const bridge = {
  read: () => ({ ownedId: 'owned-1', generation: model.workspace.activeGeneration, draft, attachments: stagedAttachments }),
  setDraft: (_ownedId, value) => {
    draft = value;
  },
  setAttachments: (_ownedId, value) => {
    stagedAttachments.splice(0, stagedAttachments.length, ...value);
  }
};

const preview = model.stageBrowserFeedbackPreview({ bridge, draftSnapshot: draft });
assert.equal(preview.status, 'ready');
assert.match(preview.mergedDraft, /Make this button easier to find/);
assert.equal(model.workspace.queue.length, 1, 'preview never removes the queue item');
const committed = await preview.commit();
assert.equal(committed.status, 'staged');
assert.equal(model.workspace.queue.length, 0, 'only commit removes the queue item');
assert.match(draft, /Annotated browser page/);

model.beginBrowserElementPicker('annotation');
model.acceptBrowserElementSelection({ selector: '#question' });
model.queueBrowserAnnotation({ note: 'Is this label clear?', intent: 'question' });
const stalePreview = model.stageBrowserFeedbackPreview({ bridge, generation: model.workspace.activeGeneration - 1, draftSnapshot: draft });
assert.equal(stalePreview.status, 'stale-target');
assert.equal(stalePreview.queueRetained, true);

const conflictPreview = model.stageBrowserFeedbackPreview({ bridge, draftSnapshot: 'a different draft' });
assert.equal(conflictPreview.status, 'draft-conflict');
assert.equal(conflictPreview.queueRetained, true);
draft = `${draft} changed outside preview`;
assert.equal((await conflictPreview.commit()).status, 'draft-conflict');
assert.equal(model.workspace.queue.length, 1);

const markupModel = createBrowserModel({
  workspace: createBrowserWorkspace({ workspaceId: 'workspace-markup', ownedId: 'owned-markup' }),
  now: () => '2026-08-05T00:00:00.000Z'
});
markupModel.createBrowserTab({ url: 'https://example.com/' });
await markupModel.captureBrowserWorkspace({ forMarkup: true });
const markup = markupModel.queueBrowserAnnotation({
  capture: {
    mimeType: 'image/png',
    bytes: [1, 2, 3],
    width: 10,
    height: 10,
    sourceHash: 'capture-1'
  },
  note: 'Circle this area',
  intent: 'context'
});
assert.equal(markup.kind, 'markup');
assert.equal(markupModel.workspace.queue.length, 1);

console.log('browserFeedback: all tests passed');
