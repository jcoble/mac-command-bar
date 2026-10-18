//! Push notifications for new inbox items (phone spec §5.4): a Firebase Cloud Messaging sender that
//! signs a service-account JWT, trades it for an access token, and sends each paired phone one
//! message that names the kind of item and its task, never the agent's words.

use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use mcb_core::session_store::SessionStore;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};

const SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";
const FCM_BASE: &str = "https://fcm.googleapis.com";

#[derive(Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    project_id: String,
    token_uri: String,
}

pub(super) struct PushSender {
    account: ServiceAccount,
    key: EncodingKey,
    fcm_base: String,
    http: reqwest::Client,
    /// The access token and when to stop using it.
    access: tokio::sync::Mutex<Option<(String, Instant)>>,
}

impl PushSender {
    /// Reads the service-account JSON at `path`. A missing or unusable file turns push off with one log line.
    pub(super) fn from_key_file(path: &Path) -> Option<PushSender> {
        let sender = std::fs::read_to_string(path).map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str::<ServiceAccount>(&text).map_err(|error| error.to_string()))
            .and_then(|account| {
                let key = EncodingKey::from_rsa_pem(account.private_key.as_bytes()).map_err(|error| error.to_string())?;
                let http = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().map_err(|error| error.to_string())?;
                Ok(PushSender { account, key, fcm_base: FCM_BASE.into(), http, access: tokio::sync::Mutex::new(None) })
            });
        match sender {
            Ok(sender) => Some(sender),
            Err(error) => {
                eprintln!("Phone push is off: could not use the service account at {}: {error}", path.display());
                None
            }
        }
    }

    /// A Google access token, signed for and fetched when the last one is within a minute of ending.
    async fn access_token(&self) -> Result<String, String> {
        let mut cached = self.access.lock().await;
        if let Some((token, until)) = cached.as_ref() {
            if Instant::now() < *until { return Ok(token.clone()); }
        }
        let issued = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
        let claims = json!({ "iss": self.account.client_email, "scope": SCOPE, "aud": self.account.token_uri,
            "iat": issued, "exp": issued + 3600 });
        let assertion = jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &self.key).map_err(|error| error.to_string())?;
        let response = self.http.post(&self.account.token_uri)
            .form(&[("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"), ("assertion", assertion.as_str())])
            .send().await.map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("the token endpoint answered {}", response.status()));
        }
        #[derive(Deserialize)]
        struct Granted {
            access_token: String,
            expires_in: u64,
        }
        let granted: Granted = response.json().await.map_err(|error| error.to_string())?;
        let until = Instant::now() + Duration::from_secs(granted.expires_in.saturating_sub(60));
        *cached = Some((granted.access_token.clone(), until));
        Ok(granted.access_token)
    }

    /// Sends `item` to every phone with a push token. A token Google no longer knows is forgotten;
    /// any other failure is one log line, with no retry.
    pub(super) async fn send_inbox_push(&self, store: &SessionStore, machine: &(String, String), item: &Value) {
        let tokens = match store.phone_push_tokens() {
            Ok(tokens) if !tokens.is_empty() => tokens,
            Ok(_) => return,
            Err(error) => return eprintln!("Phone push could not list tokens: {error}"),
        };
        let access = match self.access_token().await {
            Ok(access) => access,
            Err(error) => return eprintln!("Phone push could not get an access token: {error}"),
        };
        let url = format!("{}/v1/projects/{}/messages:send", self.fcm_base, self.account.project_id);
        for token in tokens {
            let response = match self.http.post(&url).bearer_auth(&access).json(&message(&token, machine, item)).send().await {
                Ok(response) => response,
                Err(error) => { eprintln!("Phone push failed: {error}"); continue; }
            };
            let status = response.status();
            if status.is_success() { continue; }
            let body = response.text().await.unwrap_or_default();
            if status == StatusCode::NOT_FOUND || body.contains("UNREGISTERED") {
                if let Err(error) = store.clear_phone_push_token(&token) { eprintln!("Phone push could not clear a token: {error}"); }
            } else {
                eprintln!("Phone push failed: {status} {body}");
            }
        }
    }
}

