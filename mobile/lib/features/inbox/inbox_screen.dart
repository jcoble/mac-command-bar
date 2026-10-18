import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/machines/machine_store.dart';
import '../../core/theme/app_recipes.dart';
import '../../core/theme/app_tokens.dart';
import '../runs/runs_screen.dart';
import 'inbox_repository.dart';

/// Everything waiting for the owner on the selected machine, newest first.
/// With [focusId] (route `/inbox/:id`) it scrolls to that card and highlights it.
class InboxScreen extends ConsumerStatefulWidget {
  const InboxScreen({super.key, this.focusId});

  final String? focusId;

  @override
  ConsumerState<InboxScreen> createState() => _InboxScreenState();
}

class _InboxScreenState extends ConsumerState<InboxScreen> {
  final _focusKey = GlobalKey();
  final _busy = <String>{};

  /// Whether [InboxScreen.focusId] was missing when the inbox first loaded.
  /// Decided once, so a card this phone answers is not reported as answered
  /// elsewhere.
  bool? _focusGone;

  Future<void> _answer(String machineId, InboxItem item, String option) async {
    setState(() => _busy.add(item.id));
    try {
      await ref.read(inboxProvider(machineId).notifier).answer(item, option);
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(errorText(error))));
      }
    } finally {
      if (mounted) setState(() => _busy.remove(item.id));
    }
  }

  @override
  Widget build(BuildContext context) {
    final machine = ref.watch(machinesProvider).value?.selected;
    if (machine == null) return const SizedBox.shrink();
    final inbox = ref.watch(inboxProvider(machine.id));
    return RefreshIndicator(
      onRefresh: () => ref.read(inboxProvider(machine.id).notifier).refresh(),
      child: switch (inbox) {
        AsyncValue(:final value?) => _list(context, machine.id, value),
        AsyncValue(:final error?) => ListView(
          children: [
            RunsMessage('Could not load the inbox. ${errorText(error)}'),
          ],
        ),
        _ => const Center(child: CircularProgressIndicator()),
      },
    );
  }

  Widget _list(BuildContext context, String machineId, Inbox inbox) {
    final focus = widget.focusId;
    if (focus != null && _focusGone == null) {
      _focusGone = !inbox.items.any((i) => i.id == focus);
      if (!_focusGone!) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          final target = _focusKey.currentContext;
          if (target != null && target.mounted) {
            Scrollable.ensureVisible(target, duration: M3Motion.medium2);
          }
        });
      }
    }
    // A column, not a lazy list, so the focused card exists to scroll to.
    return SingleChildScrollView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_focusGone ?? false)
            const Padding(
              padding: EdgeInsets.only(bottom: 12),
              child: _Note('Already answered elsewhere'),
            ),
          if (inbox.items.isEmpty) const RunsMessage('Nothing waiting for you'),
          for (final item in inbox.items)
            Padding(
              key: item.id == focus ? _focusKey : null,
              padding: const EdgeInsets.only(bottom: 12),
              child: _InboxCard(
                item: item,
                highlighted: item.id == focus,
                answeredElsewhere: inbox.answeredElsewhere.contains(item.id),
                busy: _busy.contains(item.id),
                onAnswer: (option) => _answer(machineId, item, option),
              ),
            ),
        ],
      ),
    );
  }
}

/// The push lane's words for each kind.
String inboxKindLabel(String kind) => switch (kind) {
  'gate' => 'Approval needed',
  'round_limit' => 'Round limit reached',
  'loop_stalled' => 'Loop stalled',
  'invalid_review' => 'Review report invalid',
  'timeout' => 'Step out of time',
  'restart' => 'Step needs a restart',
  'permission' => 'Permission request',
  _ => kind,
};

class _InboxCard extends StatelessWidget {
  const _InboxCard({
    required this.item,
    required this.highlighted,
    required this.answeredElsewhere,
    required this.busy,
    required this.onAnswer,
  });

  final InboxItem item;
  final bool highlighted;
  final bool answeredElsewhere;
  final bool busy;
  final ValueChanged<String> onAnswer;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final permission = item.kind == 'permission';
    final accent = permission ? scheme.error : context.tokens.warning;
    // A permission card leads with its session; its title is often the
    // command itself, shown once in the command box below.
    final heading = permission ? item.sessionName ?? item.title : item.title;
    final title = permission && item.title != item.detail ? item.title : null;
    final card = M3TonalCard(
      family: permission ? M3TonalFamily.rose : M3TonalFamily.amber,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  [
                    inboxKindLabel(item.kind),
                    ?item.stepId,
                  ].join(' · ').toUpperCase(),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelMedium?.copyWith(
                    color: accent,
                    letterSpacing: 0.6,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Text(
                whenText(item.createdAt),
                style: theme.textTheme.labelSmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            heading,
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          if (title != null) ...[
            const SizedBox(height: 4),
            Text(title, style: theme.textTheme.bodyMedium),
          ],
          if (item.detail.isNotEmpty) ...[
            const SizedBox(height: 8),
            if (permission)
              Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 10,
                ),
                decoration: BoxDecoration(
                  color: scheme.surfaceContainerLowest,
                  borderRadius: M3Shape.radiusMedium,
                ),
                child: Text(
                  item.detail,
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontFamily: 'monospace',
                    fontSize: 12,
                  ),
                ),
              )
            else
              Text(item.detail, style: theme.textTheme.bodyMedium),
          ],
          for (final e in item.evidence)
            Padding(
              padding: const EdgeInsets.only(top: 6),
              child: Text(
                '${e.label}: ${e.text ?? e.url ?? ''}',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
            ),
          const SizedBox(height: 14),
          if (answeredElsewhere)
            const _Note('Already answered elsewhere')
          else
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final option in item.options)
                  _OptionButton(
                    label: option.label,
                    style: option.style,
                    onPressed: busy ? null : () => onAnswer(option.id),
                  ),
              ],
            ),
        ],
      ),
    );
    if (!highlighted) return card;
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(30)),
        border: Border.all(color: scheme.primary, width: 2),
      ),
      child: Padding(padding: const EdgeInsets.all(2), child: card),
    );
  }
}

/// A backend option as a button: primary filled, neutral tonal, danger outlined.
class _OptionButton extends StatelessWidget {
  const _OptionButton({
    required this.label,
    required this.style,
    required this.onPressed,
  });

  final String label;
  final String style;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final text = Text(label);
    switch (style) {
      case 'primary':
        return FilledButton(onPressed: onPressed, child: text);
      case 'danger':
        final error = Theme.of(context).colorScheme.error;
        return OutlinedButton(
          style: OutlinedButton.styleFrom(
            foregroundColor: error,
            side: BorderSide(color: error.withValues(alpha: 0.6)),
          ),
          onPressed: onPressed,
          child: text,
        );
      default:
        // The theme paints every filled button primary; tonal needs its own colors.
        final scheme = Theme.of(context).colorScheme;
        return FilledButton(
          style: FilledButton.styleFrom(
            backgroundColor: scheme.primaryContainer,
            foregroundColor: scheme.onPrimaryContainer,
          ),
          onPressed: onPressed,
          child: text,
        );
    }
  }
}

class _Note extends StatelessWidget {
  const _Note(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.colorScheme.onSurfaceVariant;
    return Row(
      children: [
        Icon(Symbols.check_circle_rounded, size: 18, color: muted),
        const SizedBox(width: 8),
        Text(text, style: theme.textTheme.bodyMedium?.copyWith(color: muted)),
      ],
    );
  }
}
