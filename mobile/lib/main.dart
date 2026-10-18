import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'core/machines/machine_store.dart';
import 'core/push/push_service.dart';
import 'core/router.dart';
import 'core/theme/app_theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Replace Flutter's default red/grey "error box" with a friendly card so a
  // screen that fails to build degrades gracefully instead of flashing a raw
  // error. Self-contained (own Directionality + colors) because the failing
  // widget may sit above the MaterialApp, where no Theme is in scope.
  ErrorWidget.builder = (FlutterErrorDetails details) =>
      _FriendlyErrorWidget(details: details);

  // Load the paired machines before the first frame, so the router knows at
  // once whether to open pairing; then retry any unfinished removals.
  final container = ProviderContainer();
  await container.read(machinesProvider.future);
  container.read(machinesProvider.notifier).retryRemovals().ignore();

  // Push never throws; without Firebase it stays off.
  container.read(pushServiceProvider).init().ignore();

  runApp(
    UncontrolledProviderScope(container: container, child: const AssemblyApp()),
  );
}

/// On-theme fallback shown by [ErrorWidget.builder] when a widget fails to
/// build. Renders an icon + "Couldn't load this screen"; in debug builds it
/// also surfaces the exception text to aid diagnosis. In release/profile the
/// detail is hidden so users never see a raw stack trace.
class _FriendlyErrorWidget extends StatelessWidget {
  const _FriendlyErrorWidget({required this.details});

  final FlutterErrorDetails details;

  @override
  Widget build(BuildContext context) {
    // The dark theme's colors (app_tokens.dart). Hard-coded rather than read
    // from Theme.of(context): this widget can be inserted into the tree
    // before/above MaterialApp, so an inherited theme is not guaranteed.
    const surface = Color(0xFF121217);
    const onSurface = Color(0xFFE6E0E9);
    const onSurfaceVariant = Color(0xFFCAC4D0);
    const errorColor = Color(0xFFF2B8B5);

    return Directionality(
      textDirection: TextDirection.ltr,
      child: Material(
        color: surface,
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(
                  Icons.error_outline_rounded,
                  size: 44,
                  color: errorColor,
                ),
                const SizedBox(height: 16),
                const Text(
                  "Couldn't load this screen",
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    color: onSurface,
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 8),
                const Text(
                  'Something went wrong. Try going back and opening it again.',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: onSurfaceVariant, fontSize: 14),
                ),
                if (kDebugMode) ...[
                  const SizedBox(height: 16),
                  Text(
                    details.exceptionAsString(),
                    textAlign: TextAlign.center,
                    maxLines: 6,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: onSurfaceVariant,
                      fontSize: 12,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class AssemblyApp extends ConsumerWidget {
  const AssemblyApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp.router(
      title: 'Assembly',
      theme: AppTheme.light,
      darkTheme: AppTheme.dark,
      // Dark only, as Rental Command.
      themeMode: ThemeMode.dark,
      routerConfig: ref.watch(routerProvider),
    );
  }
}
