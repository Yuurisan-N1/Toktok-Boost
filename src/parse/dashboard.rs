use super::dom;

#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
    pub slug: String,
    pub enabled: bool,
    pub badge: String,
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub slug: String,
    pub action: String,
    pub field: String,
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub cards: Vec<Card>,
    pub panels: Vec<Panel>,
}

impl Dashboard {
    pub fn parse(html: &str) -> Self {
        Self {
            cards: read_cards(html),
            panels: read_panels(html),
        }
    }

    pub fn panel_for(&self, slug: &str) -> Option<&Panel> {
        self.panels.iter().find(|panel| panel.slug == slug)
    }
}

fn read_cards(html: &str) -> Vec<Card> {
    let mut cards = Vec::new();

    for block in dom::blocks_with_class(html, "div", "card") {
        let Some(title) = dom::element_text(block, "card-title") else {
            continue;
        };
        if title.is_empty() {
            continue;
        }

        let mut slug = String::new();
        let mut enabled = false;

        for tag in html_tags(block, "button") {
            let classes = dom::attr(&tag, "class").unwrap_or_default();
            if let Some(found) = slug_from_classes(&classes, "-button") {
                slug = found;
                enabled = !tag.contains("disabled");
                break;
            }
        }

        if slug.is_empty() {
            continue;
        }

        let badge = dom::element_text(block, "badge").unwrap_or_default();
        cards.push(Card {
            title,
            slug,
            enabled,
            badge,
        });
    }

    cards
}

fn read_panels(html: &str) -> Vec<Panel> {
    let mut panels = Vec::new();

    for block in dom::blocks_with_class(html, "div", "-menu") {
        let head = &block[..block.find('>').unwrap_or(block.len())];
        let classes = dom::attr(head, "class").unwrap_or_default();
        let Some(slug) = slug_from_classes(&classes, "-menu") else {
            continue;
        };

        let Some(form_start) = block.find("<form") else {
            continue;
        };
        let form_head = &block[form_start
            ..block[form_start..]
                .find('>')
                .map(|e| form_start + e)
                .unwrap_or(block.len())];
        let Some(action) = dom::attr(form_head, "action") else {
            continue;
        };

        let mut field = String::new();
        for tag in html_tags(&block[form_start..], "input") {
            let placeholder = dom::attr(&tag, "placeholder")
                .unwrap_or_default()
                .to_lowercase();
            if placeholder.contains("url") || placeholder.contains("username") {
                if let Some(name) = dom::attr(&tag, "name") {
                    field = name;
                    break;
                }
            }
        }

        if field.is_empty() {
            continue;
        }

        panels.push(Panel {
            slug,
            action,
            field,
        });
    }

    panels
}

fn slug_from_classes(classes: &str, suffix: &str) -> Option<String> {
    for token in classes.split_whitespace() {
        if let Some(rest) = token.strip_prefix("t-") {
            if let Some(slug) = rest.strip_suffix(suffix) {
                if !slug.is_empty() {
                    return Some(slug.to_string());
                }
            }
        }
    }
    None
}

fn html_tags(html: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let open = format!("<{tag}");
    let mut cursor = 0;

    while let Some(offset) = html[cursor..].find(&open) {
        let start = cursor + offset;
        let end = match html[start..].find('>') {
            Some(e) => start + e + 1,
            None => break,
        };
        out.push(html[start..end].to_string());
        cursor = end;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<div class="col-sm-4 col-xs-12 p-1 container colsmenu">
<div class="card border-0 shadow-sm bg-dark rounded-0"><h5 class="card-title"> Hearts</h5>
<button disabled class="btn btn-primary btn-block rounded-0 t-hearts-button"><i class="fa fa-heart"></i> Search</button>
<p class="card-text"><small class="badge badge-round badge-danger">soon will be update</small></p></div></div>
<div class="col-sm-4 col-xs-12 p-1 container colsmenu">
<div class="card border-0 shadow-sm bg-dark rounded-0"><h5 class="card-title"> Comments Hearts</h5>
<button class="btn btn-primary btn-block rounded-0 t-chearts-button"><i class="fa fa-heart"></i> Search</button>
<p class="card-text"><small class="badge badge-round badge-warning">20 hours ago updated</small></p></div></div>
<div class="col-sm-5 col-xs-12 p-1 container t-hearts-menu nonec">
<h5 class="card-title mb-3"> Hearts</h5>
<form action="c2VuZE9nb2xsb3dlcnNfdGlrdG9r"><input type="text" class="form-control" name="d31acc81af" placeholder="Enter Video URL" required>
<button type="submit" class="btn">Search</button></form><div id="c2VuZE9nb2xsb3dlcnNfdGlrdG9r"></div></div>"#;

    #[test]
    fn reads_cards_with_state() {
        let board = Dashboard::parse(PAGE);
        assert_eq!(board.cards.len(), 2);
        assert_eq!(board.cards[0].title, "Hearts");
        assert_eq!(board.cards[0].slug, "hearts");
        assert!(!board.cards[0].enabled);
        assert_eq!(board.cards[0].badge, "soon will be update");
        assert_eq!(board.cards[1].title, "Comments Hearts");
        assert!(board.cards[1].enabled);
    }

    #[test]
    fn reads_panel_form() {
        let board = Dashboard::parse(PAGE);
        let panel = board.panel_for("hearts").expect("panel");
        assert_eq!(panel.action, "c2VuZE9nb2xsb3dlcnNfdGlrdG9r");
        assert_eq!(panel.field, "d31acc81af");
    }
}
