import 'package:flutter/material.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/theme/app_recipes.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/mobile_m3_list.dart';
import '../runs/runs_screen.dart';
import 'chat_repository.dart';

/// Rental Command's `_MessageThread` (`message_detail_screen.dart`), newest at
/// the bottom. Reversed so new items keep the view pinned to the bottom.
class ChatThread extends StatelessWidget {
  const ChatThread({
    super.key,
    required this.items,
    required this.working,
    required this.onOpenTool,
  });

  final List<ChatItem> items;
  final bool working;

  /// Opens a tool line that has detail.
  final void Function(ChatItem item) onOpenTool;

  @override
  Widget build(BuildContext context) {
    final count = items.length + (working ? 1 : 0);
    return ListView.builder(
      reverse: true,
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
      itemCount: count,
      itemBuilder: (context, i) {
        if (working && i == 0) return const _WorkingLine();
        final item = items[count - 1 - i];
        return Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: switch (item.kind) {
            'owner' || 'agent' => _MessageBubble(item: item),
            'relayed' => _RelayedCard(item: item),
            _ => _ToolLine(
              item: item,
              onTap: item.kind == 'tool' && item.hasDetail
                  ? () => onOpenTool(item)
                  : null,
            ),
          },
        );
      },
    );
  }
}

/// `_MessageBubble`: the owner on the right in primary, the session on the
/// left in surfaceContainerHighest.
class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.item});

  final ChatItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final mine = item.kind == 'owner';
    return Align(
      alignment: mine ? Alignment.centerRight : Alignment.centerLeft,
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: MediaQuery.sizeOf(context).width * 0.78,
        ),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: mine ? scheme.primary : scheme.surfaceContainerHighest,
            borderRadius: BorderRadius.only(
              topLeft: const Radius.circular(16),
              topRight: const Radius.circular(16),
              bottomLeft: Radius.circular(mine ? 16 : 4),
              bottomRight: Radius.circular(mine ? 4 : 16),
            ),
          ),
          child: Text(
            item.text,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: mine ? scheme.onPrimary : scheme.onSurface,
              height: 1.4,
            ),
          ),
        ),
      ),
    );
  }
}

/// A message another session sent this one: `[message from <label>]\n<text>`.
class _RelayedCard extends StatelessWidget {
  const _RelayedCard({required this.item});

  final ChatItem item;

  static final _prefix = RegExp(r'^\[message from ([^\]]*)\]\s*');

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final match = _prefix.firstMatch(item.text);
    final sender = match?.group(1) ?? 'another session';
    return M3TonalCard(
      family: M3TonalFamily.sky,
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'MESSAGE FROM $sender',
            style: theme.textTheme.labelSmall?.copyWith(
              color: context.tokens.info,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.6,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            item.text.substring(match?.end ?? 0),
            style: theme.textTheme.bodyMedium?.copyWith(height: 1.4),
          ),
        ],
      ),
    );
  }
}

/// One grey line per tool call ("2 runs started"), or an error in red. A
/// tool call with detail opens on tap.
class _ToolLine extends StatelessWidget {
  const _ToolLine({required this.item, this.onTap});

  final ChatItem item;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final error = item.kind == 'error';
    final color = error
        ? theme.colorScheme.error
        : theme.colorScheme.onSurfaceVariant;
    return InkWell(
      onTap: onTap,
      borderRadius: M3Shape.radiusSmall,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              error
                  ? Symbols.error_rounded
                  : item.done
                  ? Symbols.check_rounded
                  : Symbols.more_horiz_rounded,
              size: 16,
              color: error || !item.done ? color : theme.colorScheme.secondary,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                item.text,
                maxLines: error ? null : 2,
                overflow: error ? null : TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: color,
                  fontFamily: error ? null : 'monospace',
                  fontSize: error ? null : 11.5,
                ),
              ),
            ),
            if (onTap != null)
              Icon(Symbols.chevron_right_rounded, size: 16, color: color),
          ],
        ),
      ),
    );
  }
}

enum DiffLineKind { header, hunk, added, removed, context }

/// What a unified-diff line is: a file header, a hunk line, an added or
/// removed line, or unchanged context.
DiffLineKind diffLineKind(String line) {
  if (line.startsWith('diff ') ||
      line.startsWith('index ') ||
      line.startsWith('--- ') ||
      line.startsWith('+++ ')) {
    return DiffLineKind.header;
  }
  if (line.startsWith('@@')) return DiffLineKind.hunk;
  if (line.startsWith('+')) return DiffLineKind.added;
  if (line.startsWith('-')) return DiffLineKind.removed;
  return DiffLineKind.context;
}

