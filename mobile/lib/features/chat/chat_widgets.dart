import 'package:flutter/material.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/theme/app_recipes.dart';
import '../../core/theme/app_tokens.dart';
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
