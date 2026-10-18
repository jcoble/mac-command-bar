import 'package:assembly_phone/core/machines/machine_store.dart';
import 'package:assembly_phone/core/router.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('no_machine_redirects_to_pair', (tester) async {
    FlutterSecureStorage.setMockInitialValues({});
    final container = ProviderContainer(
      overrides: [
        machineStoreProvider.overrideWithValue(
          MachineStore(const FlutterSecureStorage()),
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.runAsync(() => container.read(machinesProvider.future));
    final router = container.read(routerProvider);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: router),
      ),
    );

    expect(router.state.matchedLocation, '/pair');
  });
}
