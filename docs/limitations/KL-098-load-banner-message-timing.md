# Load-banner message timing

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-098)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 98. Almost nobody will ever see the second flavour message

**Limitation.** The load banner carries fifty rotating flavour lines
(`loadProgress.flavour.01` … `.50`, English and German), shown one at a time
beside the truthful phase label and swapped every 1 800 ms. The measured
release load of the maintainer's reference project is **114 ms**, and the
debug build's is roughly 1.2 s. At that interval a release load shows
**one** message and a debug load shows one or two. The list is written for
slow loads and large projects; a normal import will never cycle it.

**Cause.** The interval is deliberately slower than the 250 ms progress
poll, because the phase label is the news and a joke changing faster than
the news competes with it. Shortening the interval to make the list visible
would trade a real reading problem for an imaginary entertainment one.

**Consequence.** Fifty strings exist, translated twice, and the overwhelming
majority of loads render exactly one of them. They are not dead
code — every one is reachable, and the shuffle picks a different opener each
load — but nobody should expect the list to be seen as a list.

**Not a progress indicator.** Worth stating next to entry 97: the flavour
line is decoration. It is `aria-hidden`, it is excluded from the banner's
polite live region, its timer advances nothing but its own text, and it is
not rendered at all once a load has failed. Under
`prefers-reduced-motion: reduce` or the application's own "Motion: off" it
freezes on its first entry rather than disappearing.

**Lifted when.** Nothing lifts this. It is what the feature is.
<!-- END retained catalogue record -->
