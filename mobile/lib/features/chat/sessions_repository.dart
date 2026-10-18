import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';

/// One session on a machine, as `GET /phone/v1/sessions` lists it.
class PhoneSession {
  PhoneSession.fromJson(Map<String, dynamic> json)
    : ownedId = json['ownedId'] as String,
      handle = json['handle'] as String? ?? '',
      name = json['name'] as String? ?? '',
      provider = json['provider'] as String? ?? '',
      project = json['project'] as String?,
      state = json['state'] as String? ?? 'idle',
      runId = json['runId'] as String?,
      stepId = json['stepId'] as String?,
      orchestrator = json['orchestrator'] == true,
      lastActivityAt = json['lastActivityAt'] as int? ?? 0;

  final String ownedId;
  final String handle;
  final String name;
  final String provider;
  final String? project;

  /// running, waiting, idle or stopped.
  final String state;

  /// Set for a workflow worker session.
  final String? runId;
  final String? stepId;
  final bool orchestrator;
  final int lastActivityAt;
}

/// The machine's sessions: the orchestrator first, then the rest in the
/// backend's order, newest activity first.
final sessionsProvider = FutureProvider.autoDispose
    .family<List<PhoneSession>, String>((ref, machineId) async {
      try {
        final response = await ref
            .watch(machineClientProvider(machineId))
            .get<Map<String, dynamic>>('sessions');
        final all = [
          for (final json in (response.data?['sessions'] as List?) ?? const [])
            PhoneSession.fromJson(json as Map<String, dynamic>),
        ];
        return [
          ...all.where((s) => s.orchestrator),
          ...all.where((s) => !s.orchestrator),
        ];
      } on DioException catch (e) {
        throw ApiException.fromDioException(e);
      }
    });

/// The sessions whose name or handle contains [query], ignoring case.
List<PhoneSession> filterSessions(List<PhoneSession> sessions, String query) {
  final q = query.trim().toLowerCase();
  if (q.isEmpty) return sessions;
  return [
    for (final s in sessions)
      if (s.name.toLowerCase().contains(q) ||
          s.handle.toLowerCase().contains(q))
        s,
  ];
}
