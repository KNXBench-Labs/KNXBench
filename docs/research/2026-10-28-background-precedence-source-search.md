# `DownloadInvisibleParameters="Background"`: public source search and owner decision

28 October 2026. Scope: [S2-03](../open-work/S2-03-up258-image-interpretation.md),
application `M-0001_A-9205-04-9270`. This note records which public sources
were checked for a definition of `Background` precedence, what each actually
contains, and the owner decision taken because none defines it.

## Result

No inspected public source defines how an engineering tool builds the image for
invisible (inactive) parameters under `Background`. The KNX-published schema
descriptions name the three values and nothing more. The question cannot be
settled from documents; the owner therefore chose an explicitly labelled,
observation-supported hypothesis (below).

## Sources inspected

| Source | Retrieval | Observation | What it does not establish |
| --- | --- | --- | --- |
| *Project Scheme Documentation*, Project Scheme20 v01.00.00, 22 March 2019, KNX Association (Help Center attachment `360024169360`) | Wayback snapshot 4 October 2025 of the KNX attachment, raw bytes, SHA-256 `0cae24dbe795633c6d77acac189f211936011f76e8b6371888642fee1a1f73ec`, 61 pages | Printed page 22, §1.1.2.32: simple type `DownloadBehavior_t`, string enumeration `None`, `Background`, `DefaultValue`, described in one line as the download behaviours for invisible parameters | Any image algorithm; which octets a value writes; interaction with Union defaults or `LegacyPatchAlways` |
| *Project Schema Documentation*, Project Schema23 v01.00.00, 1 March 2024, KNX Association (Help Center attachment `17389755651474`) | Wayback snapshot 3 October 2025, SHA-256 `8fe9193e701350643be5527644955d23b6933c1c4cc66264a3bb234301c21144`, 64 pages | Printed page 22, §1.1.2.32: the same type, values and one-line description | The same; the attribute name itself does not occur in the text |
| Namespace-20 schema mirror already cited in [the validation-rule sources](2026-10-20-system2-validation-rules.md) | Same revision, re-retrieved, same SHA-256 `7147f18b…b2c3` | Line 2049 binds `DownloadInvisibleParameters` to `DownloadBehavior_t`, optional, default `DefaultValue` | Semantics of any value |
| ETS v5.5.3 release notes, KNX Help Center article `360000453164` | Help Center JSON API (the HTML page is behind a bot check) | One SDK bug-fix line: the SDK reported an incorrect `DownloadInvisibleParameters` attribute value | Download behaviour of any value |
| KNX Help Center full-text search (`DownloadInvisibleParameters`, `invisible`, `invisible parameters`, `download behavior`, `DefaultValue`) and the Manufacturer Tool articles *Parameters*, *Reconstructing parameters and group objects*, *Download functions* (and its detailed variant), *Transfer parameters & flags*, *Modular Application Programs*, *Application program data sheet* | Help Center JSON API | The exact attribute name occurs only in the release note above; no article describes invisible-parameter download behaviour | — |

Correction to [the 20 October note](2026-10-20-system2-validation-rules.md):
its text search of Project Schema23 v01.00.00 correctly found none of the five
attribute names, but the document does contain the type `DownloadBehavior_t`
with the three values (printed page 22). This adds no semantics.

## Observation used by the decision

The [26 October differential](2026-10-26-up258-image-differential.md) found
that, for this application in its default configuration, a reference
engineering tool's image keeps the source segment `Data` for every inactive
parameter (94 octets in the parameter segment), and that the KNXBench
configured path, which already reads `Background` as template retention,
reproduces those octets. The superseded defaults reading (inactive parameter
defaults and one declared Union default base) does not. The value name
`Background` is consistent with "keep the background image", but a name is not
a definition.

## Owner decision (28 October 2026)

Defaults profile v3 adopts the configured path's reading: inactive parameters
keep the source segment `Data`; explicit `LegacyPatchAlways` reference
consensus and active overlays are written as before. This removes the
contradiction between two KNXBench paths for one source option. It is labelled
an observation-supported hypothesis: no public document defines it, and
agreement with one reference image establishes neither ETS equivalence nor
hardware support. Profile v2 stays readable for simulator reproduction and
recovery of existing operations; new physical execution of a v2 bundle refuses.
`None` and `DefaultValue` remain undefined; the generic System-2 entry keeps
refusing an explicit `DownloadInvisibleParameters` declaration.

Implementation and acceptance: [UP258 validation contract](../contracts/up258-system2-validation.md).
