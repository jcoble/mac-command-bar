/**
 * nextTokens.test.mjs — checks the /next color file against the files that
 * read it.
 *
 * The /next shell is painted entirely from custom property names
 * (`--color-text`, `--shadow-lg`, ...). If the /next color file misses one,
 * that one value silently falls back to the old shell's palette and the
 * screen comes out two-tone. These checks read the actual files, collect
 * every name they use, and prove the /next file covers the ones it is
 * responsible for.
 *
 * Two more rules about that stylesheet are checked at the bottom of the file:
 * that the old shell cannot reach it (it would restyle the whole old page), and
 * that no /next component styles itself on a `data-…` attribute the component
 * library never actually writes (such a rule is silently dead CSS).
 *
 * Run: node --experimental-strip-types scripts/nextTokens.test.mjs
 */
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const read = (relativePath) => readFileSync(resolve(projectRoot, relativePath), 'utf8');

const NEXT_TOKENS_PATH = 'src/lib/shell/styles/nextTokens.css';
const SHARED_TOKENS_PATH = 'src/lib/styles/tokens.css';
const SETTINGS_HOST_PATH = 'src/lib/shell/components/SettingsHost.svelte';
const NEXT_STYLESHEET_PATH = 'src/lib/shell/styles/next.css';

/**
 * Every file whose styling ends up inside the settings dialog.
 *
 * `next.css` is the big one: it defines the library components' color names
 * (`--background`, `--primary`, ...) FROM the token names below, so every
 * token it reads has to exist. The two shell files read a couple of tokens
 * directly for the parts the library has no name for.
 */
const CONSUMER_PATHS = [
  NEXT_STYLESHEET_PATH,
  'src/lib/shell/components/SettingsDialog.svelte',
  'src/lib/shell/components/SettingsHost.svelte',
  'src/lib/shell/components/ShellOverlays.svelte'
];

const nextTokensSource = read(NEXT_TOKENS_PATH);
const sharedTokensSource = read(SHARED_TOKENS_PATH);
const NEXT_ONLY_SEMANTIC_TOKENS = new Set([
  '--color-selected',
  '--color-selected-border',
  '--color-hover',
  '--color-focus-solid',
  '--color-disabled-text',
  '--color-status-idle'
]);

// ── Tiny CSS reader ────────────────────────────────────────────────────────
// Enough to walk `selector { --name: value; }` blocks, which is all either
// file contains.

const stripComments = (css) => css.replace(/\/\*[\s\S]*?\*\//g, '');

/** @returns {{selector: string, declarations: Map<string, string>}[]} */
function readRules(css, label) {
  const source = stripComments(css);
  assert.equal(
    (source.match(/\{/g) ?? []).length,
    (source.match(/\}/g) ?? []).length,
    `${label}: every { must have a matching }`
  );

  const rules = [];
  const blockPattern = /([^{}]+)\{([^{}]*)\}/g;
  let match;
  while ((match = blockPattern.exec(source)) !== null) {
    const selector = match[1].trim().replace(/\s+/g, ' ');
    const declarations = new Map();
    for (const piece of match[2].split(';')) {
      const text = piece.trim();
      if (!text) continue;
      const separator = text.indexOf(':');
      assert.ok(
        separator > 0,
        `${label}: "${text}" is not a name: value pair`
      );
      const name = text.slice(0, separator).trim();
      const value = text.slice(separator + 1).trim();
      assert.ok(
        name.startsWith('--'),
        `${label}: "${name}" is not a custom property name`
      );
      assert.ok(value.length > 0, `${label}: "${name}" has no value`);
      declarations.set(name, value);
    }
    rules.push({ selector, declarations });
  }

  assert.ok(rules.length > 0, `${label}: no rules found`);
  return rules;
}

/** @returns {Set<string>} every `var(--name)` a file reads. */
function readUsedTokens(css) {
  const used = new Set();
  for (const match of css.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)/g)) used.add(match[1]);
  return used;
}

