// The upstream package declares `types: index.d.ts`, but version 25.1.2 does
// not publish that file. Its entry point is side-effect-only: importing it
// registers Monaco's built-in tokenizers.
declare module "@codingame/monaco-vscode-standalone-languages";
