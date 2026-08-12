import fs from 'node:fs/promises';
import path from 'node:path';
import { chromium } from '/tmp/railmatch16-playwright/node_modules/playwright/index.mjs';

const root = process.cwd();
const output = path.join(root, 'output/railmatch16');
const mockupPath = path.join(root, 'docs/superpowers/mockups/2026-08-12-rail-redesign.html');
const profile = '/tmp/railmatch16-chrome-profile';
const viewport = { width: 1710, height: 990 };

const now = Date.now();
const iso = (minutesAgo) => new Date(now - minutesAgo * 60_000).toISOString();

function record({ id, title, project, branch, model, minutesAgo, state = 'background', runtimeState = 'idle', executionOwner = 'structured', completedAt = null, settledAt = null, viaCmux = false, agent = 'claude', ptySessionId = null }) {
  return {
    ownedId: id,
    agent,
    origin: id === 'rail-07' ? 'external' : 'app',
    viaCmux,
    source: 'fresh',
    title,
    model,
    projectPath: `/Users/blackcolours/dev/work/${project}`,
    cwd: `/Users/blackcolours/dev/work/${project}`,
    resumeCommand: null,
    nativeSessionId: `${id}-native`,
    ptySessionId,
    state,
    lastError: null,
    executionOwner,
    runtimeState,
    providerInstanceId: null,
    activeTurnId: runtimeState === 'working' ? `${id}-turn` : null,
    capabilityRevision: 0,
    lastRuntimeError: null,
    completedAt,
    settledAt,
    branch,
    taskId: null,
    pullRequest: null,
    messageCount: 4,
    latestTurnPreview: null,
    lastActivity: iso(minutesAgo)
  };
}

const owned = [
  record({ id: 'rail-01', title: 'Review the long session title that should ellipsize', project: 'mac-command-bar', branch: 'tsk-810-rail', model: 'gpt-5.6-luna', minutesAgo: 0, runtimeState: 'working', executionOwner: 'structured', agent: 'other' }),
  record({ id: 'rail-02', title: 'Build session rail surface 2', project: 'mac-command-bar', branch: 'feature/rail-surface-2', model: 'gpt-5.6-sol', minutesAgo: 4, runtimeState: 'idle', executionOwner: 'structured' }),
  record({ id: 'rail-03', title: 'Build session rail surface 3', project: 'mac-command-bar', branch: 'tsk-810-rail-3', model: 'gpt-5.6-sol', minutesAgo: 8, runtimeState: 'waiting-input', executionOwner: 'structured' }),
  record({ id: 'rail-04', title: 'Build session rail surface 4', project: 'mac-command-bar', branch: 'feature/rail-surface-4', model: 'gpt-5.6-sol', minutesAgo: 12, runtimeState: 'idle', executionOwner: 'structured' }),
  record({ id: 'rail-05', title: 'Build session rail surface 5', project: 'mac-command-bar', branch: 'tsk-810-rail-5', model: 'gpt-5.6-sol', minutesAgo: 16, runtimeState: 'idle', executionOwner: 'structured' }),
  record({ id: 'rail-06', title: 'Investigate failed workspace handoff', project: 'mac-command-bar', branch: 'tsk-810-handoff', model: 'gpt-5.6-sol', minutesAgo: 16, runtimeState: 'failed', executionOwner: 'stopped', state: 'exited' }),
  record({ id: 'rail-07', title: 'Resume the stopped terminal session', project: 'mac-command-bar', branch: 'tsk-810-resume', model: 'gpt-5.6-sol', minutesAgo: 16, runtimeState: 'closed', executionOwner: 'stopped', state: 'exited', viaCmux: true, agent: 'other' }),
  record({ id: 'rail-08', title: 'Build session rail surface 6', project: 'mac-command-bar', branch: 'feature/rail-surface-6', model: 'gpt-5.6-sol', minutesAgo: 20, runtimeState: 'idle', executionOwner: 'structured' }),
  record({ id: 'rail-09', title: 'Build session rail surface 7', project: 'mac-command-bar', branch: 'tsk-810-rail-7', model: 'gpt-5.6-sol', minutesAgo: 24, runtimeState: 'idle', executionOwner: 'structured' }),
  record({ id: 'rail-10', title: 'Build session rail surface 8', project: 'mac-command-bar', branch: 'feature/rail-surface-8', model: 'gpt-5.6-sol', minutesAgo: 28, runtimeState: 'idle', executionOwner: 'structured' }),
  record({ id: 'rail-11', title: 'Confirm completed rail redesign', project: 'mac-command-bar', branch: 'tsk-810-rail-redesign', model: 'gpt-5.6-sol', minutesAgo: 240, runtimeState: 'idle', executionOwner: 'structured', completedAt: iso(240) }),
  record({ id: 'rail-12', title: 'Archive stopped terminal session', project: 'mac-command-bar', branch: 'tsk-809-settle', model: 'gpt-5.6-sol', minutesAgo: 480, runtimeState: 'closed', executionOwner: 'stopped', state: 'exited', settledAt: iso(480), viaCmux: true, agent: 'other' }),
  record({ id: 'rail-13', title: 'Keep a settled reference visible', project: 'EdiPlatform', branch: 'main', model: 'gpt-5.6-sol', minutesAgo: 720, runtimeState: 'idle', executionOwner: 'structured', settledAt: iso(720) })
];

