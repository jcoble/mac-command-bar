import 'dart:async';
import 'dart:typed_data';

import 'package:assembly_phone/core/api/api_exception.dart';
import 'package:assembly_phone/core/api/machine_client.dart';
import 'package:assembly_phone/core/live/live_providers.dart';
import 'package:assembly_phone/features/inbox/inbox_repository.dart';
import 'package:assembly_phone/features/runs/runs_repository.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'chat_test.dart' show FakeAdapter, FakeLiveSocket;
import 'runs_test.dart' show detail, row;

Map<String, Object?> pause(String run, int createdAt) => {
  'id': 'step:$run:checks',
  'kind': 'loop_stalled',
  'runId': run,
  'taskKey': 'TSK-1415',
  'stepId': 'checks',
  'title': 'TSK-1415 · Phone loop demo',
  'detail': '1 finding(s) repeated from last round',
  'evidence': [],
  'options': [
    {'id': 'another-round', 'label': 'Another round', 'style': 'primary'},
    {'id': 'accept', 'label': 'Accept', 'style': 'neutral'},
    {'id': 'stop', 'label': 'Stop', 'style': 'danger'},
  ],
  'createdAt': createdAt,
  'version': 2,
};

const permission = {
  'id': 'perm-1',
  'kind': 'permission',
  'sessionName': 'tsk-1412/build',
  'title': 'Bash',
  'detail': 'rm -rf target/phone-api-tmp',
  'evidence': [
    {'label': 'Provider', 'text': 'codex'},
  ],
  'options': [
    {'id': 'reject-1', 'label': 'Reject', 'style': 'danger'},
    {'id': 'always-2', 'label': 'Always allow', 'style': 'neutral'},
    {'id': 'once-3', 'label': 'Allow once', 'style': 'primary'},
  ],
  'createdAt': 3000,
  'version': 7,
};

const machineId = 'm-test';

/// Each request so far as `'<METHOD> <path>'`.
List<String> sent(FakeAdapter adapter) => [
  for (final r in adapter.requests) '${r.method} ${r.path}',
];

/// Answers like [FakeAdapter], but holds the response to the next request
/// until [gate] completes (the answer is chosen when the request arrives).
class GatedAdapter extends FakeAdapter {
  GatedAdapter(super.handler);

  Completer<void>? gate;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    final response = await super.fetch(options, requestStream, cancelFuture);
    final held = gate;
    gate = null;
    if (held != null) await held.future;
    return response;
  }
}

Future<(ProviderContainer, FakeAdapter, FakeLiveSocket)> inboxWith(
  (int, Object?) Function(RequestOptions request) handler, {
  FakeAdapter Function((int, Object?) Function(RequestOptions)) makeAdapter =
      FakeAdapter.new,
}) async {
  final adapter = makeAdapter(handler);
  final dio = Dio(BaseOptions(baseUrl: 'https://m.example/phone/v1/'))
    ..httpClientAdapter = adapter;
  final socket = FakeLiveSocket();
  final container = ProviderContainer(
    overrides: [
      machineClientProvider(machineId).overrideWithValue(dio),
      liveSocketProvider(machineId).overrideWithValue(socket),
      runsRepositoryProvider.overrideWithValue(RunsRepository(dio)),
    ],
  );
  addTearDown(container.dispose);
  container.listen(inboxProvider(machineId), (_, _) {});
  await container.read(inboxProvider(machineId).future);
  return (container, adapter, socket);
}

