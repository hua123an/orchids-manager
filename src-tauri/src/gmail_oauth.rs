use base64::Engine;
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use url::Url;

// Your OAuth credentials
const CLIENT_ID: &str = "490083629532-4us5kdsak228rrhcts7qc31qclrceqbr.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "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";
const REDIRECT_URI: &str = "http://127.0.0.1:8085";
const SCOPES: &str = "https://mail.google.com/";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: Option<String>,
}

// Global token storage (in production, use secure storage)
lazy_static::lazy_static! {
    pub static ref GMAIL_TOKENS: Mutex<Option<OAuthTokens>> = Mutex::new(None);
    pub static ref GMAIL_USER_EMAIL: Mutex<Option<String>> = Mutex::new(None);
}

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn get_token_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("gmail_tokens.json")
}

pub fn save_tokens_to_disk(app: &AppHandle, tokens: &OAuthTokens) {
    let path = get_token_path(app);
    if let Ok(json) = serde_json::to_string_pretty(tokens) {
        let _ = std::fs::write(path, json);
    }
}

pub fn try_load_from_disk(app: &AppHandle) -> bool {
    // If already in memory, return true
    if GMAIL_TOKENS.lock().unwrap().is_some() {
        return true;
    }

    let path = get_token_path(app);
    if !path.exists() {
        return false;
    }

    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(tokens) = serde_json::from_str::<OAuthTokens>(&content) {
            *GMAIL_TOKENS.lock().unwrap() = Some(tokens);
            return true;
        }
    }
    false
}

/// Generate the OAuth authorization URL
pub fn get_auth_url() -> String {
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
        client_id={}&\
        redirect_uri={}&\
        response_type=code&\
        scope={}&\
        access_type=offline&\
        prompt=consent",
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(SCOPES)
    )
}

/// Start OAuth flow: opens browser and waits for callback
pub fn start_oauth_flow() -> Result<OAuthTokens, String> {
    let auth_url = get_auth_url();

    // Open browser
    open::that(&auth_url).map_err(|e| format!("Failed to open browser: {}", e))?;

    // Start local server to receive callback
    let listener = TcpListener::bind("127.0.0.1:8085")
        .map_err(|e| format!("Failed to start callback server: {}", e))?;

    println!("Waiting for OAuth callback on http://127.0.0.1:8085 ...");

    // Accept one connection
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("Failed to accept connection: {}", e))?;

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| format!("Failed to read request: {}", e))?;

    // Parse the authorization code from the request
    // Example: GET /?code=4/0AfJoh...&scope=... HTTP/1.1
    let code = extract_code_from_request(&request_line)?;

    // Send success response to browser
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body style='font-family: sans-serif; text-align: center; padding: 50px;'>\
        <h1>✅ Authorization Successful!</h1>\
        <p>You can close this window and return to the app.</p>\
        </body></html>";
    stream.write_all(response.as_bytes()).ok();

    // Exchange code for tokens
    let tokens = exchange_code_for_tokens(&code)?;

    // Store tokens
    *GMAIL_TOKENS.lock().unwrap() = Some(tokens.clone());

    Ok(tokens)
}

fn extract_code_from_request(request: &str) -> Result<String, String> {
    // Parse: GET /?code=XXX&scope=... HTTP/1.1
    let parts: Vec<&str> = request.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid request format".to_string());
    }

    let path = parts[1];
    let url = Url::parse(&format!("http://localhost{}", path))
        .map_err(|e| format!("Failed to parse URL: {}", e))?;

    for (key, value) in url.query_pairs() {
        if key == "code" {
            return Ok(value.to_string());
        }
        if key == "error" {
            return Err(format!("OAuth error: {}", value));
        }
    }

    Err("No authorization code found".to_string())
}

fn exchange_code_for_tokens(code: &str) -> Result<OAuthTokens, String> {
    let client = Client::new();

    // Build form body manually
    let body = format!(
        "client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(CLIENT_SECRET),
        urlencoding::encode(code),
        urlencoding::encode(REDIRECT_URI)
    );

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .map_err(|e| format!("Token request failed: {}", e))?;

    let status = response.status();
    let body_text = response.text().map_err(|e: reqwest::Error| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Token exchange failed: {}", body_text));
    }

    let tokens: OAuthTokens = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to parse tokens: {} - Body: {}", e, body_text))?;

    Ok(tokens)
}

/// Refresh the access token using refresh token
pub fn refresh_access_token(refresh_token: &str) -> Result<OAuthTokens, String> {
    let client = Client::new();

    let body = format!(
        "client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token",
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(CLIENT_SECRET),
        urlencoding::encode(refresh_token)
    );

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .map_err(|e| format!("Token refresh failed: {}", e))?;

    let status = response.status();
    let body_text = response.text().map_err(|e: reqwest::Error| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Token refresh failed: {}", body_text));
    }

    let mut tokens: OAuthTokens =
        serde_json::from_str(&body_text).map_err(|e| format!("Failed to parse tokens: {}", e))?;

    // Refresh response doesn't include refresh_token, keep the old one
    if tokens.refresh_token.is_none() {
        tokens.refresh_token = Some(refresh_token.to_string());
    }

    // Update stored tokens
    *GMAIL_TOKENS.lock().unwrap() = Some(tokens.clone());

    Ok(tokens)
}

