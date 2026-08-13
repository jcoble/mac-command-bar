export interface ConversationDraftBackend {
  set(ownedId: string, text: string): Promise<void>;
  get(ownedId: string): Promise<string | null>;
  clear(ownedId: string): Promise<void>;
}

type PendingDraft = {
  text: string;
  timer: ReturnType<typeof setTimeout>;
};

/** One debounce timer and one ordered write queue per owned session. */
export class ConversationDraftPersistence {
  private readonly pending = new Map<string, PendingDraft>();
  private readonly writes = new Map<string, Promise<void>>();
  private readonly revisions = new Map<string, number>();
  private readonly backend: ConversationDraftBackend;
  private readonly applyLoadedDraft: (ownedId: string, text: string) => void;
  private readonly delayMs: number;

  constructor(
    backend: ConversationDraftBackend,
    applyLoadedDraft: (ownedId: string, text: string) => void,
    delayMs = 500
  ) {
    this.backend = backend;
    this.applyLoadedDraft = applyLoadedDraft;
    this.delayMs = delayMs;
  }

  schedule(ownedId: string, text: string): void {
    this.bumpRevision(ownedId);
    this.cancelTimer(ownedId);
    const timer = setTimeout(() => {
      void this.flush(ownedId).catch(() => undefined);
    }, this.delayMs);
    this.pending.set(ownedId, { text, timer });
  }

  flush(ownedId: string): Promise<void> {
    const pending = this.pending.get(ownedId);
    if (!pending) return this.writes.get(ownedId) ?? Promise.resolve();
    clearTimeout(pending.timer);
    this.pending.delete(ownedId);
    return this.enqueue(ownedId, () => this.backend.set(ownedId, pending.text));
  }

  async load(ownedId: string): Promise<void> {
    const revision = this.revisions.get(ownedId) ?? 0;
    await (this.writes.get(ownedId) ?? Promise.resolve()).catch(() => undefined);
    const text = await this.backend.get(ownedId);
    if ((this.revisions.get(ownedId) ?? 0) === revision) {
      this.applyLoadedDraft(ownedId, text ?? '');
    }
  }

  clear(ownedId: string): Promise<void> {
    this.bumpRevision(ownedId);
    this.cancelTimer(ownedId);
    return this.enqueue(ownedId, () => this.backend.clear(ownedId));
  }

  private cancelTimer(ownedId: string): void {
    const pending = this.pending.get(ownedId);
    if (!pending) return;
    clearTimeout(pending.timer);
    this.pending.delete(ownedId);
  }

  private bumpRevision(ownedId: string): void {
    this.revisions.set(ownedId, (this.revisions.get(ownedId) ?? 0) + 1);
  }

  private enqueue(ownedId: string, action: () => Promise<void>): Promise<void> {
    const previous = this.writes.get(ownedId) ?? Promise.resolve();
    const work = previous.catch(() => undefined).then(action);
    this.writes.set(ownedId, work);
    const forget = (): void => {
      if (this.writes.get(ownedId) === work) this.writes.delete(ownedId);
    };
    work.then(forget, forget);
    return work;
  }
}
