import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  emptyAgentConversationConfigState,
  hasAgentConversationConfig,
  snapshotAgentConversationConfig
} from '../src/lib/shell/conversation/conversationConfig.ts';

const menu = readFileSync(
  new URL('../src/lib/shell/components/conversation/ComposerConfigMenu.svelte', import.meta.url),
  'utf8'
);

assert.match(
  menu,
  /const hasAgentSettings = \$derived\(hasAgentConversationConfig\(configState\)\)/,
  'the menu uses the shared empty-state decision'
);
assert.match(
  menu,
  /\{#if !hasAgentSettings\}[\s\S]*Agent settings unavailable[\s\S]*\{:else\}[\s\S]*conversation-config-approval-policy[\s\S]*conversation-config-pill[\s\S]*\{\/if\}/,
  'an empty config renders one unavailable label instead of disabled menu fragments'
);

assert.equal(hasAgentConversationConfig(emptyAgentConversationConfigState()), false);
assert.equal(
  hasAgentConversationConfig({
    ...emptyAgentConversationConfigState(),
    availableModels: ['default', 'sonnet', 'haiku']
  }),
  true,
  'advertised models keep the real settings menus visible'
);
assert.equal(
  hasAgentConversationConfig({
    ...emptyAgentConversationConfigState(),
    availableApprovalPolicies: ['default', 'acceptEdits']
  }),
  true,
  'advertised permission modes keep the real settings menus visible'
);

const liveConfig = {
  ...emptyAgentConversationConfigState(),
  model: 'gpt-5.6-sol',
  availableModels: ['gpt-5.6-sol'],
  reasoningEffort: 'medium',
  availableEfforts: ['medium'],
  approvalPolicy: 'never',
  availableApprovalPolicies: ['never']
};
const openMenuConfig = snapshotAgentConversationConfig(liveConfig);
liveConfig.availableModels.push('streamed-later');
liveConfig.model = 'streamed-later';
assert.deepEqual(openMenuConfig.availableModels, ['gpt-5.6-sol']);
assert.equal(openMenuConfig.model, 'gpt-5.6-sol', 'an open config menu does not follow provider events');

console.log('composerConfigMenu.test.ts passed');
