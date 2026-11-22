# Device configuration modules

Status: package WP-42.1 (format, engine and CLI). Project storage and the web
editor follow in WP-42.2/42.3.
Decision: [ADR-0175](../adr/0175-device-configuration-modules-are-declarative-data.md).

A **device configuration module** describes, as data, a configuration document
that a device with its own firmware understands: which fields it has, which
values are valid, how records refer to each other and how the document is
encoded into the bytes the device reads. KNXBench validates and exports such
documents with one generic, tested engine (`knx-devconf`). A module contains no
code, and KNXBench contains no code for any particular device.

Not to be confused with ETS application *modules* (`ModuleDef`,
`ModuleInstance`, ADR-0013), which are part of a manufacturer application
program.

## Files

| File | Content |
| --- | --- |
| Module package (`*.devconf.json`) | `format`, `format_version`, `id`, `version`, `title`, `max_document_bytes`, `document`, `rules`, `exports` |
| Configuration document (`*.json`) | one value shaped by the module's `document` type |
| Export output | bytes produced by one of the module's `exports` |

Both JSON inputs are strict: UTF-8, a single JSON value, no duplicate object
member names (refused, not last-wins), numbers must be integers without fraction
or exponent and fit in a signed 64-bit integer (`-0` is refused as well). A module package is at most
**262144 bytes**; a document is at most the module's `max_document_bytes`
(1..1048576).

## Module package

```json
{
  "format": "knxbench-device-configuration-module",
  "format_version": 1,
  "id": "example.demo-panel",
  "version": "1.0.0",
  "title": "Demo panel",
  "max_document_bytes": 16384,
  "document": { "type": "object", "members": { ... } },
  "rules": [ ... ],
  "exports": [ ... ]
}
```

- `id`: 1..64 bytes `[a-z0-9]` first, then `[a-z0-9.-]`.
- `version`: `MAJOR.MINOR.PATCH`, each 0..65535 without leading zeros.
- `title`: 1..80 printable characters (no control characters).
- Every object in the package has exactly the members listed here; unknown
  members are refused so that a newer module never loads with ignored meaning.
- Optional members are marked *optional*.

### Types

Every type is an object with `type` and optional `title` (1..80 characters) and
`description` (1..400 characters) used by editors.

| `type` | Members | Accepts |
| --- | --- | --- |
| `int` | `min`, `max` (`min <= max`) | an integer in range |
| `enum` | `values`: 1..256 distinct strings, each 1..64 printable ASCII | one of the strings |
| `text` | `charset` = `printable-ascii` or `utf8`; `min_bytes`, `max_bytes` (0..4096) | a string whose UTF-8 length is in range; `printable-ascii` allows bytes 0x20..0x7E only; `utf8` refuses control characters |
| `const` | `value`: any JSON value | exactly that value (object member order does not matter) |
| `array` | `items`: an `int`, `enum` or `text` type; `min_len`, `max_len` (0..4096) | an array of such values |
| `ref` | `table`: a table member of the document root; `none` *optional*: an integer meaning "no reference" | an integer that is `none` or the key of an existing row of that table |
| `object` | `members`: 1..64 named types | an object with exactly these members |
| `table` | `row`: an `object` type whose members are not `object`/`table`; `min_rows`, `max_rows` (0..4096); `key` *optional*: name of an `int` row member | an array of rows; key values must be unique |

Restrictions that keep the engine small: `object` types appear only as the
document root and as table rows; `table` types appear only as members of the
root; `ref` targets a table that has a `key`; rows contain only `int`, `enum`,
`text`, `const`, `array` and `ref`.
Member names are 1..64 bytes `[a-z_][a-z0-9_]*`.

Validation of the structure happens before any rule is evaluated and reports the
JSON path of each problem (`/nodes/3/label`). Reference existence is part of the
structural check.

### Rules

```json
{ "id": "number-binding-kind", "table": "nodes",
  "when": <expression>, "require": <expression>,
  "field": "binding", "message": "A number row needs a value binding." }
```

- `id`: unique, same syntax as member names (`-` also allowed).
- `table` *optional*: evaluate once per row of that table; without it the rule
  is evaluated once for the document.
- `when` *optional*: the rule applies only where this is `true`.
- `require`: must evaluate to `true`; `false` **or** `null` is a violation.
- `field` *optional*: the row member a violation is reported at (editor focus).
- `message`: 1..400 characters shown to the user.

