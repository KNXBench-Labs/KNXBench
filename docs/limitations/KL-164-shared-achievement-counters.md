# Shared achievement counters

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-164)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 164. Achievements are per installation, count only in the UI, and can lose a concurrent increment

**Limitation.** Achievements ([ADR-0089](../adr/0089-achievements.md)) are
recorded in the server's `achievements.json`, so everyone who uses one
server shares one record (like the first-run guide, §160). Only actions
taken in the web or desktop UI count. The CLI and direct API use do not.
When two windows raise the same counter at the same moment, the server
keeps the higher value instead of adding the two, so one step can be
lost. Hidden achievements are hidden only in the UI: the catalogue
(`apps/knx-web/src/achievementCatalog.ts`) is open source. The server's
messages about a refused or damaged file are in English (§122).

**Why.** There are no user accounts (ADR-0026). Detection lives where the
user's actions are visible, and the merge is a maximum so that a save can
be repeated without counting twice. Maintainer decisions from 2026-10-07
(achievements interview, Q1, Q3, Q4 and Q6).

**Impact.** A counter can need one more step than it says. Nothing
else is affected: achievements gate nothing, never reach a project or an
export, and a reset moves the old record aside instead of deleting it.

**Counting edges (package 2).**

- `commissioner` counts different *individual addresses*, not physical
  devices: a replaced device at the same address does not count again,
  and the same address in another project counts only once.
- `fly-on-the-wire` counts whole minutes while a monitor session is open
  in an editor window. A session shorter than a minute counts nothing.
  With two editor windows showing the same session, minutes can count up
  to twice as fast. The diagnostics companion and the separate flow window
  count nothing (they have no tracker).
- Three triggers from the interview were changed because KNXBench cannot
  detect them as worded (ADR-0089, "Package 2"): `clean-sheet` (no
  project validation in the UI), `clean-bill` (no bus diagnosis) and
  `read-only-friday` (no send event, by design).
- Hidden achievements are about 16 % of the catalogue (6 of 38), not the
  25 % suggested in the interview, because the approved catalogue marked
  exactly these six as hidden.

**Also not verified.** The UI is tested in Vitest (happy-dom) and in
Chromium. WebKitGTK in the desktop shell and screen-reader output were
not checked.

**Lifted when.** Per-user records exist, or counters move to an additive
merge with request ids.
<!-- END retained catalogue record -->

## Update 2026-10-13: steps add up (Goal B6)

The "lifted when" condition for the lost step is met: `count` rules now
send their "one more" as an additive `increments` entry under a client
`requestId` instead of an absolute value, and the server adds it up.
Two windows that both saw 5 end at 7. The ids of the last 1,024 applied
batches are kept in `achievements.json` (`appliedRequests`), so a
duplicated, retried or reordered batch counts once, also after a server
restart. A batch whose answer was lost is retried unchanged under its
first id, before and never mixed with newer steps. A failed write applies
nothing; its retry counts once. Every answer is reconciled in the window:
a counter that reached its goal through another window unlocks there,
and a `distinct` total is recomputed from its markers. Unlocks, thresholds
and steps stay maxima (they were never lossy).

**Still true.** One shared record per server, counting only in the UI,
hidden entries readable in the open-source catalogue, English server
messages, and `fly-on-the-wire` counting up to twice as fast with two
editor windows on the same monitor session (each window's minutes are
real steps now, which they were before as well).

**Bounds.** A batch replayed after 1,024 newer ones would count again; real
retries happen within seconds. A reset moves the old file aside with its
log, so the new record starts with an empty one. An older build keeps
`appliedRequests` verbatim but merges `progress` as a maximum and refuses
`increments` (unknown field), so a newer window talking to an older
server records nothing until it is updated. The file stays schema 1.

Evidence: `apps/knx-server/src/achievements.rs` unit tests (two windows,
duplicate/reordered batches, bounded log across a reload, saturation,
validation, damaged log quarantined), `tests/http_achievements.rs`
(twenty concurrent windows, duplicate/retry/reorder across a restart, a
failed write and its retry, reset), and Vitest for rules and tracker
(steps as increments, two trackers on one fake server, lost answer retried
under the same id, reconcile on load, reset forgets a pending retry).
