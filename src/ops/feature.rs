use anyhow::{anyhow, Result};
use std::collections::HashSet;

use crate::api::{endpoints, protocol};
use crate::net::client::pairs;
use crate::parse::{Form, Fragment};
use crate::support::input;
use crate::support::log::{ask, lg, lr, ly, reason, sanitize, short, unit_word};
use crate::ui::picker;

use super::session::Session;
use super::target;

const MAX_COOLDOWN: u64 = 900;
const MARGIN: u64 = 5;
const STALLS: u8 = 1;
const RESETS: u8 = 3;
const RETRIES: u8 = 2;
const GATES: u8 = 2;
const STEPS: u8 = 3;
const SUBMITS: u8 = 2;
const ROUNDS: u32 = 200;

pub fn masked(target: &str) -> String {
    target
        .split('/')
        .map(|part| {
            if let Some(name) = part.strip_prefix('@') {
                if name.is_empty() {
                    return part.to_string();
                }
                return format!("@{}", "*".repeat(name.chars().count()));
            }

            let digits = !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
            if digits && part.len() >= 6 {
                return "*".repeat(part.chars().count());
            }

            part.to_string()
        })
        .collect::<Vec<String>>()
        .join("/")
}

pub fn target_line(target: &str) -> String {
    let shown = masked(target);
    let shown = shown
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!("Target: {}", short(shown, 56))
}

pub async fn run(session: &mut Session, slug: &str) -> Result<()> {
    session.relogin().await?;

    session
        .dashboard
        .panel_for(slug)
        .ok_or_else(|| anyhow!("panel {slug} missing from dashboard"))?;

    let mut target = target::clean(&input::load("data.txt")?);

    if target::is_short(&target) {
        match session.http.expand(&target).await {
            Ok(Some(full)) if full.contains("tiktok.com") => {
                let clean = target::clean(&full);
                if target::is_photo(&clean) {
                    ly("The short link was expanded to the full post link");
                } else if target::video_id(&clean).is_some() {
                    ly("The short link was expanded to the full video link");
                } else {
                    ly("The short link was expanded to the full profile link");
                }
                target = clean;
            }
            _ => ly("The short link could not be expanded, sending it as it is"),
        }
    }

    lg(&target_line(&target));

    let label = session
        .dashboard
        .cards
        .iter()
        .find(|card| card.slug == slug)
        .map(|card| card.title.clone())
        .unwrap_or_else(|| "Service".to_string());

    match search(session, slug, &target).await {
        Ok(Some((action, fragment))) => deliver(session, &action, &label, fragment, 0).await?,
        Ok(None) => {}
        Err(error) => {
            let text = error.to_string();
            let shown = if text.starts_with("The ") {
                sanitize(&text)
            } else {
                reason(&text).to_string()
            };
            lr(&format!(
                "Search failed: {}, leaving the target for the next cycle",
                shown
            ));
        }
    }

    session.sleep_cycle().await;

    Ok(())
}

fn candidates(target: &str) -> Vec<String> {
    let bare = target
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let plain = bare.trim_start_matches("www.");

    let other = if plain == bare {
        format!("https://www.{}", plain)
    } else {
        format!("https://{}", bare)
    };

    let mut out: Vec<String> = vec![format!("https://{}", plain)];
    out.push(other);

    if let Some(id) = target::video_id(target) {
        if !out.iter().any(|value| value == &id) {
            out.push(id);
        }
    }

    out
}

