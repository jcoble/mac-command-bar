import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';
import '../../core/machines/machine_store.dart';

/// One run as the phone API lists it (`RunRow`).
class RunRow {
  RunRow.fromJson(Map<String, dynamic> json)
    : id = json['id'] as String,
      taskKey = json['taskKey'] as String?,
      taskTitle = json['taskTitle'] as String?,
      workflow = json['workflow'] as String? ?? '',
      state = json['state'] as String,
      currentStep = json['currentStep'] as String?,
      round = json['round'] as int? ?? 1,
      startedAt = json['startedAt'] as int? ?? 0,
      updatedAt = json['updatedAt'] as int? ?? 0,
      iMerge = json['iMerge'] == true,
      prUrl = json['prUrl'] as String?,
      waitingItemId = json['waitingItemId'] as String?;

  final String id;
  final String? taskKey;
  final String? taskTitle;
  final String workflow;

  /// The engine's run state: running, paused, waiting-input, completed, …
  final String state;
  final String? currentStep;
  final int round;
  final int startedAt;
  final int updatedAt;
  final bool iMerge;
  final String? prUrl;
  final String? waitingItemId;
}

class RunAttempt {
  RunAttempt.fromJson(Map<String, dynamic> json)
    : n = json['n'] as int? ?? 1,
      summary = (json['summary'] as String?)?.trim().nonEmpty,
      startedAt = json['startedAt'] as int?,
      endedAt = json['endedAt'] as int?,
      findings = [...?(json['findings'] as List?)?.cast<String>()],
      failingCommands = [
        for (final c in (json['failingCommands'] as List?) ?? const [])
          (
            command: '${c['command']}',
            exit: c['exit'] as int?,
            tail: c['tail'] as String? ?? '',
          ),
      ];

  final int n;
  final String? summary;
  final int? startedAt;
  final int? endedAt;
  final List<String> findings;
  final List<({String command, int? exit, String tail})> failingCommands;
}

extension on String {
  String? get nonEmpty => isEmpty ? null : this;
}

class RunStep {
  RunStep.fromJson(Map<String, dynamic> json)
    : id = json['id'] as String,
      kind = json['kind'] as String? ?? 'agent',
      state = json['state'] as String,
      attempts = [
        for (final a in (json['attempts'] as List?) ?? const [])
          RunAttempt.fromJson(a as Map<String, dynamic>),
      ];

  final String id;
  final String kind;

  /// The engine's step state: blocked, running, waiting-input, completed, …
  final String state;
  final List<RunAttempt> attempts;
}

/// `GET /runs/{id}`: the row plus branch, steps and the last 20 timeline lines.
class RunDetail {
  RunDetail.fromJson(Map<String, dynamic> json)
    : row = RunRow.fromJson(json),
      branch = json['branch'] as String?,
      steps = [
        for (final s in (json['steps'] as List?) ?? const [])
          RunStep.fromJson(s as Map<String, dynamic>),
      ],
      timeline = [
        for (final t in (json['timeline'] as List?) ?? const [])
          (at: t['at'] as int? ?? 0, text: t['text'] as String? ?? ''),
      ];

  final RunRow row;
  final String? branch;
  final List<RunStep> steps;
  final List<({int at, String text})> timeline;
}

class RunsList {
  const RunsList(this.active, this.recent);

  final List<RunRow> active;
  final List<RunRow> recent;
}

enum RunAction { pause, resume, stop }

/// The controls a run in [state] offers: the engine pauses only a running run
/// and resumes only a paused one; any unfinished run can be stopped.
List<RunAction> runActionsFor(String state) => switch (state) {
  'running' => const [RunAction.pause, RunAction.stop],
  'paused' => const [RunAction.resume, RunAction.stop],
  'completed' || 'failed' || 'cancelled' => const [],
  _ => const [RunAction.stop],
};

class RunsRepository {
  RunsRepository(this._dio);

  final Dio _dio;

  Future<RunsList> list() async {
    final body = await _get('runs');
    List<RunRow> rows(Object? list) => [
      for (final r in (list as List?) ?? const [])
        RunRow.fromJson(r as Map<String, dynamic>),
    ];
    return RunsList(rows(body['active']), rows(body['recent']));
  }

  Future<RunDetail> get(String id) async =>
      RunDetail.fromJson(await _get('runs/$id'));

  Future<void> act(String id, RunAction action) async {
    try {
      await _dio.post<void>('runs/$id/${action.name}');
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  Future<Map<String, dynamic>> _get(String path) async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(path);
      return response.data ?? const {};
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }
}

/// The selected machine's runs.
final runsRepositoryProvider = Provider<RunsRepository>((ref) {
  final machine = ref.watch(machinesProvider).requireValue.selected!;
  return RunsRepository(ref.watch(machineClientProvider(machine.id)));
});

/// Active and recent runs (Rental Command's list-notifier pattern).
class RunsNotifier extends AsyncNotifier<RunsList> {
  RunsRepository get _repo => ref.read(runsRepositoryProvider);

  @override
  Future<RunsList> build() => ref.watch(runsRepositoryProvider).list();

  /// Refetches the list, keeping the current rows on screen meanwhile. Called
  /// by pull to refresh and by a live `changed runs` notice.
  Future<void> refresh() async => state = await AsyncValue.guard(_repo.list);

  /// A live `changed run <id>` notice: refetches that run's detail if a
  /// screen shows it, and nothing else.
  void runChanged(String id) => ref.invalidate(runProvider(id));

  /// Pauses, resumes or stops [id]. Stop runs only after [confirm] says yes.
  /// Throws [ApiException] with the engine's refusal so the screen can show it.
  Future<void> act(
    String id,
    RunAction action, {
    required Future<bool> Function() confirm,
  }) async {
    if (action == RunAction.stop && !await confirm()) return;
    await _repo.act(id, action);
    runChanged(id);
    await refresh();
  }
}

final runsProvider = AsyncNotifierProvider<RunsNotifier, RunsList>(
  RunsNotifier.new,
);

final runProvider = FutureProvider.autoDispose.family<RunDetail, String>(
  (ref, id) => ref.watch(runsRepositoryProvider).get(id),
);
