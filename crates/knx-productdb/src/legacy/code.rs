//! A legacy program's download code, read from its `s19_block` rows (ADR-0094, L4).
//!
//! An EX-IM product database carries each program's load procedure as
//! `s19_block` rows, one row per step in `BLOCK_NUMBER` order. This module
//! turns them into the same [`crate::code`] types an XML product yields, so
//! the image builder and the download planner treat a legacy program
//! exactly like a modern one. No step is invented and none is dropped: a
//! row this module cannot read with evidence becomes a
//! [`LoadStep::Unmodelled`] step, which the planner refuses by name.
//!
//! What a row means, and the evidence (docs/research/legacy-vd-mapping.md,
//! *L4*):
//!
//! - `[V]` `CONTROL_CODE` is the first octet of the step's record. For load
//!   state machine events it is `(LsmIdx << 4) | event`, the event numbered
//!   1 Start Loading, 2 Load Completed, 3 additional load control, 4 Unload
//!   (the same first octet as MP §3.31.2's memory-mapped record,
//!   [`knx_core::commissioning::load_control_memory`]). `0Eh` is Connect,
//!   `0Fh` Disconnect, `0Ch` Restart and `07h` Compare Property. Ten
//!   programs (the `.vd4`'s N000520 and nine `070nh` programs of the
//!   Siemens `.vd5`) read this way give, step for step, the load procedure
//!   ETS's own conversion of the same program holds.
//! - `[V]` `Record`, where the file has it, is the step as sixteen octets.
//!   Its first octet equals `CONTROL_CODE` in every row of all three real
//!   files that carry it (1,104 rows). An allocation record holds the
//!   segment's *end* address where MP's memory-mapped record holds its
//!   length; it is the Cookbook *Load Controls* record, not a frame. Each
//!   record is checked against the row's columns, and a disagreement is
//!   refused. A Compare Property record holds object index, property id and
//!   ten octets of data at octets 1, 2 and 6–15; those equal ETS's
//!   `LdCtrlCompareProp`. Without a `Record` column (the `.vd3`) a Compare
//!   Property step is not readable and stays unmodelled.
//! - `[V]` `BLOCK_DATA` and `BLOCK_MASK` are a segment's base image and
//!   mask, as long as `SEGMENT_LENGTH`. The data equals ETS's `Data` in all
//!   ten programs. A mask octet is `00h`, `01h` or `FFh`; ETS writes `FFh`
//!   where the legacy file has `01h` (four programs) or `FFh` (six), so
//!   both read as `FFh`. Any other mask octet is refused.
//! - The task segment's application identity is not taken from the record.
//!   `[V]` Its record names `0079h 0001h 01h` for N000520, but the nine
//!   devices ETS programmed with it report `PID_PROGRAM_VERSION`
//!   `006Ah 0001h 22h`: the program's manufacturer, `DEVICE_TYPE` and
//!   `PROGRAM_VERSION`, which the planner takes from the program's
//!   attributes, as for an XML program.
//! - Tables. `[D]` Load state machine 1 is the address table (MP §3.31.1),
//!   so its first data segment holds it. `[V]` The association and group
//!   object tables start at `ASSOCTAB_ADDRESS` and `COMMSTAB_ADDRESS`.
//!   `MaxEntries` is `(ADDRESS_TAB_SIZE − 1) / 2 − 1` and
//!   `(ASSOCTAB_SIZE − 1) / 2`: an address table has a length octet and the
//!   device's own address before its group addresses, an association table
//!   a length octet. All ten programs' placements and limits equal ETS's.
//! - `[V]` A TaskCtrl1 record (segment type 4) is MP §3.31.2's
//!   `L3 04h 00h AAAA NN`: the interface objects' address at octets 3–4 and
//!   their count at octet 5. Five `.vd5` programs carry one, and each equals
//!   ETS's `LdCtrlTaskCtrl1`. Without a `Record` column it stays unmodelled.
//! - `[V]` A row with a non-empty `MERGE_ID` or a non-zero `PROC_MASK` (the
//!   `.vd5`'s `07B0h` programs) belongs to a merged procedure. Nothing
//!   documents how one merges, so such a row is unmodelled.

use std::collections::BTreeMap;
use std::fmt;

use rusqlite::{Connection, OptionalExtension};

use crate::code::{
    AbsoluteSegment, CodeError, LoadProcedure, LoadStep, MemoryPlacement, ParameterPlacement,
    ProgramCode, TablePlacement,
};

use super::exim::{parse_exim, ExImDocument, ExImTable};