async fn search(
    session: &mut Session,
    slug: &str,
    target: &str,
) -> Result<Option<(String, Fragment)>> {
    let mut waited: u64 = 0;
    let mut resets: u8 = 0;
    let mut retries: u8 = 0;
    let mut stalls: u8 = 0;
    let mut gates: u8 = 0;
    let mut last_wait: u64 = 0;
    let mut tries = candidates(target);

    'round: loop {
        let Some(panel) = session.dashboard.panel_for(slug) else {
            return Err(anyhow!("The dashboard no longer carries this service"));
        };

        let action = panel.action.clone();
        let field = panel.field.clone();
        let value = tries.first().cloned().unwrap_or_default();
        let url = endpoints::service(&action);
        let form = pairs(&[(field.as_str(), value.as_str())]);

        if waited > MAX_COOLDOWN {
            return Err(anyhow!(
                "Site is throttling this address, waited {} {}, try again later",
                waited,
                unit_word(waited, "second", "seconds")
            ));
        }

        let answer = session.http.post_form(&url, &form).await;
        let (status, raw) = match answer {
            Ok(pair) => pair,
            Err(_) => {
                if gates == GATES {
                    return Err(anyhow!(
                        "The service stopped answering after the gate was cleared {} times",
                        gates
                    ));
                }
                gates += 1;
                ly("The request did not come back, signing in from scratch before the next try");
                session.relogin().await?;
                stalls = 0;
                last_wait = 0;
                continue 'round;
            }
        };
        let body = protocol::unseal(&raw);

        if status != 200 {
            return Err(anyhow!(
                "The service answered with status {}, try later",
                status
            ));
        }

        if protocol::is_gate(&body) || body.contains("captchalogin") {
            if gates == GATES {
                return Err(anyhow!(
                    "The gate came back {} times, the service will not hold a session",
                    gates
                ));
            }
            gates += 1;
            session.relogin().await?;
            stalls = 0;
            last_wait = 0;
            continue 'round;
        }

        if let Some(wait) = protocol::wait_seconds(&body) {
            if wait == last_wait {
                stalls += 1;
            } else {
                stalls = 0;
                last_wait = wait;
            }

            if stalls > STALLS {
                ly("The pause is stuck on the same number, leaving the target for the next cycle");
                return Ok(None);
            }

            if waited + wait > MAX_COOLDOWN {
                ly(&format!(
                    "The service kept asking for a pause for {} {}, leaving the target for the next cycle",
                    waited,
                    unit_word(waited, "second", "seconds")
                ));
                return Ok(None);
            }

            waited += wait;

            ly(&format!(
                "The site asked for a pause of {} {}",
                wait,
                unit_word(wait, "second", "seconds")
            ));
            session.sleep_server(wait + MARGIN).await;
            ly("The pause is over, sending the same search again");
            continue;
        }

        if protocol::is_dashboard(&body) {
            if resets == RESETS {
                return Err(anyhow!(
                    "The action token kept expiring, refresh the dashboard and try again"
                ));
            }
            resets += 1;
            ly("The action token expired, reading the dashboard again");
            if !session.refresh().await.unwrap_or(false) {
                session.relogin().await?;
            }
            continue 'round;
        }

        let fragment = Fragment::parse(&body);
        if !fragment.forms.is_empty() || !fragment.pages.is_empty() {
            return Ok(Some((action, fragment)));
        }

        let mut said = protocol::loose_text(&body)
            .map(|text| plain(&text))
            .unwrap_or_default();
        if said.is_empty() {
            said = plain(&fragment.message);
        }

        if protocol::link_rejected(&said) || protocol::link_rejected(&body) {
            tries.remove(0);
            if let Some(next) = tries.first() {
                if next.chars().all(|c| c.is_ascii_digit()) {
                    ly("The server wants the video ID, sending it on its own");
                } else {
                    ly("The server wants the link in another form, sending it again");
                }
                continue 'round;
            }
        }

        if retries < RETRIES {
            retries += 1;
            ly("The service stumbled, reading the dashboard again");
            if !session.refresh().await.unwrap_or(false) {
                session.relogin().await?;
            }
            continue 'round;
        }

        return Err(anyhow!(
            "The service did not open the panel for this target, try another link"
        ));
    }
}

