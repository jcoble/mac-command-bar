import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:assembly_phone/core/api/machine_client.dart';
import 'package:assembly_phone/core/live/live_providers.dart';
import 'package:assembly_phone/core/live/live_socket.dart';
import 'package:assembly_phone/features/chat/chat_repository.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// Answers each request with [handler] and records it.
class FakeAdapter implements HttpClientAdapter {
  FakeAdapter(this.handler);

  final (int, Object?) Function(RequestOptions request) handler;
  final requests = <RequestOptions>[];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(options);
    final (status, body) = handler(options);
    return ResponseBody.fromString(
      body == null ? '' : jsonEncode(body),
      status,
      headers: {
        Headers.contentTypeHeader: [Headers.jsonContentType],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

/// A live socket whose frames the test pushes by hand.
class FakeLiveSocket implements LiveSocket {
  final controller = StreamController<Map<String, dynamic>>.broadcast();

  @override
  Stream<Map<String, dynamic>> get frames => controller.stream;

  @override
  void close() => controller.close();
}

Map<String, dynamic> item(
  String id,
  int seq,
  String kind,
  String text, {
  bool done = true,
}) => {
  'id': id,
  'seq': seq,
  'kind': kind,
  'text': text,
  'at': seq,
  'done': done,
};

/// A pending permission request for the orchestrator itself (D13: only a tap answers it).
const ownerItem = {
  'id': 'perm-1',
  'kind': 'permission',
  'title': 'Run a command',
  'detail': 'rm -rf target',
  'evidence': [],
  'options': [
    {'id': 'allow', 'label': 'Allow once', 'style': 'primary'},
  ],
  'createdAt': 1,
  'version': 1,
};

const machineId = 'm-workbox';

Future<(ProviderContainer, FakeAdapter, FakeLiveSocket)> chatWith(
  (int, Object?) Function(RequestOptions request) handler,
) async {
  final adapter = FakeAdapter(handler);
  final socket = FakeLiveSocket();
  final container = ProviderContainer(
    overrides: [
      machineClientProvider(machineId).overrideWithValue(
        Dio(BaseOptions(baseUrl: 'https://m.example/phone/v1/'))
          ..httpClientAdapter = adapter,
      ),
      liveSocketProvider(machineId).overrideWithValue(socket),
    ],
  );
  addTearDown(container.dispose);
  container.listen(chatProvider(machineId), (_, _) {});
  await pumpEventQueue();
  return (container, adapter, socket);
}

List<(String, String)> texts(ProviderContainer c) => [
  for (final i in c.read(chatProvider(machineId)).requireValue.items)
    (i.id, i.text),
];

void main() {
  test('chat_applies_live_items_in_seq_order_and_ignores_duplicates', () async {
    final (c, _, socket) = await chatWith(
      (_) => (
        200,
        {
          'items': [item('u1', 3, 'owner', 'Start the build')],
          'cursor': 4,
          'turn': 'idle',
          'ownerItems': [],
        },
      ),
    );
    socket.controller
      ..add({
        'type': 'chat',
        'seq': 5,
        'item': item('t1', 5, 'tool', 'start_run', done: false),
      })
      ..add({
        'type': 'chat',
        'seq': 6,
        'item': item('t1', 6, 'tool', '2 runs started'),
      })
      ..add({
        'type': 'chat',
        'seq': 6,
        'item': item('t1', 6, 'tool', 'duplicate'),
      })
      ..add({
        'type': 'chat',
        'seq': 4,
        'item': item('old', 4, 'agent', 'already loaded'),
      })
      ..add({'type': 'turn', 'seq': 7, 'state': 'running'})
      ..add({
        'type': 'chat',
        'seq': 8,
        'item': item('a1', 8, 'agent', 'On it.'),
      });
    await pumpEventQueue();

    expect(texts(c), [
      ('u1', 'Start the build'),
      ('t1', '2 runs started'),
      ('a1', 'On it.'),
    ]);
    final state = c.read(chatProvider(machineId)).requireValue;
    expect(state.cursor, 8);
    expect(state.working, isTrue);
  });

  test('chat_first_delta_creates_agent_item_then_appends', () async {
    final (c, _, socket) = await chatWith(
      (_) => (
        200,
        {'items': [], 'cursor': 1, 'turn': 'running', 'ownerItems': []},
      ),
    );
    socket.controller
      ..add({'type': 'chatDelta', 'seq': 2, 'itemId': 'a2', 'delta': 'Hel'})
      ..add({'type': 'chatDelta', 'seq': 3, 'itemId': 'a2', 'delta': 'lo'});
    await pumpEventQueue();
    final open = c.read(chatProvider(machineId)).requireValue.items.single;
    expect(
      (open.id, open.kind, open.text, open.done),
      ('a2', 'agent', 'Hello', false),
    );

    socket.controller
      ..add({
        'type': 'chat',
        'seq': 4,
        'item': item('a2', 4, 'agent', 'Hello there'),
      })
      ..add({'type': 'chatDelta', 'seq': 5, 'itemId': 'a2', 'delta': ' late'});
    await pumpEventQueue();
    expect(texts(c), [('a2', 'Hello there')]);
  });

  (int, Object?) withPendingRequest(RequestOptions r) => r.method == 'POST'
      ? (200, {'itemId': 'u9'})
      : (
          200,
          {
            'items': [],
            'cursor': 1,
            'turn': 'idle',
            'ownerItems': [ownerItem],
          },
        );

  test('typing_yes_in_chat_sends_text_not_an_answer', () async {
    final (c, adapter, _) = await chatWith(withPendingRequest);

    await c.read(chatProvider(machineId).notifier).send('yes');

    final post = adapter.requests.singleWhere((r) => r.method == 'POST');
    expect(post.path, 'chat');
    expect(post.data, {'text': 'yes'});
    final sent = c.read(chatProvider(machineId)).requireValue.items.single;
    expect((sent.id, sent.kind, sent.text), ('u9', 'owner', 'yes'));
  });

  test('typed_yes_sends_chat_text_and_answers_nothing', () async {
    final (c, adapter, _) = await chatWith(withPendingRequest);

    for (final text in ['yes', 'Yes', 'y', 'allow', 'Allow once', '1']) {
      await c.read(chatProvider(machineId).notifier).send(text);
    }

    expect(adapter.requests.where((r) => r.path.contains('inbox')), isEmpty);
    expect(
      [
        for (final r in adapter.requests.where((r) => r.method == 'POST'))
          r.data,
      ],
      [
        for (final text in ['yes', 'Yes', 'y', 'allow', 'Allow once', '1'])
          {'text': text},
      ],
    );
  });

  test('reconnect_refetches_after_cursor', () async {
    final (c, adapter, socket) = await chatWith(
      (r) => r.queryParameters['after'] == null
          ? (
              200,
              {
                'items': [
                  item('u1', 10, 'owner', 'Hi'),
                  item('a1', 12, 'agent', 'Hel', done: false),
                ],
                'cursor': 12,
                'turn': 'running',
                'ownerItems': [],
              },
            )
          : (
              200,
              {
                'items': [item('a1', 14, 'agent', 'Hello')],
                'cursor': 15,
                'turn': 'idle',
                'ownerItems': [],
              },
            ),
    );

    socket.controller.add({'type': 'open'});
    await pumpEventQueue();

    expect(adapter.requests.last.queryParameters, {'after': 12});
    expect(texts(c), [('u1', 'Hi'), ('a1', 'Hello')]);
    final state = c.read(chatProvider(machineId)).requireValue;
    expect((state.cursor, state.working), (15, false));
  });
}
