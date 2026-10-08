"""Renders a verified candidate into one self-contained, offline HTML preview file."""

from __future__ import annotations

import base64
import hashlib
import json
from datetime import date
from html import escape
from pathlib import Path

from .release import check_approval, verify_candidate

SITE_DIR = Path(__file__).resolve().parent.parent / "site"

STATUS_LABELS = {
    "recorded": "Recorded", "planned": "Planned", "implemented": "Implemented",
    "verified": "Verified (bounded)", "superseded": "Superseded", "abandoned": "Abandoned",
    "open": "Open",
}
RELATION_LABELS = {
    "documented_cause": "Documented cause",
    "documented_association": "Documented association",
    "editorial": "Editorial link",
}
SPEAKER_LABELS = {"user": "User prompt", "agent": "Agent message", "document": "Document"}
EVIDENCE_LABELS = {
    "commit": "Commit", "adr": "Decision record", "doc": "Document", "conversation": "Conversation",
    "database": "Database", "local": "Local file",
}
SOURCE_STATUS_LABELS = {"read": "Read", "partial": "Partly read", "unavailable": "Not accessed"}
DEFAULT_ASIDE_LABEL = "Editorial aside"

# The original, unnarrated hero. Kept as literal markup so that editions without a
# narrator keep rendering byte-for-byte as their committed previews.
_DEFAULT_HERO = (
    '<p class="kicker">The shape of an idea</p>'
    '<h1 id="title"><span>One prompt.</span> <span>Then it</span> <span>branched.</span></h1>'
    '<p class="lede">How a request to look inside one home\'s light switches became an independent KNX '
    'engineering application, told from the project\'s own records: the decisions, the detours, and the '
    'people and AI agents behind the branches.</p>'
    '<p class="aside-joke">Scope creep, now with a family tree.</p>'
)
_DEFAULT_ASIDE_GUIDE = '<li><strong>Editorial aside</strong>: our commentary, not part of the record.</li>'


def _hero(narrator: dict | None) -> str:
    if narrator is None:
        return _DEFAULT_HERO
    hero = narrator["hero"]
    title = " ".join(f"<span>{e(line)}</span>" for line in hero["title_lines"])
    return (f'<p class="kicker">{e(hero["kicker"])}</p>'
            f'<h1 id="title">{title}</h1>'
            f'<p class="lede">{e(hero["lede"])}</p>'
            f'<p class="aside-joke">{e(hero["aside"])}</p>')


def _aside_guide(narrator: dict | None) -> str:
    if narrator is None:
        return _DEFAULT_ASIDE_GUIDE
    return (f'<li><strong>{e(narrator["aside_label"])}</strong>: the narrator\'s commentary, not part of the '
            'record.</li>'
            f'<li><strong>Narrator</strong>: {e(narrator["disclosure"])}</li>')


def e(text: object) -> str:
    """Escapes text for HTML element content and quoted attribute values."""
    return escape(str(text), quote=True)


def json_for_script(payload: dict) -> str:
    """Encodes JSON so that no character sequence can end or alter the enclosing script element."""
    text = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return (text.replace("&", "\\u0026").replace("<", "\\u003c").replace(">", "\\u003e")
            .replace("\u2028", "\\u2028").replace("\u2029", "\\u2029"))


def csp_hash(text: str) -> str:
    return "'sha256-" + base64.b64encode(hashlib.sha256(text.encode("utf-8")).digest()).decode("ascii") + "'"


def display_date(event: dict) -> str:
    if event["date_precision"] == "unknown" or not event.get("date"):
        return "Date unknown"
    day = date.fromisoformat(event["date"])
    text = f"{day.day} {day.strftime('%b')} {day.year}"
    return f"Around {text}" if event["date_precision"] == "approximate" else text


