//! The versioned application settings file in the data directory, with its migration chain.
//!
//! One JSON document, `settings.json`, next to everything else under
//! `AppState::data_dir`:
//!
//! ```json
//! { "schemaVersion": 1, "settings": { "theme": "graphite" } }
//! ```
//!
//! The server owns this file, not the browser: two browser profiles, a
//! private window and the Tauri shell all read the same preferences
//! because they all ask the same server for them. `apps/knx-web` keeps a
//! `localStorage` copy so the first paint is not a flash of the default
//! theme, but that copy is a cache and this file is the record — on any
//! disagreement, this file wins.
//!
//! Three cases, and none of them deletes anything a user wrote:
//!
//! * **Older file** — the migration chain runs, the result is written
//!   back, and the caller is told what moved (`SettingsLoad::Migrated`).
//! * **Newer file** — written by a build from the future, so this one
//!   refuses to interpret it, leaves it exactly where it is, and runs the
//!   session on defaults (`SettingsLoad::RefusedNewer`). Rewriting a file
//!   you do not understand is how a user loses their settings by opening
//!   an old build once.
//! * **Damaged file** — moved aside under a name that says what happened
//!   and never deleted (`SettingsLoad::Quarantined`).
//!
//! Preferences are carried as an opaque `serde_json::Map`, deliberately:
//! which theme ids, accents or motion styles exist is a frontend registry
//! (`apps/knx-web/src/theme.ts`, `motion.ts`), and a second copy of those
//! lists over here would rot the day one of them grows an entry. This
//! module owns *structure* — the version, the migration chain, the
//! read-modify-write — and validates nothing about a preference's value.
//! That is also what makes an unknown key survive a round trip: a key this
//! build has never heard of is just another entry in the map.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// The schema version this build writes. Bumping it means adding exactly
/// one step to [`MIGRATIONS`] — the two are checked against each other in
/// this module's tests, so they cannot drift.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The shape preferences had while they lived in the browser: eight loose
/// `knx-desktop:` keys in `localStorage`, written by builds that had no
/// settings file at all. `POST /api/settings/adopt` hands them over
/// verbatim under this version and the chain below normalizes them, so the
/// browser era's quirks are dealt with in exactly one place instead of
/// being re-litigated by every reader.
pub const BROWSER_ERA_SCHEMA_VERSION: u32 = 0;

/// The file, relative to `AppState::data_dir`.
pub const SETTINGS_FILE_NAME: &str = "settings.json";

const SCHEMA_VERSION_FIELD: &str = "schemaVersion";
const SETTINGS_FIELD: &str = "settings";

/// The preference key the v0 -> v1 step below touches. The only key name
/// this crate knows; everything else is the frontend's business.
const THEME_KEY: &str = "theme";

/// Preferences as stored: key -> whatever JSON the frontend put there.
pub type Preferences = Map<String, Value>;

/// A settings document in memory — the version it was written against and
/// the preferences it carries, known and unknown alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDocument {
    pub version: u32,
    pub preferences: Preferences,
}

impl SettingsDocument {
    /// A document at the current version with nothing in it — what a
    /// session runs on when there is no usable file.
    pub fn defaults() -> Self {
        Self {
            version: CURRENT_SCHEMA_VERSION,
            preferences: Preferences::new(),
        }
    }
}

/// What [`load`] found. Every variant except [`SettingsLoad::Current`]
/// is worth telling the user about, which is why the reason travels with
/// the outcome instead of being logged and forgotten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsLoad {
    /// No settings file yet — a fresh install, or a user who has never
    /// changed a preference. Not an error, and not a reason to write one.
    Absent,
    /// A file at the current version, read as-is.
    Current(SettingsDocument),
    /// An older file, migrated and written back. `from` is the version it
    /// was found at.
    Migrated {
        document: SettingsDocument,
        from: u32,
    },
    /// A file this build is too old to understand. Left untouched.
    RefusedNewer { file_version: u32 },
    /// An unreadable or malformed file, moved aside to `moved_to`.
    Quarantined { moved_to: PathBuf, reason: String },
}

