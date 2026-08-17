use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::process::{validated_conversation_cwd, ConversationProcess};
use super::protocol::{
    AgentConversationConnection, AgentConversationPayload, AgentConversationProvider,
    ApprovalState, ConversationConnectionState, ToolState, TurnState,
};
use super::{publish_payload, AgentConversationRegistry, ProviderCommand};

pub(crate) async fn run_provider(
    registry: AgentConversationRegistry,
    app: AppHandle,
    connection: AgentConversationConnection,
    cwd: String,
    commands: mpsc::UnboundedReceiver<ProviderCommand>,
) {
    let result = match connection.provider {
        AgentConversationProvider::Codex => {
            run_codex(&registry, &app, &connection, &cwd, commands).await
        }
        AgentConversationProvider::Claude => {
            run_claude(&registry, &app, &connection, &cwd, commands).await
        }
        AgentConversationProvider::Antigravity => {
            Err("Antigravity is handled by the ACP runtime".to_string())
        }
    };
    if let Err(message) = result {
        emit(
            &registry,
            &app,
            &connection,
            AgentConversationPayload::Error {
                code: "providerFailed".into(),
                message,
                recoverable: true,
            },
        );
        emit(
            &registry,
            &app,
            &connection,
            AgentConversationPayload::Connection {
                state: ConversationConnectionState::Failed,
                native_session_id: None,
            },
        );
    }
}

async fn write_json(process: &mut ConversationProcess, value: Value) -> Result<(), String> {
    let mut line = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    line.push(b'\n');
    process
        .stdin
        .write_all(&line)
        .await
        .map_err(|e| format!("Could not write to provider: {e}"))?;
    process
        .stdin
        .flush()
        .await
        .map_err(|e| format!("Could not flush provider input: {e}"))
}

fn emit(
    registry: &AgentConversationRegistry,
    app: &AppHandle,
    c: &AgentConversationConnection,
    payload: AgentConversationPayload,
) {
    publish_payload(registry, app, &c.owned_id, c.generation, payload);
}

async fn run_codex(
    registry: &AgentConversationRegistry,
    app: &AppHandle,
    c: &AgentConversationConnection,
    cwd: &str,
    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
) -> Result<(), String> {
    let cwd_path = validated_conversation_cwd(cwd)?;
    let mut process =
        ConversationProcess::spawn("codex", &["app-server".into()], &cwd_path, &c.owned_id)?;
    write_json(&mut process, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"mac-command-bar","version":env!("CARGO_PKG_VERSION")}}})).await?;
    write_json(
        &mut process,
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await?;
    let method = if c.native_session_id.is_some() {
        "thread/resume"
    } else {
        "thread/start"
    };
    let params = if let Some(id) = &c.native_session_id {
        json!({"threadId":id,"cwd":cwd})
    } else {
        json!({"cwd":cwd})
    };
    write_json(
        &mut process,
        json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}),
    )
    .await?;
    let mut thread_id = c.native_session_id.clone();
    let mut turn_id: Option<String> = None;
    let mut request_counter = 10u64;
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(ProviderCommand::SendMessage(text)) => {
                    let id = thread_id.clone().ok_or_else(|| "Codex thread is still connecting".to_string())?;
                    request_counter += 1;
                    write_json(&mut process, json!({"jsonrpc":"2.0","id":request_counter,"method":"turn/start","params":{"threadId":id,"input":[{"type":"text","text":text}]}})).await?;
                }
                Some(ProviderCommand::RespondApproval { request_id, decision }) => {
                    let decision = match decision { super::protocol::ApprovalDecision::Accept => "accept", super::protocol::ApprovalDecision::Decline => "decline" };
                    let id: Value = serde_json::from_str(&request_id).unwrap_or(Value::String(request_id));
                    write_json(&mut process, json!({"jsonrpc":"2.0","id":id,"result":{"decision":decision}})).await?;
                }
                Some(ProviderCommand::Stop) => if let (Some(thread), Some(turn)) = (&thread_id, &turn_id) {
                    request_counter += 1;
                    write_json(&mut process, json!({"jsonrpc":"2.0","id":request_counter,"method":"turn/interrupt","params":{"threadId":thread,"turnId":turn}})).await?;
                },
                Some(ProviderCommand::Close) | None => { process.stop().await; return Ok(()); }
            },
            line = process.stdout.next_line() => {
                let Some(line) = line.map_err(|e| format!("Could not read Codex output: {e}"))? else { return Err("Codex app-server exited".into()); };
                let value: Value = match serde_json::from_str(&line) { Ok(v) => v, Err(_) => continue };
                if value.get("id") == Some(&json!(2)) {
                    thread_id = value.pointer("/result/thread/id").and_then(Value::as_str).map(str::to_string);
                    emit(registry, app, c, AgentConversationPayload::Connection { state: ConversationConnectionState::Connected, native_session_id: thread_id.clone() });
                }
                normalize_codex(registry, app, c, &value, &mut turn_id);
            },
            line = process.stderr.next_line() => {
                if let Ok(Some(line)) = line { crate::debug_log::stderr_log!("mcb structured codex [{}]: {}", c.owned_id, line); }
            }
        }
    }
}

