/**
 * One glyph per kind of agent, and the name to put beside it.
 *
 * The collapsed session strip and the rail rows both mark a session with the
 * same small icon, so the mapping lives here rather than in either component.
 * The name is the provider id read back as words, so adding a provider needs
 * no new string here.
 */
import Bot from '@lucide/svelte/icons/bot';
import Gem from '@lucide/svelte/icons/gem';
import SquareCode from '@lucide/svelte/icons/square-code';
import SquareTerminal from '@lucide/svelte/icons/square-terminal';
import Terminal from '@lucide/svelte/icons/terminal';

import type { AgentKind } from './ownedSessions.ts';

export const AGENT_ICONS: Record<AgentKind, typeof Bot> = {
  claude: Bot,
  codex: SquareCode,
  gemini: Gem,
  opencode: SquareTerminal,
  other: Terminal
};

/** What to call this kind of session in a tooltip or a detail line. */
export function agentDisplayName(agent: AgentKind, viaCmux = false): string {
  const name = agent
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((part) => `${part[0]?.toUpperCase() ?? ''}${part.slice(1)}`)
    .join(' ');
  const base = agent === 'other' ? 'Terminal session' : name;
  return viaCmux ? `${base} (multiplexed)` : base;
}