/// The download-relevant part of one legacy program.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegacyProgramCode {
    /// The data and stack segments the procedure allocates, in step order.
    /// Ids are `<program id>_AS-<address as four hex digits>`, the scheme
    /// ETS's conversion uses.
    pub segments: Vec<AbsoluteSegment>,
    /// The procedure, or `None` when the program has no `s19_block` row.
    pub load_procedure: Option<LoadProcedure>,
    pub address_table: Option<TablePlacement>,
    pub association_table: Option<TablePlacement>,
    pub com_object_table: Option<TablePlacement>,
}

/// Why a legacy program's code was not read. Every variant names the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyCodeError {
    /// The payload has no such `application_program` row.
    UnknownProgram(String),
    /// One `s19_block` row is inconsistent or unreadable.
    Block {
        /// Its `BLOCK_ID`.
        block: String,
        /// What is wrong.
        cause: String,
    },
    /// The `application_program` row is unreadable.
    Program(String),
}

impl std::error::Error for LegacyCodeError {}

impl fmt::Display for LegacyCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LegacyCodeError::UnknownProgram(id) => {
                write!(f, "the payload has no application_program {id}")
            }
            LegacyCodeError::Block { block, cause } => write!(f, "s19_block {block}: {cause}"),
            LegacyCodeError::Program(cause) => write!(f, "application_program: {cause}"),
        }
    }
}

const CONNECT: u8 = 0x0E;
const DISCONNECT: u8 = 0x0F;
const RESTART: u8 = 0x0C;
const COMPARE_PROPERTY: u8 = 0x07;
const EVENT_START_LOADING: u8 = 1;
const EVENT_LOAD_COMPLETED: u8 = 2;
const EVENT_ADDITIONAL: u8 = 3;
const EVENT_UNLOAD: u8 = 4;
const SEGMENT_DATA: u8 = 0;
const SEGMENT_STACK: u8 = 1;
const SEGMENT_TASK: u8 = 2;
const TASK_CONTROL_1: u8 = 4;
const RECORD_OCTETS: usize = 16;
/// The address table's load state machine (MP §3.31.1).
const ADDRESS_TABLE_LSM: u8 = 1;

/// One `s19_block` row, read by column name.
struct Block<'a> {
    table: &'a ExImTable,
    row: usize,
    id: String,
}

impl Block<'_> {
    fn get(&self, column: &str) -> Option<String> {
        self.table
            .text_by_name(self.row, column)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    fn error(&self, cause: impl Into<String>) -> LegacyCodeError {
        LegacyCodeError::Block {
            block: self.id.clone(),
            cause: cause.into(),
        }
    }

    fn number<T: TryFrom<i64>>(&self, column: &str) -> Result<T, LegacyCodeError> {
        let text = self
            .get(column)
            .ok_or_else(|| self.error(format!("{column} is empty")))?;
        text.parse::<i64>()
            .ok()
            .and_then(|n| T::try_from(n).ok())
            .ok_or_else(|| self.error(format!("{column} {text:?} is out of range")))
    }

    fn hex(&self, column: &str) -> Result<Option<Vec<u8>>, LegacyCodeError> {
        self.get(column)
            .map(|text| decode_hex(&text).ok_or_else(|| self.error(format!("{column} is not hex"))))
            .transpose()
    }

    fn record(&self, code: u8) -> Result<Option<[u8; RECORD_OCTETS]>, LegacyCodeError> {
        let Some(octets) = self.hex("Record")? else {
            return Ok(None);
        };
        let record: [u8; RECORD_OCTETS] = octets.try_into().map_err(|octets: Vec<u8>| {
            self.error(format!(
                "Record has {} octets, not {RECORD_OCTETS}",
                octets.len()
            ))
        })?;
        if record[0] != code {
            return Err(self.error(format!(
                "Record starts with {:02X}h, CONTROL_CODE is {code:02X}h",
                record[0]
            )));
        }
        Ok(Some(record))
    }

    fn unmodelled(&self, name: &str) -> LoadStep {
        let mut attributes = BTreeMap::new();
        for column in [
            "BLOCK_ID",
            "BLOCK_NUMBER",
            "CONTROL_CODE",
            "SEGMENT_TYPE",
            "Record",
        ] {
            if let Some(value) = self.get(column) {
                attributes.insert(column.to_string(), value);
            }
        }
        LoadStep::Unmodelled {
            name: format!("s19_block {name}"),
            attributes,
            has_children: false,
        }
    }
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

fn word(record: &[u8; RECORD_OCTETS], at: usize) -> u16 {
    u16::from_be_bytes([record[at], record[at + 1]])
}

/// Reads program `exim_program_id`'s code from `document`; `program_id`
/// is its KNXBench id, for the segment ids.
pub fn legacy_program_code(
    document: &ExImDocument,
    exim_program_id: &str,
    program_id: &str,
) -> Result<LegacyProgramCode, LegacyCodeError> {
    let program = program_row(document, exim_program_id)?;
    let mut code = LegacyProgramCode::default();
    let Some(table) = document.table("s19_block") else {
        return Ok(code);
    };
    let mut blocks = Vec::new();
    for row in 0..table.row_count() {
        let id = table
            .text_by_name(row, "BLOCK_ID")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let block = Block { table, row, id };
        if block.get("PROGRAM_ID").as_deref() == Some(exim_program_id) {
            let number: i64 = block.number("BLOCK_NUMBER")?;
            blocks.push((number, block));
        }
    }
    if blocks.is_empty() {
        return Ok(code);
    }
    blocks.sort_by_key(|(number, _)| *number);
    if let Some(pair) = blocks.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(pair[1]
            .1
            .error(format!("BLOCK_NUMBER {} appears twice", pair[1].0)));
    }

    let mut steps = Vec::with_capacity(blocks.len());
    for (_, block) in &blocks {
        steps.push(step(block, program_id, &mut code.segments)?);
    }
    code.load_procedure = Some(LoadProcedure {
        merge_id: None,
        steps,
    });
    code.address_table = address_table(&code, &program)?;
    code.association_table = placed_table(
        &code.segments,
        &program,
        "ASSOCTAB_ADDRESS",
        "ASSOCTAB_SIZE",
        |size| (size - 1) / 2,
    )?;
    code.com_object_table = placed_table(&code.segments, &program, "COMMSTAB_ADDRESS", "", |_| 0)?;
    Ok(code)
}

