<script lang="ts">
  import type {
    ThreadStartProviderConfig,
    ThreadStartRequest
  } from '$lib/shell/newSession/threadStartFlow.ts';

  interface Props {
    onStart: (request: ThreadStartRequest) => void | Promise<boolean | void>;
  }

  let { onStart }: Props = $props();

  type ThreadStartPaneComponent =
    (typeof import('./NewSessionThread.svelte'))['default'];

  let ThreadStartPane = $state<ThreadStartPaneComponent | null>(null);
  let open = $state(false);
  let loading = false;
  let loadFailure = $state<string | null>(null);
  let sessionRoots = $state<string[]>([]);
  let sessionProviderConfigs = $state<ThreadStartProviderConfig[]>([]);

  /** Open a fresh draft. Mounting a new pane resets every picker value. */
  export function openNewSession(input: {
    sessionRoots?: string[];
    providerConfigs?: ThreadStartProviderConfig[];
  } = {}): void {
    sessionRoots = [...(input.sessionRoots ?? [])];
    sessionProviderConfigs = (input.providerConfigs ?? []).map((config) => ({
      ...config,
      availableModels: [...config.availableModels],
      availableEfforts: [...config.availableEfforts],
      availableApprovalPolicies: [...config.availableApprovalPolicies]
    }));
    loadFailure = null;
    open = true;
    if (ThreadStartPane || loading) return;
    loading = true;
    import('./NewSessionThread.svelte')
      .then((module) => {
        ThreadStartPane = module.default;
      })
      .catch((error: unknown) => {
        loadFailure = error instanceof Error ? error.message : String(error);
        open = false;
      })
      .finally(() => {
        loading = false;
      });
  }

  export function close(): void {
    open = false;
  }

  async function submit(request: ThreadStartRequest): Promise<boolean | void> {
    const result = await onStart(request);
    if (result !== false) open = false;
    return result;
  }
</script>

{#if ThreadStartPane && open}
  <ThreadStartPane
    {sessionRoots}
    providerConfigs={sessionProviderConfigs}
    onSend={submit}
    onClose={() => (open = false)}
  />
{/if}

{#if loadFailure}
  <div
    data-testid="new-session-thread-load-error"
    role="alert"
    class="fixed bottom-3 left-1/2 z-[200] max-w-[480px] -translate-x-1/2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-1.5 text-[13px] text-destructive"
  >
    The new session pane could not open: {loadFailure}
  </div>
{/if}
