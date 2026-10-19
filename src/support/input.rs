use anyhow::{anyhow, Result};

pub fn load(path: &str) -> Result<String> {
    let raw = std::fs::read_to_string(path).map_err(|_| anyhow!("File {path} was not found"))?;

    raw.lines()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .ok_or_else(|| anyhow!("File data.txt is empty and holds no target link"))
}
