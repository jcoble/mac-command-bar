const safeLinkProtocolPattern = /^(https?:|mailto:|file:|\/|\.\/|\.\.\/|#)/i;

export function renderSourceMarkdownPreview(markdown: string): string {
  const lines = markdown.replace(/\r\n?/g, '\n').split('\n');
  const html: string[] = [];
  let paragraphLines: string[] = [];
  let listItems: string[] = [];
  let codeFenceLanguage = '';
  let codeFenceLines: string[] | null = null;

  const flushParagraph = () => {
    if (paragraphLines.length === 0) return;
    html.push(`<p>${renderInlineMarkdown(paragraphLines.join(' '))}</p>`);
    paragraphLines = [];
  };

  const flushList = () => {
    if (listItems.length === 0) return;
    html.push(`<ul>${listItems.map((item) => `<li>${renderInlineMarkdown(item)}</li>`).join('')}</ul>`);
    listItems = [];
  };

  const flushCodeFence = () => {
    if (!codeFenceLines) return;
    const language = codeFenceLanguage ? ` class="language-${escapeAttribute(codeFenceLanguage)}"` : '';
    html.push(`<pre><code${language}>${escapeHtml(`${codeFenceLines.join('\n')}\n`)}</code></pre>`);
    codeFenceLines = null;
    codeFenceLanguage = '';
  };

  for (const line of lines) {
    const fenceMatch = /^```(?<language>[A-Za-z0-9_-]*)\s*$/.exec(line);
    if (fenceMatch) {
      if (codeFenceLines) {
        flushCodeFence();
      } else {
        flushParagraph();
        flushList();
        codeFenceLanguage = fenceMatch.groups?.language ?? '';
        codeFenceLines = [];
      }
      continue;
    }

    if (codeFenceLines) {
      codeFenceLines.push(line);
      continue;
    }

    if (line.trim() === '') {
      flushParagraph();
      flushList();
      continue;
    }

    const headingMatch = /^(?<marks>#{1,6})\s+(?<text>.+)$/.exec(line);
    if (headingMatch?.groups) {
      flushParagraph();
      flushList();
      const level = headingMatch.groups.marks.length;
      html.push(`<h${level}>${renderInlineMarkdown(headingMatch.groups.text.trim())}</h${level}>`);
      continue;
    }

    const listMatch = /^[-*]\s+(?<text>.+)$/.exec(line);
    if (listMatch?.groups) {
      flushParagraph();
      listItems.push(listMatch.groups.text);
      continue;
    }

    const quoteMatch = /^>\s?(?<text>.*)$/.exec(line);
    if (quoteMatch?.groups) {
      flushParagraph();
      flushList();
      html.push(`<blockquote>${renderInlineMarkdown(quoteMatch.groups.text)}</blockquote>`);
      continue;
    }

    flushList();
    paragraphLines.push(line.trim());
  }

  flushParagraph();
  flushList();
  flushCodeFence();

  return html.join('\n');
}

export function sourceMarkdownPreviewTextSummary(markdown: string, maxLength = 160): string {
  const cleaned = markdown
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/^>\s?/gm, '')
    .replace(/^[-*]\s+/gm, '')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\*([^*]+)\*/g, '$1')
    .replace(/[_~]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  return cleaned.length > maxLength ? `${cleaned.slice(0, Math.max(0, maxLength - 3)).trimEnd()}...` : cleaned;
}

function renderInlineMarkdown(text: string): string {
  return escapeHtml(text)
    .replace(/\[([^\]]+)\]\(([^)\s]+(?:\([^)]*\)[^)]*)?)\)/g, (_match, label: string, href: string) =>
      renderMarkdownLink(label, href)
    )
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>');
}

function renderMarkdownLink(label: string, href: string): string {
  const trimmedHref = href.trim();
  if (!safeLinkProtocolPattern.test(trimmedHref)) {
    return label;
  }
  return `<a href="${escapeAttribute(trimmedHref)}" target="_blank" rel="noreferrer">${label}</a>`;
}

function escapeAttribute(value: string): string {
  return escapeHtml(value).replace(/"/g, '&quot;');
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
