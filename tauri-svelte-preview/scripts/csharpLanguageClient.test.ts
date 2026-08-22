import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const client = await readFile(
  new URL('../src/lib/shell/editor/csharpLanguageClient.ts', import.meta.url),
  'utf8'
);
const editor = await readFile(
  new URL('../src/lib/CodeMirrorSourceEditor.svelte', import.meta.url),
  'utf8'
);
const panel = await readFile(
  new URL('../src/lib/shell/components/EditorPanel.svelte', import.meta.url),
  'utf8'
);

assert.match(client, /from ['"]@codemirror\/lsp-client['"]/);
assert.match(client, /new LSPClient\([\s\S]*serverCompletion\(\{ override: true \}\)[\s\S]*hoverTooltips\(\)[\s\S]*serverDiagnostics\(\)/);
assert.match(client, /new WebSocket\(endpoint\.wsUrl\)/);
assert.match(client, /ensureNativeCsharpLanguageClientFromTauri\(requestedRoot\)/);
assert.match(client, /markNativeCsharpLanguageClientReadyFromTauri\(requestedRoot\)/);
assert.match(client, /activeSession\?\.dispose\(\)/);
assert.match(client, /client\?\.disconnect\(\)/);
assert.match(client, /socket\?\.close\(\)/);
assert.doesNotMatch(client, /monaco|LanguageClientWrapper|fallback/i);
assert.doesNotMatch(client, /(?:document|result)(?:Cache|Map|State)/i);

assert.match(editor, /languageServerRoot/);
assert.match(editor, /connectCodeMirrorCsharpClient\(root\)/);
assert.match(editor, /intelligence\.reconfigure\(extension\)/);
assert.match(editor, /function clearLspSupport\(disposeClient = true\)/);
assert.match(
  editor,
  /function showFile\(\)[\s\S]*clearLspSupport\(false\);[\s\S]*rememberCurrentView\(\)/,
  'file exit should remove the prior document plugin before retaining editor state'
);
assert.match(
  editor,
  /function clearLspSupport\(disposeClient = true\)[\s\S]*intelligence\.reconfigure\(callbackIntelligence\)/,
  'file switches should reconfigure the document overlay without disconnecting the root client'
);
assert.match(
  editor,
  /\.catch\(\(\) => \{[\s\S]*generation !== lspGeneration \|\| requestedLspKey !== key[\s\S]*intelligence\.reconfigure\(callbackIntelligence\)[\s\S]*setDiagnostics\(view\.state, diagnosticsFor\(view\.state\)\)/,
  'a current failed official client load should restore callback intelligence and diagnostics'
);
assert.match(editor, /if \(visible\)[\s\S]*loadVisibleLspSupport\(\)[\s\S]*else \{[\s\S]*clearLspSupport\(\)/);
assert.match(panel, /languageServerRoot=\{fullMode \? editorState\.projectRoot : null\}/);

console.log('official CodeMirror C# language client contract tests passed');
