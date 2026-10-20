use mcb_core::session_store::SessionStore;
use serde_json::{json, Value};

pub fn tool() -> Value {
    json!({
        "name": "board_update",
        "description": "Create or update a card on the owner's Build board. The session the owner gave a task to keeps one card for it; the main orchestrator may update any card. Helpers cannot post, and workflow workers should not: their parent's card or their run already shows. Key: the task key if there is one, otherwise a short name no other session would pick. Post when you start, update when the state changes, and use needs_you only when the owner must act, with steps saying what to do. Questions in your own session, approvals and workflow pauses already show.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "key": {"type": "string", "maxLength": 80,
                    "description": "Short id for this work (see above). One line, at most 80 characters."},
                "title": {"type": "string", "maxLength": 120,
                    "description": "Required for a new card. One line, at most 120."},
                "state": {"type": "string", "enum": ["needs_you", "running", "in_review", "done"],
                    "description": "Required for a new card."},
                "summary": {"type": "string", "maxLength": 600, "description": "At most 600."},
                "steps": {"type": "array", "maxItems": 6, "items": {"type": "string", "maxLength": 160},
                    "description": "What the owner should do, plain text, at most 6 of 160 each."},
                "note": {"type": "string", "maxLength": 600,
                    "description": "One note for the owner, at most 600."},
                "task_key": {"type": "string", "description": "The Notion task; hidden when Notion is off."},
                "task_url": {"type": "string", "description": "The Notion task; hidden when Notion is off."},
                "pr_url": {"type": "string", "description": "The PR."},
                "run_id": {"type": "string", "description": "A run on this machine."},
                "session": {"type": "string",
                    "description": "A session on this machine (handle or name); default the caller."}
            },
            "required": ["key"]
        }
    })
}

pub(super) fn update(store: &SessionStore, caller: &str, role: &str, mut args: Value) -> Result<Value, String> {
    if role == "helper" {
        return Err("helpers cannot post to the board".into());
    }
    if let Some(session) = args["session"].as_str() {
        let owned_id = store.resolve_session_name_for(session, caller, "local")
            .map_err(|error| error.to_string())?;
        args["session"] = json!(owned_id);
    }
    let key = args["key"].as_str().ok_or("key must be a string")?;
    store.board_update(key, caller, role == "orchestrator", &args, chrono::Utc::now().timestamp_millis())
        .map_err(|error| error.to_string())?;
    Ok(json!({"key": key, "saved": true}))
}
