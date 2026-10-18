//! The phone-only listener: pairing codes and device tokens (phone spec §5.2).
//!
//! It runs beside the desktop listener when `ASSEMBLY_PHONE_BIND` is set and
//! serves only `/phone/v1`; the desktop's full-control `/assembly` socket and
//! its server token are never reachable through it.

use std::io::Read;
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::remote::ServerState;

/// How long a pairing code works: 10 minutes.
pub(super) const PAIRING_CODE_LIFETIME_MS: i64 = 10 * 60 * 1000;
const WRONG_TRIES_ALLOWED: u32 = 5;
const NOT_SET_UP: &str =
    "Phone access is not set up on this machine: set ASSEMBLY_PHONE_URL in ~/.config/assembly/phone.env";
const REMOVED: &str = "This phone was removed — pair again";

/// The one pairing code that works right now, kept in memory only.
pub(super) struct PairingCode {
    code: String,
    expires_at_ms: i64,
    wrong_tries: u32,
}

/// The phone listener's routes. It never serves the desktop's `/assembly` socket.
pub(super) fn router() -> Router<ServerState> {
    Router::new()
        .route("/phone/v1/pair", post(pair))
        .route("/phone/v1/me", get(me))
}

/// Answers the desktop's phone operations, sent as workflow operations over its
/// socket; `None` for every other operation, which goes to the workflow engine.
pub(super) fn operation(state: &ServerState, operation: &str, args: &Value) -> Option<Result<Value, String>> {
    let store = state.manager.store();
    Some(match operation {
        "phonePairing" => start_pairing(state, args),
        "phoneDevices" => store.phone_devices().map(|devices| json!({ "devices": devices })).map_err(|error| error.to_string()),
        "phoneRevoke" => match args["deviceId"].as_str() {
            Some(device_id) => store.revoke_phone_device(device_id, now_ms())
                .map(|revoked| json!({ "revoked": revoked })).map_err(|error| error.to_string()),
            None => Err("phoneRevoke needs a deviceId".into()),
        },
        _ => return None,
    })
}

/// Names this machine for phones and replaces the pairing code with a new one.
fn start_pairing(state: &ServerState, args: &Value) -> Result<Value, String> {
    let url = std::env::var("ASSEMBLY_PHONE_URL").ok().filter(|url| !url.is_empty()).ok_or(NOT_SET_UP)?;
    let name = args["machineName"].as_str().ok_or("phonePairing needs a machineName")?;
    let (_, machine_name) = state.manager.store()
        .name_phone_machine(&uuid::Uuid::new_v4().to_string(), name).map_err(|error| error.to_string())?;
    let (code, expires_at) = new_pairing_code(&state.pairing, now_ms())?;
    Ok(json!({ "url": url, "machineName": machine_name, "code": code, "expiresAt": expires_at }))
}

/// Makes a 10-character base32 code that replaces any earlier one. Returns it and its expiry (ms).
pub(super) fn new_pairing_code(slot: &Mutex<Option<PairingCode>>, now_ms: i64) -> Result<(String, i64), String> {
    const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let code: String = random_bytes::<10>()?.iter().map(|byte| char::from(BASE32[usize::from(byte & 31)])).collect();
    let expires_at_ms = now_ms + PAIRING_CODE_LIFETIME_MS;
    *slot.lock().unwrap_or_else(PoisonError::into_inner) =
        Some(PairingCode { code: code.clone(), expires_at_ms, wrong_tries: 0 });
    Ok((code, expires_at_ms))
}

/// Uses up the code when `code` matches it in time. A wrong guess counts
/// toward the five that kill it; an expired code is dropped.
pub(super) fn redeem_pairing_code(slot: &Mutex<Option<PairingCode>>, code: &str, now_ms: i64) -> bool {
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(current) = slot.as_mut() else { return false };
    if now_ms > current.expires_at_ms {
        *slot = None;
        return false;
    }
    if current.code == code {
        *slot = None;
        return true;
    }
    current.wrong_tries += 1;
    if current.wrong_tries >= WRONG_TRIES_ALLOWED {
        *slot = None;
    }
    false
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequest {
    code: String,
    device_name: String,
}

/// `POST /phone/v1/pair {code, deviceName}` → `{deviceId, deviceToken, machine:{id, name}}`.
async fn pair(State(state): State<ServerState>, Json(request): Json<PairRequest>) -> Response {
    if !redeem_pairing_code(&state.pairing, &request.code, now_ms()) {
        return refuse(StatusCode::UNAUTHORIZED, "This pairing code is wrong or has expired; show a new one on the desktop");
    }
    match add_device(&state, &request.device_name) {
        Ok(body) => Json(body).into_response(),
        Err(error) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &error),
    }
}

