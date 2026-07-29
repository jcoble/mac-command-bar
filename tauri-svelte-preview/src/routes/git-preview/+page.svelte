<script lang="ts">
  /**
   * /git-preview — a page for LOOKING AT the source-control panes in a browser.
   *
   * WHY IT EXISTS. The panes are built to live in the /next shell, and the
   * shell is wired together by the integrator. While the panes are being built
   * there is nothing to mount them in, and in a browser the desktop commands
   * they normally use answer nothing at all. This page mounts them on their own
   * against the dev server's read-only git bridge, so the whole surface —
   * rows, the commit graph, an opened commit, a file's changes — can be seen
   * and corrected against a real repository.
   *
   * It is SCRATCH, like the integration note beside the components. Delete the
   * route and `vite.config.gitpreview.ts` once the panes are wired into the
   * shell for real.
   *
   * Nothing loads until the button is pressed: same rule as the shell.
   */
  import '$lib/shell/styles/nextTokens.css';
  import '$lib/shell/styles/next.css';

  import GitDiffView from '$lib/shell/components/GitDiffView.svelte';
  import SourceControlPanes from '$lib/shell/components/git/SourceControlPanes.svelte';
  import { bridgeGitBackend } from '$lib/shell/git/gitBackendExtra';
  import { createGitCommitFilesService } from '$lib/shell/git/gitCommitFilesService';
  import { gitCommitFiles } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import { createGitService } from '$lib/shell/git/gitService';

  /** A service that reads through the dev server instead of the desktop app. */
  const service = createGitService({ backend: bridgeGitBackend() });
  const commitFiles = createGitCommitFilesService();

  let root = $state('/Users/blackcolours/dev/work/mac-command-bar');

  function show(): void {
    service.activate(null);
    commitFiles.activate(null);
    service.activate(root.trim());
    commitFiles.activate(root.trim());
  }
</script>

<div class="next-shell" style="height:100vh;display:flex;flex-direction:column;background:#101014">
  <div
    style="display:flex;gap:8px;padding:8px;border-bottom:1px solid #22222c;
           font:13px/18px ui-sans-serif,system-ui,sans-serif;color:#d8d8e0"
  >
    <input
      bind:value={root}
      aria-label="Repository folder"
      style="flex:1;background:#17171d;border:1px solid #22222c;border-radius:6px;
             padding:4px 8px;color:#d8d8e0;font:inherit"
    />
    <button
      type="button"
      onclick={show}
      style="border:1px solid #2a2a34;border-radius:6px;padding:4px 12px;
             background:#1f1f27;color:#d8d8e0;font:inherit;cursor:pointer"
    >
      Show this repository
    </button>
  </div>

  <div style="flex:1;min-height:0;display:flex">
    <div style="width:340px;border-right:1px solid #22222c;min-height:0">
      <SourceControlPanes {service} {commitFiles} commitFilesState={gitCommitFiles} />
    </div>
    <div style="flex:1;min-width:0;min-height:0">
      <GitDiffView />
    </div>
  </div>
</div>
