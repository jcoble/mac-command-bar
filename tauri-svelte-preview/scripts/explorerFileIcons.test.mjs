/**
 * explorerFileIcons.test.mjs — the explorer's file icons, its folder counts, and
 * the editor's second-chance language lookup.
 *
 * All three are plain modules with no Svelte in them, which is the point: the
 * rules that decide what a row looks like and which colouring a file opens with
 * are checkable here, and the components are left with nothing but drawing.
 */
import assert from 'node:assert/strict';

import {
  DEFAULT_FILE_ICON,
  fileIconForName
} from '../src/lib/shell/components/explorer/fileIcons.ts';
import {
  folderFileCountLabel,
  folderFileCounts,
  folderFileCountTitle
} from '../src/lib/shell/components/explorer/folderFileCounts.ts';
import {
  editorLanguageForPath,
  upgradeUnknownLanguage
} from '../src/lib/shell/components/editor/editorLanguage.ts';

/** Minimal stand-in for one file the backend scan returned. */
function record(relativePath) {
  return {
    path: `/repo/${relativePath}`,
    relativePath,
    fileName: relativePath.split('/').at(-1),
    language: 'plain',
    byteCount: 120
  };
}

// ── File icons ───────────────────────────────────────────────────────────────

// Every language the shell shows a lot of gets its own colour, so a folder of
// mixed sources can be read without looking at any of the names.
assert.equal(fileIconForName('ExplorerPanel.svelte').tone, 'svelte');
assert.equal(fileIconForName('explorerTree.ts').tone, 'typescript');
assert.equal(fileIconForName('build.mjs').tone, 'javascript');
assert.equal(fileIconForName('main.rs').tone, 'rust');
assert.equal(fileIconForName('Program.cs').tone, 'csharp');
assert.equal(fileIconForName('tokens.css').tone, 'style');
assert.equal(fileIconForName('index.html').tone, 'markup');
assert.equal(fileIconForName('schema.sql').tone, 'database');
assert.equal(fileIconForName('deploy.sh').tone, 'shell');
assert.equal(fileIconForName('notes.md').tone, 'document');

// A Svelte 5 rune module is TypeScript that merely has "svelte" in its name.
// Reading the LAST extension is what keeps it out of the Svelte bucket.
assert.equal(fileIconForName('explorerStore.svelte.ts').tone, 'typescript');
assert.equal(fileIconForName('ExplorerPanel.svelte').kind, 'flame');

// Whole names win over extensions: a lock file is a lock file, not settings.
assert.equal(fileIconForName('pnpm-lock.yaml').kind, 'lock');
assert.equal(fileIconForName('config.yaml').kind, 'sliders');
assert.equal(fileIconForName('Cargo.toml').tone, 'rust');
assert.equal(fileIconForName('tsconfig.json').kind, 'braces');

// Case does not matter, and a name that starts with a dot is a name, not an
// extension — `.gitignore` must not be read as a file of type "gitignore".
assert.equal(fileIconForName('MAIN.RS').tone, 'rust');
assert.equal(fileIconForName('.gitignore').kind, 'sliders');

// Anything unrecognised still draws something, and says only "File".
assert.deepEqual(fileIconForName('mystery.qqq'), DEFAULT_FILE_ICON);
assert.deepEqual(fileIconForName(''), DEFAULT_FILE_ICON);
assert.equal(DEFAULT_FILE_ICON.label, 'File');

// Every label reads as words a non-programmer could follow.
for (const name of ['a.ts', 'a.rs', 'a.sql', 'pnpm-lock.yaml', 'Dockerfile']) {
  const { label } = fileIconForName(name);
  assert.ok(label.length > 0, `${name} has no label`);
  assert.equal(label, label.trim());
}

// ── Folder counts ────────────────────────────────────────────────────────────

const project = [
  record('README.md'),
  record('src/app.ts'),
  record('src/lib/one.ts'),
  record('src/lib/two.ts'),
  record('src/lib/deep/three.ts')
];
const counts = folderFileCounts(project);

// The whole point: a folder counts everything underneath it, not just what sits
// directly inside. `src` holds one file of its own and three more below it.
assert.equal(counts.get('folder:src'), 4);
assert.equal(counts.get('folder:src/lib'), 3);
assert.equal(counts.get('folder:src/lib/deep'), 1);

// A file at the top level belongs to no folder, so it adds to no count.
assert.equal(counts.has('folder:README.md'), false);
assert.equal([...counts.keys()].length, 3);

// An empty project counts nothing, and an unknown folder reads as zero rather
// than as a missing badge.
assert.equal(folderFileCounts([]).size, 0);
assert.equal(folderFileCountLabel(counts, 'folder:nowhere'), '0');
assert.equal(folderFileCountTitle(counts, 'folder:nowhere'), 'No files in this folder');

// The tooltip says what the number means, in a sentence, and gets its singular
// and plural right.
assert.equal(
  folderFileCountTitle(counts, 'folder:src/lib/deep'),
  '1 file in this folder, including everything in its subfolders'
);
assert.equal(
  folderFileCountTitle(counts, 'folder:src'),
  '4 files in this folder, including everything in its subfolders'
);

// Big numbers are grouped so a large folder does not read as one long digit run.
const wide = folderFileCounts(
  Array.from({ length: 1200 }, (_, index) => record(`src/file-${index}.ts`))
);
assert.equal(folderFileCountLabel(wide, 'folder:src'), (1200).toLocaleString());

// ── The editor's second-chance language lookup ───────────────────────────────

// The one that gave the gap away: SQL files opened as flat grey text.
assert.equal(editorLanguageForPath('/repo/db/schema.sql'), 'sql');

// Others the editor can already colour but the old mapping did not name.
assert.equal(editorLanguageForPath('/repo/style.scss'), 'scss');
assert.equal(editorLanguageForPath('/repo/run.bash'), 'shell');
assert.equal(editorLanguageForPath('/repo/tool.py'), 'python');
assert.equal(editorLanguageForPath('/repo/server.go'), 'go');
assert.equal(editorLanguageForPath('/repo/App.java'), 'java');
assert.equal(editorLanguageForPath('/repo/Api.csproj'), 'xml');
assert.equal(editorLanguageForPath('/repo/build/Dockerfile'), 'dockerfile');
assert.equal(editorLanguageForPath('/repo/legacy.cjs'), 'javascript');

// These agree with what the backend scan calls the same file (`detect_language`
// in `src-tauri/src/main.rs`), so the tree and the open file never disagree
// about what a file is.
assert.equal(editorLanguageForPath('/repo/Api.props'), 'xml');
assert.equal(editorLanguageForPath('/repo/main.hxx'), 'cpp');
assert.equal(editorLanguageForPath('/repo/page.astro'), 'html');
assert.equal(editorLanguageForPath('/repo/local.env'), 'ini');

// Unknown stays unknown — claiming a language the editor cannot colour would
// label the file and still leave it grey.
assert.equal(editorLanguageForPath('/repo/mystery.qqq'), 'plain');
assert.equal(editorLanguageForPath(''), 'plain');

// A file the usual mapping already worked out is never second-guessed, so no
// file that opens correctly today can change.
assert.equal(upgradeUnknownLanguage('/repo/a.ts', 'typescript'), 'typescript');
assert.equal(upgradeUnknownLanguage('/repo/App.svelte', 'svelte'), 'svelte');
assert.equal(upgradeUnknownLanguage('/repo/schema.sql', 'plain'), 'sql');
assert.equal(upgradeUnknownLanguage('/repo/mystery.qqq', 'plain'), 'plain');

console.log('explorerFileIcons.test.mjs: all assertions passed');
