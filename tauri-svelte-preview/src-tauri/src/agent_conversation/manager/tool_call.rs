//! Fold a phone tool's saved events into the complete call.
use super::*;

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
