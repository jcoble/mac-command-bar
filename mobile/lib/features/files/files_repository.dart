import 'package:dio/dio.dart';

import '../../core/api/api_exception.dart';

/// `GET sessions/{id}/files?dir=`: one folder of the session, folders first
/// then files by name, as the machine sorts them.
class FolderListing {
  FolderListing.fromJson(Map<String, dynamic> json)
    : dir = json['dir'] as String,
      truncated = json['truncated'] == true,
      entries = [
        for (final e in json['entries'] as List)
          (
            name: e['name'] as String,
            isDir: e['kind'] == 'dir',
            size: e['size'] as int?,
          ),
      ];

  /// Relative to the session's folder; empty for the folder itself.
  final String dir;

  /// The machine sent only the first 2,000 entries.
  final bool truncated;
  final List<({String name, bool isDir, int? size})> entries;
}

/// `GET sessions/{id}/file?path=`: a text file, or a flag saying why there is
/// no text (over 1 MiB, or not UTF-8).
class FileContent {
  FileContent.fromJson(Map<String, dynamic> json)
    : path = json['path'] as String,
      size = json['size'] as int,
      text = json['text'] as String?,
      version = json['version'] as String?,
      tooLarge = json['tooLarge'] == true,
      binary = json['binary'] == true;

  final String path;
  final int size;
  final String? text;

  /// The hash of the bytes read, sent back with a save.
  final String? version;
  final bool tooLarge;
  final bool binary;
}

/// The file changed on the machine since the phone read it (409).
class FileChangedException implements Exception {}

/// One session's files on one machine: browse, read, and save an existing file.
class FilesRepository {
  FilesRepository(this._dio, this._ownedId);

  final Dio _dio;
  final String _ownedId;

  String get _session => 'sessions/${Uri.encodeComponent(_ownedId)}';

  Future<FolderListing> list(String dir) async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        '$_session/files',
        queryParameters: {'dir': dir},
      );
      return FolderListing.fromJson(response.data!);
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  Future<FileContent> read(String path) async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        '$_session/file',
        queryParameters: {'path': path},
      );
      return FileContent.fromJson(response.data!);
    } on DioException catch (e) {
      throw ApiException.fromDioException(e);
    }
  }

  /// Saves [text] over [path] only while the file is still at [version].
  Future<void> save(String path, String text, String version) async {
    try {
      await _dio.put<void>(
        '$_session/file',
        data: {'path': path, 'text': text, 'version': version},
      );
    } on DioException catch (e) {
      if (e.response?.statusCode == 409) throw FileChangedException();
      throw ApiException.fromDioException(e);
    }
  }
}