def _relation_items(event: dict, payload: dict, titles: dict[str, str]) -> str:
    items = []
    for relation in payload["relations"]:
        if event["id"] not in (relation["from"], relation["to"]):
            continue
        outgoing = relation["from"] == event["id"]
        other = relation["to"] if outgoing else relation["from"]
        direction = "Led to" if outgoing else "Grew from"
        items.append(
            f'<li><span class="rel-dir">{direction}</span> <a href="#ev-{e(other)}">{e(titles[other])}</a> '
            f'<span class="rel-type rel-{e(relation["type"])}">{e(RELATION_LABELS[relation["type"]])}</span>'
            f'<span class="rel-note">{e(relation["note"])}</span></li>'
        )
    return "".join(items)


def render_event_card(event: dict, payload: dict, strands: dict[str, dict], sources: dict[str, dict],
                      titles: dict[str, str], aside_label: str = DEFAULT_ASIDE_LABEL) -> str:
    parts = [
        f'<article class="event-card strand-{e(event["strand"])} status-{e(event["status"])}" '
        f'id="ev-{e(event["id"])}" data-event="{e(event["id"])}" tabindex="-1">',
        '<p class="event-meta">',
        f'<span class="strand-tag">{e(strands[event["strand"]]["label"])}</span>',
        f'<time{" datetime=" + chr(34) + e(event["date"]) + chr(34) if event.get("date") else ""}>'
        f'{e(display_date(event))}</time>',
        f'<span class="status-tag">{e(STATUS_LABELS[event["status"]])}</span>',
        "</p>",
        f'<h3>{e(event["title"])}</h3>',
        f'<p class="event-summary">{e(event["summary"])}</p>',
        f'<p class="event-why"><span class="why-label">Why it mattered</span> {e(event["why"])}</p>',
    ]
    if event.get("aside"):
        parts.append(f'<p class="event-aside"><span class="aside-label">{e(aside_label)}</span> {e(event["aside"])}</p>')
    count = len(event["excerpts"]) + len(event["evidence"])
    parts.append(f'<details class="evidence"><summary>Prompts, evidence and uncertainty '
                 f'<span class="count">{count}</span></summary><div class="evidence-body">')
    for excerpt in event["excerpts"]:
        labels = " · ".join(excerpt["labels"])
        quote_tag = "blockquote" if excerpt["kind"] == "quote" else "p"
        parts.append(
            f'<figure class="excerpt excerpt-{e(excerpt["kind"])}">'
            f'<figcaption class="source-label">{e(SPEAKER_LABELS[excerpt["speaker"]])} · {e(labels)}</figcaption>'
            f'<{quote_tag} class="excerpt-text">{e(excerpt["text"])}</{quote_tag}>'
            + (f'<p class="excerpt-note">{e(excerpt["note"])}</p>' if excerpt.get("note") else "")
            + "</figure>"
        )
    parts.append('<h4>Evidence</h4><ul class="evidence-list">')
    for item in event["evidence"]:
        ref = f' <code>{e(item["ref"])}</code>' if item.get("ref") else ""
        parts.append(f'<li><span class="ev-kind">{e(EVIDENCE_LABELS[item["kind"]])}</span> {e(item["label"])}{ref}'
                     f' <span class="ev-source">{e(sources[item["source"]]["label"])}</span></li>')
    parts.append("</ul><h4>Uncertainty</h4>")
    if event["uncertainty"]:
        parts.append('<ul class="uncertainty">' + "".join(f"<li>{e(note)}</li>" for note in event["uncertainty"]) + "</ul>")
    else:
        parts.append('<p class="uncertainty-none">No specific uncertainty recorded beyond the edition\'s source gaps.</p>')
    relations = _relation_items(event, payload, titles)
    if relations:
        parts.append(f'<h4>Connections</h4><ul class="relations">{relations}</ul>')
    parts.append("</div></details></article>")
    return "".join(parts)


