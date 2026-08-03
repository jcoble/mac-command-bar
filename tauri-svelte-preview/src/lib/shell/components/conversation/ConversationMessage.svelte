<script lang="ts">
  interface Props { text: string; role: 'user' | 'assistant'; label: string; }
  let { text, role, label }: Props = $props();
  const blocks = $derived(text.split(/```/g).map((value, index) => ({
    kind: index % 2 === 1 ? 'code' : 'text',
    value: value.replace(/^\w+\n/, index % 2 === 1 ? '' : '$&').trim()
  })).filter((block) => block.value));
</script>

<article class:user={role === 'user'} class:assistant={role === 'assistant'}>
  <div class="turn-label">{label}</div>
  <div class="turn-body">
    {#each blocks as block}
      {#if block.kind === 'code'}
        <pre><code>{block.value}</code></pre>
      {:else}
        {#each block.value.split(/\n\s*\n/) as paragraph}
          <p>{paragraph}</p>
        {/each}
      {/if}
    {/each}
  </div>
</article>

<style>
  article{max-width:760px;user-select:text;-webkit-user-select:text}.turn-label{margin-bottom:7px;color:var(--color-text-2);font-size:12px;font-weight:650;letter-spacing:.04em;text-transform:uppercase}.turn-body{font-size:14px;line-height:1.65}.turn-body p{margin:0 0 12px;white-space:pre-wrap}.turn-body p:last-child{margin-bottom:0}.user{align-self:flex-end;max-width:min(680px,88%);padding:12px 15px;border-radius:16px;background:color-mix(in srgb,var(--color-surface) 88%,var(--color-accent) 12%)}.user .turn-label{color:color-mix(in srgb,var(--color-accent) 70%,var(--color-text) 30%)}pre{overflow:auto;margin:12px 0;padding:14px 16px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:10px;background:color-mix(in srgb,var(--color-bg) 76%,black 24%);font:12px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}code{user-select:text;-webkit-user-select:text}
</style>
