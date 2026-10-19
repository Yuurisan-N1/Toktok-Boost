use crate::api::endpoints;

pub fn host(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    authority
        .split(':')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

pub fn is_short(url: &str) -> bool {
    let name = host(url);
    endpoints::SHORT_HOSTS.contains(&name.as_str())
}

pub fn clean(url: &str) -> String {
    let bare = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_string();

    match bare.split_once("://") {
        Some((scheme, rest)) => match rest.strip_prefix("www.") {
            Some(rest) => format!("{scheme}://{rest}"),
            None => bare,
        },
        None => bare,
    }
}

pub fn is_photo(url: &str) -> bool {
    clean(url).contains("/photo/")
}

pub fn video_id(url: &str) -> Option<String> {
    let clean_url = clean(url);
    let tail = clean_url
        .rsplit_once("/video/")
        .or_else(|| clean_url.rsplit_once("/photo/"))
        .map(|(_, id)| id)?;

    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 6 {
        Some(digits)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_hosts_are_recognised() {
        assert!(is_short("https://vm.tiktok.com/ZSg6hY3Jx/"));
        assert!(is_short("https://vt.tiktok.com/ZSg6hWkjX/"));
        assert!(!is_short("https://www.tiktok.com/@im_dyiiir/video/7659266382223232274"));
        assert_eq!(host("http://127.0.0.1:8099/short"), "127.0.0.1");
    }

    #[test]
    fn the_video_id_survives_a_query_string() {
        assert_eq!(
            video_id("https://www.tiktok.com/@im_dyiiir/video/7659266382223232274?_r=1&_t=ZS-9"),
            Some("7659266382223232274".to_string())
        );
        assert_eq!(video_id("https://vm.tiktok.com/ZSg6hY3Jx/"), None);
        assert_eq!(
            video_id("https://www.tiktok.com/@im_dyiiir/photo/7600589606219828487"),
            Some("7600589606219828487".to_string())
        );
    }

    #[test]
    fn the_clean_link_drops_the_query_and_the_slash() {
        assert_eq!(
            clean("https://www.tiktok.com/@im_dyiiir/video/7659266382223232274?_r=1/"),
            "https://tiktok.com/@im_dyiiir/video/7659266382223232274"
        );
    }
}
