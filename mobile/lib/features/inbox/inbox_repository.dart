import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';
import '../../core/live/live_providers.dart';
import '../runs/runs_repository.dart';

/// One thing waiting for the owner (spec §5.3 `InboxItem`).
class InboxItem {
  InboxItem.fromJson(Map<String, dynamic> json)
    : id = json['id'] as String,
      kind = json['kind'] as String,
      runId = json['runId'] as String?,
      taskKey = json['taskKey'] as String?,
      stepId = json['stepId'] as String?,
      sessionName = json['sessionName'] as String?,
      title = json['title'] as String? ?? '',
      detail = json['detail'] as String? ?? '',
      evidence = [
        for (final e in (json['evidence'] as List?) ?? const [])
          (
            label: '${e['label']}',
            text: e['text'] as String?,
            url: e['url'] as String?,
          ),
      ],
      options = [
        for (final o in (json['options'] as List?) ?? const [])
          (id: '${o['id']}', label: '${o['label']}', style: '${o['style']}'),
      ],
      createdAt = json['createdAt'] as int? ?? 0,
      version = json['version'] as int;

  final String id;

  /// gate, round_limit, loop_stalled, invalid_review, timeout, restart or permission.
  final String kind;
  final String? runId;
  final String? taskKey;
  final String? stepId;
  final String? sessionName;
  final String title;

  /// Why the step paused, or the command a permission request asks to run.
  final String detail;
  final List<({String label, String? text, String? url})> evidence;

  /// The backend's buttons, in its order; style is primary, neutral or danger.
  final List<({String id, String label, String style})> options;
  final int createdAt;
  final int version;
}

class Inbox {
  const Inbox(this.items, [this.answeredElsewhere = const {}]);

  /// Newest first, including cards kept to say they were answered elsewhere.
  final List<InboxItem> items;

  /// Ids of cards another answer got to first; they show a note, not buttons.
  final Set<String> answeredElsewhere;

  List<InboxItem> get pending =>
      items.where((i) => !answeredElsewhere.contains(i.id)).toList();
}

class InboxRepository {
  InboxRepository(this._dio);

  final Dio _dio;

  Future<List<InboxItem>> list() async {
    try {
      final response = await _dio.get<Map<String, dynamic>>('inbox');
      return [
        for (final json in (response.data?['items'] as List?) ?? const [])
          InboxItem.fromJson(json as Map<String, dynamic>),
      ];
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  /// Holds this machine's desktop and phone notifications until [until];
  /// board cards still show. Answers with when the mute ends.
  Future<void> mute(DateTime until) async {
    try {
      await _dio.post<void>(
        'mute',
        data: {'untilMs': until.millisecondsSinceEpoch},
      );
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  /// True when the answer was taken (204), false when another answer got
  /// there first (409 stale). Anything else throws [ApiException].
  Future<bool> answer(InboxItem item, String optionId) async {
    try {
      await _dio.post<void>(
        'inbox/${Uri.encodeComponent(item.id)}/answer',
        data: {'optionId': optionId, 'version': item.version},
      );
      return true;
    } on DioException catch (e) {
      final body = e.response?.data;
      if (e.response?.statusCode == 409 &&
          body is Map &&
          body['error'] == 'stale') {
        return false;
      }
      throw ApiException.fromDioException(e);
    }
  }
}

/// One machine's inbox (Rental Command's list-notifier pattern). It also
/// carries the machine's live `changed` notices to the runs screens, so the
/// shell's badge keeps one socket open for all of them while the app shows.
class InboxNotifier extends AsyncNotifier<Inbox> {
  InboxNotifier(this._machineId);

  final String _machineId;

  /// Bumped when a refresh starts and when an answer is taken. A fetch that
  /// finishes after a later bump is older than what the screen shows, so its
  /// result is dropped.
  int _generation = 0;

  InboxRepository get _repo =>
      InboxRepository(ref.read(machineClientProvider(_machineId)));

  @override
  Future<Inbox> build() async {
    final frames = ref.watch(liveSocketProvider(_machineId)).frames;
    final subscription = frames.listen(_onFrame);
    ref.onDispose(subscription.cancel);
    return Inbox(await _repo.list());
  }

  void _onFrame(Map<String, dynamic> frame) {
    final runs = ref.exists(runsProvider)
        ? ref.read(runsProvider.notifier)
        : null;
    switch ((frame['type'], frame['entity'])) {
      // A reconnect: refetch what the socket missed.
      case ('open', _):
        refresh();
        runs?.refresh();
        // Every open run detail, too; only the watched ones refetch.
        ref.invalidate(runProvider);
      case ('changed', 'inbox'):
        refresh();
      case ('changed', 'runs'):
        runs?.refresh();
      case ('changed', 'run'):
        runs?.runChanged('${frame['id']}');
    }
  }

  /// Refetches the inbox. Called by pull to refresh and live notices.
  Future<void> refresh() async {
    final generation = ++_generation;
    final next = await AsyncValue.guard(() async => Inbox(await _repo.list()));
    if (ref.mounted && generation == _generation) state = next;
  }

  /// Sends the owner's choice of one of [item]'s own options. A taken answer
  /// drops the card; a stale one marks it "Already answered elsewhere" and
  /// refetches. Other refusals throw [ApiException] and keep the card.
  Future<void> answer(InboxItem item, String optionId) async {
    if (await _repo.answer(item, optionId)) {
      _generation++;
      final current = state.value;
      if (ref.mounted && current != null) {
        state = AsyncData(
          Inbox(
            current.items.where((i) => i.id != item.id).toList(),
            current.answeredElsewhere,
          ),
        );
      }
      return;
    }
    final fresh = await _repo.list();
    if (!ref.mounted) return;
    // Still listed means it is waiting again (a new round): show it fresh.
    final gone = !fresh.any((i) => i.id == item.id);
    state = AsyncData(
      Inbox(
        [...fresh, if (gone) item]
          ..sort((a, b) => b.createdAt.compareTo(a.createdAt)),
        {if (gone) item.id},
      ),
    );
  }
}

final inboxProvider = AsyncNotifierProvider.autoDispose
    .family<InboxNotifier, Inbox, String>(InboxNotifier.new);