fn step(
    block: &Block<'_>,
    program_id: &str,
    segments: &mut Vec<AbsoluteSegment>,
) -> Result<LoadStep, LegacyCodeError> {
    let code: u8 = block.number("CONTROL_CODE")?;
    let record = block.record(code)?;
    if let Some(merge) = block.get("MERGE_ID") {
        return Ok(block.unmodelled(&format!("of merged procedure {merge}")));
    }
    if block.get("PROC_MASK").is_some_and(|mask| mask != "0") {
        return Ok(block.unmodelled("with a PROC_MASK"));
    }
    let lsm = code >> 4;
    let event = code & 0x0F;
    Ok(match (code, lsm, event) {
        (CONNECT, ..) => LoadStep::Connect,
        (DISCONNECT, ..) => LoadStep::Disconnect,
        (RESTART, ..) => LoadStep::Restart,
        (COMPARE_PROPERTY, ..) => match record {
            Some(record) => LoadStep::CompareProp {
                object_index: record[1],
                property_id: record[2],
                data: record[6..].to_vec(),
            },
            None => block.unmodelled("Compare Property without a Record"),
        },
        (_, 1.., EVENT_START_LOADING) => LoadStep::Load { lsm },
        (_, 1.., EVENT_LOAD_COMPLETED) => LoadStep::LoadCompleted { lsm },
        (_, 1.., EVENT_UNLOAD) => LoadStep::Unload { lsm },
        (_, 1.., EVENT_ADDITIONAL) => additional(block, lsm, record, program_id, segments)?,
        _ => block.unmodelled(&format!("control code {code:02X}h")),
    })
}