Rules run only on a structurally valid document. All violations are reported,
in rule order and then row order. A document is valid when it has no structural
error and no rule violation.

### Expressions

Expressions are JSON values (no string syntax is parsed):

| Form | Value |
| --- | --- |
| JSON integer, string, `true`, `false`, `null` | that literal |
| `{"field": "name"}` | member of the current row in a table rule or `rows` layout; member of the document root elsewhere |
| `{"field": "name", "of": "q"}` | member of the row bound to variable `q` |
| `{"index": null}` / `{"index": "q"}` | array position of the current / bound row |
| `{"lookup": {"table": "t", "key": E}}` | the row of `t` whose key equals `E`, else `null` |
| `{"row": {"table": "t", "index": E}}` | the row of `t` at position `E`, else `null` |
| `{"get": [ROW, "name"]}` | member of a row value; `null` if `ROW` is `null` |
| `{"item": [ARRAY, E]}` | element of an `array` value; `null` when out of range |
| `{"ordinal": E}` | 0-based position of an `enum` value in its type's `values` |
| `{"length": E}` | UTF-8 byte length of a text, or length of an array |
| `{"rows": "t"}` | number of rows in table `t` |
| `{"count": {"table": "t", "as": "q", "where": E}}` | number of rows for which `E` is `true` |
| `{"any": {...same...}}` / `{"all": {...same...}}` | whether some / every row satisfies `E` |
| `{"depth": {"table": "t", "parent": "p", "root": V}}` | number of ancestors of the current row following member `p` until the value `V`; `null` on a missing parent or a cycle |
| `{"op": NAME, "args": [ ... ]}` | operator, see below |

Operators: `eq`, `ne`, `lt`, `le`, `gt`, `ge` (two args, integers or strings,
both the same kind; `eq` and `ne` also compare two booleans; strings compare by
bytes); `add`, `sub`, `mul`, `mod`, `xor` (two integers);
`neg` (one integer); `mod` is the truncated remainder (sign of the dividend),
`xor` is bitwise; `and`, `or` (one or more booleans), `not` (one boolean);
`in` (value, then one or more candidates); `if` (condition, then, else);
`is_null` (one value).

`null` semantics are fixed: any operator with a `null` operand yields `null`,
except `is_null`, `if` (a `null` condition selects the else branch),
`and`/`or` which follow three-valued logic (`false and null` is `false`,
`true or null` is `true`), and `in`, which is `true` when a candidate equals
the value even if other candidates are `null`, `null` when the value is `null`
or no candidate matches but one is `null`, else `false`. Quantifiers count a
`null` condition as not satisfied. Integer overflow and `mod` by zero yield `null`.
Expressions are type-checked when the module is loaded: an operand of the wrong
kind (a string where an integer is required), an unknown member, table or row
variable, or comparing an enum with a string literal that is not one of its
values refuses the module. A `const` member of the document is usable when its
value is `null`, a boolean, an integer or a string. Rows and arrays can only be
read through `get`, `item`, `length` and quantifiers, not compared.

Limits: expression depth 32, quantifier nesting 3, `depth` follows at most the
row count of its table. Validation has a budget of 50 million evaluation steps;
an export validates first and then gets the same budget again for its layout.
Exhausting a budget refuses the operation. At most 1000 findings are reported
with their details; further findings are only counted, so memory does not grow
with the number of violations.

### Exports

```json
{ "id": "device-profile", "title": "Device profile", "extension": "bin",
  "max_bytes": 4096, "layout": [ <item>, ... ] }
```

Exports run only on a valid document. Layout items, in order:

| Item | Bytes |
| --- | --- |
| `{"ascii": "TEXT"}` | the literal printable ASCII characters |
| `{"u8": E}` / `{"u16le": E}` / `{"i16le": E}` / `{"u32le": E}` | `E` as little-endian integer; outside the encoding range (or `null`) refuses the export |
| `{"text": E, "size": N, "pad": "nul-terminated"}` | the text's bytes, then zero bytes to `N`; requires at least one zero byte |
| `{"text": E, "size": N, "pad": "zero"}` | the text's bytes, then zero bytes to `N`; the text may fill `N` |
| `{"zero": N}` | `N` zero bytes |
| `{"rows": "t", "layout": [ ... ]}` | the nested layout once per row of `t`, in array order, with that row as current row |
| `{"total_length": "u16le"}` / `{"total_length": "u32le"}` | the final output length |
| `{"crc": "crc16-ccitt-false", "encoding": "u16le"}` | CRC-16 (poly 0x1021, init 0xFFFF, no reflection, no xor-out) over the final output with these two bytes as zero |

