//! Shared, transport-free ACP projection support for the provider adapters.
//!
//! The ACP transport in `acp.rs` owns process supervision and request/response I/O.  This
//! module deliberately does not open a process (and never creates a PTY): it only projects
//! already decoded ACP JSON frames into the frozen Assembly protocol types.  Keeping the
//! projection state here lets the Codex and Claude adapters apply the same ordering and ID
//! rules while retaining their own metadata namespace.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use super::super::protocol::{
    AgentCapabilities, AgentCommandDescriptor, AgentConfigOption, AgentConfigOptionChoice,
    AgentContent, AgentContentChannel, AgentConversationProvider, AgentEvent, AgentEventType,
    AgentImplementation, AgentInteractionCapabilities, AgentItem, AgentItemType,
    AgentPromptCapabilities, AgentRawFrameReference, AgentSessionCapabilities,
};

/// The context supplied by the manager for one provider-owned stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterContext {
    pub owned_id: String,
    pub provider_instance_id: String,
    pub generation: u64,
    pub native_session_id: Option<String>,
    pub timestamp_ms: u128,
}

impl AdapterContext {
    pub fn new(
        owned_id: impl Into<String>,
        provider_instance_id: impl Into<String>,
        generation: u64,
        native_session_id: Option<impl Into<String>>,
    ) -> Self {
        Self {
            owned_id: owned_id.into(),
            provider_instance_id: provider_instance_id.into(),
            generation,
            native_session_id: native_session_id.map(Into::into),
            timestamp_ms: now_millis(),
        }
    }

    pub fn with_timestamp(mut self, timestamp_ms: u128) -> Self {
        self.timestamp_ms = timestamp_ms;
        self
    }
}

/// One ordered projection result.  Events are already sequenced; `items` contains the item
/// snapshots changed by those events in the same order.  A caller can append the snapshots to
/// its timeline without sorting by type or by provider-specific IDs.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionBatch {
    pub events: Vec<AgentEvent>,
    pub items: Vec<AgentItem>,
}

impl ProjectionBatch {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            items: Vec::new(),
        }
    }

    fn extend(&mut self, other: ProjectionBatch) {
        self.events.extend(other.events);
        self.items.extend(other.items);
    }
}

#[derive(Clone, Debug)]
struct PendingRequest {
    method: String,
    params: Value,
}

/// Stateful canonical projection used by both ACP adapters.
pub struct AcpProjectionState {
    context: AdapterContext,
    provider: AgentConversationProvider,
    namespace: String,
    sequence: u64,
    timestamp_ms: u128,
    native_session_id: Option<String>,
    active_turn_id: Option<String>,
    active_items: Vec<String>,
    active_item_set: HashSet<String>,
    items: Vec<AgentItem>,
    item_index: HashMap<String, usize>,
    pending_requests: HashMap<String, PendingRequest>,
    capabilities: AgentCapabilities,
}

impl AcpProjectionState {
    pub fn new(
        context: AdapterContext,
        provider: AgentConversationProvider,
        namespace: &str,
    ) -> Self {
        let timestamp_ms = context.timestamp_ms;
        let capabilities = empty_capabilities(provider);
        Self {
            native_session_id: context.native_session_id.clone(),
            timestamp_ms,
            context,
            provider,
            namespace: namespace.to_string(),
            sequence: 0,
            active_turn_id: None,
            active_items: Vec::new(),
            active_item_set: HashSet::new(),
            items: Vec::new(),
            item_index: HashMap::new(),
            pending_requests: HashMap::new(),
            capabilities,
        }
    }

    pub fn capabilities(&self) -> AgentCapabilities {
        self.capabilities.clone()
    }

    pub fn ordered_items(&self) -> Vec<AgentItem> {
        self.items.clone()
    }

    pub fn uses_pty(&self) -> bool {
        false
    }

    pub fn map_frame(&mut self, frame: &Value) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let method = frame.get("method").and_then(Value::as_str);

        if let Some(method) = method {
            let params = frame.get("params").cloned().unwrap_or(Value::Null);
            let request_id = frame.get("id").and_then(request_id_string);
            self.remember_native_session(&params);
            if is_session_update_method(method) {
                let update = params
                    .get("update")
                    .or_else(|| params.get("sessionUpdate"))
                    .unwrap_or(&params);
                batch.extend(self.map_update(update, &params, frame));
            } else if is_agent_request(method) {
                batch.extend(self.map_agent_request(method, &params, request_id.as_deref(), frame));
            } else {
                if let Some(request_id) = request_id.as_ref() {
                    self.pending_requests.insert(
                        request_id.clone(),
                        PendingRequest {
                            method: method.to_string(),
                            params: params.clone(),
                        },
                    );
                }
                batch.extend(self.map_outgoing_request(
                    method,
                    &params,
                    request_id.as_deref(),
                    frame,
                ));
            }
            return batch;
        }

        if frame.get("result").is_some() || frame.get("error").is_some() {
            return self.map_response(frame);
        }

        // Fixture readers may hand us the update body directly.  This is also useful when a
        // transport has already removed the JSON-RPC envelope.
        if frame.get("sessionUpdate").is_some() || frame.get("update").is_some() {
            self.remember_native_session(frame);
            let update = frame.get("update").unwrap_or(frame);
            return self.map_update(update, frame, frame);
        }

