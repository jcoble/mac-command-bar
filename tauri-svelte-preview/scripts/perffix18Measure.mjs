import { execFileSync } from 'node:child_process';
import { chromium } from '/Users/blackcolours/.nvm/versions/node/v24.12.0/lib/node_modules/@playwright/cli/node_modules/playwright/index.mjs';

const URL = 'http://127.0.0.1:5177/next';
const SESSION_KEY = 'mac-command-bar.next.owned-sessions';
const OPTIONS_KEY = 'mac-command-bar.next.my-work-view-options';
const SESSION_COUNT = 30;
const VISIBLE_ROW = '[data-testid="worktree-agent-row"]:visible';

const sessions = Array.from({ length: SESSION_COUNT }, (_, index) => ({
  ownedId: `perffix18-session-${index + 1}`,
  agent: index % 2 ? 'claude' : 'codex',
  origin: 'external',
  viaCmux: false,
  source: 'scanned',
  title: `Performance session ${String(index + 1).padStart(2, '0')}`,
  model: null,
  projectPath: '/tmp/perffix18-project',
  cwd: `/tmp/perffix18-project/worktree-${index + 1}`,
  resumeCommand: null,
  nativeSessionId: null,
  ptySessionId: null,
  state: 'exited',
  lastError: null,
  executionOwner: 'stopped',
  runtimeState: 'closed',
  providerInstanceId: null,
  activeTurnId: null,
  capabilityRevision: 0,
  lastRuntimeError: null,
  completedAt: null,
  settledAt: null,
  branch: `perffix18-${index + 1}`,
  taskId: null,
  pullRequest: null,
  messageCount: 4 + index,
  latestTurnPreview: `Finished measurement fixture ${index + 1}`,
  lastActivity: new Date(Date.now() - index * 60_000).toISOString()
}));

function descendantsOf(rootPid) {
  const rows = execFileSync('ps', ['-axo', 'pid=,ppid=,command='], { encoding: 'utf8', maxBuffer: 20 * 1024 * 1024 })
    .trim()
    .split('\n')
    .map((line) => {
      const match = line.trim().match(/^(\d+)\s+(\d+)\s+(.*)$/);
      return match ? { pid: Number(match[1]), ppid: Number(match[2]), command: match[3] } : null;
    })
    .filter(Boolean);
  const descendants = [];
  const parents = new Set([rootPid]);
  let added = true;
  while (added) {
    added = false;
    for (const row of rows) {
      if (!parents.has(row.ppid) || parents.has(row.pid)) continue;
      parents.add(row.pid);
      descendants.push(row);
      added = true;
    }
  }
  return descendants;
}

function cpuFor(pid) {
  const output = execFileSync('ps', ['-o', '%cpu=', '-p', String(pid)], { encoding: 'utf8' }).trim();
  return Number(output || 0);
}

function summarize(values) {
  if (!values.length) return { count: 0, average: 0, max: 0 };
  return {
    count: values.length,
    average: Number((values.reduce((sum, value) => sum + value, 0) / values.length).toFixed(1)),
    max: Number(Math.max(...values).toFixed(1))
  };
}

