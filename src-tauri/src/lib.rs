mod capture_service;
mod clerk_utils;
mod cookie_reader;
mod gmail_oauth;
mod imap_service;
mod injection;
mod models;
mod proxy_service;
mod store;

use capture_service::CaptureService;
use injection::InjectionManager;
use models::Account;
use proxy_service::ProxyService;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use store::AccountManagerState;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn reset_machine_id(app: AppHandle) -> Result<String, String> {
    injection::reset_machine_id(&app)
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("你好, {}! 这是来自 Rust 的问候!", name)
}

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
fn import_current_session(
    app: AppHandle,
    state: State<'_, AccountManagerState>,
) -> Result<Account, String> {
    let path_buf = capture_service::get_orchids_cookie_path();
    let path = path_buf.as_path();

    // Also try the webview partition if main one fails or is empty?
    // For now stick to main.

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

#[tauri::command]
fn set_active_account(
    app: AppHandle,
    state: State<'_, AccountManagerState>,
    capture: State<'_, Arc<CaptureService>>,
    id: String,
) -> Result<(), String> {
    // 1. Stop monitoring to release any read locks
    capture.stop();
    println!("Capture service stopped for account switch.");

    state.set_active(id.clone())?;

    // Find the account data
    let accounts = state.get_accounts()?;
    if let Some(account) = accounts.iter().find(|a| a.id == id) {
        injection::trigger_restore(&app, account)?;
    }

    Ok(())
}

#[tauri::command]
fn inject_orchids() -> Result<String, String> {
    let manager = InjectionManager::new();
    manager.inject()
}

#[tauri::command]
fn uninject_orchids() -> Result<String, String> {
    let manager = InjectionManager::new();
    manager.uninject()
}

#[tauri::command]
fn check_captured_account(
    app: AppHandle,
    state: State<'_, AccountManagerState>,
) -> Result<Option<Account>, String> {
    injection::check_captured_data(&app, &state)
}

#[tauri::command]
fn logout_and_restart(app: AppHandle) -> Result<(), String> {
    injection::clear_cookies_and_restart(&app)
}

#[tauri::command]
async fn start_proxy(state: State<'_, Arc<ProxyService>>, port: u16) -> Result<String, String> {
    state.start(port).await
}

#[tauri::command]
async fn stop_proxy(state: State<'_, Arc<ProxyService>>) -> Result<String, String> {
    state.stop().await
}

#[tauri::command]
fn get_active_id(state: State<'_, AccountManagerState>) -> Result<Option<String>, String> {
    state.get_active_id()
}

#[tauri::command]
async fn refresh_active_account(state: State<'_, AccountManagerState>) -> Result<Account, String> {
    // 1. Prepare Data (Fast DB Read)
    let active_id = state.get_active_id()?.ok_or("无活跃账号")?;
    let accounts = state.get_accounts()?;
    let account = accounts
        .iter()
        .find(|a| a.id == active_id)
        .ok_or("账号未找到")?;

    let token = account
        .cookies
        .iter()
        .find(|c| c.name == "__session")
        .map(|c| c.value.clone())
        .ok_or("无会话令牌")?;
    let user_id = account.id.clone();

    // 2. Heavy Lift (HTTP Request in Thread)
    let new_credits = tauri::async_runtime::spawn_blocking(move || {
        capture_service::fetch_fresh_credits(&user_id, &token)
    })
    .await
    .map_err(|e| e.to_string())??;

    // 3. Update State (Fast DB Write)
    let mut updated_account = account.clone();
    if let Some(ref mut info) = updated_account.user_info {
        info.credits = Some(new_credits);
    }

    state.add_account(updated_account.clone())?;

    Ok(updated_account)
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
async fn clerk_action_prepare_verification(sign_up_id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        clerk_utils::clerk_prepare_verification(&sign_up_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clerk_action_verify(
    state: State<'_, AccountManagerState>,
    sign_up_id: String,
    code: String,
) -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        clerk_utils::clerk_attempt_verification(&sign_up_id, &code)
    })
    .await
    .map_err(|e| e.to_string())??;

    if result.starts_with('{') {
        return Ok(result);
    }

    let token = result;
    let cookie = models::CookieData {
        name: "__session".to_string(),
        value: token.clone(),
        domain: ".orchids.app".to_string(),
        path: "/".to_string(),
        secure: true,
        http_only: false,
        expiration_date: None,
        samesite: None,
        priority: None,
    };

    let account = tauri::async_runtime::spawn_blocking(move || {
        capture_service::create_account_from_token(&token, vec![cookie])
    })
    .await
    .map_err(|e| e.to_string())??;

    state.add_account(account.clone())?;
    Ok("验证成功。账号已保存。".to_string())
}

#[tauri::command]
async fn clerk_action_login(
    state: State<'_, AccountManagerState>,
    email: String,
    pass: String,
) -> Result<String, String> {
    let token =
        tauri::async_runtime::spawn_blocking(move || clerk_utils::clerk_login(&email, &pass))
            .await
            .map_err(|e| e.to_string())??;

    let cookie = models::CookieData {
        name: "__session".to_string(),
        value: token.clone(),
        domain: ".orchids.app".to_string(),
        path: "/".to_string(),
        secure: true,
        http_only: false,
        expiration_date: None,
        samesite: None,
        priority: None,
    };

    let account = tauri::async_runtime::spawn_blocking(move || {
        capture_service::create_account_from_token(&token, vec![cookie])
    })
    .await
    .map_err(|e| e.to_string())??;

    state.add_account(account.clone())?;
    Ok("登录并保存成功".to_string())
}

#[tauri::command]
async fn clerk_action_register(
    email: String,
    pass: String,
    token: String,
) -> Result<String, String> {
    println!("LIB: clerk_action_register called. Token: {}", token);
    let captcha_opt = if token.is_empty() { None } else { Some(token) };
    tauri::async_runtime::spawn_blocking(move || {
        clerk_utils::clerk_register(&email, &pass, captcha_opt)
    })
    .await
    .map_err(|e| e.to_string())?
}

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
    // 1. Reset Machine ID & Hard Restart (Kills app, clears cookies, keeps LocalStorage, restarts)
    println!("Resetting Device Identity for Registration...");

    // Log Current ID
    let updater_path = capture_service::get_orchids_data_dir().join(".updaterId");
    if let Ok(current_id) = std::fs::read_to_string(&updater_path) {
        println!("Current Machine ID: {}", current_id.trim());
    } else {
        println!("Current Machine ID: None (File not found)");
    }

    let _ = app.emit("imap_log", "正在重置设备身份 (Machine ID)...".to_string());
    let reset_msg = injection::reset_machine_id(&app)?;
    let _ = app.emit("imap_log", format!("重置完成: {}", reset_msg));
    println!("{}", reset_msg);

    // Ultra-Fast launch
    std::thread::sleep(std::time::Duration::from_millis(800));

    // 2. Click "Log in" / "Sign Up" via AppleScript to prepare for Deep Link
    println!("Clicking Log In...");
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(
            r#"
        tell application "System Events"
            if exists process "Orchids" then
                tell process "Orchids"
                    set frontmost to true
                    try
                        if exists button "Log in" of window 1 then
                            click button "Log in" of window 1
                        else if exists button "Sign in" of window 1 then
                            click button "Sign in" of window 1
                        else
                            key code 36 -- Enter
                        end if
                    on error err
                        log "AppleScript Error: " & err
                    end try
                end tell
            end if
        end tell
        "#,
        )
        .output();

    // Minimizing wait
    std::thread::sleep(std::time::Duration::from_millis(200));

    let url_str = "https://accounts.orchids.app/sign-up";

    // JS to auto-fill form (Updated with polling)
    let init_script = format!(
        r#"
        (function() {{
            console.log("Auto-Fill Script Injected");
            
            const capturePort = {};

            // Session Watcher (Ping local server with cookie)
            setInterval(() => {{
                if (document.cookie.includes('__session')) {{
                    new Image().src = 'http://127.0.0.1:' + capturePort + '/?cookie=' + encodeURIComponent(document.cookie);

                    // Redirect to Auth after successful login/registration
                    if (!window.hasRedirected) {{
                        window.hasRedirected = true;
                        console.log("Orchis: Session found, redirecting to auth...");
                        setTimeout(() => {{
                            window.location.href = "https://orchids.app/desktop/auth";
                        }}, 10);
                    }}
                }}
            }}, 2000);

            // Deep Link Watcher
            setInterval(() => {{
                if (window.location.href.includes("desktop/auth")) {{
                     // 1. Text Search Strategy (Regex scrape)
                     const html = document.body.innerHTML;
                     const match = html.match(/orchids:\/\/[^"'\s<>]+/);
                     if(match && match[0]) {{
                         let link = match[0];
                         link = link.replace(/&amp;/g, '&'); // Fix entities
                         
                         if(!window.lastSentLink || window.lastSentLink !== link) {{
                             window.lastSentLink = link;
                             console.log("Found Deep Link via Regex: " + link);
                             new Image().src = 'http://127.0.0.1:' + capturePort + '/?deeplink=' + encodeURIComponent(link);
                         }}
                     }}
                     
                     // 2. Auto Click Button (Triggers navigation, which we catch in Rust)
                     const btns = Array.from(document.querySelectorAll('button, a'));
                     const openBtn = btns.find(b => b.innerText && b.innerText.includes('Open Orchids'));
                     if(openBtn && !window.btnClicked) {{
                        window.btnClicked = true;
                        console.log("Auto-Clicking Open Orchids...");
                        openBtn.click();
                     }}
                }}
            }}, 1000);

            let attempts = 0;
            const maxAttempts = 50; 
            
            const interval = setInterval(() => {{
                attempts++;
                const emailInput = document.querySelector('input[name="emailAddress"]') || document.querySelector('input[type="email"]');
                const passInput = document.querySelector('input[name="password"]') || document.querySelector('input[type="password"]');
                
                if (emailInput && passInput) {{
                    function setNativeValue(element, value) {{
                        const valueSetter = Object.getOwnPropertyDescriptor(element, 'value').set;
                        const prototype = Object.getPrototypeOf(element);
                        const prototypeValueSetter = Object.getOwnPropertyDescriptor(prototype, 'value').set;
                        if (valueSetter && valueSetter !== prototypeValueSetter) {{
                            prototypeValueSetter.call(element, value);
                        }} else {{
                            valueSetter.call(element, value);
                        }}
                        element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    }}

                    setNativeValue(emailInput, "{}");
                    setNativeValue(passInput, "{}");
                    
                    clearInterval(interval);
                    
                    setTimeout(() => {{
                        let submitBtn = Array.from(document.querySelectorAll('button[type="submit"]'))
                                       .find(b => !b.innerText.toLowerCase().includes("google"));
                                       
                        if (!submitBtn) {{
                            const buttons = Array.from(document.querySelectorAll('button'));
                            submitBtn = buttons.find(b => {{
                                const t = b.innerText.trim();
                                return (t === 'Continue' || t === 'Sign Up' || t === 'Create account') && !t.toLowerCase().includes("google");
                            }});
                        }}
                        
                         if(submitBtn) {{
                             submitBtn.click();
                         }}
                     }}, 50);
                }} else if (attempts >= maxAttempts) {{
                     clearInterval(interval);
                }}
            }}, 200);
        }})();
        "#,
        capture_port, email, pass
    );

    let window_ref = WebviewWindowBuilder::new(
        &app,
        "register_popup",
        WebviewUrl::External(url_str.parse().unwrap()),
    )
    .title("Orchids Registration - Auto Fill & IMAP")
    .inner_size(500.0, 700.0)
    .incognito(true)
    .initialization_script(&init_script)
    .on_navigation(|url| {
        let url_str = url.as_str();
        if url_str.starts_with("orchids://") {
            println!("Intercepted Deep Link via Navigation: {}", url_str);
            let _ = open::that(url_str);
            return false;
        }
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    // Start background IMAP polling task if credentials provided
    if !imap_host.is_empty() {
        let w_handle = window_ref.clone();
        let app_handle = app.clone(); // Clone app for emitting events
        let h = imap_host.clone();
        let p = imap_port;
        let u = imap_user.clone();
        let pw = imap_pass.clone();

        let start_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

    // Start Session Capture Server
    let app_handle_server = app_handle.clone();
    let port = capture_port;
    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(listener) = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
            for stream in listener.incoming() {
                    if let Ok(mut stream) = stream {
                        let mut buffer = [0; 4096];
                        use std::io::{Read, Write};
                        if let Ok(n) = stream.read(&mut buffer) {
                            if n > 0 {
                                let req = String::from_utf8_lossy(&buffer[..n]);
                                if let Some(pos) = req.find("cookie=") {
                                    let rest = &req[pos + 7..];
                                    // Find end of URL (space)
                                    if let Some(end) = rest.find(' ') {
                                        let encoded = &rest[..end];
                                        if let Ok(cookie) = urlencoding::decode(encoded) {
                                            println!("会话 Cookie 已捕获！");
                                            let _ = app_handle_server
                                                .emit("session_captured", cookie.to_string());
                                        }
                                    }
                                } else if let Some(pos) = req.find("deeplink=") {
                                    let rest = &req[pos + 9..];
                                    if let Some(end) = rest.find(' ') {
                                        let encoded = &rest[..end];
                                        if let Ok(link) = urlencoding::decode(encoded) {
                                            println!("捕获深度链接: {}", link);
                                            let _ = open::that(link.to_string());

                                            // Close window after 2s
                                            let app_close = app_handle_server.clone();
                                            std::thread::spawn(move || {
                                                std::thread::sleep(std::time::Duration::from_secs(
                                                    2,
                                                ));
                                                if let Some(w) =
                                                    app_close.get_webview_window("register_popup")
                                                {
                                                    println!("Closing registration window...");
                                                    let _ = w.close();
                                                }
                                            });
                                        }
                                    }
                                }
                                // Close response
                                let response = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 0\r\n\r\n";
                                let _ = stream.write_all(response.as_bytes());
                            }
                        }
                    }
                }
            }
        });

        tauri::async_runtime::spawn(async move {
            let _ = app_handle.emit("imap_log", format!("IMAP 轮询已开始：{}", u));
            println!("IMAP 轮询已开始：{}", u);

            for i in 0..80 {
                // 500ms interval for high frequency polling
                if i > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }

                let _ = app_handle.emit("imap_log", format!("正在检查收件箱 ({}/80)...", i + 1));

                // Check if OAuth is available
                let use_oauth = gmail_oauth::is_authenticated() && h == "imap.gmail.com";

                let code_res = if use_oauth {
                    let app_clone = app_handle.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        gmail_oauth::fetch_verification_code_api(&app_clone, start_ts)
                    })
                    .await
                } else {
                    let h_c = h.clone();
                    let u_c = u.clone();
                    let pw_c = pw.clone();
                    // Note: IMAP fetch doesn't support timestamp filtering yet, assuming it fetches latest
                    tauri::async_runtime::spawn_blocking(move || {
                        imap_service::fetch_verification_code(&h_c, p, &u_c, &pw_c)
                    })
                    .await
                };

                match code_res {
                    Ok(Ok(c)) => {
                        println!("找到候选验证码: {}", c);

                        let _ =
                            app_handle.emit("imap_log", format!("找到验证码: {}. 正在验证...", c));

                        // Copy to clipboard
                        if let Ok(mut child) = std::process::Command::new("pbcopy")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                        {
                            if let Some(mut stdin) = child.stdin.take() {
                                use std::io::Write;
                                let _ = stdin.write_all(c.as_bytes());
                            }
                        }

                        let _ =
                            app_handle.emit("imap_log", format!("验证码已复制。正在模拟粘贴...",));

                        // Instant focus
                        std::thread::sleep(std::time::Duration::from_millis(50));

                        // System-level Cmd+V
                        let _ = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg("tell application \"System Events\" to keystroke \"v\" using command down")
                            .output();

                        let _ = app_handle.emit("imap_log", format!("验证码已验证: {}", c));

                        let inject_js = format!(
                            r#"
                            (function() {{
                                    const code = '{}';
                                    console.log("Orchis 验证码: " + code);
                                    
                                    function tryFill() {{
                                        const otpInput = document.querySelector('input[data-input-otp="true"]') || document.querySelector('input[name="code"]') || document.querySelector('.cl-otpCodeField input');
                                        if(!otpInput) return false;
                                        
                                        // If already filled, stop
                                        if(otpInput.value === code) return true;
                                        
                                        console.log("Orchis: 尝试填充...");
                                        otpInput.focus();
                                        otpInput.click();
                                        
                                        // 1. Native Setter
                                        const nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                                        nativeSetter.call(otpInput, code);
                                        
                                        // 2. Events covering all bases
                                        otpInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                        otpInput.dispatchEvent(new InputEvent('input', {{ bubbles: true, inputType: 'insertFromPaste', data: code }}));
                                        otpInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                        
                                        return otpInput.value === code;
                                    }}
                                    
                                    // Try immediately
                                    tryFill();
                                    
                                    // And retry periodically
                                    const interval = setInterval(() => {{
                                        if(tryFill()) {{
                                            clearInterval(interval);
                                            console.log("Orchis: 填充成功");
                                            setTimeout(() => {{
                                                const btns = Array.from(document.querySelectorAll('button'));
                                                const nextBtn = btns.find(b => b.innerText.includes('Next') || b.innerText.includes('Continue') || b.innerText.includes('Verify'));
                                                if(nextBtn) nextBtn.click();
                                            }}, 50);
                                        }}
                                    }}, 1000);
                            }})();
                            "#,
                            c
                        );

                        // Execute JS directly in the Webview
                        if let Some(w) = app_handle.get_webview_window("register_popup") {
                            println!("Injecting JS into register_popup...");
                            if let Err(e) = w.eval(&inject_js) {
                                println!("JS Injection Error: {}", e);
                            }
                        } else {
                            println!("Error: Register popup window not found!");
                        }

                        // Stop loop
                        break;
                    }
                    Ok(Err(e)) => {
                        // Log error but continue polling (maybe email hasn't arrived yet)
                        // But if error is "Login failed", we should stop?
                        // For now just log.
                        let _ = app_handle.emit("imap_log", format!("IMAP Check: {}", e));
                    }
                    Err(e) => {
                        let _ = app_handle.emit("imap_log", format!("Task Error: {}", e));
                    }
                }
            }
        });
    }

    Ok("WebView 已打开，带有自动填充 + 异步 IMAP 轮询".to_string())
}

