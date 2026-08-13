import { spawn, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';

import { createPerfHarnessMockPayload, type PerfHarnessMockPayload } from './perfHarnessMock.ts';

const HOST = '127.0.0.1';
const PORT = 5181;
const APP_URL = `http://${HOST}:${PORT}/next`;
const VIEWPORT = { width: 1710, height: 990 };
const PLAYWRIGHT_PATH = '/Users/blackcolours/.nvm/versions/node/v24.12.0/lib/node_modules/@playwright/cli/node_modules/playwright/index.mjs';
const SESSION_TAG = 'perfharness20-chrome';
const HOVER_SAMPLE_COUNT = 10;
const IDLE_MS = 15_000;
const TIMER_WINDOW_MS = 10_000;

interface HoverReceipt {
  phase: string;
  row: string;
  highlightMs: number;
  buttonsMs: number;
}

interface LongTaskReceipt {
  startTime: number;
  duration: number;
  name: string;
}

interface TimerCallsiteReceipt {
  kind: 'interval' | 'timeout';
  callsite: string;
  active: number;
  scheduled: number;
  fired: number;
}

interface BrowserHarnessApi {
  resetIdleWindow(): void;
  longTasks(): LongTaskReceipt[];
  timerCensus(): TimerCallsiteReceipt[];
}

interface ProfileNode {
  id: number;
  callFrame: { functionName: string; url: string; lineNumber: number };
}

interface CpuProfile {
  nodes: ProfileNode[];
  samples?: number[];
}

function percentile(values: number[], fraction: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)];
}

function round(value: number): number {
  return Math.round(value * 10) / 10;
}

function printTable<T extends Record<string, unknown>>(title: string, rows: T[]): void {
  process.stdout.write(`\n${title}\n`);
  console.table(rows);
}

async function waitForVite(child: ChildProcess): Promise<void> {
  let output = '';
  let announcedReady = false;
  const deadline = Date.now() + 30_000;
  child.stdout?.on('data', (chunk: Buffer) => {
    output += chunk.toString();
    announcedReady ||= output.includes(`http://${HOST}:${PORT}/`);
  });
  child.stderr?.on('data', (chunk: Buffer) => { output += chunk.toString(); });
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`Vite exited ${child.exitCode}:\n${output}`);
    if (announcedReady) {
      try {
        const response = await fetch(APP_URL);
        if (response.ok && child.exitCode === null) return;
      } catch {
        // This Vite announced first, but has not accepted its first request yet.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Vite did not become ready on ${APP_URL}:\n${output}`);
}

async function stopProcessGroup(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null || child.pid === undefined) return;
  try {
    process.kill(-child.pid, 'SIGTERM');
  } catch {
    child.kill('SIGTERM');
  }
  await Promise.race([
    once(child, 'exit'),
    new Promise((resolve) => setTimeout(resolve, 3_000))
  ]);
  if (child.exitCode === null) {
    try {
      process.kill(-child.pid, 'SIGKILL');
    } catch {
      child.kill('SIGKILL');
    }
  }
}

async function processGroupMembers(processGroupId: number | undefined): Promise<string[]> {
  if (processGroupId === undefined) return [];
  const result = await new Promise<string>((resolve) => {
    const ps = spawn('ps', ['-axo', 'pid=,pgid=,command=']);
    let output = '';
    ps.stdout.on('data', (chunk: Buffer) => { output += chunk.toString(); });
    ps.on('exit', () => resolve(output));
  });
  return result.split('\n').filter((line) => {
    const fields = line.trim().split(/\s+/, 3);
    return Number(fields[1]) === processGroupId;
  });
}

