# ADR 0029: Application settings live in one versioned file on the server

Date: 2026-09-21
Status: Accepted
Session: Goal-completion, Task 28

## Context

Eight preferences — theme, accent, density, motion level, motion style,
UI language, installed UI language packs, product-data language — were
stored in eight `localStorage` keys under the `knx-desktop:` prefix. That
has three consequences, and only the first is obvious:

1. **Two front ends, two sets of preferences.** The same server serves
   the web application and the Tauri desktop shell. A browser and the
   desktop shell do not share `localStorage`, so the same user with the
   same server has two unrelated sets of preferences, and neither of them
   knows the other exists.
2. **No schema.** A `localStorage` value is a string with no version on
   it. `theme.ts` still reads `"light"` and `"dark"` and maps them to
   `porcelain`/`graphite` on every read, because there was no moment at
   which anything could have rewritten them. That is a migration with no
   place to happen, performed forever.
3. **Nothing to back up.** A preference that exists only in a browser
   profile is not in the data directory, is not in a debug report, and
   does not survive the browser being reinstalled.

The server already owns a data directory with a containment rule
(`apps/knx-server/src/paths.rs`), and `crates/knx-store/src/migration.rs`
already establishes how this project migrates a persisted artefact: a
current-version constant, one function per step, and tests over an
artefact a real older build could have written.

## Decision

**One JSON file, `settings.json`, in the server's data directory, is the
record. Everything else is a cache.**

```json
{
  "schemaVersion": 1,
  "settings": {
    "theme": "graphite",
    "accent": "mint",
    "density": "compact",
    "motionLevel": "standard",
    "motionStyle": "apple",
    "uiLanguage": "de",
    "productLanguage": "de-DE",
    "uiLanguagePacks": { "nl-NL": { "formatVersion": 1, "tag": "nl-NL", "…": "…" } }
  }
}
```

Three routes, all behind the same guard as the rest of `/api/`
(ADR-0026), none of which accepts a path from the client:

* `GET /api/settings` — what this session should run on, plus how it got
  there.
* `PUT /api/settings` — a **patch**, not a replacement.
* `POST /api/settings/adopt` — the one-time handover of a browser's old
  keys, accepted only while no file exists.

Four rules make the file safe to own:

**The version is a gate, not a suggestion.** `CURRENT_SCHEMA_VERSION`
names what this build writes. `MIGRATIONS[n]` migrates version `n` to
`n + 1`, so the chain's length *is* the current version and a unit test
asserts exactly that — a step added without bumping the constant fails
the build rather than corrupting a file. Version 0 is "the browser era":
the shape `localStorage` actually holds, which is what makes the first
step a real migration rather than a placeholder.

**A file from a newer build is refused, not flattened.** Reading it
yields `status: "refusedNewer"`, the session runs on defaults, the file
is left byte-for-byte alone, and *writes are refused with 409* for the
rest of that session. Downgrading a build must not cost you the
preferences the newer one recorded.

**A damaged file is moved aside, never deleted.** Unparseable JSON, a
missing or non-integer `schemaVersion`, a non-object `settings` member:
the file is renamed to `settings.damaged-<UTC timestamp>.json` (with a
counter for the second failure in the same second) and the session starts
from defaults. The bytes stay on disk, next to the file that replaced
them, for whoever wants to read them.

**Unknown keys survive a read-modify-write.** Preferences are carried as
an opaque `serde_json::Map`. The server validates the document's
structure and nothing about the values — the theme, accent and motion
vocabularies stay in the frontend registries that already own them,
rather than being copied into Rust where they would rot. A `PUT` names
the keys it changes and leaves every other key exactly where it was,
including one this build has never heard of. A `null` value removes a
key; that is the only way to unset a preference.

Each of the three non-quiet outcomes is reported to the user through the
existing session log (`source: "settings"`), which the Log panel already
renders, and repeated in the response as a plain-English `notice`.

**The browser keeps one cache key, and it cannot be mistaken for the
record.** `index.html`'s pre-mount bootstrap and the first React render
both need a theme before a network round trip can finish, so
`apps/knx-web/src/settingsStore.ts` mirrors the whole document into a
single opaque key, `knx-desktop:settings-cache`. One key holding one
document, not eight hand-editable ones; `initSettings()` overwrites it
wholesale with whatever the server says, including "nothing".

**Adoption happens once, and only into an empty file.** A user with
preferences in `localStorage` today is not reset to defaults: the
frontend posts them to `/api/settings/adopt` at version 0, the server
migrates them on the way in, and the frontend then deletes exactly the
keys it handed over. The server refuses to adopt into an existing file
(409), so a second browser profile cannot stamp its defaults over a
record somebody has been curating.

**Three `knx-desktop:` keys deliberately stay in `localStorage`**:
`project-context`, `bus-session-context` and `context-changed`, all owned
by `apps/knx-web/src/busContext.ts`. They are per-window session state,
not preferences. Moving them into a shared record would make two windows
fight over one value.

## Consequences

Adding a preference costs, end to end:

1. A key in the settings document, chosen by the module that owns the
   preference — no central registry to edit, no server-side change, no
   new route, and no migration (a preference nobody has set yet reads as
   absent and falls back to its default, which is what every `loadX`
   already does for an unrecognised value).
2. The existing pattern in that module: `loadX(storage)` / `saveX(storage)`
   over `settingsStorage`, and a hook that re-reads on
   `useSettingsRevision()`.
3. Nothing in `BROWSER_ERA_KEYS`. That table is a closed historical
   record of what the browser stored before this file existed, not a
   registry of preferences.

A migration is needed only when the *meaning* of an existing key changes
— a renamed id, a value that becomes an object — and then it is one
function pushed onto `MIGRATIONS` plus a bump of
`CURRENT_SCHEMA_VERSION`, with a test whose input the production writer
produced at the older version.

What this does not do:

* It does not build a settings *screen*. `SettingsPanel.tsx` already
  exists and is unchanged by this; where a preference is *edited* is a
  separate question from where it is *kept*.
* It does not make preferences per-user. There is one settings file per
  data directory, as there is one project per server. Multi-user is
  parked (T22), and when it arrives this file is where the per-user split
  will have to happen.
* It does not synchronize live between two open windows. A second window
  reads the record when it loads and writes its own changes to it; it
  does not learn about the first window's change until it reloads. The
  server serializes the read-modify-write (`AppState::settings_lock`), so
  the two cannot lose each other's *keys* — only the second window's view
  of a key the first one changed goes stale.
* It does not encrypt anything. These are preferences; the file holds no
  credentials and no bus addresses.
