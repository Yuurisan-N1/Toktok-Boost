use std::fs;

#[derive(Clone, Debug)]
pub struct ProxyEntry {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub pass: Option<String>,
}

impl ProxyEntry {
    pub fn url(&self) -> String {
        let mut out = format!("{}://", self.scheme);
        if let (Some(u), Some(p)) = (&self.user, &self.pass) {
            out.push_str(&encode_userinfo(u));
            out.push(':');
            out.push_str(&encode_userinfo(p));
            out.push('@');
        }
        out.push_str(&self.host);
        out.push(':');
        out.push_str(&self.port.to_string());
        out
    }

    pub fn label(&self) -> String {
        format!(
            "http://user:pass@{}:{}",
            crate::support::log::mask_host(&self.host),
            self.port
        )
    }
}

fn encode_userinfo(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn parse_line(line: &str) -> Option<ProxyEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (scheme, rest) = match trimmed.find("://") {
        Some(i) => (trimmed[..i].to_string(), &trimmed[i + 3..]),
        None => ("http".to_string(), trimmed),
    };
    let (creds, hostport) = match rest.rfind('@') {
        Some(i) => (Some(&rest[..i]), &rest[i + 1..]),
        None => (None, rest),
    };
    let (user, pass) = match creds {
        Some(c) => match c.split_once(':') {
            Some((u, p)) => (Some(u.to_string()), Some(p.to_string())),
            None => (Some(c.to_string()), None),
        },
        None => (None, None),
    };

    if user.is_none() && hostport.matches(':').count() == 3 {
        let parts: Vec<&str> = hostport.split(':').collect();
        let port = parts[1].parse::<u16>().ok()?;
        return Some(ProxyEntry {
            scheme,
            host: parts[0].to_string(),
            port,
            user: Some(parts[2].to_string()),
            pass: Some(parts[3].to_string()),
        });
    }

    let (host, port_text) = hostport.rsplit_once(':')?;
    let port = port_text.parse::<u16>().ok()?;
    Some(ProxyEntry {
        scheme,
        host: host.to_string(),
        port,
        user,
        pass,
    })
}

#[derive(Clone, Debug, Default)]
pub struct ProxyPool {
    entries: Vec<ProxyEntry>,
}

impl ProxyPool {
    pub fn load_from(path: &str) -> Self {
        let text = fs::read_to_string(path).unwrap_or_default();
        let mut entries = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match parse_line(line) {
                Some(p) => entries.push(p),
                None => crate::support::log::ly(
                    "A proxy line was skipped because its format could not be read",
                ),
            }
        }
        Self { entries }
    }

    pub fn next(&self, index: usize) -> Option<ProxyEntry> {
        if self.entries.is_empty() {
            return None;
        }
        Some(self.entries[index % self.entries.len()].clone())
    }
}