function mockInitSource(payload: PerfHarnessMockPayload): string {
  return `(() => {
    const payload = ${JSON.stringify(payload)};
    const callbacks = new Map();
    let callbackId = 1;
    const timerRows = new Map();
    const intervalSites = new Map();
    const longTasks = [];
    let idleStart = 0;
    const stackSite = () => {
      const lines = String(new Error().stack || '').split('\\n').slice(3);
      return lines.find((line) => !line.includes('perfHarness'))?.trim() || lines[0]?.trim() || 'unknown';
    };
    const note = (kind, site, field) => {
      const key = kind + ':' + site;
      const row = timerRows.get(key) || { kind, callsite: site, active: 0, scheduled: 0, fired: 0 };
      row[field] += 1;
      timerRows.set(key, row);
    };
    const nativeSetTimeout = window.setTimeout.bind(window);
    const nativeSetInterval = window.setInterval.bind(window);
    const nativeClearInterval = window.clearInterval.bind(window);
    window.setTimeout = (handler, delay = 0, ...args) => {
      const site = stackSite();
      note('timeout', site, 'scheduled');
      const wrapped = typeof handler === 'function'
        ? (...inner) => { note('timeout', site, 'fired'); return handler(...inner); }
        : handler;
      return nativeSetTimeout(wrapped, delay, ...args);
    };
    window.setInterval = (handler, delay = 0, ...args) => {
      const site = stackSite();
      note('interval', site, 'scheduled');
      const wrapped = typeof handler === 'function'
        ? (...inner) => { note('interval', site, 'fired'); return handler(...inner); }
        : handler;
      const handle = nativeSetInterval(wrapped, delay, ...args);
      intervalSites.set(handle, site);
      const row = timerRows.get('interval:' + site);
      row.active += 1;
      return handle;
    };
    window.clearInterval = (handle) => {
      const site = intervalSites.get(handle);
      if (site) {
        const row = timerRows.get('interval:' + site);
        row.active = Math.max(0, row.active - 1);
        intervalSites.delete(handle);
      }
      nativeClearInterval(handle);
    };
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        longTasks.push({ startTime: entry.startTime, duration: entry.duration, name: entry.name });
      }
    }).observe({ type: 'longtask', buffered: true });
    window.__perfHarness = {
      resetIdleWindow() {
        idleStart = performance.now();
        for (const [key, row] of timerRows) {
          row.scheduled = 0;
          row.fired = 0;
          if (row.active === 0) timerRows.delete(key);
        }
      },
      longTasks() { return longTasks.filter((entry) => entry.startTime >= idleStart); },
      timerCensus() { return [...timerRows.values()].sort((a, b) => b.fired - a.fired || b.scheduled - a.scheduled); }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      transformCallback(callback, once = false) {
        const id = callbackId++;
        callbacks.set(id, { callback, once });
        return id;
      },
      unregisterCallback(id) { callbacks.delete(id); },
      convertFileSrc(path) { return path; },
      async invoke(command, args = {}) {
        switch (command) {
          case 'plugin:event|listen': return callbackId++;
          case 'plugin:event|unlisten': return true;
          case 'list_terminal_sessions': return [];
          case 'list_agent_conversation_sessions': return payload.sessions;
          case 'list_agent_sessions': return [];
          case 'read_agent_conversation_snapshot': return payload.snapshots[args.ownedId] || null;
          case 'list_agent_conversation_events': return payload.snapshots[args.ownedId]?.events || [];
          case 'read_agent_conversation_draft': return null;
          case 'read_session_browser_annotations': return [];
          case 'read_agent_conversation_capabilities': return null;
          case 'list_runtime_contexts': return [];
          case 'read_resource_snapshot': return null;
          case 'read_usage_snapshot': return null;
          case 'list_project_worktrees': return [];
          case 'list_git_repository_summaries': return [];
          case 'read_project_git_status': return null;
          case 'backend_capabilities': return [];
          default: return null;
        }
      }
    };
  })();`;
}

async function measureHover(page: { locator(selector: string): { count(): Promise<number>; nth(index: number): { hover(): Promise<void>; evaluate<T>(fn: (element: HTMLElement) => Promise<T>): Promise<T>; getAttribute(name: string): Promise<string | null> } }; mouse: { move(x: number, y: number): Promise<void> } }, phase: string): Promise<HoverReceipt[]> {
  const rows = page.locator('[data-testid="worktree-agent-row"]');
  const count = await rows.count();
  if (count < HOVER_SAMPLE_COUNT) throw new Error(`Expected ${HOVER_SAMPLE_COUNT} rows, found ${count}`);
  const indexes = Array.from({ length: count }, (_, index) => index)
    .sort((left, right) => ((left * 7 + 3) % count) - ((right * 7 + 3) % count))
    .slice(0, HOVER_SAMPLE_COUNT);
  const receipts: HoverReceipt[] = [];
  for (const index of indexes) {
    await page.mouse.move(1700, 980);
    const row = rows.nth(index);
    const pending = row.evaluate(async (element): Promise<{ highlightMs: number; buttonsMs: number }> => {
      const button = element.querySelector<HTMLElement>('[data-testid="worktree-agent-select"]');
      const actions = element.querySelector<HTMLElement>('[data-testid="worktree-agent-overlay"]');
      if (!button || !actions) throw new Error('row measurement targets are missing');
      const before = getComputedStyle(button).backgroundColor;
      return new Promise((resolve) => {
        element.addEventListener('mouseenter', () => {
          const started = performance.now();
          let highlightMs: number | null = null;
          const sample = (): void => {
            const elapsed = performance.now() - started;
            if (highlightMs === null && getComputedStyle(button).backgroundColor !== before) highlightMs = elapsed;
            const visible = Number.parseFloat(getComputedStyle(actions).opacity) >= 0.95;
            if (highlightMs !== null && visible) resolve({ highlightMs, buttonsMs: elapsed });
            else if (elapsed > 1_000) resolve({ highlightMs: highlightMs ?? elapsed, buttonsMs: elapsed });
            else requestAnimationFrame(sample);
          };
          requestAnimationFrame(sample);
        }, { once: true });
      });
    });
    await row.hover();
    const measured = await pending;
    receipts.push({ phase, row: String(index + 1), highlightMs: round(measured.highlightMs), buttonsMs: round(measured.buttonsMs) });
  }
  return receipts;
}

