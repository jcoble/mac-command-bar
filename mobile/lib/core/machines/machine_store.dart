import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../api/machine_client.dart';

const _keyMachines = 'assembly_machines';
const _keySelected = 'assembly_selected_machine';
const _deviceName = 'Android phone';

/// What the desktop's "Pair phone" QR code holds:
/// `{"v":1,"url":…,"machine":…,"code":…}`.
class PairingPayload {
  const PairingPayload({required this.url, required this.code, this.machine});

  final String url;
  final String code;
  final String? machine;

  factory PairingPayload.parse(String text) {
    final Object? json;
    try {
      json = jsonDecode(text);
    } on FormatException {
      throw const FormatException('This is not an Assembly pairing code.');
    }
    if (json is! Map || json['v'] != 1) {
      throw const FormatException('This is not an Assembly pairing code.');
    }
    final url = json['url'];
    final uri = url is String ? Uri.tryParse(url) : null;
    if (uri == null || uri.scheme != 'https' || uri.host.isEmpty) {
      throw const FormatException('The machine URL must start with https://.');
    }
    final code = json['code'];
    if (code is! String || code.trim().isEmpty) {
      throw const FormatException('The pairing code is missing.');
    }
    final machine = json['machine'];
    return PairingPayload(
      url: (url as String).replaceFirst(RegExp(r'/+$'), ''),
      code: code.trim().toUpperCase(),
      machine: machine is String ? machine : null,
    );
  }
}

/// A paired machine. [removing] means the owner removed it but the machine has
/// not yet confirmed the token is revoked (`DELETE /me`); it stays hidden and is
/// retried on each app start.
class Machine {
  const Machine({
    required this.id,
    required this.name,
    required this.url,
    required this.deviceToken,
    this.removing = false,
  });

  final String id;
  final String name;
  final String url;
  final String deviceToken;
  final bool removing;

  Machine markedRemoving() => Machine(
    id: id,
    name: name,
    url: url,
    deviceToken: deviceToken,
    removing: true,
  );

  Map<String, Object> toJson() => {
    'id': id,
    'name': name,
    'url': url,
    'deviceToken': deviceToken,
    'removing': removing,
  };

  factory Machine.fromJson(Map<String, dynamic> json) => Machine(
    id: json['id'] as String,
    name: json['name'] as String,
    url: json['url'] as String,
    deviceToken: json['deviceToken'] as String,
    removing: json['removing'] == true,
  );
}

class SavedMachines {
  const SavedMachines(this.all, this.selectedId);

  /// Every record, including tokens still being revoked.
  final List<Machine> all;
  final String? selectedId;

  List<Machine> get visible => all.where((m) => !m.removing).toList();

  Machine? get selected {
    final shown = visible;
    return shown.where((m) => m.id == selectedId).firstOrNull ??
        shown.firstOrNull;
  }
}

/// Paired machines in secure storage, under one key, plus the selected id.
class MachineStore {
  MachineStore(this._storage);

  final FlutterSecureStorage _storage;

  Future<SavedMachines> read() async {
    final raw = await _storage.read(key: _keyMachines);
    final list = raw == null ? const [] : jsonDecode(raw) as List;
    return SavedMachines([
      for (final m in list) Machine.fromJson(m as Map<String, dynamic>),
    ], await _storage.read(key: _keySelected));
  }

  Future<void> write(List<Machine> machines) => _storage.write(
    key: _keyMachines,
    value: jsonEncode([for (final m in machines) m.toJson()]),
  );

  Future<void> select(String machineId) =>
      _storage.write(key: _keySelected, value: machineId);

  /// Deletes the record holding [machine]'s token (a re-paired machine can
  /// have an old "removing" record with the same id).
  Future<void> remove(Machine machine) async {
    final saved = await read();
    await write([
      for (final m in saved.all)
        if (m.deviceToken != machine.deviceToken) m,
    ]);
  }
}

final machineStoreProvider = Provider<MachineStore>(
  (ref) => MachineStore(const FlutterSecureStorage()),
);

final machinesProvider = AsyncNotifierProvider<MachinesNotifier, SavedMachines>(
  MachinesNotifier.new,
);

class MachinesNotifier extends AsyncNotifier<SavedMachines> {
  MachineStore get _store => ref.read(machineStoreProvider);