async function settle(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function animations(page) {
  return page.evaluate(() => document.getAnimations().map((animation) => {
    const target = animation.effect?.target;
    return {
      name: animation.animationName || '(transition)',
      state: animation.playState,
      element: target instanceof Element
        ? `${target.tagName.toLowerCase()}${target.id ? `#${target.id}` : ''}${target.classList.length ? `.${[...target.classList].join('.')}` : ''}`
        : '(unknown)'
    };
  }));
}

async function measureAction(page, name, action) {
  await page.evaluate((label) => {
    window.__perffix18.currentAction = label;
    window.__perffix18.actionStart = performance.now();
    window.__perffix18.actionGeometryStart = window.__perffix18.geometryReads;
    window.__perffix18.actionLongTaskStart = window.__perffix18.longTasks.length;
  }, name);
  const started = await page.evaluate(() => performance.now());
  await action();
  await settle(page);
  return page.evaluate(({ label, startedAt }) => {
    const data = window.__perffix18;
    const tasks = data.longTasks.slice(data.actionLongTaskStart);
    return {
      name: label,
      durationMs: Number((performance.now() - startedAt).toFixed(1)),
      longTaskCount: tasks.length,
      longTaskTotalMs: Number(tasks.reduce((sum, task) => sum + task.duration, 0).toFixed(1)),
      longTaskMaxMs: Number(Math.max(0, ...tasks.map((task) => task.duration)).toFixed(1)),
      geometryReads: data.geometryReads - data.actionGeometryStart
    };
  }, { label: name, startedAt: started });
}

function topProfileEntries(profile, limit = 20) {
  const nodes = new Map(profile.nodes.map((node) => [node.id, node]));
  const totals = new Map();
  for (let index = 0; index < profile.samples.length; index += 1) {
    const node = nodes.get(profile.samples[index]);
    if (!node) continue;
    const frame = node.callFrame;
    const key = `${frame.functionName || '(anonymous)'}|${frame.url || '(native)'}:${frame.lineNumber + 1}`;
    totals.set(key, (totals.get(key) ?? 0) + (profile.timeDeltas[index] ?? 0) / 1000);
  }
  return [...totals.entries()]
    .map(([source, sampledMs]) => ({ source, sampledMs: Number(sampledMs.toFixed(1)) }))
    .sort((left, right) => right.sampledMs - left.sampledMs)
    .slice(0, limit);
}

async function profileAction(cdp, action) {
  await cdp.send('Profiler.enable');
  await cdp.send('Profiler.start');
  await action();
  const { profile } = await cdp.send('Profiler.stop');
  return topProfileEntries(profile);
}

const browser = await chromium.launch({ headless: true, channel: 'chrome' });
const browserPid = descendantsOf(process.pid).find((row) =>
  !row.command.includes('--type=')
  && (row.command.includes('Google Chrome') || row.command.includes('/chrome '))
)?.pid ?? null;
const context = await browser.newContext({ viewport: { width: 1710, height: 990 } });
await context.addInitScript(({ sessionKey, optionsKey, seededSessions }) => {
  localStorage.setItem(sessionKey, JSON.stringify(seededSessions));
  localStorage.setItem(optionsKey, JSON.stringify({
    groupBy: 'status',
    sortBy: 'recent',
    visibleStatuses: ['working', 'done', 'settled']
  }));

  const data = window.__perffix18 = {
    timers: [],
    timerCallbacks: [],
    longTasks: [],
    resizeCallbacks: 0,
    resizeEntries: 0,
    geometryReads: 0,
    currentAction: 'load'
  };
  const timerIds = new Map();
  const nativeSetTimeout = window.setTimeout.bind(window);
  const nativeClearTimeout = window.clearTimeout.bind(window);
  const nativeSetInterval = window.setInterval.bind(window);
  const nativeClearInterval = window.clearInterval.bind(window);
  const stackOrigin = () => (new Error().stack ?? '')
    .split('\n')
    .slice(2)
    .find((line) => !line.includes('at record') && !line.includes('window.set'))
    ?.trim() ?? '(unknown)';
  const record = (kind, delay) => {
    const entry = { id: data.timers.length + 1, kind, delay: Number(delay) || 0, origin: stackOrigin(), scheduledAt: performance.now(), cleared: false, fired: 0 };
    data.timers.push(entry);
    return entry;
  };
  window.setTimeout = (callback, delay = 0, ...args) => {
    const entry = record('timeout', delay);
    const id = nativeSetTimeout((...callbackArgs) => {
      entry.fired += 1;
      data.timerCallbacks.push({ timerId: entry.id, at: performance.now(), action: data.currentAction });
      if (typeof callback === 'function') callback(...callbackArgs);
      else Function(callback)();
    }, delay, ...args);
    timerIds.set(id, entry);
    return id;
  };
  window.clearTimeout = (id) => {
    const entry = timerIds.get(id);
    if (entry) entry.cleared = true;
    return nativeClearTimeout(id);
  };
  window.setInterval = (callback, delay = 0, ...args) => {
    const entry = record('interval', delay);
    const id = nativeSetInterval((...callbackArgs) => {
      entry.fired += 1;
      data.timerCallbacks.push({ timerId: entry.id, at: performance.now(), action: data.currentAction });
      if (typeof callback === 'function') callback(...callbackArgs);
      else Function(callback)();
    }, delay, ...args);
    timerIds.set(id, entry);
    return id;
  };
  window.clearInterval = (id) => {
    const entry = timerIds.get(id);
    if (entry) entry.cleared = true;
    return nativeClearInterval(id);
  };

  const nativeRect = Element.prototype.getBoundingClientRect;
  Element.prototype.getBoundingClientRect = function (...args) {
    data.geometryReads += 1;
    return nativeRect.apply(this, args);
  };

  if ('ResizeObserver' in window) {
    const NativeResizeObserver = window.ResizeObserver;
    window.ResizeObserver = class extends NativeResizeObserver {
      constructor(callback) {
        super((entries, observer) => {
          data.resizeCallbacks += 1;
          data.resizeEntries += entries.length;
          callback(entries, observer);
        });
      }
    };
  }

  if ('PerformanceObserver' in window) {
    try {
      const observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          data.longTasks.push({ startTime: entry.startTime, duration: entry.duration, action: data.currentAction });
        }
      });
      observer.observe({ type: 'longtask', buffered: true });
    } catch {}
  }
}, { sessionKey: SESSION_KEY, optionsKey: OPTIONS_KEY, seededSessions: sessions });

