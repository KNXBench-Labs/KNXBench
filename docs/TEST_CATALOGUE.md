Last updated: 2026-10-10 13:39 CEST

# KNXBench test catalogue

[Repository overview](../README.md) · [Manual](manual/README.md) · [Running tests](manual/development/02-building-from-source.md#running-the-tests)

A human-readable map of the tests and checks already in the repository. Short
explanations, actual source links, and no requirement to read Rust before breakfast.

**Source snapshot:** `8901affa9629384a23e0a4b5461af5aeb1b6b8b0` (fetched `main`, inspected on 10 October 2026).
**Inventory:** 199 related suite groups covering 640 test-bearing modules,
acceptance scripts and explicit probes, plus the quality gates below. Related unit,
integration and interface suites share a row; expand each category's source list
to find every inventoried file. These are **not individual test-case counts**.

> **Important**
>
> This catalogue records what exists, not what just passed. No application test,
> private-corpus regression or live hardware test was run to write it. Runner-generated
> cases, parameterized scenarios and generated binding tests are not counted individually.
> Passing synthetic or simulated tests does not establish full ETS compatibility,
> native accessibility or approval for an arbitrary real device.

## Contents

- [How to read the catalogue](#how-to-read-the-catalogue)
- [Privacy and repository hygiene](#privacy-and-repository-hygiene)
- [KNX domain and validation](#knx-domain-and-validation)
- [Commissioning plans and recovery](#commissioning-plans-and-recovery)
- [Project import and interchange](#project-import-and-interchange)
- [Manufacturer databases and package ingestion](#manufacturer-databases-and-package-ingestion)
- [Parameter evaluation and offline images](#parameter-evaluation-and-offline-images)
- [Native storage and project history](#native-storage-and-project-history)
- [Reports, comparisons and projections](#reports-comparisons-and-projections)
- [KNXnet/IP and simulated device communication](#knxnetip-and-simulated-device-communication)
- [Explicit live hardware tests](#explicit-live-hardware-tests)
- [Command-line interface](#command-line-interface)
- [Server services and HTTP contracts](#server-services-and-http-contracts)
- [Desktop shell and read-only MCP](#desktop-shell-and-read-only-mcp)
- [Frontend components and editing workflows](#frontend-components-and-editing-workflows)
- [Frontend accessibility, preferences and diagnostics](#frontend-accessibility-preferences-and-diagnostics)
- [Telegram Flow models and browser views](#telegram-flow-models-and-browser-views)
- [Browser workflows and layout contracts](#browser-workflows-and-layout-contracts)
- [Performance studies and capture checks](#performance-studies-and-capture-checks)
- [Repository, build and documentation checks](#repository-build-and-documentation-checks)
- [Website and project Story checks](#website-and-project-story-checks)
- [Additional quality gates](#additional-quality-gates)
- [Running the right tests](#running-the-right-tests)
- [Keeping this catalogue useful](#keeping-this-catalogue-useful)

## How to read the catalogue

Each description contains **10–15 words**. The linked suite name opens one source
module; the expandable list underneath provides all companion modules in that row.
Fixtures and configuration files are prerequisites, not extra passing tests.

| Prerequisite label | What it actually means |
| --- | --- |
| Rust / offline or loopback | Unit or integration checks; some open temporary files or local sockets. |
| Vitest / synthetic DOM | Frontend logic or component checks, not a real browser or native screen reader. |
| Browser — intercepted API | A real browser drives the UI, but API responses are supplied by fixtures. |
| Explicit configuration / harness | A separate runner, build, probe or capture configuration is needed. |
| Explicit private cases¹ | Some cases require authorized local data and explicit selection; ordinary cases may coexist. |
| LIVE | Real gateway/device contact; some tests write or reconfigure devices. Never include blindly. |

¹ Read each source's ignore reason and fixture requirements. Availability of a
private file is not permission to publish it, and a skipped test is not a pass.

## Privacy and repository hygiene

Original files stay private. The repository is a workshop, not a customer filing cabinet.


### Keeping other people's files out of Git

The engineering policy is **synthetic public fixtures, authorized private inputs,
reviewed public outputs**. The tests below cover parts of that policy; not every
protection is itself a test.

- **Repository exclusion:** [`.gitignore`](../.gitignore) excludes `OriginalData/`
  and the same-name symlink, runtime `data/`, local project dumps and private
  working state. Ignore rules prevent ordinary accidental additions; they do not
  remove tracked history or stop force-adding files. Review every outgoing change.
- **Fictional public fixtures:** sample projects and community demos are built
  from authored fictional specifications. Real customer archives are not needed
  to reproduce these public tests.
- **Private local regression inputs:** real project/product fixtures remain outside
  the tracked source inventory. The [corpus gate](../xtask/src/corpus_gates.rs)
  and [runner](../tools/run_corpus_tests.py) distinguish unavailable inputs from
  actual executed results. The runner clears hardware environment variables and
  isolates data directories; network isolation remains the caller's responsibility.
- **Reduced contribution reports:** [bundle tests](../crates/knx-app/tests/contribution_bundle.rs)
  check audience restrictions, explicit consent and key-material refusals. Public
  reduced exports are different from unmodified optional source samples or originals.
  Analysis on your instance may upload to **your own server**; this is not automatic
  public submission to a maintainer.
- **Diagnostic redaction:** [redaction tests](../apps/knx-server/src/debug_report_redaction_tests.rs)
  and [report tests](../apps/knx-server/src/debug_report.rs) cover network identifiers,
  home prefixes and hostnames in selected outputs. User text, KNX addresses and
  explicitly selected telegrams can still identify an installation.
- **Separate publication review:** Story privacy/approval checks and reduced
  contribution previews provide additional boundaries. Review reports before sharing;
  a green test cannot turn an arbitrary attachment into anonymous data.

**Fictional illustration — not an executed test:** an invented customer archive
`Example-Customer.knxproj` remains in private local storage. A public regression
uses a hand-authored miniature project instead. A reduced report can explain a
missing declaration without attaching that customer archive. Calling a real file
“example” would not make it fictional.

There is no absolute no-leak guarantee here. These controls reduce exposure;
source review, authorization and careful handling of attachments still matter.
See [contributing](manual/development/01-contributing.md#reporting-a-bug) and the
[contribution guide](contribution-intake/README.md) before sharing anything.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Diagnostic redaction](../apps/knx-server/src/debug_report.rs)** | Checks network identifiers, home paths and hostnames disappear from selected diagnostic outputs. | Rust / offline or loopback |
| **[Debug-report consent](../apps/knx-server/src/debug_report_routes.rs)** | Checks optional project summaries and telegram attachments require deliberate selection before local export. | Rust + Vitest / synthetic |
| **[Reduced contribution bundles](../apps/knx-cli/tests/contribution.rs)** | Checks reduced reports omit original sources, enforce consent and refuse unsafe disclosure choices. | Rust + Vitest / synthetic |
| **[Read-only AI disclosure boundary](../apps/knx-mcp/tests/retained_source.rs)** | Checks AI tool responses withhold retained source values while returning permitted read-only projections. | Rust / offline or loopback |
| **[Legacy secret removal](../crates/knx-productdb/tests/legacy_secrets.rs)** | Checks password columns are blanked while unrelated legacy payload bytes remain unchanged. | Rust / offline or loopback |
| **[Passwords never become output](../apps/knx-cli/tests/cli_legacy_inspect.rs)** | Checks protected imports reject wrong passwords without echoing credentials into responses or diagnostics. | Rust / offline or loopback |
| **[Locally remembered credentials](../apps/knx-web/src/LegacyPasswordSettings.test.tsx)** | Checks remembered legacy passwords use private storage and can be explicitly forgotten. | Rust + Vitest / synthetic |
| **[Private corpus means explicit execution](../tools/tests/test_run_corpus_tests.py)** | Checks missing private fixtures cannot masquerade as passing tests or empty regression runs. | Local runners / synthetic |
| **[Corpus reports and safe probe output](../crates/knx-productdb/tests/ar05_language_probe.rs)** | Checks corpus measurements distinguish outcomes, withhold package fingerprints and protect private input files. | Rust; explicit private cases¹ |
| **[Fictional documentation and demo fixtures](../crates/knx-app/examples/community_demos.rs)** | Checks generated demo projects use fictional data, deterministic packages and validated project structures. | Local runners / synthetic |
| **[Story privacy review](../story/tests/test_privacy.py)** | Checks nested story records and evidence excerpts against privacy rules before publication eligibility. | Python / local fixtures |

<details>
<summary>Source modules for this category</summary>

- **Diagnostic redaction:** [debug_report.rs](../apps/knx-server/src/debug_report.rs) · [debug_report_redaction_tests.rs](../apps/knx-server/src/debug_report_redaction_tests.rs)
- **Debug-report consent:** [debug_report_routes.rs](../apps/knx-server/src/debug_report_routes.rs) · [http_debug_report.rs](../apps/knx-server/tests/http_debug_report.rs) · [DebugReportButton.test.tsx](../apps/knx-web/src/DebugReportButton.test.tsx)
- **Reduced contribution bundles:** [contribution.rs](../apps/knx-cli/tests/contribution.rs) · [contribution_routes.rs](../apps/knx-server/src/contribution_routes.rs) · [http_contributions.rs](../apps/knx-server/tests/http_contributions.rs) · [ContributionButton.test.tsx](../apps/knx-web/src/ContributionButton.test.tsx) · [contributionApi.procedures.test.ts](../apps/knx-web/src/contributionApi.procedures.test.ts) · [contributionApi.test.ts](../apps/knx-web/src/contributionApi.test.ts) · [contribution.rs](../crates/knx-app/tests/contribution.rs) · [contribution_bundle.rs](../crates/knx-app/tests/contribution_bundle.rs) · [contribution_procedures.rs](../crates/knx-app/tests/contribution_procedures.rs)
- **Read-only AI disclosure boundary:** [retained_source.rs](../apps/knx-mcp/tests/retained_source.rs)
- **Legacy secret removal:** [legacy_secrets.rs](../crates/knx-productdb/tests/legacy_secrets.rs)
- **Passwords never become output:** [cli_legacy_inspect.rs](../apps/knx-cli/tests/cli_legacy_inspect.rs) · [cli_password_import.rs](../apps/knx-cli/tests/cli_password_import.rs) · [http_password_import.rs](../apps/knx-server/tests/http_password_import.rs) · [password_import.rs](../crates/knx-app/tests/password_import.rs) · [password_import.rs](../crates/knx-etsproj/tests/password_import.rs) · [knx-secure/src/lib.rs](../crates/knx-secure/src/lib.rs) · [zipcrypto.rs](../crates/knx-secure/src/zipcrypto.rs)
- **Locally remembered credentials:** [LegacyPasswordSettings.test.tsx](../apps/knx-web/src/LegacyPasswordSettings.test.tsx) · [legacy_remembered.rs](../crates/knx-app/tests/legacy_remembered.rs)
- **Private corpus means explicit execution:** [test_run_corpus_tests.py](../tools/tests/test_run_corpus_tests.py) · [corpus_gates.rs](../xtask/src/corpus_gates.rs)
- **Corpus reports and safe probe output:** [ar05_language_probe.rs](../crates/knx-productdb/tests/ar05_language_probe.rs) · [corpus_compatibility_matrix.rs](../crates/knx-productdb/tests/corpus_compatibility_matrix.rs)
- **Fictional documentation and demo fixtures:** [community_demos.rs](../crates/knx-app/examples/community_demos.rs) · [test_community_demos.py](../tools/tests/test_community_demos.py) · [test_manual_sample_project.py](../tools/tests/test_manual_sample_project.py)
- **Story privacy review:** [test_privacy.py](../story/tests/test_privacy.py)

</details>

## KNX domain and validation

Even an address deserves boundaries.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Address notation and allocation](../crates/knx-core/src/address.rs)** | Checks address parsing, formatting and allocation reject malformed values, duplicates and exhausted lines. | Rust / offline or loopback |
| **[Buildings and topology](../crates/knx-core/src/building.rs)** | Checks buildings, installations and topology preserve nesting, placement and explicit unassigned device states. | Rust / offline or loopback |
| **[Canonical devices and communication objects](../crates/knx-core/src/device.rs)** | Checks instance values, program defaults and communication objects retain ownership and override distinctions. | Rust / offline or loopback |
| **[Identity and raw source values](../crates/knx-core/src/ids.rs)** | Checks stable identifiers, module arguments and parameter values remain distinct and uninterpreted. | Rust / offline or loopback |
| **[Project identity and naming](../crates/knx-core/src/group_address_names.rs)** | Checks project identity, allocator high water and Unicode names preserve exact admitted text. | Rust / offline or loopback |
| **[Undo and redo](../crates/knx-core/src/command.rs)** | Checks failed edits retain history and successful inverses restore values without recycling identifiers. | Rust / offline or loopback |
| **[Validation and placement repair](../crates/knx-core/src/validation.rs)** | Checks dangling links and duplicate placements are diagnosed or repaired through explicit commands. | Rust / offline or loopback |
| **[Multiple installation ownership](../crates/knx-core/tests/multi_installation.rs)** | Checks edits, links and undo remain within the installation owning each entity. | Rust / offline or loopback |
| **[Localization and provenance](../crates/knx-core/src/provenance.rs)** | Checks language fallback and value provenance distinguish absence, malformed values and program defaults. | Rust / offline or loopback |
| **[Datapoint parsing and resolution](../crates/knx-core/src/dpt/mod.rs)** | Checks DPT identifiers parse correctly and conflicting linked declarations remain explicit conflicts. | Rust / offline or loopback |
| **[Datapoint encoding and decoding](../crates/knx-core/src/dpt/codec.rs)** | Checks supported payload codecs obey explicit input grammars, ranges and deterministic roundtrip behavior. | Rust / offline or loopback |
| **[Datapoint specification audits](../crates/knx-core/tests/dpt_spec_document_inventory.rs)** | Checks supported types match documented widths and semantics, with explicit refusals outside scope. | Rust / offline or loopback |

<details>
<summary>Source modules for this category</summary>

- **Address notation and allocation:** [address.rs](../crates/knx-core/src/address.rs) · [allocation.rs](../crates/knx-core/src/allocation.rs) · [scan.rs](../crates/knx-core/src/scan.rs)
- **Buildings and topology:** [building.rs](../crates/knx-core/src/building.rs) · [group.rs](../crates/knx-core/src/group.rs) · [installation.rs](../crates/knx-core/src/installation.rs) · [topology.rs](../crates/knx-core/src/topology.rs)
- **Canonical devices and communication objects:** [device.rs](../crates/knx-core/src/device.rs) · [devices.rs](../crates/knx-core/src/devices.rs) · [flags.rs](../crates/knx-core/src/flags.rs)
- **Identity and raw source values:** [ids.rs](../crates/knx-core/src/ids.rs) · [module.rs](../crates/knx-core/src/module.rs) · [parameter.rs](../crates/knx-core/src/parameter.rs)
- **Project identity and naming:** [group_address_names.rs](../crates/knx-core/src/group_address_names.rs) · [names.rs](../crates/knx-core/src/names.rs) · [project.rs](../crates/knx-core/src/project.rs)
- **Undo and redo:** [command.rs](../crates/knx-core/src/command.rs)
- **Validation and placement repair:** [validation.rs](../crates/knx-core/src/validation.rs) · [topology_repair.rs](../crates/knx-core/tests/topology_repair.rs)
- **Multiple installation ownership:** [multi_installation.rs](../crates/knx-core/tests/multi_installation.rs)
- **Localization and provenance:** [provenance.rs](../crates/knx-core/src/provenance.rs) · [string_table.rs](../crates/knx-core/src/string_table.rs)
- **Datapoint parsing and resolution:** [src/dpt/mod.rs](../crates/knx-core/src/dpt/mod.rs) · [resolve.rs](../crates/knx-core/src/dpt/resolve.rs)
- **Datapoint encoding and decoding:** [codec.rs](../crates/knx-core/src/dpt/codec.rs)
- **Datapoint specification audits:** [dpt_spec_document_inventory.rs](../crates/knx-core/tests/dpt_spec_document_inventory.rs) · [dpt_spec_semantics_audit.rs](../crates/knx-core/tests/dpt_spec_semantics_audit.rs) · [dpt_spec_width_audit.rs](../crates/knx-core/tests/dpt_spec_width_audit.rs)

</details>

## Commissioning plans and recovery

A plan is not a permission slip.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Device facts and authorization](../crates/knx-app/src/access_key.rs)** | Checks commissioning facts never arise from project intent and authorization levels remain explicit. | Rust / offline or loopback |
| **[Write-scope confirmation](../crates/knx-core/src/commissioning/mutation.rs)** | Checks confirmation phrases bind writes to one target and one authorized operation scope. | Rust / offline or loopback |
| **[Backup durability and restore admission](../crates/knx-app/src/backup_directory.rs)** | Checks durable backups preserve recovery bytes and refuse incompatible or incomplete restoration inputs. | Rust / offline or loopback |
| **[Programming identity and device properties](../crates/knx-core/src/commissioning/domain_address.rs)** | Checks serials, domain addresses and property fields decode without guessing missing device facts. | Rust / offline or loopback |
| **[Group tables and parameter images](../crates/knx-core/src/commissioning/group_object_table.rs)** | Checks compiled tables and parameter images preserve surrounding bytes and expected object mappings. | Rust / offline or loopback |
| **[Load states and reset encodings](../crates/knx-core/src/commissioning/error_code.rs)** | Checks load transitions, control records and reset codes use explicit permitted wire representations. | Rust / offline or loopback |
| **[Complete and partial download plans](../crates/knx-core/src/commissioning/memory.rs)** | Checks ordered procedures, transfer sizes and partial scopes preserve guards before device mutation. | Rust / offline or loopback |
| **[Offline preparation and evidence grades](../crates/knx-app/src/device_download.rs)** | Checks device preparation rejects ambiguity and keeps verified evidence separate from untested plans. | Rust / offline or loopback |
| **[Durable commissioning activity](../crates/knx-app/src/commissioning_activity.rs)** | Checks recorded outcomes survive interruptions without replacing witnessed results or inventing completion facts. | Rust / offline or loopback |
| **[Private download and readiness regressions](../crates/knx-app/tests/download_coverage_corpus.rs)** | Checks private offline plans, coverage and instance flags against explicitly selected fixture expectations. | Rust; explicit private cases¹ |

<details>
<summary>Source modules for this category</summary>

- **Device facts and authorization:** [access_key.rs](../crates/knx-app/src/access_key.rs) · [commissioning.rs](../crates/knx-core/src/commissioning.rs) · [authorisation.rs](../crates/knx-core/src/commissioning/authorisation.rs)
- **Write-scope confirmation:** [mutation.rs](../crates/knx-core/src/commissioning/mutation.rs) · [hardware_write_gate.rs](../crates/knx-net/tests/hardware_write_gate.rs)
- **Backup durability and restore admission:** [backup_directory.rs](../crates/knx-app/src/backup_directory.rs) · [device_backup.rs](../crates/knx-app/src/device_backup.rs) · [service_control_backup.rs](../crates/knx-app/src/service_control_backup.rs) · [device_backup.rs](../crates/knx-core/src/commissioning/device_backup.rs)
- **Programming identity and device properties:** [domain_address.rs](../crates/knx-core/src/commissioning/domain_address.rs) · [programming_mode.rs](../crates/knx-core/src/commissioning/programming_mode.rs) · [properties.rs](../crates/knx-core/src/commissioning/properties.rs) · [serial_number.rs](../crates/knx-core/src/commissioning/serial_number.rs)
- **Group tables and parameter images:** [group_object_table.rs](../crates/knx-core/src/commissioning/group_object_table.rs) · [group_tables.rs](../crates/knx-core/src/commissioning/group_tables.rs) · [parameter_image.rs](../crates/knx-core/src/commissioning/parameter_image.rs)
- **Load states and reset encodings:** [error_code.rs](../crates/knx-core/src/commissioning/error_code.rs) · [load_control.rs](../crates/knx-core/src/commissioning/load_control.rs) · [load_control_memory.rs](../crates/knx-core/src/commissioning/load_control_memory.rs) · [load_state.rs](../crates/knx-core/src/commissioning/load_state.rs) · [master_reset.rs](../crates/knx-core/src/commissioning/master_reset.rs) · [mcb.rs](../crates/knx-core/src/commissioning/mcb.rs)
- **Complete and partial download plans:** [memory.rs](../crates/knx-core/src/commissioning/memory.rs) · [memory_download.rs](../crates/knx-core/src/commissioning/memory_download.rs) · [partial_download_variant.rs](../crates/knx-core/src/commissioning/partial_download_variant.rs) · [partial_memory_download.rs](../crates/knx-core/src/commissioning/partial_memory_download.rs) · [procedure.rs](../crates/knx-core/src/commissioning/procedure.rs) · [rf_configuration.rs](../crates/knx-core/src/commissioning/rf_configuration.rs)
- **Offline preparation and evidence grades:** [device_download.rs](../crates/knx-app/src/device_download.rs) · [download_support.rs](../crates/knx-app/src/download_support.rs) · [project_readiness.rs](../crates/knx-app/src/project_readiness.rs) · [serial_number.rs](../crates/knx-app/src/serial_number.rs) · [secure_capable_not_activated.rs](../crates/knx-app/tests/secure_capable_not_activated.rs)
- **Durable commissioning activity:** [commissioning_activity.rs](../crates/knx-app/src/commissioning_activity.rs) · [commissioning_activity.rs](../crates/knx-app/tests/commissioning_activity.rs)
- **Private download and readiness regressions:** [download_coverage_corpus.rs](../crates/knx-app/tests/download_coverage_corpus.rs) · [house_instance_flags.rs](../crates/knx-app/tests/house_instance_flags.rs) · [house_readiness.rs](../crates/knx-app/tests/house_readiness.rs) · [legacy_download_oracle.rs](../crates/knx-app/tests/legacy_download_oracle.rs)

</details>

## Project import and interchange

Unknown data gets a label, not a trapdoor.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Container inventory and version detection](../crates/knx-etsproj/src/container.rs)** | Checks archive inventory and format detection reject invalid containers and identify supported source schemas. | Rust; explicit private cases¹ |
| **[Archive member identity](../crates/knx-etsproj/tests/archive_member_integrity.rs)** | Checks duplicate, case-colliding and decoded filenames cannot redirect source members into another identity. | Rust / offline or loopback |
| **[Source parsing and unknown preservation](../crates/knx-etsproj/src/opaque.rs)** | Checks project metadata and installation parsers retain unknown elements and exact source values. | Rust; explicit private cases¹ |
| **[Normalized mapping and reference validation](../crates/knx-etsproj/src/id_table.rs)** | Checks source identifiers map consistently and ambiguous or dangling references produce explicit diagnostics. | Rust; explicit private cases¹ |
| **[Importer reports and progress](../crates/knx-etsproj/src/lib.rs)** | Checks import counts, diagnostics and stage progress describe actual work and preserved unsupported content. | Rust; explicit private cases¹ |
| **[Communication-object source semantics](../crates/knx-etsproj/tests/com_object_activity.rs)** | Checks local object identities, activation and declared DPT states survive supported project imports. | Rust; explicit private cases¹ |
| **[Link directions and installation scope](../crates/knx-etsproj/tests/links_direction.rs)** | Checks sending links follow source order and repeated identifiers never cross installation boundaries. | Rust; explicit private cases¹ |
| **[Metadata and unsupported project parts](../crates/knx-etsproj/tests/download_state.rs)** | Checks languages, download state, site hierarchy and additional project parts receive explicit treatment. | Rust / offline or loopback |
| **[Malformed and private reference projects](../crates/knx-etsproj/src/compare.rs)** | Checks malformed inputs fail clearly and private reference imports match narrowly pinned expectations. | Rust; explicit private cases¹ |
| **[Application import and enrichment](../crates/knx-app/tests/enrichment_gap_measurement.rs)** | Checks importer services persist preserved sources and report manufacturer enrichment without inventing values. | Rust; explicit private cases¹ |
| **[Selective project import](../crates/knx-app/tests/selective_import.rs)** | Checks selected devices bring their dependencies atomically, with preview refusal and reversible native persistence. | Rust / offline or loopback |
| **[Authorized restore-export fixtures](../crates/knx-app/tests/authorized_restore_exports.rs)** | Checks authorized export manifests reject unsafe paths and compare explicitly supplied private baselines. | Rust; explicit private cases¹ |
| **[Legacy files and conversion oracles](../crates/knx-app/tests/legacy_corpus.rs)** | Checks legacy containers decrypt correctly and private conversion expectations remain separately gated regressions. | Rust; explicit private cases¹ |
| **[CSV parsing and rendering](../crates/knx-csv/src/read.rs)** | Checks delimiter variants, quoting and address styles survive CSV parsing and export roundtrips. | Rust / offline or loopback |
| **[CSV planning and allocator integrity](../crates/knx-app/tests/csv_roundtrip.rs)** | Checks stale plans, exhausted identifiers and CSV batches cannot partially corrupt existing project state. | Rust; explicit private cases¹ |

<details>
<summary>Source modules for this category</summary>

- **Container inventory and version detection:** [container.rs](../crates/knx-etsproj/src/container.rs) · [detect.rs](../crates/knx-etsproj/src/detect.rs) · [known.rs](../crates/knx-etsproj/src/known.rs)
- **Archive member identity:** [archive_member_integrity.rs](../crates/knx-etsproj/tests/archive_member_integrity.rs) · [decoded_member_names.rs](../crates/knx-etsproj/tests/decoded_member_names.rs)
- **Source parsing and unknown preservation:** [opaque.rs](../crates/knx-etsproj/src/opaque.rs) · [installation.rs](../crates/knx-etsproj/src/parse/installation.rs) · [installation_v21.rs](../crates/knx-etsproj/src/parse/installation_v21.rs) · [project_info.rs](../crates/knx-etsproj/src/parse/project_info.rs) · [values.rs](../crates/knx-etsproj/src/values.rs)
- **Normalized mapping and reference validation:** [id_table.rs](../crates/knx-etsproj/src/id_table.rs) · [infer.rs](../crates/knx-etsproj/src/infer.rs) · [map.rs](../crates/knx-etsproj/src/map.rs) · [validate.rs](../crates/knx-etsproj/src/validate.rs)
- **Importer reports and progress:** [knx-etsproj/src/lib.rs](../crates/knx-etsproj/src/lib.rs) · [progress.rs](../crates/knx-etsproj/src/progress.rs) · [report.rs](../crates/knx-etsproj/src/report.rs)
- **Communication-object source semantics:** [com_object_activity.rs](../crates/knx-etsproj/tests/com_object_activity.rs) · [device_local_com_object_refs.rs](../crates/knx-etsproj/tests/device_local_com_object_refs.rs) · [group_address_declared_dpt.rs](../crates/knx-etsproj/tests/group_address_declared_dpt.rs) · [private_schema23_refids.rs](../crates/knx-etsproj/tests/private_schema23_refids.rs)
- **Link directions and installation scope:** [links_direction.rs](../crates/knx-etsproj/tests/links_direction.rs) · [links_installation_scope.rs](../crates/knx-etsproj/tests/links_installation_scope.rs)
- **Metadata and unsupported project parts:** [download_state.rs](../crates/knx-etsproj/tests/download_state.rs) · [project_language.rs](../crates/knx-etsproj/tests/project_language.rs) · [second_project_part.rs](../crates/knx-etsproj/tests/second_project_part.rs) · [site_hierarchy.rs](../crates/knx-etsproj/tests/site_hierarchy.rs)
- **Malformed and private reference projects:** [compare.rs](../crates/knx-etsproj/src/compare.rs) · [golden_reference_project.rs](../crates/knx-etsproj/tests/golden_reference_project.rs) · [malformed_input.rs](../crates/knx-etsproj/tests/malformed_input.rs) · [oracle_xknxproject.rs](../crates/knx-etsproj/tests/oracle_xknxproject.rs)
- **Application import and enrichment:** [enrichment_gap_measurement.rs](../crates/knx-app/tests/enrichment_gap_measurement.rs) · [ets6_device_local_enrichment.rs](../crates/knx-app/tests/ets6_device_local_enrichment.rs) · [import_service.rs](../crates/knx-app/tests/import_service.rs) · [product_db.rs](../crates/knx-app/tests/product_db.rs)
- **Selective project import:** [selective_import.rs](../crates/knx-app/tests/selective_import.rs)
- **Authorized restore-export fixtures:** [authorized_restore_exports.rs](../crates/knx-app/tests/authorized_restore_exports.rs)
- **Legacy files and conversion oracles:** [legacy_corpus.rs](../crates/knx-app/tests/legacy_corpus.rs) · [legacy_files.rs](../crates/knx-app/tests/legacy_files.rs) · [legacy_oracle.rs](../crates/knx-app/tests/legacy_oracle.rs)
- **CSV parsing and rendering:** [read.rs](../crates/knx-csv/src/read.rs) · [write.rs](../crates/knx-csv/src/write.rs)
- **CSV planning and allocator integrity:** [csv_roundtrip.rs](../crates/knx-app/tests/csv_roundtrip.rs) · [id_allocation_integrity.rs](../crates/knx-app/tests/id_allocation_integrity.rs) · [id_exhaustion.rs](../crates/knx-app/tests/id_exhaustion.rs) · [plan.rs](../crates/knx-csv/src/plan.rs)

</details>

## Manufacturer databases and package ingestion

Manufacturers bring plenty of paperwork. We keep the receipts.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Blobs, identities and package ingestion](../crates/knx-productdb/src/blob.rs)** | Checks content identities, retained blobs and duplicate ingestion preserve original bytes and ownership. | Rust / offline or loopback |
| **[Catalogue, hardware and master parsers](../crates/knx-productdb/src/parse/catalog.rs)** | Checks manufacturers, catalogue items and hardware relations store supported fields and report unknown attributes. | Rust / offline or loopback |
| **[Programs, parameters and communication objects](../crates/knx-productdb/src/parse/comobject.rs)** | Checks program declarations, parameter kinds and object overrides retain supported values and explicit unknowns. | Rust / offline or loopback |
| **[Translations and source catalogue metadata](../crates/knx-productdb/src/parse/translation.rs)** | Checks source metadata and translations keep exact ownership without implying executable Secure capability. | Rust / offline or loopback |
| **[Retained evidence and unknown reports](../crates/knx-productdb/src/parse/scheme_evidence.rs)** | Checks stored unknowns and explicitly rebuilt evidence remain stable, scoped and transactionally consistent. | Rust / offline or loopback |
| **[Baggage and nested file inventory](../crates/knx-productdb/src/baggage.rs)** | Checks baggage declarations resolve by exact ownership while content classification avoids misleading filenames. | Rust / offline or loopback |
| **[XML and archive admission boundaries](../crates/knx-productdb/src/xml.rs)** | Checks XML and package limits refuse malformed or oversized input without partial database changes. | Rust / offline or loopback |
| **[Supported package schema branches](../crates/knx-productdb/tests/scheme10.rs)** | Checks supported schema adapters preserve evidence and reject foreign namespaces without publishing partial rows. | Rust / offline or loopback |
| **[Package conflicts and replay](../crates/knx-productdb/tests/package_identity.rs)** | Checks repeated installations retain reports, disclose competing identities and preserve atomic ingestion boundaries. | Rust; explicit private cases¹ |
| **[Database migration and read-only opening](../crates/knx-productdb/src/migration.rs)** | Checks manufacturer database upgrades preserve retained data and read-only consumers leave source files untouched. | Rust; explicit private cases¹ |
| **[Catalogue queries and enrichment](../crates/knx-productdb/src/enrich.rs)** | Checks translated catalogue queries and program enrichment preserve explicit project overrides and missing values. | Rust / offline or loopback |
| **[Legacy product format adapters](../crates/knx-productdb/src/legacy/code.rs)** | Checks legacy containers, text decoding and program mapping preserve supported declarations and source provenance. | Rust; explicit private cases¹ |
| **[Private reference products and parameter views](../crates/knx-productdb/tests/golden_reference_products.rs)** | Checks private manufacturer fixtures retain blob integrity and expected parameter views under explicit selection. | Rust; explicit private cases¹ |

<details>
<summary>Source modules for this category</summary>

- **Blobs, identities and package ingestion:** [blob.rs](../crates/knx-productdb/src/blob.rs) · [identity.rs](../crates/knx-productdb/src/identity.rs) · [ingest.rs](../crates/knx-productdb/src/ingest.rs) · [package.rs](../crates/knx-productdb/src/package.rs)
- **Catalogue, hardware and master parsers:** [catalog.rs](../crates/knx-productdb/src/parse/catalog.rs) · [hardware.rs](../crates/knx-productdb/src/parse/hardware.rs) · [master.rs](../crates/knx-productdb/src/parse/master.rs) · [src/parse/mod.rs](../crates/knx-productdb/src/parse/mod.rs)
- **Programs, parameters and communication objects:** [comobject.rs](../crates/knx-productdb/src/parse/comobject.rs) · [program.rs](../crates/knx-productdb/src/parse/program.rs) · [parameter_attribute_unknowns.rs](../crates/knx-productdb/tests/parameter_attribute_unknowns.rs) · [parameter_kinds.rs](../crates/knx-productdb/tests/parameter_kinds.rs)
- **Translations and source catalogue metadata:** [translation.rs](../crates/knx-productdb/src/parse/translation.rs) · [catalog_metadata.rs](../crates/knx-productdb/tests/catalog_metadata.rs) · [channel_name_number.rs](../crates/knx-productdb/tests/channel_name_number.rs) · [master_language_evidence.rs](../crates/knx-productdb/tests/master_language_evidence.rs)
- **Retained evidence and unknown reports:** [scheme_evidence.rs](../crates/knx-productdb/src/parse/scheme_evidence.rs) · [report.rs](../crates/knx-productdb/src/report.rs) · [install_reports.rs](../crates/knx-productdb/tests/install_reports.rs) · [master_evidence_rederive.rs](../crates/knx-productdb/tests/master_evidence_rederive.rs)
- **Baggage and nested file inventory:** [baggage.rs](../crates/knx-productdb/src/baggage.rs) · [baggage.rs](../crates/knx-productdb/src/parse/baggage.rs) · [baggage_inventory.rs](../crates/knx-productdb/tests/baggage_inventory.rs)
- **XML and archive admission boundaries:** [xml.rs](../crates/knx-productdb/src/xml.rs) · [malformed_input.rs](../crates/knx-productdb/tests/malformed_input.rs) · [zip_cap_boundaries.rs](../crates/knx-productdb/tests/zip_cap_boundaries.rs)
- **Supported package schema branches:** [scheme10.rs](../crates/knx-productdb/tests/scheme10.rs) · [scheme12_14.rs](../crates/knx-productdb/tests/scheme12_14.rs) · [scheme13.rs](../crates/knx-productdb/tests/scheme13.rs) · [scheme21.rs](../crates/knx-productdb/tests/scheme21.rs) · [scheme23.rs](../crates/knx-productdb/tests/scheme23.rs)
- **Package conflicts and replay:** [package_identity.rs](../crates/knx-productdb/tests/package_identity.rs) · [public_api_compat.rs](../crates/knx-productdb/tests/public_api_compat.rs) · [standalone_packages.rs](../crates/knx-productdb/tests/standalone_packages.rs)
- **Database migration and read-only opening:** [migration.rs](../crates/knx-productdb/src/migration.rs) · [ar05_corpus_upgrade.rs](../crates/knx-productdb/tests/ar05_corpus_upgrade.rs) · [read_only_open.rs](../crates/knx-productdb/tests/read_only_open.rs)
- **Catalogue queries and enrichment:** [enrich.rs](../crates/knx-productdb/src/enrich.rs) · [query.rs](../crates/knx-productdb/src/query.rs)
- **Legacy product format adapters:** [code.rs](../crates/knx-productdb/src/legacy/code.rs) · [text.rs](../crates/knx-productdb/src/legacy/text.rs) · [legacy_container.rs](../crates/knx-productdb/tests/legacy_container.rs) · [legacy_exim.rs](../crates/knx-productdb/tests/legacy_exim.rs) · [legacy_mapping.rs](../crates/knx-productdb/tests/legacy_mapping.rs) · [legacy_member_names_corpus.rs](../crates/knx-productdb/tests/legacy_member_names_corpus.rs) · [legacy_publish.rs](../crates/knx-productdb/tests/legacy_publish.rs)
- **Private reference products and parameter views:** [golden_reference_products.rs](../crates/knx-productdb/tests/golden_reference_products.rs) · [parameter_views_corpus.rs](../crates/knx-productdb/tests/parameter_views_corpus.rs)

</details>

## Parameter evaluation and offline images

A pretty parameter label is not permission to write it.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Dynamic tree and channel ownership](../crates/knx-productdb/tests/dynamic_channel_owner.rs)** | Checks dynamic declarations preserve ordering, channel ownership and explicitly unsupported control kinds. | Rust; explicit private cases¹ |
| **[Module scope isolation](../crates/knx-productdb/tests/nested_module_private.rs)** | Checks nested module declarations keep separate dynamic trees and restore their enclosing argument scopes. | Rust; explicit private cases¹ |
| **[Evaluation and write authority](../crates/knx-productdb/tests/device_evaluation.rs)** | Checks parameter values follow scoped choices and writable fields respect access and calculation declarations. | Rust / offline or loopback |
| **[Evaluation budgets and text amplification](../crates/knx-productdb/src/dynamic/evaluate/text_projection_tests.rs)** | Checks repeated traversal and copied labels consume bounded budgets before unsafe evaluation proceeds. | Rust / offline or loopback |
| **[Offline program code and image requests](../crates/knx-productdb/src/code.rs)** | Checks code segments, masks and requested configurations preserve byte boundaries and explicit refusal conditions. | Rust; explicit private cases¹ |
| **[Read-only AP1 procedure resolution](../crates/knx-app/tests/procedure_resolution_corpus.rs)** | Checks retained procedure fragments resolve in order while omissions and cycles remain explicit diagnostics. | Rust; explicit private cases¹ |

<details>
<summary>Source modules for this category</summary>

- **Dynamic tree and channel ownership:** [dynamic_channel_owner.rs](../crates/knx-productdb/tests/dynamic_channel_owner.rs) · [dynamic_control_kind.rs](../crates/knx-productdb/tests/dynamic_control_kind.rs) · [dynamic_tree.rs](../crates/knx-productdb/tests/dynamic_tree.rs)
- **Module scope isolation:** [nested_module_private.rs](../crates/knx-productdb/tests/nested_module_private.rs) · [nested_module_storage.rs](../crates/knx-productdb/tests/nested_module_storage.rs)
- **Evaluation and write authority:** [device_evaluation.rs](../crates/knx-productdb/tests/device_evaluation.rs) · [write_authority.rs](../crates/knx-productdb/tests/write_authority.rs)
- **Evaluation budgets and text amplification:** [text_projection_tests.rs](../crates/knx-productdb/src/dynamic/evaluate/text_projection_tests.rs) · [work_budget_tests.rs](../crates/knx-productdb/src/dynamic/evaluate/work_budget_tests.rs) · [tests/dynamic_scalar_copy/mod.rs](../crates/knx-productdb/tests/dynamic_scalar_copy/mod.rs)
- **Offline program code and image requests:** [code.rs](../crates/knx-productdb/src/code.rs) · [download_plan.rs](../crates/knx-productdb/src/download_plan.rs) · [image.rs](../crates/knx-productdb/src/image.rs) · [image_request.rs](../crates/knx-productdb/src/image_request.rs) · [program_code.rs](../crates/knx-productdb/tests/program_code.rs)
- **Read-only AP1 procedure resolution:** [procedure_resolution_corpus.rs](../crates/knx-app/tests/procedure_resolution_corpus.rs) · [xml.rs](../crates/knx-productdb/src/procedure_resolution/xml.rs) · [procedure_resolution.rs](../crates/knx-productdb/tests/procedure_resolution.rs)

</details>

## Native storage and project history

Save means save. Preferably all of it.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Complete project roundtrips](../crates/knx-store/src/building.rs)** | Checks saved projects preserve buildings, topology and device data across native reopen cycles. | Rust; explicit private cases¹ |
| **[Ordered values and source retention](../crates/knx-store/src/group.rs)** | Checks addresses, modules, parameters, strings and retained source manifests survive ordered persistence. | Rust / offline or loopback |
| **[Atomic save and structural integrity](../crates/knx-store/tests/lossless_save.rs)** | Checks invalid ownership and late database failures leave existing saved project content unchanged. | Rust / offline or loopback |
| **[Persisted edits and inverses](../crates/knx-store/src/command_sync.rs)** | Checks edited fields and structural command batches survive save, undo and subsequent reopen. | Rust / offline or loopback |
| **[Native schema migrations](../crates/knx-store/src/migration.rs)** | Checks upgrades roll back on failure and reject newer stores without damaging existing files. | Rust / offline or loopback |
| **[Read-only project opening](../crates/knx-store/tests/read_only_open.rs)** | Checks read-only consumers migrate in memory and never rewrite their original native project files. | Rust / offline or loopback |
| **[Persistent history and named versions](../crates/knx-store/src/project_history/snapshot.rs)** | Checks recovery journals preserve working snapshots, edit stacks and independently identified saved baselines. | Rust / offline or loopback |
| **[Compare-and-save generation guards](../crates/knx-store/tests/history_compare_and_save.rs)** | Checks unchanged journal generations cannot authorize overwriting a saved root never reviewed. | Rust / offline or loopback |
| **[Durable bus activity history](../crates/knx-store/src/activity_history.rs)** | Checks durable activity metadata preserves terminal evidence and refuses foreign databases or orphan sidecars. | Rust / offline or loopback |

<details>
<summary>Source modules for this category</summary>

- **Complete project roundtrips:** [building.rs](../crates/knx-store/src/building.rs) · [devices.rs](../crates/knx-store/src/devices.rs) · [project.rs](../crates/knx-store/src/project.rs) · [topology.rs](../crates/knx-store/src/topology.rs) · [reference_project.rs](../crates/knx-store/tests/reference_project.rs)
- **Ordered values and source retention:** [group.rs](../crates/knx-store/src/group.rs) · [manifest.rs](../crates/knx-store/src/manifest.rs) · [module_instance.rs](../crates/knx-store/src/module_instance.rs) · [opaque.rs](../crates/knx-store/src/opaque.rs) · [parameter.rs](../crates/knx-store/src/parameter.rs) · [strings.rs](../crates/knx-store/src/strings.rs)
- **Atomic save and structural integrity:** [lossless_save.rs](../crates/knx-store/tests/lossless_save.rs) · [store_integrity.rs](../crates/knx-store/tests/store_integrity.rs)
- **Persisted edits and inverses:** [command_sync.rs](../crates/knx-store/src/command_sync.rs) · [command_persistence.rs](../crates/knx-store/tests/command_persistence.rs)
- **Native schema migrations:** [migration.rs](../crates/knx-store/src/migration.rs)
- **Read-only project opening:** [read_only_open.rs](../crates/knx-store/tests/read_only_open.rs)
- **Persistent history and named versions:** [snapshot.rs](../crates/knx-store/src/project_history/snapshot.rs) · [project_history.rs](../crates/knx-store/tests/project_history.rs)
- **Compare-and-save generation guards:** [history_compare_and_save.rs](../crates/knx-store/tests/history_compare_and_save.rs)
- **Durable bus activity history:** [activity_history.rs](../crates/knx-store/src/activity_history.rs)

</details>

## Reports, comparisons and projections

A difference is a fact; a guessed match is a plot twist.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Normalized project projections](../crates/knx-projection/src/lib.rs)** | Checks projected trees preserve canonical ownership, address styles and supported building structure. | Rust / offline or loopback |
| **[Diff matching and ambiguity](../crates/knx-app/tests/comparison_input.rs)** | Checks identity matching and semantic comparison expose ambiguity rather than guessing corresponding entities. | Rust; explicit private cases¹ |
| **[Self-contained HTML reports](../crates/knx-app/tests/documentation_composition.rs)** | Checks report hierarchies, warnings and escaped text render deterministically without external document dependencies. | Rust; explicit private cases¹ |
| **[Seeded projects and import progress](../crates/knx-app/src/project_seed.rs)** | Checks structured project seeds and progress updates reflect actual admitted data and completed work. | Rust / offline or loopback |

<details>
<summary>Source modules for this category</summary>

- **Normalized project projections:** [knx-projection/src/lib.rs](../crates/knx-projection/src/lib.rs)
- **Diff matching and ambiguity:** [comparison_input.rs](../crates/knx-app/tests/comparison_input.rs) · [diff_correlation_measurement.rs](../crates/knx-app/tests/diff_correlation_measurement.rs) · [project_diff.rs](../crates/knx-app/tests/project_diff.rs) · [diff.rs](../crates/knx-diff/src/diff.rs) · [key.rs](../crates/knx-diff/src/key.rs) · [semantic.rs](../crates/knx-diff/src/semantic.rs)
- **Self-contained HTML reports:** [documentation_composition.rs](../crates/knx-app/tests/documentation_composition.rs) · [documentation_export.rs](../crates/knx-app/tests/documentation_export.rs) · [html.rs](../crates/knx-report/src/html.rs) · [model.rs](../crates/knx-report/src/model.rs) · [render.rs](../crates/knx-report/src/render.rs)
- **Seeded projects and import progress:** [project_seed.rs](../crates/knx-app/src/project_seed.rs) · [load_progress.rs](../crates/knx-app/tests/load_progress.rs)

</details>

## KNXnet/IP and simulated device communication

Most devices here are simulated. Their complaints are still useful.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Frames and cEMI payloads](../crates/knx-net/src/cemi.rs)** | Checks frame lengths, cEMI fields and control evidence survive encoding and malformed-input rejection. | Rust / offline or loopback |
| **[Discovery, endpoints and device information](../crates/knx-net/src/core/dib.rs)** | Checks discovery packets, endpoint structures and device information decode validated lengths and unknown blocks. | Rust / offline or loopback |
| **[Tunnels, routing and connection lifetime](../crates/knx-net/src/client.rs)** | Checks loopback transport, acknowledgments and disconnect handling obey packet identities and connection boundaries. | Loopback / simulated |
| **[Line scans and project comparisons](../crates/knx-net/src/scan.rs)** | Checks exclusions, cancellation and observed scan outcomes remain separate from configured project expectations. | Rust / offline or loopback |
| **[Simulated addressing and identification](../crates/knx-net/src/commissioning/domain_address.rs)** | Checks simulated device addressing validates responders, serial identity and restart uncertainty before reporting outcomes. | Simulated |
| **[Simulated download and reset procedures](../crates/knx-net/src/commissioning/download.rs)** | Checks simulated downloads and resets enforce device guards and preserve witnessed partial outcomes. | Simulated |
| **[Simulated service-control recovery](../crates/knx-net/src/commissioning/service_control.rs)** | Checks recovery callbacks precede property writes and malformed control data prevents device mutation. | Simulated |
| **[Private telegram replay](../crates/knx-net/tests/private_telegram_log.rs)** | Checks locally captured frames roundtrip faithfully while aggregate census output withholds payload values. | Rust; explicit private cases¹ |
| **[Private image simulator regressions](../apps/knx-cli/tests/memory_download_simulated.rs)** | Checks explicitly supplied private images reach simulated memory without inventing acknowledged restart outcomes. | Private + simulated |

<details>
<summary>Source modules for this category</summary>

- **Frames and cEMI payloads:** [cemi.rs](../crates/knx-net/src/cemi.rs) · [frame.rs](../crates/knx-net/src/frame.rs)
- **Discovery, endpoints and device information:** [dib.rs](../crates/knx-net/src/core/dib.rs) · [hpai.rs](../crates/knx-net/src/core/hpai.rs) · [services.rs](../crates/knx-net/src/core/services.rs) · [discovery.rs](../crates/knx-net/src/discovery.rs)
- **Tunnels, routing and connection lifetime:** [client.rs](../crates/knx-net/src/client.rs) · [commissioning.rs](../crates/knx-net/src/commissioning.rs) · [routing.rs](../crates/knx-net/src/routing.rs) · [tunnelling.rs](../crates/knx-net/src/tunnelling.rs)
- **Line scans and project comparisons:** [scan.rs](../crates/knx-net/src/scan.rs)
- **Simulated addressing and identification:** [domain_address.rs](../crates/knx-net/src/commissioning/domain_address.rs) · [individual_address_reset.rs](../crates/knx-net/src/commissioning/individual_address_reset.rs) · [individual_address_write.rs](../crates/knx-net/src/commissioning/individual_address_write.rs) · [programming_button_wait.rs](../crates/knx-net/src/commissioning/programming_button_wait.rs) · [serial_number_write.rs](../crates/knx-net/src/commissioning/serial_number_write.rs)
- **Simulated download and reset procedures:** [download.rs](../crates/knx-net/src/commissioning/download.rs) · [master_reset.rs](../crates/knx-net/src/commissioning/master_reset.rs) · [memory_download.rs](../crates/knx-net/src/commissioning/memory_download.rs) · [rf_configuration.rs](../crates/knx-net/src/commissioning/rf_configuration.rs)
- **Simulated service-control recovery:** [service_control.rs](../crates/knx-net/src/commissioning/service_control.rs)
- **Private telegram replay:** [private_telegram_log.rs](../crates/knx-net/tests/private_telegram_log.rs)
- **Private image simulator regressions:** [memory_download_simulated.rs](../apps/knx-cli/tests/memory_download_simulated.rs)

</details>

## Explicit live hardware tests

Real hardware is not a faster unit-test runner.

> **Warning**
>
> These ignored tests contact real hardware. Writing tests need fresh, device-specific
> authorization and verified recovery. Do not run a blanket `--include-ignored`
> or `--ignored` sweep: read-only tests are not authorization for writing tests.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Gateway discovery and telegram traffic](../crates/knx-net/tests/live_gateway.rs)** | Checks a configured real gateway discovers, connects and acknowledges explicitly selected telegram operations. | LIVE — includes write |
| **[Read-only device observations](../crates/knx-net/tests/live_commissioning_readonly.rs)** | Checks explicitly selected live identification, programming status and memory reads report actual device responses. | LIVE — read-only |
| **[Individual-address programming](../crates/knx-net/tests/live_individual_address_write.rs)** | Checks authorized live address assignment against the selected device currently in programming mode. | LIVE — changes address |
| **[Live memory download and readback](../apps/knx-cli/tests/live_memory_download.rs)** | Checks an explicitly authorized real-device configuration download against subsequent device memory readback. | LIVE — rewrites device |

<details>
<summary>Source modules for this category</summary>

- **Gateway discovery and telegram traffic:** [live_gateway.rs](../crates/knx-net/tests/live_gateway.rs)
- **Read-only device observations:** [live_commissioning_readonly.rs](../crates/knx-net/tests/live_commissioning_readonly.rs) · [live_identify.rs](../crates/knx-net/tests/live_identify.rs) · [live_memory_readonly.rs](../crates/knx-net/tests/live_memory_readonly.rs) · [live_programming_mode.rs](../crates/knx-net/tests/live_programming_mode.rs)
- **Individual-address programming:** [live_individual_address_write.rs](../crates/knx-net/tests/live_individual_address_write.rs)
- **Live memory download and readback:** [live_memory_download.rs](../apps/knx-cli/tests/live_memory_download.rs)

</details>

## Command-line interface

The terminal gets the same rules, minus the mouse.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Address, serial and reset arguments](../apps/knx-cli/src/device_address.rs)** | Checks address-related commands default to plans and require correctly scoped confirmation before writes. | Rust / offline or loopback |
| **[Download, comparison and readiness commands](../apps/knx-cli/src/device_compare.rs)** | Checks download plans, read-only comparisons and readiness summaries expose guards and narrow evidence scopes. | Rust; explicit private cases¹ |
| **[Durable command activity admission](../apps/knx-cli/tests/cli_activity_history.rs)** | Checks CLI operations admit durable history before contact and refuse unsafe input aliases. | Rust / offline or loopback |
| **[Bus values, notation and scans](../apps/knx-cli/src/scan.rs)** | Checks bus commands use explicit DPT formats, project notation and validated scan arguments. | Rust / offline or loopback |
| **[Project and legacy imports](../apps/knx-cli/tests/cli_import.rs)** | Checks CLI imports report outcomes, reject invalid options and handle supported legacy credentials. | Rust; explicit private cases¹ |
| **[Package size and identity admission](../apps/knx-cli/tests/catalog_metadata.rs)** | Checks package commands enforce raw limits, supported extensions and explicit product identity queries. | Rust / offline or loopback |
| **[CSV, reports and project comparisons](../apps/knx-cli/tests/cli_documentation_export.rs)** | Checks CLI exports, CSV edits and diffs retain installation scope and meaningful failure codes. | Rust / offline or loopback |
| **[Selective import CLI workflow](../apps/knx-cli/tests/selective_import_cli.rs)** | Checks source inventory, confirmed selection and refusal behavior through the actual command-line binary. | Rust / offline or loopback |
| **[Version and monitor presentation](../apps/knx-cli/src/main.rs)** | Checks binary versions and monitor text preserve build identity, control evidence and project notation. | Rust / offline or loopback |

<details>
<summary>Source modules for this category</summary>

- **Address, serial and reset arguments:** [device_address.rs](../apps/knx-cli/src/device_address.rs) · [device_reset_address.rs](../apps/knx-cli/src/device_reset_address.rs) · [device_serial.rs](../apps/knx-cli/src/device_serial.rs) · [cli_address_programming_recovery.rs](../apps/knx-cli/tests/cli_address_programming_recovery.rs) · [cli_address_reset_recovery.rs](../apps/knx-cli/tests/cli_address_reset_recovery.rs)
- **Download, comparison and readiness commands:** [device_compare.rs](../apps/knx-cli/src/device_compare.rs) · [device_download.rs](../apps/knx-cli/src/device_download.rs) · [device_readiness.rs](../apps/knx-cli/src/device_readiness.rs) · [device_service_control.rs](../apps/knx-cli/src/device_service_control.rs)
- **Durable command activity admission:** [cli_activity_history.rs](../apps/knx-cli/tests/cli_activity_history.rs) · [cli_compare_activity.rs](../apps/knx-cli/tests/cli_compare_activity.rs) · [cli_read_activity.rs](../apps/knx-cli/tests/cli_read_activity.rs)
- **Bus values, notation and scans:** [scan.rs](../apps/knx-cli/src/scan.rs) · [cli_bus_address_style.rs](../apps/knx-cli/tests/cli_bus_address_style.rs) · [cli_bus_dpt.rs](../apps/knx-cli/tests/cli_bus_dpt.rs)
- **Project and legacy imports:** [cli_import.rs](../apps/knx-cli/tests/cli_import.rs) · [cli_legacy_import.rs](../apps/knx-cli/tests/cli_legacy_import.rs)
- **Package size and identity admission:** [catalog_metadata.rs](../apps/knx-cli/tests/catalog_metadata.rs) · [cli_large_package.rs](../apps/knx-cli/tests/cli_large_package.rs) · [cli_package_raw_admission.rs](../apps/knx-cli/tests/cli_package_raw_admission.rs) · [cli_product_extension_case.rs](../apps/knx-cli/tests/cli_product_extension_case.rs) · [cli_product_identity.rs](../apps/knx-cli/tests/cli_product_identity.rs)
- **CSV, reports and project comparisons:** [cli_documentation_export.rs](../apps/knx-cli/tests/cli_documentation_export.rs) · [cli_ga_csv_installations.rs](../apps/knx-cli/tests/cli_ga_csv_installations.rs) · [cli_group_address_csv.rs](../apps/knx-cli/tests/cli_group_address_csv.rs) · [cli_project_diff.rs](../apps/knx-cli/tests/cli_project_diff.rs)
- **Selective import CLI workflow:** [selective_import_cli.rs](../apps/knx-cli/tests/selective_import_cli.rs)
- **Version and monitor presentation:** [main.rs](../apps/knx-cli/src/main.rs) · [cli_version.rs](../apps/knx-cli/tests/cli_version.rs)

</details>

## Server services and HTTP contracts

HTTP success should mean something more than a cheerful status code.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Authentication and password hashing](../apps/knx-server/src/auth.rs)** | Checks session cookies, login guards and password hashing reject malformed or unauthenticated requests. | Rust / offline or loopback |
| **[TLS certificates and HTTPS listener](../apps/knx-server/src/tls.rs)** | Checks certificate persistence, redirect safety and HTTPS handling preserve validated connection and identity boundaries. | Rust / offline or loopback |
| **[Domain state and guarded replacement](../apps/knx-server/src/domain.rs)** | Checks project replacement, saving and concurrent state publication preserve authoritative snapshots and refusal boundaries. | Rust; explicit private cases¹ |
| **[Project opening, saving and selective import](../apps/knx-server/tests/http_project_routes.rs)** | Checks HTTP project workflows persist exact accepted changes and refuse stale selection confirmations. | Rust; explicit private cases¹ |
| **[Project history and naming routes](../apps/knx-server/tests/http_project_history.rs)** | Checks native recovery, named versions and guarded renames preserve exact state across revisions. | Rust / offline or loopback |
| **[Editing, batches and topology repair](../apps/knx-server/tests/command_dispatch.rs)** | Checks HTTP edits, atomic batches and placement repairs validate ownership and preserve undo behavior. | Rust / offline or loopback |
| **[Catalogue allocation and request replay](../apps/knx-server/src/catalog_requests.rs)** | Checks device allocation and retried requests remain atomic without duplicate creation or recycled identities. | Rust / offline or loopback |
| **[Catalogue, product installation and legacy routes](../apps/knx-server/tests/http_catalog_to_device.rs)** | Checks installed products, translated catalogue entries and legacy packages produce scoped reports and devices. | Rust; explicit private cases¹ |
| **[Device details and channel activation](../apps/knx-server/src/com_object_activation.rs)** | Checks device projections expose activation, channel ownership and product metadata without fabricating missing evidence. | Rust; explicit private cases¹ |
| **[Parameter access, languages and text budgets](../apps/knx-server/tests/http_external_text_budget.rs)** | Checks parameter fields disclose authority, translation fallback and bounded text costs before accepting edits. | Rust / offline or loopback |
| **[CSV, documentation and diff routes](../apps/knx-server/tests/csv_installation_scope.rs)** | Checks report previews, comparisons and CSV mutations preserve installation scope and explicit failure diagnostics. | Rust / offline or loopback |
| **[Files, uploads and path boundaries](../apps/knx-server/src/data_file.rs)** | Checks uploads and downloads enforce size, traversal and temporary-file cleanup without partial publication. | Rust / offline or loopback |
| **[Settings and conditional acknowledgments](../apps/knx-server/src/settings.rs)** | Checks settings persistence and conditional updates retain unrelated values and refuse stale snapshots. | Rust / offline or loopback |
| **[Achievements, logs and load progress](../apps/knx-server/src/achievements.rs)** | Checks achievements, bounded logs and progress snapshots preserve counts, unknown values and failure visibility. | Rust; explicit private cases¹ |
| **[Monitor, Flow and encoded bus writes](../apps/knx-server/src/bus.rs)** | Checks simulated monitor sessions retain generation context and send only explicitly admitted bus values. | Synthetic / loopback; some private cases |
| **[Discovery, scans and volatile activity](../apps/knx-server/src/bus_scan.rs)** | Checks simulated discovery and scans expose cancellation, exclusions and explicitly partial activity observations. | Synthetic / loopback |
| **[HTTP recovery gates and durable outcomes](../apps/knx-server/src/device_download_task_tests.rs)** | Checks device operations persist recovery evidence before simulated writes and retain uncertain cleanup outcomes. | Synthetic / loopback |
| **[Device downloads, comparisons and readiness](../apps/knx-server/tests/http_device_compare.rs)** | Checks planned images and simulated device outcomes disclose guards, scope and available readiness evidence. | Synthetic / simulated; some private cases |
| **[Shutdown, health and version behavior](../apps/knx-server/src/graceful_stop.rs)** | Checks shutdown drains or times out explicitly and version queries never start serving. | Rust / offline or loopback |

<details>
<summary>Source modules for this category</summary>

- **Authentication and password hashing:** [auth.rs](../apps/knx-server/src/auth.rs) · [auth_password.rs](../apps/knx-server/src/auth_password.rs) · [http_auth.rs](../apps/knx-server/tests/http_auth.rs)
- **TLS certificates and HTTPS listener:** [tls.rs](../apps/knx-server/src/tls.rs) · [tls_cert.rs](../apps/knx-server/src/tls_cert.rs) · [tls_listener.rs](../apps/knx-server/src/tls_listener.rs) · [https_listener.rs](../apps/knx-server/tests/https_listener.rs)
- **Domain state and guarded replacement:** [domain.rs](../apps/knx-server/src/domain.rs) · [routes.rs](../apps/knx-server/src/routes.rs)
- **Project opening, saving and selective import:** [http_project_routes.rs](../apps/knx-server/tests/http_project_routes.rs) · [http_selective_import.rs](../apps/knx-server/tests/http_selective_import.rs) · [open_reference_project.rs](../apps/knx-server/tests/open_reference_project.rs) · [save_load_roundtrip.rs](../apps/knx-server/tests/save_load_roundtrip.rs)
- **Project history and naming routes:** [http_project_history.rs](../apps/knx-server/tests/http_project_history.rs) · [http_rename.rs](../apps/knx-server/tests/http_rename.rs)
- **Editing, batches and topology repair:** [command_dispatch.rs](../apps/knx-server/tests/command_dispatch.rs) · [http_batch_routes.rs](../apps/knx-server/tests/http_batch_routes.rs) · [http_edit_routes.rs](../apps/knx-server/tests/http_edit_routes.rs) · [http_project_seed.rs](../apps/knx-server/tests/http_project_seed.rs) · [http_validation_errors.rs](../apps/knx-server/tests/http_validation_errors.rs) · [multi_installation_routes.rs](../apps/knx-server/tests/multi_installation_routes.rs) · [topology_repair_routes.rs](../apps/knx-server/tests/topology_repair_routes.rs)
- **Catalogue allocation and request replay:** [catalog_requests.rs](../apps/knx-server/src/catalog_requests.rs) · [catalog_allocation.rs](../apps/knx-server/tests/catalog_allocation.rs) · [catalog_request_replay.rs](../apps/knx-server/tests/catalog_request_replay.rs) · [http_device_wizard.rs](../apps/knx-server/tests/http_device_wizard.rs)
- **Catalogue, product installation and legacy routes:** [http_catalog_to_device.rs](../apps/knx-server/tests/http_catalog_to_device.rs) · [http_catalog_translation.rs](../apps/knx-server/tests/http_catalog_translation.rs) · [http_device_routes.rs](../apps/knx-server/tests/http_device_routes.rs) · [http_legacy_device.rs](../apps/knx-server/tests/http_legacy_device.rs) · [http_legacy_install.rs](../apps/knx-server/tests/http_legacy_install.rs) · [http_product_install.rs](../apps/knx-server/tests/http_product_install.rs)
- **Device details and channel activation:** [com_object_activation.rs](../apps/knx-server/src/com_object_activation.rs) · [com_object_activation_corpus.rs](../apps/knx-server/tests/com_object_activation_corpus.rs) · [coupler_address.rs](../apps/knx-server/tests/coupler_address.rs) · [device_detail.rs](../apps/knx-server/tests/device_detail.rs) · [http_com_object_language.rs](../apps/knx-server/tests/http_com_object_language.rs) · [http_device_detail.rs](../apps/knx-server/tests/http_device_detail.rs) · [http_device_product.rs](../apps/knx-server/tests/http_device_product.rs)
- **Parameter access, languages and text budgets:** [http_external_text_budget.rs](../apps/knx-server/tests/http_external_text_budget.rs) · [http_parameter_panel.rs](../apps/knx-server/tests/http_parameter_panel.rs) · [http_parameter_type_none.rs](../apps/knx-server/tests/http_parameter_type_none.rs) · [http_parameter_write_authority.rs](../apps/knx-server/tests/http_parameter_write_authority.rs) · [http_product_language.rs](../apps/knx-server/tests/http_product_language.rs)
- **CSV, documentation and diff routes:** [csv_installation_scope.rs](../apps/knx-server/tests/csv_installation_scope.rs) · [http_documentation_export.rs](../apps/knx-server/tests/http_documentation_export.rs) · [http_group_address_csv.rs](../apps/knx-server/tests/http_group_address_csv.rs) · [http_project_diff.rs](../apps/knx-server/tests/http_project_diff.rs)
- **Files, uploads and path boundaries:** [data_file.rs](../apps/knx-server/src/data_file.rs) · [fs_routes.rs](../apps/knx-server/src/fs_routes.rs) · [paths.rs](../apps/knx-server/src/paths.rs) · [http_fs_routes.rs](../apps/knx-server/tests/http_fs_routes.rs)
- **Settings and conditional acknowledgments:** [settings.rs](../apps/knx-server/src/settings.rs) · [http_settings.rs](../apps/knx-server/tests/http_settings.rs) · [http_settings_conditional.rs](../apps/knx-server/tests/http_settings_conditional.rs)
- **Achievements, logs and load progress:** [achievements.rs](../apps/knx-server/src/achievements.rs) · [load_progress.rs](../apps/knx-server/src/load_progress.rs) · [session_log.rs](../apps/knx-server/src/session_log.rs) · [http_achievements.rs](../apps/knx-server/tests/http_achievements.rs) · [http_load_progress.rs](../apps/knx-server/tests/http_load_progress.rs) · [http_log_route.rs](../apps/knx-server/tests/http_log_route.rs)
- **Monitor, Flow and encoded bus writes:** [bus.rs](../apps/knx-server/src/bus.rs) · [bus_routes.rs](../apps/knx-server/src/bus_routes.rs) · [flow.rs](../apps/knx-server/src/flow.rs) · [http_bus_flow.rs](../apps/knx-server/tests/http_bus_flow.rs) · [http_bus_monitor.rs](../apps/knx-server/tests/http_bus_monitor.rs) · [http_bus_write.rs](../apps/knx-server/tests/http_bus_write.rs)
- **Discovery, scans and volatile activity:** [bus_scan.rs](../apps/knx-server/src/bus_scan.rs) · [http_bus_activity.rs](../apps/knx-server/tests/http_bus_activity.rs) · [http_bus_discover.rs](../apps/knx-server/tests/http_bus_discover.rs) · [http_bus_scan.rs](../apps/knx-server/tests/http_bus_scan.rs)
- **HTTP recovery gates and durable outcomes:** [device_download_task_tests.rs](../apps/knx-server/src/device_download_task_tests.rs) · [http_activity_history.rs](../apps/knx-server/tests/http_activity_history.rs) · [http_address_programming.rs](../apps/knx-server/tests/http_address_programming.rs) · [http_serial_address.rs](../apps/knx-server/tests/http_serial_address.rs) · [http_service_control.rs](../apps/knx-server/tests/http_service_control.rs)
- **Device downloads, comparisons and readiness:** [http_device_compare.rs](../apps/knx-server/tests/http_device_compare.rs) · [http_device_download.rs](../apps/knx-server/tests/http_device_download.rs) · [http_device_readiness.rs](../apps/knx-server/tests/http_device_readiness.rs) · [project_download_request.rs](../apps/knx-server/tests/project_download_request.rs)
- **Shutdown, health and version behavior:** [graceful_stop.rs](../apps/knx-server/src/graceful_stop.rs) · [knx-server/src/lib.rs](../apps/knx-server/src/lib.rs) · [main.rs](../apps/knx-server/src/main.rs) · [bin_version.rs](../apps/knx-server/tests/bin_version.rs) · [healthz.rs](../apps/knx-server/tests/healthz.rs) · [http_version_route.rs](../apps/knx-server/tests/http_version_route.rs) · [signal_stop.rs](../apps/knx-server/tests/signal_stop.rs)

</details>

## Desktop shell and read-only MCP

A desktop shell and an AI client still need boundaries.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Native export and WebKit crash policy](../apps/knx-desktop/src-tauri/src/lib.rs)** | Checks dialog-selected exports preserve exact content and crash handling respects bounded recovery budgets. | Rust / offline or loopback |
| **[MCP arguments and tool envelopes](../apps/knx-mcp/src/args.rs)** | Checks read-only tool inputs, bounded paging and response envelopes reject unsupported or ambiguous requests. | Rust / offline or loopback |
| **[MCP stdio transport](../apps/knx-mcp/tests/stdio.rs)** | Checks real stdio clients enumerate read-only tools and invalid configuration keeps stdout clean. | Rust / offline or loopback |

<details>
<summary>Source modules for this category</summary>

- **Native export and WebKit crash policy:** [src-tauri/src/lib.rs](../apps/knx-desktop/src-tauri/src/lib.rs) · [web_process.rs](../apps/knx-desktop/src-tauri/src/web_process.rs)
- **MCP arguments and tool envelopes:** [args.rs](../apps/knx-mcp/src/args.rs) · [server.rs](../apps/knx-mcp/src/server.rs) · [src/tools/mod.rs](../apps/knx-mcp/src/tools/mod.rs) · [tools.rs](../apps/knx-mcp/tests/tools.rs)
- **MCP stdio transport:** [stdio.rs](../apps/knx-mcp/tests/stdio.rs)

</details>

## Frontend components and editing workflows

The mouse is optional. Correctness is not.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Application startup and authentication](../apps/knx-web/src/AboutDialog.test.tsx)** | Checks editor resume and login screens follow server authority without inventing an open project. | Vitest / synthetic DOM |
| **[Project creation and onboarding](../apps/knx-web/src/NewProjectDialog.test.tsx)** | Checks seeded defaults, guided startup and cancellation create only deliberately requested project structures. | Vitest / synthetic DOM |
| **[Explorer, structure and group addresses](../apps/knx-web/src/GroupAddressTable.test.tsx)** | Checks structured navigation, selection and address tables preserve canonical ownership and reported conflicts. | Vitest / synthetic DOM |
| **[Device catalogue and creation wizard](../apps/knx-web/src/CatalogBrowser.test.tsx)** | Checks catalogue choices and placement previews create exactly the requested devices or show refusals. | Vitest / synthetic DOM |
| **[Device workspace and communication editing](../apps/knx-web/src/ComObjectTable.test.tsx)** | Checks device tables retain drafts, selection and grouped channel evidence across view changes. | Vitest / synthetic DOM |
| **[Property inspection and device links](../apps/knx-web/src/DeviceLink.test.tsx)** | Checks inspector actions and device links use exact identities rather than guessing placement. | Vitest / synthetic DOM |
| **[Parameter fields and diagnostics](../apps/knx-web/src/ParameterPanel.test.tsx)** | Checks parameter fields preserve diagnostic records, stale values and guarded refresh ownership. | Vitest / synthetic DOM |
| **[Inline name editing](../apps/knx-web/src/rename.entry.test.tsx)** | Checks exact Unicode drafts, pending commits and stale contexts cannot silently overwrite names. | Vitest / synthetic DOM |
| **[CSV operations and typed dragging](../apps/knx-web/src/GroupAddressCsvButtons.test.tsx)** | Checks CSV cancellation and typed address dragging never trigger unintended edits or exports. | Vitest / synthetic DOM |
| **[Report previews and comparison panels](../apps/knx-web/src/DocumentationDialog.test.tsx)** | Checks report selection and comparison filters preserve every entry and render previews safely. | Vitest / synthetic DOM |
| **[Native history and selective import dialogs](../apps/knx-web/src/ProjectHistoryDialog.test.tsx)** | Checks restored versions and imported selections require bound previews, consent and current project revisions. | Vitest / synthetic DOM |
| **[Protected projects and legacy installation](../apps/knx-web/src/LegacyInstallReport.test.tsx)** | Checks masked password prompts and legacy installation controls disclose failures without unwanted credential reuse. | Vitest / synthetic DOM |
| **[File pickers and export delivery](../apps/knx-web/src/FsPicker.test.tsx)** | Checks file cancellation, local downloads and native save dialogs report actual outcomes without invented success. | Vitest / synthetic DOM |
| **[Address programming and consent](../apps/knx-web/src/AddressProgrammingPanel.test.tsx)** | Checks unavailable programming stays blocked and stage-specific consent never substitutes for device recovery. | Vitest / synthetic DOM |
| **[Device downloads and inspection](../apps/knx-web/src/DeviceDownloadPanel.test.tsx)** | Checks device actions stay offline until explicit requests and preserve scoped plan disclosures. | Vitest / synthetic DOM |
| **[Service-control opt-in and write phrases](../apps/knx-web/src/ServiceControlDebugSetting.test.tsx)** | Checks saved opt-in, read evidence and fresh target confirmation precede service-control write requests. | Vitest / synthetic DOM |
| **[Line scan inputs and exclusions](../apps/knx-web/src/LineScanExclusionsEditor.test.tsx)** | Checks scan preferences and exclusion edits preserve invalid legacy entries without silently repairing them. | Vitest / synthetic DOM |

<details>
<summary>Source modules for this category</summary>

- **Application startup and authentication:** [AboutDialog.test.tsx](../apps/knx-web/src/AboutDialog.test.tsx) · [App.test.tsx](../apps/knx-web/src/App.test.tsx) · [AuthGate.test.tsx](../apps/knx-web/src/AuthGate.test.tsx)
- **Project creation and onboarding:** [NewProjectDialog.test.tsx](../apps/knx-web/src/NewProjectDialog.test.tsx) · [OnboardingGuide.test.tsx](../apps/knx-web/src/OnboardingGuide.test.tsx) · [onboardingGuide.test.ts](../apps/knx-web/src/onboardingGuide.test.ts) · [projectSeed.test.ts](../apps/knx-web/src/projectSeed.test.ts) · [useOnboardingGuide.test.tsx](../apps/knx-web/src/useOnboardingGuide.test.tsx)
- **Explorer, structure and group addresses:** [GroupAddressTable.test.tsx](../apps/knx-web/src/GroupAddressTable.test.tsx) · [ProjectExplorer.test.tsx](../apps/knx-web/src/ProjectExplorer.test.tsx) · [StructureWorkspace.test.tsx](../apps/knx-web/src/StructureWorkspace.test.tsx) · [dashboardStats.test.ts](../apps/knx-web/src/dashboardStats.test.ts) · [treeUtils.test.ts](../apps/knx-web/src/treeUtils.test.ts)
- **Device catalogue and creation wizard:** [CatalogBrowser.test.tsx](../apps/knx-web/src/CatalogBrowser.test.tsx) · [DeviceWizard.test.tsx](../apps/knx-web/src/DeviceWizard.test.tsx) · [deviceWizardPlacement.test.ts](../apps/knx-web/src/deviceWizardPlacement.test.ts)
- **Device workspace and communication editing:** [ComObjectTable.test.tsx](../apps/knx-web/src/ComObjectTable.test.tsx) · [DeviceWorkspace.test.tsx](../apps/knx-web/src/DeviceWorkspace.test.tsx) · [DevicesWorkspace.test.tsx](../apps/knx-web/src/DevicesWorkspace.test.tsx) · [comObjectView.test.ts](../apps/knx-web/src/comObjectView.test.ts) · [deviceList.test.ts](../apps/knx-web/src/deviceList.test.ts)
- **Property inspection and device links:** [DeviceLink.test.tsx](../apps/knx-web/src/DeviceLink.test.tsx) · [Inspector.test.tsx](../apps/knx-web/src/Inspector.test.tsx)
- **Parameter fields and diagnostics:** [ParameterPanel.test.tsx](../apps/knx-web/src/ParameterPanel.test.tsx) · [parameterPresentation.test.ts](../apps/knx-web/src/parameterPresentation.test.ts)
- **Inline name editing:** [rename.entry.test.tsx](../apps/knx-web/src/rename.entry.test.tsx) · [rename.field.test.tsx](../apps/knx-web/src/rename.field.test.tsx) · [rename.inspector.test.tsx](../apps/knx-web/src/rename.inspector.test.tsx) · [rename.test.ts](../apps/knx-web/src/rename.test.ts)
- **CSV operations and typed dragging:** [GroupAddressCsvButtons.test.tsx](../apps/knx-web/src/GroupAddressCsvButtons.test.tsx) · [groupAddressDrag.test.ts](../apps/knx-web/src/groupAddressDrag.test.ts) · [groupStructurePresets.test.ts](../apps/knx-web/src/groupStructurePresets.test.ts)
- **Report previews and comparison panels:** [DocumentationDialog.test.tsx](../apps/knx-web/src/DocumentationDialog.test.tsx) · [DocumentationExportButton.test.tsx](../apps/knx-web/src/DocumentationExportButton.test.tsx) · [ProjectDiffDetails.test.tsx](../apps/knx-web/src/ProjectDiffDetails.test.tsx) · [ProjectDiffPanel.test.tsx](../apps/knx-web/src/ProjectDiffPanel.test.tsx) · [documentationOptions.test.ts](../apps/knx-web/src/documentationOptions.test.ts) · [projectDiffView.test.ts](../apps/knx-web/src/projectDiffView.test.ts) · [virtualWindow.test.ts](../apps/knx-web/src/virtualWindow.test.ts)
- **Native history and selective import dialogs:** [ProjectHistoryDialog.test.tsx](../apps/knx-web/src/ProjectHistoryDialog.test.tsx) · [SelectiveImportButton.test.tsx](../apps/knx-web/src/SelectiveImportButton.test.tsx) · [projectHistory.test.ts](../apps/knx-web/src/projectHistory.test.ts)
- **Protected projects and legacy installation:** [LegacyInstallReport.test.tsx](../apps/knx-web/src/LegacyInstallReport.test.tsx) · [ProductInstallControl.test.tsx](../apps/knx-web/src/ProductInstallControl.test.tsx) · [ProjectPasswordDialog.test.tsx](../apps/knx-web/src/ProjectPasswordDialog.test.tsx) · [legacyInstall.test.ts](../apps/knx-web/src/legacyInstall.test.ts) · [projectPassword.test.ts](../apps/knx-web/src/projectPassword.test.ts)
- **File pickers and export delivery:** [FsPicker.test.tsx](../apps/knx-web/src/FsPicker.test.tsx) · [busMonitorCaptureDelivery.test.ts](../apps/knx-web/src/busMonitorCaptureDelivery.test.ts) · [filePicker.test.ts](../apps/knx-web/src/filePicker.test.ts) · [sessionLogExportDelivery.test.ts](../apps/knx-web/src/sessionLogExportDelivery.test.ts)
- **Address programming and consent:** [AddressProgrammingPanel.test.tsx](../apps/knx-web/src/AddressProgrammingPanel.test.tsx) · [programmingConsent.test.ts](../apps/knx-web/src/programmingConsent.test.ts) · [useProgrammingConsent.test.tsx](../apps/knx-web/src/useProgrammingConsent.test.tsx)
- **Device downloads and inspection:** [DeviceDownloadPanel.test.tsx](../apps/knx-web/src/DeviceDownloadPanel.test.tsx) · [DeviceInspectionPanel.test.tsx](../apps/knx-web/src/DeviceInspectionPanel.test.tsx)
- **Service-control opt-in and write phrases:** [ServiceControlDebugSetting.test.tsx](../apps/knx-web/src/ServiceControlDebugSetting.test.tsx) · [ServiceControlPanel.test.tsx](../apps/knx-web/src/ServiceControlPanel.test.tsx)
- **Line scan inputs and exclusions:** [LineScanExclusionsEditor.test.tsx](../apps/knx-web/src/LineScanExclusionsEditor.test.tsx) · [LineScanPanel.test.tsx](../apps/knx-web/src/LineScanPanel.test.tsx) · [lineScanExclusions.test.ts](../apps/knx-web/src/lineScanExclusions.test.ts)

</details>

## Frontend accessibility, preferences and diagnostics

The interface has preferences; the data has rights.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Dialogs and keyboard focus](../apps/knx-web/src/CommandPalette.test.tsx)** | Checks shared overlays keep backgrounds inert and keyboard selection visible without stealing focus. | Vitest / synthetic DOM |
| **[Workbench panes and contextual help](../apps/knx-web/src/HelpPanel.test.tsx)** | Checks pane resizing and help bubbles preserve keyboard access, bounds and contextual descriptions. | Vitest / synthetic DOM |
| **[Search ranking and command availability](../apps/knx-web/src/commandRegistry.test.ts)** | Checks searches rank meaningful matches and command registries expose only contextually available actions. | Vitest / synthetic DOM |
| **[Diagnostic companion isolation](../apps/knx-web/src/BusDiagnosticsPanel.test.tsx)** | Checks separate diagnostics navigation imports no editing surfaces and retains explicit editor return paths. | Vitest / synthetic DOM |
| **[Logs and actual progress counts](../apps/knx-web/src/LoadProgressBanner.test.tsx)** | Checks log filtering and progress displays preserve raw messages and distinguish measurements from guesses. | Vitest / synthetic DOM |
| **[Durable and volatile activity views](../apps/knx-web/src/BusActivityHistory.test.tsx)** | Checks activity views distinguish history from live snapshots, interruptions and unavailable evidence. | Vitest / synthetic DOM |
| **[Monitor controls, captures and statistics](../apps/knx-web/src/BusMonitorPanel.test.tsx)** | Checks monitor details and retained captures preserve control evidence, loss metadata and bounded statistics. | Vitest / synthetic DOM |
| **[Bus values and gateway preferences](../apps/knx-web/src/BusComposeForm.test.tsx)** | Checks explicit input grammars and validated endpoints prevent ambiguous or malformed bus requests. | Vitest / synthetic DOM |
| **[API response contracts](../apps/knx-web/src/api.test.ts)** | Checks frontend API responses validate catalogue shapes and preserve authoritative operation arguments. | Vitest / synthetic DOM |
| **[Settings acknowledgment and persistence](../apps/knx-web/src/SettingsPanel.test.tsx)** | Checks settings adopt validated server acknowledgments while retaining failed local patches and unrelated preferences. | Vitest / synthetic DOM |
| **[Autosave and user preferences](../apps/knx-web/src/appearance.test.tsx)** | Checks autosave timing and product language preferences preserve explicit choices and cancellation behavior. | Vitest / synthetic DOM |
| **[Language packs and fallback](../apps/knx-web/src/bundledLanguagePacks.test.ts)** | Checks admitted language packs preserve placeholders and fall back honestly for missing translations. | Vitest / synthetic DOM |
| **[Achievements and novelty shortcuts](../apps/knx-web/src/AchievementSettings.test.tsx)** | Checks achievement rules count actual events, persist unlocks and honor tracking opt-out. | Vitest / synthetic DOM |
| **[Toasts, timings and issue handoff](../apps/knx-web/src/Toast.test.tsx)** | Checks toast lifetime, error disclosures and bounded issue text remain visible and predictable. | Vitest / synthetic DOM |
| **[Theme packs and untrusted palettes](../apps/knx-web/src/themePack.test.ts)** | Checks imported palettes reject unsafe values while preserving author data and unrelated settings. | Vitest / synthetic DOM |
| **[Theme lifecycle and cold restart](../apps/knx-web/src/ThemePackManager.integration.test.tsx)** | Checks installed themes survive acknowledged persistence, replacements and cold restart without unwanted writes. | Vitest / synthetic DOM |
| **[Motion, CRT and LCARS presentation](../apps/knx-web/src/crtInteractions.test.ts)** | Checks motion preferences stop animations while presentation layers remain inert and bounded. | Vitest / synthetic DOM |

<details>
<summary>Source modules for this category</summary>

- **Dialogs and keyboard focus:** [CommandPalette.test.tsx](../apps/knx-web/src/CommandPalette.test.tsx) · [Overlay.test.tsx](../apps/knx-web/src/Overlay.test.tsx) · [Search.test.tsx](../apps/knx-web/src/Search.test.tsx) · [overlayShell.test.ts](../apps/knx-web/src/overlayShell.test.ts) · [useActiveOptionScroll.test.tsx](../apps/knx-web/src/useActiveOptionScroll.test.tsx)
- **Workbench panes and contextual help:** [HelpPanel.test.tsx](../apps/knx-web/src/HelpPanel.test.tsx) · [HelpTip.test.tsx](../apps/knx-web/src/HelpTip.test.tsx) · [PaneSplitter.test.tsx](../apps/knx-web/src/PaneSplitter.test.tsx) · [Workbench.test.tsx](../apps/knx-web/src/Workbench.test.tsx) · [help.test.ts](../apps/knx-web/src/help.test.ts)
- **Search ranking and command availability:** [commandRegistry.test.ts](../apps/knx-web/src/commandRegistry.test.ts) · [searchMatch.test.ts](../apps/knx-web/src/searchMatch.test.ts)
- **Diagnostic companion isolation:** [BusDiagnosticsPanel.test.tsx](../apps/knx-web/src/BusDiagnosticsPanel.test.tsx) · [DiagnosticsCompanion.test.tsx](../apps/knx-web/src/DiagnosticsCompanion.test.tsx) · [diagnosticShell.test.ts](../apps/knx-web/src/diagnosticShell.test.ts) · [diagnosticsWindow.test.ts](../apps/knx-web/src/diagnosticsWindow.test.ts)
- **Logs and actual progress counts:** [LoadProgressBanner.test.tsx](../apps/knx-web/src/LoadProgressBanner.test.tsx) · [LogPanel.test.tsx](../apps/knx-web/src/LogPanel.test.tsx) · [loadFlavour.test.ts](../apps/knx-web/src/loadFlavour.test.ts) · [loadProgress.test.ts](../apps/knx-web/src/loadProgress.test.ts) · [sessionLogExport.test.ts](../apps/knx-web/src/sessionLogExport.test.ts)
- **Durable and volatile activity views:** [BusActivityHistory.test.tsx](../apps/knx-web/src/BusActivityHistory.test.tsx) · [BusActivityLive.test.tsx](../apps/knx-web/src/BusActivityLive.test.tsx) · [activityHistory.test.ts](../apps/knx-web/src/activityHistory.test.ts)
- **Monitor controls, captures and statistics:** [BusMonitorPanel.test.tsx](../apps/knx-web/src/BusMonitorPanel.test.tsx) · [busContext.test.ts](../apps/knx-web/src/busContext.test.ts) · [busMonitorCapture.test.ts](../apps/knx-web/src/busMonitorCapture.test.ts) · [busMonitorStatistics.test.ts](../apps/knx-web/src/busMonitorStatistics.test.ts)
- **Bus values and gateway preferences:** [BusComposeForm.test.tsx](../apps/knx-web/src/BusComposeForm.test.tsx) · [busDiscovery.test.ts](../apps/knx-web/src/busDiscovery.test.ts) · [gaNotation.test.tsx](../apps/knx-web/src/gaNotation.test.tsx) · [gatewayEndpoint.test.ts](../apps/knx-web/src/gatewayEndpoint.test.ts) · [gatewayPreference.test.tsx](../apps/knx-web/src/gatewayPreference.test.tsx)
- **API response contracts:** [api.test.ts](../apps/knx-web/src/api.test.ts)
- **Settings acknowledgment and persistence:** [SettingsPanel.test.tsx](../apps/knx-web/src/SettingsPanel.test.tsx) · [settingsAcknowledgement.test.ts](../apps/knx-web/src/settingsAcknowledgement.test.ts) · [settingsDiagnostic.test.ts](../apps/knx-web/src/settingsDiagnostic.test.ts) · [settingsStore.test.ts](../apps/knx-web/src/settingsStore.test.ts)
- **Autosave and user preferences:** [appearance.test.tsx](../apps/knx-web/src/appearance.test.tsx) · [autosaveSettings.test.ts](../apps/knx-web/src/autosaveSettings.test.ts) · [productLanguage.test.tsx](../apps/knx-web/src/productLanguage.test.tsx) · [useAutosave.test.tsx](../apps/knx-web/src/useAutosave.test.tsx)
- **Language packs and fallback:** [bundledLanguagePacks.test.ts](../apps/knx-web/src/bundledLanguagePacks.test.ts) · [i18n.duForm.test.ts](../apps/knx-web/src/i18n.duForm.test.ts) · [i18n.fallback.test.ts](../apps/knx-web/src/i18n.fallback.test.ts) · [i18n.test.tsx](../apps/knx-web/src/i18n.test.tsx) · [languagePack.test.tsx](../apps/knx-web/src/languagePack.test.tsx) · [languageSelfName.test.ts](../apps/knx-web/src/languageSelfName.test.ts) · [uiLanguage.test.tsx](../apps/knx-web/src/uiLanguage.test.tsx)
- **Achievements and novelty shortcuts:** [AchievementSettings.test.tsx](../apps/knx-web/src/AchievementSettings.test.tsx) · [AchievementsDialog.test.tsx](../apps/knx-web/src/AchievementsDialog.test.tsx) · [achievementCatalog.test.ts](../apps/knx-web/src/achievementCatalog.test.ts) · [achievementObservation.test.ts](../apps/knx-web/src/achievementObservation.test.ts) · [achievementPreference.test.tsx](../apps/knx-web/src/achievementPreference.test.tsx) · [achievementRules.test.ts](../apps/knx-web/src/achievementRules.test.ts) · [achievementTracker.test.ts](../apps/knx-web/src/achievementTracker.test.ts) · [konami.test.ts](../apps/knx-web/src/konami.test.ts) · [useAchievements.test.tsx](../apps/knx-web/src/useAchievements.test.tsx)
- **Toasts, timings and issue handoff:** [Toast.test.tsx](../apps/knx-web/src/Toast.test.tsx) · [githubIssue.test.ts](../apps/knx-web/src/githubIssue.test.ts) · [toast.test.ts](../apps/knx-web/src/toast.test.ts) · [useToasts.test.tsx](../apps/knx-web/src/useToasts.test.tsx)
- **Theme packs and untrusted palettes:** [themePack.test.ts](../apps/knx-web/src/themePack.test.ts) · [themePackAgreement.test.ts](../apps/knx-web/src/themePackAgreement.test.ts) · [themePackDom.test.ts](../apps/knx-web/src/themePackDom.test.ts) · [themePackFiles.test.ts](../apps/knx-web/src/themePackFiles.test.ts) · [themePackStorage.test.ts](../apps/knx-web/src/themePackStorage.test.ts) · [themePackStore.test.ts](../apps/knx-web/src/themePackStore.test.ts)
- **Theme lifecycle and cold restart:** [ThemePackManager.integration.test.tsx](../apps/knx-web/src/ThemePackManager.integration.test.tsx) · [retroGreenTheme.test.ts](../apps/knx-web/src/retroGreenTheme.test.ts) · [theme.test.ts](../apps/knx-web/src/theme.test.ts) · [themePackRoundtrip.test.ts](../apps/knx-web/src/themePackRoundtrip.test.ts) · [themePackRuntime.test.tsx](../apps/knx-web/src/themePackRuntime.test.tsx) · [themeSettings.test.tsx](../apps/knx-web/src/themeSettings.test.tsx) · [themeTokens.test.ts](../apps/knx-web/src/themeTokens.test.ts)
- **Motion, CRT and LCARS presentation:** [crtInteractions.test.ts](../apps/knx-web/src/crtInteractions.test.ts) · [crtStyles.test.ts](../apps/knx-web/src/crtStyles.test.ts) · [lcarsPresentation.test.ts](../apps/knx-web/src/lcarsPresentation.test.ts) · [motion.test.ts](../apps/knx-web/src/motion.test.ts) · [motionGuard.test.ts](../apps/knx-web/src/motionGuard.test.ts)

</details>

## Telegram Flow models and browser views

The graph may dance. The evidence stays put.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Flow membership and wire evidence](../apps/knx-web/src/flowIdentity.test.ts)** | Checks graph members follow exact project identities and unknown observations remain explicitly unresolved. | Vitest / synthetic DOM |
| **[Flow feed and value expiry](../apps/knx-web/src/flowChannel.test.ts)** | Checks generation-bound feeds expire values correctly and ignore delayed snapshots from replaced sessions. | Vitest / synthetic DOM |
| **[Graph layout and motion lifecycle](../apps/knx-web/src/flowAnimator.test.ts)** | Checks deterministic layouts avoid crowded labels and stop animation work when motion is disabled. | Vitest / synthetic DOM |
| **[Dedicated Flow window and controls](../apps/knx-web/src/TelegramFlowView.test.tsx)** | Checks dedicated windows preserve graph context and expose selection, fit and source-loss boundaries. | Vitest / synthetic DOM |
| **[Synthetic Flow study models](../apps/knx-web/e2e/flow-study/layout.test.ts)** | Checks synthetic graph workloads stay deterministic while frozen layout preserves live value updates. | Vitest / synthetic DOM |

<details>
<summary>Source modules for this category</summary>

- **Flow membership and wire evidence:** [flowIdentity.test.ts](../apps/knx-web/src/flowIdentity.test.ts) · [flowModel.test.ts](../apps/knx-web/src/flowModel.test.ts) · [flowNavigation.test.ts](../apps/knx-web/src/flowNavigation.test.ts) · [flowWire.test.ts](../apps/knx-web/src/flowWire.test.ts)
- **Flow feed and value expiry:** [flowChannel.test.ts](../apps/knx-web/src/flowChannel.test.ts) · [flowFeed.test.tsx](../apps/knx-web/src/flowFeed.test.tsx)
- **Graph layout and motion lifecycle:** [flowAnimator.test.ts](../apps/knx-web/src/flowAnimator.test.ts) · [flowDynamics.test.ts](../apps/knx-web/src/flowDynamics.test.ts) · [flowLayout.test.ts](../apps/knx-web/src/flowLayout.test.ts) · [flowMotion.test.tsx](../apps/knx-web/src/flowMotion.test.tsx) · [flowPresentation.test.ts](../apps/knx-web/src/flowPresentation.test.ts)
- **Dedicated Flow window and controls:** [TelegramFlowView.test.tsx](../apps/knx-web/src/TelegramFlowView.test.tsx) · [flowWindow.test.ts](../apps/knx-web/src/flowWindow.test.ts)
- **Synthetic Flow study models:** [layout.test.ts](../apps/knx-web/e2e/flow-study/layout.test.ts) · [model.test.ts](../apps/knx-web/e2e/flow-study/model.test.ts) · [synthetic.test.ts](../apps/knx-web/e2e/flow-study/synthetic.test.ts)

</details>

## Browser workflows and layout contracts

Browsers click tirelessly, but intercepted answers are still intercepted answers.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[New projects, onboarding and protected imports](../apps/knx-web/e2e/legacy-install.e2e.ts)** | Checks browser project creation and password prompts remain usable across languages and narrow viewports. | Browser — intercepted API |
| **[Catalogue allocation, retry and placement](../apps/knx-web/e2e/catalog-allocation.e2e.ts)** | Checks browser catalogue requests retain reviewed quantities, placement and replay identities after failures. | Browser — intercepted API |
| **[Device navigation and editor layout](../apps/knx-web/e2e/device-editor-layout.e2e.ts)** | Checks device list state and editor content survive navigation without unintended hardware requests. | Browser — intercepted API |
| **[Communication-object table and address linking](../apps/knx-web/e2e/communication-objects.e2e.ts)** | Checks table filtering, sorting and drag linking preserve drafts, keyboard access and server refusals. | Browser — intercepted API |
| **[Installation ownership and placement repair](../apps/knx-web/e2e/installations.e2e.ts)** | Checks later-installation edits and explicit placement repairs keep displayed entities under their chosen owners. | Browser — intercepted API |
| **[History versions and restore consent](../apps/knx-web/e2e/project-history.e2e.ts)** | Checks browser history separates native versions from session-only editing and requires explicit restore confirmation. | Browser — intercepted API |
| **[Programming refusal and device-operation scope](../apps/knx-web/e2e/address-unavailable.e2e.ts)** | Checks recovery refusals, simulated inspection and partial download choices remain visible before consent. | Browser — intercepted API |
| **[Live and historical activity presentation](../apps/knx-web/e2e/activity-history.e2e.ts)** | Checks browser activity separates running operations, historical outcomes and unavailable storage without false success. | Browser — intercepted API |
| **[Monitor control evidence](../apps/knx-web/e2e/monitor-control.e2e.ts)** | Checks monitor control fields and context status render without contacting a real KNX bus. | Browser — intercepted API |
| **[Keyboard, panes and virtualized comparisons](../apps/knx-web/e2e/diff-virtual.e2e.ts)** | Checks keyboard navigation, focus isolation and scrolling reach content while respecting viewport bounds. | Browser — intercepted API |
| **[Companion return navigation](../apps/knx-web/e2e/companion-return.e2e.ts)** | Checks standalone diagnostic and Flow windows provide explicit return paths to the editor. | Browser — intercepted API |
| **[Themes, malicious palettes and toast layout](../apps/knx-web/e2e/achievements.e2e.ts)** | Checks browser themes reject hostile palettes and keep visual states and notifications readable. | Browser — intercepted API |
| **[Telegram Flow behavior and navigation](../apps/knx-web/e2e/flow-navigation.e2e.ts)** | Checks Flow layouts, value expiry and navigation preserve exact context without sending bus writes. | Browser — intercepted API |
| **[Synthetic Flow study browser](../apps/knx-web/e2e/flow-study.e2e.ts)** | Checks an isolated synthetic map supports keyboard interaction, freezing and motion-free value updates. | Browser — synthetic study |
| **[Native naming and reopen acceptance](../apps/knx-web/e2e/rename.native.ts)** | Checks built browser and server naming survive conflicts and native reopen on synthetic data. | Explicit browser + built server |

<details>
<summary>Source modules for this category</summary>

- **New projects, onboarding and protected imports:** [legacy-install.e2e.ts](../apps/knx-web/e2e/legacy-install.e2e.ts) · [new-project.e2e.ts](../apps/knx-web/e2e/new-project.e2e.ts) · [onboarding-guide.e2e.ts](../apps/knx-web/e2e/onboarding-guide.e2e.ts) · [project-password.e2e.ts](../apps/knx-web/e2e/project-password.e2e.ts)
- **Catalogue allocation, retry and placement:** [catalog-allocation.e2e.ts](../apps/knx-web/e2e/catalog-allocation.e2e.ts) · [catalog-retry.e2e.ts](../apps/knx-web/e2e/catalog-retry.e2e.ts) · [coupler-address.e2e.ts](../apps/knx-web/e2e/coupler-address.e2e.ts) · [device-wizard.e2e.ts](../apps/knx-web/e2e/device-wizard.e2e.ts)
- **Device navigation and editor layout:** [device-editor-layout.e2e.ts](../apps/knx-web/e2e/device-editor-layout.e2e.ts) · [devices-navigation.e2e.ts](../apps/knx-web/e2e/devices-navigation.e2e.ts)
- **Communication-object table and address linking:** [communication-objects.e2e.ts](../apps/knx-web/e2e/communication-objects.e2e.ts) · [ga-type-detail.e2e.ts](../apps/knx-web/e2e/ga-type-detail.e2e.ts) · [group-address-drag.e2e.ts](../apps/knx-web/e2e/group-address-drag.e2e.ts)
- **Installation ownership and placement repair:** [installations.e2e.ts](../apps/knx-web/e2e/installations.e2e.ts) · [repair.e2e.ts](../apps/knx-web/e2e/repair.e2e.ts) · [site.e2e.ts](../apps/knx-web/e2e/site.e2e.ts)
- **History versions and restore consent:** [project-history.e2e.ts](../apps/knx-web/e2e/project-history.e2e.ts)
- **Programming refusal and device-operation scope:** [address-unavailable.e2e.ts](../apps/knx-web/e2e/address-unavailable.e2e.ts) · [device-checks.e2e.ts](../apps/knx-web/e2e/device-checks.e2e.ts) · [device-download-scope.e2e.ts](../apps/knx-web/e2e/device-download-scope.e2e.ts) · [service-control.e2e.ts](../apps/knx-web/e2e/service-control.e2e.ts)
- **Live and historical activity presentation:** [activity-history.e2e.ts](../apps/knx-web/e2e/activity-history.e2e.ts) · [activity-live.e2e.ts](../apps/knx-web/e2e/activity-live.e2e.ts)
- **Monitor control evidence:** [monitor-control.e2e.ts](../apps/knx-web/e2e/monitor-control.e2e.ts)
- **Keyboard, panes and virtualized comparisons:** [diff-virtual.e2e.ts](../apps/knx-web/e2e/diff-virtual.e2e.ts) · [keyboard-contract.e2e.ts](../apps/knx-web/e2e/keyboard-contract.e2e.ts) · [workbench-splitters.e2e.ts](../apps/knx-web/e2e/workbench-splitters.e2e.ts)
- **Companion return navigation:** [companion-return.e2e.ts](../apps/knx-web/e2e/companion-return.e2e.ts)
- **Themes, malicious palettes and toast layout:** [achievements.e2e.ts](../apps/knx-web/e2e/achievements.e2e.ts) · [theme-manager.e2e.ts](../apps/knx-web/e2e/theme-manager.e2e.ts) · [theme-pack.e2e.ts](../apps/knx-web/e2e/theme-pack.e2e.ts) · [theme-state.e2e.ts](../apps/knx-web/e2e/theme-state.e2e.ts) · [toast.e2e.ts](../apps/knx-web/e2e/toast.e2e.ts)
- **Telegram Flow behavior and navigation:** [flow-navigation.e2e.ts](../apps/knx-web/e2e/flow-navigation.e2e.ts) · [flow-ux.e2e.ts](../apps/knx-web/e2e/flow-ux.e2e.ts) · [telegram-flow-hub.e2e.ts](../apps/knx-web/e2e/telegram-flow-hub.e2e.ts) · [telegram-flow-motion.e2e.ts](../apps/knx-web/e2e/telegram-flow-motion.e2e.ts) · [telegram-flow.e2e.ts](../apps/knx-web/e2e/telegram-flow.e2e.ts)
- **Synthetic Flow study browser:** [flow-study.e2e.ts](../apps/knx-web/e2e/flow-study.e2e.ts)
- **Native naming and reopen acceptance:** [rename.native.ts](../apps/knx-web/e2e/rename.native.ts)

</details>

## Performance studies and capture checks

Measurements need a workload, not a stopwatch-shaped opinion.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Synthetic project performance baseline](../crates/knx-app/tests/perf_baseline.rs)** | Measures synthetic project persistence, projection and search, with separately reported optional private import timing. | Explicit benchmark; optional private import |
| **[Large manufacturer-member memory bound](../crates/knx-productdb/tests/large_member_memory.rs)** | Measures private large-member ingestion against documented memory limits under an explicitly selected corpus. | Rust; explicit private cases¹ |
| **[Flow load and layout studies](../apps/knx-web/e2e/flow-load.load.ts)** | Measures synthetic Flow workloads, graph growth and rendering behavior through separate study configurations. | Explicit browser study / benchmark |
| **[Manual and README capture checks](../apps/knx-web/e2e/manual-screenshots.shots.ts)** | Exercises fictional application workflows while capturing documentation images, not certifying hardware or ETS compatibility. | Explicit capture configs; built server or fixtures |
| **[CRT and parameter-workspace browser probes](../design/verify-crt-interactions.mjs)** | Checks separate browser probes validate CRT interactions, reference styling and synthetic parameter workspace behavior. | Explicit browser probes; synthetic fixtures |

<details>
<summary>Source modules for this category</summary>

- **Synthetic project performance baseline:** [perf_baseline.rs](../crates/knx-app/tests/perf_baseline.rs)
- **Large manufacturer-member memory bound:** [large_member_memory.rs](../crates/knx-productdb/tests/large_member_memory.rs)
- **Flow load and layout studies:** [flow-load.load.ts](../apps/knx-web/e2e/flow-load.load.ts) · [flow-study.study.ts](../apps/knx-web/e2e/flow-study.study.ts)
- **Manual and README capture checks:** [manual-screenshots.shots.ts](../apps/knx-web/e2e/manual-screenshots.shots.ts) · [readme-flow.shots.ts](../apps/knx-web/e2e/readme-flow.shots.ts) · [readme-hero.shots.ts](../apps/knx-web/e2e/readme-hero.shots.ts)
- **CRT and parameter-workspace browser probes:** [verify-crt-interactions.mjs](../design/verify-crt-interactions.mjs) · [verify-crt-reference.mjs](../design/verify-crt-reference.mjs) · [verify-browser.js](../docs/parameter-workspace/verify-browser.js)

</details>

## Repository, build and documentation checks

These checks guard the workshop, not the bus.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Release build provenance](../crates/knx-build-stamp/src/lib.rs)** | Checks build stamps use the actual clean Git tree and reject foreign or dirty provenance. | Rust / offline or loopback |
| **[Dependency layering](../xtask/src/layering.rs)** | Checks direct and transitive dependencies obey architectural boundaries without hanging on graph cycles. | Rust / offline or loopback |
| **[Source headers and status ledger](../xtask/src/headers.rs)** | Checks source headers and status records satisfy repository conventions without accepting missing inventories. | Rust / offline or loopback |
| **[Documentation links and heading anchors](../tools/tests/test_check_documentation.py)** | Checks Markdown links, heading slugs and chapter navigation distinguish actual references from code examples. | Local runners / synthetic |
| **[Nonempty gate and fixture scope](../apps/knx-web/src/fixtureServer.test.ts)** | Checks repository gates inspect the requested roots and test helpers refuse absent fixture directories. | Rust + Vitest / synthetic |
| **[AppImage artifact and launcher policy](../tools/tests/test_appimage_launcher.py)** | Checks packaged AppImages and display-backend selection satisfy explicit artifact and Linux launch requirements. | Local runners / synthetic |
| **[Docker release workflow contract](../tools/tests/test_docker_release_workflow.py)** | Checks release jobs authenticate safely, retain platform independence and avoid exposing credential values. | Python / local fixtures |
| **[Read-only conversion-exception analysis](../tools/tests/test_analyze_cvexc.py)** | Checks conversion-exception analysis reports source declarations without activating unverified device behavior. | Python / local fixtures |
| **[Container smoke acceptance](../apps/knx-server/scripts/smoke-test.sh)** | Checks a real container boots, authenticates and reopens saved projects through its deployed API. | Explicit Docker; optional private import |
| **[Built-server naming and fictional-demo checks](../tools/verify_community_demos.js)** | Checks actual application naming and fictional demo workflows persist expected edits without bus operations. | Explicit harness + built app / server |

<details>
<summary>Source modules for this category</summary>

- **Release build provenance:** [knx-build-stamp/src/lib.rs](../crates/knx-build-stamp/src/lib.rs) · [git_facts.rs](../crates/knx-build-stamp/tests/git_facts.rs)
- **Dependency layering:** [layering.rs](../xtask/src/layering.rs)
- **Source headers and status ledger:** [headers.rs](../xtask/src/headers.rs) · [ledger.rs](../xtask/src/ledger.rs)
- **Documentation links and heading anchors:** [test_check_documentation.py](../tools/tests/test_check_documentation.py) · [anchors.rs](../xtask/src/anchors.rs)
- **Nonempty gate and fixture scope:** [fixtureServer.test.ts](../apps/knx-web/src/fixtureServer.test.ts) · [knx-testsupport/src/lib.rs](../crates/knx-testsupport/src/lib.rs) · [gate_scope.rs](../xtask/tests/gate_scope.rs)
- **AppImage artifact and launcher policy:** [test_appimage_launcher.py](../tools/tests/test_appimage_launcher.py) · [appimage.rs](../xtask/src/appimage.rs)
- **Docker release workflow contract:** [test_docker_release_workflow.py](../tools/tests/test_docker_release_workflow.py)
- **Read-only conversion-exception analysis:** [test_analyze_cvexc.py](../tools/tests/test_analyze_cvexc.py)
- **Container smoke acceptance:** [smoke-test.sh](../apps/knx-server/scripts/smoke-test.sh)
- **Built-server naming and fictional-demo checks:** [verify_community_demos.js](../tools/verify_community_demos.js) · [verify_name_editing.py](../tools/verify_name_editing.py)

</details>

## Website and project Story checks

The website and Story are separate products, not KNX compatibility certificates.

| Suite / representative source | What it checks | Prerequisites |
| --- | --- | --- |
| **[Bilingual static-site build](../website/tests/test_build.py)** | Checks generated website pages preserve language routes, local assets and expected download handoffs. | Python / local fixtures |
| **[Website browser and language routes](../website/tests/verify-browser.js)** | Checks built website navigation, explicit languages and local media work without unexpected external requests. | Explicit browser script + static site |
| **[Website headlines, legal page and demos](../website/tests/verify-demos.js)** | Checks headline motion, legal navigation and fictional demo downloads remain readable and reachable. | Explicit browser script + static site |
| **[Story schema, payload and rendering](../story/tests/test_narrator.py)** | Checks Story structures and escaped payloads build deterministically without losing graph relationships. | Python / local fixtures |
| **[Story candidate approval and release gates](../story/tests/test_approvals.py)** | Checks candidates, previews and approvals bind publication eligibility to exact reviewed content. | Python / local fixtures |
| **[Story browser acceptance](../story/tests/browser/check_story.mjs)** | Checks built Story scrolling, keyboard access and reduced motion against real browser behavior. | Explicit browser script + built Story |

<details>
<summary>Source modules for this category</summary>

- **Bilingual static-site build:** [test_build.py](../website/tests/test_build.py)
- **Website browser and language routes:** [verify-browser.js](../website/tests/verify-browser.js) · [verify-default-language.js](../website/tests/verify-default-language.js) · [verify-release.js](../website/tests/verify-release.js)
- **Website headlines, legal page and demos:** [verify-demos.js](../website/tests/verify-demos.js) · [verify-headlines.js](../website/tests/verify-headlines.js) · [verify-imprint.js](../website/tests/verify-imprint.js)
- **Story schema, payload and rendering:** [test_narrator.py](../story/tests/test_narrator.py) · [test_payload.py](../story/tests/test_payload.py) · [test_render.py](../story/tests/test_render.py) · [test_schema.py](../story/tests/test_schema.py)
- **Story candidate approval and release gates:** [test_approvals.py](../story/tests/test_approvals.py) · [test_candidate.py](../story/tests/test_candidate.py) · [test_previews.py](../story/tests/test_previews.py) · [test_release.py](../story/tests/test_release.py)
- **Story browser acceptance:** [check_story.mjs](../story/tests/browser/check_story.mjs)

</details>

## Additional quality gates

These are checks, not extra functional test suites. Their own regression tests
appear above; CI wiring is in [the workflow](../.github/workflows/ci.yml).

| Check / source | What it checks | Entry point |
| --- | --- | --- |
| **[Formatting](../.github/workflows/ci.yml)** | Checks Rust formatting stays consistent across the workspace without changing runtime behavior. | `cargo fmt --all --check` |
| **[Warnings-denied lint](../.github/workflows/ci.yml)** | Checks all workspace targets satisfy Clippy diagnostics with warnings treated as failures. | `cargo clippy --workspace --all-targets -- -D warnings` |
| **[Frontend type and build checks](../apps/knx-web/package.json)** | Checks TypeScript contracts compile and the frontend produces its expected production bundle. | `npm run build --prefix apps/knx-web` |
| **[Generated API bindings](../.github/workflows/ci.yml)** | Checks regenerated TypeScript declarations match committed API bindings except tolerated trailing whitespace. | `CI ts-rs bindings comparison` |
| **[Dependency policy](../deny.toml)** | Checks third-party dependency licenses, advisories and source policies against the maintained repository configuration. | `cargo deny check` |
| **[Local documentation targets](../tools/check_documentation.py)** | Checks local document links, image descriptions and the complete manual chapter chain. | `python3 tools/check_documentation.py` |
| **[Layer, header, anchor, ledger and corpus gates](../xtask/src/main.rs)** | Checks repository structure, source headers, links, status records and honest private-test admission. | `cargo run --locked -p xtask -- <task>` |
| **[Private corpus runner](../tools/run_corpus_tests.py)** | Runs explicitly identified private regressions with isolated data settings and refuses absent inputs. | `python3 tools/run_corpus_tests.py` |

The five repository tasks are `check-layering`, `check-headers`, `check-anchors`,
`check-ledger` and `check-corpus-gates`. Run them explicitly against the intended
checkout. The AppImage and container checks need actual built artifacts.

## Running the right tests

Start with [building and running tests](manual/development/02-building-from-source.md#running-the-tests)
and [contributor quality gates](manual/development/01-contributing.md).
The following commands are entry points, not a claim they were executed here.

```bash
# Ordinary Rust tests; ignored private/hardware cases are not selected.
cargo test --workspace

# Frontend unit/component tests after npm ci.
npm test --prefix apps/knx-web

# Intercepted-API browser suite: inspect browser/build prerequisites first.
npm run test:e2e --prefix apps/knx-web

# Repository tooling unit tests; no application or hardware acceptance claim.
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/tests
```

- **Browser scope matters:** the [default Playwright configuration](../apps/knx-web/playwright.config.ts)
  selects `*.e2e.ts` fixtures. Real browser rendering does not make intercepted
  API answers real backend results.
- **Studies and captures are separate:** `*.load.ts`, `*.study.ts`, `*.shots.ts`
  and `*.native.ts` use their own configurations. Some need a built server;
  [the README Flow capture](../apps/knx-web/playwright.readme.config.ts) uses fixtures.
  Capture scripts can write images or measurement files: use owned output paths.
- **Private regressions:** inspect ignore reasons and fixture variables before
  explicit selection. The corpus runner only selects its declared corpus cases;
  other private tests may require separate opt-ins. Supply authorized inputs,
  isolate networking and never publish their contents or identifying fingerprints.
- **Website and Story:** use their own [website](../website/README.md) and
  [Story](../story/README.md) instructions. Browser snippets are explicit harness
  recipes, not necessarily standalone Node programs or normal CI tests.
- **Hardware:** no generic live-test command is offered. Verify target, permitted
  operation and recovery before any contact. A simulator passing does not grant
  write permission, and Undo cannot take back a telegram.

## Keeping this catalogue useful

1. Add or revise the relevant suite group when tests are added, moved or materially changed.
2. Keep each description at 10–15 whitespace-separated words; names, examples and prerequisites are separate.
3. Keep actual module links in the expandable source list. Do not count fixtures or generated cases as independent suites.
4. Update the first-line timestamp and inspected source revision after a complete inventory refresh.
5. Check all local links, anchors, prerequisite labels and test-family coverage before delivery.
6. Keep executed evidence in dated verification records, not imaginary green ticks in this inventory.
7. Never add real customer inputs, excerpts or private fixture identities to explain a test.

This is deliberately curated Markdown, not a new generator or CI subsystem.
New tests should bring their documentation with them. They already know the way.
