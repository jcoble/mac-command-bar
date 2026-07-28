<script lang="ts">
  /** A shallow slot: names the future panel, does nothing, loads nothing. */
  interface Props {
    name: string;
    hint?: string;
  }
  let { name, hint = 'Coming soon.' }: Props = $props();
</script>

<div class="placeholder">
  <p class="name">{name}</p>
  <p class="hint">{hint}</p>
</div>

<style>
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 100%;
    width: 100%;
    background: #101014;
    color: #6d6d7d;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
    user-select: none;
  }
  .placeholder p {
    margin: 0;
  }
  .hint {
    color: #4c4c5a;
    font-size: 11px;
  }
</style>
