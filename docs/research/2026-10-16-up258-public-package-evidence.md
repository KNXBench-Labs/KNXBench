# Public UP 258D12 package evidence — issue #22

Date: 16 October 2026. Research-only contribution beside the opt-in test
`crates/knx-productdb/tests/system2_public_package.rs`; no production code,
mask admission or hardware operation changes. Tags as in
[RESEARCH.md](../RESEARCH.md): **[V]** measured here, **[D]** documented by a
cited source, **[A]** inference. Customer projects, keys and licensed material
stayed local and unread; this dossier draws only on the public Siemens package.

## Pinned public source

[V] `UP258_DB12__920504d_ETS5.knxprod`, the manufacturer's public download for
the reported order number 5WG1 258-2DB12
(<https://knx-td.sihub.siemens.com/product_data/ets5/UP258_DB12__920504d_ETS5.knxprod>),
SHA-256 `520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f`.
The opt-in test refuses to run on any other digest, so every observation below
is bound to exactly these bytes. The package is never added to the repository.

[V] The exact application `M-0001_A-9205-04-9270` of issue #22 installs from
it: mask `MV-0025`, `ProductProcedure`, one unmerged procedure with 21 ordered
steps and no unmodelled step, and the `TaskSegment/5` tool-metadata witness
resolves as in the [prerequisite contract](../contracts/system2-download-prerequisites.md).
Public admission stays refused (`not-memory-mapped`), unchanged.

## Measured template behaviour of the parameter segment

[V] The real-source image assembly still refuses, first at
`M-0001_A-9205-04-9270_P-901_R-901` ("Sensor_Short_Signal_T1", 16-bit,
`AS-3C00` offset 240, declared default 120): the template octets at its
placement are `00 00`, so the declared default cannot corroborate any byte
order there. [D] The source declares no `ParameterByteOrder`
(consistent with the 49-procedure census of the
[source-semantics investigation](2026-10-16-system2-source-semantics.md)).

[V] The following **selected** 16-bit declarations of `AS-3C00` show
heterogeneous agreement between declared defaults and source-template octets.
This is a representative table, **not an exhaustive placement census**. It
includes direct `Memory` declarations and parameters whose location is supplied
by their enclosing `Union`; alternatives can share a location and must not be
counted as independent storage slots or simultaneously active values:

| Observation | Fields (default → template) |
| --- | --- |
| Matches low-octet-first integer representation | P-87 (1200 → `b0 04`), P-89 (2400 → `60 09`) |
| Matches high-octet-first integer representation | P-903 (1800 → `07 08`) |
| No match in either order | P-88 (1200 → `c0 12`), P-386 (10 → `ff ff`), P-652 (500 → `00 c8`), P-653 (900 → `02 bc`), P-633 (600 → `20 03`), P-753 (400 → `20 00`) |
| Zero template, nonzero default | P-901 (120), UP-385/UP-399/UP-478/UP-500/UP-459/UP-551/UP-573/UP-532 (11802), P-446 (65535) |

[A] These agreements and disagreements are not a wire-encoding rule. This
comparison does not establish that the source template is an actual factory
memory image, nor that all declarations have been activated and written with
their stated defaults. In particular, overlapping Union alternatives are not
independent observations of runtime state. A match in either order cannot by
itself distinguish intentional encoding from coincidental source bytes; a
mismatch does not establish a second encoding convention. The
[byte-order evidence review](parameter-byte-order.md) likewise separates
source-template observations from a verified write witness.

## Consequence for the missing support

[A] Template analysis cannot close the 16-bit encoding gap of this
application, in either direction; weakening the corroboration rule or picking
a byte order from the majority pattern would be a guess written into a
device's memory. The smallest missing evidence stays what the prerequisite
contract names: a verified witness of what ETS actually writes for an edited
value of a multi-octet parameter of this application — an ETS-programmed
device read or an engineering-tool capture — plus the separately open
procedure-translation and recovery contracts. The existing
[byte-order probe](../adr/0119-read-only-byte-order-probe.md) refuses this
`MV-0025` application under its current memory-planner admission policy. This
contribution adds no alternate probe or live-write route.

## Reproducing the bounded regression

Download the exact public package above to a local path outside the repository,
then opt in explicitly from the workspace root:

```sh
KNXBENCH_PUBLIC_UP258=/path/to/UP258_DB12__920504d_ETS5.knxprod \
  cargo test -p knx-productdb --test system2_public_package -- --ignored
```

A missing file or different digest fails, rather than returning an empty pass.
The test imports into a temporary database, pins the exact `0x0400` metadata
pointer at source position 16 (zero-based), checks all other projected steps
retain their original values and order, and asserts both current refusals.
It does **not** automate the table of selected template observations, infer a
wire encoding, or execute any bus operation. The ordinal source assertions are
not a complete source-to-executor translation.

Scoped test/review execution and publication are recorded in the
[acceptance summary](../evidence/public-up258-review-2026-10-16.json).

No real bus operation, vendor-tool execution or support claim is part of this
dossier. Issue #22 remains open.
