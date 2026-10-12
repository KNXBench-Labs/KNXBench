# Issue #7 — ETS 5.7 (schema 20) project import + real ZipCrypto layout (Claude, 2026-10-12)

**Trigger:** reporter of #6 (Kubuntu, alpha.9 via the `LD_PRELOAD` workaround)
got `no known-element table for schema version 20` importing an ETS 5.7.4
project. Tracking issue #7 opened; English questions posted on #6.

**Evidence base:** xknxproject (GPL-2.0, checkout `f97db068`) test resources,
used read-only and never vendored: three schema-20 projects (ETS 5.7.1428:
plain 1 device, ZipCrypto 4 devices with Secure material, module-based 4
devices), three schema-22 projects (ETS 6.1.5686, no devices), and protected
ETS4 (schema 11, ZipCrypto) and ETS6 (schema 23, AES) samples. Its `stubs/*.json`
expected output serve as the independent oracle.

**Findings:**
1. Schema 20 = ETS6 vocabulary without `Segment` (devices under `Line`, medium
   on `Line`). `SCHEMA_20` = `SCHEMA_23` minus `Segment` plus three measured
   attributes; predicates `uses_ets6_content_model` (>=20) and
   `has_segment_level` (>=21) replace `>= 21` in parser choice, mapper,
   validator and device keys (`xpath::line_device`).
2. Real protected ETS4/ETS5/ETS6 exports store `0.xml`/`project.xml` bare inside
   the nested zip. KNXBench's ZipCrypto path (built on a synthetic fixture with
   `P-xxxx/` names) failed every real protected file with `EntryNotFound`.
   Fixed by `rooted_nested_name`; new Info-ZIP fixture `zipcrypto-flat.knxproj`.
3. Nested sub-module object ids (`…_SM-1_…`) are not decoded (8 objects, one
   device): reported `MalformedRefId`, number 0, links kept. Pre-existing gap,
   not schema-20 specific; documented in KL §177.
4. KV demo is ETS 6.0.5030, not ETS 5.7 as several docs claimed; corrected.

**Verification:** RED for the table tests and the nested-layout test; 10
mutants each killed by a named test (the first sweep found the validator
survivor, closed with a duplicate-`Space` test). Opt-in comparison: the three
ETS 5.7 projects and the protected ETS4 one agree with the oracle on every group
address, device address and link. Pitfall seen: restoring a mutant with `mv`
gives the file an older mtime than the mutant build, so Cargo keeps the stale
mutant; `touch` after restore.

**Not verified:** the reporter's project, larger ETS 5.7 installations, schema 22.