/// Opens [item]'s detail in a bottom sheet; [detail] is its fetch, started on tap.
void showToolSheet(
  BuildContext context,
  ChatItem item,
  Future<ToolItem> detail,
) => showModalBottomSheet<void>(
  context: context,
  showDragHandle: true,
  isScrollControlled: true,
  builder: (_) => DraggableScrollableSheet(
    expand: false,
    initialChildSize: 0.6,
    minChildSize: 0.3,
    maxChildSize: 0.9,
    builder: (context, controller) => FutureBuilder<ToolItem>(
      future: detail,
      builder: (context, snapshot) {
        final diff = snapshot.data?.diff ?? '';
        final lines = diff.isEmpty ? const <String>[] : diff.split('\n');
        if (lines.isNotEmpty && lines.last.isEmpty) lines.removeLast();
        return ListView.builder(
          controller: controller,
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 32),
          itemCount: 1 + lines.length,
          itemBuilder: (context, i) => i == 0
              ? _ToolHead(item: item, snapshot: snapshot)
              : _DiffLine(lines[i - 1], first: i == 1, last: i == lines.length),
        );
      },
    ),
  ),
);

/// The sheet above the diff: name, state, path, command, output.
class _ToolHead extends StatelessWidget {
  const _ToolHead({required this.item, required this.snapshot});

  final ChatItem item;
  final AsyncSnapshot<ToolItem> snapshot;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final mono = theme.textTheme.bodySmall?.copyWith(
      fontFamily: 'monospace',
      fontSize: 11.5,
      height: 1.4,
    );
    final tool = snapshot.data;
    final error = snapshot.error;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                tool?.name ?? item.text,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            if (tool != null)
              StateChip(
                RunLook.of(context, switch (tool.state) {
                  'completed' || 'failed' => tool.state,
                  _ => 'running',
                }),
              ),
          ],
        ),
        if (tool?.path case final path?) ...[
          const SizedBox(height: 4),
          Text(
            _middleEllipsis(path, 48),
            maxLines: 1,
            style: mono?.copyWith(color: scheme.onSurfaceVariant),
          ),
        ],
        if (tool?.summary case final summary?
            when summary.isNotEmpty && summary != tool?.path) ...[
          const SizedBox(height: 12),
          Text(
            summary,
            maxLines: 4,
            overflow: TextOverflow.ellipsis,
            style: mono,
          ),
        ],
        if (snapshot.connectionState != ConnectionState.done)
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 32),
            child: Center(child: CircularProgressIndicator()),
          )
        else if (error != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 24),
            child: Text(
              error is ApiException
                  ? error.message
                  : "Couldn't load this tool call.",
              style: theme.textTheme.bodyMedium?.copyWith(color: scheme.error),
            ),
          ),
        if (tool?.output case final output? when output.isNotEmpty) ...[
          const SectionLabel('Output'),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: scheme.surfaceContainerLowest,
              borderRadius: M3Shape.radiusMedium,
            ),
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: SelectableText(output, style: mono),
            ),
          ),
          if (tool!.outputTruncated) _CutNote('Output cut at 32 KB'),
        ],
        if (tool?.diff case final diff? when diff.isNotEmpty) ...[
          const SectionLabel('Diff'),
          if (tool!.diffTruncated) _CutNote('Diff cut at 64 KB'),
        ],
      ],
    );
  }
}

/// [text] with its middle replaced by "…" when longer than [max] characters.
String _middleEllipsis(String text, int max) => text.length <= max
    ? text
    : '${text.substring(0, max ~/ 3)}…${text.substring(text.length - (max - max ~/ 3 - 1))}';

class _CutNote extends StatelessWidget {
  const _CutNote(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 6, bottom: 6),
      child: Text(
        text,
        style: theme.textTheme.labelSmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

/// One unified-diff line: added green, removed red, hunks muted, headers bold.
class _DiffLine extends StatelessWidget {
  const _DiffLine(this.line, {required this.first, required this.last});

  final String line;
  final bool first;
  final bool last;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final kind = diffLineKind(line);
    const radius = Radius.circular(M3Shape.medium);
    return Container(
      padding: EdgeInsets.fromLTRB(12, first ? 10 : 1, 12, last ? 10 : 1),
      decoration: BoxDecoration(
        color: switch (kind) {
          DiffLineKind.added => context.tokens.success.withValues(alpha: 0.16),
          DiffLineKind.removed => scheme.error.withValues(alpha: 0.16),
          _ => scheme.surfaceContainerLowest,
        },
        borderRadius: BorderRadius.vertical(
          top: first ? radius : Radius.zero,
          bottom: last ? radius : Radius.zero,
        ),
      ),
      child: Text(
        line,
        style: theme.textTheme.bodySmall?.copyWith(
          fontFamily: 'monospace',
          fontSize: 11.5,
          height: 1.4,
          fontWeight: kind == DiffLineKind.header ? FontWeight.w700 : null,
          color: kind == DiffLineKind.hunk ? scheme.onSurfaceVariant : null,
        ),
      ),
    );
  }
}

/// The files a session's branch changed, from the chat's Changes button.
/// [load] reads them; pull down to read again.
class ChangesScreen extends StatefulWidget {
  const ChangesScreen({super.key, this.title, required this.load});

