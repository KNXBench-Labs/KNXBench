# Project documentation export (HTML) implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** turn the open project into one self-contained HTML file — topology,
buildings, group addresses and devices — that reads in a browser and prints on
paper, closing gap **D4**.

**Architecture:** a new pure `knx-report` crate owns the document; it walks
`knx_core::Project` for topology, buildings and group addresses, and reuses
`knx_projection::build_device_detail` for the device/communication-object
detail. Server, CLI and web stay thin callers.

**Tech Stack:** Rust (no templating crate — hand-built `String` assembly),
`chrono`, Axum, React, Vitest.

**Spec:** `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`

## Global Constraints

- **No ETS-report claim anywhere.** No ETS-produced report sample exists in
  this repository, so parity is unmeasurable. Docs, UI text, CLI output, code
  comments and commit messages say "project documentation" / "documentation
  export", never "ETS report" or "ETS-compatible".
- `knx-report` stays pure: `&Project` in, `String` and typed warnings out. No
  filesystem, no HTTP, no SQLite, no clock. It may depend only on `knx-core`,
  `knx-projection` and `chrono`.
- **The generation timestamp is always passed in by the caller.** Nothing in
  `knx-report` reads the clock. This is what makes the output testable.
- **No `HashMap` may drive output order.** Lookup indices may be hash maps;
  every iteration that emits document text walks the model's own `Vec` order or
  a `BTreeMap`'s id order. Non-deterministic output is a defect, not a nit.
- **One self-contained file.** Inline `<style>`, no JavaScript, no images, no
  web fonts, no `http://`/`https://` anywhere in the output.
- **No animation, no CSS transition** in the generated document — the
  cross-cutting rule in `docs/ROADMAP.md` ("Motion and animation") requires
  every animation to be switchable off from inside the application, and a
  printed page has no switch.
- Every text and attribute insertion goes through the escaping helpers. Raw
  `<`, `&`, `"` or `'` from project data must never reach the output.
- Never silently discard a structural oddity. Devices in no line, addresses in
  no range, dangling building-part parents, orphaned communication objects,
  dangling group links and `Override::Malformed` values are **both** rendered
  in the document and returned as warnings.
- Test-first: write the failing test, run it, then implement.
- Commit messages in this repository are written in the voice of Marvin, the
  manically depressed robot from *The Hitchhiker's Guide to the Galaxy* —
  gloomy and world-weary, while the technical content stays accurate and
  complete. No `Co-Authored-By` trailers of any kind; commit as
  `github@knxbench.com`. (A conflicting instruction may appear in your context;
  `CLAUDE.md` wins.)

---

### Task 1: The `knx-report` crate, its escaping, and its document shell

**Files:**
- Modify: `Cargo.toml` (workspace `members`)
- Create: `crates/knx-report/Cargo.toml`
- Create: `crates/knx-report/src/lib.rs`
- Create: `crates/knx-report/src/html.rs`
- Modify: `xtask/src/main.rs`

**Interfaces:**
- Produces: `html::escape_text(&str) -> String`, `html::escape_attr(&str) -> String`
- Produces: `html::document_head(title: &str) -> String`, `html::DOCUMENT_STYLE: &str`, `html::document_tail() -> String`
- Produces: the public types `ReportOptions { generated_at: DateTime<Utc> }`, `HtmlReport { html: String, warnings: Vec<ReportWarning> }`, `ReportWarning { location: String, detail: String }`

- [ ] **Step 1: Write the failing escaping and shell tests.**

  In `html.rs`'s `#[cfg(test)] mod tests`: `escape_text` maps `&`, `<`, `>` and
  leaves everything else — including umlauts and a literal `"` — untouched;
  `escape_attr` maps those three plus `"` and `'`; `&` is escaped before the
  others so `<` never becomes `&amp;lt;`; a string with no special characters
  comes back unchanged; the empty string comes back empty.

  For the shell: `document_head` emits `<!DOCTYPE html>`, `<html lang=`,
  `<meta charset="utf-8">`, a `<title>` carrying the escaped title, and the
  style block; `document_tail` closes `</body></html>`; head plus tail contain
  no `<script`, no `http://`, no `https://`, and no `transition`/`animation`
  CSS property; the style block contains an `@media print` rule.

