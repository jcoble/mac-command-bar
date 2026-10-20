import type { IncomingHttpHeaders } from 'node:http';

const LOCAL_HOST = /^(127\.0\.0\.1|localhost|\[::1\])(:\d+)?$/;

/**
 * The dev server's source bridge reads and writes any file, so it answers only
 * pages this dev server served. A local Host header shuts out websites whose DNS
 * name was pointed at 127.0.0.1, and the page's Origin (or its Referer when a
 * browser leaves Origin out) must be this server, which shuts out pages on other
 * origins and callers that name no page at all.
 */
export function isOwnPageRequest(headers: Pick<IncomingHttpHeaders, 'host' | 'origin' | 'referer'>): boolean {
  const host = headers.host?.toLowerCase() ?? '';
  const page = headers.origin ?? headers.referer;
  if (!LOCAL_HOST.test(host) || !page) return false;
  try {
    return new URL(page).origin === `http://${host}`;
  } catch {
    return false;
  }
}
