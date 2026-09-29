# ADR 0048: Commissioning v1 is the verified memory path for mask 070nh; everything else is refused by name

Date: 2026-09-28
Status: Accepted
Session: 7 (integration / hardening), goal-commission K9

## Context

KNXBench can download to a device through two executors:

- `run_memory_download` (`crates/knx-net/src/commissioning/memory_download.rs`)
  writes a memory image built by `knx_productdb::image` for BIM M112
  (mask `0701h`). It is verified on one real device: `1.1.67`, MDT
  *Taster 2-fach Plus*, option C (RESEARCH §19.4; KNOWN_LIMITATIONS §136).
- The property-based `Downloader` (`commissioning/download.rs`), from
  phase 2, is verified only against this project's simulator and refuses
  hardware.

KNOWN_LIMITATIONS §7 lists what the image builder refuses by name:

- module instances;
- `Property` placement;
- unions that start mid-octet;
- other parameter types;
- `High`/`Alert`;
- `ReadOnInitFlag`.

It also lists the `[A]` rules that no PDF read backs:

- `Mask`;
- `CompareProp`;
- the 12 s wait;
- the strict load record;
- no rollback.

In the corpus, 40 of 310 programs use `LdCtrlMerge`, and 59 have steps
nothing executes.

goal-commission K9 asked the user what counts as done for v1. The answer,
2026-09-28, was *"das was am sinnvollsten ist"*: whatever makes the most
sense, i.e. the recommendation.

## Decision

1. **v1 device download = the memory path for mask `070nh`**, verified on
   one device. The CLI (`knx device download`) and the web tab (ADR-0045)
   are its only entry points.
2. **Everything else is refused by name, not guessed.** Every refusal in
   KNOWN_LIMITATIONS §7 names what it refuses, and none of them is a v1
   gap:
   - the image builder's refusals;
   - `LdCtrlMerge` programs;
   - unexecuted steps;
   - any other mask;
   - a plan that needs `A_Key_Write` (§112);
   - Master Reset on `0701h` (§136).
3. **The `[A]` rules stay as documented assumptions.** They are backed by
   the one device's behaviour, not by a PDF. A device that disagrees
   reopens the rule concerned.
4. **The property-based `Downloader` stays in the tree, simulator-only.**
   It keeps refusing hardware. Retiring it would lose the phase-2 test
   coverage of the CP §3.5.3 variants, for no gain in v1.
5. **Further device families** are added only with corpus evidence plus a
   real device of that family. They get their own ADR or goal item.

## Consequences

- goal-commission K9 is closed. v1 completeness is measured against this
  list, not against the whole corpus.
- The K8 acceptances rest on this scope:
  - §113, partial downloads: no surface offers them.
  - §93, `procedure.rs`: the v1 path does not run it.
- A user with a device outside `070nh` gets a named refusal, never a
  best-effort write.
