import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';

import 'core/machines/machine_store.dart';
import 'core/theme/app_recipes.dart';
import 'core/theme/app_tokens.dart';
import 'features/inbox/inbox_repository.dart';

/// The three tabs, each with the selected machine as a chip in the app bar.
class AppShell extends ConsumerWidget {
  const AppShell({super.key, required this.navigationShell});

  final StatefulNavigationShell navigationShell;

  static const _tabs = [
    _TabItem(label: 'Sessions', title: 'Sessions', icon: Symbols.chat_rounded),
    _TabItem(label: 'Runs', title: 'Runs', icon: Symbols.list_rounded),
    _TabItem(label: 'Inbox', title: 'Inbox', icon: Symbols.inbox_rounded),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final index = navigationShell.currentIndex;
    final machine = ref.watch(machinesProvider).value?.selected;
    // Watching the inbox keeps its live socket open while the app shows.
    final pending = machine == null
        ? 0
        : ref.watch(inboxProvider(machine.id)).value?.pending.length ?? 0;
    return Scaffold(
      appBar: AppBar(
        title: Text(_tabs[index].title),
        actions: [
          if (machine != null)
            Padding(
              padding: const EdgeInsets.only(right: 12),
              child: ActionChip(
                avatar: Icon(
                  Icons.circle,
                  size: 10,
                  color: context.tokens.success,
                ),
                label: Text(machine.name),
                onPressed: () => context.push('/machines'),
              ),
            ),
        ],
      ),
      body: navigationShell,
      bottomNavigationBar: _MorphNavBar(
        tabs: _tabs,
        selectedIndex: index,
        inboxCount: pending,
        onSelected: (i) =>
            navigationShell.goBranch(i, initialLocation: i == index),
      ),
    );
  }
}

class _TabItem {
  const _TabItem({
    required this.label,
    required this.title,
    required this.icon,
  });

  final String label;
  final String title;

  /// A `Symbols.*_rounded` glyph whose FILL axis is animated when active.
  final IconData icon;
}

/// Rental Command's `_MorphNavBar` (`home_shell.dart`), three tabs, no FAB gap.
class _MorphNavBar extends StatelessWidget {
  const _MorphNavBar({
    required this.tabs,
    required this.selectedIndex,
    required this.inboxCount,
    required this.onSelected,
  });

  final List<_TabItem> tabs;
  final int selectedIndex;

  /// Pending inbox items, shown as a badge on the last tab when above 0.
  final int inboxCount;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      decoration: BoxDecoration(
        color: scheme.surfaceContainerLow,
        border: Border(
          top: BorderSide(color: scheme.outlineVariant.withValues(alpha: 0.4)),
        ),
      ),
      child: SafeArea(
        top: false,
        child: SizedBox(
          height: 64,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              for (var i = 0; i < tabs.length; i++)
                Badge(
                  isLabelVisible: i == tabs.length - 1 && inboxCount > 0,
                  label: Text('$inboxCount'),
                  child: M3MorphNavItem(
                    icon: tabs[i].icon,
                    label: tabs[i].label,
                    selected: i == selectedIndex,
                    onTap: () => onSelected(i),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
