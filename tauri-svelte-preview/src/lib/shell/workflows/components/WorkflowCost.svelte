<!--
  WorkflowCost.svelte — a run's Cost tab: what it has cost so far, its tokens
  and time, then one row per step attempt and model from the usage rows the
  agents' turns wrote. A cost the provider reported reads "reported"; one
  worked out from the rate table reads "est.". A step attempt that ran without a
  usage row reads "engine" for a command step, or its role's model and "not measured" for an
  agent. Earlier attempts' times come from the run's log.
-->
<script lang="ts">
  import * as Card from '$lib/components/ui/card/index.js';
  import { shortDuration, stepGlyph } from '../workflowBoard.ts';
  import { costLines, costSummary, money, tokenCount as count } from '../workflowStepInfo.ts';
  import type { WorkflowTimelineItem } from '../workflowTimeline.ts';
  import type { WorkflowRunRecord, WorkflowUsageRow } from '../workflowTypes.ts';
  import StepGlyphIcon from './StepGlyphIcon.svelte';

  interface Props {
    run: WorkflowRunRecord;
    /** Null while reading. */
    rows: WorkflowUsageRow[] | null;
    now: number;
    /** The run's log, for earlier attempts' times. */
    timeline: WorkflowTimelineItem[];
  }

  let { run, rows, now, timeline }: Props = $props();

  const ENDED = ['completed', 'cancelled', 'failed'];
  const tokensIn = $derived((rows ?? []).reduce((sum, row) => sum + row.inputTokens + row.cacheReadTokens + row.cacheWriteTokens, 0));
  const tokensOut = $derived((rows ?? []).reduce((sum, row) => sum + row.outputTokens, 0));
  const cacheRead = $derived((rows ?? []).reduce((sum, row) => sum + row.cacheReadTokens, 0));

  const percent = (part: number, whole: number): string => whole ? `${Math.round((100 * part) / whole)}%` : '—';

  const lines = $derived(costLines(run, rows, timeline, now));
  const summary = $derived(costSummary(rows, lines));

  function cost(row: WorkflowUsageRow | null, engine: boolean): string {
    if (!row) return engine ? '—' : 'not measured';
    // A turn's reported cost sits on its largest model's row; its other models' rows carry an estimate of 0.
    if (row.reportedCostMicros === null && row.estimatedCostMicros === 0) return "in the turn's reported cost";
    const reported = row.reportedCostMicros === null ? '' : `${money(row.reportedCostMicros)} reported`;
    const estimated = row.estimatedCostMicros ? `${money(row.estimatedCostMicros)} est.` : '';
    return [reported, estimated].filter(Boolean).join(' + ') || 'not priced';
  }
</script>

<div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto" data-testid="workflow-cost">
  <div class="grid grid-cols-3 gap-3">
    <Card.Root size="sm" class="gap-1 shadow-sm">
      <Card.Content class="flex flex-col gap-1">
        <span class="text-[13px] text-muted-foreground">Cost so far</span>
        <span class="text-(length:--text-heading) font-semibold tabular-nums">{summary.total}</span>
        {#if summary.note}<span class="text-[13px] text-muted-foreground">{summary.note}</span>{/if}
      </Card.Content>
    </Card.Root>
    <Card.Root size="sm" class="gap-1 shadow-sm">
      <Card.Content class="flex flex-col gap-1">
        <span class="text-[13px] text-muted-foreground">Tokens in / out</span>
        <span class="text-(length:--text-heading) font-semibold tabular-nums">{count(tokensIn)} / {count(tokensOut)}</span>
        <span class="text-[13px] text-muted-foreground">{percent(cacheRead, tokensIn)} of input read from cache</span>
      </Card.Content>
    </Card.Root>
    <Card.Root size="sm" class="gap-1 shadow-sm">
      <Card.Content class="flex flex-col gap-1">
        <span class="text-[13px] text-muted-foreground">Time</span>
        <span class="text-(length:--text-heading) font-semibold tabular-nums">{shortDuration((ENDED.includes(run.state) ? run.updatedAtMs : now) - run.createdAtMs)}</span>
      </Card.Content>
    </Card.Root>
  </div>

  <Card.Root size="sm" class="shadow-sm">
    <Card.Content>
      {#if rows === null}
        <p class="text-[13px] text-muted-foreground">Reading usage…</p>
      {:else}
        <table class="w-full text-[13px] tabular-nums">
          <thead class="text-left text-muted-foreground">
            <tr class="border-b">
              <th class="py-2 font-medium">Step · attempt</th><th class="font-medium">Model</th><th class="text-right font-medium">Time</th>
              <th class="text-right font-medium">In</th><th class="text-right font-medium">Out</th><th class="text-right font-medium">Cached</th>
              <th class="text-right font-medium">Cost</th><th class="pl-6 font-medium">Result</th>
            </tr>
          </thead>
          <tbody>
            {#each lines as line (line.key)}
              {@const node = run.nodes.find((candidate) => candidate.nodeId === line.step)}
              {@const latest = node?.attempt === line.attempt}
              {@const tokens = line.row ? line.row.inputTokens + line.row.cacheReadTokens + line.row.cacheWriteTokens : 0}
              <tr class="border-b last:border-b-0">
                <td class="py-2"><span class="font-semibold">{line.step}</span> <span class="text-muted-foreground">a{line.attempt}</span></td>
                <td>{line.model}</td>
                <td class="text-right">{line.time}</td>
                <td class="text-right">{line.row ? count(tokens) : '—'}</td>
                <td class="text-right">{line.row ? count(line.row.outputTokens) : '—'}</td>
                <td class="text-right">{line.row ? percent(line.row.cacheReadTokens, tokens) : '—'}</td>
                <td class="text-right">{cost(line.row, line.engine)}</td>
                <td class="pl-6">
                  {#if node && latest}
                    <span class="flex items-center gap-1.5"><StepGlyphIcon glyph={stepGlyph(node)} />{node.state}</span>
                  {:else}
                    <span class="text-muted-foreground">earlier attempt</span>
                  {/if}
                </td>
              </tr>
            {:else}
              <tr><td colspan="8" class="py-2 text-muted-foreground">No step has run yet.</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </Card.Content>
  </Card.Root>
  <p class="text-[13px] text-muted-foreground">
    Reported is the cost the agent's provider sends with its usage. Est. is the step's tokens times the rate table, used where the provider sends no cost. In counts all input tokens: uncached, cache reads and cache writes.
  </p>
</div>
