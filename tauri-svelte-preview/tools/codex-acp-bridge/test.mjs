#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { createInterface } from "node:readline";

const directory = path.dirname(fileURLToPath(import.meta.url));
const bridgePath = path.join(directory, "bridge.mjs");
const mockCodexPath = path.join(directory, "mock-codex.mjs");

class AcpPeer {
  constructor(scenario = "normal", permissionChoice = "allow") {
    this.nextId = 1;
    this.pending = new Map();
    this.updates = [];
    this.updateWaiters = [];
    this.permissions = [];
    this.permissionWaiters = [];
    this.permissionChoice = permissionChoice;
    this.stderr = "";
    this.child = spawn(process.execPath, [bridgePath], {
      env: {
        ...process.env,
        CODEX_BIN: mockCodexPath,
        BRIDGE_MOCK_SCENARIO: scenario,
      },
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.child.stderr.on("data", (chunk) => {
      this.stderr += chunk;
    });
    this.exit = new Promise((resolve) => {
      this.child.on("close", (code, signal) => resolve({ code, signal }));
    });
    this.lines = createInterface({ input: this.child.stdout, crlfDelay: Infinity });
    this.reader = this.read();
  }

  send(message) {
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  request(method, params, timeoutMs = 10_000) {
    const id = this.nextId++;
    const response = new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(String(id));
        reject(new Error(`Timed out waiting for ${method}`));
      }, timeoutMs);
      this.pending.set(String(id), {
        resolve: (value) => {
          clearTimeout(timer);
          resolve(value);
        },
        reject: (error) => {
          clearTimeout(timer);
          reject(error);
        },
      });
    });
    this.send({ jsonrpc: "2.0", id, method, params });
    return response;
  }

  notify(method, params) {
    this.send({ jsonrpc: "2.0", method, params });
  }

  waitForUpdate(timeoutMs = 10_000) {
    if (this.updates.length > 0) return Promise.resolve(this.updates[this.updates.length - 1]);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("Timed out waiting for session/update")), timeoutMs);
      this.updateWaiters.push((message) => {
        clearTimeout(timer);
        resolve(message);
      });
    });
  }

  waitForPermission(timeoutMs = 10_000) {
    if (this.permissions.length > 0) {
      return Promise.resolve(this.permissions[this.permissions.length - 1]);
    }
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error("Timed out waiting for session/request_permission")),
        timeoutMs,
      );
      this.permissionWaiters.push((message) => {
        clearTimeout(timer);
        resolve(message);
      });
    });
  }

  respondToPermission(message) {
    if (this.permissionChoice === "defer") return;
    if (this.permissionChoice === "error") {
      this.send({
        jsonrpc: "2.0",
        id: message.id,
        error: { code: -32000, message: "permission UI closed" },
      });
      return;
    }
    if (this.permissionChoice === "cancel") {
      this.send({
        jsonrpc: "2.0",
        id: message.id,
        result: { outcome: { outcome: "cancelled" } },
      });
      return;
    }
    this.send({
      jsonrpc: "2.0",
      id: message.id,
      result: {
        outcome: { outcome: "selected", optionId: this.permissionChoice },
      },
    });
  }

  async read() {
    for await (const line of this.lines) {
      const message = JSON.parse(line);
      if (message.method === "session/update") {
        this.updates.push(message);
        for (const waiter of this.updateWaiters.splice(0)) waiter(message);
        continue;
      }
      if (message.method === "session/request_permission") {
        this.permissions.push(message);
        for (const waiter of this.permissionWaiters.splice(0)) waiter(message);
        this.respondToPermission(message);
        continue;
      }
      if (Object.hasOwn(message, "id")) {
        const pending = this.pending.get(String(message.id));
        if (!pending) continue;
        this.pending.delete(String(message.id));
        if (message.error) pending.reject(Object.assign(new Error(message.error.message), message.error));
        else pending.resolve(message.result);
      }
    }
    for (const pending of this.pending.values()) pending.reject(new Error("Bridge transport closed"));
    this.pending.clear();
  }

  async initialize() {
    return this.request("initialize", { protocolVersion: 1 });
  }

  async newSession() {
    return this.request("session/new", { cwd: directory, mcpServers: [] });
  }

  async close(sessionId) {
    const result = await this.request("session/close", { sessionId });
    const exited = await this.exit;
    await this.reader;
    return { result, exited };
  }
}

