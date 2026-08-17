<!--
  SettingsDialog.svelte — the /next shell's settings screen.

  Same settings as before, same wording, same store. What changed is the shape:
  it was a 560px dialog with four tabs, and it is now a full-window screen —
  a way back to the app in the top-left, a search field that filters the rows
  as you type, a grouped list of sections down the left, and cards of rows on
  the right. That is the layout every desktop app of this kind uses, and it is
  the only one that stays readable as the list of settings grows.

  Everything is built from the components in `$lib/components/ui`, coloured
  from the /next tokens. No native `<select>` anywhere; no setting was added,
  renamed or dropped in the move.

  Why the file is still called `SettingsDialog` and still takes `open`: the
  host that mounts it (`SettingsHost.svelte`) and the shell above it both
  address it that way, and neither is this lane's to change.

  Nothing here touches the backend except the C# language-server switch, which
  has always asked the app to start or stop the server.

  Usage:
    <SettingsDialog bind:open />
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Palette from '@lucide/svelte/icons/palette';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Search from '@lucide/svelte/icons/search';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import SquareTerminal from '@lucide/svelte/icons/square-terminal';
  import Type from '@lucide/svelte/icons/type';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import SettingsSelect from '$lib/shell/components/SettingsSelect.svelte';
  import { Slider } from '$lib/components/ui/slider/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import {
    PROBLEMS_LOCATIONS,
    PROBLEMS_LOCATION_LABELS,
    resetSettings,
    settings,
    type ProblemsLocation,
    type SettingsSection
  } from '$lib/settingsStore.svelte';
  import { setCsharpLanguageServerEnabled } from '$lib/shell/editor/sourceIntelligence';
  import { DEFAULT_THEME_ID } from '$lib/shell/themes/themeRegistry';
  import { apply as applyTheme, themeChoices } from '$lib/shell/themes/themeService';

  interface Props {
    /** Whether the settings screen is showing. */
    open?: boolean;
    /**
     * The user moved the Problems list. The setting is already written by the
     * time this runs — this is only so the shell can close or reopen the strip
     * along the bottom straight away, rather than on the next launch.
     */
    onProblemsLocationChange?: (location: ProblemsLocation) => void;
  }

  let { open = $bindable(false), onProblemsLocationChange }: Props = $props();

  // ── Option lists ────────────────────────────────────────────────────────
  /** The themes the app actually ships, straight from the registry. */
  const themeItems = themeChoices();

  const fontFamilyItems = [
    { value: 'Google Sans Mono', label: 'Google Sans Mono' },
    { value: 'SF Mono', label: 'SF Mono' },
    { value: 'JetBrains Mono', label: 'JetBrains Mono' },
    { value: 'Fira Code', label: 'Fira Code' },
    { value: 'Menlo', label: 'Menlo' },
    { value: 'Monaco', label: 'Monaco' },
    { value: 'Consolas', label: 'Consolas' }
  ];

  const terminalThemeItems = [
    { value: 'dracula', label: 'Dracula' },
    { value: 'houston', label: 'Houston' },
    { value: 'solarized-dark', label: 'Solarized Dark' },
    { value: 'one-dark', label: 'One Dark' }
  ];

  // Mirrors the SourceTerminalApp options in +page.svelte.
  const terminalAppItems = [
    { value: 'Warp', label: 'Warp' },
    { value: 'Terminal', label: 'Terminal' },
    { value: 'iTerm', label: 'iTerm' },
    { value: 'iTerm2', label: 'iTerm2' },
    { value: 'Ghostty', label: 'Ghostty' },
    { value: 'WezTerm', label: 'WezTerm' },
    { value: 'Alacritty', label: 'Alacritty' }
  ];

  const problemsLocationItems = PROBLEMS_LOCATIONS.map((value) => ({
    value,
    label: PROBLEMS_LOCATION_LABELS[value]
  }));

  // ── The screen's own map ────────────────────────────────────────────────
  /**
   * Sections in the order the left-hand list shows them, under the group each
   * one belongs to. The groups are labels only — nothing collapses.
   */
  const sections = [
    { id: 'appearance', group: 'Interface', label: 'Appearance', icon: Palette },
    { id: 'editor', group: 'Workspace', label: 'Editor', icon: Type },
    { id: 'terminal', group: 'Workspace', label: 'Terminal', icon: SquareTerminal },
    { id: 'general', group: 'Application', label: 'General', icon: SlidersHorizontal }
  ];

  /**
   * Every row on the screen, once, so that search has something to read and
   * the markup below has one source for its wording. `card` groups rows into
   * the boxes within a section; `keywords` catches the words someone is likely
   * to type that the row does not itself say.
   */
  const rows = [
    {
      id: 'theme',
      section: 'appearance',
      card: 'Colors',
      title: 'Theme',
      description: 'Overall color scheme for the app.',
      keywords: 'dark light palette houston'
    },
    {
      id: 'app-font-size',
      section: 'appearance',
      card: 'Text',
      title: 'Interface font size',
      description: 'Base size for chrome, labels and menus.',
      keywords: 'type scale bigger smaller'
    },
    {
      id: 'editor-font-family',
      section: 'editor',
      card: 'Text',
      title: 'Font family',
      description: 'Typeface used in the code editor.',
      keywords: 'monospace typeface'
    },
    {
      id: 'editor-font-size',
      section: 'editor',
      card: 'Text',
      title: 'Font size',
      description: 'Editor text size in pixels.',
      keywords: 'type scale'
    },
    {
      id: 'editor-line-height',
      section: 'editor',
      card: 'Text',
      title: 'Line height',
      description: 'Absolute line height in pixels.',
      keywords: 'spacing leading'
    },
    {
      id: 'editor-ligatures',
      section: 'editor',
      card: 'Text',
      title: 'Font ligatures',
      description: 'Render combined glyphs like => and !==.',
      keywords: 'glyphs arrows'
    },
    {
      id: 'terminal-font-family',
      section: 'terminal',
      card: 'Text',
      title: 'Font family',
      description: 'Typeface used in the embedded terminal.',
      keywords: 'monospace typeface'
    },
    {
      id: 'terminal-font-size',
      section: 'terminal',
      card: 'Text',
      title: 'Font size',
      description: 'Terminal text size in pixels.',
      keywords: 'type scale'
    },
    {
      id: 'terminal-line-height',
      section: 'terminal',
      card: 'Text',
      title: 'Line height',
      description: 'Line spacing multiplier (relative to font size).',
      keywords: 'spacing leading'
    },
    {
      id: 'terminal-theme',
      section: 'terminal',
      card: 'Colors',
      title: 'Color theme',
      description: 'Palette applied to terminal output.',
      keywords: 'dracula solarized one dark'
    },
    {
      id: 'terminal-cursor-blink',
      section: 'terminal',
      card: 'Behaviour',
      title: 'Cursor blink',
      description: 'Pulse the terminal cursor when idle.',
      keywords: 'caret flash'
    },
    {
      id: 'terminal-app',
      section: 'general',
      card: 'Outside the app',
      title: 'External terminal app',
      description: 'Used when opening a folder in a terminal.',
      keywords: 'warp iterm ghostty wezterm alacritty'
    },
    {
      id: 'problems-location',
      section: 'general',
      card: 'Panels',
      title: 'Where the problems list sits',
      description: 'The list of mistakes the language server has found in this project.',
      keywords: 'errors warnings bottom right hidden panel'
    },
    {
      id: 'csharp-language-server',
      section: 'general',
      card: 'Language support',
      title: 'C# language server',
      description:
        'Off saves about 800MB of memory. Reference counts and project-wide search keep working. What you lose is the squiggles under mistakes, and the precision of go-to-definition when a name is used in more than one place.',
      keywords: 'roslyn omnisharp intellisense memory'
    }
  ];

  let activeSection = $state('appearance');
  let query = $state('');
  let searchField = $state<HTMLElement | null>(null);

  /** Rows the current search leaves visible. An empty search leaves them all. */
  const matches = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const visible = new Set<string>();
    for (const row of rows) {
      if (!needle) {
        visible.add(row.id);
        continue;
      }
      const sectionLabel = sections.find((section) => section.id === row.section)?.label ?? '';
      const haystack =
        `${row.title} ${row.description} ${row.keywords} ${row.card} ${sectionLabel}`.toLowerCase();
      if (haystack.includes(needle)) visible.add(row.id);
    }
    return visible;
  });

  const searching = $derived(query.trim().length > 0);

  /** Sections with at least one row left after the search. */
  const shownSections = $derived(
    sections.filter((section) =>
      rows.some((row) => row.section === section.id && matches.has(row.id))
    )
  );

  /**
   * Which section the content pane shows. A search that empties the chosen
   * section moves to the first one that still has something in it, so the pane
   * is never blank while results exist elsewhere.
   */
  const shownSection = $derived(
    shownSections.find((section) => section.id === activeSection) ?? shownSections[0] ?? null
  );

  /** The cards of the shown section, in the order their rows first appear. */
  const shownCards = $derived.by(() => {
    if (!shownSection) return [] as { name: string; rowIds: string[] }[];
    const cards: { name: string; rowIds: string[] }[] = [];
    for (const row of rows) {
      if (row.section !== shownSection.id || !matches.has(row.id)) continue;
      const existing = cards.find((card) => card.name === row.card);
      if (existing) existing.rowIds.push(row.id);
      else cards.push({ name: row.card, rowIds: [row.id] });
    }
    return cards;
  });

  function rowMeta(id: string) {
    // Every id passed below is one of the entries above, so this cannot miss.
    return rows.find((row) => row.id === id)!;
  }

  // Local UI-only toggles (no store field yet) — kept here so the switches are
  // functional in isolation. Application is a later step.
  let editorLigatures = $state(false);
  let terminalCursorBlink = $state(true);

  /**
   * What the app said the last time the C# language server was switched. Shown
   * as-is under the switch: the backend answers in whole sentences, including
   * the one that says this build of the app cannot do it at all.
   */
  let csharpLanguageServerNote = $state<string | null>(null);
  /** False once the app has said it cannot switch the server. The control is
   * then switched off rather than left looking live and doing nothing. */
  let csharpLanguageServerSupported = $state(true);

  function moveProblems(location: ProblemsLocation): void {
    settings.panels.problemsLocation = location;
    onProblemsLocationChange?.(location);
  }

  async function switchCsharpLanguageServer(enabled: boolean): Promise<void> {
    const before = settings.intelligence.csharpLanguageServer;
    settings.intelligence.csharpLanguageServer = enabled;
    const result = await setCsharpLanguageServerEnabled(enabled);
    csharpLanguageServerSupported = result.supported;
    csharpLanguageServerNote = result.message;
    // An app that cannot do this leaves the server exactly as it was, so the
    // setting has to go back to saying so. Otherwise the switch reads "off"
    // over a server that is still running — and since the control is disabled
    // from here on, there would be no way to put it right.
    if (!result.supported) settings.intelligence.csharpLanguageServer = before;
  }

  /** The label to show on a closed dropdown, given what is selected. */
  function labelFor(items: { value: string; label: string }[], value: string): string {
    return items.find((item) => item.value === value)?.label ?? value;
  }

  /**
   * The theme this screen should show as chosen.
   *
   * Every install made before themes existed has `dark` stored, which is not one
   * of the themes the app ships. The screen renders the shipped theme in that
   * case, so this says so rather than naming a theme nobody can see.
   */
  const shownThemeId = $derived(
    themeItems.some((item) => item.value === settings.appearance.themeId)
      ? settings.appearance.themeId
      : DEFAULT_THEME_ID
  );

  function resetShownSection(): void {
    if (!shownSection) return;
    resetSettings(shownSection.id as SettingsSection);
    // Putting the settings back has to put the SCREEN back too, or the app says
    // one thing and shows another. A reset stores `dark`, which resolves to the
    // theme the app ships with.
    applyTheme(settings.appearance.themeId);
  }

  function backToApp(): void {
    open = false;
  }

  /**
   * Escape leaves the screen — unless something inside it has already used the
   * key, which is what closing an open dropdown looks like from here.
   */
  function onWindowKeyDown(event: KeyboardEvent): void {
    if (!open || event.key !== 'Escape' || event.defaultPrevented) return;
    event.preventDefault();
    backToApp();
  }

  // Opening the screen puts the caret in the search field: it is the fastest
  // way to reach a setting, and it costs a person nothing to ignore.
  $effect(() => {
    if (open) searchField?.focus();
  });
