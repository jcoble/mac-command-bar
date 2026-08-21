import assert from 'node:assert/strict';
import test from 'node:test';

import { codeMirrorLanguageForPath, loadCodeMirrorLanguage } from './codeMirrorLanguage.ts';

test('maps source and diff paths to the matching CodeMirror language', () => {
  assert.equal(codeMirrorLanguageForPath('/work/App.tsx'), 'tsx');
  assert.equal(codeMirrorLanguageForPath('/work/Program.cs'), 'csharp');
  assert.equal(codeMirrorLanguageForPath('/work/schema.proto'), 'protobuf');
  assert.equal(codeMirrorLanguageForPath('/work/Dockerfile'), 'dockerfile');
  assert.equal(codeMirrorLanguageForPath('/work/unknown.bin'), 'plain');
});

test('every configured CodeMirror language loader resolves', async () => {
  for (const language of [
    'javascript', 'jsx', 'typescript', 'tsx', 'json', 'css', 'html', 'svelte',
    'markdown', 'python', 'rust', 'sql', 'xml', 'yaml', 'java', 'cpp', 'csharp',
    'kotlin', 'dart', 'fsharp', 'go', 'shell', 'powershell', 'ruby', 'lua', 'swift',
    'toml', 'ini', 'scss', 'less', 'dockerfile', 'protobuf'
  ]) {
    assert.notDeepEqual(await loadCodeMirrorLanguage(language), [], `${language} must load`);
  }
});