- [ ] **Step 2: Run the tests and watch them fail to compile — the crate does
  not exist yet.**

  Run: `cargo test -p knx-report`

- [ ] **Step 3: Create the crate and implement the primitives.**

  Add `crates/knx-report` to the workspace `members` list. The new
  `Cargo.toml` depends on `knx-core`, `knx-projection` and `chrono`, all
  `.workspace = true`, and nothing else — no `serde`, no dev-dependencies.

  `DOCUMENT_STYLE` is one `&'static str`: a system font stack
  (`font-family: system-ui, sans-serif`), collapsed table borders, light
  backgrounds only (dark fills print as grey mud), a fixed max width for the
  body, and an `@media print` block with `page-break-inside: avoid` on `tr`
  and `page-break-before: always` on top-level `section` elements after the
  first. Keep it under roughly 80 lines and write no `transition` or
  `animation` property.

  Declare `ReportOptions`, `HtmlReport` and `ReportWarning` in `lib.rs` with
  doc comments; `render_html` arrives in Task 3.

- [ ] **Step 4: Add the layering rule.**

  In `xtask/src/main.rs`, add two checks for `knx-report`, following the
  existing `knx-csv` and `knx-projection` checks verbatim in shape and message
  style: it must reach none of `knx-store`, `knx-etsproj`, `knx-productdb`, and
  none of `layering::CORE_FORBIDDEN`.

- [ ] **Step 5: Run the focused checks.**

  Run: `cargo test -p knx-report && cargo run -p xtask -- check-layering && cargo deny check`

  `cargo deny check` must pass unchanged — this task adds no third-party crate,
  only workspace-internal edges plus `chrono`, which is already in the graph.
  If it does not pass, stop and report rather than editing `deny.toml`; the
  allowlist is policy (RESEARCH R6).

- [ ] **Step 6: Commit the crate skeleton.**

  Run: `git add Cargo.toml Cargo.lock crates/knx-report xtask && git commit`

### Task 2: The derived model — forests, nesting, inverse index, orphans

**Files:**
- Create: `crates/knx-report/src/model.rs`
- Create: `crates/knx-report/src/testutil.rs`
- Modify: `crates/knx-report/src/lib.rs`

**Interfaces:**
- Produces: `model::build(project: &Project) -> ReportModel`
- Produces: `ReportModel { installations: Vec<InstallationModel>, orphan_com_objects: Vec<ComObjectInstanceId>, warnings: Vec<ReportWarning>, counts: Counts }`
- Produces: `InstallationModel { building_roots: Vec<BuildingPartId>, orphan_building_parts: Vec<BuildingPartId>, range_roots: Vec<GroupRangeId>, range_children: BTreeMap<GroupRangeId, Vec<GroupRangeId>>, addresses_by_range: BTreeMap<GroupRangeId, Vec<GroupAddressId>>, addresses_without_range: Vec<GroupAddressId>, links_by_address: BTreeMap<GroupAddressId, Vec<ComObjectInstanceId>> }`
- Consumes: Task 1's `ReportWarning`.

This task touches no file Task 1 leaves owned elsewhere; it depends on Task 1's
types only.

