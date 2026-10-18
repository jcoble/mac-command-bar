import 'dart:async';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/api/api_exception.dart';
import '../../core/api/machine_client.dart';
import '../../core/live/live_providers.dart';

/// One chat line (spec §5.3 `ChatItem`): kind is owner, agent, relayed, tool or error.
class ChatItem {
  const ChatItem({
    required this.id,
    required this.kind,
    required this.text,
    required this.at,
    required this.done,
    this.hasDetail = false,
    this.images = 0,
    this.localImages = const [],
  });

  factory ChatItem.fromJson(Map<String, dynamic> json) => ChatItem(
    id: json['id'] as String,
    kind: json['kind'] as String,
    text: json['text'] as String,
    at: json['at'] as int,
    done: json['done'] as bool,
    hasDetail: json['hasDetail'] == true,
    images: json['images'] as int? ?? 0,
  );

  final String id;
  final String kind;
  final String text;

  /// Milliseconds since the epoch.
  final int at;
  final bool done;

  /// A tool call with output, a path or a diff to open.
  final bool hasDetail;

  /// How many images an owner message carries.
  final int images;

  /// The files of the images this phone sent with the message, shown as thumbnails.
  final List<String> localImages;
}

/// The `Content-Type` the machine accepts for an image at [path], from its
/// extension; null for anything else.
String? imageMimeType(String path) =>
    switch (path.split('.').last.toLowerCase()) {
      'png' => 'image/png',
      'jpg' || 'jpeg' => 'image/jpeg',
      'webp' => 'image/webp',
      'gif' => 'image/gif',
      _ => null,
    };

/// `GET sessions/{id}/items/{itemId}`: one tool call whole, output and diff capped.
class ToolItem {
  ToolItem.fromJson(Map<String, dynamic> json)
    : name = json['name'] as String,
      state = json['state'] as String,
      summary = json['summary'] as String?,
      path = json['path'] as String?,
      output = json['output'] as String?,
      outputTruncated = json['outputTruncated'] == true,
      diff = json['diff'] as String?,
      diffTruncated = json['diffTruncated'] == true;

  final String name;

  /// started, updated, completed or failed.
  final String state;
  final String? summary;
  final String? path;
  final String? output;
  final bool outputTruncated;
  final String? diff;
  final bool diffTruncated;
}

/// `GET sessions/{id}/changes`: what the session's branch changed since [base].
class BranchChanges {
  BranchChanges.fromJson(Map<String, dynamic> json)
    : base = json['base'] as String,
      files = [
        for (final f in json['files'] as List)
          (
            path: f['path'] as String,
            added: f['added'] as int,
            removed: f['removed'] as int,
          ),
      ],
      diffs = splitDiffFiles(json['diff'] as String),
      diffTruncated = json['diffTruncated'] == true;

  /// The branch compared with, e.g. `origin/main`.
  final String base;
  final List<({String path, int added, int removed})> files;

  /// Each file's part of the diff, in the order of [files]; fewer than
  /// [files] when the diff was cut.
  final List<String> diffs;
  final bool diffTruncated;

  int get added => files.fold(0, (sum, f) => sum + f.added);
  int get removed => files.fold(0, (sum, f) => sum + f.removed);
}

/// [diff] split before each line that starts `diff --git `, one part per file.
List<String> splitDiffFiles(String diff) => [
  for (final part in diff.split(RegExp(r'^(?=diff --git )', multiLine: true)))
    if (part.isNotEmpty) part,
];

/// `GET sessions/{id}/config`: the models and efforts the machine offers this
/// session, and the ones it has chosen.
class SessionConfig {
  SessionConfig.fromJson(Map<String, dynamic> json)
    : models = [
        for (final m in json['models'] as List)
          (
            id: m['id'] as String,
            label: m['label'] as String? ?? modelLabel(m['id'] as String),
          ),
      ],
      model = json['model'] as String?,
      efforts = [for (final e in json['efforts'] as List) e as String],
      effort = json['effort'] as String?;