At most one `total_length` and one `crc` item, both outside `rows`; `rows` items
cannot be nested. The output is refused when longer than `max_bytes`. The same module and document always produce the same bytes.

## CLI

```text
knx devconf check   <module> [--json]              # load and verify a module package
knx devconf validate <module> <document> [--json]  # structural errors and rule violations
knx devconf export  <module> <document> --export <id> --output <file>
knx devconf export-stored <project.knxdb> --device <id> --module <module> --export <id> --output <file>
```

`check`, `validate` and `export` are files only. `export-stored` additionally reads one saved
project **read-only** (an older schema is upgraded in a memory copy, never in place; nothing is
written to the project) and exports the document stored for the device. The stored profile's module
id, version and SHA-256 must equal those of the explicitly given module (no fallback to another
version); the stored bytes reach the module's validation unchanged, so whitespace or member order
is not an error. A missing device or profile, an unreadable or foreign project and an unknown export
exit 2; a module identity mismatch, an invalid document or an over-long export exit 1; none of them
creates or replaces the output. An output that is the same file as the project or module (also via
symlink or hard link) is refused before anything is read. No bus, device or download is involved.

Exit 0 on success; 1 when the document is invalid or an export is refused; 2 for
unusable input (unreadable file, malformed or unsupported module). `export`
writes the output atomically and only after successful validation.

## Project-stored documents (WP-42.2/42.3, WP-48.4)

A device's document is stored with the project as opaque bytes next to the module
identity it was written for. The admission levels are separate:

1. **Identity syntax** (`knx-core`): module id, version, SHA-256 syntax, a
   non-empty document of at most 1 MiB. Used on every load and undo/redo replay, so
   previously stored bytes are never discarded, whatever they contain.
2. **Strict JSON admission** (`knx-app`, `check_document`, on every new or replaced
   document, before any command is built): UTF-8, one JSON value (no trailing content),
   no duplicate member names at any depth, integers only (no fraction/exponent/`-0`,
   signed 64-bit), nesting depth at most 128, at most 1 MiB. Refusal is `422` with kind
   `devconfDocumentInvalid` and changes neither project, history nor revision. It uses
   the same strict parser as the engine; it does **not** check the document against any
   module or rule and does not prove a module package is installed or valid.
3. **Module validation** (`knx devconf validate`, engine): structure and rules against a
   module package. Not performed by the project edit routes.

A stored document that fails level 2 (legacy or foreign bytes) is kept byte for byte and
reported: the single-device read carries `documentError`. It can be removed or replaced,
not re-saved unchanged.

Reading: `GET /api/project/device-config-profiles/devices/{id}` returns that device's
profile with `documentB64` (exact stored bytes, base64url), `documentBytes` and
`documentSha256`, plus the server/project incarnation and snapshot revision it was read
at (`404 devconfDeviceNotFound` for an unknown device, `profile: null` without a
profile). The installation-wide list stays metadata only (identity, length, content
SHA-256), never document bytes.

Editor draft policy: the panel reads the device again on every snapshot revision and only
adopts a response carrying exactly the context it asked for. A clean editor reloads; a
dirty draft is never overwritten. If the stored content changed underneath it, the draft
is kept, previewing is blocked and the user chooses *Keep my draft* or *Load stored
version*. Unedited text is sent back as the exact stored bytes (no reformatting).
A new revision whose stored content equals the draft (typically the user's own apply) is
adopted silently, without the stale alert. A preview confirmation always belongs to the
text it was requested for: an edit made while the preview request is in flight discards
the late answer and cancels its token (draft-version check, inputs stay editable), so
*Apply* can never write text the user no longer sees.

## What is not claimed

- A valid document is valid **for the module**. Whether a device accepts it is
  the module author's claim, to be evidenced outside KNXBench (for example by
  differential tests against the firmware's own validator).
- Nothing here talks to a device or the bus; downloading a configuration is not
  part of this contract.
