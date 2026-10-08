# Claude Code cloud sessions

> **Status 2026-09-28: paused.** The user decided to run the remaining work
> locally again. Delivered through the cloud:
>
> - CT-1 (`313489e`);
> - CT-2 (`d9ff0db`);
> - CT-6 (`826466a`).
>
> The briefs CT-3, CT-5 and CT-7 to CT-10 below remain valid as task
> specifications for local work. Only their cloud mechanics do not apply:
> draft PR, `claude-cloud` log name, and the root and WebKit workarounds.
>
> The environment, the setup script and the SessionStart hook stay in place,
> dormant. The hook exits immediately outside a cloud session. Resuming only
> needs a new session in the existing claude.ai environment. The credit
> (243 $ left) expires on 2026-11-04.

KNXBench can hand self-contained work to Claude Code **cloud sessions**. These
are Anthropic-hosted VMs that clone this GitHub repository, work on their own
branch and open a pull request. This document covers:

- what such a session can and cannot do;
- how the environment is configured;
- the task briefs a session receives.

Session rules for the agent itself are in
[`tools/cloud/SESSION_RULES.md`](../../tools/cloud/SESSION_RULES.md). The
SessionStart hook prints them into every cloud session.

Facts about the platform were taken from the Claude Code documentation
(`code.claude.com/docs/en/claude-code-on-the-web`, `…/cloud-environments`,
`…/settings-reference`) on 2026-09-28. Re-check them there if behaviour
differs.

## 1. Boundaries

| Available in a cloud session | Not available |
| --- | --- |
| The committed repository (fresh clone of the selected branch) | Uncommitted local work, local worktrees |
| Rust (toolchain installed by the setup script), Node 22, Docker | `OriginalData/`: the private corpus is gitignored and must never be uploaded |
| crates.io, npm, Ubuntu archive, GitHub (Trusted network) | KNX bus, LAN, KNXnet/IP gateway |
| 4 vCPU, 16 GB RAM, 30 GB disk (approximate) | Local Claude/Hermes memories, `~/.claude` settings |

Consequences:

- **Corpus-backed work cannot be finished in the cloud.** Corpus tests report
  as *ignored* there (§131 made this explicit). Anything whose acceptance
  needs the product-database matrix or the `.knxproj` oracle (the PDB
  packages, corpus regressions) stays local, or gets its corpus gate run
  locally before merge.
- **Commissioning and live-bus work stays local.**
- **Integration stays local.** Cloud sessions open draft pull requests. The
  local session reviews each one, runs the corpus gates, merges and updates
  `.ai/CURRENT_STATE.md`.
- **Parallelism.** `goal.md` §9/§12.3 allows at most two implementers and one
  serial `apps/knx-web` chain. Treat a cloud session as one of those
  implementers: never run two web tasks at once.

## 2. One-time configuration

### 2.1 Repository side (versioned)

- [`.claude/settings.json`](../../.claude/settings.json) turns off every Claude
  attribution: `attribution.commit` and `attribution.pr` are empty, and
  `attribution.sessionUrl` is `false`. Without this, cloud commits would carry
  `Co-Authored-By` and `Claude-Session` trailers, which `AGENTS.md` forbids.
  The empty-string form is used instead of `"attribution": false` because older
  Claude Code versions reject `false` and then skip the whole file.
- The same file registers the SessionStart hook
  [`tools/cloud/session-start.sh`](../../tools/cloud/session-start.sh). Locally it
  exits at once: it only acts when `CLAUDE_CODE_REMOTE=true`. In the cloud it
  does four things:
  - sets the git identity `KNXBench <github@knxbench.com>`;
  - puts cargo on `PATH`;
  - runs `npm ci` for `apps/knx-web`;
  - prints the session rules plus an environment report.
- `.gitignore` now ignores `.claude/*` except `.claude/settings.json`, so
  `settings.local.json` and other local state stay private.

### 2.2 Environment side (claude.ai/code, not versioned there)