        self.unknown(frame, "frame")
    }

    pub fn map_jsonl(&mut self, input: &str) -> Vec<ProjectionBatch> {
        input
            .lines()
            .enumerate()
            .filter_map(|(line_number, line)| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                match serde_json::from_str::<Value>(line) {
                    Ok(frame) => Some(self.map_frame(&frame)),
                    Err(_) => {
                        Some(self.warning("malformed-jsonl", json!({ "line": line_number + 1 })))
                    }
                }
            })
            .collect()
    }

    fn remember_native_session(&mut self, params: &Value) {
        if self.native_session_id.is_some() {
            return;
        }
        let Some(session_id) = string_at(params, &["sessionId", "session_id"]) else {
            return;
        };
        self.native_session_id = Some(session_id.clone());
        self.context.native_session_id = Some(session_id);
    }

    fn map_outgoing_request(
        &mut self,
        method: &str,
        params: &Value,
        request_id: Option<&str>,
        frame: &Value,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        match method {
            "session/prompt" => {
                if let Some(turn_id) = string_at(params, &["turnId", "turn_id"]) {
                    self.start_turn(&mut batch, turn_id, frame);
                }
                let (item_id, turn_id) = self.prompt_identity(params);
                self.ensure_item(&item_id, AgentItemType::UserMessage, turn_id.clone(), None);
                if let Some(turn_id) = turn_id.clone() {
                    self.active_turn_id = Some(turn_id);
                }
                let payload = prompt_payload(params);
                self.touch_item_metadata(&item_id, json!({ "prompt": payload.clone() }));
                self.emit_item(
                    &mut batch,
                    AgentEventType::ItemStarted,
                    &item_id,
                    turn_id,
                    json!({ "item": payload, "source": "session/prompt" }),
                );
            }
            "session/cancel" => {
                let turn_id = string_at(params, &["turnId", "turn_id"])
                    .or_else(|| self.active_turn_id.clone())
                    .unwrap_or_else(|| "unknown-turn".to_string());
                self.emit_turn(&mut batch, &turn_id, "interrupted", frame);
                self.finish_active_items(&mut batch, true);
            }
            "session/new" | "session/load" | "session/resume" => {
                // The response owns the native session ID.  Keep the operation pending and do
                // not invent a session ID from the application-owned ID.
            }
            "session/close" => {}
            "session/set_config_option" | "session/set_mode" => {}
            "session/steer" => {}
            "initialize" => {}
            _ if method.contains("mcp") => {
                batch.extend(self.unknown(frame, "mcp-request"));
            }
            _ => batch.extend(self.unknown(frame, "request")),
        }
        if let Some(request_id) = request_id {
            for event in &mut batch.events {
                event.request_id = Some(request_id.to_string());
            }
        }
        batch
    }

    fn map_response(&mut self, frame: &Value) -> ProjectionBatch {
        let request_id = frame.get("id").and_then(request_id_string);
        let pending = request_id
            .as_deref()
            .and_then(|id| self.pending_requests.remove(id));
        let Some(pending) = pending else {
            if frame.get("error").is_some() {
                return self.error_from_frame(frame, "unmatched-response");
            }
            return self.unknown(frame, "response");
        };

        if frame.get("error").is_some() {
            return self.error_from_frame(frame, &pending.method);
        }
        let result = frame.get("result").cloned().unwrap_or(Value::Null);
        let mut batch = ProjectionBatch::new();
        match pending.method.as_str() {
            "initialize" => {
                self.capabilities =
                    capabilities_from_initialize(self.provider, &result, &self.namespace);
                batch.extend(self.session_config_event(&result, frame));
            }
            "session/new" | "session/load" | "session/resume" => {
                // `session/new` returns a new ID.  ACP `session/load` and `session/resume`
                // deliberately return only mode/config state, so retain the ID from the
                // request (or the current managed session) instead of inventing one.
                if let Some(session_id) = string_at(&result, &["sessionId", "session_id", "id"])
                    .or_else(|| string_at(&pending.params, &["sessionId", "session_id"]))
                    .or_else(|| self.native_session_id.clone())
                {
                    self.update_session_capabilities(&result);
                    self.native_session_id = Some(session_id.clone());
                    self.context.native_session_id = Some(session_id.clone());
                    let payload = json!({
                        "operation": pending.method,
                        "nativeSessionId": session_id,
                        "result": result,
                    });
                    self.emit_event(
                        &mut batch,
                        AgentEventType::SessionStarted,
                        None,
                        None,
                        None,
                        payload,
                        frame,
                    );
                } else {
                    batch.extend(self.warning(
                        "session-response-missing-id",
                        json!({ "operation": pending.method, "result": result }),
                    ));
                }
            }
            "session/close" => {
                self.emit_event(
                    &mut batch,
                    AgentEventType::SessionClosed,
                    self.native_session_id.clone(),
                    None,
                    None,
                    json!({ "operation": pending.method, "result": result }),
                    frame,
                );
            }
            "session/prompt" => {
                let turn_id = string_at(&result, &["turnId", "turn_id"])
                    .or_else(|| self.active_turn_id.clone())
                    .unwrap_or_else(|| "unknown-turn".to_string());
                self.active_turn_id = Some(turn_id.clone());
                let interrupted = string_at(&result, &["stopReason", "stop_reason"])
                    .map(|reason| {
                        matches!(
                            reason.as_str(),
                            "cancelled" | "canceled" | "interrupted" | "abort" | "error"
                        )
                    })
                    .unwrap_or(false);
                self.finish_active_items(&mut batch, interrupted);
                self.emit_turn(
                    &mut batch,
                    &turn_id,
                    if interrupted {
                        "interrupted"
                    } else {
                        "completed"
                    },
                    frame,
                );
                self.active_turn_id = None;
            }
            "session/set_config_option" | "session/set_mode" => {
                self.update_session_capabilities(&result);
                batch.extend(self.session_config_event(&result, frame));
            }
            "session/cancel" | "session/steer" => {}
            "session/request_permission" => {
                batch.extend(self.request_resolved(
                    AgentEventType::ApprovalResolved,
                    request_id.as_deref(),
                    &result,
                    frame,
                ));
            }
            "session/request_user_input" | "session/request_input" => {
                batch.extend(self.request_resolved(
                    AgentEventType::UserInputResolved,
                    request_id.as_deref(),
                    &result,
                    frame,
                ));
            }
            method
                if method.starts_with("terminal/")
                    || method.starts_with("fs/")
                    || method.contains("mcp") =>
            {
                batch.extend(self.tool_request_response(
                    &pending.method,
                    &pending.params,
                    &result,
                    frame,
                ));
            }
            _ => batch.extend(self.unknown(frame, "response")),
        }
        batch
    }

    fn map_agent_request(
        &mut self,
        method: &str,
        params: &Value,
        request_id: Option<&str>,
        frame: &Value,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        if let Some(request_id) = request_id {
            self.pending_requests.insert(
                request_id.to_string(),
                PendingRequest {
                    method: method.to_string(),
                    params: params.clone(),
                },
            );
        }
        match method {
            "session/request_permission" | "session/permission_request" => {
                let request_id = request_id.unwrap_or("unknown-request");
                let tool_call = params.get("toolCall").or_else(|| params.get("tool_call"));
                self.emit_event(
                    &mut batch,
                    AgentEventType::ApprovalRequested,
                    self.native_session_id.clone(),
                    string_at(params, &["turnId", "turn_id"]),
                    string_at(params, &["toolCallId", "tool_call_id"]).or_else(|| {
                        tool_call.and_then(|value| {
                            string_at(value, &["toolCallId", "tool_call_id"])
                        })
                    }),
                    json!({
                        "requestId": request_id,
                        "title": params.get("title").cloned().or_else(|| tool_call.and_then(|value| value.get("title").cloned())).unwrap_or_else(|| json!("Permission requested")),
                        "description": params.get("description").cloned().or_else(|| tool_call.and_then(|value| value.get("description").cloned())).unwrap_or(Value::Null),
                        "options": params.get("options").cloned().unwrap_or_else(|| json!([])),
                        "params": params,
                    }),
                    frame,
                );
                if let Some(event) = batch.events.last_mut() {
                    event.request_id = Some(request_id.to_string());
                }
            }
            "session/request_user_input" | "session/request_input" | "session/elicit" => {
                self.emit_event(
                    &mut batch,
                    AgentEventType::UserInputRequested,
                    self.native_session_id.clone(),
                    string_at(params, &["turnId", "turn_id"]),
                    string_at(params, &["itemId", "item_id"]),
                    json!({
                        "requestId": request_id.unwrap_or("unknown-request"),
                        "params": params,
                    }),
                    frame,
                );
                if let Some(event) = batch.events.last_mut() {
                    event.request_id = request_id.map(str::to_string);
                }
            }
            method
                if method.starts_with("terminal/")
                    || method.starts_with("fs/")
                    || method.contains("mcp") =>
            {
                batch.extend(self.tool_request(method, params, request_id, frame));
            }
            _ => batch.extend(self.unknown(frame, "agent-request")),
        }
        batch
    }

    fn map_update(&mut self, update: &Value, params: &Value, frame: &Value) -> ProjectionBatch {
        let tag = update
            .get("sessionUpdate")
            .or_else(|| update.get("session_update"))
            .or_else(|| update.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("");
        match tag {
            "user_message_chunk" | "user-message-chunk" | "user_message" => self.map_content_chunk(
                update,
                params,
                frame,
                AgentItemType::UserMessage,
                AgentContentChannel::Assistant,
            ),
            "agent_message_chunk" | "agent-message-chunk" | "assistant_message_delta" => self
                .map_content_chunk(
                    update,
                    params,
                    frame,
                    AgentItemType::AssistantMessage,
                    AgentContentChannel::Assistant,
                ),
            "agent_thought_chunk" | "agent-thought-chunk" | "reasoning_delta" | "thought_chunk" => {
                self.map_content_chunk(
                    update,
                    params,
                    frame,
                    AgentItemType::Reasoning,
                    AgentContentChannel::Reasoning,
                )
            }
            "tool_call" | "tool-call" | "tool_use" | "tool_use_start" => {
                self.map_tool_call(update, params, frame)
            }
            "tool_call_update" | "tool-call-update" | "tool_result" | "tool_use_update" => {
                self.map_tool_update(update, params, frame)
            }
            "plan" | "plan_update" | "plan-updated" | "plan_updated" => {
                self.map_plan(update, params, frame)
            }
            "tasks" | "task_update" | "tasks_updated" | "todo" | "todo_update" => {
                self.map_tasks(update, params, frame)
            }
            "available_commands_update" | "commands_updated" | "custom_commands" => {
                self.map_commands(update, frame)
            }
            "current_mode_update" | "mode_update" | "mode_changed" => {
                let mut batch = ProjectionBatch::new();
                let mode = update
                    .get("currentModeId")
                    .or_else(|| update.get("current_mode_id"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let turn_id = string_at(update, &["turnId", "turn_id"])
                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
                self.emit_event(
                    &mut batch,
                    AgentEventType::SessionConfigUpdated,
                    self.native_session_id.clone(),
                    turn_id,
                    None,
                    json!({ "mode": mode, "update": update }),
                    frame,
                );
                batch
            }
            "config_option_update" | "config_options_update" | "session_config_updated" => {
                self.map_config_update(update, frame)
            }
            "session_info_update" | "session_state_changed" | "state_changed" => {
                let mut batch = ProjectionBatch::new();
                let turn_id = string_at(update, &["turnId", "turn_id"])
                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
                self.emit_event(
                    &mut batch,
                    AgentEventType::SessionStateChanged,
                    self.native_session_id.clone(),
                    turn_id,
                    None,
                    json!({ "update": update }),
                    frame,
                );
                batch
            }
            "usage_update" | "usage_updated" | "usage" => {
                let mut batch = ProjectionBatch::new();
                let turn_id = string_at(update, &["turnId", "turn_id"])
                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
                self.emit_event(
                    &mut batch,
                    AgentEventType::UsageUpdated,
                    self.native_session_id.clone(),
                    turn_id,
                    None,
                    json!({ "usage": update }),
                    frame,
                );
                batch
            }
            "rate_limits_update" | "rate_limits_updated" | "rate_limit" => {
                let mut batch = ProjectionBatch::new();
                let turn_id = string_at(update, &["turnId", "turn_id"])
                    .or_else(|| string_at(params, &["turnId", "turn_id"]));
                self.emit_event(
                    &mut batch,
                    AgentEventType::RateLimitsUpdated,
                    self.native_session_id.clone(),
                    turn_id,
                    None,
                    json!({ "rateLimits": update }),
                    frame,
                );
                batch
            }
            _ => self.unknown(frame, "session-update"),
        }
    }

    fn map_content_chunk(
        &mut self,
        update: &Value,
        params: &Value,
        frame: &Value,
        item_type: AgentItemType,
        channel: AgentContentChannel,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let turn_id = string_at(update, &["turnId", "turn_id"])
            .or_else(|| string_at(params, &["turnId", "turn_id"]))
            .or_else(|| self.active_turn_id.clone());
        if let Some(turn_id) = turn_id.clone() {
            if self.active_turn_id.is_none() {
                self.start_turn(&mut batch, turn_id.clone(), frame);
            }
            self.active_turn_id = Some(turn_id);
        }
        let item_id = string_at(update, &["messageId", "message_id", "itemId", "item_id"])
            .or_else(|| string_at(params, &["messageId", "message_id", "itemId", "item_id"]))
            .unwrap_or_else(|| format!("{}-item-{}", self.namespace, self.sequence + 1));
        let new_item = !self.item_index.contains_key(&item_id);
        self.ensure_item(&item_id, item_type, turn_id.clone(), None);
        if new_item {
            self.emit_item(
                &mut batch,
                AgentEventType::ItemStarted,
                &item_id,
                turn_id.clone(),
                json!({ "item": update }),
            );
        }
        let content = update
            .get("content")
            .or_else(|| update.get("delta"))
            .cloned()
            .unwrap_or(Value::Null);
        let blocks = content_blocks(&content);
        let mut text_values = Vec::new();
        for block in blocks {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                text_values.push(text.to_string());
                self.append_content(
                    &item_id,
                    AgentContent {
                        channel,
                        text: text.to_string(),
                        mime_type: block
                            .get("mimeType")
                            .or_else(|| block.get("mime_type"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    },
                );
            } else if let Some(text) = block.as_str() {
                text_values.push(text.to_string());
                self.append_content(
                    &item_id,
                    AgentContent {
                        channel,
                        text: text.to_string(),
                        mime_type: None,
                    },
                );
            } else {
                self.touch_item_metadata(&item_id, json!({ "content": block }));
            }
        }
        let delta = text_values.concat();
        if !delta.is_empty() {
            self.emit_event(
                &mut batch,
                AgentEventType::ContentDelta,
                self.native_session_id.clone(),
                turn_id.clone(),
                Some(item_id.clone()),
                json!({ "channel": serde_json::to_value(channel).unwrap_or(Value::Null), "text": delta }),
                frame,
            );
        }
        let event_type = if new_item {
            AgentEventType::ItemUpdated
        } else {
            AgentEventType::ItemUpdated
        };
        self.emit_item(
            &mut batch,
            event_type,
            &item_id,
            turn_id,
            json!({ "item": update }),
        );
        batch
    }

    fn map_tool_call(&mut self, update: &Value, params: &Value, frame: &Value) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let tool_id = string_at(update, &["toolCallId", "tool_call_id", "id"])
            .or_else(|| string_at(params, &["toolCallId", "tool_call_id"]))
            .unwrap_or_else(|| format!("{}-tool-{}", self.namespace, self.sequence + 1));
        let turn_id = string_at(update, &["turnId", "turn_id"])
            .or_else(|| string_at(params, &["turnId", "turn_id"]))
            .or_else(|| self.active_turn_id.clone());
        if let Some(turn_id) = turn_id.clone() {
            self.active_turn_id = Some(turn_id);
        }
        let item_type = item_type_for_tool(update);
        self.ensure_item(&tool_id, item_type, turn_id.clone(), Some(update.clone()));
        self.emit_item(
            &mut batch,
            AgentEventType::ItemStarted,
            &tool_id,
            turn_id.clone(),
            json!({ "item": update, "toolCallId": tool_id }),
        );
        self.emit_item(
            &mut batch,
            AgentEventType::ItemUpdated,
            &tool_id,
            turn_id.clone(),
            json!({ "item": update }),
        );
        let maps_followup = update.get("content").is_some() || update.get("status").is_some();
        if maps_followup {
            batch.extend(self.map_tool_update(update, params, frame));
        }
        if !maps_followup && is_subagent_update(update) {
            self.emit_event(
                &mut batch,
                AgentEventType::ChildrenUpdated,
                self.native_session_id.clone(),
                turn_id,
                Some(tool_id),
                json!({ "child": child_identity(update) }),
                frame,
            );
        }
        batch
    }

    fn map_tool_update(
        &mut self,
        update: &Value,
        params: &Value,
        frame: &Value,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let tool_id = string_at(update, &["toolCallId", "tool_call_id", "id"])
            .or_else(|| string_at(params, &["toolCallId", "tool_call_id"]))
            .unwrap_or_else(|| format!("{}-tool-{}", self.namespace, self.sequence + 1));
        let turn_id = string_at(update, &["turnId", "turn_id"])
            .or_else(|| string_at(params, &["turnId", "turn_id"]))
            .or_else(|| self.active_turn_id.clone());
        let item_type = item_type_for_tool(update);
        let new_item = !self.item_index.contains_key(&tool_id);
        self.ensure_item(&tool_id, item_type, turn_id.clone(), Some(update.clone()));
        if new_item {
            self.emit_item(
                &mut batch,
                AgentEventType::ItemStarted,
                &tool_id,
                turn_id.clone(),
                json!({ "item": update }),
            );
        }
        let content = update.get("content").cloned().unwrap_or(Value::Null);
        let mut file_index = 0usize;
        for block in content_blocks(&content) {
            let block_type = block.get("type").and_then(Value::as_str).unwrap_or("");
            if is_diff_block(block_type, &block) {
                let file_id = string_at(&block, &["id", "fileId", "file_id"])
                    .unwrap_or_else(|| format!("{tool_id}:file:{file_index}"));
                file_index += 1;
                self.ensure_item(
                    &file_id,
                    AgentItemType::FileChange,
                    turn_id.clone(),
                    Some(json!({ self.namespace.clone() + ".fileChange": block.clone() })),
                );
                self.touch_item_metadata(&file_id, json!({ "fileChange": block.clone() }));
                self.emit_item(
                    &mut batch,
                    AgentEventType::ItemStarted,
                    &file_id,
                    turn_id.clone(),
                    json!({ "item": block, "parentToolCallId": tool_id }),
                );
                self.emit_item(
                    &mut batch,
                    AgentEventType::ItemCompleted,
                    &file_id,
                    turn_id.clone(),
                    json!({ "item": block, "parentToolCallId": tool_id }),
                );
            } else if let Some(text) = tool_output_text(&block) {
                self.append_content(
                    &tool_id,
                    AgentContent {
                        channel: AgentContentChannel::CommandOutput,
                        text: text.clone(),
                        mime_type: block
                            .get("mimeType")
                            .or_else(|| block.get("mime_type"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    },
                );
                self.emit_event(
                    &mut batch,
                    AgentEventType::ContentDelta,
                    self.native_session_id.clone(),
                    turn_id.clone(),
                    Some(tool_id.clone()),
                    json!({ "channel": "command-output", "text": text }),
                    frame,
                );
            } else {
                self.touch_item_metadata(&tool_id, json!({ "toolContent": block }));
            }
        }
        let status =
            string_at(update, &["status", "state"]).unwrap_or_else(|| "updated".to_string());
        let completed = matches!(
            status.as_str(),
            "completed" | "complete" | "failed" | "cancelled" | "canceled"
        );
        self.emit_item(
            &mut batch,
            if completed {
                AgentEventType::ItemCompleted
            } else {
                AgentEventType::ItemUpdated
            },
            &tool_id,
            turn_id.clone(),
            json!({ "item": update, "status": status }),
        );
        if is_subagent_update(update) {
            self.emit_event(
                &mut batch,
                AgentEventType::ChildrenUpdated,
                self.native_session_id.clone(),
                turn_id,
                Some(tool_id),
                json!({ "child": child_identity(update) }),
                frame,
            );
        }
        batch
    }

    fn map_plan(&mut self, update: &Value, params: &Value, frame: &Value) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let turn_id = string_at(update, &["turnId", "turn_id"])
            .or_else(|| string_at(params, &["turnId", "turn_id"]))
            .or_else(|| self.active_turn_id.clone());
        let id = string_at(update, &["planId", "plan_id", "id"])
            .unwrap_or_else(|| format!("{}-plan", self.namespace));
        self.ensure_item(
            &id,
            AgentItemType::Plan,
            turn_id.clone(),
            Some(json!({ self.namespace.clone() + ".plan": update.clone() })),
        );
        self.touch_item_metadata(&id, json!({ "plan": update }));
        self.emit_event(
            &mut batch,
            AgentEventType::PlanUpdated,
            self.native_session_id.clone(),
            turn_id.clone(),
            Some(id.clone()),
            json!({ "plan": update }),
            frame,
        );
        self.emit_item(
            &mut batch,
            AgentEventType::ItemUpdated,
            &id,
            turn_id,
            json!({ "item": update }),
        );
        batch
    }

    fn map_tasks(&mut self, update: &Value, params: &Value, _frame: &Value) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let turn_id = string_at(update, &["turnId", "turn_id"])
            .or_else(|| string_at(params, &["turnId", "turn_id"]))
            .or_else(|| self.active_turn_id.clone());
        let id = string_at(
            update,
            &["taskListId", "task_list_id", "todoId", "todo_id", "id"],
        )
        .unwrap_or_else(|| format!("{}-tasks", self.namespace));
        self.ensure_item(
            &id,
            AgentItemType::TaskList,
            turn_id.clone(),
            Some(json!({ self.namespace.clone() + ".tasks": update.clone() })),
        );
        self.touch_item_metadata(&id, json!({ "tasks": update }));
        self.emit_event(
            &mut batch,
            AgentEventType::TasksUpdated,
            self.native_session_id.clone(),
            turn_id.clone(),
            Some(id.clone()),
            json!({ "tasks": update }),
            _frame,
        );
        self.emit_item(
            &mut batch,
            AgentEventType::ItemUpdated,
            &id,
            turn_id,
            json!({ "item": update }),
        );
        batch
    }

    fn map_commands(&mut self, update: &Value, frame: &Value) -> ProjectionBatch {
        let commands_value = update
            .get("availableCommands")
            .or_else(|| update.get("available_commands"))
            .or_else(|| update.get("commands"))
            .cloned()
            .unwrap_or_else(|| json!([]));
        self.capabilities.commands = parse_commands(&commands_value, &self.namespace);
        self.capabilities.revision = self.capabilities.revision.saturating_add(1);
        let mut batch = ProjectionBatch::new();
        self.emit_event(
            &mut batch,
            AgentEventType::SessionConfigUpdated,
            self.native_session_id.clone(),
            None,
            None,
            json!({ "commands": commands_value }),
            frame,
        );
        batch
    }

    fn map_config_update(&mut self, update: &Value, frame: &Value) -> ProjectionBatch {
        let options = update
            .get("configOptions")
            .or_else(|| update.get("config_options"))
            .or_else(|| update.get("sessionConfigOptions"))
            .cloned()
            .unwrap_or_else(|| json!([]));
        self.capabilities.config_options = parse_config_options(&options, &self.namespace);
        self.capabilities.revision = self.capabilities.revision.saturating_add(1);
        self.session_config_event(update, frame)
    }

    fn session_config_event(&mut self, value: &Value, frame: &Value) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        self.emit_event(
            &mut batch,
            AgentEventType::SessionConfigUpdated,
            self.native_session_id.clone(),
            None,
            None,
            json!({ "config": value }),
            frame,
        );
        batch
    }

    fn update_session_capabilities(&mut self, value: &Value) {
        let Some(options) = value
            .get("configOptions")
            .or_else(|| value.get("config_options"))
            .or_else(|| value.get("sessionConfigOptions"))
        else {
            return;
        };
        if !options.is_array() {
            return;
        }
        self.capabilities.config_options = parse_config_options(options, &self.namespace);
        self.capabilities.revision = self.capabilities.revision.saturating_add(1);
    }

    fn tool_request(
        &mut self,
        method: &str,
        params: &Value,
        request_id: Option<&str>,
        frame: &Value,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let id = string_at(
            params,
            &[
                "toolCallId",
                "tool_call_id",
                "terminalId",
                "terminal_id",
                "id",
            ],
        )
        .unwrap_or_else(|| {
            format!(
                "{}-{}-{}",
                self.namespace,
                method.replace('/', "-"),
                self.sequence + 1
            )
        });
        let item_type = if method.contains("mcp") {
            AgentItemType::McpTool
        } else if method.starts_with("terminal/") {
            AgentItemType::Command
        } else {
            AgentItemType::FileChange
        };
        self.ensure_item(
            &id,
            item_type,
            self.active_turn_id.clone(),
            Some(params.clone()),
        );
        self.emit_item(
            &mut batch,
            AgentEventType::ItemStarted,
            &id,
            self.active_turn_id.clone(),
            json!({ "method": method, "params": params }),
        );
        self.emit_event(
            &mut batch,
            AgentEventType::ContentDelta,
            self.native_session_id.clone(),
            self.active_turn_id.clone(),
            Some(id),
            json!({ "channel": "command-output", "request": params }),
            frame,
        );
        if let Some(request_id) = request_id {
            for event in &mut batch.events {
                event.request_id = Some(request_id.to_string());
            }
        }
        batch
    }

    fn tool_request_response(
        &mut self,
        method: &str,
        params: &Value,
        result: &Value,
        _frame: &Value,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let id = string_at(
            params,
            &[
                "toolCallId",
                "tool_call_id",
                "terminalId",
                "terminal_id",
                "id",
            ],
        )
        .unwrap_or_else(|| format!("{}-{}", self.namespace, method.replace('/', "-")));
        self.touch_item_metadata(&id, json!({ "result": result }));
        self.emit_item(
            &mut batch,
            AgentEventType::ItemCompleted,
            &id,
            self.active_turn_id.clone(),
            json!({ "method": method, "result": result }),
        );
        batch
    }

    fn request_resolved(
        &mut self,
        event_type: AgentEventType,
        request_id: Option<&str>,
        result: &Value,
        frame: &Value,
    ) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        self.emit_event(
            &mut batch,
            event_type,
            self.native_session_id.clone(),
            self.active_turn_id.clone(),
            request_id.map(str::to_string),
            json!({ "requestId": request_id.unwrap_or("unknown-request"), "result": result }),
            frame,
        );
        if let Some(event) = batch.events.last_mut() {
            event.request_id = request_id.map(str::to_string);
        }
        batch
    }

    fn start_turn(&mut self, batch: &mut ProjectionBatch, turn_id: String, frame: &Value) {
        self.active_turn_id = Some(turn_id.clone());
        self.emit_turn(batch, &turn_id, "started", frame);
    }

    fn emit_turn(
        &mut self,
        batch: &mut ProjectionBatch,
        turn_id: &str,
        state: &str,
        frame: &Value,
    ) {
        let event_type = match state {
            "started" => AgentEventType::TurnStarted,
            "interrupted" => AgentEventType::TurnInterrupted,
            _ => AgentEventType::TurnCompleted,
        };
        self.emit_event(
            batch,
            event_type,
            self.native_session_id.clone(),
            Some(turn_id.to_string()),
            None,
            json!({ "turnId": turn_id, "state": state }),
            frame,
        );
    }

    fn finish_active_items(&mut self, batch: &mut ProjectionBatch, interrupted: bool) {
        let ids = self.active_items.clone();
        for id in ids {
            self.emit_item(
                batch,
                AgentEventType::ItemCompleted,
                &id,
                self.active_turn_id.clone(),
                json!({ "completed": true, "interrupted": interrupted }),
            );
        }
        self.active_items.clear();
        self.active_item_set.clear();
    }

    fn prompt_identity(&self, params: &Value) -> (String, Option<String>) {
        let turn_id = string_at(params, &["turnId", "turn_id"]);
        let id = string_at(params, &["itemId", "item_id", "messageId", "message_id"])
            .unwrap_or_else(|| {
                turn_id
                    .as_deref()
                    .map(|turn| format!("{}:user", turn))
                    .unwrap_or_else(|| format!("{}-user-{}", self.namespace, self.sequence + 1))
            });
        (id, turn_id)
    }

    fn ensure_item(
        &mut self,
        id: &str,
        item_type: AgentItemType,
        turn_id: Option<String>,
        metadata: Option<Value>,
    ) {
        if let Some(index) = self.item_index.get(id).copied() {
            if let Some(metadata) = metadata {
                self.merge_item_metadata(index, metadata);
            }
            return;
        }
        let provider_metadata =
            metadata.map(|value| BTreeMap::from([(format!("{}.item", self.namespace), value)]));
        let index = self.items.len();
        self.items.push(AgentItem {
            id: id.to_string(),
            item_type,
            turn_id: turn_id.clone(),
            content: Vec::new(),
            provider_metadata,
        });
        self.item_index.insert(id.to_string(), index);
        if self.active_item_set.insert(id.to_string()) {
            self.active_items.push(id.to_string());
        }
    }

    fn merge_item_metadata(&mut self, index: usize, value: Value) {
        let item = &mut self.items[index];
        let metadata = item.provider_metadata.get_or_insert_with(BTreeMap::new);
        metadata.insert(format!("{}.item", self.namespace), value);
    }

    fn touch_item_metadata(&mut self, id: &str, value: Value) {
        if let Some(index) = self.item_index.get(id).copied() {
            self.merge_item_metadata(index, value);
        }
    }

    fn append_content(&mut self, id: &str, content: AgentContent) {
        if let Some(index) = self.item_index.get(id).copied() {
            self.items[index].content.push(content);
        }
    }

    fn emit_item(
        &mut self,
        batch: &mut ProjectionBatch,
        event_type: AgentEventType,
        id: &str,
        turn_id: Option<String>,
        payload: Value,
    ) {
        self.emit_event(
            batch,
            event_type,
            self.native_session_id.clone(),
            turn_id,
            Some(id.to_string()),
            payload,
            &Value::Null,
        );
        if let Some(index) = self.item_index.get(id).copied() {
            batch.items.push(self.items[index].clone());
        }
        if event_type == AgentEventType::ItemCompleted {
            self.active_item_set.remove(id);
            self.active_items.retain(|active_id| active_id != id);
        }
    }

    fn emit_event(
        &mut self,
        batch: &mut ProjectionBatch,
        event_type: AgentEventType,
        native_session_id: Option<String>,
        turn_id: Option<String>,
        item_id: Option<String>,
        payload: Value,
        frame: &Value,
    ) {
        self.sequence = self.sequence.saturating_add(1);
        self.timestamp_ms =
            frame_timestamp(frame).unwrap_or_else(|| self.timestamp_ms.saturating_add(1));
        let method = frame
            .get("method")
            .filter(|value| !value.is_null())
            .cloned();
        let category = frame
            .get("params")
            .and_then(|params| params.get("update"))
            .and_then(|update| update.get("sessionUpdate"))
            .or_else(|| frame.get("sessionUpdate"))
            .filter(|value| !value.is_null())
            .cloned();
        let provider_metadata = if method.is_some() || category.is_some() {
            Some(BTreeMap::from([(
                format!("{}.frame", self.namespace),
                json!({ "method": method, "category": category }),
            )]))
        } else {
            None
        };
        batch.events.push(AgentEvent {
            event_type,
            owned_id: self.context.owned_id.clone(),
            provider: self.provider,
            provider_instance_id: self.context.provider_instance_id.clone(),
            generation: self.context.generation,
            sequence: self.sequence,
            timestamp_ms: self.timestamp_ms,
            native_session_id: native_session_id.or_else(|| self.native_session_id.clone()),
            turn_id,
            item_id,
            request_id: None,
            payload: object_payload(payload),
            provider_metadata,
            raw_frame_reference: raw_frame_reference(frame),
        });
    }

    fn unknown(&mut self, frame: &Value, category: &str) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        let mut metadata = BTreeMap::new();
        metadata.insert(format!("{}.unknown", self.namespace), frame.clone());
        self.emit_event(
            &mut batch,
            AgentEventType::RuntimeWarning,
            self.native_session_id.clone(),
            self.active_turn_id.clone(),
            None,
            json!({ "category": category, "frame": frame }),
            frame,
        );
        if let Some(event) = batch.events.last_mut() {
            event.provider_metadata = Some(metadata);
        }
        batch
    }

    fn warning(&mut self, category: &str, payload: Value) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        self.emit_event(
            &mut batch,
            AgentEventType::RuntimeWarning,
            self.native_session_id.clone(),
            self.active_turn_id.clone(),
            None,
            json!({ "category": category, "details": payload }),
            &Value::Null,
        );
        batch
    }

    fn error_from_frame(&mut self, frame: &Value, operation: &str) -> ProjectionBatch {
        let mut batch = ProjectionBatch::new();
        self.emit_event(
            &mut batch,
            AgentEventType::RuntimeError,
            self.native_session_id.clone(),
            self.active_turn_id.clone(),
            None,
            json!({ "operation": operation, "error": frame.get("error").cloned().unwrap_or(Value::Null) }),
            frame,
        );
        batch
    }
}