impl SettingsLoad {
    /// The document this session should run on: whatever was loadable,
    /// defaults otherwise.
    pub fn document(&self) -> SettingsDocument {
        match self {
            SettingsLoad::Current(document) => document.clone(),
            SettingsLoad::Migrated { document, .. } => document.clone(),
            SettingsLoad::Absent
            | SettingsLoad::RefusedNewer { .. }
            | SettingsLoad::Quarantined { .. } => SettingsDocument::defaults(),
        }
    }

    /// Whether a write would overwrite a file this build cannot read. The
    /// one outcome that must block `PUT /api/settings`.
    pub fn blocks_writes(&self) -> bool {
        matches!(self, SettingsLoad::RefusedNewer { .. })
    }
}

/// One migration step, from version `index` to version `index + 1`. A
/// step only ever edits preferences in place; the version number is the
/// chain's business, not the step's.
type MigrationStep = fn(&mut Preferences);

/// The chain, oldest step first. `MIGRATIONS[n]` migrates a version-`n`
/// document to version `n + 1`, so the chain's length *is*
/// [`CURRENT_SCHEMA_VERSION`] — there is no table of version numbers to
/// keep in step with a table of functions, because the index is the
/// version. Copied in shape from `crates/knx-store/src/migration.rs`
/// (a current-version constant, one function per step, a test per step
/// over a file an older build could really have written); not in
/// mechanism, since that one rides SQLite's `user_version` and this is a
/// JSON document with no pragma to lean on.
const MIGRATIONS: &[MigrationStep] = &[migrate_browser_era_to_v1];

/// v0 -> v1: the browser era's two legacy theme ids become the palettes
/// that replaced them. `apps/knx-web/src/theme.ts` has mapped `"light"`
/// and `"dark"` on *read* since the palettes were introduced, and rewrote
/// storage only when the user next changed the theme — so a
/// `localStorage` handed over by [`BROWSER_ERA_SCHEMA_VERSION`] can still
/// genuinely contain either. Normalizing here means the file never does.
fn migrate_browser_era_to_v1(preferences: &mut Preferences) {
    let Some(Value::String(theme)) = preferences.get(THEME_KEY) else {
        return;
    };
    let replacement = match theme.as_str() {
        "light" => "porcelain",
        "dark" => "graphite",
        _ => return,
    };
    preferences.insert(
        THEME_KEY.to_string(),
        Value::String(replacement.to_string()),
    );
}

/// Runs every step from `from` up to [`CURRENT_SCHEMA_VERSION`].
/// `from > CURRENT_SCHEMA_VERSION` never reaches here — [`load`] refuses
/// such a file before it gets this far, and `adopt` rejects it at the
/// route boundary.
fn migrate(preferences: &mut Preferences, from: u32) {
    for step in MIGRATIONS.iter().skip(from as usize) {
        step(preferences);
    }
}

/// Applies the chain to `preferences` and hands back a document stamped
/// at the current version.
pub fn migrated_document(mut preferences: Preferences, from: u32) -> SettingsDocument {
    migrate(&mut preferences, from);
    SettingsDocument {
        version: CURRENT_SCHEMA_VERSION,
        preferences,
    }
}

pub fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SETTINGS_FILE_NAME)
}

