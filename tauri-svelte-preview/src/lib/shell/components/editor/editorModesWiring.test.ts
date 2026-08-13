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
 *     switch — warming, and attaching the C# document.
 *  2. Turning the switch off closes the language client before asking the
 *     desktop app to stop the process, so the server gets its goodbye.
 *  3. The switch is the kit's `Switch`, not a hand-rolled control.
 *  4. A diff hunk opens the real file at that line.
 */
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const panelSource = readFileSync(path.join(here, '..', 'EditorPanel.svelte'), 'utf8');
const diffSource = readFileSync(path.join(here, '..', 'GitDiffView.svelte'), 'utf8');
const nativeDiffSource = readFileSync(
  path.join(here, '..', 'git', 'NativeGitDiffEditor.svelte'),
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
    /languageIntelligenceOn\(languageIntelligenceChoices, projectRoot\)/,
    'warming must not run for a project in read mode'
  );
  assert.ok(
    warm.indexOf('languageIntelligenceOn') < warm.indexOf('warmSourceLspForRootFromTauri'),
    'the mode is checked BEFORE the desktop app is asked to warm anything'
  );
});

test('attaching the C# document is behind the same switch', () => {
  const attach = panelSource.slice(
    panelSource.indexOf('async function ensureNativeCsharpForActiveFile'),
    panelSource.indexOf('function applySavedModeForProject')
  );
  assert.ok(attach.length > 0, 'ensureNativeCsharpForActiveFile must still exist');
  assert.match(attach, /languageIntelligenceOn\(languageIntelligenceChoices, root\)/);
  assert.ok(
    attach.indexOf('languageIntelligenceOn') < attach.indexOf('ensureNativeCsharpDocument'),
    'Roslyn must not be reached for a project in read mode'
  );
});

test('turning the switch off says goodbye before stopping the process', () => {
  const off = panelSource.slice(
    panelSource.indexOf('async function switchLanguageIntelligence'),
    panelSource.indexOf('/** Keep the lookup service pointed')
  );
  assert.ok(off.length > 0, 'switchLanguageIntelligence must still exist');
  assert.ok(
    off.indexOf('stopNativeCsharpLanguageClient') <
      off.indexOf('setWorkspaceLanguageIntelligenceFromTauri'),
    'the language client is closed first; the desktop app stopping the process is the backstop'
  );
});

test('only one project is restored at launch', () => {
  assert.match(panelSource, /launchRestoreFor\(/);
  assert.ok(
    !panelSource.includes('Object.keys(languageIntelligenceChoices)'),
    'restoring every remembered project would wake servers nobody asked for'
  );
});

test('the switch is the kit component', () => {
  assert.match(panelSource, /import \{ Switch \} from '\$lib\/components\/ui\/switch\/index\.js'/);
  assert.match(panelSource, /<Switch\b/);
  assert.match(panelSource, /aria-label="Language intelligence"/);
});

test('file tabs scroll while the complete right-side control group stays pinned', () => {
  assert.match(panelSource, /<div class="file-strip"[\s\S]*?<div class="editor-controls">/);
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

test('the diff editor opens a line on a double click, never a single one', () => {
  assert.match(nativeDiffSource, /event\.event\.detail < 2/);
  assert.match(nativeDiffSource, /openLineListener\?\.dispose\(\)/);
});
