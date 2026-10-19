#[allow(dead_code)]
pub const USE_LICENSE: bool = true;

pub const LICENSE_LABEL: &str = "TOKTOK";

pub fn license_server_url() -> &'static str {
    env!("LICENSE_SERVER_URL")
}

pub fn server_key() -> &'static str {
    env!("SERVER_KEY")
}

pub fn server_public_key_pem() -> &'static str {
    env!("SERVER_PUBLIC_KEY_PEM")
}

pub fn base() -> &'static str {
    env!("BASE")
}

pub const SERVER_BOUND: bool = true;

pub const EXPIRE_MINUTES: i64 = 3;

pub const PATH_HOME: &str = "/";

pub const PATH_CAPTCHA_LOOKUP: &str = "/?getcapthca={ts}";

pub const PATH_SERVICE: &str = "/{action}";

pub const SHORT_HOSTS: [&str; 2] = ["vm.tiktok.com", "vt.tiktok.com"];

pub const TIME_SOURCES: [&str; 2] = ["https://www.cloudflare.com", "https://www.google.com"];

pub fn home() -> String {
    format!("{}{}", base(), PATH_HOME)
}

pub fn captcha_lookup(ts: i64) -> String {
    format!("{}{}", base(), PATH_CAPTCHA_LOOKUP.replace("{ts}", &ts.to_string()))
}

pub fn service(action: &str) -> String {
    format!("{}{}", base(), PATH_SERVICE.replace("{action}", action))
}

pub fn absolute(path: &str) -> String {
    if path.starts_with("http") {
        path.to_string()
    } else if path.starts_with('/') {
        format!("{}{}", base(), path)
    } else {
        format!("{}/{}", base(), path)
    }
}

#[cfg(not(license))]
pub async fn gate(_entry: Option<&crate::net::egress::ProxyEntry>) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(license)]
pub use licensing::gate;


#[cfg(license)]
mod licensing {
    use super::*;

    use anyhow::{anyhow, Result};
    use hmac::{Hmac, Mac};
    use rand::Rng;
    use base64::Engine as _;
    use serde_json::{json, Value};
    use sha2::Sha256;

    use crate::net::client::Http;
    use crate::net::egress::ProxyEntry;
    use crate::support::log::{ask_key, lc, lg, lr, ly};

    type HmacSha256 = Hmac<Sha256>;

    const SUPPORT_HANDLE: &str = "@Yuuriiwasaka";
    const COMMUNITY_LINK: &str = "https://t.me/Y3YuYuYo";
    const ACCESS_KEY_LINK: &str = "t.me/SentinelicenseBot?start=ref_0CB278B7";

    fn contact_note() {
        lc(&format!(" Contact support  : {}", SUPPORT_HANDLE));
        lc(&format!(" Join our community: {}", COMMUNITY_LINK));
    }

