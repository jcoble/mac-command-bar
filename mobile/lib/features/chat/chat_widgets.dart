import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/theme/app_recipes.dart';
import '../../core/theme/app_tokens.dart';
import '../../core/widgets/app_sheet.dart';
import '../../core/widgets/mobile_m3_list.dart';
import '../runs/runs_screen.dart';
import 'chat_repository.dart';

/// The thread's rows: each run of consecutive tool calls is one row, and
/// every other item is a row of its own.
List<List<ChatItem>> chatRows(List<ChatItem> items) {
  final rows = <List<ChatItem>>[];
  for (final item in items) {
    if (item.kind == 'tool' && rows.lastOrNull?.last.kind == 'tool') {
      rows.last.add(item);
    } else {
      rows.add([item]);
    }
  }
  return rows;
}

/// "Ran 3 commands", or "Ran" and the summary for one tool call.
String toolGroupLabel(List<ChatItem> tools) => tools.length == 1
    ? 'Ran ${tools.single.text}'
    : 'Ran ${tools.length} commands';

/// Rental Command's `_MessageThread` (`message_detail_screen.dart`), newest at
/// the bottom. Reversed so new items keep the view pinned to the bottom. A
/// round button scrolls back down once the owner has scrolled up.
class ChatThread extends StatefulWidget {
  const ChatThread({super.key, required this.items, required this.onOpenTools});

  final List<ChatItem> items;

  /// Opens a run of tool calls in the tool sheet.
  final void Function(List<ChatItem> tools) onOpenTools;

  @override
  State<ChatThread> createState() => _ChatThreadState();
}

class _ChatThreadState extends State<ChatThread> {
  final _scroll = ScrollController();
  var _scrolledUp = false;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(() {
      final up = _scroll.offset > 400;
      if (up != _scrolledUp) setState(() => _scrolledUp = up);
    });
  }

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final rows = chatRows(widget.items);
    return Stack(
      alignment: Alignment.bottomCenter,
      children: [
        ListView.builder(
          controller: _scroll,
          reverse: true,
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 16),
          itemCount: rows.length,
          itemBuilder: (context, i) {
            final row = rows[rows.length - 1 - i];
            final item = row.first;
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: switch (item.kind) {
                'owner' => _OwnerBubble(item: item),
                'agent' => _AgentReply(item: item),
                'relayed' => _RelayedCard(item: item),
                'tool' => _ToolGroupRow(
                  label: toolGroupLabel(row),
                  // A single call with no detail has nothing to open.
                  onTap: row.length == 1 && !item.hasDetail
                      ? null
                      : () => widget.onOpenTools(row),
                ),
                _ => _ErrorLine(item: item),
              },
            );
          },
        ),
        if (_scrolledUp)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: IconButton(
              tooltip: 'Scroll to the latest',
              style: IconButton.styleFrom(
                backgroundColor: scheme.surfaceContainerHighest,
                side: BorderSide(color: scheme.outlineVariant),
              ),
              icon: const Icon(Symbols.arrow_downward_rounded),
              onPressed: () => _scroll.animateTo(
                0,
                duration: const Duration(milliseconds: 300),
                curve: Curves.easeOutCubic,
              ),
            ),
          ),
      ],
    );
  }
}

/// Chat text: 17 with room between lines, for owner and agent alike.
TextStyle? _chatText(ThemeData theme) =>
    theme.textTheme.bodyLarge?.copyWith(fontSize: 17, height: 1.45);

/// The owner's message, a rounded bubble on the right.
class _OwnerBubble extends StatelessWidget {
  const _OwnerBubble({required this.item});

  final ChatItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Align(
      alignment: Alignment.centerRight,
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: MediaQuery.sizeOf(context).width * 0.8,
        ),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHigh,
            borderRadius: M3Shape.radiusLargeIncreased,
          ),
          child: Text(item.text, style: _chatText(theme)),
        ),
      ),
    );
  }
}

/// The session's reply: markdown on the background, full width, with a copy
/// button once it is finished.
class _AgentReply extends StatelessWidget {
  const _AgentReply({required this.item});

  final ChatItem item;

