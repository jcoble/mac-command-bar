import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/machine_client.dart';
import '../machines/machine_store.dart';
import '../router.dart';

/// The channel the backend's pushes name (`phone_push.rs`: `channel_id`). It is
/// also the manifest's default, so tray notifications and the foreground ones
/// below land on the same channel.
const _channelId = 'assembly_inbox';

/// Firebase Cloud Messaging for new inbox items, adapted from Rental Command's
/// `push_service.dart`.
///
/// FAIL-SOFT: without `google-services.json`, or on a platform Firebase does
/// not support (the Linux desktop render), [init] logs once, push stays off and
/// the app runs normally.
class PushService {
  PushService(this._ref);

  final Ref _ref;
  final _local = FlutterLocalNotificationsPlugin();

  bool _ready = false;
  String? _token;

  /// Device tokens of the machines that already hold [_token].
  final _registered = <String>{};
  final _subscriptions = <StreamSubscription<Object?>>[];

  /// Whether Firebase started, so pushes can arrive.
  bool get isAvailable => _ready;

  /// Call once at app start. Never throws.
  Future<void> init() async {
    try {
      await Firebase.initializeApp();
      _ready = true;
    } catch (e) {
      debugPrint('[Push] Firebase unavailable; push is off. ($e)');
      return;
    }
    try {
      await _initChannel();
      final messaging = FirebaseMessaging.instance;
      _subscriptions
        ..add(FirebaseMessaging.onMessage.listen(_showForeground))
        ..add(
          FirebaseMessaging.onMessageOpenedApp.listen(
            (message) => unawaited(openFromData(message.data)),
          ),
        )
        ..add(
          messaging.onTokenRefresh.listen((token) {
            _registered.clear();
            unawaited(registerToken(token));
          }),
        );
      _ref.listen(machinesProvider, _onMachinesChanged);
      // Android shows its dialog only while the owner has not decided, so
      // asking on every start is safe and covers a phone paired before push.
      if (_ref.read(machinesProvider).value?.visible.isNotEmpty ?? false) {
        unawaited(messaging.requestPermission());
      }

      final initial = await messaging.getInitialMessage();
      if (initial != null) unawaited(openFromData(initial.data));
      final token = await messaging.getToken();
      if (token != null && token.isNotEmpty) await registerToken(token);
    } catch (e) {
      debugPrint('[Push] setup failed; push may not work. ($e)');
    }
  }

  /// `POST /phone/v1/push-token {fcmToken}` to every paired machine that does
  /// not hold [token] yet. A machine that cannot be reached is skipped and
  /// tried again on the next change or app start.
  Future<void> registerToken(String token) async {
    _token = token;
    final machines = [
      for (final m
          in _ref.read(machinesProvider).value?.visible ?? const <Machine>[])
        if (!_registered.contains(m.deviceToken)) m,
    ];
    await Future.wait([
      for (final m in machines)
        () async {
          try {
            await _ref
                .read(machineClientProvider(m.id))
                .post<void>('push-token', data: {'fcmToken': token});
            _registered.add(m.deviceToken);
          } on DioException catch (e) {
            debugPrint(
              '[Push] could not register with ${m.name}: ${e.message}',
            );
          }
        }(),
    ]);
  }

  /// A new pairing registers the token with the new machine and, the first time
  /// the phone is paired at all, asks for the Android 13+ notification
  /// permission (not at launch, when the owner has no context for it).
  void _onMachinesChanged(
    AsyncValue<SavedMachines>? before,
    AsyncValue<SavedMachines> after,
  ) {
    final token = _token;
    if (token != null) unawaited(registerToken(token));
    final wasEmpty = before?.value?.visible.isEmpty ?? true;
    if (wasEmpty && (after.value?.visible.isNotEmpty ?? false)) {
      unawaited(FirebaseMessaging.instance.requestPermission());
    }
  }

  /// A tap on a notification whose data is `{machineId, itemId}`: selects that
  /// machine and returns the route to open, `/inbox/<itemId>`. Null when the
  /// data is incomplete or the machine is not paired here.
  Future<String?> routeForTap(Map<String, dynamic> data) async {
    final machineId = data['machineId'];
    final itemId = data['itemId'];
    if (machineId is! String || itemId is! String || itemId.isEmpty) {
      return null;
    }
    final paired =
        _ref.read(machinesProvider).value?.visible ?? const <Machine>[];
    if (!paired.any((m) => m.id == machineId)) return null;
    await _ref.read(machinesProvider.notifier).select(machineId);
    return '/inbox/${Uri.encodeComponent(itemId)}';
  }

  Future<void> openFromData(Map<String, dynamic> data) async {
    final route = await routeForTap(data);
    if (route != null) _ref.read(routerProvider).go(route);
  }

  // Android shows tray notifications for background pushes by itself, on the
  // channel the message names. The channel must exist for its name and
  // importance to apply, and a push that arrives while the app is open is only
  // shown if the app posts it.
  Future<void> _initChannel() async {
    await _local.initialize(
      const InitializationSettings(
        android: AndroidInitializationSettings('@mipmap/ic_launcher'),
      ),
      onDidReceiveNotificationResponse: (response) {
        final payload = response.payload;
        if (payload == null || payload.isEmpty) return;
        final data = jsonDecode(payload);
        if (data is Map) {
          unawaited(openFromData(Map<String, dynamic>.from(data)));
        }
      },
    );
    await _local
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(
          const AndroidNotificationChannel(
            _channelId,
            'Inbox',
            description: 'Approvals and questions waiting for you.',
            importance: Importance.high,
          ),
        );
  }

  Future<void> _showForeground(RemoteMessage message) async {
    final notification = message.notification;
    if (notification == null) return;
    await _local.show(
      message.hashCode,
      notification.title,
      notification.body,
      const NotificationDetails(
        android: AndroidNotificationDetails(
          _channelId,
          'Inbox',
          importance: Importance.high,
          priority: Priority.high,
        ),
      ),
      payload: jsonEncode(message.data),
    );
  }

  void dispose() {
    for (final s in _subscriptions) {
      unawaited(s.cancel());
    }
  }
}

final pushServiceProvider = Provider<PushService>((ref) {
  final service = PushService(ref);
  ref.onDispose(service.dispose);
  return service;
});
