# Inactive whole-byte write selection

Status: **implemented, focused software proof and independent source review
verified; maintained package acceptance and publication pending**, 30 October
2026. Baseline `a6cc2ebe99f`. The [result record](../status/2026-10-30-inactive-byte-write-selection.md)
separates executed evidence from remaining delivery work; the
[research record](../research/2026-10-30-inactive-byte-write-selection.md) explains
the source and compatibility boundaries. This is a positive regular-App-to-executor
transfer slice, not another declaration-only refusal, physical storage-preservation
or new device-family acceptance.

## Policy and evidence boundary

KNXBench must not insert inactive parameter values or directly write bytes proved
to belong exclusively to inactive parameters. This is an explicit engineering
constraint. The project-schema names `None`, `Background` and `DefaultValue` do
not independently define the manufacturer-format algorithm or every meaning of
inactivity. No token interpretation or implicit missing-option default is added.

Only the existing regular Easy3/070n memory route is in scope. Existing product
procedure order, controls, allocation, tasks, Kind/plug-in/CustomerAdjustable
checks and authorization stay intact. A configuration-limited Complete download
is still a complete procedure, not a Partial download. Existing concrete Partial
and Changes derivation follows preparation; no-pending still produces no plan.

## Positive selection

Use the existing successful Dynamic evaluator and type/placement resolution for
both current values and source defaults. Do not create another Choose evaluator.
The first supported exclusion is a closed, unscoped, supported numeric
Choose/When case with an unambiguously placed byte-aligned eight-bit field in a
proved unselected branch. Merely unreferenced or invisible is not enough.

For each candidate byte, account for active winning field bits, proved inactive
bits, necessary generated data and unresolved possible owners. Omit it only when
all eight bits are proved inactive, no active/required bits use it and no unknown
overlap defeats exclusivity. Any active reference keeps its parameter active.
An active Union alternative remains writable at the same address; existing
Union winner/conflict rules are neither replaced nor newly standardized.
Necessary source-literal bytes without parameter ownership remain present.

The omission proof also needs trustworthy structural provenance. Presence of
opaque Dynamic attributes in the retained SQL `extra` field blocks proof,
including empty or delimiter-containing values; do not parse audit text to
reconstruct attribute semantics. For a tree with Choose, resolve expanded
XML element names from the selected application's hash-checked retained original
source. Require a unique owning application, clean admitted KNX ancestry and
own Dynamic elements, and agreement with the ordered stored local-kind inventory.
Inherited namespaces, default resets, unbound prefixes and prefix rebinding must
be resolved in their actual ancestor context. A local name or NULL `extra` alone
cannot prove namespace ownership. Missing, foreign or inconsistent provenance
returns `unresolved-ownership` before omission or connector activity. Genuine
prefixes bound to the admitted KNX namespace remain supported. This is a
write-selection proof, not a global prefix ban, catalogue migration, read-only
restriction or rejection of historical restore generators.

Presence of the unqualified `DownloadInvisibleParameters` key in any own root
`Static/Options` contributes to the existing option-presence flag before repeated
Options are collapsed into the retained map. A later option must not erase an
earlier occurrence, whatever its value. Where an exclusion needs this flag,
presence prevents ownership proof; it does not interpret `None`, `Background`,
`DefaultValue` or an absent option as a manufacturer algorithm.

The source Mask is a byte permission: only `FF` permits writing; other values
protect the whole byte. Effective permission is source permission AND selection
permission. Keep original source Data/Mask and existing image/mask/plan checks;
never replace source Mask early to evade validation or reinterpret `0F` as a
transport bit mask. Stored inactive values, unknown data and read-only
Compare/Probe/image behavior remain intact.

Disjoint active/inactive bits in a writable byte are unsupported in this slice:
no whole-byte omission, template rewrite or guessed read-modify-write. Unknown
ownership and unmodelled transformations cannot produce false positive proof.
Current/default permissions must match per segment/address/length, not merely in
omission count or payload values. A topology-changing project branch is outside
this first support boundary; other active values may differ from defaults.

New payload-free category: `inactive-write-selection`. Internal reasons are
`unresolved-ownership`, `shared-active-inactive-byte` and
`different-source-default-topology`. No raw IDs, values, XML or paths in these
new errors. Preserve previous validation/error precedence and no connector/frame
activity on a new unsupported case. No behavior-changing endpoint flag or public
legacy-download mode is added. Counters use actual plan data writes, including
after Partial derivation, rather than the original image's writeable octets.

## Explicit restore generator binding

Globally filtering the old default plan would break legitimate old backups.
Restore must select one generator from explicit versioned provenance, never try
both generators, guess from region shape, or weaken existing comparisons.

| Backup format | `write_selection` | Generator |
|---|---|---|
| 1–3 | Must be absent; even null is rejected | Historical `source-mask-only-v1` by explicit version rule |
| 4 | Required `source-default-inactive-whole-bytes-v1` | New default-topological selection |
| 4 | Required `source-mask-only-v1` | Explicit historical restore generator |
| 4 | Missing, null, unknown, duplicate or wrong type | Malformed; no generator chosen |
| Unknown format | Any | Rejected |

Format 4 retains all format-3 presence/null/width rules for prior application and
exemplar evidence, strict fields/duplicates and durable owner-only create-new,
sync/reopen/directory-sync behavior. Formats 1/2 remain readable but cannot gain
an exemplar exemption. Old files are not migrated or rewritten. Older readers
may reject format 4; no downgrade compatibility is promised.

