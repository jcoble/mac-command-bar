import assert from 'node:assert/strict';
import {
  renderSourceMarkdownPreview,
  sourceMarkdownPreviewTextSummary
} from '../src/lib/sourceMarkdownPreview.ts';

const rendered = renderSourceMarkdownPreview(`# Plan

Intro with **bold** and \`code\`.

- first
- second

> quoted <script>alert(1)</script>

\`\`\`ts
const value = "<unsafe>";
\`\`\`

[Docs](https://example.com/docs)
[Bad](javascript:alert(1))
`);

assert.match(rendered, /<h1>Plan<\/h1>/, 'Top-level headings should render as headings');
assert.match(rendered, /<strong>bold<\/strong>/, 'Bold inline text should render');
assert.match(rendered, /<code>code<\/code>/, 'Inline code should render');
assert.match(rendered, /<ul>\s*<li>first<\/li>\s*<li>second<\/li>\s*<\/ul>/, 'Bulleted lists should group adjacent items');
assert.match(rendered, /<blockquote>quoted &lt;script&gt;alert\(1\)&lt;\/script&gt;<\/blockquote>/, 'Blockquotes should escape raw HTML');
assert.match(rendered, /<pre><code class="language-ts">const value = &quot;&lt;unsafe&gt;&quot;;\n<\/code><\/pre>/, 'Fenced code should preserve escaped code content');
assert.match(rendered, /<a href="https:\/\/example\.com\/docs" target="_blank" rel="noreferrer">Docs<\/a>/, 'Safe links should render as external anchors');
assert.doesNotMatch(rendered, /javascript:alert/, 'Unsafe link protocols should not render as hrefs');
assert.equal(
  sourceMarkdownPreviewTextSummary('# Title\n\nBody text with **formatting**.'),
  'Title Body text with formatting.',
  'Markdown preview summary should strip simple syntax for labels'
);