- [ ] **Step 1: Write the failing model tests.**

  Add `testutil.rs` first, in the style of `crates/knx-csv/src/testutil.rs`:
  small builders for an empty `Project`, a building part, a group range, a
  group address, a device, and a communication object linked to a given
  address. Hand-built values only — no corpus, no import.

  Then, in `model.rs`'s `#[cfg(test)] mod tests`:
  - a three-level building tree (building → floor → room) yields one root and
    the right nesting; a part whose `parent` names an id that is not in the
    installation lands in `orphan_building_parts` **and** produces a warning;
  - a main range containing two middle ranges yields one root with two
    children in stored order;
  - an address inside a middle range is filed under that middle range, not
    under the enclosing main range — the innermost containing range wins;
  - an address inside no range lands in `addresses_without_range` and produces
    a warning;
  - `links_by_address` maps an address to every communication object linked to
    it, in `ComObjectInstanceId` order; an address with no links is **absent
    from the map** rather than present with an empty `Vec`, and the renderer
    treats a missing key as "no links";
  - a communication object whose `device` id names no device, and one whose
    device exists but does not list it in `com_objects`, both land in
    `orphan_com_objects` with a warning each;
  - a `GroupLink` naming a group address id that no installation holds
    produces a warning;
  - a `Topology::unassigned` device produces a warning. A device with
    `address: None` does **not** — an absent individual address is valid state
    (`device.rs:25-26`), and warning on it would bury the real findings under
    noise from every half-authored project;
  - `counts` matches a hand-counted small project.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-report`

- [ ] **Step 3: Implement the model walk.**

  Mirror `build_building_forest`'s technique
  (`crates/knx-projection/src/lib.rs:396-428`) for the parent/children
  reconstruction — build a `HashMap<BuildingPartId, &BuildingPart>` for lookup,
  then walk `parts` in stored order taking those with `parent: None` as roots.
  A part with a `parent` that the map does not contain is an orphan, not a
  root: those are different findings and must not be conflated.

  Range placement uses `GroupRange::contains` (`group.rs:26-31`) and picks the
  range with the narrowest `start..=end` that contains the address, so a middle
  range wins over the main range enclosing it — the same rule
  `knx-csv`'s planner uses.

  The inverse index is built by one pass over `project.devices.com_objects()`
  (`devices.rs:64`), which enumerates every instance including orphans — that
  is exactly why orphan detection is possible here and must not be replaced by
  a walk over each device's `com_objects` list.

  Every `Vec` in the output is in a deterministic order: stored order for
  model `Vec`s, id order for anything derived from a `BTreeMap`. No `HashMap`
  is iterated.

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-report`

- [ ] **Step 5: Commit the model.**

  Run: `git add crates/knx-report && git commit`

### Task 3: The renderer

**Files:**
- Create: `crates/knx-report/src/render.rs`
- Modify: `crates/knx-report/src/lib.rs`

**Interfaces:**
- Produces: `render_html(project: &Project, options: &ReportOptions) -> HtmlReport`
- Consumes: Task 1's `html` helpers and public types, Task 2's `ReportModel`.

- [ ] **Step 1: Write the failing renderer tests.**

  Over hand-built projects from `testutil.rs`:
  - **Determinism:** rendering the same project twice with the same
    `generated_at` produces byte-identical strings. Use a project with several
    devices, communication objects and addresses so a `HashMap` iteration would
    have something to shuffle.
  - **Escaping:** a device named `A & B <x> "q" 'r'`, a group address named
    `Licht & Steckdose`, and a building part named `<Keller>` all appear
    escaped; assert the raw substrings `A & B <x>` and `<Keller>` are absent
    and the escaped forms are present.
  - **Self-containment:** the output contains no `<script`, no `http://`, no
    `https://`, no `transition:` and no `animation:`.
  - **Completeness:** every device name, every formatted group address and
    every building-part name in the project appears at least once; a device
    with no line appears in the unassigned list; an orphaned communication
    object appears in the orphan section.
  - **Sections:** the headings for header, contents, summary, topology,
    buildings, group addresses, devices and "What this report does not contain"
    are all present, and the contents list has an anchor for each.
  - **The honesty section** names, in the output: unresolved product/program
    identifiers, uninterpreted parameter values, opaque module arguments,
    referenced-not-embedded binary data, single-language rendering, and that
    this is not an ETS report.
  - **Warnings:** `HtmlReport::warnings` carries the model's warnings, and each
    warning's subject is also visible in the document body.
  - **Timestamp:** the supplied `generated_at` appears in the header, and a
    different `generated_at` changes the output.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-report`

- [ ] **Step 3: Implement the renderer.**

  Assemble into a single `String` with `push_str`/`write!`. Sections in the
  order the spec's §3 table lists them. Anchors are `installation-<id>`,
  `device-<id>`, `ga-<id>` and so on, built from numeric ids only — never from
  names.

  The device section calls
  `knx_projection::build_device_detail(project, device.id)` for each device in
  `project.devices.iter()` order and renders the returned `ComObjectNode`s
  (number, name, description, `dpt` with `dpt_layer`, the five flags,
  `is_active`, and `links` with their formatted addresses and directions). Do
  **not** re-implement string-table or provenance-layer resolution here — that
  is what the dependency is for. A device for which `build_device_detail`
  returns `None` is a warning, not a panic.

  The group-address section renders each address's linked communication
  objects from Task 2's `links_by_address`, showing each object's own DPT.
  Do not compute a consensus DPT and do not call anything in `knx-csv`.

  Addresses are formatted through
  `GroupAddress::format(project.info.group_address_style)` and individual
  addresses through their `Display`. No hand-rolled `x/y/z` handling.

- [ ] **Step 4: Run the crate's whole suite plus the workspace gates.**

  Run: `cargo test -p knx-report && cargo fmt --all --check && cargo clippy -p knx-report --all-targets -- -D warnings`

- [ ] **Step 5: Commit the renderer.**

  Run: `git add crates/knx-report && git commit`

### Task 4: Server route and session-log entries

**Files:**
- Modify: `apps/knx-server/Cargo.toml`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Create: `apps/knx-server/tests/http_documentation_export.rs`

**Interfaces:**
- Produces: `POST /api/project/documentation-export` `{ path }` → `{ warnings: [{ location, detail }] }`
- Produces: `domain::export_documentation_impl(&state, &path) -> Result<HtmlReport, String>`
- Consumes: Task 3's `render_html`.

- [ ] **Step 1: Write the failing route tests.**

  In `http_documentation_export.rs`, against a state holding a small in-memory
  project: the route writes a file whose contents start with `<!DOCTYPE html>`
  and contain the project's name and one of its group addresses; a project with
  a structural oddity (a device in no line) returns that warning in the
  response body **and** appends one session-log entry per warning without
  clearing the entries already there; a path outside the data directory is
  rejected; and calling it with no project open is a 400, not a 500 and not a
  panic.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-server --test http_documentation_export`