def render_body(payload: dict, story_sha: str, published: bool = False) -> str:
    strands = {strand["id"]: strand for strand in payload["strands"]}
    sources = {source["id"]: source for source in payload["edition"]["sources"]}
    events = {event["id"]: event for event in payload["events"]}
    titles = {ident: event["title"] for ident, event in events.items()}
    edition = payload["edition"]
    narrator = edition.get("narrator")
    aside_label = narrator["aside_label"] if narrator else DEFAULT_ASIDE_LABEL
    first = payload["events"][0]
    out: list[str] = []
    add = out.append

    add('<a class="skip-link" href="#story">Skip to the story</a>')
    add('<div class="frame">')
    add('<header class="masthead"><div class="wordmark">'
        '<svg class="mark" viewBox="0 0 27 33" aria-hidden="true"><path d="M13.5 2v11M13.5 13L3 24M13.5 13L24 24" '
        'fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="13.5" cy="3" r="2.5" fill="currentColor"/>'
        '<circle cx="3" cy="25" r="2.5" fill="currentColor"/><circle cx="24" cy="25" r="2.5" fill="currentColor"/></svg>'
        'KNXBench</div>'
        f'<div class="masthead-note">Project evolution · Edition {e(edition["id"])}'
        + ('<span>Published edition</span></div></header>' if published
           else '<span>Private review candidate · Not published</span></div></header>'))
    if not published:
        add('<p class="preview-banner" role="note"><strong>Private preview.</strong> This candidate has not been '
            'approved for publication. Its content is awaiting manual review.</p>')

    add('<main id="main">')
    add('<section class="hero" aria-labelledby="title"><div class="intro">'
        + _hero(narrator)
        + '<nav class="hero-links" aria-label="Ways to read">'
        '<a class="explore" href="#chapter-1">Start the story</a>'
        '<a class="explore secondary" href="#atlas">Explore the whole tree</a>'
        '<a class="explore secondary" href="#all-steps">Read every step as text</a>'
        '</nav></div>')
    add('<aside class="edition-facts" aria-labelledby="facts-title"><h2 id="facts-title">This edition</h2><dl>'
        + (f'<dt>Narrator</dt><dd>{e(narrator["label"])}</dd>' if narrator else '')
        + f'<dt>Earliest prompt found</dt><dd>{e(display_date(first))}</dd>'
        f'<dt>Evidence cutoff</dt><dd>{e(edition["cutoff"]["label"])}</dd>'
        f'<dt>Development steps</dt><dd>{len(payload["events"])} steps on {len(payload["strands"])} strands, '
        f'{len(payload["relations"])} connections</dd>'
        f'<dt>Known gaps</dt><dd>{len(payload["gaps"])}, listed under <a href="#sources">Sources and gaps</a></dd>'
        '</dl><p class="facts-note">Reading time for the guided story: about seven minutes. '
        'Evidence panels are optional.</p></aside></section>')

    add('<div class="reading-controls" role="group" aria-label="Display options">'
        '<label><input type="checkbox" id="motion-off"> Motion off</label>'
        '<label><input type="checkbox" id="text-only"> Text only, no graph</label>'
        '<span class="reduced-note" id="reduced-note" hidden>Your system asks for reduced motion; animations stay off.</span>'
        '</div>')

    add('<section id="story" class="story" aria-label="The guided story"><div class="story-text">')
    for chapter in payload["chapters"]:
        add(f'<article class="chapter-section" id="chapter-{chapter["number"]}" data-chapter="{e(chapter["id"])}" '
            f'aria-labelledby="chapter-{chapter["number"]}-title">')
        add(f'<p class="kicker">Chapter {chapter["number"]} · {e(chapter["kicker"])}</p>')
        add(f'<h2 id="chapter-{chapter["number"]}-title">{e(chapter["title"])}</h2>')
        add(f'<p class="chapter-lede">{e(chapter["lede"])}</p>')
        for paragraph in chapter["body"]:
            add(f"<p>{e(paragraph)}</p>")
        add('<div class="event-cards">')
        for member in sorted(chapter["events"], key=lambda ident: events[ident]["layout"]["row"]):
            add(render_event_card(events[member], payload, strands, sources, titles, aside_label))
        add("</div></article>")
    add('</div><aside class="story-graph" aria-label="Growing ancestry graph">'
        '<div class="graph-heading"><h2>The ancestry blueprint</h2>'
        '<p class="graph-progress" id="graph-progress" aria-live="polite"></p></div>'
        '<div class="graph-stage" id="story-graph"><noscript><p>The graph needs JavaScript. Every step is '
        'also described in the text.</p></noscript></div>'
        '<div class="graph-controls"><button type="button" id="prev-chapter">Previous chapter</button>'
        '<button type="button" id="next-chapter">Next chapter</button>'
        '<button type="button" id="replay-growth">Replay growth</button></div>'
        '<ul class="graph-legend" aria-label="Line styles">'
        '<li><span class="line documented_cause"></span>Documented cause</li>'
        '<li><span class="line documented_association"></span>Documented association</li>'
        '<li><span class="line editorial"></span>Editorial link</li></ul>'
        '</aside></section>')

    add('<section id="atlas" class="atlas" aria-labelledby="atlas-title">'
        '<p class="kicker">Full complexity view · optional</p>'
        '<h2 id="atlas-title">Show me the whole beautiful mess.</h2>'
        '<p class="section-lede">Every step and connection of this edition at once. Search it, focus one strand, '
        'drag to pan and zoom with the buttons, the + and − keys, or Ctrl and the scroll wheel. Select a step to '
        'read its evidence. Nothing here goes beyond the story above; it is the same material, all at once.</p>'
        '<div class="atlas-toolbar">'
        '<div class="search"><label for="atlas-search">Search steps</label>'
        '<input id="atlas-search" type="search" autocomplete="off" spellcheck="false" '
        'placeholder="e.g. licence, schema, Paperclip"></div>'
        '<div class="zoom" role="group" aria-label="Zoom"><button type="button" id="zoom-in" aria-label="Zoom in">+</button>'
        '<button type="button" id="zoom-out" aria-label="Zoom out">−</button>'
        '<button type="button" id="zoom-reset">Fit</button></div></div>'
        '<div class="strand-chips" role="group" aria-label="Focus a strand">'
        '<button type="button" class="chip" data-strand="" aria-pressed="true">All strands</button>')
    for strand in payload["strands"]:
        add(f'<button type="button" class="chip accent-{e(strand["accent"])}" data-strand="{e(strand["id"])}" '
            f'aria-pressed="false">{e(strand["label"])}</button>')
    add('</div><p class="search-status" id="search-status" aria-live="polite"></p>'
        '<ul class="search-results" id="search-results"></ul>'
        '<div class="atlas-body"><div class="atlas-canvas" id="atlas-canvas" tabindex="0" role="group" '
        'aria-label="Complete development graph. Drag or use arrow keys to pan, plus and minus to zoom. '
        'Tab moves between steps; Enter opens a step.">'
        '<noscript><p>The interactive graph needs JavaScript. Use the step list below instead.</p></noscript></div>'
        '<aside class="inspector" id="inspector" aria-live="polite" aria-label="Selected step">'
        '<p class="inspector-empty">Select a step in the graph or a search result to see its details here.</p>'
        '</aside></div></section>')

    add('<section id="all-steps" class="all-steps" aria-labelledby="all-steps-title">'
        '<h2 id="all-steps-title">Every step, in order</h2>'
        '<p class="section-lede">The same history without any graph. Each entry links to its full description '
        'and evidence.</p><ol class="step-list">')
    for event in payload["events"]:
        add(f'<li data-event="{e(event["id"])}"><time>{e(display_date(event))}</time> '
            f'<a href="#ev-{e(event["id"])}">{e(event["title"])}</a> '
            f'<span class="step-strand">{e(strands[event["strand"]]["label"])} · '
            f'{e(STATUS_LABELS[event["status"]])}</span></li>')
    add("</ol></section>")

    add('<section id="sources" class="sources" aria-labelledby="sources-title">'
        '<h2 id="sources-title">Sources and gaps</h2>'
        f'<p class="section-lede">{e(edition["baseline_note"])}</p>'
        '<dl class="cutoff">'
        f'<dt>Evidence cutoff</dt><dd>{e(edition["cutoff"]["label"])}</dd>'
        f'<dt>Published history</dt><dd><code>{e(edition["cutoff"]["git_ref"])}</code> at '
        f'<code>{e(edition["cutoff"]["git_commit"][:12])}</code></dd></dl>'
        '<h3>What was read</h3><ul class="source-list">')
    for source in edition["sources"]:
        add(f'<li class="source-{e(source["status"])}"><span class="source-status">'
            f'{e(SOURCE_STATUS_LABELS[source["status"]])}</span> <strong>{e(source["label"])}</strong>. '
            f'{e(source["scope"])}.' + (f' <span class="source-note">{e(source["note"])}</span>' if source.get("note") else "")
            + "</li>")
    add('</ul><h3>What is missing</h3><ul class="gap-list">')
    for gap in payload["gaps"]:
        add(f'<li><strong>{e(gap["title"])}.</strong> {e(gap["detail"])}</li>')
    add('</ul><h3>How to read the labels</h3><ul class="label-guide">'
        '<li><strong>Translated from German</strong>: the original prompt was German; the English is a faithful translation.</li>'
        '<li><strong>Edited for privacy</strong>: private details were replaced by a bracketed description.</li>'
        '<li><strong>Shortened</strong>: parts were left out, marked with […].</li>'
        '<li><strong>Paraphrased</strong>: a summary of a message, not its wording.</li>'
        + _aside_guide(narrator)
        + '<li><strong>Documented cause</strong>, <strong>documented association</strong> and '
        '<strong>editorial link</strong>: how strongly the sources support a connection. Closeness in time alone '
        'is never treated as cause.</li>'
        '<li><strong>Verified (bounded)</strong>: checked in the stated, limited setting only.</li></ul></section>')
    add("</main>")
    add(f'<footer class="footer"><div>{"Static page" if published else "Offline preview"}. No analytics. '
        'No remote assets. No cookies; the motion '
        'and text-only preferences are kept in this browser\'s local storage.</div>'
        f'<div>Edition {e(edition["id"])} · content digest <code>{e(story_sha[:16])}</code> · '
        + ('approved for publication by the project owner' if published else 'not approved for publication')
        + '</div></footer>')
    add("</div>")
    return "\n".join(out)


