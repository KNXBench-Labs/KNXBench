# Converter-exception XML observations

Date: 2026-10-10. Scope: bounded read-only analysis of the owner-approved local
`knx_cvexc.xml` inputs. [Selective import ADR](../adr/0106-selective-project-import.md).
Markers follow [RESEARCH](../RESEARCH.md): **[V]** measured local/source facts;
**[A]** unverified interpretation. Private source bytes, filenames, program
identities, signatures and individual digests are not redistributed.

## Verified inventory

**[V]** Three inputs have two distinct byte contents. The inputs remained
byte-identical after analysis. The directory's apparent project-version suffix
is not the XML namespace version.

| Observed technical category | cvexc11 | cvexc14 |
| --- | ---: | ---: |
| Root namespace suffix | 11 | 14 |
| Raw table `Version` | `5` | `201` |
| Program declarations | 33 | 33 |
| Exclusion declarations | 202 | 203 |
| Explicit `Comparable="true"` | 30 | 30 |
| Explicit `Reconstructable="true"` | 2 | 2 |
| Neither option attribute present | 3 | 3 |
| Unknown observations / findings | 0 / 0 | 0 / 0 |

**[V]** Pairing by the exact observed manufacturer reference/application number/
application version/mask-version tuple matches all 33 declarations, without
unmatched tuples. Program attributes and option records are unchanged; exactly
one matched record's exclusion list differs. The added exclusion is an observed
declaration change, not proof that a device's runtime behavior changed. The two
true `Reconstructable` flags overlap the `Comparable` set; option totals are not
disjoint. Missing options stay **absent**, never inferred false/default.

## Observed structure

**[V]** The actual hierarchy is:

```text
{cvexc}ConverterExceptions
  {cvexc}ManufacturerData
    {project}Manufacturer
      {project}ApplicationPrograms
        {project}ApplicationProgram
          {project}Static
            {project}DeviceCompare
              {project}ExcludeMemory
            {project}Options
```

The exact namespace pair is `http://knx.org/xml/cvexc/11` with
`http://knx.org/xml/project/11`, or cvexc14 with project14. A local-name lookalike
under another namespace/path remains unknown. Program identity attributes are
retained as raw lexemes. `ExcludeMemory` holds a `CodeSegment` reference plus
`Offset` and `Size` lexemes; the analyzer records the observed reference and
bounded numeric span beside those raw strings. It does not turn this into an
absolute memory location or a download exclusion. Duplicate identities, invalid
numbers/options and misplaced declarations are explicit findings.

## Analysis tool

```text
python3 tools/analyze_cvexc.py /private/knx_cvexc.xml \
  --output /private/reports/cvexc-observations.json
```

The output is **private source-bearing evidence**, not anonymous: program IDs,
raw attributes and unknown XML may identify manufacturer/install data. Existing
output files are refused rather than overwritten. The original input is opened
read-only. Limits are 8 MiB input, 100,000 nodes and depth 128. DTD/entity
constructs, unsupported roots/namespaces and malformed XML are refused. Known
wrapper character content is reported as unknown, including text split by
children; foreign attributes/elements are retained, not silently dropped.
Every result sets `executable: false`.

Public synthetic tests cover both observed namespace pairs, exact raw/numeric
values, options/absence, unknown/misplaced/foreign content, wrapper text and
malformed/unsafe/over-budget input. They prove analysis behavior, not ETS or
hardware parity.

## Unverified semantics and research boundary

**[A]** No complete normative converter-exception schema/runtime contract was
retrieved. Signature verification, table-version meaning, option defaults,
merge/override precedence, reference resolution and use during comparison,
reconstruction or download remain unverified. Names alone are not operational
rules. No downloader, planner, product-database policy or bus service consumes
this analysis output.

A public KNX support troubleshooting page was initially unavailable (HTTP403).
The lead was not successfully retrieved in this session, so it supplies no
verified cvexc semantic authority. Exact-file/ConverterExceptions searches
yielded no primary format specification. This is a bounded research result,
not proof that no specification exists.

To activate any rule later, obtain authoritative semantics and authorized
export/device differential evidence, add per-rule negative/roundtrip tests, and
make a separate reviewed architecture decision. Do not silently promote this
observational tool into commissioning behavior.
