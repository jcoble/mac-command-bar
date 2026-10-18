import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/live/live_providers.dart';
import '../../core/machines/machine_store.dart';
import 'chat_repository.dart';
import 'chat_widgets.dart';

/// The orchestrator conversation on the selected machine (spec §4.1).
class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key});

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
    setState(() => _sending = true);
    try {
      await ref.read(chatProvider(machineId).notifier).send(text);
      _text.clear();
    } on DioException catch (e) {
      if (mounted) {
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
    final chat = ref.watch(chatProvider(machine.id));
    return switch (chat) {
      AsyncData(:final value) => Column(
        children: [
          Expanded(
            child: value.items.isEmpty && !value.working
                ? const ChatNotice(
                    icon: Symbols.forum_rounded,
                    title: 'Start a conversation',
                    message:
                        'Ask the orchestrator to start a run, check on one, '
                        'or pass a message along.',
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
      AsyncError(:final error) => _ErrorState(
        machine: machine,
        error: error,
        // A fresh socket also rebuilds the chat, so a newly marked
        // orchestrator's live updates reach the phone.
        onRetry: () {
          ref.invalidate(homeMachineIdProvider);
          ref.invalidate(liveSocketProvider(machine.id));
        },
      ),
      _ => const Center(child: CircularProgressIndicator()),
    };
  }
}

/// No orchestrator here (404): point at the home machine when another paired
/// machine hosts it. Otherwise the machine is unreachable.
class _ErrorState extends ConsumerWidget {
  const _ErrorState({
    required this.machine,
    required this.error,
    required this.onRetry,
  });

  final Machine machine;
  final Object error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final retry = FilledButton.tonalIcon(
      onPressed: onRetry,
      icon: const Icon(Symbols.refresh_rounded),
      label: const Text('Check again'),
    );
    if (error is! ApiException || (error as ApiException).statusCode != 404) {
      return ChatNotice(
        icon: Symbols.cloud_off_rounded,
        title: "Can't reach ${machine.name}",
        message: error is ApiException
            ? (error as ApiException).message
            : 'Something went wrong loading the chat.',
        action: retry,
      );
    }
    final saved = ref.watch(machinesProvider).value;
    final homeId = ref.watch(homeMachineIdProvider).value;
    final home = saved?.visible.where((m) => m.id == homeId).firstOrNull;
    if (home != null && home.id != machine.id) {
      return ChatNotice(
        icon: Symbols.hub_rounded,
        title: 'The orchestrator lives on ${home.name}',
        message: '${machine.name} does not host it. Switch to talk to it.',
        action: FilledButton.icon(
          onPressed: () => ref.read(machinesProvider.notifier).select(home.id),
          icon: const Icon(Symbols.swap_horiz_rounded),
          label: Text('Switch to ${home.name}'),
        ),
      );
    }
    return ChatNotice(
      icon: Symbols.hub_rounded,
      title: 'No orchestrator on ${machine.name}',
      message:
          'In Assembly on your computer, mark a session as the orchestrator. '
          'Then check again.',
      action: retry,
    );
  }
}