fn add_device(state: &ServerState, device_name: &str) -> Result<Value, String> {
    let store = state.manager.store();
    let (machine_id, machine_name) = store.phone_machine().map_err(|error| error.to_string())?
        .ok_or("This machine has not been named for phones yet")?;
    let device_id = uuid::Uuid::new_v4().to_string();
    let token = URL_SAFE_NO_PAD.encode(random_bytes::<32>()?);
    store.add_phone_device(&device_id, device_name, &token_hash(&token), now_ms()).map_err(|error| error.to_string())?;
    Ok(json!({ "deviceId": device_id, "deviceToken": token, "machine": { "id": machine_id, "name": machine_name } }))
}

/// `GET /phone/v1/me` → `{deviceId, machine:{id, name}, hostsOrchestrator, orchestratorMarkedAt}`.
async fn me(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    let device_id = match authorized_device(&state, &headers) {
        Ok(device_id) => device_id,
        Err(response) => return response,
    };
    let store = state.manager.store();
    let body = store.phone_machine().and_then(|machine| Ok((machine, store.main_orchestrator()?)))
        .map(|(machine, orchestrator)| json!({
            "deviceId": device_id,
            "machine": machine.map(|(id, name)| json!({ "id": id, "name": name })),
            "hostsOrchestrator": orchestrator.is_some(),
            "orchestratorMarkedAt": orchestrator.map(|(_, marked_at)| marked_at),
        }));
    match body {
        Ok(body) => Json(body).into_response(),
        Err(error) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
    }
}

/// The paired device whose bearer token this is, found among non-revoked
/// devices in one SQL statement that also marks it seen. Anything else is 401.
fn authorized_device(state: &ServerState, headers: &HeaderMap) -> Result<String, Response> {
    let token = headers.get("authorization").and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| refuse(StatusCode::UNAUTHORIZED, REMOVED))?;
    match state.manager.store().touch_phone_device(&token_hash(token), now_ms()) {
        Ok(Some(device_id)) => Ok(device_id),
        Ok(None) => Err(refuse(StatusCode::UNAUTHORIZED, REMOVED)),
        Err(error) => Err(refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string())),
    }
}

