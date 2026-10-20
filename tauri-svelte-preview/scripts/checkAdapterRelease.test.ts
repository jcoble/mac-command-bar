import assert from 'node:assert/strict';
import test from 'node:test';
import { releaseConcerns, type NpmRelease } from './checkAdapterRelease.ts';

// Shapes copied from `npm view @agentclientprotocol/codex-acp@1.13.1 --json`.
const ci = 'GitHub Actions <npm-oidc-no-reply@github.com>';
const maintainers = ['benbrandt <benjamin.j.brandt@gmail.com>', 'aguzubiaga <sup@aguz.me>', 'cirwin <conrad.irwin@gmail.com>'];
const previous: NpmRelease = {
  _npmUser: ci,
  maintainers,
  scripts: { build: 'node build.mjs', prepublishOnly: 'npm run build' },
  dist: { attestations: { url: 'https://registry.npmjs.org/-/npm/v1/attestations/x', provenance: {} } },
};

test('a release from the same publisher and maintainers passes', () => {
  assert.deepEqual(releaseConcerns(previous, { ...previous, maintainers: [...maintainers].reverse() }), []);
});

test('a release by one of the previous maintainers passes', () => {
  assert.deepEqual(releaseConcerns(previous, { ...previous, _npmUser: maintainers[1] }), []);
});

test('a publisher who is neither the previous publisher nor a maintainer fails', () => {
  const concerns = releaseConcerns(previous, { ...previous, _npmUser: 'mallory <m@example.com>' });
  assert.equal(concerns.length, 1);
  assert.match(concerns[0], /mallory/);
});

test('a changed maintainer list fails', () => {
  const concerns = releaseConcerns(previous, { ...previous, maintainers: [...maintainers, 'mallory <m@example.com>'] });
  assert.equal(concerns.length, 1);
  assert.match(concerns[0], /maintainers changed/);
});

test('a new install script fails', () => {
  const concerns = releaseConcerns(previous, { ...previous, scripts: { ...previous.scripts, postinstall: 'node x.js' } });
  assert.equal(concerns.length, 1);
  assert.match(concerns[0], /postinstall/);
});

test('an install script the previous release already had passes', () => {
  const withInstall = { ...previous, scripts: { install: 'node-gyp rebuild' } };
  assert.deepEqual(releaseConcerns(withInstall, withInstall), []);
});

test('losing npm provenance fails, but never having it does not', () => {
  const concerns = releaseConcerns(previous, { ...previous, dist: {} });
  assert.equal(concerns.length, 1);
  assert.match(concerns[0], /provenance/);
  const unsigned = { ...previous, dist: {} };
  assert.deepEqual(releaseConcerns(unsigned, unsigned), []);
});
