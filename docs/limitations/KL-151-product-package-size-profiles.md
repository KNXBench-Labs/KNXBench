# Product-package size profiles

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-151)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Census 3 (18 October 2026): first member above the large profile

[Census 3](../research/2026-10-18-product-database-programmability-census.md#installer-outcomes)
measured the first package member beyond `PackageLimits::LARGE`: a 281.8 MiB
application XML in a DALI-2 vendor package, so that download has no install
route at all. A complete 2017 ETS5 database (74 MB package) installs with the
large profile in 49.9 s at a peak of at most 333 MiB and a 1,497 MiB product
database. Bounds unchanged (measurement first).

## Vendor ZIPs (18 October 2026, after Alpha 13)

[ADR-0140](../adr/0140-vendor-zips-of-product-packages-install-as-a-whole.md):
A vendor ZIP is checked with the chosen profile, and its inner packages then share
one expansion budget of that same profile. `knx products ingest <file>.zip
--allow-large-package` (and `knx products update`) accept the large profile for
vendor ZIPs. The web keeps the standard profile: a ZIP whose inner package needs
more is refused there with a message naming that inner package. The
measurement-first rule for the profiles themselves is unchanged.

<!-- BEGIN retained catalogue record -->

## §151 Real manufacturer packages exceed the product-ZIP size limits

**2026-10-09 offline regression scope.** The explicitly invoked ignored
`download_coverage_corpus` test uses the existing bounded `PackageLimits::LARGE`
profile (ADR-0082) to include complete manufacturer bundles, rather than
excluding packages or raising production defaults. Every install error still
fails the test; there is no size-refusal skip. This does not lift the web
catalogue limit or verify device downloads on hardware.

**2026-10-04 AR06V count-boundary evidence, not a limit decision.** Public
synthetic Scheme11 ZIPs assert actual reader counts:4096 entries install,
retain the exact original archive and replay without changing any database
value;4097 entries receive SizeLimit while all seeded tables/columns/values
and the original seed archive remain unchanged. Native2/0/0 and two freshly
compiled semantic controls at4095/4097 independently verified with source,
test, log and binary hashes. Canonical cap4096 never mutated. This does not
cover compressed/member/expanded byte boundaries, real manufacturer workload
resources or CLI/HTTP ownership/latency. Branch public8 all exit0,ProductDB
640/0/25, strict Clippy/fmt/doc-policy/whitespace verified; integrated gates
accepted integrated4dbca7e5 public15 recordsRust3147/0/177,Web1761,Chromium90
plus separate probe1; original startup/browser infrastructure failures retained.
New Web-owner integration/regating and publication remain pending;
no new private corpus run, production raise or KL151 closure.

**Bounded resource research verified 2026-10-03 22:02 UTC, base5540dcac.**
All853 original hashes/manifest rechecked; declared-size selection15 is seven
member-only, five total-only and three both (eight total/ten member violations).
The extra two member-only cases follow KL149 dispatch admission. A scratch-only
fixed member256MiB/expanded4GiB variant, compressed256MiB and other706 inputs
unchanged, installs14 and explicitly refuses one unsupported namespace;
production baseline refuses15 atomically. Same Release profile, fresh per-side
DBs,16 registered public hostile controls/six helper controls verified. Raised
cohort maxima:789976KiB RSS,221.55s ingest,7556988928-byte DB; total638.04s and
21970833408 DB bytes. Original/retained archive hashes and refusal empty tables
verified, private copies/DBs gone, no private raw/per-item vectors persisted.
This is measured admission, not runtime/ETS compatibility. Production bounds
remain unchanged: a blanket raise is not accepted by these corpus figures alone.
The HTTP catalog route still performs synchronous install while holding the
product DB mutex; CLI timings do not prove HTTP latency/progress/cancellation
acceptance. Reconcile caller ownership/resource guards and add cap-boundary
hostile regressions before choosing a smaller documented bound or streaming.

**Observed 2026-10-03.** `MAX_EXPANDED_SIZE` (256 MiB) and
`MAX_MEMBER_SIZE` (64 MiB) in `knx-productdb/src/package.rs` refuse 13 of
853 crawled files, plus 2 more Hager packages once §149 is worked around.

- **Expanded total > 256 MiB (8):** complete manufacturer bundles, 295 MiB
  to 2,740 MiB expanded. These include Siemens' complete
  `Siemens_HVAC_All_PDB_Oct_2023_ETS5_ETS6.knxprod` (1,006 MiB, which is
  Siemens' *only* current download) and ABB's
  `IBUS_ETS5_{ABB,BJE}_XX_V24-12-20_…` all-products bundles (2,740 and
  1,700 MiB).
- **Single member > 64 MiB (5):** individual ABB device packages whose
  application XML is 65–139 MiB (e.g. `DGS_264511_…`, `6197_46_…`).
- The refusal is clean and correct for the current limits. The limits
  remain a deliberate denial-of-service bound and are not a defect in
  themselves. But Siemens' entire public offering is currently unusable,
  and so are several real single-device packages.
- **Lifted when** the limits are revisited with measured memory/time costs,
  for example a streaming ingest or a per-package opt-in raise, while
  keeping a bound.

**Update 2026-10-05 (ADR-0082): lifted for the command line.**
`knx products ingest --allow-large-package` uses `PackageLimits::LARGE`
(256 MiB per member, 4 GiB in total); every other bound is unchanged and the
default stays 64 MiB / 256 MiB. Measured with the release CLI on the 15
size-refused public packages: 14 install and verify, one remains a scheme-10
namespace refusal (§153); worst case 760.5 MiB peak RSS, 256 s, 7.18 GiB
database; Siemens' bundle 91 s / 402 MiB / 2.5 GiB
([measurement](../contracts/PRODUCT_ZIP_LARGE_PROFILE.md)). **Residue, accepted:** the web
catalog install keeps the standard bounds, because it installs synchronously
while holding the product database; a server opt-in would need progress and
cancellation first.
<!-- END retained catalogue record -->
