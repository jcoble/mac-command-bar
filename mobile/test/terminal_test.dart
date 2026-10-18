import 'package:assembly_phone/features/terminal/terminal_connection.dart';
import 'package:assembly_phone/features/terminal/terminal_keys.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('key_bar_bytes', () {
    expect(applyLatches('c', ctrl: true), '\x03');
    expect(applyLatches('C', ctrl: true), '\x03');
    expect(applyLatches('x', alt: true), '\x1bx');
    expect(applyLatches(keySends['tab']!, shift: true), '\x1b[Z');
    expect(applyLatches('a', shift: true), 'A');
    expect(
      applyLatches('1', ctrl: true),
      '1',
      reason: 'Ctrl only maps letters',
    );
    expect(
      [
        for (final key in ['up', 'down', 'right', 'left']) keySends[key],
      ],
      ['\x1b[A', '\x1b[B', '\x1b[C', '\x1b[D'],
    );
    expect(applyLatches(keySends['up']!, shift: true), '\x1b[1;2A');
    expect(applyLatches(keySends['left']!, alt: true), '\x1b\x1b[D');
    expect((keySends['esc'], keySends['ctrl-c']), ('\x1b', '\x03'));

    // Tap once: the next key only. Tap again: stays on. A third tap: off.
    expect(Latch.off.tapped, Latch.once);
    expect(Latch.once.tapped, Latch.locked);
    expect(Latch.locked.tapped, Latch.off);
    expect((Latch.once.used, Latch.locked.used), (Latch.off, Latch.locked));
  });

  test('terminal_frames_parse', () {
    final replay = parseTerminalFrame(
      r'{"type":"scrollback","data":"\u001b[32m$ \u001b[0m"}',
    )!;
    expect(
      (replay.replay, replay.data, replay.terminated),
      (true, '\x1b[32m\$ \x1b[0m', false),
    );
    final output = parseTerminalFrame(
      r'{"type":"output","data":"hi\r\n","terminated":false,"exitCode":null}',
    )!;
    expect(
      (output.replay, output.data, output.terminated),
      (false, 'hi\r\n', false),
    );
    final ended = parseTerminalFrame(
      '{"type":"output","data":"","terminated":true,"exitCode":0}',
    )!;
    expect(ended.terminated, true);
    for (final junk in [
      'not json',
      '[]',
      '{"type":"other","data":"x"}',
      '{"type":"output"}',
      null,
    ]) {
      expect(parseTerminalFrame(junk), isNull, reason: '$junk');
    }
  });
}
