// Tests for the API-model sidecar. No network: TanStack's fake adapter stands in for the model, a local
// MCP server stands in for Assembly's endpoint, and a local HTTP server stands in for Ollama.
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { mkdtemp, readFile } from 'node:fs/promises'
import { createServer, type Server } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { after, test } from 'node:test'
import { fileURLToPath } from 'node:url'
import * as acp from '@agentclientprotocol/sdk'
import { toolDefinition } from '@tanstack/ai'
import { createMCPServer } from '@tanstack/ai-mcp/server'
import { fakeText, type FakeResponseStep } from '@tanstack/ai/testing'
import { serve, type Endpoint } from './agent.ts'

const ENDPOINTS: Array<Endpoint> = [{
  adapter: 'ollama',
  base_url: 'http://127.0.0.1:1',
  models: [
    { id: 'small', tools: true, images: false, judge: false, context_window: 4096 },
    { id: 'plain', tools: false, images: false, judge: false, context_window: 8192 },
  ],
}]

const servers: Array<Server> = []
after(() => servers.forEach((server) => { server.closeAllConnections(); server.close() }))

async function listen(server: Server): Promise<string> {
  servers.push(server)
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  const address = server.address()
  return `http://127.0.0.1:${typeof address === 'object' && address ? address.port : 0}`
}

// Serves a Fetch-style handler over node:http, so it runs the same on Node and Deno.
const fetchServer = (handler: (request: Request) => Promise<Response>) => createServer(async (req, res) => {
  const chunks: Array<Buffer> = []
  for await (const chunk of req) chunks.push(chunk as Buffer)
  const body = ['GET', 'HEAD'].includes(req.method ?? 'GET') ? undefined : Buffer.concat(chunks)
  const headers = new Headers(Object.entries(req.headers).flatMap(([k, v]) => (v === undefined ? [] : [[k, String(v)]])))
  const response = await handler(new Request(`http://127.0.0.1${req.url}`, { method: req.method, headers, body }))
  res.writeHead(response.status, Object.fromEntries(response.headers))
  res.end(Buffer.from(await response.arrayBuffer()))
})

async function assemblyMcp() {
  const calls: Array<unknown> = []
  const echo = toolDefinition({
    name: 'echo',
    description: 'Echo the text back.',
    inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] },
  }).server(async (args: unknown) => { calls.push(args); return { echoed: (args as { text: string }).text } })
  const mcp = createMCPServer({ name: 'assembly', version: '1.0.0', tools: [echo] })
  const url = `${await listen(fetchServer((request) => mcp.fetch(request)))}/mcp`
  return { calls, config: { type: 'http' as const, name: 'assembly', url, headers: [{ name: 'Authorization', value: 'Bearer t' }] } }
}

async function start(responses: Array<FakeResponseStep>, options: { permission?: 'allow' | 'reject', dataDir?: string, tokensPerSecond?: number } = {}) {
  const fake = fakeText({ tokensPerSecond: options.tokensPerSecond })
  fake.setResponses(responses)
  const dataDir = options.dataDir ?? await mkdtemp(join(tmpdir(), 'models-acp-'))
  const toAgent = new TransformStream<Uint8Array, Uint8Array>()
  const toClient = new TransformStream<Uint8Array, Uint8Array>()
  serve(acp.ndJsonStream(toClient.writable, toAgent.readable), { endpoints: ENDPOINTS, dataDir, adapterFor: () => fake })
  const updates: Array<acp.SessionUpdate> = []
  const permissions: Array<acp.RequestPermissionRequest> = []
  const client = new acp.ClientSideConnection(() => ({
    async requestPermission(params) {
      permissions.push(params)
      return { outcome: { outcome: 'selected', optionId: options.permission ?? 'allow' } }
    },
    async sessionUpdate(params) { updates.push(params.update) },
  }), acp.ndJsonStream(toAgent.writable, toClient.readable))
  const init = await client.initialize({ protocolVersion: acp.PROTOCOL_VERSION, clientCapabilities: {} })
  return { client, init, fake, dataDir, updates, permissions }
}

const say = (sessionId: string, text: string) => ({ sessionId, prompt: [{ type: 'text' as const, text }] })

test('a denied tool call returns the refusal as the tool result and the turn continues', async () => {
  const mcp = await assemblyMcp()
  let toolResult = ''
  const t = await start([
    { toolCalls: [{ name: 'echo', input: { text: 'hi' } }] },
    ({ request }) => { toolResult = JSON.stringify(request.messages.at(-1)); return { text: 'understood' } },
  ], { permission: 'reject' })
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [mcp.config] })
  const result = await t.client.prompt(say(sessionId, 'use echo'))
  assert.equal(result.stopReason, 'end_turn')
  assert.match(toolResult, /The user refused this tool call/)
  assert.equal(mcp.calls.length, 0)
  const reply = t.updates.map((u) => (u.sessionUpdate === 'agent_message_chunk' && u.content.type === 'text' ? u.content.text : '')).join('')
  assert.equal(reply, 'understood')
})

