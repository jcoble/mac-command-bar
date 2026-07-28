import assert from 'node:assert/strict';
import { normalizeBrowserUrl } from '../src/lib/shell/browser/normalizeBrowserUrl.ts';

// empty / whitespace input is not a URL
{
  assert.equal(normalizeBrowserUrl(''), '');
  assert.equal(normalizeBrowserUrl('   '), '');
  assert.equal(normalizeBrowserUrl('\t\n'), '');
}

// a bare port becomes localhost on that port
{
  assert.equal(normalizeBrowserUrl(':5177'), 'http://localhost:5177/');
  assert.equal(normalizeBrowserUrl('  :3000  '), 'http://localhost:3000/');
  assert.equal(normalizeBrowserUrl(':5177/next'), 'http://localhost:5177/next');
}

// loopback host shorthands get the http scheme
{
  assert.equal(normalizeBrowserUrl('localhost:5177'), 'http://localhost:5177/');
  assert.equal(normalizeBrowserUrl('localhost'), 'http://localhost/');
  assert.equal(normalizeBrowserUrl('LOCALHOST:5177'), 'http://localhost:5177/');
  assert.equal(normalizeBrowserUrl('127.0.0.1:8080/health'), 'http://127.0.0.1:8080/health');
  assert.equal(normalizeBrowserUrl('0.0.0.0:9000'), 'http://0.0.0.0:9000/');
  assert.equal(normalizeBrowserUrl('[::1]:5177'), 'http://[::1]:5177/');
}

// full URLs survive, normalized by the URL parser
{
  assert.equal(normalizeBrowserUrl('http://example.com'), 'http://example.com/');
  assert.equal(normalizeBrowserUrl('https://example.com/a?b=1#c'), 'https://example.com/a?b=1#c');
  assert.equal(normalizeBrowserUrl('HTTPS://Example.COM/Path'), 'https://example.com/Path');
  assert.equal(normalizeBrowserUrl(' http://localhost:5177/next '), 'http://localhost:5177/next');
}

// anything that is not http or https is rejected
{
  assert.equal(normalizeBrowserUrl('file:///Users/me/index.html'), '');
  assert.equal(normalizeBrowserUrl('mailto:someone@example.com'), '');
  assert.equal(normalizeBrowserUrl('javascript:alert(1)'), '');
  assert.equal(normalizeBrowserUrl('ftp://example.com'), '');
  assert.equal(normalizeBrowserUrl('tauri://localhost'), '');
}

// unparseable text is rejected rather than thrown
{
  assert.equal(normalizeBrowserUrl('not a url'), '');
  assert.equal(normalizeBrowserUrl('/Users/me/project'), '');
  assert.equal(normalizeBrowserUrl('::::'), '');
  assert.equal(normalizeBrowserUrl('example.com'), '', 'a bare non-loopback host is not assumed to be http');
}

// the result of normalizing is stable: feeding it back changes nothing
{
  for (const input of [':5177', 'localhost:5177/next', 'https://example.com/a']) {
    const once = normalizeBrowserUrl(input);
    assert.equal(normalizeBrowserUrl(once), once, `stable for ${input}`);
  }
}

console.log('normalizeBrowserUrl: all tests passed');
