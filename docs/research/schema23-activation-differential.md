# Schema-23 saved activation: independent differential boundary

Date: 2026-10-11
Status: delivered diagnostic-only on `70c05fdc58e5`; KL-146 retained

## Primary source

KNX Association, *Project Schema23 v01.00.00*, page49,
sections1.2.5.13–1.2.5.15. Direct PDF retrieved on2026-10-11 and the table
visually inspected; this is not a search-index excerpt.

Source: <https://support.knx.org/hc/en-us/article_attachments/17389755651474>

The tables distinguish the object’s optional `ChannelId` program-channel
reference from a project `ChannelInstance`’s required `Id`. The instance’s
optional `RefId` references a Dynamic channel with its parent prefix removed;
a user-created channel need not have that reference. `IsActive` is optional
and the table supplies no default. Absence is therefore not evidence of false.

These attribute descriptions do not specify Repeat expansion, MI mapping or
complete ETS evaluation behaviour. They do not establish that a saved snapshot
is current device/bus state.

## Test contract

`apps/knx-server/tests/private_saved_activation.rs` performs real project/product
import and HTTP reads, then compares against the independent source reader in
`tests/common/saved_activation.rs`. Neither reader copies production XML mapping.

1. Reconcile every source/imported device and tree-object identity and every HTTP
   object identity. Reject duplicates, malformed/unsupported selected XML, root or
   namespace/ancestry lookalikes and budget overflow rather than shrink the census.
2. Keep an explicitly empty tree distinct from an absent tree. Only the declared
   tree enumerates objects; outside-tree overrides remain source evidence. Existing
   stored-activity semantics are ADR-0014 and IMPORT_EXPORT9.3, not a new inferred
   producer algorithm.
3. Compare evaluated `Active`/`Inactive` separately from `Undetermined` and
   `NotEvaluated` (ADR-0050). Detect extra positive unscoped activation against
   an explicit saved tree. Unsupported repeated/scoped-set identity remains
   unadjudicated; no counter interpretation or guessed ownership is added.
4. Compare an explicit unscoped object `ChannelId` with the positively activated
   owner’s program element identity, not the project channel’s `Id`. Observe
   explicit channel activity only where a matching unscoped owner was reached.
   An empty/user-created/missing/scoped channel with no such observation is not
   labelled inactive. Thus this is not full channel activation parity.
5. Keep original bytes/project snapshot unchanged. All original-derived evidence
   stays ignored/local; assertions and public admission report only booleans.
   Explicit missing opt-in/file must refuse, never skip as success.

Eight synthetic controls and focused strict Clippy pass. The first complete
original run found evaluated-object differences/extra unscoped refs, with source
and project unchanged. Its channel-identity finding isolated a harness assumption:
source ChannelId parent-relative spelling versus database-qualified IDs. A new
synthetic RED/GREEN control accepts only exact or own-program-prefixed spellings
of the same channel, never a project instance Id. This is an identity comparison,
not an expansion/inactivity inference. Corrected original comparison completed with remaining findings; no
parity acceptance. Owner then chose read-only diagnostic delivery: successful
admission requires complete coverage and unchanged input/project, not agreement.
Findings/unknowns are preserved, with explicit `parity_accepted=false`. Final
source-bound diagnostic replay passed on `f045390fdf7b`: complete coverage, unchanged
original/project and preserved differences/unknowns, with no parity acceptance.
The same source passed full non-desktop Rust/Web/build/repo/corpus and the
independent RefId, instance-own-value, reconciliation and raw-evidence gates.
Three compiled assertion mutants were detected and restored before this run.
Prior harness-budget refusal and interrupted wrapper timeout
are not accepted evidence. Production semantics and KL-146’s retained boundary are
unchanged. No ETS/Secure/hardware compatibility or release claim follows.
