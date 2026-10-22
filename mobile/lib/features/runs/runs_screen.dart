import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/mobile_m3_list.dart';
import 'runs_repository.dart';

/// Whether a screen draws the value in [v]. After the selected machine
/// changes, Riverpod keeps the previous machine's value while the new load
/// runs ([AsyncValue.isReloading]); that value must never be drawn, so the
/// screen shows the spinner or the new load's error instead. Pull to refresh
/// ([AsyncValue.isRefreshing]) keeps its rows.
bool showsValue<T>(AsyncValue<T> v) => v.hasValue && !v.isReloading;

/// The selected machine's runs, grouped: Needs you, Running, Done today.
class RunsScreen extends ConsumerWidget {
  const RunsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final runs = ref.watch(runsProvider);
    return RefreshIndicator(
      onRefresh: () => ref.read(runsProvider.notifier).refresh(),
      child: switch (runs) {
        AsyncValue(:final value?) when showsValue(runs) => _groups(
          groupRuns(value, DateTime.now()),
        ),
        AsyncValue(:final error?) => ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          children: [RunsMessage('Could not load runs. ${errorText(error)}')],
        ),
        _ => const Center(child: CircularProgressIndicator()),
      },
    );
  }

  Widget _groups(RunGroups groups) {
    final shown = [
      ('Needs you', groups.needsYou),
      ('Running', groups.running),
      ('Done today', groups.doneToday),
    ].where((g) => g.$2.isNotEmpty);
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
      children: [
        if (shown.isEmpty) const RunsMessage('No runs today.'),
        for (final (label, runs) in shown) ...[
          SectionLabel('$label · ${runs.length}'),
          _RunGroup(runs),
        ],
      ],
    );
  }
}

typedef RunGroups = ({
  List<RunRow> needsYou,
  List<RunRow> running,
  List<RunRow> doneToday,
});

/// Splits [runs]: unfinished runs waiting on the owner, the other unfinished
/// ones, and finished runs last updated since local midnight of [now]. Runs
/// that finished before today are not shown.
RunGroups groupRuns(RunsList runs, DateTime now) {
  final midnight = DateTime(
    now.year,
    now.month,
    now.day,
  ).millisecondsSinceEpoch;
  bool waiting(RunRow run) =>
      run.state == 'waiting-input' || run.state == 'waiting-approval';
  return (
    needsYou: runs.active.where(waiting).toList(),
    running: runs.active.where((run) => !waiting(run)).toList(),
    doneToday: runs.recent.where((run) => run.updatedAt >= midnight).toList(),
  );
}

class _RunGroup extends StatelessWidget {
  const _RunGroup(this.runs);

  final List<RunRow> runs;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    return Column(
      children: [
        for (var i = 0; i < runs.length; i++) ...[
          if (i > 0) const MobileM3ListDivider(),
          Builder(
            builder: (context) {
              final run = runs[i];
              final look = RunLook.of(context, run.state);
              return MobileM3ListItem(
                position: MobileM3ListItemPositionForIndex.forIndex(
                  i,
                  runs.length,
                ),
                leading: MobileM3LeadingIcon(
                  icon: look.icon,
                  backgroundColor: look.color.withValues(alpha: 0.14),
                  foregroundColor: look.color,
                ),
                title: Text(
                  runTitle(run),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                supporting: [
                  Text(
                    runSubtitle(run),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: muted,
                  ),
                ],
                trailing: StateChip(look),
                onTap: () => context.push('/runs/${run.id}'),
              );
            },
          ),
        ],
      ],
    );
  }
}

/// "TSK-1412 · Phone pairing endpoint", or the workflow when a run has no task.
String runTitle(RunRow run) =>
    [?run.taskKey, run.taskTitle ?? run.workflow].join(' · ');

