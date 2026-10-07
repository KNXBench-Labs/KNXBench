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
- **Presentation:** a Steam-style popup (new toast kind, about 6 s, follows
  the motion settings, at most two at once plus a "+N more" summary) and an
  overview dialog. No sound.
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
declarative rule (`event`, `count`, `threshold`, `localHours`,
`localDate`). Adding an achievement takes that entry plus two strings per
language. Nothing else in the application lists achievements.

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
- Delivery comes in two packages. Package 1 (this ADR) ships the mechanism
  and 11 achievements. Package 2 adds the rest of the 38 agreed in the
  interview, each with an event emitted from the place where the outcome
  is known. A bus or commissioning event may only be added for a
  *verified* outcome (see Context). A rule that would need a volume of
  bus writes violates this ADR.
- Every rule kind, the merge rules, the persistence edge cases and the
  guard are under test (`achievement*.test.ts(x)`, `useAchievements.test.tsx`,
  `konami.test.ts`, `src/achievements.rs`, `tests/http_achievements.rs`,
  `tests/http_auth.rs`).