`StoredBackup` carries a typed generator identity. New regular writes and offline
readiness use the new generator. A bounded internal restore path uses the stored
identity, all current admission checks, the same concrete Partial derivation,
source-step-description comparison and exact ordered Core region/identity/
exemplar checks. No crop, padding, fallback, missing bytes or guessed defaults.
Historical restore intentionally retains its captured write shape; it is not a
claim that historical programming already met the new selection policy.

Both CLI and HTTP keepers receive identity from the actual prepared operation,
not a current global default. CLI restore passes it from the read record. A newly
observed backup taken during historical restore is format 4 explicitly declaring
`source-mask-only-v1`; the original old file remains byte-identical. Reopening
and restoring that new capture must keep the same generator. No full Dynamic
value snapshot or new global recovery/prestate-mismatch model is required.

## Historical support evidence

Historical hardware evidence does not automatically apply to a changed write
selection. Reuse requires equality of the complete ordered `MemoryDownloadPlan`
from the new and historical generators, after the same actual concrete Partial
derivation where applicable, as well as existing program/scope and inference
checks. Counts, addresses alone or matching selected payload values are not
substitutes. A changed plan is `Untested` and needs the existing CLI/HTTP
acknowledgement; this policy change is not itself an inference token. An identical
plan, including an unchanged group-address-only Partial, retains applicable
`Verified` evidence. Restore support compares the source-generated plans before
substituting captured bytes; the explicitly historical generator keeps its
historical evidence scope. No new hardware result is inferred.

## Independent behavioral oracle and required execution

Source-default-1 application bytes at `4400h`: `01 9A C7 44 6D 22`; source Mask:
`FF FF FF FF 00 FF`. Selected values are selector `01`, active Union `53`, tail
`68`; the excluded branch owns the eight-bit field at `4401h`. Direct application
data writes must be exactly:

- `4400h: 01`
- `4402h: C7 53`
- `4405h: 68`

No data write addresses `4401h`; `4404h` stays source-protected, literal `C7`
remains, and active Union data is not lost. A separate source-default-0 control
writes `4400h: 00 E2 C7 A6` and `4405h: 68`. Source-default-1/current-0 is explicitly
not a supported branch change in this first slice. These expected schedules are
independent policy oracles, not outputs copied from the planner.

The original compiled real-writer assertion failed before production edits,
with the source-default-0 imported control passing. The unchanged positive oracle
now passes through actual preparation and executor frames. Compiler, importer,
fixture, readiness and baseline failures are not semantic RED. Execution evidence
must follow preparation, frames, active-byte landing, backup and readback schedules;
a handbuilt filtered plan or helper unit does not deliver this contract. The
[result record](../status/2026-10-30-inactive-byte-write-selection.md#executed-evidence-and-attribution)
records subsequent corrections without treating later calibration or unreached
table assertions as original RED.

Acceptance includes Complete, all concrete Partials/Changes, no-pending,
CLI/HTTP Plan/Start/source freshness, no connector on unsupported cases,
read-only/save-load preservation, actual counts, error privacy and affected Web
fixture TypeScript projects. Capture-close-reopen-prepare-restore-execute must
succeed for valid link-free default-shaped Full and parameters-only controls with
independently seeded OLD bytes different from NEW data. Backup-of-restore keeps
identity. Legitimate unfiltered format-3 Full/Partial restores remain successful
and original bytes unchanged. Exercise the full format/field matrix and existing
shape, application, load-state and exemplar refusals without weakening them.

## Specification and storage limits

Primary-source attribution, checked on original pages in private evidence:

- Manufacturer Tool v01.00.01, sections 2.2.1–2.2.2, printed page 5: Parameter,
  references, presentation and shared Union storage; not a universal skip rule.
- Load Controls v01.00.02, Figure 4, page 7: template plus project modifications;
  unowned source literals are not automatically disposable.
- Project Schema23 v01.00.00, sections 1.1 and 1.1.2.32, pages 4/22: format scope
  and behavior enumeration, not manufacturer algorithm/binding proof.
- Management Procedures v02.01.02, section 3.31.2, pages 134–137: 070nh memory
  controls and absolute records without a storage-retention promise. The relative
  `0Bh` keep/fill mode in section 3.31.3, pages 141–142, is not interchangeable.
- Resources v01.10.01, section 4.23.3, page 297: this edition does not specify that
  memory LSM realization; invalidation is neither deletion nor retention proof.
- Configuration Procedures v02.01.01, section 3.9.2.4, pages 69–70, is under the
  `5705h` heading on page 66. Existing partial translation is not newly proved
  normative for 070n by that citation; no procedure rewrite is admitted here.

Transfer evidence (no inactive overlay/direct data writes) is distinct from
actual storage survival through Unload, allocation, task control, LoadCompleted
and Restart. A retain-style simulator and a plan-region backup do not prove the
latter. `regular-070n-exclusive-inactive-storage-survival` and the named partial
procedure attribution remain open; no new physical support or whole-device
recovery follows. Existing arbitrary group-link/table restore limits also remain
outside the bounded link-free positive controls. No System2 experimental, Secure,
rights, calibration, default-token or broader Union algorithm changes.
