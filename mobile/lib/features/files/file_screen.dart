import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:re_highlight/languages/bash.dart';
import 'package:re_highlight/languages/css.dart';
import 'package:re_highlight/languages/csharp.dart';
import 'package:re_highlight/languages/dart.dart';
import 'package:re_highlight/languages/go.dart';
import 'package:re_highlight/languages/ini.dart';
import 'package:re_highlight/languages/javascript.dart';
import 'package:re_highlight/languages/json.dart';
import 'package:re_highlight/languages/kotlin.dart';
import 'package:re_highlight/languages/markdown.dart';
import 'package:re_highlight/languages/python.dart';
import 'package:re_highlight/languages/rust.dart';
import 'package:re_highlight/languages/sql.dart';
import 'package:re_highlight/languages/swift.dart';
import 'package:re_highlight/languages/typescript.dart';
import 'package:re_highlight/languages/xml.dart';
import 'package:re_highlight/languages/yaml.dart';
import 'package:re_highlight/re_highlight.dart';
import 'package:re_highlight/styles/atom-one-dark.dart';

import '../../core/api/api_exception.dart';
import '../../core/theme/app_tokens.dart';
import '../chat/chat_widgets.dart';
import 'files_repository.dart';

/// One file of a session: its text with line numbers and colours, or a note
/// when it is too large or not text. The pencil switches the page to a plain
/// editor whose save only lands while the file is unchanged on the machine.
class FileScreen extends StatefulWidget {
  const FileScreen({super.key, required this.files, required this.path});

  final FilesRepository files;

  /// Relative to the session's folder.
  final String path;

  @override
  State<FileScreen> createState() => _FileScreenState();
}

class _FileScreenState extends State<FileScreen> {
  FileContent? _content;
  ({List<TextSpan> lines, int longest})? _code;
  Object? _error;

  /// Set while editing.
  TextEditingController? _editor;
  var _saving = false;
  var _conflict = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _editor?.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() => _error = null);
    try {
      final content = await widget.files.read(widget.path);
      if (!mounted) return;
      setState(() {
        _content = content;
        _code = switch (content.text) {
          final text? => highlightLines(text, widget.path),
          null => null,
        };
      });
    } on Object catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  void _edit() => setState(() {
    _editor = TextEditingController(text: _content!.text);
    _conflict = false;
  });

  void _stopEditing() => setState(() {
    _editor?.dispose();
    _editor = null;
    _conflict = false;
  });

  Future<void> _reload() async {
    _stopEditing();
    await _load();
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    try {
      await widget.files.save(widget.path, _editor!.text, _content!.version!);
      if (!mounted) return;
      await _reload();
    } on FileChangedException {
      if (mounted) setState(() => _conflict = true);
    } on ApiException catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(e.message)));
      }
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  /// True when the editor has no changes or the owner agrees to drop them.
  Future<bool> _mayDiscard() async {
    if (_editor!.text != _content!.text) {
      final discard = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Discard your changes?'),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('Keep editing'),
            ),
            TextButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('Discard'),
            ),
          ],
        ),
      );
      if (discard != true || !mounted) return false;
    }
    return true;
  }

  /// Back while editing leaves the editor, after asking when there are changes.
  Future<void> _leaveEditor() async {
    if (await _mayDiscard()) _stopEditing();
  }

  /// Reload after a conflict drops the edits, so it asks first too.
  Future<void> _reloadAfterConflict() async {
    if (await _mayDiscard()) await _reload();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final slash = widget.path.lastIndexOf('/');
    final editor = _editor;
    return PopScope(
      canPop: editor == null,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _leaveEditor();
      },
      child: Scaffold(
        appBar: AppBar(
          title: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                widget.path.substring(slash + 1),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              if (slash > 0)
                Text(
                  widget.path.substring(0, slash),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
            ],
          ),
          actions: [
            if (editor != null)
              Padding(
                padding: const EdgeInsets.only(right: 8),
                child: FilledButton(
                  onPressed: _saving || _conflict ? null : _save,
                  child: _saving
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('Save'),
                ),
              )
            else if (_content?.text != null)
              IconButton(
                tooltip: 'Edit',
                icon: const Icon(Symbols.edit_rounded),
                onPressed: _edit,
              ),
          ],
        ),
        body: switch ((_content, _error)) {
          _ when editor != null => Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (_conflict) _ConflictBanner(onReload: _reloadAfterConflict),
              // Read-only while saving: the save sends the text as it was, then
              // reloads, so anything typed meanwhile would be lost.
              Expanded(child: _Editor(controller: editor, readOnly: _saving)),
            ],
          ),
          (_, final error?) => ChatNotice(
            icon: Symbols.draft_rounded,
            title: "Can't open this file",
            message: error is ApiException
                ? error.message
                : 'Something went wrong reading the file.',
            action: FilledButton.tonalIcon(
              onPressed: _load,
              icon: const Icon(Symbols.refresh_rounded),
              label: const Text('Try again'),
            ),
          ),
          (null, _) => const Center(child: CircularProgressIndicator()),
          (final content?, _) when content.tooLarge => ChatNotice(
            icon: Symbols.draft_rounded,
            title: 'Too large to open here',
            message:
                'This file is ${_megabytes(content.size)}. The phone opens files up to 1 MB.',
          ),
          (final content?, _) when content.binary => const ChatNotice(
            icon: Symbols.draft_rounded,
            title: 'Not a text file',
            message: 'The phone shows text files only.',
          ),
          _ => _CodeView(code: _code!),
        },
      ),
    );
  }
}