void main() {
  test('inbox_notifier_keeps_backend_options_only', () async {
    final (c, _, _) = await inboxWith(
      (_) => (
        200,
        {
          'items': [permission, pause('r1', 2000)],
        },
      ),
    );

    final inbox = c.read(inboxProvider(machineId)).requireValue;

    expect(inbox.items.map((i) => i.id), ['perm-1', 'step:r1:checks']);
    expect(
      [for (final o in inbox.items.first.options) (o.id, o.label, o.style)],
      [
        ('reject-1', 'Reject', 'danger'),
        ('always-2', 'Always allow', 'neutral'),
        ('once-3', 'Allow once', 'primary'),
      ],
    );
    expect(inbox.items.last.options.map((o) => o.id), [
      'another-round',
      'accept',
      'stop',
    ]);
    expect(inbox.items.first.sessionName, 'tsk-1412/build');
    expect(inbox.items.first.evidence.single.text, 'codex');
    expect(inbox.pending, hasLength(2));
  });

  test('inbox_notifier_marks_stale_answer', () async {
    var answered = false;
    final (c, adapter, _) = await inboxWith(
      (r) => switch ('${r.method} ${r.path}') {
        'GET inbox' => (
          200,
          {
            'items': [
              if (!answered) pause('r1', 2000),
              pause('r2', 1000),
              permission,
            ],
          },
        ),
        'POST inbox/step%3Ar1%3Achecks/answer' => () {
          answered = true;
          return (409, {'error': 'stale'});
        }(),
        'POST inbox/step%3Ar2%3Achecks/answer' => (204, null),
        _ => (500, {'error': 'Server error'}),
      },
    );
    final notifier = c.read(inboxProvider(machineId).notifier);
    final r1 = c.read(inboxProvider(machineId)).requireValue.items.first;
    adapter.requests.clear();

    await notifier.answer(r1, 'accept');

    expect(sent(adapter), [
      'POST inbox/step%3Ar1%3Achecks/answer',
      'GET inbox',
    ]);
    expect(adapter.requests.first.data, {'optionId': 'accept', 'version': 2});
    var inbox = c.read(inboxProvider(machineId)).requireValue;
    expect(inbox.answeredElsewhere, {'step:r1:checks'});
    expect(inbox.items.map((i) => i.id), contains('step:r1:checks'));
    expect(inbox.pending.map((i) => i.id), isNot(contains('step:r1:checks')));

    // A 204 drops the card; any other refusal keeps it and reaches the screen.
    final r2 = inbox.items.firstWhere((i) => i.id == 'step:r2:checks');
    await notifier.answer(r2, 'stop');
    inbox = c.read(inboxProvider(machineId)).requireValue;
    expect(inbox.items.map((i) => i.id), isNot(contains('step:r2:checks')));

    final perm = inbox.items.firstWhere((i) => i.id == 'perm-1');
    await expectLater(
      notifier.answer(perm, 'once-3'),
      throwsA(isA<ApiException>()),
    );
    inbox = c.read(inboxProvider(machineId)).requireValue;
    expect(inbox.pending.map((i) => i.id), contains('perm-1'));
  });

  test(
    'inbox_changed_frame_refreshes_inbox_and_run_frame_refreshes_that_run',
    () async {
      final (c, adapter, socket) = await inboxWith(
        (r) => switch ('${r.method} ${r.path}') {
          'GET inbox' => (200, {'items': []}),
          'GET runs' => (
            200,
            {
              'active': [row('a', 'running'), row('b', 'running')],
              'recent': [],
            },
          ),
          'GET runs/a' => (200, detail('a')),
          'GET runs/b' => (200, detail('b')),
          _ => (404, null),
        },
      );
      c.listen(runsProvider, (_, _) {});
      c.listen(runProvider('a'), (_, _) {});
      c.listen(runProvider('b'), (_, _) {});
      await c.read(runsProvider.future);
      await c.read(runProvider('a').future);
      await c.read(runProvider('b').future);

      Future<List<String>> after(Map<String, dynamic> frame) async {
        adapter.requests.clear();
        socket.controller.add(frame);
        await pumpEventQueue();
        return sent(adapter);
      }

      expect(await after({'type': 'changed', 'entity': 'inbox'}), [
        'GET inbox',
      ]);
      expect(await after({'type': 'changed', 'entity': 'runs'}), ['GET runs']);
      expect(await after({'type': 'changed', 'entity': 'run', 'id': 'a'}), [
        'GET runs/a',
      ]);
      expect(await after({'type': 'chatDelta', 'seq': 9}), isEmpty);
    },
  );

  test('reconnect_refetches_open_run_detail', () async {
    final (c, adapter, socket) = await inboxWith(
      (r) => switch ('${r.method} ${r.path}') {
        'GET inbox' => (200, {'items': []}),
        'GET runs' => (
          200,
          {
            'active': [row('a', 'running')],
            'recent': [],
          },
        ),
        'GET runs/a' => (200, detail('a')),
        _ => (404, null),
      },
    );
    c.listen(runsProvider, (_, _) {});
    c.listen(runProvider('a'), (_, _) {});
    await c.read(runsProvider.future);
    await c.read(runProvider('a').future);
    adapter.requests.clear();

    socket.controller.add({'type': 'open'});
    await pumpEventQueue();

    expect(
      sent(adapter),
      unorderedEquals(['GET inbox', 'GET runs', 'GET runs/a']),
    );
  });

  test('older_inbox_fetch_does_not_restore_answered_card', () async {
    var answered = false;
    final (c, adapter, _) = await inboxWith(
      (r) => switch ('${r.method} ${r.path}') {
        'GET inbox' => (
          200,
          {
            'items': [if (!answered) pause('r1', 2000)],
          },
        ),
        'POST inbox/step%3Ar1%3Achecks/answer' => () {
          answered = true;
          return (204, null);
        }(),
        _ => (500, {'error': 'Server error'}),
      },
      makeAdapter: GatedAdapter.new,
    );
    final notifier = c.read(inboxProvider(machineId).notifier);
    final r1 = c.read(inboxProvider(machineId)).requireValue.items.single;

    // A fetch starts while the card is still listed, and finishes late.
    final gate = (adapter as GatedAdapter).gate = Completer<void>();
    final slow = notifier.refresh();
    await pumpEventQueue();
    await notifier.answer(r1, 'accept');
    expect(c.read(inboxProvider(machineId)).requireValue.items, isEmpty);

    gate.complete();
    await slow;

    expect(c.read(inboxProvider(machineId)).requireValue.items, isEmpty);
  });
}