  /// Ends when the last queued storage change has finished.
  Future<void> _tail = Future.value();

  /// Runs [action] after every earlier call has finished, so each
  /// read-modify-write of the saved machines runs alone. Never call it from
  /// inside [action]. Network calls stay outside it.
  Future<T> _serial<T>(Future<T> Function() action) async {
    final previous = _tail;
    final done = Completer<void>();
    _tail = done.future;
    await previous;
    try {
      return await action();
    } finally {
      done.complete();
    }
  }

  @override
  Future<SavedMachines> build() => _store.read();

  Future<void> select(String machineId) async {
    await _store.select(machineId);
    state = AsyncData(await _store.read());
  }

  /// `POST /phone/v1/pair`, then saves and selects the machine. Pairing a
  /// machine the phone already holds replaces its record and revokes the old
  /// token.
  Future<void> pair(PairingPayload payload) async {
    final response = await ref
        .read(dioProvider)
        .post<Map<String, dynamic>>(
          '${payload.url}/phone/v1/pair',
          data: {'code': payload.code, 'deviceName': _deviceName},
        );
    final body = response.data!;
    final info = body['machine'] as Map<String, dynamic>;
    final machine = Machine(
      id: info['id'] as String,
      name: info['name'] as String,
      url: payload.url,
      deviceToken: body['deviceToken'] as String,
    );
    final old = await _serial(() async {
      final saved = await _store.read();
      final old = [
        for (final m in saved.visible)
          if (m.id == machine.id) m.markedRemoving(),
      ];
      await _store.write([
        for (final m in saved.all)
          if (m.removing || m.id != machine.id) m,
        ...old,
        machine,
      ]);
      return old;
    });
    await select(machine.id);
    for (final m in old) {
      await _revoke(m);
    }
  }

  /// Hides the machine at once, then revokes its token.
  Future<void> remove(Machine machine) async {
    await _serial(() async {
      final saved = await _store.read();
      await _store.write([
        for (final m in saved.all)
          m.deviceToken == machine.deviceToken ? m.markedRemoving() : m,
      ]);
    });
    state = AsyncData(await _store.read());
    await _revoke(machine);
  }

  /// Called on each app start for tokens an earlier removal could not revoke.
  Future<void> retryRemovals() async {
    for (final m in (await _store.read()).all.where((m) => m.removing)) {
      await _revoke(m);
    }
  }

  /// `DELETE /me`. 204 or 401 (an earlier call already revoked it and only its
  /// answer was lost) deletes the record; anything else keeps it for a retry.
  Future<void> _revoke(Machine machine) async {
    try {
      await machineDio(ref.read(dioProvider), machine).delete<void>('me');
    } on DioException catch (e) {
      if (e.response?.statusCode != 401) return;
    }
    await _serial(() => _store.remove(machine));
    state = AsyncData(await _store.read());
  }
}

typedef OrchestratorMark = ({
  String machineId,
  bool hostsOrchestrator,
  int? markedAt,
});

/// The home machine is the one whose orchestrator mark is newest: a machine
/// that was unreachable when the owner moved the orchestrator keeps its old
/// mark until the desktop reconnects to it.
String? homeMachineId(Iterable<OrchestratorMark> marks) {
  OrchestratorMark? home;
  for (final m in marks) {
    if (!m.hostsOrchestrator) continue;
    if (home == null || (m.markedAt ?? 0) > (home.markedAt ?? 0)) home = m;
  }
  return home?.machineId;
}

/// Asks every visible machine `GET /me`; unreachable machines count as not
/// hosting the orchestrator.
final homeMachineIdProvider = FutureProvider<String?>((ref) async {
  final machines = (await ref.watch(machinesProvider.future)).visible;
  final dio = ref.watch(dioProvider);
  final marks = await Future.wait(
    machines.map((m) async {
      try {
        final me = (await machineDio(
          dio,
          m,
        ).get<Map<String, dynamic>>('me')).data!;
        return (
          machineId: m.id,
          hostsOrchestrator: me['hostsOrchestrator'] == true,
          markedAt: me['orchestratorMarkedAt'] as int?,
        );
      } on DioException {
        return (machineId: m.id, hostsOrchestrator: false, markedAt: null);
      }
    }),
  );
  return homeMachineId(marks);
});
