import 'package:assembly_phone/features/files/files_repository.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'chat_test.dart' show FakeAdapter;

FilesRepository filesWith(FakeAdapter adapter) => FilesRepository(
  Dio(BaseOptions(baseUrl: 'https://m.example/phone/v1/'))
    ..httpClientAdapter = adapter,
  'lane 1',
);

void main() {
  test('files_parses_entries_dirs_first', () async {
    final adapter = FakeAdapter(
      (_) => (
        200,
        {
          'dir': 'src',
          'truncated': true,
          'entries': [
            {'name': 'agent', 'kind': 'dir'},
            {'name': 'main.rs', 'kind': 'file', 'size': 2048},
          ],
        },
      ),
    );
    final listing = await filesWith(adapter).list('src');
    expect(adapter.requests.single.path, 'sessions/lane%201/files');
    expect(adapter.requests.single.queryParameters, {'dir': 'src'});
    expect(
      [for (final e in listing.entries) (e.name, e.isDir, e.size)],
      [('agent', true, null), ('main.rs', false, 2048)],
    );
    expect((listing.dir, listing.truncated), ('src', true));
  });

  test('file_save_conflict_maps_to_changed', () async {
    final adapter = FakeAdapter(
      (r) => r.method == 'PUT' ? (409, {'error': 'changed'}) : (204, null),
    );
    final files = filesWith(adapter);
    await expectLater(
      files.save('docs/a.md', 'new text', 'v1'),
      throwsA(isA<FileChangedException>()),
    );
    final put = adapter.requests.single;
    expect((put.method, put.path), ('PUT', 'sessions/lane%201/file'));
    expect(put.data, {
      'path': 'docs/a.md',
      'text': 'new text',
      'version': 'v1',
    });
  });
}
