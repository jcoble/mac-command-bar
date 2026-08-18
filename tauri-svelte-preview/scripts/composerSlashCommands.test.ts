/**
 * composerSlashCommands.test.mjs — the slash menu model behind the
 * conversation composer.
 *
 * The composer opens a command menu when the message starts with a slash.
 * These checks pin when the menu is open, what it lists, how the keyboard
 * moves through it, and what the chosen row puts back into the draft.
 *
 * Run: node --experimental-strip-types scripts/composerSlashCommands.test.mjs
 */
import assert from 'node:assert/strict';
import {
  contextMeterState,
  draftAfterSlashCommand,
  formatContextTokens,
  moveSlashMenuIndex,
  remainingContextPercent,
  slashCommandQuery,
  slashMenuState,
  snapshotConversationCommands
} from '../src/lib/shell/conversation/composerSlashCommands.ts';
import { mergeConversationCommandCatalog } from '../src/lib/shell/conversation/conversationCommandCatalog.ts';

const catalog = mergeConversationCommandCatalog(
  [
    { id: 'review', label: 'Review changes' },
    { id: 'status', label: 'Session status' },
    { id: 'compact', label: 'Compact the transcript' }
  ],
  [],
  []
);

// ── When the menu is open ──────────────────────────────────────────────────
assert.equal(slashCommandQuery(''), null, 'an empty composer has no menu');
assert.equal(slashCommandQuery('hello'), null, 'plain text has no menu');
assert.equal(slashCommandQuery('tell me about /review'), null, 'a slash mid-sentence is not a command');
assert.equal(slashCommandQuery('/'), '', 'a bare slash opens the menu with no filter');
assert.equal(slashCommandQuery('  /rev'), 'rev', 'leading spaces still count as the start');
assert.equal(slashCommandQuery('/Review'), 'Review', 'the raw text is returned, not lowercased');
assert.equal(slashCommandQuery('/review please'), null, 'once arguments start the menu closes');
assert.equal(slashCommandQuery('/review\n'), null, 'a newline ends the command word');

const firstCharacterMenu = slashMenuState('/', catalog, { activeIndex: 0, dismissed: false });
assert.equal(firstCharacterMenu.open, true, 'the first slash character opens the menu');

// ── What the menu lists ────────────────────────────────────────────────────
const opened = slashMenuState('/', catalog, { activeIndex: 0, dismissed: false });
assert.equal(opened.open, true);
assert.deepEqual(opened.commands.map((command) => command.name), ['review', 'status', 'compact']);
assert.equal(opened.activeIndex, 0);
assert.equal(opened.emptyText, '');

const filtered = slashMenuState('/st', catalog, { activeIndex: 3, dismissed: false });
assert.deepEqual(filtered.commands.map((command) => command.name), ['status']);
assert.equal(filtered.activeIndex, 0, 'a stale index is clamped to the shortened list');

const noMatch = slashMenuState('/zzz', catalog, { activeIndex: 0, dismissed: false });
assert.equal(noMatch.open, true, 'the menu stays open so the user sees why nothing matched');
assert.deepEqual(noMatch.commands, []);
assert.equal(noMatch.emptyText, 'No commands match');

const noCommands = slashMenuState('/', [], { activeIndex: 0, dismissed: false });
assert.equal(noCommands.open, true);
assert.equal(noCommands.emptyText, 'No commands', 'an agent that advertises nothing says so');

const dismissed = slashMenuState('/rev', catalog, { activeIndex: 0, dismissed: true });
assert.equal(dismissed.open, false, 'Escape keeps the menu shut while the draft is unchanged');

// Name matches come before description-only matches.
const ranked = slashMenuState('/compact', catalog, { activeIndex: 0, dismissed: false });
assert.deepEqual(ranked.commands.map((command) => command.name), ['compact']);

const longProviderCatalog = mergeConversationCommandCatalog(
  Array.from({ length: 14 }, (_, index) => ({
    id: `custom-${index + 1}`,
    label: `Custom ${index + 1}`,
    description: `Description ${index + 1}`
  })),
  [],
  [{ id: 'assembly-built-in', name: 'built-in', label: 'Built-in', description: 'Built-in action' }]
);
assert.equal(longProviderCatalog.length, 15, 'all provider and built-in commands survive the merge');
assert.equal(longProviderCatalog[13].description, 'Description 14');
assert.equal(longProviderCatalog[14].source, 'assembly');

const openCatalog = snapshotConversationCommands(catalog);
catalog[0].label = 'Streamed replacement';
catalog.push({ ...catalog[0], id: 'late', name: 'late' });
assert.equal(openCatalog[0]?.label, 'Review changes');
assert.equal(openCatalog.length, 3, 'an open slash menu does not follow streamed command updates');

// ── Keyboard movement ──────────────────────────────────────────────────────
assert.equal(moveSlashMenuIndex(0, 3, 'ArrowDown'), 1);
assert.equal(moveSlashMenuIndex(2, 3, 'ArrowDown'), 0, 'the list wraps to the top');
assert.equal(moveSlashMenuIndex(0, 3, 'ArrowUp'), 2, 'the list wraps to the bottom');
assert.equal(moveSlashMenuIndex(1, 0, 'ArrowDown'), 0, 'an empty list stays at zero');
assert.equal(moveSlashMenuIndex(1, 3, 'Enter'), 1, 'other keys leave the index alone');

// ── Choosing a row ─────────────────────────────────────────────────────────
const review = catalog.find((command) => command.name === 'review');
assert.equal(draftAfterSlashCommand('/rev', review), '/review ');
assert.equal(draftAfterSlashCommand('   /rev', review), '/review ', 'the rewritten draft drops stray leading space');
assert.equal(draftAfterSlashCommand('anything', review), '/review ');

// ── Context math ─────────────────────────────────────────────────────────
assert.equal(remainingContextPercent(120, 400), 70);
assert.equal(remainingContextPercent(401, 400), 0, 'usage over the window clamps to zero');
assert.equal(remainingContextPercent(-1, 400), 100, 'negative usage clamps to the full window');
assert.equal(remainingContextPercent(1, 0), null, 'an absent context window hides the indicator');
assert.equal(remainingContextPercent(null, 400), null);

// ── What the composer shows for context ──────────────────────────────────
assert.deepEqual(
  contextMeterState(50_000, 200_000),
  { kind: 'percent', remaining: 75 },
  'a known context window still reports remaining percent'
);
assert.deepEqual(
  contextMeterState(398_832, null),
  { kind: 'absolute', usedTokens: 398_832 },
  'a missing context window falls back to the token count'
);
assert.deepEqual(
  contextMeterState(334_581_117, 237_500),
  { kind: 'absolute', usedTokens: 334_581_117 },
  'usage larger than the window is treated as untrustworthy'
);
assert.equal(contextMeterState(null, null), null, 'nothing usable renders nothing');

// ── Token labels ─────────────────────────────────────────────────────────
assert.equal(formatContextTokens(940), '940');
assert.equal(formatContextTokens(9_400), '9.4k');
assert.equal(formatContextTokens(398_832), '399k');
assert.equal(formatContextTokens(1_250_000), '1.2m');

console.log('composerSlashCommands.test.ts passed');