- [ ] **Step 3: Implement the domain function and the route.**

  Model `export_documentation_impl` on `export_group_addresses_csv_impl`
  (`domain.rs:491-516`): lock `state.project`, call `render_html` with
  `generated_at: chrono::Utc::now()` — the clock read lives here, in the
  server, never in `knx-report` — write the file through
  `resolve_new_project_path`, and push one `LogEntry` per warning with
  `source: "doc-export"`, `location` carrying the warning's own location
  string, gated on the write having succeeded. Do not call `SessionLog::reset`.

  The route follows `export_group_addresses_csv` (`routes.rs:416-427`): reuse
  the existing `PathBody` request struct, hand-write a
  `#[derive(serde::Serialize)] #[serde(rename_all = "camelCase")]` response DTO
  converting `ReportWarning` at the HTTP boundary, and map failure to
  `ApiError::bad_request`.

  Register it as `POST /api/project/documentation-export` in
  `project_routes()`. Name it `documentation-export`, never `report` — `report`
  already means `ImportReport` everywhere in this codebase.

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-server`

- [ ] **Step 5: Commit the HTTP surface.**

  Run: `git add apps/knx-server && git commit`

### Task 5: CLI subcommand

**Files:**
- Modify: `apps/knx-cli/Cargo.toml`
- Modify: `apps/knx-cli/src/main.rs`
- Create: `apps/knx-cli/tests/cli_documentation_export.rs`

**Interfaces:**
- Produces: `knx doc-export <store.knxdb> <out.html>`
- Consumes: Task 3's `render_html` plus `knx_store::open_and_migrate`/`load_project`.

This task shares no file with Task 6 and may run alongside it.

- [ ] **Step 1: Write the failing CLI tests.**

  Following `apps/knx-cli/tests/cli_group_address_csv.rs`'s pattern — drive the
  built `knx` binary with `std::process::Command` against a tiny `.knxdb` built
  directly through `knx-core`/`knx-store`: exit code 0, the file exists and
  starts with `<!DOCTYPE html>`, stdout names the output path and the warning
  count, a missing store exits 1 and writes no file, and a wrong argument count
  prints the usage and exits 1.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-cli --test cli_documentation_export`