/// Reads the settings file, repairing it in place where that is both
/// possible and safe: an older file is migrated and written back, a
/// damaged one is moved aside. A newer file is neither read nor touched.
///
/// The `io::Error` this can still return is the filesystem refusing the
/// repair itself (the data directory is read-only, the disk is full) —
/// never a complaint about the file's *contents*, which is what
/// [`SettingsLoad::Quarantined`] is for.
pub fn load(data_dir: &Path) -> std::io::Result<SettingsLoad> {
    let path = settings_path(data_dir);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(SettingsLoad::Absent),
        // Present but unreadable — a directory where a file should be, a
        // mode nobody can read. Renaming it needs permission on the
        // *directory*, not on the file, so quarantine is usually still
        // available; if it is not, the error surfaces from `quarantine`.
        Err(e) => return quarantine(data_dir, &path, &format!("could not be read: {e}")),
    };

    let Ok(parsed) = serde_json::from_str::<Value>(&raw) else {
        return quarantine(data_dir, &path, "is not valid JSON");
    };
    let Value::Object(mut root) = parsed else {
        return quarantine(data_dir, &path, "is not a JSON object");
    };
    let Some(version) = root.get(SCHEMA_VERSION_FIELD).and_then(schema_version) else {
        return quarantine(
            data_dir,
            &path,
            "has no readable \"schemaVersion\" (a whole number is required)",
        );
    };
    let preferences = match root.remove(SETTINGS_FIELD) {
        // A document with no "settings" member at all is a version stamp
        // and nothing else: odd, but unambiguous and lossless to read as
        // "no preferences set".
        None | Some(Value::Null) => Preferences::new(),
        Some(Value::Object(map)) => map,
        Some(_) => {
            return quarantine(
                data_dir,
                &path,
                "has a \"settings\" member that is not an object",
            )
        }
    };

    if version > CURRENT_SCHEMA_VERSION {
        return Ok(SettingsLoad::RefusedNewer {
            file_version: version,
        });
    }
    if version == CURRENT_SCHEMA_VERSION {
        return Ok(SettingsLoad::Current(SettingsDocument {
            version,
            preferences,
        }));
    }

    let document = migrated_document(preferences, version);
    store(data_dir, &document)?;
    Ok(SettingsLoad::Migrated {
        document,
        from: version,
    })
}

/// A schema version must be a non-negative whole number that fits a
/// `u32`. A float, a string, `-1` or something larger than `u32::MAX` is
/// not a version this build can compare against, so it reads as damaged
/// rather than as "newer".
fn schema_version(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|v| u32::try_from(v).ok())
}

/// Writes `document` atomically: a sibling temp file, flushed to the
/// platter, then a rename, then an fsync of the directory that holds the
/// rename. A crash mid-write therefore leaves the previous settings
/// intact rather than half a document, and a crash just after the rename
/// leaves a whole document rather than a durable name pointing at bytes
/// that never landed. The file is small and written rarely, so the two
/// syncs cost nothing worth counting.
///
/// The temp file has one fixed name, which is safe only because every
/// caller holds `AppState::settings_lock` (`crate::settings_routes`); two
/// unsynchronized writers would race on that path.
///
/// Keys come out in `serde_json`'s map order (alphabetical, without the
/// `preserve_order` feature), which makes the file stable to diff between
/// writes.
pub fn store(data_dir: &Path, document: &SettingsDocument) -> std::io::Result<()> {
    let mut root = Map::new();
    root.insert(
        SCHEMA_VERSION_FIELD.to_string(),
        Value::from(document.version),
    );
    root.insert(
        SETTINGS_FIELD.to_string(),
        Value::Object(document.preferences.clone()),
    );
    let mut body = serde_json::to_string_pretty(&Value::Object(root))?;
    body.push('\n');

    std::fs::create_dir_all(data_dir)?;
    let target = settings_path(data_dir);
    let temporary = data_dir.join(format!("{SETTINGS_FILE_NAME}.tmp"));
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, &target)?;
    // A rename is atomic, which is not the same as durable: without this
    // the directory entry can survive a crash the payload did not.
    std::fs::File::open(data_dir)?.sync_all()?;
    Ok(())
}