async function withSession(scenario, callback, permissionChoice = "allow") {
  const peer = new AcpPeer(scenario, permissionChoice);
  const initialized = await peer.initialize();
  assert.equal(initialized.protocolVersion, 1);
  const session = await peer.newSession();
  try {
    await callback(peer, session);
  } finally {
    if (peer.child.exitCode === null && peer.child.signalCode === null) {
      const closed = await peer.close(session.sessionId);
      assert.deepEqual(closed.result, {});
      assert.equal(closed.exited.code, 0, peer.stderr);
    }
  }
}

function updatesOf(peer, kind) {
  return peer.updates
    .map((message) => message.params.update)
    .filter((update) => update.sessionUpdate === kind);
}

const tests = [];

tests.push([
  "protocol v1 validation, memoized child initialization, and real child version",
  async () => {
    const peer = new AcpPeer("normal");
    await assert.rejects(
      peer.request("initialize", { protocolVersion: 2 }),
      (error) => error.code === -32602,
    );
    const [first, second] = await Promise.all([peer.initialize(), peer.initialize()]);
    assert.equal(first.protocolVersion, 1);
    assert.equal(first.agentInfo.version, "9.8.7");
    assert.deepEqual(second, first);
    assert.equal(first.agentCapabilities.plans, true);
    const session = await peer.newSession();
    const closed = await peer.close(session.sessionId);
    assert.equal(closed.exited.code, 0, peer.stderr);
  },
]);

tests.push([
  "handshake, text delta, model, unknown ACP method, and clean shutdown",
  async () => {
    await withSession("normal", async (peer, session) => {
      assert.equal(session.model, "gpt-5.6-sol");
      await assert.rejects(
        peer.request("bridge/unknown", {}),
        (error) => error.code === -32601,
      );
      const prompt = await peer.request("session/prompt", {
        sessionId: session.sessionId,
        prompt: [{ type: "text", text: "Reply with exactly: pong" }],
      });
      assert.equal(prompt.stopReason, "end_turn");
      assert.equal(prompt.model, "gpt-5.6-sol");
      assert.equal(
        updatesOf(peer, "agent_message_chunk").map((update) => update.content.text).join(""),
        "pong",
      );
      assert.deepEqual(
        await peer.request("session/set_config_option", {
          sessionId: session.sessionId,
          configId: "model",
          value: "ignored",
        }),
        {},
      );
    });
  },
]);

tests.push([
  "session/load and session/resume preserve id and exclude ignored turns",
  async () => {
    const peer = new AcpPeer("normal");
    await peer.initialize();
    const loaded = await peer.request("session/load", {
      sessionId: "persisted-thread",
      cwd: directory,
    });
    assert.equal(loaded.sessionId, "persisted-thread");
    assert.equal(loaded.model, "gpt-5.6-sol");
    const resumed = await peer.request("session/resume", {
      sessionId: "persisted-thread",
      cwd: directory,
    });
    assert.equal(resumed.sessionId, "persisted-thread");
    const closed = await peer.close(resumed.sessionId);
    assert.equal(closed.exited.code, 0, peer.stderr);
  },
]);

tests.push([
  "approval allow, reject, cancel, error, and rich permission metadata",
  async () => {
    const cases = [
      {
        prompt: "V2_COMMAND",
        title: "echo hi",
        description: "mock approval\n/mock/work",
        kind: "command",
        idPrefix: "item-",
      },
      {
        prompt: "V2_FILE",
        title: "Apply file changes",
        description: "mock approval\n/mock/grant",
        kind: "file_change",
        idPrefix: "item-",
      },
      {
        prompt: "LEGACY_EXEC",
        title: "echo hi",
        description: "mock approval\n/mock/work",
        kind: "command",
        idPrefix: "call-",
      },
      {
        prompt: "LEGACY_PATCH",
        title: "Apply file changes",
        description: "mock approval\n/mock/file.txt",
        kind: "file_change",
        idPrefix: "call-",
      },
    ];
    for (const permissionChoice of ["allow", "reject", "cancel", "error"]) {
      for (const approvalCase of cases) {
        const suffix =
          permissionChoice === "allow"
            ? ""
            : permissionChoice === "reject"
              ? " REJECT"
              : " CANCEL";
        await withSession(
          "approval",
          async (peer, session) => {
            const result = await peer.request("session/prompt", {
              sessionId: session.sessionId,
              prompt: [{ type: "text", text: `${approvalCase.prompt}${suffix}` }],
            });
            assert.equal(result.stopReason, "end_turn");
            assert.equal(peer.permissions.length, 1);
            const permission = peer.permissions[0].params;
            assert.equal(permission.title, approvalCase.title);
            assert.equal(permission.description, approvalCase.description);
            assert.equal(permission.toolCall.title, approvalCase.title);
            assert.equal(permission.toolCall.kind, approvalCase.kind);
            assert.match(permission.toolCall.toolCallId, new RegExp(`^${approvalCase.idPrefix}`));
            assert.deepEqual(permission.options, [
              { optionId: "allow", name: "Allow", kind: "allow_once" },
              { optionId: "reject", name: "Reject", kind: "reject_once" },
            ]);
          },
          permissionChoice,
        );
      }
    }
  },
]);

