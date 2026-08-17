/**
 * How the editor panel and its status chip are wired together.
 *
 * Run it with:
 *   node --experimental-strip-types \
 *     src/lib/shell/components/editor/editorPanelLanguageServer.test.ts
 *
 * The words on the chip and the waiting rules are tested for real in
 * `languageServerStatus.test.ts`. What cannot be tested that way is the wiring
 * inside the Svelte components, so it is read out of their source here. These
 * are the promises that would be silently broken by an ordinary-looking
 * edit:
 *
 *  0. The project's switch lives with the centre pane's pill tabs, and the
 *     editor's file-tab row keeps only the open file's own controls. The panel
 *     remains the single owner of the status pipeline; the switch only reads
 *     what the panel publishes.
 *  1. Nothing is asked on a timer. The panel reads the status when a file
 *     opens and otherwise waits to be told — a poll would put the desktop app
 *     back under the load this whole change exists to remove.
 *  2. Pushed updates are only listened for when the desktop build says it
 *     sends them, so an older build is left exactly as it was.
 *  3. The editor's own inline-hint callback is replaced by the one that waits,
 *     and it is set AFTER the callback set is spread in — set before, the
 *     spread would put the original straight back.
 *  4. Nothing about the chip is painted from a colour written into the file.
 */
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const panelSource = readFileSync(path.join(here, '..', 'EditorPanel.svelte'), 'utf8');
const chipSource = readFileSync(path.join(here, '..', 'LanguageServerStatusChip.svelte'), 'utf8');
const intelligenceSource = readFileSync(
  path.join(here, '..', '..', 'editor', 'sourceIntelligence.ts'),
  'utf8'
);
const controlsSource = readFileSync(
  path.join(here, '..', 'LanguageIntelligenceControls.svelte'),
  'utf8'
);
const barSource = readFileSync(
  path.join(here, '..', '..', 'editor', 'languageIntelligenceBar.svelte.ts'),
  'utf8'
);
const shellPageSource = readFileSync(
  path.join(here, '..', '..', '..', '..', 'routes', 'next', '+page.svelte'),
  'utf8'
);
const pillsSource = readFileSync(path.join(here, '..', 'CenterCornerTabs.svelte'), 'utf8');

test('the switch says which language it is about', () => {
  assert.match(controlsSource, /languageShortLabel/);
  assert.match(
    controlsSource,
    /\{shortLanguage\}/,
    'the badge inside the control is what names the active language'
  );
  assert.ok(
    !controlsSource.includes('<LanguageServerStatusChip'),
    'the chip said the same thing again in words; colour and the badge carry it now'
  );
});

test('the centre pane\'s pill group mounts the controls', () => {
  assert.match(
    pillsSource,
    /import LanguageIntelligenceControls from '\.\/LanguageIntelligenceControls\.svelte'/
  );
  const group = pillsSource.slice(pillsSource.indexOf('<nav'), pillsSource.indexOf('</nav>'));
  assert.ok(group.length > 0, 'the pill group must still exist');
  assert.match(group, /<LanguageIntelligenceControls \/>/);
});

test('the shell has no strip along its top any more', () => {
  assert.ok(
    !shellPageSource.includes('<div class="top-bar">'),
    'the last thing in that strip was this switch, and it now travels with the pills'
  );
  assert.ok(
    !shellPageSource.includes('LanguageIntelligenceControls'),
    'the page must not mount the switch a second time'
  );
});

test('the switch is painted from its state, not from a selector nothing writes', () => {
  assert.match(
    controlsSource,
    /data-tone=\{tone\}/,
    'one attribute carries the three states, the way the rest of the shell does it'
  );
  assert.ok(
    !controlsSource.includes(':has(') && !controlsSource.includes('has-['),
    'no `:has()` selectors'
  );
});

test('position and colour are worked out separately', () => {
  // The knob follows what was ASKED FOR and the track follows what is TRUE, so
  // a switch turned on while the server is down must not be able to claim green.
  assert.match(controlsSource, /checked=\{languageIntelligenceBar\.fullMode\}/);
  // A button carries the browser's own padding, and on a 24px track that padding
  // parked the knob in the middle at rest — a switch that shows neither position.
  assert.match(
    controlsSource,
    /\[data-slot='switch'\]\) \{\s*padding:\s*0;/,
    'the switch must drop the browser button padding or its knob shows no position'
  );
  assert.match(
    controlsSource,
    /readLanguageServerState\(languageIntelligenceBar\.status\) === 'ready'/,
    'only a server that reports itself ready earns the running colour'
  );
  for (const [tone, token] of [
    ['running', '--switch-track-running'],
    ['waiting', '--switch-track-waiting'],
    ['off', '--switch-track-off']
  ] as const) {
    assert.match(
      controlsSource,
      new RegExp(`data-tone='${tone}'[\\s\\S]{0,200}${token}`),
      `the ${tone} state must paint the track from ${token}`
    );
  }
});

