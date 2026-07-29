<script lang="ts">
  /**
   * BrowserPanel.svelte — the /next embedded browser tab.
   *
   * A page preview inside the app: an address box, a reload button, a button
   * that hands the address to the real web browser, and the frame itself.
   *
   * Started from the old shell's `src/lib/components/panels/BrowserPanel.svelte`
   * (presentational, no backend calls). What was kept, and what changed:
   *
   *  - KEPT VERBATIM: the `{#key `${frameKey}:${url}`}` wrapper and the frame's
   *    `sandbox` / `referrerpolicy` attributes. The key wrapper is the ONLY
   *    thing allowed to rebuild the frame, and it only changes when the user
   *    submits an address or presses Reload. Switching tabs must never rebuild
   *    it — the dock keeps every panel attached, so the page inside (scroll
   *    position, form input, open sockets) survives a tab switch for free.
   *  - DROPPED: the `panelAction` prop (the new dock moves panels itself), the
   *    "hide" button (the tab strip owns that), and the dev-server shortcut row
   *    (running-server cards live in the context panel now; they can call
   *    `openBrowserUrl` on this panel's store).
   *  - NO PROPS: the panel is self-contained; all state lives in the store.
   *  - STYLES: the shared `.file-action-button` chrome from the old `app.css`
   *    is inlined below in the /next colours instead of reaching into that
   *    global stylesheet.
   *
   * Nothing loads at launch: until `activateBrowser()` runs (the shell calls it
   * the first time the user opens this tab, and any action here calls it too),
   * there is no frame and nothing is fetched.
   */
  import { ExternalLink, Globe, RefreshCw } from '@lucide/svelte';

  import {
    browser,
    clearBrowserError,
    reloadBrowserFrame,
    setBrowserUrl
  } from '$lib/shell/browser/browserStore.svelte';

  function submitUrl(event: SubmitEvent): void {
    event.preventDefault();
    setBrowserUrl(browser.inputUrl);
  }

  function openInWebBrowser(): void {
    if (!browser.url || typeof window === 'undefined') return;
    window.open(browser.url, '_blank', 'noopener,noreferrer');
  }
</script>

<section class="browser-panel" aria-label="Browser">
  <header class="browser-header">
    <div class="browser-title">
      <Globe size={13} strokeWidth={2} />
      <strong>Browser</strong>
      <span>{browser.url || 'No page open'}</span>
    </div>
    <div class="browser-actions">
      <button
        class="file-action-button"
        type="button"
        aria-label="Reload the page"
        title="Reload the page"
        disabled={!browser.url}
        onclick={reloadBrowserFrame}
      >
        <RefreshCw size={13} strokeWidth={2} />
      </button>
      <button
        class="file-action-button"
        type="button"
        aria-label="Open this address in your web browser"
        title="Open this address in your web browser"
        disabled={!browser.url}
        onclick={openInWebBrowser}
      >
        <ExternalLink size={13} strokeWidth={2} />
      </button>
    </div>
  </header>

  <form class="browser-url-form" onsubmit={submitUrl}>
    <input
      bind:value={browser.inputUrl}
      oninput={clearBrowserError}
      aria-label="Address"
      autocomplete="off"
      spellcheck="false"
      placeholder="localhost:5177"
    />
    <button class="file-action-button open-button" type="submit" disabled={!browser.inputUrl.trim()}>
      <span>Open</span>
    </button>
  </form>

  {#if browser.error}
    <p class="browser-message">{browser.error}</p>
  {/if}

  {#if browser.activated && browser.url}
    <div class="browser-frame-wrap">
      {#key `${browser.frameKey}:${browser.url}`}
        <iframe
          class="browser-frame"
          title="Page preview"
          src={browser.url}
          sandbox="allow-downloads allow-forms allow-modals allow-popups allow-same-origin allow-scripts"
          referrerpolicy="no-referrer"
        ></iframe>
      {/key}
    </div>
  {:else}
    <div class="browser-empty">
      Type an address to preview a page here — a port on its own, like <code>:5177</code>, means
      this machine.
    </div>
  {/if}
</section>

<style>
  /*
   * `.file-action-button` used to be a `:global` rule in `src/app.css` shared by
   * the old shell's surfaces. /next does not load that chrome, so the base rule
   * is inlined here in the /next colours (bg #101014, raised #17171d, border
   * #22222c, text #d8d8e0, muted #6d6d7d, faint #4c4c5a). It is scoped to this
   * component, which is the only place that uses it here.
   */
  /* A flex column, not a fixed grid: the message strip appears and disappears,
     and the frame must keep taking whatever height is left either way. */
  .browser-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 8px;
    background: #101014;
    color: #d8d8e0;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
  }

  .browser-header,
  .browser-url-form {
    display: grid;
    align-items: center;
    flex: 0 0 auto;
    gap: 6px;
    min-width: 0;
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .browser-title,
  .browser-actions {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .browser-title {
    gap: 6px;
    color: #d8d8e0;
  }

  .browser-title strong,
  .browser-title span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .browser-title strong {
    font-size: 12px;
    font-weight: 700;
  }

  .browser-title span {
    color: #6d6d7d;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }

  .browser-actions {
    justify-content: end;
    gap: 5px;
  }

  .file-action-button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 28px;
    color: #d8d8e0;
    border: 1px solid #22222c;
    border-radius: 6px;
    background: #17171d;
    cursor: pointer;
  }

  .file-action-button:hover {
    color: #ffffff;
    border-color: #3a3a48;
  }

  .file-action-button:focus-visible {
    border-color: #4c4c5a;
    outline: 0;
    box-shadow: 0 0 0 3px rgba(120, 120, 160, 0.18);
  }

  .file-action-button:disabled {
    color: #4c4c5a;
    cursor: default;
    opacity: 0.6;
  }

  .browser-actions .file-action-button {
    width: 28px;
    height: 24px;
    min-width: 28px;
    padding: 0;
  }

  .open-button {
    display: inline-flex;
    width: auto;
    height: 28px;
    padding: 0 10px;
    font-family: inherit;
    font-size: 12px;
    font-weight: 700;
  }

  .browser-url-form input {
    width: 100%;
    height: 28px;
    min-width: 0;
    padding: 0 9px;
    color: #d8d8e0;
    border: 1px solid #22222c;
    border-radius: 6px;
    background: #17171d;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }

  .browser-url-form input::placeholder {
    color: #4c4c5a;
  }

  .browser-url-form input:focus-visible {
    border-color: #4c4c5a;
    outline: 0;
  }

  .browser-message {
    flex: 0 0 auto;
    margin: 0;
    padding: 4px 8px;
    color: #e0b268;
    border: 1px solid #22222c;
    border-radius: 6px;
    background: #17171d;
    font-size: 12px;
  }

  .browser-frame-wrap {
    flex: 1 1 auto;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    border: 1px solid #22222c;
    border-radius: 6px;
    background: #17171d;
  }

  .browser-frame {
    display: block;
    width: 100%;
    height: 100%;
    border: 0;
    background: #101014;
  }

  .browser-empty {
    display: grid;
    place-items: center;
    flex: 1 1 auto;
    min-height: 0;
    padding: 12px;
    text-align: center;
    color: #6d6d7d;
    border: 1px dashed #22222c;
    border-radius: 6px;
    font-size: 12px;
    line-height: 1.5;
  }

  .browser-empty code {
    color: #d8d8e0;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }
</style>
