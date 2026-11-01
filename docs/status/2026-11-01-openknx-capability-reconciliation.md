# OpenKNX capability reconciliation (OKX-07)

Observed baseline: 1 November 2026, against main at `f22ef1b04`.

OKX-04 update: canonical baggage joins, bounded PNG decoding and explicit
offline HTML are delivered with owning API/CLI tests, independent review,
real-browser proof and a 37/37 release measurement. Other rows
retain the baseline observations below; no new execution is inferred for them.

## Purpose and scope

The [OpenKNX product-data analysis of 30 October 2026](../research/2026-10-30-openknx-distribution-analysis.md)
ended in a 32-row
capability audit (C01–C32) pinned to `72db561a4`. It mapped each row to the
OKX-01…OKX-07 work packages and gave it one of four dispositions: `missing`,
`reuse`, `blocked` or `unrelated`. This record says where each row stands on
current main, which commit delivered it, which document owns it and which
tests on main back it. OKX-07 is acceptance across those packages, not a
feature of its own. `OKX-*` are research handles, not ledger IDs. The
[ledger](LEDGER.md) remains the status of record for every ledger ID named here.

## Method

- Static check only. Each delivering commit was looked up with `git log` and
  `git show --stat` and confirmed to be an ancestor of `f22ef1b04`. Each
  test name below was found by `git grep` on that revision, with its
  `#[test]`, `#[tokio::test]`, `it(...)` or `test(...)` marker.
- **No test, build or runtime check was run for this record.** "Test on main"
  means the function exists there, not that it passed here. Pass results are
  claimed only by the delivery records the owner documents link to.
- Every test the audit cited as existing acceptance is still on main under the
  same name. The audit's existing-test review was static too.
- The audit's own inputs (private corpus witnesses, hashes and scratch files)
  are not reproduced here.

Path prefixes in the table: `pdb/` is `crates/knx-productdb/tests/it/`,
`pdb-src/` is `crates/knx-productdb/src/`, `srv/` is
`apps/knx-server/tests/it/`, `cli/` is `apps/knx-cli/tests/it/`, `app/` is
`crates/knx-app/tests/it/` and `web/` is `apps/knx-web/src/`.

## Row reconciliation

Delivering commits, by package:

- **OKX-01:** feature `a0466e9ea`, merged by `d606b3938`.
- **OKX-02:** `2f0440c04` and `f3b895da4`, merged by `a86781b65`. Follow-up
  `128233100`, merged by `2f5814441`. Script-range hotfix `61e730abf`.
- **OKX-03:** `457295909`, `504b3ad5b`, `95262e8ff`, `4c149d5e9` and
  `a5a1b912b`, merged by `afb28da1f`. Measurement docs in `0e3d9677b`.
- **OKX-04:** the OKX-04 package merge of 1 November (see `git log`).
- **OKX-05:** `2a4e43db9`, merged by `6c731be2d`.
- **OKX-06:** `e7a175ae4` and `fe6bc8af4`, merged by `484e946a8`. Follow-up
  `8a597c648`, merged by `2e88d9b12`.

"Reuse-unchanged" means the audit found the capability already there and its
reuse disposition still holds: no OKX commit replaced or widened the existing
behaviour. That is not "untouched": rows marked "consumed by OKX-0x" gained
inspection-side reporting from those packages (their tests are listed beside
the baseline tests), and OKX-06 tightened the shared source-hash guard that
C08's editor host relies on.

