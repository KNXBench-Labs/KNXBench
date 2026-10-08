# Legacy VD2/VD3/VD4 product databases

Investigation date: **2026-09-16**.

This document covers the supplied file
`OriginalData/ProductDatabases/Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4`,
the available installation paths, and prior public reverse engineering of the
VD2/VD3/VD4 family.

## Result

The legacy format has already been reverse engineered publicly. Two independent
GPL implementations can read the encrypted container and parse its textual
`EX-IM` payload:

| Project | Implementation | Coverage | State |
| --- | --- | --- | --- |
| [christoph2/knx](https://github.com/christoph2/knx) (`knxReTk`) | Python 2 reader and pre-ETS4 converter | Recognizes `.vd1` through `.vd5` and corresponding `.pr*` files | Last substantial development in 2016; GPL-2.0-or-later file headers |
| [selfbus/tools-libraries](https://github.com/selfbus/tools-libraries) (`sbtools-vdio`) | Java reader, writer, JAXB model, XSD, and tests | Reads `.vd[0-9]`; history explicitly mentions BCU2 and ETS5 support | Module developed mainly in 2013-2015; GPL-3.0 |

The Selfbus implementation is the more complete format reference because it
includes a reader, writer, table model, schema, and fixtures. `knxReTk` is also
useful because it includes conversion from pre-ETS4 data into its newer model.
Neither project is currently integrated into KNXBench.

The supplied `.vd4` still **cannot be installed directly in KNXBench**. It is a
legacy encrypted product-database container rather than a modern `.knxprod`
package. Today there are three practical paths:

1. **Use the original file in ETS6.** ETS6 accepts `.vd1` through `.vd5`.
2. **Convert it with the official KNX converter**, then install the generated
   `.knxprod` in KNXBench if its XML scheme is supported.
3. **Implement an independent legacy importer in `knx-productdb`**, using the
   observed format and original tests. GPL source must not be copied without full GPL/AGPL compliance and a
   deliberate architecture and dependency-policy change.

## Verified structure of the supplied VD4

| Property | Observed value |
| --- | --- |
| Size | 173,157 bytes |
| SHA-256 | `f5d698d58769ba8c787f63aec1a751761720c8e29117b4590ecca37a6bf880b6` |
| Outer format | ZIP with one member |
| Member | `ets/präsmit kl/ets.vd_` (legacy DOS filename encoding) |
| Member state | Traditional ZIP encryption, deflated |
| Expanded size | 1,029,465 bytes |
| Password handling | Verified locally; value stored only in `OriginalData/ProductDatabases/.vd-import-password` |
| Payload SHA-256 | `a3ff864b0a4b01da7b81c8574300c74547da755e026d326df0cb7099de14f487` |
| Payload format | Textual `EX-IM`, 37 tables and 14,734 rows |
| Modern package layout | Absent: no root `knx_master.xml` or `M-xxxx/` partition |

`knxReTk` contains a working archive password in
[`ets_loader.py`](https://github.com/christoph2/knx/blob/72b6732322cc2f46df0379719148fb45210e23da/knxReTk/vdimex/ets_loader.py).
The literal value is intentionally excluded from the repository. The local
verification reads it from the ignored file without placing it in a process
argument:

```python
from pathlib import Path
from zipfile import ZipFile

archive = Path(
    "OriginalData/ProductDatabases/"
    "Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4"
)
password = Path(
    "OriginalData/ProductDatabases/.vd-import-password"
).read_bytes().strip()

with ZipFile(archive) as vd_file:
    member = vd_file.namelist()[0]
    payload = vd_file.read(member, pwd=password)
```

The decoded payload starts with:

```text
EX-IM
N C:\ets\präsmit kl\ets.vd_
K ETS3
K 
D 2012-05-04 13:11:16
V 6.2
H virtual_device
```

Its tables include `manufacturer`, `functional_entity`, `hw_product`,
`catalog_entry`, `application_program`, `virtual_device`, `parameter_type`,
`parameter`, `communication_object`, `text_attribute`, `device_object`,
`device_parameter`, and `ApplicationProgramAttributes`.

This proves decryption and the raw table structure for this exact file. It does
not prove that every VD2/VD3/VD4 variation is understood, or that all data can be
mapped losslessly into the current KNXBench model.

## What the public implementations reveal

### `knxReTk`

The relevant sources are
[`loader.py`](https://github.com/christoph2/knx/blob/72b6732322cc2f46df0379719148fb45210e23da/knxReTk/vdimex/loader.py),
[`ets_loader.py`](https://github.com/christoph2/knx/blob/72b6732322cc2f46df0379719148fb45210e23da/knxReTk/vdimex/ets_loader.py),
and
[`preETS4Converter.py`](https://github.com/christoph2/knx/blob/72b6732322cc2f46df0379719148fb45210e23da/knxReTk/vdimex/preETS4Converter.py).

The loader:

- recognizes `.vd1` through `.vd5` and `.pr1` through `.pr5`;
- opens the ZIP container and locates `ets2.vd_`, `ets.vd_`, or `ets.pr_`;
- parses the `EX-IM` record types `T` (table), `C` (columns), `R` (row), and
  `XXX` (end); and
- converts legacy tables into a newer in-memory representation.

### `sbtools-vdio`

The relevant sources are
[`ProductsReader.java`](https://github.com/selfbus/tools-libraries/blob/a915cbaf550331725c5ac7cefbeed527598937ed/sbtools-vdio/src/main/java/org/selfbus/sbtools/vdio/ProductsReader.java),
[`VDReader.java`](https://github.com/selfbus/tools-libraries/blob/a915cbaf550331725c5ac7cefbeed527598937ed/sbtools-vdio/src/main/java/org/selfbus/sbtools/vdio/VDReader.java),
[`ProductsWriter.java`](https://github.com/selfbus/tools-libraries/blob/a915cbaf550331725c5ac7cefbeed527598937ed/sbtools-vdio/src/main/java/org/selfbus/sbtools/vdio/ProductsWriter.java),
and
[`vd.xsd`](https://github.com/selfbus/tools-libraries/blob/a915cbaf550331725c5ac7cefbeed527598937ed/sbtools-vdio/src/main/resources/org/selfbus/sbtools/vdio/vd.xsd).

The module:

- reads plain `.vd_` payloads and encrypted `.vd[0-9]` ZIP files;
- parses `EX-IM` into a broad product, parameter, communication-object, and
  translation model;
- can write VD data; and
- tests plain, unencrypted, password-protected, and wrong-password cases.

The related experimental
[`sbtools-products-editor`](https://github.com/selfbus/development-tools-incubation/tree/master/sbtools-products-editor)
uses this library for imports and exports.

Both codebases are useful evidence. The following assessment defines whether and
how KNXBench may use them.

## Licence and legal assessment

This is an engineering assessment of the published licences and German/EU
interoperability rules. It cannot resolve unpublished contracts or replace
advice for a commercial release.

### Licence evidence

| Codebase | Verified grant | Conservative interpretation |
| --- | --- | --- |
| `knxReTk` | Every relevant parser file carries Christoph Schueler's copyright notice and permits redistribution and modification under GPL version 2 or any later version. | Treat it as `GPL-2.0-or-later`. The headers refer to `FLOSS-EXCEPTION.txt`, but that file is absent from the investigated commit and all objects in the cloned repository. No additional exception can therefore be relied upon. |
| `sbtools-vdio` | The repository root has `LICENSE` and the module has `sbtools-vdio/COPYING`; both contain GPL version 3. GitHub also identifies the repository as GPL-3.0. | Treat the module as GPL version 3, without assuming an unrecorded exception or broader permission. |
| KNXBench | `Cargo.toml`, `README.md`, and the canonical [`LICENSE`](../LICENSE) specify `AGPL-3.0-or-later`. | The licence decision was finalized on 2026-09-16. GPLv3 code is compatible in principle under GPLv3 section 13 expressly permits combining GPLv3 and AGPLv3 code.
KNXBench is now formally licensed under AGPL version 3 or later, so `knxReTk`
can be used under its GPLv3 option and the Selfbus GPLv3 code is compatible in
principle. GPL continues to govern its portions, and AGPL network-source
obligations apply to the combined work.

That compatibility is not unconditional implementation approval. Copying,
translating line by line, or closely preserving the structure of either parser
would create strong derivative-work risk. A compliant release would at least
need to:

- retain upstream copyright and licence notices and identify modifications and
  their dates;
- provide complete corresponding source and the required AGPL source offer to
  remote users;
- record the upstream commit and copied files in repository notices;
- deliberately change KNXBench's architecture and `cargo-deny` policy if a GPL
  dependency is added. Pasted or translated source would evade the dependency
  scanner but still require the same compliance.

A future change to a permissive or proprietary KNXBench licence could not cover
those GPL implementations without separate permission from their copyright
holders.

### Independent implementation

An independently written parser is the recommended route. The Court of Justice
of the European Union held in case C-406/10 that program functionality,
programming languages, and data-file formats used to exploit program functions
are not protected forms of expression under the Software Directive. German
`UrhG` section 69d(3) permits an authorized user to observe, study, or test a
program to determine underlying ideas and principles. Section 69e separately
permits necessary decompilation for interoperability under strict purpose,
access, disclosure, and non-copying conditions.

For a defensible independent implementation:

1. Derive a factual format specification from lawfully obtained VD files and
   externally observable behavior. Record field meanings, byte grammar, and
   test vectors without copying source expression.
2. Do not copy or translate upstream code, class structure, comments, XSD,
   diagrams, or tests. Using a separate specification author and implementer is
   the strongest provenance model if commercial distribution is planned.
3. Commit only synthetic VD fixtures created for KNXBench. Keep manufacturer
   databases such as the supplied Eibmarkt file local and ignored unless the
   rights holder gives redistribution permission.
4. Report every unsupported record or mapping loss. Place the parser at the
   `knx-productdb` trust boundary and prove atomic imports.

### Encryption and the published password

Publication of the archive password in upstream source proves that the
credential decrypts these archives, but a software licence does not grant rights
in third-party product data or authorize circumvention of access controls.
German `UrhG` sections 69f and 95a regulate tools for bypassing technical
protection measures, while the exact vendor and KNX terms for the supplied
database have not been established.

The lower-risk release design is therefore to import only files lawfully
obtained by the user, request the password from the user, avoid advertising a
circumvention feature, and keep the official ETS conversion route available.
Embedding and distributing the legacy password should wait for written
authorization from the relevant rights holder or case-specific legal advice.

### Current implementation decision

- **No approval to copy or port either GPL implementation into the current
  repository.** The AGPL compatibility question is resolved, but KNXBench's
  incoming-dependency policy still rejects GPL runtime dependencies.
- **Conditional approval for an independently written parser** based on format
  facts, synthetic fixtures, user-supplied lawful input, and explicit
  provenance. Publishing the built-in password remains excluded from that
  approval.
- **Direct reuse is legally plausible under the selected AGPL licence only with
  full GPL/AGPL compliance and an explicit architecture and dependency-policy
  change.** The existing licence decision and dependency gate do not themselves
  approve copied or translated GPL source.

## Current KNXBench behavior

**Update 2026-10-08 (ADR-0094).** KNXBench now reads legacy files itself, for
inspection only:
`knx products inspect-legacy <file> --password-stdin` (or
`--password-file <path>`) decrypts the file with the password you supply,
parses it and lists its tables and products. Nothing is written. This
supplied `.vd4`, a `.vd3` from 2006 and the MDT `.pr5` all read with zero
diagnostics. A legacy file renamed `.knxprod` is refused as
`legacy ETS3 product database (EX-IM)`. Importing the products is the next
step (L2). Until then the official conversion route below still applies.
The text that follows describes the state before this update.

The CLI recognizes only `.knxprod` and `.vd2` suffixes as standalone product
packages. Passing the supplied `.vd4` reaches the project-import path and fails
before publishing any data:

```text
import failed for ...N520_IRBM_N530_IRBM.vd4:
not a zip archive: unsupported Zip archive: Password required to decrypt file
```

The probe used a fresh temporary product database. It contained zero packages,
manufacturers, products, and application programs after the failure, so the
failure was atomic for this sample.

Renaming a copy to `.knxprod` does not convert it. The standalone package
validator rejects the legacy member path:

```text
failed to install product package .../sample.knxprod:
unsafe product ZIP member: ets/präsmit kl/ets.vd_
```

The archive also lacks the modern `knx_master.xml` and `M-xxxx/` layout. The web
catalogue picker currently advertises `.knxprod` and `.vd2`, but not `.vd4`.

## Installing the original file in ETS6

Use this route when the product is needed in ETS itself:

1. Open the target project's **Catalog** panel.
2. Start the product import action.
3. Select `Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4`.
4. Select the products and languages, then complete the import.

The KNX Association ETS6 FAQ explicitly lists `.vd4` among the accepted product
file types. This procedure does not add the product to KNXBench's product
database.

## Converting for possible KNXBench installation

The KNX Association documents two conversion methods. Both require official
Windows tooling.

### ETS command-line converter

1. Install ETS5 or ETS6 on Windows.
2. Locate `KnxCvNext.exe` below `C:\Program Files (x86)\ETS5` or
   `C:\Program Files (x86)\ETS6`.
3. Drag the `.vd4` file onto the converter.
4. Copy the generated `.knxprod` to the Linux system running KNXBench.

The converter requires the corresponding ETS installation on the machine where
it runs.

### Manufacturer Tool

1. Create a project from the **KNX Convert knxprod/vd file to KNX MT Project**
   template.
2. Choose ETS5 or newer as the target version.
3. Select the original `.vd4` product database.
4. Open the project and run **Build**.
5. Take the generated `.knxprod` from the project's `Out` directory.

### Check before installing in KNXBench

KNXBench's standalone installer currently accepts a root `knx_master.xml` only
with one of these namespaces:

```text
http://knx.org/xml/project/11
http://knx.org/xml/project/20
```

Schemes 12 through 19, 21, and 22 are not supported as standalone `.knxprod`
packages. Inspect the generated package before relying on it:

```bash
unzip -p converted.knxprod knx_master.xml | head
```

If the namespace is 11 or 20, install it from KNXBench's device catalogue with
**Install product database**, or use the CLI:

```bash
cargo run -p knx-cli -- products ingest /path/to/converted.knxprod
cargo run -p knx-cli -- products list
cargo run -p knx-cli -- products verify
```

Without `--product-db`, these commands use
`$XDG_DATA_HOME/knx/products.sqlite`, falling back to
`$HOME/.local/share/knx/products.sqlite`. To test without changing the normal
catalogue, pass the same temporary path to every command:

```bash
cargo run -p knx-cli -- products ingest /path/to/converted.knxprod \
  --product-db /tmp/knx-vd4-probe.sqlite
cargo run -p knx-cli -- products list \
  --product-db /tmp/knx-vd4-probe.sqlite
cargo run -p knx-cli -- products verify \
  --product-db /tmp/knx-vd4-probe.sqlite
```

The official conversion and subsequent KNXBench install remain unverified for
this exact product. The Linux-only direct-import path is technically plausible
because the password and format are known, but it has not been implemented.

**Update 2026-09-26 (DIN-9).** The supplied MDT `.pr5` was measured as the
same `EX-IM` container family (`ets.pr_`, header `H project`), and it has no
application program. The design basis for a direct importer is now
[2026-09-26-legacy-vd-pr-product-import-design.md](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md).
That design decrypts only with a user-supplied password. It does not rely on
the password being known locally, and it keeps every constraint of this
document.

## Sources

- [Eibmarkt download centre](https://www.eibmarkt.de/en/downloadcenter.html)
  (product download and ETS3/4 import note; accessed 2026-09-16)
- [KNX Association: Converting vdx databases to knxprod](https://support.knx.org/hc/en-us/articles/115002601169-Converting-vdx-databases-to-knxprod)
  (`KnxCvNext.exe` and Manufacturer Tool procedures; accessed 2026-09-16)
- [KNX Association: Import/Add products](https://support.knx.org/hc/en-us/articles/360022205039-Import-Add-products)
  (ETS catalogue import procedure; accessed 2026-09-16)
- [KNX Association: ETS6 FAQ](https://www.knx.org/de/news/haeufig-gestellte-fragen-zur-ets6)
  (accepted ETS6 product-file extensions; accessed 2026-09-16)
- [christoph2/knx at investigated commit](https://github.com/christoph2/knx/tree/72b6732322cc2f46df0379719148fb45210e23da)
- [selfbus/tools-libraries at investigated commit](https://github.com/selfbus/tools-libraries/tree/a915cbaf550331725c5ac7cefbeed527598937ed)
- [GNU GPLv3 section 13](https://www.gnu.org/licenses/gpl-3.0.html#section13)
  and [GNU GPL FAQ: combining GPLv3 and AGPLv3](https://www.gnu.org/licenses/gpl-faq.html#AGPLGPL)
- [GNU AGPLv3](https://www.gnu.org/licenses/agpl-3.0.html)
- [CJEU case C-406/10, SAS Institute v World Programming](https://curia.europa.eu/juris/liste.jsf?num=C-406/10)
  and the Court's [official press release 53/12](https://curia.europa.eu/jcms/upload/docs/application/pdf/2012-05/cp120053en.pdf)
- German Copyright Act:
  [`§ 69d`](https://www.gesetze-im-internet.de/urhg/__69d.html),
  [`§ 69e`](https://www.gesetze-im-internet.de/urhg/__69e.html),
  [`§ 69f`](https://www.gesetze-im-internet.de/urhg/__69f.html), and
  [`§ 95a`](https://www.gesetze-im-internet.de/urhg/__95a.html)
