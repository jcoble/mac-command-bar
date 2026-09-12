/**
 * How the editor modes are wired into the two components that carry them.
 *
 * Run it with:
 *   node --experimental-strip-types \
 *     src/lib/shell/components/editor/editorModesWiring.test.ts
 *
 * The rules themselves are tested in `languageIntelligenceMode.test.ts` and in
 * the Rust registry. What cannot be tested that way is the wiring inside Svelte
 * components, so it is read out of their source here. These are the promises an
 * ordinary-looking edit would break in silence:
 *
 *  1. Every path that can start a language server is behind the project's
 *     switch.
 *  2. The switch is the kit's `Switch`, not a hand-rolled control, and it lives
 *     in the editor status bar.
 *  3. A diff hunk opens the real file at that line.
 */
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const panelSource = readFileSync(path.join(here, '..', 'EditorPanel.svelte'), 'utf8');
const editorSessionSource = readFileSync(
  path.join(here, '..', '..', 'controllers', 'editorSessionController.svelte.ts'),
  'utf8'
);
const diffSource = readFileSync(path.join(here, '..', 'GitDiffView.svelte'), 'utf8');
const controlsSource = readFileSync(
  path.join(here, '..', 'LanguageIntelligenceControls.svelte'),
  'utf8'
);

test('warming a project is behind that project switch', () => {
  const warm = panelSource.slice(
    panelSource.indexOf('async function warmLanguageServer'),
    panelSource.indexOf('async function switchLanguageIntelligence')
  );
  assert.ok(warm.length > 0, 'warmLanguageServer must still exist');
  assert.match(
    warm,
    /activeServerEnabled !== true/,
    'warming must not run while the active language server is disabled'
  );
  assert.ok(
    warm.indexOf('activeServerEnabled') < warm.indexOf('warmSourceLspForRootFromTauri'),
    'the mode is checked BEFORE the desktop app is asked to warm anything'
  );
});

test('turning the switch on asks for the open file language server', () => {
  const on = panelSource.slice(
    panelSource.indexOf('async function switchLanguageIntelligence'),
    panelSource.indexOf('/** Keep the lookup service pointed')
  );
  assert.match(
    on,
    /queueLanguageIntelligenceOwner\(\s*root,\s*enabled,\s*enabled \? activeFileLanguage\(\) : null,\s*true\s*\)/,
    'the language of the file on screen is what the desktop app starts a server for'
  );
});

test('a language server that will not start reaches the switch tooltip', () => {
  const warm = panelSource.slice(
    panelSource.indexOf('async function warmLanguageServer'),
    panelSource.indexOf('async function switchLanguageIntelligence')
  );
  assert.match(
    warm,
    /catch \(error\)[\s\S]*?languageIntelligenceNote =/,
    'a failed warm must say why instead of being discarded'
  );
});

test('only one project is restored at launch', () => {
  assert.match(panelSource, /const root = activeLanguageRoot\(\)/);
  assert.match(panelSource, /generation !== ownerSelectionGeneration/);
  assert.ok(
    !panelSource.includes('Object.keys(languageIntelligenceChoices)'),
    'restoring every remembered project would wake servers nobody asked for'
  );
});

test('session restore leaves hidden editor files unhydrated', () => {
  assert.match(editorSessionSource, /restoreEditorFiles\(plan\.openFiles, plan\.activePath\)/);
  assert.doesNotMatch(editorSessionSource, /requestOpenFile/);

  const visibleEditor = panelSource.slice(
    panelSource.indexOf('Fetch and start the code editor'),
    panelSource.indexOf('Keep the selected tab')
  );
  assert.ok(
    visibleEditor.indexOf('!showing') < visibleEditor.indexOf('readFileIntoEditor'),
    'the active file must not be read until its editor is on screen'
  );
});

test('the switch is the kit component in the editor status bar', () => {
  assert.match(
    controlsSource,
    /import \{ Switch \} from '\$lib\/components\/ui\/switch\/index\.js'/
  );
  assert.match(controlsSource, /<Switch\b/);
  assert.match(controlsSource, /aria-label="Supercharged editor"/);
  assert.match(panelSource, /<div class="editor-status">[\s\S]*?<LanguageIntelligenceControls \/>/);
});

test('flipping the global switch uses the shared async language-server controller', () => {
  assert.match(controlsSource, /await setLanguageServersEnabled\(enabled\)/);
  assert.match(controlsSource, /onCheckedChange=\{\(checked\) => void chooseMode\(checked\)\}/);
});

test('file tabs scroll while the complete right-side control group stays pinned', () => {
  assert.match(panelSource, /<div[^>]*class="file-strip"[\s\S]*?<div class="editor-controls">/);
  assert.match(
    panelSource,
    /\.file-strip\s*\{[\s\S]*?flex:\s*1 1 auto;[\s\S]*?min-width:\s*0;[\s\S]*?overflow-x:\s*auto;[\s\S]*?overflow-y:\s*hidden;/
  );
  assert.match(panelSource, /\.file-chip\s*\{[\s\S]*?flex:\s*0 0 auto;/);
  assert.match(
    panelSource,
    /\.editor-controls\s*\{[\s\S]*?flex:\s*0 0 auto;[\s\S]*?min-width:\s*max-content;/
  );
});

test('a diff hunk opens the file in the editor at its line', () => {
  assert.match(diffSource, /import \{ requestOpenFile \} from '\$lib\/shell\/openFileBus'/);
  assert.match(diffSource, /onclick=\{\(\) => openAtLine\(hunk\.afterStart\)\}/);
  assert.match(diffSource, /requestOpenFile\(\{/);
  assert.match(
    diffSource,
    /onOpenLine=\{openAtLine\}/,
    'the real diff editor must offer the same jump as the plain fallback'
  );
});