/// An additional load control: a segment allocation or a task segment.
fn additional(
    block: &Block<'_>,
    lsm: u8,
    record: Option<[u8; RECORD_OCTETS]>,
    program_id: &str,
    segments: &mut Vec<AbsoluteSegment>,
) -> Result<LoadStep, LegacyCodeError> {
    let segment_type: u8 = block.number("SEGMENT_TYPE")?;
    if let Some(record) = record {
        if record[1] != segment_type {
            return Err(block.error(format!(
                "Record names segment type {}, SEGMENT_TYPE is {segment_type}",
                record[1]
            )));
        }
    }
    match segment_type {
        SEGMENT_DATA | SEGMENT_STACK => {
            let address: u16 = block.number("SEGMENT_ADDRESS")?;
            let size: u16 = block.number("SEGMENT_LENGTH")?;
            let access: u8 = block.number("ACCESS_ATTRIBUTES")?;
            let memory_type: u8 = block.number("MEMORY_TYPE")?;
            let flags: u8 = block.number("MEMORY_ATTRIBUTES")?;
            let end = address
                .checked_add(size.wrapping_sub(1))
                .filter(|_| size > 0)
                .ok_or_else(|| block.error(format!("a segment of {size} octets at {address}")))?;
            if let Some(record) = record {
                let stated = (
                    word(&record, 3),
                    word(&record, 5),
                    record[7],
                    record[8],
                    record[9],
                );
                if stated != (address, end, access, memory_type, flags) {
                    return Err(block.error(
                        "Record and the SEGMENT_*, ACCESS_ATTRIBUTES and MEMORY_* columns disagree",
                    ));
                }
            }
            let id = format!("{program_id}_AS-{address:04X}");
            if segments.iter().any(|segment| segment.id == id) {
                return Err(block.error(format!("a second segment at {address:04X}h")));
            }
            segments.push(AbsoluteSegment {
                id,
                address: u32::from(address),
                size: u32::from(size),
                data: sized(block, "BLOCK_DATA", size)?,
                mask: sized(block, "BLOCK_MASK", size)?
                    .map(|mask| normalised_mask(block, mask))
                    .transpose()?,
                other: BTreeMap::new(),
            });
            Ok(LoadStep::AbsSegment {
                lsm,
                segment_type,
                address,
                size,
                access,
                memory_type,
                flags,
            })
        }
        SEGMENT_TASK => {
            let address: u16 = block.number("SEGMENT_ADDRESS")?;
            if record.is_some_and(|record| word(&record, 3) != address) {
                return Err(block.error("Record and SEGMENT_ADDRESS disagree"));
            }
            Ok(LoadStep::TaskSegment { lsm, address })
        }
        // The real rows leave SEGMENT_ADDRESS empty here; only the record
        // holds the interface objects' address and count.
        TASK_CONTROL_1 => Ok(match record {
            Some(record) => LoadStep::TaskCtrl1 {
                lsm,
                address: word(&record, 3),
                count: record[5],
            },
            None => block.unmodelled("TaskCtrl1 without a Record"),
        }),
        other => Ok(block.unmodelled(&format!("segment type {other}"))),
    }
}

fn sized(block: &Block<'_>, column: &str, size: u16) -> Result<Option<Vec<u8>>, LegacyCodeError> {
    let octets = block.hex(column)?;
    match &octets {
        Some(octets) if octets.len() != usize::from(size) => Err(block.error(format!(
            "{column} has {} octets, SEGMENT_LENGTH is {size}",
            octets.len()
        ))),
        _ => Ok(octets),
    }
}

/// `01h` and `FFh` both mean "written" (see the module documentation).
fn normalised_mask(block: &Block<'_>, mask: Vec<u8>) -> Result<Vec<u8>, LegacyCodeError> {
    mask.into_iter()
        .map(|octet| match octet {
            0x00 => Ok(0x00),
            0x01 | 0xFF => Ok(0xFF),
            other => Err(block.error(format!("BLOCK_MASK holds {other:02X}h"))),
        })
        .collect()
}

fn program_row(
    document: &ExImDocument,
    exim_program_id: &str,
) -> Result<BTreeMap<String, String>, LegacyCodeError> {
    let unknown = || LegacyCodeError::UnknownProgram(exim_program_id.to_string());
    let table = document.table("application_program").ok_or_else(unknown)?;
    let row = (0..table.row_count())
        .find(|&row| {
            table
                .text_by_name(row, "PROGRAM_ID")
                .is_some_and(|id| id.trim() == exim_program_id)
        })
        .ok_or_else(unknown)?;
    Ok(table
        .columns()
        .iter()
        .enumerate()
        .map(|(index, column)| {
            (
                column.name.clone(),
                table.text(row, index).trim().to_string(),
            )
        })
        .filter(|(_, value)| !value.is_empty())
        .collect())
}

fn table_number(
    program: &BTreeMap<String, String>,
    column: &str,
) -> Result<Option<u32>, LegacyCodeError> {
    program
        .get(column)
        .map(|text| {
            text.parse()
                .map_err(|_| LegacyCodeError::Program(format!("{column} {text:?} is not a number")))
        })
        .transpose()
}

/// The address table: the first data segment of load state machine 1.
fn address_table(
    code: &LegacyProgramCode,
    program: &BTreeMap<String, String>,
) -> Result<Option<TablePlacement>, LegacyCodeError> {
    let mut steps = code.load_procedure.iter().flat_map(|p| &p.steps);
    let Some(address) = steps.find_map(|step| match step {
        LoadStep::AbsSegment {
            lsm: ADDRESS_TABLE_LSM,
            segment_type: SEGMENT_DATA,
            address,
            ..
        } => Some(u32::from(*address)),
        _ => None,
    }) else {
        return Ok(None);
    };
    let size = table_number(program, "ADDRESS_TAB_SIZE")?;
    Ok(
        placement(&code.segments, address).map(|(segment, offset)| TablePlacement {
            code_segment: Some(segment),
            offset: Some(offset),
            max_entries: size
                .filter(|&size| size >= 3)
                .map(|size| (size - 1) / 2 - 1),
        }),
    )
}