| Row | Capability | Audit | Current source status | Commit(s) | Owner | Baseline / owning tests |
| --- | --- | --- | --- | --- | --- | --- |
| C01 | Native package and container admission | missing | delivered (OKX-01) | OKX-01 | [contract][di] | `pdb/distribution_archive.rs::windows_distribution_is_positive_without_weakening_native_package_rejection`; `cli/distribution_inspect_cli.rs::real_cli_emits_useful_json_without_source_sidecar_profile_or_authority_mutation`; `pdb/standalone_packages.rs::malformed_and_unsupported_packages_leave_no_rows` |
| C02 | Single-file dispatch, monolithic manufacturer XML | missing | delivered (OKX-01) | OKX-01 | [contract][di] | `pdb/distribution_matrix.rs::split_oracle_order_and_repeat_keep_typed_facts`; `pdb/distribution_inspection.rs::combined_source_exposes_typed_catalogue_hardware_and_dynamic_fallback_without_installing`; `pdb/distribution_split.rs::split_archive_links_declared_manufacturer_sources_with_exact_candidate_coordinates`; `pdb/distribution_matrix.rs::foreign_names_and_qualified_attributes_cannot_escape_scope` |
| C03 | Parameter kinds, normalized schema | reuse | reuse-unchanged | — | [WP-08][wp08] | `pdb/parameter_kinds.rs::every_observed_parameter_kind_is_stored_as_its_own_kind` |
| C04 | Defaults, stored values, lexical fidelity | reuse | reuse-unchanged | consumed by OKX-01 | [contract][di] | `srv/http_parameter_panel.rs::get_returns_stored_and_defaulted_top_level_fields`; `pdb/dynamic_tree.rs::a_scoped_value_wins_for_its_own_instantiation_and_the_other_sees_the_program_default`; `pdb/distribution_matrix.rs::absent_empty_override_and_ambiguous_defaults_stay_distinct` |
| C05 | Number and Restriction bounds versus memory | missing | delivered (OKX-03, diagnostics only; TypeNumber candidates only) | OKX-03 | [source candidates][di-cand] | `pdb/distribution_candidates.rs::numeric_candidate_presence_and_exact_closed_bounds`; `pdb/distribution_candidates.rs::default_lists_findings_and_counts_every_checked_occurrence`; `cli/distribution_inspect_cli.rs::real_cli_reports_source_candidates_exactly_like_the_library` |
| C06 | Float | blocked | still blocked | — | [WP-09][wp09] | `srv/http_parameter_panel.rs::non_finite_float_bounds_refuse_http_writes_without_changing_project_or_source`; `pdb-src/image.rs::a_float_parameter_is_refused_for_its_type`; `apps/knx-server/src/domain.rs::float_accepts_scientific_notation_without_bounds` |
| C07 | Text and text memory encoding | blocked | still blocked | — | [KL-171][kl171] | `pdb-src/image.rs::a_text_value_is_encoded_as_declared_and_ascii_needs_no_declaration` |
| C08 | Time and checked plain-count controls | reuse | reuse-unchanged | — | [editor host][peh] | `web/features/parameter-editors/plan.test.ts` "does not offer a packed or unknown time unit as a decimal count (census G6)"; `srv/http_parameter_editor_source.rs::exact_device_program_source_is_read_only_and_not_a_write_permit` |
| C09 | Date | blocked | still blocked | — | [WP-08][wp08] | `pdb/date_kind.rs::a_fresh_install_stores_date_and_reports_its_encoding`; `pdb-src/image.rs::a_date_parameter_is_refused_for_its_type` |
| C10 | Colour, Picture, Raw, IPAddress | blocked | partially delivered (OKX-06: IPAddress literal editor only) | OKX-06 | [checked editors][cpe] | `srv/http_parameter_editor_source.rs::okx06_real_bound_http_vectors_undo_redo_and_native_reopen_keep_exact_ip_lexemes`; `pdb/parameter_editor.rs::okx06_placed_ip_literals_still_have_no_image_codec`; `pdb/parameter_editor.rs::color_and_picture_attributes_remain_evidence_not_a_codec` |
| C11 | Memory and Property placement, Union membership | reuse | reuse-unchanged | consumed by OKX-03 | [WP-09][wp09] | `pdb-src/code.rs::property_placements_and_unparsable_ones_are_kept_as_unmodelled`; `pdb/relative_program_code.rs::relative_metadata_and_blobs_are_retained_without_inventing_absolute_addresses`; `pdb/distribution_candidates.rs::each_union_keeps_its_own_placement` |
| C12 | Union overlapping active alternatives | blocked | partially delivered (OKX-03 interval diagnostics; semantics blocked) | OKX-03 | [source candidates][di-cand], [PRG-01][prg01] | `pdb/distribution_candidates.rs::union_declarations_intervals_and_subset_alternatives_separate`; `pdb/distribution_candidates.rs::source_union_interval_units_and_unknown_width`; `pdb/system2_image.rs::two_active_union_members_never_use_070n_last_member_wins` |
| C13 | Dynamic conditions, no-match disposition | reuse | reuse-unchanged | consumed by OKX-01 | [contract][di] | `pdb/dynamic_tree.rs::no_matching_branch_and_no_default_activates_nothing_and_is_reported`; `pdb/dynamic_tree.rs::every_comparison_operator_selects_its_matching_when_through_evaluate`; `pdb/distribution_grammar.rs::uninspected_dynamic_subtree_is_unknown_rather_than_covered` |
| C14 | Access and calculation write authority | reuse | reuse-unchanged | consumed by OKX-01/02 | [semantics boundary][psb] | `pdb/write_authority.rs::install_records_parameter_ref_access_and_calculation_members`; `srv/http_parameter_write_authority.rs::a_program_without_recorded_write_authority_is_read_only`; `pdb/distribution_dependencies.rs::source_calculation_inventory_is_not_native_authority` |
| C15 | Enclosing block access, separators | reuse | reuse-unchanged | consumed by OKX-05 | [controls and separators][cas] | `srv/http_parameter_block_access.rs::writes_below_a_restricting_block_are_400_and_change_nothing`; `srv/http_parameter_block_access.rs::separators_are_listed_between_the_fields_with_their_declarations`; `srv/http_parameter_grid/passive_labels.rs::ancestor_access_survives_into_the_dto_verbatim` |
| C16 | Module definitions, nested activation, arguments | blocked | still blocked | — | [WP-07][wp07] | `pdb/dynamic_tree.rs::a_module_inside_a_module_def_naming_a_different_module_def_is_expanded_two_levels`; `pdb/distribution_modules.rs::evaluated_module_instances_keep_distinct_view_bound_scope_chains` |
| C17 | Repeated modules, own-instance read-only values | reuse | reuse-unchanged | — | [WP-07][wp07] | `pdb/repeated_instance_values.rs::sibling_instance_values_are_not_stale_or_fed_to_the_shared_evaluator`; `srv/http_repeated_instance_values.rs::own_instance_values_are_inspectable_but_never_writable` |
| C18 | Calculations, validations, event handlers | missing | delivered (OKX-02, declarations only) | OKX-02 | [dependency evidence][di-dep] | `pdb/distribution_dependencies.rs::declaration_inventory_does_not_apply_assign_or_calculation`; `pdb/distribution_dependencies.rs::validation_declared_not_run_for_two_ordinary_values`; `pdb/distribution_dependencies.rs::handler_mode_and_opaque_script_are_not_function_resolution` |
| C19 | Buttons, conditional renames, Assign, allocators | missing | delivered (OKX-02 Assign and handler evidence; OKX-05 passive labels) | OKX-02, OKX-05 | [dependency evidence][di-dep], [Button labels][btn] | `pdb/distribution_dependencies.rs::declared_graph_cycle_and_unresolved_endpoints_are_not_runtime_effects`; `pdb/distribution_dependencies.rs::subset_activity_is_not_handler_reachability`; `srv/http_parameter_grid/passive_labels.rs::display_reads_change_nothing_and_a_button_id_is_no_write_target` |
| C20 | Parameter workspace, grids, indentation | blocked | partially delivered (OKX-05: in-cell Button literal label only) | OKX-05 | [Button labels][btn], [WP-05][wp05] | `pdb/parameter_layout/passive_buttons.rs::only_the_exact_literal_form_is_admitted`; `srv/http_parameter_grid/passive_labels.rs::a_mixed_program_returns_literal_passive_labels_and_unchanged_fields`; `web/ParameterWorkspace.test.tsx` "stacks separator, passive label and field in one cell and keeps drafts across area and search changes" |
| C21 | Language overlays, suffixes, positional text | blocked | still blocked | — | [WP-05][wp05] | `pdb/suffix_text.rs::a_translated_suffix_follows_the_layer_that_supplies_it`; `srv/http_parameter_captions.rs::requested_language_overlays_caption_templates_before_positional_fill`; `srv/http_parameter_grid/passive_labels.rs::alternate_sources_keep_their_literal_and_a_product_language_translates_nothing` |
| C22 | Communication objects: DPT, layered flags and size | unrelated | unrelated | — | [WP-06][wp06] | `srv/http_parameter_panel.rs::newly_created_objects_report_unresolved_dpt_without_guessing`; `pdb-src/image.rs::product_defaults_write_parameters_objects_and_tables` |
| C23 | Parameter edits, object reconciliation | reuse | reuse-unchanged | — | [object refresh][por] | `srv/http_parameter_panel.rs::parameter_enable_adds_missing_objects_in_one_persistable_undo_step`; `srv/http_parameter_panel.rs::missing_active_module_object_refuses_the_entire_parameter_edit` |
| C24 | Baggage resources, payload isolation | missing | delivered (OKX-04) | OKX-04 | [contract][di], [KL-187][kl187] | `pdb/distribution_baggage.rs::archive_maps_literal_adjacent_stem_and_exact_member_identity`; `pdb/distribution_baggage.rs::png_preview_is_decoded_reencoded_and_bound_to_original_and_derived_hashes`; `pdb/distribution_baggage_bounds.rs` and `pdb/distribution_baggage_paths.rs`; `cli/distribution_inspect_cli.rs::explicit_html_is_offline_passive_and_json_remains_the_default`; native inventory regression retained |
| C25 | On-demand code and load source model | reuse | reuse-unchanged | — | [PRG-01][prg01] | `pdb-src/code.rs::table_placements_and_options_are_read`; `srv/http_download_support_guidance.rs::a_declared_download_plugin_is_refused_for_every_scope_and_readiness_agrees` |
| C26 | Ordinary offline image, load plan | unrelated | unrelated | — | [PRG-01][prg01] | `pdb-src/image.rs::product_defaults_write_parameters_objects_and_tables`; `app/vendor_download_plugin.rs::customer_markerless_public_writer_keeps_literal_bytes_and_steps` |
| C27 | System 1 and System 2 strict and experimental images | unrelated | unrelated | — | [System-2 image][s2img] | `pdb/system2_image.rs::real_product_evaluator_builds_parameter_and_legacy_table_images_without_admitting_a_download`; `pdb/experimental_system2_image.rs::project_experimental_union_overlay_retains_template_not_default_union_base` |
| C28 | Relative, System B and System 300 prerequisites | blocked | still blocked | — | [WP-14][wp14] | `pdb/relative_program_code.rs::absent_source_data_is_not_synthesized_even_for_a_large_declared_size` |
| C29 | Customer-adjustable, invisible directives, loaded images | unrelated | unrelated | — | [customer-adjustable][caw] | `srv/http_download_exemplar/customer_adjustable.rs::http_start_rechecks_each_genuinely_shown_scope_after_source_changes_before_tunnel_or_frame`; `app/vendor_download_plugin/customer_adjustable.rs::kind_then_plugin_then_customer_guard_precede_image_failures`; `pdb/system2_image.rs::explicit_invisible_parameter_policies_never_return_an_uninterpreted_image` |
| C30 | Unknown retention, conflict identity, reporting | missing | delivered (OKX-01) | OKX-01 | [contract][di] | `pdb/distribution_matrix.rs::unknown_occurrences_are_not_multiplied_by_sibling_readers`; `pdb/distribution_grammar.rs::nested_dynamic_unknowns_are_inventoried_in_application_and_module_scopes`; `pdb/distribution_matrix.rs::foreign_names_and_qualified_attributes_cannot_escape_scope`; `pdb/source_diagnostics.rs::absent_measurement_differs_from_measured_zero_and_corrupt_rows_fail` (native baseline, not inspection) |
| C31 | Resource budgets, fail-closed projections | reuse | reuse-unchanged | consumed by OKX-01 | [limits][di-lim] | `pdb/distribution_resources.rs::callers_cannot_raise_default_security_profile_or_disable_report_work_bound`; `pdb/distribution_dynamic_budget.rs::dynamic_scalar_copy_budget_refuses_before_evaluation_instead_of_expanding_raw_values`; `pdb/dynamic_tree.rs::a_wide_non_cyclic_fan_out_trips_the_expansion_budget_before_full_expansion` |
| C32 | Catalogue, hardware and application joins; master boundary | missing | delivered (OKX-01) | OKX-01 | [contract][di] | `pdb/distribution_matrix.rs::multi_manufacturer_application_duplicate_and_dangling_scopes_are_explicit`; `pdb/distribution_split.rs::repeated_declared_manufacturer_ids_stay_separate_in_one_original_document`; `pdb-src/query.rs::catalog_item_program_reports_a_missing_relation` (native baseline, not inspection) |

