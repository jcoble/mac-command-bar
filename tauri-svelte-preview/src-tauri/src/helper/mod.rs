//! The helper model: one small language model the app calls on its own behalf.
//!
//! This is not the coding agent. It is a short, cheap call — a few hundred
//! tokens — that the app makes to name a session or to answer a question about
//! one, using a key the person supplies themselves. Nothing here streams,
//! nothing here remembers a previous call, and with no key stored the whole
//! feature is simply off.
//!
//! The shape is deliberately flat. Two services are supported; each one gets a
//! module that knows how to phrase one request and read one answer, and a
//! single transport sends the resulting HTTP call. Tests replace the transport,
//! so the request bodies below are checked without a network.

mod anthropic;
mod keychain;
mod openai;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// How long one helper call may take before it is given up on.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// What the helper is being asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HelperJob {
    Title,
    Inspect,
}

impl HelperJob {
    /// The name this job is logged and stored under.
    fn name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Inspect => "inspect",
        }
    }

    /// The standing instruction for the job. Both are short on purpose: the
    /// helper is given one thing to do and no room to editorialise.
    fn system(self) -> &'static str {
        match self {
            Self::Title => {
                "Name this conversation in at most six words. Answer with the name alone: no quotation marks, no trailing period, no preamble."
            }
            Self::Inspect => {
                "Answer the question about the text below in plain English, in three to five sentences; name one recommended next action. Use only what the text says."
            }
        }
    }

    /// The ceiling on the answer. Far above what a title needs, because on one
    /// of the two services this number covers the model's own reasoning as
    /// well as the words it writes: a ceiling set to the length of a title
    /// leaves a reasoning model no room to reach one, and it stops with
    /// nothing written. The system prompt is what keeps a title short; this
    /// only stops a runaway.
    fn max_output(self) -> u32 {
        match self {
            Self::Title => 256,
            Self::Inspect => 1024,
        }
    }
}

/// Which service the call goes to, and therefore which key it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HelperVendor {
    #[serde(rename = "openai")]
    OpenAi,
    Anthropic,
}

impl HelperVendor {
    /// The Keychain account name, and the provider name usage rows are filed
    /// under.
    fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

// Model ids taken from each vendor's own published model list on 2026-08-19
// (developers.openai.com/api/docs/models and
// platform.claude.com/docs/en/about-claude/models). Cheapest per million
// tokens first, since a job of this size is billed almost entirely by the
// token; the third entry is the vendor's strongest model, for someone who
// wants a better title badly enough to pay for it.
const OPENAI_MODELS: &[&str] = &["gpt-5.6-luna", "gpt-5.6-terra", "gpt-5.6-sol"];
const ANTHROPIC_MODELS: &[&str] = &["claude-haiku-4-5", "claude-sonnet-5", "claude-opus-5"];

/// One thing to ask the helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperRequest {
    pub job: HelperJob,
    pub system: String,
    pub input: String,
    pub max_output: u32,
}

/// Everything that can go wrong on the way to an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperError {
    /// No key is stored for the chosen vendor, so the helper is off.
    NoKey,
    /// The vendor did not answer inside `REQUEST_TIMEOUT`.
    Timeout,
    /// The call never reached the vendor.
    Transport(String),
    /// The vendor answered, and the answer was a refusal.
    Vendor { status: u16, message: String },
    /// The vendor answered with something this code cannot read.
    BadResponse(String),
    /// The helper settings store or its JSON value could not be used.
    Settings(String),
}

impl HelperError {
    /// The whole sentence shown to the person. Settings puts this straight on
    /// its status line, so it reads as English rather than as a variant name.
    pub fn sentence(&self) -> String {
        match self {
            Self::NoKey => "No key — helper off".to_string(),
            Self::Timeout => "The helper model did not answer in time.".to_string(),
            Self::Transport(detail) => {
                format!("The helper model could not be reached: {detail}")
            }
            Self::Vendor { status, message } => {
                format!("The helper model refused the call ({status}): {message}")
            }
            Self::BadResponse(detail) => {
                format!("The helper model's answer could not be read: {detail}")
            }
            Self::Settings(detail) => format!("The helper settings could not be used: {detail}"),
        }
    }
}

/// One HTTP call, as a vendor module describes it and the transport sends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperHttpRequest {
    pub url: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

/// What came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperHttpResponse {
    pub status: u16,
    pub body: String,
}

/// Sends one HTTP call. The only thing tests replace.
pub trait HelperTransport {
    fn send(&self, request: &HelperHttpRequest) -> Result<HelperHttpResponse, HelperError>;
}

/// An answer, with what it cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperCompletion {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Vendor, model — the part of the helper's configuration that is not secret.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperSettings {
    pub vendor: HelperVendor,
    pub model: String,
}