pub fn empty_capabilities(provider: AgentConversationProvider) -> AgentCapabilities {
    AgentCapabilities {
        revision: 0,
        provider,
        implementation: AgentImplementation {
            name: "ACP adapter".to_string(),
            version: "unknown".to_string(),
        },
        session: AgentSessionCapabilities {
            list: false,
            load: false,
            resume: false,
            close: false,
            steering: false,
        },
        prompt: AgentPromptCapabilities {
            text: true,
            image: false,
            embedded_context: false,
            resource_links: false,
        },
        interaction: AgentInteractionCapabilities {
            permissions: false,
            structured_user_input: false,
            tool_terminals: false,
            plans: false,
            tasks: false,
            subagents: false,
        },
        config_options: Vec::new(),
        commands: Vec::new(),
    }
}

pub fn capabilities_from_initialize(
    provider: AgentConversationProvider,
    result: &Value,
    namespace: &str,
) -> AgentCapabilities {
    let mut capabilities = empty_capabilities(provider);
    capabilities.revision = 1;
    let implementation = result.get("agentInfo").or_else(|| result.get("agent_info"));
    capabilities.implementation = AgentImplementation {
        name: implementation
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("ACP adapter")
            .to_string(),
        version: implementation
            .and_then(|value| value.get("version"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
    };
    capabilities.session.list = advertised(
        result,
        &[
            &["agentCapabilities", "sessionCapabilities", "list"],
            &["agentCapabilities", "session_capabilities", "list"],
        ],
    );
    capabilities.session.load = advertised(
        result,
        &[
            &["agentCapabilities", "loadSession"],
            &["agentCapabilities", "load_session"],
        ],
    );
    capabilities.session.resume = advertised(
        result,
        &[
            &["agentCapabilities", "sessionCapabilities", "resume"],
            &["agentCapabilities", "session_capabilities", "resume"],
        ],
    );
    capabilities.session.close = advertised(
        result,
        &[
            &["agentCapabilities", "sessionCapabilities", "close"],
            &["agentCapabilities", "session_capabilities", "close"],
        ],
    );
    capabilities.session.steering = advertised(
        result,
        &[
            &["agentCapabilities", "sessionCapabilities", "steering"],
            &["agentCapabilities", "session_capabilities", "steering"],
        ],
    );
    capabilities.prompt.image = advertised(
        result,
        &[
            &["agentCapabilities", "promptCapabilities", "image"],
            &["agentCapabilities", "prompt_capabilities", "image"],
        ],
    );
    capabilities.prompt.embedded_context = advertised(
        result,
        &[
            &["agentCapabilities", "promptCapabilities", "embeddedContext"],
            &[
                "agentCapabilities",
                "prompt_capabilities",
                "embedded_context",
            ],
        ],
    );
    capabilities.prompt.resource_links = advertised(
        result,
        &[
            &["agentCapabilities", "promptCapabilities", "resourceLinks"],
            &["agentCapabilities", "prompt_capabilities", "resource_links"],
        ],
    );
    capabilities.interaction.permissions = advertised(
        result,
        &[
            &["agentCapabilities", "permissions"],
            &["agentCapabilities", "permission"],
            &["agentCapabilities", "sessionCapabilities", "permissions"],
        ],
    );
    capabilities.interaction.structured_user_input = advertised(
        result,
        &[
            &["agentCapabilities", "structuredUserInput"],
            &["agentCapabilities", "structured_user_input"],
            &["agentCapabilities", "userInput"],
        ],
    );
    capabilities.interaction.tool_terminals = advertised(
        result,
        &[
            &["agentCapabilities", "toolTerminals"],
            &["agentCapabilities", "tool_terminals"],
            &["agentCapabilities", "terminal"],
        ],
    );
    capabilities.interaction.plans = advertised(
        result,
        &[
            &["agentCapabilities", "plans"],
            &["agentCapabilities", "plan"],
            &["agentCapabilities", "planUpdates"],
        ],
    );
    capabilities.interaction.tasks = advertised(
        result,
        &[
            &["agentCapabilities", "tasks"],
            &["agentCapabilities", "taskUpdates"],
            &["agentCapabilities", "todos"],
        ],
    );
    capabilities.interaction.subagents = advertised(
        result,
        &[
            &["agentCapabilities", "subagents"],
            &["agentCapabilities", "subAgents"],
            &["agentCapabilities", "children"],
        ],
    );
    let options = result
        .get("sessionConfigOptions")
        .or_else(|| result.get("session_config_options"))
        .or_else(|| result.get("configOptions"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    capabilities.config_options = parse_config_options(&options, namespace);
    let commands = result
        .get("commands")
        .or_else(|| result.get("availableCommands"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    capabilities.commands = parse_commands(&commands, namespace);
    capabilities
}

pub fn parse_config_options(value: &Value, namespace: &str) -> Vec<AgentConfigOption> {
    let Some(values) = value.as_array() else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|raw| {
            let id = string_at(raw, &["id", "configId", "config_id"])?;
            let label = string_at(raw, &["label", "name", "title"]).unwrap_or_else(|| id.clone());
            let category = raw
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let value = raw
                .get("value")
                .or_else(|| raw.get("currentValue"))
                .or_else(|| raw.get("current_value"))
                .or_else(|| raw.get("current"))
                .cloned()
                .unwrap_or(Value::Null);
            let choices_value = raw
                .get("choices")
                .or_else(|| raw.get("options"))
                .or_else(|| raw.pointer("/kind/options"));
            let choices = choices_value.and_then(parse_choices);
            let mut provider_metadata = BTreeMap::new();
            provider_metadata.insert(format!("{}.config", namespace), raw.clone());
            Some(AgentConfigOption {
                id,
                label,
                category,
                description: string_at(raw, &["description", "help"]),
                value,
                choices,
                provider_metadata: Some(provider_metadata),
            })
        })
        .collect()
}

pub fn parse_commands(value: &Value, namespace: &str) -> Vec<AgentCommandDescriptor> {
    let Some(values) = value.as_array() else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|raw| {
            let id = string_at(raw, &["id", "name", "command"])?;
            let mut provider_metadata = BTreeMap::new();
            provider_metadata.insert(format!("{}.command", namespace), raw.clone());
            Some(AgentCommandDescriptor {
                id: id.clone(),
                label: string_at(raw, &["label", "name", "title"]).unwrap_or(id),
                description: string_at(raw, &["description", "help"]),
                input_hint: string_at(raw, &["inputHint", "input_hint", "argumentHint"]),
                provider_metadata: Some(provider_metadata),
            })
        })
        .collect()
}

fn parse_choices(value: &Value) -> Option<Vec<AgentConfigOptionChoice>> {
    let values = value.as_array()?;
    let mut choices = Vec::new();
    for raw in values {
        if let Some(grouped) = raw.get("options") {
            if let Some(group_choices) = parse_choices(grouped) {
                choices.extend(group_choices);
                continue;
            }
        }
        if raw.is_string() || raw.is_number() || raw.is_boolean() {
            choices.push(AgentConfigOptionChoice {
                value: raw.clone(),
                label: raw.to_string().trim_matches('"').to_string(),
                description: None,
            });
            continue;
        }
        choices.push(AgentConfigOptionChoice {
            value: raw
                .get("value")
                .or_else(|| raw.get("id"))
                .cloned()
                .unwrap_or(Value::Null),
            label: string_at(raw, &["label", "name", "title"]).unwrap_or_else(|| raw.to_string()),
            description: string_at(raw, &["description", "help"]),
        });
    }
    Some(choices)
}

fn is_session_update_method(method: &str) -> bool {
    matches!(method, "session/update" | "session_update")
}

fn is_agent_request(method: &str) -> bool {
    matches!(
        method,
        "session/request_permission"
            | "session/permission_request"
            | "session/request_user_input"
            | "session/request_input"
            | "session/elicit"
    ) || method.starts_with("terminal/")
        || method.starts_with("fs/")
        || method.contains("mcp")
}

fn item_type_for_tool(update: &Value) -> AgentItemType {
    let kind = string_at(update, &["kind", "toolKind", "tool_kind", "name", "title"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    if kind.contains("subagent") || kind.contains("sub_agent") || is_subagent_update(update) {
        AgentItemType::Subagent
    } else if kind.contains("mcp") {
        AgentItemType::McpTool
    } else if kind.contains("web") || kind.contains("search") || kind == "fetch" {
        AgentItemType::WebSearch
    } else if kind.contains("review") {
        AgentItemType::Review
    } else if kind.contains("file")
        || kind.contains("diff")
        || matches!(kind.as_str(), "edit" | "delete" | "move")
    {
        AgentItemType::FileChange
    } else if kind.contains("command")
        || kind.contains("shell")
        || kind.contains("terminal")
        || matches!(kind.as_str(), "execute" | "run" | "bash" | "apply_patch")
    {
        AgentItemType::Command
    } else {
        AgentItemType::Unknown
    }
}

fn is_subagent_update(update: &Value) -> bool {
    let kind = string_at(update, &["kind", "toolKind", "tool_kind", "type"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    kind.contains("subagent")
        || kind.contains("sub_agent")
        || update.get("childSessionId").is_some()
        || update.get("child_session_id").is_some()
        || update.get("subagent").is_some()
        || update.get("subAgent").is_some()
}

fn child_identity(update: &Value) -> Value {
    json!({
        "childSessionId": update
            .get("childSessionId")
            .or_else(|| update.get("child_session_id"))
            .cloned()
            .unwrap_or(Value::Null),
        "parentToolCallId": update
            .get("parentToolCallId")
            .or_else(|| update.get("parent_tool_call_id"))
            .or_else(|| update.get("toolCallId"))
            .or_else(|| update.get("tool_call_id"))
            .cloned()
            .unwrap_or(Value::Null),
        "parentItemId": update
            .get("parentItemId")
            .or_else(|| update.get("parent_item_id"))
            .cloned()
            .unwrap_or(Value::Null),
        "label": update.get("label").cloned().unwrap_or(Value::Null),
    })
}

fn is_diff_block(block_type: &str, block: &Value) -> bool {
    matches!(block_type, "diff" | "file_change" | "fileChange" | "edit")
        || block.get("diff").is_some()
        || block.get("path").is_some() && block.get("operation").is_some()
}

fn prompt_payload(params: &Value) -> Value {
    params
        .get("prompt")
        .or_else(|| params.get("content"))
        .or_else(|| params.get("input"))
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn content_blocks(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(values) => values.clone(),
        Value::Null => Vec::new(),
        other => vec![other.clone()],
    }
}

fn tool_output_text(block: &Value) -> Option<String> {
    block
        .get("text")
        .or_else(|| block.get("output"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            block
                .get("content")
                .and_then(|content| content.get("text").or_else(|| content.get("output")))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn object_payload(value: Value) -> BTreeMap<String, Value> {
    match value {
        Value::Object(map) => map.into_iter().collect(),
        other => BTreeMap::from([("value".to_string(), other)]),
    }
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(string_value))
}

fn string_value(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string).or_else(|| {
        value
            .as_i64()
            .map(|number| number.to_string())
            .or_else(|| value.as_u64().map(|number| number.to_string()))
    })
}

fn request_id_string(value: &Value) -> Option<String> {
    string_value(value).or_else(|| (!value.is_null()).then(|| value.to_string()))
}

fn advertised(value: &Value, paths: &[&[&str]]) -> bool {
    paths.iter().any(|path| {
        let Some(value) = path
            .iter()
            .try_fold(value, |current, key| current.get(*key))
        else {
            return false;
        };
        value.as_bool().unwrap_or(!value.is_null())
    })
}

fn frame_timestamp(frame: &Value) -> Option<u128> {
    frame
        .get("timestampMs")
        .or_else(|| frame.get("timestamp_ms"))
        .or_else(|| frame.pointer("/params/timestampMs"))
        .and_then(|value| value.as_u64().map(u128::from))
}

fn raw_frame_reference(frame: &Value) -> Option<AgentRawFrameReference> {
    let value = frame
        .get("rawFrameReference")
        .or_else(|| frame.get("raw_frame_reference"))?;
    let id = value.get("id").and_then(Value::as_str)?.to_string();
    let redacted = value
        .get("redacted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    redacted.then_some(AgentRawFrameReference { id, redacted })
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