  final List<({String id, String label})> models;
  final String? model;
  final List<String> efforts;
  final String? effort;

  /// The current model's label, e.g. "Opus 5"; null when none is chosen.
  String? get modelName => switch (model) {
    final model? =>
      models.where((m) => m.id == model).firstOrNull?.label ??
          modelLabel(model),
    null => null,
  };
}

/// A model id as the desktop words it: `gpt-5.6-sol` → "GPT-5.6 Sol",
/// `claude-opus-5` → "Opus 5" (`agentConfigLabels.ts` `modelLabel`).
String modelLabel(String id) {
  final parts = [
    for (final part in id.trim().split(RegExp(r'[-_\s]+')))
      ...part.split(RegExp(r'(?<=[a-z0-9])(?=[A-Z])')),
  ].where((part) => part.isNotEmpty).toList();
  if (parts.isEmpty) return id;
  String word(String part) =>
      const {'gpt', 'llm', 'api', 'xl'}.contains(part.toLowerCase())
      ? part.toUpperCase()
      : part.contains(RegExp(r'\d'))
      ? part
      : part[0].toUpperCase() + part.substring(1);
  if (parts.length > 1 && parts[0].toLowerCase() == 'gpt') {
    return ['GPT-${parts[1]}', ...parts.skip(2).map(word)].join(' ');
  }
  const vendors = {'claude', 'openai', 'anthropic', 'google', 'meta'};
  final named = parts.length > 1 && vendors.contains(parts[0].toLowerCase())
      ? parts.skip(1)
      : parts;
  return named.map(word).join(' ');
}

/// An effort id as the desktop words it: `xhigh` → "Extra High".
String effortLabel(String effort) =>
    const {
      'minimal': 'Minimal',
      'low': 'Low',
      'medium': 'Medium',
      'high': 'High',
      'xhigh': 'Extra High',
      'max': 'Max',
    }[effort.toLowerCase()] ??
    modelLabel(effort);

/// How much of the context window is left, 0 to 100, from the chat's `usage`;
/// null when it is missing or incomplete. Rounded as the desktop's meter is.
int? contextLeftPercent(Map<String, dynamic>? usage) {
  final used = usage?['usedTokens'];
  final window = usage?['contextWindow'];
  if (used is! int || window is! int || window <= 0) return null;
  return 100 - (used * 100 / window).round().clamp(0, 100);
}

/// The open chat's session config; refreshed after the owner changes it.
final sessionConfigProvider = FutureProvider.autoDispose
    .family<SessionConfig, (String, String)>((ref, session) async {
      try {
        final response = await ref
            .watch(machineClientProvider(session.$1))
            .get<Map<String, dynamic>>(
              'sessions/${Uri.encodeComponent(session.$2)}/config',
            );
        return SessionConfig.fromJson(response.data!);
      } on DioException catch (e) {
        throw ApiException.fromDioException(e);
      }
    });

class ChatState {
  const ChatState({
    required this.items,
    required this.cursor,
    required this.working,
    this.contextLeft,
  });

  final List<ChatItem> items;

  /// The highest event sequence applied; live frames at or below it are duplicates.
  final int cursor;

  /// The session's turn is running.
  final bool working;

  /// Percent of the context window left, from the latest read; null when unknown.
  final int? contextLeft;
}

/// One session's chat, keyed by (machine id, owned id) (Rental Command's
/// `conversationProvider` pattern). Live frames and refetches run one at a
/// time, in arrival order. While it is open the machine's live socket watches
/// this session, so its chat frames are the ones that come.
class ChatNotifier extends Notifier<AsyncValue<ChatState>> {
  ChatNotifier(this._session);

  final (String, String) _session;
  Future<void> _tail = Future.value();

  Dio get _dio => ref.read(machineClientProvider(_session.$1));

