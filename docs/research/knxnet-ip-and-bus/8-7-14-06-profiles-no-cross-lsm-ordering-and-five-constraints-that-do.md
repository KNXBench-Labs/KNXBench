# 8.7.14 `06 Profiles`: no cross-LSM ordering, and five constraints that do exist

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="8714-06-profiles-no-cross-lsm-ordering-and-five-constraints-that-do-exist"></a>
<!-- BEGIN preserved record-1-023 -->
#### 8.7.14 `06 Profiles`: no cross-LSM ordering, and five constraints that do exist

The question §8.7.9 originally deferred was whether `06 Profiles v02.01.01`
states an ordering requirement between the Load State Machines of different
loadable parts. **That document does not — and, corrected 2026-09-14, that is a
weaker result than it first looked, because the Standard explicitly delegates the
question to per-device Profiles.** `03_05_01` §4.23.2.4.1: **[D]** *"If a device
contains more than one Load State Machine, dependencies between these Load State
Machines have to be defined in the Profiles (see [17]) of these devices."* RES's
reference `[17]` is Volume 6, "Profiles". So a rule is *expected* to exist; it is
just expected to live per device.

What was searched, so the negative is auditable rather than asserted:
`pdftotext -layout` of `06 Profiles v02.01.01.pdf`, swept for ordering and
sortedness language, plus keyword queries against both knowledge bases
(`knx_spec_kb_programming.sqlite` and `knx_spec_kb_full179_clean.sqlite`) which
returned `[]`. The one grep hit was about sorting table *contents*, not load
order. **What was not read:** the per-mask Profile documents where such a rule
would actually live — `06_01_33` (mask `2705h`), `06_01_35` (mask `27B0h`),
`06_02_42` (mask `2920h`), `06_03_31` (mask `2311h`). Absence of a rule in the
general Profiles document is therefore not absence of a rule for any given
device, and the design spec's phase 2 never reorders loadable parts on the
strength of this negative. The spec's use of `03_05_03` §3.5.2's one concrete
System B order stands, and so does its refusal to generalise that order to other
masks — the absence of a general rule is not permission to invent one.

What `06 Profiles` does constrain, and the design spec now records:

- **One Table 94 cell is narrowed per mask.** §5.3 footnote a: *"The Load State
  Machine transition table allows an optional transition from state “Loaded” to
  “Error” in case of an event “Load Completed”. This is not allowed for mask
  version 0912h Couplers. Mask 0912h shall stay in state “Loaded” in case of an
  error."* **[D, corpus]** So the permitted-outcome set for a cell with
  alternatives is narrowed by mask, never widened.
- **Which Load Controls a device must support is per mask.** Annex A.2.4.1,
  *"Table 7 – Required Load Controls"*: absolute allocation and the four record
  subtypes are `M` across the System 2 and System 300 columns and `n/a` for mask
  `57B0h`; `0Ah` Relative Allocation is `M` for mask `0300h`; `0Bh` Data Relative
  Allocation is `M` for `07B0h`, `17B0h` and `57B0h`. Footnote 66: *"The
  Additional Load Controls are only required if Additional Data shall be
  downloaded."* And the NOTE that decides how to use the table: *"Table 7
  specifies globally for the device which Load Controls shall be supported. Which
  Load Controls shall be supported for the management of a specific Resource is
  specified in the (Realisation Type of) the Resource in [11]."* There is
  therefore **no fallback between allocation styles** — pick by mask or decline.
- **Authorisation is mandatory for some profiles and optional for others**, and
  the number of access levels is 4 or 16 depending on the profile. Row-level
  facts only (see the extraction caveat below).
- **A device without protected areas grants level 0 to anything.** Footnotes 10
  and 18: *"The support of the A_Authorize- and the A_Keywrite-service does not
  imply that the device itself has access protected areas. If this is not the
  case, a device shall always allow – regardless of the attributed keys – access
  to the highest level (0), including when receiving an illegal key"*. So a
  successful authorisation proves nothing about enforcement.
- **`"If Verify Mode is not implemented, it shall always be off."`** (footnotes 8
  and 16). A device may conformantly report bit 2 clear after the client sets it,
  which means §8.7.6's re-assertion rule needs a read-back that decides which
  write path runs rather than an error.
- **LSM Realisation Type 1 is not universally mandatory** — the row contains both
  `M` and `O` values — so a device without `PID_LOAD_STATE_CONTROL` is not
  necessarily faulty.
- **Access levels in Annex A are recommendations.** Annex A.1.2.1: levels are
  noted *"read access level"/"write access level"*, and Table 3's legend defines
  them as *"recommended default"* values. The `PID_LOAD_STATE_CONTROL` row's
  values across masks include `3/3`, `3/1`, `3/0`, `15/2` and `15/1` — i.e. there
  exist devices where reading the load state is permitted at the minimum level
  but **writing a load event needs level 2, 1 or 0**. That is a silent failure
  mode: pre-flight reads succeed and the first `Start Loading` write is ignored
  with no negative response.

**Extraction caveat, recorded because it limits what may be cited:** in the
`-layout` extraction of `06 Profiles`, the mask-version columns of the feature
tables in §4.2 and §5.3 are misaligned against their headers. Per-mask
attribution from those two tables is therefore **not** citable and neither this
section nor the design spec makes any. Annex A's Table 7 does survive extraction
and is cited per mask; footnotes have no columns to misalign and are cited
verbatim.


<!-- END preserved record-1-023 -->
