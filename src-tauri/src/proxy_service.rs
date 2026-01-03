use crate::store::AccountManagerState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

// Upstream URL: https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io
const ORCHIDS_API_BASE: &str =
    "https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io";

pub struct ProxyService {
    server_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    app_handle: AppHandle,
}

impl ProxyService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            server_handle: Arc::new(Mutex::new(None)),
            app_handle,
        }
    }

    pub async fn start(&self, port: u16) -> Result<String, String> {
        let mut handle_guard = self.server_handle.lock().await;
        if handle_guard.is_some() {
            return Err("Proxy already running".to_string());
        }

        let app_state = self.app_handle.state::<AccountManagerState>();
        let state_for_server = app_state.inner().clone();

        let client = Client::new();

        let app = Router::new()
            .route("/{*path}", any(handle_request))
            .with_state(ProxyState {
                account_state: state_for_server,
                client,
            });

        // Use 127.0.0.1 explicitly
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;

        let server_h = async move {
            axum::serve(listener, app).await.unwrap();
        };

        let handle = tokio::spawn(server_h);
        *handle_guard = Some(handle);

        Ok(format!("Proxy started on http://{}", addr))
    }

    pub async fn stop(&self) -> Result<String, String> {
        let mut handle_guard = self.server_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            return Ok("Proxy stopped".to_string());
        }
        Err("Proxy is not running".to_string())
    }
}

#[derive(Clone)]
struct ProxyState {
    account_state: AccountManagerState,
    client: Client,
}

async fn handle_request(
    State(state): State<ProxyState>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();

    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let mut token_to_use = None;

    // 1. Check for specific account ID in header if provided
    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let potential_id = &auth_str[7..];

            // Access store via the Arc<Mutex> inside AccountManagerState
            let store = state
                .account_state
                .store
                .lock()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if let Some(account) = store.accounts.iter().find(|a| a.id == potential_id) {
                if let Some(cookie) = account.cookies.iter().find(|c| c.name == "__session") {
                    token_to_use = Some(cookie.value.clone());
                }
            }
        }
    }

    // 2. Fallback to Active Account or First Account
    if token_to_use.is_none() {
        let store = state
            .account_state
            .store
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Try Active User
        let active_account = store
            .active_user_id
            .as_ref()
            .and_then(|id| store.accounts.iter().find(|a| &a.id == id));

        if let Some(active) = active_account {
            if let Some(cookie) = active.cookies.iter().find(|c| c.name == "__session") {
                token_to_use = Some(cookie.value.clone());
            }
        }

        // Fallback to first if active fails
        if token_to_use.is_none() {
            if let Some(first) = store.accounts.first() {
                if let Some(cookie) = first.cookies.iter().find(|c| c.name == "__session") {
                    token_to_use = Some(cookie.value.clone());
                }
            }
        }
    }

    let final_token = token_to_use.ok_or(StatusCode::UNAUTHORIZED)?;

    // 3. Construct URL
    // Handle path cleaning if needed.
    // Usually passed path starts with /.
    let target_url = format!(
        "{}{}{}?{}",
        ORCHIDS_API_BASE,
        path,
        if query.is_empty() { "" } else { "?" },
        query
    );

    *req.uri_mut() = target_url
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    req.headers_mut().remove("host");
    req.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", final_token)).unwrap(),
    );

    // 4. Forward
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024)
        .await
        .unwrap_or_default();

    let mut upstream_req = state.client.request(method, &target_url);
    for (k, v) in headers {
        if let Some(name) = k {
            if name != "host" && name != "content-length" {
                upstream_req = upstream_req.header(name, v);
            }
        }
    }
    upstream_req = upstream_req.body(body_bytes);

    let resp = upstream_req
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let resp_bytes = resp
        .bytes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut builder = Response::builder().status(status);
    for (k, v) in resp_headers {
        if let Some(name) = k {
            builder = builder.header(name, v);
        }
    }

    Ok(builder.body(Body::from(resp_bytes)).unwrap())
}
