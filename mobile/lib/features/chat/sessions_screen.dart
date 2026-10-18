import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/machines/machine_store.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/mobile_m3_list.dart';
import '../runs/runs_screen.dart';
import 'sessions_repository.dart';

/// Every session on the selected machine: the orchestrator pinned on top, then
/// the rest by recent activity, with a search by name or handle. A tap opens
/// that session's chat.
class SessionsScreen extends ConsumerStatefulWidget {
  const SessionsScreen({super.key});

  @override
  ConsumerState<SessionsScreen> createState() => _SessionsScreenState();
}

class _SessionsScreenState extends ConsumerState<SessionsScreen> {
  final _search = TextEditingController();

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _open(String machineId, PhoneSession session) async {
    await context.push('/chat/${session.ownedId}');
    // Back from the chat: its turn may have started or finished.
    ref.invalidate(sessionsProvider(machineId));
  }

  @override
  Widget build(BuildContext context) {
    final machine = ref.watch(machinesProvider).value?.selected;
    if (machine == null) return const SizedBox.shrink();
    final sessions = ref.watch(sessionsProvider(machine.id));
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
          child: _SearchField(
            controller: _search,
            onChanged: (_) => setState(() {}),
          ),
        ),
        Expanded(
          child: RefreshIndicator(
            onRefresh: () => ref.refresh(sessionsProvider(machine.id).future),
            child: switch (sessions) {
              AsyncValue(:final value?) when showsValue(sessions) =>
                _SessionList(
                  machine: machine,
                  all: value,
                  query: _search.text.trim(),
                  onOpen: (s) => _open(machine.id, s),
                ),
              AsyncValue(:final error?) => ListView(
                physics: const AlwaysScrollableScrollPhysics(),
                children: [
                  RunsMessage('Could not load sessions. ${errorText(error)}'),
                ],
              ),
              _ => const Center(child: CircularProgressIndicator()),
            },
          ),
        ),
      ],
    );
  }
}

class _SearchField extends StatelessWidget {
  const _SearchField({required this.controller, required this.onChanged});

  final TextEditingController controller;
  final ValueChanged<String> onChanged;

  static const _pill = OutlineInputBorder(
    borderRadius: BorderRadius.all(Radius.circular(24)),
    borderSide: BorderSide.none,
  );

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return TextField(
      controller: controller,
      onChanged: onChanged,
      textInputAction: TextInputAction.search,
      decoration: InputDecoration(
        hintText: 'Search by name or handle',
        filled: true,
        fillColor: scheme.surfaceContainerHigh,
        prefixIcon: const Icon(Symbols.search_rounded),
        suffixIcon: controller.text.isEmpty
            ? null
            : IconButton(
                tooltip: 'Clear',
                icon: const Icon(Symbols.close_rounded),
                onPressed: () {
                  controller.clear();
                  onChanged('');
                },
              ),
        contentPadding: const EdgeInsets.symmetric(vertical: 12),
        border: _pill,
        enabledBorder: _pill,
        focusedBorder: _pill,
      ),
    );
  }
}

/// The pinned orchestrator group and the rest, built lazily: there can be
/// hundreds of sessions.
class _SessionList extends StatelessWidget {
  const _SessionList({
    required this.machine,
    required this.all,
    required this.query,
    required this.onOpen,
  });

  final Machine machine;
  final List<PhoneSession> all;

  /// The trimmed search text; empty shows every session.
  final String query;
  final ValueChanged<PhoneSession> onOpen;

  @override
  Widget build(BuildContext context) {
    final shown = filterSessions(all, query);
    final pinned = [...shown.where((s) => s.orchestrator)];
    final rest = [...shown.where((s) => !s.orchestrator)];
    final noOrchestrator = query.isEmpty && !all.any((s) => s.orchestrator);
    // Each row: a section label, the no-orchestrator card, or a session with
    // its place in its group.
    final rows = <Object>[
      if (noOrchestrator || pinned.isNotEmpty) 'Orchestrator',
      if (noOrchestrator) const _NoOrchestrator(),
      for (final s in pinned) (s, MobileM3ListItemPosition.single),
      if (rest.isNotEmpty) 'Sessions · ${rest.length}',
      for (var i = 0; i < rest.length; i++)
        (rest[i], MobileM3ListItemPositionForIndex.forIndex(i, rest.length)),
    ];
    if (shown.isEmpty && !noOrchestrator) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: [
          RunsMessage(
            query.isNotEmpty
                ? 'No session matches "$query".'
                : 'No sessions on ${machine.name} yet.',
          ),
        ],
      );
    }
    return ListView.builder(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
      itemCount: rows.length,
      itemBuilder: (context, i) => switch (rows[i]) {
        final String label => SectionLabel(label),
        (final PhoneSession s, final MobileM3ListItemPosition position) =>
          Column(
            children: [
              if (position == MobileM3ListItemPosition.middle ||
                  position == MobileM3ListItemPosition.last)
                const MobileM3ListDivider(),
              _SessionTile(
                session: s,
                position: position,
                onTap: () => onOpen(s),
              ),
            ],
          ),
        final Widget widget => widget,
        _ => const SizedBox.shrink(),
      },
    );
  }
}

