/**
 * Built-in declarative language contributions from VS Code.
 *
 * These packages register grammars, language metadata, snippets, and related
 * declarative contributions. They do not start language servers or create a
 * second VS Code service container. Keep every package on the same version as
 * @codingame/monaco-vscode-api.
 */
import '@codingame/monaco-vscode-bat-default-extension';
import '@codingame/monaco-vscode-cpp-default-extension';
import '@codingame/monaco-vscode-css-default-extension';
import '@codingame/monaco-vscode-docker-default-extension';
import '@codingame/monaco-vscode-fsharp-default-extension';
import '@codingame/monaco-vscode-go-default-extension';
import '@codingame/monaco-vscode-html-default-extension';
import '@codingame/monaco-vscode-ini-default-extension';
import '@codingame/monaco-vscode-java-default-extension';
import '@codingame/monaco-vscode-javascript-default-extension';
import '@codingame/monaco-vscode-json-default-extension';
import '@codingame/monaco-vscode-make-default-extension';
import '@codingame/monaco-vscode-markdown-basics-default-extension';
// The npm default extension is deliberately absent: it is the one bundled
// extension that contributes a VIEW ("NPM Scripts", into the Explorer
// container), and with the views service on and no Explorer registered yet,
// processing that contribution throws at every start. What it otherwise
// carries - grammar for .npmignore and .npmrc, npm task wiring - is nothing
// this app uses. When Explorer lands, adding it back is one import.
import '@codingame/monaco-vscode-php-default-extension';
import '@codingame/monaco-vscode-powershell-default-extension';
import '@codingame/monaco-vscode-python-default-extension';
import '@codingame/monaco-vscode-razor-default-extension';
import '@codingame/monaco-vscode-ruby-default-extension';
import '@codingame/monaco-vscode-rust-default-extension';
import '@codingame/monaco-vscode-shellscript-default-extension';
import '@codingame/monaco-vscode-sql-default-extension';
import '@codingame/monaco-vscode-swift-default-extension';
import '@codingame/monaco-vscode-typescript-basics-default-extension';
import '@codingame/monaco-vscode-xml-default-extension';
import '@codingame/monaco-vscode-yaml-default-extension';

export const BUILT_IN_LANGUAGE_CONTRIBUTIONS_REGISTERED = true;