async fn comment_flow(
    session: &mut Session,
    action: &str,
    label: &str,
    first: Fragment,
) -> Result<()> {
    let mut rows: Vec<Form> = first
        .forms
        .into_iter()
        .filter(|form| !form.username.is_empty())
        .collect();
    let mut pending = first.pages;
    let mut visited: HashSet<String> = HashSet::new();

    while let Some(page) = pending.pop() {
        let key = page
            .fields
            .iter()
            .find(|(name, _)| name == "p")
            .map(|(_, value)| value.clone())
            .unwrap_or_default();
        if key.is_empty() || visited.contains(&key) {
            continue;
        }

        let target = if page.action.is_empty() {
            endpoints::service(action)
        } else {
            endpoints::service(&page.action)
        };

        let (_, raw) = session.http.post_form(&target, &page.fields).await?;
        let body = protocol::unseal(&raw);

        if let Some(wait) = protocol::wait_seconds(&body) {
            session.sleep_server(wait).await;
            pending.push(page);
            continue;
        }

        visited.insert(key);

        let fragment = Fragment::parse(&body);
        rows.extend(
            fragment
                .forms
                .into_iter()
                .filter(|form| !form.username.is_empty()),
        );
        pending.extend(fragment.pages);
    }

    if rows.is_empty() {
        lr("No comment came back from the server");
        return Ok(());
    }

    let total = rows.len() as u64;
    lg(&format!(
        "{} {} ready to pick",
        total,
        unit_word(total, "comment is", "comments are")
    ));

    loop {
        let refs: Vec<&Form> = rows.iter().collect();
        let Some(index) = picker::pick(&refs, "Search comment")? else {
            break;
        };
        let chosen = rows[index].clone();
        let limit = picker::best_limit(&chosen.limits);
        let before = Some(chosen.hearts as u64);

        let reply = submit(session, action, &chosen, &limit).await?;
        totals(label, before, reply.sent);

        if reply.panel.is_some() {
            ly("The service answered the submission with another panel");
        }

        if let Some(wait) = reply.wait {
            session.sleep_server(wait).await;
        }

        if !again()? {
            break;
        }
    }

    Ok(())
}

async fn deliver(
    session: &mut Session,
    action: &str,
    label: &str,
    fragment: Fragment,
    depth: u8,
) -> Result<()> {
    if depth > STEPS {
        lr("The service kept answering with another panel, stopping here");
        return Ok(());
    }

    if fragment.has_rows() {
        return comment_flow(session, action, label, fragment).await;
    }

    if fragment.forms.iter().any(|form| !form.limits.is_empty()) {
        return direct_flow(session, action, label, fragment, depth).await;
    }

    if !fragment.forms.is_empty() {
        return button_flow(session, action, label, fragment, depth).await;
    }

    if !fragment.message.is_empty() {
        ly("The service closed the panel without a form to send");
        return Ok(());
    }

    lr("The reply carried no submittable form");
    Ok(())
}

async fn button_flow(
    session: &mut Session,
    action: &str,
    label: &str,
    fragment: Fragment,
    depth: u8,
) -> Result<()> {
    let Some(form) = fragment.forms.first().cloned() else {
        lr("The reply carried no button to press");
        return Ok(());
    };

    let before: Option<u64> = None;
    let mut rounds: u32 = 0;

    loop {
        if rounds == ROUNDS {
            ly("The round limit is reached, waiting for the next cycle");
            break;
        }
        rounds += 1;

        let reply = submit(session, action, &form, "").await?;

        if reply.expired {
            ly("The action token expired, waiting for the next cycle");
            break;
        }

        totals(label, before, reply.sent);

        if !reply.ok {
            break;
        }

        if let Some(next) = reply.panel {
            return Box::pin(deliver(session, action, label, next, depth + 1)).await;
        }

        if let Some(wait) = reply.wait {
            session.sleep_server(wait + MARGIN).await;
        }
    }

    Ok(())
}

async fn direct_flow(
    session: &mut Session,
    action: &str,
    label: &str,
    fragment: Fragment,
    depth: u8,
) -> Result<()> {
    let Some(form) = fragment.forms.iter().find(|form| !form.limits.is_empty()) else {
        lr("The reply carried no limit selector");
        return Ok(());
    };

    let limit = picker::best_limit(&form.limits);
    let before = counter_value(&form.counter);

    let reply = submit(session, action, form, &limit).await?;
    totals(label, before, reply.sent);

    if let Some(wait) = reply.wait {
        session.sleep_server(wait).await;
    }

    match reply.panel {
        Some(next) => Box::pin(deliver(session, action, label, next, depth + 1)).await,
        None => Ok(()),
    }
}