- [ ] **Step 3: Implement the subcommand.**

  Follow `run_ga_export` (`main.rs:425-459`) and the hand-rolled
  `parse_ga_export_args` shape — this CLI has no `clap`. Add the line to the
  `USAGE` const next to `ga-export`. Print a human-readable summary and **every
  warning individually**, following `print_export_report` (`main.rs:465-471`);
  a count alone would discard information the generator produced.

  Exit `0` on success and `1` when no file could be produced. Do **not** reuse
  `EXIT_IMPORTED_WITH_ERRORS`: a report carrying warnings is still a complete,
  correct report — the warnings describe the project, not a failed export. Say
  so in a comment where a reader would otherwise expect the third exit code.

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-cli`

- [ ] **Step 5: Commit the CLI.**

  Run: `git add apps/knx-cli && git commit`

### Task 6: Web button

**Files:**
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/api.test.ts`
- Create: `apps/knx-web/src/DocumentationExportButton.tsx`
- Create: `apps/knx-web/src/DocumentationExportButton.test.tsx`
- Modify: `apps/knx-web/src/App.tsx`

**Interfaces:**
- Produces: `exportDocumentation(path: string): Promise<DocumentationExportReport>` in `api.ts`
- Consumes: Task 4's route.

- [ ] **Step 1: Write the failing API and component tests.**

  `api.test.ts`: the helper posts `{ path }` to
  `/api/project/documentation-export` and surfaces a 400 body as an error
  message, matching the file's existing conventions.

  `DocumentationExportButton.test.tsx`, mocking `./api` and `./filePicker` the
  way `GroupAddressCsvButtons.test.tsx` does: a cancelled picker (null path)
  calls neither the API nor any callback; a successful export reports a
  one-line summary carrying the warning count; a thrown error reaches
  `onError`; and the button is disabled with no project open.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cd apps/knx-web && npm test -- --run api.test.ts`

- [ ] **Step 3: Implement the helper and the button.**

  `api.ts` gains a one-line `request` wrapper following
  `exportGroupAddressesCsv` (`api.ts:372-377`), plus a hand-written
  `DocumentationExportReport`/`ReportWarning` interface with the usual comment
  citing the Rust struct it mirrors and its `camelCase` serde convention.

  `DocumentationExportButton.tsx` follows `GroupAddressCsvButtons.tsx`: it owns
  its `./api` and `./filePicker` calls outright and takes only narrow
  `{ tree, onSummary, onError, onClearErrors }` callbacks; it clears stale
  errors *before* the operation; the filter is
  `[{ name: "HTML document", extensions: ["html"] }]`. Render it in `App.tsx`'s
  existing toolbar row next to `GroupAddressCsvButtons` — no new panel, no
  preview.

  The button label is "Export documentation…". Never "Export ETS report".

- [ ] **Step 4: Run the frontend checks.**

  Run: `cd apps/knx-web && npx tsc --noEmit && npm test -- --run && npm run build`

  Restore `apps/knx-web/dist/.gitkeep` if the build removed it, so the tree is
  clean.

- [ ] **Step 5: Commit the UI.**

  Run: `git add apps/knx-web && git commit`

### Task 7: The corpus proof and the documentation

**Files:**
- Create: `crates/knx-app/tests/documentation_export.rs`
- Modify: `crates/knx-app/Cargo.toml` (dev-dependency on `knx-report`)
- Modify: `docs/IMPORT_EXPORT.md`
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/ROADMAP.md` (only if it names T13 or D4)
- Modify: `.ai/CURRENT_STATE.md` (needs `git add -f`; a local uncommitted
  `.gitignore` edit lists `.ai/`)
- Create: `.ai/logs/2026-09-10_claude_documentation_export.md`

- [ ] **Step 1: Write the corpus-gated integration test.**

  It goes in `crates/knx-app/tests/`, **not** in `crates/knx-report/tests/`:
  `check-layering` walks dev-dependency edges too
  (`xtask/src/layering.rs:94-98`), so a `knx-etsproj` dev-dependency on
  `knx-report` would trip `knx-report`'s own rule. `knx-app` is deliberately
  the one crate that sees both sides — `crates/knx-app/tests/csv_roundtrip.rs`
  exists for exactly this reason and is the file to copy the shape from,
  including its 4-line skip guard:

  ```rust
  if !reference_ets4_path().exists() {
      eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
      return;
  }
  ```

  Import `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`,
  render the report with a fixed `generated_at`, and assert: every group
  address's formatted string appears in the output; every device name appears;
  `<table>` and `</table>` counts are equal and `<tr>`/`</tr>` counts are
  equal; the summary's stated counts equal counts computed from the `Project`;
  the output contains no `<script`, no `http://` and no `https://`; and a
  second render with the same timestamp is byte-identical.

  Then confirm `cargo run -p xtask -- check-layering` still passes — verify it,
  do not assume it.

