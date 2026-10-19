
pub fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '&' {
            if let Some(semi) = chars[i..].iter().position(|c| *c == ';') {
                let body: String = chars[i + 1..i + semi].iter().collect();
                if let Some(text) = entity_value(&body) {
                    out.push_str(&text);
                    i += semi + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

fn entity_value(body: &str) -> Option<String> {
    if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
        let code = u32::from_str_radix(hex, 16).ok()?;
        return char::from_u32(code).map(|c| c.to_string());
    }
    if let Some(dec) = body.strip_prefix('#') {
        let code = dec.parse::<u32>().ok()?;
        return char::from_u32(code).map(|c| c.to_string());
    }
    let text = match body {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => " ",
        _ => return None,
    };
    Some(text.to_string())
}

pub fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let mut search_from = 0;

    while let Some(offset) = tag[search_from..].find(&needle) {
        let start = search_from + offset;
        let before_ok = start == 0
            || tag[..start]
                .chars()
                .last()
                .map(|c| c.is_whitespace() || c == '<')
                .unwrap_or(true);
        if !before_ok {
            search_from = start + needle.len();
            continue;
        }
        let rest = &tag[start + needle.len()..];
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let inner = &rest[1..];
            let end = inner.find(quote)?;
            return Some(inner[..end].to_string());
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }

    None
}

pub fn block_at<'a>(html: &'a str, start: usize, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let mut depth = 0i32;
    let mut cursor = start;

    while cursor < html.len() {
        let next_open = html[cursor..].find(&open).map(|p| cursor + p);
        let next_close = html[cursor..].find(&close).map(|p| cursor + p);

        match (next_open, next_close) {
            (Some(o), Some(c)) if o < c => {
                depth += 1;
                cursor = o + open.len();
            }
            (Some(o), None) => {
                depth += 1;
                cursor = o + open.len();
            }
            (_, Some(c)) => {
                depth -= 1;
                cursor = c + close.len();
                if depth <= 0 {
                    return Some(&html[start..cursor]);
                }
            }
            (None, None) => return None,
        }
    }

    None
}

pub fn blocks_with_class<'a>(html: &'a str, tag: &str, marker: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let open = format!("<{tag}");
    let mut cursor = 0;

    while let Some(offset) = html[cursor..].find(&open) {
        let start = cursor + offset;
        let tag_end = match html[start..].find('>') {
            Some(end) => start + end + 1,
            None => break,
        };
        let head = &html[start..tag_end];
        let classes = attr(head, "class").unwrap_or_default();
        if classes.contains(marker) {
            if let Some(block) = block_at(html, start, tag) {
                out.push(block);
                cursor = start + block.len();
                continue;
            }
        }
        cursor = tag_end;
    }

    out
}

pub fn strip_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut depth = 0usize;

    for c in input.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }

    out
}

pub fn text_only(input: &str) -> String {
    decode_entities(&strip_tags(input))
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

pub fn element_text(html: &str, marker: &str) -> Option<String> {
    let hit = html.find(marker)?;
    let after = &html[hit..];
    let tag_close = after.find('>')?;
    let body = &after[tag_close + 1..];
    let end = body.find('<').unwrap_or(body.len());
    Some(decode_entities(body[..end].trim()))
}
