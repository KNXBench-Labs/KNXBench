# UP258 System-2 image: classification of remaining engineering-tool differences

Date: 2026-10-26.

## Scope and evidence boundary

This note classifies the byte differences that remained between KNXBench's
experimental UP258 (`M-0001_A-9205-04-9270`, mask `0025h`) candidate images and a
private, simulator-only reference-tool observation. The observation is a test
oracle, not a normative source: no device address, memory content, parameter
value, capture or provenance detail is recorded here, and no code was changed to
reproduce observed bytes. Every rule adopted below rests on the public application
source (package digest `520d3fce…`, application digest `d06a04eb…`, see the
[validation profile](../../data/system2-validation/up258d12-920504-v2.json)) or on
the cited specification clause. Simulator agreement is not hardware compatibility
and not ETS equivalence.

Before differences were classified, input equivalence was established: the
reference tool's application definition matched the public source element for
element (only tool metadata, schema namespace and translated texts differed),
and its configuration was taken from its own project export, so both sides
prepared the same desired configuration rather than a defaults candidate against a
differently configured project device.

## Class 1 — independent source octets beyond declared table capacity (defect, corrected)

The association-table segment `AS-0260` (416 octets) declares
`AssociationTable MaxEntries="200"`. With the indexed legacy grammar (count octet
plus two-octet entries, Resources v01.10.01 §§4.17.3.3.1–4.17.4.3.1, already
cited by `legacy_group_tables`), the declared reservation ends 401 octets into the
segment. The remaining 15 octets carry source `Data` with a writable mask: they
are independent application content, not table capacity.

Defaults profile v1 used the procedure policy
`declared-counts-zero-flags-full-table-identity-v1`, which writes only the computed
table span of a table segment and therefore never wrote those 15 octets. The
project-configured path already used `source-capacity-preserving-independent-data-v1`.
Defaults profile **v2** adopts that existing, reviewed policy; everything else in
the profile is unchanged. Profile v1 stays readable for existing evidence and
recovery, but new physical execution with a v1 bundle refuses.

## Class 2 — invisible-parameter precedence (unresolved, no change)

The application declares `DownloadInvisibleParameters="Background"`. The
defaults policies interpret inactive parameters by writing their defaults
(and one declared union default per union) over the segment data; the
project-configured policy retains the segment data for inactive parameters and
overlays only active references. The two named hypotheses produce different
bytes only for inactive parameters: every active overlay, including multi-octet
numeric values written high octet first, agrees with the oracle. The oracle agrees
with template retention. No public document found defines the precedence behind
`Background`; Project Scheme20 v01.00.00 (printed p. 22) only names the values
and the namespace-20 schema mirror only declares the enumeration. The precedence
therefore remains an explicit open interpretation, and the defaults profile keeps
its v1 image policy.

## Class 3 — communication-enable policy for unlinked and inactive objects (tool policy, no change)

The remaining object-table differences follow one rule on each side, without
exception across the tested configurations. KNXBench encodes an active object's
effective flags (its communication-enable bit from the product/instance flags,
also when the object has no association) and keeps an inactive object's base
octet with communication enable cleared. The oracle clears communication enable
for every object without an association and encodes inactive objects from their
product flags with communication enable cleared. Flag overrides on linked
objects, including Update Enable in the Type-2 config octet, are identical.

Resources v01.10.01 §4.18.3.1.2.1 (printed pp. 263–264) makes read, write and
transmit behaviour conditional on communication enable, and a cleared bit means
messages for the object are not handled. Inactive descriptors are therefore
inert on both sides; an active object without association has no group address
to transmit or receive on. No clause obliges a tool to clear the bit on unlinked
objects, so this stays a documented tool-policy difference, not a defect.

## Configured differentials

With the reference tool's own exported configuration as input, the KNXBench
configured path produced, for a defaults configuration, one changed multi-octet
parameter, added group addresses with an extra receive association, and changed
object flags, the same write set as the reference and the same expected deltas
between variants; only Class 3 remained. The first transmission of a configured plan through the simulator gateway exposed
a separate runtime defect: the CLI's current-thread runtime stopped acknowledging
tunnel frames during multi-second checkpoint persistence, the gateway dropped the
connection as Tunnelling v01.07.01 §2.6.1 (printed p. 9) prescribes, and the run
stopped at the recovery barrier without a write. With that corrected (a
multi-thread tunnel runtime for the System-2 commands), all four configured runs
finished through the simulator gateway; the transmitted bytes equalled each
prepared plan, frame types matched their lengths, nothing was written outside the
reference image, and the activity journal recorded verified write evidence.

## Not changed and still open

TaskSegment identity, the omitted program-version octets, `FEh` handling, the PID 7
width blocker, gateway PID 56/83 assumptions and physical recovery are untouched.
The invisible-parameter precedence (Class 2) and the communication-enable tool
policy (Class 3) stay open until a defining source exists.
