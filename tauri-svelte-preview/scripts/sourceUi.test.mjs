import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const nextPageSource = readFileSync(
  new URL('../src/routes/next/+page.svelte', import.meta.url),
  'utf8'
);
const sourcePageSource = readFileSync(
  new URL('../src/routes/+page.svelte', import.meta.url),
  'utf8'
);
const activityFilesPanelSource = readFileSync(
  new URL('../src/lib/components/panels/ActivityFilesPanel.svelte', import.meta.url),
  'utf8'
);
const centerDockSource = readFileSync(
  new URL('../src/lib/shell/layout/centerDock.ts', import.meta.url),
  'utf8'
);
const shellOverlaysSource = readFileSync(
  new URL('../src/lib/shell/components/ShellOverlays.svelte', import.meta.url),
  'utf8'
);
const browserOverlaySource = readFileSync(
  new URL('../src/lib/shell/browser/SessionBrowserOverlay.svelte', import.meta.url),
  'utf8'
);
const dockLayoutSource = readFileSync(
  new URL('../src/lib/sourceDockLayout.ts', import.meta.url),
  'utf8'
);

assert.match(
  activityFilesPanelSource,
  /\.source-browser-stack\s*\{[\s\S]*?flex:\s*1\s+1\s+auto;[\s\S]*?height:\s*100%;[\s\S]*?overflow:\s*hidden;/,
  'The source browser stack styles should live with the Activity Files panel that renders it'
);
assert.doesNotMatch(
  sourcePageSource,
  /\.source-browser-stack\s*\{/,
  'The route should not duplicate the source browser stack style owned by Activity Files'
);

assert.match(
  nextPageSource,
  /function handleCenterPanelShown\(id: string\): void \{[\s\S]*?if \(id === ['"]browser['"]\) \{[\s\S]*?openSessionBrowserOverlay\(rail\.activeOwnedId\)[\s\S]*?showCenterPanel\(['"]session['"]\)/,
  'The center Browser entry should open the active session overlay and return to Session'
);
assert.match(
  nextPageSource,
  /<div class="browser-center-proxy"[^>]*data-testid="browser-center-proxy"/,
  'The center Browser tab should render a proxy host rather than a second browser surface'
);
assert.doesNotMatch(
  nextPageSource,
  /BrowserPanel|browserInputUrl|openRuntimeContextInBrowserDock|aria-label="Browser dock"/,
  'The consolidated shell should not retain the retired embedded browser dock'
);
assert.match(
  centerDockSource,
  /renderer: panel\.renderer/,
  'Center surface activation should preserve the panel-specific renderer contract'
);
assert.match(
  shellOverlaysSource,
  /<SessionBrowserOverlay\s+onClose=\{onSessionBrowserClose\}\s*\/>/,
  'The global overlay layer should mount one session-owned browser'
);
assert.match(
  browserOverlaySource,
  /class="session-browser-overlay"[\s\S]*?position:\s*fixed[\s\S]*?inset:\s*0/,
  'The session browser should cover the complete window'
);
assert.match(
  browserOverlaySource,
  /data-testid="session-browser-frame"[\s\S]*?src=\{view\.url\}/,
  'The session browser should render the current session URL'
);

assert.match(
  dockLayoutSource,
  /const centerRuntimePanelIDs: SourceDockPanelID\[\] = \['terminal', 'browser', 'markdown'\]/,
  'Terminal and Browser should be classified as center runtime panels'
);
assert.match(
  dockLayoutSource,
  /bottomGroup\.panelIDs = bottomGroup\.panelIDs\.filter\(\(panelID\) => !isCenterRuntimePanelID\(panelID\)\)/,
  'Runtime panels should be normalized out of the retired bottom dock'
);

console.log('sourceUi: Activity-owned source browser styles and the single session browser overlay are pinned');
