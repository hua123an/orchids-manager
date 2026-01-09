mod capture_service;
mod checkpoint_manager;
mod clerk_utils;
mod cookie_reader;
mod gmail_oauth;
mod imap_service;
mod injection;
mod models;
mod proxy_service;
mod store;

use capture_service::CaptureService;
use injection::OrchidsCore;
use models::Account;
use proxy_service::ProxyService;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use store::AccountManagerState;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

// Global state for manual verification code input
lazy_static::lazy_static! {
    static ref MANUAL_VERIFICATION_CODE: Mutex<Option<String>> = Mutex::new(None);
}

// --- CORE COMMANDS ---

#[tauri::command]
fn ensure_patched() -> Result<String, String> {
    OrchidsCore::ensure_patched()
}

#[tauri::command]
fn set_active_account(
    app: AppHandle,
    state: State<'_, AccountManagerState>,
    capture: State<'_, Arc<CaptureService>>,
    id: String,
) -> Result<(), String> {
    capture.stop();
    state.set_active(id.clone())?;
    let accounts = state.get_accounts()?;
    if let Some(account) = accounts.iter().find(|a| a.id == id) {
        OrchidsCore::switch_account(&app, account)?;
    }
    Ok(())
}

#[tauri::command]
fn reset_machine_id(app: AppHandle) -> Result<String, String> {
    // Re-using the manual logic we vetted
    let data_dir = capture_service::get_orchids_data_dir();
    let updater_path = data_dir.join(".updaterId");

    let output = std::process::Command::new("uuidgen")
        .output()
        .map_err(|e| e.to_string())?;
    let new_uuid = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();

    if let Some(parent) = updater_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&updater_path, &new_uuid).map_err(|e| e.to_string())?;

    // Clear State
    let cookie_path = capture_service::get_orchids_cookie_path();
    if cookie_path.exists() {
        let _ = std::fs::remove_file(&cookie_path);
    }
    let ls_path = data_dir.join("Local Storage");
    if ls_path.exists() {
        let _ = std::fs::remove_dir_all(ls_path);
    }

    Ok(format!("ID Reset: {}", new_uuid))
}

// --- UTILS ---

#[tauri::command]
fn get_accounts(state: State<'_, AccountManagerState>) -> Result<Vec<Account>, String> {
    state.get_accounts()
}

#[tauri::command]
fn save_account(state: State<'_, AccountManagerState>, account: Account) -> Result<(), String> {
    state.add_account(account)
}

#[tauri::command]
fn delete_account(state: State<'_, AccountManagerState>, id: String) -> Result<(), String> {
    state.delete_account(id)
}

#[tauri::command]
fn start_listener(app: AppHandle, state: State<'_, Arc<CaptureService>>) -> Result<String, String> {
    state.start_monitoring(app);
    Ok("监听已启动".into())
}

#[tauri::command]
fn stop_listener(state: State<'_, Arc<CaptureService>>) -> Result<String, String> {
    state.stop();
    Ok("监听已停止".into())
}

#[tauri::command]
fn get_active_id(state: State<'_, AccountManagerState>) -> Result<Option<String>, String> {
    state.get_active_id()
}

#[tauri::command]
fn import_current_session(state: State<'_, AccountManagerState>) -> Result<Account, String> {
    let path_buf = capture_service::get_orchids_cookie_path();
    let path = path_buf.as_path();

    if !path.exists() {
        return Err("未找到 Orchids Cookie 文件。请打开 Orchids 应用。".into());
    }

    match capture_service::process_cookies(path) {
        Ok(Some(account)) => {
            state.add_account(account.clone())?;
            Ok(account)
        }
        Ok(None) => Err("未在 Orchids 中找到活跃会话。请登录。".into()),
        Err(e) => Err(format!("读取会话失败: {}", e)),
    }
}

// --- PROXY / IMAP / OAUTH WRAPPERS (Restored for Compilation) ---

#[tauri::command]
async fn start_proxy(state: State<'_, Arc<ProxyService>>, port: u16) -> Result<String, String> {
    state.start(port).await
}

#[tauri::command]
async fn stop_proxy(state: State<'_, Arc<ProxyService>>) -> Result<String, String> {
    state.stop().await
}