tests.push([
  "session/cancel drains current and legacy approvals with cancel outcomes",
  async () => {
    for (const promptText of ["V2_COMMAND CANCEL", "LEGACY_PATCH CANCEL"]) {
      await withSession(
        "approval",
        async (peer, session) => {
          const prompt = peer.request("session/prompt", {
            sessionId: session.sessionId,
            prompt: [{ type: "text", text: promptText }],
          });
          await peer.waitForPermission();
          peer.notify("session/cancel", { sessionId: session.sessionId });
          assert.equal((await prompt).stopReason, "cancelled");
        },
        "defer",
      );
    }
  },
]);

tests.push([
  "plan, reasoning, four tool kinds, output, progress, patch, and turn diff mappings",
  async () => {
    await withSession("rich", async (peer, session) => {
      const runRichPrompt = () =>
        peer.request("session/prompt", {
          sessionId: session.sessionId,
          prompt: [{ type: "text", text: "rich" }],
        });

      assert.equal((await runRichPrompt()).stopReason, "end_turn");
      const plans = updatesOf(peer, "plan");
      assert.deepEqual(
        plans.filter((update) => update.planId === "plan-delta").map((update) => update.entries[0]),
        [
          { content: "Inspect", status: "in_progress" },
          { content: "Inspect files", status: "in_progress" },
        ],
      );
      assert.deepEqual(
        plans.find((update) => update.planId.startsWith("plan-mock-turn-")).entries,
        [
          { content: "Build bridge", status: "in_progress" },
          { content: "Verify bridge", status: "pending" },
        ],
      );
      assert.deepEqual(
        plans
          .filter((update) => update.planId === "plan-lifecycle")
          .map((update) => update.entries[0].status),
        ["in_progress", "completed"],
      );

      assert.deepEqual(
        updatesOf(peer, "agent_thought_chunk").map((update) => update.content.text),
        ["Thinking", " carefully", "Fallback thought"],
      );

      const starts = updatesOf(peer, "tool_call");
      const toolIds = ["tool-command", "tool-file", "tool-mcp", "tool-dynamic"];
      for (const toolId of toolIds) {
        assert.equal(starts.filter((update) => update.toolCallId === toolId).length, 1);
      }
      assert.deepEqual(
        toolIds.map((toolId) => starts.find((update) => update.toolCallId === toolId).kind),
        ["command", "file_change", "mcp", "dynamic_tool"],
      );

      const toolUpdates = updatesOf(peer, "tool_call_update");
      for (const toolId of toolIds) {
        assert.equal(
          toolUpdates.some((update) => update.toolCallId === toolId && update.status === "completed"),
          true,
        );
      }
      assert.equal(
        toolUpdates.some(
          (update) =>
            update.toolCallId === "tool-command" &&
            update.content?.some((content) => content.text === "hello\n"),
        ),
        true,
      );
      assert.equal(
        toolUpdates.some(
          (update) =>
            update.toolCallId === "tool-command" &&
            update.content?.some((content) => content.text === "stdin: yes"),
        ),
        true,
      );
      assert.equal(
        toolUpdates.some(
          (update) =>
            update.toolCallId === "tool-file" &&
            update.locations?.some((location) => location.path === "src/app.js"),
        ),
        true,
      );
      assert.equal(
        toolUpdates.some(
          (update) =>
            update.toolCallId === "tool-mcp" &&
            update.content?.some((content) => content.text === "halfway"),
        ),
        true,
      );
      assert.equal(
        toolUpdates.some(
          (update) =>
            update.toolCallId === "tool-dynamic" &&
            update.content?.some((content) => content.text.includes("dynamic result")),
        ),
        true,
      );
      assert.equal(
        starts.some((update) => update.toolCallId === "diff-mock-turn-1"),
        true,
      );
      assert.equal(
        toolUpdates.some(
          (update) =>
            update.toolCallId === "diff-mock-turn-1" &&
            update.content?.some((content) => content.text.startsWith("diff --git")),
        ),
        true,
      );
      assert.equal(
        toolUpdates.some(
          (update) => update.toolCallId === "diff-mock-turn-1" && update.status === "completed",
        ),
        true,
      );

      assert.equal((await runRichPrompt()).stopReason, "end_turn");
      assert.equal(
        updatesOf(peer, "tool_call").filter((update) => update.toolCallId === "tool-command").length,
        2,
      );
    });
  },
]);

