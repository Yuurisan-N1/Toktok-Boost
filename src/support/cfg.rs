use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

#[derive(Clone, Debug)]
pub struct Config {
    raw: Value,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        if !std::path::Path::new(path).exists() {
            let mut f = fs::File::create(path).context("create config.json")?;
            use std::io::Write;
            f.write_all(defaults().as_bytes())
                .context("write default config.json")?;
        }
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path))?;
        let raw: Value =
            serde_json::from_str(&text).with_context(|| format!("{} is not valid json", path))?;
        Ok(Self { raw })
    }

    pub fn sleep_seconds(&self) -> i64 {
        self.raw
            .get("settings")
            .and_then(|s| s.get("sleep_seconds"))
            .and_then(|v| v.as_i64())
            .unwrap_or(3600)
            .max(30)
    }
}

pub fn defaults() -> String {
    r#"{
  "settings": {
    "sleep_seconds": 3600
  }
}
"#
    .to_string()
}
