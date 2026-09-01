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
const shellSource = await readFile(
  new URL('../src/routes/next/+page.svelte', import.meta.url),
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
  shellSource,
  /onStartWorkspaceCommand=\{\(request\) =>[\s\S]*?onStartStack\(\{/,
  'workspace actions must reuse the owned terminal-session starter'
);

console.log('workspace CodeLens tests passed');
