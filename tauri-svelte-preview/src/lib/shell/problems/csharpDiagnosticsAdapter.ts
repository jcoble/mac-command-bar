import type { RawProblemDiagnostic } from './problemsStore.svelte';

const diagnosticsByRoot = new Map<string, RawProblemDiagnostic[]>();
const listeners = new Set<(root: string) => void>();

function normalizedRoot(root: string): string {
  return root.replaceAll('\\', '/').replace(/\/+$/, '');
}

export function recordNativeCsharpDiagnostics(
  root: string,
  diagnostics: RawProblemDiagnostic[]
): void {
  const key = normalizedRoot(root);
  diagnosticsByRoot.set(key, diagnostics.map((diagnostic) => ({ ...diagnostic })));
  for (const listener of listeners) listener(key);
}

export function nativeCsharpDiagnosticsForRoot(root: string): RawProblemDiagnostic[] {
  return diagnosticsByRoot.get(normalizedRoot(root))?.map((diagnostic) => ({ ...diagnostic })) ?? [];
}

export function onNativeCsharpDiagnosticsChanged(listener: (root: string) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
