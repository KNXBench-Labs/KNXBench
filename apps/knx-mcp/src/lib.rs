//! Read-only MCP server over saved KNXBench project files (ADR-0090).
//!
//! Module map: [`args`] parses the command line, [`workspace`] holds the
//! read-only project snapshots and product database, [`tools`] implements
//! the eight tools as plain functions, [`server`] exposes them over MCP.

pub mod args;
pub mod diff_render;
pub mod issues;
pub mod server;
pub mod tools;
pub mod workspace;