const page = await context.newPage();
const result = { command: 'node scripts/perffix18Measure.mjs', url: URL, browserPid, viewport: '1710x990' };

try {
  await page.goto(URL, { waitUntil: 'networkidle' });
  await page.locator(VISIBLE_ROW).first().waitFor();
  await page.waitForTimeout(1000);
  result.rowCount = await page.locator(VISIBLE_ROW).count();
  result.animationsAtRest = await animations(page);

  const firstRow = page.locator(VISIBLE_ROW).first();
  await firstRow.hover();
  await page.waitForTimeout(220);
  await page.mouse.move(1000, 500);
  await page.waitForTimeout(250);
  result.animationsAfterRailHoverOff = await animations(page);

  const idleStart = await page.evaluate(() => ({
    resizeCallbacks: window.__perffix18.resizeCallbacks,
    resizeEntries: window.__perffix18.resizeEntries,
    longTasks: window.__perffix18.longTasks.length
  }));
  await page.waitForTimeout(5000);
  result.idle5s = await page.evaluate((start) => ({
    resizeCallbacks: window.__perffix18.resizeCallbacks - start.resizeCallbacks,
    resizeEntries: window.__perffix18.resizeEntries - start.resizeEntries,
    longTasks: window.__perffix18.longTasks.length - start.longTasks
  }), idleStart);

  const rendererRows = browserPid === null
    ? []
    : descendantsOf(browserPid).filter((row) => row.command.includes('--type=renderer'));
  result.rendererPids = rendererRows.map((row) => row.pid);
  result.idleCpuSamples = [];
  for (let sample = 0; sample < 10; sample += 1) {
    const rendererCpu = rendererRows.map((row) => ({ pid: row.pid, cpu: cpuFor(row.pid) }));
    result.idleCpuSamples.push({
      atSecond: sample + 1,
      renderers: rendererCpu,
      totalCpu: Number(rendererCpu.reduce((sum, row) => sum + row.cpu, 0).toFixed(1))
    });
    await page.waitForTimeout(1000);
  }
  result.idleCpuTotal = summarize(result.idleCpuSamples.map((sample) => sample.totalCpu));

  result.interactions = [];
  const clickDurations = [];
  const clickHarnessWallDurations = [];
  for (const index of [1, 2, 3, 4, 5]) {
    const row = page.locator(VISIBLE_ROW).nth(index);
    const box = await row.boundingBox();
    if (!box) throw new Error(`Session row ${index + 1} has no box`);
    await page.evaluate(() => {
      window.__perffix18.sessionClickEventMs = null;
      window.__perffix18.sessionInteractiveMs = null;
      document.addEventListener('click', (event) => {
        const target = event.target instanceof Element
          ? event.target.closest('[data-testid="worktree-agent-select"]')
          : null;
        if (!(target instanceof HTMLElement)) return;
        const clickedAt = performance.now();
        window.__perffix18.sessionClickEventMs = clickedAt;
        const observer = new MutationObserver(() => {
          if (target.getAttribute('aria-current') !== 'true') return;
          window.__perffix18.sessionInteractiveMs = performance.now() - clickedAt;
          observer.disconnect();
        });
        observer.observe(target, { attributes: true, attributeFilter: ['aria-current'] });
      }, { capture: true, once: true });
    });
    const measurement = await measureAction(page, `session-click-${index + 1}`, async () => {
      await page.mouse.click(box.x + 20, box.y + box.height / 2);
      await page.locator('[data-testid="worktree-agent-select"][aria-current="true"]:visible').waitFor();
    });
    measurement.eventToInteractiveMs = await page.evaluate(() => Number(
      (window.__perffix18.sessionInteractiveMs ?? Number.NaN).toFixed(1)
    ));
    clickDurations.push(measurement.eventToInteractiveMs);
    clickHarnessWallDurations.push(measurement.durationMs);
    result.interactions.push(measurement);
  }
  result.sessionClick = summarize(clickDurations);
  result.sessionClickHarnessWall = summarize(clickHarnessWallDurations);

  await page.evaluate(async () => {
    const railStore = await import('/src/lib/shell/stores/sessionRailStore.svelte.ts');
    const store = await import('/src/lib/shell/conversation/conversationStore.svelte.ts');
    const ownedId = railStore.rail.activeOwnedId;
    if (!ownedId) throw new Error('No active fixture session for composer measurements');
    railStore.updateOwnedSession(ownedId, {
      agent: 'codex',
      origin: 'app',
      state: 'background',
      executionOwner: 'structured',
      runtimeState: 'working'
    });
    const state = store.ensureConversationSession(ownedId, 'codex');
    state.availableCommands = Array.from({ length: 40 }, (_, commandIndex) => ({
      id: `fixture-${commandIndex + 1}`,
      name: `fixture-${commandIndex + 1}`,
      label: `Fixture command ${commandIndex + 1}`,
      description: `Performance command ${commandIndex + 1}`,
      source: commandIndex % 2 ? 'provider' : 'assembly',
      action: 'insert'
    }));
    state.metadata.usedTokens = 1000;
    state.metadata.contextWindow = 10000;
  });
  await page.locator('[data-testid="conversation-composer-input"]').waitFor();
  await settle(page);
  result.workingRowAnimations = await animations(page);
  await page.evaluate(async () => {
    const railStore = await import('/src/lib/shell/stores/sessionRailStore.svelte.ts');
    if (railStore.rail.activeOwnedId) {
      railStore.updateOwnedSession(railStore.rail.activeOwnedId, { runtimeState: 'ready' });
    }
  });
  await settle(page);

  const composer = page.locator('[data-testid="conversation-composer-input"]');
  result.interactions.push(await measureAction(page, 'slash-menu-open', async () => {
    await composer.fill('/');
    await page.locator('[data-testid="conversation-command-menu"]').waitFor();
  }));
  await composer.fill('');
  result.interactions.push(await measureAction(page, 'composer-type-20', async () => {
    await composer.pressSequentially('abcdefghijklmnopqrst', { delay: 0 });
  }));

  result.interactions.push(await measureAction(page, 'hover-10-rail-rows', async () => {
    for (let index = 0; index < 10; index += 1) {
      await page.locator(VISIBLE_ROW).nth(index).hover();
      await page.waitForTimeout(20);
    }
    await page.mouse.move(1000, 500);
    await page.waitForTimeout(220);
  }));
  result.hoverCardsAfterChurn = await page.locator('[data-testid="session-hover-card"]').count();

  const browserEntry = page.locator('[data-testid="session-browser-entry"]');
  result.interactions.push(await measureAction(page, 'browser-overlay-open', async () => {
    await browserEntry.click();
    await page.locator('[data-testid="session-browser-overlay"]').waitFor();
  }));
  result.interactions.push(await measureAction(page, 'browser-overlay-close', async () => {
    await page.getByRole('button', { name: 'Close the browser overlay' }).click();
    await page.locator('[data-testid="session-browser-overlay"]').waitFor({ state: 'detached' });
  }));

  if (process.env.PERFFIX18_PROFILE === '1') {
    const cdp = await context.newCDPSession(page);
    result.profiles = {};
    result.profiles.sessionClick = await profileAction(cdp, async () => {
      const row = page.locator(VISIBLE_ROW).nth(7);
      const box = await row.boundingBox();
      if (!box) throw new Error('Profile session row has no box');
      await page.mouse.click(box.x + 20, box.y + box.height / 2);
      await settle(page);
    });
    result.profiles.railHover = await profileAction(cdp, async () => {
      for (let index = 0; index < 10; index += 1) {
        await page.locator(VISIBLE_ROW).nth(index).hover();
        await page.waitForTimeout(20);
      }
      await page.mouse.move(1000, 500);
      await page.waitForTimeout(220);
      await settle(page);
    });
    await cdp.detach();
  }

  result.timerCensus = await page.evaluate(() => {
    const timers = window.__perffix18.timers;
    const groupedTimeouts = new Map();
    for (const timer of timers.filter((entry) => entry.kind === 'timeout')) {
      const key = `${timer.delay}|${timer.origin}`;
      const group = groupedTimeouts.get(key) ?? { kind: 'repeated-timeout', delay: timer.delay, origin: timer.origin, scheduled: 0, fired: 0, cleared: 0 };
      group.scheduled += 1;
      group.fired += timer.fired;
      group.cleared += timer.cleared ? 1 : 0;
      groupedTimeouts.set(key, group);
    }
    return {
      intervals: timers.filter((entry) => entry.kind === 'interval'),
      recurringTimeouts: [...groupedTimeouts.values()].filter((group) => group.scheduled > 1),
      allTimerCount: timers.length
    };
  });
} finally {
  await context.close();
  await browser.close();
  result.browserCleanup = browserPid === null ? 'no browser pid' : descendantsOf(browserPid).length === 0 ? 'stopped' : 'descendants remain';
}

console.log(JSON.stringify(result, null, 2));