test('an allowed tool call runs once', async () => {
  const mcp = await assemblyMcp()
  const t = await start([{ toolCalls: [{ name: 'echo', input: { text: 'hi' } }] }, { text: 'done' }])
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [mcp.config] })
  await t.client.prompt(say(sessionId, 'use echo'))
  assert.deepEqual(mcp.calls, [{ text: 'hi' }])
})

test('the permission request carries the assembly MCP meta', async () => {
  const mcp = await assemblyMcp()
  const t = await start([{ toolCalls: [{ name: 'echo', input: { text: 'hi' } }] }, { text: 'done' }])
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [mcp.config] })
  await t.client.prompt(say(sessionId, 'use echo'))
  assert.equal(t.permissions.length, 1)
  assert.deepEqual(t.permissions[0].toolCall._meta, { is_mcp_tool_call: true, mcp: { server: 'assembly' } })
})

test('a cancelled turn keeps its partial reply in history', async () => {
  const t = await start([{ text: 'one two three four five six seven eight nine ten '.repeat(20) }], { tokensPerSecond: 20 })
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [] })
  const turn = t.client.prompt(say(sessionId, 'count'))
  while (!t.updates.some((u) => u.sessionUpdate === 'agent_message_chunk')) await new Promise((r) => setTimeout(r, 10))
  await t.client.cancel({ sessionId })
  assert.equal((await turn).stopReason, 'cancelled')
  const saved = JSON.parse(await readFile(join(t.dataDir, `${sessionId}.json`), 'utf8'))
  assert.deepEqual(saved.messages.map((m: { role: string }) => m.role), ['user', 'assistant'])
  assert.match(saved.messages[1].content, /^one/)
})

test('cancel stops the model request', async () => {
  let signal: AbortSignal | undefined
  const t = await start([({ request }) => {
    signal = request.abortController?.signal ?? request.request?.signal ?? undefined
    return { text: 'word '.repeat(200) }
  }], { tokensPerSecond: 20 })
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [] })
  const turn = t.client.prompt(say(sessionId, 'talk'))
  while (!t.updates.some((u) => u.sessionUpdate === 'agent_message_chunk')) await new Promise((r) => setTimeout(r, 10))
  await t.client.cancel({ sessionId })
  await turn
  assert.equal(signal?.aborted, true)
})

test('resume restores history and the next turn sees earlier turns', async () => {
  const first = await start([{ text: 'The answer is 42.' }])
  const { sessionId } = await first.client.newSession({ cwd: '/tmp', mcpServers: [] })
  await first.client.setSessionConfigOption({ sessionId, configId: 'model', value: 'ollama/plain' })
  await first.client.prompt(say(sessionId, 'remember 42'))
  let seen = ''
  const second = await start([({ request }) => { seen = JSON.stringify(request.messages); return { text: '42' } }], { dataDir: first.dataDir })
  assert.equal(second.init.agentCapabilities?.sessionCapabilities?.resume !== undefined, true)
  assert.equal(second.init.agentCapabilities?.loadSession, false)
  const resumed = await second.client.resumeSession({ sessionId, cwd: '/tmp', mcpServers: [] })
  assert.equal(resumed.configOptions?.[0].currentValue, 'ollama/plain')
  await second.client.prompt(say(sessionId, 'what was it?'))
  assert.match(seen, /remember 42/)
  assert.match(seen, /The answer is 42\./)
})

test('resume of an unknown session is an ACP error, not a crash', async () => {
  const t = await start([{ text: 'still here' }])
  await assert.rejects(t.client.resumeSession({ sessionId: 'no-such-session', cwd: '/tmp', mcpServers: [] }), /Resource not found|unknown session/i)
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [] })
  assert.equal((await t.client.prompt(say(sessionId, 'hi'))).stopReason, 'end_turn')
})

test('usage reports the configured context window', async () => {
  const t = await start([{ text: 'hello there' }])
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [] })
  const result = await t.client.prompt(say(sessionId, 'hi'))
  const usage = t.updates.find((u) => u.sessionUpdate === 'usage_update')
  assert.equal(usage?.sessionUpdate === 'usage_update' && usage.size, 4096)
  assert.ok((result.usage?.totalTokens ?? 0) > 0)
})

