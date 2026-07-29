<!--
  SettingsDialog.svelte — the /next shell's settings screen.

  Same screen as before, same wording, same switches, same store: Appearance /
  Editor / Terminal / General, bound to `settingsStore`, with "Reset section"
  and "Done" at the bottom. What changed is what it is BUILT from — the shadcn
  components in `$lib/components/ui`, coloured entirely from the /next tokens,
  instead of the older shared widgets in `$lib/components`.

  Why a second file rather than an edit to `$lib/SettingsPanel.svelte`: that
  panel is also the old shell's settings screen, and the old shell must keep
  rendering exactly as it does today. So /next gets its own, and the old page
  keeps the one it has.

  Nothing here touches the backend.

  Usage:
    <SettingsDialog bind:open />
-->
<script lang="ts">
  import { Palette, RotateCcw, SlidersHorizontal, SquareTerminal, Type } from '@lucide/svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Slider } from '$lib/components/ui/slider/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import { resetSettings, settings, type SettingsSection } from '$lib/settingsStore.svelte';

  interface Props {
    /** Whether the settings dialog is open. */
    open?: boolean;
  }

  let { open = $bindable(false) }: Props = $props();

  // ── Option lists ────────────────────────────────────────────────────────
  const themeItems = [
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
    { value: 'houston', label: 'Houston' }
  ];

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

  const tabs = [
    { value: 'appearance', label: 'Appearance', icon: Palette },
    { value: 'editor', label: 'Editor', icon: Type },
    { value: 'terminal', label: 'Terminal', icon: SquareTerminal },
    { value: 'general', label: 'General', icon: SlidersHorizontal }
  ];

  let activeTab = $state('appearance');

  // Local UI-only toggles (no store field yet) — kept here so the switches are
  // functional in isolation. Application is a later step.
  let editorLigatures = $state(false);
  let terminalCursorBlink = $state(true);

  /** The label to show on a closed dropdown, given what is selected. */
  function labelFor(items: { value: string; label: string }[], value: string): string {
    return items.find((item) => item.value === value)?.label ?? value;
  }

  function resetActiveSection() {
    resetSettings(activeTab as SettingsSection);
  }
</script>