  /// The session's name, under "Changes".
  final String? title;
  final Future<BranchChanges> Function() load;

  @override
  State<ChangesScreen> createState() => _ChangesScreenState();
}

class _ChangesScreenState extends State<ChangesScreen> {
  late Future<BranchChanges> _changes = widget.load();

  Future<void> _reload() async {
    final next = widget.load();
    setState(() => _changes = next);
    try {
      await next;
    } on Object {
      // The FutureBuilder shows the error.
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Changes'),
            if (widget.title case final title?)
              Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
          ],
        ),
      ),
      body: RefreshIndicator(
        onRefresh: _reload,
        child: FutureBuilder<BranchChanges>(
          future: _changes,
          builder: (context, snapshot) {
            final changes = snapshot.data;
            final error = snapshot.error;
            Widget notice(IconData icon, String title, String message) =>
                CustomScrollView(
                  physics: const AlwaysScrollableScrollPhysics(),
                  slivers: [
                    SliverFillRemaining(
                      hasScrollBody: false,
                      child: ChatNotice(
                        icon: icon,
                        title: title,
                        message: message,
                      ),
                    ),
                  ],
                );
            if (snapshot.connectionState != ConnectionState.done &&
                changes == null) {
              return const Center(child: CircularProgressIndicator());
            }
            if (error != null) {
              return notice(
                Symbols.difference_rounded,
                "Can't show changes",
                error is ApiException
                    ? error.message
                    : 'Something went wrong reading the changes.',
              );
            }
            if (changes!.files.isEmpty) {
              return notice(
                Symbols.check_circle_rounded,
                'No changes on this branch',
                'Nothing here differs from ${changes.base}.',
              );
            }
            final files = changes.files;
            return ListView.builder(
              physics: const AlwaysScrollableScrollPhysics(),
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
              itemCount: files.length + 1,
              itemBuilder: (context, i) {
                if (i == 0) return _ChangesTotal(changes);
                final file = files[i - 1];
                final position = MobileM3ListItemPositionForIndex.forIndex(
                  i - 1,
                  files.length,
                );
                return Column(
                  children: [
                    if (i > 1) const MobileM3ListDivider(indent: 16),
                    MobileM3ListItem(
                      position: position,
                      title: Text(
                        _fileName(file.path),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      supporting: [
                        if (_folder(file.path) case final folder?)
                          Text(
                            folder,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                      ],
                      trailing: _Counts(file.added, file.removed),
                      onTap: () => _showFileDiff(
                        context,
                        file,
                        changes.diffs.elementAtOrNull(i - 1),
                        cut:
                            changes.diffTruncated &&
                            i - 1 >= changes.diffs.length - 1,
                      ),
                    ),
                  ],
                );
              },
            );
          },
        ),
      ),
    );
  }
}

String _fileName(String path) => path.substring(path.lastIndexOf('/') + 1);

String? _folder(String path) {
  final slash = path.lastIndexOf('/');
  return slash < 0 ? null : path.substring(0, slash);
}

/// "4 files · +120 −37", compared with the base branch.
class _ChangesTotal extends StatelessWidget {
  const _ChangesTotal(this.changes);

  final BranchChanges changes;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final count = changes.files.length;
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 8, 4, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                '$count ${count == 1 ? 'file' : 'files'}  ·  ',
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              _Counts(changes.added, changes.removed, large: true),
            ],
          ),
          const SizedBox(height: 2),
          Text(
            'Compared with ${changes.base}',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

/// "+N" in green and "−M" in red, in monospace. A file row leaves out a zero;
/// the [large] totals and sheet head show both.
class _Counts extends StatelessWidget {
  const _Counts(this.added, this.removed, {this.large = false});

  final int added;
  final int removed;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style =
        (large ? theme.textTheme.titleSmall : theme.textTheme.labelMedium)
            ?.copyWith(fontFamily: 'monospace', fontWeight: FontWeight.w600);
    return Wrap(
      spacing: 8,
      children: [
        if (added > 0 || large)
          Text(
            '+$added',
            style: style?.copyWith(color: context.tokens.success),
          ),
        if (removed > 0 || large)
          Text(
            '\u2212$removed',
            style: style?.copyWith(color: theme.colorScheme.error),
          ),
      ],
    );
  }
}

