import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import {
  dotnetWorkspaceCommandIds,
  dotnetWorkspaceLensTitles,
  dotnetWorkspaceSessionRequest
} from '../src/lib/workspaceCodeLens.ts';

assert.deepEqual(dotnetWorkspaceCommandIds, {
  build: 'mcb.source.dotnetBuild',
  test: 'mcb.source.dotnetTest'
});
assert.deepEqual(dotnetWorkspaceLensTitles, {
  build: 'Build workspace',
  test: 'Test workspace'
});

assert.deepEqual(dotnetWorkspaceSessionRequest('build', '/work/My App'), {
  id: 'dotnet-build:/work/My App',
  cwd: '/work/My App',
  script: 'dotnet build --nologo',
  title: 'Build My App'
});

assert.deepEqual(dotnetWorkspaceSessionRequest('test', '/work/My App/'), {
  id: 'dotnet-test:/work/My App/',
  cwd: '/work/My App/',
  script: 'dotnet test --nologo',
  title: 'Test My App'
});

// The path is data, not shell text. A hostile-looking folder cannot alter the
// fixed command line the Rust terminal session executes.
assert.equal(
  dotnetWorkspaceSessionRequest('build', '/work/Project; touch nope').script,
  'dotnet build --nologo'
);

assert.throws(
  () => dotnetWorkspaceSessionRequest('test', '   '),
  /needs a project folder/
);

const codeMirrorLensSource = await readFile(
  new URL('../src/lib/shell/editor/codeMirrorCodeLens.ts', import.meta.url),
  'utf8'
);
const editorPanelSource = await readFile(
  new URL('../src/lib/shell/components/EditorPanel.svelte', import.meta.url),
  'utf8'
);
const codeMirrorEditorSource = await readFile(
  new URL('../src/lib/CodeMirrorSourceEditor.svelte', import.meta.url),
  'utf8'
);

assert.match(
  codeMirrorLensSource,
  /preview\.language\.toLowerCase\(\) === 'csharp'[\s\S]*?dotnetWorkspaceLensTitles\.build/,
  'Build and Test lenses must appear only on C# documents'
);
assert.match(
  editorPanelSource,
  /dotnetWorkspaceSessionRequest\(action, root\)/,
  'the active editor workspace must provide the command working directory'
);
assert.match(
  editorPanelSource,
  /onDotnetBuildRequest=\{onStartWorkspaceCommand \? \(\) => runDotnetWorkspace\('build'\) : undefined\}/,
  'the editor must not expose an inert Build workspace action without a command owner'
);
assert.match(
  codeMirrorEditorSource,
  /onDotnetAction: onDotnetBuildRequest && onDotnetTestRequest[\s\S]*?: undefined/,
  'CodeMirror must omit workspace lenses when their command callbacks are unavailable'
);
assert.match(
  codeMirrorLensSource,
  /showCodeMirrorReferences = StateEffect\.define<SourceLookupRequest>\(\)/,
  'the editor context menu must be able to open the existing inline reference viewer'
);
assert.match(
  codeMirrorEditorSource,
  /function peekReferences\(\): void \{[\s\S]*?configureCodeLens\(\);[\s\S]*?showCodeMirrorReferences\.of\(request\)/,
  'Peek References must restore and reuse the CodeLens reference viewer'
);
assert.match(
  codeMirrorEditorSource,
  /key: 'Shift-F12',[\s\S]*?peekReferences\(\)/,
  'the non-LSP Find References shortcut must open the same inline reference viewer'
);
assert.match(
  codeMirrorLensSource,
  /refreshCodeMirrorCodeLens\.of\(\)/,
  'asynchronous reference results must dispatch a real effect so CodeMirror repaints the peek viewer'
);
assert.match(
  codeMirrorLensSource,
  /effect\.is\(showCodeMirrorReferences\)[\s\S]*?queueMicrotask\([\s\S]*?openReferences\(request\)/,
  'Peek References must wait until the current CodeMirror update finishes before dispatching its panel refresh'
);
assert.doesNotMatch(
  codeMirrorLensSource,
  /effect\.is\(showCodeMirrorReferences\)[\s\S]{0,500}?requestMeasure/,
  'Peek References must not dispatch from CodeMirror DOM measurement callbacks'
);
assert.match(
  codeMirrorLensSource,
  /new PeekWidget[\s\S]*?block: true/,
  'Peek References must render as a left-aligned editor block instead of after the symbol text'
);

console.log('workspace CodeLens tests passed');
