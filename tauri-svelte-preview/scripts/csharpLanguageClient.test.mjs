import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const client = await readFile(
  new URL('../src/lib/shell/editor/csharpLanguageClient.ts', import.meta.url),
  'utf8'
);
const fileSystem = await readFile(
  new URL('../src/lib/shell/editor/csharpFileSystem.ts', import.meta.url),
  'utf8'
);
const editor = await readFile(new URL('../src/lib/MonacoSourceEditor.svelte', import.meta.url), 'utf8');
const panel = await readFile(
  new URL('../src/lib/shell/components/EditorPanel.svelte', import.meta.url),
  'utf8'
);
const shellPanels = await readFile(
  new URL('../src/lib/shell/shellPanels.ts', import.meta.url),
  'utf8'
);

assert.match(client, /new vscode\.RelativePattern\(workspaceFolder, '\*\*\/\*\.cs'\)/);
assert.match(client, /provideCodeLenses:[\s\S]*withinRoot\(document\.uri\)/);
assert.match(client, /handleDiagnostics:[\s\S]*withinRoot\(uri\)/);
assert.match(client, /NATIVE_CSHARP_CODE_LENS_REFRESH_METHOD = 'workspace\/codeLens\/refresh'/);
assert.match(client, /const pending = new Map/);
assert.match(client, /if \(inFlight\) return inFlight/);
assert.match(client, /MAX_WARM_CSHARP_ROOTS = 5/);
assert.match(client, /arguments: \[documentUri\]/);
assert.doesNotMatch(client, /0 references/);

assert.match(fileSystem, /readNativeCsharpFileFromTauri\(root, path\)/);
assert.match(fileSystem, /nativeCsharpPathIsWithinRoot/);
assert.match(fileSystem, /read-only/);

assert.match(editor, /if \(!nativeCsharpLanguageClient\) \{/);
assert.match(editor, /if \(!nativeCsharpLanguageClient\) installLazyTargetModelResolver/);
assert.match(editor, /language !== "csharp"/);
assert.match(panel, /nativeCsharpLanguageClient=\{nativeCsharpActive\}/);
assert.match(panel, /runDotnetWorkspaceAction\('build', documentUri\)/);
assert.match(shellPanels, /warmNativeCsharpOnWorkspaceSelection\(root\)/);

console.log('csharpLanguageClient contract tests passed');
