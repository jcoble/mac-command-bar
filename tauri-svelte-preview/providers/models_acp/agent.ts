// assembly-models-acp: an ACP v1 agent whose loop is TanStack AI's chat(), so API models (Ollama, OpenAI,
// Anthropic, Gemini, Grok, OpenRouter, any OpenAI-compatible endpoint) run as an Assembly provider.
// Rust launches it with the endpoint list (keys filled in) in ASSEMBLY_MODELS_ENDPOINTS and a history folder
// in ASSEMBLY_MODELS_DATA. stdout carries only JSON-RPC; every log goes to stderr.
import { Console } from 'node:console'
import { randomUUID } from 'node:crypto'
import { Readable, Writable } from 'node:stream'
import { pathToFileURL } from 'node:url'
import * as acp from '@agentclientprotocol/sdk'
import { chat, type AnyTextAdapter, type ChatMiddleware, type ModelMessage, type Tool } from '@tanstack/ai'
import { createAnthropicChat } from '@tanstack/ai-anthropic'
import { createGeminiChat } from '@tanstack/ai-gemini'
import { createGrokText } from '@tanstack/ai-grok'
import { createMCPClient } from '@tanstack/ai-mcp'
import { createOllamaChat } from '@tanstack/ai-ollama'
import { createOpenaiChat, createOpenaiChatCompletions } from '@tanstack/ai-openai'
import { createOpenRouterText } from '@tanstack/ai-openrouter'
import { loadHistory, saveHistory } from './history.ts'

export type Model = { id: string, tools: boolean, images: boolean, judge: boolean, context_window: number, rates?: unknown }
export type Endpoint = { adapter: string, base_url: string, secret?: string, key?: string, models: Array<Model> }
type Choice = { value: string, endpoint: Endpoint, model: Model }
type AdapterFor = (endpoint: Endpoint, model: Model) => AnyTextAdapter

// Model names are checked by the endpoint, not by TanStack's model lists, hence `as any`.
const ADAPTERS: Record<string, (model: string, key: string, baseURL: string) => AnyTextAdapter> = {
  ollama: (model, _key, baseURL) => createOllamaChat(model, { baseURL }),
  openai: (model, key, baseURL) => createOpenaiChat(model as any, key, { baseURL }),
  'openai-compatible': (model, key, baseURL) => createOpenaiChatCompletions(model as any, key, { baseURL }),
  anthropic: (model, key, baseURL) => createAnthropicChat(model as any, key, { baseURL }),
  gemini: (model, key, baseURL) => createGeminiChat(model as any, key, { baseURL }),
  grok: (model, key, baseURL) => createGrokText(model as any, key, { baseURL }),
  openrouter: (model, key, baseURL) => createOpenRouterText(model as any, key, { serverURL: baseURL }),
}

function adapterFor(endpoint: Endpoint, model: Model): AnyTextAdapter {
  const make = ADAPTERS[endpoint.adapter]
  if (!make) throw acp.RequestError.invalidParams(undefined, `unknown adapter ${endpoint.adapter}`)
  return make(model.id, endpoint.key ?? '', endpoint.base_url)
}

type Session = {
  id: string
  choice: Choice
  messages: Array<ModelMessage>
  tools: Array<Tool>
  toolServer: Map<string, string>
  closers: Array<() => Promise<void>>
  abort?: AbortController
}

const REFUSED = 'The user refused this tool call.'
const text = (t: string) => ({ type: 'text' as const, text: t })

