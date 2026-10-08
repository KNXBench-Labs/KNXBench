# ADR 0089: Achievements are a frontend catalogue over a grow-only server record

Date: 2026-10-07
Status: Accepted
Session: 5 (UI / UX)

## Context

The maintainer wants Steam-like achievements, 30 to 40 of them, to give
routine engineering work some momentum. In a grill-me interview on
2026-10-07 the maintainer settled these points:

- **Audience:** every user, **on by default**, with a switch in Settings.
  Off means off: no popups, no menu entry, nothing counted. Achievements
  already unlocked are kept.
- **Themes:** a mix of light-hearted and serious ones. Bus and
  commissioning achievements are allowed **only as results**: reading
  (monitor, scan, compare) may be rewarded freely, but writing counts only
  when its result is *verified*, and counters run over distinct devices.
  Nothing rewards telegram counts, group writes or speed.
- **Storage:** a separate, versioned `achievements.json` on the server,
  **not** a key in `settings.json` and **never** part of a project or
  export. Resetting or quarantining preferences must not take unlocks with
  it, and gamification has no place in the KNX domain model.
- **Detection:** in the frontend, through typed events. The server stores
  unlocks and does not count anything itself. The CLI does not count.
- **Presentation:** a Steam-style popup (new toast kind, at most two at
  once plus a "+N more" summary) and an overview dialog. No sound. The
  popup stays 9 s (raised from 6 s on 2026-10-08 at the user's request),
  then slides out; both animations follow the motion settings, and with
  reduced or no motion it simply disappears.
- **No telemetry:** no network contact and no global rarity percentages.
  A fixed tier (bronze, silver, gold, legendary) stands in for rarity.
- **Mix:** roughly a quarter hidden, a fifth with a progress counter.
- **Retroactivity:** achievements based on project state are judged
  against the open project; achievements based on actions count from
  their introduction only. The activity history is `coverage: "partial"`,
  so nothing is reconstructed from it.
- **Names:** invented separately for each language under one permanent
  id. Descriptions stay factual.

The repository already has the building blocks this needs: a versioned
server-side record with quarantine (`settings.rs`, ADR-0026's guard over
`/api/`), a toast stack with a `fun` kind (`toast.ts`), the i18n
catalogues, the command palette and the motion guard for animations.

## Decision

**Catalogue in the frontend, record on the server.**
`apps/knx-web/src/achievementCatalog.ts` is pure data. Each entry has a
permanent id, a tier, a hidden flag, a glyph, two message keys and a
declarative rule:

- `event`, optionally with `where` conditions (`eq`, `gte`) on the event's
  payload, type-checked against the event's fields;
- `count`, `threshold` (on any `projectObserved` measure), `localHours`
  (optionally on one weekday) and `localDate`;
- `steps`: events in order, the step reached is the progress;
- `distinct`: an event for `goal` different subjects;
- `allOthers`: every other catalogue entry unlocked.

Adding an achievement takes that entry plus two strings per language.
Nothing else in the application lists achievements.

A `distinct` rule remembers each subject as a progress entry
`<id>--<slug>-<fnv1a>` with the value 1, within the server's id rule, and
counts those entries. Ids therefore never contain `--`. Markers are only
written until the achievement unlocks, so they stay far below the
record's 1024-entry limit.

**The server stores a grow-only record.**
`apps/knx-server/src/achievements.rs` keeps
`{ schemaVersion, unlocked: id → RFC 3339, progress: id → u64 }` in the
data directory. It stores ids it does not know exactly like ids it does,
and it keeps unknown top-level members verbatim. A write is a *delta*,
merged under `AppState::achievements_lock`: an unlock keeps its earliest
instant (an existing timestamp the server cannot parse is never
displaced), and a counter keeps its highest value. Nothing is removed.

`GET /api/achievements`, `POST /api/achievements/record` and
`POST /api/achievements/reset` sit behind the ordinary guard. The record
gets the same protection as `settings.json`:

- A file from a newer build is refused, left byte for byte, and makes
  writes return 409.
- A damaged file is moved aside as `achievements.damaged-<stamp>.json`.
- A reset moves the record aside as `achievements.reset-<stamp>.json`;
  it never deletes it.

The atomic write and the move-aside step are shared with `settings.rs`
through `data_file.rs`.

Input limits:

- Ids are lowercase slugs of at most 64 characters.
- Timestamps must be RFC 3339.
- Counters are at most `Number.MAX_SAFE_INTEGER`, because the reader is
  JavaScript.
- A delta may name at most 256 ids, and the record may hold at most 1024
  entries.

**Detection runs through one channel.** UI code calls
`emitAchievementEvent({ type })` with facts, never with time. There is
deliberately no event type for sending to the bus. `useAchievements` (App
level) owns one `createAchievementTracker` per window. The tracker:

- asks the on/off switch on every event;
- buffers events until the record has loaded, so nothing already unlocked
  is announced again;
- evaluates against an injected clock;
- shows unlocks at once and persists only the delta;
- keeps a failed save in an outbox that rides along with the next report,
  with no retry loop;
- becomes read-only on `refusedNewer` or a 409, and unavailable when the
  record cannot be read.

The on/off switch is the `achievementsEnabled` key in `settings.json`.
Achievement data never enters `crates/`.

## Alternatives considered

- **A key in `settings.json`.** Rejected by the maintainer. Unlocks are
  history, not preferences, and must survive a settings reset or
  quarantine.
- **Server-side detection** (hooks in routes). It would count CLI and API
  use as well. But it would put a UI concern into many server routes and
  need a second copy of the catalogue. The frontend already sees every
  user action that matters.
- **A catalogue on the server too.** Two lists drift. The server owning
  structure but not meaning follows the pattern of `settings.rs`.
- **Replace instead of merge** on write. Two windows reporting at the same
  moment would undo each other. Merging (earliest, maximum) makes writes
  commutative and idempotent.
- **Global rarity from telemetry.** Rejected: no network contact.

## Consequences

- Hidden achievements are only hidden in the UI. The catalogue is open
  source.
- Achievements are per installation, not per person, like the first-run
  guide (KL §160). Two windows that increment the same counter at the same
  moment can lose one increment, because the server keeps the maximum and
  does not add. The CLI does not count. All of this is recorded in KL §164.
- Delivery came in two packages. Package 1 shipped the mechanism and 11
  achievements; package 2 the other 27 of the 38 agreed in the interview,
  each with an event emitted where the outcome is known. A bus or
  commissioning event may only be added for a *verified* outcome (see
  Context). A rule that would need a volume of bus writes violates this
  ADR.
- A locked hidden achievement shows no progress bar either: "3 / 10" would
  give it away.
- Every rule kind, the merge rules, the persistence edge cases and the
  guard are under test (`achievement*.test.ts(x)`, `useAchievements.test.tsx`,
  `konami.test.ts`, `src/achievements.rs`, `tests/http_achievements.rs`,
  `tests/http_auth.rs`).

## Package 2: where the outcomes come from

Each event is emitted by the component that knows the outcome, after the
server confirmed it. What counts as "verified" is the server's own
statement, not a guess in the UI:

| Event | Emitted by | Condition |
|---|---|---|
| `etsImported` | `App.tsx` after an ETS import | carries `ProjectTree.errors` (genuine losses, `Severity::Error`) and `.warnings` (warnings, unknown constructs, conflicts, unsupported features), filled by the server from the `ImportReport` (`apply_report_counts`); `passwordProtected` when the retry carried a password (the password never travels on the channel) |
| `deviceDownloadVerified` | `DeviceDownloadPanel` | `finished`, `written: "yes"` (every block read back unchanged, `device_download.rs`) and a restart that was not left `unconfirmed`; `subject` is the individual address |
| `individualAddressVerified` | `AddressProgrammingPanel` | `finished` with `written: "yes"`: the device answers at the new address (`unconfirmed`, `noNeed` and `no` do not count) |
| `deviceCompared` | `DeviceInspectionPanel` | only a read-only comparison that passed the panel's own consistency checks |
| `readinessChecked` | `DeviceInspectionPanel` | the offline grading; plannable means `verified` or `untested`, the panel's existing predicate |
| `busMonitorStarted`, `busMonitorMinute` | `BusMonitorPanel` | a session the server opened; one tick per minute while it is `active` |
| `flowWatched`, `busCaptureExported`, `lineScanCompleted` | monitor and scan panels | flow view on screen; capture actually saved; scan `completed` (once per session) |
| `groupAddressCsvImported` | `GroupAddressCsvButtons` | only a response with `applied: true` |

Three triggers from the interview catalogue were not detectable as
worded. They were changed and reported, not simulated:

- **#19 "zero defects with validation, ≥ 50 devices".** KNXBench has no
  project-wide validation in the UI. `clean-sheet` is now an ETS import of
  at least 50 devices whose report has neither losses nor notices.
- **#26 "bus diagnosis: healthy".** There is no bus diagnosis.
  `clean-bill` is the offline readiness check finding a download plannable
  for every device.
- **#34 "Friday after 3 pm without writing to the bus".** "Without
  writing" cannot be shown without a send event, which this ADR forbids.
  `read-only-friday` starts the (read-only) bus monitor on a Friday after
  15:00.
