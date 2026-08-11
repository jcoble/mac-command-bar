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
  draftAfterSlashCommand,
  moveSlashMenuIndex,
  slashCommandQuery,
  slashMenuState
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

console.log('composerSlashCommands.test.mjs passed');