class _SessionTile extends StatelessWidget {
  const _SessionTile({
    required this.session,
    required this.position,
    required this.onTap,
  });

  final PhoneSession session;
  final MobileM3ListItemPosition position;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final tokens = context.tokens;
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: scheme.onSurfaceVariant,
    );
    final s = session;
    final (icon, color) = s.orchestrator
        ? (Symbols.hub_rounded, scheme.primary)
        : s.runId != null
        ? (Symbols.account_tree_rounded, tokens.info)
        : (Symbols.forum_rounded, scheme.secondary);
    return MobileM3ListItem(
      position: position,
      leading: MobileM3LeadingIcon(
        icon: icon,
        backgroundColor: color.withValues(alpha: 0.14),
        foregroundColor: color,
      ),
      title: Text(
        s.name,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: theme.textTheme.titleSmall?.copyWith(
          fontWeight: FontWeight.w600,
        ),
      ),
      supporting: [
        Text(
          // The handle tells apart sessions with the same name, and is what
          // the search matches besides the name.
          [
            providerLabel(s.provider),
            ?folderName(s.project),
            s.handle,
          ].join(' · '),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: muted,
        ),
        if (s.runId != null)
          Text(
            [
              if (s.stepId != null) 'step ${s.stepId}',
              'run ${shortRunId(s.runId!)}',
            ].join(' · '),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: muted?.copyWith(fontSize: 11.5),
          ),
      ],
      trailing: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          if (s.lastActivityAt > 0)
            Text(whenText(s.lastActivityAt), style: muted),
          if (s.state == 'running') ...[
            const SizedBox(height: 6),
            Icon(Icons.circle, size: 8, color: tokens.info),
          ],
        ],
      ),
      onTap: onTap,
    );
  }
}

/// "Claude", "Codex" or "Antigravity"; any other provider id as it is.
String providerLabel(String provider) => switch (provider) {
  'claude' => 'Claude',
  'codex' => 'Codex',
  'antigravity' => 'Antigravity',
  _ => provider,
};

/// `workflow-run-437f73b7-…` as `437f73b7`.
String shortRunId(String runId) {
  final id = runId.replaceFirst('workflow-run-', '');
  return id.length > 8 ? id.substring(0, 8) : id;
}

/// The last part of a project path, or the project as it is.
String? folderName(String? project) {
  if (project == null || project.isEmpty) return null;
  return project
      .split('/')
      .lastWhere((p) => p.isNotEmpty, orElse: () => project);
}

/// The orchestrator slot when none is marked here: point at the home machine
/// when another paired machine hosts it.
class _NoOrchestrator extends ConsumerWidget {
  const _NoOrchestrator();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final saved = ref.watch(machinesProvider).value;
    final machine = saved?.selected;
    final homeId = ref.watch(homeMachineIdProvider).value;
    final home = saved?.visible.where((m) => m.id == homeId).firstOrNull;
    final elsewhere = home != null && home.id != machine?.id;
    return MobileM3ListItem(
      position: MobileM3ListItemPosition.single,
      leading: MobileM3LeadingIcon(
        icon: elsewhere ? Symbols.swap_horiz_rounded : Symbols.hub_rounded,
        backgroundColor: scheme.onSurfaceVariant.withValues(alpha: 0.12),
        foregroundColor: scheme.onSurfaceVariant,
      ),
      title: Text(
        elsewhere
            ? 'The orchestrator lives on ${home.name}'
            : 'No orchestrator on ${machine?.name ?? 'this machine'}',
        style: theme.textTheme.titleSmall?.copyWith(
          fontWeight: FontWeight.w600,
        ),
      ),
      supporting: [
        Text(
          elsewhere
              ? 'Tap to switch to ${home.name}.'
              : 'Mark a session as the orchestrator in Assembly on your computer.',
          style: theme.textTheme.bodySmall?.copyWith(
            color: scheme.onSurfaceVariant,
          ),
        ),
      ],
      onTap: elsewhere
          ? () => ref.read(machinesProvider.notifier).select(home.id)
          : null,
    );
  }
}
