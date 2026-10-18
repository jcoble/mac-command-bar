import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/machines/machine_store.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/mobile_m3_list.dart';

/// Paired machines: select one, pair another, or remove one from this phone.
class MachinesScreen extends ConsumerWidget {
  const MachinesScreen({super.key});

  Future<void> _confirmRemove(
    BuildContext context,
    WidgetRef ref,
    Machine machine,
  ) async {
    final remove = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Remove ${machine.name}?'),
        content: const Text(
          'This phone stops reaching it. Pair again to bring it back.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
    if (remove == true) {
      await ref.read(machinesProvider.notifier).remove(machine);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final saved = ref.watch(machinesProvider).value;
    final machines = saved?.visible ?? const <Machine>[];
    final homeId = ref.watch(homeMachineIdProvider).value;
    return Scaffold(
      appBar: AppBar(title: const Text('Machines')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          for (var i = 0; i < machines.length; i++) ...[
            if (i > 0) const MobileM3ListDivider(),
            MobileM3ListItem(
              position: MobileM3ListItemPositionForIndex.forIndex(
                i,
                machines.length,
              ),
              leading: MobileM3LeadingIcon(
                icon: Symbols.dns_rounded,
                backgroundColor: scheme.secondaryContainer,
                foregroundColor: scheme.onSecondaryContainer,
              ),
              title: Text(machines[i].name, style: theme.textTheme.titleMedium),
              supporting: [
                if (machines[i].id == homeId)
                  Text(
                    'HOME · ORCHESTRATOR',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: context.tokens.info,
                    ),
                  ),
                Text(
                  Uri.parse(machines[i].url).authority,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
              trailing: machines[i].id == saved?.selected?.id
                  ? Text(
                      'Selected',
                      style: theme.textTheme.labelMedium?.copyWith(
                        color: context.tokens.success,
                      ),
                    )
                  : null,
              actions: [
                IconButton(
                  tooltip: 'Remove machine',
                  icon: const Icon(Symbols.delete_rounded),
                  onPressed: () => _confirmRemove(context, ref, machines[i]),
                ),
              ],
              onTap: () async {
                await ref
                    .read(machinesProvider.notifier)
                    .select(machines[i].id);
                if (context.mounted) context.pop();
              },
            ),
          ],
          const SizedBox(height: 16),
          FilledButton.tonalIcon(
            onPressed: () => context.push('/pair'),
            icon: const Icon(Symbols.qr_code_scanner_rounded),
            label: const Text('Pair another machine'),
          ),
        ],
      ),
    );
  }
}