#[tauri::command]
async fn check_imap_code(
    host: String,
    port: u16,
    user: String,
    pass: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        imap_service::fetch_verification_code(&host, port, &user, &pass)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn gmail_oauth_start(app: AppHandle) -> Result<String, String> {
    let auth_url = gmail_oauth::get_auth_url();

    // Create a WebView window for OAuth
    let oauth_window = WebviewWindowBuilder::new(
        &app,
        "oauth_popup",
        WebviewUrl::External(auth_url.parse().unwrap()),
    )
    .title("Google 授权登录")
    .inner_size(500.0, 650.0)
    .center()
    .on_navigation({
        let app_handle = app.clone();
        move |url| {
            let u = url.as_str();
            // Intercept the redirect to localhost callback
            if u.starts_with("http://127.0.0.1:8085") {
                // Parse the code from the URL
                if let Ok(parsed) = url::Url::parse(u) {
                    for (key, value) in parsed.query_pairs() {
                        if key == "code" {
                            // Exchange code for tokens in background
                            let code = value.to_string();
                            let app_clone = app_handle.clone();
                            std::thread::spawn(move || {
                                match gmail_oauth::exchange_and_save_tokens(&app_clone, &code) {
                                    Ok(_) => {
                                        let _ = app_clone
                                            .emit("automation_log", "✅ Google 授权成功！");
                                    }
                                    Err(e) => {
                                        let _ = app_clone
                                            .emit("automation_log", format!("❌ 授权失败: {}", e));
                                    }
                                }
                                // Close the OAuth window
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                if let Some(w) = app_clone.get_webview_window("oauth_popup") {
                                    let _ = w.close();
                                }
                            });
                            return false; // Prevent navigation
                        }
                        if key == "error" {
                            let _ = app_handle
                                .emit("automation_log", format!("❌ 授权错误: {}", value));
                            return false;
                        }
                    }
                }
                return false;
            }
            true
        }
    })
    .build()
    .map_err(|e| e.to_string())?;

    Ok("OAuth Window Opened".to_string())
}

#[tauri::command]
fn gmail_oauth_status(app: AppHandle) -> Result<String, String> {
    if !gmail_oauth::is_authenticated() {
        gmail_oauth::try_load_from_disk(&app);
    }
    Ok(if gmail_oauth::is_authenticated() {
        "Authenticated"
    } else {
        "Not Authenticated"
    }
    .to_string())
}

#[tauri::command]
async fn check_imap_code_oauth(app: AppHandle, _email: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || gmail_oauth::fetch_verification_code_api(&app, 0))
        .await
        .map_err(|e| e.to_string())?
}

/// Set manual verification code (fallback when auto-fetch fails)
#[tauri::command]
fn set_manual_code(code: String) -> Result<String, String> {
    // Validate the code (must be 4-8 digits)
    let trimmed = code.trim();
    if trimmed.len() < 4 || trimmed.len() > 8 {
        return Err("验证码长度必须在 4-8 位之间".to_string());
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err("验证码必须全部为数字".to_string());
    }

    *MANUAL_VERIFICATION_CODE.lock().unwrap() = Some(trimmed.to_string());
    Ok(format!("手动验证码已设置: {}", trimmed))
}

/// Get manual verification code
#[tauri::command]
fn get_manual_code() -> Option<String> {
    MANUAL_VERIFICATION_CODE.lock().unwrap().clone()
}

/// Clear manual verification code
#[tauri::command]
fn clear_manual_code() {
    *MANUAL_VERIFICATION_CODE.lock().unwrap() = None;
}

