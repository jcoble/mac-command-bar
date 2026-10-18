import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:assembly_phone/core/api/machine_client.dart';
import 'package:assembly_phone/core/machines/machine_store.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';

/// Answers every request with [handler] and records what was sent. A handler
/// that returns null makes the machine unreachable (connection error).
class FakeAdapter implements HttpClientAdapter {
  FakeAdapter(this.handler);

  final (int, Object?)? Function(RequestOptions request) handler;
  final requests = <RequestOptions>[];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(options);
    final answer = handler(options);
    if (answer == null) {
      throw DioException.connectionError(
        requestOptions: options,
        reason: 'unreachable',
      );
    }
    final (status, body) = answer;
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

/// A store whose writes land 10 ms after they start, once [holdWrites] is set,
/// so two unserialized read-modify-writes overlap.
class SlowWriteStore extends MachineStore {
  SlowWriteStore(super.storage);

  bool holdWrites = false;
  final writeStarted = Completer<void>();

  @override
  Future<void> write(List<Machine> machines) async {
    if (holdWrites) {
      if (!writeStarted.isCompleted) writeStarted.complete();
      await Future<void>.delayed(const Duration(milliseconds: 10));
    }
    await super.write(machines);
  }
}

/// Holds every DELETE until [releaseDelete] is completed.
class GatedDeleteAdapter extends FakeAdapter {
  GatedDeleteAdapter(super.handler);

  final releaseDelete = Completer<void>();

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    if (options.method == 'DELETE') await releaseDelete.future;
    return super.fetch(options, requestStream, cancelFuture);
  }
}

const workbox = Machine(
  id: 'm-workbox',
  name: 'Workbox',
  url: 'https://agent-workbox.example.ts.net:8777',
  deviceToken: 'old-token',
);

const laptop = Machine(
  id: 'm-laptop',
  name: 'Laptop',
  url: 'https://laptop.example.ts.net:8777',
  deviceToken: 'laptop-token',
);

Future<ProviderContainer> containerWith(
  FakeAdapter adapter, {
  List<Machine> machines = const [],
}) async {
  FlutterSecureStorage.setMockInitialValues({});
  final store = MachineStore(const FlutterSecureStorage());
  await store.write(machines);
  final container = ProviderContainer(
    overrides: [
      machineStoreProvider.overrideWithValue(store),
      dioProvider.overrideWithValue(Dio()..httpClientAdapter = adapter),
    ],
  );
  addTearDown(container.dispose);
  await container.read(machinesProvider.future);
  return container;
}

