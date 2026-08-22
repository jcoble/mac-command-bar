<script lang="ts">
  import CodeMirrorSourceEditor from '$lib/CodeMirrorSourceEditor.svelte';
  import fixtureJson from '$lib/dev-fixtures/assembly.json';
  import ConversationTimeline from '$lib/shell/components/conversation/ConversationTimeline.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import AgentRow from '$lib/shell/panels/agents/AgentRow.svelte';
  import type { AgentActivityRow } from '$lib/shell/panels/agents/agentActivityModel.ts';
  import FileTreeRow from '$lib/shell/panels/files/FileTreeRow.svelte';
  import {
    visibleFileTreeNodes,
    type FileTreeNode
  } from '$lib/shell/panels/files/fileTreeModel.ts';
  import ChangedFileRow from '$lib/shell/panels/sourceControl/ChangedFileRow.svelte';
  import type { SourcePreview } from '$lib/sourceData.ts';
  import type { ProjectGitFileStatus } from '$lib/tauriSource.ts';

  import '$lib/shell/styles/nextTokens.css';
  import '$lib/shell/styles/next.css';

  interface FixtureData {
    conversation: ConversationDisplayItem[];
    files: FileTreeNode[];
    changes: ProjectGitFileStatus[];
    agents: AgentActivityRow[];
    source: SourcePreview;
  }

  const fixture = fixtureJson as unknown as FixtureData;
  let expandedPaths = $state(new Set(fixture.files.filter((node) => node.isDirectory).map((node) => node.path)));
  let selectedFile = $state(fixture.files.find((node) => !node.isDirectory)?.path ?? '');
  let selectedAgent = $state(fixture.agents[0]?.childId ?? '');
  const visibleFiles = $derived(visibleFileTreeNodes(fixture.files, expandedPaths));

  function selectFile(node: FileTreeNode): void {
    if (!node.isDirectory) {
      selectedFile = node.path;
      return;
    }

    const next = new Set(expandedPaths);
    if (next.has(node.path)) next.delete(node.path);
    else next.add(node.path);
    expandedPaths = next;
  }
</script>

<svelte:head><title>Assembly development fixtures</title></svelte:head>

<main class="next-shell fixture-page" data-testid="dev-fixtures-page">
  <header class="fixture-header">
    <div>
      <p class="eyebrow">Development fixture</p>
      <h1>Assembly presentation surfaces</h1>
    </div>
    <p class="fixture-note">Static JSON · read-only · no application services</p>
  </header>

  <section class="surface conversation-surface" data-testid="dev-fixture-conversation">
    <div class="surface-heading">
      <div>
        <h2>Long conversation</h2>
        <p>{fixture.conversation.length} timeline items</p>
      </div>
      <span class="surface-tag">Virtualized</span>
    </div>
    <div class="conversation-frame">
      <ConversationTimeline
        items={fixture.conversation}
        conversationId="dev-fixture-assembly"
        timelineRevision={1}
        assistantLabel="Codex"
      />
    </div>
  </section>

  <div class="panel-grid">
    <section class="surface" data-testid="dev-fixture-file-tree">
      <div class="surface-heading">
        <div><h2>Files</h2><p>Compact nested tree</p></div>
      </div>
      <div class="row-frame">
        {#each visibleFiles as node (node.path)}
          <FileTreeRow
            {node}
            expanded={expandedPaths.has(node.path)}
            selected={selectedFile === node.path}
            onclick={selectFile}
            onaction={() => {}}
          />
        {/each}
      </div>
    </section>

    <section class="surface" data-testid="dev-fixture-source-control">
      <div class="surface-heading">
        <div><h2>Source control</h2><p>Dense mixed states</p></div>
        <span class="surface-tag">Read-only</span>
      </div>
      <div class="row-frame">
        {#each fixture.changes as file (file.relativePath)}
          <ChangedFileRow
            {file}
            root=""
            canWrite={false}
            readOnlyReason="Development fixture rows are read-only."
            busy={false}
            onStage={() => {}}
            onUnstage={() => {}}
            onRequestDiscard={() => {}}
          />
        {/each}
      </div>
    </section>

    <section class="surface" data-testid="dev-fixture-agents">
      <div class="surface-heading">
        <div><h2>Agents</h2><p>Activity and status rows</p></div>
      </div>
      <div class="row-frame">
        {#each fixture.agents as row (row.childId)}
          <AgentRow
            {row}
            selected={selectedAgent === row.childId}
            onselect={(childId) => selectedAgent = childId}
          />
        {/each}
      </div>
    </section>
  </div>

  <section class="surface editor-surface" data-testid="dev-fixture-editor">
    <div class="surface-heading">
      <div><h2>Code editor</h2><p>{fixture.source.relativePath}</p></div>
      <span class="surface-tag">TypeScript</span>
    </div>
    <div class="editor-frame">
      <CodeMirrorSourceEditor preview={fixture.source} editable={false} />
    </div>
  </section>
</main>

<style>
  .fixture-page { min-height: 100vh; box-sizing: border-box; padding: 24px; background: var(--color-bg); color: var(--color-text); font-family: var(--font-sans); }
  .fixture-header { display: flex; align-items: end; justify-content: space-between; gap: 24px; max-width: 1500px; margin: 0 auto 16px; }
  .eyebrow { margin: 0 0 4px; color: var(--color-accent); font-size: 12px; font-weight: 700; letter-spacing: .12em; text-transform: uppercase; }
  h1, h2, p { margin: 0; }
  h1 { font-size: 22px; font-weight: 650; letter-spacing: -.02em; }
  h2 { font-size: 14px; font-weight: 650; }
  .fixture-note, .surface-heading p { color: var(--color-text-3); font-size: 12px; }
  .surface { min-width: 0; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); box-shadow: var(--shadow-sm); }
  .surface-heading { display: flex; min-height: 48px; box-sizing: border-box; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 12px; border-bottom: 1px solid var(--color-border); background: var(--color-elevated); }
  .surface-heading > div { min-width: 0; }
  .surface-heading p { overflow: hidden; margin-top: 2px; text-overflow: ellipsis; white-space: nowrap; }
  .surface-tag { flex: none; padding: 3px 7px; border: 1px solid var(--color-border); border-radius: var(--radius-pill); color: var(--color-text-2); font-size: 12px; }
  .conversation-surface, .panel-grid, .editor-surface { max-width: 1500px; margin-right: auto; margin-left: auto; }
  .conversation-frame { display: flex; height: 560px; min-height: 0; }
  .panel-grid { display: grid; grid-template-columns: 1fr 1.25fr 1fr; gap: 12px; margin-top: 12px; }
  .row-frame { height: 336px; overflow: auto; padding: 4px; scrollbar-width: thin; scrollbar-color: var(--scrollbar-thumb) transparent; }
  .editor-surface { margin-top: 12px; }
  .editor-frame { height: 520px; min-height: 0; }
  @media (max-width: 980px) {
    .fixture-page { padding: 16px; }
    .fixture-header { align-items: start; flex-direction: column; gap: 6px; }
    .panel-grid { grid-template-columns: 1fr; }
  }
</style>
