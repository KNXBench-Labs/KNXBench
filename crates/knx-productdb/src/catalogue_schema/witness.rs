//! Typed full-current-schema streaming witness for one reserved catalogue.
//!
//! Streams a complete, deterministic, lossless digest of one transaction: the
//! schema relation plus every persisted column and hidden rowid of every
//! admitted main table, chunked so no TEXT/BLOB cell becomes a whole Rust
//! `String`/`Vec`. Construction-only — no production route is wired, and
//! equality is exact witness equality, never a commit history. See
//! `docs/contracts/catalogue-jobs/managed-binding.md` §4 and `witness-increment.md`.

use sha2::{Digest, Sha256};

use crate::operation::{PublicationGate, Stop};
use rusqlite::types::{Value, ValueRef};
use rusqlite::Transaction;

use super::{admit, AdmissionError, Record, Relation};

/// Fixed streamed encoding version.
const ENCODING_VERSION: u32 = 1;
/// Maximum admitted byte length of one TEXT/BLOB cell before refusal.
const CELL_BYTE_BOUND: u64 = 1_000_000_000;
/// Chunk size for substr-based TEXT/BLOB streaming.
const CHUNK_SIZE: i64 = 65536;

/// The two witness variants: the live Ready/apply witness and the copy witness.
/// They share the descriptor/table/cell payload and differ only in the header
/// domain and whether the live `schema_version` cookie is included.
#[derive(Clone, Copy)]
enum WitnessKind {
    Live,
    Copy,
}

impl WitnessKind {
    fn domain(self) -> &'static [u8] {
        match self {
            WitnessKind::Live => b"KNXBench/CatalogueWitness/1",
            WitnessKind::Copy => b"KNXBench/CopyContentWitness/1",
        }
    }

    fn includes_schema_version(self) -> bool {
        matches!(self, WitnessKind::Live)
    }
}

/// One complete catalogue witness: a digest plus exact scan accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatalogueWitness {
    pub(crate) encoding_version: u32,
    pub(crate) sha256: [u8; 32],
    pub(crate) table_count: u32,
    pub(crate) schema_object_count: u32,
    pub(crate) row_count: u64,
    pub(crate) encoded_bytes: u64,
}

/// One completed native copy's content witness. Same scan accounting as the
/// live witness but a distinct header domain that omits `schema_version`, so a
/// copy is never accepted as a `CatalogueStamp` or live Ready/apply consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CopyContentWitness {
    encoding_version: u32,
    sha256: [u8; 32],
    table_count: u32,
    schema_object_count: u32,
    row_count: u64,
    encoded_bytes: u64,
}

#[derive(Debug)]
pub(crate) enum WitnessError {
    Admission(AdmissionError),
    Observation(rusqlite::Error),
    Stop(Stop),
    CellTooLarge(u64),
    NegativeLength(i64),
    LengthMissing,
    ChunkNotBlob,
    ChunkLengthMismatch,
    RowMissing,
    UnsupportedCellClass(String),
    NotText,
    Overflow,
}

impl From<rusqlite::Error> for WitnessError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Observation(error)
    }
}

impl From<AdmissionError> for WitnessError {
    fn from(error: AdmissionError) -> Self {
        match error {
            AdmissionError::Stop(stop) => Self::Stop(stop),
            other => Self::Admission(other),
        }
    }
}

impl From<Stop> for WitnessError {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WitnessError::Admission(e) => write!(f, "catalogue witness admission refused: {e:?}"),
            WitnessError::Observation(e) => write!(f, "catalogue witness observation error: {e}"),
            WitnessError::Stop(_) => write!(f, "catalogue witness stopped"),
            WitnessError::CellTooLarge(len) => {
                write!(f, "catalogue witness cell too large: {len} bytes")
            }
            WitnessError::NegativeLength(len) => {
                write!(f, "catalogue witness negative cell length: {len}")
            }
            WitnessError::LengthMissing => write!(f, "catalogue witness cell length missing"),
            WitnessError::ChunkNotBlob => write!(f, "catalogue witness chunk was not a blob"),
            WitnessError::ChunkLengthMismatch => {
                write!(f, "catalogue witness chunk length mismatch")
            }
            WitnessError::RowMissing => write!(f, "catalogue witness row missing"),
            WitnessError::UnsupportedCellClass(class) => {
                write!(f, "catalogue witness unsupported cell class: {class}")
            }
            WitnessError::NotText => write!(f, "catalogue witness expected a text value"),
            WitnessError::Overflow => write!(f, "catalogue witness arithmetic overflow"),
        }
    }
}

