# AR09 DPT format-width audit (2026-10-05 10:45 CEST)

Spec: 03_07_02 DPT-AS v02.02.01, 03_03_07 AL-AS v02.01.01 (Fig. 7/8),
03_05_01 Resources v01.10.01 Table 88, from local knx-spec-kb (extracts in
scratch only). New test dpt_spec_width_audit.rs: RED on 17.001 (reported 6
bit, spec 8). Root cause: codec ruling treated 6 significant bits of the
1-octet r2U6 format as eligible for the optimised 6-bit PDU. Fix: 1-octet
encode/decode, reserved bits InvalidData. Old tests pinning Short rewritten.
Mutants: encode-inline and reserved-check-removed both failed their tests;
source restored (cmp). knx-core all tests 678/0/0 + audit 3/0/0.
