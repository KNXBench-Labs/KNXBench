# ADR 0086: The specification, product data and project files are enough evidence; a working inference is shipped and disclosed

Date: 2026-10-07
Status: Accepted (supersedes ADR-0048 decision 5 and narrows its decision 2)
Session: 7 (integration / hardening)

## Context

ADR-0048 admitted further device families only "with corpus evidence plus a
real device of that family". The maintainer owns a handful of test devices,
so that rule bounds the supported devices by the hardware on one desk: of
246 corpus programs 78 plan (1 verified on hardware, 77 untested), and the
house project plans 17 of 35 devices (RESEARCH §19.12). Manufacturers are
not expected to answer format questions (the maintainer, 2026-10-07: asking
MDT about `A-0019-13-B655`'s union would most likely not get an answer).

Several refusals are not caused by missing evidence of *what the product
wants*, but by the KNX documents being silent on a detail that the product
data, the project files or a read-back already settle in practice — e.g.
two active members of one union (§19.12: the device ETS programmed holds the
member later in the parameter tree), or `LsmIdx 5` steps after the final
restart on mask `0701h`, a machine `03_05_02` §3.31.2 does not have.

The maintainer's decision, 2026-10-07: *"Single Source of truth ist
momentan die Spezifikation, Produktdatenbanken und Projektdateien. […] Das
soll genügend Evidenz sein. Wenn etwas nicht in den Specs zu finden ist, du
aber eine funktionierende Lösung hast, dann weise darauf hin."* The goal is
as many working functions as possible, so that alpha testers and
contributors with other devices can use and improve KNXBench.

## Decision

1. **Evidence of record** for device support, download images and load
   procedures is:
   - the KNX specification (`knx-spec-kb`, `[D]`);
   - manufacturer product databases (`.knxprod`, the product corpus);
   - project files (`.knxproj`), including what ETS stored there.

   A rule backed by these is admitted without a device of that family. A
   real device run remains the only way to reach **Verified** (ADR-0049's
   support levels are unchanged).
2. **Where these sources are silent but a working solution exists**, it is
   implemented as a named **inference** and disclosed, not refused. A
   working solution is one that is consistent with every available source
   and reproduces what is known: a read-back of a device ETS programmed,
   the product's own base image, or the corpus. Third-party
   implementations and captures may support an inference; they are hints,
   not evidence of record.
3. **Every inference is reported** where the user decides: in the
   readiness/coverage detail of the program and in the plan the download
   acknowledgement shows, with the reason and the RESEARCH section that
   holds the analysis. It is tagged `[A]` in the code comment that applies
   it.
4. **What is still refused**: anything no source and no working solution
   supports (a pure guess), and anything where the sources contradict each
   other. Those refusals keep naming what they refuse.
5. **The safety gates are unchanged.** A program that plans only through an
   inference is **Untested**, so a write still needs the explicit
   acknowledgement; recovery, verification and the bus guards
   (ADR-0058/0059, ADR-0067, ADR-0080) apply as before.

## Alternatives considered

- **Keep ADR-0048 decision 5** (a real device per family). Rejected by the
  maintainer: it limits support to the devices one person owns and wins no
  testers or contributors.
- **Ask manufacturers first.** Not a dependable source; an answer, when it
  comes, is welcome additional evidence.
- **Ship inferences silently.** Rejected: data integrity and the user's
  decision depend on knowing which part of a plan rests on an inference.

## Consequences

- Refusals in KNOWN_LIMITATIONS §7 and RESEARCH §19 are re-examined in
  coverage order; each one that an inference lifts is implemented with
  tests, documented in RESEARCH and listed in the program's readiness.
- Coverage tests (`download_coverage_corpus`, `house_readiness`) are
  re-pinned whenever an inference lifts a refusal.
- A device that contradicts an inference reopens it; the inference is then
  replaced by the observed rule or turned back into a refusal.
- Inferences raise the share of Untested plans; the acknowledgement must
  stay readable and name them.