function profileAttribution(profile: CpuProfile): Array<Record<string, unknown>> {
  const counts = new Map<number, number>();
  for (const id of profile.samples ?? []) counts.set(id, (counts.get(id) ?? 0) + 1);
  return profile.nodes
    .map((node) => ({
      function: node.callFrame.functionName || '(anonymous)',
      file: node.callFrame.url ? `${node.callFrame.url.split('/').at(-1)}:${node.callFrame.lineNumber + 1}` : '(runtime)',
      samples: counts.get(node.id) ?? 0
    }))
    .filter((row) => row.samples > 0 && !String(row.file).includes('(runtime)'))
    .sort((left, right) => right.samples - left.samples)
    .slice(0, 12);
}

async function main(): Promise<number> {
  const vite = spawn('node_modules/.bin/vite', ['--host', HOST, '--port', String(PORT), '--strictPort'], {
    cwd: process.cwd(),
    detached: true,
    stdio: ['ignore', 'pipe', 'pipe']
  });
  let browser: { close(): Promise<void> } | null = null;
  try {
    await waitForVite(vite);
    const playwright = await import(PLAYWRIGHT_PATH);
    browser = await playwright.chromium.launch({ headless: true, channel: 'chrome', args: [`--${SESSION_TAG}`] });
    const context = await browser.newContext({ viewport: VIEWPORT });
    const page = await context.newPage();
    await page.addInitScript({ content: mockInitSource(createPerfHarnessMockPayload()) });
    const consoleErrors: string[] = [];
    page.on('console', (message: { type(): string; text(): string }) => {
      if (message.type() === 'error') consoleErrors.push(message.text());
    });
    await page.goto(APP_URL, { waitUntil: 'domcontentloaded' });
    await page.locator('[data-testid="worktree-agent-row"]').first().waitFor({ state: 'visible', timeout: 30_000 });
    const actualViewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
    if (actualViewport.width !== VIEWPORT.width || actualViewport.height !== VIEWPORT.height) {
      await page.setViewportSize(VIEWPORT);
    }
    const verifiedViewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
    if (verifiedViewport.width !== VIEWPORT.width || verifiedViewport.height !== VIEWPORT.height) {
      throw new Error(`Viewport verification failed: ${JSON.stringify(verifiedViewport)}`);
    }

    const beforeHover = await measureHover(page, 'before snapshot');
    const cdp = await context.newCDPSession(page);
    await cdp.send('Profiler.enable');
    await cdp.send('Profiler.setSamplingInterval', { interval: 1_000 });
    await cdp.send('Profiler.start');
    const largeRow = page.locator('[data-testid="worktree-agent-row"]').nth(0);
    await page.evaluate(() => (window as unknown as { __perfHarness: BrowserHarnessApi }).__perfHarness.resetIdleWindow());
    const clickStarted = await page.evaluate(() => performance.now());
    await largeRow.locator('[data-testid="worktree-agent-select"]').click({ position: { x: 12, y: 38 } });
    await page.locator('[data-testid="conversation-timeline-item"]').nth(399).waitFor({ state: 'attached', timeout: 30_000 });
    await page.locator('[data-testid="conversation-composer-input"]').waitFor({ state: 'visible', timeout: 30_000 });
    await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
    const clickInteractiveMs = round((await page.evaluate(() => performance.now())) - clickStarted);
    const snapshotLongTasks = await page.evaluate(() => (window as unknown as { __perfHarness: BrowserHarnessApi }).__perfHarness.longTasks());
    const afterHover = await measureHover(page, 'after 2k events');

    await page.mouse.move(1700, 980);
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForTimeout(250);
    await page.evaluate(() => (window as unknown as { __perfHarness: BrowserHarnessApi }).__perfHarness.resetIdleWindow());
    await page.waitForTimeout(TIMER_WINDOW_MS);
    const timerCensus = await page.evaluate(() => (window as unknown as { __perfHarness: BrowserHarnessApi }).__perfHarness.timerCensus());
    await page.waitForTimeout(IDLE_MS - TIMER_WINDOW_MS);
    const idleLongTasks = await page.evaluate(() => (window as unknown as { __perfHarness: BrowserHarnessApi }).__perfHarness.longTasks());
    const animationCount = await page.evaluate(() => document.getAnimations().length);
    const profileResult = await cdp.send('Profiler.stop') as { profile: CpuProfile };

    const hoverRows = [...beforeHover, ...afterHover];
    printTable('Hover latency (10 deterministic rows per phase)', hoverRows.map((row) => ({
      phase: row.phase,
      row: row.row,
      'highlight ms': row.highlightMs,
      'buttons visible ms': row.buttonsMs
    })));
    printTable('Hover summary', ['before snapshot', 'after 2k events'].map((phase) => {
      const phaseRows = hoverRows.filter((row) => row.phase === phase);
      return {
        phase,
        'highlight p95 ms': round(percentile(phaseRows.map((row) => row.highlightMs), 0.95)),
        'buttons p95 ms': round(percentile(phaseRows.map((row) => row.buttonsMs), 0.95))
      };
    }));
    printTable('Click to transcript interactive', [{ events: 2_000, 'latency ms': clickInteractiveMs }]);
    printTable('Snapshot-load long tasks >50ms', snapshotLongTasks.length > 0
      ? snapshotLongTasks.map((entry) => ({ start: round(entry.startTime), duration: round(entry.duration), name: entry.name }))
      : [{ start: '-', duration: 0, name: 'none' }]);
    printTable('Idle long tasks >50ms (15s)', idleLongTasks.length > 0
      ? idleLongTasks.map((entry) => ({ start: round(entry.startTime), duration: round(entry.duration), name: entry.name }))
      : [{ start: '-', duration: 0, name: 'none' }]);
    printTable('CDP CPU profile attribution', profileAttribution(profileResult.profile));
    printTable('Animation census at rest', [{ animations: animationCount }]);
    printTable('Timer census (first 10s of idle)', timerCensus.slice(0, 20).map((row) => ({
      kind: row.kind,
      active: row.active,
      scheduled: row.scheduled,
      fired: row.fired,
      callsite: row.callsite
    })));
    if (consoleErrors.length > 0) printTable('Browser console errors', consoleErrors.map((message) => ({ message })));

    const afterP95 = percentile(afterHover.map((row) => row.highlightMs), 0.95);
    const failed = afterP95 > 100 || idleLongTasks.length > 0 || animationCount > 0;
    return failed ? 1 : 0;
  } finally {
    await browser?.close();
    await stopProcessGroup(vite);
    const browserLeftovers = await new Promise<string[]>((resolve) => {
      const ps = spawn('ps', ['-axo', 'pid=,command=']);
      let output = '';
      ps.stdout.on('data', (chunk: Buffer) => { output += chunk.toString(); });
      ps.on('exit', () => resolve(output.split('\n').filter((line) => line.includes(`--${SESSION_TAG}`))));
    });
    const viteLeftovers = await processGroupMembers(vite.pid);
    process.stdout.write(`\nBrowser cleanup: ${browserLeftovers.length === 0 ? `stopped ${SESSION_TAG} (Chrome helper tree exited)` : `not stopped - ${browserLeftovers.join(' | ')}`}\n`);
    process.stdout.write(`Browser cleanup: ${viteLeftovers.length === 0 ? `stopped perfharness20-vite (${PORT} process tree exited)` : `not stopped - ${viteLeftovers.join(' | ')}`}\n`);
  }
}

main().then((code) => { process.exitCode = code; }).catch((error: unknown) => {
  console.error(error instanceof Error ? error.stack : error);
  process.exitCode = 1;
});
