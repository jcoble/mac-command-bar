<script lang="ts">
	import FileCode from "@lucide/svelte/icons/file-code";
	import GitBranch from "@lucide/svelte/icons/git-branch";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import type { Snippet } from "svelte";

	interface Props {
		active?: boolean;
		onSelect?(): void;
		onOpenSession?(): void;
		onOpenEditor?(): void;
		onOpenSourceControl?(): void;
		children: Snippet;
	}

	let {
		active = false,
		onSelect,
		onOpenSession,
		onOpenEditor,
		onOpenSourceControl,
		children,
	}: Props = $props();

	function runShortcut(event: MouseEvent, action: (() => void) | undefined): void {
		event.stopPropagation();
		action?.();
	}
</script>

<div class="row-visual">
	<button
		data-testid="worktree-agent-select"
		type="button"
		class="session-row"
		class:active
		aria-current={active ? "true" : undefined}
		onclick={onSelect}
	>
		{@render children()}
	</button>

	<span class="row-actions" aria-label="Session shortcuts">
		<button type="button" aria-label="Open session" onclick={(event) => runShortcut(event, onOpenSession ?? onSelect)}>
			<MessageSquare aria-hidden="true" />
		</button>
		<button
			type="button"
			aria-label="Open editor"
			onclick={(event) => runShortcut(event, onOpenEditor ?? onSelect)}
		>
			<FileCode aria-hidden="true" />
		</button>
		<button
			type="button"
			aria-label="Open source control"
			onclick={(event) => runShortcut(event, onOpenSourceControl ?? onSelect)}
		>
			<GitBranch aria-hidden="true" />
		</button>
	</span>
</div>

<style>
	.row-visual {
		position: relative;
	}

	/* A 32px provider tile, then the two text lines; the age sits in line one's
	   right corner. 6px corners, the rows' step in the shell's radius scale. */
	.session-row {
		position: relative;
		display: grid;
		grid-template-columns: 32px minmax(0, 1fr);
		column-gap: 12px;
		align-items: center;
		width: 100%;
		min-height: 48px;
		padding: 6px 8px;
		border: 0;
		border-radius: 6px;
		overflow: hidden;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		outline: none;
	}

	.session-row:hover {
		background: var(--accent);
	}

	/* The selected row: a fill plus a short primary bar on its left edge. */
	.session-row.active {
		background: var(--secondary);
	}

	.session-row.active::before {
		content: "";
		position: absolute;
		left: 0;
		top: 12px;
		bottom: 12px;
		width: 2px;
		border-radius: 2px;
		background: var(--primary);
	}

	.row-actions {
		position: absolute;
		top: 4px;
		right: 8px;
		z-index: 2;
		display: flex;
		gap: 3px;
		opacity: 0;
		pointer-events: none;
	}

	.row-visual:hover .row-actions,
	.row-visual:focus-within .row-actions {
		opacity: 1;
		pointer-events: auto;
	}

	.row-visual:hover :global(.line-title),
	.row-visual:focus-within :global(.line-title) {
		padding-right: 82px;
	}

	.row-visual:hover :global(.age),
	.row-visual:focus-within :global(.age) {
		opacity: 0;
	}

	.row-actions button {
		display: grid;
		width: 24px;
		height: 24px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-pill);
		place-items: center;
		background: var(--muted);
		color: var(--muted-foreground);
		cursor: pointer;
	}

	.row-actions button:hover,
	.row-actions button:focus-visible {
		background: color-mix(in srgb, var(--primary) 16%, var(--muted));
		color: var(--primary);
		outline: none;
	}

	.row-actions button:focus-visible {
		box-shadow: var(--focus-ring);
	}

	.row-actions :global(svg) {
		width: 15px;
		height: 15px;
	}
</style>
