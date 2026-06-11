import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import {
  readLocalSourceFile,
  scanLocalSourceFiles,
  searchLocalSourceFiles,
  validateLocalProjectRoot,
  writeLocalSourceFile
} from '../src/lib/server/localSourceFs.ts';

const root = await mkdtemp(join(tmpdir(), 'mcb-local-source-'));

try {
  await mkdir(join(root, 'src', 'Services'), { recursive: true });
  await mkdir(join(root, '.history', 'Docs'), { recursive: true });
  await mkdir(join(root, '.pytest_cache'), { recursive: true });
  await mkdir(join(root, '.vscode'), { recursive: true });
  await mkdir(join(root, 'node_modules'), { recursive: true });
  await mkdir(join(root, 'bin'), { recursive: true });
  await mkdir(join(root, 'plain-folder'), { recursive: true });
  await writeFile(join(root, '.git'), 'gitdir: /tmp/mcb-local-source-fake-git\n', 'utf8');
  await writeFile(
    join(root, 'src', 'Services', 'FormatResolver.cs'),
    'namespace Demo;\npublic sealed class FormatResolver {}',
    'utf8'
  );
  await writeFile(join(root, 'src', 'App.ts'), 'export const appName = "MacCommandBar";\n', 'utf8');
  await writeFile(join(root, '.history', 'Docs', 'Old.md'), '# old\n', 'utf8');
  await writeFile(join(root, '.pytest_cache', 'README.md'), '# cache\n', 'utf8');
  await writeFile(join(root, '.vscode', 'settings.json'), '{}\n', 'utf8');
  await writeFile(join(root, 'node_modules', 'Ignored.ts'), 'export const ignored = true;\n', 'utf8');
  await writeFile(join(root, 'bin', 'Ignored.cs'), 'public sealed class Ignored {}\n', 'utf8');

  assert.deepEqual(await validateLocalProjectRoot(root), {
    path: root,
    exists: true,
    isDirectory: true,
    isGitRepository: true,
    gitRoot: root,
    message: 'Project root ready'
  });
  assert.deepEqual(await validateLocalProjectRoot(join(root, 'plain-folder')), {
    path: join(root, 'plain-folder'),
    exists: true,
    isDirectory: true,
    isGitRepository: false,
    gitRoot: root,
    message:
      `Folder is inside a Git repository. Add ${root} for full project context.`
  });
  assert.deepEqual(await validateLocalProjectRoot(join(root, 'missing')), {
    path: join(root, 'missing'),
    exists: false,
    isDirectory: false,
    isGitRepository: false,
    gitRoot: null,
    message: 'Project path not found'
  });

  const scan = await scanLocalSourceFiles({ root, limit: 20 });
  assert.deepEqual(
    scan.records.map((record) => record.relativePath),
    ['src/Services/FormatResolver.cs', 'src/App.ts']
  );
  assert.equal(scan.truncated, false);

  await mkdir(join(root, 'Docs'), { recursive: true });
  await mkdir(join(root, 'EdiPlatform.Core', 'Services'), { recursive: true });
  await mkdir(join(root, 'EdiPlatform.Core', 'Models'), { recursive: true });
  await writeFile(join(root, 'Docs', 'A.md'), '# docs\n', 'utf8');
  await writeFile(join(root, 'Docs', 'B.md'), '# docs\n', 'utf8');
  await writeFile(
    join(root, 'EdiPlatform.Core', 'Services', 'RuntimeService.cs'),
    'namespace Demo;\npublic sealed class RuntimeService {}',
    'utf8'
  );
  await writeFile(
    join(root, 'EdiPlatform.Core', 'Models', 'RuntimeModel.cs'),
    'namespace Demo;\npublic sealed class RuntimeModel {}',
    'utf8'
  );

  const prioritizedScan = await scanLocalSourceFiles({ root, limit: 3 });
  assert.deepEqual(
    prioritizedScan.records.map((record) => record.relativePath),
    [
      'EdiPlatform.Core/Models/RuntimeModel.cs',
      'EdiPlatform.Core/Services/RuntimeService.cs',
      'src/Services/FormatResolver.cs'
    ]
  );
  assert.equal(prioritizedScan.truncated, true);
  assert.ok(
    prioritizedScan.records.every((record) => !record.relativePath.startsWith('Docs/')),
    'Truncated scans should prioritize app source over docs'
  );

  const filteredScan = await scanLocalSourceFiles({ root, query: 'resolver', limit: 20 });
  assert.deepEqual(
    filteredScan.records.map((record) => record.fileName),
    ['FormatResolver.cs']
  );

  const preview = await readLocalSourceFile(join(root, 'src', 'Services', 'FormatResolver.cs'));
  assert.equal(preview.language, 'csharp');
  assert.equal(preview.lineCount, 2);

  const updatedPreview = await writeLocalSourceFile(
    join(root, 'src', 'App.ts'),
    'export const appName = "Updated";\n'
  );
  assert.match(updatedPreview.content, /Updated/);

  const matches = await searchLocalSourceFiles(scan.records, 'FormatResolver', 10);
  assert.equal(matches.length, 1);
  assert.equal(matches[0].relativePath, 'src/Services/FormatResolver.cs');
} finally {
  await rm(root, { recursive: true, force: true });
}
