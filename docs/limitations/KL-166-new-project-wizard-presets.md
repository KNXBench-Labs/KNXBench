# New-project wizard presets

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-166)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 166. The new-project wizard seeds one installation from a fixed preset vocabulary

**Limitation.** The wizard's starting structure ([ADR-0093](../adr/0093-wizards-are-views-over-existing-commands.md))
always goes into the new project's single installation. It offers four
building-part kinds (building, floor, room, distribution board) and two
group-structure presets built from five fixed functions (lighting, shading,
heating, ventilation, central functions) and the building step's floors. The
quick floor fill names floors `Floor 1`, `Floor 2`… (generic, no regional
naming). A seed may describe at most 2,000 structure nodes; the bound is a
request-size guard checked before any ID is allocated, not a KNX limit. Group
addresses are never created by the wizard.

**Why.** User decisions of 2026-10-08 (grill-me Q2, Q10, Q11, Q17): every step
optional, nothing invented beyond what the user names, presets as data rather
than code, and no guessed language-specific floor names.

**Impact.** Further installations, other building-part kinds, individual
group addresses and devices are added afterwards in the explorer, as before.
A larger structure is built in several steps. The seed is not an undo step:
it is part of the new project's starting point.
<!-- END retained catalogue record -->
