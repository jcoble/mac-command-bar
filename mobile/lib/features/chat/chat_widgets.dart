import 'package:flutter/material.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/theme/app_recipes.dart';
import '../../core/theme/app_tokens.dart';
import 'chat_repository.dart';

/// Rental Command's `_MessageThread` (`message_detail_screen.dart`), newest at
/// the bottom. Reversed so new items keep the view pinned to the bottom.
class ChatThread extends StatelessWidget {
  const ChatThread({super.key, required this.items, required this.working});

  final List<ChatItem> items;
  final bool working;

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
            _ => _ToolLine(item: item),
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

/// One grey line per tool call ("2 runs started"), or an error in red.
class _ToolLine extends StatelessWidget {
  const _ToolLine({required this.item});

  final ChatItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final error = item.kind == 'error';
    final color = error
        ? theme.colorScheme.error
        : theme.colorScheme.onSurfaceVariant;
    return Padding(
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
        ],
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