</script>

<svelte:window onkeydown={onWindowKeyDown} />

<!--
  One row of the screen: what the setting is, on the left; the control, on the
  right. Rows that the search has filtered out are simply not rendered.
-->
{#snippet settingRow(id: string, control: Snippet, wide = false)}
  {@const meta = rowMeta(id)}
  <div
    class="border-border/60 flex gap-6 border-b px-4 py-3 last:border-b-0 {wide
      ? 'flex-col items-stretch'
      : 'items-center justify-between'}"
    data-setting={id}
  >
    <div class="flex min-w-0 flex-col gap-1">
      <span class="text-[13px] leading-[1.3] font-medium">{meta.title}</span>
      <span class="text-[12px] leading-[1.45] text-[var(--color-text-3)]">{meta.description}</span>
    </div>
    <div class={wide ? 'w-full' : 'flex w-[220px] shrink-0 justify-end'}>
      {@render control()}
    </div>
  </div>
{/snippet}

<!-- A slider with the number it is currently on, so the value is not a guess. -->
{#snippet sliderReading(value: number, unit: string)}
  <span class="text-muted-foreground w-12 shrink-0 text-right text-[12px] tabular-nums">
    {value}{unit}
  </span>
{/snippet}

{#if open}
  <div class="settings-screen" role="dialog" aria-modal="true" aria-label="Settings">
    <header class="border-border/70 flex items-center gap-3 border-b px-4 py-2.5">
      <Button variant="ghost" size="sm" onclick={backToApp} class="gap-1.5 px-2">
        <ArrowLeft aria-hidden="true" />
        Back to app
      </Button>
      <span class="text-[13px] font-semibold">Settings</span>
      <div class="relative ml-auto w-[280px]">
        <Search
          aria-hidden="true"
          class="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2"
        />
        <Input
          bind:ref={searchField}
          bind:value={query}
          type="search"
          placeholder="Search settings"
          aria-label="Search settings"
          class="h-7 pl-7"
        />
      </div>
    </header>

    <div class="flex min-h-0 flex-1">
      <nav class="border-border/70 w-[208px] shrink-0 overflow-y-auto border-r px-2 py-3">
        {#each shownSections as section, index (section.id)}
          {@const newGroup = index === 0 || shownSections[index - 1].group !== section.group}
          {#if newGroup}
            <div
              class="text-muted-foreground px-2 pb-1 text-[11px] font-medium tracking-wide uppercase {index >
              0
                ? 'pt-4'
                : ''}"
            >
              {section.group}
            </div>
          {/if}
          {@const Icon = section.icon}
          <button
            type="button"
            class="focus-visible:ring-ring/50 mb-0.5 flex h-8 w-full items-center gap-2 rounded-lg px-2 text-left text-[13px] transition-colors outline-none focus-visible:ring-3 {shownSection?.id ===
            section.id
              ? 'bg-secondary text-foreground'
              : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground'}"
            aria-current={shownSection?.id === section.id ? 'page' : undefined}
            onclick={() => (activeSection = section.id)}
          >
            <Icon aria-hidden="true" class="size-4" />
            {section.label}
          </button>
        {/each}
      </nav>

      <main class="min-w-0 flex-1 overflow-y-auto px-6 py-5">
        {#if !shownSection}
          <p class="text-muted-foreground text-[13px]">
            Nothing matches “{query.trim()}”.
          </p>
        {:else}
          <div class="mx-auto flex w-full max-w-[720px] flex-col gap-5">
            <div class="flex items-center justify-between gap-4">
              <div class="flex flex-col gap-1">
                <h2 class="text-[15px] leading-[1.3] font-semibold">{shownSection.label}</h2>
                {#if searching}
                  <p class="text-muted-foreground text-[12px]">
                    Showing the rows that match “{query.trim()}”.
                  </p>
                {/if}
              </div>
              <Button variant="ghost" size="sm" onclick={resetShownSection} class="gap-1.5">
                <RotateCcw aria-hidden="true" />
                Reset section
              </Button>
            </div>

            {#each shownCards as card (card.name)}
              <section class="flex flex-col gap-2">
                <h3 class="text-muted-foreground px-1 text-[12px] font-medium">{card.name}</h3>
                <div class="border-border/70 bg-card rounded-xl border">
                  {#each card.rowIds as id (id)}
                    {#if id === 'theme'}
                      {@render settingRow(id, themeControl)}
                    {:else if id === 'app-font-size'}
                      {@render settingRow(id, appFontSizeControl, true)}
                    {:else if id === 'editor-font-family'}
                      {@render settingRow(id, editorFontFamilyControl)}
                    {:else if id === 'editor-font-size'}
                      {@render settingRow(id, editorFontSizeControl, true)}
                    {:else if id === 'editor-line-height'}
                      {@render settingRow(id, editorLineHeightControl, true)}
                    {:else if id === 'editor-ligatures'}
                      {@render settingRow(id, editorLigaturesControl)}
                    {:else if id === 'terminal-font-family'}
                      {@render settingRow(id, terminalFontFamilyControl)}
                    {:else if id === 'terminal-font-size'}
                      {@render settingRow(id, terminalFontSizeControl, true)}
                    {:else if id === 'terminal-line-height'}
                      {@render settingRow(id, terminalLineHeightControl, true)}
                    {:else if id === 'terminal-theme'}
                      {@render settingRow(id, terminalThemeControl)}
                    {:else if id === 'terminal-cursor-blink'}
                      {@render settingRow(id, terminalCursorBlinkControl)}
                    {:else if id === 'terminal-app'}
                      {@render settingRow(id, terminalAppControl)}
                    {:else if id === 'problems-location'}
                      {@render settingRow(id, problemsLocationControl)}
                    {:else if id === 'csharp-language-server'}
                      {@render settingRow(id, csharpLanguageServerControl)}
                    {/if}
                  {/each}
                </div>
              </section>
            {/each}
          </div>
        {/if}
      </main>
    </div>
  </div>
{/if}

<!-- ── The controls themselves ──────────────────────────────────────────── -->

{#snippet themeControl()}
  <!-- Not `bind:value`: applying a theme writes the setting itself, and a
       binding would fight it. -->
  <SettingsSelect
    items={themeItems}
    value={shownThemeId}
    ariaLabel="Theme"
    onChange={(value) => applyTheme(value)}
  />
{/snippet}

{#snippet appFontSizeControl()}
  <div class="flex items-center gap-3">
    <Slider
      type="single"
      bind:value={settings.appearance.appFontSize}
      min={11}
      max={18}
      step={1}
      aria-label="Interface font size"
    />
    {@render sliderReading(settings.appearance.appFontSize, 'px')}
  </div>
{/snippet}

{#snippet editorFontFamilyControl()}
  <SettingsSelect
    items={fontFamilyItems}
    value={settings.editor.fontFamily}
    ariaLabel="Editor font family"
    onChange={(value) => (settings.editor.fontFamily = value)}
  />
{/snippet}

{#snippet editorFontSizeControl()}
  <div class="flex items-center gap-3">
    <Slider
      type="single"
      bind:value={settings.editor.fontSize}
      min={10}
      max={24}
      step={1}
      aria-label="Editor font size"
    />
    {@render sliderReading(settings.editor.fontSize, 'px')}
  </div>
{/snippet}

{#snippet editorLineHeightControl()}
  <div class="flex items-center gap-3">
    <Slider
      type="single"
      bind:value={settings.editor.lineHeight}
      min={14}
      max={36}
      step={1}
      aria-label="Editor line height"
    />
    {@render sliderReading(settings.editor.lineHeight, 'px')}
  </div>
{/snippet}

{#snippet editorLigaturesControl()}
  <Switch bind:checked={editorLigatures} aria-label="Font ligatures" />
{/snippet}

{#snippet terminalFontFamilyControl()}
  <SettingsSelect
    items={fontFamilyItems}
    value={settings.terminal.fontFamily}
    ariaLabel="Terminal font family"
    onChange={(value) => (settings.terminal.fontFamily = value)}
  />
{/snippet}

{#snippet terminalFontSizeControl()}
  <div class="flex items-center gap-3">
    <Slider
      type="single"
      bind:value={settings.terminal.fontSize}
      min={10}
      max={24}
      step={1}
      aria-label="Terminal font size"
    />
    {@render sliderReading(settings.terminal.fontSize, 'px')}
  </div>
{/snippet}

{#snippet terminalLineHeightControl()}
  <div class="flex items-center gap-3">
    <Slider
      type="single"
      bind:value={settings.terminal.lineHeight}
      min={1}
      max={2}
      step={0.1}
      aria-label="Terminal line height"
    />
    {@render sliderReading(Math.round(settings.terminal.lineHeight * 10) / 10, '×')}
  </div>
{/snippet}

{#snippet terminalThemeControl()}
  <SettingsSelect
    items={terminalThemeItems}
    value={settings.terminal.theme}
    ariaLabel="Terminal color theme"
    onChange={(value) => (settings.terminal.theme = value)}
  />
{/snippet}

{#snippet terminalCursorBlinkControl()}
  <Switch bind:checked={terminalCursorBlink} aria-label="Cursor blink" />
{/snippet}

{#snippet terminalAppControl()}
  <SettingsSelect
    items={terminalAppItems}
    value={settings.general.terminalApp}
    ariaLabel="External terminal app"
    onChange={(value) => (settings.general.terminalApp = value)}
  />
{/snippet}

{#snippet problemsLocationControl()}
  <!-- Three choices, which would suit a segmented control if the labels were
       short; they are whole phrases ("In the strip along the bottom"), so this
       stays a dropdown. Not `bind:value`: the shell has to be told as well as
       the store, so the strip along the bottom opens or closes now rather than
       at the next launch. -->
  <SettingsSelect
    items={problemsLocationItems}
    value={settings.panels.problemsLocation}
    ariaLabel="Where the problems list sits"
    onChange={(value) => moveProblems(value as ProblemsLocation)}
  />
{/snippet}

{#snippet csharpLanguageServerControl()}
  <div class="flex flex-col items-end gap-1.5">
    <Switch
      checked={settings.intelligence.csharpLanguageServer}
      disabled={!csharpLanguageServerSupported}
      onCheckedChange={(checked) => void switchCsharpLanguageServer(checked)}
      aria-label="C# language server"
    />
    {#if csharpLanguageServerNote}
      <p class="text-[12px] leading-[1.4] text-[var(--color-text-2)]">{csharpLanguageServerNote}</p>
    {/if}
  </div>
{/snippet}

<style>
  /* A screen, not a sheet: it covers the window and the app waits behind it. */
  .settings-screen {
    position: fixed;
    inset: 0;
    /* Above every strip and rail of the shell: this screen replaces the app
       while it is up, and a status bar showing through the bottom of it reads
       as a rendering fault. */
    z-index: 200;
    display: flex;
    flex-direction: column;
    background: var(--color-bg);
    color: var(--color-text);
  }
</style>