Counts: delivered 8, partially delivered 3, reuse-unchanged 11, still
blocked 6, unrelated 4 (32 rows). The audit's own counts
were missing 8, reuse 11, blocked 9, unrelated 4.

Each delivered row is delivered within the audit's needed change, which was
read-only inspection or a diagnostic. None of them adds installation, write,
initialization or programming authority. The partially delivered C10 slice
(OKX-06) is different: it adds a checked project edit for IP literals through
the existing undoable write path, with no memory codec and no device
programming. None claims ETS or KNX compatibility.

Related changes that deliver no row: the master BOM byte-range fix
(`f4761e69a`, merged by `5eaadab0b`, product database 31) corrects the
native master reader next to C32 and does not touch distribution inspection.
The GATE-01 SIGTERM fixture fix (`a729db18f`, merged by `f22ef1b04`) is test
tooling.

## Still open

- **Baggage beyond inspection (C24).** `nativePackageBaggageNotPerformed`
  still excludes native baggage installation; help archives remain metadata
  only and no Picture editor exists ([KL-187][kl187]).
- **Blocked rows and the evidence each one still lacks:**
  - C06: independent product-parameter Float vectors for encoding, width,
    byte order and sentinels. A bus float codec is not that evidence.
  - C07: a defining UTF-8 text contract for capacity, termination and cuts,
    with multibyte vectors ([KL-171][kl171]).
  - C09: a defined memory encoding for `TypeDate` (an `Encoding="DPT 11"`
    label does not set the bytes) and a checked editor form with its own
    vectors ([WP-08][wp08]).
  - C10: exact Colour, Picture and Raw type definitions (the Colour/Picture
    definition found is behind a login) and their codec vectors. The
    IPAddress editor has no memory codec.
  - C12: independent layout and effect evidence for which Union alternative
    is active. Any change to shared selection needs [PRG-01][prg01] owner
    coordination.
  - C16: independent BaseOffset, BaseNumber, allocator and Repeat mapping
    evidence ([WP-07][wp07], KL-068).
  - C20: per-form evidence for parameter-driven and translated row/column
    captions, inline, nested and `CollapseIfEmpty` blocks, and Table versus
    Grid rules. The last needs the referenced schema rules document
    ([WP-05][wp05], KL-003).
  - C21: an independently authored multilingual oracle for row/column
    captions and `TextParameterRefId`.
  - C28: per-family real-source fill, relocation, resource-placement and
    effect contracts, with their own oracle and recovery gates
    ([WP-14][wp14]).
