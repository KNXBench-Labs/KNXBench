# AR15 slice 1: source-ID dispositions (Claude, 2026-10-06)

## KL-9 (SQLite not diffable)
Synthetic repo in scratch: fixture `zipcrypto-minimal.knxproj` imported,
`knx ga-import` renamed 0/0/1 and added 0/0/2, two commits. Plain `git diff`:
"Binary files differ". External driver `knx diff "$2" "$5"`: entity lines.
`git show`/`git log -p` need `--ext-diff`; Git passes `/dev/null` for
added/removed files and `knx diff` refuses it ("unsupported comparison input
format"), aborting the log — documented script guards both. `sqlite3 .dump`
textconv works (raw rows). `knx diff` on current-schema files leaves both
byte-identical (sha256 before/after).

## KL-16 (GTK3)
Cargo.lock: tauri 2.11.5, gtk 0.18.2, webkit2gtk 2.0.2, wry 0.55.1; crates.io
stable tauri 2.12.1, max 3.0.0-alpha.4. `cargo deny --offline check
advisories` (DB ef6173c, 2026-10-03): advisories ok. GitHub API 2026-10-06:
wry#1767 and tauri#14684 open.

## KL-46 (report names)
Behaviour unchanged; named tests green in the 2026-10-06 workspace run.

## DOC-03 (stale manual)
Commissioning claims contradicted KL §7 and manual 07 (download verified on
one device; address programming refused before a tunnel, ADR-0058, read in
`apps/knx-server/src/address_programming_routes.rs`). Fixed in README,
getting-started 01/03, user-guide 06 §12 and 10, FAQ, known issues,
reference/02, implementation-status, ideas-and-roadmap. Workflow §12 told
users to export a `.knxproj` that does not exist (ADR-0028). Roadmap
suggestions: New-project hint and browser project export are resolved,
`.vd2` remains only in CLI help. Counts checked: session log 1,000
(`session_log.rs` MAX_ENTRIES), diff filter 20 (`DIFF_FILTER_THRESHOLD`).