/// The FCM message for one phone. Its body is the item's kind and task key only: no title,
/// detail, session name, evidence or command.
fn message(token: &str, (machine_id, machine_name): &(String, String), item: &Value) -> Value {
    let label = match item["kind"].as_str() {
        Some("gate") => "Approval needed",
        Some("round_limit") => "Round limit reached",
        Some("loop_stalled") => "Loop stalled",
        Some("invalid_review") => "Review report invalid",
        Some("timeout") => "Step out of time",
        Some("permission") => "Permission request",
        _ => "Step needs a restart",
    };
    let body = match item["taskKey"].as_str().filter(|key| !key.is_empty()) {
        Some(key) => format!("{label} · {key}"),
        None => label.to_string(),
    };
    json!({ "message": {
        "token": token,
        "notification": { "title": format!("Assembly — {machine_name}"), "body": body },
        "data": { "machineId": machine_id, "itemId": item["id"] },
        "android": { "notification": { "channel_id": "assembly_inbox" } },
    } })
}

#[cfg(test)]
pub(super) mod tests {
    use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex};

    use axum::{extract::State, http::{StatusCode, Uri}, routing::any, Router};
    use mcb_core::session_store::SessionStore;
    use serde_json::{json, Value};

    use std::future::IntoFuture;

    use super::{message, PushSender};

    /// A throwaway RSA key made for these tests with `openssl genrsa 2048`. It signs nothing real.
    const TEST_KEY: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCtIAbeDE64Fcmt\n+z2fsDCBAmjfbLTBfdxdDnZAbaKFXojbY4XVqQsnc7sOyYzytV/ezrI3OsWVreC4\nbcr1neWAACtVhKnirCvoYKuAiMqrZIGicq+S4b56MT2oS52a1HNx+XIOq5YbeHh3\nJla5bF2PNv3W9bAI0F/rmhEZFc1K6PmioX6be0TlEEa/1QvtQetFTWwskB9jJ7mO\nYTRqJF9UQiSSWsjMU95w+DTbS7pULtyP/zdIuZ3qjrSkL68K8aY22MTTc4AsSDLL\nz+LG0ljexL0ub+t78QBn3defZ8zneE73UzNK4tyFURjtFjgG39lovU4BmTx9dD9V\nLRpZvKbxAgMBAAECggEAAmYR2ueGNk9H5TJ9iW/Xx+TlTo1W/h0E+M7LS/SKJzIQ\nnvxTXs/pSicSJ564IZLDEwX1v6Mm8XR5BljXZgW7xWyYtvTjzAF8Tmj7P/THtXp0\nrJeSR0UjX3OCDbE9mF1SNFHflc5fqUDHGPpMgsam3khjMl92jIMtDzWqJ13wdwvf\nt9Y51TITbqSCBolACP0QW3fRgq5UTyKvqTEfzvyvdCEI8rwZMKzs1nBOAmEZt4D5\nkPDni+Q223AIO8FRghPTPl5IoUcgr7YAidmNp4ESOYZwIbdxYnQuPTut8hwAA7fR\nBrhfCdOPq6z76NKb2/bgtjewKvJJ+/oTYRJn9SjLUwKBgQDWtbHQuSGVOjnhn+y/\n0AMCpH2rhvqoU2DnqJamYDwx3PZKgbqxldD958XA/JyHvLjgLySFgIp1KwPnPCY/\nc483qtqZ1YZI8UybPUadmSsHcVSAir8XM+P+WMOADGBwao8ir/EHah654afN1+kP\nVe8tW4aeZqpC36Li7NC2GaJ1GwKBgQDOaxendFwXBLwTAa9s+mZg5K9AE+UO0CCu\nhIhxZxMRnlMbHShvIRiAGe0XeetA3VXesPveImy7vad02AilYcSU8VZxo/Z8bHvP\nHch8UnUSFg94YASpt84uf2A7KMeLwe7OxDrYfcgrfFfbjG/b8XTFHb7QFh1vwWOB\nJYdroIlw4wKBgFQ/74orWtYnhtbV3hxWDxhWW0mXKXwrW3cLXDHkyTn3rl0GEq/k\nTaGT9LEJbpKEpzawyO/pxGq9ekad/5QeUDBTxnteoda1eI3huXfVxmChQ7s8tkbw\ngaIoSm5QbPcvmQLPbxCtv2jI9wN6mWT/H1747xXwyxZ6knvg2PTbkjfJAoGAEe5R\nnrVH7qP1pn0F/SR/UjlrU9imlDPWMvLGW3uFt4kWVQuiYIzycOZPpzNrETZo6DBd\noL1Czsg5zLebgKpOhlW0maiRl9tgoexHZy1OiCSscVt97DRQT8+2z0zKkMRm90Uq\ni3XZtpVVmHfJm8SMzTa5RaA2LGgTObvF2L6YUdsCgYAW3pwUGG/h1Im9FL4sMaWh\niHPjGObf6hnydhwCgYJdze0bhaxsBTfMyt4XV8bB4Ro5NFsrl4dDvovUgyaprXye\nAYzInDUZvnosd3IP3Gs+jcQ0ZhwoebYWN0tZo1Brk6o/2tz807pvX/KtLXH6j56g\nJtBMgn/gPbwyNAK0DZns+g==\n-----END PRIVATE KEY-----";

    /// A fake Google: the token endpoint hands out a token, and every FCM send is recorded and answered with `status`.
    pub(in crate::agent_conversation) struct FakeFcm {
        pub base: String,
        pub messages: Arc<Mutex<Vec<Value>>>,
        pub token_requests: Arc<AtomicUsize>,
    }

    #[derive(Clone)]
    struct Fake { status: u16, body: &'static str, messages: Arc<Mutex<Vec<Value>>>, token_requests: Arc<AtomicUsize> }

    async fn answer(State(fake): State<Fake>, uri: Uri, body: String) -> (StatusCode, String) {
        if uri.path() == "/token" {
            fake.token_requests.fetch_add(1, Ordering::SeqCst);
            return (StatusCode::OK, json!({ "access_token": "t", "expires_in": 3600 }).to_string());
        }
        assert_eq!(uri.path(), "/v1/projects/test-project/messages:send");
        fake.messages.lock().unwrap().push(serde_json::from_str(&body).unwrap());
        (StatusCode::from_u16(fake.status).unwrap(), fake.body.to_string())
    }

    pub(in crate::agent_conversation) async fn fake_fcm(status: u16, body: &'static str) -> FakeFcm {
        let (messages, token_requests) = (Arc::new(Mutex::new(Vec::new())), Arc::new(AtomicUsize::new(0)));
        let fake = Fake { status, body, messages: messages.clone(), token_requests: token_requests.clone() };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(axum::serve(listener, Router::new().fallback(any(answer)).with_state(fake)).into_future());
        FakeFcm { base, messages, token_requests }
    }

    /// A sender with a key file whose token endpoint is the fake, sending to the fake.
    pub(in crate::agent_conversation) fn test_sender(fake: &FakeFcm) -> PushSender {
        let path = std::env::temp_dir().join(format!("assembly-fcm-key-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, json!({ "client_email": "push@test-project.iam.gserviceaccount.com", "private_key": TEST_KEY,
            "project_id": "test-project", "token_uri": format!("{}/token", fake.base) }).to_string()).unwrap();
        let mut sender = PushSender::from_key_file(&path).expect("the test key loads");
        sender.fcm_base = fake.base.clone();
        sender
    }

    fn item(kind: &str, task_key: Option<&str>) -> Value {
        json!({ "id": format!("item-{kind}"), "kind": kind, "taskKey": task_key, "title": "MARKER-TITLE", "detail": "MARKER-DETAIL rm -rf /",
            "sessionName": "MARKER-SESSION", "evidence": [{ "label": "Provider", "text": "MARKER-EVIDENCE" }] })
    }

    fn machine() -> (String, String) { ("machine-1".into(), "Workbox Test".into()) }

    #[test]
    fn inbox_push_body_has_kind_and_task_key_but_no_agent_text() {
        let gate = message("tok", &machine(), &item("gate", Some("TSK-1")));
        let permission = message("tok", &machine(), &item("permission", None));
        assert_eq!(gate["message"]["notification"]["body"], "Approval needed · TSK-1");
        assert_eq!(permission["message"]["notification"]["body"], "Permission request");
        assert_eq!(gate["message"]["notification"]["title"], "Assembly — Workbox Test");
        assert_eq!(gate["message"]["data"], json!({ "machineId": "machine-1", "itemId": "item-gate" }));
        assert_eq!(gate["message"]["android"]["notification"]["channel_id"], "assembly_inbox");
        for text in [gate.to_string(), permission.to_string()] {
            assert!(!text.contains("MARKER") && !text.contains("rm -rf"), "no agent text in {text}");
        }
    }

    #[tokio::test]
    async fn inbox_push_sends_one_per_device_with_token() {
        let store = SessionStore::open_in_memory().unwrap();
        for (id, token) in [("a", Some("token-a")), ("b", Some("token-b")), ("c", None)] {
            store.add_phone_device(id, id, &format!("hash-{id}"), 1).unwrap();
            if let Some(token) = token { store.set_phone_push_token(id, token).unwrap(); }
        }
        store.revoke_phone_device("b", 2).unwrap();
        let fake = fake_fcm(200, "{}").await;
        test_sender(&fake).send_inbox_push(&store, &machine(), &item("gate", Some("TSK-1"))).await;
        let messages = fake.messages.lock().unwrap();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert_eq!(messages[0]["message"]["token"], "token-a");
    }

    #[tokio::test]
    async fn inbox_push_unregistered_response_clears_token() {
        let store = SessionStore::open_in_memory().unwrap();
        store.add_phone_device("a", "a", "hash-a", 1).unwrap();
        store.set_phone_push_token("a", "token-a").unwrap();
        let fake = fake_fcm(404, r#"{"error":{"status":"NOT_FOUND","details":[{"errorCode":"UNREGISTERED"}]}}"#).await;
        test_sender(&fake).send_inbox_push(&store, &machine(), &item("gate", None)).await;
        assert_eq!(fake.messages.lock().unwrap().len(), 1);
        assert!(store.phone_push_tokens().unwrap().is_empty(), "the dead token is gone");
    }

    #[tokio::test]
    async fn inbox_push_token_is_fetched_once_and_cached() {
        let store = SessionStore::open_in_memory().unwrap();
        store.add_phone_device("a", "a", "hash-a", 1).unwrap();
        store.set_phone_push_token("a", "token-a").unwrap();
        let fake = fake_fcm(200, "{}").await;
        let sender = test_sender(&fake);
        for _ in 0..3 { sender.send_inbox_push(&store, &machine(), &item("gate", None)).await; }
        assert_eq!((fake.messages.lock().unwrap().len(), fake.token_requests.load(Ordering::SeqCst)), (3, 1));
    }

    #[test]
    fn inbox_push_missing_service_account_skips_without_error() {
        let missing = std::env::temp_dir().join(format!("assembly-no-such-key-{}.json", uuid::Uuid::new_v4()));
        assert!(PushSender::from_key_file(&missing).is_none());
        let bad = std::env::temp_dir().join(format!("assembly-bad-key-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&bad, "not json").unwrap();
        assert!(PushSender::from_key_file(&bad).is_none());
    }
}
