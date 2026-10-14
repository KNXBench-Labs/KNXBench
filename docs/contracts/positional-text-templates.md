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
  headings) and the documentation report's object names. One product-database
  helper applies the rule; object names go through one `knx-app` function shared
  by the device detail and the report. Resolution runs after the translation
  overlay, on the translated template.
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

Shown exactly as written, never guessed:

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

## Bounds

Filled text counts against the existing device text projection budget. When it
would not fit, nothing is shortened: the device detail is refused with the same
`device text projection budget exhausted` error as any other over-budget product
text, and the documentation report prints that device's names unfilled.

## Not covered

- Group-address member lists, the Log and CSV exports keep showing the stored
  name: they read the project projection, which has no product database. Search
  is not affected; it finds devices, addresses and building parts, not objects.
- Whether ETS fills in a value while the text parameter itself is hidden by the
  evaluated tree is not documented. KNXBench fills it in and says so.
- Editing the text parameter from a channel heading.
- Parameter block and separator titles: their reference is stored, but the
  parameter panel lists fields, not headings, so no surface shows them yet.
