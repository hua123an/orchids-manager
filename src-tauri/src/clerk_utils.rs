use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CLERK_API_BASE: &str = "https://clerk.orchids.app/v1";
const CLERK_KEY: &str = "pk_live_Y2xlcmsub3JjaGlkcy5hcHAk";

#[derive(Serialize)]
struct SignInPayload {
    identifier: String,
    password: String,
    strategy: String,
}

#[derive(Serialize)]
struct SignUpPayload {
    email_address: String,
    password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    captcha_token: Option<String>,
}

pub fn clerk_login(email: &str, password: &str) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "{}/client/sign_ins?_clerk_js_version=5.0.0&publishable_key={}",
        CLERK_API_BASE, CLERK_KEY
    );

    let payload = SignInPayload {
        identifier: email.to_string(),
        password: password.to_string(),
        strategy: "password".to_string(),
    };

    let resp = client
        .post(&url)
        .header(ORIGIN, "https://www.orchids.app")
        .header(REFERER, "https://www.orchids.app/")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .json(&payload)
        .send()
        .map_err(|e: reqwest::Error| e.to_string())?;

    let status = resp.status();

    // Extract __session cookie
    let mut session_token = String::new();
    for cookie in resp.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(c_str) = cookie.to_str() {
            if c_str.starts_with("__session=") {
                if let Some(val) = c_str.split(';').next() {
                    if let Some(token) = val.strip_prefix("__session=") {
                        session_token = token.to_string();
                    }
                }
            }
        }
    }

    let body = resp.text().map_err(|e: reqwest::Error| e.to_string())?;

    if !status.is_success() {
        if let Ok(json) = serde_json::from_str::<Value>(&body) {
            if let Some(errs) = json["errors"].as_array() {
                if let Some(msg) = errs[0]["message"].as_str() {
                    return Err(msg.to_string());
                }
            }
        }
        return Err(format!("Clerk 登录失败 ({}): {}", status, body));
    }

    if session_token.is_empty() {
        return Err("登录成功但未收到会话 Cookie".to_string());
    }

    Ok(session_token)
}

pub fn clerk_register(
    email: &str,
    password: &str,
    captcha_token: Option<String>,
) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "{}/client/sign_ups?_clerk_js_version=5.0.0&publishable_key={}",
        CLERK_API_BASE, CLERK_KEY
    );

    let payload = SignUpPayload {
        email_address: email.to_string(),
        password: password.to_string(),
        captcha_token,
    };

    println!("Payload: {:?}", serde_json::to_string(&payload));

    let resp = client
        .post(&url)
        .header(ORIGIN, "https://accounts.orchids.app")
        .header(REFERER, "https://accounts.orchids.app/sign-up")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .json(&payload)
        .send()
        .map_err(|e: reqwest::Error| e.to_string())?;

    let status = resp.status();
    let body = resp.text().map_err(|e: reqwest::Error| e.to_string())?;

    println!("Register Response Status: {}", status);
    println!("Register Response Body: {}", body);

    if !status.is_success() {
        if let Ok(json) = serde_json::from_str::<Value>(&body) {
            if let Some(errs) = json["errors"].as_array() {
                if let Some(msg) = errs[0]["message"].as_str() {
                    // Include code/long message
                    let code = errs[0]["code"].as_str().unwrap_or("unknown_code");
                    return Err(format!("Clerk 错误 [{}]: {}", code, msg));
                }
            }
        }
        return Err(format!("Clerk 注册失败 ({}): {}", status, body));
    }

    Ok(body)
}

#[derive(Serialize)]
struct PrepareVerificationPayload {
    strategy: String,
}

#[derive(Serialize)]
struct AttemptVerificationPayload {
    code: String,
    strategy: String,
}

pub fn clerk_prepare_verification(sign_up_id: &str) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "{}/client/sign_ups/{}/prepare_verification?_clerk_js_version=5.0.0&publishable_key={}",
        CLERK_API_BASE, sign_up_id, CLERK_KEY
    );

    let payload = PrepareVerificationPayload {
        strategy: "email_code".to_string(),
    };

    let resp = client
        .post(&url)
        .header(ORIGIN, "https://www.orchids.app")
        .header(REFERER, "https://www.orchids.app/")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .json(&payload)
        .send()
        .map_err(|e: reqwest::Error| e.to_string())?;

    let status = resp.status();
    let body = resp.text().map_err(|e: reqwest::Error| e.to_string())?;

    if !status.is_success() {
        return Err(format!("准备验证失败 ({}): {}", status, body));
    }
    Ok(body)
}

pub fn clerk_attempt_verification(sign_up_id: &str, code: &str) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "{}/client/sign_ups/{}/attempt_verification?_clerk_js_version=5.0.0&publishable_key={}",
        CLERK_API_BASE, sign_up_id, CLERK_KEY
    );

    let payload = AttemptVerificationPayload {
        code: code.to_string(),
        strategy: "email_code".to_string(),
    };

    let resp = client
        .post(&url)
        .header(ORIGIN, "https://www.orchids.app")
        .header(REFERER, "https://www.orchids.app/")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .json(&payload)
        .send()
        .map_err(|e: reqwest::Error| e.to_string())?;

    let status = resp.status();

    // Check for Set-Cookie __session
    let mut session_token = String::new();
    for cookie in resp.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(c_str) = cookie.to_str() {
            if c_str.starts_with("__session=") {
                if let Some(val) = c_str.split(';').next() {
                    if let Some(token) = val.strip_prefix("__session=") {
                        session_token = token.to_string();
                    }
                }
            }
        }
    }

    let body = resp.text().map_err(|e: reqwest::Error| e.to_string())?;

    if !status.is_success() {
        return Err(format!("尝试验证失败 ({}): {}", status, body));
    }

    if !session_token.is_empty() {
        return Ok(session_token);
    }

    Ok(body)
}
