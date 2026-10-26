#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const root = new URL('..', import.meta.url).pathname;
const repoRoot = join(root, '..');
const srcRoot = join(root, 'src');
const srcPrefix = relative(repoRoot, srcRoot).split(sep).join('/');
const includedExtensions = ['.ts', '.svelte', '.css'];
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
const cssHasPattern = /:has\s*\(/gi;
const tailwindHasPattern = /\b(?:(?:group|peer)-)?has-(?:\[[^\]\r\n]+\]|[A-Za-z][\w-]*)(?:\/[\w-]+)?:/g;
// Animations end (AGENTS.md): an endless loop repaints every frame, so CSS, inline styles and the
// Tailwind loop classes may not repeat forever. A value ends at a quote, except a quoted keyframe name, and
// may hold Svelte {expressions}.
const endlessAnimationPattern = /\banimation(?:-iteration-count)?(?:\|important)?\s*(?::|=\s*\{?\s*["'`]?)(?:[^;{}"'`<>]|\{[^{}"'`<>]*\}|(["'])[\w-]*\1)*?(?<![\w-])infinite(?![\w-])|\banimate-(?:spin|pulse|ping|bounce)\b/gi;

const allowedHits = new Set([
  // The page clock owns the app's one timer for labels and one-shot waits; it is armed only while a
  // watcher or deadline needs it, and every watcher and deadline is cancelled by its owner.
  'src/lib/pageClock.ts:setTimeout:let timer: ReturnType<typeof setTimeout> | undefined;',
  'src/lib/pageClock.ts:setTimeout:if (atMs !== Infinity) timer = setTimeout(fire, Math.max(0, atMs - Date.now()));',
  // The two waits nothing else can signal: the next frame, for the WebKit width workarounds in
  // ShellFrame and layout/frame.ts, and Svelte's render, for ConversationTimeline's chat placement.
  "src/lib/pageClock.ts:tick:import { tick } from 'svelte';",
  'src/lib/pageClock.ts:requestAnimationFrame:const frame = requestAnimationFrame(fn);',
  'src/lib/pageClock.ts:cancelAnimationFrame:return () => cancelAnimationFrame(frame);',
  'src/lib/pageClock.ts:tick:await tick();',
  // The one Promise constructor: turns a callback or event into an await; each owner settles it on abort.
  'src/lib/deferred.ts:new Promise:const promise = new Promise<T>((accept, decline) => {',
]);

interface Hit {
  file: string;
  line: number;
  category:
    | 'scheduler'
    | 'promise-chain'
    | 'promise-wrapper'
    | 'anonymous-async-iife'
    | 'non-async-promise-implementation'
    | 'css-has-selector'
    | 'tailwind-has-variant'
    | 'endless-animation';
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
  return scanSource(rel, source);
}

function scanStagedFiles(): { count: number; hits: Hit[] } {
  const paths = execFileSync(
    'git',
    ['diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z', '--', srcPrefix],
    { cwd: repoRoot, encoding: 'utf8' }
  ).split('\0').filter(Boolean).filter((path) => {
    const rel = path.slice(`${srcPrefix}/`.length);
    return isSourceFile(path) && isProductionPath(rel);
  }).sort();
  return {
    count: paths.length,
    hits: paths.flatMap((path) => {
      const rel = relative(root, join(repoRoot, path));
      const source = execFileSync('git', ['show', `:${path}`], { cwd: repoRoot, encoding: 'utf8' });
      return scanSource(rel, source);
    })
  };
}

export function scanSource(rel: string, source: string): Hit[] {
  const cssHits = scanCssText(rel, source);
  if (rel.endsWith('.css')) return cssHits;
  return [
    ...scanText(rel, source, 0),
    ...scanConcretePromiseImplementations(rel, source),
    ...cssHits
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

function scanCssText(rel: string, source: string): Hit[] {
  const withoutComments = source.replace(/<!--[\s\S]*?-->|\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\r\n]/g, ' '));
  const lines = withoutComments.split(/\r?\n/);
  const hits: Hit[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const text = lines[index];
    for (const [pattern, category, term] of [
      [cssHasPattern, 'css-has-selector', ':has()'],
      [tailwindHasPattern, 'tailwind-has-variant', 'has-*']
    ] as const) {
      pattern.lastIndex = 0;
      if (pattern.test(text)) hits.push({ file: rel, line: index + 1, category, term, text: text.trim() });
    }
  }
  // A declaration can wrap across lines, so the loop check reads the whole text and reports the line it starts on.
  for (const match of withoutComments.matchAll(endlessAnimationPattern)) {
    const line = withoutComments.slice(0, match.index).split(/\r?\n/).length;
    hits.push({ file: rel, line, category: 'endless-animation', term: 'infinite', text: lines[line - 1].trim() });
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

function run(): void {
  const staged = process.argv.includes('--staged');
  const sourceFiles = staged ? [] : files(srcRoot).sort();
  const scan = staged ? scanStagedFiles() : { count: sourceFiles.length, hits: sourceFiles.flatMap(scanFile) };
  const hits = scan.hits.filter((hit) =>
    hit.category === 'endless-animation'
    || hit.category.startsWith('css-')
    || hit.category === 'tailwind-has-variant'
    || !allowedHits.has(`${hit.file}:${hit.term}:${hit.text}`)
  );
  const count = (category: Hit['category']): number => hits.filter((hit) => hit.category === category).length;

  console.log(`production source files scanned: ${scan.count}`);
  console.log(`mode: ${staged ? 'staged index' : 'working tree'}`);
  console.log(`scope: src/**/*.ts, src/**/*.svelte, src/**/*.css`);
  console.log(`excluded: ${excludedFileSuffixes.join(', ')}; path segments ${[...excludedPathSegments].join(', ')}`);
  console.log(`scheduler hits: ${count('scheduler')}`);
  console.log(`promise chain hits: ${count('promise-chain')}`);
  console.log(`promise wrapper hits: ${count('promise-wrapper')}`);
  console.log(`anonymous async IIFE hits: ${count('anonymous-async-iife')}`);
  console.log(`non-async Promise implementation hits: ${count('non-async-promise-implementation')}`);
  console.log(`CSS :has() hits: ${count('css-has-selector')}`);
  console.log(`Tailwind has variant hits: ${count('tailwind-has-variant')}`);
  console.log(`endless animation hits: ${count('endless-animation')}`);
  console.log(`total prohibited hits: ${hits.length}`);
  for (const hit of hits) {
    console.log(`${hit.file}:${hit.line}: ${hit.category}:${hit.term}: ${hit.text}`);
  }

  if (hits.length > 0) process.exitCode = 1;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) run();
