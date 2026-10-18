import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:assembly_phone/features/runs/runs_repository.dart';
import 'package:assembly_phone/features/runs/runs_screen.dart';
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

/// Holds which machine's repository the test has "selected".
class _Selected extends Notifier<RunsRepository> {
  @override
  RunsRepository build() => throw UnimplementedError();

  void select(RunsRepository repo) => state = repo;
}

/// Answers `GET runs` at once until [hold] is set, then only when [release]
/// completes.
class GatedMachine extends FakeMachine {
  GatedMachine(super.routes);

  final release = Completer<void>();
  var hold = false;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    if (hold) await release.future;
    return super.fetch(options, requestStream, cancelFuture);
  }
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

  test('retry_offered_only_for_failed_step', () async {
    RunStep step(String state) =>
        RunStep.fromJson({'id': 'checks', 'state': state, 'attempts': []});
    expect(canRetry(step('failed')), isTrue);
    for (final state in ['running', 'completed', 'waiting-input', 'blocked']) {
      expect(canRetry(step(state)), isFalse, reason: state);
    }

    final (container, machine) = phone({
      'GET runs': {
        'active': [],
        'recent': [row('a', 'failed')],
      },
      'POST runs/a/steps/checks/retry': row('a', 'running'),
    });
    container.listen(runsProvider, (_, _) {});
    await container.read(runsProvider.future);
    machine.requests.clear();

    await container.read(runsProvider.notifier).retryStep('a', 'checks');

    expect(machine.requests, ['POST runs/a/steps/checks/retry', 'GET runs']);
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

  test('refresh_result_for_previous_machine_is_dropped', () async {
    Map<String, Object?> runs(String id) => {
      'GET runs': {
        'active': [row(id, 'running')],
        'recent': [],
      },
    };
    final machineA = FakeMachine(runs('a'));
    final machineB = FakeMachine(runs('b'));
    final slowA = GatedMachine(runs('a'));
    final selected = NotifierProvider<_Selected, RunsRepository>(_Selected.new);
    final container = ProviderContainer(
      overrides: [
        selected.overrideWithBuild(
          (ref, _) => RunsRepository(Dio()..httpClientAdapter = machineA),
        ),
        runsRepositoryProvider.overrideWith((ref) => ref.watch(selected)),
      ],
    );
    addTearDown(container.dispose);
    container.listen(runsProvider, (_, _) {});
    await container.read(runsProvider.future);

    // Machine A's pull to refresh is still in flight when the user switches.
    container
        .read(selected.notifier)
        .select(RunsRepository(Dio()..httpClientAdapter = slowA));
    await container.read(runsProvider.future);
    slowA.hold = true;
    final refresh = container.read(runsProvider.notifier).refresh();
    container
        .read(selected.notifier)
        .select(RunsRepository(Dio()..httpClientAdapter = machineB));
    await container.read(runsProvider.future);
    slowA.release.complete();
    await refresh;

    expect(container.read(runsProvider).requireValue.active.map((r) => r.id), [
      'b',
    ]);
  });

  test('machine_switch_does_not_show_previous_machine_runs', () async {
    Map<String, Object?> runs(String id) => {
      'GET runs': {
        'active': [row(id, 'running')],
        'recent': [],
      },
    };
    final slowB = GatedMachine(runs('b'))..hold = true;
    final selected = NotifierProvider<_Selected, RunsRepository>(_Selected.new);
    final container = ProviderContainer(
      overrides: [
        selected.overrideWithBuild(
          (ref, _) =>
              RunsRepository(Dio()..httpClientAdapter = FakeMachine(runs('a'))),
        ),
        runsRepositoryProvider.overrideWith((ref) => ref.watch(selected)),
      ],
    );
    addTearDown(container.dispose);
    container.listen(runsProvider, (_, _) {});
    await container.read(runsProvider.future);
    expect(showsValue(container.read(runsProvider)), isTrue);

    // Switch to machine B while its list is still loading: A's rows are
    // still held by the provider but the screen must not draw them.
    container
        .read(selected.notifier)
        .select(RunsRepository(Dio()..httpClientAdapter = slowB));
    expect(container.read(runsProvider).isLoading, isTrue);
    expect(showsValue(container.read(runsProvider)), isFalse);

    slowB.release.complete();
    await container.read(runsProvider.future);
    expect(showsValue(container.read(runsProvider)), isTrue);
    expect(container.read(runsProvider).requireValue.active.map((r) => r.id), [
      'b',
    ]);

    // Machine C fails to answer: the screen reaches its error branch rather
    // than showing B's rows.
    container
        .read(selected.notifier)
        .select(RunsRepository(Dio()..httpClientAdapter = FakeMachine({})));
    while (!container.read(runsProvider).hasError) {
      await Future<void>.delayed(const Duration(milliseconds: 10));
    }
    expect(showsValue(container.read(runsProvider)), isFalse);
  });
}