Create one cloud environment named **KNXBench**:

- **Network access:** Trusted.
- **Environment variables:**

  ```text
  BASH_DEFAULT_TIMEOUT_MS=600000
  BASH_MAX_TIMEOUT_MS=1800000
  ```

  These give workspace builds and tests enough time before the Bash tool moves
  them to the background.
- **Setup script:** the full content of
  [`tools/cloud/setup-env.sh`](../../tools/cloud/setup-env.sh). It installs the
  Tauri/WebKit dev packages that CI also installs, plus the pinned Rust
  toolchain. The environment snapshot is cached only if setup finishes in about
  five minutes. The script therefore does not build anything.

Verified locally in a `ubuntu:24.04` container running as root (cloud-VM
stand-in; the real VM also has Node, Rust and more pre-installed). The results:

- the setup script exits 0 within the time budget;
- the hook sets the identity, writes `PATH` to `CLAUDE_ENV_FILE` and reports
  the toolchain;
- `cargo check -p knx-desktop` succeeds with the installed packages.

This is not proof that the real VM behaves identically. The first real session
must be checked against the environment report (section 3.1).

## 3. Operating procedure

1. Start sessions from **claude.ai/code**:
   - repository `KNXBench-Labs/KNXBench`;
   - branch `main`;
   - environment **KNXBench**;
   - mode **Accept edits** (or **Auto** where offered).
2. Paste the task prompt from section 4.
3. The session ends with a draft pull request titled `CT-n: …`.
4. The local session then fetches the branch, reviews it, runs the full gates
   including the corpus matrix where relevant, and merges.

   Cloud results are self-reports. Do not trust them until they have been
   verified locally.

`claude --cloud "…"` from a terminal also works, but it clones *the current
local branch's* GitHub remote. If the Claude GitHub App is missing, it uploads
a bundle of the local repository **including uncommitted changes to tracked
files**. Only use it from a clean checkout of `main`.

### 3.1 First-session check

The first session of a new environment should show this environment report:

- `rustc 1.98.0`;
- `webkit2gtk-4.1: present`;
- `OriginalData/: absent`;
- git identity `KNXBench <github@knxbench.com>`.

Also check that the pull request's commits carry no `Co-Authored-By` or
`Claude-Session` trailer. If either check fails, stop and fix the environment
before spending more credit.

**Self-repair (added 2026-09-28 after the first real session).**

*Observed.* The first real cloud session reported:

- rustc 1.98.0 and Node 22;
- `webkit2gtk-4.1: MISSING`.

Rust and Node are pre-installed on the VM, so that report cannot tell whether
the environment's setup script ran at all.

*Unknown.* Whether the setup script ran and failed, or never ran. The session
had no exit code.

*Change.* Two changes make this visible and repairable:

- `setup-env.sh` now writes `/var/tmp/knxbench-cloud/setup.status` with the
  time, the caller, the uid and the apt/rust outcome. It also keeps the full
  apt log next to that file.
- If WebKit is missing, the SessionStart hook runs the same script itself.
  This works when the hook is root or has passwordless sudo, and takes about
  70 s in the local probe.

The report now carries two extra lines:

- `environment setup: …` either says `NOT RUN by the environment` or names who
  ran the script and when;
- `setup fallback in this hook: …` says whether the fallback ran and where its
  log is.

`run_by=session-start-fallback` means the environment's own setup script did
not do the job. In that case re-check the environment dialog.

*Remaining limit.* If the hook is neither root nor passwordless sudo, it says
`IMPOSSIBLE` and changes nothing. Then only the environment's setup script can
install WebKit. For `apps/knx-web`-only work (CT-1, CT-2) a session may gate
with `--exclude knx-desktop` on **both** `cargo test` and `cargo clippy`, but
it must say so in the PR. The local merge gate then covers `knx-desktop`.

## 4. Task briefs

Each prompt is deliberately short. The brief below is the specification, and
the session reads it from this file.