export function serve(stream: acp.Stream, config: { endpoints: Array<Endpoint>, dataDir: string, adapterFor?: AdapterFor }) {
  const makeAdapter = config.adapterFor ?? adapterFor
  const choices: Array<Choice> = config.endpoints.flatMap((endpoint) =>
    endpoint.models.map((model) => ({ value: `${endpoint.adapter}/${model.id}`, endpoint, model })))
  const sessions = new Map<string, Session>()

  const configOptions = (s: Session) => [{
    id: 'model', name: 'Model', category: 'model' as const, type: 'select' as const, currentValue: s.choice.value,
    options: choices.map((c) => ({ value: c.value, name: c.value })),
  }]

  const session = (id: string) => {
    const s = sessions.get(id)
    if (!s) throw acp.RequestError.invalidParams(undefined, `unknown session ${id}`)
    return s
  }

  async function open(id: string, choice: Choice, messages: Array<ModelMessage>, servers: Array<acp.McpServer>) {
    const s: Session = { id, choice, messages, tools: [], toolServer: new Map(), closers: [] }
    for (const server of servers) {
      if (!('type' in server) || server.type !== 'http') continue
      try {
        const headers = Object.fromEntries(server.headers.map((h) => [h.name, h.value]))
        const client = await createMCPClient({ transport: { type: 'http', url: server.url, headers } })
        for (const tool of await client.tools()) {
          s.tools.push(tool)
          s.toolServer.set(tool.name, server.name)
        }
        s.closers.push(() => client.close())
      } catch (error) {
        console.error(`mcp ${server.name} failed: ${error}`)
      }
    }
    sessions.set(id, s)
    return s
  }

  async function prompt(params: acp.PromptRequest, conn: acp.AgentSideConnection): Promise<acp.PromptResponse> {
    const s = session(params.sessionId)
    const { endpoint, model } = s.choice
    if (endpoint.secret && !endpoint.key) throw acp.RequestError.invalidParams(undefined, `no key for ${endpoint.secret}`)
    s.abort?.abort()
    const abort = (s.abort = new AbortController())
    const update = (u: acp.SessionUpdate) => conn.sessionUpdate({ sessionId: s.id, update: u })
    const userText = params.prompt.map((b) => (b.type === 'text' ? b.text : b.type === 'resource_link' ? `[${b.name}](${b.uri})` : '')).join('\n')
    const messages: Array<ModelMessage> = [...s.messages, { role: 'user', content: userText }]
    let finished: Array<ModelMessage> | undefined
    let partial = ''
    const usage = { totalTokens: 0, inputTokens: 0, outputTokens: 0 }

    // Every tool call asks the client first; Rust's permission handler answers Assembly's own tools.
    const permission: ChatMiddleware = {
      name: 'acp-permission',
      async onBeforeToolCall(_ctx, call) {
        const answer = await conn.requestPermission({
          sessionId: s.id,
          toolCall: {
            toolCallId: call.toolCallId, title: call.toolName, kind: 'other', status: 'pending', rawInput: call.args,
            _meta: { is_mcp_tool_call: true, mcp: { server: s.toolServer.get(call.toolName) } },
          },
          options: [{ optionId: 'allow', name: 'Allow', kind: 'allow_once' }, { optionId: 'reject', name: 'Reject', kind: 'reject_once' }],
        })
        const allowed = answer.outcome.outcome === 'selected' && answer.outcome.optionId === 'allow'
        return allowed ? undefined : { type: 'skip', result: { error: REFUSED } }
      },
      onFinish(ctx, info) {
        finished = [...ctx.messages]
        if (info.content && finished.at(-1)?.role !== 'assistant') finished.push({ role: 'assistant', content: info.content })
      },
    }

    try {
      const stream = chat({
        adapter: makeAdapter(endpoint, model),
        messages,
        tools: model.tools ? s.tools : [],
        middleware: [permission],
        abortController: abort,
        modelOptions: endpoint.adapter === 'ollama' ? { options: { num_ctx: model.context_window } } : undefined,
      })
      for await (const c of stream as AsyncIterable<any>) {
        if (abort.signal.aborted) break
        switch (c.type) {
          case 'TEXT_MESSAGE_CONTENT':
            partial += c.delta
            await update({ sessionUpdate: 'agent_message_chunk', content: text(c.delta) })
            break
          case 'REASONING_MESSAGE_CONTENT': await update({ sessionUpdate: 'agent_thought_chunk', content: text(c.delta) }); break
          case 'TOOL_CALL_START': await update({ sessionUpdate: 'tool_call', toolCallId: c.toolCallId, title: c.toolName, kind: 'other', status: 'pending' }); break
          case 'TOOL_CALL_END': await update({ sessionUpdate: 'tool_call_update', toolCallId: c.toolCallId, status: 'in_progress', rawInput: c.input }); break
          case 'TOOL_CALL_RESULT':
            await update({ sessionUpdate: 'tool_call_update', toolCallId: c.toolCallId, status: 'completed', content: [{ type: 'content', content: text(String(c.content)) }] })
            break
          case 'RUN_FINISHED':
            if (!c.usage) break
            usage.inputTokens += c.usage.promptTokens ?? 0
            usage.outputTokens += c.usage.completionTokens ?? 0
            usage.totalTokens += c.usage.totalTokens ?? 0
            await update({ sessionUpdate: 'usage_update', used: c.usage.totalTokens ?? 0, size: model.context_window })
            break
          case 'RUN_ERROR': throw new Error(c.message ?? c.error?.message ?? 'model run failed')
        }
      }
    } catch (error) {
      if (!abort.signal.aborted) throw error
    } finally {
      if (s.abort === abort) s.abort = undefined
    }
    // A cancelled turn keeps what the model said so far, so its user message isn't left unanswered.
    const cancelled = abort.signal.aborted
    s.messages = !cancelled && finished ? finished : [...messages, ...(partial ? [{ role: 'assistant' as const, content: partial }] : [])]
    await saveHistory(config.dataDir, s.id, { model: s.choice.value, messages: s.messages })
    return { stopReason: cancelled ? 'cancelled' : 'end_turn', usage }
  }

  return new acp.AgentSideConnection((conn) => ({
    initialize: () => ({
      protocolVersion: acp.PROTOCOL_VERSION,
      agentCapabilities: {
        loadSession: false,
        sessionCapabilities: { resume: {} },
        mcpCapabilities: { http: true, sse: false },
        promptCapabilities: {},
      },
      agentInfo: { name: 'assembly-models-acp', version: '0.1.0' },
      authMethods: [],
    }),
    authenticate: () => ({}),
    async newSession(params) {
      if (!choices.length) throw acp.RequestError.invalidParams(undefined, 'no models configured')
      const s = await open(randomUUID(), choices[0], [], params.mcpServers ?? [])
      await saveHistory(config.dataDir, s.id, { model: s.choice.value, messages: [] })
      return { sessionId: s.id, configOptions: configOptions(s) }
    },
    async resumeSession(params) {
      const saved = await loadHistory(config.dataDir, params.sessionId)
      if (!saved) throw acp.RequestError.invalidParams(undefined, `unknown session ${params.sessionId}`)
      const choice = choices.find((c) => c.value === saved.model) ?? choices[0]
      const s = await open(params.sessionId, choice, saved.messages, params.mcpServers ?? [])
      return { configOptions: configOptions(s) }
    },
    prompt: (params) => prompt(params, conn),
    async cancel(params) { sessions.get(params.sessionId)?.abort?.abort() },
    async setSessionConfigOption(params) {
      const s = session(params.sessionId)
      const choice = choices.find((c) => c.value === params.value)
      if (params.configId !== 'model' || !choice) throw acp.RequestError.invalidParams(undefined, `unknown model ${String(params.value)}`)
      s.choice = choice
      return { configOptions: configOptions(s) }
    },
    async closeSession(params) {
      const s = sessions.get(params.sessionId)
      s?.abort?.abort()
      await Promise.all(s?.closers.map((close) => close()) ?? [])
      sessions.delete(params.sessionId)
      return {}
    },
  }), stream)
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  // TanStack's logger prints error details with console.dir, which Node sends to stdout.
  globalThis.console = new Console(process.stderr, process.stderr)
  serve(acp.ndJsonStream(Writable.toWeb(process.stdout), Readable.toWeb(process.stdin) as ReadableStream<Uint8Array>), {
    endpoints: JSON.parse(process.env.ASSEMBLY_MODELS_ENDPOINTS ?? '[]'),
    dataDir: process.env.ASSEMBLY_MODELS_DATA ?? '.',
  })
}
