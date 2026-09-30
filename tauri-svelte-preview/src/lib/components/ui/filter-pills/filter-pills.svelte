<!--
  filter-pills.svelte — a row of grouped, multi-select filter pills, in the
  manner of Spotify's Your Library filters.

  At rest the row shows one pill per group. Pressing a group opens it: the row
  shows an "All" pill, which clears that group and closes it, then the group's
  options, which toggle on and off. A press anywhere outside the row, or ten
  seconds with no press, closes the group and keeps its selection. A closed
  group with a selection shows the chosen labels joined ("Claude/Codex") as a
  filled pill. The X at the far left of the resting row clears every group.

  Plain config in, selected values per group id out. The caller owns what the
  values mean and whether they are saved. Needs only Tailwind and the shadcn
  colour slots, so it drops into any shadcn-svelte project.

  Usage:
    <FilterPills label="Filter tasks" groups={GROUPS} value={filters} onChange={(v) => (filters = v)} />
-->
<script lang="ts">
  interface FilterPillOption {
    value: string;
    label: string;
  }

  interface FilterPillGroup {
    id: string;
    label: string;
    options: readonly FilterPillOption[];
  }

  interface Props {
    groups: readonly FilterPillGroup[];
    /** Selected option values per group id; a missing or empty list means unfiltered. */
    value: Record<string, string[]>;
    onChange(value: Record<string, string[]>): void;
    /** Names the row for a screen reader. */
    label: string;
  }

  let { groups, value, onChange, label }: Props = $props();

  const AUTO_CLOSE_MS = 10_000;
  let openId = $state<string | null>(null);
  let timer: number | undefined;
  let row = $state<HTMLDivElement | null>(null);

  const openGroup = $derived(groups.find((group) => group.id === openId) ?? null);
  const anySelected = $derived(groups.some((group) => selected(group.id).length > 0));

  $effect(() => () => clearTimeout(timer));

  // While a group is open, a press outside the row closes it and keeps its
  // selection. The listener exists only while open; closing or unmounting removes it.
  $effect(() => {
    if (openId === null) return;
    const closeOnOutsidePress = (event: PointerEvent): void => {
      if (!row?.contains(event.target as Node)) close();
    };
    window.addEventListener('pointerdown', closeOnOutsidePress, true);
    return () => window.removeEventListener('pointerdown', closeOnOutsidePress, true);
  });

  function selected(groupId: string): string[] {
    return value[groupId] ?? [];
  }

  function restartTimer(): void {
    clearTimeout(timer);
    timer = window.setTimeout(close, AUTO_CLOSE_MS);
  }

  function open(groupId: string): void {
    openId = groupId;
    restartTimer();
  }

  function close(): void {
    clearTimeout(timer);
    openId = null;
  }

  function toggle(group: FilterPillGroup, optionValue: string): void {
    const current = selected(group.id);
    const next = group.options
      .map((option) => option.value)
      .filter((candidate) =>
        candidate === optionValue ? !current.includes(candidate) : current.includes(candidate)
      );
    onChange({ ...value, [group.id]: next });
    restartTimer();
  }

  function clearGroup(group: FilterPillGroup): void {
    onChange({ ...value, [group.id]: [] });
    close();
  }

  function clearAll(): void {
    onChange(Object.fromEntries(groups.map((group) => [group.id, []])));
    close();
  }

  function pillLabel(group: FilterPillGroup): string {
    const chosen = group.options.filter((option) => selected(group.id).includes(option.value));
    return chosen.length > 0 ? chosen.map((option) => option.label).join('/') : group.label;
  }

  const PILL =
    'inline-flex h-[28px] shrink-0 cursor-pointer items-center justify-center rounded-full border-0 ' +
    'text-[13px] whitespace-nowrap outline-none transition-[background-color,color,filter] ' +
    'focus-visible:ring-ring/50 focus-visible:ring-3';
  const REST = 'bg-secondary text-foreground hover:bg-accent/60 hover:text-foreground';
  const CHOSEN = 'bg-foreground text-background hover:brightness-125';
  const ICON = 'w-[28px] bg-secondary text-muted-foreground hover:bg-accent/60 hover:text-foreground';
</script>

{#snippet closeIcon()}
  <svg
    class="size-3.5"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M18 6 6 18M6 6l12 12" />
  </svg>
{/snippet}

<div bind:this={row} role="group" aria-label={label} class="flex flex-nowrap items-center gap-2 overflow-x-auto">
  {#if openGroup}
    {@const group = openGroup}
    {@const all = selected(group.id).length === 0}
    <button
      type="button"
      class="{PILL} px-3 {all ? CHOSEN : REST}"
      aria-pressed={all}
      onclick={() => clearGroup(group)}
    >All</button>
    {#each group.options as option (option.value)}
      {@const pressed = selected(group.id).includes(option.value)}
      <button
        type="button"
        class="{PILL} px-3 {pressed ? CHOSEN : REST}"
        aria-pressed={pressed}
        onclick={() => toggle(group, option.value)}
      >{option.label}</button>
    {/each}
  {:else}
    {#if anySelected}
      <button
        type="button"
        class="{PILL} {ICON}"
        aria-label="Clear all filters"
        onclick={clearAll}
      >{@render closeIcon()}</button>
    {/if}
    {#each groups as group (group.id)}
      <button
        type="button"
        class="{PILL} px-3 {selected(group.id).length > 0 ? CHOSEN : REST}"
        aria-expanded="false"
        onclick={() => open(group.id)}
      >{pillLabel(group)}</button>
    {/each}
  {/if}
</div>
