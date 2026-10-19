use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};

use super::execute_server_command;
use super::super::{PendingAttachment, ServerState, HISTORY_DOWNLOAD_CHUNK_BYTES,
    MAX_SERVER_FRAME_BYTES, MAX_WIRE_FRAME_BYTES, MESSAGE_CHANGES, OUTBOUND_FRAME_CAPACITY,
    PROTOCOL_VERSION, REPLAY_PAGE_BYTES};
use super::super::wire::{encode_history_chunk, encode_server_frame, ClientFrame, RemoteCommand, RemoteResponse, ServerFrame};

pub(in crate::agent_conversation::remote) async fn upgrade_remote_socket(
    State(state): State<ServerState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    let expected = format!("Bearer {}", state.token);
    if headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        != Some(expected.as_str())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    upgrade
        .max_message_size(MAX_WIRE_FRAME_BYTES)
        .on_upgrade(move |socket| serve_remote_socket(socket, state))
        .into_response()
}

pub(in crate::agent_conversation::remote) async fn next_outbound_frame(outbound: &mut mpsc::Receiver<ServerFrame>, history: &mut mpsc::Receiver<ServerFrame>) -> Option<ServerFrame> {
    loop {
        tokio::select! {
            biased;
            frame = outbound.recv(), if !outbound.is_closed() || !outbound.is_empty() => { if frame.is_some() { return frame; } }
            frame = history.recv(), if !history.is_closed() || !history.is_empty() => { if frame.is_some() { return frame; } }
            else => return None,
        }
    }
}

pub(in crate::agent_conversation::remote) async fn stream_history_download(state: &ServerState, id: u64, owned_id: &str, mut before_sequence: Option<i64>,
    history_outbound: &mpsc::Sender<ServerFrame>) -> Result<(), String> {
    let store = state.manager.store_handle();
    let facts = store.history_download_facts(owned_id, before_sequence).map_err(|e| e.to_string())?;
    let snapshot = state.manager.snapshot_since(owned_id, Some(i64::MAX))?.ok_or("Remote session was not found")?;
    let mut covered = facts.covered_bytes;
    let start = ServerFrame::HistoryDownloadStart { id, owned_id: owned_id.into(), low_sequence: facts.low_sequence,
        high_sequence: facts.high_sequence, row_count: facts.row_count, total_bytes: facts.total_bytes, covered_bytes: covered, snapshot };
    send_history_frame(history_outbound, start).await?;
    if let (Some(low), Some(high)) = (facts.low_sequence, facts.high_sequence) {
        loop {
            let rows = store.history_download_rows(owned_id, low, high, before_sequence, HISTORY_DOWNLOAD_CHUNK_BYTES, covered).map_err(|e| e.to_string())?;
            let Some(oldest) = rows.events.last().map(|row| row.seq) else { break; };
            covered = rows.covered_bytes;
            let events = rows.events.into_iter().map(crate::agent_conversation::manager::stored_event).collect::<Result<Vec<_>, _>>()?;
            send_history_frame(history_outbound, ServerFrame::HistoryDownloadChunk { id, owned_id: owned_id.into(), oldest_sequence: oldest,
                covered_bytes: covered, gzip_base64: encode_history_chunk(&events)? }).await?;
            before_sequence = Some(oldest);
        }
    }
    history_outbound.send(ServerFrame::HistoryDownloadDone { id, owned_id: owned_id.into() }).await.map_err(|e| e.to_string())
}

