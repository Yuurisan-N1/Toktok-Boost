use anyhow::Result;

use crate::ops::{feature, session::Session};
use crate::support::log::{ask, gap, lg, lines_written, lr, ly, reason, short};

pub async fn run(session: &mut Session) -> Result<()> {
    let mut lines = show(session);
    let mut extra = 0usize;

    loop {
        let answer = ask("Choice: ");
        if answer.is_empty() {
            return Ok(());
        }
        let answer = answer.to_lowercase();

        match answer.as_str() {
            "q" | "quit" | "exit" => return Ok(()),
            "r" | "refresh" => {
                let mark = lines_written();
                let before = signature(session);
                session.ensure().await?;
                let printed = lines_written() - mark;
                if signature(session) == before {
                    ly("The service list is unchanged");
                    extra += printed + 1;
                } else {
                    erase(lines + extra + printed);
                    lines = show(session);
                    extra = 0;
                }
            }
            _ => {
                let Ok(index) = answer.parse::<usize>() else {
                    lr("That choice is not a number");
                    extra += 1;
                    continue;
                };
                let Some(card) = session
                    .dashboard
                    .cards
                    .get(index.saturating_sub(1))
                    .cloned()
                else {
                    lr("There is no service with that number");
                    extra += 1;
                    continue;
                };
                if !card.enabled {
                    let reason = if card.badge.is_empty() {
                        "closed by the site".to_string()
                    } else {
                        card.badge.clone()
                    };
                    ly(&format!("{} is not open right now: {}", card.title, reason));
                    extra += 1;
                    continue;
                }
                let slug = card.slug.clone();
                let title = card.title.clone();

                loop {
                    if let Err(error) = feature::run(session, &slug).await {
                        lr(&format!(
                            "{}: {}, leaving it for the next cycle",
                            title,
                            reason(&error.to_string())
                        ));
                        break;
                    }

                    let still_open = session
                        .dashboard
                        .cards
                        .iter()
                        .any(|item| item.slug == slug && item.enabled);

                    if !still_open {
                        ly(&format!(
                            "The {} service is closed, back to the menu",
                            title
                        ));
                        break;
                    }

                    ly(&format!(
                        "Starting the next cycle for the {} service",
                        title
                    ));
                }

                lines = show(session);
                extra = 0;
            }
        }
    }
}

fn signature(session: &Session) -> String {
    session
        .dashboard
        .cards
        .iter()
        .map(|card| {
            format!(
                "{}|{}|{}|{}",
                card.title, card.slug, card.enabled, card.badge
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn erase_seq(lines: usize) -> String {
    format!("\x1b[{}A\x1b[0J", lines)
}

fn erase(lines: usize) {
    print!("{}", erase_seq(lines));
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

fn show(session: &Session) -> usize {
    gap();
    ly("Services on the dashboard");

    let mut lines = 2;

    if session.dashboard.cards.is_empty() {
        lr("The dashboard carried no service card");
        lines += 1;
    }

    for (index, card) in session.dashboard.cards.iter().enumerate() {
        let row = format!(
            "{:>2}. {:<22} {}",
            index + 1,
            short(&card.title, 22),
            if card.enabled {
                "open".to_string()
            } else if card.badge.is_empty() {
                "closed".to_string()
            } else {
                short(&card.badge, 24)
            }
        );
        if card.enabled {
            lg(&row);
        } else {
            ly(&row);
        }
        lines += 1;
    }

    ly("r refreshes the list, q quits");
    lines + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::dashboard::{Card, Dashboard};

    fn sample() -> Session {
        Session::for_test(
            crate::net::client::Http::new(None).expect("client"),
            3600,
            Dashboard {
                cards: vec![
                    Card {
                        title: "Followers".to_string(),
                        slug: "followers".to_string(),
                        enabled: true,
                        badge: String::new(),
                    },
                    Card {
                        title: "Hearts".to_string(),
                        slug: "hearts".to_string(),
                        enabled: false,
                        badge: "Full".to_string(),
                    },
                ],
                panels: Vec::new(),
            },
        )
    }

    #[test]
    fn show_reports_its_own_height() {
        let session = sample();
        assert_eq!(show(&session), 5);
    }

    #[test]
    fn erase_walks_back_over_the_block() {
        assert_eq!(erase_seq(5), "\x1b[5A\x1b[0J");
        assert_eq!(erase_seq(7), "\x1b[7A\x1b[0J");
    }

    #[test]
    fn signature_moves_when_a_card_moves() {
        let mut session = sample();
        let before = signature(&session);
        session.dashboard.cards[0].badge = "Full".to_string();
        assert_ne!(before, signature(&session));
    }
}