void main() {
  test('pairing_payload_parses_v1_and_rejects_others', () {
    final payload = PairingPayload.parse(
      '{"v":1,"url":"https://agent-workbox.example.ts.net:8777/","machine":"Workbox","code":"K7QM4XTD9P"}',
    );
    expect(payload.url, 'https://agent-workbox.example.ts.net:8777');
    expect(payload.machine, 'Workbox');
    expect(payload.code, 'K7QM4XTD9P');
    // Codes are uppercase base32 and the machine compares them exactly; a typed code may be lowercase.
    expect(
      PairingPayload.parse('{"v":1,"url":"https://a.example","code":" k7qm4xtd9p "}').code,
      'K7QM4XTD9P',
    );

    for (final bad in [
      '{"v":2,"url":"https://a.example","machine":"A","code":"X"}',
      '{"url":"https://a.example","machine":"A","code":"X"}',
      '{"v":1,"url":"http://a.example","machine":"A","code":"X"}',
      '{"v":1,"url":"https://a.example","machine":"A"}',
      'not json',
    ]) {
      expect(
        () => PairingPayload.parse(bad),
        throwsFormatException,
        reason: bad,
      );
    }
  });

  test('machine_store_round_trips_and_selects', () async {
    FlutterSecureStorage.setMockInitialValues({});
    final store = MachineStore(const FlutterSecureStorage());
    await store.write([workbox, laptop]);
    await store.select(laptop.id);

    final saved = await MachineStore(const FlutterSecureStorage()).read();
    expect(saved.all.map((m) => m.toJson()), [
      workbox.toJson(),
      laptop.toJson(),
    ]);
    expect(saved.selected?.id, laptop.id);

    await store.remove(laptop);
    final afterRemove = await store.read();
    expect(afterRemove.all.map((m) => m.id), [workbox.id]);
    expect(afterRemove.selected?.id, workbox.id);
  });

  test('machine_client_sends_device_token', () async {
    final adapter = FakeAdapter((_) => (200, {'deviceId': 'd1'}));
    final container = await containerWith(adapter, machines: [workbox]);

    await container.read(machineClientProvider(workbox.id)).get<void>('me');

    final request = adapter.requests.single;
    expect(request.uri.toString(), '${workbox.url}/phone/v1/me');
    expect(request.headers['Authorization'], 'Bearer old-token');
  });

  test('removing_machine_treats_401_as_already_revoked', () async {
    var reachable = false;
    final adapter = FakeAdapter((_) => reachable ? (401, null) : null);
    final container = await containerWith(adapter, machines: [workbox]);
    final notifier = container.read(machinesProvider.notifier);

    // Unreachable: hidden at once, token kept as "removing" for a retry.
    await notifier.remove(workbox);
    var saved = container.read(machinesProvider).requireValue;
    expect(saved.visible, isEmpty);
    expect(saved.all.single.removing, isTrue);

    // Next app start: 401 means an earlier call already revoked it.
    reachable = true;
    await notifier.retryRemovals();
    saved = container.read(machinesProvider).requireValue;
    expect(saved.all, isEmpty);
    expect(adapter.requests.map((r) => r.method), ['DELETE', 'DELETE']);
    expect(adapter.requests.last.headers['Authorization'], 'Bearer old-token');
  });

  test('pairing_same_machine_again_revokes_old_token', () async {
    final adapter = FakeAdapter(
      (r) => r.method == 'POST'
          ? (
              200,
              {
                'deviceId': 'd2',
                'deviceToken': 'new-token',
                'machine': {'id': workbox.id, 'name': 'Workbox'},
              },
            )
          : (204, null),
    );
    final container = await containerWith(adapter, machines: [workbox]);

    await container
        .read(machinesProvider.notifier)
        .pair(
          PairingPayload.parse(
            '{"v":1,"url":"${workbox.url}","machine":"Workbox","code":"K7QM4XTD9P"}',
          ),
        );

    final pair = adapter.requests.first;
    expect(pair.uri.toString(), '${workbox.url}/phone/v1/pair');
    expect((pair.data as Map)['code'], 'K7QM4XTD9P');
    final revoke = adapter.requests.last;
    expect(revoke.method, 'DELETE');
    expect(revoke.headers['Authorization'], 'Bearer old-token');

    final saved = container.read(machinesProvider).requireValue;
    expect(saved.all.single.deviceToken, 'new-token');
    expect(saved.selected?.id, workbox.id);
  });

  test('concurrent_pair_and_removal_retry_keep_both_changes', () async {
    FlutterSecureStorage.setMockInitialValues({});
    final store = SlowWriteStore(const FlutterSecureStorage());
    await store.write([workbox.markedRemoving()]);
    final adapter = GatedDeleteAdapter(
      (r) => r.method == 'POST'
          ? (
              200,
              {
                'deviceId': 'd3',
                'deviceToken': 'laptop-new-token',
                'machine': {'id': laptop.id, 'name': 'Laptop'},
              },
            )
          : (204, null),
    );
    final container = ProviderContainer(
      overrides: [
        machineStoreProvider.overrideWithValue(store),
        dioProvider.overrideWithValue(Dio()..httpClientAdapter = adapter),
      ],
    );
    addTearDown(container.dispose);
    await container.read(machinesProvider.future);
    final notifier = container.read(machinesProvider.notifier);
    store.holdWrites = true;

    // The retry's DELETE is waiting; pairing starts and begins saving; then the
    // DELETE completes while the pairing save is still in flight.
    final retry = notifier.retryRemovals();
    final pair = notifier.pair(
      PairingPayload.parse(
        '{"v":1,"url":"${laptop.url}","machine":"Laptop","code":"K7QM4XTD9P"}',
      ),
    );
    await store.writeStarted.future;
    adapter.releaseDelete.complete();
    await Future.wait([retry, pair]);

    final saved = await store.read();
    expect(saved.all.map((m) => m.deviceToken), ['laptop-new-token']);
    expect(saved.selected?.id, laptop.id);
  });

  test('home_machine_is_the_newest_orchestrator_mark', () {
    expect(
      homeMachineId([
        (machineId: 'a', hostsOrchestrator: true, markedAt: 100),
        (machineId: 'b', hostsOrchestrator: true, markedAt: 300),
        (machineId: 'c', hostsOrchestrator: false, markedAt: null),
      ]),
      'b',
    );
    expect(
      homeMachineId([
        (machineId: 'c', hostsOrchestrator: false, markedAt: null),
      ]),
      isNull,
    );
  });
}
