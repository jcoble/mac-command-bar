import 'package:assembly_phone/core/machines/machine_store.dart';
import 'package:assembly_phone/core/push/push_service.dart';
import 'package:flutter_test/flutter_test.dart';

import 'machines_test.dart' show FakeAdapter, containerWith, laptop, workbox;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('push_init_fails_soft_without_firebase', () async {
    final container = await containerWith(FakeAdapter((_) => (204, null)));
    final push = container.read(pushServiceProvider);

    await push.init();

    expect(push.isAvailable, isFalse);
  });

  test('notification_tap_opens_item_on_its_machine', () async {
    final container = await containerWith(
      FakeAdapter((_) => (204, null)),
      machines: [workbox, laptop],
    );
    final push = container.read(pushServiceProvider);
    expect(
      container.read(machinesProvider).requireValue.selected?.id,
      workbox.id,
    );

    final route = await push.routeForTap({
      'machineId': laptop.id,
      'itemId': 'perm-7',
    });

    expect(route, '/inbox/perm-7');
    expect(
      container.read(machinesProvider).requireValue.selected?.id,
      laptop.id,
    );
    // Data for a machine this phone is not paired with, or without an item, opens nothing.
    expect(
      await push.routeForTap({'machineId': 'm-other', 'itemId': 'x'}),
      isNull,
    );
    expect(await push.routeForTap({'machineId': workbox.id}), isNull);
    // A finished reply opens that session's chat.
    expect(
      await push.routeForTap({
        'machineId': laptop.id,
        'sessionId': 's-1',
        'kind': 'turn',
      }),
      '/chat/s-1',
    );
    expect(
      container.read(machinesProvider).requireValue.selected?.id,
      laptop.id,
    );
  });

  test('push_token_registers_with_every_paired_machine', () async {
    final adapter = FakeAdapter(
      (request) => request.uri.host.startsWith('laptop') ? null : (204, null),
    );
    final container = await containerWith(adapter, machines: [workbox, laptop]);
    final push = container.read(pushServiceProvider);

    await push.registerToken('fcm-1');

    final posts = adapter.requests
        .where((r) => r.path.endsWith('push-token'))
        .toList();
    expect(
      posts.map((r) => r.uri.host),
      unorderedEquals([
        Uri.parse(workbox.url).host,
        Uri.parse(laptop.url).host,
      ]),
    );
    for (final post in posts) {
      expect(post.method, 'POST');
      expect(post.data, {'fcmToken': 'fcm-1'});
      final machine = post.uri.host.startsWith('laptop') ? laptop : workbox;
      expect(post.headers['Authorization'], 'Bearer ${machine.deviceToken}');
    }

    // The unreachable laptop is tried again; the workbox already holds the token.
    adapter.requests.clear();
    await push.registerToken('fcm-1');
    expect(adapter.requests.map((r) => r.uri.host), [
      Uri.parse(laptop.url).host,
    ]);
  });
}
