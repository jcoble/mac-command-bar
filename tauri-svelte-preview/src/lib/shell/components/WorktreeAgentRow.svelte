<script lang="ts">
	/**
	 * One session in the rail: project and status, title and model, branch.
	 *
	 * The row currently owns one visual-only click. It changes the rail's local
	 * active paint and calls no session, panel, persistence, or native service.
	 *
	 * The rail passes one shared timestamp into every row. This row mounts no
	 * timer, clock subscription, spinner, or visibility observer.
	 */
	import { AGENT_ICONS, agentDisplayName } from "$lib/shell/agentIcons.ts";
	import {
		deriveSessionPresence,
		EMPTY_SESSION_PRESENCE_HISTORY,
		sessionPresenceHistory,
	} from "$lib/shell/conversation/sessionPresence.ts";
	import { presentAgentError } from "$lib/shell/errorPresentation";
	import { resolveOwnedSessionProject, type OwnedSession } from "$lib/shell/ownedSessions";
	import { deriveOwnedLibraryState } from "$lib/shell/sessionLibrary/sessionLibraryModel";
	import { sessionLabel } from "$lib/shell/sessionStrip";
	import { formatRailElapsed } from "./railElapsedTicker.ts";
	import SessionRowVisual from "./SessionRowVisual.svelte";

	interface Props {
		session: OwnedSession;
		nowMs: number;
		active?: boolean;
		onSelect?(): void;
		onOpenSession?(): void;
		onOpenEditor?(): void;
		onOpenSourceControl?(): void;
		onContextMenu?(event: MouseEvent): void;
	}

	type RowPresence = "working" | "attention" | "idle" | "done" | "failed";
	let {
		session,
		nowMs,
		active = false,
		onSelect,
		onOpenSession,
		// onOpenEditor,
		// onOpenSourceControl,
		onContextMenu,
	}: Props = $props();

	const label = $derived(sessionLabel(session));
	const shelf = $derived(deriveOwnedLibraryState(session));
	const projectInfo = $derived(resolveOwnedSessionProject(session));
	const project = $derived(projectInfo.label);
	const presentedError = $derived(session.lastError ? presentAgentError(session.lastError) : null);

	const presenceHistory = $derived($sessionPresenceHistory[session.ownedId] ?? EMPTY_SESSION_PRESENCE_HISTORY);
	// Presence is rail-record truth plus events received live. Loading a stored
	// transcript may populate `conversation`, but it must not repaint this row.
	const needsYou = $derived(
		session.pendingPermission === true ||
			session.pendingInput === true ||
			session.runtimeState === "waiting-approval" ||
			session.runtimeState === "waiting-input",
	);
	const pendingApprovalCount = $derived(needsYou ? 1 : 0);
	const runtimeState = $derived(session.runtimeState);
	const activeTurnId = $derived(session.activeTurnId ?? presenceHistory.activeTurnId);
	const suspended = $derived(session.runtimeState === "suspended");
	const presenceSignals = $derived(
		deriveSessionPresence(
			{
				terminalState: session.state,
				suspended,
				activeTurnId,
				sending: false,
				pendingApprovalCount,
				runtimeState,
			},
			presenceHistory,
			0,
		).state,
	);

	const presence = $derived<RowPresence>(
		shelf === "done"
			? "done"
			: session.lastError || runtimeState === "failed"
				? "failed"
				: suspended
					? "idle"
					: needsYou || presenceSignals === "needs-attention"
						? "attention"
						: presenceSignals === "working"
							? "working"
							: "idle",
	);
	const presenceLabel = $derived(
		{
			working: "Working",
			attention: "Waiting on you",
			idle: "Idle",
			done: "Finished",
			failed: "Error",
		}[presence],
	);
	const ProviderIcon = $derived(AGENT_ICONS[session.agent]);
	const providerName = $derived(agentDisplayName(session.agent, session.viaCmux));

	// ── The age, and the working indicator in the rail ─────────────────────────
	/**
	 * How old this session is, counted from when it started rather than from the
	 * current turn — the number stays put when the agent stops working, which is
	 * what makes it comparable between rows.
	 */
	const startedAtMs = $derived(
		session.startedAtMs ??
			(session.lastActivity && Number.isFinite(Date.parse(session.lastActivity))
				? Date.parse(session.lastActivity)
				: null),
	);
	const ageMs = $derived(startedAtMs === null ? null : Math.max(0, nowMs - startedAtMs));
	const ageText = $derived(ageMs === null ? null : formatRailElapsed(ageMs));
</script>

<li
	data-testid="worktree-agent-row"
	data-presence={presence}
	data-shelf={shelf}
	class:needs-you-row={needsYou}
	class="row"
	oncontextmenu={onContextMenu}
