# Exact 0700 property run state: source admission and description layout

29 October 2026, PRG-01. Design/source admission only; no new runtime or hardware
acceptance. Source baseline `8d14de9ca` still supports run-state observation
only for exact 0701/0705. The [contract](../contracts/post-download-run-state-0700.md)
consolidates the independently reviewed corrected proposal.

## Original definitions inspected

Original KNX Standard v3.0.0 PDFs were independently inspected in the authorized
local corpus. Printed footers and the response figure were checked; licensed
extracts/renderings stay private. Only relevant facts are paraphrased here.

| Source | Clause / printed page | Established fact |
| --- | --- | --- |
| `06 Profiles v02.01.01.pdf` | §4.2, p37; Annex A.1.1, p133; A.2.6, p146 | 0700 Type-1 property run state is optional, not prohibited. The Applicationprogram table's mandatory 0701/0705 entries are not a mandatory 0700 declaration. |
| `03_05_01 Resources v01.10.01 AS.pdf` | §4.1.2.2 Table 7, p25 | 0700 belongs to BIM M112 / TP1; family membership does not establish every optional resource. |
| Same | §4.2.6, p31; §4.24.2.1, p298; Table 95, p299 | PID 6 reads the hosting object's executable run state. Type 1 uses PDT_CONTROL; the old PDT_UNSIGNED_CHAR exception is limited to 0020/0701, not 0700. The read is one state octet; six states are defined. |
| `03_05_02 Management Procedures v02.01.02 AS.pdf` | §3.36.3, pp157–158; §3.34.3.4, p153 | The property procedure has a conditional description step when PID 6 is unknown to the client, then start 1/count 1 value read. It does not prescribe object index 3. |
| `03_03_07 Application Layer v02.01.01 AS.pdf` | §3.4.3.1, pp55–58, Figures 29/43 | Description Read/Response APCIs are 3D8/3D9; named PID addressing ignores the request index. A response may contain an actual or echoed index. Its payload after both header octets is 3/7 octets. |
| Same | p56; p57 footnote 4; p58 | Zero write/type/count/access reports a description problem without a unique cause. Management clients must not evaluate legacy bit 6. Maximum elements can be zero to indicate a problem. |
| `03_04_01 Application Interface Layer AS v02.01.01.pdf` | §4.3.2.2, pp15–16 | The upper four maximum-elements bits are described as an exponent, unlike the AL figure's reserved-zero presentation. This unresolved conflict is not permission to invent capacity semantics. |
| `03_07_03 Standardized Identifier Tables v01.04.01 AS.pdf` | §4, Table 6, p18 | PDT_CONTROL is 00h; PDT_UNSIGNED_CHAR is 02h. Zero is a real datatype, not an undefined marker. |

## Publicly traceable corroboration, not a replacement wire definition

KNX Association's public Project Scheme Documentation, attachment
`360024169360`, §1.1.2.15 `PropType_t` (p13), lists PDT_CONTROL and
PDT_UNSIGNED_CHAR as distinct property-type names. Its public Falcon note
“Since older versions”, article `360000921624`, lists property-description read.
These official public sources corroborate names/capability only; they do not
establish the byte layout, numeric PDT code, maximum-elements interpretation or
0700 client admission policy used here. No foreign implementation was copied.

## Corrections and conservative policy

The response has seven payload octets: object/PID/index, one type/flag octet,
two maximum-elements octets, one access octet. Nine includes both header octets;
eight is the suffix after only the first. An earlier “body 8” lead conflated
coordinates and is superseded. The raw `[03 D9]` codec specimen is not a valid
connected management response without the actual numbered-data TPCI state.

A zero semantic descriptor does not prove absence, lack of authorization or a
broken device. Mixed zero-count metadata likewise has no independently defined
error category. Preserve legacy bit 6 without evaluating it. Preserve but refuse
to qualify nonzero upper maximum-elements bits rather than resolve the AL/AIL
conflict by guessing. Positive lower count plus PDT_CONTROL qualifies the
bounded value read; write/access metadata grants no new authority.

The Standard does not define every meaning of “unknown to the client”. Requiring
fresh description evidence for exact 0700 is **KNXBench's conservative policy**
for an optional property, not a claim that every client must always do so. A
one-byte value cannot establish its PDT. Simulator evidence is therefore
explicitly independent of value width/presence and mask, not synthesized from it.

The current shared matching record drops observed mask. The new optional,
non-null 0700 marker is an explicit compatibility/provenance decision. Older
strict readers may refuse new records; no old record is rewritten or inferred.

## Residual boundaries

The property encoding does not define the separate memory realization.
Resources §4.24.3, p302 leaves Type 2 unspecified; its memory octet interpretation
remains `MISSING_MASK070NH_MEMORY_RUNSTATE_ENCODING`. Generic object discovery,
relocated or multiple Applicationprogram selection, broader profile support,
physical function and whole-mutation recovery remain outside this increment.
No device, bus, VM, new retry/wait, release, deployment or vendor contact occurred.
