import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';
import '../../core/live/live_providers.dart';

/// One chat line (spec §5.3 `ChatItem`): kind is owner, agent, relayed, tool or error.
class ChatItem {
  const ChatItem({
    required this.id,
    required this.kind,
    required this.text,
    required this.at,
    required this.done,
  });

  factory ChatItem.fromJson(Map<String, dynamic> json) => ChatItem(
    id: json['id'] as String,
    kind: json['kind'] as String,
    text: json['text'] as String,
    at: json['at'] as int,
    done: json['done'] as bool,
  );

  final String id;
  final String kind;
  final String text;

  /// Milliseconds since the epoch.
  final int at;
  final bool done;
}

class ChatState {
  const ChatState({
    required this.items,
    required this.cursor,
    required this.working,
  });

  final List<ChatItem> items;

  /// The highest event sequence applied; live frames at or below it are duplicates.
  final int cursor;

  /// The session's turn is running.
  final bool working;
}

/// One session's chat, keyed by (machine id, owned id) (Rental Command's
/// `conversationProvider` pattern). Live frames and refetches run one at a
/// time, in arrival order. While it is open the machine's live socket watches
/// this session, so its chat frames are the ones that come.
class ChatNotifier extends Notifier<AsyncValue<ChatState>> {
  ChatNotifier(this._session);

  final (String, String) _session;
  Future<void> _tail = Future.value();

  Dio get _dio => ref.read(machineClientProvider(_session.$1));

  String get _path => 'sessions/${Uri.encodeComponent(_session.$2)}/chat';

  @override
  AsyncValue<ChatState> build() {
    final socket = ref.watch(liveSocketProvider(_session.$1));
    final subscription = socket.frames.listen((f) => _enqueue(() => _apply(f)));
    socket.watch(_session.$2);
    ref.onDispose(() {
      subscription.cancel();
      socket.watch(null);
    });
    _enqueue(_fetch);
    return const AsyncValue.loading();
  }

  void _enqueue(Future<void> Function() step) => _tail = _after(_tail, step);

  Future<void> _after(
    Future<void> previous,
    Future<void> Function() step,
  ) async {
    try {
      await previous;
    } on Object {
      // That step already ended and its caller saw the error; keep the queue moving.
    }
    if (ref.mounted) await step();
  }

  /// Reloads the latest 100 items (the retry button).
  Future<void> load() {
    state = const AsyncValue.loading();
    _enqueue(_fetch);
    return _tail;
  }

  /// `GET sessions/{id}/chat`, or only what came after the cursor once loaded.
  /// Never throws.
  Future<void> _fetch() async {
    final loaded = state.value;
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        _path,
        queryParameters: {'after': ?loaded?.cursor},
      );
      if (!ref.mounted) return;
      final body = response.data!;
      final items = [
        for (final json in body['items'] as List)
          ChatItem.fromJson(json as Map<String, dynamic>),
      ];
      final cursor = body['cursor'] as int;
      state = AsyncValue.data(
        ChatState(
          items: loaded == null ? items : _merge(loaded.items, items),
          cursor: loaded == null || cursor > loaded.cursor
              ? cursor
              : loaded.cursor,
          working: body['turn'] == 'running',
        ),
      );
    } on DioException catch (e) {
      if (!ref.mounted) return;
      if (loaded == null) {
        state = AsyncValue.error(
          ApiException.fromDioException(e),
          StackTrace.current,
        );
      }
    }
  }

  Future<void> _apply(Map<String, dynamic> frame) async {
    if (frame['type'] == 'open') return _fetch();
    final current = state.value;
    final seq = frame['seq'];
    if (current == null || seq is! int || seq <= current.cursor) return;
    var items = current.items;
    var working = current.working;
    switch (frame['type']) {
      case 'chat':
        items = _merge(items, [
          ChatItem.fromJson(frame['item'] as Map<String, dynamic>),
        ]);
      case 'chatDelta':
        final id = frame['itemId'] as String;
        final delta = frame['delta'] as String;
        final at = items.indexWhere((i) => i.id == id);
        if (at < 0) {
          items = [
            ...items,
            ChatItem(
              id: id,
              kind: 'agent',
              text: delta,
              at: DateTime.now().millisecondsSinceEpoch,
              done: false,
            ),
          ];
        } else if (!items[at].done) {
          final open = items[at];
          items = [...items]
            ..[at] = ChatItem(
              id: id,
              kind: open.kind,
              text: open.text + delta,
              at: open.at,
              done: false,
            );
        }
      case 'turn':
        working = frame['state'] == 'running';
    }
    state = AsyncValue.data(
      ChatState(items: items, cursor: seq, working: working),
    );
  }

  /// `POST sessions/{id}/chat`. Shows the owner's bubble at once; the live
  /// `chat` frame for the same item replaces it. Plain chat text never answers
  /// an inbox item (D13).
  Future<void> send(String text) async {
    final response = await _dio.post<Map<String, dynamic>>(
      _path,
      data: {'text': text},
    );
    final id = response.data!['itemId'] as String;
    _enqueue(() async {
      final current = state.value;
      if (current == null || current.items.any((i) => i.id == id)) return;
      state = AsyncValue.data(
        ChatState(
          items: [
            ...current.items,
            ChatItem(
              id: id,
              kind: 'owner',
              text: text,
              at: DateTime.now().millisecondsSinceEpoch,
              done: true,
            ),
          ],
          cursor: current.cursor,
          working: current.working,
        ),
      );
    });
    await _tail;
  }
}

/// [items] with each of [updates] replacing the item with its id, or appended.
List<ChatItem> _merge(List<ChatItem> items, List<ChatItem> updates) {
  final merged = [...items];
  for (final update in updates) {
    final at = merged.indexWhere((i) => i.id == update.id);
    if (at < 0) {
      merged.add(update);
    } else {
      merged[at] = update;
    }
  }
  return merged;
}

final chatProvider = NotifierProvider.autoDispose
    .family<ChatNotifier, AsyncValue<ChatState>, (String, String)>(
      ChatNotifier.new,
    );