  Future<void> _copy(BuildContext context) async {
    final messenger = ScaffoldMessenger.of(context);
    await Clipboard.setData(ClipboardData(text: item.text));
    messenger
      ..hideCurrentSnackBar()
      ..showSnackBar(const SnackBar(content: Text('Copied')));
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final body = _chatText(theme)!;
    final mono = body.copyWith(
      fontFamily: 'monospace',
      fontFamilyFallback: const ['monospace'],
      fontSize: 14.5,
      height: 1.5,
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        MarkdownBody(
          data: item.text,
          // Full width, so code blocks span the screen.
          fitContent: false,
          syntaxHighlighter: _PlainCode(mono),
          styleSheet: MarkdownStyleSheet.fromTheme(theme).copyWith(
            p: body,
            a: body.copyWith(
              color: scheme.primary,
              decoration: TextDecoration.underline,
            ),
            code: mono.copyWith(
              fontSize: 15.5,
              color: scheme.primary,
              backgroundColor: scheme.surfaceContainerHigh,
            ),
            h1: body.copyWith(fontSize: 22, fontWeight: FontWeight.w700),
            h2: body.copyWith(fontSize: 20, fontWeight: FontWeight.w700),
            h3: body.copyWith(fontSize: 18, fontWeight: FontWeight.w700),
            blockSpacing: 8,
            listBullet: body,
            tableHead: body.copyWith(fontWeight: FontWeight.w700),
            tableBody: body,
            codeblockPadding: const EdgeInsets.all(14),
            codeblockDecoration: BoxDecoration(
              color: scheme.surfaceContainerLowest,
              borderRadius: M3Shape.radiusMedium,
              border: Border.all(color: scheme.outlineVariant),
            ),
          ),
        ),
        if (item.done)
          IconButton(
            tooltip: 'Copy',
            visualDensity: VisualDensity.compact,
            padding: EdgeInsets.zero,
            alignment: Alignment.centerLeft,
            icon: Icon(
              Symbols.content_copy_rounded,
              size: 20,
              color: scheme.onSurfaceVariant,
            ),
            onPressed: () => _copy(context),
          ),
      ],
    );
  }
}

/// Code blocks in plain monospace, without the inline-code tint.
class _PlainCode extends SyntaxHighlighter {
  _PlainCode(this.style);

  final TextStyle style;

  @override
  TextSpan format(String source) => TextSpan(text: source, style: style);
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
          Text(item.text.substring(match?.end ?? 0), style: _chatText(theme)),
        ],
      ),
    );
  }
}

/// A run of tool calls as one muted line, "Ran 3 commands ›".
class _ToolGroupRow extends StatelessWidget {
  const _ToolGroupRow({required this.label, this.onTap});

  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = theme.colorScheme.onSurfaceVariant;
    return InkWell(
      onTap: onTap,
      borderRadius: M3Shape.radiusSmall,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Row(
          children: [
            Flexible(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodyLarge?.copyWith(color: color),
              ),
            ),
            if (onTap != null) ...[
              const SizedBox(width: 4),
              Icon(Symbols.chevron_right_rounded, size: 20, color: color),
            ],
          ],
        ),
      ),
    );
  }
}

/// A session error, in red.
class _ErrorLine extends StatelessWidget {
  const _ErrorLine({required this.item});

  final ChatItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = theme.colorScheme.error;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(top: 2),
          child: Icon(Symbols.error_rounded, size: 18, color: color),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            item.text,
            style: theme.textTheme.bodyMedium?.copyWith(color: color),
          ),
        ),
      ],
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

/// Opens a run of tool calls in a sheet: the list, then one call's detail
/// with a back arrow. A single call with detail opens on its detail.
/// [load] reads a call's detail when it is opened.
void showToolSheet(
  BuildContext context,
  List<ChatItem> tools,
  Future<ToolItem> Function(String itemId) load,
) => showAppSheet<void>(context, (_) => _ToolSheet(tools: tools, load: load));

class _ToolSheet extends StatefulWidget {
  const _ToolSheet({required this.tools, required this.load});

  final List<ChatItem> tools;
  final Future<ToolItem> Function(String itemId) load;

  @override
  State<_ToolSheet> createState() => _ToolSheetState();
}

class _ToolSheetState extends State<_ToolSheet> {
  late final _single =
      widget.tools.length == 1 && widget.tools.single.hasDetail;
  Future<ToolItem>? _detail;

  @override
  void initState() {
    super.initState();
    if (_single) _detail = widget.load(widget.tools.single.id);
  }

