use anyhow::{anyhow, Result};
use std::sync::Mutex;
use wreq::header::{HeaderMap, HeaderValue, REFERER};
use wreq::Client;
use wreq_util::{Emulation, Platform, Profile};

use crate::api::endpoints;
use crate::net::egress::ProxyEntry;

pub struct Http {
    client: Client,
    probe: Client,
    cookie: Mutex<Option<String>>,
    pub user_agent: String,
    pub language: String,
}

impl Http {
    pub fn new(entry: Option<&ProxyEntry>) -> Result<Self> {
        let emulation = Emulation::builder()
            .profile(Profile::Chrome149)
            .platform(Platform::Windows)
            .build();

        let mut builder = Client::builder()
            .emulation(emulation.clone())
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(25))
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(wreq::redirect::Policy::limited(10));

        if let Some(proxy) = entry {
            builder = builder.proxy(wreq::Proxy::all(proxy.url())?);
        }

        let client = builder.build()?;

        let mut probe_builder = Client::builder()
            .emulation(emulation)
            .cookie_store(false)
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(wreq::redirect::Policy::none());

        if let Some(proxy) = entry {
            probe_builder = probe_builder.proxy(wreq::Proxy::all(proxy.url())?);
        }

        let probe = probe_builder.build()?;

        Ok(Self {
            client,
            probe,
            cookie: Mutex::new(None),
            user_agent: String::new(),
            language: String::new(),
        })
    }

    pub async fn expand(&self, url: &str) -> Result<Option<String>> {
        let res = self.probe.head(url).send().await?;
        let status = res.status().as_u16();
        if status < 300 || status >= 400 {
            return Ok(None);
        }

        let location = res
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim().to_string());

        Ok(location.filter(|value| !value.is_empty()))
    }

    pub async fn capture_identity(&mut self) -> Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        let capture = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut socket, _) = listener.accept().await.ok()?;
            let mut collected: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 1024];

            loop {
                match socket.read(&mut chunk).await {
                    Ok(0) => break,
                    Ok(read) => {
                        collected.extend_from_slice(&chunk[..read]);
                        if collected.windows(4).any(|w| w == b"\r\n\r\n") || collected.len() > 8192
                        {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            let _ = socket
                .write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            Some(String::from_utf8_lossy(&collected).to_string())
        });

        let url = format!("http://127.0.0.1:{port}/");
        let _ = self.client.get(&url).send().await;

        let head = capture
            .await
            .map_err(|_| anyhow!("identity capture task failed"))?
            .ok_or_else(|| anyhow!("identity capture returned nothing"))?;

        self.user_agent = header_value(&head, "user-agent")
            .ok_or_else(|| anyhow!("client sent no user agent"))?;
        self.language =
            header_value(&head, "accept-language").unwrap_or_else(|| "en-US,en;q=0.9".to_string());

        Ok(())
    }

    pub fn cookie(&self) -> Option<String> {
        self.cookie.lock().ok().and_then(|value| value.clone())
    }

    fn absorb_cookie(&self, headers: &HeaderMap) {
        let Some(raw) = headers.get("set-cookie").and_then(|v| v.to_str().ok()) else {
            return;
        };
        let pair = raw.split(';').next().unwrap_or("").trim();
        if !pair.is_empty() {
            if let Ok(mut slot) = self.cookie.lock() {
                *slot = Some(pair.to_string());
            }
        }
    }

    fn base_headers(&self, extra: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        if let Ok(referer) = HeaderValue::from_str(&endpoints::home()) {
            map.insert(REFERER, referer);
        }
        if let Some(cookie) = self.cookie() {
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                map.insert("cookie", value);
            }
        }
        for (key, value) in extra {
            if let (Ok(name), Ok(parsed)) = (
                key.parse::<wreq::header::HeaderName>(),
                HeaderValue::from_str(value),
            ) {
                map.insert(name, parsed);
            }
        }
        map
    }

    pub async fn get_text(&self, url: &str) -> Result<(u16, String)> {
        let res = self
            .client
            .get(url)
            .headers(self.base_headers(&[]))
            .send()
            .await?;
        self.absorb_cookie(res.headers());
        let status = res.status().as_u16();
        let body = res.text().await?;
        Ok((status, body))
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let res = self
            .client
            .get(url)
            .headers(self.base_headers(&[]))
            .send()
            .await?;
        self.absorb_cookie(res.headers());
        if !res.status().is_success() {
            return Err(anyhow!("image fetch returned {}", res.status()));
        }
        Ok(res.bytes().await?.to_vec())
    }

    pub async fn get_xhr(&self, url: &str) -> Result<String> {
        let res = self
            .client
            .get(url)
            .headers(self.base_headers(&[
                ("x-requested-with", "XMLHttpRequest"),
                ("accept", "application/json, text/javascript, */*; q=0.01"),
            ]))
            .send()
            .await?;
        self.absorb_cookie(res.headers());
        Ok(res.text().await?)
    }

    pub async fn post_json(&self, url: &str, body: &str) -> Result<(u16, String)> {
        let res = self
            .client
            .post(url)
            .headers(self.base_headers(&[("content-type", "application/json"), ("accept", "*/*")]))
            .body(body.to_string())
            .send()
            .await?;
        let status = res.status().as_u16();
        let text = res.text().await?;
        Ok((status, text))
    }

    pub async fn head_date(&self, url: &str) -> Result<Option<i64>> {
        let res = self.probe.head(url).send().await?;
        let stamp = res
            .headers()
            .get("date")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| chrono::DateTime::parse_from_rfc2822(value).ok())
            .map(|value| value.timestamp());
        Ok(stamp)
    }

    pub async fn post_form(&self, url: &str, form: &[(String, String)]) -> Result<(u16, String)> {
        let body = encode_pairs(form);
        let res = self
            .client
            .post(url)
            .headers(self.base_headers(&[
                (
                    "content-type",
                    "application/x-www-form-urlencoded; charset=UTF-8",
                ),
                ("origin", endpoints::base()),
                ("accept", "*/*"),
                ("x-requested-with", "XMLHttpRequest"),
                ("sec-fetch-dest", "empty"),
                ("sec-fetch-mode", "cors"),
                ("sec-fetch-site", "same-origin"),
            ]))
            .body(body)
            .send()
            .await?;
        self.absorb_cookie(res.headers());
        let status = res.status().as_u16();
        let text = res.text().await?;
        Ok((status, text))
    }
}

fn header_value(head: &str, name: &str) -> Option<String> {
    for line in head.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case(name) {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn encode_pairs(pairs: &[(String, String)]) -> String {
    let mut out = String::new();
    for (key, value) in pairs {
        if !out.is_empty() {
            out.push('&');
        }
        out.push_str(&url_encode(key));
        out.push('=');
        out.push_str(&url_encode(value));
    }
    out
}

pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}