#[tauri::command]
async fn gmail_oauth_start(app: AppHandle) -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(|| gmail_oauth::start_oauth_flow())
        .await
        .map_err(|e| e.to_string())??;

    // Save tokens to disk
    gmail_oauth::save_tokens_to_disk(&app, &result);

    Ok(format!(
        "OAuth 成功！获取到访问令牌 (过期时间 {:?}秒)",
        result.expires_in
    ))
}

#[tauri::command]
fn gmail_oauth_status(app: AppHandle) -> Result<String, String> {
    // Try loading from disk if not authenticated
    if !gmail_oauth::is_authenticated() {
        gmail_oauth::try_load_from_disk(&app);
    }

    if gmail_oauth::is_authenticated() {
        Ok("已认证".to_string())
    } else {
        Ok("未认证".to_string())
    }
}

#[tauri::command]
async fn check_imap_code_oauth(app: AppHandle, _email: String) -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        // For testing, fetch any recent code (last 24h?)
        // Or just 0
        gmail_oauth::fetch_verification_code_api(&app, 0)
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AccountManagerState::new(app.handle());
            app.manage(state);

            // Initialize Capture Service
            let capture_service = Arc::new(CaptureService::new());
            app.manage(capture_service);

            // Initialize Proxy Service
            let proxy_service = Arc::new(ProxyService::new(app.handle().clone()));
            app.manage(proxy_service);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_accounts,
            get_active_id,
            save_account,
            delete_account,
            set_active_account,
            inject_orchids,
            uninject_orchids,
            check_captured_account,
            start_listener,
            stop_listener,
            import_current_session,
            start_proxy,
            stop_proxy,
            logout_and_restart,
            refresh_active_account,
            clerk_action_login,
            clerk_action_register,
            check_imap_code,
            clerk_action_prepare_verification,
            clerk_action_verify,
            clerk_action_register_webview,
            gmail_oauth_start,
            gmail_oauth_status,
            check_imap_code_oauth,
            reset_machine_id
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
