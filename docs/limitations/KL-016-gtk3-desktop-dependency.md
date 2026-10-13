# GTK3 desktop dependency

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-016)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="16-tauri-v2s-linux-backend-depends-on-archived-gtk3-bindings"></a>

## 16. Tauri v2 remains on GTK3; former maintenance advisories are resolved

**Resolved premise (verified 2026-09-22).** The locked desktop shell uses
`tauri` 2.11.5 and GTK3, but those bindings are no longer archived. RustSec
withdrew RUSTSEC-2024-0411 through RUSTSEC-2024-0420 on 2026-08-14 after the
`gtk3-rs` repository was unarchived and development resumed. **[D]** The
withdrawal and reason are recorded in each advisory, for example
[RUSTSEC-2024-0415](https://rustsec.org/advisories/RUSTSEC-2024-0415.html).
Before this edit, `cargo deny check advisories` exited 0 but reported the ten
old ignore entries as unmatched. After their removal it exits 0 with
`advisories ok`. **[V]** Keeping withdrawn suppressions would conceal future
policy drift rather than reduce risk.

**Remaining exposure.** Six unrelated transitive maintenance notices remain:
RUSTSEC-2024-0370 for `proc-macro-error`, plus RUSTSEC-2025-0075, -0080,
-0081, -0098 and -0100 for `unic-*` crates pulled through
`urlpattern`/`tauri-utils`. They are not reported vulnerabilities, have no
patched release, and each acceptance in `deny.toml` carries its review date
and dependency reason. **[V]** `proc-macro-error` is reached through
`glib-macros` while compiling the desktop, so exploiting an unknown defect
there would require influence over the build or macro input; it is not linked
as a runtime request handler. The five `unic-*` crates are reached through
`urlpattern` in Tauri's remote-URL matching for capability/IPC permissions.
Exercising an unknown defect there would first require attacker-controlled
remote-URL or capability pattern data;
KNXBench's desktop opens its own local server and exposes no plugin or
arbitrary-site surface. **[V]** These are boundaries, not proof that an
unknown defect cannot exist. The concrete accepted cost is code whose
maintainers no longer promise fixes, not a known exploit chain.


*Update 2026-10-07.* GitHub Dependabot on the recreated repository reports
[GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g)
(medium): unsound `Iterator`/`DoubleEndedIterator` impls for
`glib::VariantStrIter`, fixed in `glib` 0.20. The lockfile has `glib` 0.18.5,
pulled in only by the GTK3 stack (`atk`, `cairo-rs`, `gdk`, `gtk`,
`webkit2gtk`), which cannot move to 0.20 without the GTK4 migration named
below. `cargo deny check advisories` does not fail on it. A source search of
every crate in the tree that depends on `glib` 0.18.5 found no caller of
`VariantStrIter` or `str_iter()` outside `glib` itself, and KNXBench's own code
has none. **[V]** That is a static search of the locked sources, not a proof
that no dynamic path reaches it. On the user's decision the alert was
dismissed on GitHub as "vulnerable code not used", pointing to this section.

**Decision.** Keep Tauri 2 for this alpha. Tauri 3.0.0-alpha.2 was published
on 2026-09-21, while Tauri's normal Wry Linux GTK4/WebKitGTK 6 migration
([tauri#14684](https://github.com/tauri-apps/tauri/pull/14684)) and Wry's own
migration ([wry#1767](https://github.com/tauri-apps/wry/pull/1767)) remain open
as of 2026-09-22. **[D]** The separate experimental CEF runtime's
[3.0.0-alpha.2 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-runtime-cef-v3.0.0-alpha.2)
lists its GTK4 dependencies. **[D]** Adopting that runtime and distributing
Chromium would be a platform migration, not a maintenance-warning fix. **[A]**
Re-evaluate after the Wry GTK4 work ships in a stable Tauri release. New,
unaccepted advisories block `cargo deny`; obsolete ignore entries are reported
as warnings and must be removed during the dependency review.

<details>
<summary>Historical finding before the RustSec withdrawals</summary>

**Limitation.** The desktop shell's Linux runtime depends on `tauri` 2.11,
which pulls in the archived gtk-rs GTK3 bindings; `cargo deny check` flags
16 upstream "unmaintained" notices in the advisory database, all of which
must be suppressed in `deny.toml` to build.

**Cause.** The gtk-rs project archived its GTK3 bindings repository in 2024.
The bindings are not vulnerabilities — every advisory explicitly states "no
safe upgrade is available" — but they are no longer maintained upstream.
Tauri's own GTK4 migration is in progress and not yet shipped.

**Impact.** Each `tauri` or `tauri-*` dependency bump requires manual
re-check of the 16 suppressed IDs: RUSTSEC-2024-0370, -0411 through -0420
(minus one gap), and RUSTSEC-2025-0075, -0080, -0081, -0098, -0100. As
Tauri moves to GTK4, some or all of these may disappear from the advisory
database. Until then, the `deny.toml` ignore list is permanent infrastructure.

**Lifted when.** Tauri v3 or a later `tauri` 2.x release ships its GTK4
backend and becomes the default on Linux.

</details>

**Update 2026-10-06 (AR15): rechecked, decision unchanged.** `Cargo.lock`
pins `tauri` 2.11.5 (crates.io stable is 2.12.1), `gtk` 0.18.2, `webkit2gtk`
2.0.2 and `wry` 0.55.1. `cargo deny --offline check advisories` against the
local advisory database of 2026-10-03 (`ef6173c`) reports `advisories ok`
with no unmatched ignore. Wry's GTK4/WebKitGTK 6 migration
([wry#1767](https://github.com/tauri-apps/wry/pull/1767)) and Tauri's
([tauri#14684](https://github.com/tauri-apps/tauri/pull/14684)) are still open
(GitHub API, 2026-10-06); Tauri 3.0.0-alpha.4 was published on 2026-10-01.
The GTK3 platform dependency is an accepted boundary for the Alpha.
<!-- END retained catalogue record -->