tests.push([
  "cancel racing turn completion resolves the pending prompt once as cancelled",
  async () => {
    await withSession("cancel", async (peer, session) => {
      const prompt = peer.request("session/prompt", {
        sessionId: session.sessionId,
        prompt: [{ type: "text", text: "wait" }],
      });
      await peer.waitForUpdate();
      peer.notify("session/cancel", { sessionId: session.sessionId });
      const result = await prompt;
      assert.equal(result.stopReason, "cancelled");
      assert.equal(result.turnId, "mock-turn-1");
    });
  },
]);

tests.push([
  "session/steer uses the active app-server turn",
  async () => {
    await withSession("steer", async (peer, session) => {
      const prompt = peer.request("session/prompt", {
        sessionId: session.sessionId,
        prompt: [{ type: "text", text: "wait" }],
      });
      await peer.waitForUpdate();
      assert.deepEqual(
        await peer.request("session/steer", {
          sessionId: session.sessionId,
          prompt: [{ type: "text", text: "continue" }],
        }),
        {},
      );
      assert.equal((await prompt).stopReason, "end_turn");
    });
  },
]);

tests.push([
  "child crash mid-turn fails the prompt and exits the bridge with code 1",
  async () => {
    const peer = new AcpPeer("crash");
    await peer.initialize();
    const session = await peer.newSession();
    const prompt = peer.request("session/prompt", {
      sessionId: session.sessionId,
      prompt: [{ type: "text", text: "crash" }],
    });
    await assert.rejects(prompt, /app-server (?:exited|stdout closed)|transport closed/i);
    const exited = await peer.exit;
    assert.equal(exited.code, 1, peer.stderr);
    await peer.reader;
  },
]);

tests.push([
  "app-server stdout EOF while child stays alive is fatal and the child is reaped",
  async () => {
    const peer = new AcpPeer("stdout-eof");
    await peer.initialize();
    const session = await peer.newSession();
    const prompt = peer.request("session/prompt", {
      sessionId: session.sessionId,
      prompt: [{ type: "text", text: "close stdout" }],
    });
    await assert.rejects(prompt, /stdout closed unexpectedly|transport closed/i);
    const exited = await peer.exit;
    assert.equal(exited.code, 1, peer.stderr);
    assert.equal(exited.signal, null);
    await peer.reader;
  },
]);

tests.push([
  "huge assistant output survives downstream backpressure without data loss",
  async () => {
    await withSession("huge", async (peer, session) => {
      const prompt = peer.request(
        "session/prompt",
        {
          sessionId: session.sessionId,
          prompt: [{ type: "text", text: "huge" }],
        },
        30_000,
      );
      await peer.waitForUpdate();
      peer.child.stdout.pause();
      await new Promise((resolve) => setTimeout(resolve, 100));
      assert.equal(peer.child.exitCode, null);
      peer.child.stdout.resume();
      assert.equal((await prompt).stopReason, "end_turn");
      const textLength = updatesOf(peer, "agent_message_chunk").reduce(
        (total, update) => total + update.content.text.length,
        0,
      );
      assert.equal(textLength, 5 + 512 * 16 * 1024);
    });
  },
]);

let failures = 0;
for (const [name, run] of tests) {
  try {
    await run();
    console.log(`PASS ${name}`);
  } catch (error) {
    failures += 1;
    console.error(`FAIL ${name}`);
    console.error(error.stack || error);
  }
}

if (failures > 0) {
  console.error(`STATUS FAIL (${failures}/${tests.length})`);
  process.exit(1);
}
console.log(`STATUS PASS (${tests.length}/${tests.length})`);
