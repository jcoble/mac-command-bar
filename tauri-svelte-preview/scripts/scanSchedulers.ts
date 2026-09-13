#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import ts from 'typescript';

const root = new URL('..', import.meta.url).pathname;
const srcRoot = join(root, 'src');
const includedExtensions = ['.ts', '.svelte'];
const excludedFileSuffixes = ['.test.ts', '.test.svelte', '.server.ts'];
const excludedPathSegments = new Set([
  '__generated__',
  'generated',
  'server'
]);

const schedulerPattern =
  /\b(setTimeout|setInterval|requestAnimationFrame|requestIdleCallback|cancelAnimationFrame|cancelIdleCallback|queueMicrotask|tick)\b/g;
const promiseChainPattern = /\.(then|catch|finally)\s*\(/g;
const promiseWrapperPattern = /\b(new\s+Promise|Promise\.(?:race|withResolvers))\s*(?:<[^>]+>)?\s*\(/g;
const anonymousAsyncIifePattern =
  /\b(void|[A-Za-z_$][\w$]*(?:\s*\?\?=|\s*=))\s*\(\s*async\s*\(\s*\)\s*=>/g;

const allowedHits = new Set([
  'src/lib/shell/components/UtilityStrip.svelte:setInterval:const refreshInterval = window.setInterval(() => {',
  'src/lib/shell/components/UtilityStrip.svelte:clearInterval:window.clearInterval(refreshInterval);',
  // Svelte tick is awaited only after mounting a user-requested lazy surface.
  "src/lib/shell/components/DockPanel.svelte:tick:import { tick } from 'svelte';",
  'src/lib/shell/components/DockPanel.svelte:tick:await tick();',
  // The one-shot WebSocket readiness bridge removes every listener when it
  // settles, and disposing the editor closes the socket so the wait rejects.
  'src/lib/shell/editor/csharpLanguageClient.ts:new Promise:await new Promise<void>((resolve, reject) => {',
  "src/lib/shell/panels/tasks/TasksPanel.svelte:tick:import { onDestroy, onMount, tick } from 'svelte';",
  'src/lib/shell/panels/tasks/TasksPanel.svelte:tick:await tick();',
  // These one-shot timers are both stopped by the AbortSignal-owning surface.
  'src/lib/shell/notionOAuth.ts:new Promise:await new Promise<void>((resolve, reject) => {',
  'src/lib/shell/notionOAuth.ts:setTimeout:const timeout = window.setTimeout(finish, CLAIM_INTERVAL_MS);',
  'src/lib/shell/panels/agents/WorkflowRuns.svelte:setTimeout:const timer = window.setTimeout(() => {'
]);

interface Hit {
  file: string;
  line: number;
  category:
    | 'scheduler'
    | 'promise-chain'
    | 'promise-wrapper'
    | 'anonymous-async-iife'
    | 'non-async-promise-implementation';
  term: string;
  text: string;
}

function isSourceFile(path: string): boolean {
  return includedExtensions.some((extension) => path.endsWith(extension));
}

function isProductionPath(path: string): boolean {
  if (excludedFileSuffixes.some((suffix) => path.endsWith(suffix))) return false;
  const parts = path.split(sep);
  return !parts.some((part) => excludedPathSegments.has(part));
}

function files(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stats = statSync(path);
    if (stats.isDirectory()) out.push(...files(path));
    else if (isSourceFile(path) && isProductionPath(relative(srcRoot, path))) out.push(path);
  }
  return out;
}

function scanFile(path: string): Hit[] {
  const rel = relative(root, path);
  const source = readFileSync(path, 'utf8');
  return [
    ...scanText(rel, source, 0),
    ...scanConcretePromiseImplementations(rel, source)
  ];
}

function scanText(rel: string, source: string, lineOffset: number): Hit[] {
  const lines = source.split(/\r?\n/);
  const hits: Hit[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const text = lines[index];
    schedulerPattern.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = schedulerPattern.exec(text)) !== null) {
      if (match[1] === 'tick' && !/\btick\s*\(/.test(text) && !/\bimport\s+\{[^}]*\btick\b/.test(text)) continue;
      hits.push({ file: rel, line: lineOffset + index + 1, category: 'scheduler', term: match[1], text: text.trim() });
    }
    promiseChainPattern.lastIndex = 0;
    while ((match = promiseChainPattern.exec(text)) !== null) {
      hits.push({ file: rel, line: lineOffset + index + 1, category: 'promise-chain', term: `promise.${match[1]}`, text: text.trim() });
    }
    promiseWrapperPattern.lastIndex = 0;
    while ((match = promiseWrapperPattern.exec(text)) !== null) {
      hits.push({ file: rel, line: lineOffset + index + 1, category: 'promise-wrapper', term: match[1].replace(/\s+/, ' '), text: text.trim() });
    }
    anonymousAsyncIifePattern.lastIndex = 0;
    while ((match = anonymousAsyncIifePattern.exec(text)) !== null) {
      hits.push({ file: rel, line: lineOffset + index + 1, category: 'anonymous-async-iife', term: match[1].replace(/\s+/, ' '), text: text.trim() });
    }
  }
  return hits;
}

function scriptBlocks(source: string): { text: string; lineOffset: number }[] {
  const blocks: { text: string; lineOffset: number }[] = [];
  const pattern = /<script\b[^>]*>([\s\S]*?)<\/script>/gi;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    blocks.push({
      text: match[1],
      lineOffset: source.slice(0, match.index).split(/\r?\n/).length - 1
    });
  }
  return blocks;
}