  @override
  Widget build(BuildContext context) {
    final detail = _detail;
    if (detail == null) {
      return AppSheet(
        title: toolGroupLabel(widget.tools),
        itemCount: widget.tools.length,
        itemBuilder: (context, i) {
          final item = widget.tools[i];
          return _ToolListRow(
            item: item,
            onTap: item.hasDetail
                // A block body: setState must not be handed the Future.
                ? () => setState(() {
                    _detail = widget.load(item.id);
                  })
                : null,
          );
        },
      );
    }
    // The system back gesture returns to the list, as the arrow does.
    return PopScope(
      canPop: _single,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) setState(() => _detail = null);
      },
      child: FutureBuilder<ToolItem>(
        future: detail,
        builder: (context, snapshot) {
          final tool = snapshot.data;
          final error = snapshot.error;
          final head = <Widget>[
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
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.error,
                  ),
                ),
              ),
            if (tool?.summary case final summary?
                when summary.isNotEmpty && summary != tool?.path)
              _ToolField(
                label: tool!.name == 'Bash' ? 'Command' : 'Summary',
                text: summary,
              ),
            if (tool?.path case final path?)
              _ToolField(label: 'Path', text: path, copy: false),
            if (tool?.output case final output? when output.isNotEmpty) ...[
              _ToolField(label: 'Output', text: output),
              if (tool!.outputTruncated) const _CutNote('Output cut at 32 KB'),
            ],
            if (tool?.diff case final diff? when diff.isNotEmpty) ...[
              const _FieldLabel('Diff'),
              if (tool!.diffTruncated) const _CutNote('Diff cut at 64 KB'),
            ],
          ];
          final diff = tool?.diff ?? '';
          final lines = diff.isEmpty ? <String>[] : diff.split('\n');
          if (lines.isNotEmpty && lines.last.isEmpty) lines.removeLast();
          return AppSheet(
            title: tool?.name ?? 'Tool call',
            subtitle: switch (tool?.state) {
              null => null,
              'completed' => 'Completed',
              'failed' => 'Failed',
              _ => 'Running',
            },
            onBack: _single ? null : () => setState(() => _detail = null),
            itemCount: head.length + (lines.isEmpty ? 0 : 1),
            itemBuilder: (context, i) =>
                i < head.length ? head[i] : _DiffBlock(lines),
          );
        },
      ),
    );
  }
}

/// One tool call in the sheet's list: icon, "Ran", its summary.
class _ToolListRow extends StatelessWidget {
  const _ToolListRow({required this.item, this.onTap});

  final ChatItem item;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.colorScheme.onSurfaceVariant;
    return InkWell(
      onTap: onTap,
      borderRadius: M3Shape.radiusSmall,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 12),
        child: Row(
          children: [
            Icon(Symbols.terminal_rounded, size: 20, color: muted),
            const SizedBox(width: 12),
            Text('Ran', style: theme.textTheme.bodyLarge),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                item.text,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodyLarge?.copyWith(color: muted),
              ),
            ),
            if (onTap != null)
              Icon(Symbols.chevron_right_rounded, size: 20, color: muted),
          ],
        ),
      ),
    );
  }
}

class _FieldLabel extends StatelessWidget {
  const _FieldLabel(this.text, {this.trailing});

  final String text;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 16, bottom: 6),
      child: SizedBox(
        height: 36,
        child: Row(
          children: [
            Expanded(
              child: Text(
                text,
                style: theme.textTheme.bodyLarge?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            ?trailing,
          ],
        ),
      ),
    );
  }
}

/// A labelled monospace box in the tool detail, with a copy button.
class _ToolField extends StatefulWidget {
  const _ToolField({required this.label, required this.text, this.copy = true});

  final String label;
  final String text;
  final bool copy;

  @override
  State<_ToolField> createState() => _ToolFieldState();
}

