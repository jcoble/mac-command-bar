#!/usr/bin/env node
// TSK-1463 prototype. Runs the stock Claude ACP adapter as a library and adds
// what Assembly's pnpm patch added, from outside the adapter:
//   A. session/prompt answers at the turn's result, also while background
//      subagents still run (the foreground hook reports that result).
//   B. session_info_update with _meta.jetbrains.air.sessionState.
//   C. session_info_update with _meta.jetbrains.air.backgroundTaskCount, from
//      Claude Code's background_tasks_changed (raw SDK messages, opted in per
//      session through the documented `emitRawSDKMessages` session meta).
//   D. _meta.jetbrains.air.outOfTurn on root updates while no prompt is open.
//   E. session/cancel also stops the running background tasks.
import { createRequire } from 'node:module';
import { ClaudeAcpAgent, nodeToWebReadable, nodeToWebWritable } from '@agentclientprotocol/claude-agent-acp';
import { runPromptWithCancellation } from '@agentclientprotocol/claude-agent-acp/dist/acp-agent.js';
import { withAirMeta } from '@agentclientprotocol/claude-agent-acp/dist/air-extension.js';
import { GOAL_CONTROL_METHOD, parseGoalRequest } from '@agentclientprotocol/claude-agent-acp/dist/goal-extension.js';
import { applyManagedPolicyEnv } from '@agentclientprotocol/claude-agent-acp/dist/managed-policy.js';
import { onSessionIndexRequests } from '@agentclientprotocol/claude-agent-acp/dist/session-index/connection.js';

// The ACP SDK copy that the adapter itself uses.
const adapterRequire = createRequire(createRequire(import.meta.url).resolve('@agentclientprotocol/claude-agent-acp/package.json'));
const { agent: acpAgent, methods, ndJsonStream } = await import(adapterRequire.resolve('@agentclientprotocol/sdk'));

await applyManagedPolicyEnv();
// stdout carries ACP; everything else goes to stderr.
console.log = console.info = console.warn = console.debug = console.error;
process.on('unhandledRejection', (reason) => console.error('Unhandled Rejection:', reason));

const RAW_TASKS = [{ type: 'system', subtype: 'background_tasks_changed' }];
const SESSION_LEVEL = new Set(['available_commands_update', 'current_mode_update', 'config_option_update', 'session_info_update']);
const openPrompts = new Map(); // sessionId -> listener for the end of the prompt's work
const backgroundTasks = new Map(); // sessionId -> Claude Code's latest background task list
let agent;
let client;

class Client {
  constructor(ctx) { this.ctx = ctx; }
  info(sessionId, key, value) {
    return this.ctx.notify(methods.client.session.update, {
      sessionId,
      update: { sessionUpdate: 'session_info_update', _meta: withAirMeta(undefined, key, value) }
    });
  }
  sessionUpdate(n) {
    if (agent.sessions[n.sessionId] && !openPrompts.has(n.sessionId) && !SESSION_LEVEL.has(n.update.sessionUpdate)) {
      n = { ...n, update: { ...n.update, _meta: withAirMeta(n.update._meta, 'outOfTurn', true) } };
    }
    return this.ctx.notify(methods.client.session.update, n);
  }
  requestPermission(p, signal) { return this.ctx.request(methods.client.session.requestPermission, p, { cancellationSignal: signal }); }
  readTextFile(p) { return this.ctx.request(methods.client.fs.readTextFile, p); }
  writeTextFile(p) { return this.ctx.request(methods.client.fs.writeTextFile, p); }
  createElicitation(p, signal) { return this.ctx.request(methods.client.elicitation.create, p, { cancellationSignal: signal }); }
  completeElicitation(p) { return this.ctx.notify(methods.client.elicitation.complete, p); }
  extNotification(method, params) {
    if (method === '_claude/sdkMessage' && params.message?.subtype === 'background_tasks_changed') {
      backgroundTasks.set(params.sessionId, params.message.tasks);
      return this.info(params.sessionId, 'backgroundTaskCount', params.message.tasks.length);
    }
    return this.ctx.notify(method, params);
  }
}