fn placed_table(
    segments: &[AbsoluteSegment],
    program: &BTreeMap<String, String>,
    address_column: &str,
    size_column: &str,
    entries: fn(u32) -> u32,
) -> Result<Option<TablePlacement>, LegacyCodeError> {
    let Some(address) = table_number(program, address_column)? else {
        return Ok(None);
    };
    let size = if size_column.is_empty() {
        None
    } else {
        table_number(program, size_column)?
    };
    Ok(
        placement(segments, address).map(|(segment, offset)| TablePlacement {
            code_segment: Some(segment),
            offset: Some(offset),
            max_entries: size.filter(|&size| size >= 1).map(entries),
        }),
    )
}

/// `[V]` ETS's conversion of every `070nh` program in the corpus states
/// `ProductProcedure`: the program's own `s19_block` rows are its procedure.
const PRODUCT_PROCEDURE: &str = "ProductProcedure";

/// The suffix of the one segment id the legacy import places parameters
/// in, with absolute addresses as offsets (ADR-0094, L2).
const ABSOLUTE_SEGMENT: &str = "_AS-0000";

/// A legacy program's [`ProgramCode`]: the payload's `s19_block` rows read
/// by [`legacy_program_code`], and the program's attributes and parameter
/// placements from the database.
///
/// The payload is parsed on every call. For the measured `.vd5` (173 MB)
/// that takes about two seconds and 500 MiB (KNOWN_LIMITATIONS §128).
pub fn load_legacy_program_code(
    conn: &Connection,
    program_id: &str,
    payload_sha256: &str,
    exim_program_id: &str,
) -> Result<ProgramCode, CodeError> {
    let legacy = |cause: String| CodeError::Legacy {
        program_id: program_id.to_string(),
        cause,
    };
    let Some(payload) = crate::load_source_file(conn, payload_sha256)? else {
        return Err(CodeError::MissingSource {
            program_id: program_id.to_string(),
            sha256: payload_sha256.to_string(),
        });
    };
    let document = parse_exim(&payload).map_err(|error| legacy(error.to_string()))?;
    let code = legacy_program_code(&document, exim_program_id, program_id)
        .map_err(|error| legacy(error.to_string()))?;
    drop(document);

    let row: Option<[Option<String>; 4]> = conn
        .query_row(
            "SELECT mask_version, application_number, application_version, pei_type
             FROM application_program WHERE id = ?1",
            [program_id],
            |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?]),
        )
        .optional()?;
    let [mask_version, number, version, pei_type] =
        row.ok_or_else(|| legacy("no application_program row".to_string()))?;
    let mut program_attributes = BTreeMap::new();
    for (name, value) in [
        ("MaskVersion", &mask_version),
        ("ApplicationNumber", &number),
        ("ApplicationVersion", &version),
        ("PeiType", &pei_type),
    ] {
        if let Some(value) = value {
            program_attributes.insert(name.to_string(), value.clone());
        }
    }
    let parameters = parameter_placements(conn, program_id, &code.segments)?;
    Ok(ProgramCode {
        program_id: program_id.to_string(),
        load_procedure_style: code
            .load_procedure
            .as_ref()
            .map(|_| PRODUCT_PROCEDURE.to_string()),
        mask_version,
        segments: code.segments,
        address_table: code.address_table,
        association_table: code.association_table,
        com_object_table: code.com_object_table,
        load_procedures: code.load_procedure.into_iter().collect(),
        options: BTreeMap::new(),
        parameters,
        program_attributes,
    })
}