class PublicationRefused(Exception):
    """The approval record does not match this exact candidate; nothing was written."""


def build(candidate_dir: Path, out_dir: Path, filename: str = "index.html",
          approval: Path | None = None) -> Path:
    """Verifies the candidate, then writes out_dir/filename; returns the file path.

    The output is a pure function of the candidate and the site sources (no timestamps),
    so a committed preview can be checked by rebuilding it. With ``approval`` the
    published variant is rendered (no preview banner, indexable), but only when the
    record matches this exact candidate; otherwise nothing is written.
    """
    manifest = verify_candidate(candidate_dir)
    published = approval is not None
    if published:
        gate = check_approval(candidate_dir, approval)
        if not gate.eligible:
            raise PublicationRefused("; ".join(gate.reasons))
    payload = json.loads((candidate_dir / "story.json").read_text(encoding="utf-8"))
    css = (SITE_DIR / "style.css").read_text(encoding="utf-8")
    script = (SITE_DIR / "app.js").read_text(encoding="utf-8")
    data = json_for_script(payload)
    policy = ("default-src 'none'; img-src data:; "
              f"style-src {csp_hash(css)}; script-src {csp_hash(script)}; "
              "base-uri 'none'; form-action 'none'")
    edition = payload["edition"]
    document = (
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n"
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        '<meta name="color-scheme" content="dark">\n'
        + ('' if published else '<meta name="robots" content="noindex, nofollow">\n')
        + f'<meta http-equiv="Content-Security-Policy" content="{e(policy)}">\n'
        + (f"<title>{e(edition['title'])} — KNXBench project evolution (edition {e(edition['id'])})</title>\n"
           if published else
           f"<title>{e(edition['title'])} — KNXBench project evolution (private preview {e(edition['id'])})</title>\n")
        + f"<style>{css}</style>\n</head>\n<body>\n"
        + render_body(payload, manifest["story_sha256"], published)
        + f'\n<script id="story-data" type="application/json">{data}</script>\n'
        + f"<script>{script}</script>\n</body>\n</html>\n"
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    target = out_dir / filename
    target.write_text(document, encoding="utf-8")
    return target
