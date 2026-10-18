import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../app_shell.dart';
import '../features/machines/machines_screen.dart';
import '../features/machines/pair_screen.dart';
import '../features/runs/run_screen.dart';
import '../features/runs/runs_screen.dart';
import 'machines/machine_store.dart';

/// Rental Command's redirect guard (`app_router.dart`), with one rule: no
/// paired machine → `/pair`.
final routerProvider = Provider<GoRouter>((ref) {
  bool hasMachine(AsyncValue<SavedMachines> v) =>
      v.value?.visible.isNotEmpty ?? false;
  final paired = ValueNotifier<bool>(hasMachine(ref.read(machinesProvider)));
  ref.listen(machinesProvider, (_, next) => paired.value = hasMachine(next));
  ref.onDispose(paired.dispose);

  return GoRouter(
    initialLocation: '/chat',
    refreshListenable: paired,
    redirect: (context, state) =>
        !paired.value && state.matchedLocation != '/pair' ? '/pair' : null,
    routes: [
      StatefulShellRoute.indexedStack(
        builder: (context, state, shell) => AppShell(navigationShell: shell),
        branches: [
          for (final path in ['/chat', '/runs', '/inbox'])
            StatefulShellBranch(
              routes: [
                GoRoute(
                  path: path,
                  builder: (_, _) =>
                      path == '/runs' ? const RunsScreen() : const EmptyTab(),
                ),
              ],
            ),
        ],
      ),
      GoRoute(
        path: '/runs/:id',
        builder: (_, state) => RunScreen(runId: state.pathParameters['id']!),
      ),
      GoRoute(path: '/pair', builder: (_, _) => const PairScreen()),
      GoRoute(path: '/machines', builder: (_, _) => const MachinesScreen()),
    ],
  );
});
