import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/machines/machine_store.dart';
import '../../core/theme/app_recipes.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/mobile_m3_list.dart';
import 'runs_repository.dart';
import 'runs_screen.dart';

/// One run: header, Pause/Resume and Stop, the waiting banner, its steps and
/// its recent activity.
class RunScreen extends ConsumerStatefulWidget {
  const RunScreen({super.key, required this.runId});

  final String runId;

  @override
  ConsumerState<RunScreen> createState() => _RunScreenState();
}

class _RunScreenState extends ConsumerState<RunScreen> {
  RunAction? _busy;

  Future<void> _act(RunAction action) async {
    // Stop shows its spinner only once the owner has said yes.
    if (action != RunAction.stop) setState(() => _busy = action);
    try {
      await ref
          .read(runsProvider.notifier)
          .act(
            widget.runId,
            action,
            confirm: () async {
              final yes = await _confirmStop();
              if (yes && mounted) setState(() => _busy = action);
              return yes;
            },
          );
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(errorText(error))));
      }
    } finally {
      if (mounted) setState(() => _busy = null);
    }
  }

  Future<bool> _confirmStop() async =>
      await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Stop this run?'),
          content: const Text('Running turns are cancelled.'),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              style: FilledButton.styleFrom(
                backgroundColor: Theme.of(context).colorScheme.error,
                foregroundColor: Theme.of(context).colorScheme.onError,
              ),
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Stop'),
            ),
          ],
        ),
      ) ??
      false;

  @override
  Widget build(BuildContext context) {
    final run = ref.watch(runProvider(widget.runId));
    final machine = ref.watch(machinesProvider).value?.selected;
    return Scaffold(
      appBar: AppBar(
        title: const Text('Run'),
        actions: [
          if (machine != null)
            Padding(
              padding: const EdgeInsets.only(right: 12),
              child: Chip(
                avatar: Icon(
                  Icons.circle,
                  size: 10,
                  color: context.tokens.success,
                ),
                label: Text(machine.name),
              ),
            ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: () => ref.refresh(runProvider(widget.runId).future),
        child: switch (run) {
          AsyncValue(:final value?) => _body(context, value),
          AsyncValue(:final error?) => ListView(
            children: [
              RunsMessage('Could not load this run. ${errorText(error)}'),
            ],
          ),
          _ => const Center(child: CircularProgressIndicator()),
        },
      ),
    );
  }

  Widget _body(BuildContext context, RunDetail detail) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final run = detail.row;
    final look = RunLook.of(context, run.state);
    final actions = runActionsFor(run.state);
    final stopped = run.state == 'failed'
        ? detail.steps.where((s) => s.state == 'failed').firstOrNull
        : null;
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 32),
      children: [
        Text(
          [?run.taskKey, run.workflow].join(' · '),
          style: theme.textTheme.labelLarge?.copyWith(color: scheme.secondary),
        ),
        const SizedBox(height: 2),
        Text(
          run.taskTitle ?? run.workflow,
          style: theme.textTheme.headlineSmall?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 10),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            StateChip(look),
            if (detail.branch != null) _Tag(detail.branch!),
            _Tag("I'll merge: ${run.iMerge ? 'on' : 'off'}"),
            if (run.prUrl != null)
              _Tag(
                'PR #${Uri.parse(run.prUrl!).pathSegments.last}',
                icon: Symbols.content_copy_rounded,
                onTap: () {
                  Clipboard.setData(ClipboardData(text: run.prUrl!));
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('PR link copied')),
                  );
                },
              ),
          ],
        ),
        if (stopped != null) ...[
          const SizedBox(height: 12),
          Text(
            stoppedLine(stopped),
            style: theme.textTheme.bodyMedium?.copyWith(color: scheme.error),
          ),
        ],
        if (actions.isNotEmpty) ...[
          const SizedBox(height: 16),
          Row(
            children: [
              for (final action in actions) ...[
                if (action != actions.first) const SizedBox(width: 8),
                Expanded(child: _actionButton(context, action)),
              ],
            ],
          ),
        ],
        if (run.waitingItemId != null) ...[
          const SizedBox(height: 16),
          M3TonalCard(
            family: M3TonalFamily.amber,
            padding: const EdgeInsets.fromLTRB(16, 12, 12, 12),
            onTap: () =>
                context.go('/inbox/${Uri.encodeComponent(run.waitingItemId!)}'),
            child: Row(
              children: [
                Expanded(
                  child: Text.rich(
                    TextSpan(
                      children: [
                        TextSpan(
                          text: 'Waiting for you: ',
                          style: TextStyle(
                            color: context.tokens.warning,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        TextSpan(text: '${run.currentStep} needs your answer.'),
                      ],
                    ),
                    style: theme.textTheme.bodyMedium,
                  ),
                ),
                Text(
                  'Open',
                  style: theme.textTheme.labelLarge?.copyWith(
                    color: scheme.primary,
                  ),
                ),
                Icon(Symbols.chevron_right_rounded, color: scheme.primary),
              ],
            ),
          ),
        ],
        const SectionLabel('Steps'),
        for (var i = 0; i < detail.steps.length; i++) ...[
          if (i > 0) const MobileM3ListDivider(indent: 60),
          _StepRow(
            detail,
            detail.steps[i],
            MobileM3ListItemPositionForIndex.forIndex(i, detail.steps.length),
          ),
        ],
        if (detail.timeline.isNotEmpty) ...[
          const SectionLabel('Recent activity'),
          for (final line in detail.timeline.reversed)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 3),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SizedBox(
                    width: 48,
                    child: Text(
                      clockText(line.at),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                        fontFeatures: const [FontFeature.tabularFigures()],
                      ),
                    ),
                  ),
                  Expanded(
                    child: Text(line.text, style: theme.textTheme.bodySmall),
                  ),
                ],
              ),
            ),
        ],
      ],
    );
  }

  Widget _actionButton(BuildContext context, RunAction action) {
    final busy = _busy == action;
    final onPressed = _busy == null ? () => _act(action) : null;
    final label = Text(switch (action) {
      RunAction.pause => 'Pause',
      RunAction.resume => 'Resume',
      RunAction.stop => 'Stop',
    });
    final child = busy
        ? const SizedBox.square(
            dimension: 18,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : label;
    if (action == RunAction.stop) {
      final error = Theme.of(context).colorScheme.error;
      return OutlinedButton(
        style: OutlinedButton.styleFrom(
          foregroundColor: error,
          side: BorderSide(color: error.withValues(alpha: 0.6)),
        ),
        onPressed: onPressed,
        child: child,
      );
    }
    return FilledButton(onPressed: onPressed, child: child);
  }
}