Prompt template (replace `CT-n`):

```text
Task CT-n from docs/CLOUD_SESSIONS.md. Read AGENTS.md, tools/cloud/SESSION_RULES.md
and the CT-n brief first, then carry it out completely: implementation, tests, gates,
documentation, log, draft PR. If the environment report printed at session start
shows something MISSING, stop and report it before working.
```

### CT-1 — Project-diff web panel shows entities and values (KNOWN_LIMITATIONS §59, §60)

- **Scope:** `apps/knx-web` only.
- **Context:**
  - The HTTP diff response already carries the needed data:
    - entity keys and match kind;
    - ordered `fieldChanges` (`field`, `left`, `right`);
    - typed `left`/`right` snapshots, nested object/parameter tables and
      ambiguity counts.
  - `ProjectDiffPanel.tsx` still renders grouped counts only.
  - Read §59/§60 including the "T15 handoff" paragraph, and the design spec
    `docs/superpowers/specs/2026-09-10-project-diff-design.md`.
- **Deliver:**
  - An expandable per-table list of added, removed, changed and ambiguous
    entities, identified by their natural key.
  - Each changed entity lists its `fieldChanges` as field / before / after.
  - Keep the existing grouped counts as the summary line.
  - Must be keyboard accessible and readable by screen readers. It must not use
    colour alone for added/removed.
  - English and German messages.
  - Large diffs stay usable: collapsed by default, and no rendering of
    thousands of rows at once without a limit plus a "show more" control.
- **Tests:** Vitest for rendering, for expand/collapse via the keyboard, for
  German output, and for a large synthetic diff.
- **Docs:** Update §59/§60 honestly (lifted, or what remains), and
  IMPLEMENTATION_STATUS.
- **Not in scope:**
  - applying a diff (§55);
  - three-way compare (§56);
  - raw `.knxproj` in the web route (§57).

### CT-2 — Documentation export: preview and section selection in the web UI (§49, §50)

- **Scope:** `apps/knx-web` only. Start only after CT-1 is merged, because the
  web chain runs serially.
- **Context:**
  - `POST /api/project/documentation-preview` returns self-contained HTML with
    an embedded print stylesheet, plus warning DTOs.
  - Preview and export accept `sections` (`summary`, `topology`, `buildings`,
    `groupAddresses`, `devices`). Header, contents and limits are always
    present.
  - The current frontend entry point is `DocumentationExportButton.tsx`.
- **Deliver:**
  - A section selector that sends the same selection to preview and export.
  - A preview in a **sandboxed** iframe (no script execution, `srcdoc`).
  - Warnings shown next to the preview.
  - A Print action that opens the browser print dialog for the preview
    document.
  - English and German.
- **Tests:** Vitest:
  - the selection is sent identically to both endpoints;
  - the sandbox attributes are set;
  - warnings are rendered;
  - print is invoked;
  - preview errors are shown.
- **Docs:** Update §49/§50 and IMPLEMENTATION_STATUS.

### CT-3 — Fuzz the byte and text parsers (hardening, no corpus)

- **Scope:**
  - A new `fuzz/` cargo-fuzz crate outside the main workspace, with its own
    `[workspace]` table or listed in the workspace's `exclude`.
  - Fixes in the owning crates.
- **Context:** Several public parsers are exposed to untrusted input, e.g. in
  `knx-net`:
  - `frame::decode_frame`
  - `cemi::decode_l_data`
  - `core::dib::decode_device_info` and `decode_service_families`
  - `core::services::decode_*`
  - `tunnelling::decode_*`
  - `discovery::decode_search_response`
  - `routing::decode_*`

  Also `knx-core::dpt::codec::decode`, `knx-csv::read::parse_group_addresses`,
  the `knx-etsproj::values::parse_*` helpers, and any public
  byte/string entry point for archives or XML you find. Use public APIs only.
  Do not widen visibility just for fuzzing.
- **Setup:** Install a nightly toolchain and `cargo install cargo-fuzz` inside
  the session. Both come from allowed hosts.
