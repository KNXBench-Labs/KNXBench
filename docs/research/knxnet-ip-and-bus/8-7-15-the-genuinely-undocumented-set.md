# 8.7.15 The genuinely undocumented set

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="8715-the-genuinely-undocumented-set"></a>
<!-- BEGIN preserved record-1-024 -->
#### 8.7.15 The genuinely undocumented set

After both passes and the 2026-09-14 fix round, **six** items. Each carries the
stable identifier defined in the design spec §12; those ids are canonical across
this file, the spec and `docs/KNOWN_LIMITATIONS.md` §7, and are never reused or
renumbered. The ordinals this list used before are given in the spec's mapping
table, because all three files had renumbered independently and "item 5" used to
mean different things in different files.

1. `GAP-T30-01` — per-`Legacy*`-flag semantics (§8.6; zero hits across the
   extracted Standard corpus);
2. `GAP-T30-02` — the `LdCtrl*`-name to load-control-subtype mapping for 13 of 25
   kinds (§8.6);
3. `GAP-T30-03` — what an `EtsDownloadPlugin`/`Baggage` DLL does (compiled code;
   not documentable from either knowledge base);
4. `GAP-T30-04` — the *"differential download algorithm"* named in `03_05_03`
   §3.5.3, p. 46 — **narrowed 2026-09-19**: the trigger, goal, required
   client-side state and consistency argument are documented across three
   more sources (`03_01_02 Glossary` p. 9, RES §4.2.27.1.2 p. 39, `Project
   Schema23` `DeviceInstance` attributes p. 44); only the diffing/
   chunk-selection strategy is genuinely absent, and the Glossary's own *"may
   for instance"* marks that absence as implementation-defined — a licence,
   not a gap (design spec §12 item 4;
   `docs/spec-audits/2026-09-19-cp-3_5_3-partial-download.md` Q1);
5. `GAP-T30-07` — the unquantified *"delay for programming the memory in the
   device"* (`03_05_02` §3.16 and four sibling procedures). The footnote does name
   a reference — *"The delay time depends on the Management Server and on the
   amount of written octets (see [08])."* — and `[08]` is Chapter 3/7/2 Datapoint
   Types, which was followed: it yields only `DPT_Time_Delay` 20.013, an
   enumeration of 26 coarse delay *labels* for a Use FB, not a formula and not a
   per-device value. The gap survives being chased to its own citation;
6. `GAP-T30-08` — LSM Realisation Type 2, a gap the Standard states itself
   (`03_05_01` §4.23.3).

Plus one that is **narrowed rather than open or closed**, and is therefore counted
separately instead of being folded into the six: `GAP-T30-09`, cross-Load-State-
Machine ordering. `03_05_01` §4.23.2.4.1 delegates it to per-device Profiles
(§8.7.14), the general Profiles document states no rule, and the per-mask Profile
documents where a rule would live were not read.

Two items that used to be on this list are gone, for two different reasons, both
recorded so neither looks like an oversight:

- `GAP-T30-05` — `L_Data_Extended` discovery — **withdrawn 2026-09-13**: it is
  documented, at `03_05_01` §4.3.7 `PID_MAX_APDU_LENGTH` (§8.7.13).
- `GAP-T30-06` — the `60h` programming-mode parity computation — **reclassified
  2026-09-14**: also documented, at `03_05_01` §4.26.3.1 and §4.26.3.4.1
  (§8.7.7). Writing to `60h` on real hardware is still forbidden, but as an
  explicit project risk decision (spec §13 **R11**) rather than as a documentation
  gap. The only residual unknown — whether a given device uses odd or even parity
  — is not a gap either, because the derivation inverts the bit instead of
  computing it and therefore never needs the convention.

Each item was searched for in both KNX specification knowledge bases
(`knx_spec_kb_programming`, 27 programming PDFs with figures; the 177-PDF
text-only base) and, where relevant, in the extracted Standard corpus and §8.6's
product corpus. That naming convention is the point: it is what lets a later
reader tell "the Standard is silent" from "nobody looked".

---


<!-- END preserved record-1-024 -->
