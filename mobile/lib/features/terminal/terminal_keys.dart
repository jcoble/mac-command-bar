// The key bar's bytes, adapted from code-yeongyu/web-terminal
// src/client/ui/toolbar-keys.ts and applyLatches in src/client/ui/toolbar.ts
// (MIT License, Copyright (c) 2026 YeonGyu Kim - keep this notice).

/// What each key bar key sends to the shell.
const keySends = {
  'esc': '\x1b',
  'tab': '\t',
  'up': '\x1b[A',
  'down': '\x1b[B',
  'right': '\x1b[C',
  'left': '\x1b[D',
  'ctrl-c': '\x03',
};

/// Shift variants: BackTab and xterm's CSI 1;2 arrows.
const _shifted = {
  '\t': '\x1b[Z',
  '\x1b[A': '\x1b[1;2A',
  '\x1b[B': '\x1b[1;2B',
  '\x1b[C': '\x1b[1;2C',
  '\x1b[D': '\x1b[1;2D',
};

/// [data] as it reaches the shell with the key bar's modifiers on: Shift
/// shifts keys and letters, Ctrl turns a letter into its control byte, Alt
/// puts ESC before anything.
String applyLatches(
  String data, {
  bool shift = false,
  bool ctrl = false,
  bool alt = false,
}) {
  var out = data;
  if (shift) out = _shifted[out] ?? (out.length == 1 ? out.toUpperCase() : out);
  if (ctrl && out.length == 1) {
    final code = out.toLowerCase().codeUnitAt(0);
    if (code >= 0x61 && code <= 0x7a) out = String.fromCharCode(code & 0x1f);
  }
  return alt ? '\x1b$out' : out;
}

/// A key bar modifier. Tapped once it applies to the next key only, tapped
/// again it stays on, and a third tap turns it off.
enum Latch {
  off,
  once,
  locked;

  bool get on => this != off;

  Latch get tapped => switch (this) {
    off => once,
    once => locked,
    locked => off,
  };

  /// The latch after a key has used it.
  Latch get used => this == once ? off : this;
}
