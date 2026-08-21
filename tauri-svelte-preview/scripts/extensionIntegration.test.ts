import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path: string): string => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

const catalog = read('src/lib/shell/extensions/extensionCatalog.ts');
const runtime = read('src/lib/shell/extensions/extensionRuntime.ts');
const languages = read('src/lib/shell/extensions/languageContributions.ts');
const csharp = read('src/lib/shell/editor/csharpLanguageClient.ts');
const probeBridge = read('src/lib/shell/editor/extensionApiProbeBridge.ts');
const probeController = read('src/lib/shell/editor/extensionApiProbeController.ts');
const scm = read('src/lib/shell/extensions/rustGitScmProvider.ts');
const houston = JSON.parse(read('src/lib/shell/extensions/houston/houston.json'));

assert.match(catalog, /astro-build\.houston/);
assert.match(catalog, /svelte\.svelte-vscode-syntax/);
assert.match(catalog, /mac-command-bar\.extension-api-probe/);
assert.match(catalog, /mcb\.rust-git-scm/);
assert.match(runtime, /__mcbCuratedExtensionRegistrations/);
assert.match(runtime, /if \(registrations\.size > 0\) return/);
assert.match(runtime, /MCB_EXTENSION_API_PROBE_BROWSER_ENTRY/);
assert.match(runtime, /mcbExtensionApiProbeBrowserUrl/);
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
assert.doesNotMatch(csharp, /updateWorkspaceFolders/);
assert.match(csharp, /registerExtensionApiProbeBridgeCommands/);
assert.match(csharp, /enableExtHostWorker:\s*true/);
assert.match(probeBridge, /MCB_EXTENSION_API_PROBE_CONTEXT_COMMAND/);
assert.match(probeController, /setExtensionApiProbeWorkspace/);
assert.match(probeController, /terminalService\.createProbe/);
assert.match(probeController, /acquireExtensionApiProbeScmLease/);

assert.doesNotMatch(scm, /child_process|Command::new|invoke\(/);
assert.doesNotMatch(scm, /vscode\.scm\.createSourceControl/);
assert.doesNotMatch(scm, /from ['"]vscode['"]/);
assert.match(scm, /activeStatus = snapshot\.status/);
assert.match(scm, /rustGitScmProbeGroups/);
assert.match(scm, /activeProbeOwners\.clear\(\)/);

console.log('extensionIntegration.test.ts: all checks passed');