const viewOptions = {
  groupBy: 'status',
  sortBy: 'recent',
  visibleStatuses: ['working', 'done', 'settled']
};

const seedScript = `
  localStorage.setItem('mac-command-bar.next.owned-sessions', ${JSON.stringify(JSON.stringify(owned))});
  localStorage.setItem('mac-command-bar.next.my-work-view-options', ${JSON.stringify(JSON.stringify(viewOptions))});
  localStorage.removeItem('mac-command-bar.next.side-panes-v1');
`;

async function styles(page, selectors) {
  return page.evaluate((selectors) => {
    const serialize = (element) => {
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return {
        tag: element.tagName.toLowerCase(),
        className: typeof element.className === 'string' ? element.className : '',
        text: (element.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 160),
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        style: Object.fromEntries([
          'display', 'position', 'padding', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
          'margin', 'marginTop', 'marginRight', 'marginBottom', 'marginLeft', 'gap', 'width', 'height',
          'minHeight', 'fontFamily', 'fontSize', 'fontWeight', 'lineHeight', 'letterSpacing', 'color',
          'backgroundColor', 'border', 'borderRadius', 'boxShadow', 'opacity', 'transform', 'overflow',
          'textOverflow', 'whiteSpace', 'zIndex'
        ].map((key) => [key, style[key]]))
      };
    };
    return Object.fromEntries(selectors.map((selector) => [selector, serialize(selector.includes('::') ? null : document.querySelector(selector))]));
  }, selectors);
}

async function hoverAndCapture(page, row, name) {
  await row.hover();
  await page.waitForTimeout(220);
  await page.screenshot({ path: path.join(output, `${name}-full.png`), fullPage: true });
  const crop = page.locator('[data-testid="sessions-column"]').first();
  const box = await crop.boundingBox();
  if (box) {
    await page.screenshot({
      path: path.join(output, `${name}-rail.png`),
      clip: { x: box.x, y: box.y, width: Math.min(300, box.width), height: 671.1875 }
    });
  }
}

const context = await chromium.launchPersistentContext(profile, {
  channel: 'chrome',
  headless: true,
  viewport,
  deviceScaleFactor: 1,
  args: ['--disable-background-networking', '--disable-features=Translate']
});