- **Deferred from OKX-05:**
  - A refused label binding had no diagnostic of its own. Delivered on
    1 November 2026 after this record: one `passiveLabelsUnbound` warning
    names the cause ([Button labels][btn], Refused binding).
  - The extra source read per panel was not measured on the largest retained
    real program. Measured on 1 November 2026 after this record: about 70 ms
    on the largest (26.6 MB) program ([Button labels][btn], Limits).
  - Draft retention next to passive labels was covered in Vitest only. A
    Chromium case in `apps/knx-web/e2e/parameter-matrix.e2e.ts` now shows the
    real-browser behaviour: the typed value is written when focus leaves it,
    and the same editor and the label survive area and search changes.
  - The owner had not accepted the cue wording ("Source button label – action
    unsupported"). Accepted on 1 November 2026 after this record
    ([Button labels][btn], Browser).
- **Limitation references:** [KL-187][kl187] (distribution inspection),
  KL-003 (layout and editing boundaries), KL-068 (repeated module
  activation), [KL-171][kl171] (UTF-8 text encoding), KL-172 (stored
  `LoadedImage`, C29) and the [checked editors][cpe] catalogue entry (C10).

## Discrepancies found while checking

- The 30 October 2026 analysis note was not tracked on main when this record
  was written, so the [distribution inspection contract][di] linked the
  record instead. Its public English edition is now the
  [analysis note](../research/2026-10-30-openknx-distribution-analysis.md);
  the private German original and its evidence stay untracked.
- [IMPLEMENTATION_STATUS](../IMPLEMENTATION_STATUS.md#read-only-manufacturer-distribution-inspection-31-october-2026-source)
  gives the delivery-time test counts "Distribution 50 … CLI 6". That history
  is not rewritten; for current figures, main now has 67 `#[test]` functions
  in `pdb/distribution_*.rs` (17 of them in `distribution_candidates.rs`) and
  7 in `cli/distribution_inspect_cli.rs`.
- C24 baseline: `baggageAssociationNotPerformed` was stated but not directly
  test-backed on the observed main revision. The format-5 candidate replaces it
  only for the performed distribution association pass and tests the separate
  `nativePackageBaggageNotPerformed` qualification; installation is unchanged.
- C01: when an archive contains `knx_master.xml`, `pdb-src/distribution_zip.rs`
  emits `masterMemberPresentNotInspected` and reports `MasterNotInspected`
  instead of `MissingMaster`. No test on main asserts that branch, and there
  is no separate wrapper class; the audit asked for native-package, wrapper
  and distribution classification controls. The status stays "delivered" for
  the distribution side; this branch is stated, not test-backed.
- Inspection-side acceptance the audit listed but OKX did not deliver. These
  rows keep their status on their baseline behaviour; the inspection report
  simply does not cover them yet:
  - C25: no report of code-reader availability; the distribution sources do
    not call the on-demand code reader.
  - C17: no "repeat activation not evaluated" marker; the distribution code
    does not handle `Repeat` at all.
  - C16: no report of unresolved allocator bindings versus literal module
    arguments.
  - C28: inspection never reads `Code` or `RelativeSegment`, so it cannot tell
    absent, empty and malformed payloads apart.
  - C07: no report of the declared text encoding or capacity.
- C03: inspection reports declared `Type` and `SizeInBit` only for TypeNumber
  candidates. It does not mirror every typed-reader kind, which the audit
  listed as acceptance.

[di]: ../contracts/distribution-inspection.md
[di-dep]: ../contracts/distribution-inspection.md#dependency-evidence
[di-cand]: ../contracts/distribution-inspection.md#source-candidates
[di-lim]: ../contracts/distribution-inspection.md#limits
[kl187]: ../limitations/KL-187-distribution-inspection.md
[kl171]: ../limitations/KL-171-utf-8-text-download-encoding.md
[btn]: ../contracts/alpha16-parameter-layout.md#passive-source-button-labels-button-literal-passive-v1
[cpe]: ../limitations/checked-parameter-editors.md
[peh]: ../contracts/parameter-editor-host.md
[psb]: ../contracts/PARAMETER_SEMANTICS_BOUNDARY.md
[cas]: ../contracts/controls-and-separators.md
[por]: ../contracts/parameter-object-refresh.md
[s2img]: ../contracts/offline-download-system2-image.md
[caw]: ../contracts/customer-adjustable-write-admission.md
[wp05]: ../open-work/WP-05-parameter-layout-fidelity.md
[wp06]: ../open-work/WP-06-communication-object-organisation.md
[wp07]: ../open-work/WP-07-module-instance-authority.md
[wp08]: ../open-work/WP-08-parameter-value-codecs.md
[wp09]: ../open-work/WP-09-offline-image-evidence.md
[wp14]: ../open-work/WP-14-offline-family-outcomes.md
[prg01]: ../open-work/PRG-01-programming-gaps.md
