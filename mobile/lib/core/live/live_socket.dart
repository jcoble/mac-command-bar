import 'dart:async';
import 'dart:convert';

import 'package:flutter/widgets.dart';
import 'package:web_socket_channel/io.dart';

import '../machines/machine_store.dart';

/// One machine's `GET /phone/v1/live` WebSocket (spec §5.3). Rental Command's
/// SignalR reconnect delays over a plain WebSocket. Each successful connect
/// emits `{type: "open"}` so listeners refetch what they missed. The socket is
/// closed while the app is hidden and reopened when it shows again; [close]
/// stops it for good.
class LiveSocket {
  LiveSocket(this._machine) {
    _lifecycle = AppLifecycleListener(onHide: _disconnect, onShow: _connect);
    _connect();
  }

  static const _delays = [0, 2, 5, 10, 30, 60];

  final Machine _machine;
  final _frames = StreamController<Map<String, dynamic>>.broadcast();
  late final AppLifecycleListener _lifecycle;
  IOWebSocketChannel? _channel;
  Timer? _retry;
  var _attempt = 0;
  var _closed = false;

  /// Decoded server frames (`chat`, `chatDelta`, `turn`, `changed`) plus `open`.
  Stream<Map<String, dynamic>> get frames => _frames.stream;

  Future<void> _connect() async {
    if (_closed || _channel != null) return;
    _retry?.cancel();
    final url = Uri.parse('${_machine.url}/phone/v1/live');
    final channel = IOWebSocketChannel.connect(
      url.replace(scheme: url.scheme == 'https' ? 'wss' : 'ws'),
      headers: {'Authorization': 'Bearer ${_machine.deviceToken}'},
      // Notices a dead network (phone changed Wi-Fi) instead of waiting forever.
      pingInterval: const Duration(seconds: 30),
    );
    _channel = channel;
    try {
      await channel.ready;
    } on Object {
      if (_channel == channel) _dropAndRetry();
      return;
    }
    if (_channel != channel) return;
    _attempt = 0;
    _frames.add(const {'type': 'open'});
    channel.stream.listen(
      (data) {
        final frame = data is String ? jsonDecode(data) : null;
        if (frame is Map<String, dynamic>) _frames.add(frame);
      },
      onError: (Object _) {},
      onDone: () {
        if (_channel == channel) _dropAndRetry();
      },
    );
  }

  void _dropAndRetry() {
    _channel = null;
    if (_closed) return;
    final delay = _delays[_attempt.clamp(0, _delays.length - 1)];
    _attempt++;
    _retry = Timer(Duration(seconds: delay), _connect);
  }

  /// Forgets the channel before closing it: closing can run its done handler at
  /// once, and that handler must not schedule a reconnect.
  void _disconnect() {
    final channel = _channel;
    _channel = null;
    _retry?.cancel();
    _attempt = 0;
    channel?.sink.close();
  }

  void close() {
    _closed = true;
    _lifecycle.dispose();
    _disconnect();
    _frames.close();
  }
}
