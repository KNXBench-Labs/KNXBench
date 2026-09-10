//! HTML escaping and the one-page document shell shared by every section
//! `render.rs` (Task 3) writes into. Kept separate from the model walk so
//! the two properties that must never rot — nothing unescaped reaches the
//! page, nothing but `<style>` decorates it — have one small file each test
//! can hold in its head at once.

/// Escapes text content for insertion between HTML tags: `&`, `<` and `>`.
/// `&` is replaced first so an already-produced `&lt;`/`&gt;` never gets a
/// second pass that turns it into `&amp;lt;`. Every other character,
/// including `"` and umlauts, passes through untouched — text content has
/// no quoting rules to escape.
pub fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

/// Escapes a value for insertion inside a double-quoted HTML attribute:
/// everything [`escape_text`] escapes, plus `"` and `'`. `&` still goes
/// first, for the same reason.
pub fn escape_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// One inline `<style>` block, shared by every generated document. A
/// system font stack (no web font to embed and no network to fetch one
/// from), collapsed table borders, and light backgrounds only — a dark
/// fill prints as grey mud and wastes toner. The `@media print` rule keeps
/// a table row from splitting across a page break and starts each
/// top-level section on its own page. Deliberately free of `transition`
/// and `animation`: a printed page has no switch to turn motion off with,
/// so the only honest choice is to give it nothing to switch
/// (`docs/ROADMAP.md`, "Motion and animation").
pub const DOCUMENT_STYLE: &str = r#"<style>
  :root {
    color-scheme: light;
  }
  body {
    font-family: system-ui, sans-serif;
    max-width: 960px;
    margin: 0 auto;
    padding: 1rem 2rem 3rem;
    color: #1a1a1a;
    background: #ffffff;
  }
  h1, h2, h3 {
    color: #10233d;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: 0.5rem 0 1.5rem;
  }
  caption {
    text-align: left;
    font-weight: 600;
    margin-bottom: 0.25rem;
  }
  th, td {
    border: 1px solid #c8c8c8;
    padding: 0.3rem 0.5rem;
    text-align: left;
    vertical-align: top;
  }
  th {
    background: #f0f0f0;
  }
  tr:nth-child(even) td {
    background: #fafafa;
  }
  section {
    margin-bottom: 2rem;
  }
  nav ul {
    list-style: none;
    padding-left: 0;
  }
  .warning {
    color: #7a4b00;
    background: #fff6e5;
  }
  @media print {
    body {
      max-width: none;
      padding: 0;
    }
    tr {
      page-break-inside: avoid;
    }
    section + section {
      page-break-before: always;
    }
  }
</style>"#;

/// Opens the document: doctype, `<html lang>`, the UTF-8 `<meta charset>`,
/// an escaped `<title>`, and [`DOCUMENT_STYLE`]. Callers write the `<body>`
/// content and close with [`document_tail`].
pub fn document_head(title: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n{}\n</head>\n<body>\n",
        escape_text(title),
        DOCUMENT_STYLE
    )
}

/// Closes what [`document_head`] opened.
pub fn document_tail() -> String {
    "</body></html>".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_text_maps_amp_lt_gt() {
        assert_eq!(escape_text("&<>"), "&amp;&lt;&gt;");
    }

    #[test]
    fn escape_text_leaves_quotes_and_umlauts_untouched() {
        assert_eq!(escape_text("Büro \"Süd\" 'West'"), "Büro \"Süd\" 'West'");
    }

    #[test]
    fn escape_text_amp_before_lt_does_not_double_escape() {
        // If `&` were escaped after `<`, `<` would first become `&lt;` and
        // then the leading `&` of that replacement would be escaped again
        // into `&amp;lt;`. Order matters; this pins it down.
        assert_eq!(escape_text("<"), "&lt;");
        assert!(!escape_text("<").contains("&amp;lt;"));
    }

    #[test]
    fn escape_text_no_special_characters_is_unchanged() {
        assert_eq!(escape_text("Licht Wohnzimmer"), "Licht Wohnzimmer");
    }

    #[test]
    fn escape_text_empty_string_is_empty() {
        assert_eq!(escape_text(""), "");
    }

    #[test]
    fn escape_attr_maps_amp_lt_gt_quot_apos() {
        assert_eq!(escape_attr(r#"&<>"'"#), "&amp;&lt;&gt;&quot;&#39;");
    }

    #[test]
    fn escape_attr_amp_before_others_does_not_double_escape() {
        assert_eq!(escape_attr("\""), "&quot;");
        assert!(!escape_attr("\"").contains("&amp;quot;"));
    }

    #[test]
    fn escape_attr_no_special_characters_is_unchanged() {
        assert_eq!(escape_attr("device-42"), "device-42");
    }

    #[test]
    fn escape_attr_empty_string_is_empty() {
        assert_eq!(escape_attr(""), "");
    }

    #[test]
    fn document_head_emits_doctype_and_lang() {
        let head = document_head("Project");
        assert!(head.starts_with("<!DOCTYPE html>"));
        assert!(head.contains("<html lang="));
    }

    #[test]
    fn document_head_emits_utf8_meta_charset() {
        let head = document_head("Project");
        assert!(head.contains(r#"<meta charset="utf-8">"#));
    }

    #[test]
    fn document_head_title_is_escaped() {
        let head = document_head("Licht & Steckdose <Süd>");
        assert!(head.contains("<title>Licht &amp; Steckdose &lt;S\u{fc}d&gt;</title>"));
        assert!(!head.contains("<title>Licht & Steckdose <Süd></title>"));
    }

    #[test]
    fn document_head_contains_the_style_block() {
        let head = document_head("Project");
        assert!(head.contains(DOCUMENT_STYLE));
    }

    #[test]
    fn document_tail_closes_body_and_html() {
        assert_eq!(document_tail(), "</body></html>");
    }

    #[test]
    fn head_and_tail_contain_no_script_tag() {
        let doc = format!("{}{}", document_head("Project"), document_tail());
        assert!(!doc.contains("<script"));
    }

    #[test]
    fn head_and_tail_contain_no_external_urls() {
        let doc = format!("{}{}", document_head("Project"), document_tail());
        assert!(!doc.contains("http://"));
        assert!(!doc.contains("https://"));
    }

    #[test]
    fn head_and_tail_contain_no_animation_or_transition() {
        let doc = format!("{}{}", document_head("Project"), document_tail());
        assert!(!doc.contains("transition"));
        assert!(!doc.contains("animation"));
    }

    #[test]
    fn style_block_has_a_print_media_rule() {
        assert!(DOCUMENT_STYLE.contains("@media print"));
    }
}
