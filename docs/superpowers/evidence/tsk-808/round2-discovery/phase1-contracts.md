Discovery is complete. The worktree is clean (`git status --short --branch` → `## tsk-808-assembly-wave`); no files were edited or committed. The key contract gaps are: ACP notifications are currently skipped by `request`, the force-raw code is in `/routes/next/+page.svelte`, browser navigation passes the wrong Tauri argument shape, and usage reads depend on an external `sqlite3` binary.

## 1. Manager payloads, normalized events, and registry

[verified] `ManagedAgentSession` stores both normalized canonical events and frontend conversation events.

`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:31-50`

```rust
pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub provider_instance_id: String,
    pub native_session_id: Option<String>,
    pub generation: u64,
    pub owner: AgentExecutionOwner,
    pub state: AgentRuntimeState,
    pub capabilities: AgentCapabilities,
    pub next_sequence: u64,
    pub active_turn_id: Option<String>,
    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
    pub recent_events: VecDeque<AgentEvent>,
    pub writer_lease: AgentWriterLease,
    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
    cwd: String,
    connection: AgentConversationConnection,
    frontend_events: VecDeque<AgentConversationEvent>,
    journal: AgentEventJournal,
}
```

[verified] Sessions are keyed by `owned_id`.

`manager.rs:58-62`

```rust
pub struct AgentRuntimeManager {
    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    providers: Arc<ProviderRegistry>,
}
```

[verified] `ensure` looks up `sessions.get(&owned_id)`, removes a replaced entry, increments `generation`, and inserts the new session under the same `owned_id`.

`manager.rs:142-186`

[verified] Exact `emit_payload` signature and behavior:

`manager.rs:265-308`

```rust
pub fn emit_payload(
    &self,
    owned_id: &str,
    generation: u64,
    payload: AgentConversationPayload,
) -> Result<AgentConversationEvent, String> {
    let mut sessions = self
        .sessions
        .lock()
        .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
    let session = current_session_mut(&mut sessions, owned_id, generation)?;
    let sequence = session.next_sequence;
    session.next_sequence = session.next_sequence.saturating_add(1);
    if let AgentConversationPayload::Connection {
        state,
        native_session_id,
    } = &payload
    {
        session.connection.state = *state;
        if native_session_id.is_some() {
            session.connection.native_session_id = native_session_id.clone();
            session.native_session_id = native_session_id.clone();
        }
    }
    let timestamp_ms = timestamp_millis();
    let frontend_event = AgentConversationEvent {
        owned_id: owned_id.to_string(),
        provider: session.provider,
        generation,
        sequence,
        timestamp_ms,
        payload: payload.clone(),
    };
    let canonical = canonical_event(session, sequence, timestamp_ms, &payload)?;
    session.journal.append(canonical.clone())?;
    session.recent_events.push_back(canonical);
    session.frontend_events.push_back(frontend_event.clone());
    while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
        session.recent_events.pop_front();
    }
    while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
        session.frontend_events.pop_front();
    }
    Ok(frontend_event)
}
```

[verified] `emit_payload` itself does not call `app.emit`; it stores the canonical event and returns the frontend event. The only direct event emission found is terminal projection’s `app.emit` in `terminal_projection.rs:357-369`.

[verified] Normalized frontend payload type:

