/**
 * sessionContextModel.ts — the arithmetic behind the Context panel.
 *
 * The panel answers four questions about one session: what it is running with,
 * how much of its context window it has used, which files it has touched, and
 * what is attached to it. Three of those come out of numbers a provider may or
 * may not have sent, and the one rule this whole file exists to enforce is that
 * a number nobody sent is never filled in. A missing model comes back
 * `missing: true` and the panel prints "not reported". A used-token count with
 * no window beside it produces no percentage, because a percentage of an
 * unknown total is a guess wearing a number's clothes.
 *
 * Everything here is pure so the script test can check exactly that, with no
 * store, no runtime and no backend in the way.
 */

import type {
  AgentConfigValue,
  AgentConversationEvent,
  ConversationMetadata,
  ConversationUsage
} from '../../conversation/conversationTypes.ts';
import type { AgentConversationConfigState } from '../../conversation/conversationConfig.ts';

export interface SessionContextFact {
  label: string;
  /** The value to show, already formatted. Empty when `missing`. */
  value: string;
  /** True when the provider did not report this. The view shows "not reported". */
  missing: boolean;
}

export interface SessionContextUsage {
  usedTokens: number | null;
  contextWindow: number | null;
  /** 0-100, or null when either number is missing. Never guessed. */
  percentUsed: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface SessionFileTouch {
  path: string;
  /** How many tool calls referenced this path. */
  count: number;
  /** Milliseconds, of the most recent reference. */
  lastTouchedMs: number;
}

/** A reported number, or null. Guards against the NaN and the negative that a
 * malformed payload can hand us; neither is a number worth showing. */
function reportedNumber(value: unknown): number | null {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) return null;
  return value;
}

/** A reported string, or null. Blank and whitespace count as not reported. */
function reportedText(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function fact(label: string, value: string | null): SessionContextFact {
  return value === null ? { label, value: '', missing: true } : { label, value, missing: false };
}

/** Model, effort, and access mode. A null field comes back with missing: true. */
export function sessionContextFacts(metadata: ConversationMetadata | null): SessionContextFact[] {
  return [
    fact('Model', reportedText(metadata?.model)),
    fact('Effort', reportedText(metadata?.effort)),
    fact('Access mode', reportedText(metadata?.approvalPolicy))
  ];
}

/**
 * Fill a null metadata field from the same session's agent config.
 *
 * Both sides are the provider's own report about this one session — the config
 * is simply where the approval policy usually arrives, while the metadata is
 * where the model and effort usually do. Nothing is inferred across sessions
 * and nothing is defaulted: when both sides are null the field stays null and
 * the fact still reads "not reported".
 */
export function metadataWithConfigFallback(
  metadata: ConversationMetadata | null,
  config: AgentConversationConfigState | null | undefined
): ConversationMetadata | null {
  if (!metadata && !config) return null;
  return {
    model: reportedText(metadata?.model) ?? reportedText(config?.model),
    effort: reportedText(metadata?.effort) ?? reportedText(config?.reasoningEffort),
    approvalPolicy: reportedText(metadata?.approvalPolicy) ?? reportedText(config?.approvalPolicy),
    usedTokens: reportedNumber(metadata?.usedTokens),
    contextWindow: reportedNumber(metadata?.contextWindow)
  };
}

/** Never computes a percentage from a partial pair, and never invents a context window. */
export function sessionContextUsage(
  metadata: ConversationMetadata | null,
  usage: ConversationUsage | null | undefined
): SessionContextUsage {
  const usedTokens = reportedNumber(usage?.usedTokens) ?? reportedNumber(metadata?.usedTokens);
  const contextWindow =
    reportedNumber(usage?.contextWindow) ?? reportedNumber(metadata?.contextWindow);
  const percentUsed =
    usedTokens !== null && contextWindow !== null && contextWindow > 0
      ? Math.min(100, Math.round((usedTokens / contextWindow) * 100))
      : null;

  return {
    usedTokens,
    contextWindow,
    percentUsed,
    inputTokens: reportedNumber(usage?.inputTokens),
    outputTokens: reportedNumber(usage?.outputTokens)
  };
}

/** Every path a single tool-call payload names, in the order it names them. */
function pathsInPayload(payload: AgentConversationEvent['payload']): string[] {
  if (payload.kind !== 'toolCall' && payload.kind !== 'toolCallUpdate') return [];

  const paths: string[] = [];
  const direct = reportedText(payload.path);
  if (direct !== null) paths.push(direct);

  // `locations` is whatever the provider chose to send: a list of strings, a
  // list of objects with a path, or a single object. Anything else is skipped
  // rather than guessed at.
  const locations: AgentConfigValue | undefined = payload.locations;
  const entries = Array.isArray(locations) ? locations : locations ? [locations] : [];
  for (const entry of entries) {
    const asText = reportedText(entry);
    if (asText !== null) {
      paths.push(asText);
      continue;
    }
    if (entry && typeof entry === 'object' && !Array.isArray(entry)) {
      const nested = reportedText(entry.path);
      if (nested !== null) paths.push(nested);
    }
  }

  return paths;
}

/**
 * Pulls `path` and `locations` out of toolCall / toolCallUpdate payloads,
 * de-duplicates by path, and sorts by most recently touched.
 *
 * One payload naming the same path twice counts once, so an update that repeats
 * its own path does not inflate the tally; two separate calls on the same path
 * count twice, because that is two touches.
 */
export function sessionFilesTouched(
  events: readonly AgentConversationEvent[]
): SessionFileTouch[] {
  const touches = new Map<string, SessionFileTouch>();

  for (const event of events) {
    const timestampMs = reportedNumber(event.timestampMs) ?? 0;
    for (const path of new Set(pathsInPayload(event.payload))) {
      const existing = touches.get(path);
      if (existing) {
        existing.count += 1;
        existing.lastTouchedMs = Math.max(existing.lastTouchedMs, timestampMs);
      } else {
        touches.set(path, { path, count: 1, lastTouchedMs: timestampMs });
      }
    }
  }

  return [...touches.values()].sort(
    (left, right) =>
      right.lastTouchedMs - left.lastTouchedMs || left.path.localeCompare(right.path)
  );
}

/** A token count with thousands separators, for a line a person reads. */
export function formatTokenCount(value: number): string {
  return value.toLocaleString('en-US');
}

/** The last segment of a path, for the row's headline. */
export function fileNameOf(path: string): string {
  const segments = path.split('/').filter((segment) => segment.length > 0);
  return segments.at(-1) ?? path;
}
