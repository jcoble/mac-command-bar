<script lang="ts">
  import type {
    AgentRoleDefinition,
    ConfigSelectionPolicy,
    WorkflowDefinitionV1
  } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    definition?: WorkflowDefinitionV1 | null;
    input?: Record<string, unknown>;
    compact?: boolean;
    onCreate?(definition: WorkflowDefinitionV1, input: Record<string, unknown>, startAfterCreate: boolean): void;
  }

  let {
    definition = null,
    input = {},
    compact = false,
    onCreate
  }: Props = $props();

  const defaultDefinition: WorkflowDefinitionV1 = {
    version: 1,
    id: 'new-workflow',
    name: 'New workflow',
    description: '',
    trigger: {},
    inputs: [],
    roles: [
      {
        id: 'implementer',
        name: 'Implementer',
        purpose: 'Make the requested change.',
        providerPolicy: { provider: 'codex', allowedProviders: ['codex', 'claude'] },
        modelPolicy: { value: null, overrides: {} },
        effortPolicy: { value: null, overrides: {} },
        permissionPolicy: { value: null, overrides: {} },
        promptTemplateId: '',
        outputContract: 'ImplementationReceipt',
        workspacePolicy: { kind: 'read-only-current' },
        retryPolicy: { maxAttempts: 1, retryableCodes: [] }
      }
    ],
    nodes: [
      {
        id: 'implement',
        title: 'Implement change',
        roleId: 'implementer',
        dependsOn: [],
        condition: null,
        fanOut: null,
        approvalGate: null,
        timeoutSeconds: 3600,
        maxAttempts: 1
      }
    ],
    edges: [],
    concurrency: { global: 8, workflow: 4, providers: {} },
    budgets: {
      maximumActiveAgents: 8,
      maximumChildDepth: 3,
      maximumAttemptsPerNode: 1,
      maximumWallTimeSeconds: 86400,
      maximumTokens: null,
      maximumToolTerminals: 8,
      maximumWorktrees: 4
    },
    completion: { cancelDescendantsOnFailure: false }
  };

  let draftDefinition = $state<WorkflowDefinitionV1>(clone(defaultDefinition));
  let inputText = $state('{}');
  let error = $state<string | null>(null);

  $effect(() => {
    draftDefinition = clone(definition ?? defaultDefinition);
    inputText = JSON.stringify(input ?? {}, null, 2);
  });

  function clone(value: WorkflowDefinitionV1): WorkflowDefinitionV1 {
    return JSON.parse(JSON.stringify(value)) as WorkflowDefinitionV1;
  }

  function updateRolePolicy(
    roleId: string,
    policy: 'modelPolicy' | 'effortPolicy' | 'permissionPolicy',
    value: string
  ): void {
    draftDefinition = {
      ...draftDefinition,
      roles: draftDefinition.roles.map((role) =>
        role.id === roleId
          ? { ...role, [policy]: { ...role[policy], value: value.trim() || null } }
          : role
      )
    };
  }

  function updateProvider(roleId: string, provider: string): void {
    draftDefinition = {
      ...draftDefinition,
      roles: draftDefinition.roles.map((role) =>
        role.id === roleId ? { ...role, providerPolicy: { ...role.providerPolicy, provider } } : role
      )
    };
  }

  function rolePolicyValue(role: AgentRoleDefinition, policy: keyof Pick<AgentRoleDefinition, 'modelPolicy' | 'effortPolicy' | 'permissionPolicy'>): string {
    const selection: ConfigSelectionPolicy = role[policy];
    return selection.value ?? '';
  }

  function updateInput(value: string): void {
    inputText = value;
    error = null;
  }

  function submit(startAfterCreate: boolean): void {
    error = null;
    try {
      const parsed = JSON.parse(inputText) as Record<string, unknown>;
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        throw new Error('Workflow input must be a JSON object.');
      }
      onCreate?.(clone(draftDefinition), parsed, startAfterCreate);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Workflow input must be valid JSON.';
    }
  }
</script>