String _megabytes(int bytes) =>
    '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';

/// The text style of code on this page.
TextStyle _codeStyle(ThemeData theme) => theme.textTheme.bodyMedium!.copyWith(
  fontFamily: 'monospace',
  fontFamilyFallback: const ['monospace'],
  fontSize: 14,
  height: 1.5,
  color: theme.colorScheme.onSurface,
);

/// How wide [chars] characters of [style] are. Code is monospace, so a line's
/// width follows from its length; a little slack covers wider glyphs.
double _textWidth(TextStyle style, int chars) {
  final painter = TextPainter(
    text: TextSpan(text: 'M' * 10, style: style),
    textDirection: TextDirection.ltr,
  )..layout();
  final width = painter.width / 10 * chars;
  painter.dispose();
  return width;
}

/// The code with its line numbers pinned on the left. Lines never wrap: the
/// code scrolls sideways as one block, and the numbers follow it up and down.
/// Only the lines on screen are built.
class _CodeView extends StatefulWidget {
  const _CodeView({required this.code});

  final ({List<TextSpan> lines, int longest}) code;

  @override
  State<_CodeView> createState() => _CodeViewState();
}

class _CodeViewState extends State<_CodeView> {
  final _lines = ScrollController();
  final _numbers = ScrollController();

  @override
  void initState() {
    super.initState();
    _lines.addListener(() {
      if (_numbers.hasClients) _numbers.jumpTo(_lines.offset);
    });
  }