function parseBlocks(rel: string, source: string): { parsed: ts.SourceFile; lineOffset: number; text: string }[] {
  const blocks = rel.endsWith('.svelte') ? scriptBlocks(source) : [{ text: source, lineOffset: 0 }];
  return blocks.map((block) => ({
    ...block,
    parsed: ts.createSourceFile(`${rel}.ts`, block.text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  }));
}

function hasAsyncModifier(node: ts.Node): boolean {
  return Boolean(ts.canHaveModifiers(node) && ts.getModifiers(node)?.some((modifier) => modifier.kind === ts.SyntaxKind.AsyncKeyword));
}

function promiseReturn(type: ts.TypeNode | undefined): boolean {
  if (!type || ts.isUnionTypeNode(type)) return false;
  if (ts.isParenthesizedTypeNode(type)) return promiseReturn(type.type);
  return ts.isTypeReferenceNode(type) && type.typeName.getText() === 'Promise';
}

function implementationName(node: ts.Node): string {
  if ((ts.isFunctionDeclaration(node) || ts.isMethodDeclaration(node)) && node.name) return node.name.getText();
  if (ts.isArrowFunction(node) && ts.isVariableDeclaration(node.parent) && ts.isIdentifier(node.parent.name)) return node.parent.name.text;
  return 'anonymous';
}

function scanConcretePromiseImplementations(rel: string, source: string): Hit[] {
  const hits: Hit[] = [];
  for (const block of parseBlocks(rel, source)) {
    const visit = (node: ts.Node): void => {
      const concrete =
        (ts.isFunctionDeclaration(node) || ts.isMethodDeclaration(node) || ts.isArrowFunction(node))
        && Boolean(node.body)
        && !hasAsyncModifier(node)
        && promiseReturn(node.type);
      if (concrete) {
        const { line } = block.parsed.getLineAndCharacterOfPosition(node.getStart(block.parsed));
        const text = block.text.split(/\r?\n/)[line]?.trim() ?? '';
        hits.push({
          file: rel,
          line: block.lineOffset + line + 1,
          category: 'non-async-promise-implementation',
          term: implementationName(node),
          text
        });
      }
      ts.forEachChild(node, visit);
    };
    visit(block.parsed);
  }
  return hits;
}

const sourceFiles = files(srcRoot).sort();
const hits = sourceFiles.flatMap(scanFile).filter((hit) =>
  !allowedHits.has(`${hit.file}:${hit.term}:${hit.text}`)
);
const schedulerHits = hits.filter((hit) => hit.category === 'scheduler');
const promiseChainHits = hits.filter((hit) => hit.category === 'promise-chain');
const promiseWrapperHits = hits.filter((hit) => hit.category === 'promise-wrapper');
const anonymousAsyncIifeHits = hits.filter((hit) => hit.category === 'anonymous-async-iife');
const nonAsyncPromiseImplementationHits = hits.filter((hit) => hit.category === 'non-async-promise-implementation');

console.log(`production source files scanned: ${sourceFiles.length}`);
console.log(`scope: src/**/*.ts, src/**/*.svelte`);
console.log(`excluded: ${excludedFileSuffixes.join(', ')}; path segments ${[...excludedPathSegments].join(', ')}`);
console.log(`scheduler hits: ${schedulerHits.length}`);
console.log(`promise chain hits: ${promiseChainHits.length}`);
console.log(`promise wrapper hits: ${promiseWrapperHits.length}`);
console.log(`anonymous async IIFE hits: ${anonymousAsyncIifeHits.length}`);
console.log(`non-async Promise implementation hits: ${nonAsyncPromiseImplementationHits.length}`);
console.log(`total prohibited async/scheduler hits: ${hits.length}`);
for (const hit of hits) {
  console.log(`${hit.file}:${hit.line}: ${hit.category}:${hit.term}: ${hit.text}`);
}

if (hits.length > 0) process.exitCode = 1;