test('only the open file\'s own controls stay in the file-tab title row', () => {
  const titleRow = panelSource.slice(
    panelSource.indexOf('<div class="editor-header">'),
    panelSource.indexOf('<div class="editor-canvas">')
  );
  assert.ok(titleRow.length > 0, 'the editor title row must still exist');
  assert.match(titleRow, /aria-label="Open files"/);
  assert.match(
    titleRow,
    /aria-label="Markdown view"/,
    'the Source/Preview toggle is about the open file, so it stays with the tabs'
  );
  assert.ok(
    !panelSource.includes('<LanguageServerStatusChip'),
    'the chip is about the project and has moved to the top strip'
  );
  assert.ok(
    !panelSource.includes('class="intelligence"'),
    'the switch is about the project and has moved to the top strip'
  );
  assert.doesNotMatch(panelSource, /editor-status-bar|status-slot/);
});

test('the status pipeline has one owner', () => {
  assert.match(panelSource, /publishLanguageIntelligenceBar\(/);
  for (const call of ['invoke', 'listen(', 'read_source_lsp_status']) {
    assert.ok(
      !controlsSource.includes(call) && !barSource.includes(call),
      `the top-strip controls must not run the status pipeline a second time (${call})`
    );
  }
});

test('with no project the switch is still there, and inert', () => {
  // It used to remove itself, which left the pill group three pills wide with a
  // gap where the switch belongs. It is part of that group now, so it stays and
  // states the situation: off position, the neutral colour Off already wears,
  // and no way to flip it.
  assert.ok(
    !controlsSource.includes('{#if languageIntelligenceBar.hasProject}'),
    'the switch must not remove itself from the pill group'
  );
  assert.match(
    controlsSource,
    /disabled=\{languageIntelligenceBar\.busy \|\| !languageIntelligenceBar\.hasProject\}/,
    'with nothing to switch on, the switch cannot be flipped'
  );
  assert.match(
    controlsSource,
    /: 'No project is open, so there is no language server to switch on\.'/,
    'a disabled control must carry its reason'
  );
});

test('the chip is only rendered when there is something truthful to say', () => {
  assert.match(
    chipSource,
    /\{#if chip\}/,
    'with no status — a browser tab, or an older desktop build — the chip must not render at all'
  );
});

test('nothing is asked on a repeating timer', () => {
  for (const [name, source] of [
    ['EditorPanel.svelte', panelSource],
    ['LanguageServerStatusChip.svelte', chipSource],
    ['LanguageIntelligenceControls.svelte', controlsSource]
  ] as const) {
    assert.ok(!source.includes('setInterval'), `${name} must not poll the desktop app`);
  }
});

test('the panel reads the status when a file opens', () => {
  assert.match(panelSource, /readSourceLspStatusFromTauri/);
  assert.match(panelSource, /refreshLanguageServerStatus\(\)/);
});

test('pushed updates are only listened for when the build says it sends them', () => {
  assert.match(panelSource, /'source-lsp-status-changed'/);
  assert.match(panelSource, /hasBackendCapability\('lspStatusEvents'\)/);
});

test('a pushed update about another project or language is ignored', () => {
  assert.match(panelSource, /statusMessageIsAboutThisFile\(/);
});

test('held questions are released only by a ready update for the open file', () => {
  assert.match(intelligenceSource, /statusMessageIsAboutThisFile\(/);
  assert.match(
    intelligenceSource,
    /statusMessageIsAboutThisFile\(\s*event\.payload,\s*projectRoot,\s*activePreview\?\.language \?\? null\s*\)/
  );
});

test('the waiting inline-hint lookup replaces the original, not the other way round', () => {
  const spreadAt = panelSource.indexOf('{...sourceIntelligence.callbacks}');
  const overrideAt = panelSource.indexOf('onInlayHintLookup={');
  assert.ok(spreadAt >= 0, 'the panel still spreads the callback set onto the editor');
  assert.ok(overrideAt >= 0, 'the panel overrides the inline-hint lookup');
  assert.ok(
    overrideAt > spreadAt,
    'the override must come after the spread or the spread wins and hints are never held back'
  );
});

test('the second diagnostics read goes through the waiting room', () => {
  assert.match(panelSource, /languageServerGate/);
  assert.match(panelSource, /waitUntilReady\(\)/);
});

test('the chip and the controls are painted only from the shared colour names', () => {
  for (const [name, source] of [
    ['LanguageServerStatusChip.svelte', chipSource],
    ['LanguageIntelligenceControls.svelte', controlsSource]
  ] as const) {
    const style = source.slice(source.indexOf('<style>'));
    const writtenInColours = style.match(/#[0-9a-fA-F]{3,8}\b|\brgba?\(/g) ?? [];
    assert.deepEqual(
      writtenInColours,
      [],
      `colours belong in the shared token file, not in ${name}`
    );
  }
});

test('no text in the chip or the controls is smaller than 12 pixels', () => {
  for (const [name, source] of [
    ['LanguageServerStatusChip.svelte', chipSource],
    ['LanguageIntelligenceControls.svelte', controlsSource]
  ] as const) {
    const sizes = [...source.matchAll(/font-size:\s*(\d+)px/g)].map((match) => Number(match[1]));
    assert.ok(sizes.length > 0, `${name} should state its text size`);
    for (const size of sizes) {
      assert.ok(size >= 12, `text at ${size}px in ${name} is below the 12px floor`);
    }
  }
});
