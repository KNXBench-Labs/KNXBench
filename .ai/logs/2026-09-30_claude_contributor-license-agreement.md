# 2026-09-30 Claude: contributor license agreement

## Request

The user wants commercial use of KNXBench allowed, but not the sale of a product
based on it. The user chose to stay open source (AGPL) and to add a CLA, so a
commercial license can be sold to vendors of closed products (dual licensing).

## Facts checked

- OSD criteria 1 and 6: an open-source license cannot forbid selling.
- `Cargo.toml` license `AGPL-3.0-or-later`, inherited by all crates.
- `git log origin/main --format=%ae`: 1,706 commits, all `github@knxbench.com`,
  so no outside rights exist yet.
- The contributing guide said "no CLA, no copyright assignment"; replaced.

## Done

`CLA.md`, ADR-0053, `.github/pull_request_template.md`, README, FAQ, contributing
guide, KNOWN_LIMITATIONS §148, IMPLEMENTATION_STATUS. Gate: `xtask check-anchors`
(fresh target, this worktree), `git diff --check`.

## Design choices

- License grant, not assignment (§ 29 UrhG: German copyright is not transferable).
- Contributor protection: AGPL-or-OSI-copyleft promise; breaking it reduces the
  grant to plain AGPL rights (fallback clause, modeled on the FLA idea).
- Acceptance by a fixed PR sentence + checkbox; no CLA bot (no new dependency).

## Open

Lawyer review, organization agreement, commercial license text (§148).

## Reverted the same day

After comparing source-available licenses, the user kept the AGPL and asked to
remove the CLA (ADR-0054). Reading UrhG § 40 (written form for rights in
unspecified future works, non-waivable termination after five years) and
§ 32 (remuneration claim; exemption only for a simple right for everyone)
showed the CLA's checkbox acceptance was weaker than assumed. Not reviewed by
a lawyer. No contribution was ever made under the CLA.
