import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

const catalog = read('src/lib/shell/extensions/extensionCatalog.ts');
const runtime = read('src/lib/shell/extensions/extensionRuntime.ts');
const languages = read('src/lib/shell/extensions/languageContributions.ts');
const csharp = read('src/lib/shell/editor/csharpLanguageClient.ts');
const scm = read('src/lib/shell/extensions/rustGitScmProvider.ts');
const diffView = read('src/lib/shell/components/GitDiffView.svelte');
const nativeDiff = read('src/lib/shell/components/git/NativeGitDiffEditor.svelte');
const houston = JSON.parse(read('src/lib/shell/extensions/houston/houston.json'));

assert.match(catalog, /astro-build\.houston/);
assert.match(catalog, /svelte\.svelte-vscode-syntax/);
assert.match(catalog, /mcb\.rust-git-scm/);
assert.match(runtime, /const registrations = new Map/);
assert.match(runtime, /if \(registrations\.size > 0\) return/);
assert.match(runtime, /registerFileUrl\('\.\/themes\/houston\.json'/);
assert.match(runtime, /registerFileUrl\('\.\/syntaxes\/svelte\.tmLanguage\.json'/);
assert.match(runtime, /scopeName: 'source\.svelte'/);
assert.match(runtime, /language: 'svelte', path: '\.\/snippets\/svelte\.json'/);

for (const languagePackage of [
  'json',
  'xml',
  'html',
  'css',
  'javascript',
  'typescript-basics',
  'markdown-basics',
  'rust',
  'go',
  'python',
  'java',
  'cpp',
  'shellscript',
  'sql',
  'yaml'
]) {
  assert.match(
    languages,
    new RegExp(`@codingame/monaco-vscode-${languagePackage}-default-extension`),
    `${languagePackage} declarative language contribution must be registered`
  );
}
assert.match(csharp, /extensions\/languageContributions/);
assert.match(csharp, /monaco-vscode-csharp-default-extension/);

assert.equal(houston.name, 'Houston');
assert.equal(houston.type, 'dark');
assert.equal(houston.semanticHighlighting, true);
assert.ok(Object.keys(houston.colors).length > 200, 'Houston must include the full workbench palette');
assert.ok(houston.tokenColors.length > 200, 'Houston must include the full TextMate token palette');

assert.match(csharp, /theme\.id === "houston" \? "Houston" : "Default Dark Modern"/);
assert.match(csharp, /delete configuration\["workbench\.colorCustomizations"\]/);
assert.match(csharp, /delete configuration\["editor\.tokenColorCustomizations"\]/);
assert.match(csharp, /delete configuration\["editor\.semanticTokenColorCustomizations"\]/);
assert.match(csharp, /useExtensionTheme \? undefined : \{ \.\.\.theme\.monaco\.colors \}/);
assert.match(csharp, /registerCuratedExtensions\(\)/);
assert.match(csharp, /waitForCuratedExtensions\(\)/);

assert.doesNotMatch(scm, /child_process|Command::new|invoke\(/);
assert.match(scm, /vscode\.scm\.createSourceControl/);
assert.match(scm, /if \(!sourceControl \|\| activeRoot !== root\)/);

assert.match(diffView, /<NativeGitDiffEditor/);
assert.match(nativeDiff, /createDiffEditor/);
assert.match(nativeDiff, /Uri\.file\(absolutePath\(\)\)/);
assert.match(nativeDiff, /mcb-git=original/);
assert.match(nativeDiff, /mcb-git=modified/);

console.log('extensionIntegration.test.mjs: all checks passed');
