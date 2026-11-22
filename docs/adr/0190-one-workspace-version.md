# ADR 0190: One workspace version for every program and library

Date: 2026-11-22
Status: Accepted
Session: workspace-version (owner question, 2026-11-22)

Supersedes decision 1 of [ADR-0018](0018-program-versions-and-file-headers.md)
(independent per-package versions). ADR-0018's build metadata, file headers and
the AR13 clean-tree amendment stay in force.

## Context

ADR-0018 gave every Cargo package and the `knx-web` npm package its own
SemVer version and a bump rule: a merge that changes a program's behaviour or
a library's public API bumps that package's `-alpha.N`. It called this "a
discipline rule, not a mechanical one".

What the tree showed on 2026-11-22, at main `1cb145bb6`:

- The release process (`v0.1.0-alpha.6` onwards) bumps `knx-cli`,
  `knx-server`, `knx-mcp`, `knx-desktop` and `knx-web` together. They stood at
  `0.1.0-alpha.22`.
- Of the sixteen library and tool packages (fifteen crates plus `xtask`),
  fifteen still stood at
  `0.1.0-alpha.1`, the number ADR-0018 gave them on 2026-09-12. Their public
  APIs had changed in many hundred merges since. `knx-app` alone stood at
  `0.1.0-alpha.3`, bumped twice by hand.
- The owner asked why the numbers were never adjusted. The honest answer:
  nothing checked the rule, and no merge followed it.
- Library versions are not invisible. `knx-app` writes its
  `CARGO_PKG_VERSION` into import evidence (`producer`), monitor exports
  (`tool_version`, `producer_version`) and contribution reports
  (`analyzer_version`), so those files named a version that matched no
  release.
- Every package is `publish = false`. No crate is consumed outside this
  workspace, so a per-crate number has no reader who needs it to differ.

## Decision

**One version, declared once.** `[workspace.package] version` in the root
`Cargo.toml` is the version of KNXBench. Every workspace member, applications,
libraries, `knx-testsupport` and `xtask` alike, declares
`version.workspace = true`. `apps/knx-web/package.json` and its lock file carry
the same string, because npm cannot inherit it.

**The release bump changes the root manifest and the web package.** A release
sets the workspace version to the release's `0.1.0-<stage>.N`; between
releases, `main` keeps the last released number. Merges do not bump. The
precise identity of a build stays in ADR-0018's build metadata
(`<version>+g<short-sha>`).

**A test keeps it that way.** `xtask/tests/it/workspace_version.rs` fails when a
member declares its own `version = "..."`, when cargo resolves any member to a
different version, or when the web package or lock disagrees.

## Alternatives considered

- **Keep independent versions and enforce the bump rule in a gate.** A gate
  can see that a crate's public items changed, but not whether the change
  deserved a bump or of which size. ADR-0018 already rejected the mechanical
  per-file variant for that reason, and the per-crate variant has the same
  weakness. It would also bump sixteen packages by hand for no consumer.
- **Keep independent versions and document them as meaningless.** Cheaper,
  but leaves stale numbers in evidence files and keeps a rule nobody follows.
- **Align only the applications (status quo).** This is exactly the drift
  this record ends.

## Consequences

- Release bumps shrink from five manifests plus the npm pair to one root line
  plus the npm pair. `Cargo.lock` changes every workspace entry at each release.
- `knx-app`'s evidence and export files now name the KNXBench release they
  came from. The only reader, the import-evidence loader, checks that
  `producer` is non-empty; nothing compares the number.
- ADR-0084 rejected an `xtask` "all program versions equal" check as
  contradicting ADR-0018. That reason no longer holds; the new test is the
  check it described.
- A future crate published to crates.io would need its own version again. That
  is a new decision for then, not a reason to keep sixteen stale numbers now.
