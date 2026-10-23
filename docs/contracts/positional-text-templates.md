# Positional text placeholders `{{0}}` — Goal B14, issue #17

Product texts can carry a placeholder that ETS fills with the value of a text
parameter, so a channel can be called "Kitchen" instead of `{{0:Channel A}}`.
This contract says what KNXBench fills in, from where, and what it leaves
alone. Evidence: [research, 2026-10-14](../research/product-database.md#2026-10-14--positional-text-placeholders-0-goal-b14-issue-17);
decision: [ADR-0126](../adr/0126-positional-text-placeholders.md).

## Where it applies

- **Hosts:** a `Channel`, `ParameterBlock` or `ParameterSeparator` in the
  evaluated parameter tree, and a `ComObjectRef` (its own `@Text` or the one it
  inherits from its `ComObject`). The host must carry a `TextParameterRefId`.
  A `ParameterRef` host is allowed by the Manufacturer Tool documentation but
  absent from the corpus; it stays unmodelled and reported.
- **Surfaces:** the device's communication objects (names and channel
  headings), parameter-block captions and activated separator text (row view and
  declared-grid labels), and the documentation report's object names. One product-database
  helper applies the rule; object names go through one `knx-app` function shared
  by the device detail and the report. For object names and channel headings,
  resolution runs after the translation overlay, on the translated template.
  Block and separator captions now apply the requested product-language `Text`
  overlay by exact defining-tree/node identity before module-argument and
  positional substitution. The existing exact/prefix language rule and original
  source fallback are reused. Translation is presentation-only: `Value` is never
  overlaid and original product templates/project values remain unchanged.
- **Only product texts.** An object name at the `Program`/`ProgramRef` layer is
  resolved; a name the project owns (`Instance`, `UserEdit`, `Inferred`) is
  shown exactly as stored, braces included.

## What gets inserted

For a host with `TextParameterRefId` = *R*, on one device:

1. Module arguments `{{Name}}` are substituted first, as before, **including
   inside a default** (`{{0:Button {{ChNo}}}}` → `{{0:Button 3}}`).
2. The value of *R* on this device is its stored value, otherwise the product
   default (the `ParameterRef`'s `@Value`, otherwise its `Parameter`'s). A
   module-scoped *R* is looked up in the host's own module instance, through the
   same evaluation the parameter panel uses.
3. Each `{{0}}` / `{{0:def}}` becomes:
   - the value, when it is **not empty**;
   - `def` (or nothing, for `{{0}}`) when the value **is empty**. An empty stored
     value counts as empty; it is not the same as "no value stored".

## What stays verbatim

Retained exactly as written in source metadata and unresolved API text, never guessed
(the parameter workspace uses a presentation fallback instead of displaying raw templates):

- a placeholder on a host **without** `TextParameterRefId`;
- *R* that names no `ParameterRef` of the program, or a parameter that is not
  `TypeText`;
- a value that cannot be decided (the module instance cannot be told apart);
- any other index (`{{1}}`, `{{2001:…}}`), an empty head, an unclosed `{{` or a
  stray `}}`.

Nothing here writes to the project or the product database, and the stored
template is never replaced.

## What the API says

In the device detail, each name or heading that had a `{{0}}` carries a fill
record (`name_fill` on the object, `text_fill` on the channel) with its template
and where the inserted text came from: the device's stored value, the product
default, the template's default text, an empty value, or nothing inserted with
the reason. The web app shows the result as the name; the template and its
source appear in the tooltip, so a surprising name can always be traced back.
Names without a `{{0}}`, and every name the project owns, carry no record. The
documentation report prints the filled name only.

## Parameter workspace captions (issue #26)

The parameter panel fills block and activated separator text from the same device
evaluation and exact `(module definition, node ID)` source-reference lookup used
for channel headings. Named arguments have already been substituted. Stored
overrides win over product defaults; an explicitly empty stored value remains
empty and does not revive a product default. Missing, non-text and ambiguous
references remain unresolved, including when the template contains default text.

Group `labelFill` and separator/grid-label `textFill` are additive optional records
with the same `template` and `source` fields as the object/channel records. The
source template stays unchanged in the product database. Reading the panel adds
no project command; parameter writes and undo reuse the existing value path.

The workspace navigator, selected heading, search/context paths and Restricted
view use the resolved caption. Empty/whitespace-only text, a lone `#`, or any
remaining template delimiter gets a localized numbered parameter-group label;
separator/grid text uses a localized generic text label. Tooltips retain the
source text and explain the fallback or exact fill source. This is deliberately
conservative: a `StoredValue` or `ProductDefault` result can contain literal `{{`
or `}}` (for example `Kitchen {{North}}`) and still use the fallback. A fill
record describes the supported substitution's provenance, not whether the whole
result is free of unsupported indices: a mixed `{{0}} / {{1:Keep}}` template can
retain braces after filling `{{0}}`. The tooltip therefore describes the display
policy neutrally, never labels every such result unresolved, and retains the
result alongside its original template and exact resolver provenance. For area/
group captions, both stay searchable. Separator text/templates are not indexed
and separators are hidden during search. The UI does not strip braces, exempt
a fill-source category, invent
a default or resolve parameter references itself.

## Bounds

Filled channel headings count against the device detail's existing text
projection budget. Parameter-panel captions retain one `TextProjectionBudget`
through resolution and complete DTO construction, across all sections: resolver
work, retained fill templates, every group-caption copy on fields and separators,
separator text/provenance in both row and grid DTOs, and declared grid axis texts
are admitted before copying. The ceiling is unchanged; exhaustion refuses the
whole panel, never truncates it. Metadata and allocator overhead are outside this
content policy. A parameter edit preflights the full reconciled command through
the existing command stack on a project copy and assembles its complete panel
before durable publication; a refused projection changes neither project,
revision, history nor native bytes. Success returns that admitted panel, without
a fallible post-commit reread.

Object names use a separate budget of the same size per request. Only a text
containing `{{` is filled on the object/channel surfaces. When a fill would not
fit, nothing is shortened: device detail is refused with the same
`device text projection budget exhausted` error as any other over-budget product
text, and the documentation report prints that device's names unfilled.

## Not covered

- Group-address member lists, the Log and CSV exports keep showing the stored
  name: they read the project projection, which has no product database. Search
  is not affected; it finds devices, addresses and building parts, not objects.
- Whether ETS fills in a value while the text parameter itself is hidden by the
  evaluated tree is not documented. KNXBench fills it in and says so.
- Editing the text parameter from a channel heading.
- Declared grid row/column `TextParameterRefId` captions are preserved, not
  resolved. They are not dynamic-node hosts in this resolver. The UI uses a
  readable fallback for unresolved caption syntax rather than guessing a value.
