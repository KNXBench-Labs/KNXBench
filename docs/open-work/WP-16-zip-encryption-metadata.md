# WP-16 — ZIP encryption metadata consistency (AES extra field)

**Status:** Implemented on its branch (26 October 2026); package acceptance pending. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Decide how project and product ZIP admission treats WinZip AES extra-field
semantics and other unsupported encryption metadata. Today such entries are
refused outright; this is not verified AES support.

## Already delivered (do not redo)

Empty-directory CRC/stream checks, exact product and project DEFLATE
extents, explicit encrypted-directory refusal (`cf5b0fb0`) and bit-6-only
consistency (`daef61b74`).

## First deliverable

Source research on the AES extra-field consistency rules, recorded in
`docs/research/`, before any code change: delivered as
[the AES consistency note](../research/2026-10-26-zip-aes-extra-field-consistency.md),
which also records the decision and the measured pre-change behaviour.

## Done when (proposed, no contract yet)

Contradictory metadata is refused by name across project, product, nested and
wrapper consumers with regressions that leave existing databases unchanged;
no decryption support is claimed.

## References

[Bit-6 research](../research/2026-10-24-zip-bit6-flag-consistency.md) ·
[encrypted-directory contract](../contracts/project-encrypted-directory-admission.md) ·
[KL-159](../limitations/KL-159-project-archive-budgets-and-duplicate-names.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-16--empty-directory-zip-integrity)
