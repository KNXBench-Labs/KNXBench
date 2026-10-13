# Documentation history and research reorganization

Date: 2026-10-13
Scope: the four owner-selected large documents; no product behavior, format,
protocol, compatibility, formal task status, release or deployment change.

## Layout

| Reading entry | Complete detail/history |
| --- | --- |
| `.ai/CURRENT_STATE.md` | `.ai/archive/CURRENT_STATE_main_2026-10-13.md` |
| [Delivery index](../IMPLEMENTATION_STATUS.md) | [ISO-week history](implementation/README.md) |
| [KNXnet/IP and bus index](../research/knxnet-ip-and-bus.md) | Individual dossiers under `docs/research/knxnet-ip-and-bus/` |
| [Commissioning index](../research/commissioning.md) | Individual dossiers under `docs/research/commissioning/` |

The handover keeps current summaries per track: release/reporter follow-ups,
GA feedback, encoding/parameter prerequisites, optional MCP client acceptance,
private/parked work and completed documentation. Old lock reservations are not
renewed. Full published history stays in the archive, not in the agent startup
file. Private root-only histories stay local and are never added to publication.

Delivery entries are placed by their **own date** into ISO-week files. Nonmonotonic
entries are not silently assigned to the surrounding date. The original section
order is retained within each period; the preservation receipt reconstructs the
original global ordering. All headings, aliases, technical paragraphs, failure
records and source references are preserved rather than rewritten as new facts.

Research is partitioned at numbered subjects and individual experiment/correction
headings. The old research paths are indexes retaining all original fragments.
They link to the corresponding dossier, not merely to an unrelated root page.
The evidence vocabulary **[V]/[D]/[A]**, original dates and global section numbers
remain unchanged. Historical live operations are never new write permission.

## Maintenance

- Handover: update the relevant current track and timestamp; archive full originals
  at explicit checkpoints. Preserve every active, waiting, parked or unknown
  owner decision and the newest lock/queue disposition. Never cut by age alone.
- Deliveries: new records can be prepended to the main index. At the next archival
  checkpoint move the complete records into their own dated period and retain
  old fragments. Existing September history is not duplicated by this package.
- Research: add a dated update outside the dossier's preserved block or add a new
  dossier with an index link. Keep the small index; do not prepend full trial logs.
- Do not independently duplicate statuses from [LEDGER](../status/LEDGER.md).
- Do not edit frozen original records to erase older failures or make a gate pass.
  Append a correction with its evidence instead. Update both plain file links and
  fragment aliases if a future reviewed migration changes a dossier location.

## Preservation and verification

The [source receipt](../evidence/documentation-reorganization-2026-10-13.json)
records source revision/digests, every contiguous original section, its destination,
all original aliases, and each reversible local-link rewrite. The automated
checker verifies retained blocks, inversely restores every link, and reconstructs
the complete original source digest and order. A status log with late older dates
and fenced heading examples has dedicated regressions.

Commands:

```sh
python3 -m unittest discover -s tools/tests
python3 tools/check_documentation.py
cargo test -p xtask
cargo run -p xtask -- check-anchors
cargo run -p xtask -- check-ledger
```

The ordinary documentation gate checks the history receipt; CI runs the history
regressions as well. This is source-document preservation, not new hardware,
private-corpus, vendor, browser or full-product acceptance.

Architecture/model/import contracts, Flow, the test catalogue and the canonical
ledger were not restructured by this package. Broader audit/planning proposals
remain separate decisions; this documentation go starts no product goal.
