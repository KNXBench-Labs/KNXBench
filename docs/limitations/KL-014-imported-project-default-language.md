# Imported project default language

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-014)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 14. The project's default language is a placeholder

**Limitation.** Every imported project is created with
`Language("en")` as its `StringTable`'s default language, regardless of the
language the project was actually authored in.

**Cause.** A schema-11 project file carries no project-wide language tag:
`DefaultLanguage` belongs to an application program's `RegistrationInfo`
(RESEARCH §4.1), not to `ProjectInformation`. Session 3 imports no
application program, so there is nothing in the imported data to derive a
real value from, and inventing one from, say, the project name would be a
guess presented as a fact.

**Impact.** Nothing observable in Session 3: instance-level `@Text` and
`@Description` are literal, not translated, so every `Text` this importer
produces is `Text::Literal` and resolves without consulting the table at
all. The default language only starts to matter once localized program
text exists. Export and semantic comparison both ask the project's own
`StringTable::default_language()` rather than naming a language
themselves, so when a real value arrives there is exactly one place that
sets it.

**Lifted when.** Session 4 ingests application programs and their
`RegistrationInfo`, giving the importer a measured language to set instead
of a placeholder.

**Update 2026-10-05 (AR10): accepted boundary, not a pending fix.** The
"Lifted when" condition above assumed a measured language would arrive with
application programs. It cannot: *Project Schema23* `ProjectInformation` has
no language attribute; `ApplicationProgram/@DefaultLanguage` is one
program's language, not the project's; and the only project-side language
attribute, the optional per-device `DeviceInstance/@InitialValueLanguage`,
occurs in none of the three local reference projects. If present it is
reported as an unknown attribute and retained, never promoted. The
placeholder is still never consulted: no production path inserts a
`StringTable` entry, so every resolution is literal. Pinned by
`crates/knx-etsproj/tests/project_language.rs`; trace in
[research/backend-localization-paths.md](../research/backend-localization-paths.md).
<!-- END retained catalogue record -->
