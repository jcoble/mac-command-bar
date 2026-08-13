<!--
  AgentRow.svelte — one subagent in the Agents panel.

  Name and status on the first line, what it is doing on the second, and how
  many messages its transcript holds when that is known. A count only exists
  for an agent whose transcript has been read, so the rest show nothing at all
  rather than a zero that would be a guess.
-->
<script lang="ts">
  import FileText from '@lucide/svelte/icons/file-text';

  import { Chip } from '$lib/components/ui/chip/index.js';
  import { HoverActionButton } from '$lib/components/ui/hover-actions/index.js';
  import { ListRow } from '$lib/components/ui/list-row/index.js';

  import type { AgentActivityRow, AgentStatus } from './agentActivityModel.ts';

  interface Props {
    row: AgentActivityRow;
    selected: boolean;
    onselect: (childId: string) => void;
    /** Opens the agent's log. Only ever called when the row has a path. */
    onopenlog?: (logPath: string) => void;
  }
  let { row, selected, onselect, onopenlog }: Props = $props();

  const STATUS_TONE = {
    working: 'live',
    done: 'good',
    failed: 'bad',
    idle: 'neutral'
  } as const;

  const STATUS_WORD: Record<AgentStatus, string> = {
    working: 'Working',
    done: 'Done',
    failed: 'Failed',
    idle: 'Idle'
  };

  const NO_LOG_HINT = "This agent's log is not available yet.";
</script>

<ListRow
  {selected}
  actionsLabel="Agent actions"
  onclick={() => onselect(row.childId)}
  data-testid="agents-panel-row"
>
  <span class="flex min-w-0 flex-1 flex-col gap-0.5 py-0.5">
    <span class="flex min-w-0 items-center gap-2">
      <span class="min-w-0 flex-1 truncate text-[13px] leading-tight">{row.label}</span>
      <Chip tone={STATUS_TONE[row.status]}>{STATUS_WORD[row.status]}</Chip>
      {#if row.messageCount !== null}
        <Chip tone="count">{row.messageCount} msgs</Chip>
      {/if}
    </span>
    {#if row.activity}
      <span class="min-w-0 truncate text-sm leading-tight text-muted-foreground">{row.activity}</span>
    {/if}
  </span>

  {#snippet actions()}
    {#if row.logPath}
      <HoverActionButton
        label="Open log"
        tone="info"
        onclick={() => onopenlog?.(row.logPath ?? '')}
      >
        <FileText />
      </HoverActionButton>
    {:else}
      <!--
        A child agent record carries no path to a log, so this cannot work yet.
        It stays visible and disabled rather than disappearing, and the wrapper
        carries the reason because a disabled button never receives the hover a
        tooltip would need.
      -->
      <span title={NO_LOG_HINT} class="flex">
        <HoverActionButton label={`Open log — ${NO_LOG_HINT}`} disabled>
          <FileText />
        </HoverActionButton>
      </span>
    {/if}
  {/snippet}
</ListRow>
