use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{IsTerminal, Write};

use crate::parse::Form;
use crate::support::log::{ask, lr, ly, short};

const WINDOW: usize = 5;
const G: &str = "\x1b[1;32m";
const Y: &str = "\x1b[1;33m";
const R: &str = "\x1b[1;31m";
const Z: &str = "\x1b[0m";

pub fn pick(rows: &[&Form], label: &str) -> Result<Option<usize>> {
    if rows.is_empty() {
        return Ok(None);
    }
    if !std::io::stdin().is_terminal() {
        return pick_plain(rows, label);
    }
    pick_live(rows, label)
}

fn matches(row: &Form, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let needle = query.to_lowercase();
    row.username.to_lowercase().contains(&needle) || row.text.to_lowercase().contains(&needle)
}

fn filtered<'a>(rows: &[&'a Form], query: &str) -> Vec<&'a Form> {
    rows.iter().copied().filter(|row| matches(row, query)).collect()
}

fn pick_live(rows: &[&Form], label: &str) -> Result<Option<usize>> {
    let mut stdout = std::io::stdout();
    let mut query = String::new();
    let mut cursor = 0usize;
    let mut drawn = 0usize;

    enable_raw_mode()?;
    execute!(stdout, Hide)?;

    let outcome = loop {
        let hits = filtered(rows, &query);
        if cursor >= hits.len() {
            cursor = hits.len().saturating_sub(1);
        }

        if drawn > 0 {
            queue!(stdout, MoveToColumn(0), MoveUp(drawn as u16), Clear(ClearType::FromCursorDown))?;
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "{}{}: {}{}",
            Y,
            label,
            if query.is_empty() { "" } else { &query },
            Z
        ));

        if hits.is_empty() {
            lines.push(format!("{}No comment matches that text{}", R, Z));
        } else {
            let start = window_start(cursor, hits.len());
            for (offset, row) in hits.iter().enumerate().skip(start).take(WINDOW) {
                let color = if offset == cursor { G } else { Y };
                lines.push(format!(
                    "{}{:>2} {:<18} {:>4} {:<8} {}{}",
                    color,
                    offset + 1,
                    short(&row.username, 18),
                    row.hearts,
                    short(&row.age, 8),
                    short(&row.text, 22),
                    Z
                ));
            }
            lines.push(format!(
                "{} {} of {} comments match{}",
                Y,
                hits.len(),
                rows.len(),
                Z
            ));
        }

        lines.push(format!(
            "{}Type to filter, arrows to move, enter to pick, esc to cancel{}",
            Y, Z
        ));

        for line in &lines {
            write!(stdout, "{line}\r\n")?;
        }
        drawn = lines.len();
        stdout.flush()?;

        match event::read()? {
            Event::Key(KeyEvent { code, modifiers, kind, .. })
                if kind != KeyEventKind::Release =>
            {
                match code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => break None,
                KeyCode::Char(c) => {
                    query.push(c);
                    cursor = 0;
                }
                KeyCode::Backspace => {
                    query.pop();
                    cursor = 0;
                }
                KeyCode::Up => cursor = cursor.saturating_sub(1),
                KeyCode::Down => {
                    if cursor + 1 < hits.len() {
                        cursor += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(row) = hits.get(cursor) {
                        let target = rows.iter().position(|item| std::ptr::eq(*item, *row));
                        break target;
                    }
                }
                KeyCode::Esc => break None,
                _ => {}
                }
            }
            _ => {}
        }
    };

    execute!(stdout, Show)?;
    disable_raw_mode()?;
    println!();

    Ok(outcome)
}

fn window_start(cursor: usize, total: usize) -> usize {
    if total <= WINDOW {
        return 0;
    }
    let half = WINDOW / 2;
    cursor.saturating_sub(half).min(total - WINDOW)
}

fn pick_plain(rows: &[&Form], label: &str) -> Result<Option<usize>> {
    ly(label);
    for (row, number) in rows.iter().take(40).zip(1..) {
        ly(&format!(
            "{:>2} {:<18} {:>4} {:<8} {}",
            number,
            short(&row.username, 18),
            row.hearts,
            short(&row.age, 8),
            short(&row.text, 22)
        ));
    }
    let query = ask("Filter, blank for all: ");
    let hits = filtered(rows, &query);
    if hits.is_empty() {
        lr("No comment matches that text");
        return Ok(None);
    }

    let choice = ask(&format!("Pick 1 to {}: ", hits.len()));
    let index: usize = choice.parse().unwrap_or(1);

    let Some(row) = hits.get(index.saturating_sub(1)) else {
        return Ok(None);
    };
    Ok(rows.iter().position(|item| std::ptr::eq(*item, *row)))
}

pub fn best_limit(limits: &[String]) -> String {
    limits
        .iter()
        .filter_map(|value| value.trim().parse::<u64>().ok().map(|num| (num, value)))
        .max_by_key(|(num, _)| *num)
        .map(|(_, value)| value.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(username: &str, text: &str, hearts: u32) -> Form {
        Form {
            username: username.to_string(),
            text: text.to_string(),
            hearts,
            ..Form::default()
        }
    }

    #[test]
    fn filters_by_username_and_text() {
        let a = row("@fhquran1", "I need 40 Ameen", 21);
        let b = row("@other", "masya allah", 4);
        let c = row("@third", "hello world", 0);
        let rows = vec![&a, &b, &c];

        assert_eq!(filtered(&rows, "").len(), 3);
        assert_eq!(filtered(&rows, "fhq").len(), 1);
        assert_eq!(filtered(&rows, "MASYA").len(), 1);
        assert_eq!(filtered(&rows, "ameen").len(), 1);
        assert_eq!(filtered(&rows, "zzz").len(), 0);
    }

    #[test]
    fn window_keeps_selection_visible() {
        assert_eq!(window_start(0, 3), 0);
        assert_eq!(window_start(0, 20), 0);
        assert_eq!(window_start(9, 20), 7);
        assert_eq!(window_start(19, 20), 15);
    }

    #[test]
    fn short_never_breaks_width() {
        assert_eq!(short("short", 10), "short");
        assert_eq!(short("abcdefghij", 5).chars().count() <= 5, true);
    }

    #[test]
    fn the_biggest_amount_wins_without_asking() {
        let limits = vec![
            "25".to_string(),
            "100".to_string(),
            "50".to_string(),
            "75".to_string(),
        ];
        assert_eq!(best_limit(&limits), "100");
        assert_eq!(best_limit(&["5".to_string(), "75".to_string()]), "75");
        assert_eq!(best_limit(&[]), "");
    }
}
