/**
 * contextPanelHooks.svelte.ts — the two places outside itself the context panel
 * can send you.
 *
 * The panel is a glance surface: it shows six worktrees out of forty and six
 * agent sessions out of four hundred, because a column two hundred rows long is
 * not a glance. The rest of each list already has a proper home elsewhere in the
 * shell — the Worktrees view in the tool column, and the "Find a session" drawer
 * in the sessions column — so the panel's job at the bottom of a capped list is
 * to open the right one, not to grow into a second copy of it.
 *
 * Those two surfaces are owned by other parts of the shell, and the panel is
 * mounted deep inside the tool column with no props, so the page hands the two
 * doors in here at start-up instead of threading callbacks down through every
 * component in between.
 *
 * WHY THE PANEL CHECKS BEFORE IT DRAWS. Each entry is optional and starts
 * missing. A link that goes nowhere is worse than no link — you press it, the
 * screen does not change, and you are left wondering what you did wrong. So the
 * panel only draws a link once the door behind it is actually there.
 *
 * No `$effect`, no backend call, nothing at import: this module holds two
 * functions and hands them out.
 */

export interface ContextPanelHooks {
  /** Open the Worktrees view in the tool column on the right. */
  showWorktreesView?: () => void;
  /** Open the "Find a session" drawer in the sessions column on the left. */
  openSessionFinder?: () => void;
}

/**
 * The doors the panel currently has. Reactive on purpose: the page wires these
 * up after the panel has already been built, and the links have to appear when
 * it does.
 */
export const contextPanelHooks = $state<ContextPanelHooks>({
  showWorktreesView: undefined,
  openSessionFinder: undefined
});

/**
 * Hand the panel its doors. Replaces whatever was set before, so calling it
 * twice cannot leave half of an old wiring behind. Pass `{}` to take them all
 * away.
 */
export function setContextPanelHooks(hooks: ContextPanelHooks): void {
  contextPanelHooks.showWorktreesView = hooks.showWorktreesView;
  contextPanelHooks.openSessionFinder = hooks.openSessionFinder;
}