/// One changed file's part of the branch diff in a bottom sheet, from its
/// first hunk. [diff] is null when the file is past the cut.
void _showFileDiff(
  BuildContext context,
  ({String path, int added, int removed}) file,
  String? diff, {
  required bool cut,
}) {
  final all = diff == null ? <String>[] : diff.split('\n');
  if (all.isNotEmpty && all.last.isEmpty) all.removeLast();
  final hunk = all.indexWhere((line) => line.startsWith('@@'));
  final lines = hunk < 0 ? all : all.sublist(hunk);
  showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    isScrollControlled: true,
    builder: (_) => DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.7,
      minChildSize: 0.3,
      maxChildSize: 0.9,
      builder: (context, controller) {
        final theme = Theme.of(context);
        final head = Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    _fileName(file.path),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                _Counts(file.added, file.removed, large: true),
              ],
            ),
            if (_folder(file.path) case final folder?) ...[
              const SizedBox(height: 4),
              Text(
                _middleEllipsis(folder, 48),
                maxLines: 1,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  fontSize: 11.5,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
            const SectionLabel('Diff'),
            if (diff == null)
              const _CutNote('This file is past the 256 KB the phone loads.')
            else if (cut)
              const _CutNote('Diff cut at 256 KB'),
          ],
        );
        return ListView.builder(
          controller: controller,
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 32),
          itemCount: 1 + lines.length,
          itemBuilder: (context, i) => i == 0
              ? head
              : _DiffLine(lines[i - 1], first: i == 1, last: i == lines.length),
        );
      },
    ),
  );
}

/// The thin "working" line while the session's turn runs. A still dot:
/// an endless animation would repaint the screen for the whole turn.
class _WorkingLine extends StatelessWidget {
  const _WorkingLine();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 8, 4, 4),
      child: Row(
        children: [
          Icon(Icons.circle, size: 8, color: theme.colorScheme.primary),
          const SizedBox(width: 10),
          Text(
            'Working…',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

/// `_ComposeBar` + `_SendButton`, channel chips removed.
class ChatComposeBar extends StatelessWidget {
  const ChatComposeBar({
    super.key,
    required this.controller,
    required this.sending,
    required this.onSend,
  });

  final TextEditingController controller;
  final bool sending;
  final VoidCallback onSend;

  static const _pill = OutlineInputBorder(
    borderRadius: BorderRadius.all(Radius.circular(24)),
    borderSide: BorderSide.none,
  );

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return SafeArea(
      top: false,
      child: Container(
        decoration: BoxDecoration(
          color: scheme.surface,
          border: Border(top: BorderSide(color: scheme.outlineVariant)),
        ),
        padding: const EdgeInsets.all(8),
        child: TextField(
          controller: controller,
          minLines: 1,
          maxLines: 5,
          textCapitalization: TextCapitalization.sentences,
          keyboardType: TextInputType.multiline,
          decoration: InputDecoration(
            hintText: 'Send a message…',
            filled: true,
            fillColor: scheme.surfaceContainerHighest,
            contentPadding: const EdgeInsetsDirectional.fromSTEB(18, 12, 8, 12),
            suffixIcon: Padding(
              padding: const EdgeInsetsDirectional.only(end: 6),
              child: SizedBox(
                width: 40,
                height: 40,
                child: Material(
                  color: scheme.primary,
                  shape: const CircleBorder(),
                  child: InkWell(
                    customBorder: const CircleBorder(),
                    onTap: sending ? null : onSend,
                    child: Center(
                      child: sending
                          ? SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: scheme.onPrimary,
                              ),
                            )
                          : Icon(
                              Symbols.send_rounded,
                              color: scheme.onPrimary,
                              size: 20,
                            ),
                    ),
                  ),
                ),
              ),
            ),
            suffixIconConstraints: const BoxConstraints(
              minWidth: 48,
              minHeight: 40,
            ),
            border: _pill,
            enabledBorder: _pill,
            focusedBorder: _pill,
          ),
        ),
      ),
    );
  }
}

/// A centred icon, title, explanation and optional button for the chat's
/// empty, unreachable and no-orchestrator states.
class ChatNotice extends StatelessWidget {
  const ChatNotice({
    super.key,
    required this.icon,
    required this.title,
    required this.message,
    this.action,
  });

  final IconData icon;
  final String title;
  final String message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 64,
              height: 64,
              decoration: BoxDecoration(
                color: scheme.secondaryContainer,
                shape: BoxShape.circle,
              ),
              child: Icon(icon, size: 30, color: scheme.onSecondaryContainer),
            ),
            const SizedBox(height: 20),
            Text(
              title,
              textAlign: TextAlign.center,
              style: theme.textTheme.titleLarge,
            ),
            const SizedBox(height: 8),
            Text(
              message,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: scheme.onSurfaceVariant,
                height: 1.4,
              ),
            ),
            if (action != null) ...[const SizedBox(height: 24), action!],
          ],
        ),
      ),
    );
  }
}
