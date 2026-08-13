<script lang="ts">
  /**
   * HistoryPanel.svelte — the History tab of the right column.
   *
   * Today it hosts the existing session library unchanged, in its right-hand
   * placement. The sessions it lists come from the rail store; the actions it
   * offers belong to the page, which registers them once (see
   * `sessionLibraryService.ts`).
   */
  import SessionLibraryWorkspace from '$lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte';
  import { sessionLibraryHost } from '$lib/shell/sessionLibrary/sessionLibraryService';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let { visible, root }: Props = $props();

  // Read once, at init: the page registers its actions in its own component
  // body, which runs before this panel is created.
  const host = sessionLibraryHost();
</script>

<SessionLibraryWorkspace
  owned={rail.owned}
  available={rail.available}
  service={host.service}
  {visible}
  workspacePath={root || null}
  projectPath={root || null}
  onRefresh={() => host.rescan?.()}
/>
