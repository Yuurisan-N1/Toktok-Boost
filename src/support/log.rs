const G: &str = "\x1b[1;32m";
const Y: &str = "\x1b[1;33m";
const R: &str = "\x1b[1;31m";
const C: &str = "\x1b[1;36m";
const Z: &str = "\x1b[0m";

static LINES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn lines_written() -> usize {
    LINES.load(std::sync::atomic::Ordering::Relaxed)
}

fn line(color: &str, msg: &str) {
    LINES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("{}{}{}", color, msg, Z);
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub fn lg(msg: &str) {
    line(G, msg);
}

pub fn ly(msg: &str) {
    line(Y, msg);
}

pub fn lr(msg: &str) {
    line(R, msg);
}

pub fn lc(msg: &str) {
    line(C, msg);
}

fn echo_line(msg: &str, answer: &str, tty: bool) -> String {
    let back = if tty {
        "\x1b[1A\x1b[1G\x1b[2K"
    } else {
        "\x1b[1G\x1b[2K"
    };
    format!("{}{}{}{}{}\n", back, G, msg, answer, Z)
}

pub fn ask(msg: &str) -> String {
    print!("{}{}{}", Y, msg, Z);
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let mut answer = String::new();
    let _ = std::io::stdin().read_line(&mut answer);
    let answer = answer.trim().to_string();

    if !answer.is_empty() {
        use std::io::IsTerminal;
        print!("{}", echo_line(msg, &answer, std::io::stdin().is_terminal()));
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    answer
}

pub fn ask_key() -> Option<String> {
    print!("{} Enter license key : {}", Y, C);
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let mut answer = String::new();
    let read = std::io::stdin().read_line(&mut answer);
    print!("{}", Z);
    let _ = std::io::Write::flush(&mut std::io::stdout());

    match read {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(answer.trim().to_string()),
    }
}

pub fn gap() {
    LINES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!();
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub fn squeeze(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut sp = false;
    for c in input.chars() {
        if c.is_control() || c == '\u{feff}' {
            if c == '\n' || c == '\r' || c == '\t' {
                sp = true;
            }
            continue;
        }
        if c.is_whitespace() {
            sp = true;
            continue;
        }
        if sp && !out.is_empty() {
            out.push(' ');
        }
        sp = false;
        out.push(c);
    }
    out
}

pub fn short(input: &str, max: usize) -> String {
    let s = squeeze(input);
    if s.chars().count() <= max {
        return s;
    }
    let cut: String = s.chars().take(max + 1).collect();
    match cut.rfind(' ') {
        Some(i) if i > 0 => cut[..i].trim_end().to_string(),
        _ => s.chars().take(max).collect(),
    }
}

pub fn sanitize(input: &str) -> String {
    let mut out = squeeze(input);
    for key in [
        "accessToken",
        "access_token",
        "refreshToken",
        "initData",
        "init_data",
        "Authorization",
    ] {
        out = redact_after(&out, key);
    }
    out
}

pub fn reason(input: &str) -> &'static str {
    let text = sanitize(input).to_ascii_lowercase();
    if text.contains("timed out") || text.contains("timeout") {
        "the request timed out"
    } else if text.contains("connect") {
        "the connection could not be established"
    } else if text.contains("dns") || text.contains("resolve") || text.contains("name or service") {
        "the address could not be resolved"
    } else if text.contains("body") || text.contains("decode") || text.contains("json") {
        "the reply could not be read"
    } else if text.contains("status") || text.contains("http") {
        "the service answered with an unexpected status"
    } else {
        "the request failed"
    }
}

fn redact_after(input: &str, key: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(pos) = rest.find(key) {
        out.push_str(&rest[..pos]);
        out.push_str(key);
        let tail = &rest[pos + key.len()..];
        let mut idx = 0;
        let bytes = tail.as_bytes();
        while idx < bytes.len()
            && (bytes[idx] == b'"'
                || bytes[idx] == b':'
                || bytes[idx] == b'='
                || bytes[idx] == b' ')
        {
            out.push(bytes[idx] as char);
            idx += 1;
        }
        while idx < bytes.len() {
            let c = bytes[idx] as char;
            if c == '"' || c == '&' || c == ' ' || c == ',' || c == '}' || c == '\n' {
                break;
            }
            idx += 1;
        }
        out.push_str("***");
        rest = &tail[idx..];
    }
    out.push_str(rest);
    out
}

pub fn mask_host(host: &str) -> String {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 {
        return format!("{}*****{}", parts[0], parts[3]);
    }
    let chars: Vec<char> = host.chars().collect();
    if chars.len() > 4 {
        let head: String = chars.iter().take(2).collect();
        let tail: String = chars.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
        format!("{}*****{}", head, tail)
    } else {
        "***".to_string()
    }
}

pub fn unit_word<'a>(value: u64, singular: &'a str, plural: &'a str) -> &'a str {
    if value == 1 {
        singular
    } else {
        plural
    }
}

fn left_text(left: u64) -> String {
    let days = left / 86400;
    let hours = (left % 86400) / 3600;
    let minutes = (left % 3600) / 60;
    let seconds = left % 60;
    if days > 0 {
        return format!("{:02}:{:02}:{:02}:{:02}", days, hours, minutes, seconds);
    }
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub async fn countdown(seconds: u64, label: &str) {
    let text = squeeze(label);
    let mut left = seconds;
    while left > 0 {
        print!("\r{}{} {}{}", Y, text, left_text(left), Z);
        let _ = std::io::Write::flush(&mut std::io::stdout());
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        left -= 1;
    }
    let width = text.chars().count() + 18;
    print!("\r{}\r", " ".repeat(width));
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_text_carries_days() {
        assert_eq!(left_text(45), "00:00:45");
        assert_eq!(left_text(3661), "01:01:01");
        assert_eq!(left_text(2 * 86400 + 3 * 3600 + 47 * 60 + 33), "02:03:47:33");
    }

    #[test]
    fn answered_prompt_is_repainted_green() {
        assert_eq!(
            echo_line("Choice: ", "6", true),
            "\x1b[1A\x1b[1G\x1b[2K\x1b[1;32mChoice: 6\x1b[0m\n"
        );
        assert_eq!(
            echo_line("Choice: ", "q", false),
            "\x1b[1G\x1b[2K\x1b[1;32mChoice: q\x1b[0m\n"
        );
    }

    #[test]
    fn unit_word_pluralises() {
        assert_eq!(unit_word(1, "service is", "services are"), "service is");
        assert_eq!(unit_word(4, "service is", "services are"), "services are");
    }

    #[test]
    fn reason_never_carries_a_link() {
        let raw = "error sending request for uri (https://example.com/aGVsbG8): operation timed out";
        let text = reason(raw);
        assert_eq!(text, "the request timed out");
        assert!(!text.contains("http"), "link survived: {}", text);
        assert!(!text.contains("example"), "host survived: {}", text);
        assert_eq!(reason("dns error: failed to lookup address"), "the address could not be resolved");
        assert_eq!(reason("error decoding response body"), "the reply could not be read");
        assert_eq!(reason("connection refused"), "the connection could not be established");
        assert_eq!(reason("something odd happened"), "the request failed");
    }
}
