/**
 * A promise settled from outside: the one way this app turns a callback or an
 * event (a socket opening, a stream's next item, a backend receipt, an abort)
 * into something to await. Whoever creates one settles it, including when its
 * owner's signal aborts, so no await is left hanging.
 */
export type Deferred<T> = {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
};

export function deferred<T = void>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((accept, decline) => {
    resolve = accept;
    reject = decline;
  });
  return { promise, resolve, reject };
}
