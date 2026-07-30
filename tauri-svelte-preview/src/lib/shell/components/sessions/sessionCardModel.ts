/**
 * sessionCardModel.ts — the words a session card puts on screen.
 *
 * PURE: no store, no DOM, no backend call, no clock of its own (`now` is a
 * parameter). The card component draws what these functions return, so what a
 * card SAYS can be read — and tested — without a browser.
 *
 * Two rules live here rather than in the markup:
 *
 *  - A session's status word describes its TERMINAL, not the list it sits on.
 *    Which list a session is on is the user's own answer ("Mark done"), and a
 *    session marked done can still have a process running. Saying "Done" for a
 *    running terminal would hide a running process behind a tidy word, so a
 *    done session whose terminal is still going reads "Running" and its hover
 *    sentence says both things.
 *
 *  - A fact the store does not carry is left out, never printed empty. A
 *    session started here has no reading of a conversation on disk, so its
 *    detail block is simply shorter.
 */
import { exactLocalTime, formatLastActivity } from '../../relativeTime.ts';
import type { OwnedSession } from '../../ownedSessions';

/** Which colour the dot and the word wear. Not a colour itself — the component
 * owns the tokens, this is only the meaning. */
export type SessionStatusTone = 'running' | 'done' | 'stopped';

export interface SessionStatus {
  /** One word, on the card, beside the dot. */
  word: string;
  tone: SessionStatusTone;
  /** The whole story, as a sentence, for the hover title. */
  hint: string;
}

/**
 * Running / Done / Stopped, from the terminal's state and the user's answer.
 *
 * "Stopped" rather than "Idle": an idle terminal is one waiting for you, and
 * this one has ended. The sentence beside it says what to do about that.
 */
export function sessionStatus(session: Pick<OwnedSession, 'state' | 'completedAt'>): SessionStatus {
  const markedDone = session.completedAt !== null;

  if (session.state !== 'exited') {
    return {
      word: 'Running',
      tone: 'running',
      hint: markedDone
        ? 'You marked this done, but its terminal is still running.'
        : 'Its terminal is running.'
    };
  }

  if (markedDone) {
    return {
      word: 'Done',
      tone: 'done',
      hint: 'You marked this done, and its terminal has ended.'
    };
  }

  return {
    word: 'Stopped',
    tone: 'stopped',
    hint: 'Its terminal has ended. Start it again to pick this session up.'
  };
}

/**
 * "12 messages" — a plain count of the turns the scanner saw. It is a floor
 * rather than a total (the scanner reads a bounded window of a transcript),
 * which is why it is not dressed up as a total with a "+". Nothing to say
 * about a session with no conversation on disk, so it says nothing.
 */
export function messageCountLabel(count: number | null | undefined): string | null {
  if (typeof count !== 'number' || count <= 0) return null;
  return count === 1 ? '1 message' : `${count} messages`;
}

/** One line of the detail block: a heading and its value, with the exact
 * moment kept for the hover where the value is a rounded-off time. */
export interface SessionFact {
  label: string;
  value: string;
  /** The full local date and time, when the value is a shortened stamp. */
  exact?: string;
}

/**
 * Everything the detail block says about a session, in the order a person asks
 * for it: where it lives, where it came from, how big the conversation is,
 * when anything last happened, when it was finished, and what typing it back
 * to life looks like.
 *
 * The folder is always present because a session always has one. Everything
 * else appears only when the store actually carries it.
 */
export function sessionFacts(session: OwnedSession, now: Date): SessionFact[] {
  const facts: SessionFact[] = [
    {
      label: 'Folder',
      value: session.cwd || session.projectPath || 'Not recorded'
    },
    {
      label: 'Where it came from',
      value:
        session.source === 'fresh'
          ? 'Started here'
          : 'Found on this machine and picked up here'
    }
  ];

  const messages = messageCountLabel(session.messageCount);
  if (messages) {
    facts.push({ label: 'Conversation so far', value: `${messages} (at least)` });
  }

  const lastActivity = formatLastActivity(session.lastActivity, now);
  if (lastActivity) {
    facts.push({
      label: 'Last thing the scan saw',
      value: lastActivity,
      exact: exactLocalTime(session.lastActivity)
    });
  }

  const finished = formatLastActivity(session.completedAt, now);
  if (finished) {
    facts.push({
      label: 'You marked it done',
      value: finished,
      exact: exactLocalTime(session.completedAt)
    });
  }

  if (session.resumeCommand) {
    facts.push({ label: 'Starts back up with', value: session.resumeCommand });
  }

  return facts;
}