impl std::error::Error for WitnessError {}

/// A SHA-256 hasher with a checked external byte counter.
struct Writer {
    hash: Sha256,
    bytes: u64,
}

impl Writer {
    fn new() -> Self {
        Self {
            hash: Sha256::new(),
            bytes: 0,
        }
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<(), WitnessError> {
        self.hash.update(bytes);
        let add = u64::try_from(bytes.len()).map_err(|_| WitnessError::Overflow)?;
        self.bytes = self.bytes.checked_add(add).ok_or(WitnessError::Overflow)?;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), WitnessError> {
        self.raw(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), WitnessError> {
        self.raw(&value.to_be_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), WitnessError> {
        self.raw(&value.to_be_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), WitnessError> {
        self.raw(&value.to_be_bytes())
    }

    fn f64_bits(&mut self, value: f64) -> Result<(), WitnessError> {
        self.raw(&value.to_bits().to_be_bytes())
    }

    fn framed(&mut self, bytes: &[u8]) -> Result<(), WitnessError> {
        let len = u64::try_from(bytes.len()).map_err(|_| WitnessError::Overflow)?;
        self.u64(len)?;
        self.raw(bytes)
    }

    fn finish(self) -> [u8; 32] {
        self.hash.finalize().into()
    }
}

fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn value_text(value: &Value) -> Result<&str, WitnessError> {
    match value {
        Value::Text(text) => Ok(text.as_str()),
        _ => Err(WitnessError::NotText),
    }
}

/// Encodes one whole (already bounded) cell, used for retained descriptor
/// values only. Table data cells use the chunked path instead.
fn encode_cell(writer: &mut Writer, value: ValueRef<'_>) -> Result<(), WitnessError> {
    match value {
        ValueRef::Null => writer.u8(0x00),
        ValueRef::Integer(int) => {
            writer.u8(0x01)?;
            writer.i64(int)
        }
        ValueRef::Real(real) => {
            writer.u8(0x02)?;
            writer.f64_bits(real)
        }
        ValueRef::Text(bytes) => {
            writer.u8(0x03)?;
            writer.framed(bytes)
        }
        ValueRef::Blob(bytes) => {
            writer.u8(0x04)?;
            writer.framed(bytes)
        }
    }
}

fn encode_tuple<const N: usize>(
    writer: &mut Writer,
    record: &Record<N>,
) -> Result<(), WitnessError> {
    writer.u32(N as u32)?;
    for value in &record.0 {
        encode_cell(writer, ValueRef::from(value))?;
    }
    Ok(())
}

fn encode_descriptor(writer: &mut Writer, relation: &Relation) -> Result<(), WitnessError> {
    writer.u8(0x11)?;
    let name = value_text(&relation.table.0[1])?;
    writer.framed(name.as_bytes())?;
    encode_tuple::<6>(writer, &relation.table)?;
    writer.u32(u32::try_from(relation.columns.len()).map_err(|_| WitnessError::Overflow)?)?;
    for column in &relation.columns {
        encode_tuple::<7>(writer, column)?;
    }
    writer.u32(u32::try_from(relation.indexes.len()).map_err(|_| WitnessError::Overflow)?)?;
    for index in &relation.indexes {
        encode_tuple::<5>(writer, &index.list)?;
        writer.u32(u32::try_from(index.xinfo.len()).map_err(|_| WitnessError::Overflow)?)?;
        for xinfo in &index.xinfo {
            encode_tuple::<6>(writer, xinfo)?;
        }
    }
    writer.u32(u32::try_from(relation.foreign_keys.len()).map_err(|_| WitnessError::Overflow)?)?;
    for foreign_key in &relation.foreign_keys {
        encode_tuple::<8>(writer, foreign_key)?;
    }
    Ok(())
}

fn encode_header(
    writer: &mut Writer,
    tx: &Transaction,
    gate: &PublicationGate,
    kind: WitnessKind,
) -> Result<(), WitnessError> {
    gate.checkpoint()?;
    writer.u8(0x10)?;
    writer.raw(kind.domain())?;
    writer.u8(0x00)?;
    writer.u32(ENCODING_VERSION)?;
    let user_version: i64 = tx.query_row("PRAGMA main.user_version", [], |r| r.get(0))?;
    let application_id: i64 = tx.query_row("PRAGMA main.application_id", [], |r| r.get(0))?;
    let encoding: String = tx.query_row("PRAGMA main.encoding", [], |r| r.get(0))?;
    writer.i64(user_version)?;
    if kind.includes_schema_version() {
        let schema_version: i64 = tx.query_row("PRAGMA main.schema_version", [], |r| r.get(0))?;
        writer.i64(schema_version)?;
    }
    writer.i64(application_id)?;
    writer.framed(encoding.as_bytes())?;
    gate.checkpoint()?;
    Ok(())
}

/// Streams one relation's rows by `_rowid_`, returning the row count. The
/// built-in `sqlite_schema` relation is streamed here like the other 46 tables.
fn stream_table(
    writer: &mut Writer,
    tx: &Transaction,
    table: &str,
    columns: &[&str],
    gate: &PublicationGate,
) -> Result<u64, WitnessError> {
    let table_q = quote_identifier(table);
    let column_qs: Vec<String> = columns.iter().map(|c| quote_identifier(c)).collect();

    writer.u8(0x12)?;
    writer.framed(table.as_bytes())?;
    writer.u32(u32::try_from(columns.len()).map_err(|_| WitnessError::Overflow)?)?;

    let id_sql = format!("SELECT _rowid_ FROM main.{table_q} ORDER BY _rowid_ ASC");
    let mut id_stmt = tx.prepare(&id_sql)?;
    let mut id_rows = id_stmt.query([])?;
    let mut row_count: u64 = 0;
    while let Some(row) = id_rows.next()? {
        gate.checkpoint()?;
        let rowid: i64 = row.get(0)?;
        writer.u8(0x13)?;
        writer.i64(rowid)?;
        for column_q in &column_qs {
            stream_data_cell(writer, tx, &table_q, column_q, rowid, gate)?;
        }
        writer.u8(0x14)?;
        row_count = row_count.checked_add(1).ok_or(WitnessError::Overflow)?;
    }
    drop(id_rows);
    drop(id_stmt);

    writer.u8(0x15)?;
    writer.u64(row_count)?;
    Ok(row_count)
}

/// Streams one data cell, chunking TEXT/BLOB through `substr` so no value is
/// materialized whole in Rust.
fn stream_data_cell(
    writer: &mut Writer,
    tx: &Transaction,
    table_q: &str,
    column_q: &str,
    rowid: i64,
    gate: &PublicationGate,
) -> Result<(), WitnessError> {
    gate.checkpoint()?;

    let type_sql = format!(
        "SELECT typeof({column_q}), length(CAST({column_q} AS BLOB)) \
         FROM main.{table_q} WHERE _rowid_=?1"
    );
    let (kind, len) = {
        let mut stmt = tx.prepare(&type_sql)?;
        let mut rows = stmt.query([rowid])?;
        let row = rows.next()?.ok_or(WitnessError::RowMissing)?;
        let kind: String = row.get(0)?;
        let len: Option<i64> = row.get(1)?;
        (kind, len)
    };

    match kind.as_str() {
        "null" => writer.u8(0x00),
        "integer" => {
            let value: i64 = tx.query_row(
                &format!("SELECT {column_q} FROM main.{table_q} WHERE _rowid_=?1"),
                [rowid],
                |r| r.get(0),
            )?;
            writer.u8(0x01)?;
            writer.i64(value)
        }
        "real" => {
            let value: f64 = tx.query_row(
                &format!("SELECT {column_q} FROM main.{table_q} WHERE _rowid_=?1"),
                [rowid],
                |r| r.get(0),
            )?;
            writer.u8(0x02)?;
            writer.f64_bits(value)
        }
        "text" | "blob" => {
            let len_u64 = admitted_cell_length(len)?;
            writer.u8(if kind == "text" { 0x03 } else { 0x04 })?;
            writer.u64(len_u64)?;
            let expr = if kind == "text" {
                format!("CAST({column_q} AS BLOB)")
            } else {
                column_q.to_string()
            };
            let sub_sql =
                format!("SELECT substr({expr}, ?1, ?2) FROM main.{table_q} WHERE _rowid_=?3");
            let mut sub = tx.prepare(&sub_sql)?;
            let mut remaining = len_u64;
            let mut offset: i64 = 1;
            while remaining > 0 {
                gate.checkpoint()?;
                let take = remaining.min(CHUNK_SIZE as u64);
                let take_i64 = i64::try_from(take).map_err(|_| WitnessError::Overflow)?;
                let mut rows = sub.query((offset, take_i64, rowid))?;
                let row = rows.next()?.ok_or(WitnessError::RowMissing)?;
                let ValueRef::Blob(bytes) = row.get_ref(0)? else {
                    return Err(WitnessError::ChunkNotBlob);
                };
                if bytes.len() != take as usize {
                    return Err(WitnessError::ChunkLengthMismatch);
                }
                writer.raw(bytes)?;
                offset = offset.checked_add(take_i64).ok_or(WitnessError::Overflow)?;
                remaining -= take;
            }
            Ok(())
        }
        other => Err(WitnessError::UnsupportedCellClass(other.to_owned())),
    }
}

/// Validates and converts a TEXT/BLOB byte length into the admitted `u64`
/// bound. Helper-only seam; see `admitted_cell_length` tests.
fn admitted_cell_length(len: Option<i64>) -> Result<u64, WitnessError> {
    let len = len.ok_or(WitnessError::LengthMissing)?;
    if len < 0 {
        return Err(WitnessError::NegativeLength(len));
    }
    let len = u64::try_from(len).map_err(|_| WitnessError::Overflow)?;
    if len > CELL_BYTE_BOUND {
        return Err(WitnessError::CellTooLarge(len));
    }
    Ok(len)
}

/// The shared stream outcome, independent of the witness variant.
struct WitnessParts {
    sha256: [u8; 32],
    table_count: u32,
    schema_object_count: u32,
    row_count: u64,
    encoded_bytes: u64,
}

/// Produces the complete witness for one transaction in the given variant.
/// Reuses B1 admission for shape validation and descriptor retention, then
/// streams schema + data. The live and copy variants share the payload and
/// differ only in the header emitted by `encode_header`.
fn stream_witness(
    tx: &Transaction,
    gate: &PublicationGate,
    kind: WitnessKind,
) -> Result<WitnessParts, WitnessError> {
    let admitted = admit(tx, gate)?;

    let mut writer = Writer::new();

    encode_header(&mut writer, tx, gate, kind)?;

    let mut table_count: u32 = 0;
    let mut total_rows: u64 = 0;

    for relation in &admitted.relations {
        encode_descriptor(&mut writer, relation)?;
        let name = value_text(&relation.table.0[1])?.to_owned();
        let mut columns: Vec<&str> = Vec::with_capacity(relation.columns.len());
        for column in &relation.columns {
            columns.push(value_text(&column.0[1])?);
        }
        let rows = stream_table(&mut writer, tx, &name, &columns, gate)?;
        table_count = table_count.checked_add(1).ok_or(WitnessError::Overflow)?;
        total_rows = total_rows.checked_add(rows).ok_or(WitnessError::Overflow)?;
    }

    writer.u8(0x16)?;
    writer.u32(table_count)?;
    writer.u32(u32::try_from(admitted.objects.len()).map_err(|_| WitnessError::Overflow)?)?;
    writer.u64(total_rows)?;

    let encoded_bytes = writer.bytes;
    let sha256 = writer.finish();

    Ok(WitnessParts {
        sha256,
        table_count,
        schema_object_count: u32::try_from(admitted.objects.len())
            .map_err(|_| WitnessError::Overflow)?,
        row_count: total_rows,
        encoded_bytes,
    })
}

/// Produces the live Ready/apply witness for one transaction.
pub(crate) fn witness_current(
    tx: &Transaction,
    gate: &PublicationGate,
) -> Result<CatalogueWitness, WitnessError> {
    let parts = stream_witness(tx, gate, WitnessKind::Live)?;
    Ok(CatalogueWitness {
        encoding_version: ENCODING_VERSION,
        sha256: parts.sha256,
        table_count: parts.table_count,
        schema_object_count: parts.schema_object_count,
        row_count: parts.row_count,
        encoded_bytes: parts.encoded_bytes,
    })
}

/// Produces the copy-content witness for one transaction (omits
/// `schema_version`, so a copied destination's fresh cookie does not affect it).
fn copy_content_witness(
    tx: &Transaction,
    gate: &PublicationGate,
) -> Result<CopyContentWitness, WitnessError> {
    let parts = stream_witness(tx, gate, WitnessKind::Copy)?;
    Ok(CopyContentWitness {
        encoding_version: ENCODING_VERSION,
        sha256: parts.sha256,
        table_count: parts.table_count,
        schema_object_count: parts.schema_object_count,
        row_count: parts.row_count,
        encoded_bytes: parts.encoded_bytes,
    })
}

#[cfg(test)]
mod tests;