const nextRules = readRules(nextTokensSource, NEXT_TOKENS_PATH);
const sharedRules = readRules(sharedTokensSource, SHARED_TOKENS_PATH);

const nextTokens = new Map();
for (const rule of nextRules) {
  for (const [name, value] of rule.declarations) nextTokens.set(name, value);
}
const sharedTokens = new Map();
for (const rule of sharedRules) {
  for (const [name, value] of rule.declarations) sharedTokens.set(name, value);
}

// ── The file reaches both the shell and the dialogs it opens ───────────────
{
  const selectors = nextRules.map((rule) => rule.selector).join(' | ');
  assert.ok(
    selectors.includes('.next-shell'),
    'the colors must apply to the /next shell element'
  );
  assert.ok(
    /:root:has\(\s*\.next-shell\s*\)/.test(selectors),
    'the colors must also apply document-wide while /next is on screen, ' +
      'because dialogs are moved outside the shell element when they open'
  );
  assert.ok(
    /:is\(/.test(selectors),
    'the two selectors belong in one :is() list so an engine without :has() ' +
      'still gets the .next-shell half'
  );
}

// ── The palette the plan asked for ────────────────────────────────────────
{
  const expected = {
    '--color-bg': '#101014',
    '--color-surface': '#17171d',
    '--color-border': '#858599',
    '--color-text': '#eef0f9',
    '--color-text-2': '#a7a7b5',
    '--color-text-3': '#767687',
    '--color-selected': '#26302f',
    '--color-selected-border': '#4bbd9f',
    '--color-hover': '#25252e',
    '--color-focus-solid': '#4bf3c8',
    '--color-disabled-text': '#858599',
    '--color-status-idle': '#9494a5'
  };
  for (const [name, value] of Object.entries(expected)) {
    assert.equal(nextTokens.get(name), value, `${name} should be ${value}`);
  }
}

// ── Only the six planned semantic names are /next-only ────────────────────
// Everything else remains a shared token; broadening this list is an explicit
// product-level theme-contract decision.
{
  const nextOnly = new Set([...nextTokens.keys()].filter((name) => !sharedTokens.has(name)));
  assert.deepEqual(
    [...nextOnly].sort(),
    [...NEXT_ONLY_SEMANTIC_TOKENS].sort(),
    'the /next palette may add only the six approved semantic theme tokens'
  );
  for (const name of nextTokens.keys()) {
    assert.ok(
      sharedTokens.has(name) || NEXT_ONLY_SEMANTIC_TOKENS.has(name),
      `${name} is not a name any component reads (missing from ${SHARED_TOKENS_PATH})`
    );
  }
}

// ── Every name the settings dialog reads resolves ─────────────────────────
{
  /** @returns {Set<string>} every `--name:` a file DEFINES. */
  const readDefinedNames = (css) => {
    const defined = new Set();
    for (const match of stripComments(css).matchAll(/(?:^|[{;\s])(--[a-zA-Z0-9-]+)\s*:/g)) {
      defined.add(match[1]);
    }
    return defined;
  };

  // `next.css` both defines and reads names. The ones it defines are the
  // library's own slots (`--background`, `--primary`, ...) and belong to that
  // file, not to the token files, so they are not ours to look up here.
  const suppliedByNextStylesheet = readDefinedNames(read(NEXT_STYLESHEET_PATH));

  const used = new Set();
  for (const path of CONSUMER_PATHS) {
    for (const name of readUsedTokens(read(path))) {
      // `--bits-*` names are supplied at runtime by the bits-ui library
      // (menu width, available height) and `--tw-*` by Tailwind; neither is
      // ours to define.
      if (name.startsWith('--bits-') || name.startsWith('--tw-')) continue;
      if (suppliedByNextStylesheet.has(name)) continue;
      used.add(name);
    }
  }

  assert.ok(used.size >= 10, 'expected the dialog to read a real set of names');

  for (const name of used) {
    assert.ok(
      nextTokens.has(name) || sharedTokens.has(name),
      `${name} is read by the settings dialog but defined nowhere`
    );
  }

  // Colors and shadows are exactly what this file exists to change: if one of
  // them is left out, that part of the dialog keeps the old shell's color.
  const colorish = [...used].filter(
    (name) =>
      name.startsWith('--color-') || name.startsWith('--shadow-') || name === '--focus-ring'
  );
  assert.ok(colorish.length >= 10, 'expected a substantial set of color names');
  for (const name of colorish) {
    assert.ok(
      nextTokens.has(name),
      `${name} is a color the settings dialog reads, but ${NEXT_TOKENS_PATH} ` +
        'does not set it — it would render in the old shell palette'
    );
  }

  // Sizes, weights and radii are the same in both shells on purpose: the
  // /next file must NOT restate them, or they drift apart silently.
  const layoutish = [...used].filter(
    (name) =>
      name.startsWith('--space-') ||
      name.startsWith('--text-') ||
      name.startsWith('--weight-') ||
      name.startsWith('--radius-')
  );
  assert.ok(layoutish.length > 0, 'expected spacing, type or radius names to be in use');
  for (const name of layoutish) {
    assert.ok(
      !nextTokens.has(name),
      `${name} is shared with the old shell and should stay in ${SHARED_TOKENS_PATH} only`
    );
  }
}

// ── The host wires the pieces together ────────────────────────────────────
{
  const host = read(SETTINGS_HOST_PATH);
  assert.ok(
    host.includes("import '$lib/shell/styles/nextTokens.css'"),
    'SettingsHost must pull in the /next colors'
  );
  assert.ok(
    host.includes("import '$lib/shell/styles/next.css'"),
    'SettingsHost must pull in the Tailwind + shadcn variables too'
  );
  assert.ok(
    host.includes("import('$lib/shell/components/SettingsDialog.svelte')"),
    'SettingsHost must load the settings dialog on demand'
  );
  assert.ok(
    !host.includes('$lib/components/Dialog.svelte'),
    'the /next settings dialog is built from $lib/components/ui, not the old ' +
      'shell’s shared widgets'
  );
  assert.ok(
    /export function open\(\): void/.test(host),
    'SettingsHost must expose open()'
  );
  assert.ok(
    /export function close\(\): void/.test(host),
    'SettingsHost must expose close()'
  );
  assert.ok(
    host.includes('bind:open={dialogOpen}'),
    'SettingsHost must bind the panel’s open state'
  );
  assert.ok(
    !/invoke|tauriSource/.test(host),
    'SettingsHost must not talk to the backend'
  );
}

// ── The old shell cannot reach the /next stylesheet ───────────────────────
//
// The whole wave rests on one rule: the old page keeps rendering exactly as it
// did, because nothing it loads pulls in `next.css`. Up to now that rule lived
// only in a comment at the top of `next.css`, and the wave already had to add a
// second file that imports it. So walk the old shell's imports for real —
// starting at its route and the layout every route shares — and fail if
// `next.css`, or anything that imports it, turns up anywhere in that graph.
{
  const OLD_SHELL_ENTRY_POINTS = ['src/routes/+page.svelte', 'src/routes/+layout.svelte'];
  const FORBIDDEN = 'src/lib/shell/styles/next.css';

  /** Where a file's own imports could resolve to. */
  const CANDIDATE_SUFFIXES = ['', '.ts', '.js', '.svelte', '.css', '/index.ts', '/index.js'];

  /** Turn one import specifier into a project-relative path, or null. */
  const resolveSpecifier = (specifier, fromPath) => {
    let base;
    if (specifier.startsWith('$lib/')) base = 'src/lib/' + specifier.slice('$lib/'.length);
    else if (specifier === '$lib') base = 'src/lib';
    else if (specifier.startsWith('.')) {
      base = resolve(dirname(resolve(projectRoot, fromPath)), specifier).slice(
        projectRoot.length + 1
      );
    } else return null; // a package, not a file of ours

    // The vendored components import each other as `./thing.js` even though the
    // file on disk is `thing.ts`, so try the sibling extension too.
    const withoutJs = base.replace(/\.js$/, '');
    for (const stem of base === withoutJs ? [base] : [base, withoutJs]) {
      for (const suffix of CANDIDATE_SUFFIXES) {
        const candidate = stem + suffix;
        if (!candidate.includes('.')) continue;
        try {
          readFileSync(resolve(projectRoot, candidate), 'utf8');
          return candidate;
        } catch {
          /* keep trying */
        }
      }
    }
    return null;
  };

  /** Every import specifier a file names, static, dynamic, or CSS `@import`. */
  const importSpecifiers = (source) => {
    const found = [];
    const patterns = [
      /\bfrom\s*['"]([^'"]+)['"]/g,
      /\bimport\s*['"]([^'"]+)['"]/g,
      /\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
      /@import\s*(?:url\()?\s*['"]([^'"]+)['"]/g
    ];
    for (const pattern of patterns) {
      for (const match of source.matchAll(pattern)) found.push(match[1]);
    }
    return found;
  };

  const seen = new Set();
  /** path -> the file that pulled it in, so a hit can be explained. */
  const broughtInBy = new Map();
  const queue = [...OLD_SHELL_ENTRY_POINTS];
  for (const entry of queue) seen.add(entry);

  while (queue.length > 0) {
    const current = queue.shift();
    let source;
    try {
      source = read(current);
    } catch {
      continue;
    }
    for (const specifier of importSpecifiers(source)) {
      const target = resolveSpecifier(specifier, current);
      if (!target || seen.has(target)) continue;
      seen.add(target);
      broughtInBy.set(target, current);
      queue.push(target);
    }
  }

  // The walk has to actually walk, or it proves nothing.
  assert.ok(
    seen.size > 20,
    `the old shell import walk only found ${seen.size} files; it is not reaching ` +
      'its own components, so a hit could not be detected either'
  );

  if (seen.has(FORBIDDEN)) {
    const chain = [];
    let step = FORBIDDEN;
    while (step && chain.length < 20) {
      chain.unshift(step);
      step = broughtInBy.get(step);
    }
    assert.fail(
      `the old shell can now reach ${FORBIDDEN}, which restyles the whole old ` +
        `page. It gets there like this: ${chain.join(' -> ')}`
    );
  }

  // And prove the walk would have caught it: the /next page really does import
  // the stylesheet, so the same walk started there must find it.
  {
    const nextQueue = ['src/routes/next/+page.svelte'];
    const nextSeen = new Set(nextQueue);
    while (nextQueue.length > 0) {
      const current = nextQueue.shift();
      let source;
      try {
        source = read(current);
      } catch {
        continue;
      }
      for (const specifier of importSpecifiers(source)) {
        const target = resolveSpecifier(specifier, current);
        if (!target || nextSeen.has(target)) continue;
        nextSeen.add(target);
        nextQueue.push(target);
      }
    }
    assert.ok(
      nextSeen.has(FORBIDDEN),
      `the walk did not find ${FORBIDDEN} even from the /next page, so it cannot ` +
        'be trusted to find it from the old page either'
    );
  }
}

// ── Every `data-something:` class matches an attribute that really appears ──
//
// The vendored components come from a registry written against a newer version
// of bits-ui than this app pins. The newer one marks state with its own
// attribute (`data-checked`); the pinned one writes `data-state="checked"`.
// Tailwind reads a bare `data-checked:` prefix as "carries an attribute named
// data-checked", so every rule behind a name the library never writes is dead
// CSS — which is how the settings switches shipped with no visible on/off state.
//
// This check reads the attribute names bits-ui actually emits, adds the ones
// our own components set by hand and the ones `next.css` explicitly redefines,
// and fails on any other bare `data-…:` prefix in the /next tree.
{
  const nextStylesheet = read(NEXT_STYLESHEET_PATH);

  /** Names `next.css` has taught Tailwind, e.g. `@custom-variant data-checked (...)`. */
  const redefined = new Set(
    [...nextStylesheet.matchAll(/@custom-variant\s+(data-[a-z-]+)\s/g)].map((m) => m[1])
  );
  for (const required of [
    'data-checked',
    'data-unchecked',
    'data-active',
    'data-open',
    'data-closed'
  ]) {
    assert.ok(
      redefined.has(required),
      `${NEXT_STYLESHEET_PATH} must redefine \`${required}\`, because the pinned ` +
        'bits-ui writes that state as a value of `data-state` instead'
    );
    const declaration = new RegExp(
      `@custom-variant\\s+${required}\\s*\\([^)]*data-state=`
    );
    assert.ok(
      declaration.test(nextStylesheet),
      `\`${required}\` must be defined in terms of \`data-state\`, or it still ` +
        'matches nothing'
    );
  }

  /**
   * Attribute names bits-ui writes as bare presence — the value is either the
   * empty string or nothing at all — which is what a bare `data-x:` prefix
   * needs. Read out of the installed package so this cannot go stale.
   */
  const presence = new Set();
  const bitsRoot = resolve(projectRoot, 'node_modules/bits-ui/dist');
  const collect = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const full = resolve(directory, entry.name);
      if (entry.isDirectory()) collect(full);
      else if (entry.name.endsWith('.js')) {
        const source = readFileSync(full, 'utf8');
        for (const match of source.matchAll(/"(data-[a-z-]+)":\s*([^\n]*)/g)) {
          // The attribute is a presence flag when the library can write it as
          // the empty string — either literally, or through one of the helpers
          // whose whole job is "empty string when true, nothing when false".
          const value = match[2];
          if (/""/.test(value) || /boolToEmptyStrOrUndef/.test(value)) {
            presence.add(match[1]);
          }
        }
      }
    }
  };
  collect(bitsRoot);
  assert.ok(presence.size > 3, 'expected to read a real set of attributes out of bits-ui');

  const OWN_TREE = [
    'src/lib/components/ui',
    'src/lib/shell',
    'src/routes/next'
  ];

  const svelteFiles = [];
  const walk = (directory) => {
    for (const entry of readdirSync(resolve(projectRoot, directory), { withFileTypes: true })) {
      const relative = `${directory}/${entry.name}`;
      if (entry.isDirectory()) walk(relative);
      else if (entry.name.endsWith('.svelte') || entry.name.endsWith('.ts')) {
        svelteFiles.push(relative);
      }
    }
  };
  for (const directory of OWN_TREE) walk(directory);
  assert.ok(svelteFiles.length > 30, 'expected to find the /next components');

  /** Attributes our own markup sets by hand, e.g. `data-inset={inset}`. */
  const setByHand = new Set();
  for (const path of svelteFiles) {
    for (const match of read(path).matchAll(/(?:^|\s)(data-[a-z-]+)=/g)) {
      setByHand.add(match[1]);
    }
  }

  const unmatched = [];
  for (const path of svelteFiles) {
    for (const match of read(path).matchAll(/(?:^|[\s"'/:])(data-[a-z-]+):/g)) {
      const name = match[1];
      if (redefined.has(name) || presence.has(name) || setByHand.has(name)) continue;
      unmatched.push(`${path}: ${name}:`);
    }
  }

  assert.deepEqual(
    unmatched,
    [],
    'these classes are styled on an attribute nothing ever writes, so they do ' +
      'nothing at all. Either the component library spells that state as a ' +
      'value of `data-state` (add a `@custom-variant` line to next.css), or the ' +
      'attribute name is simply wrong:\n  ' + unmatched.join('\n  ')
  );
}

console.log('nextTokens.test.mjs: all checks passed');
