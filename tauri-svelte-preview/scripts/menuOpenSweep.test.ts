import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = (path: string): string => readFileSync(new URL(`../src/${path}`, import.meta.url), 'utf8');

const branchMenu = source('lib/shell/components/git/BranchMenu.svelte');
assert.match(branchMenu, /afterFloatingSurfacePaint\(\(\) => void load\(\)\)/);
assert.doesNotMatch(branchMenu, /if \(next\)[\s\S]{0,180}void load\(\);/);

const usagePopover = source('lib/shell/usage/UsagePopover.svelte');
assert.match(usagePopover, /captureCurrentUsage\(\);[\s\S]{0,100}afterFloatingSurfacePaint/);
assert.match(usagePopover, /menuSnapshots\[providerName\]/);

const resourcePopover = source('lib/shell/resources/ResourcePopover.svelte');
assert.match(resourcePopover, /afterFloatingSurfacePaint\(\(\) => void refreshResourceSample\(\)\)/);

const configMenu = source('lib/shell/components/conversation/ComposerConfigMenu.svelte');
const compactConfigMenu = source('lib/shell/components/conversation/CompactComposerControlsMenu.svelte');
for (const menu of [configMenu, compactConfigMenu]) {
  assert.match(menu, /snapshotAgentConversationConfig\(configState\)/);
  assert.match(menu, /onOpenChange=\{snapshotOnOpen\}/);
}

const composer = source('lib/shell/components/conversation/ConversationComposer.svelte');
assert.match(composer, /commandSnapshot = snapshotConversationCommands\(commands\)/);
assert.match(composer, /slashMenuState\(inputDraft, commandSnapshot,/);

const pullRequestMenu = source('lib/shell/components/git/pr/PullRequestPanel.svelte');
assert.match(pullRequestMenu, /baseMenuChoices = baseChoices\.length > 0 \? \[\.\.\.baseChoices\]/);
assert.match(pullRequestMenu, /onOpenChange=\{snapshotBaseMenu\}/);

const sessionMenu = source('lib/shell/sessionLibrary/SessionContextMenu.svelte');
assert.doesNotMatch(sessionMenu, /ResizeObserver|addEventListener\('resize'/);
assert.match(sessionMenu, /getBoundingClientRect\(\)/);

console.log('menu open sweep tests passed');
