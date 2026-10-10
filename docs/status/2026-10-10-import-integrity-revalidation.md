# Local import-integrity revalidation — 2026-10-10

This is a new local acceptance of the existing F01–F09 implementation, not a
second implementation or a publication. The detached candidate starts at
`9a5b55fe84d6608121e99bfb3c9408235f1f4b00`; four independent synthetic regression
files strengthen same-name attribute provenance and modern completion values
across native persistence, HTTP projection and UI rendering. No production code
repair was needed on this source snapshot. See [the integrity contract](../IMPORT_INTEGRITY.md)
and [ADR-0107](../adr/0107-import-source-integrity.md).

## Actually exercised

- All 19 baseline gates and all 19 strengthened-candidate gates passed.
- Nine compiled guard-removal controls produced runtime RED, restored GREEN,
  and byte-exact restoration of each temporarily changed source file.
- Full Rust/frontend tests, builds, warning-denied Clippy and repository gates
  passed; explicit private service/native and RefId tests ran rather than taking
  their missing-corpus skip path. Private input was never copied into the checkout.
- A canonically rebuilt production CLI performed a fresh original import,
  product-store validation and network-isolated offline readiness.
- An independent bounded ZIP/XML census compared actual native/product SQL rows,
  identities, ordering, values, links, retained metadata provenance, exact lexical
  observation maps and byte-exact source retention. Its observations are not
  semantic acceptance counters.
- Independent omissions of a communication-object row and a retained metadata
  source row were detected on disposable store copies. Actual accepted stores
  stayed unchanged. Original-input byte identity was checked before and after.

Private input, source-bearing outputs and project-specific measurements remain
in an access-restricted, ignored and untracked local evidence area. Public tests
contain independently authored examples only. The source snapshots and executable
identity are bound in private receipts; this note is not a portable corpus dump.

## Boundaries and follow-ups

F10 is a separately researched declaration/inspection/model follow-up for
channels, additional addresses and configured network values. Retention is not
editing, activation, observed device state or network authorization. F11 was
reevaluated offline on the complete freshly imported model; existing refusal
reasons remain in force. Byte-order, union/bit-offset, encoding/evaluation and
non-memory-loader work need applicable specifications, independent oracles and
separate decisions. Research is not implemented execution support.

This is in-session self-review, not an independent reviewer verdict. No complete
ETS compatibility, signature verification, native accessibility, new vendor
execution support or hardware approval is claimed. This request made no commit,
main integration, push, release, deployment, upload, subagent or bus action.
