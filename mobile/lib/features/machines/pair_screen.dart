import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../../core/api/api_exception.dart';
import '../../core/machines/machine_store.dart';

/// First launch, and "Pair another machine": scan the desktop's QR code, or
/// type the machine URL and code.
class PairScreen extends ConsumerStatefulWidget {
  const PairScreen({super.key});

  @override
  ConsumerState<PairScreen> createState() => _PairScreenState();
}

class _PairScreenState extends ConsumerState<PairScreen> {
  final _url = TextEditingController();
  final _code = TextEditingController();
  bool _busy = false;
  String? _error;

  /// The camera reports the same code many times a second; try each once.
  String? _lastScan;

  @override
  void dispose() {
    _url.dispose();
    _code.dispose();
    super.dispose();
  }

  Future<void> _pair(String text) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await ref
          .read(machinesProvider.notifier)
          .pair(PairingPayload.parse(text));
      if (mounted) context.go('/chat');
    } on FormatException catch (e) {
      _error = e.message;
    } on DioException catch (e) {
      _error = ApiException.fromDioException(e).message;
    }
    if (mounted) setState(() => _busy = false);
  }

  void _pairTyped() =>
      _pair(jsonEncode({'v': 1, 'url': _url.text.trim(), 'code': _code.text}));

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Pair a machine')),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
        children: [
          Text(
            'On the Mac: Assembly → Settings → Connections → the machine → '
            'Pair phone, then point the camera at the code.',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 16),
          ClipRRect(
            borderRadius: BorderRadius.circular(28),
            child: SizedBox(
              height: 260,
              child: MobileScanner(
                onDetect: (capture) {
                  final raw = capture.barcodes.firstOrNull?.rawValue;
                  if (raw == null || raw == _lastScan) return;
                  _lastScan = raw;
                  _pair(raw);
                },
              ),
            ),
          ),
          const SizedBox(height: 24),
          Text('OR ENTER THE CODE', style: theme.textTheme.labelMedium),
          const SizedBox(height: 8),
          TextField(
            controller: _url,
            keyboardType: TextInputType.url,
            decoration: const InputDecoration(
              labelText: 'Machine URL',
              hintText: 'https://agent-workbox.example.ts.net:8777',
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _code,
            textCapitalization: TextCapitalization.characters,
            decoration: const InputDecoration(labelText: 'Pairing code'),
          ),
          const SizedBox(height: 16),
          FilledButton(
            onPressed: _busy ? null : _pairTyped,
            child: Text(_busy ? 'Pairing…' : 'Pair'),
          ),
          if (_error != null) ...[
            const SizedBox(height: 12),
            Text(
              _error!,
              style: theme.textTheme.bodyMedium?.copyWith(color: scheme.error),
            ),
          ],
        ],
      ),
    );
  }
}
