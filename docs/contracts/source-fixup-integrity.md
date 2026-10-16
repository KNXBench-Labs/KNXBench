# Fixup declarations: shared image integrity boundary

P6 is integrated and published as `39c88aba`. All nineteen branch and twenty
actual merged stages pass, including isolated corpus152/152 in33targets. The
first interrupted merge attempt remains unaccepted; a fresh full run covers the
final integration including current community documentation. Local/tracking/live
main equality was verified. This session closes here; follow-up semantics remain
separate work, not implied by passing integrity tests.
The three public ProductDB image entries (`build_download_image`,
`build_system1_image`, `build_system2_image`) share one source guard before
parameter evaluation or image assembly. A retained direct `Static/FixupList`
declaration, even empty, explicitly refuses; none of these entries interprets
its transformations. Original source XML remains stored and unmodified.

## Mechanism and executed regression evidence

P5 introduced source declaration-presence retention and a System-1-only guard.
The sibling Easy3 and System-2 consumers still used the parsed source subset and
could return an apparently complete image with unhandled fixups. This does not
establish any actual prior hardware write; the regression is an image-boundary
omission found through consumer tracing and synthetic source reproduction.

Two named runtime tests failed against the uncorrected code at the intended
partial-image assertion. After centralizing the guard, both pass. They use the
real importer/reader/image builder, positive original sources, empty and populated
direct declarations, and similarly named nested extension data. The latter does
not count as a direct declaration and remains in the original source; it does
not spuriously block the image. Each refusal leaves database changes unchanged.
Existing System-1 regressions retain their exact diagnostic and source boundary.
The ordinary owning ProductDB suite passes928/0 with28ignored across56targets;
all-target Clippy passes. The complete branch gate also passes: Rust4689/0
(209ignored in345targets), Vitest3626/263files;2934source paths and six selected
external/Git commitments verified. The actual final merged run verifies those
same counts and commitments plus isolated corpus152/152. No source transformation
support is inferred from this acceptance.

Family-specific diagnostics remain `Unsupported`; early refusal intentionally
precedes unrelated parameter/placement errors. No public mask, protocol, source
schema, stored-data migration, transport or UI route is added. This is a stricter
integrity boundary: previously returned images from sources with direct fixup
declarations now refuse rather than silently omit transformation semantics.

## Original-source observations are not a patch algorithm

Seven exact retained System-1 sources in one package contain36 referenced sites
and eight distinct function IDs. The exact original package master resolves all
eight IDs through `MasterData/MaskVersions/MaskVersion/MaskEntries/MaskEntry`.
A strict independent base64 decode of the named absolute segments corroborates
all36 candidate two-octet big-endian windows with those retained target values;
windows do not overlap. This is a source/master correlation, **not** proof that
all fixups are no-ops, the target device uses that same master, or the general
patch width, instruction semantics, byte order and offset rules are established.
Do not waive a declaration guard on this correlation.

The original Application Programmer's Interface `03_06_01` v01.01.01, printed
p6 (§1.2.1 source pointers) and p15 (`U_deb30` mask-specific call-address table),
were rendered and visually inspected. The latter corroborates one selected
function target for mask0012. These inspected clauses are not a modern XML
`FixupList` transformation contract. Text-only locator searches in five selected
original volumes found no exact fixup-token body; that is neither a complete
spec audit nor proof that the required contract does not exist elsewhere.

An initial private context reader wrongly treated source Data as hex, stopped
before producing a result and was retained as a harness error. The corrected
reader independently decodes strict base64 and verifies every declared size and
source digest. No product result was relabelled by that correction. Licensed
pages, original XML, function addresses, exact identities and raw contexts stay
private; only aggregate evidence is published.

## Remaining work

Establish the authoritative source transform/no-op contract, exact profile/master
binding, allowed sites, range/overlap/mask interactions and complete source
coverage before interpreting or waiving fixups. Then obtain independently
specified output bytes through the actual image consumer. Complete real-product
images, default-procedure translation, durable whole-mutation recovery and
hardware admission remain separate and unimplemented by this correction.
