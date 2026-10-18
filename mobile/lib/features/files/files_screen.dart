import 'package:flutter/material.dart';
import 'package:material_symbols_icons/symbols.dart';

import '../../core/api/api_exception.dart';
import '../../core/widgets/mobile_m3_list.dart';
import '../chat/chat_widgets.dart';
import 'file_screen.dart';
import 'files_repository.dart';

/// A session's folder from the chat's Files button: a breadcrumb, then the
/// folder's entries. Back goes up a folder before it leaves the page.
class FilesScreen extends StatefulWidget {
  const FilesScreen({super.key, required this.files, this.title});

  final FilesRepository files;

  /// The session's folder name, the breadcrumb's first segment.
  final String? title;

  @override
  State<FilesScreen> createState() => _FilesScreenState();
}

class _FilesScreenState extends State<FilesScreen> {
  var _dir = '';
  late Future<FolderListing> _listing = widget.files.list('');

  void _open(String dir) => setState(() {
    _dir = dir;
    _listing = widget.files.list(dir);
  });

  Future<void> _reload() async {
    final next = widget.files.list(_dir);
    setState(() => _listing = next);
    try {
      await next;
    } on Object {
      // The FutureBuilder shows the error.
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final parts = _dir.isEmpty ? <String>[] : _dir.split('/');
    return PopScope(
      canPop: _dir.isEmpty,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _open(parts.sublist(0, parts.length - 1).join('/'));
      },
      child: Scaffold(
        appBar: AppBar(title: const Text('Files')),
        body: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _Breadcrumb(
              root: widget.title ?? 'Files',
              parts: parts,
              onOpen: (depth) => _open(parts.take(depth).join('/')),
            ),
            Expanded(
              child: RefreshIndicator(
                onRefresh: _reload,
                child: FutureBuilder<FolderListing>(
                  future: _listing,
                  builder: (context, snapshot) {
                    final listing = snapshot.data;
                    final error = snapshot.error;
                    Widget notice(
                      IconData icon,
                      String title,
                      String message,
                    ) => CustomScrollView(
                      physics: const AlwaysScrollableScrollPhysics(),
                      slivers: [
                        SliverFillRemaining(
                          hasScrollBody: false,
                          child: ChatNotice(
                            icon: icon,
                            title: title,
                            message: message,
                          ),
                        ),
                      ],
                    );
                    if (snapshot.connectionState != ConnectionState.done) {
                      return const Center(child: CircularProgressIndicator());
                    }
                    if (error != null) {
                      return notice(
                        Symbols.folder_off_rounded,
                        "Can't open this folder",
                        error is ApiException
                            ? error.message
                            : 'Something went wrong reading the folder.',
                      );
                    }
                    final entries = listing!.entries;
                    if (entries.isEmpty) {
                      return notice(
                        Symbols.folder_open_rounded,
                        'This folder is empty',
                        'Nothing to show here.',
                      );
                    }
                    return ListView.builder(
                      physics: const AlwaysScrollableScrollPhysics(),
                      padding: const EdgeInsets.fromLTRB(16, 4, 16, 24),
                      itemCount: entries.length + (listing.truncated ? 1 : 0),
                      itemBuilder: (context, i) {
                        if (i == entries.length) {
                          return Padding(
                            padding: const EdgeInsets.only(top: 16),
                            child: Text(
                              'Showing the first 2,000',
                              textAlign: TextAlign.center,
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ),
                          );
                        }
                        final entry = entries[i];
                        final path = _dir.isEmpty
                            ? entry.name
                            : '$_dir/${entry.name}';
                        return Column(
                          children: [
                            if (i > 0) const MobileM3ListDivider(indent: 16),
                            MobileM3ListItem(
                              position:
                                  MobileM3ListItemPositionForIndex.forIndex(
                                    i,
                                    entries.length,
                                  ),
                              leading: Icon(
                                entry.isDir
                                    ? Symbols.folder_rounded
                                    : _fileIcon(entry.name),
                                fill: entry.isDir ? 1 : 0,
                                color: entry.isDir
                                    ? theme.colorScheme.primary
                                    : theme.colorScheme.onSurfaceVariant,
                              ),
                              title: Text(
                                entry.name,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: theme.textTheme.bodyLarge?.copyWith(
                                  fontSize: 17,
                                ),
                              ),
                              supporting: [
                                if (entry.size case final size?)
                                  Text(
                                    _sizeLabel(size),
                                    style: theme.textTheme.bodySmall?.copyWith(
                                      color: theme.colorScheme.onSurfaceVariant,
                                    ),
                                  ),
                              ],
                              trailing: entry.isDir
                                  ? Icon(
                                      Symbols.chevron_right_rounded,
                                      color: theme.colorScheme.onSurfaceVariant,
                                    )
                                  : null,
                              onTap: () => entry.isDir
                                  ? _open(path)
                                  : Navigator.of(context).push(
                                      MaterialPageRoute<void>(
                                        builder: (_) => FileScreen(
                                          files: widget.files,
                                          path: path,
                                        ),
                                      ),
                                    ),
                            ),
                          ],
                        );
                      },
                    );
                  },
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// "project › src › agent": tap a segment to go up to it. Scrolled to its end
/// so the open folder is always in view.
class _Breadcrumb extends StatelessWidget {
  const _Breadcrumb({
    required this.root,
    required this.parts,
    required this.onOpen,
  });

  final String root;
  final List<String> parts;

  /// Opens the folder [depth] segments below the root.
  final ValueChanged<int> onOpen;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.colorScheme.onSurfaceVariant;
    final names = [root, ...parts];
    return LayoutBuilder(
      builder: (context, box) => SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        reverse: true,
        padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
        child: ConstrainedBox(
          // At least as wide as the screen, so a short path starts on the left.
          constraints: BoxConstraints(minWidth: box.maxWidth - 24),
          child: Row(
            children: [
              for (final (depth, name) in names.indexed) ...[
                if (depth > 0)
                  Icon(Symbols.chevron_right_rounded, size: 18, color: muted),
                depth == parts.length
                    ? Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 10,
                        ),
                        child: Text(
                          name,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      )
                    : TextButton(
                        onPressed: () => onOpen(depth),
                        style: TextButton.styleFrom(
                          foregroundColor: muted,
                          minimumSize: const Size(0, 40),
                          padding: const EdgeInsets.symmetric(horizontal: 8),
                        ),
                        child: Text(name),
                      ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

IconData _fileIcon(String name) => switch (name
    .substring(name.lastIndexOf('.') + 1)
    .toLowerCase()) {
  'md' || 'txt' => Symbols.description_rounded,
  'json' || 'yaml' || 'yml' || 'toml' => Symbols.data_object_rounded,
  'png' || 'jpg' || 'jpeg' || 'gif' || 'svg' || 'webp' => Symbols.image_rounded,
  'rs' ||
  'dart' ||
  'ts' ||
  'js' ||
  'svelte' ||
  'py' ||
  'cs' ||
  'go' => Symbols.code_rounded,
  _ => Symbols.draft_rounded,
};

String _sizeLabel(int bytes) => bytes < 1024
    ? '$bytes B'
    : bytes < 1024 * 1024
    ? '${(bytes / 1024).toStringAsFixed(1)} KB'
    : '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
