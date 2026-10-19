
use super::dom;

#[derive(Debug, Clone, Default)]
pub struct Form {
    pub action: String,
    pub fields: Vec<(String, String)>,
    pub limits: Vec<String>,
    pub username: String,
    pub text: String,
    pub age: String,
    pub hearts: u32,
    pub counter: String,
}

#[derive(Debug, Default)]
pub struct Fragment {
    pub forms: Vec<Form>,
    pub message: String,
    pub pages: Vec<Form>,
}

impl Fragment {
    pub fn parse(html: &str) -> Self {
        let mut forms = Vec::new();
        let mut pages = Vec::new();
        let mut cursor = 0;

        while let Some(offset) = html[cursor..].find("<form") {
            let start = cursor + offset;
            let Some(block) = dom::block_at(html, start, "form") else {
                break;
            };
            cursor = start + block.len();

            let head_end = block.find('>').unwrap_or(block.len());
            let head = &block[..head_end];
            let action = dom::attr(head, "action").unwrap_or_default();
            let fields = hidden_fields(block);

            let is_pager = fields.iter().any(|(name, _)| name == "p");
            if is_pager {
                pages.push(Form { action, fields, ..Form::default() });
                continue;
            }

            let items = dom::blocks_with_class(block, "li", "list-group-item");
            if items.len() > 1 {
                for item in items {
                    let row_fields = hidden_fields(item);
                    if row_fields.is_empty() {
                        continue;
                    }
                    let mut row = row_of(action.clone(), item);
                    row.fields = row_fields;
                    row.limits = option_values(item);
                    forms.push(row);
                }
                continue;
            }

            forms.push(row_of(action, block));
        }

        let message = dom::element_text(html, "text-danger nonec error")
            .or_else(|| dom::element_text(html, "text-primary font-weight-bold"))
            .unwrap_or_default();

        Self { forms, message, pages }
    }

    pub fn has_rows(&self) -> bool {
        self.forms.iter().any(|form| !form.username.is_empty())
    }

}

fn counter_of(block: &str) -> String {
    let Some(open) = block.find("<button") else {
        return String::new();
    };
    let Some(close) = block[open..].find("</button>") else {
        return String::new();
    };
    let text = dom::text_only(&block[open..open + close]);

    text.split_whitespace()
        .filter(|word| word.chars().any(|c| c.is_ascii_digit()))
        .last()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_ascii_digit() && c != ',')
                .to_string()
        })
        .unwrap_or_default()
}

fn row_of(action: String, block: &str) -> Form {
    let username = dom::element_text(block, "kadi-rengi").unwrap_or_default();
    let text = dom::element_text(block, "text-dark font-weight-bold").unwrap_or_default();
    let age = dom::element_text(block, "text-muted").unwrap_or_default();
    let hearts = dom::element_text(block, "text-green font-weight-bold")
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(0);

    Form {
        action,
        fields: hidden_fields(block),
        limits: option_values(block),
        username,
        text,
        age,
        hearts,
        counter: counter_of(block),
    }
}

fn hidden_fields(block: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cursor = 0;

    while let Some(offset) = block[cursor..].find("<input") {
        let start = cursor + offset;
        let end = match block[start..].find('>') {
            Some(e) => start + e + 1,
            None => break,
        };
        let tag = &block[start..end];
        cursor = end;

        let kind = dom::attr(tag, "type").unwrap_or_else(|| "text".to_string());
        if kind != "hidden" {
            continue;
        }
        if let Some(name) = dom::attr(tag, "name") {
            let value = dom::attr(tag, "value").unwrap_or_default();
            out.push((name, value));
        }
    }

    out
}

