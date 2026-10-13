# Help text format

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-100)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->
## 100. Help prose lives in the message catalogue, one paragraph per key

**Limitation.** T28's help text (ADR-0024) is stored the same way every other
user-facing string is: as entries in `apps/knx-web/src/messages/en.ts` and
`messages/de.ts`. One key holds one paragraph of plain text. There is no
Markdown, no rich text, no per-topic file, and no separate help store. The
`help.*` keys alone are 50 of them (51 with `toolbar.help`), and they are the
longest strings in either catalogue by a wide margin: adding them grew
`en.ts` by 16% in bytes while adding 7% of its keys (50 of 684).

**Cause.** A deliberate trade, made in ADR-0024 and recorded here rather than
rediscovered later. The catalogue is the one storage mechanism where a missing
German string is a *compile* error, because `de.ts` is typed as
`Record<MessageKey, string>`. Any second mechanism — Markdown files, JSON, a
help-only catalogue — would have to re-earn that guarantee, and until it did,
an untranslated help topic would ship silently. Sentence-shaped prose in a
TypeScript object literal is the price of that check.

**Consequence.** Three concrete costs, none of them fatal, all of them real.
(a) Reflowing a topic — merging two paragraphs, splitting one — is a key
rename in two files plus the `bodyKeys` list in `help.ts`, not an edit to a
paragraph. (b) No translation tooling sees this text: no translation memory,
no fuzzy matching, no `.po` round trip, so a future third language is a
manual rewrite of every paragraph rather than a diff against the last one.
(c) The catalogue is now doing two jobs — labels and prose — and prose is by
volume the larger. If a third language arrives, or if help grows past roughly
double its current size, the right answer is probably a dedicated help store
*that keeps the compile-time completeness check*; designing that is not this
task's work, and doing it speculatively would have shipped a second mechanism
with no evidence that the first one was inadequate.

**Not a rendering limitation.** Plain text is also a security choice, not only
a storage one: help paragraphs are rendered as text nodes, never through
`dangerouslySetInnerHTML`, so a translated string cannot introduce markup. A
help topic that genuinely needs a list or a table is a signal that the topic
is too long for a panel, not a signal that the catalogue needs a parser.

**Lifted when.** A third UI language lands, or the help corpus roughly
doubles — whichever comes first. Either is enough evidence to design a help
store properly; neither has happened.
<!-- END retained catalogue record -->