- **Run:** Each target with `-max_total_time=600` (raise it for productive
  targets), about two hours in total. Seeds must be synthetic only, never
  corpus-derived.
- **Findings:**
  - A panic, abort, OOM, hang or a single input taking more than 1 s is a
    finding.
  - A returned `Err` is not a finding.
  - For each finding:
    1. Add a regression unit test in the owning crate with the minimized input
       inlined, so it runs in the normal `cargo test` without the fuzzer.
    2. Fix the bug in a separate focused commit.
  - Record anything deliberately not fixed in KNOWN_LIMITATIONS.
- **Docs:** A short section in `docs/IMPLEMENTATION_STATUS.md`, listing
  targets, run time per target and findings. Also add a pointer in
  `docs/ARCHITECTURE.md` or RESEARCH if a parser contract changed.
- **Gates:** The normal gates, plus `cargo deny check` if the root lock file
  changes. It should not change.

### CT-4 — Independent branch review (read-only)

Prompt, replacing `<BRANCH>` and `<BASE>` (usually `main`):

```text
Task CT-4 from docs/CLOUD_SESSIONS.md for branch <BRANCH> against <BASE>.
```

- **Do:**
  - Check out `<BRANCH>` and review `git diff <BASE>...<BRANCH>` in full.
  - Priorities are correctness, data integrity, migration safety, error
    handling, test adequacy (do the tests fail without the fix?), and
    documentation honesty (no overclaims, no ETS parity claims).
  - Run the gates. Corpus gates will show as ignored; say so.
  - Classify each finding as CRITICAL / IMPORTANT / MINOR, with `path:line`,
    reasoning and a suggested fix. Separate verified findings (reproduced) from
    suspected ones.
- **Do not** change product code.
- **Output:**
  - Commit `.ai/logs/<date>_claude-cloud_review-<branch>.md` (with
    `git add -f`) on a new branch `cloud-review/<branch>`.
  - Push it. No PR is needed.
  - End the session with the verdict line.

### CT-5 — Documentation hygiene (goal.md §8.1, §8.2, §8.5; ISSUE-04 plan checkboxes)

- **Scope:** Documentation only.
- **Deliver:**
  1. **§8.1:** Verify T37 against the code and mark its ROADMAP section as
     shipped, the way T38's is.
  2. **§8.2:** Verify that `codex-goal.md` is gone from `main`. If it still
     exists, handle it as §8.2 describes.
  3. **§8.5:** Re-count `docs/LIMITATION_TRIAGE.md` against
     `docs/KNOWN_LIMITATIONS.md` **by command**. Put the command and its
     output in the log, then classify every missing entry using the triage's
     existing categories.
  4. **ISSUE-04:** In `docs/superpowers/plans/2026-09-21-user-reported-issues.md`,
     tick the ISSUE-04 checkboxes that DIN-12 and the ISSUE-04 merge delivered.
     Tick each box only after naming the test that covers it. Leave a box
     unticked when no test covers it, and say so.
- **Not in scope:**
  - `ideas.md` (gitignored, not in the clone);
  - `goal.md` and `.ai/CURRENT_STATE.md` (owned by the local goal session).
- **Gates:** `check-anchors`, `git diff --check`.

### Web queue after CT-2 (CT-6 to CT-10)

These tasks continue goal §12.3's serial web chain. Run them **one at a
time, in this order**. Start the next one only after the previous PR is
merged locally, because they share `App.tsx`, `styles.css` and the message
catalogues.

None of them needs the corpus or a bus. Each has at least one server half,
so the full Rust gates apply.

Sources:

