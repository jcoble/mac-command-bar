import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';
import '../../core/live/live_providers.dart';
import '../../core/machines/machine_store.dart';
import '../files/files_repository.dart';
import '../files/files_screen.dart';
import '../source_control/source_control_repository.dart';
import '../source_control/source_control_sheet.dart';
import '../terminal/terminal_screen.dart';
import 'chat_repository.dart';
import 'chat_widgets.dart';
import 'sessions_repository.dart';
import 'sessions_screen.dart';

/// One session's conversation on the selected machine (spec §4.1), opened from
/// the sessions list.
class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key, required this.ownedId});

  final String ownedId;

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen> {
  final _text = TextEditingController();
  var _sending = false;

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  Future<void> _send(String machineId) async {
    final text = _text.text.trim();
    if (text.isEmpty) return;
    // Clear first so anything typed while the send is pending is kept.
    _text.clear();
    setState(() => _sending = true);
    try {
      await ref
          .read(chatProvider((machineId, widget.ownedId)).notifier)
          .send(text);
    } on DioException catch (e) {
      if (mounted) {
        // Give the failed message back, unless the owner has typed since.
        if (_text.text.isEmpty) _text.text = text;
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(ApiException.fromDioException(e).message)),
        );
      }
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final machine = ref.watch(machinesProvider).value?.selected;
    if (machine == null) return const SizedBox.shrink();
    final session = ref
        .watch(sessionsProvider(machine.id))
        .value
        ?.where((s) => s.ownedId == widget.ownedId)
        .firstOrNull;
    final chat = ref.watch(chatProvider((machine.id, widget.ownedId)));
    return Scaffold(
      appBar: AppBar(
        centerTitle: true,
        title: Column(
          children: [
            Text(
              session?.name ?? 'Chat',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontSize: 17),
            ),
            if (session != null)
              Text(
                [
                  folderName(session.project) ??
                      '${providerLabel(session.provider)} · ${session.handle}',
                  if (chat.value?.contextLeft case final left?)
                    '$left% context left',
                ].join(' · '),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontSize: 13,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
          ],
        ),
        actions: [
          if (chat.value?.working ?? false)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 8),
              child: RepaintBoundary(
                child: SizedBox.square(
                  dimension: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            ),
          IconButton(
            tooltip: 'Files',
            icon: const Icon(Symbols.folder_rounded),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) => FilesScreen(
                  files: FilesRepository(
                    ref.read(machineClientProvider(machine.id)),
                    widget.ownedId,
                  ),
                  title: folderName(session?.project) ?? session?.name,
                ),
              ),
            ),
          ),
          IconButton(
            tooltip: 'Terminal',
            icon: const Icon(Symbols.terminal_rounded),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) => TerminalScreen(
                  ownedId: widget.ownedId,
                  title: folderName(session?.project) ?? session?.name,
                ),
              ),
            ),
          ),
          IconButton(
            tooltip: 'Source Control',
            icon: const Icon(Symbols.fork_right_rounded),
            onPressed: () => showSourceControlSheet(
              context,
              SourceControlRepository(
                ref.read(machineClientProvider(machine.id)),
                widget.ownedId,
              ),
              title: folderName(session?.project) ?? session?.name,
            ),
          ),
          IconButton(
            tooltip: 'Changes',
            icon: const Icon(Symbols.difference_rounded),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) => ChangesScreen(
                  title: session?.name,
                  load: ref
                      .read(chatProvider((machine.id, widget.ownedId)).notifier)
                      .changes,
                ),
              ),
            ),
          ),
        ],
      ),
      body: switch (chat) {
        AsyncData(:final value) => Column(
          children: [
            Expanded(
              child: value.items.isEmpty && !value.working
                  ? const ChatNotice(
                      icon: Symbols.forum_rounded,
                      title: 'Start a conversation',
                      message:
                          'Your message goes to this session as your own turn.',
                    )
                  : ChatThread(
                      items: value.items,
                      onOpenTools: (tools) => showToolSheet(
                        context,
                        tools,
                        ref
                            .read(
                              chatProvider((
                                machine.id,
                                widget.ownedId,
                              )).notifier,
                            )
                            .toolItem,
                      ),
                    ),
            ),
            ChatComposeBar(
              controller: _text,
              sending: _sending,
              onSend: () => _send(machine.id),
              leading: ModelChip(session: (machine.id, widget.ownedId)),
            ),
          ],
        ),
        AsyncError(:final error) => ChatNotice(
          icon: Symbols.cloud_off_rounded,
          title: error is ApiException && error.statusCode == 404
              ? 'This session is gone'
              : "Can't reach ${machine.name}",
          message: error is ApiException
              ? error.message
              : 'Something went wrong loading the chat.',
          action: FilledButton.tonalIcon(
            onPressed: () => ref.invalidate(liveSocketProvider(machine.id)),
            icon: const Icon(Symbols.refresh_rounded),
            label: const Text('Check again'),
          ),
        ),
        _ => const Center(child: CircularProgressIndicator()),
      },
    );
  }
}