fn normalize_codex(
    registry: &AgentConversationRegistry,
    app: &AppHandle,
    c: &AgentConversationConnection,
    value: &Value,
    turn_id: &mut Option<String>,
) {
    let method = value.get("method").and_then(Value::as_str).unwrap_or("");
    let p = value.get("params").unwrap_or(&Value::Null);
    let item_id = || {
        p.pointer("/item/id")
            .and_then(Value::as_str)
            .unwrap_or("assistant")
            .to_string()
    };
    match method {
        "turn/started" => {
            *turn_id = p
                .pointer("/turn/id")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(id) = turn_id.clone() {
                emit(
                    registry,
                    app,
                    c,
                    AgentConversationPayload::Turn {
                        turn_id: id,
                        state: TurnState::Started,
                    },
                );
            }
        }
        "turn/completed" => {
            if let Some(id) = turn_id.take() {
                emit(
                    registry,
                    app,
                    c,
                    AgentConversationPayload::Turn {
                        turn_id: id,
                        state: TurnState::Completed,
                    },
                );
            }
        }
        "item/agentMessage/delta" => emit(
            registry,
            app,
            c,
            AgentConversationPayload::AssistantDelta {
                item_id: item_id(),
                delta: p
                    .get("delta")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
        ),
        "item/completed"
            if p.pointer("/item/type").and_then(Value::as_str) == Some("agentMessage") =>
        {
            emit(
                registry,
                app,
                c,
                AgentConversationPayload::AssistantMessage {
                    item_id: item_id(),
                    text: p
                        .pointer("/item/text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    completed: true,
                },
            )
        }
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => emit(
            registry,
            app,
            c,
            AgentConversationPayload::Approval {
                request_id: value.get("id").map(Value::to_string).unwrap_or_default(),
                state: ApprovalState::Requested,
                summary: p
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("Approval requested")
                    .to_string(),
            },
        ),
        "item/started" => {
            if let Some(kind) = p
                .pointer("/item/type")
                .and_then(Value::as_str)
                .filter(|k| *k != "agentMessage" && *k != "userMessage")
            {
                emit(
                    registry,
                    app,
                    c,
                    AgentConversationPayload::Tool {
                        item_id: item_id(),
                        name: kind.to_string(),
                        state: ToolState::Started,
                        summary: None,
                    },
                );
            }
        }
        _ => {}
    }
}

async fn run_claude(
    registry: &AgentConversationRegistry,
    app: &AppHandle,
    c: &AgentConversationConnection,
    cwd: &str,
    mut commands: mpsc::UnboundedReceiver<ProviderCommand>,
) -> Result<(), String> {
    let cwd_path = validated_conversation_cwd(cwd)?;
    let mut args = vec![
        "-p".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--include-partial-messages".into(),
        "--replay-user-messages".into(),
    ];
    if let Some(id) = &c.native_session_id {
        args.extend(["--resume".into(), id.clone()]);
    }
    let mut process = ConversationProcess::spawn("claude", &args, &cwd_path, &c.owned_id)?;
    let mut assistant_item_id: Option<String> = None;
    let mut assistant_text = String::new();
    let mut response_sequence = 0_u64;
    emit(
        registry,
        app,
        c,
        AgentConversationPayload::Connection {
            state: ConversationConnectionState::Connected,
            native_session_id: c.native_session_id.clone(),
        },
    );
    loop {
        tokio::select! {
            command=commands.recv()=>match command {
                Some(ProviderCommand::SendMessage(text))=>{
                    let (user_item_id, response_item_id) = next_claude_turn_ids(c.generation, &mut response_sequence);
                    assistant_item_id = Some(response_item_id);
                    assistant_text.clear();
                    emit(registry, app, c, AgentConversationPayload::UserMessage {
                        item_id: user_item_id,
                        text: text.clone(),
                        completed: true,
                        attachment_ids: Vec::new(),
                    });
                    write_json(&mut process,json!({"type":"user","message":{"role":"user","content":text}})).await?
                },
                Some(ProviderCommand::Stop)=>write_json(&mut process,json!({"type":"control_request","request_id":format!("stop-{}",c.generation),"request":{"subtype":"interrupt"}})).await?,
                Some(ProviderCommand::RespondApproval{request_id,decision})=>write_json(&mut process,json!({"type":"control_response","response":{"subtype":"success","request_id":request_id,"response":{"behavior":match decision { super::protocol::ApprovalDecision::Accept=>"allow",super::protocol::ApprovalDecision::Decline=>"deny"}}}})).await?,
                Some(ProviderCommand::Close)|None=>{process.stop().await;return Ok(());}
            },
            line=process.stdout.next_line()=>{ let Some(line)=line.map_err(|e|e.to_string())? else{return Err("Claude exited".into())}; if let Ok(v)=serde_json::from_str::<Value>(&line){normalize_claude(registry,app,c,&v,&mut assistant_item_id,&mut assistant_text);} },
            line=process.stderr.next_line()=>{if let Ok(Some(line))=line{crate::debug_log::stderr_log!("mcb structured claude [{}]: {}",c.owned_id,line);}}
        }
    }
}

fn next_claude_turn_ids(generation: u64, response_sequence: &mut u64) -> (String, String) {
    *response_sequence = response_sequence.saturating_add(1);
    (
        format!("claude-user-{generation}-{response_sequence}"),
        format!("claude-response-{generation}-{response_sequence}"),
    )
}

fn completed_claude_text(streamed_text: &str, final_text: String) -> String {
    if streamed_text.is_empty() {
        final_text
    } else {
        streamed_text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{completed_claude_text, next_claude_turn_ids};

    #[test]
    fn claude_responses_receive_distinct_fallback_item_ids() {
        let mut sequence = 0;

        let (first_user, first) = next_claude_turn_ids(4, &mut sequence);
        let (second_user, second) = next_claude_turn_ids(4, &mut sequence);

        assert_eq!(first_user, "claude-user-4-1");
        assert_eq!(first, "claude-response-4-1");
        assert_eq!(second_user, "claude-user-4-2");
        assert_eq!(second, "claude-response-4-2");
        assert_ne!(first, second);
    }

    #[test]
    fn claude_final_snapshot_cannot_erase_streamed_sentences() {
        let streamed = "First sentence. Second sentence.";
        let incomplete_final_snapshot = "Second sentence.".to_string();

        assert_eq!(
            completed_claude_text(streamed, incomplete_final_snapshot),
            streamed
        );
    }
}

fn normalize_claude(
    registry: &AgentConversationRegistry,
    app: &AppHandle,
    c: &AgentConversationConnection,
    v: &Value,
    assistant_item_id: &mut Option<String>,
    assistant_text: &mut String,
) {
    match v.get("type").and_then(Value::as_str).unwrap_or("") {
        "system" => {
            if let Some(id) = v.get("session_id").and_then(Value::as_str) {
                emit(
                    registry,
                    app,
                    c,
                    AgentConversationPayload::Connection {
                        state: ConversationConnectionState::Connected,
                        native_session_id: Some(id.into()),
                    },
                );
            }
        }
        "stream_event" => {
            if v.pointer("/event/type").and_then(Value::as_str) == Some("message_start") {
                if let Some(item_id) = v.pointer("/event/message/id").and_then(Value::as_str) {
                    *assistant_item_id = Some(item_id.to_string());
                }
                assistant_text.clear();
            } else if v.pointer("/event/type").and_then(Value::as_str)
                == Some("content_block_delta")
            {
                if let Some(delta) = v.pointer("/event/delta/text").and_then(Value::as_str) {
                    assistant_text.push_str(delta);
                    emit(
                        registry,
                        app,
                        c,
                        AgentConversationPayload::AssistantDelta {
                            item_id: assistant_item_id
                                .clone()
                                .unwrap_or_else(|| format!("assistant-{}", c.generation)),
                            delta: delta.into(),
                        },
                    );
                }
            }
        }
        "assistant" => {
            let text = v
                .pointer("/message/content")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            let completed_text = completed_claude_text(assistant_text, text);
            if !completed_text.is_empty() {
                let item_id = v
                    .pointer("/message/id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| assistant_item_id.clone())
                    .unwrap_or_else(|| format!("assistant-{}", c.generation));
                emit(
                    registry,
                    app,
                    c,
                    AgentConversationPayload::AssistantMessage {
                        item_id,
                        text: completed_text,
                        completed: true,
                    },
                );
            }
        }
        "result" => {
            *assistant_item_id = None;
            assistant_text.clear();
            emit(
                registry,
                app,
                c,
                AgentConversationPayload::Turn {
                    turn_id: v
                        .get("session_id")
                        .and_then(Value::as_str)
                        .unwrap_or("turn")
                        .into(),
                    state: if v.get("is_error").and_then(Value::as_bool) == Some(true) {
                        TurnState::Failed
                    } else {
                        TurnState::Completed
                    },
                },
            )
        }
        "control_request" => emit(
            registry,
            app,
            c,
            AgentConversationPayload::Approval {
                request_id: v
                    .get("request_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                state: ApprovalState::Requested,
                summary: v
                    .pointer("/request/tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or("Tool approval requested")
                    .into(),
            },
        ),
        _ => {}
    }
}