#[derive(Debug, Default)]
struct Reply {
    panel: Option<Fragment>,
    sent: Option<u64>,
    wait: Option<u64>,
    ok: bool,
    expired: bool,
}

async fn submit(session: &mut Session, action: &str, form: &Form, limit: &str) -> Result<Reply> {
    let mut fields = form.fields.clone();
    if !form.limits.is_empty() {
        fields.push(("select_lmt".to_string(), limit.to_string()));
    }

    let target = if form.action.is_empty() {
        endpoints::service(action)
    } else {
        endpoints::service(&form.action)
    };

    let mut tries: u8 = 0;
    let mut waited: u64 = 0;

    loop {
        tries += 1;

        let answer = session.http.post_form(&target, &fields).await;
        let (status, raw) = match answer {
            Ok(pair) => pair,
            Err(_) => {
                if tries > SUBMITS {
                    ly("The submission did not come back, leaving it for the next cycle");
                    return Ok(Reply::default());
                }
                ly("The submission did not come back, signing in again before the next try");
                session.relogin().await?;
                continue;
            }
        };

        if status != 200 {
            lr(&format!("Submission returned HTTP {}", status));
            return Ok(Reply::default());
        }

        let body = protocol::unseal(&raw);

        if protocol::is_gate(&body) {
            if tries > SUBMITS {
                ly("The session dropped during the submission, leaving it for the next cycle");
                return Ok(Reply::default());
            }
            ly("The session dropped during the submission, signing in from scratch");
            session.relogin().await?;
            ly("The session was renewed, sending the same submission again");
            continue;
        }

        if protocol::is_dashboard(&body) {
            return Ok(Reply {
                expired: true,
                ..Reply::default()
            });
        }

        let mut sent = protocol::sent_count(&body);
        let counted = sent.is_some() || protocol::sent_ok(&body);

        if sent.is_none() && protocol::sent_ok(&body) {
            sent = limit.trim().parse::<u64>().ok();
        }

        let fragment = Fragment::parse(&body);

        if !fragment.forms.is_empty() || !fragment.pages.is_empty() {
            return Ok(Reply {
                panel: Some(fragment),
                sent,
                wait: None,
                ok: true,
                expired: false,
            });
        }

        if counted {
            if let Some(value) = sent {
                lg(&format!(
                    "The site counted {} for this target",
                    group(value)
                ));
            } else {
                lg("The site accepted the submission for this target");
            }
            return Ok(Reply {
                panel: None,
                sent,
                wait: protocol::wait_seconds(&body),
                ok: true,
                expired: false,
            });
        }

        if let Some(wait) = protocol::wait_seconds(&body) {
            if waited + wait > MAX_COOLDOWN {
                ly(&format!(
                    "The service kept asking for a pause for {} {}, leaving the target for the next cycle",
                    waited,
                    unit_word(waited, "second", "seconds")
                ));
                return Ok(Reply::default());
            }

            waited += wait;

            ly(&format!(
                "The site asked for a pause of {} {}",
                wait,
                unit_word(wait, "second", "seconds")
            ));
            session.sleep_server(wait + MARGIN).await;
            ly("The pause is over, sending the same submission again");
            continue;
        }

        if protocol::is_rejection(&body) {
            lr("Submission was refused");
            return Ok(Reply::default());
        }

        lr("The reply to the submission could not be read");
        return Ok(Reply::default());
    }
}

fn counter_value(text: &str) -> Option<u64> {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u64>().ok()
}

fn group(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();

    for (index, c) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }

    out
}

