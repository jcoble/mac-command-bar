import assert from 'node:assert/strict';
import { configOptionPlacement } from '../src/lib/shell/conversation/conversationTypes.ts';
import {
  configValueIsAdvertised,
  conversationControlViews,
  controlsForPlacement
} from '../src/lib/shell/conversation/conversationControls.ts';

const capabilities = {
  revision: 4,
  provider: 'codex',
  implementation: { name: 'fixture', version: '1' },
  session: { list: true, load: true, resume: true, close: true, steering: true },
  prompt: { text: true, image: true, embeddedContext: true, resourceLinks: true },
  interaction: { permissions: true, structuredUserInput: true, toolTerminals: true, plans: true, tasks: true, subagents: true },
  configOptions: [
    { id: 'model', label: 'Model', category: 'model', value: 'fixture-model', choices: [{ value: 'fixture-model', label: 'Fixture' }] },
    { id: 'effort', label: 'Effort', category: 'thought_level', value: 'medium', choices: [{ value: 'medium', label: 'Medium' }] },
    { id: 'mode', label: 'Mode', category: 'mode', value: 'safe', choices: [{ value: 'safe', label: 'Safe' }] },
    { id: 'future', label: 'Future', category: 'provider.future/category', value: { nested: true } }
  ],
  commands: []
};

assert.equal(configOptionPlacement('model'), 'model-picker');
assert.equal(configValueIsAdvertised(capabilities.configOptions[0], 'fixture-model'), true);
assert.equal(configValueIsAdvertised(capabilities.configOptions[0], 'other-model'), false);

const views = conversationControlViews(capabilities, { model: 'fixture-model' }, { effort: 'high' }, { mode: 'provider rejected value' });
assert.deepEqual(controlsForPlacement(views, 'model-picker').map((view) => view.option.id), ['model']);
assert.equal(views.find((view) => view.option.id === 'effort').pending, true);
assert.equal(views.find((view) => view.option.id === 'mode').reason, 'provider rejected value');
assert.equal(views.find((view) => view.option.id === 'future').placement, 'more-options');
assert.deepEqual(conversationControlViews(null), [], 'unsupported snapshots render no controls');

console.log('conversationControls.test.mjs passed');