`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:467-516`

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AgentConversationPayload {
    Connection {
        state: ConversationConnectionState,
        #[serde(skip_serializing_if = "Option::is_none")]
        native_session_id: Option<String>,
    },
    UserMessage {
        item_id: String,
        text: String,
        completed: bool,
    },
    AssistantDelta {
        item_id: String,
        delta: String,
    },
    AssistantMessage {
        item_id: String,
        text: String,
        completed: bool,
    },
    Tool {
        item_id: String,
        name: String,
        state: ToolState,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
    Approval {
        request_id: String,
        state: ApprovalState,
        summary: String,
    },
    Turn {
        turn_id: String,
        state: TurnState,
    },
    Usage {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}
```

[verified] The frontend event envelope is:

`protocol.rs:518-527`

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationEvent {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub generation: u64,
    pub sequence: u64,
    pub timestamp_ms: u128,
    pub payload: AgentConversationPayload,
}
```

[verified] Canonical `AgentEvent` mapping is determined by payload kind.

`manager.rs:880-930`

Examples:

```rust
AgentConversationPayload::AssistantDelta { .. } => AgentEventType::ContentDelta,
AgentConversationPayload::AssistantMessage { .. } => AgentEventType::ItemCompleted,
AgentConversationPayload::Tool { .. } => AgentEventType::ItemUpdated,
AgentConversationPayload::Turn {
    state: TurnState::Started,
    ..
} => AgentEventType::TurnStarted,
AgentConversationPayload::Usage { .. } => AgentEventType::UsageUpdated,
AgentConversationPayload::Error { .. } => AgentEventType::RuntimeError,
```

## 2. ACP request/notification loop and wire shape

[verified] The dependency is pinned to ACP `2.0.0`; its schema dependency resolves to `1.5.0`.

`tauri-svelte-preview/src-tauri/Cargo.toml:11-13`

```toml
agent-client-protocol = "=2.0.0"
```

`tauri-svelte-preview/src-tauri/Cargo.lock:47-48,81-83`

```toml
name = "agent-client-protocol"
version = "2.0.0"

name = "agent-client-protocol-schema"
version = "1.5.0"
```

[verified] Exact request loop:

`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:349-371`

```rust
async fn request<T: Serialize>(
    &mut self,
    method: &str,
    params: &T,
) -> Result<Value, AgentRuntimeError> {
    self.next_request_id = self.next_request_id.saturating_add(1);
    let id = self.next_request_id;
    let params = serde_json::to_value(params).map_err(serialization_error)?;
    self.process
        .write_json(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
        .await
        .map_err(transport_error)?;
    loop {
        let frame = self.process.next_json().await.map_err(transport_error)?;
        if frame.get("id") != Some(&json!(id)) {
            continue;
        }
        if let Some(error) = frame.get("error") {
            return Err(AgentRuntimeError::new("acp-error", error.to_string()));
        }
        return Ok(frame.get("result").cloned().unwrap_or(Value::Null));
    }
}
```

[verified] A `session/update` notification has no matching `id`, so this loop reads it at line 362 and then discards it at lines 363-365.

[verified] `prompt_once` has a separate manual frame loop that treats any non-response frame as a possible update, without checking the JSON-RPC method.

`acp_client.rs:184-241`

```rust
let frame = self.process.next_json().await.map_err(transport_error)?;
if frame.get("id") == Some(&json!(request_id)) {
    ...
}

let params = frame.get("params").unwrap_or(&Value::Null);
let update = params
    .get("update")
    .or_else(|| params.get("sessionUpdate"))
    .unwrap_or(params);
...
if update
    .get("sessionUpdate")
    .or_else(|| update.get("session_update"))
    .or_else(|| update.get("type"))
    .and_then(Value::as_str)
    .is_some_and(|kind| {
        matches!(
            kind,
            "agent_message_chunk"
                | "agent-message-chunk"
                | "agent_message"
                | "assistant_message_delta"
        )
    })
{
    if let Some(chunk) = update.get("content").and_then(extract_text_value) {
        text.push_str(&chunk);
    }
    if let Some(message) = update.get("message").and_then(extract_text_value) {
        text.push_str(&message);
    }
}
```

[verified] ACP crate type shape:

`agent-client-protocol-schema-1.5.0/src/v1/client.rs:39-65`

```rust
pub struct SessionNotification {
    pub session_id: SessionId,
    pub update: SessionUpdate,
    #[serde(default)]
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}
```

`client.rs:90-109`

```rust
#[serde(tag = "sessionUpdate", rename_all = "snake_case")]
pub enum SessionUpdate {
    UserMessageChunk(ContentChunk),
    AgentMessageChunk(ContentChunk),
    AgentThoughtChunk(ContentChunk),
    ToolCall(ToolCall),
    ToolCallUpdate(ToolCallUpdate),
    Plan(Plan),
    ...
}
```

`client.rs:401-428`

```rust
pub struct ContentChunk {
    pub content: ContentBlock,
    pub message_id: Option<MessageId>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}
```

[verified] The ACP wrapper maps `SessionNotification` to method `"session/update"`.

`agent-client-protocol-2.0.0/src/schema/enum_impls.rs:104-106`

```rust
impl_jsonrpc_notification_enum!(AgentNotification {
    SessionNotification => "session/update",
```

`agent-client-protocol-schema-1.5.0/src/v1/client.rs:2281-2282`

```rust
/// Notification name for session updates.
pub(crate) const SESSION_UPDATE_NOTIFICATION: &str = "session/update";
```

[verified] The vendored crate’s RPC serialization test separately expects `"sessionUpdate"` rather than `"session/update"`.

`agent-client-protocol-schema-1.5.0/src/rpc.rs:356-392`

```rust
let outgoing_msg = JsonRpcMessage::wrap(Notification {
    method: "sessionUpdate".into(),
    ...
});

assert_eq!(
    serialized,
    json!({
        "jsonrpc": "2.0",
        "method": "sessionUpdate",
        "params": {
            "sessionId": "test-456",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": {
                    "type": "text",
                    "text": "Hello"
                }
            }
        }
    })
);
```

[assumed] Phase 1 should decide whether to accept both method spellings, because the current crate source maps the typed notification to `"session/update"` while the vendored wire-format test asserts `"sessionUpdate"`.

## 3. `StructuredRuntimeHandle`

[verified] The enum currently has exactly one variant.

`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs:146-148`

```rust
pub enum StructuredRuntimeHandle {
    Acp(AcpRuntimeAdapter),
}
```

[verified] Exact methods and signatures:

`acp.rs:150-218`

```rust
impl StructuredRuntimeHandle {
    pub fn process_id(&self) -> Option<u32> {
        match self {
            Self::Acp(adapter) => adapter.process_id(),
        }
    }

    pub async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.prompt(input).await,
        }
    }

    pub async fn prompt_once(
        &mut self,
        input: AgentPrompt,
    ) -> Result<GeneratedText, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.prompt_once(input).await,
        }
    }

    pub async fn cancel_turn(&mut self, turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.cancel_turn(turn_id).await,
        }
    }

    pub async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.steer(input).await,
        }
    }

    pub async fn set_config(
        &mut self,
        option_id: &str,
        value: AgentConfigValue,
    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.set_config(option_id, value).await,
        }
    }

    pub async fn respond_permission(
        &mut self,
        input: PermissionResponse,
    ) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.respond_permission(input).await,
        }
    }

    pub async fn respond_user_input(
        &mut self,
        input: UserInputResponse,
    ) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.respond_user_input(input).await,
        }
    }

    pub async fn close_session(&mut self) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.close_session().await,
        }
    }

    pub async fn detach_session(&mut self) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.detach_session().await,
        }
    }
}
```

[verified] `AgentRuntimeManager::activate` wraps the adapter as `StructuredRuntimeHandle::Acp(adapter)`.

`manager.rs:239`

```rust
let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
```

## 4. Frontend event consumption and dispatch

[verified] Event channel and listener:

`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:356-368`

```ts
export async function startConversationEvents(): Promise<void> {
  if (!isTauri() || unlisten) return;
  unlisten = await listen<AgentConversationEvent | AgentEvent>('agent-conversation-event', ({ payload }) => {
    applyAgentConversationEvent(payload);
    if (getConversationSession(payload.ownedId)?.desynchronized) {
      void resyncConversation(payload.ownedId);
    }
    if (('kind' in payload.payload && (payload.payload.kind === 'turn' || payload.payload.kind === 'error'))
      || ('type' in payload && ['turn.completed', 'turn.interrupted', 'runtime.error'].includes(payload.type))) {
      setConversationSending(payload.ownedId, false);
    }
  });
}
```

[verified] The same event name is declared for terminal projection.

`tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:13`

```rust
pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
```

[verified] TypeScript canonical event type:

`conversationTypes.ts:225-247,280-295`

```ts
export type AgentEventType =
  | 'session.started'
  | 'session.config.updated'
  | 'session.state.changed'
  | 'session.closed'
  | 'turn.started'
  | 'turn.completed'
  | 'turn.interrupted'
  | 'item.started'
  | 'item.updated'
  | 'item.completed'
  | 'content.delta'
  | 'approval.requested'
  | 'approval.resolved'
  | 'user-input.requested'
  | 'user-input.resolved'
  | 'plan.updated'
  | 'tasks.updated'
  | 'children.updated'
  | 'usage.updated'
  | 'rate-limits.updated'
  | 'runtime.warning'
  | 'runtime.error';

export interface AgentEvent {
  type: AgentEventType;
  ownedId: string;
  provider: AgentConversationProvider;
  providerInstanceId: string;
  generation: number;
  sequence: number;
  timestampMs: number;
  nativeSessionId?: string;
  turnId?: string;
  itemId?: string;
  requestId?: string;
  payload: Record<string, AgentConfigValue>;
  providerMetadata?: Record<string, AgentConfigValue>;
  rawFrameReference?: AgentRawFrameReference;
}
```

[verified] TypeScript normalized conversation event and payload types:

`conversationTypes.ts:388-410`

```ts
export type AgentConversationPayload =
  | { kind: 'connection'; state: ConversationConnectionState; nativeSessionId?: string }
  | { kind: 'userMessage'; itemId: string; text: string; completed: true }
  | { kind: 'assistantDelta'; itemId: string; delta: string }
  | { kind: 'assistantMessage'; itemId: string; text: string; completed: true }
  | { kind: 'tool'; itemId: string; name: string; state: ToolState; summary?: string }
  | { kind: 'approval'; requestId: string; state: ApprovalState; summary: string }
  | { kind: 'turn'; turnId: string; state: TurnState }
  | { kind: 'usage'; inputTokens?: number; outputTokens?: number }
  | { kind: 'error'; code: string; message: string; recoverable: boolean };

export interface AgentConversationEvent {
  ownedId: string;
  provider: AgentConversationProvider;
  generation: number;
  sequence: number;
  timestampMs: number;
  payload: AgentConversationPayload;
}
```

[verified] Timeline message and turn shapes:

`conversationTypes.ts:412-451`

```ts
interface ConversationTextEntry {
  itemId: string;
  text: string;
  completed: boolean;
  timestampMs: number;
}

export type ConversationTimelineEntry =
  | (ConversationTextEntry & { kind: 'user' | 'assistant' })
  | {
      kind: 'tool';
      itemId: string;
      name: string;
      state: ToolState;
      summary?: string;
      timestampMs: number;
    }
  | {
      kind: 'turn';
      itemId: string;
      turnId: string;
      state: TurnState;
      timestampMs: number;
    }
  | {
      kind: 'error';
      itemId: string;
      code: string;
      message: string;
      recoverable: boolean;
      timestampMs: number;
    };
```

[verified] `applyAgentConversationEvent` first dispatches by envelope shape, then canonical events use sequence/generation validation.

`conversationStore.svelte.ts:127-170`

```ts
export function applyAgentConversationEvent(event: AgentConversationEvent | AgentEvent): boolean {
  if ('type' in event) return applyCanonicalAgentEvent(event);
  const current = ensureConversationSession(event.ownedId, event.provider);
  const next = applyConversationEvent(current, event);
  if (next === current) return false;
  ...
  const typedItem = agentItemFromEvent(event);
  if (typedItem) {
    const delta = event.payload.kind === 'assistantDelta';
    ...
  }
  applyTypedEventPayload(conversationSessions[event.ownedId], event);
  return true;
}
```

[verified] The reducer dispatches on `payload.kind`.

`conversationReducer.ts:47-72`

```ts
export function applyConversationEvent(
  state: ConversationSessionState,
  event: AgentConversationEvent
): ConversationSessionState {
  ...
  const { payload } = event;
  switch (payload.kind) {
    case 'connection':
```

[verified] Cases for user, assistant delta/completion, tools, approval, turn, usage, and error are implemented at `conversationReducer.ts:80-181`.

## 5. Force-raw selection and structured close

[verified] The requested line numbers do not match the root route. `src/routes/+page.svelte:983-992` is unrelated embedded-terminal state.

`tauri-svelte-preview/src/routes/+page.svelte:983-992`

```ts
let sourceContextCardDockviewHostToken = 0;
const sourceContextCardDockviewPanelElements = new Map<SourceContextCardDockviewPanelID, HTMLElement>();
let embeddedTerminal: XTermTerminal | null = null;
let embeddedTerminalFitAddon: XTermFitAddon | null = null;
let embeddedTerminalSearchAddon: XTermSearchAddon | null = null;
let embeddedTerminalSerializeAddon: XTermSerializeAddon | null = null;
let embeddedTerminalWebglAddon: XTermWebglAddon | null = null;
let embeddedTerminalAddonStatus = $state("fit");
let embeddedTerminalInputDisposable: { dispose: () => void } | null = null;
```

[verified] The actual force-raw selection path is in `src/routes/next/+page.svelte:983-1002`.

```ts
const provider = conversationProviderFor(ownedId);
if (selected && provider) {
  // A running PTY is already the owner of this agent session. Starting a
  // second `claude --resume` / `codex app-server` here makes two agents
  // append different answers to one conversation. Until the structured
  // surface mirrors the PTY's transcript, keep the real terminal visible.
  if (selected.ptySessionId) {
    void closeStructuredConversation(ownedId);
    setConversationMode(ownedId, 'raw');
    return;
  }
  void ensureStructuredConversation({
    ownedId,
    provider,
    cwd: selected.cwd,
    nativeSessionId: selected.nativeSessionId
  }).catch((error) => {
    rail.error = `could not open structured ${provider}: ${describeError(error)}`;
  });
}
```

[verified] Exact `startNewSession` path requested at lines 1128-1149:

`src/routes/next/+page.svelte:1128-1149`

```ts
async function startNewSession(request: NewSessionRequest): Promise<void> {
  if (!service || disposed) return;
  const owned = {
    ...createFreshSession({ cwd: request.cwd, title: request.title }),
    agent: request.agent,
    resumeCommand: request.command
  };
  addOwnedSession(owned);
  const host = await hostFor(owned.ownedId);
  if (!host) {
    rail.error = `no terminal host for "${owned.title}"`;
    return;
  }
  const ptySessionId = await service.startOwned(owned, host);
  if (!ptySessionId) {
    updateOwnedSession(owned.ownedId, { state: 'exited' });
    rail.error = `failed to start a terminal for "${owned.title}"`;
    return;
  }
  // Persist the PTY id: reload re-attach reads it back out of localStorage.
  updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  await selectOwned(owned.ownedId);
```

[verified] Exact structured close:

`conversationService.ts:376-379`

```ts
export async function closeStructuredConversation(ownedId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<boolean>('close_agent_conversation', { ownedId });
}
```

## 6. Terminal projection event payload

[verified] Terminal projection emits canonical `AgentEvent`, not `AgentConversationEvent`.

`terminal_projection.rs:318-354`

```rust
fn canonical_event(
    owned_id: &str,
    provider: AgentConversationProvider,
    native_session_id: &str,
    generation: u64,
    sequence: u64,
    record: ProjectedRecord,
) -> AgentEvent {
    AgentEvent {
        event_type: record.event_type,
        owned_id: owned_id.to_string(),
        provider,
        provider_instance_id: format!("terminal-transcript:{native_session_id}"),
        generation,
        sequence,
        timestamp_ms: if record.timestamp_ms == 0 {
            now_millis()
        } else {
            record.timestamp_ms
        },
        native_session_id: Some(native_session_id.to_string()),
        turn_id: None,
        item_id: record.item_id.clone(),
        request_id: None,
        payload: record.payload,
        provider_metadata: Some(std::collections::BTreeMap::from([
            ("source".into(), serde_json::Value::String("terminal-transcript".into())),
            ("historical".into(), serde_json::Value::Bool(true)),
        ])),
        raw_frame_reference: Some(AgentRawFrameReference {
            id: record.key,
            redacted: true,
        }),
    }
}
```

[verified] The canonical Rust struct serializes `event_type` as JSON `"type"` and uses camelCase for the remaining fields.

`protocol.rs:255-279`

```rust
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    #[serde(rename = "type")]
    pub event_type: AgentEventType,
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub provider_instance_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub timestamp_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub payload: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_frame_reference: Option<AgentRawFrameReference>,
}
```

[verified] Emission and bounded history:

`terminal_projection.rs:357-369`

```rust
fn remember_and_emit<R: Runtime>(
    app: &tauri::AppHandle<R>,
    events: &Arc<Mutex<VecDeque<AgentEvent>>>,
    event: AgentEvent,
) {
    if let Ok(mut recent) = events.lock() {
        recent.push_back(event.clone());
        while recent.len() > MAX_RECENT_EVENTS {
            recent.pop_front();
        }
    }
    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);
}
```

## 7. Browser navigation mismatch

[verified] Frontend navigation call:

`tauri-svelte-preview/src/lib/shell/browser/browserModel.ts:417-420`

```ts
callBackend(context, () => context.backend!.navigate_browser_tab({ ...backendTarget(tab), url: normalized }), (error) => {
  tab.error = describeError(error);
  tab.loadState = 'error';
  setWorkspaceError(context, error);
});
```

[verified] Tauri backend passes that object directly:

`browserBackend.ts:241-243`

```ts
navigate_browser_tab(input: BrowserBackendNavigationInput): Promise<void> {
  return invokeBrowserCommandFromTauri<void>('navigate_browser_tab', input);
}
```

`tauriSource.ts:808-817`

```ts
export async function invokeBrowserCommandFromTauri<T>(
  command: string,
  input: unknown
): Promise<T> {
  ...
  return invoke<T>(command, input as Record<string, unknown>);
}
```

[verified] Rust command has a named `input` argument:

`tauri-svelte-preview/src-tauri/src/browser.rs:100-107,1254-1260`

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserNavigationInput {
    pub workspace_id: String,
    pub tab_id: String,
    pub generation: u64,
    pub url: String,
}

#[tauri::command]
pub fn navigate_browser_tab(
    registry: tauri::State<'_, BrowserRegistry>,
    input: BrowserNavigationInput,
) -> Result<(), BrowserCommandError> {
    registry.navigate(input)
}
```

[verified shape mismatch] The frontend sends:

```json
{
  "workspaceId": "...",
  "tabId": "...",
  "generation": 1,
  "url": "..."
}
```

The Rust command contract expects:

```json
{
  "input": {
    "workspaceId": "...",
    "tabId": "...",
    "generation": 1,
    "url": "..."
  }
}
```

[assumed] The missing `input` wrapper is the native Tauri failure cause. The fake backend test does not exercise Tauri IPC; it only calls the in-memory implementation.

`tauri-svelte-preview/scripts/browserBackend.test.mjs:4-30`

## 8. Usage history read failure

[verified] Usage commands open the database through `usage_db_path`.

`tauri-svelte-preview/src-tauri/src/usage_history.rs:28-67`

```rust
#[tauri::command]
pub fn read_usage_summary(app: AppHandle, query: UsageHistoryQuery) -> Result<UsageSummary, String> {
    UsageDb::open(usage_db_path(&app)?)?.read_usage_summary(&query.into())
}

pub fn usage_db_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    if let Some(path) = std::env::var_os("MAC_COMMAND_BAR_USAGE_DB") {
        return Ok(path.into());
    }
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?
        .join("usage-history.sqlite3"))
}
```

[verified] `UsageDb::open` creates the parent directory and applies the embedded schema.

`usage_db.rs:8,162-170`

```rust
pub const USAGE_SCHEMA: &str = include_str!("../../migrations/0001_usage_history.sql");

pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
    let path = path.into();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create usage database folder: {error}"))?;
    }
    let db = Self { path };
    db.execute(&format!("BEGIN IMMEDIATE;\n{USAGE_SCHEMA}\nCOMMIT;"))?;
    Ok(db)
}
```

[verified] Reads shell out to an external binary. The binary is `MCB_SQLITE_BIN` or `"sqlite3"`.

`usage_db.rs:402-420`

```rust
let output = sqlite_command()
    .arg("-batch")
    .arg("-json")
    .arg(&self.path)
    .arg(sql)
    .output()
    .map_err(|error| format!("SQLite is unavailable: {error}"))?;
...
fn sqlite_command() -> Command {
    let binary = std::env::var_os("MCB_SQLITE_BIN").unwrap_or_else(|| "sqlite3".into());
    let mut command = Command::new(binary);
```

[verified] Frontend fallback string:

`tauri-svelte-preview/src/lib/shell/usage/usageStore.svelte.ts:80-82`

```ts
} catch (error) {
  usageState.error = error instanceof Error ? error.message : 'Usage history could not be read.';
  return null;
}
```

Ranked causes:

1. [assumed, strongest] Packaged app cannot find `sqlite3` on `PATH`. Receipt: `usage_db.rs:404,410,418-420` explicitly launches an external binary and returns `"SQLite is unavailable: ..."`.
2. [assumed] App-data directory creation or permissions failure. Receipt: `usage_history.rs:63-67` can return `"Application data directory is unavailable: ..."`, and `usage_db.rs:165-167` can return `"Could not create usage database folder: ..."`.
3. [assumed] Schema or SQLite execution failure. Receipt: `usage_db.rs:169`, `402-406`, and `408-415` return `"SQLite command failed: ..."`, `"SQLite query failed: ..."`, or invalid JSON errors.
4. [assumed] `MAC_COMMAND_BAR_USAGE_DB` points to an invalid or inaccessible path. Receipt: `usage_history.rs:60-62` bypasses app-data resolution entirely.
5. [assumed, weakest] “Database was not created” by itself. `UsageDb::open` creates the parent and runs the schema during the read path (`usage_db.rs:163-170`), so simple file absence is not sufficient to explain the failure.

## 9. Existing tests and pins

[verified] Capability list pin:

`tauri-svelte-preview/src-tauri/src/main.rs:8254-8289`

```rust
#[test]
fn backend_capabilities_name_every_addition_the_frontend_cannot_otherwise_detect() {
    assert_eq!(
        backend_capabilities(),
        vec![
            ...
            "providerUsageQuota".to_string(),
            "usageHistory".to_string(),
            "usageHistoryIncremental".to_string(),
            "usageProviderSummary".to_string(),
            "usageDailyTotals".to_string(),
            ...
        ]
    );
}
```

[verified] Rust manager pins:

- `manager.rs:1022-1063` — `ensure_is_idempotent_and_generation_rejects_stale_writers`
- `manager.rs:1065-1095` — `canonical_sequence_and_snapshot_are_bounded_and_repairable`
- `manager.rs:1097-1117` — `structured_session_does_not_claim_a_user_pty`
- `manager.rs:1119-1135` — `distinct_tool_terminal_identity_is_not_a_user_pty_identity`

[verified] Rust protocol pins:

- `protocol.rs:592-610` — `runtime_owner_and_state_use_the_public_wire_names`
- `protocol.rs:612-637` — `unknown_config_categories_round_trip_losslessly`
- `protocol.rs:639-659` — `canonical_event_round_trips_without_acp_frames`

[verified] ACP adapter pins:

- `providers/acp_client.rs:502-586` — `acp_initialize_new_prompt_image_config_correlations_cancel_and_close`
- `providers/acp_client.rs:588-609` — `acp_load_and_resume_use_distinct_session_methods`

[verified] Frontend conversation contract scripts:

- `scripts/agentConversationProtocol.test.mjs:20-112` — generation/sequence rejection, gap handling, assistant deltas/completion, errors, history identity, config categories.
- `scripts/agentConversationStore.test.mjs:29-152` — session isolation, raw/structured mode, event application, snapshot repair, stale snapshot rejection, workspace restore.
- `scripts/agentConversationTerminalProjection.test.mjs:13-40` — canonical event channel, historical metadata, no invented live state, no PTY creation.
- `scripts/agentConversationHandoff.test.mjs:12-96` — structured/raw handoff, generation matching, receipt restoration, explicit projection trigger.
- `scripts/browserBackend.test.mjs:4-37` — fake navigation only; it does not test the Tauri `{ input: ... }` wrapper.
- `scripts/conversationSessionIsolation.test.mjs:59-68` — persisted raw/structured session modes.

### Files Phase 1 will need to touch

[assumed plan scope] For the ACP-only foundation itself:

- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs`
- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs`
- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs`
- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs`
- `tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs` — only if parity tests or shared emission wiring change
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts`
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts`
- `tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts`
- `tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts`
- `tauri-svelte-preview/src/routes/next/+page.svelte`
- the Rust and script test files listed above

[assumed, only if Phase 1 includes the surfaced bugs] Additional files:

- `tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts`
- `tauri-svelte-preview/src/lib/tauriSource.ts`
- `tauri-svelte-preview/src/lib/shell/browser/browserModel.ts`
- `tauri-svelte-preview/src-tauri/src/browser.rs`
- `tauri-svelte-preview/src/lib/shell/usage/usageStore.svelte.ts`
- `tauri-svelte-preview/src-tauri/src/usage_history.rs`
- `tauri-svelte-preview/src-tauri/src/usage_db.rs`

`tauri-svelte-preview/src-tauri/Cargo.toml` already pins `agent-client-protocol = "=2.0.0"` at line 12, so no dependency edit is evidenced unless the implementation deliberately changes the ACP version.