test('a model with tools false is offered no tools', async () => {
  const mcp = await assemblyMcp()
  const offered: Array<number> = []
  const step: FakeResponseStep = ({ request }) => { offered.push(request.tools?.length ?? 0); return { text: 'ok' } }
  const t = await start([step, step])
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [mcp.config] })
  await t.client.prompt(say(sessionId, 'with tools'))
  await t.client.setSessionConfigOption({ sessionId, configId: 'model', value: 'ollama/plain' })
  await t.client.prompt(say(sessionId, 'without tools'))
  assert.deepEqual(offered, [1, 0])
})

test('an unknown model id in set_config_option is refused', async () => {
  const t = await start([])
  const { sessionId } = await t.client.newSession({ cwd: '/tmp', mcpServers: [] })
  await assert.rejects(t.client.setSessionConfigOption({ sessionId, configId: 'model', value: 'ollama/nope' }))
})

// The real sidecar process, with the model side served by a fake Ollama that breaks mid-stream.
async function runSidecar(endpoints: Array<Endpoint>, model: string) {
  const agent = fileURLToPath(new URL('./agent.ts', import.meta.url))
  const args = 'Deno' in globalThis ? ['run', '-A', agent] : ['--experimental-strip-types', '--no-warnings', agent]
  const child = spawn(process.execPath, args, {
    env: { ...process.env, ASSEMBLY_MODELS_ENDPOINTS: JSON.stringify(endpoints), ASSEMBLY_MODELS_DATA: await mkdtemp(join(tmpdir(), 'models-acp-')) },
  })
  let stdout = ''
  let stderr = ''
  child.stdout.on('data', (d) => { stdout += d })
  child.stderr.on('data', (d) => { stderr += d })
  const send = (id: number, method: string, params: unknown) => child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`)
  const reply = async (id: number) => {
    for (let i = 0; i < 1000; i++) {
      const line = stdout.split('\n').find((l) => l.includes(`"id":${id},`) || l.endsWith(`"id":${id}}`))
      if (line) return JSON.parse(line)
      await new Promise((r) => setTimeout(r, 20))
    }
    throw new Error(`no reply ${id}; stderr: ${stderr}`)
  }
  send(1, 'initialize', { protocolVersion: 1, clientCapabilities: {} })
  await reply(1)
  send(2, 'session/new', { cwd: '/tmp', mcpServers: [] })
  const { sessionId } = (await reply(2)).result
  send(3, 'session/set_config_option', { sessionId, configId: 'model', value: model })
  await reply(3)
  send(4, 'session/prompt', { sessionId, prompt: [{ type: 'text', text: 'hi' }] })
  const result = await reply(4)
  child.kill()
  return { result, stdout, stderr }
}

test('stdout carries only JSON-RPC when the model errors', async () => {
  // A fake Ollama that sends half a line, then drops the connection.
  const url = await listen(createServer((_req, res) => {
    res.writeHead(200, { 'content-type': 'application/x-ndjson' })
    res.write('{"model":"small","message":{"role":"assistant","content":"par')
    setTimeout(() => res.destroy(), 50)
  }))
  const endpoints = [{ ...ENDPOINTS[0], base_url: url }]
  const { result, stdout } = await runSidecar(endpoints, 'ollama/small')
  assert.ok(result.error, 'the turn fails')
  for (const line of stdout.split('\n').filter(Boolean)) assert.equal(JSON.parse(line).jsonrpc, '2.0', line)
})

test('an endpoint with a missing key fails the turn with "no key for <endpoint>" and never prints a key', async () => {
  const endpoints: Array<Endpoint> = [
    { adapter: 'openai', base_url: 'https://api.openai.com/v1', secret: 'openai', models: [{ id: 'gpt-5', tools: true, images: true, judge: false, context_window: 400000 }] },
    { adapter: 'grok', base_url: 'https://api.x.ai/v1', secret: 'xai', key: 'xai-KEY-MUST-NOT-PRINT', models: [{ id: 'grok-4', tools: true, images: true, judge: false, context_window: 256000 }] },
  ]
  const { result, stdout, stderr } = await runSidecar(endpoints, 'openai/gpt-5')
  assert.match(JSON.stringify(result.error), /no key for openai/)
  assert.doesNotMatch(stdout + stderr, /xai-KEY-MUST-NOT-PRINT/)
})

// Waits for the owner's API key (Build Board Q17): a real key-based model and an image prompt.
test('a key-based model answers one turn', { skip: 'needs an API key from the owner (Build Board Q17)' }, () => {})
test('an image prompt reaches a model with images true', { skip: 'needs an API key from the owner (Build Board Q17)' }, () => {})
