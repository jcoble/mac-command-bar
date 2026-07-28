/**
 * The xterm seam for the /next shell.
 *
 * `LiveConversationTerminalDeps.createTerminal` is SYNCHRONOUS `(host) =>
 * TerminalView`, so every dynamic import happens once up front in
 * {@link loadXtermModules}; {@link makeTerminalView} then only constructs.
 *
 * WebGL is treated as a per-view RESOURCE, not a setting: a page may only hold
 * ~16 live WebGL contexts, and this shell keeps N terminals alive while showing
 * one. So the addon is acquired on `setVisible(true)` and released on
 * `setVisible(false)` — the canvas renderer keeps the hidden view correct (and
 * still receiving output), it is just slower, which nobody can see.
 *
 * The terminal options below are copied from the proven old-shell config
 * (`src/routes/+page.svelte`), Dracula theme included, so /next looks and
 * behaves identically to the terminal users already have.
 */
import type { TerminalView } from '../liveConversationTerminals';
import { defaultSettings, settings } from '../settingsStore.svelte';

/** Everything {@link makeTerminalView} needs, resolved once. */
export type XtermModules = Awaited<ReturnType<typeof loadXtermModules>>;

/** Side effects the view reports back to its owner (the terminal service). */
export type ViewHooks = {
  onData(data: string): void;
  onResize(cols: number, rows: number): void;
};

/**
 * The old shell's hardcoded font stack. `settings.terminal` stores the bare
 * family name ('Google Sans Mono'); while it is untouched we keep the original
 * fallback stack rather than the bare name, so an unconfigured install renders
 * exactly as it did before.
 */
const TERMINAL_FONT_DEFAULTS = {
  fontFamily: '"Google Sans Mono", "SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace',
  fontSize: 15,
  lineHeight: 1.2
} as const;

/** Dracula — copied verbatim from the old shell's xterm config. */
const DRACULA_THEME = {
  background: '#282a36',
  foreground: '#f8f8f2',
  cursor: '#f8f8f2',
  cursorAccent: '#282a36',
  selectionBackground: '#44475a',
  black: '#000000',
  red: '#ff5555',
  green: '#50fa7b',
  yellow: '#f1fa8c',
  blue: '#bd93f9',
  magenta: '#ff79c6',
  cyan: '#8be9fd',
  white: '#bbbbbb',
  brightBlack: '#555555',
  brightRed: '#ff5555',
  brightGreen: '#50fa7b',
  brightYellow: '#f1fa8c',
  brightBlue: '#caa9fa',
  brightMagenta: '#ff79c6',
  brightCyan: '#8be9fd',
  brightWhite: '#ffffff'
} as const;

function terminalAppearance(): { fontFamily: string; fontSize: number; lineHeight: number } {
  const fallbacks = defaultSettings().terminal;
  const current = settings.terminal;
  return {
    fontFamily:
      current.fontFamily !== fallbacks.fontFamily
        ? current.fontFamily
        : TERMINAL_FONT_DEFAULTS.fontFamily,
    fontSize:
      current.fontSize !== fallbacks.fontSize ? current.fontSize : TERMINAL_FONT_DEFAULTS.fontSize,
    lineHeight:
      current.lineHeight !== fallbacks.lineHeight
        ? current.lineHeight
        : TERMINAL_FONT_DEFAULTS.lineHeight
  };
}

/**
 * Resolve the xterm constructors once. WebGL is optional: if its chunk fails to
 * load we return `null` and every view silently stays on the canvas renderer.
 */
export async function loadXtermModules(): Promise<{
  Terminal: typeof import('@xterm/xterm').Terminal;
  FitAddon: typeof import('@xterm/addon-fit').FitAddon;
  SerializeAddon: typeof import('@xterm/addon-serialize').SerializeAddon;
  WebglAddon: typeof import('@xterm/addon-webgl').WebglAddon | null;
}> {
  const [{ Terminal }, { FitAddon }, { SerializeAddon }] = await Promise.all([
    import('@xterm/xterm'),
    import('@xterm/addon-fit'),
    import('@xterm/addon-serialize')
  ]);

  let WebglAddon: typeof import('@xterm/addon-webgl').WebglAddon | null = null;
  try {
    ({ WebglAddon } = await import('@xterm/addon-webgl'));
  } catch {
    WebglAddon = null;
  }

  return { Terminal, FitAddon, SerializeAddon, WebglAddon };
}

/**
 * Build one live terminal bound to `host`. Synchronous by construction — the
 * caller passes modules already resolved by {@link loadXtermModules}.
 */
export function makeTerminalView(
  modules: XtermModules,
  host: HTMLElement,
  hooks: ViewHooks
): TerminalView {
  const appearance = terminalAppearance();

  const terminal = new modules.Terminal({
    convertEol: true,
    cursorBlink: true,
    cursorStyle: 'block',
    allowProposedApi: true,
    macOptionIsMeta: true,
    fontFamily: appearance.fontFamily,
    fontSize: appearance.fontSize,
    fontWeight: 500,
    fontWeightBold: 760,
    lineHeight: appearance.lineHeight,
    scrollback: 8000,
    theme: { ...DRACULA_THEME }
  });

  const fitAddon = new modules.FitAddon();
  const serializeAddon = new modules.SerializeAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(serializeAddon);
  terminal.open(host);

  const inputDisposable = terminal.onData((data) => hooks.onData(data));

  let webglAddon: InstanceType<NonNullable<XtermModules['WebglAddon']>> | null = null;
  let disposed = false;

  /** Acquire a FRESH WebGL context for this view. Never fatal. */
  function acquireWebgl(): void {
    const WebglAddon = modules.WebglAddon;
    if (disposed || webglAddon || !WebglAddon) return;
    try {
      const addon = new WebglAddon();
      addon.onContextLoss(() => {
        // Browser yanked the context (usually because another view took one of
        // the ~16 slots). Drop it and fall back to canvas, silently.
        addon.dispose();
        if (webglAddon === addon) webglAddon = null;
      });
      terminal.loadAddon(addon);
      webglAddon = addon;
    } catch {
      webglAddon = null;
    }
  }

  /** Release this view's WebGL context so a visible view can have it. */
  function releaseWebgl(): void {
    if (!webglAddon) return;
    const addon = webglAddon;
    webglAddon = null;
    try {
      addon.dispose();
    } catch {
      // Already gone (context loss raced us) — nothing to do.
    }
  }

  return {
    write(data: string): void {
      terminal.write(data);
    },

    fit(): void {
      try {
        fitAddon.fit();
      } catch {
        // Hidden or zero-sized host: keep the last known geometry.
      }
      hooks.onResize(terminal.cols, terminal.rows);
    },

    focus(): void {
      terminal.focus();
    },

    setVisible(visible: boolean): void {
      if (disposed) return;
      if (visible) {
        host.style.display = '';
        acquireWebgl();
      } else {
        releaseWebgl();
        host.style.display = 'none';
      }
    },

    dispose(): void {
      if (disposed) return;
      disposed = true;
      inputDisposable.dispose();
      releaseWebgl();
      try {
        serializeAddon.dispose();
      } catch {
        // xterm disposes loaded addons itself; double-dispose is harmless.
      }
      try {
        fitAddon.dispose();
      } catch {
        // Same.
      }
      terminal.dispose();
    }
  };
}
