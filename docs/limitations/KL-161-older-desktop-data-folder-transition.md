# Older desktop data-folder transition

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-161)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 161. Alpha.5 does not pick up the alpha.4 data folder

**Limitation.** `v0.1.0-alpha.5` changed the desktop app identifier to
`com.knxbench.knxbench-labs` ([ADR-0087](../adr/0087-desktop-app-identifier.md)).
The desktop data folder follows the identifier, so alpha.5 reads and writes
`~/.local/share/com.knxbench.knxbench-labs`. It does not look at the folder of
the withdrawn alpha.4 build. Started without that copy, alpha.5 shows an
empty project library and default settings.

**Why.** An automatic migration would have to name the old folder in the code,
and the old identifier carried a personal account name that was removed from
the product on the user's decision (2026-10-07). alpha.4 was only published
in a private repository with one user, whose folder was copied by hand and
checked with `diff -r`.

**Impact.** Whoever still runs an alpha.4 build copies its data folder (the one
under `~/.local/share` named after the old identifier) to
`~/.local/share/com.knxbench.knxbench-labs` while KNXBench is closed. alpha.5
never deletes or changes the old folder. The Docker/web server is not
affected: it takes its data folder from its own setting.

**Lifted when.** Not planned. The identifier is a stable contract from alpha.5
on.
<!-- END retained catalogue record -->
