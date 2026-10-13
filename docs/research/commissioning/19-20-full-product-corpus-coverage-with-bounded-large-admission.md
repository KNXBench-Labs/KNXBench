# 19.20 Full product-corpus coverage with bounded large admission (2026-10-09)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1920-full-product-corpus-coverage-with-bounded-large-admission-2026-10-09"></a>
<!-- BEGIN preserved record-2-029 -->
### 19.20 Full product-corpus coverage with bounded large admission (2026-10-09)

**[V]** The local product collection grew from 103 to 105 `.knxprod` files.
The previous ignored `download_coverage_corpus` test stopped at the standard
ZIP expansion limit, before any coverage result. Its full Release baseline
reproduces the typed size refusal; a separate minimal probe agrees. Originals
were rehashed unchanged before and after each completed corpus run.

The explicit offline test now uses `install_package_with_limits` with
`PackageLimits::LARGE` (ADR-0082, KL §151): 256 MiB per member, 4 GiB expanded
in total. Raw input, member-count, path, XML and atomicity guards are unchanged.
No package is excluded, and every installation error still fails the test.
This is test-only admission, not a new production or HTTP default.

A full Release measurement imports all 105 files and evaluates 467 programs
(previously 246). It adds 221 programs, not a change to the download planner:

| Outcome | Old 103-file corpus | Full 105-file corpus |
| --- | ---: | ---: |
| Verified | 1 | 1 |
| Untested, offline plan builds | 89 | 98 |
| `not-memory-mapped` | 65 | 273 |
| `parameter-evaluation` | 51 | 51 |
| `parameter-value` | 9 | 9 |
| `image-structure` | 31 | 33 |
| `procedure-contents` | 0 | 1 |
| `product-data` | 0 | 1 |
| **Total** | **246** | **467** |

Mask `0701h`: 50 programs, 43 plan (was 40/37); `0705h`: 144 programs,
56 plan (was 141/53). The second hardware-verified program (§19.19) is not
in this product-file corpus, so the verified count stays one. Aggregate
pins are updated deliberately; drift continues to fail. Diagnostics disclose
aggregate coverage, not private program IDs, names or full refusal text.
These results measure product-default offline planning, not project-specific
configuration, live writes, ETS semantic equivalence or full compatibility.


<!-- END preserved record-2-029 -->