<section data-testid="workflow-template-editor" class:compact class="editor" aria-label="Workflow template editor">
  <header data-testid="workflow-template-editor-header" class="editor-header">
    <div data-testid="workflow-template-editor-title" class="editor-title">Workflow template</div>
    <span data-testid="workflow-template-editor-copy" class="editor-copy">Choose role settings before start</span>
  </header>

  <div data-testid="workflow-template-editor-fields" class="fields">
    <label data-testid="workflow-template-name-label"><span data-testid="workflow-template-name-copy">Name</span><input data-testid="workflow-template-name" value={draftDefinition.name} oninput={(event) => (draftDefinition.name = (event.currentTarget as HTMLInputElement).value)} /></label>
    <label data-testid="workflow-template-id-label"><span data-testid="workflow-template-id-copy">Workflow id</span><input data-testid="workflow-template-id" value={draftDefinition.id} oninput={(event) => (draftDefinition.id = (event.currentTarget as HTMLInputElement).value)} /></label>
    <label data-testid="workflow-template-description-label"><span data-testid="workflow-template-description-copy">Description</span><textarea data-testid="workflow-template-description" value={draftDefinition.description} oninput={(event) => (draftDefinition.description = (event.currentTarget as HTMLTextAreaElement).value)}></textarea></label>
  </div>

  <section data-testid="workflow-template-role-settings" class="role-settings">
    <h3 data-testid="workflow-template-role-heading">Role settings</h3>
    {#if draftDefinition.roles.length === 0}
      <p data-testid="workflow-template-no-roles" class="muted">Add roles to the definition before starting.</p>
    {:else}
      {#each draftDefinition.roles as role (role.id)}
        <article data-testid={`workflow-template-role-${role.id}`} class="role-card">
          <header data-testid={`workflow-template-role-header-${role.id}`} class="role-header"><span data-testid={`workflow-template-role-name-${role.id}`} class="role-name">{role.name}</span><span data-testid={`workflow-template-role-provider-label-${role.id}`} class="role-provider-label">{role.providerPolicy.provider}</span></header>
          <div data-testid={`workflow-template-role-fields-${role.id}`} class="role-fields">
            <label data-testid={`workflow-template-provider-label-${role.id}`}><span data-testid={`workflow-template-provider-copy-${role.id}`}>Provider</span><input data-testid={`workflow-template-provider-${role.id}`} value={role.providerPolicy.provider} oninput={(event) => updateProvider(role.id, (event.currentTarget as HTMLInputElement).value)} /></label>
            <label data-testid={`workflow-template-model-label-${role.id}`}><span data-testid={`workflow-template-model-copy-${role.id}`}>Model</span><input data-testid={`workflow-template-model-${role.id}`} value={rolePolicyValue(role, 'modelPolicy')} placeholder="Use provider default" oninput={(event) => updateRolePolicy(role.id, 'modelPolicy', (event.currentTarget as HTMLInputElement).value)} /></label>
            <label data-testid={`workflow-template-effort-label-${role.id}`}><span data-testid={`workflow-template-effort-copy-${role.id}`}>Effort</span><input data-testid={`workflow-template-effort-${role.id}`} value={rolePolicyValue(role, 'effortPolicy')} placeholder="Use provider default" oninput={(event) => updateRolePolicy(role.id, 'effortPolicy', (event.currentTarget as HTMLInputElement).value)} /></label>
            <label data-testid={`workflow-template-permission-label-${role.id}`}><span data-testid={`workflow-template-permission-copy-${role.id}`}>Permission</span><input data-testid={`workflow-template-permission-${role.id}`} value={rolePolicyValue(role, 'permissionPolicy')} placeholder="Use provider default" oninput={(event) => updateRolePolicy(role.id, 'permissionPolicy', (event.currentTarget as HTMLInputElement).value)} /></label>
          </div>
        </article>
      {/each}
    {/if}
  </section>

  <label data-testid="workflow-template-input-label" class="input-editor"><span data-testid="workflow-template-input-copy">Run input (JSON object)</span><textarea data-testid="workflow-template-input" value={inputText} oninput={(event) => updateInput((event.currentTarget as HTMLTextAreaElement).value)}></textarea></label>
  {#if error}<p data-testid="workflow-template-error" class="error">{error}</p>{/if}
  <div data-testid="workflow-template-actions" class="actions">
    <button data-testid="workflow-template-create" class="action" type="button" onclick={() => submit(false)}>Create draft</button>
    <button data-testid="workflow-template-create-start" class="action accent" type="button" onclick={() => submit(true)}>Create and start</button>
  </div>
</section>

<style>
  .editor { display: grid; gap: 10px; min-width: 0; padding: 10px; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .editor.compact { padding: 8px; }
  .editor-header { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; border-bottom: 1px solid var(--color-border); padding-bottom: 8px; }
  .editor-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .editor-copy, .muted { color: var(--color-text-2); font-size: 12px; }
  .fields, .role-fields { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 7px; }
  label { display: grid; gap: 4px; color: var(--color-text-2); font-size: 12px; }
  input, textarea { box-sizing: border-box; width: 100%; min-height: 29px; padding: 5px 7px; border: 1px solid var(--color-border); border-radius: 5px; color: var(--color-text); background: var(--color-bg); font-size: 13px; }
  textarea { min-height: 68px; resize: vertical; }
  input:focus-visible, textarea:focus-visible { outline: 1px solid var(--color-focus-solid); outline-offset: 1px; }
  .role-settings { display: grid; gap: 7px; padding-top: 8px; border-top: 1px solid var(--color-border); }
  h3 { margin: 0; font-size: 13px; font-weight: 600; }
  .role-card { display: grid; gap: 7px; padding: 8px; border: 1px solid var(--color-border); border-radius: 5px; background: var(--color-bg); }
  .role-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .role-name { font-size: 13px; font-weight: 600; }
  .role-provider-label { color: var(--color-accent); font-size: 12px; }
  .input-editor { padding-top: 8px; border-top: 1px solid var(--color-border); }
  .actions { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: 6px; }
  .action { min-height: 29px; padding: 0 8px; border: 0; border-radius: 5px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; cursor: pointer; }
  .action:hover, .action:focus-visible { color: var(--color-text); background: var(--color-hover); outline: none; }
  .action.accent { color: var(--color-on-accent); background: var(--color-accent); }
  .error { margin: 0; color: var(--color-bad); font-size: 12px; }
</style>
