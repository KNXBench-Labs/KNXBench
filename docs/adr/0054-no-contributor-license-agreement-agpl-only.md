# ADR 0054: No contributor license agreement; contributions come in under the AGPL alone

Date: 2026-09-30
Status: Accepted
Session: licensing (user request, outside the numbered sessions)

## Context

ADR-0053 added `CLA.md` on the same day, so that KNXBench could later be
dual-licensed. Before any outside contribution arrived, the user decided to
keep the original situation: `AGPL-3.0-or-later`, no CLA.

Two findings from reading the German copyright act (UrhG, gesetze-im-internet.de,
read 2026-09-30) made the CLA weaker than ADR-0053 assumed. Neither has been
reviewed by a lawyer:

- **§ 40 UrhG.** A contract granting rights of use in future works that are
  not specified, or specified only by kind, needs written form, and either
  side may terminate it after five years; that right cannot be waived in
  advance. CLA §7 ("for this and all my future contributions") was accepted
  by a pull request sentence and checkbox, which is not written form.
- **§ 32 UrhG.** An author may claim equitable remuneration for granted
  rights of use. Paragraph 3 exempts only a royalty-free *simple right of
  use for everyone*. The CLA granted rights to the maintainer alone,
  including commercial sublicensing, so the exemption probably did not
  apply.

Also verified: no outside contribution was ever made under `CLA.md`. The
repository is private, has no forks, and all merged pull requests (#1 to #3)
came from the maintainer.

## Decision

KNXBench stays `AGPL-3.0-or-later`, and contributions are licensed to
everyone under that same license, as before ADR-0053. There is no CLA and no
copyright assignment. `CLA.md` and the pull request template it needed are
removed; README, FAQ and contributing guide return to their earlier text.

Everyone may use KNXBench commercially under the AGPL. A closed product based
on it is not possible under the AGPL; the maintainer may still license his
own code on other terms, but not code contributed by others.

## Alternatives considered

- **Keep the CLA and weaken it to a per-PR agreement.** This addresses § 40,
  but not § 32, and it still needs a lawyer before the first outside
  contribution. The user chose not to.
- **A source-available license (PolyForm Noncommercial, Prosperity, BUSL,
  FSL, PolyForm Shield).** These forbid commercial use or competing
  products directly, but KNXBench would no longer be open source, and a
  noncommercial license would also forbid paid installation work. The user
  rejected them.

## Consequences

- Once outside contributions are merged, the project can only be relicensed
  or commercially licensed with every contributor's consent. That is
  accepted.
- Nothing has to be checked in pull request review for licensing beyond the
  existing `cargo deny` gate for dependencies.
- KNOWN_LIMITATIONS §148 is withdrawn: there is no CLA left to review.
