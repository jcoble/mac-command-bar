import 'package:flutter/material.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/api/api_exception.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/app_sheet.dart';
import '../../core/widgets/mobile_m3_list.dart';
import '../chat/chat_widgets.dart';
import '../runs/runs_screen.dart';
import 'source_control_repository.dart';

/// A session's Source Control, from the chat's branch button: the branch and
/// sync menus, the changed files (tap one for its diff), the commit box, and
/// the open pull requests. [title] is the session's folder or name.
void showSourceControlSheet(
  BuildContext context,
  SourceControlRepository git, {
  String? title,
}) => showAppSheet<void>(
  context,
  (_) => _SourceControlSheet(git: git, title: title),
);

class _SourceControlSheet extends StatefulWidget {
  const _SourceControlSheet({required this.git, this.title});

  final SourceControlRepository git;
  final String? title;

  @override
  State<_SourceControlSheet> createState() => _SourceControlSheetState();
}

class _SourceControlSheetState extends State<_SourceControlSheet> {
  final _message = TextEditingController();
  late Future<GitStatus> _status = widget.git.status();
  late Future<({String current, List<String> names})> _branches = widget.git
      .branches();
  late Future<List<PullRequest>> _prs = widget.git.prs();

  /// What is running, such as "Pulling", while an action waits on the machine.
  String? _busy;
  String? _error;

  /// The file whose diff is open, and its diff.
  String? _file;
  Future<({String diff, bool capped})>? _diff;

  @override
  void dispose() {
    _message.dispose();
    super.dispose();
  }

  /// Runs [action], shows git's message if it fails, then reads the status
  /// and branches again (and the pull requests when [prs]).
  Future<void> _run(
    String busy,
    Future<void> Function() action, {
    bool prs = false,
  }) async {
    setState(() {
      _busy = busy;
      _error = null;
    });
    try {
      await action();
    } on ApiException catch (e) {
      _error = e.message;
    }
    if (!mounted) return;
    setState(() {
      _busy = null;
      _status = widget.git.status();
      _branches = widget.git.branches();
      if (prs) _prs = widget.git.prs();
    });
  }

  void _sync(String action) => switch (action) {
    'refresh' => _run('Refreshing', () async {}, prs: true),
    'pull' => _run('Pulling', () => widget.git.sync('pull')),
    'fetch' => _run('Fetching', () => widget.git.sync('fetch')),
    _ => _run('Pushing', () => widget.git.sync('push'), prs: true),
  };

