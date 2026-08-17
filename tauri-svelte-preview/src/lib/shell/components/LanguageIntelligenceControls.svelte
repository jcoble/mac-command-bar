<script lang="ts">
  /**
   * LanguageIntelligenceControls.svelte — the project's language-server switch,
   * at the head of the centre pane's pill group.
   *
   * It has been elsewhere, and each move was a lesson. In a strip of its own
   * along the top of the shell it cost a row of window height to say one word.
   * Among the file tabs it read as a property of whichever file was showing. It
   * belongs with the pills: they are the controls for the middle of the window,
   * and this is one of them. The pills carry icons rather than words now, so the
   * whole group fits a narrow centre pane with the file tabs left whole.
   *
   * TWO INDEPENDENT THINGS, and the whole design rests on keeping them apart:
   *
   *   the knob's POSITION says what the user asked for.
   *   the track's COLOUR says what is actually true.
   *
   * So a switch that has been turned on while the server has not come up sits to
   * the right in the warning colour — asked for, not delivered — rather than
   * claiming green. Green means the server is up and answering; grey means off.
   * The badge says which language the switch is currently about.
   *
   * It owns nothing. Everything shown here is published by `EditorPanel.svelte`,
   * which asks the desktop app for the status, listens for pushed updates and
   * starts or stops the server. Before a project resolves it still renders, as
   * an inert switch that says so.
   */
  import Zap from '@lucide/svelte/icons/zap';
  import ZapOff from '@lucide/svelte/icons/zap-off';

  import {
    languageShortLabel,
    readLanguageServerState
  } from './editor/languageServerStatus.ts';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import { languageIntelligenceLabel } from '$lib/shell/editor/languageIntelligenceMode';
  import {
    languageIntelligenceBar,
    requestLanguageIntelligence
  } from '$lib/shell/editor/languageIntelligenceBar.svelte';

  /**
   * Which of the three colours the control wears.
   *
   * `running` is deliberately narrow: only a server that reports itself ready
   * can answer a question, so only that one earns green. Starting and indexing
   * are servers on their way up, and a build that reports no state at all is a
   * build that cannot tell us — all of them are "asked for, not there yet".
   */
  const tone = $derived.by((): 'running' | 'waiting' | 'off' => {
    if (!languageIntelligenceBar.fullMode) return 'off';
    return readLanguageServerState(languageIntelligenceBar.status) === 'ready'
      ? 'running'
      : 'waiting';
  });

  const shortLanguage = $derived(languageShortLabel(languageIntelligenceBar.language));

  /** The sentence on hover: the panel's own wording, plus the state word. With
   * no project the control says so itself rather than relying on a sentence the
   * panel may not have published yet — a disabled control with no reason on it
   * is just a broken one. */
  const hoverText = $derived(
    languageIntelligenceBar.hasProject
      ? [languageIntelligenceBar.title, `Language intelligence: ${languageIntelligenceLabel(languageIntelligenceBar.fullMode)}`]
          .filter((part) => part.length > 0)
          .join(' · ')
      : 'No project is open, so there is no language server to switch on.'
  );
</script>

<!-- Always rendered. A group that is three capsules wide sometimes and four at
     others reads as a bug, and the pills draw long before a project resolves;
     with nothing to act on it states that instead of vanishing. -->
<div
  class="language-switch"
  class:no-project={!languageIntelligenceBar.hasProject}
  data-tone={tone}
  title={hoverText}
>
  <!-- The glyph is the on-state mark: a bolt while the switch is on, a struck
       bolt while it is off. It takes its colour from the tone, so it says the
       same thing as the track without repeating the words. -->
  {#if languageIntelligenceBar.fullMode}
    <Zap class="size-3.5" strokeWidth={1.8} aria-hidden="true" />
  {:else}
    <ZapOff class="size-3.5" strokeWidth={1.8} aria-hidden="true" />
  {/if}

  <!-- Which language the switch is about, in the two or three characters the
       control has room for. With no file open there is no language, and the
       badge shows a dash rather than a plausible-looking guess. -->
  <span class="language-badge">{shortLanguage}</span>

  <Switch
    size="sm"
    checked={languageIntelligenceBar.fullMode}
    disabled={languageIntelligenceBar.busy || !languageIntelligenceBar.hasProject}
    onCheckedChange={(checked) => requestLanguageIntelligence(checked)}
    aria-label="Language intelligence"
  />
</div>

<style>
  /* One capsule, the same height and shape as the icon pills it leads, so the
   * row reads as a single group rather than a control parked beside one. */
  .language-switch {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: 0 0 auto;
    min-width: max-content;
    height: 30px;
    padding: 0 10px;
    border-radius: var(--radius-pill);
    background: var(--pill-surface);
    box-shadow: var(--shadow-sm);
    color: var(--color-text-2);
    user-select: none;
  }

  .language-badge {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
    line-height: 1;
    white-space: nowrap;
    /* Room for the widest three-character badge, so the group keeps its width
       as the open file changes and the row stops twitching. */
    min-width: 24px;
    text-align: center;
  }

  /* Nothing to switch on yet. This is NOT a fourth state: the colour is the same
   * neutral Off wears, because off is what is true. What changes is only that
   * the control refuses the pointer, and the title says why. */
  .language-switch.no-project {
    cursor: not-allowed;
  }

  /* The switch is a <button>, and a button carries the browser's own padding —
   * 1px by 6px in this engine. On a track only 24 by 14 that padding is most of
   * the box: it squashed the knob out of round and parked it in the MIDDLE of
   * the track at rest, which is the one thing this control cannot afford, since
   * position is how it says what was asked for. With the padding gone the kit's
   * own arithmetic lands: a 12px knob inside a 1px border, travelling the
   * 10px it translates by, flush at both ends. */
  .language-switch :global([data-slot='switch']) {
    padding: 0;
  }

  /* ── The three states ────────────────────────────────────────────────────
     Colour is the TRUTH channel. The knob has already moved by the time any of
     these apply; what changes here is only what the control claims. */

  /* Up and answering. */
  .language-switch[data-tone='running'] {
    color: var(--color-good);
  }

  .language-switch[data-tone='running'] :global([data-slot='switch']) {
    background: var(--switch-track-running);
  }

  /* Asked for, not there yet: starting, reading the project, or a build that
     cannot say. The switch is to the right and the colour is a warning. */
  .language-switch[data-tone='waiting'] {
    color: var(--color-attention);
  }

  .language-switch[data-tone='waiting'] :global([data-slot='switch']) {
    background: var(--switch-track-waiting);
  }

  /* Off: read mode, nothing started, nothing claimed. */
  .language-switch[data-tone='off'] {
    color: var(--color-idle);
  }

  .language-switch[data-tone='off'] :global([data-slot='switch']) {
    background: var(--switch-track-off);
  }

  /* The knob is one colour in every state — it is the position that carries
   * meaning here, and a knob that changed colour too would say it twice. */
  .language-switch :global([data-slot='switch-thumb']) {
    background: var(--switch-knob);
  }
</style>
