<!--
  RunConfigurationDialog.svelte — add or change one run configuration.

  A run configuration is one saved way to start something: a name, a command, a
  folder to run it in, and any environment variables it needs. This dialog is
  the only place any of that can be typed.

  WHAT IT DOES NOT DO
   - It never starts anything. Saving writes to storage and closes; running is
     the play button's job, and the pane's.
   - It never touches the terminal a configuration is already running in.
     Editing changes what the NEXT run does; the run going on right now keeps
     the command it was started with, because that is the command whose output
     is on screen.

  NO `$effect`: the draft is filled in by `openFor()`, which the host calls when
  it opens the dialog, and nothing else writes to it afterwards.

  Usage:
    <RunConfigurationDialog bind:this={editor} bind:open={editorOpen} />
    editor.openFor(null, folder)          // add a new one
    editor.openFor(row.definition, '')    // change an existing one
-->
<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import {
    addStack,
    commandWithEnv,
    describeStackProblem,
    formatEnvLines,
    readEnvLines,
    updateStack,
    type StackDefinition
  } from '$lib/shell/stacks/stackStore.svelte';

  interface Props {
    /** Whether the dialog is showing. */
    open?: boolean;
    /** Told the id of whatever was just saved, so a host can select it. */
    onSaved?: (stackId: string) => void;
  }
  let { open = $bindable(false), onSaved }: Props = $props();

  /** The configuration being changed, or `null` while adding a new one. */
  let editing = $state<StackDefinition | null>(null);
  let draftName = $state('');
  let draftScript = $state('');
  let draftFolder = $state('');
  let draftEnv = $state('');
  /** Set when a save was refused, so the reason sits under the fields. */
  let refusal = $state<string | null>(null);

  const env = $derived(readEnvLines(draftEnv));
  const problem = $derived(
    describeStackProblem({ name: draftName, script: draftScript, cwd: draftFolder })
  );
  const preview = $derived(commandWithEnv(draftScript, env.variables));
  const heading = $derived(editing ? 'Change this run configuration' : 'New run configuration');

  /**
   * Fill the dialog in and show it. `configuration` is the one being changed,
   * or `null` to add a new one; `defaultFolder` is what a new one starts with.
   */
  export function openFor(
    configuration: StackDefinition | null,
    defaultFolder: string = ''
  ): void {
    editing = configuration;
    draftName = configuration?.name ?? '';
    draftScript = configuration?.script ?? '';
    draftFolder = configuration?.cwd ?? defaultFolder;
    draftEnv = formatEnvLines(configuration?.env);
    refusal = null;
    open = true;
  }

  function save(): void {
    const draft = {
      name: draftName,
      script: draftScript,
      cwd: draftFolder,
      env: env.variables
    };
    const saved = editing ? updateStack(editing.id, draft) : addStack(draft);
    if (!saved) {
      // `addStack` and `updateStack` both say why they refused. Show it here
      // rather than leaving the user with a button that did nothing.
      refusal = describeStackProblem(draft) ?? 'That could not be saved.';
      return;
    }
    open = false;
    onSaved?.(saved.id);
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content
    class="w-[min(580px,calc(100vw-3rem))] sm:max-w-none max-h-[calc(100vh-3rem)] gap-0
           overflow-y-auto rounded-lg bg-background p-0 text-foreground ring-border
           shadow-[var(--shadow-lg)]"
  >
    <Dialog.Header class="gap-1 border-b px-5 pt-5 pb-4">
      <Dialog.Title class="text-[14px] leading-[1.4] font-semibold">{heading}</Dialog.Title>
      <Dialog.Description class="text-[12px] leading-[1.5] text-muted-foreground">
        One saved way to start something. Running it opens a terminal in the folder below and runs
        the command in it.
      </Dialog.Description>
    </Dialog.Header>

    <div class="flex flex-col gap-5 px-5 py-4">
      <section class="flex flex-col gap-2">
        <label for="run-config-name" class="text-[13px] leading-[1.3] font-medium">Name</label>
        <Input
          id="run-config-name"
          bind:value={draftName}
          placeholder="Web"
          autocomplete="off"
          spellcheck={false}
          class="h-8 text-[13px]"
        />
        <p class="text-[12px] leading-[1.5] text-[var(--color-text-3)]">
          What it is called in the run list and on the session it opens.
        </p>
      </section>

      <section class="flex flex-col gap-2">
        <label for="run-config-command" class="text-[13px] leading-[1.3] font-medium">Command</label>
        <Input
          id="run-config-command"
          bind:value={draftScript}
          placeholder="pnpm dev"
          autocomplete="off"
          spellcheck={false}
          class="h-8 font-mono text-[13px]"
        />
        <p class="text-[12px] leading-[1.5] text-[var(--color-text-3)]">
          Exactly what you would type in a terminal.
        </p>
      </section>

      <section class="flex flex-col gap-2">
        <label for="run-config-folder" class="text-[13px] leading-[1.3] font-medium">Folder</label>
        <Input
          id="run-config-folder"
          bind:value={draftFolder}
          placeholder="/Users/you/dev/some-project"
          autocomplete="off"
          spellcheck={false}
          class="h-8 font-mono text-[13px]"
        />
        <p class="text-[12px] leading-[1.5] text-[var(--color-text-3)]">
          The command runs here. This starts as the project folder; point it at a folder inside the
          project when the command belongs to one part of it.
        </p>
      </section>

      <section class="flex flex-col gap-2">
        <label for="run-config-env" class="text-[13px] leading-[1.3] font-medium">
          Environment variables
        </label>
        <textarea
          id="run-config-env"
          bind:value={draftEnv}
          rows="4"
          autocomplete="off"
          spellcheck="false"
          placeholder={'NODE_ENV=development\nPORT=5173'}
          class="w-full resize-y rounded-md border border-[var(--color-border)]
                 bg-[var(--color-bg)] px-2 py-1.5 font-mono text-[13px] leading-[1.5]
                 text-[var(--color-text)] outline-none focus-visible:border-[var(--color-accent)]"
        ></textarea>
        <p class="text-[12px] leading-[1.5] text-[var(--color-text-3)]">
          One <span class="font-mono">NAME=value</span> per line. Leave it empty if the command
          needs none. Quotes are optional — a value with spaces in it is quoted for you.
        </p>
        {#if env.unreadable.length > 0}
          <p class="flex items-start gap-1.5 text-[12px] leading-[1.5] text-[var(--color-attention)]">
            <TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
            <span>
              {env.unreadable.length === 1 ? 'This line is' : 'These lines are'} not in
              <span class="font-mono">NAME=value</span> form and will not be used:
              <span class="font-mono">{env.unreadable.join(' · ')}</span>
            </span>
          </p>
        {/if}
      </section>

      <section class="flex flex-col gap-1.5">
        <span class="text-[13px] leading-[1.3] font-medium">What will run</span>
        <code
          class="rounded-md bg-[var(--color-elevated)] px-2 py-1.5 font-mono text-[12px]
                 leading-[1.5] break-all text-[var(--color-text-2)]"
        >
          {preview || 'Type a command to see this.'}
        </code>
      </section>

      {#if problem || refusal}
        <p class="flex items-start gap-1.5 text-[12px] leading-[1.5] text-destructive">
          <TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
          <span>{problem ?? refusal}</span>
        </p>
      {/if}
    </div>

    <Dialog.Footer class="gap-2 border-t px-5 py-3">
      <Button variant="ghost" size="sm" class="text-[13px]" onclick={() => (open = false)}>
        Cancel
      </Button>
      <Button size="sm" class="text-[13px]" disabled={problem !== null} onclick={save}>
        {editing ? 'Save changes' : 'Save it'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