#[tauri::command]
async fn refresh_active_account(state: State<'_, AccountManagerState>) -> Result<Account, String> {
    let active_id = state.get_active_id()?.ok_or("No Active Account")?;
    let accounts = state.get_accounts()?;
    let account = accounts
        .iter()
        .find(|a| a.id == active_id)
        .ok_or("Account Not Found")?;

    let token = account
        .cookies
        .iter()
        .find(|c| c.name == "__session")
        .map(|c| c.value.clone())
        .ok_or("No Token")?;
    let user_id = account.id.clone();

    let new_credits = tauri::async_runtime::spawn_blocking(move || {
        capture_service::fetch_fresh_credits(&user_id, &token)
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut updated_account = account.clone();
    if let Some(ref mut info) = updated_account.user_info {
        info.credits = Some(new_credits);
    }
    state.add_account(updated_account.clone())?;
    Ok(updated_account)
}

// --- COMPLEX REGISTRATION ---

#[tauri::command]
async fn clerk_action_register_webview(
    app: AppHandle,
    email: String,
    pass: String,
    imap_host: String,
    imap_port: u16,
    imap_user: String,
    imap_pass: String,
    capture_port: u16,
) -> Result<String, String> {
    // 1. Reset Machine ID
    let _ = reset_machine_id(app.clone());
    let _ = app.emit("imap_log", "Identity Reset. Restarting App...".to_string());

    // Restart App
    use std::process::Command;
    let _ = Command::new("killall").arg("Orchids").status();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let _ = Command::new("/usr/bin/open")
        .arg("-a")
        .arg("Orchids")
        .spawn();

    // 2. Click "Log in" via AppleScript
    std::thread::sleep(std::time::Duration::from_millis(3000));
    let _ = Command::new("osascript").arg("-e").arg(r#"
        tell application "System Events"
            if exists process "Orchids" then
                tell process "Orchids"
                    set frontmost to true
                    try
                        if exists button "Log in" of window 1 then click button "Log in" of window 1
                        else if exists button "Sign in" of window 1 then click button "Sign in" of window 1
                        else key code 36
                        end if
                    end try
                end tell
            end if
        end tell
    "#).output();

    // 3. Open Webview
    let url_str = "https://accounts.orchids.app/sign-up";
    let init_script = format!(
        r#"
        (function() {{
            const email = "{}";
            const pass = "{}";
            const capturePort = {};

            // --- LOGGING ---
            function log(msg) {{
                console.log(msg);
                try {{
                    const req = new XMLHttpRequest();
                    req.open("POST", "http://127.0.0.1:" + capturePort + "/log");
                    req.send(msg);
                }} catch (e) {{}}
            }}
            log("🚀 注入脚本已启动! 目标: " + email);
            
            // Debug Inputs
            setTimeout(() => {{
                const inputs = Array.from(document.querySelectorAll('input'));
                const details = inputs.map(i => `[name=${{i.name}} type=${{i.type}} id=${{i.id}} auto=${{i.autocomplete}}]`).join('; ');
                log(`🔍 页面输入框检测 (${{inputs.length}}): ${{details}}`);
            }}, 3000);

            console.log("Orchids Injection Started");

            // --- 1. Session Capture & Deep Link Flow ---
            // Phase 1: When session is captured, redirect to desktop auth page
            setInterval(() => {{
                if (document.cookie.includes('__session') && !window.location.href.includes('desktop/auth')) {{
                    if (!window.sessionRedirected) {{
                        window.sessionRedirected = true;
                        console.log("Session detected! Reporting to backend...");
                        
                        // Use Image (Passive Mixed Content) to bypass HTTPS->HTTP blocking
                        new Image().src = 'http://127.0.0.1:' + capturePort + '/?cookie=' + encodeURIComponent(document.cookie);

                        // Then redirect to get the deeplink
                        setTimeout(() => {{
                            console.log("Redirecting to desktop auth...");
                            window.location.href = "https://orchids.app/desktop/auth";
                        }}, 1500);
                    }}
                }}
            }}, 1000);
            
            // Phase 2: On desktop/auth page, get deeplink and open Orchids app
            setInterval(() => {{
                if (window.location.href.includes("desktop/auth")) {{
                     // Show progress indicator
                     if (!window.authPageShown) {{
                         window.authPageShown = true;
                         console.log("On desktop/auth page, looking for deeplink...");
                     }}
                     
                     // Method 1: Find deeplink in page content
                     const html = document.body.innerHTML;
                     const match = html.match(/orchids:\/\/[^"'\s<>]+/);
                     if(match && match[0]) {{
                         let link = match[0].replace(/&amp;/g, '&');
                         if(!window.lastSentLink || window.lastSentLink !== link) {{
                             window.lastSentLink = link;
                             console.log("DeepLink found: " + link);
                             // Report to backend to open Orchids app
                             new Image().src = 'http://127.0.0.1:' + capturePort + '/?deeplink=' + encodeURIComponent(link);
                             // Also try clicking the button
                             const openBtn = Array.from(document.querySelectorAll('button, a')).find(b => b.innerText && b.innerText.includes('Open Orchids'));
                             if(openBtn && !window.btnClicked) {{
                                window.btnClicked = true;
                                openBtn.click();
                             }}
                         }}
                     }}
                     
                     // Method 2: Auto-click "Open Orchids" button if visible
                     const openBtn = Array.from(document.querySelectorAll('button, a')).find(b => b.innerText && b.innerText.includes('Open Orchids'));
                     if(openBtn && !window.btnClicked) {{
                        window.btnClicked = true;
                        console.log("Clicking Open Orchids button...");
                        openBtn.click();
                     }}
                }}
            }}, 500);

            // --- 2. Auto Fill Input ---
            function fillInput(selector, value) {{
                const el = document.querySelector(selector);
                if (el && el.value !== value) {{
                    el.focus();
                    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                    setter.call(el, value);
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    el.blur();
                    return true;
                }}
                return false;
            }}

            const fillInterval = setInterval(() => {{
                let filledEmail = fillInput('input[name="emailAddress"]', email);
                let filledPass = fillInput('input[name="password"]', pass);
                
                // Always try to find the button if credentials are in
                if (filledEmail || (document.querySelector('input[name="password"]')?.value === pass)) {{
                    const buttons = Array.from(document.querySelectorAll('button, a, input[type="submit"]'));
                    const btn = buttons.find(b => {{
                        const text = (b.innerText || b.value || "").trim();
                        return (text.includes('Continue') || text === 'Sign Up') 
                               && !text.toLowerCase().includes('google')
                               && !b.disabled;
                    }});
                    
                    if (btn) {{
                        console.log("Auto Clicking Continue...", btn);
                        btn.click();
                        btn.dispatchEvent(new MouseEvent('mousedown', {{ bubbles: true }}));
                        btn.dispatchEvent(new MouseEvent('mouseup', {{ bubbles: true }}));
                    }}
                }}
            }}, 500);

            // --- 3. Poll for Verification Code ---
            let codeAttempts = 0;
            const maxCodeAttempts = 30; // 30 attempts * 2 seconds = 60 seconds max
            let codeFilled = false;
            
            setInterval(() => {{
                if (codeFilled || codeAttempts >= maxCodeAttempts) return;
                
                // Enhanced OTP Detection (Single vs Multi-input)
                const allInputs = Array.from(document.querySelectorAll('input'));
                
                // 1. Look for explicit code/otp fields
                let targets = allInputs.filter(i => 
                    (i.name && (i.name.includes('code') || i.name.includes('otp') || i.name.includes('verification'))) || 
                    (i.autocomplete === 'one-time-code') ||
                    (i.getAttribute('data-testid') && i.getAttribute('data-testid').includes('code'))
                );
                
                // 2. If not found, look for likely OTP digit fields (numeric, len 1, grouped)
                if (targets.length === 0) {{
                     const digitInputs = allInputs.filter(i => 
                         (i.inputMode === 'numeric' && i.maxLength === 1) ||
                         (i.type === 'tel' && i.maxLength === 1) ||
                         (i.pattern && i.pattern === '[0-9]*' && i.maxLength === 1)
                     );
                     if (digitInputs.length >= 4) targets = digitInputs.slice(0, Math.min(digitInputs.length, 8));
                }}
                
                // 3. Also check for single input with length 4-8
                if (targets.length === 0) {{
                    const singleCodeInput = allInputs.find(i => 
                        i.maxLength >= 4 && i.maxLength <= 8 &&
                        (i.inputMode === 'numeric' || i.type === 'number' || i.type === 'tel')
                    );
                    if (singleCodeInput) targets = [singleCodeInput];
                }}

                if (targets.length > 0) {{
                    // Check if any are empty
                    const isFilled = targets.every(i => i.value && i.value.length > 0);
                    
                    if (!isFilled) {{
                        codeAttempts++;
                        if(codeAttempts % 5 === 1) log(`⏳ 等待验证码 (${{codeAttempts}}/${{maxCodeAttempts}}), 检测到 ${{targets.length}} 个输入框`);
                        
                        fetch("http://127.0.0.1:" + capturePort + "/get_code")
                            .then(res => res.text())
                            .then(code => {{
                                const trimmedCode = code.trim();
                                // Support 4-8 digit codes
                                if (trimmedCode && trimmedCode.length >= 4 && trimmedCode.length <= 8 && /^\d+$/.test(trimmedCode)) {{
                                    log("✅ 获取到验证码: " + trimmedCode);
                                    codeFilled = true;
                                    
                                    if (targets.length >= trimmedCode.length) {{
                                        // Distributed Filling for multi-input
                                        targets.forEach((input, idx) => {{
                                            if(idx < trimmedCode.length) {{
                                                input.focus();
                                                const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                                                nativeInputValueSetter.call(input, trimmedCode[idx]);
                                                input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                input.dispatchEvent(new KeyboardEvent('keyup', {{ bubbles: true, key: trimmedCode[idx], keyCode: trimmedCode.charCodeAt(idx) }}));
                                                input.dispatchEvent(new KeyboardEvent('keydown', {{ bubbles: true, key: trimmedCode[idx], keyCode: trimmedCode.charCodeAt(idx) }}));
                                            }}
                                        }});
                                        targets[trimmedCode.length - 1].blur();
                                        // Try to auto-submit after filling
                                        setTimeout(() => {{
                                            const submitBtn = document.querySelector('button[type="submit"], input[type="submit"]');
                                            if (submitBtn && !submitBtn.disabled) {{
                                                console.log("[验证码] 自动提交...");
                                                submitBtn.click();
                                            }}
                                        }}, 300);
                                    }} else {{
                                        // Single Field Filling
                                        const input = targets[0];
                                        input.focus();
                                        const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                                        nativeInputValueSetter.call(input, trimmedCode);
                                        input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                        input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                        input.blur();
                                        // Try to auto-submit
                                        setTimeout(() => {{
                                            const submitBtn = document.querySelector('button[type="submit"], input[type="submit"]');
                                            if (submitBtn && !submitBtn.disabled) {{
                                                console.log("[验证码] 自动提交...");
                                                submitBtn.click();
                                            }}
                                        }}, 300);
                                    }}
                                }} else if (codeAttempts >= maxCodeAttempts) {{
                                    console.log("[验证码] 超时，请手动输入验证码");
                                }}
                            }})
                            .catch(err => {{
                                console.log("[验证码] 请求失败:", err.message);
                            }});
                    }} else {{
                        codeFilled = true;
                        console.log("[验证码] 已填写完成");
                    }}
                }}
            }}, 2000);

            // 4. Auto-click 'Open Orchids' or Deep Link Buttons
            setInterval(() => {{
                const deepLinkBtn = Array.from(document.querySelectorAll('a, button')).find(el => 
                    (el.href && el.href.startsWith('orchids://')) || 
                    (el.innerText && el.innerText.includes('Open Orchids'))
                );
                if (deepLinkBtn) {{
                     log("🔗 发现 Deep Link 按钮，尝试点击...");
                     deepLinkBtn.click();
                     if(deepLinkBtn.href && deepLinkBtn.href.startsWith('orchids://')) {{
                         window.location.href = deepLinkBtn.href;
                     }}
                }}
            }}, 1000);

        }})();
    "#,
        email, pass, capture_port
    );

    let app_handle_nav = app.clone(); // Clone for navigation handler

    let window_ref = WebviewWindowBuilder::new(
        &app,
        "register_popup",
        WebviewUrl::External(url_str.parse().unwrap()),
    )
    .title("Orchids Auto-Reg")
    .inner_size(500.0, 700.0)
    .incognito(true)
    .initialization_script(&init_script)
    .on_navigation(move |url| {
        let u = url.as_str();
        if u.starts_with("orchids://") {
            let _ = open::that(u);
            // Auto close window when deep link is triggered
            if let Some(w) = app_handle_nav.get_webview_window("register_popup") {
                let _ = w.close();
            }
            return false;
        }
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    // Capture start time for email filtering (only get emails after this point)
    let start_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        - 60; // 1 minute buffer for clock skew

    // --- Backup Polling Thread (Doesn't rely on frontend script) ---
    let app_handle_polling = app.clone();
    let imap_host_polling = imap_host.clone();
    let imap_user_polling = imap_user.clone();
    let imap_pass_polling = imap_pass.clone();
    let polling_port = imap_port;

    std::thread::spawn(move || {
        // Wait just 4s for page load & initial auto-fill
        std::thread::sleep(std::time::Duration::from_secs(4));
        let _ = app_handle_polling.emit("automation_log", "🛡️ [后台] 极速模式：启动验证码轮询...");

        let mut attempts = 0;
        let max_attempts = 60; // 60 * 1.5s = 90 seconds timeout

        while attempts < max_attempts {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            attempts += 1;

            let code_result =
                if imap_pass_polling.is_empty() {
                    gmail_oauth::fetch_verification_code_api(&app_handle_polling, start_timestamp)
                } else {
                    imap_service::fetch_verification_code(
                        &imap_host_polling,
                        polling_port,
                        &imap_user_polling,
                        &imap_pass_polling,
                    )
                };

            if let Ok(code) = code_result {
                let _ = app_handle_polling
                    .emit("automation_log", format!("✅ [后台] 捕获验证码: {}", code));

                // JS Paste Event Simulation (Works best for React OTP inputs)
                if let Some(window) = app_handle_polling.get_webview_window("register_popup") {
                    let _ =
                        app_handle_polling.emit("automation_log", "⚡️ 模拟 Clipboard 粘贴事件...");

                    let js = format!(
                        r#"
                        (function() {{
                            const code = "{}";
                            console.log("Simulating filling for code: " + code);
                            
                            // 1. Find all potential inputs
                            const inputs = Array.from(document.querySelectorAll('input'));
                            
                            // 2. Identify likely OTP fields (Digits)
                            // Clerk often uses separate inputs for each digit
                            let digitInputs = inputs.filter(i => 
                                (i.name && (i.name.includes('code') || i.name.includes('otp'))) || 
                                (i.autocomplete === 'one-time-code') ||
                                (i.inputMode === 'numeric' && i.maxLength === 1) ||
                                (i.classList.toString().includes('otp'))
                            );

                            // Filter visible only
                            digitInputs = digitInputs.filter(i => i.offsetParent !== null);
                            
                            // If we found a sequence of inputs matching the code length, fill them
                            if (digitInputs.length >= code.length) {{
                                console.log("Found distributed inputs:", digitInputs.length);
                                code.split('').forEach((char, index) => {{
                                    if(index < digitInputs.length) {{
                                        const input = digitInputs[index];
                                        input.focus();
                                        input.click();
                                        
                                        // Simulate minimal delay typing
                                        setTimeout(() => {{
                                            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
                                            if (nativeInputValueSetter) {{
                                                nativeInputValueSetter.call(input, char);
                                            }} else {{
                                                input.value = char;
                                            }}
                                            
                                            input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                            input.dispatchEvent(new KeyboardEvent('keydown', {{ bubbles: true, key: char, code: 'Digit' + char, charCode: char.charCodeAt(0), keyCode: char.charCodeAt(0), which: char.charCodeAt(0) }}));
                                            input.dispatchEvent(new KeyboardEvent('keypress', {{ bubbles: true, key: char, code: 'Digit' + char, charCode: char.charCodeAt(0), keyCode: char.charCodeAt(0), which: char.charCodeAt(0) }}));
                                            input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                            input.dispatchEvent(new KeyboardEvent('keyup', {{ bubbles: true, key: char, code: 'Digit' + char, charCode: char.charCodeAt(0), keyCode: char.charCodeAt(0), which: char.charCodeAt(0) }}));
                                        }}, index * 50);
                                    }}
                                }});
                                
                                // Auto-Submit if button exists
                                setTimeout(() => {{
                                     const submitBtn = document.querySelector('button[type="submit"], input[type="submit"]');
                                     if(submitBtn) submitBtn.click();
                                }}, code.length * 60 + 500);

                            }} else {{
                                // Fallback: Try filling the first found input field with the whole code (Paste method)
                                console.log("Fallback to single input paste...");
                                let target = inputs.find(i => i.name === 'code' || i.name === 'otp' || i.type === 'text');
                                if (target) {{
                                    target.focus();
                                    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
                                    nativeInputValueSetter.call(target, code);
                                    target.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                    target.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                }}
                            }}
                        }})();
                    "#,
                        code
                    );
                    let _ = window.eval(&js);
                }
                break;
            }
        }
    });

    // Start Server
    let app_handle_server = app.clone();
    let imap_host_server = imap_host.clone();
    let imap_user_server = imap_user.clone();
    let imap_pass_server = imap_pass.clone();

    // Capture email/pass for saving after success
    let reg_email = email.clone();
    let reg_pass = pass.clone();

    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(listener) = std::net::TcpListener::bind(format!("127.0.0.1:{}", capture_port)) {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut buffer = [0; 4096];
                    use std::io::{Read, Write};
                    if let Ok(n) = stream.read(&mut buffer) {
                        if n > 0 {
                            let req_str = String::from_utf8_lossy(&buffer[..n]);

                            // Headers for CORS
                            let header =
                                "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\n\r\n";

                            // Route: GET /get_code
                            if req_str.contains("GET /get_code") {
                                let mut code = String::new();

                                // First, check for manually entered code
                                if let Some(manual_code) =
                                    MANUAL_VERIFICATION_CODE.lock().unwrap().take()
                                {
                                    code = manual_code.clone();
                                    let _ = app_handle_server.emit(
                                        "automation_log",
                                        format!("✅ 使用手动输入的验证码: {}", code),
                                    );
                                } else {
                                    let _ = app_handle_server.emit(
                                        "automation_log",
                                        format!(
                                            "📧 正在获取验证码 (查找 {} 之后的邮件)...",
                                            start_timestamp
                                        ),
                                    );

                                    // Determine Strategy
                                    if imap_pass_server.is_empty()
                                    {
                                        // OAuth Strategy
                                        let _ = app_handle_server.emit(
                                            "automation_log",
                                            "🔑 使用 Google OAuth 获取邮件...",
                                        );
                                        match gmail_oauth::fetch_verification_code_api(
                                            &app_handle_server,
                                            start_timestamp,
                                        ) {
                                            Ok(c) => {
                                                code = c.clone();
                                                let _ = app_handle_server.emit(
                                                    "automation_log",
                                                    format!("✅ 验证码已获取: {}", c),
                                                );
                                            }
                                            Err(e) => {
                                                let _ = app_handle_server.emit(
                                                    "automation_log",
                                                    format!(
                                                        "⚠️ 自动获取失败: {} - 请手动输入验证码",
                                                        e
                                                    ),
                                                );
                                                // Emit event for UI to show manual input
                                                let _ = app_handle_server
                                                    .emit("request_manual_code", true);
                                            }
                                        }
                                    } else {
                                        // Standard Strategy
                                        let _ = app_handle_server
                                            .emit("automation_log", "📬 使用 IMAP 获取邮件...");
                                        match imap_service::fetch_verification_code(
                                            &imap_host_server,
                                            imap_port,
                                            &imap_user_server,
                                            &imap_pass_server,
                                        ) {
                                            Ok(c) => {
                                                code = c.clone();
                                                let _ = app_handle_server.emit(
                                                    "automation_log",
                                                    format!("✅ 验证码已获取: {}", c),
                                                );
                                            }
                                            Err(e) => {
                                                let _ = app_handle_server.emit(
                                                    "automation_log",
                                                    format!(
                                                        "⚠️ 自动获取失败: {} - 请手动输入验证码",
                                                        e
                                                    ),
                                                );
                                                // Emit event for UI to show manual input
                                                let _ = app_handle_server
                                                    .emit("request_manual_code", true);
                                            }
                                        }
                                    }
                                }
                                let _ = stream.write_all(format!("{}{}", header, code).as_bytes());
                                continue;
                            }

                            // Route: POST /log (For Frontend Script Debugging)
                            if req_str.contains("POST /log") {
                                // Extract body
                                if let Some(body_start) = req_str.find("\r\n\r\n") {
                                    let body = &req_str[body_start + 4..];
                                    let clean_body = body.trim_matches(char::from(0)); // Remove null bytes
                                    let _ = app_handle_server.emit(
                                        "automation_log",
                                        format!("🖥️ [WebView]: {}", clean_body),
                                    );
                                }
                                let _ = stream.write_all(format!("{}OK", header).as_bytes());
                                continue;
                            }

                            // Route: Capture / Deeplink
                            if let Some(pos) = req_str.find("cookie=") {
                                let rest = &req_str[pos + 7..];
                                if let Some(end) = rest.find(' ') {
                                    let encoded = &rest[..end];
                                    if let Ok(cookie_str) = urlencoding::decode(encoded) {
                                        let _ = app_handle_server
                                            .emit("automation_log", "🍪 Session Cookie 已捕获");
                                        let _ = app_handle_server
                                            .emit("session_captured", cookie_str.to_string());

                                        // --- AUTO SAVE LOGIC ---
                                        // 1. Extract __session
                                        let mut session_token = String::new();
                                        for part in cookie_str.split("; ") {
                                            if part.starts_with("__session=") {
                                                session_token = part.replace("__session=", "");
                                                break;
                                            }
                                        }

                                        if !session_token.is_empty() {
                                            let _ = app_handle_server.emit("automation_log", "🔄 解析 ID & 获取资料...");
                                            
                                            // 2. Decode JWT to get User ID (sub)
                                            // JWT Format: Header.Payload.Signature
                                            let parts: Vec<&str> = session_token.split('.').collect();
                                            if parts.len() >= 2 {
                                                use base64::{engine::general_purpose, Engine as _};
                                                // Pad if necessary
                                                let payload_segment = parts[1];
                                                // Handle URL Safe Base64
                                                let mut padded = payload_segment.to_string();
                                                while padded.len() % 4 != 0 {
                                                    padded.push('=');
                                                }
                                                // Replace -_ with +/
                                                let normalized = padded.replace("-", "+").replace("_", "/");

                                                if let Ok(decoded) = general_purpose::STANDARD.decode(normalized) {
                                                    if let Ok(json_str) = String::from_utf8(decoded) {
                                                        if let Ok(claims) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                                            if let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) {
                                                                let user_id = sub.to_string();
                                                                let _ = app_handle_server.emit("automation_log", format!("🆔 User ID: {}", user_id));
                                                                
                                                                // 3. Fetch Profile
                                                                let client = reqwest::blocking::Client::new();
                                                                match client.get(format!("https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/{}", user_id))
                                                                    .header("Authorization", format!("Bearer {}", session_token))
                                                                    .send() 
                                                                {
                                                                    Ok(resp) => {
                                                                        if resp.status().is_success() {
                                                                            if let Ok(profile) = resp.json::<serde_json::Value>() {
                                                                                // 4. Construct Account
                                                                                let credits = profile["credits"].as_i64().unwrap_or(0);
                                                                                let plan = profile["plan"].as_str().unwrap_or("Free").to_string();
                                                                                let email = profile["email"].as_str().unwrap_or(&reg_email).to_string();
                                                                                let image = profile["imageUrl"].as_str().map(|s| s.to_string());
                                                                                let full_name = profile["firstName"].as_str().unwrap_or("User").to_string();

                                                                                // Reconstruct Cookies Vec
                                                                                let mut cookie_vec = Vec::new();
                                                                                cookie_vec.push(models::CookieData {
                                                                                    name: "__session".to_string(),
                                                                                    value: session_token.clone(),
                                                                                    domain: ".orchids.app".to_string(),
                                                                                    path: "/".to_string(),
                                                                                    secure: true,
                                                                                    http_only: false,
                                                                                    expiration_date: None,
                                                                                    samesite: None,
                                                                                    priority: None,
                                                                                }); 

                                                                                let new_account = Account {
                                                                                    id: user_id.clone(),
                                                                                    display_name: full_name.clone(),
                                                                                    email: Some(email.clone()),
                                                                                    password: Some(reg_pass.clone()), // Save password for auto-login
                                                                                    avatar_url: image.clone(),
                                                                                    last_active_at: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64),
                                                                                    user_info: Some(models::UserInfo {
                                                                                        id: None,
                                                                                        user_id: user_id.clone(),
                                                                                        full_name: Some(full_name),
                                                                                        email: Some(email.clone()),
                                                                                        image_url: image,
                                                                                        plan: Some(plan),
                                                                                        credits: Some(credits),
                                                                                    }),
                                                                                    cookies: cookie_vec,
                                                                                };

                                                                                // 5. Save to State
                                                                                let state = app_handle_server.state::<AccountManagerState>();
                                                                                if let Ok(_) = state.add_account(new_account) {
                                                                                    let _ = app_handle_server.emit("automation_log", "💾 账号已保存到数据库");
                                                                                    let _ = app_handle_server.emit("register_success", serde_json::json!({
                                                                                        "email": email,
                                                                                        "password": reg_pass
                                                                                    }));
                                                                                }
                                                                            }
                                                                        } else {
                                                                             let _ = app_handle_server.emit("automation_log", format!("❌ Profile API Failed: {}", resp.status()));
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                         let _ = app_handle_server.emit("automation_log", format!("❌ Network Error: {}", e));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Close window after capture
                                        let app_done = app_handle_server.clone();
                                        std::thread::spawn(move || {
                                            std::thread::sleep(std::time::Duration::from_secs(2));
                                            if let Some(w) =
                                                app_done.get_webview_window("register_popup")
                                            {
                                                let _ = w.close();
                                            }
                                        });
                                    }
                                }
                            } else if let Some(pos) = req_str.find("deeplink=") {
                                let rest = &req_str[pos + 9..];
                                if let Some(end) = rest.find(' ') {
                                    let encoded = &rest[..end];
                                    if let Ok(link) = urlencoding::decode(encoded) {
                                        let _ = app_handle_server
                                            .emit("automation_log", "🔗 正在打开 Orchids App...");
                                        
                                        // Just open the app, assume cookie capture happened or will happen via listener
                                        let _ = open::that(link.to_string());
                                        let _ = app_handle_server
                                            .emit("automation_log", "✅ 注册流程结束");

                                        // Close window logic
                                        let app_done = app_handle_server.clone();
                                        std::thread::spawn(move || {
                                            std::thread::sleep(std::time::Duration::from_secs(4));
                                            if let Some(w) =
                                                app_done.get_webview_window("register_popup")
                                            {
                                                let _ = w.close();
                                            }
                                        });
                                    }
                                }
                            }
                            let _ = stream.write_all(header.as_bytes());
                        }
                    }
                }
            }
        }
    });

    Ok("WebView Launched".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
        let state = AccountManagerState::new(app.handle());
        app.manage(state);
        app.manage(Arc::new(CaptureService::new()));
        app.manage(Arc::new(ProxyService::new(app.handle().clone())));
        Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        ensure_patched,
        get_accounts,
        get_active_id,
        save_account,
        delete_account,
        set_active_account,
        start_listener,
        stop_listener,
        import_current_session,
        reset_machine_id,
        start_proxy,
        stop_proxy,
        check_imap_code,
        gmail_oauth_start,
        gmail_oauth_status,
        check_imap_code_oauth,
        set_manual_code,
        get_manual_code,
        clear_manual_code,
        refresh_active_account,
        clerk_action_register_webview,
        // Keeping checkpoint managers for safety
        checkpoint_manager::get_projects,
        checkpoint_manager::get_checkpoints,
        checkpoint_manager::get_file_content_base64,
        checkpoint_manager::rollback_checkpoint
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