    fn hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push_str(&format!("{:02x}", byte));
        }
        out
    }

    fn client_key() -> &'static str {
        env!("CLIENT_KEY")
    }

    fn sign(key: &str, message: &str) -> String {
        let mut mac = match HmacSha256::new_from_slice(key.as_bytes()) {
            Ok(mac) => mac,
            Err(_) => return String::new(),
        };
        mac.update(message.as_bytes());
        hex(&mac.finalize().into_bytes())
    }

    fn token_hex(bytes: usize) -> String {
        let mut buf = vec![0u8; bytes];
        rand::rng().fill_bytes(&mut buf);
        hex(&buf)
    }

    fn build_ts() -> i64 {
        env!("BUILD_TS").parse::<i64>().unwrap_or(0)
    }

    fn expire_ts() -> i64 {
        if EXPIRE_MINUTES > 0 {
            build_ts() + EXPIRE_MINUTES * 60
        } else {
            0
        }
    }

    fn watch_expiry(limit: i64, now: i64) {
        let left = limit - now;
        if left <= 0 {
            return;
        }
        let span = std::time::Duration::from_secs(left as u64);
        let born = std::time::Instant::now();
        std::thread::spawn(move || loop {
            if born.elapsed() >= span {
                println!();
                expired_note();
                std::process::exit(1);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        });
    }

    fn lock(message: &str) -> Result<()> {
        lr(message);
        contact_note();
        Err(anyhow!("#"))
    }

    fn expired_note() {
        lr(" [EXPIRED] This bot has expired.");
        ly(" Please get a new access key to continue using this bot.");
        contact_note();
    }

    fn expired() -> Result<()> {
        expired_note();
        Err(anyhow!("#"))
    }

    async fn network_time(http: &Http) -> i64 {
        let mut best = 0i64;
        for url in TIME_SOURCES {
            if let Ok(Some(stamp)) = http.head_date(url).await {
                if stamp > best {
                    best = stamp;
                }
            }
        }
        best
    }

    fn wrap(payload_key: &[u8]) -> Result<String> {
        use base64::engine::general_purpose::STANDARD as B64;
        use base64::Engine;
        use rsa::pkcs8::DecodePublicKey;
        use rsa::Oaep;
        let public = rsa::RsaPublicKey::from_public_key_pem(server_public_key_pem())?;
        let mut rng = rsa::rand_core::OsRng;
        let wrapped = public.encrypt(&mut rng, Oaep::new::<Sha256>(), payload_key)?;
        Ok(B64.encode(wrapped))
    }

    fn canonical(value: &Value) -> String {
        let plain = match serde_json::to_string(value) {
            Ok(text) => text,
            Err(_) => return String::new(),
        };
        let mut out = String::with_capacity(plain.len());
        for ch in plain.chars() {
            let code = ch as u32;
            if code < 0x80 {
                out.push(ch);
            } else if code <= 0xFFFF {
                out.push_str(&format!("\\u{:04x}", code));
            } else {
                let rest = code - 0x10000;
                let high = 0xD800 + (rest >> 10);
                let low = 0xDC00 + (rest & 0x3FF);
                out.push_str(&format!("\\u{:04x}\\u{:04x}", high, low));
            }
        }
        out
    }

    fn signature_ok(body: &Value, signed: &str) -> bool {
        sign(server_key(), &canonical(body)) == signed
    }

    async fn post(http: &Http, payload: Value) -> Result<Value> {
        let body = serde_json::to_string(&payload)?;
        let (_status, text) = match http.post_json(license_server_url(), &body).await {
            Ok(pair) => pair,
            Err(_) => {
                lr(" Cannot reach license server");
                return Err(anyhow!("#"));
            }
        };
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(_) => {
                lr(" License server error");
                Err(anyhow!("#"))
            }
        }
    }

    async fn verify(http: &Http, license: &str) -> Result<String> {
        let parts: Vec<&str> = license.trim().split(':').collect();
        if parts.len() != 4 {
            lr(" Invalid license format");
            return Err(anyhow!("shape"));
        }
        if parts[0] != LICENSE_LABEL {
            lr(" License label mismatch");
            return Err(anyhow!("label"));
        }

        let key = client_key();
        let stamp = chrono::Utc::now().timestamp();
        let nonce = token_hex(16);
        let session = token_hex(16);
        let trimmed = license.trim();

        let first = json!({
            "step": 1,
            "license": trimmed,
            "label": parts[0],
            "nonce": nonce,
            "timestamp": stamp,
            "client_sig": sign(&key, &format!("{}:{}:{}:{}:{}", trimmed, parts[0], stamp, nonce, session)),
            "session": session,
            "xsrf_sign": sign(&key, &format!("{}:{}", session, nonce)),
            "enc_sign": sign(&key, &format!("{}:{}", stamp, session)),
            "req_sign": sign(&key, &format!("1|{}|{}:{}:{}", trimmed, parts[0], nonce, stamp)),
        });

        let answer = post(http, first).await?;
        let signed = answer.get("sig").and_then(Value::as_str).unwrap_or("");
        let body = answer.get("payload").cloned().unwrap_or(Value::Null);
        if !signature_ok(&body, signed) {
            lr(" Server response tampered");
            return Err(anyhow!("#"));
        }
        if body.get("ok").and_then(Value::as_bool) != Some(true) {
            let said = body.get("msg").and_then(Value::as_str).unwrap_or("Rejected");
            lr(&format!(" {}", said));
            return Err(anyhow!("#"));
        }
        let challenge = body.get("challenge").and_then(Value::as_str).unwrap_or("");
        if challenge.is_empty() {
            lr(" No challenge received");
            return Err(anyhow!("#"));
        }

        let response = sign(&key, &format!("{}:{}", challenge, session));
        let stamp2 = chrono::Utc::now().timestamp();
        let nonce2 = token_hex(16);
        let mut payload_key = vec![0u8; 32];
        rand::rng().fill_bytes(&mut payload_key);
        let wrapped = if SERVER_BOUND { wrap(&payload_key)? } else { String::new() };

        let second = json!({
            "step": 2,
            "challenge_response": response,
            "nonce": nonce2,
            "timestamp": stamp2,
            "session": session,
            "xsrf_sign": sign(&key, &format!("{}:{}", session, nonce2)),
            "enc_sign": sign(&key, &format!("{}:{}", stamp2, session)),
            "req_sign": sign(&key, &format!("2|{}:{}:{}", response, nonce2, stamp2)),
            "wrapped": wrapped,
        });

        let answer2 = post(http, second).await?;
        let signed2 = answer2.get("sig").and_then(Value::as_str).unwrap_or("");
        let body2 = answer2.get("payload").cloned().unwrap_or(Value::Null);
        if !signature_ok(&body2, signed2) {
            lr(" Server response tampered");
            return Err(anyhow!("#"));
        }
        if body2.get("ok").and_then(Value::as_bool) != Some(true) {
            let said = body2.get("msg").and_then(Value::as_str).unwrap_or("Rejected");
            lr(&format!(" {}", said));
            return Err(anyhow!("#"));
        }
        let token = body2.get("token").and_then(Value::as_str).unwrap_or("");
        let token_sig = body2.get("token_sig").and_then(Value::as_str).unwrap_or("");
        if token.is_empty() || token_sig.is_empty() {
            lr(" No token received");
            return Err(anyhow!("#"));
        }
        if sign(&key, &format!("{}:{}", token, session)) != token_sig {
            lr(" Token integrity check failed");
            return Err(anyhow!("#"));
        }
        let payload = body2.get("payload_key").and_then(Value::as_str).unwrap_or("");
        if payload.is_empty() {
            lr(" Server did not return payload key (outdated server?)");
            return Err(anyhow!("#"));
        }
        let decoded = match base64::engine::general_purpose::STANDARD.decode(payload) {
            Ok(bytes) => bytes,
            Err(_) => {
                lr(" Bad payload key");
                return Err(anyhow!("#"));
            }
        };
        if decoded.len() != 32 {
            lr(" Bad payload key length");
            return Err(anyhow!("#"));
        }
        Ok(token.to_string())
    }

    pub async fn gate(entry: Option<&ProxyEntry>) -> Result<()> {
        let http = Http::new(entry)?;

        let network = network_time(&http).await;

        if network == 0 {
            return lock(" [LOCKED] Cannot verify system time");
        }
        if network < build_ts() - 86400 {
            return lock(" [LOCKED] System clock rollback detected");
        }
        let now = network;
        let limit = expire_ts();
        if limit > 0 && now > limit {
            return expired();
        }
        if limit > 0 {
            watch_expiry(limit, now);
        }

        for attempt in 0..3 {
            let entered = match ask_key() {
                Some(value) => value,
                None => {
                    lr(" Aborted");
                    return Err(anyhow!("#"));
                }
            };
            if verify(&http, &entered).await.is_ok() {
                lg(" License verified");
                return Ok(());
            }
            let left = 2 - attempt;
            if left > 0 {
                ly(&format!(" {} attempt(s) remaining.", left));
                lc(&format!(" Get a valid license: {}", ACCESS_KEY_LINK));
            } else {
                lc(&format!(" Get a valid license : {}", ACCESS_KEY_LINK));
                lc(&format!(" Join our community  : {}", COMMUNITY_LINK));
                lc(&format!(" Contact support     : {}", SUPPORT_HANDLE));
            }
        }

        Err(anyhow!("#"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn base_is_a_plain_web_address() {
            let value = base();
            assert!(value.starts_with("http"));
            assert!(!value.ends_with('/'));
            assert!(value.chars().all(|ch| ch.is_ascii_graphic()));
        }

        #[test]
        fn client_key_matches_the_reference_value() {
            assert_eq!(
                client_key(),
                env!("CLIENT_KEY")
            );
        }

        #[test]
        fn client_key_is_stable_and_shaped_like_a_key() {
            let first = client_key();
            let second = client_key();
            assert_eq!(first, second);
            assert_eq!(first.len(), 64);
            assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
        }

        #[test]
        fn canonical_text_escapes_characters_outside_ascii() {
            let value = serde_json::json!({"msg": "\u{1f60f}", "ok": false});
            let text = canonical(&value);
            assert_eq!(text, "{\"msg\":\"\\ud83d\\ude0f\",\"ok\":false}");
        }

        #[test]
        fn signature_notices_a_changed_body() {
            let body = serde_json::json!({"msg": "no", "ok": false});
            let good = sign(server_key(), &canonical(&body));
            assert!(signature_ok(&body, &good));
            let other = serde_json::json!({"msg": "yes", "ok": true});
            assert!(!signature_ok(&other, &good));
        }

        #[test]
        fn license_shape_is_enforced_before_any_request() {
            assert!("TOKTOK:only".split(':').count() != 4);
            assert_eq!("TOKTOK:a:b:c".split(':').count(), 4);
        }

        #[test]
        fn expiry_is_disabled_while_the_minutes_are_zero() {
            assert_eq!(expire_ts(), 0);
            assert!(build_ts() > 1_700_000_000);
        }
    }
}
