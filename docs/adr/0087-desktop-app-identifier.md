# ADR 0087: The desktop app identifier is `com.knxbench.knxbench-labs`

Date: 2026-10-07
Status: Accepted
Session: 7 (integration / hardening)

## Context

The Tauri bundle identifier names the per-user data folder of the desktop
shell. `apps/knx-desktop/src-tauri/src/lib.rs` resolves it through
`app_data_dir()`, so projects, `settings.json`, theme packs and the WebView
storage live in `$XDG_DATA_HOME/<identifier>` (normally
`~/.local/share/<identifier>`). Nothing else in the code base derives a path
from it; the Docker/web server takes its data folder from its own setting.

The previous identifier was derived from a personal account name. The project
now lives under the `KNXBench-Labs` GitHub account, and the user decided on
2026-10-07 that no personal identity may remain anywhere in the repository,
its history or its artifacts. Tauri accepts identifiers made of alphanumerics,
hyphens and periods in reverse-domain order.

Up to this decision exactly one pre-release (`v0.1.0-alpha.4`) shipped the old
identifier, from a private repository with a single user.

## Decision

The identifier is `com.knxbench.knxbench-labs`, chosen by the user. The desktop
data folder is therefore `~/.local/share/com.knxbench.knxbench-labs`.
`v0.1.0-alpha.5` is the first build that uses it. The `v0.1.0-alpha.4`
pre-release was withdrawn together with the history rewrite.

There is no automatic migration from the old folder. A migration would have to
name the old folder in the code, which is exactly the personal identifier this
decision removes. The one existing installation was copied by hand
(`cp -a`, verified with `diff -r`), and the original folder was left in place.

## Alternatives considered

- **Keep the old identifier.** Rejected: it contradicts the user's decision
  that no personal identity remains in the product.
- **Automatic migration on first start.** Rejected for this release: it needs
  the old identifier as a literal in the code, and the only affected
  installation could be moved by hand with a byte-level check.
- **`com.knxbench.app` or similar.** Offered; the user chose
  `com.knxbench.knxbench-labs`.

## Consequences

- Anyone still holding data from the withdrawn alpha.4 build has to copy the old
  data folder to the new name before starting alpha.5; otherwise alpha.5
  starts with an empty library and default settings. The old folder is never
  deleted or changed by alpha.5
  ([KNOWN_LIMITATIONS §161](../KNOWN_LIMITATIONS.md#161-alpha5-does-not-pick-up-the-alpha4-data-folder)).
- Changing the identifier again moves the data folder again. Treat it as a
  stable, user-visible contract from now on.
- The manual, the theme-manager hint and its tests name the new folder.
