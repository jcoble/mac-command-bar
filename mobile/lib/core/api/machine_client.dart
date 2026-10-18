import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../machines/machine_store.dart';

/// Shared Dio settings (Rental Command's `dio_client.dart` timeouts). Pairing
/// uses it directly; each machine client is a clone of it.
final dioProvider = Provider<Dio>(
  (ref) => Dio(
    BaseOptions(
      // Fail fast when the phone is offline so the UI shows its error promptly.
      connectTimeout: const Duration(seconds: 8),
      receiveTimeout: const Duration(seconds: 20),
      sendTimeout: const Duration(seconds: 20),
      headers: {
        HttpHeaders.contentTypeHeader: 'application/json',
        HttpHeaders.acceptHeader: 'application/json',
      },
      responseType: ResponseType.json,
    ),
  ),
);

/// A Dio for one paired machine: its phone API and this phone's device token.
Dio machineDio(Dio base, Machine machine) => base.clone(
  options: base.options.copyWith(
    baseUrl: '${machine.url}/phone/v1/',
    headers: {
      ...base.options.headers,
      'Authorization': 'Bearer ${machine.deviceToken}',
    },
  ),
);

final machineClientProvider = Provider.family<Dio, String>((ref, machineId) {
  final machine = ref
      .watch(machinesProvider)
      .requireValue
      .visible
      .firstWhere((m) => m.id == machineId);
  return machineDio(ref.watch(dioProvider), machine);
});