fn refuse(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

/// Only this hash of a device token is stored.
fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    std::fs::File::open("/dev/urandom").and_then(|mut source| source.read_exact(&mut bytes))
        .map_err(|error| format!("Could not read random bytes: {error}"))?;
    Ok(bytes)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::future::IntoFuture;
    use std::sync::{Arc, Mutex};

    use super::super::manager::AgentRuntimeManager;
    use super::super::providers::ProviderRegistry;
    use super::super::remote::{server_state, ServerState};
    use super::{new_pairing_code, operation, redeem_pairing_code, router, PAIRING_CODE_LIFETIME_MS};

    const PHONE_URL: &str = "https://phone.example.ts.net:8778";

    async fn phone_server() -> (ServerState, String) {
        let directory = std::env::temp_dir().join(format!("assembly-phone-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &directory.join("sessions.db")).unwrap();
        let state = server_state(manager, directory, Arc::from("desktop-server-token-0123456789abcdef")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(axum::serve(listener, router().with_state(state.clone())).into_future());
        (state, format!("http://{address}"))
    }

    async fn pair(base: &str, code: &str) -> reqwest::Response {
        reqwest::Client::new().post(format!("{base}/phone/v1/pair"))
            .json(&serde_json::json!({"code": code, "deviceName": "Test phone"})).send().await.unwrap()
    }

    async fn me(base: &str, authorization: Option<&str>) -> reqwest::StatusCode {
        let request = reqwest::Client::new().get(format!("{base}/phone/v1/me"));
        let request = match authorization { Some(value) => request.header("authorization", value), None => request };
        request.send().await.unwrap().status()
    }

    fn pairing_code(state: &ServerState) -> String {
        // Edition 2021: setting the variable is safe; every phone test sets the same value.
        std::env::set_var("ASSEMBLY_PHONE_URL", PHONE_URL);
        let created = operation(state, "phonePairing", &serde_json::json!({"machineName": "Workbox Test"}))
            .expect("phonePairing is a phone operation").unwrap();
        assert_eq!(created["url"], PHONE_URL);
        assert_eq!(created["machineName"], "Workbox Test");
        created["code"].as_str().unwrap().to_string()
    }

    #[test]
    fn pairing_code_is_single_use_and_expires() {
        let slot = Mutex::new(None);
        let (code, expires_at) = new_pairing_code(&slot, 1_000).unwrap();
        assert_eq!(code.len(), 10);
        assert!(code.bytes().all(|byte| b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(&byte)));
        assert_eq!(expires_at, 1_000 + PAIRING_CODE_LIFETIME_MS);
        assert!(redeem_pairing_code(&slot, &code, 2_000));
        assert!(!redeem_pairing_code(&slot, &code, 2_000), "a code pairs one phone only");

        let (code, expires_at) = new_pairing_code(&slot, 1_000).unwrap();
        assert!(!redeem_pairing_code(&slot, &code, expires_at + 1), "an expired code is refused");

        let (old, _) = new_pairing_code(&slot, 1_000).unwrap();
        let (new, _) = new_pairing_code(&slot, 1_000).unwrap();
        assert!(!redeem_pairing_code(&slot, &old, 2_000), "a new code replaces the old one");
        assert!(redeem_pairing_code(&slot, &new, 2_000));
    }

    #[test]
    fn pairing_code_dies_after_five_wrong_tries() {
        let slot = Mutex::new(None);
        let (code, _) = new_pairing_code(&slot, 0).unwrap();
        for _ in 0..4 { assert!(!redeem_pairing_code(&slot, "WRONGWRONG", 1)); }
        assert!(redeem_pairing_code(&slot, &code, 1), "four wrong tries leave the code alive");

        let (code, _) = new_pairing_code(&slot, 0).unwrap();
        for _ in 0..5 { assert!(!redeem_pairing_code(&slot, "WRONGWRONG", 1)); }
        assert!(!redeem_pairing_code(&slot, &code, 1), "the fifth wrong try kills the code");
    }

    #[tokio::test]
    async fn phone_routes_reject_missing_unknown_and_revoked_tokens() {
        let (state, base) = phone_server().await;
        let paired = pair(&base, &pairing_code(&state)).await;
        assert_eq!(paired.status(), reqwest::StatusCode::OK);
        let paired: serde_json::Value = paired.json().await.unwrap();
        let token = paired["deviceToken"].as_str().unwrap();
        assert_eq!(paired["machine"]["name"], "Workbox Test");

        assert_eq!(me(&base, None).await, reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(me(&base, Some("Bearer not-a-device-token")).await, reqwest::StatusCode::UNAUTHORIZED);
        let response = reqwest::Client::new().get(format!("{base}/phone/v1/me"))
            .bearer_auth(token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["deviceId"], paired["deviceId"]);
        assert_eq!(body["machine"], paired["machine"]);
        assert_eq!(body["hostsOrchestrator"], false);

        let devices = operation(&state, "phoneDevices", &serde_json::json!({})).unwrap().unwrap();
        assert_eq!(devices["devices"][0]["id"], paired["deviceId"]);
        assert_eq!(devices["devices"][0]["name"], "Test phone");
        operation(&state, "phoneRevoke", &serde_json::json!({"deviceId": paired["deviceId"]})).unwrap().unwrap();
        assert_eq!(me(&base, Some(&format!("Bearer {token}"))).await, reqwest::StatusCode::UNAUTHORIZED);
        let devices = operation(&state, "phoneDevices", &serde_json::json!({})).unwrap().unwrap();
        assert_eq!(devices["devices"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn phone_listener_does_not_serve_desktop_socket() {
        let (state, base) = phone_server().await;
        let response = reqwest::Client::new().get(format!("{base}/assembly"))
            .bearer_auth(&*state.token).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        assert!(operation(&state, "list", &serde_json::json!({})).is_none(), "engine operations are not phone operations");
    }

    #[tokio::test]
    async fn desktop_server_token_is_not_a_phone_token() {
        let (state, base) = phone_server().await;
        assert_eq!(pair(&base, &pairing_code(&state)).await.status(), reqwest::StatusCode::OK);
        assert_eq!(me(&base, Some(&format!("Bearer {}", state.token))).await, reqwest::StatusCode::UNAUTHORIZED);
    }
}