- goal.md §4 and §12.3;
- `docs/superpowers/plans/2026-09-21-user-reported-issues.md` (the "issue
  plan" below).

In the issue plan, tick only the checkboxes you close, and only after naming
the test that covers each one.

**Deliberately not queued for the cloud:**

- ISSUE-12 needs the AppImage and a real multicast network.
- ISSUE-07 and ISSUE-08 need product data from the corpus.
- ISSUE-05 waits on the command-apply phases (ADR-0039) and the site
  decision.
- ISSUE-09's address-editor half needs a KNX-standard check first.
- ISSUE-02 and ISSUE-03 are candidates for a later brief once this queue
  is empty.

### CT-6 — Project diff against a raw `.knxproj` in the web UI (KNOWN_LIMITATIONS §57)

- **Scope:** `apps/knx-server` (the diff route) and `apps/knx-web`
  (`ProjectDiffPanel.tsx`).
- **Context:**
  - `knx diff` already accepts `.knxdb` and `.knxproj` on either side,
    through the `knx-app` loader, which keeps the full ETS import report.
  - `POST /api/project/diff {path}` and the web picker still take `.knxdb`
    only.
  - Read §57, especially "Lifted when". Silently normalising a browser path
    **without** the import diagnostics is explicitly not acceptable.
- **Deliver:**
  - The diff request names its input kind explicitly (`knxdb` | `knxproj`),
    or detects it and says so in the response. It reuses the existing
    `knx-app` loader: no second import path.
  - For a `.knxproj`, the response carries the import report next to the
    diff. An import whose report has error-level diagnostics is a failure,
    not a comparison, as with `knx diff`.
  - The web panel's picker offers both file types. The panel shows the
    import diagnostics collapsed above the diff, with their count in the
    summary line.
  - Browser uploads go through the existing mount/upload boundary
    (`crate::paths`, `/api/fs/upload`); never expose a server path the
    user did not choose.
- **Tests:**
  - HTTP tests in `apps/knx-server/tests/` using a **synthetic** `.knxproj`
    built in the test. Existing ZIP builders to copy from:
    `knxproj_with_installation` in `crates/knx-etsproj/tests/malformed_input.rs`,
    and `crates/knx-etsproj/tests/site_hierarchy.rs`. No corpus. Cover:
    - a clean import;
    - an import with warnings;
    - an import with an error-level diagnostic (refused);
    - an unknown input kind (400).
  - Vitest for the picker filter and the diagnostics block, in English and
    German.
- **Docs:** §57 (lifted or what remains), IMPLEMENTATION_STATUS, and the
  manual page `docs/manual/user-guide/08-reports-and-diff.md`.
- **Not in scope:**
  - applying a diff (§55);
  - three-way compare (§56);
  - correlation gaps (§52–§54).

### CT-7 — Session log: freetext search and export (issue plan ISSUE-13)

- **Scope:** `apps/knx-web/src/LogPanel.tsx`, plus a server export route if
  needed (`apps/knx-server/src/session_log.rs`, `routes.rs`).
- **Context:**
  - `LogPanel` filters by severity only, client-side, over the fetched
    entries.
  - The server log is capped at 1000 entries (`MAX_ENTRIES`). It counts
    dropped entries and inserts one synthetic "N log entries dropped"
    entry.
- **Deliver:** everything in the ISSUE-13 section of the issue plan:
  - A labelled search input with a clear action and a result count. It is
    case-insensitive over operation, summary and detail, and composes with
    the severity filters.
  - Export with an explicit scope: all entries, or the current filtered
    view. It keeps timestamp, severity, operation, summary, detail and
    report metadata.
  - The export goes through the existing file-picker/save boundary (like
    the group-address CSV export). Do not expose server filesystem paths to
    the browser.
  - The file format is documented and open. If it is CSV, escape cells
    that start with `=`, `+`, `-`, `@`, tab or CR (spreadsheet formula
    injection).
  - The export states the cap and the dropped count. It must never look
    like a complete lifetime audit.
- **Tests:**
  - Search: case-insensitivity, composition with severity, empty result.
  - Export: quotes, newlines, Unicode, an empty log, formula-prefixed
    user text, and a log over the cap (the dropped marker is present).
  - Server tests in `apps/knx-server/tests/http_log_route.rs` if a route
    is added.
- **Docs:** IMPLEMENTATION_STATUS, the ISSUE-13 checkboxes, and the manual
  page that describes the log panel (`docs/manual/user-guide/01-user-interface.md`,
  or wherever `grep -rn -i "log" docs/manual/user-guide` finds it).

### CT-8 — Actionable 422 errors and topic-targeted help (issue plan ISSUE-10)

- **Scope:** `apps/knx-server/src/errors.rs` and the route parsers in
  `routes.rs`, plus `apps/knx-web/src/{api.ts,HelpTip.tsx,HelpPanel.tsx,help.ts,GroupAddressTable.tsx}`.
- **Context:**
  - `ApiError` is `{status, message}`.
  - `HelpPanel` always opens on `DEFAULT_HELP_TOPIC_ID`.
  - `help.ts` has ten topic ids (`comObjectFlags`, `groupAddresses`, …).
- **Deliver:** the ISSUE-10 section of the issue plan:
  - Validation errors carry a stable machine-readable `kind`, the human
    `detail`, and, where applicable, `syntax` and `example` fields. Cover
    individual address, group address, DPT and BCP-47 language. Existing
    clients that read only the message must keep working, so this is an
    additive JSON change.
  - `api.ts` surfaces the structured fields; the error toast shows the
    syntax hint. The raw server detail stays available for diagnostics.
  - Help opens on a topic id: a `HelpTip` opens its own topic. F1 uses the
    focused control's registered topic and falls back to the overview. The
    active topic heading becomes the dialog's accessible name.
  - Short "Range" help for group ranges. It describes hierarchy and
    containment, not a datapoint range.
- **Tests:**
  - A 422 test for each of the four parsers, asserting the exact `kind`,
    `syntax` and `example`.
  - Help routing: a flag tip opens `comObjectFlags`, range help opens the
    group-range topic, plain F1 opens the overview.
  - German output.
- **Docs:** IMPLEMENTATION_STATUS, the ISSUE-10 checkboxes, and ADR-0024's
  help list if a topic is added.

### CT-9 — Application zoom and remembered pane widths (issue plan ISSUE-01)

- **Scope:**
  - `apps/knx-web/src/{App.tsx,ResizablePane.tsx,settingsStore.ts,styles.css}`;
  - `apps/knx-server/src/settings.rs` only if the settings document needs a
    migration step.
- **Context:**
  - The versioned settings document (`settings.rs`,
    `CURRENT_SCHEMA_VERSION = 1`, `Preferences` is an open JSON map) and
    `settingsStore.ts` are the **only** place for new preferences. Do not
    add `localStorage` keys (issue-plan constraint).
  - `ResizablePane` keeps its width in local state today, so hiding the
    pane loses it.
- **Deliver:** the ISSUE-01 section of the issue plan:
  - Ctrl+Plus, Ctrl+Minus and Ctrl+0 scale the UI, bounded. They are
    ignored while typing in an editable field, where they must still work
    as the browser/field expects.
  - The navigation and inspector widths survive hide/show and a restart,
    and are clamped on load.
  - Hovering a diagram device no longer hides its icon, address or object
    count.
  - Pointer and arrow-key resizing both keep working.
- **Tests:**
  - The three shortcuts including the editable-field case, and bounds.
  - Resize, then hide, show, remount, and the same clamped width comes
    back.
  - The hover CSS regression.
  - A settings round-trip through the server document, with an old
    document lacking the new keys.
- **Docs:** IMPLEMENTATION_STATUS, the ISSUE-01 checkboxes,
  `docs/manual/user-guide/09-settings-and-appearance.md`, and
  KNOWN_LIMITATIONS if a bound or a fallback is a real limit.

### CT-10 — Bus monitor: pause, export, decode visibility, statistics (issue plan ISSUE-11)

- **Scope:** `apps/knx-web/src/BusMonitorPanel.tsx` and `styles.css`, plus
  `apps/knx-server/src/{bus.rs,bus_routes.rs}` as needed.
- **Context:**
  - No bus in the cloud. Use the existing fakes (`knx_server::fake`:
    `FakeConnector`, `FakeTunnel`), as `apps/knx-server/tests/http_bus_monitor.rs`
    does.
  - Text and service filters already exist. The report that they are
    "missing" is a reachability/layout problem to reproduce, not a feature
    to rebuild.
- **Deliver:** the ISSUE-11 section of the issue plan:
  - **Pause/Resume:**
    - Pause stops client polling and render advancement. It does not
      disconnect and does not reset the server cursor.
    - Resume fetches the buffered gap with the existing dropped-row
      accounting.
  - **Export** of the captured rows keeps timestamp, source, destination,
    service, raw payload, DPT, decoded value or error, and sequence. Same
    format rules as CT-7: open, documented, formula-safe.
  - **Decode visibility:** the UI distinguishes "no DPT assigned",
    "conflicting DPT", "unsupported DPT" and "decode failed".
  - **Statistics:** service counts, busiest group addresses and talkative
    sources.
    - Only when the DTO carries the source data.
    - Labelled as buffer-limited.
    - Bounded in size.
- **Tests:**
  - Fake-timer tests: pause/resume, gateway close while paused, buffer
    overflow while paused, disconnect while paused.
  - Export over raw-only, decoded, decode-error and dropped-gap rows.
  - The four decode states.
  - A large buffer does not block polling.
- **Docs:** IMPLEMENTATION_STATUS, the ISSUE-11 checkboxes, and
  `docs/manual/user-guide/07-bus-and-interfaces.md`.
- **Not in scope:** live-bus verification. Say in the PR that it remains
  for a local hardware check.

## 5. Budget notes

- The promotional cloud credit is spent first. After it runs out, cloud
  sessions draw on the normal plan limits, which are shared with local work.
- Long autonomous runs are the most expensive: CT-3's fuzzing, and full
  workspace gates repeated after every small change. Briefs ask for gates at
  the end, not after every edit.
- Check the remaining balance in the usage menu after the first session. That
  first session shows the real cost of one brief better than any estimate.
- **Measured (CT-1, 2026-09-28):** the whole brief took 3 $ of the 250 $ credit.
  It covered web implementation, 13 tests, the full workspace gates and the
  docs. One data point only; CT-3 (fuzzing) may cost considerably more.

## 6. Experience log

- **CT-1 (PR #1, merged as `313489e`).** Attribution settings held: the
  commits carried no `Co-Authored-By` and no `Claude-Session` trailer. Only
  the PR description ended with "Generated by Claude Code", which does not
  reach a commit message when the PR is merged locally.
- **Cloud sessions run as root.** Root ignores the read-only file bit, so
  `knx-cli`'s `ga_import_of_a_real_change_against_a_readonly_store_reports_the_save_error`
  failed in the cloud and nowhere else. The test now probes whether the bit
  is enforced, and skips with a `SKIPPED:` line on stderr when it is not.
  Local and CI runs, which are not root, still exercise the save-error path.
  Any future test that relies on file permissions needs the same probe.
- **CT-2 start (2026-09-28): the VM image had an interrupted dpkg run.**
  apt refused every install ("dpkg was interrupted, you must manually run
  'dpkg --configure -a'"), and the hook fallback reported `apt=failed`.
  The VM's proxy also answers 403 for the deadsnakes and ondrej/php PPAs,
  which makes `apt-get update` exit non-zero. `setup-env.sh` now:
  - runs `dpkg --configure -a` first;
  - no longer lets the `apt-get update` status gate the install;
  - waits up to 120 s for the dpkg lock.

  In an `ubuntu:24.04` probe with an interrupted-dpkg journal plus an
  unreachable repository, WebKit ends up present.

  *Unknown:* why the image was left mid-dpkg. One possibility is that the
  environment's own setup script was cut off during its install, but this
  is not verified.
