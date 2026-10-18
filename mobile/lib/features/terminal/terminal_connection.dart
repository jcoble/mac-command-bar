import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:web_socket_channel/io.dart';

import '../../core/api/api_exception.dart';
import '../../core/machines/machine_store.dart';

/// One frame from `GET terminals/{id}/stream`: the screen to replay (sent
/// first on every connect), or output after it.
class TerminalFrame {
  const TerminalFrame({
    required this.replay,
    required this.data,
    this.terminated = false,
  });

  final bool replay;
  final String data;

  /// The shell has exited.
  final bool terminated;
}

/// The frame in a stream [message], or null for anything else.
TerminalFrame? parseTerminalFrame(Object? message) {
  if (message is! String) return null;
  final Object? json;
  try {
    json = jsonDecode(message);
  } on FormatException {
    return null;
  }
  if (json is! Map<String, dynamic> || json['data'] is! String) return null;
  final data = json['data'] as String;
  return switch (json['type']) {
    'scrollback' => TerminalFrame(replay: true, data: data),
    'output' => TerminalFrame(
      replay: false,
      data: data,
      terminated: json['terminated'] == true,
    ),
    _ => null,
  };
}

/// A session's shell on one machine. The first [resize] starts it (or finds
/// the live one) with `POST sessions/{id}/terminal` and opens its stream; a
/// dropped stream reconnects with backoff, starting again from the POST so a
/// restarted machine gets a new shell, and each connect replays the screen.
/// [dispose] leaves the shell running; [closeShell] ends it.
class TerminalConnection {
  TerminalConnection(
    this._machine,
    this._dio,
    this._ownedId, {
    required this.onFrame,
    required this.onStatus,
  });

  static const _delays = [0, 1, 2, 5, 10, 30];

  final Machine _machine;
  final Dio _dio;
  final String _ownedId;
  final void Function(TerminalFrame frame) onFrame;

  /// A line to show, or null while connected.
  final void Function(String? status) onStatus;

  IOWebSocketChannel? _channel;
  String? _terminalId;
  ({int cols, int rows})? _size;
  Timer? _retry;
  var _attempt = 0;
  var _closed = false;

  void resize(int cols, int rows) {
    final first = _size == null;
    _size = (cols: cols, rows: rows);
    if (first) {
      _connect();
    } else {
      _sendSize();
    }
  }

  /// Typed text or key bytes; dropped while disconnected.
  void input(String data) => _channel?.sink.add(jsonEncode({'input': data}));

  void _sendSize() => _channel?.sink.add(
    jsonEncode({
      'resize': {'cols': _size!.cols, 'rows': _size!.rows},
    }),
  );

  Future<void> _connect() async {
    if (_closed) return;
    onStatus(_attempt == 0 ? 'Connecting…' : 'Reconnecting…');
    try {
      final response = await _dio.post<Map<String, dynamic>>(
        'sessions/${Uri.encodeComponent(_ownedId)}/terminal',
        data: {'cols': _size!.cols, 'rows': _size!.rows},
      );
      _terminalId = response.data!['terminalId'] as String;
    } on DioException catch (e) {
      // The machine refused (no such session, folder gone): say why and stop.
      if (e.response != null) {
        if (!_closed) onStatus(ApiException.fromDioException(e).message);
        return;
      }
      _retryLater();
      return;
    }
    if (_closed) return;
    final url = Uri.parse(
      '${_machine.url}/phone/v1/terminals/${Uri.encodeComponent(_terminalId!)}/stream',
    );
    final channel = IOWebSocketChannel.connect(
      url.replace(scheme: url.scheme == 'https' ? 'wss' : 'ws'),
      headers: {'Authorization': 'Bearer ${_machine.deviceToken}'},
      pingInterval: const Duration(seconds: 30),
    );
    _channel = channel;
    try {
      await channel.ready;
    } on Object {
      if (_channel == channel) _retryLater();
      return;
    }
    if (_channel != channel) return;
    _attempt = 0;
    // The page may have changed size while it was away.
    _sendSize();
    onStatus(null);
    channel.stream.listen(
      (message) {
        final frame = parseTerminalFrame(message);
        if (frame == null) return;
        if (frame.terminated) onStatus('Shell ended');
        onFrame(frame);
      },
      onError: (Object _) {},
      onDone: () {
        if (_channel == channel) _retryLater();
      },
    );
  }

  void _retryLater() {
    _channel = null;
    if (_closed) return;
    final delay = _delays[_attempt.clamp(0, _delays.length - 1)];
    _attempt++;
    onStatus('Reconnecting…');
    _retry = Timer(Duration(seconds: delay), _connect);
  }

  /// Ends the shell on the machine (`DELETE terminals/{id}`), then [dispose].
  Future<void> closeShell() async {
    final id = _terminalId;
    dispose();
    if (id != null) {
      await _dio.delete<void>('terminals/${Uri.encodeComponent(id)}');
    }
  }

  /// Forgets the channel before closing it, so its done handler schedules nothing.
  void dispose() {
    _closed = true;
    _retry?.cancel();
    final channel = _channel;
    _channel = null;
    channel?.sink.close();
  }
}