/// Moves a file this build cannot read out of the way, under a name that
/// says what happened and when: `settings.damaged-20260921T120000Z.json`.
/// Never a delete — the file may be the only copy of preferences a user
/// spent an evening arranging, and "this build could not parse it" is a
/// long way from "nobody can".
fn quarantine(data_dir: &Path, path: &Path, reason: &str) -> std::io::Result<SettingsLoad> {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    quarantine_at(data_dir, path, reason, &stamp)
}

/// The body of [`quarantine`] with the clock handed in, so the
/// collision counter can be tested without waiting for two failures to
/// fall inside the same second.
fn quarantine_at(
    data_dir: &Path,
    path: &Path,
    reason: &str,
    stamp: &str,
) -> std::io::Result<SettingsLoad> {
    let mut moved_to = data_dir.join(format!("settings.damaged-{stamp}.json"));
    // Two damaged files in the same second is contrived, but overwriting
    // the first one with the second would be exactly the data loss this
    // function exists to avoid.
    let mut attempt = 1;
    while moved_to.exists() {
        moved_to = data_dir.join(format!("settings.damaged-{stamp}-{attempt}.json"));
        attempt += 1;
    }
    std::fs::rename(path, &moved_to)?;
    Ok(SettingsLoad::Quarantined {
        moved_to,
        reason: format!("{SETTINGS_FILE_NAME} {reason}"),
    })
}

