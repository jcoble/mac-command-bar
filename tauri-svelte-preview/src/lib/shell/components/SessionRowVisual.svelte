<script lang="ts">
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
		// onOpenSession,
		// onOpenEditor,
		// onOpenSourceControl,
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

	<!-- <span class="row-actions" aria-label="Session shortcuts">
		<button type="button" aria-label="Open session" onclick={(event) => runShortcut(event, onOpenSession)}>
			<MessageCircle aria-hidden="true" />
		</button>
		<button type="button" aria-label="Open editor" onclick={(event) => runShortcut(event, onOpenEditor)}>
			<FileCode2 aria-hidden="true" />
		</button>
		<button type="button" aria-label="Open source control" onclick={(event) => runShortcut(event, onOpenSourceControl)}>
			<GitBranch aria-hidden="true" />
		</button>
	</span> -->
</div>

<style>
	.row-visual {
		position: relative;
	}

	.session-row {
		position: relative;
		display: grid;
		grid-template-columns: 44px minmax(0, 1fr);
		column-gap: 10px;
		align-items: center;
		width: 100%;
		min-height: 58px;
		padding: var(--rail-row-content-inset);
		border: 0;
		border-radius: var(--radius-sm);
		overflow: hidden;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		outline: none;
	}

	.session-row.active {
		background: color-mix(in srgb, var(--color-elevated) 88%, var(--color-text));
	}

	.row-actions {
		position: absolute;
		top: 5px;
		right: 11px;
		z-index: 2;
		display: none;
		gap: 3px;
	}

	.row-visual:focus-within .row-actions {
		display: flex;
	}

	:global(.line-title) {
		padding-right: 82px;
	}

	.row-visual:focus-within :global(.age) {
		display: none;
	}

	.row-actions button {
		display: grid;
		width: 25px;
		height: 25px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-pill);
		place-items: center;
		background: var(--color-elevated);
		color: var(--color-text-2);
		cursor: pointer;
	}

	.row-actions button:focus-visible {
		background: var(--color-hover);
		color: var(--color-text);
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
