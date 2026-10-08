# 2026-10-08 Claude — legacy VD package L1 (read-only inspection)

Follows the grill-me interview in `2026-10-08_claude_legacy-vd-grilling.md`
(Q1–Q18 answered, explicit "go").

## Delivered

- Branch `feature/legacy-vd-l1-20261008`, feature commit on top of
  `e96bfb5d`; ADR-0094 (0093 was taken upstream by the wizard ADR during
  the package, so the ADR was renumbered before the rebase).
- `knx-productdb::legacy` (container/exim/inspect/text/error), with no
  decryption. `knx_app::legacy` (LegacyPassword, open/inspect with
  knx-secure). CLI `knx products inspect-legacy`. Content-based
  `PackageError::LegacyExIm` in `install_package`.
- New check-layering rule: knx-productdb must not reach knx-secure (dev
  edges included).

## Findings during the package

- First design used the edge `knx-productdb → knx-secure` (spec B-4). The
  check-layering gate refused it as `knx-mcp → knx-productdb → knx-secure`,
  because ADR-0090 forbids key material in the MCP adapter. Redesigned so
  knx-productdb hands out the encrypted stream plus check bytes and finishes
  from the plaintext. knx-app decrypts.
- Info-ZIP `zip -P` writes bit 3 plus real local values, which the hardened
  package validator refuses. Real VD files use flags 0x0001 without a
  descriptor. The fixtures are built with `zipcloak -O` (via a pty), which
  writes exactly that layout.
- Plaintext EX-IM fixtures need the `binary` git attribute: CRLF and the
  `K ` trailing space are data and would otherwise fail `git diff --check`.
- Mutation sweep: 25/25 killed. The first sweep had two survivors (bare-LF
  and size check, shadowed by the CR and CRC checks); tests now assert the
  refusal reason.

## Evidence (gate)

Full gate on `d01cc58d`: workspace 3,618 passed / 0 failed / 180 ignored,
clippy, fmt, web build, five xtask gates, corpus (legacy 2/2, member names
1/1, standalone 3/3, matrix 1/1). Re-gate on `3d8eef7f` after rebasing onto
`a642eaf9`: 1,786 / 0 / 113 for server+app+productdb+cli, clippy, xtask. The first
gate attempt was stopped by me because origin/main had moved (stale base).
The second attempt's workspace test step died at link time (`ld` signal 9),
before any test ran. Both attempts are kept in task scratch. The test step
was rerun with `-j 2`.

## Next

- L2: publish `.vd*` application programs into the product database, with
  direct table mapping, `M-xxxx_LX-<sha8>_A-<PROGRAM_ID>` ids,
  P-/O-/R- numbering, translations and `dynamic_node` visibility.
  Acceptance is semantic equivalence against ETS's conversion of
  N000520_IRBM_20 in the house project.
- L3: server/web upload, password dialog, optional remembered password
  (0600, server XDG config).
- L4 (separate package): download.
