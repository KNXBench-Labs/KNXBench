# Conditional parameter-block rename: one ETS6 observation (9 November 2026)

Clean-room observation under the [evidence routes](../RESEARCH.md#evidence-routes) for
[WP-05](../open-work/WP-05-parameter-layout-fidelity.md). The project imported one
unchanged product package into an ETS6 installation, put one device on a line and varied a
single enumeration parameter. The product, the five project exports and the screenshots stay
private. Observed behaviour is separated below from what KNXBench implements
([contract](../contracts/conditional-parameter-block-rename.md)).

## Observed tool and source

- **[V]** ETS6 Demo 6.4.1, build 8718, German interface and German product text, one run.
  No language-selection dialog appeared; the displayed captions and selections matched the
  German translation entries of the product source.
- **[V]** One unchanged product package, using the KNX XML `project/11` namespace. The
  observed parameter block sits beneath a channel in the application's own `Dynamic` tree;
  the application contains no module constructs.
- **[V]** The block's controller is one enumeration parameter with the values 0, 1 and 2,
  default 0, no reference-level default override.
- **[V]** The block carries a base caption. Two `ParameterBlockRename` elements sit as leaves,
  each directly under its own `when` of a `choose` on that controller (tests 1 and 2), and each
  names the enclosing block as its target. Base caption and both replacements are three
  distinct texts, each with its own German translation entry.
- **[V]** The same product declares further blocks of the same shape, each with its own
  controller. The block in the second channel, controlled by a different parameter, stayed on
  its base caption throughout.
- **[V]** ETS converted the application to schema 23 and re-keyed its identifiers (the suffix
  changed), kept the block, kept both replacements and renamed the leaf element to `Rename`.
  The converted application member is byte-identical across all five exports and is not the
  original XML.

## Observed behaviour

| State | Controller | Navigation entry | Page-heading suffix |
| --- | --- | --- | --- |
| initial | 0 | base | base |
| changed | 1 | replacement A | replacement A |
| changed | 2 | replacement B | replacement B |
| after close and reopen | 2 | replacement B | replacement B |
| returned | 0 | base | base |

- **[V]** Navigation and page heading agreed in every settled frame. Right after the change the
  frame still showed the old caption; the settled frame came after a wait of about three
  seconds. **Nothing is claimed about whether the repaint is immediate or atomic.**
- **[V]** The selected replacement persisted over close and reopen; returning to the default
  restored the base caption with no stale replacement.
- **[V]** The export omits an explicit reference for a controller left at its default and
  writes one for a non-default value. The members of the state-2 and the reopened-state-2
  export are equal; only archive timestamps differ.

## Not observed

This run did not show the control and page of the second-channel block, module scope, a
`Rename` that targets anything but its enclosing block, competing active replacements,
replacement texts with placeholders or text parameters, restricted blocks, other ETS versions,
other languages, or whether a replacement ever hides the base caption elsewhere in ETS.
"Not observed" is a limit of this evidence, not a verdict that a case is unsupported.

KNXBench's policy for those cases is a separate decision, recorded in the
[contract](../contracts/conditional-parameter-block-rename.md):

- **Refused conservatively** (base caption kept, existing `UnrecognizedNode` diagnostic):
  module scope, renames naming anything but the enclosing block, competing replacements,
  replacement source texts with placeholders, and any target another rename leaf could also
  name on a path the evaluation did not decide.
- **Implemented as policy, proved only with synthetic tests:** a restricted block is renamed
  under its unchanged access rules, and a replacement is translated by its own identity under
  the existing language rule. That is KNXBench behaviour, not additional ETS observation.
- **Not unsupported merely because unobserved:** a second block of the observed shape is
  handled by the same rule as the observed one; this run just did not open its page.

## Limits

One ETS version, one product, one run, German text only. Not evidence about device behaviour,
the bus, downloads or commissioning, and not a compatibility claim for other products or
families. The earlier working note's claim that enumeration value texts have no translation,
and its wider product-language statement, were wrong or too strong and are not used.
