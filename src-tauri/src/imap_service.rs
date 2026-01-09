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

    // Fetch last 10 messages
    let start = if last_seq > 9 { last_seq - 9 } else { 1 };
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
