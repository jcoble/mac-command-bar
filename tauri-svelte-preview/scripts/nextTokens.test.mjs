/**
 * nextTokens.test.mjs — checks the /next color file against the components
 * that will read it.
 *
 * The settings dialog is painted entirely from custom property names
 * (`--color-text`, `--shadow-lg`, ...). If the /next color file misses one,
 * that one value silently falls back to the old shell's palette and the
 * dialog comes out two-tone. These checks read the actual component files,
 * collect every name they use, and prove the /next file covers the ones it
 * is responsible for.
 *
 * Run: node --experimental-strip-types scripts/nextTokens.test.mjs
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const read = (relativePath) => readFileSync(resolve(projectRoot, relativePath), 'utf8');

const NEXT_TOKENS_PATH = 'src/lib/shell/styles/nextTokens.css';
const SHARED_TOKENS_PATH = 'src/lib/styles/tokens.css';
const SETTINGS_HOST_PATH = 'src/lib/shell/components/SettingsHost.svelte';

/** Every file whose styling ends up inside the settings dialog. */
const CONSUMER_PATHS = [
  'src/lib/SettingsPanel.svelte',
  'src/lib/components/Dialog.svelte',
  'src/lib/components/Tabs.svelte',
  'src/lib/components/Select.svelte',
  'src/lib/components/Slider.svelte',
  'src/lib/components/Switch.svelte',
  'src/lib/components/Button.svelte'
];

const nextTokensSource = read(NEXT_TOKENS_PATH);
const sharedTokensSource = read(SHARED_TOKENS_PATH);

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
    '--color-border': '#22222c',
    '--color-text': '#d8d8e0',
    '--color-text-2': '#6d6d7d',
    '--color-text-3': '#4c4c5a'
  };
  for (const [name, value] of Object.entries(expected)) {
    assert.equal(nextTokens.get(name), value, `${name} should be ${value}`);
  }
}

// ── Nothing new is invented ───────────────────────────────────────────────
// Every name here must already exist in the shared token file, otherwise it
// is a name no component reads.
{
  for (const name of nextTokens.keys()) {
    assert.ok(
      sharedTokens.has(name),
      `${name} is not a name any component reads (missing from ${SHARED_TOKENS_PATH})`
    );
  }
}

// ── Every name the settings dialog reads resolves ─────────────────────────
{
  const used = new Set();
  for (const path of CONSUMER_PATHS) {
    for (const name of readUsedTokens(read(path))) {
      // `--bits-*` names are supplied at runtime by the bits-ui library
      // (menu width, available height); they are not ours to define.
      if (name.startsWith('--bits-')) continue;
      used.add(name);
    }
  }

  assert.ok(used.size > 20, 'expected the dialog to read a real set of names');

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
  assert.ok(colorish.length >= 12, 'expected a substantial set of color names');
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
      name.startsWith('--space-') || name.startsWith('--text-') || name.startsWith('--weight-')
  );
  assert.ok(layoutish.length > 0, 'expected spacing and type names to be in use');
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
    host.includes("import('$lib/SettingsPanel.svelte')"),
    'SettingsHost must load the existing settings panel on demand'
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

console.log('nextTokens.test.mjs: all checks passed');