fn option_values(block: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0;

    while let Some(offset) = block[cursor..].find("<option") {
        let start = cursor + offset;
        let end = match block[start..].find('>') {
            Some(e) => start + e + 1,
            None => break,
        };
        let tag = &block[start..end];
        cursor = end;

        if tag.contains("disabled") {
            continue;
        }
        if let Some(value) = dom::attr(tag, "value") {
            if !out.contains(&value) {
                out.push(value);
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMENT: &str = r#"<form action="c2VuZC9mb2xsb3dlcnNfdGlrdG9r" class="w1a">
<ul class="list-group"><li class="list-group-item"><div class="d-flex flex-column text-break">
<span><div class="font-weight-bold d-inline-flex kadi-rengi">@&#101;&#120;&#97;&#109;&#112;&#108;&#101;&#95;&#117;&#115;&#101;&#114;</div></span>
<span class="text-dark font-weight-bold">&#83;&#97;&#109;&#112;&#108;&#101;&#32;&#99;&#111;&#109;&#109;&#101;&#110;&#116;</span>
<span class="text-muted">3 days ago</span>
<div><span class="text-green font-weight-bold">21</span> <i class="text-red fa fa-heart"></i></div>
<input type="hidden" name="7dd9396c98693d55104c98a" value="1000000000000000001" required>
<input type="hidden" name="b722469d194d110db19d939c74b" value="1000000000000000002" required>
<input type="hidden" name="6d6644e5850826aaa85f8f9fcdf59" value="https://www.tiktok.com/@example_user/video/1000000000000000001">
<input type="hidden" class="input" name="915e24732ea092e1cd335c72510141" value="">
<select name="select_lmt"><option value="25" disabled>Select Limit</option><option value="25">25</option><option value="50">50</option><option value="75">75</option><option value="100">100</option></select>
<button type="submit">go</button></div></li></ul></form>
<form action="c2VuZC9mb2xsb3dlcnNfdGlrdG9r"><input type="hidden" name="cfbec9ac647c06723a7480bdd2f6" value="1000000000000000001"><input type="hidden" name="p" value="40"><button type="submit">next</button></form>"#;

    #[test]
    fn reads_comment_row() {
        let fragment = Fragment::parse(COMMENT);
        assert!(fragment.has_rows());
        let row = fragment.forms.first().expect("row");
        assert_eq!(row.username, "@example_user");
        assert_eq!(row.text, "Sample comment");
        assert_eq!(row.age, "3 days ago");
        assert_eq!(row.hearts, 21);
        assert_eq!(row.limits, vec!["25", "50", "75", "100"]);
        assert_eq!(row.fields.len(), 4);
    }

    #[test]
    fn separates_pagination_from_rows() {
        let fragment = Fragment::parse(COMMENT);
        assert_eq!(fragment.pages.len(), 1);
        assert_eq!(fragment.forms.len(), 1);
        assert_eq!(fragment.pages[0].fields[1], ("p".to_string(), "40".to_string()));
    }

    #[test]
    fn one_form_with_many_items_gives_one_row_per_item() {
        let item = |index: usize| {
            format!(
                "<li class=\"list-group-item\"><div class=\"d-flex flex-column text-break\">\
<span><div class=\"font-weight-bold d-inline-flex kadi-rengi\">@user{index}</div></span>\
<span class=\"text-dark font-weight-bold\">komentar {index}</span>\
<span class=\"text-muted\">{index} hours ago</span>\
<div><span class=\"text-green font-weight-bold\">0</span></div>\
<input type=\"hidden\" name=\"4fcd0ca6f423cdb2\" value=\"7697663840804293908\" required>\
<input type=\"hidden\" name=\"248713af7dd8be\" value=\"76978497407937257{index}\" required>\
<select name=\"select_lmt\"><option value=\"25\">25</option><option value=\"50\">50</option></select>\
</div></li>"
            )
        };
        let body = format!(
            "<form action=\"c2VuZC9mb2xsb3dlcnNfdGlrdG9r\" class=\"w1a\"><ul class=\"list-group\">{}{}{}</ul></form>",
            item(1),
            item(2),
            item(3)
        );

        let fragment = Fragment::parse(&body);
        assert_eq!(fragment.forms.len(), 3);
        assert_eq!(fragment.forms[2].username, "@user3");
        assert_eq!(fragment.forms[1].text, "komentar 2");
        assert_eq!(fragment.forms[0].fields.len(), 2);
        assert_eq!(fragment.forms[0].limits, vec!["25", "50"]);
        assert_eq!(fragment.forms[0].action, "c2VuZC9mb2xsb3dlcnNfdGlrdG9r");
    }

    #[test]
    fn the_panel_button_carries_the_total() {
        let body = "<div class=\"row\"><form action=\"c2VuZF9mb2xsb3dlcnNfdGlrdG9L\">\
<input type=\"hidden\" name=\"3afb0b120d1d0b30bad2abbd6f\" value=\"7697663840804293908\">\
<button type=\"submit\" class=\"wbutton\"><i class=\"fa fa-bookmark text-danger fa-lg\"></i> 5,893</button>\
<select name=\"select_lmt\"><option value=\"25\">25</option></select></form></div>";
        let fragment = Fragment::parse(body);
        assert_eq!(fragment.forms[0].counter, "5,893");
        assert_eq!(fragment.forms[0].limits, vec!["25"]);
    }
}
