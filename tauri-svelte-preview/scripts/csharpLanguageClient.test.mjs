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

assert.match(client, /new vscode\.RelativePattern\(workspaceFolder, ["']\*\*\/\*\.cs["']\)/);
assert.match(client, /provideCodeLenses:[\s\S]*withinRoot\(document\.uri\)/);
assert.match(client, /handleDiagnostics:[\s\S]*withinRoot\(uri\)/);
assert.match(
  client,
  /NATIVE_CSHARP_CODE_LENS_REFRESH_METHOD\s*=\s*["']workspace\/codeLens\/refresh["']/
);
assert.match(client, /const pending = new Map/);
assert.match(client, /if \(inFlight\) return inFlight/);
assert.match(client, /MAX_WARM_CSHARP_ROOTS = 5/);
assert.match(client, /arguments: \[documentUri\]/);
assert.doesNotMatch(client, /0 references/);
assert.doesNotMatch(client, /updateWorkspaceFolders/);
assert.match(client, /ensureExtensionApiProbeWorkspaceRoot\(root\)/);
assert.match(client, /registerExtensionApiProbeBridgeCommands\(\)/);
assert.match(client, /enableExtHostWorker:\s*true/);
assert.match(client, /registerBrowserWorkspaceRoot\(workspaceContext\.activeRoot\)/);

assert.match(fileSystem, /readNativeCsharpFileFromTauri\(root, path\)/);
assert.match(fileSystem, /nativeCsharpPathIsWithinRoot/);
assert.match(fileSystem, /async writeFile\(resource: URI, content: Uint8Array/);
assert.match(fileSystem, /writeSourceToTauri\(current, decoder\.decode\(content\)\)/);

assert.match(editor, /language !== "csharp"/);
assert.match(panel, /nativeCsharpLanguageClient=\{nativeCsharpActive\}/);
assert.doesNotMatch(
  panel,
  /\{#key\s+nativeCsharpActive\}/,
  'native C# readiness must update the existing Monaco editor instead of remounting it'
);
assert.match(
  editor,
  /function reconcileLanguageProviderOwnership\(monaco: typeof Monaco, nativeMode: boolean\)/,
  'Monaco should reconcile custom/native provider ownership in one named transition'
);
assert.match(editor, /if \(reconciledNativeCsharpMode === nativeMode\) return;/);
const reconcileStart = editor.indexOf('function reconcileLanguageProviderOwnership');
const reconcileEnd = editor.indexOf('\n\n\tonMount', reconcileStart + 1);
const reconcileBody = editor.slice(reconcileStart, reconcileEnd);
assert.ok(reconcileStart >= 0 && reconcileEnd > reconcileStart);
assert.match(
  reconcileBody,
  /if \(nativeMode\) \{[\s\S]*codeLensProviderDisposable\?\.dispose\(\);[\s\S]*uninstallLazyTargetModelResolver\(\);[\s\S]*\} else \{[\s\S]*registerSourceCodeLensProvider\(monaco\);[\s\S]*installLazyTargetModelResolver\(monaco\);/,
  'native activation should release custom C# CodeLens/resolver ownership and deactivation should restore it'
);
assert.match(
  editor,
  /\$effect\(\(\) => \{[\s\S]*const nativeMode = nativeCsharpLanguageClient;[\s\S]*reconcileLanguageProviderOwnership\(monacoApi, nativeMode\);[\s\S]*\}\);/,
  'native mode should have one reactive in-place reconciliation path'
);
assert.equal(
  editor.match(/reconcileLanguageProviderOwnership\(monacoApi, nativeMode\);/g)?.length,
  1,
  'native mode should have exactly one reactive reconciliation call'
);
assert.doesNotMatch(reconcileBody, /editor\?\.dispose\(|model\.dispose\(|ownedModels/);
assert.match(panel, /runDotnetWorkspaceAction\('build', documentUri\)/);
assert.match(shellPanels, /warmNativeCsharpOnWorkspaceSelection\(root\)/);

console.log('csharpLanguageClient contract tests passed');
