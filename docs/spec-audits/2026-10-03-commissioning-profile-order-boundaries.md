# Commissioning profile-order boundaries — 2026-10-03

Scope: bounded offline follow-up to GAP-T30-09, using directly read local
KNX Standard primary PDFs, not knowledge-base snippets as final evidence.
No device/USB/RF access, payload execution or new download support is implied.

## Direct evidence [D]

| Primary source (literal filename) | Clauses / printed pages | Established boundary |
|---|---|---|
| `06_01_33 mask 2705h v01.01.02.pdf` | §4.2.7 p17; §4.6–4.7 pp23–24 | Names property-based and memory-mapped LSM forms and references MP/Resources; linking/application tables are not a verified universal execution order |
| `06_01_35 mask 27B0h v01.01.02.pdf` | §4.2.7 p17; §4.6–4.7 pp23–24 | Similar service references, but no standardized System B memory layout in §4.2.9–4.2.10; do not derive placement from another mask |
| `06_02_42 mask 2920h v01.01.01.pdf` | §2.4.2.10 p17; §2.4.5–2.4.6 p22; §2.5.1 p23 | Coupler profile explicitly excludes memory-mapped Type 2 despite a following reference table; application handling is not a standard coupler feature and object indexes are not generally fixed |
| `06_03_31 mask 2311h v01.01.01 AS.pdf` | §1.2 pp4–5; §1.3 pp5–7; §2 p8 | Standalone RF USB interface is not an ordinary remotely managed application device; a combined end-device mask has separate identity and management constraints |

Extraction used `pdftotext -layout`; chapter bodies and printed footers were
checked, not merely tables of contents or adjacent PDF columns. All four PDFs
were present and readable. This does **not** establish absence of all other
Profile evidence or substitute for following every cited Resources/MP clause.

## Source inconsistencies retained, not guessed away

The 2705h filename says 01.01.02 while its printed footer says 01.01.01.
The 27B0h linking section has mismatched mask/realisation/reference labels;
these were not normalized into an implementation rule. The 2920h explicit
prohibition takes precedence over treating a nearby service-reference table as
permission. Profile-local Type-2 terminology does not establish every general
Configuration Procedure discriminator or its payload/placement mapping.

## Application policy [A] and residual evidence

Keep the already verified, specific ordering/refusal behavior. Do not widen the
executor to RF, USB, couplers, duplicate part kinds or undocumented placement
based on these tables. The bounded investigation is complete; a new variant
still needs an unambiguous exact-profile dependency/placement mapping and
independent sequence/image fixtures before changing admission. Thus GAP-T30-09
moves from an unperformed offline audit to a **documented reference boundary**,
not complete download parity. GAP-T30-08 likewise remains refused where its
actual executor mapping is unproved. No hardware authorization is transferred.
