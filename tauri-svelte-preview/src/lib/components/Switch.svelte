<!--
  Switch.svelte — bits-ui v2 Switch wrapper
  Props:
    checked  — $bindable boolean
    label    — optional string label
    disabled — optional boolean

  Off track: --color-idle
  On  track: --color-accent
  Thumb:     near-white circle
  Transition: 130ms ease all
  Focus:     --focus-ring
-->
<script lang="ts">
  import { Switch } from 'bits-ui';

  interface Props {
    checked?: boolean;
    label?: string;
    disabled?: boolean;
  }

  let {
    checked = $bindable(false),
    label,
    disabled = false,
  }: Props = $props();

  // Stable id so the label span can name the role="switch" button via
  // aria-labelledby. A <label> does NOT associate with a <button>, so we
  // wire the name (and the click-to-toggle affordance) explicitly.
  const labelId = $props.id();

  function toggleFromLabel() {
    if (!disabled) checked = !checked;
  }
</script>

<div class="switch-wrapper" class:switch-wrapper--disabled={disabled}>
  {#if label}
    <!-- Clicking the label toggles the switch; the button keeps focus/role.
         aria-hidden keeps SRs from announcing the label twice (it already
         names the switch via aria-labelledby). -->
    <span
      class="switch-label"
      id={labelId}
      aria-hidden="true"
      onclick={toggleFromLabel}
    >{label}</span>
  {/if}

  <Switch.Root
    bind:checked
    {disabled}
    class="switch-root"
    aria-labelledby={label ? labelId : undefined}
  >
    <Switch.Thumb class="switch-thumb" />
  </Switch.Root>
</div>

<style>
  /* ── Wrapper — label + control in a row ─────────────── */
  .switch-wrapper {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    cursor: pointer;
    user-select: none;
  }

  .switch-wrapper--disabled {
    opacity: 0.4;
    cursor: not-allowed;
    pointer-events: none;
  }

  /* ── Label text ──────────────────────────────────────── */
  .switch-label {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    color: var(--color-text-2);
    cursor: pointer;
  }

  /* NOTE: .switch-root / .switch-thumb are forwarded to bits-ui-rendered
     elements (button + span), so every selector touching them must be
     :global(...). In particular the checked-slide rule below: a scoped
     left-hand side (.switch-root[data-state='checked']) never matches the
     unhashed bits button, so the thumb would never move. */

  /* ── Track (the Switch.Root button) ─────────────────── */
  :global(.switch-root) {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 36px;
    height: 20px;
    border-radius: var(--radius-pill);
    background-color: var(--color-idle);
    border: none;
    cursor: pointer;
    padding: 0;
    outline: none;
    flex-shrink: 0;
    transition: background-color 130ms ease;
  }

  /* Checked state — bits-ui sets data-state="checked" */
  :global(.switch-root[data-state='checked']) {
    background-color: var(--color-accent);
  }

  :global(.switch-root:focus-visible) {
    box-shadow: var(--focus-ring);
  }

  /* ── Thumb (the sliding circle) ──────────────────────── */
  :global(.switch-thumb) {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: var(--radius-pill);
    /* Near-white thumb — no token covers a bright neutral; closest is color-text */
    background-color: var(--color-text);
    box-shadow: var(--shadow-thumb-sm);
    /* Start position: 3px from left */
    transform: translateX(3px);
    transition: transform 130ms ease;
    pointer-events: none;
    flex-shrink: 0;
  }

  /* Slide to the right when checked — whole compound selector must be global */
  :global(.switch-root[data-state='checked'] .switch-thumb) {
    transform: translateX(19px);
  }
</style>