  @override
  void dispose() {
    _lines.dispose();
    _numbers.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = _codeStyle(theme);
    final lines = widget.code.lines;
    final lineHeight = style.fontSize! * style.height!;
    const padding = EdgeInsets.symmetric(vertical: 12);
    final numberWidth = _textWidth(style, '${lines.length}'.length) + 28;
    final codeWidth = _textWidth(style, widget.code.longest) + 48;
    return ColoredBox(
      color: theme.colorScheme.surfaceContainerLowest,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          DecoratedBox(
            decoration: BoxDecoration(
              border: Border(
                right: BorderSide(color: theme.colorScheme.outlineVariant),
              ),
            ),
            child: SizedBox(
              width: numberWidth,
              child: ListView.builder(
                controller: _numbers,
                physics: const NeverScrollableScrollPhysics(),
                padding: padding,
                itemExtent: lineHeight,
                itemCount: lines.length,
                itemBuilder: (context, i) => Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: Text(
                    '${i + 1}',
                    textAlign: TextAlign.right,
                    style: style.copyWith(
                      color: theme.colorScheme.onSurfaceVariant.withValues(
                        alpha: 0.6,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
          Expanded(
            child: LayoutBuilder(
              builder: (context, box) => SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: SizedBox(
                  width: math.max(codeWidth, box.maxWidth),
                  child: SelectionArea(
                    child: ListView.builder(
                      controller: _lines,
                      padding: padding,
                      itemExtent: lineHeight,
                      itemCount: lines.length,
                      itemBuilder: (context, i) => Padding(
                        padding: const EdgeInsets.only(left: 12),
                        child: Text.rich(
                          lines[i],
                          softWrap: false,
                          maxLines: 1,
                          style: style,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// A plain monospace text box that never wraps: it is as wide as its longest
/// line and scrolls sideways.
class _Editor extends StatefulWidget {
  const _Editor({required this.controller, required this.readOnly});

  final TextEditingController controller;
  final bool readOnly;

  @override
  State<_Editor> createState() => _EditorState();
}

class _EditorState extends State<_Editor> {
  late var _longest = _longestLine(widget.controller.text);

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_measure);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_measure);
    super.dispose();
  }

  void _measure() {
    final longest = _longestLine(widget.controller.text);
    if (longest != _longest) setState(() => _longest = longest);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = _codeStyle(theme);
    return ColoredBox(
      color: theme.colorScheme.surfaceContainerLowest,
      child: LayoutBuilder(
        builder: (context, box) => SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: SizedBox(
            width: math.max(_textWidth(style, _longest) + 64, box.maxWidth),
            child: TextField(
              controller: widget.controller,
              readOnly: widget.readOnly,
              maxLines: null,
              expands: true,
              autofocus: true,
              keyboardType: TextInputType.multiline,
              autocorrect: false,
              enableSuggestions: false,
              style: style,
              textAlignVertical: TextAlignVertical.top,
              decoration: const InputDecoration(
                filled: false,
                border: InputBorder.none,
                enabledBorder: InputBorder.none,
                focusedBorder: InputBorder.none,
                contentPadding: EdgeInsets.fromLTRB(16, 12, 16, 12),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// The longest line of [text] in characters, a tab counted as eight.
int _longestLine(String text) {
  var longest = 0;
  var current = 0;
  for (final unit in text.codeUnits) {
    if (unit == 0x0A) {
      current = 0;
    } else {
      current += unit == 0x09 ? 8 : 1;
      if (current > longest) longest = current;
    }
  }
  return longest;
}

class _ConflictBanner extends StatelessWidget {
  const _ConflictBanner({required this.onReload});

  final VoidCallback onReload;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.fromLTRB(12, 4, 12, 8),
      padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: M3Shape.radiusMedium,
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Row(
        children: [
          Icon(Symbols.sync_problem_rounded, color: theme.colorScheme.error),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              'This file changed on the machine. Reload to see the new version.',
              style: theme.textTheme.bodyLarge,
            ),
          ),
          const SizedBox(width: 8),
          FilledButton.tonal(onPressed: onReload, child: const Text('Reload')),
        ],
      ),
    );
  }
}

final _highlight = Highlight()
  ..registerLanguages({
    'bash': langBash,
    'csharp': langCsharp,
    'css': langCss,
    'dart': langDart,
    'go': langGo,
    'ini': langIni,
    'javascript': langJavascript,
    'json': langJson,
    'kotlin': langKotlin,
    'markdown': langMarkdown,
    'python': langPython,
    'rust': langRust,
    'sql': langSql,
    'swift': langSwift,
    'typescript': langTypescript,
    'xml': langXml,
    'yaml': langYaml,
  });

const _languageOf = {
  'sh': 'bash',
  'bash': 'bash',
  'zsh': 'bash',
  'cs': 'csharp',
  'css': 'css',
  'dart': 'dart',
  'go': 'go',
  'toml': 'ini',
  'ini': 'ini',
  'js': 'javascript',
  'mjs': 'javascript',
  'json': 'json',
  'kt': 'kotlin',
  'md': 'markdown',
  'py': 'python',
  'rs': 'rust',
  'sql': 'sql',
  'swift': 'swift',
  'ts': 'typescript',
  'tsx': 'typescript',
  'html': 'xml',
  'xml': 'xml',
  'svelte': 'xml',
  'yaml': 'yaml',
  'yml': 'yaml',
};

/// Past this size a file shows without colours, so opening it stays quick.
const _maxHighlightChars = 256 * 1024;

/// [text] as one span per line, coloured by the language its [path]'s
/// extension names (Atom One Dark colours), with its longest line's length.
/// Tabs show as four spaces.
({List<TextSpan> lines, int longest}) highlightLines(String text, String path) {
  final code = text.replaceAll('\t', '    ');
  final language =
      _languageOf[path.substring(path.lastIndexOf('.') + 1).toLowerCase()];
  final lines = _LineRenderer();
  if (language == null || code.length > _maxHighlightChars) {
    lines.addText(code);
  } else {
    _highlight.highlight(code: code, language: language).render(lines);
  }
  if (lines.lines.length > 1 && lines.lines.last.isEmpty) {
    lines.lines.removeLast();
  }
  return (
    lines: [for (final spans in lines.lines) TextSpan(children: spans)],
    longest: _longestLine(code),
  );
}

/// Collects the highlighter's output as flat spans, split into lines.
class _LineRenderer implements HighlightRenderer {
  final lines = <List<TextSpan>>[[]];
  final _scopes = <String?>[];

  @override
  void addText(String text) {
    final style = _style();
    for (final (i, part) in text.split('\n').indexed) {
      if (i > 0) lines.add([]);
      if (part.isNotEmpty) lines.last.add(TextSpan(text: part, style: style));
    }
  }

  /// The innermost scope's colour, trying `title` for `title.function`.
  TextStyle? _style() {
    for (final scope in _scopes.reversed) {
      if (scope == null || scope == 'root') continue;
      final style =
          atomOneDarkTheme[scope] ?? atomOneDarkTheme[scope.split('.').first];
      if (style != null) return style;
    }
    return null;
  }

  @override
  void openNode(DataNode node) => _scopes.add(node.scope);

  @override
  void closeNode(DataNode node) => _scopes.removeLast();
}