impl Default for HelperSettings {
    fn default() -> Self {
        Self {
            vendor: HelperVendor::OpenAi,
            model: OPENAI_MODELS[0].to_string(),
        }
    }
}

/// What Settings needs to draw the Helper section: the stored choice, whether
/// a key is in the Keychain, and the models each vendor offers — so that the
/// list of model ids lives in one place, here, and not also in the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperSettingsView {
    pub vendor: HelperVendor,
    pub model: String,
    pub has_key: bool,
    pub openai_models: Vec<String>,
    pub anthropic_models: Vec<String>,
}

/// What the Test button gets back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperTestResult {
    pub ok: bool,
    pub message: String,
}

/// A refusal, as both vendors phrase one: a status and an `error.message`.
/// A body that does not say that is shown as far as it goes, so the status is
/// never the only thing a person has to go on.
fn vendor_error(response: &HelperHttpResponse) -> HelperError {
    let message = serde_json::from_str::<Value>(&response.body)
        .ok()
        .and_then(|document| document["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| response.body.chars().take(200).collect());
    HelperError::Vendor {
        status: response.status,
        message,
    }
}

/// Asks the vendor, once, with one retry if the call never left the machine.
///
/// A timeout is not retried: the ceiling is already twenty seconds, and a
/// second wait would leave someone looking at a spinner for the better part of
/// a minute. A refusal is not retried either — it would be refused again.
fn complete(
    transport: &dyn HelperTransport,
    vendor: HelperVendor,
    model: &str,
    key: &str,
    request: &HelperRequest,
) -> Result<HelperCompletion, HelperError> {
    let http = match vendor {
        HelperVendor::OpenAi => openai::build_request(model, key, request),
        HelperVendor::Anthropic => anthropic::build_request(model, key, request),
    };
    let response = match transport.send(&http) {
        Err(HelperError::Transport(_)) => transport.send(&http),
        first => first,
    }?;
    match vendor {
        HelperVendor::OpenAi => openai::read_response(&response),
        HelperVendor::Anthropic => anthropic::read_response(&response),
    }
}

/// The real transport: one HTTP call with a twenty-second ceiling.
struct HttpTransport {
    client: reqwest::blocking::Client,
}

impl HttpTransport {
    fn new() -> Result<Self, HelperError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| HelperError::Transport(error.to_string()))?;
        Ok(Self { client })
    }
}

/// A failed call is a timeout when the clock ran out, and a transport error
/// otherwise. Only the second is worth trying again.
fn transport_error(error: reqwest::Error) -> HelperError {
    if error.is_timeout() {
        HelperError::Timeout
    } else {
        HelperError::Transport(error.to_string())
    }
}

impl HelperTransport for HttpTransport {
    fn send(&self, request: &HelperHttpRequest) -> Result<HelperHttpResponse, HelperError> {
        let mut call = self.client.post(request.url).json(&request.body);
        for (name, value) in &request.headers {
            call = call.header(name.as_str(), value.as_str());
        }
        let response = call.send().map_err(transport_error)?;
        let status = response.status().as_u16();
        // Reading the body is subject to the same clock as sending, so it maps
        // the same way. Calling a timeout here a transport error would have it
        // retried, which is exactly what the ceiling exists to prevent.
        let body = response.text().map_err(transport_error)?;
        Ok(HelperHttpResponse { status, body })
    }
}

const SETTINGS_KEY: &str = "helper.settings";

/// The stored choice, or the default when nothing has been chosen yet. Store
/// and JSON errors stay visible to the caller instead of silently changing the
/// vendor or model.
fn read_settings(app: &AppHandle) -> Result<HelperSettings, HelperError> {
    let runtime = app
        .try_state::<crate::agent_conversation::manager::AgentRuntimeManager>()
        .ok_or_else(|| HelperError::Settings("the session database is unavailable".to_string()))?;
    let Some(value_json) = runtime
        .read_app_setting(SETTINGS_KEY)
        .map_err(HelperError::Settings)?
    else {
        return Ok(HelperSettings::default());
    };
    serde_json::from_str(&value_json).map_err(|error| HelperError::Settings(error.to_string()))
}

fn write_settings(app: &AppHandle, settings: &HelperSettings) -> Result<(), HelperError> {
    let value_json = serde_json::to_string(settings)
        .map_err(|error| HelperError::Settings(error.to_string()))?;
    let runtime = app
        .try_state::<crate::agent_conversation::manager::AgentRuntimeManager>()
        .ok_or_else(|| HelperError::Settings("the session database is unavailable".to_string()))?;
    runtime
        .write_app_setting(SETTINGS_KEY, &value_json)
        .map_err(HelperError::Settings)
}