try {
  const mockup = await context.newPage();
  await mockup.goto(`file://${mockupPath}`);
  await mockup.evaluate(() => scrollTo(0, 0));
  await mockup.screenshot({ path: path.join(output, 'initial-mockup-full.png'), fullPage: true });
  await mockup.locator('#rail-after').screenshot({ path: path.join(output, 'initial-mockup-rail.png') });
  await mockup.locator('.rail-with-card').screenshot({ path: path.join(output, 'initial-mockup-card.png') });
  const mockupSelectors = [
    '.rail', '.rail-top', '.rail-title', '.rail-btn', '.rail-body', '.group-head', '.group-count',
    '.row', '.row.selected', '.row-title-line', '.row-title', '.row-meta-line', '.row-meta',
    '.row-meta .branch', '.chip', '.row-time', '.provider', '.presence', '.presence.working .dot',
    '.presence.attention .dot', '.presence.idle .dot', '.presence.stopped .dot', '.presence.done .dot',
    '.presence.failed .dot', '.row.is-hovered .cluster', '.act', '.cluster::before', '.card',
    '.card-title', '.card-sub', '.card-line', '.card-line svg', '.card-foot', '.card-foot .pill',
    '.legend', '.legend-item'
  ];
  await fs.writeFile(path.join(output, 'initial-mockup-measure.json'), JSON.stringify({ viewport, styles: await styles(mockup, mockupSelectors) }, null, 2));

  const live = await context.newPage();
  live.on('console', (message) => console.log(`[live:${message.type()}] ${message.text()}`));
  live.on('pageerror', (error) => console.log(`[live:pageerror] ${error.message}`));
  await live.addInitScript({ content: seedScript });
  await live.goto('http://127.0.0.1:5177/next', { waitUntil: 'domcontentloaded' });
  await live.waitForSelector('[data-testid="worktree-agent-row"]', { timeout: 30_000 });
  await live.waitForTimeout(900);
  await live.screenshot({ path: path.join(output, 'initial-live-full.png'), fullPage: true });
  const liveCrop = live.locator('[data-testid="sessions-column"]').first();
  const liveRailBox = await liveCrop.boundingBox();
  if (liveRailBox) {
    await live.screenshot({
      path: path.join(output, 'initial-live-rail.png'),
      clip: { x: liveRailBox.x, y: liveRailBox.y, width: Math.min(300, liveRailBox.width), height: 671.1875 }
    });
  }
  const liveSelectors = [
    '[data-testid="sessions-column"]', '[data-testid="my-work-session-list"]', '[data-testid="working-pane"]',
    '[data-testid="sessions-column"] > header', '[data-testid="sessions-column"] > header h2',
    '[data-testid="sessions-column"] > header button', '[data-testid="working-pane-header"] .pane-heading-label',
    '[data-testid="working-pane-header"] .pane-count',
    '[data-testid="working-pane-header"]', '[data-testid="done-pane-header"]', '[data-testid="settled-pane-header"]',
    '[data-testid="worktree-agent-row"]', '[data-testid="worktree-agent-row"] .row-body',
    '[data-testid="worktree-agent-row"] .row-title-line', '[data-testid="worktree-agent-title"]',
    '[data-testid="worktree-agent-meta-line"]', '[data-testid="worktree-agent-meta"]',
    '[data-testid="worktree-agent-model"]', '[data-testid="worktree-agent-activity"]',
    '[data-testid="worktree-agent-provider-icon"]', '[data-testid="worktree-agent-runtime"]',
    '[data-testid="worktree-agent-runtime"] .presence-dot', '[data-testid="worktree-agent-select"]',
    '[data-testid="session-finder"]'
  ];
  const initialLiveStyles = await styles(live, liveSelectors);
  await fs.writeFile(path.join(output, 'initial-live-measure.json'), JSON.stringify({ viewport, styles: initialLiveStyles, rowCount: await live.locator('[data-testid="worktree-agent-row"]').count() }, null, 2));

  const rows = live.locator('[data-testid="worktree-agent-row"]');
  const first = rows.nth(0);
  await live.mouse.move(900, 900);
  await live.waitForTimeout(80);
  const restOverlayCount = await live.locator('[data-testid="worktree-agent-overlay"]').count();
  await first.locator('[data-testid="worktree-agent-select"]').click({ position: { x: 80, y: 10 } });
  await live.waitForTimeout(250);
  await live.waitForTimeout(250);
  const selectedPseudo = await live.evaluate(() => {
    const row = document.querySelector('[data-testid="worktree-agent-row"].active');
    if (!row) return null;
    const style = getComputedStyle(row, '::before');
    return { width: style.width, backgroundColor: style.backgroundColor, inset: style.inset };
  });
  await fs.writeFile(path.join(output, 'initial-live-selected-measure.json'), JSON.stringify({
    viewport,
    styles: await styles(live, [
      '[data-testid="worktree-agent-row"].active',
      '[data-testid="worktree-agent-row"].active .row-title',
      '[data-testid="worktree-agent-row"].active::before'
    ]),
    selectedTitle: await first.getAttribute('title'),
    selectedPseudo
  }, null, 2));
  await live.screenshot({ path: path.join(output, 'initial-live-selected-full.png'), fullPage: true });
  const selectedRailBox = await liveCrop.boundingBox();
  if (selectedRailBox) {
    await live.screenshot({
      path: path.join(output, 'initial-live-selected-rail.png'),
      clip: { x: selectedRailBox.x, y: selectedRailBox.y, width: Math.min(300, selectedRailBox.width), height: 671.1875 }
    });
  }
  await live.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  await live.mouse.move(900, 900);
  await live.waitForTimeout(220);
  const behaviorRow = await rows.nth(1).boundingBox();
  if (!behaviorRow) throw new Error('second row is not measurable');
  await live.mouse.move(behaviorRow.x + 20, behaviorRow.y + 20);
  const delayReceipt = await live.evaluate(async () => {
    const row = document.querySelectorAll('[data-testid="worktree-agent-row"]')[1];
    row.dispatchEvent(new MouseEvent('mouseleave'));
    row.dispatchEvent(new MouseEvent('mouseenter'));
    const cardVisible = () => Boolean(document.querySelector('[data-testid="session-hover-card"]'));
    const before = cardVisible();
    await new Promise((resolve) => setTimeout(resolve, 80));
    const beforeDelay = cardVisible();
    await new Promise((resolve) => setTimeout(resolve, 120));
    return { before, beforeDelay, afterDelay: cardVisible() };
  });
  const overlayBeforeDelay = delayReceipt.beforeDelay ? 1 : 0;
  const overlayAfterDelay = delayReceipt.afterDelay ? 1 : 0;
  const hoveredRow = await rows.nth(1).boundingBox();
  const hoveredCard = await live.locator('[data-testid="session-hover-card"]').boundingBox();
  await fs.writeFile(path.join(output, 'initial-live-behavior.json'), JSON.stringify({
    overlayHiddenAtRest: restOverlayCount === 0,
    cardHiddenBeforeDelay: delayReceipt.before === false && overlayBeforeDelay === 0,
    cardVisibleAfterDelay: overlayAfterDelay === 1,
    cardRightOfRow: hoveredRow && hoveredCard ? hoveredCard.x >= hoveredRow.x + hoveredRow.width + 8 : false,
    row: hoveredRow,
    card: hoveredCard
  }, null, 2));
  await hoverAndCapture(live, rows.nth(1), 'initial-live-hover');
  const hovered = await styles(live, [
    '[data-testid="worktree-agent-row"]:hover', '[data-testid="worktree-agent-overlay"]',
    '[data-testid="worktree-agent-overlay"] .action', '[data-testid="worktree-agent-hover-popover"]',
    '[data-testid="session-hover-card"]', '[data-testid="session-hover-card"] .session-card-title',
    '[data-testid="session-hover-card"] .session-card-sub', '[data-testid="session-hover-card"] .session-card-line',
    '[data-testid="session-hover-card"] .session-card-line svg', '[data-testid="session-hover-card"] .session-card-foot',
    '[data-testid="session-hover-card"] .status-pill'
  ]);
  const gradientPseudo = await live.evaluate(() => {
    const overlay = document.querySelector('[data-testid="worktree-agent-overlay"]');
    if (!overlay) return null;
    const style = getComputedStyle(overlay, '::before');
    return { left: style.left, width: style.width, backgroundImage: style.backgroundImage };
  });
  await fs.writeFile(path.join(output, 'initial-live-hover-measure.json'), JSON.stringify({ viewport, styles: hovered, gradientPseudo }, null, 2));
  const liveCard = live.locator('[data-testid="session-hover-card"]').first();
  if (await liveCard.count()) await liveCard.screenshot({ path: path.join(output, 'initial-live-hover-card.png') });

  const stopped = live.locator('[data-presence="stopped"]').first();
  if (await stopped.count()) {
    await stopped.hover();
    await live.waitForTimeout(220);
    await live.screenshot({ path: path.join(output, 'initial-live-stopped-full.png'), fullPage: true });
    await live.screenshot({
      path: path.join(output, 'initial-live-stopped-rail.png'),
      clip: { x: 0, y: 35, width: 300, height: 671.1875 }
    });
    await fs.writeFile(path.join(output, 'initial-live-stopped-actions.json'), JSON.stringify({
      labels: await stopped.locator('[data-testid="worktree-agent-overlay"] button').evaluateAll((buttons) => buttons.map((button) => button.getAttribute('aria-label') || button.getAttribute('title'))),
      overlay: await stopped.locator('[data-testid="worktree-agent-overlay"]').boundingBox()
    }, null, 2));
  }

  await fs.writeFile(path.join(output, 'initial-live-presence.json'), JSON.stringify({
    rows: await rows.evaluateAll((items) => items.map((item) => ({
      title: item.getAttribute('title'),
      state: item.getAttribute('data-presence'),
      label: item.querySelector('[data-testid="worktree-agent-runtime"]')?.getAttribute('aria-label') ?? null
    })))
  }, null, 2));

  await fs.writeFile(path.join(output, 'initial-seed.json'), JSON.stringify({ key: 'mac-command-bar.next.owned-sessions', viewKey: 'mac-command-bar.next.my-work-view-options', count: owned.length, groups: { working: 10, done: 1, settled: 2 } }, null, 2));
  console.log(JSON.stringify({ mockup: 'captured', live: 'captured', rowCount: await rows.count(), viewport }, null, 2));
} finally {
  await context.close();
}