/// "Stopped at checks: one or more commands failed", as the desktop says it.
String stoppedLine(RunStep step) {
  final why = whyText(step.attempts.lastOrNull);
  if (why == null) return 'Stopped at ${step.id}';
  return 'Stopped at ${step.id}: ${why[0].toLowerCase()}${why.substring(1)}';
}

/// Why an attempt ended, in one line: a command step's output is shown in
/// its sheet, so it reads as the desktop's "One or more commands failed".
String? whyText(RunAttempt? attempt) {
  if (attempt == null) return null;
  if (attempt.failingCommands.isNotEmpty) return 'One or more commands failed';
  return attempt.summary?.split('\n').first;
}

/// What a step's state means now, in the desktop Workflows view's words.
String stepLine(RunDetail run, RunStep step) {
  final attempt = step.attempts.lastOrNull;
  final start = attempt?.startedAt;
  final span = start == null
      ? null
      : spanText(
          (attempt?.endedAt ?? DateTime.now().millisecondsSinceEpoch) - start,
        );
  switch (step.state) {
    case 'completed':
      return span == null ? 'Completed' : 'Completed in $span';
    case 'failed':
      return whyText(attempt) ?? 'Failed';
    case 'starting' || 'running':
      return span == null ? 'Starting' : 'Running for $span';
    case 'ready' || 'queued':
      return 'Queued to start';
    case 'waiting-approval':
      return 'Waiting for your approval';
    case 'waiting-input':
      return whyText(attempt) ?? 'Waiting for your answer';
    case 'skipped':
      return 'Skipped';
  }
  // Blocked or cancelled: say whether it ran at all, and if not, why not.
  if (start != null) return whyText(attempt) ?? 'Stopped before it finished';
  final failed = run.steps.where((s) => s.state == 'failed').firstOrNull;
  if (failed != null) return 'Not run — ${failed.id} failed';
  if (runActionsFor(run.row.state).isEmpty) {
    return 'Not run — the run stopped first';
  }
  return 'Waiting to start';
}

