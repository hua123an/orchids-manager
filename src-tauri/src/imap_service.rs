use native_tls::TlsConnector;
use regex::Regex;

pub fn fetch_verification_code(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<String, String> {
    let tls = TlsConnector::builder().build().map_err(|e| e.to_string())?;

    // Connect (IMAP over SSL/TLS)
    let client = imap::connect((host, port), host, &tls).map_err(|e| format!("连接失败: {}", e))?;

    // Login
    let mut session = client
        .login(user, pass)
        .map_err(|e| format!("登录失败: {}", e.0))?;

    // Select INBOX and get count
    let mailbox = session
        .select("INBOX")
        .map_err(|e| format!("选择收件箱失败: {}", e))?;
    let last_seq = mailbox.exists;

    if last_seq == 0 {
        return Err("收件箱为空".to_string());
    }

    // Fetch last 3 messages
    let start = if last_seq > 2 { last_seq - 2 } else { 1 };
    let fetch_range = format!("{}:{}", start, last_seq);

    let messages = session
        .fetch(fetch_range, "RFC822.TEXT") // Fetch Body text only
        .map_err(|e| format!("获取邮件失败: {}", e))?;

    let re = Regex::new(r"\b\d{6}\b").map_err(|e| e.to_string())?;

    // Iterate in reverse (latest first)
    for msg in messages.iter().rev() {
        let body_bytes = msg.text().or(msg.body()).unwrap_or(&[]);
        let text = String::from_utf8_lossy(body_bytes);

        // Find all matches and filter
        for mat in re.find_iter(&text) {
            let code = mat.as_str();
            // Check preceding character to avoid Hex colors (#123456)
            let start = mat.start();
            if start > 0 {
                let prefix = &text[start - 1..start];
                if prefix == "#" {
                    continue;
                }
            }
            return Ok(code.to_string());
        }
    }

    Err("近期邮件中未找到 6 位验证码".to_string())
}

/// Fetch verification code using OAuth2 (for Gmail)
pub fn fetch_verification_code_oauth(
    user_email: &str,
    access_token: &str,
) -> Result<String, String> {
    use base64::Engine;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let tls = TlsConnector::builder().build().map_err(|e| e.to_string())?;

    // Connect to Gmail IMAP directly (relies on system proxy/TUN)
    let stream =
        TcpStream::connect("imap.gmail.com:993").map_err(|e| format!("TCP 连接失败: {}", e))?;
    let mut tls_stream = tls
        .connect("imap.gmail.com", stream)
        .map_err(|e| format!("TLS 连接失败: {}", e))?;

    // Read greeting
    let mut buf = [0u8; 4096];
    tls_stream.read(&mut buf).ok();

    // Build XOAUTH2 string: "user=<email>\x01auth=Bearer <token>\x01\x01"
    let auth_string = format!(
        "user={}\x01auth=Bearer {}\x01\x01",
        user_email, access_token
    );
    let auth_base64 = base64::engine::general_purpose::STANDARD.encode(&auth_string);

    // Send AUTHENTICATE command
    let cmd = format!("A1 AUTHENTICATE XOAUTH2 {}\r\n", auth_base64);
    tls_stream
        .write_all(cmd.as_bytes())
        .map_err(|e| e.to_string())?;

    // Read response
    let n = tls_stream.read(&mut buf).map_err(|e| e.to_string())?;
    let response = String::from_utf8_lossy(&buf[..n]);

    if !response.contains("A1 OK") {
        return Err(format!("XOAUTH2 认证失败: {}", response));
    }

    // Select INBOX
    let cmd = "A2 SELECT INBOX\r\n";
    tls_stream
        .write_all(cmd.as_bytes())
        .map_err(|e| e.to_string())?;
    let n = tls_stream.read(&mut buf).map_err(|e| e.to_string())?;
    let response = String::from_utf8_lossy(&buf[..n]);

    // Parse EXISTS count
    let exists_re = Regex::new(r"\* (\d+) EXISTS").unwrap();
    let exists = exists_re
        .captures(&response)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
        .unwrap_or(0);

    if exists == 0 {
        return Err("收件箱为空".to_string());
    }

    // Fetch last 3 messages
    let start = if exists > 2 { exists - 2 } else { 1 };
    let cmd = format!("A3 FETCH {}:{} BODY[TEXT]\r\n", start, exists);
    tls_stream
        .write_all(cmd.as_bytes())
        .map_err(|e| e.to_string())?;

    // Read response (may need multiple reads for large emails)
    let mut full_response = String::new();
    loop {
        let n = tls_stream.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        full_response.push_str(&String::from_utf8_lossy(&buf[..n]));
        if full_response.contains("A3 OK")
            || full_response.contains("A3 NO")
            || full_response.contains("A3 BAD")
        {
            break;
        }
    }

    // Find 6-digit code
    let re = Regex::new(r"\b\d{6}\b").unwrap();
    for mat in re.find_iter(&full_response) {
        let code = mat.as_str();
        let start_idx = mat.start();
        if start_idx > 0 {
            let prefix = &full_response[start_idx - 1..start_idx];
            if prefix == "#" {
                continue;
            }
        }

        // Logout
        tls_stream.write_all(b"A4 LOGOUT\r\n").ok();

        return Ok(code.to_string());
    }

    // Logout
    tls_stream.write_all(b"A4 LOGOUT\r\n").ok();

    Err("近期邮件中未找到 6 位验证码".to_string())
}