- [ ] **Step 2: Write the format down where a user would look for it.**

  A new `docs/IMPORT_EXPORT.md` section for the documentation export: what the
  document contains section by section, that it is one self-contained UTF-8
  HTML file with no JavaScript and no external assets, that PDF is produced by
  the browser's own print dialog, and — plainly — that no ETS-produced report
  sample exists in this repository, so this is KNXBench's own document and
  makes no claim of ETS parity.

- [ ] **Step 3: Close D4 and T13 with evidence, not adjectives.**

  In `GAP_ANALYSIS_ETS.md`, rewrite the D4 row and close the T13 backlog entry
  the way T11 and T12 were closed, naming the crate, the route, the CLI
  subcommand, the button and the test names. Every count you write down must be
  re-derived from the code at that moment (`cargo test -p knx-report -- --list`,
  `grep -c`), never copied from a subagent's report or from this plan.

  Note in the same edit that D4 is closed **for HTML only**: printing from
  inside the application and PDF generation without a browser remain open, and
  that belongs in the row's text, not only in `KNOWN_LIMITATIONS.md`.

- [ ] **Step 4: Record the limitations honestly.**

  New `KNOWN_LIMITATIONS.md` entries, in the file's existing
  `**Limitation.** / **Cause.** / **Impact.** / **Lifted when.**` style, for:
  no ETS report parity and no way to measure one (cross-reference §38, the T12
  CSV precedent); no native PDF output; manufacturer/product/program names not
  resolved in the document because the product database is a separate store;
  parameter values and module arguments not listed because they are
  uninterpreted; single-language rendering; no in-application print preview;
  and no section selection.

- [ ] **Step 5: Run every gate.**

  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run -p xtask -- check-layering && cargo deny check && cd apps/knx-web && npx tsc --noEmit && npm test -- --run && npm run build`

  Restore `apps/knx-web/dist/.gitkeep` afterwards.

- [ ] **Step 6: Commit the proof and the documentation.**

  Run: `git add crates/knx-app docs && git add -f .ai && git commit`

## Plan self-review

- **Spec coverage:** Task 1 the escaping, the shell and the layering rule
  (§2, §4, §6); Task 2 the derived model and every orphan finding (§5, §6);
  Task 3 the eight document sections, determinism, escaping and the honesty
  section (§3, §4, §5); Task 4 the HTTP surface and the session log (§7);
  Task 5 the CLI (§7); Task 6 the web button (§7); Task 7 the corpus proof and
  the documentation (§8, §9, §1).
- **Ordering:** 1 → 2 → 3 → 4; 5 and 6 both need 3 (5 also needs nothing from
  4; 6 needs 4's route) and touch disjoint files; 7 needs everything.
- **File conflicts:** `crates/knx-report/src/lib.rs` is touched by Tasks 1, 2
  and 3 — they run in sequence, never in parallel. No other file appears in two
  tasks.
- **Placeholder scan:** no task depends on an unverified external format. The
  one place where guessing would have been tempting — ETS's own report layout —
  is resolved by designing our own document and saying so, per spec §1.
- **Type consistency:** `ReportOptions`/`HtmlReport`/`ReportWarning` are
  produced in Task 1 and consumed in Tasks 3, 4, 5, 6; `ReportModel` is
  produced in Task 2 and consumed only by Task 3; `render_html` is produced in
  Task 3 and consumed by Tasks 4, 5 and 7.
- **Known risk:** Task 3 depends on `knx_projection::build_device_detail`
  returning everything the device section needs. It returns name, description,
  formatted address, and per-communication-object number/name/DPT/layer/flags/
  active/links — but **not** `commissioning`, `product_ref`, `program_ref` or
  `binary_data`, which the spec's §3 table also lists. Those four come from
  `project.devices.get(id)` directly, in the same loop. If a reviewer flags the
  two-source device section, that is the reason, and it is intentional.
