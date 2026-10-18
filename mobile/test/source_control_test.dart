import 'package:assembly_phone/features/source_control/source_control_repository.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'chat_test.dart' show FakeAdapter;

SourceControlRepository gitWith(FakeAdapter adapter) => SourceControlRepository(
  Dio(BaseOptions(baseUrl: 'https://m.example/phone/v1/'))
    ..httpClientAdapter = adapter,
  'lane 1',
);

void main() {
  test('git_status_parses_files_and_counts', () async {
    final adapter = FakeAdapter(
      (_) => (
        200,
        {
          'branch': 'tsk-7-fix',
          'hasUpstream': true,
          'ahead': 2,
          'behind': 1,
          'files': [
            {'path': 'src/main.rs', 'status': 'M', 'staged': true},
            {'path': 'notes.txt', 'status': '?', 'staged': false},
          ],
        },
      ),
    );
    final status = await gitWith(adapter).status();
    expect(adapter.requests.single.path, 'sessions/lane%201/git');
    expect(
      (status.branch, status.hasUpstream, status.ahead, status.behind),
      ('tsk-7-fix', true, 2, 1),
    );
    expect(
      [for (final f in status.files) (f.path, f.status, f.staged)],
      [('src/main.rs', 'M', true), ('notes.txt', '?', false)],
    );
  });

  test('prs_parse', () async {
    final adapter = FakeAdapter(
      (_) => (
        200,
        [
          {
            'number': 7,
            'title': 'Fix it',
            'state': 'OPEN',
            'url': 'https://github.com/o/r/pull/7',
            'branch': 'tsk-7-fix',
          },
        ],
      ),
    );
    final prs = await gitWith(adapter).prs();
    expect(adapter.requests.single.path, 'sessions/lane%201/git/prs');
    expect(
      [for (final pr in prs) (pr.number, pr.title, pr.url, pr.branch)],
      [(7, 'Fix it', 'https://github.com/o/r/pull/7', 'tsk-7-fix')],
    );
  });
}
