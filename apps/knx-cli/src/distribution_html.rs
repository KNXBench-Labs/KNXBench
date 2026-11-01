//! Renders an owned bounded offline inspection view, never contained help, code or source markup.

use knx_productdb::distribution::DistributionInspection;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

const ROWS: usize = 200;
const TEXT_CHARS: usize = 256;
const IMAGE_TEXT_BYTES: usize = 16 * 1024 * 1024;
fn text(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars().take(TEXT_CHARS) {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    if value.chars().count() > TEXT_CHARS {
        out.push_str(" … [text sampled]");
    }
    out
}
fn optional(value: Option<&str>) -> String {
    value.map(text).unwrap_or_else(|| "[absent]".into())
}
fn sample(out: &mut String, shown: usize, total: usize) {
    writeln!(out,"<p class=sample>Bounded view: {shown} of {total} rows shown. Full facts remain in JSON output.</p>").unwrap();
}
pub(super) fn render(report: &DistributionInspection) -> String {
    let mut out = String::from("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"referrer\" content=\"no-referrer\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'none'; connect-src 'none'; img-src data:; style-src 'unsafe-inline'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; worker-src 'none'\"><title>Distribution inspection</title><style>body{font:16px system-ui,sans-serif;background:#101820;color:#e9f0f5;margin:2rem auto;max-width:1100px;padding:0 1rem}h1,h2{color:#b8e2dd}table{border-collapse:collapse;width:100%}td,th{padding:.6rem;border:1px solid #46535d;text-align:left;overflow-wrap:anywhere}.notice,.sample{padding:1rem;background:#263640;border-left:4px solid #d8b66d}article{border:1px solid #46535d;padding:1rem;margin:1rem 0}img{max-width:100%;height:auto;image-rendering:auto;background:#fff}.fallback{padding:.7rem;border-left:4px solid #d8b66d}code{overflow-wrap:anywhere}</style></head><body><h1>Distribution inspection</h1><p class=notice>Read-only, incomplete source inspection. No installation, authenticity, initialization or write authority. Help and icon archives are metadata only; vendor code is never run.</p>");
    writeln!(out,"<p>Source: <strong>{}</strong><br>SHA-256: <code>{}</code><br>Report format: {}; complete: false</p>",text(&report.source.name),text(&report.source.sha256),report.format_version).unwrap();
    out.push_str("<h2>Applications</h2>");
    sample(
        &mut out,
        report.applications.len().min(ROWS),
        report.applications.len(),
    );
    out.push_str("<table><thead><tr><th>Identity</th><th>Declared name</th></tr></thead><tbody>");
    for app in report.applications.iter().take(ROWS) {
        writeln!(
            out,
            "<tr><td>{}</td><td>{}</td></tr>",
            optional(app.id.as_deref()),
            optional(app.name.as_deref())
        )
        .unwrap();
    }
    out.push_str("</tbody></table><h2>Resource declarations</h2><p>Payload previews belong to declarations, not an ambiguous consumer choice. Each decoded member is embedded at most once. Embedded PNG text budget: 16 MiB; oversized views show an explicit view fallback.</p>");
    sample(
        &mut out,
        report.resource_declarations.len().min(ROWS),
        report.resource_declarations.len(),
    );
    let previews = report
        .resource_previews
        .iter()
        .map(|p| (p.member_ordinal, p))
        .collect::<HashMap<_, _>>();
    let mut shown = HashSet::new();
    let mut image_bytes = 0usize;
    for d in report.resource_declarations.iter().take(ROWS) {
        writeln!(out,"<article data-resource-state=\"{}\"><h3>{}</h3><p>Declared ID: {}<br>Path: {}<br>Resolution: {}</p>",text(&d.passive_presentation.state),optional(d.name.as_deref()),optional(d.id.as_deref()),optional(d.expected_logical_path.as_deref()),text(&d.resolution)).unwrap();
        if let Some(m) = &d.media {
            writeln!(out,"<p>Content: {}; {} bytes<br>Original SHA-256: <code>{}</code><br>Archive: {}; entries: {}</p>",text(&m.content_class),m.size,text(&m.sha256),text(&m.archive_metadata),m.entries.map(|v|v.to_string()).unwrap_or_else(||"not applicable".into())).unwrap();
        }
        let preview = d
            .passive_presentation
            .preview_member_ordinal
            .and_then(|o| previews.get(&o))
            .filter(|p| {
                d.passive_presentation.state == "decodedPreview"
                    && d.media
                        .as_ref()
                        .is_some_and(|m| m.sha256 == p.original_sha256)
            });
        if let Some(p) = preview {
            if shown.contains(&p.member_ordinal) {
                out.push_str("<p class=fallback data-resource-fallback>previewAlreadyShown — see the earlier declaration for this same member.</p>");
            } else if p.png_base64.len() > IMAGE_TEXT_BYTES.saturating_sub(image_bytes) {
                out.push_str("<p class=fallback data-resource-fallback>htmlViewPreviewBudget — decoded preview retained in JSON, not embedded in this bounded view.</p>");
            } else {
                image_bytes += p.png_base64.len();
                shown.insert(p.member_ordinal);
                // Base64 is produced only by the bounded inspector, never a vendor URL or markup.
                writeln!(out,"<img data-resource-preview src=\"data:image/png;base64,{}\" width=\"{}\" height=\"{}\" alt=\"Decoded passive PNG preview\"><p>Derived SHA-256: <code>{}</code>; transformation: {}</p>",p.png_base64,p.width,p.height,text(&p.derived_sha256),text(&p.transformation)).unwrap();
            }
        } else {
            writeln!(
                out,
                "<p class=fallback data-resource-fallback>{}</p>",
                text(&d.passive_presentation.reason)
            )
            .unwrap();
        }
        out.push_str("</article>");
    }
    out.push_str("<h2>Resource consumers</h2>");
    sample(
        &mut out,
        report.resource_consumers.len().min(ROWS),
        report.resource_consumers.len(),
    );
    out.push_str("<table><thead><tr><th>Role</th><th>Declared reference</th><th>Resolution</th><th>Candidates</th></tr></thead><tbody>");
    for c in report.resource_consumers.iter().take(ROWS) {
        writeln!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            text(&c.role),
            optional(c.declared_baggage_ref.as_deref()),
            text(&c.reason),
            c.declaration_candidates.len()
        )
        .unwrap();
    }
    out.push_str("</tbody></table><p>JSON retains all source coordinates, hashes, unknowns, dependency evidence and arithmetic candidates. This bounded presentation does not remove or alter those facts. Script bodies and contained help are not rendered.</p></body></html>\n");
    out
}