/// Runs one helper job end to end: read the choice, read the key, ask, log.
fn run_job(app: &AppHandle, job: HelperJob, input: &str) -> Result<HelperCompletion, HelperError> {
    let settings = read_settings(app)?;
    let key = keychain::read_key(settings.vendor.name())?.ok_or(HelperError::NoKey)?;
    let request = HelperRequest {
        job,
        system: job.system().to_string(),
        input: input.to_string(),
        max_output: job.max_output(),
    };
    let transport = HttpTransport::new()?;
    let completion = complete(&transport, settings.vendor, &settings.model, &key, &request)?;
    // A failed write to the usage store is not a failed helper call: the
    // person asked for a title, not for bookkeeping.
    let _ = crate::usage_history::record_helper_call(
        app,
        job.name(),
        settings.vendor.name(),
        &settings.model,
        completion.input_tokens,
        completion.output_tokens,
    );
    Ok(completion)
}

/// Names one session from its first exchange.
///
/// The app's own call rather than a person's: it is made on a background thread
/// the moment a session's first turn ends, and its answer replaces the name
/// taken from the first prompt.
pub fn name_session(app: &AppHandle, input: &str) -> Result<String, HelperError> {
    run_job(app, HelperJob::Title, input).map(|completion| completion.text)
}

#[tauri::command]
pub async fn run_helper_job(
    app: AppHandle,
    job: HelperJob,
    input: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_job(&app, job, &input)
            .map(|completion| completion.text)
            .map_err(|error| error.sentence())
    })
    .await
    .map_err(|error| format!("helper task failed: {error}"))?
}

#[tauri::command]
pub async fn set_helper_key(vendor: HelperVendor, key: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let account = vendor.name();
        let stored = if key.trim().is_empty() {
            keychain::delete_key(account)
        } else {
            keychain::write_key(account, key.trim())
        };
        stored.map_err(|error| error.sentence())
    })
    .await
    .map_err(|error| format!("helper task failed: {error}"))?
}

#[tauri::command]
pub async fn read_helper_settings(app: AppHandle) -> Result<HelperSettingsView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let settings = read_settings(&app).map_err(|error| error.sentence())?;
        let has_key = keychain::read_key(settings.vendor.name())
            .ok()
            .flatten()
            .is_some();
        Ok(HelperSettingsView {
            vendor: settings.vendor,
            model: settings.model,
            has_key,
            openai_models: OPENAI_MODELS.iter().map(|id| id.to_string()).collect(),
            anthropic_models: ANTHROPIC_MODELS.iter().map(|id| id.to_string()).collect(),
        })
    })
    .await
    .map_err(|error| format!("helper task failed: {error}"))?
}

#[tauri::command]
pub async fn write_helper_settings(app: AppHandle, settings: HelperSettings) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        write_settings(&app, &settings).map_err(|error| error.sentence())
    })
    .await
    .map_err(|error| format!("helper task failed: {error}"))?
}

