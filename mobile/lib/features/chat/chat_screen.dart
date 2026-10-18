import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/live/live_providers.dart';
import '../../core/machines/machine_store.dart';
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
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              session?.name ?? 'Chat',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            if (session != null)
              Text(
                [
                  providerLabel(session.provider),
                  ?folderName(session.project),
                  session.handle,
                ].join(' · '),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
          ],
        ),
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
                  : ChatThread(items: value.items, working: value.working),
            ),
            ChatComposeBar(
              controller: _text,
              sending: _sending,
              onSend: () => _send(machine.id),
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
