<script lang="ts">
  import BookOpen from '@lucide/svelte/icons/book-open';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Copy from '@lucide/svelte/icons/copy';
  import Check from '@lucide/svelte/icons/check';
  import Folder from '@lucide/svelte/icons/folder';
  import Globe from '@lucide/svelte/icons/globe';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Search from '@lucide/svelte/icons/search';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Wrench from '@lucide/svelte/icons/wrench';
  import FileChangeItem from './FileChangeItem.svelte';
  import ReasoningItem from './ReasoningItem.svelte';
  import SubagentSection from './SubagentSection.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import { sanitizeConversationHref } from '$lib/shell/conversation/conversationMessageSafety.ts';
  import { openUrlInBrowser } from '$lib/shell/workbenchNavigation.ts';

  interface Props {
    item: Extract<ConversationDisplayItem, { kind: 'toolRun' }>;
    onFileLink?(path: string): void;
  }

  let { item, onFileLink }: Props = $props();

  let runOpen = $state(false);
  let runWasActive = false;
  $effect(() => {
    if (!item.completed) {
      runWasActive = true;
      runOpen = true;
      return;
    }
    if (runWasActive) {
      runWasActive = false;
      runOpen = false;
    }
  });
  let expandedCommands = $state<Record<string, boolean>>({});
  let expandedDiffs = $state<Record<string, boolean>>({});
  let copiedCommandId = $state<string | null>(null);

  function toggleCommand(id: string) {
    expandedCommands[id] = !expandedCommands[id];
  }

  function toggleDiff(id: string) {
    expandedDiffs[id] = !expandedDiffs[id];
  }

  async function copyCommandOutput(id: string, text: string) {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    await navigator.clipboard.writeText(text);
    copiedCommandId = id;
  }

  function cleanToolTitle(raw: string): string {
    if (!raw) return '';
    let cleaned = raw.trim();
    if (cleaned.includes('```')) {
      const lines = cleaned
        .replace(/```[a-zA-Z0-9_-]*/g, '')
        .split('\n')
        .map((l) => l.trim())
        .filter((l) => l.length > 0 && !l.startsWith('```'));
      if (lines.length > 0) return lines[0];
      return '';
    }
    cleaned = cleaned.replace(/^`+|`+$/g, '').trim();
    if (cleaned.toLowerCase() === 'tool' || cleaned.toLowerCase() === 'mcp-tool') {
      return '';
    }
    return cleaned;
  }

  function getFileName(path: string): string {
    const cleaned = path.replace(/[\\/]+$/, '');
    return cleaned.split(/[/\\]/).pop() || cleaned;
  }

  function extractFilePath(meta: Record<string, unknown> | undefined, fallback = ''): string {
    if (!meta) return fallback;
    const rawArgs = (meta.args || meta.arguments || meta.parameters || meta.input || meta.rawInput) as Record<string, unknown> | undefined;
    const direct = meta.AbsolutePath || meta.TargetFile || meta.filePath || meta.file_path || meta.path || meta.file || meta.target;
    if (typeof direct === 'string' && direct.trim()) return direct.trim();
    if (rawArgs) {
      const fromArgs = rawArgs.AbsolutePath || rawArgs.TargetFile || rawArgs.filePath || rawArgs.file_path || rawArgs.path || rawArgs.file || rawArgs.target;
      if (typeof fromArgs === 'string' && fromArgs.trim()) return fromArgs.trim();
    }
    if (typeof meta.path === 'string' && meta.path.trim()) return meta.path.trim();
    return fallback;
  }

  function isWebSearchTool(subItem: ConversationDisplayItem): boolean {
    if (subItem.kind !== 'tool') return false;
    const title = (subItem.title || '').toLowerCase();
    const meta = subItem.metadata as Record<string, unknown> | undefined;
    const name = String(meta?.name || meta?.toolName || meta?.tool || '').toLowerCase();
    return title.includes('web') || title.includes('search_web') || title.includes('websearch') || title.includes('read_url')
      || name.includes('web') || name.includes('search_web') || name.includes('websearch') || name.includes('read_url');
  }

  function isFileEditTool(subItem: ConversationDisplayItem): boolean {
    if (subItem.kind === 'file' || subItem.kind === 'fileEdits') return true;
    if (subItem.kind !== 'tool') return false;
    if (subItem.toolKind === 'file-edit') return true;
    const title = (subItem.title || '').toLowerCase();
    const meta = subItem.metadata as Record<string, unknown> | undefined;
    const name = String(meta?.name || meta?.toolName || meta?.tool || '').toLowerCase();
    const out = String(subItem.output || meta?.output || subItem.summary || '');
    return title === 'edit' || title.startsWith('edit ') || title === 'write' || title.startsWith('write ')
      || title.includes('replace_file') || title.includes('write_to_file') || title.includes('edit_file')
      || title.includes('patch') || title.includes('apply') || title.includes('file changes')
      || name === 'edit' || name.startsWith('edit ') || name === 'write' || name.startsWith('write ')
      || name.includes('replace_file') || name.includes('write_to_file') || name.includes('edit_file')
      || name.includes('patch') || name.includes('apply') || name.includes('file changes')
      || Boolean(subItem.diff) || Boolean(meta?.diff) || Boolean(meta?.ReplacementContent)
      || Boolean(meta?.CodeContent) || out.includes('@@');
  }

  function isFileReadTool(subItem: ConversationDisplayItem): boolean {
    if (isFileEditTool(subItem)) return false;
    if (isCommandTool(subItem)) return false;
    if (isSearchTool(subItem)) return false;
    if (isDirectoryTool(subItem)) return false;
    if (subItem.kind === 'file') return true;
    if (subItem.kind !== 'tool') return false;
    if (subItem.toolKind === 'fetch' && !isWebSearchTool(subItem)) return true;
    const title = (subItem.title || '').toLowerCase();
    const meta = subItem.metadata as Record<string, unknown> | undefined;
    const name = String(meta?.name || meta?.toolName || meta?.tool || '').toLowerCase();
    if (title === 'read' || title.startsWith('read ') || title === 'view' || title.startsWith('view ') || title === 'view_file' || title === 'read_file'
      || name === 'read' || name.startsWith('read ') || name === 'view' || name.startsWith('view ') || name === 'view_file' || name === 'read_file') return true;
    const path = extractFilePath(meta, subItem.path || '');
    if (path && (path.includes('.') || path.includes('/')) && !path.includes(' ') && !path.includes('&') && !path.includes('|') && path !== '...') return true;
    return false;
  }

  function parseFileReadInfo(actionItem: ConversationDisplayItem): {
    path: string;
    fileName: string;
  } {
    if (actionItem.kind === 'file') {
      const p = typeof actionItem.metadata?.path === 'string' ? actionItem.metadata.path : '';
      return { path: p, fileName: getFileName(p) };
    }
    if (actionItem.kind === 'tool') {
      const meta = actionItem.metadata as Record<string, unknown> | undefined;
      const rawPath = extractFilePath(meta, actionItem.path || actionItem.summary || '');
      const cleaned = cleanToolTitle(rawPath);
      const fileName = (cleaned && cleaned !== '...') ? getFileName(cleaned) : '';
      const fallbackTitle = cleanToolTitle(actionItem.title);
      return {
        path: cleaned,
        fileName: fileName || (fallbackTitle && fallbackTitle !== 'Tool' ? fallbackTitle : 'file')
      };
    }
    return { path: '', fileName: '' };
  }

  function isCommandTool(subItem: ConversationDisplayItem): boolean {
    if (subItem.kind === 'command') return true;
    if (subItem.kind !== 'tool') return false;
    if (subItem.toolKind === 'command') return true;
    const title = (subItem.title || '').toLowerCase();
    const summary = (subItem.summary || '').toLowerCase();
    const meta = subItem.metadata as Record<string, unknown> | undefined;
    const name = String(meta?.name || meta?.toolName || meta?.tool || '').toLowerCase();
    const rawArgs = (meta?.args || meta?.arguments || meta?.parameters || meta?.input || meta?.rawInput) as Record<string, unknown> | undefined;
    const hasCommandArg = Boolean(meta?.CommandLine || meta?.command || meta?.cmd || rawArgs?.CommandLine || rawArgs?.command || rawArgs?.cmd);
    return hasCommandArg
      || title.includes('console') || title.includes('```') || title === 'bash' || title.startsWith('bash ') || title === 'sh' || title === 'zsh' || title.includes('command') || title.includes('run') || title.includes('exec') || title.includes('terminal') || title.includes('shell')
      || name.includes('console') || name.includes('```') || name === 'bash' || name.startsWith('bash ') || name === 'sh' || name === 'zsh' || name.includes('command') || name.includes('run') || name.includes('exec') || name.includes('terminal') || name.includes('shell')
      || summary.includes('bash') || summary.includes('sh') || summary.includes('console') || summary.includes('```') || summary.includes('command') || summary.includes('run') || summary.includes('exec');
  }

  function isSearchTool(subItem: ConversationDisplayItem): boolean {
    if (isWebSearchTool(subItem)) return false;
    if (subItem.kind !== 'tool') return false;
    if (subItem.toolKind === 'search') return true;
    const title = (subItem.title || '').toLowerCase();
    const meta = subItem.metadata as Record<string, unknown> | undefined;
    const name = String(meta?.name || meta?.toolName || meta?.tool || '').toLowerCase();
    return title === 'grep' || title.startsWith('grep ') || title === 'glob' || title.startsWith('glob ') || title.includes('search') || title.includes('find')
      || name === 'grep' || name.startsWith('grep ') || name === 'glob' || name.startsWith('glob ') || name.includes('search') || name.includes('find');
  }

  function isDirectoryTool(subItem: ConversationDisplayItem): boolean {
    if (subItem.kind !== 'tool') return false;
    const name = (subItem.title || '').toLowerCase();
    const meta = subItem.metadata as Record<string, unknown> | undefined;
    const toolName = String(meta?.name || meta?.toolName || meta?.tool || '').toLowerCase();
    return name.includes('list_dir') || name.includes('list_directory') || name.startsWith('list ') || name.startsWith('listed ')
      || toolName.includes('list_dir') || toolName.includes('list_directory') || toolName === 'ls';
  }

  function parseDirectoryInfo(actionItem: ConversationDisplayItem): { dirName: string; path: string } {
    if (actionItem.kind !== 'tool') return { dirName: '', path: '' };
    const meta = actionItem.metadata as Record<string, unknown> | undefined;
    const rawArgs = (meta?.args || meta?.arguments || meta?.parameters || meta?.input || meta?.rawInput) as Record<string, unknown> | undefined;
    const p = String(rawArgs?.DirectoryPath || rawArgs?.path || meta?.DirectoryPath || meta?.path || actionItem.path || actionItem.summary || '');
    return { dirName: p ? getFileName(p) : (cleanToolTitle(actionItem.title) || 'directory'), path: p };
  }

  function parseCommandInfo(actionItem: ConversationDisplayItem): {
    title: string;
    command: string;
    output: string;
  } {
    if (actionItem.kind === 'command') {
      const meta = actionItem.metadata as Record<string, unknown> | undefined;
      const cmd = (typeof meta?.command === 'string' ? meta.command : null) || actionItem.text;
      const out = (typeof meta?.output === 'string' ? meta.output : null) || '';
      const displayTitle = cleanToolTitle(actionItem.label || cmd) || 'command';
      return {
        title: displayTitle,
        command: cmd,
        output: out
      };
    }
    if (actionItem.kind === 'tool') {
      const meta = actionItem.metadata as Record<string, unknown> | undefined;
      const rawArgs = (meta?.args || meta?.arguments || meta?.parameters || meta?.input || meta?.rawInput) as Record<string, unknown> | undefined;
      const rawCmd = (typeof meta?.CommandLine === 'string' ? meta.CommandLine : null)
        || (typeof meta?.command === 'string' ? meta.command : null)
        || (typeof rawArgs?.CommandLine === 'string' ? rawArgs.CommandLine : null)
        || (typeof rawArgs?.command === 'string' ? rawArgs.command : null)
        || (typeof rawArgs?.cmd === 'string' ? rawArgs.cmd : null)
        || actionItem.summary
        || actionItem.title
        || '';
      const cmd = cleanToolTitle(String(rawCmd));
      const directTitle = cleanToolTitle(actionItem.title);
      const displayTitle = (directTitle && directTitle.toLowerCase() !== 'tool' && directTitle.toLowerCase() !== 'mcp-tool')
        ? directTitle
        : (cmd || 'command');
      return {
        title: displayTitle,
        command: cmd || displayTitle,
        output: actionItem.output || ''
      };
    }
    return { title: 'command', command: '', output: '' };
  }


  function parseSearchInfo(actionItem: ConversationDisplayItem): {
    query: string;
    path: string;
    isWeb: boolean;
    url: string;
  } {
    if (actionItem.kind === 'tool') {
      const meta = actionItem.metadata as Record<string, unknown> | undefined;
      const rawArgs = (meta?.args || meta?.arguments || meta?.parameters || meta?.input) as Record<string, unknown> | undefined;
      const query = typeof meta?.Query === 'string' ? meta.Query
        : (typeof meta?.query === 'string' ? meta.query
        : (typeof meta?.Pattern === 'string' ? meta.Pattern
        : (typeof meta?.pattern === 'string' ? meta.pattern
        : (typeof meta?.Url === 'string' ? meta.Url
        : (typeof meta?.url === 'string' ? meta.url
        : (typeof rawArgs?.Query === 'string' ? rawArgs.Query
        : (typeof rawArgs?.query === 'string' ? rawArgs.query
        : (typeof rawArgs?.Pattern === 'string' ? rawArgs.Pattern
        : (typeof rawArgs?.pattern === 'string' ? rawArgs.pattern
        : (typeof rawArgs?.Url === 'string' ? rawArgs.Url
        : (typeof rawArgs?.url === 'string' ? rawArgs.url
        : (actionItem.summary || ''))))))))))));
      const path = typeof meta?.SearchPath === 'string' ? meta.SearchPath
        : (typeof meta?.path === 'string' ? meta.path
        : (typeof rawArgs?.SearchPath === 'string' ? rawArgs.SearchPath
        : (typeof rawArgs?.SearchDirectory === 'string' ? rawArgs.SearchDirectory
        : (typeof rawArgs?.path === 'string' ? rawArgs.path
        : ''))));
      const isWeb = isWebSearchTool(actionItem);
      const safeUrl = isWeb ? sanitizeConversationHref(query) : null;
      const url = safeUrl && /^https?:/i.test(safeUrl) ? safeUrl : '';
      return { query, path: path ? getFileName(path) : '', isWeb, url };
    }
    return { query: '', path: '', isWeb: false, url: '' };
  }

  function synthesizeDiff(target: string, replacement: string, path: string): string {
    const fileName = getFileName(path);
    const targetLines = target.split('\n');
    const replacementLines = replacement.split('\n');
    const lines: string[] = [
      `--- a/${fileName}`,
      `+++ b/${fileName}`,
      `@@ -1,${targetLines.length} +1,${replacementLines.length} @@`
    ];
    for (const line of targetLines) lines.push(`-${line}`);
    for (const line of replacementLines) lines.push(`+${line}`);
    return lines.join('\n');
  }

  function synthesizeCreatedDiff(content: string, path: string): string {
    const fileName = getFileName(path);
    const contentLines = content.split('\n');
    const lines: string[] = [
      `--- /dev/null`,
      `+++ b/${fileName}`,
      `@@ -0,0 +1,${contentLines.length} @@`
    ];
    for (const line of contentLines) lines.push(`+${line}`);
    return lines.join('\n');
  }

  function parseEditInfo(actionItem: ConversationDisplayItem): {
    action: 'Created' | 'Edited';
    path: string;
    fileName: string;
    diff: string;
    added: number;
    removed: number;
  } {
    if (actionItem.kind === 'file') {
      const p = typeof actionItem.metadata?.path === 'string' ? actionItem.metadata.path : '';
      const d = typeof actionItem.metadata?.diff === 'string' ? actionItem.metadata.diff : actionItem.text;
      const isCreated = d.startsWith('--- /dev/null');
      const added = (d.match(/^\+[^+]/gm) || []).length;
      const removed = (d.match(/^-[^-]/gm) || []).length;
      return { action: isCreated ? 'Created' : 'Edited', path: p, fileName: getFileName(p), diff: d, added, removed };
    }
    if (actionItem.kind === 'fileEdits' && actionItem.edits?.[0]) {
      const first = actionItem.edits[0];
      const isCreated = first.diff.startsWith('--- /dev/null');
      return {
        action: isCreated ? 'Created' : 'Edited',
        path: first.path,
        fileName: getFileName(first.path),
        diff: first.diff,
        added: first.added,
        removed: first.removed
      };
    }
    if (actionItem.kind === 'tool') {
      const meta = actionItem.metadata as Record<string, unknown> | undefined;
      const rawArgs = (meta?.args || meta?.arguments || meta?.parameters || meta?.input || meta?.rawInput) as Record<string, unknown> | undefined;
      let p = extractFilePath(meta, actionItem.path || actionItem.summary || '');
      const toolName = String(meta?.name || meta?.toolName || meta?.tool || actionItem.title || '').toLowerCase();
      const isCreated = toolName.includes('write_to_file') || toolName === 'write' || toolName.includes('create') || Boolean(rawArgs?.CodeContent || meta?.CodeContent);

      let d = actionItem.diff || (typeof meta?.diff === 'string' ? meta.diff : '');
      if (!d) {
        const target = String(rawArgs?.TargetContent || rawArgs?.old_string || meta?.TargetContent || '');
        const replacement = String(rawArgs?.ReplacementContent || rawArgs?.new_string || meta?.ReplacementContent || '');
        const code = String(rawArgs?.CodeContent || rawArgs?.content || meta?.CodeContent || '');
        if (code || isCreated) {
          d = synthesizeCreatedDiff(code || actionItem.output || '', p);
        } else if (target || replacement) {
          d = synthesizeDiff(target, replacement, p);
        } else if (actionItem.output && actionItem.output.includes('@@')) {
          d = actionItem.output;
        } else if (actionItem.summary && actionItem.summary.includes('@@')) {
          d = actionItem.summary;
        }
      }

      if (!p || p === '[object Object]' || p.includes('[object Object]')) {
        const pathMatch = (d || actionItem.output || actionItem.summary || '').match(/(?:\+\+\+\s+(?:b\/)?|---\s+(?:a\/)?|path:\s*|[a-z0-9_.-]+:\s*)([^\s\n]+\.[a-zA-Z0-9_-]+)/i);
        if (pathMatch) p = pathMatch[1].trim();
        else p = '';
      }

      if (d.includes('[object Object]')) {
        d = d.replace(/\[object Object\]:\s*[^\n]*\n?/g, '').trim();
      }

      const added = (d.match(/^\+[^+]/gm) || []).length;
      const removed = (d.match(/^-[^-]/gm) || []).length;
      return {
        action: isCreated ? 'Created' : 'Edited',
        path: p,
        fileName: getFileName(p) || 'file',
        diff: d,
        added: Math.max(added, isCreated && !added ? 1 : 0),
        removed
      };
    }
    return { action: 'Edited', path: '', fileName: '', diff: '', added: 0, removed: 0 };
  }

  function getGenericToolLabel(actionItem: ConversationDisplayItem): string {
    if ('metadata' in actionItem && actionItem.metadata) {
      const meta = actionItem.metadata as Record<string, unknown>;
      if (typeof meta.toolAction === 'string' && meta.toolAction.trim()) return cleanToolTitle(meta.toolAction.trim()) || 'Tool run';
      if (typeof meta.toolSummary === 'string' && meta.toolSummary.trim()) return cleanToolTitle(meta.toolSummary.trim()) || 'Tool run';
      if (typeof meta.toolTitle === 'string' && meta.toolTitle.trim()) return cleanToolTitle(meta.toolTitle.trim()) || 'Tool run';
      if (typeof meta.toolName === 'string' && meta.toolName.trim()) {
        const name = cleanToolTitle(meta.toolName.trim());
        if (name) return `Used ${name}`;
      }
      if (typeof meta.tool === 'string' && meta.tool.trim() && meta.tool !== 'mcp-tool' && meta.tool !== 'Tool') {
        const name = cleanToolTitle(meta.tool.trim());
        if (name) return `Used ${name}`;
      }
      if (typeof meta.name === 'string' && meta.name.trim() && meta.name !== 'mcp-tool' && meta.name !== 'Tool') {
        const name = cleanToolTitle(meta.name.trim());
        if (name) return `Used ${name}`;
      }
    }
    if ('title' in actionItem && actionItem.title) {
      const cleaned = cleanToolTitle(actionItem.title);
      if (cleaned) {
        if ('summary' in actionItem && actionItem.summary) {
          const sum = cleanToolTitle(actionItem.summary);
          return sum ? `${cleaned}: ${sum}` : cleaned;
        }
        return cleaned;
      }
    }
    if ('summary' in actionItem && actionItem.summary) {
      const cleaned = cleanToolTitle(actionItem.summary);
      if (cleaned) return cleaned;
    }
    if ('label' in actionItem && actionItem.label) {
      const cleaned = cleanToolTitle(actionItem.label);
      if (cleaned) return cleaned;
    }
    if ('path' in actionItem && actionItem.path) {
      return `Accessed ${getFileName(actionItem.path)}`;
    }
    return 'Tool run';
  }
</script>

<div class="tool-run-item" data-testid="tool-run-item">
  <button
    class="run-header"
    type="button"
    aria-expanded={runOpen}
    onclick={() => (runOpen = !runOpen)}
  >
    <span class="run-icon">
      {#if item.icon === 'pencil'}
        <Pencil size={13} />
      {:else if item.icon === 'book'}
        <BookOpen size={13} />
      {:else if item.icon === 'terminal'}
        <Terminal size={13} />
      {:else if item.icon === 'search'}
        <Search size={13} />
      {:else}
        <Sparkles size={13} />
      {/if}
    </span>
    <span class="run-summary">{item.summary}</span>
    <span class="chevron-wrap" class:open={runOpen}>
      <ChevronRight size={13} />
    </span>
  </button>

  {#if runOpen}
    <div class="run-body">
      {#each item.items as subItem (subItem.itemId)}
        {#if subItem.kind === 'reasoning'}
          <div class="sub-row">
            <ReasoningItem item={subItem} {onFileLink} />
          </div>
        {:else if subItem.kind === 'subagent'}
          <div class="sub-row">
            <SubagentSection item={subItem} />
          </div>
        {:else if isFileEditTool(subItem)}
          {@const info = parseEditInfo(subItem)}
          <div class="sub-row edit-sub-row">
            <div
              class="sub-item-header"
              role="button"
              tabindex="0"
              onclick={() => toggleDiff(subItem.itemId)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleDiff(subItem.itemId); } }}
            >
              <Pencil size={13} class="item-icon" />
              <span class="item-label">
                {info.action} <button class="link-name" type="button" onclick={(e) => { e.stopPropagation(); onFileLink?.(info.path); }}>{info.fileName}</button>
                {#if info.added > 0 || info.removed > 0}
                  <span class="diff-chip">
                    {#if info.added > 0}<span class="added">+{info.added}</span>{/if}
                    {#if info.removed > 0}<span class="removed">−{info.removed}</span>{/if}
                  </span>
                {/if}
              </span>
              {#if info.diff}
                <span class="chevron-wrap" class:open={expandedDiffs[subItem.itemId]}>
                  <ChevronRight size={12} />
                </span>
              {/if}
            </div>
            {#if expandedDiffs[subItem.itemId] && info.diff}
              <div class="inline-diff-card">
                <FileChangeItem path={info.path} diff={info.diff} {onFileLink} />
              </div>
            {/if}
          </div>
        {:else if isCommandTool(subItem)}
          {@const info = parseCommandInfo(subItem)}
          <div class="sub-row command-sub-row">
            <div
              class="sub-item-header"
              role="button"
              tabindex="0"
              onclick={() => toggleCommand(subItem.itemId)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleCommand(subItem.itemId); } }}
            >
              <Terminal size={13} class="item-icon" />
              <span class="item-label">Ran {info.title}</span>
              <span class="chevron-wrap" class:open={expandedCommands[subItem.itemId]}>
                <ChevronRight size={12} />
              </span>
            </div>
            {#if expandedCommands[subItem.itemId]}
              <div class="shell-output-card">
                <div class="shell-header">
                  <span class="shell-badge">Shell</span>
                  <button
                    class="copy-shell-btn"
                    type="button"
                    title="Copy output"
                    onclick={() => copyCommandOutput(subItem.itemId, info.output || info.command)}
                  >
                    {#if copiedCommandId === subItem.itemId}
                      <Check size={12} />
                    {:else}
                      <Copy size={12} />
                    {/if}
                  </button>
                </div>
                <pre class="shell-pre"><code>{#if info.command}$ {info.command}{'\n'}{/if}{info.output || '(completed with no output)'}</code></pre>
              </div>
            {/if}
          </div>
        {:else if isWebSearchTool(subItem)}
          {@const info = parseSearchInfo(subItem)}
          <div class="sub-row search-sub-row">
            <div class="sub-item-header non-clickable">
              <Globe size={13} class="item-icon" />
              <span class="item-label">
                Searched the web for
                {#if info.url}
                  <a
                    class="search-query target-link"
                    href={info.url}
                    onclick={(event) => { event.stopPropagation(); event.preventDefault(); void openUrlInBrowser({ url: info.url }); }}
                  >{info.query}</a>
                {:else}
                  <span class="search-query">{info.query || 'information'}</span>
                {/if}
              </span>
            </div>
          </div>
        {:else if isSearchTool(subItem)}
          {@const info = parseSearchInfo(subItem)}
          <div class="sub-row search-sub-row">
            <div class="sub-item-header non-clickable">
              <Search size={13} class="item-icon" />
              <span class="item-label">
                Searched for <span class="search-query">{info.query || 'items'}</span>
                {#if info.path} in <span class="search-path">{info.path}</span>{/if}
              </span>
            </div>
          </div>
        {:else if isFileReadTool(subItem)}
          {@const info = parseFileReadInfo(subItem)}
          <div class="sub-row read-sub-row">
            <div class="sub-item-header non-clickable">
              <BookOpen size={13} class="item-icon" />
              <span class="item-label">
                Read
                {#if info.path && onFileLink}
                  <button
                    class="link-name"
                    type="button"
                    onclick={() => onFileLink?.(info.path)}
                  >{info.fileName}</button>
                {:else}
                  <span class="link-name">{info.fileName || 'file'}</span>
                {/if}
              </span>
            </div>
          </div>
        {:else if isDirectoryTool(subItem)}
          {@const info = parseDirectoryInfo(subItem)}
          <div class="sub-row dir-sub-row">
            <div class="sub-item-header non-clickable">
              <Folder size={13} class="item-icon" />
              <span class="item-label">
                Listed <span class="dir-name">{info.dirName}</span>
              </span>
            </div>
          </div>
        {:else}
          <div class="sub-row generic-sub-row">
            <div class="sub-item-header non-clickable">
              <Wrench size={13} class="item-icon" />
              <span class="item-label">{getGenericToolLabel(subItem)}</span>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .tool-run-item {
    display: flex;
    flex-direction: column;
    margin: 2px 0;
  }

  .run-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 3px 6px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--color-text-2);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .run-header:hover {
    background: color-mix(in srgb, var(--color-hover) 50%, transparent);
    color: var(--color-text);
  }

  .run-header:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .run-icon {
    display: grid;
    place-items: center;
    color: var(--color-text-3);
    flex: none;
  }

  .run-summary {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .target-link {
    color: var(--color-accent);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .chevron-wrap {
    display: grid;
    place-items: center;
    color: var(--color-text-3);
    flex: none;
    transition: transform 0.14s ease;
  }

  .chevron-wrap.open {
    transform: rotate(90deg);
  }

  .run-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-left: 8px;
    padding-right: 6px;
    margin: 0 0 4px;
    max-height: 240px;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) transparent;
  }

  .sub-row {
    display: flex;
    flex-direction: column;
  }

  .sub-item-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    padding: 2px 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-2);
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .sub-item-header:hover:not(.non-clickable) {
    background: color-mix(in srgb, var(--color-hover) 40%, transparent);
    color: var(--color-text);
  }

  .sub-item-header.non-clickable {
    cursor: default;
  }

  .item-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .link-name {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text);
    text-decoration: underline;
    text-underline-offset: 2px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    font-size: inherit;
  }

  .link-name:hover {
    color: var(--color-accent);
  }

  .diff-chip {
    display: inline-flex;
    gap: 4px;
    margin-left: 6px;
    font-family: var(--font-mono);
    font-size: 11.5px;
  }

  .added {
    color: var(--color-good);
  }

  .removed {
    color: var(--color-bad);
  }

  .search-query {
    color: var(--color-text);
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .search-path {
    color: var(--color-text-3);
  }

  .shell-output-card {
    margin: 4px 0 8px 22px;
    border: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-surface) 35%, var(--color-bg) 65%);
    overflow: hidden;
  }

  .shell-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border) 50%, transparent);
    color: var(--color-text-3);
    font-size: 11.5px;
  }

  .shell-badge {
    font-weight: 500;
  }

  .copy-shell-btn {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-text-3);
    cursor: pointer;
  }

  .copy-shell-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .shell-pre {
    margin: 0;
    padding: 8px 12px;
    max-height: 260px;
    overflow-y: auto;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-text-2);
    white-space: pre-wrap;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) transparent;
  }

  .inline-diff-card {
    margin: 4px 0 8px 22px;
  }
</style>