>
	<SessionRowVisual
		{active}
		{onSelect}
		{onOpenSession}
		// {onOpenEditor} {onOpenSourceControl}
	>
		<!-- The mark, at the height of the three lines beside it. It carries
               the provider and whether this session is working, and nothing
               else: no action is ever drawn on top of it. -->
		<span
			data-testid="worktree-agent-provider"
			class="thumb"
			data-agent={session.agent}
			role="img"
			aria-label={providerName}
		>
			<ProviderIcon class="thumb-mark" aria-hidden="true" />
		</span>

		<span class="lines">
			<!-- Line one is the title and a corner that always holds exactly one
                 thing: the last activity time at rest, the quick-jump buttons
                 while the pointer is on the row. The corner is in the same place
                 either way, so the buttons never land on the title — the title
                 simply clips a little earlier to make room for them. -->
			<span class="line line-title">
				<span data-testid="worktree-agent-title" class="session-title">{label}</span>
				<span data-testid="worktree-agent-age" class="age">
					<span class="age-text">{ageText ?? ""}</span>
				</span>
			</span>

			<span class="line line-meta">
				<span data-testid="worktree-agent-meta" class="project">{project}</span>
				{#if session.branch}
					<span class="sep" aria-hidden="true">•</span>
					<span class="branch">{session.branch}</span>
				{/if}

				<!-- What the row still needs to say in words. The working state is
                   the mark's job, so only the states a colour cannot carry are
                   spelled out here. -->
				{#if needsYou}
					<span data-testid="worktree-agent-needs-you" class="needs-you">
						<span class="attention-dot" aria-hidden="true"></span>
						Needs you
					</span>
				{:else if presence === "failed"}
					<span data-testid="worktree-agent-status" class="failed">{presentedError?.summary ?? presenceLabel}</span>
				{:else if suspended}
					<span data-testid="worktree-agent-status" class="idle-label">
						{suspended ? "Suspended" : presenceLabel}
					</span>
				{/if}
			</span>
		</span>
	</SessionRowVisual>
</li>

<style>
	.row {
		position: relative;
		display: block;
		box-sizing: border-box;
		min-width: 0;
		content-visibility: auto;
		contain-intrinsic-size: auto 58px;
		/* The highlight block is inset from the rail's edges rather than bled to
       them, so a hovered row reads as a card in the list. */
		padding: 0 6px;
		list-style: none;
		color: var(--color-text);
		font-size: 13px;
		line-height: 1.4;
	}

	/* The mark, then the three lines. The row is 58px so a rail this narrow still
     shows a useful stack of sessions; the mark matches the height of the three
     lines beside it, which is the proportion the reference keeps. */
	/* The provider mark, at the height of the text beside it. Nothing is ever
     drawn over this square: it says who is running the session and whether the
     session is working, and those are the only two things it says. */
	.thumb {
		position: relative;
		display: flex;
		width: 44px;
		height: 44px;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-sm);
		background: color-mix(in srgb, var(--color-elevated) 68%, var(--color-surface));
		color: var(--color-text-2);
	}

	:global(.thumb-mark) {
		width: 22px;
		height: 22px;
		flex: 0 0 auto;
	}

	/* The two marks their vendors publish in a colour wear it here. The rest
     stay the theme's own, because inventing a brand colour for them would be
     a worse lie than a neutral glyph. */
	.thumb[data-agent="claude"] {
		color: var(--agent-mark-claude);
	}
	.thumb[data-agent="codex"] {
		color: var(--agent-mark-codex);
	}

	/* TWO fixed line boxes, not three. The heights are declared rather than left
     to the font so the row is the same height on every machine and the mark can
     be sized against it. */
	.lines {
		display: flex;
		min-width: 0;
		flex-direction: column;
		justify-content: center;
		gap: 0;
	}

	.line {
		position: relative;
		display: flex;
		min-width: 0;
		align-items: center;
		gap: 5px;
	}

	.line-title {
		height: 21px;
		line-height: 21px;
	}

	.line-meta {
		height: 17px;
		line-height: 17px;
	}

	.project {
		min-width: 0;
		flex: 0 1 auto;
		overflow: hidden;
		color: var(--color-text-2);
		font-size: 12px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* The branch, at the project's size but a step dimmer. Not monospaced: this
     line is read, not compared character by character, and a mono face at this
     size is both wider and harder to read in a rail this narrow. The project
     gives way first, because it repeats down the whole list and the branch is
     what tells one row from the next. */
	.sep {
		flex: 0 0 auto;
		color: var(--color-text-3);
		font-size: 12px;
	}

	.branch {
		min-width: 0;
		flex: 0 1 auto;
		overflow: hidden;
		color: var(--color-text-3);
		font-size: 12px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.idle-label,
	.failed,
	.needs-you {
		margin-left: auto;
	}

	/* Last activity stays in line one's right corner while the row is hovered. */
	.age {
		display: inline-flex;
		flex: 0 0 auto;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
		padding-left: 8px;
		color: var(--color-text-3);
		font-family: var(--font-mono);
		font-size: 12px;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	/* One button wide, always: the spinner beside it must sit the same distance
     from the row's right edge whether the time reads "now" or "12m", because
     the cluster's gap for it is measured from that edge. */
	.age-text {
		min-width: 28px;
		text-align: right;
	}

	.row[data-presence="working"] .age {
		color: var(--color-text-2);
	}

	.needs-you {
		display: inline-flex;
		flex: 0 0 auto;
		align-items: center;
		gap: 5px;
		color: var(--color-attention);
		font-size: 11.5px;
		font-weight: 600;
		white-space: nowrap;
	}

	.attention-dot {
		width: 7px;
		height: 7px;
		flex: 0 0 auto;
		border-radius: 50%;
		background: var(--color-attention);
	}

	.failed {
		min-width: 0;
		flex: 0 1 auto;
		overflow: hidden;
		color: var(--color-bad);
		font-size: 11.5px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.idle-label {
		flex: 0 0 auto;
		color: var(--color-idle);
		font-size: 11.5px;
		white-space: nowrap;
	}

	.session-title {
		min-width: 0;
		flex: 1 1 auto;
		overflow: hidden;
		color: var(--color-text);
		font-size: 14px;
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Presence changes may fade their own small labels; pointer movement owns no
     transition or animation in a session row. */
	@media (prefers-reduced-motion: no-preference) {
		.age-text,
		.idle-label,
		.thumb {
			transition: opacity 120ms ease;
		}
	}
</style>