const foreground = (sessionId) => ({
  running: () => client.info(sessionId, 'sessionState', 'running'),
  awaitingUser: () => client.info(sessionId, 'sessionState', 'requires_action'),
  ended: (outcome) => {
    client.info(sessionId, 'sessionState', 'idle');
    openPrompts.get(sessionId)?.(outcome);
  },
  failed: () => client.info(sessionId, 'sessionState', 'idle')
});

class LauncherAgent extends ClaudeAcpAgent {
  prompt(params) {
    const { sessionId } = params;
    let mine;
    return new Promise((resolve, reject) => {
      let inserted = false;
      mine = (outcome) => inserted && resolve(outcome);
      openPrompts.set(sessionId, mine);
      this.startTurn(params, { inserted: () => { inserted = true; }, ended: resolve, failed: reject }).catch(reject);
    }).finally(() => openPrompts.get(sessionId) === mine && openPrompts.delete(sessionId));
  }
  async cancel(params) {
    const session = this.sessions[params.sessionId];
    await Promise.all([
      super.cancel(params),
      ...(backgroundTasks.get(params.sessionId) ?? []).map((task) => session.query.stopTask(task.task_id))
    ]);
  }
}

const withRawTasks = (p) => ({ ...p, _meta: { ...p._meta, claudeCode: { ...p._meta?.claudeCode, emitRawSDKMessages: RAW_TASKS } } });
const passThrough = { parse: (p) => p };
const app = acpAgent({ name: 'claude-code-acp' })
  .onConnect((connection) => {
    client = new Client(connection.client);
    agent = new LauncherAgent(client, undefined, { foreground });
  })
  .onRequest(methods.agent.initialize, (ctx) => agent.initialize(ctx.params))
  .onRequest(methods.agent.session.new, (ctx) => agent.newSession(withRawTasks(ctx.params)))
  .onRequest(methods.agent.session.load, (ctx) => agent.loadSession(withRawTasks(ctx.params)))
  .onRequest(methods.agent.session.fork, (ctx) => agent.unstable_forkSession(withRawTasks(ctx.params)))
  .onRequest(methods.agent.session.list, (ctx) => agent.listSessions(ctx.params))
  .onRequest(methods.agent.session.delete, (ctx) => agent.deleteSession(ctx.params))
  .onRequest(methods.agent.session.resume, (ctx) => agent.resumeSession(withRawTasks(ctx.params)))
  .onRequest(methods.agent.session.close, (ctx) => agent.closeSession(ctx.params))
  .onRequest(methods.agent.session.setMode, (ctx) => agent.setSessionMode(ctx.params))
  .onRequest(methods.agent.session.setConfigOption, (ctx) => agent.setSessionConfigOption(ctx.params))
  .onRequest(methods.agent.authenticate, (ctx) => agent.authenticate(ctx.params))
  .onRequest(methods.agent.providers.list, (ctx) => agent.unstable_listProviders(ctx.params))
  .onRequest(methods.agent.providers.set, (ctx) => agent.unstable_setProvider(ctx.params))
  .onRequest(methods.agent.providers.disable, (ctx) => agent.unstable_disableProvider(ctx.params))
  .onRequest(methods.agent.logout, (ctx) => agent.logout(ctx.params))
  .onRequest(methods.agent.session.prompt, (ctx) => runPromptWithCancellation(agent, ctx.params, ctx.signal))
  .onNotification(methods.agent.session.cancel, (ctx) => agent.cancel(ctx.params))
  // The adapter's own parsers for these two are not exported.
  .onRequest('_session/steering', passThrough, (ctx) => agent.steer(ctx.params))
  .onRequest('_session/async_task/stop', passThrough, (ctx) => agent.stopAsyncTask(ctx.params))
  .onRequest(GOAL_CONTROL_METHOD, { parse: parseGoalRequest }, (ctx) => agent.goal(ctx.params));

const connection = onSessionIndexRequests(app, () => agent)
  .connect(ndJsonStream(nodeToWebWritable(process.stdout), nodeToWebReadable(process.stdin)));
const shutdown = async () => {
  await agent?.dispose().catch((error) => console.error('Error during cleanup:', error));
  process.exit(0);
};
connection.closed.then(shutdown);
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
process.stdin.resume();