async fn send_history_frame(outbound: &mpsc::Sender<ServerFrame>, frame: ServerFrame) -> Result<(), String> {
    if serde_json::to_vec(&frame).map_err(|e| e.to_string())?.len() > MAX_SERVER_FRAME_BYTES {
        return Err("Remote response exceeds the 16 MiB transport limit".into());
    }
    outbound.send(frame).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn serve_remote_socket(socket: WebSocket, state: ServerState) {
    let (mut writer, mut reader) = socket.split();
    let (outbound, mut outgoing) = mpsc::channel::<ServerFrame>(OUTBOUND_FRAME_CAPACITY);
    let (history_outbound, mut history_outgoing) = mpsc::channel::<ServerFrame>(1);
    let restart_flushed = state.restart_flushed.clone();
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = next_outbound_frame(&mut outgoing, &mut history_outgoing).await {
            let json = match encode_server_frame(&frame) {
                Ok(json) => json,
                Err(error) => {
                    eprintln!("Remote transport closed: {error}");
                    let _ = writer.send(AxumMessage::Close(None)).await;
                    break;
                }
            };
            let restarting = matches!(frame, ServerFrame::Response { response: RemoteResponse::Restarting, .. });
            let sent = writer.send(AxumMessage::Text(json.into())).await;
            if restarting {
                restart_flushed.notify_one();
            }
            if sent.is_err() { break; }
        }
    });
    let _ = outbound
        .send(ServerFrame::Ready {
            protocol_version: PROTOCOL_VERSION,
        })
        .await;
    let socket_id = state.relay.next_id.fetch_add(1, Ordering::Relaxed);
    state.relay.desktops.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((socket_id, outbound.clone()));
    let mut live_events = state.events.subscribe();
    let mut message_changes = MESSAGE_CHANGES.subscribe();
    message_changes.mark_unchanged();
    let mut live_terminals = state.terminals.output.subscribe();
    let mut queued = BTreeMap::<String, i64>::new();
    let (completed, mut completions) = mpsc::channel::<(u64, Result<Option<RemoteResponse>, String>)>(32);
    let upload = Arc::new(Mutex::new(None::<PendingAttachment>));
    let mut request_tasks = HashMap::new();
    let mut selected_child_watches = HashSet::<(String, u64)>::new();
    loop {
        let upload_deadline = upload.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref().map(|pending| pending.deadline);
        let message = tokio::select! {
            _ = tokio::time::sleep_until(upload_deadline.unwrap_or_else(tokio::time::Instant::now)), if upload_deadline.is_some() => {
                let mut pending = upload.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if pending.as_ref().is_some_and(|item| item.deadline <= tokio::time::Instant::now()) { *pending = None; }
                None
            }
            message = reader.next() => {
                let Some(Ok(message)) = message else { break; };
                Some(message)
            }
            Ok(()) = message_changes.changed() => {
                if outbound.send(ServerFrame::MessagesChanged).await.is_err() { break; }
                None
            }
            event = live_events.recv() => {
                match event {
                    Ok(event) => {
                        let cursor = queued.entry(event.owned_id.clone()).or_insert(i64::MIN);
                        if event.sequence > *cursor {
                            *cursor = event.sequence;
                            if outbound.send(ServerFrame::Event { event }).await.is_err() { break; }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Replay only sessions known to this socket. A missing session's
                        // next live event exposes a gap for the client's snapshot repair.
                        let mut failed = false;
                        for owned_id in queued.keys().cloned().collect::<Vec<_>>() {
                            if let Err(error) = replay_session(&state, &outbound, &mut queued, &owned_id).await {
                                eprintln!("Remote lag replay failed: {error}");
                                failed = true;
                                break;
                            }
                        }
                        if failed { break; }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
                None
            }
            frame = live_terminals.recv() => {
                match frame {
                    Ok(frame) => {
                        if outbound.send(ServerFrame::TerminalOutput { frame }).await.is_err() { break; }
                    }
                    // A socket too slow to keep up loses those chunks; reopening the tab replays scrollback.
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
                None
            }
            completion = completions.recv() => {
                let Some((id, response)) = completion else { break; };
                request_tasks.remove(&id);
                let frame = match response {
                    Ok(Some(response)) => ServerFrame::Response { id, response },
                    Ok(None) => continue,
                    Err(message) => ServerFrame::Error { id, message },
                };
                if outbound.send(frame).await.is_err() { break; }
                None
            }
        };
        let Some(message) = message else {
            continue;
        };
        let AxumMessage::Text(text) = message else {
            continue;
        };
        let Ok(frame) = serde_json::from_str::<ClientFrame>(text.as_str()) else {
            continue;
        };
        match frame {
            ClientFrame::Request { id, command } => {
                match &command {
                    RemoteCommand::SelectChildHistory {
                        parent_owned_id,
                        request_id,
                        ..
                    } => {
                        state
                            .manager
                            .reserve_child_history_request(parent_owned_id, *request_id);
                        selected_child_watches
                            .insert((parent_owned_id.clone(), *request_id));
                    }
                    RemoteCommand::StopChildHistory {
                        parent_owned_id,
                        request_id,
                    } => {
                        selected_child_watches
                            .remove(&(parent_owned_id.clone(), *request_id));
                        let stopped = state
                            .manager
                            .stop_child_history(parent_owned_id, *request_id);
                        let _ = outbound
                            .send(ServerFrame::Response {
                                id,
                                response: RemoteResponse::Bool(stopped),
                            })
                            .await;
                        continue;
                    }
                    _ => {}
                }
                let request_state = state.clone();
                let request_upload = upload.clone();
                let completion_sink = completed.clone();
                let progress_outbound = outbound.clone();
                let history_sink = history_outbound.clone();
                let task = tokio::spawn(async move {
                    if let RemoteCommand::DownloadHistory { owned_id, before_sequence } = &command {
                        let result = stream_history_download(&request_state, id, owned_id, *before_sequence, &history_sink).await.map(|()| None);
                        let _ = completion_sink.send((id, result)).await;
                        return;
                    }
                    let progress: crate::agent_conversation::providers::updates::ProgressSink = Arc::new(move |progress| {
                        let _ = progress_outbound.try_send(ServerFrame::ProviderDownloadProgress { id, progress });
                    });
                    let response = execute_server_command(&request_state, &request_upload, command, progress).await;
                    let _ = completion_sink.send((id, response.map(Some))).await;
                });
                if let Some(previous) = request_tasks.insert(id, task.abort_handle()) {
                    previous.abort();
                }
            }
            ClientFrame::Cancel { id } => {
                if let Some(task) = request_tasks.remove(&id) {
                    task.abort();
                }
                let cancelled = selected_child_watches
                    .iter()
                    .filter(|(_, request_id)| *request_id == id)
                    .cloned()
                    .collect::<Vec<_>>();
                for (parent_owned_id, request_id) in cancelled {
                    state.manager.stop_child_history(&parent_owned_id, request_id);
                    selected_child_watches.remove(&(parent_owned_id, request_id));
                }
            }
            ClientFrame::RelayReply { request_id, result } => {
                if let Some(reply) = state.relay.pending.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&request_id) {
                    let _ = reply.send(result);
                }
            }
            ClientFrame::Resume { cursors } => {
                queued.extend(cursors);
                let mut failed = false;
                for owned_id in queued.keys().cloned().collect::<Vec<_>>() {
                    if let Err(error) = replay_session(&state, &outbound, &mut queued, &owned_id).await {
                        eprintln!("Remote resume replay failed: {error}");
                        failed = true;
                        break;
                    }
                }
                if failed { break; }
            }
        }
    }
    // The writer ends once every sender is gone, this one included.
    state.relay.desktops.lock().unwrap_or_else(std::sync::PoisonError::into_inner).retain(|(id, _)| *id != socket_id);
    for (_, task) in request_tasks.drain() {
        task.abort();
    }
    for (parent_owned_id, request_id) in selected_child_watches {
        state.manager.stop_child_history(&parent_owned_id, request_id);
    }
    drop(outbound);
    drop(history_outbound);
    let _ = writer_task.await;
}

pub(in crate::agent_conversation::remote) async fn replay_session(
    state: &ServerState,
    outbound: &mpsc::Sender<ServerFrame>,
    queued: &mut BTreeMap<String, i64>,
    owned_id: &str,
) -> Result<(), String> {
    let Some(snapshot) = state.manager.snapshot_since(owned_id, Some(i64::MAX))? else { return Ok(()); };
    outbound.send(ServerFrame::ReplayStart { owned_id: owned_id.into() }).await.map_err(|e| e.to_string())?;
    let mut sequence = *queued.get(owned_id).unwrap_or(&i64::MIN);
    loop {
        let page = state.manager.list_events_after(owned_id, sequence, REPLAY_PAGE_BYTES)?;
        if page.events.is_empty() { break; }
        let has_more = page.has_more;
        for event in page.events {
            sequence = event.sequence;
            outbound.send(ServerFrame::Event { event }).await.map_err(|error| error.to_string())?;
            queued.insert(owned_id.to_string(), sequence);
        }
        if !has_more { break; }
    }
    let head = snapshot.last_sequence;
    queued.insert(owned_id.into(), sequence.min(head));
    outbound.send(ServerFrame::ReplayComplete { owned_id: owned_id.into(), head, snapshot })
        .await.map_err(|e| e.to_string())?;
    Ok(())
}
