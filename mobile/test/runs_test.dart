import 'dart:convert';
import 'dart:typed_data';

import 'package:assembly_phone/features/runs/runs_repository.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

Map<String, Object?> row(String id, String state) => {
  'id': id,
  'taskKey': 'TSK-$id',
  'taskTitle': 'Run $id',
  'workflow': 'dev-pipeline',
  'state': state,
  'currentStep': 'build',
  'round': 2,
  'startedAt': 1000,
  'updatedAt': 2000,
  'iMerge': false,
  'prUrl': null,
  'waitingItemId': null,
};

Map<String, Object?> detail(String id) => {
  ...row(id, 'running'),
  'branch': 'tsk-$id',
  'steps': [
    {
      'id': 'build',
      'kind': 'run',
      'state': 'running',
      'attempts': [
        {'n': 2, 'state': 'running', 'startedAt': 1500},
      ],
    },
  ],
  'timeline': [
    {'at': 1500, 'text': 'build: started'},
  ],
};

/// Answers the phone API from [routes] (`'GET runs'` → body) and records each
/// request as `'<METHOD> <path>'`.
class FakeMachine implements HttpClientAdapter {
  FakeMachine(this.routes);

  final Map<String, Object?> routes;
  final requests = <String>[];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    final key = '${options.method} ${options.path}';
    requests.add(key);
    return ResponseBody.fromString(
      jsonEncode(routes[key]),
      routes.containsKey(key) ? 200 : 404,
      headers: {
        Headers.contentTypeHeader: [Headers.jsonContentType],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

(ProviderContainer, FakeMachine) phone(Map<String, Object?> routes) {
  final machine = FakeMachine(routes);
  final container = ProviderContainer(
    overrides: [
      runsRepositoryProvider.overrideWithValue(
        RunsRepository(Dio()..httpClientAdapter = machine),
      ),
    ],
  );
  addTearDown(container.dispose);
  return (container, machine);
}

void main() {
  test('runs_split_active_and_recent', () async {
    final (container, _) = phone({
      'GET runs': {
        'active': [row('a', 'running'), row('b', 'paused')],
        'recent': [row('c', 'completed')],
      },
    });

    final runs = await container.read(runsProvider.future);

    expect(runs.active.map((r) => r.id), ['a', 'b']);
    expect(runs.recent.map((r) => r.id), ['c']);
    expect(runs.active.first.taskKey, 'TSK-a');
    expect(runs.active.first.currentStep, 'build');
  });

  test('run_actions_offer_pause_or_resume_by_state', () {
    expect(runActionsFor('running'), [RunAction.pause, RunAction.stop]);
    expect(runActionsFor('paused'), [RunAction.resume, RunAction.stop]);
    // The engine pauses only a running run; a run waiting on the owner can
    // only be stopped.
    expect(runActionsFor('waiting-input'), [RunAction.stop]);
    expect(runActionsFor('waiting-approval'), [RunAction.stop]);
    for (final done in ['completed', 'failed', 'cancelled']) {
      expect(runActionsFor(done), isEmpty, reason: done);
    }
  });

  test('stop_action_requires_confirmation', () async {
    final (container, machine) = phone({
      'GET runs': {
        'active': [row('a', 'running')],
        'recent': [],
      },
      'GET runs/a': detail('a'),
      'POST runs/a/stop': row('a', 'cancelled'),
      'POST runs/a/pause': row('a', 'paused'),
    });
    final runs = container.read(runsProvider.notifier);
    var asked = 0;

    await runs.act(
      'a',
      RunAction.stop,
      confirm: () async {
        asked++;
        return false;
      },
    );
    expect(asked, 1);
    expect(machine.requests, isNot(contains('POST runs/a/stop')));

    await runs.act('a', RunAction.stop, confirm: () async => true);
    expect(machine.requests, contains('POST runs/a/stop'));

    await runs.act(
      'a',
      RunAction.pause,
      confirm: () async {
        asked++;
        return false;
      },
    );
    expect(asked, 1, reason: 'pause does not ask');
    expect(machine.requests, contains('POST runs/a/pause'));
  });

  test('changed_run_notice_refetches_only_that_run', () async {
    final (container, machine) = phone({
      'GET runs': {
        'active': [row('a', 'running'), row('b', 'running')],
        'recent': [],
      },
      'GET runs/a': detail('a'),
      'GET runs/b': detail('b'),
    });
    container.listen(runsProvider, (_, _) {});
    container.listen(runProvider('a'), (_, _) {});
    container.listen(runProvider('b'), (_, _) {});
    await container.read(runsProvider.future);
    await container.read(runProvider('a').future);
    await container.read(runProvider('b').future);
    machine.requests.clear();

    container.read(runsProvider.notifier).runChanged('a');
    await container.read(runProvider('a').future);
    await container.read(runProvider('b').future);

    expect(machine.requests, ['GET runs/a']);
  });
}