  String get _path => 'sessions/${Uri.encodeComponent(_session.$2)}/chat';

  @override
  AsyncValue<ChatState> build() {
    final socket = ref.watch(liveSocketProvider(_session.$1));
    final subscription = socket.frames.listen((f) => _enqueue(() => _apply(f)));
    socket.watch(_session.$2);
    ref.onDispose(() {
      subscription.cancel();
      socket.watch(null);
    });
    _enqueue(_fetch);
    return const AsyncValue.loading();
  }

  void _enqueue(Future<void> Function() step) => _tail = _after(_tail, step);

  Future<void> _after(
    Future<void> previous,
    Future<void> Function() step,
  ) async {
    try {
      await previous;
    } on Object {
      // That step already ended and its caller saw the error; keep the queue moving.
    }
    if (ref.mounted) await step();
  }

  /// Reloads the latest 100 items (the retry button).
  Future<void> load() {
    state = const AsyncValue.loading();
    _enqueue(_fetch);
    return _tail;
  }

  /// `GET sessions/{id}/chat`, or only what came after the cursor once loaded.
  /// Never throws.
  Future<void> _fetch() async {
    final loaded = state.value;
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        _path,
        queryParameters: {'after': ?loaded?.cursor},
      );
      if (!ref.mounted) return;
      final body = response.data!;
      final items = [
        for (final json in body['items'] as List)
          ChatItem.fromJson(json as Map<String, dynamic>),
      ];
      final cursor = body['cursor'] as int;
      state = AsyncValue.data(
        ChatState(
          items: loaded == null ? items : _merge(loaded.items, items),
          cursor: loaded == null || cursor > loaded.cursor
              ? cursor
              : loaded.cursor,
          working: body['turn'] == 'running',
          contextLeft:
              contextLeftPercent(body['usage'] as Map<String, dynamic>?) ??
              loaded?.contextLeft,
        ),
      );
    } on DioException catch (e) {
      if (!ref.mounted) return;
      if (loaded == null) {
        state = AsyncValue.error(
          ApiException.fromDioException(e),
          StackTrace.current,
        );
      }
    }
  }

  Future<void> _apply(Map<String, dynamic> frame) async {
    if (frame['type'] == 'open') return _fetch();
    // A frame sent before the machine saw the phone switch chats belongs to
    // the chat it left.
    if (frame['ownedId'] != _session.$2) return;
    final current = state.value;
    final seq = frame['seq'];
    if (current == null || seq is! int || seq <= current.cursor) return;
    var items = current.items;
    var working = current.working;
    switch (frame['type']) {
      case 'chat':
        items = _merge(items, [
          ChatItem.fromJson(frame['item'] as Map<String, dynamic>),
        ]);
      case 'chatDelta':
        final id = frame['itemId'] as String;
        final delta = frame['delta'] as String;
        final at = items.indexWhere((i) => i.id == id);
        if (at < 0) {
          items = [
            ...items,
            ChatItem(
              id: id,
              kind: 'agent',
              text: delta,
              at: DateTime.now().millisecondsSinceEpoch,
              done: false,
            ),
          ];
        } else if (!items[at].done) {
          final open = items[at];
          items = [...items]
            ..[at] = ChatItem(
              id: id,
              kind: open.kind,
              text: open.text + delta,
              at: open.at,
              done: false,
            );
        }
      case 'turn':
        working = frame['state'] == 'running';
    }
    state = AsyncValue.data(
      ChatState(
        items: items,
        cursor: seq,
        working: working,
        contextLeft: current.contextLeft,
      ),
    );
    // A finished turn has new usage: read it with what came after the cursor.
    if (frame['type'] == 'turn' && !working) await _fetch();
  }

  /// The tool call [itemId] whole, read when the owner opens it.
  Future<ToolItem> toolItem(String itemId) async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        'sessions/${Uri.encodeComponent(_session.$2)}/items/${Uri.encodeComponent(itemId)}',
      );
      return ToolItem.fromJson(response.data!);
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  /// The session branch's changed files and diff, read when the owner opens them.
  Future<BranchChanges> changes() async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        'sessions/${Uri.encodeComponent(_session.$2)}/changes',
      );
      return BranchChanges.fromJson(response.data!);
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  /// `POST sessions/{id}/attachments` with the image at [path] → its id, for
  /// [send]. Throws an [ApiException], before sending anything for a file that
  /// is not an image.
  Future<String> uploadImage(
    String path, {
    void Function(double)? onProgress,
  }) async {
    final mimeType = imageMimeType(path);
    if (mimeType == null) {
      throw const ApiException(
        statusCode: 415,
        message: 'Only PNG, JPEG, WebP and GIF images can be attached',
      );
    }
    try {
      final response = await _dio.post<Map<String, dynamic>>(
        'sessions/${Uri.encodeComponent(_session.$2)}/attachments',
        data: await File(path).readAsBytes(),
        options: Options(contentType: mimeType),
        onSendProgress: onProgress == null
            ? null
            : (sent, total) => onProgress(total > 0 ? sent / total : 0),
      );
      return response.data!['attachmentId'] as String;
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  /// `POST sessions/{id}/chat`, with the ids of images already uploaded. Shows
  /// the owner's bubble at once, with [localImages] as its thumbnails; the live
  /// `chat` frame for the same item replaces it and keeps them. Plain chat text
  /// never answers an inbox item (D13).
  Future<void> send(
    String text, {
    List<String> attachmentIds = const [],
    List<String> localImages = const [],
  }) async {
    final response = await _dio.post<Map<String, dynamic>>(
      _path,
      data: {
        'text': text,
        if (attachmentIds.isNotEmpty) 'attachmentIds': attachmentIds,
      },
    );
    final id = response.data!['itemId'] as String;
    _enqueue(() async {
      final current = state.value;
      if (current == null) return;
      // The live frame may have brought the item first; it still gets the thumbnails.
      final live = current.items.where((i) => i.id == id).firstOrNull;
      if (live != null && localImages.isEmpty) return;
      state = AsyncValue.data(
        ChatState(
          items: _merge(current.items, [
            ChatItem(
              id: id,
              kind: 'owner',
              text: live?.text ?? text,
              at: live?.at ?? DateTime.now().millisecondsSinceEpoch,
              done: true,
              images: live?.images ?? attachmentIds.length,
              localImages: localImages,
            ),
          ]),
          cursor: current.cursor,
          working: current.working,
          contextLeft: current.contextLeft,
        ),
      );
    });
    await _tail;
  }

  /// `POST sessions/{id}/config` with only the fields given. Throws the
  /// backend's refusal as an [ApiException].
  Future<void> setConfig({String? model, String? effort}) async {
    try {
      await _dio.post<void>(
        'sessions/${Uri.encodeComponent(_session.$2)}/config',
        data: {'model': ?model, 'effort': ?effort},
      );
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }
}

/// [items] with each of [updates] replacing the item with its id, or appended.
/// A replaced item keeps the local image files this phone sent it with.
List<ChatItem> _merge(List<ChatItem> items, List<ChatItem> updates) {
  final merged = [...items];
  for (final update in updates) {
    final at = merged.indexWhere((i) => i.id == update.id);
    if (at < 0) {
      merged.add(update);
    } else {
      final old = merged[at];
      merged[at] = old.localImages.isEmpty
          ? update
          : ChatItem(
              id: update.id,
              kind: update.kind,
              text: update.text,
              at: update.at,
              done: update.done,
              hasDetail: update.hasDetail,
              images: update.images,
              localImages: old.localImages,
            );
    }
  }
  return merged;
}

final chatProvider = NotifierProvider.autoDispose
    .family<ChatNotifier, AsyncValue<ChatState>, (String, String)>(
      ChatNotifier.new,
    );
