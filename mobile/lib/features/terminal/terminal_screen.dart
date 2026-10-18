import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:webview_flutter/webview_flutter.dart';
import 'package:webview_flutter_android/webview_flutter_android.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';
import '../../core/machines/machine_store.dart';
import 'terminal_connection.dart';
import 'terminal_keys.dart';

// The desktop terminal's Dracula colours, so the key bar reads as part of it.
const _background = Color(0xFF282A36);
const _bar = Color(0xFF21222C);
const _key = Color(0xFF343746);
const _text = Color(0xFFF8F8F2);
const _accent = Color(0xFFBD93F9);

/// MainActivity's channel that shows the keyboard for the WebView.
const _keyboardChannel = MethodChannel('assembly/terminal');

/// A session's shell, full screen: ghostty-web in a WebView draws it, and a key
/// bar sits above the keyboard. The WebView lives only while this page is open;
/// leaving keeps the shell running on the machine, and its screen comes back
/// next time.
class TerminalScreen extends ConsumerStatefulWidget {
  const TerminalScreen({super.key, required this.ownedId, this.title});

  final String ownedId;
  final String? title;

  @override
  ConsumerState<TerminalScreen> createState() => _TerminalScreenState();
}

class _TerminalScreenState extends ConsumerState<TerminalScreen> {
  late final TerminalConnection _connection;
  late final WebViewController _web;
  final _status = ValueNotifier<String?>('Connecting…');
  var _shift = Latch.off;
  var _ctrl = Latch.off;
  var _alt = Latch.off;

  @override
  void initState() {
    super.initState();
    final machine = ref.read(machinesProvider).requireValue.selected!;
    _connection = TerminalConnection(
      machine,
      ref.read(machineClientProvider(machine.id)),
      widget.ownedId,
      onFrame: _show,
      onStatus: (status) => _status.value = status,
    );
    _web = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(_background)
      ..addJavaScriptChannel(
        'TerminalBridge',
        onMessageReceived: (message) => _fromPage(message.message),
      )
      ..loadFlutterAsset('assets/terminal/index.html');
  }

  @override
  void dispose() {
    _connection.dispose();
    // Frees the page's wasm and canvas now rather than when the WebView is collected.
    unawaited(_web.loadRequest(Uri.parse('about:blank')));
    _status.dispose();
    super.dispose();
  }

  void _show(TerminalFrame frame) => unawaited(
    _web.runJavaScript(
      '${frame.replay ? 'termReplay' : 'termWrite'}(${jsonEncode(frame.data)})',
    ),
  );

  void _fromPage(String message) {
    final frame = jsonDecode(message) as Map<String, dynamic>;
    if (frame['resize'] case {'cols': final int cols, 'rows': final int rows}) {
      _connection.resize(cols, rows);
    } else if (frame['input'] case final String input) {
      _type(input);
    } else if (frame['error'] case final String error) {
      _status.value = error;
    }
  }

  /// Sends [data] with the key bar's modifiers, then clears the one-shot ones.
  void _type(String data) {
    if (_shift.on || _ctrl.on || _alt.on) {
      data = applyLatches(data, shift: _shift.on, ctrl: _ctrl.on, alt: _alt.on);
      setState(() {
        _shift = _shift.used;
        _ctrl = _ctrl.used;
        _alt = _alt.used;
      });
    }
    _connection.input(data);
  }

  Future<void> _paste() async {
    final text = (await Clipboard.getData(Clipboard.kTextPlain))?.text;
    // Through the page, so the shell gets bracketed paste when it asked for it.
    if (text != null && text.isNotEmpty && mounted) {
      await _web.runJavaScript('termPaste(${jsonEncode(text)})');
    }
  }

  /// Focuses the terminal's input, then has MainActivity show the keyboard:
  /// the page alone can't before the first tap inside the WebView.
  Future<void> _showKeyboard() async {
    await _web.runJavaScript('termFocus()');
    if (_web.platform case final AndroidWebViewController android) {
      await _keyboardChannel.invokeMethod<bool>(
        'showKeyboard',
        android.webViewIdentifier,
      );
    }
  }

