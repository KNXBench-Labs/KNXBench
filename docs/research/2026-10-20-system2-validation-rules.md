# System-2 validation candidate: independently inspected rule sources

20 October 2026. Target scope: UP 258D12, application
`M-0001_A-9205-04-9270`. This investigation supports an experimental, bounded
validation candidate, not ETS equivalence or confirmed hardware support.
Owner-supplied research is a discovery aid only; no technical passage from it
is reproduced here. Each observation below comes from a separately retrieved
source. Candidate rules and simulator acceptance must be reported separately
from those observations.

## Evidence vocabulary

- **Document observation:** visible content in the identified edition. This does
  not extend that edition's scope to another format or profile.
- **Source observation:** bytes or declarations in a digest-bound input.
- **Strong hypothesis:** an explicit, reviewable candidate interpretation awaiting
  independent target validation. Agreement with a simulator cannot confirm the
  hypothesis, because a simulator also has a model.
- **Unresolved:** no usable defining rule has been established. An indispensable
  unresolved operation must refuse; it cannot disappear from a complete plan.

## Historical parameter semantics, with an edition boundary

The original KNX-authored *KNX-XML Project-Schema-v10 Description*, revision 1.2,
24 June 2011, was inspected as rendered pages 19–21 from a public mirror.[1]
The images show the edition, date and printed page numbers. Their original
publisher's signed distribution was not obtained; this is a mirrored primary
document, not a newly authenticated KNX download.

- Printed page 19, §4.3.4.2: an ordinary parameter's placement identifies its
  device storage; absence of a placement denotes a virtual parameter. Ordinary
  memory/property parameter fields may not overlap; the document directs
  overlapping representations to Union. `LegacyPatchAlways` has a false default
  and its true case includes an inactive parameter in image writing. The prose
  mentions a reference-level override.
- Printed page 20, §4.3.4.4: a Union member's octet offset is measured from the
  Union's first octet; its bit offset is relative to the addressed octet.
  `DefaultUnionParameter` is associated with image creation, but this passage
  does **not** define initialization precedence or a general last-writer rule.
- Printed page 20, §4.3.4.7: a parameter without ParameterRef declarations is
  written using its default. References share their parameter's storage/type;
  the historical document requires at most one active reference for one
  parameter. It also describes activity when no Dynamic reference exists.
- Printed page 21, §4.3.4.7: a reference's stated value replaces its underlying
  parameter's value; an absent override inherits it.

These are historical definitions, not proof that every detail was unchanged in
project namespace 20 or that this converted Siemens program is implemented
identically by every engineering tool. In particular, page 21's reference
attribute table does not itself list the patch override mentioned on page 19.
The discrepancy is retained rather than filled with an invented attribute rule.

## Independently retrieved namespace-20 schema mirror

A separate retrieval of the schema at repository revision
`b8bd908f485596c7e142bb4e21347430e0ad16b7` yields 272,346 bytes, SHA-256
`7147f18b1fca75ee23858a8693b748f9b4edaeeede35e87ccd398592e2d4b2c3`.[2]
This is a third-party copy, not an authenticated official schema distribution.
The actual declarations were read, not merely search snippets:

| Location in retrieved file | Observation | What it does not establish |
| --- | --- | --- |
| line 1668 | Ordinary parameter `LegacyPatchAlways` is optional, default false | Inactive-value precedence or reference override behavior |
| line 2013 | `ParameterByteOrder` is optional, default `BigEndian` | Every converted-legacy runtime encoding or relationship to template bytes |
| line 2049 | `DownloadInvisibleParameters` is optional, default `DefaultValue` | The exact image algorithm for explicit `Background` |
| line 2843 | `SegFlags` is an unsigned byte | Meaning or polarity of its bits |
| line 4219 | `DefaultUnionParameter` is optional, default false | Which Union initialization/overlay order the tool uses |
| feature enumeration | `AllocExtraByte` exists as a feature name | An allocation size transformation |

Project Schema23 v01.00.00 from the authorized local standard corpus was
separately text-searched for these five policy names. None occurs. This search
is a negative discovery result, not a complete visual audit of that document
and not evidence that the missing policies are unimportant.

## Property-control wire grammar

The original authorized *Management Procedures* `03_05_02` v02.01.02 was read at
printed pages 139–141; page 140 was also rendered and visually inspected.
§3.31.3.4 specifies distinct ten-octet property controls, not the memory-control
record grammar:

- Absolute data/stack controls carry an explicit octet count, expressed as the
  inclusive end minus start plus one. Access, memory type and memory attributes
  occupy separate octets.
- Memory types 1/2/3 denote zero-page RAM/RAM/EEPROM. Attribute bit 7 clear
  disables checksum control; the lower attribute bits are reserved zero.
- The absolute task control contains a two-octet start, one-octet PEI type and
  five-octet application/table identity. This field layout alone does not choose
  the correct table identity for an XML resource.
- TaskCtrl2 contains four distinct two-octet fields. Its wire layout does not
  establish a general XML LSM-to-object mapping.

The package's adjacent declared ranges are also an important consistency check:
`0116h + 330 = 0260h`, where the next range starts. A proposed extra-byte policy
must reconcile that boundary; blindly adding one would make those intervals
overlap. This arithmetic is not a definition of `AllocExtraByte`.

## Retrieval limitations and evidence retention

Direct official help requests for *Define Parameter Types* and *ETS v5.6.6*
returned HTTP 403. The configured extractor refused URL extraction; the browser
could not start because its configured real profile was locked. No browser was
stopped and no configuration or authentication was changed. The empty HTTP 202
response from the mirror's HTML document page was not credited as document
content; the separately fetched rendered pages above were actually inspected.
Full source files, rendered licensed pages and detailed HTTP receipts remain
private. Neither failed retrievals nor search snippets count as defining evidence.

## Sources

[1] https://www.yumpu.com/en/document/view/23900306/knx-xml-project-schema-v10 — original KNX-authored revision 1.2 document, mirrored rendered printed pages 19–21 retrieved separately from the public image host.
[2] https://github.com/Blizzard26/knxTools/blob/b8bd908f485596c7e142bb4e21347430e0ad16b7/src/main/resources/knx.xsd — revision-bound third-party namespace-20 schema copy; provenance limitations stated above.
