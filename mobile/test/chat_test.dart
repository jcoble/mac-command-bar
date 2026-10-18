import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:assembly_phone/core/api/machine_client.dart';
import 'package:assembly_phone/core/live/live_providers.dart';
import 'package:assembly_phone/core/live/live_socket.dart';
import 'package:assembly_phone/features/chat/chat_repository.dart';
import 'package:assembly_phone/features/chat/chat_widgets.dart';
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

  /// Every session the chat asked the socket to watch, in order.
  final watched = <String?>[];

  @override
  Stream<Map<String, dynamic>> get frames => controller.stream;

  @override
  void watch(String? ownedId) => watched.add(ownedId);

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

/// A session that is not the orchestrator.
const session = (machineId, 'lane-1');

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
  container.listen(chatProvider(session), (_, _) {});
  await pumpEventQueue();
  return (container, adapter, socket);
}

List<(String, String)> texts(ProviderContainer c) => [
  for (final i in c.read(chatProvider(session)).requireValue.items)
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
        'ownedId': 'lane-1',
        'seq': 5,
        'item': item('t1', 5, 'tool', 'start_run', done: false),
      })
      ..add({
        'type': 'chat',
        'ownedId': 'lane-1',
        'seq': 6,
        'item': item('t1', 6, 'tool', '2 runs started'),
      })
      ..add({
        'type': 'chat',
        'ownedId': 'lane-1',
        'seq': 6,
        'item': item('t1', 6, 'tool', 'duplicate'),
      })
      ..add({
        'type': 'chat',
        'ownedId': 'lane-1',
        'seq': 4,
        'item': item('old', 4, 'agent', 'already loaded'),
      })
      ..add({'type': 'turn', 'ownedId': 'lane-1', 'seq': 7, 'state': 'running'})
      ..add({
        'type': 'chat',
        'ownedId': 'lane-1',
        'seq': 8,
        'item': item('a1', 8, 'agent', 'On it.'),
      });
    await pumpEventQueue();

    expect(texts(c), [
      ('u1', 'Start the build'),
      ('t1', '2 runs started'),
      ('a1', 'On it.'),
    ]);
    final state = c.read(chatProvider(session)).requireValue;
    expect(state.cursor, 8);
    expect(state.working, isTrue);
  });

  test('chat_ignores_frames_for_another_session', () async {
    final (c, _, socket) = await chatWith(
      (_) => (
        200,
        {
          'items': [item('u1', 1, 'owner', 'Start the build')],
          'cursor': 1,
          'turn': 'idle',
          'ownerItems': [],
        },
      ),
    );
    socket.controller
      ..add({
        'type': 'chat',
        'ownedId': 'other',
        'seq': 2,
        'item': item('x1', 2, 'agent', 'for another chat'),
      })
      ..add({
        'type': 'chatDelta',
        'ownedId': 'other',
        'seq': 3,
        'itemId': 'x2',
        'delta': 'also another',
      })
      ..add({'type': 'turn', 'ownedId': 'other', 'seq': 4, 'state': 'running'})
      ..add({
        'type': 'chat',
        'ownedId': 'lane-1',
        'seq': 5,
        'item': item('a1', 5, 'agent', 'mine'),
      });
    await pumpEventQueue();

    expect(texts(c), [('u1', 'Start the build'), ('a1', 'mine')]);
    final state = c.read(chatProvider(session)).requireValue;
    expect((state.cursor, state.working), (5, false));
  });

  test('chat_first_delta_creates_agent_item_then_appends', () async {
    final (c, _, socket) = await chatWith(
      (_) => (
        200,
        {'items': [], 'cursor': 1, 'turn': 'running', 'ownerItems': []},
      ),
    );
    socket.controller
      ..add({'type': 'chatDelta', 'ownedId': 'lane-1', 'seq': 2, 'itemId': 'a2', 'delta': 'Hel'})
      ..add({'type': 'chatDelta', 'ownedId': 'lane-1', 'seq': 3, 'itemId': 'a2', 'delta': 'lo'});
    await pumpEventQueue();
    final open = c.read(chatProvider(session)).requireValue.items.single;
    expect(
      (open.id, open.kind, open.text, open.done),
      ('a2', 'agent', 'Hello', false),
    );

    socket.controller
      ..add({
        'type': 'chat',
        'ownedId': 'lane-1',
        'seq': 4,
        'item': item('a2', 4, 'agent', 'Hello there'),
      })
      ..add({'type': 'chatDelta', 'ownedId': 'lane-1', 'seq': 5, 'itemId': 'a2', 'delta': ' late'});
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

    await c.read(chatProvider(session).notifier).send('yes');

    final post = adapter.requests.singleWhere((r) => r.method == 'POST');
    expect(post.path, 'sessions/lane-1/chat');
    expect(post.data, {'text': 'yes'});
    final sent = c.read(chatProvider(session)).requireValue.items.single;
    expect((sent.id, sent.kind, sent.text), ('u9', 'owner', 'yes'));
  });

  test('typed_yes_sends_chat_text_and_answers_nothing', () async {
    final (c, adapter, _) = await chatWith(withPendingRequest);

    for (final text in ['yes', 'Yes', 'y', 'allow', 'Allow once', '1']) {
      await c.read(chatProvider(session).notifier).send(text);
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

  test('chat_for_session_uses_its_endpoints', () async {
    final (c, adapter, _) = await chatWith(
      (r) => r.method == 'POST'
          ? (200, {'itemId': 'u1'})
          : (200, {'items': [], 'cursor': 1, 'turn': 'idle', 'ownerItems': []}),
    );

    await c.read(chatProvider(session).notifier).send('Reply with pong');

    expect(
      [for (final r in adapter.requests) (r.method, r.path)],
      [('GET', 'sessions/lane-1/chat'), ('POST', 'sessions/lane-1/chat')],
    );
  });

  test('opening_a_session_sends_watch_frame', () async {
    final (c, _, socket) = await chatWith(
      (_) =>
          (200, {'items': [], 'cursor': 1, 'turn': 'idle', 'ownerItems': []}),
    );
    expect(socket.watched, ['lane-1']);

    // Closing the chat stops watching it.
    c.dispose();
    expect(socket.watched, ['lane-1', null]);
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
    final state = c.read(chatProvider(session)).requireValue;
    expect((state.cursor, state.working), (15, false));
  });

  test('tool_item_parses_detail_fields', () async {
    final (c, adapter, _) = await chatWith(
      (r) => r.path.contains('/items/')
          ? (
              200,
              {
                'name': 'Edit',
                'state': 'completed',
                'summary': 'notes.txt',
                'path': '/work/notes.txt',
                'output': 'Wrote 2 lines',
                'outputTruncated': true,
                'diff': '+one\n',
                'diffTruncated': false,
              },
            )
          : (
              200,
              {
                'items': [
                  {...item('t1', 2, 'tool', 'notes.txt'), 'hasDetail': true},
                  item('t2', 3, 'tool', 'start_run'),
                ],
                'cursor': 3,
                'turn': 'idle',
                'ownerItems': [],
              },
            ),
    );
    final items = c.read(chatProvider(session)).requireValue.items;
    expect([for (final i in items) i.hasDetail], [true, false]);

    final tool = await c.read(chatProvider(session).notifier).toolItem('t 1');
    expect(adapter.requests.last.path, 'sessions/lane-1/items/t%201');
    expect(
      (tool.name, tool.state, tool.summary, tool.path, tool.output),
      ('Edit', 'completed', 'notes.txt', '/work/notes.txt', 'Wrote 2 lines'),
    );
    expect(
      (tool.outputTruncated, tool.diff, tool.diffTruncated),
      (true, '+one\n', false),
    );
  });

  test('diff_lines_classify_added_removed_hunk_and_header', () {
    expect(
      [
        for (final line in [
          'diff --git a/n b/n',
          '--- a/notes.txt',
          '+++ b/notes.txt',
          '@@ -1,2 +1,2 @@',
          '+two',
          '-one',
          ' same',
          '',
        ])
          diffLineKind(line),
      ],
      [
        DiffLineKind.header,
        DiffLineKind.header,
        DiffLineKind.header,
        DiffLineKind.hunk,
        DiffLineKind.added,
        DiffLineKind.removed,
        DiffLineKind.context,
        DiffLineKind.context,
      ],
    );
  });

  test('changes_parses_files_and_counts', () async {
    const diff =
        'diff --git a/a.txt b/a.txt\n@@ -1 +1,2 @@\n-one\n+two\n+three\n';
    final (c, adapter, _) = await chatWith(
      (r) => r.path.endsWith('/changes')
          ? (
              200,
              {
                'base': 'origin/main',
                'files': [
                  {'path': 'a.txt', 'added': 2, 'removed': 1},
                  {'path': 'src/b.rs', 'added': 3, 'removed': 0},
                ],
                'diff': diff,
                'diffTruncated': true,
              },
            )
          : (200, {'items': [], 'cursor': 0, 'turn': 'idle', 'ownerItems': []}),
    );
    final changes = await c.read(chatProvider(session).notifier).changes();
    expect(adapter.requests.last.path, 'sessions/lane-1/changes');
    expect(
      [for (final f in changes.files) (f.path, f.added, f.removed)],
      [('a.txt', 2, 1), ('src/b.rs', 3, 0)],
    );
    expect(
      (changes.base, changes.added, changes.removed, changes.diffTruncated),
      ('origin/main', 5, 1, true),
    );
    expect(changes.diffs, [
      diff,
    ], reason: 'the cut diff holds only the first file');
  });

  test('changes_splits_diff_per_file', () {
    const a =
        'diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-x\n+diff --git y\n';
    const b = 'diff --git a/b.txt b/b.txt\nBinary files differ\n';
    expect(splitDiffFiles('$a$b'), [a, b]);
    expect(splitDiffFiles(''), isEmpty);
  });

  test('session_config_parses_models_efforts_and_current', () async {
    final (c, adapter, _) = await chatWith(
      (r) => r.path.endsWith('/config')
          ? (
              r.method == 'POST' ? 204 : 200,
              r.method == 'POST'
                  ? null
                  : {
                      'models': [
                        {'id': 'default', 'label': 'Default (recommended)'},
                        {'id': 'claude-opus-5', 'label': null},
                        {'id': 'gpt-5.6-sol', 'label': null},
                      ],
                      'model': 'claude-opus-5',
                      'efforts': ['low', 'high', 'xhigh'],
                      'effort': 'xhigh',
                    },
            )
          : (200, {'items': [], 'cursor': 0, 'turn': 'idle', 'ownerItems': []}),
    );
    final config = await c.read(sessionConfigProvider(session).future);
    expect(adapter.requests.last.path, 'sessions/lane-1/config');
    expect(
      [for (final m in config.models) (m.id, m.label)],
      [
        ('default', 'Default (recommended)'),
        ('claude-opus-5', 'Opus 5'),
        ('gpt-5.6-sol', 'GPT-5.6 Sol'),
      ],
      reason: 'a model with no label is worded as the desktop words it',
    );
    expect((config.model, config.effort), ('claude-opus-5', 'xhigh'));
    expect(config.efforts, ['low', 'high', 'xhigh']);
    expect(config.modelName, 'Opus 5');

    await c.read(chatProvider(session).notifier).setConfig(model: 'default');
    final post = adapter.requests.last;
    expect((post.method, post.path), ('POST', 'sessions/lane-1/config'));
    expect(post.data, {'model': 'default'}, reason: 'only the field chosen');
  });

  test('context_left_percent_from_usage', () async {
    expect(
      contextLeftPercent({'usedTokens': 76000, 'contextWindow': 200000}),
      62,
    );
    expect(
      contextLeftPercent({'usedTokens': 250000, 'contextWindow': 200000}),
      0,
    );
    for (final usage in [
      null,
      {'usedTokens': null, 'contextWindow': 200000},
      {'usedTokens': 5, 'contextWindow': null},
      {'usedTokens': 5, 'contextWindow': 0},
    ]) {
      expect(contextLeftPercent(usage), isNull, reason: '$usage');
    }

    final (c, _, _) = await chatWith(
      (r) => (
        200,
        {
          'items': [],
          'cursor': 4,
          'turn': 'idle',
          'ownerItems': [],
          'usage': {'usedTokens': 76000, 'contextWindow': 200000},
        },
      ),
    );
    expect(c.read(chatProvider(session)).requireValue.contextLeft, 62);
  });
}
