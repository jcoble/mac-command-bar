/**
 * Extensions Mac Command Bar intentionally bundles.
 *
 * This is an allowlist, not a marketplace client. Each entry records which
 * part of an extension we use and why it is safe in the browser-style local
 * extension host. Native Git, files, shells and language-server processes stay
 * behind the existing Rust/Tauri boundary.
 */
export type CuratedExtensionKind = 'declarative' | 'web' | 'native-bridge';

export type CuratedExtension = {
  id: string;
  label: string;
  version: string;
  kind: CuratedExtensionKind;
  license: string;
  source: string;
  enabled: boolean;
  notes: string;
};

export const CURATED_EXTENSIONS: readonly CuratedExtension[] = [
  {
    id: 'astro-build.houston',
    label: 'Houston',
    version: '0.1.0+d297233',
    kind: 'declarative',
    license: 'MIT',
    source: 'https://github.com/withastro/houston-vscode',
    enabled: true,
    notes:
      'Registers the official color-theme contribution only. The optional mascot webview and Node entrypoint are deliberately not bundled.'
  },
  {
    id: 'svelte.svelte-vscode-syntax',
    label: 'Svelte language basics',
    version: '110.3.0',
    kind: 'declarative',
    license: 'MIT',
    source: 'https://github.com/sveltejs/language-tools',
    enabled: true,
    notes:
      'Registers the official Svelte grammar, language configuration, and snippets only. The Node extension entrypoint and language server are managed separately by the workspace LSP adapter.'
  },
  {
    id: 'mcb.rust-git-scm',
    label: 'Mac Command Bar Git provider',
    version: '0.1.0',
    kind: 'native-bridge',
    license: 'Internal',
    source: 'src/lib/shell/extensions/rustGitScmProvider.ts',
    enabled: true,
    notes:
      'Projects the status already loaded by gitService into vscode.scm. It never starts Git or changes repositories itself.'
  }
] as const;

export function enabledCuratedExtensions(): readonly CuratedExtension[] {
  return CURATED_EXTENSIONS.filter((extension) => extension.enabled);
}
