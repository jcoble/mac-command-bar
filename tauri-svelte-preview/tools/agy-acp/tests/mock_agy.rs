//! Drive the adapter over its real stdio protocol with a stand-in `agy`.
//!
//! The unit tests in `src/tests.rs` call the handlers directly, and the
//! end-to-end tests need real Antigravity credentials, so they are ignored by
//! default. These two run every time: a small script on the child's PATH plays
//! the part of `agy`, so a full turn — initialize, session/new, session/prompt,
//! session/cancel — can be exercised without credentials or a network call.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Stands in for the Antigravity CLI. `models` answers with the same
/// tab-separated shape the real one prints; a prompt containing "wait" hangs
/// so a turn can be cancelled while it is still running, and anything else
/// streams one reply in agy's NDJSON format and exits.
const MOCK_AGY: &str = r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf 'Fetching available models...\n'
  printf 'gemini-3.7-flash-high\tGemini 3.7 Flash (High)\n'
  printf 'gemini-3.1-pro-high\tGemini 3.1 Pro (High)\n'
  exit 0
fi
prompt=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-p" ]; then
    shift
    prompt="$1"
  fi
  shift
done
case "$prompt" in
  # exec, so a kill reaches the waiting process rather than only its shell.
  *wait*) exec sleep 30 ;;
esac
printf '{"event":"init","conversation_id":"conv-mock","init":{"cwd":"/tmp"}}\n'
printf '{"event":"step_update","step_update":{"conversation_id":"conv-mock","step_index":1,"state":"ACTIVE","step_type":"agent_response","text_delta":"hello there"}}\n'
printf '{"event":"result","result":{"conversation_id":"conv-mock","status":"SUCCESS","response":"hello there"}}\n'
exit 0
"#;

struct Harness {
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    child: Child,
    root: PathBuf,
}

impl Harness {
    fn start(name: &str) -> Harness {
        let root = std::env::temp_dir().join(format!("agy-acp-mock-{name}-{}", std::process::id()));
        let tools = root.join("bin");
        std::fs::create_dir_all(&tools).expect("mock directory");
        let script = tools.join("agy");
        std::fs::write(&script, MOCK_AGY).expect("mock script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("mock script is executable");

        let path = match std::env::var("PATH") {
            Ok(existing) => format!("{}:{existing}", tools.display()),
            Err(_) => tools.display().to_string(),
        };
        let mut child = Command::new(env!("CARGO_BIN_EXE_agy-acp"))
            .current_dir(&root)
            // The adapter keeps its session file under HOME; a scratch one
            // keeps the test off the real store.
            .env("HOME", &root)
            .env("PATH", path)
            .env_remove("AGY_EXTRA_ARGS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("agy-acp starts");
        let stdin = child.stdin.take().expect("adapter stdin");
        let stdout = child.stdout.take().expect("adapter stdout");
        Harness {
            stdin,
            reader: BufReader::new(stdout),
            child,
            root,
        }
    }

    fn send(&mut self, message: &str) {
        writeln!(self.stdin, "{message}").expect("write to adapter");
        self.stdin.flush().expect("flush adapter stdin");
    }

    /// Read frames until the answer to `id` arrives, collecting the
    /// `session/update` notifications that came with it.
    fn wait_for(&mut self, id: u64) -> (Value, Vec<Value>) {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut updates = Vec::new();
        loop {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for response {id}"
            );
            let mut line = String::new();
            let read = self.reader.read_line(&mut line).expect("read from adapter");
            assert!(read > 0, "the adapter closed before answering {id}");
            if line.trim().is_empty() {
                continue;
            }
            let frame: Value = serde_json::from_str(line.trim()).expect("adapter frame is JSON");
            if frame.get("method").and_then(Value::as_str) == Some("session/update") {
                updates.push(frame["params"]["update"].clone());
                continue;
            }
            if frame.get("id").and_then(Value::as_u64) == Some(id) {
                return (frame, updates);
            }
        }
    }

    fn start_session(&mut self) -> String {
        self.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientName":"mock","clientVersion":"0.1"}}"#);
        let (initialized, _) = self.wait_for(1);
        assert_eq!(initialized["result"]["protocolVersion"], 1);
        assert_eq!(
            initialized["result"]["agentCapabilities"]["loadSession"],
            true
        );

        self.send(r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}"#);
        let (started, _) = self.wait_for(2);
        let models = started["result"]["models"]["availableModels"]
            .as_array()
            .expect("session/new reports its models")
            .iter()
            .map(|model| model["modelId"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            models,
            vec!["Gemini 3.7 Flash (High)", "Gemini 3.1 Pro (High)"]
        );
        assert_eq!(
            started["result"]["models"]["currentModelId"],
            "Gemini 3.7 Flash (High)"
        );
        started["result"]["sessionId"]
            .as_str()
            .expect("session/new returns a session id")
            .to_string()
    }

    fn finish(mut self) {
        drop(self.stdin);
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn prompt(id: u64, session_id: &str, text: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"session/prompt","params":{{"sessionId":"{session_id}","prompt":[{{"type":"text","text":"{text}"}}]}}}}"#
    )
}

#[test]
fn initialize_new_and_prompt_stream_a_reply() {
    let mut harness = Harness::start("prompt");
    let session_id = harness.start_session();

    harness.send(&prompt(3, &session_id, "say hello"));
    let (answered, updates) = harness.wait_for(3);

    assert!(
        answered["error"].is_null(),
        "prompt failed: {}",
        answered["error"]
    );
    assert_eq!(answered["result"]["stopReason"], "end_turn");
    let text = updates
        .iter()
        .filter(|update| update["sessionUpdate"] == "agent_message_chunk")
        .map(|update| update["content"]["text"].as_str().unwrap_or_default())
        .collect::<String>();
    assert_eq!(text, "hello there");

    harness.finish();
}

#[test]
fn cancel_ends_a_running_turn() {
    let mut harness = Harness::start("cancel");
    let session_id = harness.start_session();

    // The stand-in waits rather than answering, so the turn is still running
    // when the cancel arrives.
    harness.send(&prompt(3, &session_id, "wait for me"));
    std::thread::sleep(Duration::from_millis(200));
    harness.send(&format!(
        r#"{{"jsonrpc":"2.0","method":"session/cancel","params":{{"sessionId":"{session_id}"}}}}"#
    ));

    let (answered, _) = harness.wait_for(3);
    assert!(
        answered["error"].is_null(),
        "cancel failed: {}",
        answered["error"]
    );
    assert_eq!(answered["result"]["stopReason"], "cancelled");

    harness.finish();
}