<Dialog.Root bind:open>
  <!-- Sized and coloured to match the dialog this screen has always had:
       560px wide, the shell backdrop, a hairline edge and a deep shadow. -->
  <Dialog.Content
    class="w-[min(560px,calc(100vw-3rem))] sm:max-w-none max-h-[calc(100vh-3rem)] gap-0 overflow-y-auto rounded-lg bg-background p-0 text-foreground ring-border shadow-[var(--shadow-lg)]"
  >
    <Dialog.Header class="gap-1 border-b px-5 pt-5 pb-4">
      <Dialog.Title class="text-[14px] leading-[1.4] font-semibold">Settings</Dialog.Title>
      <Dialog.Description class="text-[12px] leading-[1.5] text-muted-foreground">
        Tune appearance and tooling preferences.
      </Dialog.Description>
    </Dialog.Header>

    <!-- A steady minimum height so switching tabs does not make the dialog jump. -->
    <div class="min-h-[320px] px-5 py-4">
      <Tabs.Root bind:value={activeTab} class="gap-4">
        <Tabs.List class="w-full">
          {#each tabs as tab (tab.value)}
            {@const Icon = tab.icon}
            <Tabs.Trigger value={tab.value} class="text-[13px]">
              <Icon aria-hidden="true" />
              {tab.label}
            </Tabs.Trigger>
          {/each}
        </Tabs.List>

        <Tabs.Content value="appearance">
          <div class="flex flex-col gap-5 pt-1">
            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Theme</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Overall color scheme for the app.
                </span>
              </div>
              <div class="w-[200px] shrink-0">
                <Select.Root type="single" bind:value={settings.appearance.themeId}>
                  <Select.Trigger class="w-full">
                    {labelFor(themeItems, settings.appearance.themeId)}
                  </Select.Trigger>
                  <Select.Content>
                    {#each themeItems as item (item.value)}
                      <Select.Item value={item.value} label={item.label} />
                    {/each}
                  </Select.Content>
                </Select.Root>
              </div>
            </div>

            <div class="flex flex-col gap-3">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Interface font size</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Base size for chrome, labels and menus.
                </span>
              </div>
              <Slider
                type="single"
                bind:value={settings.appearance.appFontSize}
                min={11}
                max={18}
                step={1}
              />
            </div>
          </div>
        </Tabs.Content>

        <Tabs.Content value="editor">
          <div class="flex flex-col gap-5 pt-1">
            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Font family</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Typeface used in the code editor.
                </span>
              </div>
              <div class="w-[200px] shrink-0">
                <Select.Root type="single" bind:value={settings.editor.fontFamily}>
                  <Select.Trigger class="w-full">
                    {labelFor(fontFamilyItems, settings.editor.fontFamily)}
                  </Select.Trigger>
                  <Select.Content>
                    {#each fontFamilyItems as item (item.value)}
                      <Select.Item value={item.value} label={item.label} />
                    {/each}
                  </Select.Content>
                </Select.Root>
              </div>
            </div>

            <div class="flex flex-col gap-3">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Font size</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Editor text size in pixels.
                </span>
              </div>
              <Slider
                type="single"
                bind:value={settings.editor.fontSize}
                min={10}
                max={24}
                step={1}
              />
            </div>

            <div class="flex flex-col gap-3">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Line height</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Absolute line height in pixels.
                </span>
              </div>
              <Slider
                type="single"
                bind:value={settings.editor.lineHeight}
                min={14}
                max={36}
                step={1}
              />
            </div>

            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Font ligatures</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Render combined glyphs like {'=>'} and {'!=='}.
                </span>
              </div>
              <div class="flex shrink-0 justify-end">
                <Switch bind:checked={editorLigatures} aria-label="Font ligatures" />
              </div>
            </div>
          </div>
        </Tabs.Content>

        <Tabs.Content value="terminal">
          <div class="flex flex-col gap-5 pt-1">
            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Font family</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Typeface used in the embedded terminal.
                </span>
              </div>
              <div class="w-[200px] shrink-0">
                <Select.Root type="single" bind:value={settings.terminal.fontFamily}>
                  <Select.Trigger class="w-full">
                    {labelFor(fontFamilyItems, settings.terminal.fontFamily)}
                  </Select.Trigger>
                  <Select.Content>
                    {#each fontFamilyItems as item (item.value)}
                      <Select.Item value={item.value} label={item.label} />
                    {/each}
                  </Select.Content>
                </Select.Root>
              </div>
            </div>

            <div class="flex flex-col gap-3">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Font size</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Terminal text size in pixels.
                </span>
              </div>
              <Slider
                type="single"
                bind:value={settings.terminal.fontSize}
                min={10}
                max={24}
                step={1}
              />
            </div>

            <div class="flex flex-col gap-3">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Line height</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Line spacing multiplier (relative to font size).
                </span>
              </div>
              <Slider
                type="single"
                bind:value={settings.terminal.lineHeight}
                min={1}
                max={2}
                step={0.1}
              />
            </div>

            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Color theme</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Palette applied to terminal output.
                </span>
              </div>
              <div class="w-[200px] shrink-0">
                <Select.Root type="single" bind:value={settings.terminal.theme}>
                  <Select.Trigger class="w-full">
                    {labelFor(terminalThemeItems, settings.terminal.theme)}
                  </Select.Trigger>
                  <Select.Content>
                    {#each terminalThemeItems as item (item.value)}
                      <Select.Item value={item.value} label={item.label} />
                    {/each}
                  </Select.Content>
                </Select.Root>
              </div>
            </div>

            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">Cursor blink</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Pulse the terminal cursor when idle.
                </span>
              </div>
              <div class="flex shrink-0 justify-end">
                <Switch bind:checked={terminalCursorBlink} aria-label="Cursor blink" />
              </div>
            </div>
          </div>
        </Tabs.Content>

        <Tabs.Content value="general">
          <div class="flex flex-col gap-5 pt-1">
            <div class="flex items-center justify-between gap-5">
              <div class="flex min-w-0 flex-col gap-1">
                <span class="text-[14px] leading-[1.3] font-medium">External terminal app</span>
                <span class="text-[12px] leading-[1.4] text-[var(--color-text-3)]">
                  Used when opening a folder in a terminal.
                </span>
              </div>
              <div class="w-[200px] shrink-0">
                <Select.Root type="single" bind:value={settings.general.terminalApp}>
                  <Select.Trigger class="w-full">
                    {labelFor(terminalAppItems, settings.general.terminalApp)}
                  </Select.Trigger>
                  <Select.Content>
                    {#each terminalAppItems as item (item.value)}
                      <Select.Item value={item.value} label={item.label} />
                    {/each}
                  </Select.Content>
                </Select.Root>
              </div>
            </div>
          </div>
        </Tabs.Content>
      </Tabs.Root>
    </div>

    <Dialog.Footer class="mx-0 mb-0 rounded-b-lg border-t bg-transparent px-5 py-3">
      <Button variant="ghost" size="sm" onclick={resetActiveSection}>
        <RotateCcw aria-hidden="true" />
        Reset section
      </Button>
      <Button size="sm" onclick={() => (open = false)}>Done</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
