//! A tool call's output and diff stay out of history pages; its row reads them by id when opened.
use super::*;
use super::super::protocol::CommandResult;
use super::super::remote::RemoteConnectionManager;

/// One tool call as its events say it. Imported history stores a call's output in a later event with
/// no name, summary, path or diff, so the events of one item fold in order: each later non-empty
/// field replaces the earlier one, and the state is the latest event's.
#[derive(Clone, Copy)]
pub(crate) struct ToolCall<'a> {
    pub(crate) name: &'a str,
    pub(crate) state: ToolState,
    pub(crate) summary: Option<&'a str>,
    pub(crate) output: Option<&'a str>,
    pub(crate) path: Option<&'a str>,
    pub(crate) diff: Option<&'a str>,
}

impl<'a> ToolCall<'a> {
    /// The call `payload` describes, when it is a tool event of item `item_id`.
    pub(crate) fn of(payload: &'a AgentConversationPayload, item_id: &str) -> Option<Self> {
        match payload {
            AgentConversationPayload::Tool { item_id: id, name, state, summary, output, path, diff, .. } if id == item_id =>
                Some(Self { name, state: *state, summary: summary.as_deref(), output: output.as_deref(), path: path.as_deref(), diff: diff.as_deref() }),
            _ => None,
        }
    }

    pub(crate) fn fold(self, later: Self) -> Self {
        let newer = |old: Option<&'a str>, new: Option<&'a str>| new.filter(|text| !text.is_empty()).or(old);
        Self { name: if later.name.is_empty() { self.name } else { later.name }, state: later.state,
            summary: newer(self.summary, later.summary), output: newer(self.output, later.output),
            path: newer(self.path, later.path), diff: newer(self.diff, later.diff) }
    }

    pub(crate) fn has_detail(&self) -> bool {
        [self.output, self.path, self.diff].iter().any(|field| field.is_some_and(|text| !text.is_empty()))
    }
}

/// What an opened tool row shows.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationToolCall {
    pub summary: Option<String>,
    pub output: Option<String>,
    pub diff: Option<String>,
    pub path: Option<String>,
}

/// A page's tool event without its output. The row keeps what it shows closed: whether there is
/// anything to open, and the output's first line as the summary when the call has none.
pub(crate) fn without_details(mut event: AgentConversationEvent) -> AgentConversationEvent {
    if let AgentConversationPayload::Tool { summary, output, diff, has_detail, .. } = &mut event.payload {
        let output = output.take().filter(|text| !text.is_empty());
        *has_detail = output.is_some() || diff.as_deref().is_some_and(|text| !text.is_empty());
        if summary.as_deref().is_none_or(str::is_empty) {
            if let Some(line) = output.as_deref().and_then(|text| text.lines().next()) {
                *summary = Some(line.chars().take(200).collect());
            }
        }
    }
    event
}

/// The whole of tool call `item_id`: every saved event of it under `owned_id`, folded. One indexed read.
pub(crate) fn read_tool_call(store: &SessionStore, owned_id: &str, item_id: &str) -> Result<AgentConversationToolCall, String> {
    let events = store.list_item_events(owned_id, item_id).map_err(|error| error.to_string())?
        .into_iter().map(stored_event).collect::<Result<Vec<_>, _>>()?;
    let call = events.iter().filter_map(|event| ToolCall::of(&event.payload, item_id)).reduce(ToolCall::fold)
        .ok_or("This part of the conversation is not saved on this Mac yet")?;
    let owned = |text: Option<&str>| text.map(str::to_owned);
    Ok(AgentConversationToolCall { summary: owned(call.summary), output: owned(call.output), diff: owned(call.diff), path: owned(call.path) })
}

#[tauri::command]
/// Reads one tool call when its row is opened. A remote session's call comes from this Mac's saved
/// copy of that session, so this never waits on the network. A sub-agent's private copy is keyed by
/// its own id, which no remote machine can claim (see `may_route`).
pub async fn read_agent_conversation_tool_call(
    manager: tauri::State<'_, AgentRuntimeManager>,
    remote: tauri::State<'_, RemoteConnectionManager>,
    owned_id: String,
    item_id: String,
) -> CommandResult<AgentConversationToolCall> {
    let key = if remote.owns(&owned_id) { remote.history_key(&owned_id)? } else { owned_id };
    Ok(read_tool_call(manager.store(), &key, &item_id)?)
}