class _ToolFieldState extends State<_ToolField> {
  var _copied = false;

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: widget.text));
    if (mounted) setState(() => _copied = true);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _FieldLabel(
          widget.label,
          trailing: widget.copy
              ? IconButton(
                  tooltip: _copied ? 'Copied' : 'Copy',
                  visualDensity: VisualDensity.compact,
                  icon: Icon(
                    _copied
                        ? Symbols.check_rounded
                        : Symbols.content_copy_rounded,
                    size: 20,
                    color: scheme.onSurfaceVariant,
                  ),
                  onPressed: _copy,
                )
              : null,
        ),
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: scheme.surfaceContainerLowest,
            borderRadius: M3Shape.radiusSmall,
          ),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: SelectableText(
              widget.text,
              style: theme.textTheme.bodyMedium?.copyWith(
                fontFamily: 'monospace',
                fontSize: 14,
                height: 1.5,
              ),
            ),
          ),
        ),
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

/// A unified diff as one block that scrolls sideways as a whole: lines never
/// wrap, and every row is as wide as the longest so the colours line up.
class _DiffBlock extends StatelessWidget {
  const _DiffBlock(this.lines);

  final List<String> lines;

  @override
  Widget build(BuildContext context) => ClipRRect(
    borderRadius: M3Shape.radiusMedium,
    child: ColoredBox(
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      child: LayoutBuilder(
        builder: (context, box) => SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: ConstrainedBox(
            constraints: BoxConstraints(minWidth: box.maxWidth),
            child: IntrinsicWidth(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (final (i, line) in lines.indexed)
                    _DiffLine(line, first: i == 0, last: i == lines.length - 1),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
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
    return Container(
      padding: EdgeInsets.fromLTRB(12, first ? 10 : 1, 12, last ? 10 : 1),
      color: switch (kind) {
        DiffLineKind.added => context.tokens.success.withValues(alpha: 0.16),
        DiffLineKind.removed => scheme.error.withValues(alpha: 0.16),
        _ => null,
      },
      child: Text(
        line,
        softWrap: false,
        style: theme.textTheme.bodySmall?.copyWith(
          fontFamily: 'monospace',
          fontSize: 13,
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
  final head = [
    Padding(
      padding: const EdgeInsets.only(top: 4),
      child: _Counts(file.added, file.removed, large: true),
    ),
    const _FieldLabel('Diff'),
    if (diff == null)
      const _CutNote('This file is past the 256 KB the phone loads.')
    else if (cut)
      const _CutNote('Diff cut at 256 KB'),
  ];
  showAppSheet<void>(
    context,
    (_) => AppSheet(
      title: _fileName(file.path),
      subtitle: switch (_folder(file.path)) {
        final folder? => _middleEllipsis(folder, 40),
        null => null,
      },
      initialSize: 0.7,
      itemCount: head.length + (lines.isEmpty ? 0 : 1),
      itemBuilder: (context, i) =>
          i < head.length ? head[i] : _DiffBlock(lines),
    ),
  );
}

/// `_ComposeBar` + `_SendButton`, channel chips removed.
class ChatComposeBar extends StatelessWidget {
  const ChatComposeBar({
    super.key,
    required this.controller,
    required this.sending,
    required this.onSend,
    this.leading,
  });

  final TextEditingController controller;
  final bool sending;
  final VoidCallback onSend;

  /// Shown under the message box on the left, such as the model pill.
  final Widget? leading;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return SafeArea(
      top: false,
      child: Container(
        margin: const EdgeInsets.fromLTRB(12, 4, 12, 12),
        padding: const EdgeInsets.fromLTRB(18, 4, 8, 8),
        decoration: BoxDecoration(
          color: scheme.surfaceContainerHigh,
          borderRadius: M3Shape.radiusExtraLarge,
          border: Border.all(color: scheme.outlineVariant),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: controller,
              minLines: 1,
              maxLines: 5,
              textCapitalization: TextCapitalization.sentences,
              keyboardType: TextInputType.multiline,
              style: _chatText(theme),
              decoration: const InputDecoration(
                hintText: 'Send a message…',
                filled: false,
                isDense: true,
                contentPadding: EdgeInsets.symmetric(vertical: 12),
                border: InputBorder.none,
                enabledBorder: InputBorder.none,
                focusedBorder: InputBorder.none,
              ),
            ),
            Row(
              children: [
                Expanded(
                  child: Align(
                    alignment: AlignmentDirectional.centerStart,
                    child: leading,
                  ),
                ),
                SizedBox(
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
                                Symbols.arrow_upward_rounded,
                                color: scheme.onPrimary,
                                size: 22,
                              ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ],
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

/// The composer's model pill: the model's label, then its effort in a lighter
/// weight. A tap opens the model and effort sheet.
class ModelChip extends ConsumerWidget {
  const ModelChip({super.key, required this.session});

  final (String, String) session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final config = ref.watch(sessionConfigProvider(session)).value;
    return Material(
      color: scheme.surfaceContainerHighest,
      shape: const StadiumBorder(),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => showModelSheet(context, session),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 8, 8, 8),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: Text.rich(
                  TextSpan(
                    text: config?.modelName ?? 'Model',
                    children: [
                      if (config?.effort case final effort?)
                        TextSpan(
                          text: ' ${effortLabel(effort)}',
                          style: TextStyle(
                            fontWeight: FontWeight.w400,
                            color: scheme.onSurfaceVariant,
                          ),
                        ),
                    ],
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              Icon(
                Symbols.expand_more_rounded,
                size: 18,
                color: scheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The machine's models and efforts for [session], the current ones checked.
void showModelSheet(BuildContext context, (String, String) session) =>
    showAppSheet<void>(context, (_) => _ModelSheet(session));

class _ModelSheet extends ConsumerStatefulWidget {
  const _ModelSheet(this.session);

  final (String, String) session;

  @override
  ConsumerState<_ModelSheet> createState() => _ModelSheetState();
}

class _ModelSheetState extends ConsumerState<_ModelSheet> {
  var _busy = false;
  String? _error;

  /// Sends the choice, then reads the config back so the checks and the chip follow.
  Future<void> _choose({String? model, String? effort}) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await ref
          .read(chatProvider(widget.session).notifier)
          .setConfig(model: model, effort: effort);
      if (!mounted) return;
      ref.invalidate(sessionConfigProvider(widget.session));
      await ref.read(sessionConfigProvider(widget.session).future);
    } on ApiException catch (e) {
      _error = e.message;
    }
    if (mounted) setState(() => _busy = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final config = ref.watch(sessionConfigProvider(widget.session));
    return ConstrainedBox(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.sizeOf(context).height * 0.8,
      ),
      child: ListView(
        shrinkWrap: true,
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
        children: [
          const AppSheetHeader(
            title: 'Model and effort',
            subtitle: 'Used from the next message',
          ),
          SizedBox(
            height: 2,
            child: _busy ? const LinearProgressIndicator() : null,
          ),
          if (_error case final error?)
            Padding(
              padding: const EdgeInsets.fromLTRB(4, 12, 4, 0),
              child: Text(
                error,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: scheme.error,
                ),
              ),
            ),
          ...switch (config) {
            AsyncData(:final value) => [
              if (value.efforts.isNotEmpty) ...[
                const SectionLabel('Effort'),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    for (final effort in value.efforts)
                      ChoiceChip(
                        label: Text(effortLabel(effort)),
                        selected: effort == value.effort,
                        showCheckmark: false,
                        side: BorderSide(color: scheme.outlineVariant),
                        // The current one stays enabled so it reads as chosen, not greyed out.
                        onSelected: _busy
                            ? null
                            : (_) {
                                if (effort != value.effort) {
                                  _choose(effort: effort);
                                }
                              },
                      ),
                  ],
                ),
              ],
              const SectionLabel('Model'),
              for (final (i, model) in value.models.indexed)
                Column(
                  children: [
                    if (i > 0) const MobileM3ListDivider(indent: 16),
                    MobileM3ListItem(
                      position: MobileM3ListItemPositionForIndex.forIndex(
                        i,
                        value.models.length,
                      ),
                      contentPadding: const EdgeInsets.fromLTRB(16, 0, 8, 0),
                      title: Text(
                        model.label,
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: model.id == value.model
                              ? FontWeight.w600
                              : FontWeight.w500,
                        ),
                      ),
                      trailing: Icon(
                        Symbols.check_rounded,
                        color: model.id == value.model
                            ? scheme.primary
                            : Colors.transparent,
                      ),
                      onTap: _busy || model.id == value.model
                          ? null
                          : () => _choose(model: model.id),
                    ),
                  ],
                ),
            ],
            AsyncError(:final error) => [
              Text(
                error is ApiException
                    ? error.message
                    : "Couldn't load this session's models.",
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: scheme.error,
                ),
              ),
            ],
            _ => [
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 32),
                child: Center(child: CircularProgressIndicator()),
              ),
            ],
          },
        ],
      ),
    );
  }
}
