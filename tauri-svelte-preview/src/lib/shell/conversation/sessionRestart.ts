interface SessionRecoveryHandlers {
  restart(ownedId: string): void;
  reconnect(ownedId: string): void;
}

let activeBinding: SessionRecoveryHandlers | null = null;

/** Share the page-owned restart path with shell surfaces that are not siblings in the DOM. */
export function registerSessionRestart(
  restart: SessionRecoveryHandlers['restart'],
  reconnect: SessionRecoveryHandlers['reconnect']
): () => void {
  const binding = { restart, reconnect };
  activeBinding = binding;
  return () => {
    if (activeBinding === binding) activeBinding = null;
  };
}

export function requestSessionRestart(ownedId: string, restartable: boolean): boolean {
  if (!activeBinding) return false;
  if (restartable) activeBinding.restart(ownedId);
  else activeBinding.reconnect(ownedId);
  return true;
}
