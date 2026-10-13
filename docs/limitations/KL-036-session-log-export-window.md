# Session-log export window

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-036)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="36-session-log-t11-the-log-tab-was-unreachable-without-an-open-project-and-had-no-growth-cap--resolved-2026-09-10"></a>
## 36. Session-log export is a bounded retained window, not a lifetime audit

**U4 export boundary (2026-09-28 UTC).** The Log view now searches the
retained entries locally and exports either all retained entries or the
matching view as `knxbench-session-log` JSON v1. The export carries the
1000-entry cap, the pinned warning's dropped count (or `null` when its
wording is unfamiliar), and the raw warning message even if filtered out.
It cannot reconstruct entries evicted by the cap, a replaced project or a
server restart: this is **not** a lifetime audit. Exported raw entries can
contain project names, KNX addresses, filesystem locations and diagnostic
metadata. Browser export stays local; the native dialog writes atomically
but refuses a document above 16 MiB without replacing a previous file.
The desktop file chooser still needs interactive Linux validation across
supported environments; no complete desktop compatibility claim follows
from command/build tests alone (ADR-0047, `LogPanel.test.tsx`,
`sessionLogExport.test.ts`, `session_log_export_tests`).

**Resolved.** Two independent fixes: Part A in one commit, Part B in two —
its drop counter needed a follow-up correction, described at the end of
Part B below.

Part A: `apps/knx-web/src/App.tsx`'s "Log" toolbar button is
unconditionally enabled, and the `.workspace` slot now renders whenever
`tree` *or* `logOpen` is truthy, rather than `tree` alone —
`ProjectExplorer` still genuinely needs a project and stays gated on
`tree`, but `LogPanel` does not, so with no project open the Log tab is
the only thing in that area. With a project open, nothing changes: the
Log tab still takes the same slot it always did, and closing it returns
to Inspector/Dashboard as before. `LogPanel`'s `tree` prop is now
`ProjectTree | null`; it stays a `useEffect` dependency (so the panel
still refetches after a successful operation), and `refreshKey` —
bumped by `App.tsx`'s `reportError()` on every failed operation — is
untouched.

Part B: `apps/knx-server/src/session_log.rs` gained a documented
`MAX_ENTRIES: usize = 1000` const (not a bare literal at a call site).
Past it, `SessionLog::push` evicts the oldest real entries and pins a
synthetic `Severity::Warning`/`source: "log"` entry at index 0 naming
how many real entries have been dropped so far, refreshed on every
subsequent drop — CLAUDE.md's "never silently discard information" rule
applies to the log itself, not just to import data. That entry is never
itself dropped or duplicated, and it counts against the cap, so
`entries().len()` never exceeds 1000. `dropped` counts real entries
actually removed: the push that first exceeds the cap removes two (the
oldest real entry, plus one more to make room for the synthetic entry
itself), and every push after that while still over capacity removes
one more — an earlier draft of this counter tracked overflowing calls
instead of removed entries and read one low from the first drop
onward, caught before merge and fixed to match what actually happened
to the data. `reset()` clears the dropped count along with everything
else, so a freshly opened project starts with a genuinely empty log.
`GET /api/log`'s wire shape (`Vec<LogEntry>`, a bare JSON array) is
unchanged, so T12's own `session_log::from_csv_import_report` writer
and the existing `apps/knx-server/tests/http_log_route.rs` integration
tests needed no changes.

New tests: 6 in `session_log.rs`'s own `#[cfg(test)]` module (under the
cap, exactly at the cap, one past it — names 2 dropped — well past it —
cap + 250, names 251 dropped — reset-after-a-drop, and an invariant
test pinning "dropped named in the synthetic entry plus real entries
retained equals total pushes" at two different overflow sizes), plus a
new `apps/knx-web/src/App.test.tsx` (the first App-level test in this
project: reachable with no project open, unchanged behaviour with one
open). Gates: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
xtask -- check-layering`, `npx tsc --noEmit`, `npm test -- --run`
(139/139), `npm run build` all clean.

**Originally.** `apps/knx-web/src/App.tsx`'s "Log" toolbar button was
`disabled={!tree}`, and the whole `.workspace` div — the only place
`LogPanel` rendered — was itself gated on `tree` being non-null. But
`GET /api/log` deliberately worked with no project open (`routes.rs`
returned `200 []`, not `404`), specifically so a failed import with
nothing open yet still left an inspectable trail. Separately,
`SessionLog` had no cap on how many entries it accumulated, and
`LogPanel` refetched and re-serialized the whole log on every
`tree`/`refreshKey` change while the tab was open.

Both gaps traced to the same cause, per the plan's own text: the Log
tab's UI slot was scoped to "a project is open" from the start, since
every other panel in that slot (Inspector, Dashboard, Project Explorer)
needs one; a log entry cap was never in the design spec's stated
surface. Neither gap was caught until T11's final whole-branch review,
and both were parked as a follow-up rather than fixed in the
final-review-fix round that closed the rest of that review's findings.

Impact while open: the single highest-value scenario for this feature —
"my import just failed and no project is open, why?" — produced a
correct error entry on the server that the UI could not show. Unbounded
growth was never a problem at the usage levels this feature actually
saw (a single server process, one project at a time, log never
persisted), but nothing stopped it from becoming one over a very long
session.
<!-- END retained catalogue record -->