class _StepRow extends StatelessWidget {
  const _StepRow(this.run, this.step, this.position);

  final RunDetail run;
  final RunStep step;
  final MobileM3ListItemPosition position;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final tokens = context.tokens;
    final (color, icon) = switch (step.state) {
      'completed' => (tokens.success, Symbols.check_rounded),
      'failed' => (scheme.error, Symbols.priority_high_rounded),
      'starting' || 'running' => (tokens.info, Symbols.play_arrow_rounded),
      'waiting-input' ||
      'waiting-approval' => (tokens.warning, Symbols.front_hand_rounded),
      'cancelled' => (scheme.onSurfaceVariant, Symbols.stop_rounded),
      _ => (scheme.outline, Symbols.more_horiz_rounded),
    };
    final attempts = step.attempts.lastOrNull?.n ?? 0;
    final current = step.id == run.row.currentStep;
    return MobileM3ListItem(
      position: position,
      promoted: current,
      contentPadding: const EdgeInsets.fromLTRB(4, 2, 16, 2),
      leading: Container(
        width: 28,
        height: 28,
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.16),
          shape: BoxShape.circle,
        ),
        child: Icon(icon, size: 16, color: color),
      ),
      title: Text(
        step.id,
        style: theme.textTheme.titleSmall?.copyWith(
          fontWeight: FontWeight.w600,
        ),
      ),
      supporting: [
        Text(
          stepLine(run, step),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: theme.textTheme.bodySmall?.copyWith(
            color: scheme.onSurfaceVariant,
          ),
        ),
      ],
      trailing: attempts > 1
          ? Text(
              '$attempts attempts',
              style: theme.textTheme.labelSmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            )
          : null,
      onTap: () => showModalBottomSheet<void>(
        context: context,
        showDragHandle: true,
        isScrollControlled: true,
        builder: (_) => _StepSheet(run, step),
      ),
    );
  }
}

/// A step's latest attempt: summary, review findings, failing command output.
class _StepSheet extends StatelessWidget {
  const _StepSheet(this.run, this.step);

  final RunDetail run;
  final RunStep step;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final attempt = step.attempts.lastOrNull;
    final summary = attempt?.failingCommands.isEmpty ?? false
        ? attempt?.summary
        : null;
    return ConstrainedBox(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.sizeOf(context).height * 0.75,
      ),
      child: ListView(
        shrinkWrap: true,
        padding: const EdgeInsets.fromLTRB(20, 0, 20, 32),
        children: [
          Text(
            step.id,
            style: theme.textTheme.titleLarge?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            [
              switch (step.kind) {
                'run' => 'Commands',
                'review' => 'Review',
                'gate' => 'Approval gate',
                _ => 'Agent',
              },
              if (attempt != null) 'attempt ${attempt.n}',
              stepLine(run, step),
            ].join(' · '),
            style: theme.textTheme.bodySmall?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
          if (summary != null && summary != stepLine(run, step)) ...[
            const SizedBox(height: 16),
            Text(summary, style: theme.textTheme.bodyMedium),
          ],
          if (step.kind != 'run' &&
              (attempt?.findings.isNotEmpty ?? false)) ...[
            const SectionLabel('Findings'),
            for (final finding in attempt!.findings)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text('•  $finding', style: theme.textTheme.bodyMedium),
              ),
          ],
          for (final command in attempt?.failingCommands ?? const []) ...[
            SectionLabel('Failed · exit ${command.exit ?? '?'}'),
            Text(
              command.command,
              style: theme.textTheme.bodySmall?.copyWith(
                fontFamily: 'monospace',
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: scheme.surfaceContainerLowest,
                borderRadius: M3Shape.radiusMedium,
              ),
              child: Text(
                command.tail.isEmpty ? '(no output)' : command.tail,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  height: 1.4,
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// An outlined pill for the branch, merge setting and PR link.
class _Tag extends StatelessWidget {
  const _Tag(this.text, {this.icon, this.onTap});

  final String text;
  final IconData? icon;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Material(
      color: Colors.transparent,
      shape: StadiumBorder(side: BorderSide(color: scheme.outlineVariant)),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                text,
                style: theme.textTheme.labelMedium?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
              if (icon != null) ...[
                const SizedBox(width: 4),
                Icon(icon, size: 14, color: scheme.onSurfaceVariant),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