  Future<void> _closeShell() async {
    final navigator = Navigator.of(context);
    final messenger = ScaffoldMessenger.of(context);
    try {
      await _connection.closeShell();
    } on DioException catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text(ApiException.fromDioException(e).message)),
      );
    }
    navigator.pop();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: _background,
      appBar: AppBar(
        centerTitle: true,
        title: Column(
          children: [
            Text(
              widget.title ?? 'Terminal',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontSize: 17),
            ),
            ValueListenableBuilder(
              valueListenable: _status,
              builder: (context, status, _) => Text(
                status ?? 'Terminal',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontSize: 13,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
        actions: [
          PopupMenuButton<void>(
            itemBuilder: (_) => [
              PopupMenuItem(
                onTap: _closeShell,
                child: const Text('Close shell'),
              ),
            ],
          ),
        ],
      ),
      body: Column(
        children: [
          Expanded(child: WebViewWidget(controller: _web)),
          ColoredBox(
            color: _bar,
            child: SafeArea(
              top: false,
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
                child: Row(
                  children: [
                    _KeyCap(
                      label: 'Esc',
                      onPress: () => _type(keySends['esc']!),
                    ),
                    _KeyCap(
                      label: 'Tab',
                      onPress: () => _type(keySends['tab']!),
                    ),
                    _KeyCap(
                      label: '⇧',
                      latch: _shift,
                      onPress: () => setState(() => _shift = _shift.tapped),
                    ),
                    _KeyCap(
                      label: 'Ctrl',
                      latch: _ctrl,
                      onPress: () => setState(() => _ctrl = _ctrl.tapped),
                    ),
                    _KeyCap(
                      label: 'Alt',
                      latch: _alt,
                      onPress: () => setState(() => _alt = _alt.tapped),
                    ),
                    for (final (key, icon) in [
                      ('up', Symbols.arrow_upward_rounded),
                      ('down', Symbols.arrow_downward_rounded),
                      ('left', Symbols.arrow_back_rounded),
                      ('right', Symbols.arrow_forward_rounded),
                    ])
                      _KeyCap(
                        icon: icon,
                        repeats: true,
                        onPress: () => _type(keySends[key]!),
                      ),
                    for (final symbol in ['|', '~', '/', '-'])
                      _KeyCap(label: symbol, onPress: () => _type(symbol)),
                    _KeyCap(
                      label: '^C',
                      repeats: true,
                      onPress: () => _type(keySends['ctrl-c']!),
                    ),
                    _KeyCap(
                      icon: Symbols.content_paste_rounded,
                      onPress: _paste,
                    ),
                    _KeyCap(
                      icon: Symbols.keyboard_rounded,
                      onPress: () => unawaited(_showKeyboard()),
                    ),
                    _KeyCap(
                      icon: Symbols.keyboard_hide_rounded,
                      onPress: () => _web.runJavaScript('termBlur()'),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// One key bar key. It acts on pointer down so the keyboard never closes, and
/// a held [repeats] key repeats after 400 ms every 60 ms (web-terminal's timing).
class _KeyCap extends StatefulWidget {
  const _KeyCap({
    this.label,
    this.icon,
    this.latch = Latch.off,
    this.repeats = false,
    required this.onPress,
  });

  final String? label;
  final IconData? icon;
  final Latch latch;
  final bool repeats;
  final VoidCallback onPress;

  @override
  State<_KeyCap> createState() => _KeyCapState();
}

class _KeyCapState extends State<_KeyCap> {
  Timer? _delay;
  Timer? _repeat;

  void _stop([Object? _]) {
    _delay?.cancel();
    _repeat?.cancel();
    _delay = null;
    _repeat = null;
  }

  @override
  void dispose() {
    _stop();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final latch = widget.latch;
    final colour = latch == Latch.locked
        ? _background
        : (latch.on ? _accent : _text);
    return Listener(
      onPointerDown: (_) {
        widget.onPress();
        if (!widget.repeats) return;
        _stop();
        _delay = Timer(const Duration(milliseconds: 400), () {
          _repeat = Timer.periodic(
            const Duration(milliseconds: 60),
            (_) => widget.onPress(),
          );
        });
      },
      onPointerUp: _stop,
      onPointerCancel: _stop,
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 2),
        constraints: const BoxConstraints(minWidth: 44),
        height: 40,
        padding: const EdgeInsets.symmetric(horizontal: 10),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: latch == Latch.locked ? _accent : _key,
          borderRadius: BorderRadius.circular(8),
          border: latch == Latch.once ? Border.all(color: _accent) : null,
        ),
        child: widget.icon != null
            ? Icon(widget.icon, size: 20, color: colour)
            : Text(
                widget.label!,
                style: TextStyle(
                  color: colour,
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                ),
              ),
      ),
    );
  }
}
