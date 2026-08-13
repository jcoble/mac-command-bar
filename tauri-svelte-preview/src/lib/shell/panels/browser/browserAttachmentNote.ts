/**
 * browserAttachmentNote.ts — the words that go with the picture.
 *
 * A marked-up screenshot on its own leaves the reader guessing which page it
 * came from and which element was pointed at. This is the short block that goes
 * into the composer draft beside it: the address, what was picked when Select
 * was used, and what the person typed. Nothing else — the image carries the
 * rest, and a label with nothing after it is noise, so an absent field simply
 * has no line.
 *
 * PURE: no DOM, no backend call.
 */
export interface BrowserAttachmentNote {
  url: string;
  /** The picked element's selector, when Select was used. */
  selector: string | null;
  /** The picked element's tag, for example "svg". */
  tag: string | null;
  /** What the person typed in the mini composer. */
  description: string;
}

function clean(value: string | null | undefined): string {
  return typeof value === 'string' ? value.trim() : '';
}

/**
 * The element's tag, read off the end of its selector. The picker answers with
 * a selector rather than a tag, and the last step of it is the element itself —
 * `main > section:nth-child(2) h1.title` is an `h1`. Anything that is not a
 * plain tag name (an id, a class on its own) has no tag to show.
 */
export function elementTagFromSelector(selector: string | null | undefined): string | null {
  const last = clean(selector).split(/\s|>/).filter(Boolean).pop() ?? '';
  const tag = last.split(/[.#:[]/)[0]?.toLowerCase() ?? '';
  return /^[a-z][a-z0-9-]*$/.test(tag) ? tag : null;
}

/** The plain-English block appended to the composer draft alongside the image. */
export function formatAttachmentNote(note: BrowserAttachmentNote): string {
  const lines: string[] = [];
  const url = clean(note.url);
  const tag = clean(note.tag);
  const selector = clean(note.selector);
  const description = clean(note.description);

  if (url) lines.push(`Page: ${url}`);
  if (tag) lines.push(`Element: ${tag}`);
  if (selector) lines.push(`Selector: ${selector}`);
  if (description) lines.push(description);

  return lines.join('\n');
}