/// Applies a patch to `preferences`: every key in `patch` is written, and
/// a `null` value *removes* the key rather than storing JSON null.
///
/// The removal rule is what lets a frontend "unset" a preference —
/// `productLanguage.ts` deletes its key for "package default" rather than
/// writing the string `"null"` — and it is the reason a patch is merged
/// instead of replacing the document wholesale: a key this build does not
/// know is not in any patch it sends, so a read-modify-write leaves it
/// exactly where it was.
pub fn apply_patch(preferences: &mut Preferences, patch: Preferences) {
    for (key, value) in patch {
        if value.is_null() {
            preferences.remove(&key);
        } else {
            preferences.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn preferences(pairs: &[(&str, &str)]) -> Preferences {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), Value::String((*v).to_string())))
            .collect()
    }

    #[test]
    fn the_chain_has_exactly_one_step_per_version() {
        assert_eq!(MIGRATIONS.len() as u32, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn an_absent_file_is_not_an_error() {
        let dir = data_dir();
        assert_eq!(load(dir.path()).unwrap(), SettingsLoad::Absent);
        assert!(!settings_path(dir.path()).exists());
    }

    #[test]
    fn a_document_survives_a_round_trip_through_the_file() {
        let dir = data_dir();
        let document = SettingsDocument {
            version: CURRENT_SCHEMA_VERSION,
            preferences: preferences(&[("theme", "graphite"), ("uiLanguage", "de")]),
        };
        store(dir.path(), &document).unwrap();
        assert_eq!(load(dir.path()).unwrap(), SettingsLoad::Current(document));
    }

    #[test]
    fn an_older_file_is_migrated_with_its_other_values_intact() {
        let dir = data_dir();
        // Written by the writer, at the version the browser era handed
        // over — not a hand-built JSON string that could skip a rule
        // `store` enforces.
        store(
            dir.path(),
            &SettingsDocument {
                version: BROWSER_ERA_SCHEMA_VERSION,
                preferences: preferences(&[
                    ("theme", "dark"),
                    ("accent", "mint"),
                    ("productLanguage", "de-DE"),
                ]),
            },
        )
        .unwrap();

        let SettingsLoad::Migrated { document, from } = load(dir.path()).unwrap() else {
            panic!("expected a migration");
        };
        assert_eq!(from, BROWSER_ERA_SCHEMA_VERSION);
        assert_eq!(document.version, CURRENT_SCHEMA_VERSION);
        assert_eq!(document.preferences["theme"], Value::from("graphite"));
        assert_eq!(document.preferences["accent"], Value::from("mint"));
        assert_eq!(
            document.preferences["productLanguage"],
            Value::from("de-DE")
        );

        // Written back, so the next read is not a migration.
        assert!(matches!(
            load(dir.path()).unwrap(),
            SettingsLoad::Current(_)
        ));
    }

    #[test]
    fn the_browser_eras_light_theme_migrates_too() {
        let mut prefs = preferences(&[("theme", "light")]);
        migrate(&mut prefs, BROWSER_ERA_SCHEMA_VERSION);
        assert_eq!(prefs["theme"], Value::from("porcelain"));
    }

    #[test]
    fn a_theme_that_is_not_a_legacy_alias_is_left_alone_by_the_chain() {
        let mut prefs = preferences(&[("theme", "neon-grid")]);
        migrate(&mut prefs, BROWSER_ERA_SCHEMA_VERSION);
        assert_eq!(prefs["theme"], Value::from("neon-grid"));
    }

    #[test]
    fn a_newer_file_is_refused_and_left_byte_for_byte_as_it_was() {
        let dir = data_dir();
        let path = settings_path(dir.path());
        let from_the_future = format!(
            "{{\"{SCHEMA_VERSION_FIELD}\": {}, \
             \"{SETTINGS_FIELD}\": {{\"theme\": \"chartreuse\"}}}}",
            CURRENT_SCHEMA_VERSION + 7
        );
        std::fs::write(&path, &from_the_future).unwrap();

        assert_eq!(
            load(dir.path()).unwrap(),
            SettingsLoad::RefusedNewer {
                file_version: CURRENT_SCHEMA_VERSION + 7
            }
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), from_the_future);
    }

    #[test]
    fn a_damaged_file_is_moved_aside_rather_than_deleted() {
        let dir = data_dir();
        let path = settings_path(dir.path());
        std::fs::write(&path, "{ this was a text editor's fault").unwrap();

        let SettingsLoad::Quarantined { moved_to, reason } = load(dir.path()).unwrap() else {
            panic!("expected a quarantine");
        };
        assert!(reason.contains("not valid JSON"), "{reason}");
        assert!(!path.exists(), "the damaged file should have been moved");
        assert_eq!(
            std::fs::read_to_string(&moved_to).unwrap(),
            "{ this was a text editor's fault"
        );
        // And the session that follows starts from defaults.
        assert_eq!(load(dir.path()).unwrap(), SettingsLoad::Absent);
    }

    #[test]
    fn a_file_with_no_version_is_damaged_not_assumed_current() {
        let dir = data_dir();
        std::fs::write(
            settings_path(dir.path()),
            "{\"settings\": {\"theme\": \"graphite\"}}",
        )
        .unwrap();
        assert!(matches!(
            load(dir.path()).unwrap(),
            SettingsLoad::Quarantined { .. }
        ));
    }

    #[test]
    fn a_version_that_is_not_a_whole_number_is_damaged_not_newer() {
        let dir = data_dir();
        std::fs::write(
            settings_path(dir.path()),
            "{\"schemaVersion\": \"1\", \"settings\": {}}",
        )
        .unwrap();
        assert!(matches!(
            load(dir.path()).unwrap(),
            SettingsLoad::Quarantined { .. }
        ));
    }

    #[test]
    fn two_damaged_files_in_the_same_second_do_not_overwrite_each_other() {
        // The stamp is handed in rather than read from the clock: on a real
        // clock the two calls can straddle a second boundary, the names
        // differ for the wrong reason, and the counter this test exists to
        // cover never runs.
        let dir = data_dir();
        let path = settings_path(dir.path());
        let stamp = "20260921T120000Z";

        std::fs::write(&path, "first wreck").unwrap();
        let SettingsLoad::Quarantined {
            moved_to: first, ..
        } = quarantine_at(dir.path(), &path, "is not JSON", stamp).unwrap()
        else {
            panic!("expected a quarantine");
        };
        std::fs::write(&path, "second wreck").unwrap();
        let SettingsLoad::Quarantined {
            moved_to: second, ..
        } = quarantine_at(dir.path(), &path, "is not JSON", stamp).unwrap()
        else {
            panic!("expected a quarantine");
        };

        assert_eq!(
            first.file_name().unwrap(),
            format!("settings.damaged-{stamp}.json").as_str()
        );
        assert_eq!(
            second.file_name().unwrap(),
            format!("settings.damaged-{stamp}-1.json").as_str()
        );
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "first wreck");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "second wreck");
    }

    #[test]
    fn a_damaged_file_is_quarantined_under_a_stamped_name() {
        let dir = data_dir();
        std::fs::write(settings_path(dir.path()), "not JSON at all").unwrap();
        let SettingsLoad::Quarantined { moved_to, .. } = load(dir.path()).unwrap() else {
            panic!("expected a quarantine");
        };
        let name = moved_to.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("settings.damaged-"), "{name}");
        assert!(name.ends_with(".json"), "{name}");
        assert!(!settings_path(dir.path()).exists());
    }

    #[test]
    fn an_unknown_key_survives_a_read_modify_write() {
        let dir = data_dir();
        let mut stored = Preferences::new();
        stored.insert(
            "somethingFromVersionNine".to_string(),
            Value::from("keep me"),
        );
        stored.insert("theme".to_string(), Value::from("porcelain"));
        store(
            dir.path(),
            &SettingsDocument {
                version: CURRENT_SCHEMA_VERSION,
                preferences: stored,
            },
        )
        .unwrap();

        let mut document = load(dir.path()).unwrap().document();
        apply_patch(
            &mut document.preferences,
            preferences(&[("theme", "graphite")]),
        );
        store(dir.path(), &document).unwrap();

        let reread = load(dir.path()).unwrap().document();
        assert_eq!(reread.preferences["theme"], Value::from("graphite"));
        assert_eq!(
            reread.preferences["somethingFromVersionNine"],
            Value::from("keep me")
        );
    }

    #[test]
    fn a_null_in_a_patch_removes_the_key() {
        let mut stored = preferences(&[("productLanguage", "de-DE"), ("theme", "graphite")]);
        let mut patch = Preferences::new();
        patch.insert("productLanguage".to_string(), Value::Null);
        apply_patch(&mut stored, patch);
        assert!(!stored.contains_key("productLanguage"));
        assert_eq!(stored["theme"], Value::from("graphite"));
    }

    #[test]
    fn a_refused_file_blocks_writes_and_nothing_else_does() {
        assert!(SettingsLoad::RefusedNewer { file_version: 99 }.blocks_writes());
        assert!(!SettingsLoad::Absent.blocks_writes());
        assert!(!SettingsLoad::Current(SettingsDocument::defaults()).blocks_writes());
    }

    #[test]
    fn an_unreadable_document_yields_defaults_for_the_session() {
        assert_eq!(
            SettingsLoad::RefusedNewer { file_version: 99 }.document(),
            SettingsDocument::defaults()
        );
        assert!(SettingsLoad::Absent.document().preferences.is_empty());
    }

    #[test]
    fn a_settings_member_that_is_not_an_object_is_damaged() {
        let dir = data_dir();
        std::fs::write(
            settings_path(dir.path()),
            "{\"schemaVersion\": 1, \"settings\": [\"theme\"]}",
        )
        .unwrap();
        assert!(matches!(
            load(dir.path()).unwrap(),
            SettingsLoad::Quarantined { .. }
        ));
    }

    #[test]
    fn a_version_stamp_with_no_settings_member_reads_as_no_preferences() {
        let dir = data_dir();
        std::fs::write(settings_path(dir.path()), "{\"schemaVersion\": 1}").unwrap();
        assert_eq!(
            load(dir.path()).unwrap(),
            SettingsLoad::Current(SettingsDocument::defaults())
        );
    }
}