/// Every placed parameter, moved from the import's absolute addresses into
/// the segment that holds it. A union's members share one placement, at
/// the lowest member address, and keep their own offsets from it, so the
/// image builder sees the union as it sees an XML one.
fn parameter_placements(
    conn: &Connection,
    program_id: &str,
    segments: &[AbsoluteSegment],
) -> Result<BTreeMap<String, ParameterPlacement>, CodeError> {
    let mut statement = conn.prepare(
        "SELECT id, code_segment, offset, bit_offset, union_id FROM parameter
         WHERE program_id = ?1 AND code_segment IS NOT NULL AND offset IS NOT NULL",
    )?;
    let rows = statement
        .query_map([program_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut union_starts: BTreeMap<i64, i64> = BTreeMap::new();
    for (_, _, offset, _, union) in &rows {
        if let Some(union) = union {
            let start = union_starts.entry(*union).or_insert(*offset);
            *start = (*start).min(*offset);
        }
    }
    let memory = |address: i64, bit_offset: u8| {
        u32::try_from(address)
            .ok()
            .and_then(|address| placement(segments, address))
            .map(|(code_segment, offset)| MemoryPlacement {
                code_segment,
                offset,
                bit_offset,
            })
    };
    let mut placements = BTreeMap::new();
    for (id, segment, offset, bit_offset, union) in rows {
        let bit = bit_offset
            .and_then(|bit| u8::try_from(bit).ok())
            .filter(|bit| *bit < 8);
        let unplaced = |what: &str| {
            let mut attributes = BTreeMap::new();
            attributes.insert("CodeSegment".to_string(), segment.clone());
            attributes.insert("Offset".to_string(), offset.to_string());
            if let Some(bit) = bit_offset {
                attributes.insert("BitOffset".to_string(), bit.to_string());
            }
            ParameterPlacement::Unmodelled {
                name: format!("a legacy address {what}"),
                attributes,
            }
        };
        let placed = match (segment.ends_with(ABSOLUTE_SEGMENT), bit, union) {
            (false, ..) => unplaced("in an unknown segment"),
            (true, None, _) => unplaced("with a bit offset outside 0-7"),
            (true, Some(bit), None) => memory(offset, bit)
                .map(ParameterPlacement::Memory)
                .unwrap_or_else(|| unplaced("outside every segment")),
            (true, Some(bit), Some(union)) => {
                let start = union_starts[&union];
                match (memory(start, 0), u32::try_from(offset - start)) {
                    (Some(union), Ok(member)) => ParameterPlacement::UnionMember {
                        union,
                        offset: member,
                        bit_offset: bit,
                    },
                    _ => unplaced("outside every segment"),
                }
            }
        };
        placements.insert(id, placed);
    }
    Ok(placements)
}

/// The segment holding `address`, and the offset into it.
pub(crate) fn placement(segments: &[AbsoluteSegment], address: u32) -> Option<(String, u32)> {
    segments
        .iter()
        .find(|segment| (segment.address..segment.address + segment.size).contains(&address))
        .map(|segment| (segment.id.clone(), address - segment.address))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::exim::parse_exim;

    const PROGRAM: &str = "M-0001_A-LX00000000-7";

    /// One `s19_block` row of program 7, by column.
    #[derive(Clone, Copy, Default)]
    struct Row {
        code: &'static str,
        segment_type: &'static str,
        address: &'static str,
        length: &'static str,
        access: &'static str,
        memory_type: &'static str,
        attributes: &'static str,
        data: &'static str,
        mask: &'static str,
        record: &'static str,
        merge: &'static str,
    }

    fn event(code: &'static str, record: &'static str) -> Row {
        Row {
            code,
            record,
            ..Row::default()
        }
    }

    /// A nine-octet data segment of machine 1 at 4000h.
    fn data_segment() -> Row {
        Row {
            code: "19",
            segment_type: "0",
            address: "16384",
            length: "9",
            access: "255",
            memory_type: "3",
            attributes: "128",
            data: "040000100110021003",
            mask: "FF0001FFFFFFFFFFFF",
            record: "13000040004008ff0380000000000000",
            merge: "",
        }
    }

    fn procedure() -> Vec<Row> {
        vec![
            event("14", "0e000000000000000000000000000000"),
            event("7", "07004e00010100000000000300000000"),
            event("20", "14000000000000000000000000000000"),
            event("17", "11000000000000000000000000000000"),
            data_segment(),
            Row {
                code: "19",
                segment_type: "2",
                address: "16384",
                record: "13020040000000000000000000000000",
                ..Row::default()
            },
            event("18", "12000000000000000000000000000000"),
            event("12", "0c000000000000000000000000000000"),
            event("15", "0f000000000000000000000000000000"),
        ]
    }

    fn payload(rows: &[Row], record: bool) -> Vec<u8> {
        let mut text = String::from(
            "EX-IM\r\nN C:\\synthetic\\ets.vd_\r\nK ETS3\r\nK \r\nD 2026-10-09 09:00:00\r\nV 6.3\r\n\
             H virtual_device\r\n-------------------------------------\r\n\
             T 11 application_program\r\n\
             C1 T11 1 4 N PROGRAM_ID\r\nC2 T11 2 2 Y ADDRESS_TAB_SIZE\r\n\
             C3 T11 1 4 Y ASSOCTAB_ADDRESS\r\nC4 T11 2 2 Y ASSOCTAB_SIZE\r\n\
             C5 T11 1 4 Y COMMSTAB_ADDRESS\r\n\
             R 1 T 11 application_program\r\n7\r\n9\r\n16388\r\n5\r\n16391\r\n\
             -------------------------------------\r\nT 22 s19_block\r\n\
             C1 T22 1 4 N BLOCK_ID\r\nC2 T22 1 4 Y BLOCK_NUMBER\r\nC3 T22 1 4 Y PROGRAM_ID\r\n\
             C4 T22 2 2 Y CONTROL_CODE\r\nC5 T22 2 2 Y SEGMENT_TYPE\r\n\
             C6 T22 1 4 Y SEGMENT_ADDRESS\r\nC7 T22 1 4 Y SEGMENT_LENGTH\r\n\
             C8 T22 2 2 Y ACCESS_ATTRIBUTES\r\nC9 T22 2 2 Y MEMORY_TYPE\r\n\
             C10 T22 2 2 Y MEMORY_ATTRIBUTES\r\nC11 T22 8 32767 Y BLOCK_DATA\r\n\
             C12 T22 8 32767 Y BLOCK_MASK\r\nC13 T22 1 4 Y MERGE_ID\r\n",
        );
        if record {
            text.push_str("C14 T22 8 16 Y Record\r\n");
        }
        // Stored in reverse, so that BLOCK_NUMBER and not file order decides.
        for (line, (i, row)) in rows.iter().enumerate().rev().enumerate() {
            text.push_str(&format!(
                "R {} T 22 s19_block\r\n{}\r\n{}\r\n7\r\n",
                line + 1,
                100 + i,
                i + 1
            ));
            for value in [
                row.code,
                row.segment_type,
                row.address,
                row.length,
                row.access,
                row.memory_type,
                row.attributes,
                row.data,
                row.mask,
                row.merge,
            ] {
                text.push_str(value);
                text.push_str("\r\n");
            }
            if record {
                text.push_str(row.record);
                text.push_str("\r\n");
            }
        }
        text.push_str("XXX\r\n");
        text.into_bytes()
    }

    fn code_of(rows: &[Row], record: bool) -> Result<LegacyProgramCode, LegacyCodeError> {
        let document = parse_exim(&payload(rows, record)).expect("a valid payload");
        legacy_program_code(&document, "7", PROGRAM)
    }

    fn steps(code: &LegacyProgramCode) -> &[LoadStep] {
        &code.load_procedure.as_ref().expect("a procedure").steps
    }

    #[test]
    fn a_procedure_reads_as_the_steps_ets_converts_it_to() {
        let code = code_of(&procedure(), true).unwrap();
        assert_eq!(
            steps(&code),
            [
                LoadStep::Connect,
                LoadStep::CompareProp {
                    object_index: 0,
                    property_id: 78,
                    data: vec![0, 0, 0, 0, 0, 3, 0, 0, 0, 0],
                },
                LoadStep::Unload { lsm: 1 },
                LoadStep::Load { lsm: 1 },
                LoadStep::AbsSegment {
                    lsm: 1,
                    segment_type: 0,
                    address: 0x4000,
                    size: 9,
                    access: 0xFF,
                    memory_type: 3,
                    flags: 0x80,
                },
                LoadStep::TaskSegment {
                    lsm: 1,
                    address: 0x4000,
                },
                LoadStep::LoadCompleted { lsm: 1 },
                LoadStep::Restart,
                LoadStep::Disconnect,
            ]
        );
        let segment = &code.segments[0];
        assert_eq!(segment.id, format!("{PROGRAM}_AS-4000"));
        assert_eq!((segment.address, segment.size), (0x4000, 9));
        assert_eq!(
            segment.data.as_deref(),
            Some(&[4, 0, 0, 0x10, 0x01, 0x10, 0x02, 0x10, 0x03][..])
        );
        // 01h reads as FFh, the octet ETS's conversion writes.
        assert_eq!(
            segment.mask.as_deref(),
            Some(&[0xFF, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF][..])
        );
    }

    #[test]
    fn the_tables_are_placed_and_bounded_from_the_program_row() {
        let code = code_of(&procedure(), true).unwrap();
        let at = |offset, max_entries| {
            Some(TablePlacement {
                code_segment: Some(format!("{PROGRAM}_AS-4000")),
                offset: Some(offset),
                max_entries,
            })
        };
        // ADDRESS_TAB_SIZE 9: a length octet, the device's address, three
        // group addresses. ASSOCTAB_SIZE 5: a length octet, two entries.
        assert_eq!(code.address_table, at(0, Some(3)));
        assert_eq!(code.association_table, at(4, Some(2)));
        assert_eq!(code.com_object_table, at(7, None));
    }

    #[test]
    fn without_a_record_column_the_columns_still_read_but_compare_property_does_not() {
        let code = code_of(&procedure(), false).unwrap();
        let with_record = code_of(&procedure(), true).unwrap();
        assert_eq!(code.segments, with_record.segments);
        for (i, (ours, theirs)) in steps(&code).iter().zip(steps(&with_record)).enumerate() {
            if i == 1 {
                assert!(
                    matches!(ours, LoadStep::Unmodelled { name, .. } if name.contains("Compare Property")),
                    "{ours:?}"
                );
            } else {
                assert_eq!(ours, theirs);
            }
        }
    }

    #[test]
    fn a_record_that_disagrees_with_its_row_is_refused_by_block() {
        let mut rows = procedure();
        // The record states end 4009h, the columns a 9-octet segment (end 4008h).
        rows[4].record = "13000040004009ff0380000000000000";
        let error = code_of(&rows, true).unwrap_err();
        assert!(
            matches!(&error, LegacyCodeError::Block { block, .. } if block == "104"),
            "{error}"
        );

        let mut rows = procedure();
        rows[3].record = "21000000000000000000000000000000";
        assert!(code_of(&rows, true)
            .unwrap_err()
            .to_string()
            .contains("CONTROL_CODE is 11h"));

        let mut rows = procedure();
        rows[5].address = "16385";
        assert!(code_of(&rows, true)
            .unwrap_err()
            .to_string()
            .contains("SEGMENT_ADDRESS disagree"));
    }

    #[test]
    fn data_and_masks_of_the_wrong_length_or_an_unknown_mask_octet_are_refused() {
        let mut rows = procedure();
        rows[4].data = "0400";
        assert!(code_of(&rows, true)
            .unwrap_err()
            .to_string()
            .contains("BLOCK_DATA has 2 octets"));

        let mut rows = procedure();
        rows[4].mask = "FF0002FFFFFFFFFFFF";
        assert!(code_of(&rows, true)
            .unwrap_err()
            .to_string()
            .contains("BLOCK_MASK holds 02h"));
    }

    #[test]
    fn rows_nothing_documents_stay_in_the_procedure_as_unmodelled_steps() {
        let mut rows = procedure();
        rows.insert(2, event("5", "05000800000000000000000000000000"));
        rows[5].merge = "1";
        rows.insert(
            7,
            Row {
                code: "19",
                segment_type: "6",
                record: "13060040000300000000000000000000",
                ..Row::default()
            },
        );
        let code = code_of(&rows, true).unwrap();
        let unmodelled: Vec<_> = steps(&code)
            .iter()
            .filter_map(|step| match step {
                LoadStep::Unmodelled { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            unmodelled,
            [
                "s19_block control code 05h",
                "s19_block of merged procedure 1",
                "s19_block segment type 6",
            ]
        );
        assert_eq!(steps(&code).len(), rows.len());
    }

    #[test]
    fn a_task_control_record_reads_as_mp_lays_it_out() {
        let mut rows = procedure();
        rows.insert(
            6,
            Row {
                code: "19",
                segment_type: "4",
                record: "1304004c6e0300000000000000000000",
                ..Row::default()
            },
        );
        let code = code_of(&rows, true).unwrap();
        assert_eq!(
            steps(&code)[6],
            LoadStep::TaskCtrl1 {
                lsm: 1,
                address: 0x4C6E,
                count: 3
            }
        );
        let code = code_of(&rows, false).unwrap();
        assert!(matches!(
            &steps(&code)[6],
            LoadStep::Unmodelled { name, .. } if name == "s19_block TaskCtrl1 without a Record"
        ));
    }

    #[test]
    fn a_program_without_rows_has_no_procedure_and_an_unknown_one_is_refused() {
        let document = parse_exim(&payload(&procedure(), true)).unwrap();
        assert!(matches!(
            legacy_program_code(&document, "8", PROGRAM),
            Err(LegacyCodeError::UnknownProgram(id)) if id == "8"
        ));
        let code = code_of(&[], true).unwrap();
        assert_eq!(code, LegacyProgramCode::default());
    }

    #[test]
    fn a_second_segment_at_one_address_is_refused() {
        let mut rows = procedure();
        rows.insert(5, data_segment());
        assert!(code_of(&rows, true)
            .unwrap_err()
            .to_string()
            .contains("a second segment at 4000h"));
    }

    #[test]
    fn decode_hex_refuses_odd_and_foreign_digits() {
        assert_eq!(decode_hex("00ff"), Some(vec![0, 255]));
        assert_eq!(decode_hex("0"), None);
        assert_eq!(decode_hex("zz"), None);
    }
}
