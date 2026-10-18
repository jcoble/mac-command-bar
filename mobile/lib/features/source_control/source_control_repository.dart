import 'package:dio/dio.dart';

import '../../core/api/api_exception.dart';

/// `GET sessions/{id}/git`: the session folder's branch, how far it is from
/// its upstream, and its changed files.
class GitStatus {
  GitStatus.fromJson(Map<String, dynamic> json)
    : branch = json['branch'] as String?,
      hasUpstream = json['hasUpstream'] == true,
      ahead = json['ahead'] as int,
      behind = json['behind'] as int,
      files = [
        for (final f in json['files'] as List)
          (
            path: f['path'] as String,
            status: f['status'] as String,
            staged: f['staged'] == true,
            unstaged: f['unstaged'] == true,
          ),
      ];

  /// Null on a detached HEAD.
  final String? branch;
  final bool hasUpstream;
  final int ahead;
  final int behind;

  /// [status] is git's letter: M, A, D, R, C, U, or ? for a new file. A
  /// partly staged file is both [staged] and [unstaged].
  final List<({String path, String status, bool staged, bool unstaged})> files;
}

/// One open pull request, from `GET sessions/{id}/git/prs`.
typedef PullRequest = ({int number, String title, String url, String branch});

/// One session's source control on one machine: status, a file's diff, Stage
/// All, commit, fetch / pull / push, branches, and open pull requests.
class SourceControlRepository {
  SourceControlRepository(this._dio, this._ownedId);

  final Dio _dio;
  final String _ownedId;

  /// Fetch, pull, push and the pull request list wait on the network, which
  /// the machine allows 120 seconds.
  static final _network = Options(receiveTimeout: const Duration(seconds: 125));

  String get _git => 'sessions/${Uri.encodeComponent(_ownedId)}/git';

  Future<T> _call<T>(Future<T> Function() request) async {
    try {
      return await request();
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  Future<GitStatus> status() => _call(
    () async =>
        GitStatus.fromJson((await _dio.get<Map<String, dynamic>>(_git)).data!),
  );

  /// One listed file's diff, staged and unstaged together; [capped] when the
  /// machine cut it at 256 KB.
  Future<({String diff, bool capped})> diff(String path) => _call(() async {
    final body = (await _dio.get<Map<String, dynamic>>(
      '$_git/diff',
      queryParameters: {'path': path},
    )).data!;
    return (diff: body['diff'] as String, capped: body['capped'] == true);
  });

  Future<void> stageAll() => _call(() => _dio.post<void>('$_git/stage-all'));

  /// Commits what is staged; the new commit's short hash.
  Future<String> commit(String message) => _call(
    () async =>
        (await _dio.post<Map<String, dynamic>>(
              '$_git/commit',
              data: {'message': message},
            )).data!['commit']
            as String,
  );

  /// [action] is `fetch`, `pull` or `push`.
  Future<void> sync(String action) => _call(
    () => _dio.post<void>(
      '$_git/sync',
      data: {'action': action},
      options: _network,
    ),
  );

  /// The local branches, most recently committed first, and the current one.
  Future<({String current, List<String> names})> branches() => _call(() async {
    final body = (await _dio.get<Map<String, dynamic>>('$_git/branches')).data!;
    return (
      current: body['current'] as String,
      names: [for (final b in body['branches'] as List) b['name'] as String],
    );
  });

  Future<void> switchTo(String name) =>
      _call(() => _dio.post<void>('$_git/switch', data: {'name': name}));

  Future<List<PullRequest>> prs() => _call(() async {
    final body = (await _dio.get<List<dynamic>>(
      '$_git/prs',
      options: _network,
    )).data!;
    return [
      for (final pr in body)
        (
          number: pr['number'] as int,
          title: pr['title'] as String,
          url: pr['url'] as String,
          branch: pr['branch'] as String,
        ),
    ];
  });
}