#[derive(Debug, Deserialize)]
struct MessageListResponse {
    messages: Option<Vec<MessageSummary>>,
}

#[derive(Debug, Deserialize)]
struct MessageSummary {
    id: String,
}

#[derive(Debug, Deserialize)]
struct MessageDetail {
    snippet: Option<String>,
    payload: Option<MessagePayload>,
}

#[derive(Debug, Deserialize)]
struct MessagePayload {
    body: Option<MessageBody>,
    parts: Option<Vec<MessagePart>>,
}

#[derive(Debug, Deserialize)]
struct MessagePart {
    mimeType: Option<String>,
    body: Option<MessageBody>,
    parts: Option<Vec<MessagePart>>,
}

#[derive(Debug, Deserialize)]
struct MessageBody {
    data: Option<String>,
}

/// Fetch verification code (only emails received AFTER min_timestamp)
pub fn fetch_verification_code_api(app: &AppHandle, min_timestamp: i64) -> Result<String, String> {
    // Ensure we have a valid token
    let token = get_access_token()?;

    let client = Client::new();

    // Query: in:inbox AND after:timestamp
    let query = format!("in:inbox after:{}", min_timestamp);
    let url = Url::parse_with_params(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages",
        &[("maxResults", "5"), ("q", &query)],
    )
    .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| format!("API list failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("API list error: {}", res.status()));
    }

    let list_response: MessageListResponse = res
        .json()
        .map_err(|e| format!("Parse list failed: {}", e))?;

    let messages = list_response.messages.ok_or("No messages found")?;

    // Regex for 6-digit code
    let re = regex::Regex::new(r"\b\d{6}\b").map_err(|e| e.to_string())?;

    // 2. Fetch details for each message
    for msg in messages {
        let detail_url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
            msg.id
        );
        let vid_res = client
            .get(&detail_url)
            .header("Authorization", format!("Bearer {}", token))
            .send();

        if let Ok(resp) = vid_res {
            if let Ok(detail) = resp.json::<MessageDetail>() {
                // Check snippet first (fastest)
                if let Some(snippet) = &detail.snippet {
                    if let Some(code) = extract_code_from_text(snippet, &re) {
                        return Ok(code);
                    }
                }

                // Check payload body
                if let Some(payload) = detail.payload {
                    let full_text = extract_text_from_payload(&payload);
                    if let Some(code) = extract_code_from_text(&full_text, &re) {
                        return Ok(code);
                    }
                }
            }
        }
    }

    Err("No 6-digit code found in recent emails".to_string())
}

fn extract_text_from_payload(payload: &MessagePayload) -> String {
    let mut text = String::new();

    if let Some(parts) = &payload.parts {
        for part in parts {
            if let Some(mime) = &part.mimeType {
                if mime == "text/plain" {
                    if let Some(body) = &part.body {
                        if let Some(data) = &body.data {
                            if let Ok(decoded) =
                                base64::engine::general_purpose::URL_SAFE.decode(data)
                            {
                                text.push_str(&String::from_utf8_lossy(&decoded));
                            }
                        }
                    }
                } else if mime.starts_with("multipart") {
                    if let Some(sub_parts) = &part.parts {
                        // Recursive simplified
                        // For now just ignore deep nesting to avoid complexity, usually text/plain is top level or parallel
                    }
                }
            }
        }
    } else if let Some(body) = &payload.body {
        if let Some(data) = &body.data {
            if let Ok(decoded) = base64::engine::general_purpose::URL_SAFE.decode(data) {
                text.push_str(&String::from_utf8_lossy(&decoded));
            }
        }
    }
    text
}

fn extract_code_from_text(text: &str, re: &regex::Regex) -> Option<String> {
    for mat in re.find_iter(text) {
        let code = mat.as_str();
        let start = mat.start();
        if start > 0 {
            // Check for hex color #123456
            let prefix = &text[start - 1..start];
            if prefix == "#" {
                continue;
            }
        }
        return Some(code.to_string());
    }
    None
}

/// Get current access token (refresh if needed)
pub fn get_access_token() -> Result<String, String> {
    let tokens = GMAIL_TOKENS.lock().unwrap().clone();

    match tokens {
        Some(t) => Ok(t.access_token),
        None => Err("Not authenticated. Please run OAuth flow first.".to_string()),
    }
}

/// Check if we have valid tokens
pub fn is_authenticated() -> bool {
    GMAIL_TOKENS.lock().unwrap().is_some()
}

/// Store user email (obtained from first successful IMAP connection)
pub fn set_gmail_user(email: &str) {
    *GMAIL_USER_EMAIL.lock().unwrap() = Some(email.to_string());
}

pub fn get_gmail_user() -> Option<String> {
    GMAIL_USER_EMAIL.lock().unwrap().clone()
}