  Future<void> _switch(String name) async {
    final yes = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Switch to $name?'),
        content: const Text(
          "The session's folder checks out this branch. Git refuses if "
          'changes are in the way.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Switch'),
          ),
        ],
      ),
    );
    if (yes == true) await _run('Switching', () => widget.git.switchTo(name));
  }

  Future<void> _commit() => _run('Committing', () async {
    await widget.git.commit(_message.text);
    _message.clear();
  });

  @override
  Widget build(BuildContext context) {
    // The commit box stays above the keyboard.
    return Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: _file == null ? _overview(context) : _diffView(context, _file!),
    );
  }

  Widget _overview(BuildContext context) {
    final theme = Theme.of(context);
    return FutureBuilder<GitStatus>(
      future: _status,
      builder: (context, snapshot) {
        final status = snapshot.data;
        final error = snapshot.error;
        final children = <Widget>[
          if (status != null) ...[
            _topRow(context, status),
            if (_error case final error?)
              Padding(
                padding: const EdgeInsets.fromLTRB(4, 12, 4, 0),
                child: Text(
                  error,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.error,
                  ),
                ),
              ),
            SectionLabel('Changes · ${status.files.length}'),
            if (status.files.isEmpty)
              const _Muted('Nothing to commit: the folder matches the branch.')
            else
              for (final (i, file) in status.files.indexed)
                _FileRow(
                  file: file,
                  index: i,
                  count: status.files.length,
                  onTap: () => setState(() {
                    _file = file.path;
                    _diff = widget.git.diff(file.path);
                  }),
                ),
            _commitBox(context, status),
            const SectionLabel('Pull requests'),
            _pullRequests(context),
          ] else if (error != null)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 24),
              child: Text(
                error is ApiException
                    ? error.message
                    : "Couldn't read this folder's source control.",
                style: theme.textTheme.bodyLarge?.copyWith(
                  color: theme.colorScheme.error,
                ),
              ),
            )
          else
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 32),
              child: Center(child: CircularProgressIndicator()),
            ),
        ];
        return AppSheet(
          title: 'Source Control',
          subtitle: widget.title,
          itemCount: children.length,
          itemBuilder: (context, i) => children[i],
        );
      },
    );
  }

  /// The branch menu with "ahead N · behind M" under it, and the sync menu.
  Widget _topRow(BuildContext context, GitStatus status) {
    final theme = Theme.of(context);
    final muted = theme.colorScheme.onSurfaceVariant;
    final current = status.branch ?? 'Detached HEAD';
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              FutureBuilder(
                future: _branches,
                builder: (context, branches) => PopupMenuButton<String>(
                  enabled: _busy == null,
                  tooltip: 'Switch branch',
                  position: PopupMenuPosition.under,
                  onSelected: _switch,
                  itemBuilder: (_) => [
                    if (branches.error case final error?)
                      PopupMenuItem(
                        enabled: false,
                        child: Text(
                          error is ApiException
                              ? error.message
                              : "Couldn't list branches.",
                        ),
                      ),
                    for (final name in branches.data?.names ?? <String>[])
                      PopupMenuItem(
                        value: name,
                        enabled: name != status.branch,
                        child: Row(
                          children: [
                            Icon(
                              Symbols.check_rounded,
                              size: 20,
                              color: name == status.branch
                                  ? theme.colorScheme.primary
                                  : Colors.transparent,
                            ),
                            const SizedBox(width: 12),
                            Flexible(
                              child: Text(
                                name,
                                overflow: TextOverflow.ellipsis,
                                style: theme.textTheme.bodyLarge,
                              ),
                            ),
                          ],
                        ),
                      ),
                  ],
                  child: _Pill(
                    icon: Symbols.fork_right_rounded,
                    label: current,
                  ),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(4, 6, 0, 0),
                child: Text(
                  status.hasUpstream
                      ? 'ahead ${status.ahead} · behind ${status.behind}'
                      : 'No upstream branch',
                  style: theme.textTheme.bodyMedium?.copyWith(color: muted),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(width: 8),
        PopupMenuButton<String>(
          enabled: _busy == null,
          tooltip: 'Sync',
          position: PopupMenuPosition.under,
          onSelected: _sync,
          itemBuilder: (_) => [
            for (final (value, label, icon) in [
              ('pull', 'Pull', Symbols.download_rounded),
              ('fetch', 'Fetch', Symbols.cloud_sync_rounded),
              ('refresh', 'Refresh', Symbols.refresh_rounded),
              ('push', 'Push', Symbols.upload_rounded),
            ])
              PopupMenuItem(
                value: value,
                child: Row(
                  children: [
                    Icon(icon, size: 20),
                    const SizedBox(width: 12),
                    Text(label, style: theme.textTheme.bodyLarge),
                  ],
                ),
              ),
          ],
          child: _Pill(
            icon: Symbols.sync_rounded,
            label: _busy ?? 'Sync',
            busy: _busy != null,
          ),
        ),
      ],
    );
  }

  Widget _commitBox(BuildContext context, GitStatus status) {
    final theme = Theme.of(context);
    final unstaged = status.files.any((file) => file.unstaged);
    return Container(
      margin: const EdgeInsets.only(top: 20),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainer,
        borderRadius: M3Shape.radiusLarge,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            controller: _message,
            minLines: 3,
            maxLines: 6,
            textCapitalization: TextCapitalization.sentences,
            style: theme.textTheme.bodyLarge?.copyWith(fontSize: 17),
            decoration: const InputDecoration(hintText: 'Commit message'),
          ),
          const SizedBox(height: 12),
          ValueListenableBuilder(
            valueListenable: _message,
            builder: (context, message, _) => Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: _busy == null && unstaged
                        ? () => _run('Staging', widget.git.stageAll)
                        : null,
                    child: const Text('Stage All'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: FilledButton(
                    onPressed: _busy == null && message.text.trim().isNotEmpty
                        ? _commit
                        : null,
                    child: const Text('Commit'),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _pullRequests(BuildContext context) {
    final theme = Theme.of(context);
    return FutureBuilder<List<PullRequest>>(
      future: _prs,
      builder: (context, snapshot) {
        final prs = snapshot.data;
        final error = snapshot.error;
        if (error != null) {
          return _Muted(
            error is ApiException
                ? error.message
                : "Couldn't list pull requests.",
          );
        }
        if (prs == null) {
          return const Padding(
            padding: EdgeInsets.symmetric(vertical: 16),
            child: Center(child: CircularProgressIndicator()),
          );
        }
        if (prs.isEmpty) return const _Muted('No open pull requests.');
        return Column(
          children: [
            for (final (i, pr) in prs.indexed) ...[
              if (i > 0) const MobileM3ListDivider(indent: 16),
              MobileM3ListItem(
                position: MobileM3ListItemPositionForIndex.forIndex(
                  i,
                  prs.length,
                ),
                title: Text(
                  pr.title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodyLarge?.copyWith(fontSize: 17),
                ),
                supporting: [
                  Text(
                    '#${pr.number} · ${pr.branch}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
                trailing: Icon(
                  Symbols.open_in_new_rounded,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                // Android hands a github.com link to the GitHub app when it is installed.
                onTap: () => launchUrl(
                  Uri.parse(pr.url),
                  mode: LaunchMode.externalApplication,
                ),
              ),
            ],
          ],
        );
      },
    );
  }

  Widget _diffView(BuildContext context, String path) {
    final theme = Theme.of(context);
    final slash = path.lastIndexOf('/');
    void back() => setState(() => _file = null);
    // The system back gesture returns to the list, as the arrow does.
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) back();
      },
      child: FutureBuilder(
        future: _diff,
        builder: (context, snapshot) {
          final diff = snapshot.data;
          final error = snapshot.error;
          final lines = diff == null || diff.diff.isEmpty
              ? <String>[]
              : diff.diff.split('\n');
          if (lines.isNotEmpty && lines.last.isEmpty) lines.removeLast();
          final children = <Widget>[
            if (error != null)
              Text(
                error is ApiException
                    ? error.message
                    : "Couldn't load the diff.",
                style: theme.textTheme.bodyLarge?.copyWith(
                  color: theme.colorScheme.error,
                ),
              )
            else if (diff == null)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 32),
                child: Center(child: CircularProgressIndicator()),
              )
            else if (lines.isEmpty)
              const _Muted('No text changes to show.')
            else ...[
              if (diff.capped) const _Muted('Diff cut at 256 KB'),
              DiffBlock(lines),
            ],
          ];
          return AppSheet(
            title: path.substring(slash + 1),
            subtitle: slash < 0 ? null : path.substring(0, slash),
            onBack: back,
            itemCount: children.length,
            itemBuilder: (context, i) => children[i],
          );
        },
      ),
    );
  }
}

/// One changed file: its status letter, name, and folder (with "Staged").
class _FileRow extends StatelessWidget {
  const _FileRow({
    required this.file,
    required this.index,
    required this.count,
    required this.onTap,
  });

  final ({String path, String status, bool staged, bool unstaged}) file;
  final int index;
  final int count;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.colorScheme.onSurfaceVariant;
    final slash = file.path.lastIndexOf('/');
    final color = switch (file.status) {
      'M' => context.tokens.warning,
      'A' => context.tokens.success,
      'D' => theme.colorScheme.error,
      '?' => context.tokens.info,
      _ => muted,
    };
    return Column(
      children: [
        if (index > 0) const MobileM3ListDivider(indent: 16),
        MobileM3ListItem(
          position: MobileM3ListItemPositionForIndex.forIndex(index, count),
          leading: SizedBox(
            width: 24,
            child: Text(
              file.status,
              textAlign: TextAlign.center,
              style: theme.textTheme.titleMedium?.copyWith(
                fontFamily: 'monospace',
                fontWeight: FontWeight.w700,
                color: color,
              ),
            ),
          ),
          title: Text(
            file.path.substring(slash + 1),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodyLarge?.copyWith(fontSize: 17),
          ),
          supporting: [
            if (slash > 0 || file.staged)
              Text(
                [
                  if (slash > 0) file.path.substring(0, slash),
                  if (file.staged) file.unstaged ? 'Partly staged' : 'Staged',
                ].join(' · '),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodyMedium?.copyWith(color: muted),
              ),
          ],
          trailing: Icon(Symbols.chevron_right_rounded, color: muted),
          onTap: onTap,
        ),
      ],
    );
  }
}

/// A dropdown's face: icon, label, chevron; a spinner in place of the icon
/// while [busy].
class _Pill extends StatelessWidget {
  const _Pill({required this.icon, required this.label, this.busy = false});

  final IconData icon;
  final String label;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      height: 44,
      padding: const EdgeInsets.symmetric(horizontal: 14),
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.outlineVariant),
        borderRadius: M3Shape.radiusLarge,
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (busy)
            const SizedBox.square(
              dimension: 18,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          else
            Icon(icon, size: 20),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodyLarge?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          const SizedBox(width: 4),
          Icon(Symbols.expand_more_rounded, size: 20),
        ],
      ),
    );
  }
}

class _Muted extends StatelessWidget {
  const _Muted(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 4, 4, 8),
      child: Text(
        text,
        style: theme.textTheme.bodyLarge?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
