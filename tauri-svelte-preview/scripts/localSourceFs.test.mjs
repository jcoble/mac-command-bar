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
  assert.deepEqual(
    new Set(scan.stats.skippedDirectorySamples.map((sample) => sample.name)),
    new Set(['.history', '.pytest_cache', '.vscode', 'bin', 'node_modules'])
  );
  assert.deepEqual(
    scan.stats.skippedDirectorySamples.find((sample) => sample.name === 'node_modules'),
    {
      path: 'node_modules',
      name: 'node_modules',
      reason: 'dependency directory'
    }
  );
  assert.ok(
    scan.stats.skippedDirectorySamples.every(
      (sample) => sample.path && !sample.path.startsWith(root) && sample.reason
    ),
    'Skipped directory samples should include relative path, name, and reason'
  );

  for (let index = 0; index < 20; index += 1) {
    await mkdir(join(root, `sample-${String(index).padStart(2, '0')}`, 'node_modules'), {
      recursive: true
    });
  }

  const boundedSampleScan = await scanLocalSourceFiles({ root, limit: 20 });
  assert.equal(boundedSampleScan.stats.skippedDirectorySamples.length, 16);
  assert.ok(
    boundedSampleScan.stats.skippedDirectories > boundedSampleScan.stats.skippedDirectorySamples.length,
    'Skipped directory samples should be bounded while the count keeps increasing'
  );

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

  const projectNamedRoot = join(root, 'Project');
  await mkdir(join(projectNamedRoot, 'Project.Shared'), { recursive: true });
  await mkdir(join(projectNamedRoot, 'src'), { recursive: true });
  await writeFile(
    join(projectNamedRoot, 'Project.Shared', 'Alpha.cs'),
    'namespace Project.Shared;\npublic sealed class Alpha {}\n',
    'utf8'
  );
  await writeFile(
    join(projectNamedRoot, 'Project.Shared', 'Beta.cs'),
    'namespace Project.Shared;\npublic sealed class Beta {}\n',
    'utf8'
  );
  await writeFile(
    join(projectNamedRoot, 'src', 'App.cs'),
    'namespace Project;\npublic sealed class App {}\n',
    'utf8'
  );

  const projectNamedScan = await scanLocalSourceFiles({ root: projectNamedRoot, limit: 2 });
  assert.deepEqual(
    projectNamedScan.records.map((record) => record.relativePath),
    ['src/App.cs', 'Project.Shared/Alpha.cs']
  );
  assert.equal(projectNamedScan.truncated, true);

  const lowLimitRoot = join(root, 'low-limit-project');
  await mkdir(join(lowLimitRoot, 'src'), { recursive: true });
  await mkdir(join(lowLimitRoot, 'node_modules', 'pkg'), { recursive: true });
  await writeFile(join(lowLimitRoot, 'src', 'App.ts'), 'export const app = true;\n', 'utf8');
  await writeFile(join(lowLimitRoot, 'src', 'Worker.ts'), 'export const worker = true;\n', 'utf8');
  await writeFile(join(lowLimitRoot, 'src', 'Widget.ts'), 'export const widget = true;\n', 'utf8');
  await writeFile(
    join(lowLimitRoot, 'node_modules', 'pkg', 'index.ts'),
    'export const dependency = true;\n',
    'utf8'
  );

  const lowLimitScan = await scanLocalSourceFiles({ root: lowLimitRoot, limit: 2 });
  assert.equal(lowLimitScan.records.length, 2);
  assert.equal(lowLimitScan.truncated, true);
  assert.equal(lowLimitScan.stats.requestedLimit, 2);
  assert.equal(lowLimitScan.stats.returnedFiles, 2);
  assert.equal(lowLimitScan.stats.matchedFiles, 3);
  assert.equal(lowLimitScan.stats.collectionLimit, 10_001);
  assert.equal(lowLimitScan.stats.collectionLimitReached, false);
  assert.equal(lowLimitScan.stats.skippedDirectories, 1);
  assert.equal(lowLimitScan.stats.skippedDirectorySamples[0].name, 'node_modules');

  const filteredScan = await scanLocalSourceFiles({ root, query: 'resolver', limit: 20 });
  assert.deepEqual(
    filteredScan.records.map((record) => record.fileName),
    ['FormatResolver.cs']
  );

  await writeFile(join(root, 'src', 'Widget.tsx'), 'export const Widget = () => null;\n', 'utf8');
  await writeFile(join(root, 'src', 'Widget.jsx'), 'export const WidgetJsx = () => null;\n', 'utf8');
  await writeFile(join(root, 'src', 'View.swift'), 'struct View {}\n', 'utf8');

  const languageScan = await scanLocalSourceFiles({ root, query: 'widget', limit: 20 });
  const widgetLanguages = new Map(languageScan.records.map((record) => [record.fileName, record.language]));
  assert.equal(widgetLanguages.get('Widget.jsx'), 'jsx');
  assert.equal(widgetLanguages.get('Widget.tsx'), 'tsx');

  const swiftScan = await scanLocalSourceFiles({ root, query: 'View.swift', limit: 20 });
  assert.deepEqual(
    swiftScan.records.map((record) => [record.fileName, record.language]),
    [['View.swift', 'swift']]
  );

  await writeFile(join(root, 'src', 'Styles.scss'), '.app { color: red; }\n', 'utf8');
  await writeFile(join(root, 'src', 'Article.mdx'), '# Article\n', 'utf8');
  await writeFile(join(root, 'src', 'Page.astro'), '<h1>Page</h1>\n', 'utf8');
  await writeFile(join(root, 'src', 'App.vue'), '<template></template>\n', 'utf8');
  await writeFile(join(root, 'src', 'Project.fsproj'), '<Project />\n', 'utf8');
  await writeFile(join(root, 'src', 'View.xaml'), '<Page />\n', 'utf8');

  const parityLanguageScan = await scanLocalSourceFiles({ root, query: 'src/', limit: 100 });
  const languageByFileName = new Map(
    parityLanguageScan.records.map((record) => [record.fileName, record.language])
  );
  assert.equal(languageByFileName.get('Styles.scss'), 'scss');
  assert.equal(languageByFileName.get('Article.mdx'), 'mdx');
  assert.equal(languageByFileName.get('Page.astro'), 'html');
  assert.equal(languageByFileName.get('App.vue'), 'html');
  assert.equal(languageByFileName.get('Project.fsproj'), 'xml');
  assert.equal(languageByFileName.get('View.xaml'), 'xml');

  const worktreeRoot = join(root, 'worktrees', 'EdiPlatform', 'tsk-127-source-scan');
  await mkdir(join(worktreeRoot, 'EdiPlatform.Core', 'Services'), { recursive: true });
  await mkdir(join(worktreeRoot, 'EdiPlatform.Api', 'Controllers'), { recursive: true });
  await writeFile(join(worktreeRoot, '.git'), 'gitdir: /tmp/repo/.git/worktrees/tsk-127\n', 'utf8');
  await writeFile(
    join(worktreeRoot, 'EdiPlatform.Core', 'Services', 'FormatDetector.cs'),
    'namespace Demo;\npublic sealed class FormatDetector {}',
    'utf8'
  );
  await writeFile(
    join(worktreeRoot, 'EdiPlatform.Api', 'Controllers', 'DashboardController.cs'),
    'namespace Demo;\npublic sealed class DashboardController {}',
    'utf8'
  );

  const worktreeScan = await scanLocalSourceFiles({ root: worktreeRoot, limit: 20 });
  assert.deepEqual(
    worktreeScan.records.map((record) => record.relativePath),
    [
      'EdiPlatform.Core/Services/FormatDetector.cs',
      'EdiPlatform.Api/Controllers/DashboardController.cs'
    ]
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
