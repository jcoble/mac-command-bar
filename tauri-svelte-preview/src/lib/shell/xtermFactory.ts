/**
 * The xterm seam for the /next shell.
 *
 * `LiveConversationTerminalDeps.createTerminal` is SYNCHRONOUS `(host) =>
 * TerminalView`, so every dynamic import happens once up front in
 * {@link loadXtermModules}; {@link makeTerminalView} then only constructs.
 *
 * WebGL is treated as a per-view RESOURCE, not a setting: a page may only hold
 * ~16 live WebGL contexts. So the addon is acquired on `setVisible(true)` and
 * released on `setVisible(false)` — a hidden view that is kept (a finished
 * session's, see `liveConversationTerminals`) falls back to the canvas
 * renderer, which is slower and nobody can see it.
 *
 * The terminal options below are copied from the proven old-shell config
 * (`src/routes/+page.svelte`), so /next behaves identically to the terminal
 * users already have. The COLOURS now come from the theme in force rather than
 * from a constant in this file — and the theme the app ships with carries the
 * same Dracula palette the terminal has always had, so nothing changes on screen
 * until somebody picks a different theme.
 */
import type { TerminalView } from '../liveConversationTerminals';
import { defaultSettings, settings } from '../settingsStore.svelte';
import { addXtermView } from './resourceDiagnostics.svelte.ts';
import { currentTheme, registerTerminalApplier } from './themes/themeService';

/**
 * Every terminal this factory has built and not yet disposed.
 *
 * Usually that is the visible one plus any finished session whose view is kept
 * so its last output stays readable. A theme switch has to walk all of them —
 * repainting only the visible one leaves a kept view wearing the old colours
 * until it is looked at.
 */
const liveTerminals = new Set<import('@xterm/xterm').Terminal>();

// Called at once with the theme in force, and again on every switch.
registerTerminalApplier((theme) => {
  for (const terminal of liveTerminals) {
    terminal.options.theme = { ...theme.terminal };
  }
});

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
  fontFamily: '"Google Sans Mono", "SF Mono", var(--font-mono)',
  fontSize: 15,
  lineHeight: 1.2
} as const;

/**
 * Dracula — copied verbatim from the old shell's xterm config.
 *
 * The factory does not read this any more: terminals take their colours from the
 * theme in force. It stays because `scripts/themeRegistry.test.mjs` reads it out
 * of this file to prove the registry's Dracula is the same palette, value for
 * value, that the terminal has always had. Move it and update that test.
 */
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
  const { Terminal } = await import('@xterm/xterm');
  const { FitAddon } = await import('@xterm/addon-fit');
  const { SerializeAddon } = await import('@xterm/addon-serialize');

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
    // The view's own history, in lines. The DEEP buffer is the backend's 16 MB
    // ring, which survives a hidden view and is replayed on re-attach, so the
    // view only has to hold what a reader can plausibly scroll back through.
    // That matters because xterm holds roughly cols x 8 bytes per line — 8000
    // lines is about 8 MB at 120 columns, and the resident cost is paid in
    // WebKit, where this app can least afford it.
    scrollback: 8000,
    theme: { ...currentTheme().terminal }
  });

  const fitAddon = new modules.FitAddon();
  const serializeAddon = new modules.SerializeAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(serializeAddon);
  terminal.open(host);
  liveTerminals.add(terminal);
  addXtermView(1);

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

    resize(cols: number, rows: number): void {
      // Works while the host is `display: none` — xterm's own resize does not
      // measure the DOM, which is exactly why this exists alongside `fit()`.
      // A hidden host has zero width, so `fit()` silently leaves the grid at
      // 80x24 and replayed scrollback wraps at the wrong column.
      if (disposed || !Number.isFinite(cols) || !Number.isFinite(rows)) return;
      if (cols < 1 || rows < 1) return;
      try {
        terminal.resize(Math.trunc(cols), Math.trunc(rows));
      } catch {
        // Bad geometry from a stale backend record: keep the current grid.
      }
    },

    focus(): void {
      terminal.focus();
    },

    setVisible(visible: boolean): void {
      if (disposed) return;
      if (visible) {
        // `'block'`, NEVER `''`: hosts are rendered with an inline
        // `display: none` (TerminalSurface) precisely so that a host which
        // never receives a view — or a view created while another is active —
        // stays hidden. Clearing the inline value would fall back to the
        // stylesheet default (visible) and the host, being `inset: 0`, would
        // cover the active terminal.
        host.style.display = 'block';
        acquireWebgl();
      } else {
        releaseWebgl();
        host.style.display = 'none';
      }
    },

    dispose(): void {
      if (disposed) return;
      disposed = true;
      liveTerminals.delete(terminal);
      addXtermView(-1);
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
