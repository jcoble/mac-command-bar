import 'package:assembly_phone/core/api/machine_client.dart';
import 'package:assembly_phone/features/chat/sessions_repository.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'chat_test.dart' show FakeAdapter, machineId;

Map<String, Object?> session(
  String ownedId,
  String name, {
  String? handle,
  bool orchestrator = false,
  int lastActivityAt = 0,
}) => {
  'ownedId': ownedId,
  'handle': handle ?? 'handle-$ownedId',
  'name': name,
  'provider': 'codex',
  'project': 'mac-command-bar',
  'state': 'idle',
  'runId': null,
  'stepId': null,
  'orchestrator': orchestrator,
  'lastActivityAt': lastActivityAt,
};

Future<List<PhoneSession>> listed(List<Map<String, Object?>> sessions) async {
  final adapter = FakeAdapter((_) => (200, {'sessions': sessions}));
  final container = ProviderContainer(
    overrides: [
      machineClientProvider(machineId).overrideWithValue(
        Dio(BaseOptions(baseUrl: 'https://m.example/phone/v1/'))
          ..httpClientAdapter = adapter,
      ),
    ],
  );
  addTearDown(container.dispose);
  final list = await container.read(sessionsProvider(machineId).future);
  expect(adapter.requests.single.path, 'sessions');
  return list;
}

void main() {
  test('sessions_list_pins_orchestrator_then_newest', () async {
    // The backend sends newest activity first.
    final list = await listed([
      session('new', 'New lane', lastActivityAt: 300),
      session('orch', 'Orchestrator', orchestrator: true, lastActivityAt: 200),
      session('old', 'Old lane', lastActivityAt: 100),
    ]);

    expect([for (final s in list) s.ownedId], ['orch', 'new', 'old']);
  });

  test('sessions_search_filters_by_name_or_handle', () async {
    final list = await listed([
      session('a', 'TSK-1416/build', handle: 'brisk-heron'),
      session('b', 'Phone review', handle: 'calm-otter'),
      session('c', 'Docs pass', handle: 'quiet-heron'),
    ]);

    List<String> found(String query) => [
      for (final s in filterSessions(list, query)) s.ownedId,
    ];
    expect(found(''), ['a', 'b', 'c']);
    expect(found('phone'), ['b']);
    expect(found('HERON'), ['a', 'c']);
    expect(found('  tsk-1416 '), ['a']);
    expect(found('nothing'), isEmpty);
  });
}