/// Active: "build · round 2 · 18 min". Finished: "dev-pipeline · PR #251 · Oct 10".
String runSubtitle(RunRow run) {
  final finished = runActionsFor(run.state).isEmpty;
  final pr = run.prUrl == null
      ? null
      : 'PR #${Uri.parse(run.prUrl!).pathSegments.last}';
  return [
    if (finished) run.workflow else run.currentStep ?? run.workflow,
    if (!finished && run.round > 1) 'round ${run.round}',
    ?pr,
    if (finished)
      whenText(run.updatedAt)
    else
      spanText(DateTime.now().millisecondsSinceEpoch - run.startedAt),
  ].join(' · ');
}

String errorText(Object error) =>
    error is ApiException ? error.message : '$error';

/// A length of time in milliseconds: "40 s", "18 min", "1 h 05".
String spanText(int ms) {
  final d = Duration(milliseconds: ms < 0 ? 0 : ms);
  if (d.inSeconds < 1) return 'under 1 s';
  if (d.inMinutes < 1) return '${d.inSeconds} s';
  if (d.inHours < 1) return '${d.inMinutes} min';
  return '${d.inHours} h ${(d.inMinutes % 60).toString().padLeft(2, '0')}';
}

/// "9:38" today, "yesterday", otherwise "Oct 10".
String whenText(int ms) {
  final at = DateTime.fromMillisecondsSinceEpoch(ms);
  final now = DateTime.now();
  final days = DateTime(
    now.year,
    now.month,
    now.day,
  ).difference(DateTime(at.year, at.month, at.day)).inDays;
  if (days == 0) return clockText(ms);
  if (days == 1) return 'yesterday';
  const months = 'JanFebMarAprMayJunJulAugSepOctNovDec';
  return '${months.substring(at.month * 3 - 3, at.month * 3)} ${at.day}';
}

String clockText(int ms) {
  final at = DateTime.fromMillisecondsSinceEpoch(ms);
  return '${at.hour}:${at.minute.toString().padLeft(2, '0')}';
}

/// Label, color and icon for a run state, in the theme's semantic colors.
class RunLook {
  const RunLook(this.label, this.color, this.icon);

  factory RunLook.of(BuildContext context, String state) {
    final tokens = context.tokens;
    final scheme = Theme.of(context).colorScheme;
    return switch (state) {
      'running' => RunLook('Running', tokens.info, Symbols.play_arrow_rounded),
      'paused' => RunLook('Paused', scheme.primary, Symbols.pause_rounded),
      'waiting-input' || 'waiting-approval' => RunLook(
        'Waiting for you',
        tokens.warning,
        Symbols.front_hand_rounded,
      ),
      'completed' => RunLook('Done', tokens.success, Symbols.check_rounded),
      'failed' => RunLook(
        'Failed',
        scheme.error,
        Symbols.priority_high_rounded,
      ),
      'cancelled' => RunLook(
        'Stopped',
        scheme.onSurfaceVariant,
        Symbols.stop_rounded,
      ),
      _ => RunLook('Queued', scheme.onSurfaceVariant, Symbols.schedule_rounded),
    };
  }

  final String label;
  final Color color;
  final IconData icon;
}

/// Rental Command's status chip, tinted with the state's color.
class StateChip extends StatelessWidget {
  const StateChip(this.look, {super.key});

  final RunLook look;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
    decoration: BoxDecoration(
      color: look.color.withValues(alpha: 0.16),
      borderRadius: BorderRadius.circular(20),
    ),
    child: Text(
      look.label,
      style: TextStyle(
        fontSize: 11,
        fontWeight: FontWeight.w600,
        color: look.color,
      ),
    ),
  );
}

class SectionLabel extends StatelessWidget {
  const SectionLabel(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 16, 4, 8),
      child: Text(
        text.toUpperCase(),
        style: theme.textTheme.labelMedium?.copyWith(
          letterSpacing: 0.8,
          fontWeight: FontWeight.w600,
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class RunsMessage extends StatelessWidget {
  const RunsMessage(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 64),
      child: Text(
        text,
        textAlign: TextAlign.center,
        style: theme.textTheme.bodyMedium?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
