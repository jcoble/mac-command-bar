/**
 * gitAmend.ts — the Amend switch's message box. Turning Amend on fills an empty
 * box with the last commit's first line; turning it off takes back only that
 * text. Sent untouched, the line goes as an empty message so git keeps the
 * whole original message, body included.
 */
import type { GitBackend } from './gitService.ts';
import type { GitPanelState } from './gitPanelStore.svelte.ts';

export function createGitAmend(state: GitPanelState, backend: GitBackend) {
  /** What Amend put in the box, and the repository it came from; '' when it put nothing. */
  let prefill = '';
  let prefillRoot = '';
  const putByAmend = () => (state.root === prefillRoot ? prefill : '');
  return {
    async set(on: boolean): Promise<void> {
      state.amend = on;
      const put = putByAmend();
      prefill = '';
      if (!on) {
        if (put !== '' && state.commitMessage === put) state.commitMessage = '';
        return;
      }
      const root = state.root;
      if (!root || state.commitMessage.trim() !== '') return;
      let subject = '';
      try {
        subject = (await backend.readHistory(root))?.commits?.[0]?.subject ?? '';
      } catch {
        // No history to read: the box stays empty, and an empty amend still works.
      }
      // The repository, the switch or the box may have changed while git was answering.
      if (state.root !== root || !state.amend || state.commitMessage.trim() !== '') return;
      state.commitMessage = prefill = subject;
      prefillRoot = root;
    },
    /** The message to hand git for this amend. */
    messageFor(text: string): string {
      return text === putByAmend() ? '' : text;
    },
    /** The amend happened: the box is empty again, and the next commit is a new one, not a second amend. */
    finish(): void {
      prefill = '';
      state.commitMessage = '';
      state.amend = false;
    }
  };
}
