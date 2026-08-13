/**
 * Turns backend runtime errors into calm, useful session-list copy while
 * retaining the original diagnostic for an explicitly requested detail view.
 */
export interface PresentedAgentError {
  summary: string;
  detail?: string;
}

function errorCode(raw: string): string | null {
  return raw.match(/^\s*([a-z][a-z0-9-]*):/i)?.[1]?.toLowerCase() ?? null;
}

function prettyPrintEmbeddedJson(raw: string): string {
  const starts = [raw.indexOf('{'), raw.indexOf('[')]
    .filter((index) => index >= 0)
    .sort((left, right) => left - right);

  for (const start of starts) {
    const candidate = raw.slice(start).trim();
    try {
      const parsed = JSON.parse(candidate) as unknown;
      const prefix = raw.slice(0, start).trimEnd();
      const pretty = JSON.stringify(parsed, null, 2);
      return prefix ? `${prefix}\n${pretty}` : pretty;
    } catch {
      // This delimiter was ordinary message text; try the next one.
    }
  }

  return raw;
}

function humanSummary(raw: string): string {
  const normalized = raw.toLowerCase();
  const code = errorCode(raw);

  if (normalized.includes('no rollout found')) {
    return "This session's history is missing, so it can't be resumed.";
  }

  if (
    normalized.includes('authentication required')
    || normalized.includes('auth_required')
    || normalized.includes('please run /login')
  ) {
    return 'Agent login required. Run `claude /login` in a terminal, then try again.';
  }

  if (code === 'invalid-response' || normalized.includes('did not include sessionid')) {
    return 'The agent sent an unexpected reply.';
  }

  if (code === 'session-not-started' || normalized.includes('session has not started')) {
    return "The session hasn't connected yet.";
  }

  if (code === 'sidecar-spawn') {
    return "The agent couldn't start.";
  }

  if (code === 'not-initialized') {
    return "The agent hasn't finished starting.";
  }

  if (code === 'invalid-capabilities') {
    return 'The agent reported unsupported features.';
  }

  if (code === 'invalid-config') {
    return "The agent couldn't apply that setting.";
  }

  if (code === 'serialization') {
    return "The app couldn't prepare the agent request.";
  }

  if (code === 'inbound-already-taken') {
    return 'The agent connection is already in use.';
  }

  if (code === 'empty-response') {
    return 'The agent replied without any text.';
  }

  if (/\b(?:exited|exit status|terminated unexpectedly)\b/.test(normalized)) {
    return 'The agent stopped unexpectedly.';
  }

  if (
    /\b(?:transport|connection|stdout|stream)\b[^\n]*\bclosed\b/.test(normalized) ||
    /\bclosed\b[^\n]*\b(?:transport|connection|stdout|stream|response)\b/.test(normalized)
  ) {
    return 'The agent connection closed.';
  }

  if (
    code === 'transport' ||
    code === 'acp-transport' ||
    /\b(?:broken pipe|connection reset|transport failure)\b/.test(normalized)
  ) {
    return 'The connection to the agent was interrupted.';
  }

  if (code === 'acp-error') {
    return "The agent couldn't complete that request.";
  }

  return trimmedSummary(raw) || 'The agent hit an error.';
}

function trimmedSummary(raw: string): string {
  const text = raw.trim();
  const characters = Array.from(text);
  if (characters.length <= 160) return text;

  const available = characters.slice(0, 159).join('').trimEnd();
  const lastSpace = available.search(/\s+\S*$/);
  const wordSafe = lastSpace > 0 ? available.slice(0, lastSpace) : available;
  return `${wordSafe}…`;
}

export function presentAgentError(raw: string): PresentedAgentError {
  const summary = humanSummary(raw);
  if (raw.trim().length === 0) return { summary };

  return {
    summary,
    detail: prettyPrintEmbeddedJson(raw)
  };
}
