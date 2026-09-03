/**
 * normalizeBrowserUrl.ts — turn whatever the user typed into a browsable URL.
 *
 * Pure: no DOM, no storage, no backend.
 *
 * Accepted shorthands:
 *   ":5177"                  → http://localhost:5177/
 *   "localhost:5177/foo"     → http://localhost:5177/foo
 *   "127.0.0.1", "[::1]:80"  → http://…
 *   "www.example.com"         → https://www.example.com/
 *   "https://example.com"    → unchanged (normalized by the URL parser)
 *
 * Anything that is not http or https — a file path, a mailto:, gibberish —
 * comes back as an empty string. Callers treat "" as "not a URL I can show".
 */
export function normalizeBrowserUrl(value: string): string {
  const trimmedValue = value.trim();
  if (!trimmedValue) return '';

  const withProtocol = /^https?:\/\//i.test(trimmedValue)
    ? trimmedValue
    : trimmedValue.startsWith(':')
      ? `http://localhost${trimmedValue}`
      : /^(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])(:\d+)?(\/.*)?$/i.test(trimmedValue)
        ? `http://${trimmedValue}`
        : /^[\w.-]+\.[a-z]{2,}(:\d+)?(\/.*)?$/i.test(trimmedValue)
          ? `https://${trimmedValue}`
          : trimmedValue;

  try {
    const parsedUrl = new URL(withProtocol);
    if (parsedUrl.protocol !== 'http:' && parsedUrl.protocol !== 'https:') return '';
    return parsedUrl.toString();
  } catch {
    return '';
  }
}