fn totals(label: &str, before: Option<u64>, sent: Option<u64>) {
    if let Some(before) = before {
        lg(&format!("{} before: {}", label, group(before)));
    }
    if let Some(sent) = sent {
        lg(&format!("{} sent: {}", label, group(sent)));
    }
    if let (Some(before), Some(sent)) = (before, sent) {
        lg(&format!("{} after: {}", label, group(before + sent)));
    }
    if before.is_some() && sent.is_none() {
        ly(&format!(
            "The service did not report how many {} were counted",
            label.to_lowercase()
        ));
    }
}

fn plain(message: &str) -> String {
    message
        .split_whitespace()
        .filter(|word| !word.starts_with('#'))
        .collect::<Vec<&str>>()
        .join(" ")
}

fn again() -> Result<bool> {
    let answer = ask("Like another comment? y or n: ");
    Ok(matches!(answer.to_lowercase().as_str(), "y" | "ya" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_link_ladder_starts_without_www_then_keeps_it_then_tries_the_id() {
        let ladder = candidates("https://www.tiktok.com/@im_dyiiir/video/7659266382223232274");
        assert_eq!(
            ladder[0],
            "https://tiktok.com/@im_dyiiir/video/7659266382223232274"
        );
        assert_eq!(
            ladder[1],
            "https://www.tiktok.com/@im_dyiiir/video/7659266382223232274"
        );
        assert_eq!(ladder[2], "7659266382223232274");
        assert_eq!(ladder.len(), 3);
    }

    #[test]
    fn a_link_without_www_gets_the_www_form_as_its_second_try() {
        let ladder = candidates("https://tiktok.com/@im_dyiiir/video/7659266382223232274");
        assert_eq!(
            ladder[1],
            "https://www.tiktok.com/@im_dyiiir/video/7659266382223232274"
        );
        assert_eq!(ladder[2], "7659266382223232274");
    }

    #[test]
    fn a_panel_beside_an_error_line_is_still_a_panel() {
        let body = "<div class=\"row text-light d-flex justify-content-center\">
<form action=\"c2VuZC9mb2xsb3dlcnNfdGlrdG9r\" onsubmit=\"fcde()\">
<input type=\"hidden\" name=\"bf3516c5f\" value=\"7697663840804293908\">
<button type=\"submit\"><i class=\"fa fa-comments\"></i> 39</button>
</form></div>
<span class=\"text-black h6 text-danger nonec error\">An error occurred. Please try again.</span>";
        let fragment = Fragment::parse(body);
        assert_eq!(fragment.forms.len(), 1);
        assert_eq!(fragment.forms[0].fields[0].0, "bf3516c5f");
        assert_eq!(fragment.forms[0].action, "c2VuZC9mb2xsb3dlcnNfdGlrdG9r");
        assert!(fragment.forms[0].limits.is_empty());
        assert!(!fragment.message.is_empty());
    }

    #[test]
    fn target_line_drops_the_scheme() {
        assert_eq!(
            target_line("https://www.tiktok.com/@im_dyiiir/video/765926638222323227"),
            "Target: www.tiktok.com/@*********/video/******************"
        );
    }

    #[test]
    fn the_handle_and_the_post_id_are_hidden() {
        assert_eq!(
            masked("https://tiktok.com/@muhreyyyy_/video/7697663840804293908"),
            "https://tiktok.com/@**********/video/*******************"
        );
        assert_eq!(
            masked("https://tiktok.com/@im_dyiiir/photo/7659266382223232274"),
            "https://tiktok.com/@*********/photo/*******************"
        );
        assert_eq!(
            masked("https://tiktok.com/@im_dyiiir"),
            "https://tiktok.com/@*********"
        );
        assert_eq!(masked("7659266382223232274"), "*******************");
    }

    #[test]
    fn the_total_is_grouped_like_the_site_shows_it() {
        assert_eq!(group(5893), "5,893");
        assert_eq!(group(589), "589");
        assert_eq!(group(1234567), "1,234,567");
        assert_eq!(counter_value("5,893"), Some(5893));
        assert_eq!(counter_value(""), None);
    }
}