#[tauri::command]
pub async fn test_helper(app: AppHandle) -> Result<HelperTestResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        match run_job(
            &app,
            HelperJob::Title,
            "A short exchange about nothing in particular.",
        ) {
            Ok(completion) => HelperTestResult {
                ok: true,
                message: format!(
                    "OK · {} ms · {} tokens",
                    started.elapsed().as_millis(),
                    completion.input_tokens + completion.output_tokens
                ),
            },
            Err(error) => HelperTestResult {
                ok: false,
                message: error.sentence(),
            },
        }
    })
    .await
    .map_err(|error| format!("helper task failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A transport that answers from a script instead of from the network, and
    /// keeps the requests it was given.
    struct FakeTransport {
        answers: RefCell<Vec<Result<HelperHttpResponse, HelperError>>>,
        seen: RefCell<Vec<HelperHttpRequest>>,
    }

    impl FakeTransport {
        fn new(answers: Vec<Result<HelperHttpResponse, HelperError>>) -> Self {
            Self {
                answers: RefCell::new(answers),
                seen: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> usize {
            self.seen.borrow().len()
        }
    }

    impl HelperTransport for FakeTransport {
        fn send(&self, request: &HelperHttpRequest) -> Result<HelperHttpResponse, HelperError> {
            self.seen.borrow_mut().push(request.clone());
            let mut answers = self.answers.borrow_mut();
            if answers.is_empty() {
                panic!("the fake transport was called more times than it was scripted for");
            }
            answers.remove(0)
        }
    }

    fn title_request() -> HelperRequest {
        HelperRequest {
            job: HelperJob::Title,
            system: "Name this conversation.".to_string(),
            input: "We talked about the settings screen.".to_string(),
            max_output: 32,
        }
    }

    fn ok(body: &str) -> Result<HelperHttpResponse, HelperError> {
        Ok(HelperHttpResponse {
            status: 200,
            body: body.to_string(),
        })
    }

    const OPENAI_ANSWER: &str = r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"Settings screen work"}]}],"usage":{"input_tokens":11,"output_tokens":4}}"#;
    const ANTHROPIC_ANSWER: &str = r#"{"content":[{"type":"text","text":"Settings screen work"}],"usage":{"input_tokens":11,"output_tokens":4}}"#;

    #[test]
    fn openai_request_carries_the_model_input_and_output_ceiling() {
        let transport = FakeTransport::new(vec![ok(OPENAI_ANSWER)]);

        let completion = complete(
            &transport,
            HelperVendor::OpenAi,
            "gpt-5.6-luna",
            "sk-test",
            &title_request(),
        )
        .expect("the scripted answer is a good one");

        let sent = transport.seen.borrow()[0].clone();
        assert_eq!(sent.body["model"], "gpt-5.6-luna");
        assert_eq!(sent.body["max_output_tokens"], 32);
        assert_eq!(sent.body["instructions"], "Name this conversation.");
        assert_eq!(sent.body["input"], "We talked about the settings screen.");
        assert!(sent
            .headers
            .contains(&("Authorization".to_string(), "Bearer sk-test".to_string())));
        assert_eq!(completion.text, "Settings screen work");
        assert_eq!(completion.input_tokens, 11);
        assert_eq!(completion.output_tokens, 4);
    }

    #[test]
    fn anthropic_request_carries_system_messages_and_the_version_header() {
        let transport = FakeTransport::new(vec![ok(ANTHROPIC_ANSWER)]);

        let completion = complete(
            &transport,
            HelperVendor::Anthropic,
            "claude-haiku-4-5",
            "sk-ant-test",
            &title_request(),
        )
        .expect("the scripted answer is a good one");

        let sent = transport.seen.borrow()[0].clone();
        assert_eq!(sent.body["model"], "claude-haiku-4-5");
        assert_eq!(sent.body["max_tokens"], 32);
        assert_eq!(sent.body["system"], "Name this conversation.");
        assert_eq!(sent.body["messages"][0]["role"], "user");
        assert_eq!(
            sent.body["messages"][0]["content"],
            "We talked about the settings screen."
        );
        assert!(sent
            .headers
            .contains(&("anthropic-version".to_string(), "2023-06-01".to_string())));
        assert!(sent
            .headers
            .contains(&("x-api-key".to_string(), "sk-ant-test".to_string())));
        assert_eq!(completion.text, "Settings screen work");
        assert_eq!(completion.output_tokens, 4);
    }

    #[test]
    fn a_timeout_is_reported_as_a_timeout_and_is_not_retried() {
        let transport = FakeTransport::new(vec![Err(HelperError::Timeout)]);

        let error = complete(
            &transport,
            HelperVendor::OpenAi,
            "gpt-5.6-luna",
            "sk-test",
            &title_request(),
        )
        .expect_err("a timeout is not an answer");

        assert_eq!(error, HelperError::Timeout);
        assert_eq!(transport.calls(), 1);
    }

    #[test]
    fn a_call_that_never_left_the_machine_is_tried_once_more() {
        let transport = FakeTransport::new(vec![
            Err(HelperError::Transport("connection reset".to_string())),
            ok(OPENAI_ANSWER),
        ]);

        let completion = complete(
            &transport,
            HelperVendor::OpenAi,
            "gpt-5.6-luna",
            "sk-test",
            &title_request(),
        )
        .expect("the second try was scripted to work");

        assert_eq!(completion.text, "Settings screen work");
        assert_eq!(transport.calls(), 2);
    }

    #[test]
    fn a_second_transport_failure_is_given_up_on() {
        let transport = FakeTransport::new(vec![
            Err(HelperError::Transport("connection reset".to_string())),
            Err(HelperError::Transport("connection reset".to_string())),
        ]);

        let error = complete(
            &transport,
            HelperVendor::OpenAi,
            "gpt-5.6-luna",
            "sk-test",
            &title_request(),
        )
        .expect_err("both tries were scripted to fail");

        assert_eq!(
            error,
            HelperError::Transport("connection reset".to_string())
        );
        assert_eq!(transport.calls(), 2);
    }

    #[test]
    fn a_refusal_carries_the_status_and_what_the_vendor_said() {
        let transport = FakeTransport::new(vec![Ok(HelperHttpResponse {
            status: 401,
            body: r#"{"error":{"message":"Incorrect API key provided"}}"#.to_string(),
        })]);

        let error = complete(
            &transport,
            HelperVendor::OpenAi,
            "gpt-5.6-luna",
            "sk-wrong",
            &title_request(),
        )
        .expect_err("401 is not an answer");

        assert_eq!(
            error,
            HelperError::Vendor {
                status: 401,
                message: "Incorrect API key provided".to_string(),
            }
        );
        assert_eq!(transport.calls(), 1);
    }

    #[test]
    fn no_key_reads_as_the_sentence_settings_shows() {
        assert_eq!(HelperError::NoKey.sentence(), "No key — helper off");
    }
}
