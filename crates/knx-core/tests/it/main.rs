//! knx-core integration tests in one binary: one link, one test pool (ADR-0144).

mod absolute_load_control;
mod add_com_objects;
mod dpt_spec_document_inventory;
mod dpt_spec_semantics_audit;
mod dpt_spec_width_audit;
mod functions;
mod instruction;
mod instruction_commands;
mod legacy_group_tables;
mod legacy_object_table;
mod multi_installation;
mod notes;
mod property_download;
mod property_recovery;
mod system300_group_tables;
mod systemb_group_tables;
mod systemb_object_table;
mod systemb_object_table_build;
mod topology_repair;
