# Inactive whole-byte selection: source, provenance and compatibility findings

Observed: 30 October 2026. Scope: the existing regular Easy3/070n memory route.
This records the implemented bounded policy and the evidence behind its
corrections; it is not a recovered ETS algorithm or package-delivery receipt.
[Binding contract](../contracts/inactive-byte-write-selection.md) ·
[Focused result and pending acceptance](../status/2026-10-30-inactive-byte-write-selection.md).

## Primary-source boundary

The original-page checks already recorded in the
[contract's specification references](../contracts/inactive-byte-write-selection.md#specification-and-storage-limits)
remain the primary-source basis. No new proprietary text or payload is reproduced,
and this documentation pass did not reopen source PDFs or acquire new sources.

| Primary reference | What it supports here | What it does not establish |
|---|---|---|
| Manufacturer Tool v01.00.01, §§2.2.1–2.2.2, printed p.5 | Parameter/reference/presentation distinctions and shared Union storage | A universal rule to skip invisible or unreferenced parameters |
| Load Controls v01.00.02, Figure 4, p.7 | Source template plus project modifications | Permission to discard unowned source literals |
| Project Schema23 v01.00.00, §§1.1, 1.1.2.32, pp.4/22 | Schema scope and declared behavior enumeration | Manufacturer algorithm, token-to-writer binding or an absent-option default |
| Management Procedures v02.01.02, §3.31.2, pp.134–137 | 070nh memory controls and absolute records | Storage retention; relative `0Bh` keep/fill in §3.31.3, pp.141–142 is not interchangeable |
| Resources v01.10.01, §4.23.3, p.297 | This edition leaves the memory-LSM realization unspecified | Retention or deletion merely because data is invalidated |
| Configuration Procedures v02.01.01, §3.9.2.4, pp.69–70, under `5705h` on p.66 | The cited procedure's actual scope | New normative attribution of existing Partial translation to regular 070n |

The admitted rule is therefore explicitly **KNXBench policy**: no inactive-value
overlay and no direct data write to an exclusively inactive whole byte, only when
ownership and source-default topology are proved. `None`, `Background` and
`DefaultValue` remain uninterpreted here. This does not supersede the separately
bounded experimental configured System-2 policy.

## Source-backed ownership, not another evaluator

The [write-only selection implementation](../../crates/knx-productdb/src/image/inactive_selection.rs)
uses the existing successful Dynamic evaluator and type/placement resolution for
current values and source defaults. It inventories inactive candidate bits,
active winning overlays, other possible owners and required generated data before
omitting any byte. Only aligned eight-bit fields in proved unselected supported
numeric branches are admitted. Source Mask remains a separate whole-byte
permission; `FF` permits a write and other values protect the byte. Active Union
winners and required literals are not inactive storage to remove.

Three correction findings sharpen that proof without broadening the supported
Dynamic language:

1. **Raw option presence survives repeated roots.** Repeated own root
   `Static/Options` can overwrite a retained last-map view. The unqualified
   `DownloadInvisibleParameters` presence flag must be accumulated before that
   collapse. An earlier empty, false-looking or unknown value cannot be erased
   by a later markerless Options node. Presence prevents proof where an inactive
   exclusion needs it; it does not define the option's value domain.
2. **Opaque attributes are evidence, not parseable audit grammar.** The retained
   SQL `extra` field preserves unknown attribute presence, including empty,
   newline-containing and equals-containing values. Its NULL/non-NULL state is
   consumed directly by the write proof. Splitting audit text would lose evidence;
   no attribute meaning is reconstructed and even skipped branches participate
   in the proof.
3. **A local name is not an expanded XML name.** The existing structural tree
   retains local kinds, and ancestor namespace declarations need not appear in
   node audit attributes. Consequently NULL `extra` does not prove clean element
   provenance, especially for genuinely old rows. The bounded
   [retained-source namespace proof](../../crates/knx-productdb/src/image/inactive_selection/namespace.rs)
   loads the actual selected application's original source and checks its SHA-256.
   `NsReader` resolves Start/Empty/End events with real inherited contexts. A
   unique application, clean admitted KNX ancestry and own unscoped Dynamic
   elements, plus equality of the ordered source/stored local-kind inventory,
   are required. Foreign wrappers, references and controls, an unbound prefix,
   inherited foreign default, default reset or prefix rebinding cannot become
   proof merely by retaining a familiar local name.

Unprovable provenance omits nothing: the program keeps its regular plan
(byte-precise fallback, 31 October 2026; it originally returned the payload-free
`UnresolvedOwnership` refusal before connector activity). Known prefixes actually bound to
the admitted KNX namespace remain positive, including declarations at KNX and
ApplicationProgram ancestors. There is no database migration, global prefix ban,
new old-row default, global old-format ban, second Choose evaluator or read-only
restriction. No-Choose paths and explicitly historical restore generators retain
their prior scope; original XML, normalized rows, source Data/Mask and stored
project values are not rewritten to manufacture proof.

## Historical evidence follows the actual scoped plan

The [App support grading](../../crates/knx-app/src/download_support.rs) compares
complete ordered `MemoryDownloadPlan` values from the new and historical
generators. Both undergo the same actual concrete Partial derivation. Program ID,
scope, count equality or an unchanged subset of payload bytes cannot transfer an
old hardware claim to a changed plan. If selection changes that scoped plan, it
is `Untested` and the real CLI/HTTP write requires the existing acknowledgement.
Truly identical and unchanged group-address-only scoped controls retain applicable
`Verified` evidence, subject to existing inference and evidence rules. Restore
compares generated source plans before captured old bytes are substituted.

This is evidence attribution, not additional hardware verification. It avoids
both automatic transfer of old evidence and an indiscriminate downgrade of every
unchanged scope.

## Explicit generator provenance preserves legitimate restores

A global change to the restore default would make a valid old backup's unfiltered
regions disagree with the new filtered shape. Trying both generators, cropping or
padding regions would weaken integrity rather than solve provenance.

Format 4 therefore requires one typed, known, non-null `write_selection`:
`source-default-inactive-whole-bytes-v1` for a new regular capture that omitted a
byte, or `source-mask-only-v1` for a regular capture that omitted nothing and for an
explicitly historical restore capture. Formats 1–3
must omit the field, including null; their version selects the historical
generator without rewriting the original. Existing format-3 strict fields,
application/exemplar checks, exact ordered regions and durable owner-only
create-new persistence remain. Formats 1/2 remain readable, not newly restorable.
Unknown, missing, duplicate, null or wrongly typed format-4 binding refuses; older
strict readers may reject format 4.

The actual prepared operation carries this identity into CLI and HTTP backup
keepers and restore preparation. A new backup taken during historical restore
must itself be format 4/historical, and retain that identity through reopen and
another restore. Genuine pre-serializer format-3 complete/parameters-only files
and a real CLI capture/reopen/restore chain establish this compatibility boundary;
original backup bytes remain unchanged. There is no default snapshot, fallback
generator or global backup/recovery redesign.

## What execution establishes

The [result record](../status/2026-10-30-inactive-byte-write-selection.md#executed-evidence-and-attribution)
retains separate original-writer, correction and namespace epochs. In particular,
a real pre-namespace executable created the original catalogue before
recompilation, and that same database was genuinely reopened before correction.
Original source/binary/database identities were retained rather than simulating
old rows with current code. The independently fixed positive schedule remains
`4400h: 01`, `4402h: C7 53`, `4405h: 68`, with no direct data write at `4401h`;
`4404h` stays source-protected. Actual App, built CLI and HTTP consumers exercise
positive and, since the 31 October byte-precise fallback, regular-plan fallback
paths where they originally exercised refusals.

Only reached original assertions earn RED credit. Later table rows and later
checks in a compound test are not retroactive failures; later semantic mutants
are calibration, not original RED. Focused regressions, source review and static
checks do not stand in for the still-pending maintained package gates or runtime
build/smoke.

## Unresolved physical and semantic questions

`regular-070n-exclusive-inactive-storage-survival` remains **OPEN**: absence of a
direct transfer does not prove byte retention through Unload, allocation, task
controls, LoadCompleted or Restart. A retain-style simulator and a plan-region
backup do not establish that contract. The named 070n Partial-procedure attribution
also remains unresolved. Prior device evidence is not transferred to the new
policy without actual plan equality, and no new hardware operation occurred.

Broader inactive/default/customer-write semantics, editable Dynamic and Union
rules, other placements/encodings and runtime/object layouts, indirect mutation
effects, arbitrary table/link restore, whole-device recovery and historical
corpus/category provenance remain independent open work. Neither PRG-01 nor
KL-007 closes, and no ETS parity or new device-family acceptance follows.
