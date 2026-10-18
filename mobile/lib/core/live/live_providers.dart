import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../machines/machine_store.dart';
import 'live_socket.dart';

/// The live socket for one machine, shared by every screen that watches it and
/// closed when the last one stops (Rental Command's `realtimeWatcherProvider`
/// lifetime, scoped to the screens instead of the whole app).
final liveSocketProvider = Provider.autoDispose.family<LiveSocket, String>((
  ref,
  machineId,
) {
  final machine = ref
      .watch(machinesProvider)
      .requireValue
      .visible
      .firstWhere((m) => m.id == machineId);
  final socket = LiveSocket(machine);
  ref.onDispose(socket.close);
  return socket;
});
