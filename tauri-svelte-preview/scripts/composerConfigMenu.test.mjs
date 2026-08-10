import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  emptyAgentConversationConfigState,
  hasAgentConversationConfig
} from '../src/lib/shell/conversation/conversationConfig.ts';

const menu = readFileSync(
  new URL('../src/lib/shell/components/conversation/ComposerConfigMenu.svelte', import.meta.url),
  'utf8'
);

assert.match(
  menu,
  /const hasAgentSettings = \$derived\(hasAgentConversationConfig\(state\)\)/,
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

console.log('composerConfigMenu.test.mjs passed');
