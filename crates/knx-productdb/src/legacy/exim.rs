//! The observed EX-IM line grammar, parsed in one bounded linear pass.
//!
//! ```text
//! payload     = "EX-IM" CRLF header-line* (separator table)* "XXX" CRLF
//! header-line = KEY [SP text] CRLF          ; KEY: one ASCII capital letter
//! separator   = 1*"-" CRLF
//! table       = "T" SP id SP name CRLF column{1,} row*
//! column      = "C" index SP "T" id SP type SP size SP ("Y" / "N") SP name CRLF
//! row         = "R" SP number SP "T" SP id SP name CRLF value{column count}
//! value       = text CRLF *("\\" text CRLF)  ; continuation, prefix stripped
//! ```
//!
//! `[V]` for the measured `.vd3`, `.vd4` and `.pr5` files, `[A]` beyond
//! them. Values are read strictly by column count: a value line may itself
//! look like a separator (`----` occurs) or a record. Continuation lines
//! (`\\` prefix) follow value lines whose first line is exactly 40 or 80
//! bytes long in both measured product databases, which supports reading
//! them as a wrap of one value. A value whose own text starts with `\\`
//! cannot be represented in that reading; it would desynchronise the column
//! count and is refused as a syntax error rather than guessed.
//!
//! Type codes and sizes are kept verbatim and not interpreted; every value
//! is text. Bytes are kept raw; [`ExImTable::text`] decodes them as
//! Windows-1252 (see `text.rs`).

use std::borrow::Cow;

use super::error::LegacyError;
use super::text::{decode_windows_1252, unescape, unknown_escapes, windows_1252_only_bytes};

/// Type codes seen in the measured files: `1 4`, `2 2`, `3 n`, `4 32767`,
/// `5 8`, `6 16`/`6 255`, `8 32767`. Their meaning is undocumented.
const KNOWN_TYPE_CODES: [u32; 7] = [1, 2, 3, 4, 5, 6, 8];
/// Header keys seen in the measured files.
const KNOWN_HEADER_KEYS: [&str; 5] = ["N", "K", "D", "V", "H"];

/// Resource bounds for one payload. The defaults sit 10–60 times above the
/// largest measured file (82-byte lines, 2,508-byte values, 37 tables,
/// 34 columns, 14,734 rows); they bound memory and time and say nothing
/// about the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExImLimits {
    pub max_line_len: usize,
    pub max_value_len: usize,
    pub max_continuations_per_value: usize,
    pub max_tables: usize,
    pub max_columns: usize,
    pub max_rows: usize,
    pub max_values: usize,
    pub max_header_lines: usize,
}

impl Default for ExImLimits {
    fn default() -> Self {
        Self {
            max_line_len: 4096,
            max_value_len: 1024 * 1024,
            max_continuations_per_value: 16_384,
            max_tables: 512,
            max_columns: 512,
            max_rows: 2_000_000,
            max_values: 8_000_000,
            max_header_lines: 64,
        }
    }
}

/// What the `H` header line says the file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExImContent {
    /// `H virtual_device`: a product database (`.vd*`).
    ProductDatabase,
    /// `H project`: a project export (`.pr*`).
    ProjectExport,
    /// Any other `H` value, kept verbatim.
    Other(String),
    /// No `H` line at all.
    Unspecified,
}

/// A finding that does not stop parsing but must be reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExImDiagnostic {
    /// A header key outside `N K D V H`.
    UnknownHeaderKey { line: usize, key: String },
    /// A column declared type code outside the observed set.
    UnknownTypeCode {
        table: String,
        column: String,
        type_code: u32,
    },
    /// Empty values in a column declared `N` (not nullable). Whether an
    /// empty value means SQL NULL or an empty string is unknown.
    EmptyRequiredValues {
        table: String,
        column: String,
        count: usize,
    },
    /// Value bytes in `0x80`–`0x9F`, where Windows-1252 and ISO-8859-1
    /// disagree; decoded as Windows-1252.
    Windows1252OnlyBytes { count: usize },
    /// Backslashes in values that start none of the measured escapes
    /// (`\'`, `\r`, `\n`, `\\`); kept verbatim.
    UnknownEscapes { count: usize },
}

/// One declared column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExImColumn {
    /// 1-based position as declared (`C<index>`).
    pub index: u32,
    /// Undocumented type code, verbatim.
    pub type_code: u32,
    /// Undocumented size, verbatim.
    pub size: u32,
    /// `Y` (true) or `N` (false).
    pub nullable: bool,
    pub name: String,
}

/// One table with its rows. Values live in one arena; each is a span.
#[derive(Debug, Clone)]
pub struct ExImTable {
    id: u32,
    name: String,
    columns: Vec<ExImColumn>,
    rows: usize,
    spans: Vec<(u32, u32)>,
    /// Where each value sits in the source, continuation lines included and
    /// the final CRLF excluded; same order as `spans`.
    source_spans: Vec<(u32, u32)>,
    arena: Vec<u8>,
}

impl ExImTable {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn columns(&self) -> &[ExImColumn] {
        &self.columns
    }

    pub fn row_count(&self) -> usize {
        self.rows
    }

    /// The position of the column named `name`, if declared.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    /// The raw bytes of one value, continuations joined.
    ///
    /// # Panics
    /// When `row` or `column` is out of range.
    pub fn raw(&self, row: usize, column: usize) -> &[u8] {
        assert!(row < self.rows && column < self.columns.len());
        let (start, end) = self.spans[row * self.columns.len() + column];
        &self.arena[start as usize..end as usize]
    }

    /// One value decoded as Windows-1252, with the value escapes (`\'`,
    /// `\r`, `\n`, `\\`) resolved. [`ExImTable::raw`] keeps the bytes.
    /// The bytes of this value in the source document: its first line through
    /// its last continuation line, without the final CRLF.
    pub fn source_range(&self, row: usize, column: usize) -> std::ops::Range<usize> {
        let (start, end) = self.source_spans[row * self.columns.len() + column];
        start as usize..end as usize
    }

    pub fn text(&self, row: usize, column: usize) -> Cow<'_, str> {
        unescape(decode_windows_1252(self.raw(row, column)))
    }

    /// One value by column name; `None` when the column is not declared.
    pub fn text_by_name(&self, row: usize, column: &str) -> Option<Cow<'_, str>> {
        self.column_index(column).map(|index| self.text(row, index))
    }
}

/// A parsed payload.
#[derive(Debug, Clone)]
pub struct ExImDocument {
    header: Vec<(String, String)>,
    tables: Vec<ExImTable>,
    diagnostics: Vec<ExImDiagnostic>,
    continuation_lines: usize,
}

impl ExImDocument {
    /// Header lines as `(key, value)`, decoded, in file order.
    pub fn header(&self) -> &[(String, String)] {
        &self.header
    }

    fn header_value(&self, key: &str) -> Option<&str> {
        self.header
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn content(&self) -> ExImContent {
        match self.header_value("H") {
            Some("virtual_device") => ExImContent::ProductDatabase,
            Some("project") => ExImContent::ProjectExport,
            Some(other) => ExImContent::Other(other.to_string()),
            None => ExImContent::Unspecified,
        }
    }

    /// The `V` header line.
    pub fn format_version(&self) -> Option<&str> {
        self.header_value("V")
    }

    /// The `D` header line.
    pub fn exported_at(&self) -> Option<&str> {
        self.header_value("D")
    }

    /// The first `K` header line (observed: `ETS3`).
    pub fn producer(&self) -> Option<&str> {
        self.header_value("K")
    }

    pub fn tables(&self) -> &[ExImTable] {
        &self.tables
    }

    pub fn table(&self, name: &str) -> Option<&ExImTable> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn diagnostics(&self) -> &[ExImDiagnostic] {
        &self.diagnostics
    }

    /// Physical `\\` continuation lines joined into values.
    pub fn continuation_lines(&self) -> usize {
        self.continuation_lines
    }
}

/// [`parse_exim_with_limits`] with [`ExImLimits::default`].
pub fn parse_exim(bytes: &[u8]) -> Result<ExImDocument, LegacyError> {
    parse_exim_with_limits(bytes, ExImLimits::default())
}

/// Parses a decrypted, inflated EX-IM payload completely, or refuses it.
/// Nothing is interpreted beyond the grammar: no table or column name is
/// required, and every value stays text.
pub fn parse_exim_with_limits(
    bytes: &[u8],
    limits: ExImLimits,
) -> Result<ExImDocument, LegacyError> {
    Parser::new(bytes, limits).document()
}

/// CRLF line reader with one line of look-ahead. Refuses bare CR or LF, a
/// final line without CRLF, and any line longer than the limit.
struct Lines<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// 1-based number of the last line returned by `next`.
    line: usize,
    max_line_len: usize,
    peeked: Option<Option<&'a [u8]>>,
}

impl<'a> Lines<'a> {
    fn read(&mut self) -> Result<Option<&'a [u8]>, LegacyError> {
        if self.pos == self.bytes.len() {
            return Ok(None);
        }
        let number = self.line + usize::from(self.peeked.is_some()) + 1;
        let rest = &self.bytes[self.pos..];
        let end = rest
            .iter()
            .position(|&b| b == b'\r' || b == b'\n')
            .ok_or_else(|| syntax(number, "the last line is not terminated by CRLF"))?;
        if rest[end] == b'\n' {
            return Err(syntax(number, "bare LF; lines end with CRLF"));
        }
        if rest.get(end + 1) != Some(&b'\n') {
            return Err(syntax(number, "bare CR; lines end with CRLF"));
        }
        if end > self.max_line_len {
            return Err(syntax(
                number,
                &format!("line exceeds {} bytes", self.max_line_len),
            ));
        }
        self.pos += end + 2;
        Ok(Some(&rest[..end]))
    }

    fn peek(&mut self) -> Result<Option<&'a [u8]>, LegacyError> {
        if self.peeked.is_none() {
            let next = self.read()?;
            self.peeked = Some(next);
        }
        Ok(self.peeked.flatten())
    }

    fn next(&mut self) -> Result<Option<&'a [u8]>, LegacyError> {
        let next = match self.peeked.take() {
            Some(peeked) => peeked,
            None => self.read()?,
        };
        if next.is_some() {
            self.line += 1;
        }
        Ok(next)
    }

    /// Byte offset of `line`, a slice this reader returned, in the source.
    fn offset_of(&self, line: &[u8]) -> usize {
        line.as_ptr() as usize - self.bytes.as_ptr() as usize
    }

    /// The next line, or a syntax error naming what was expected.
    fn expect(&mut self, what: &str) -> Result<&'a [u8], LegacyError> {
        let number = self.line + 1;
        self.next()?
            .ok_or_else(|| syntax(number, &format!("unexpected end of text, expected {what}")))
    }
}

fn syntax(line: usize, reason: &str) -> LegacyError {
    LegacyError::Syntax {
        line,
        reason: reason.to_string(),
    }
}

fn limit(what: &'static str, limit: usize) -> LegacyError {
    LegacyError::SizeLimit {
        what,
        limit: limit as u64,
    }
}

fn is_separator(line: &[u8]) -> bool {
    !line.is_empty() && line.iter().all(|&b| b == b'-')
}

/// A decimal number of ASCII digits only: no sign, no space.
fn number(field: &[u8]) -> Option<u32> {
    if field.is_empty() || !field.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(field).ok()?.parse().ok()
}

/// A table or column name: printable ASCII without spaces.
fn name(field: &[u8]) -> Option<&str> {
    if field.is_empty() || !field.iter().all(|b| b.is_ascii_graphic()) {
        return None;
    }
    std::str::from_utf8(field).ok()
}

struct Parser<'a> {
    lines: Lines<'a>,
    limits: ExImLimits,
    diagnostics: Vec<ExImDiagnostic>,
    rows: usize,
    values: usize,
    continuation_lines: usize,
    high_control_bytes: usize,
    unknown_escapes: usize,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8], limits: ExImLimits) -> Self {
        Self {
            lines: Lines {
                bytes,
                pos: 0,
                line: 0,
                max_line_len: limits.max_line_len,
                peeked: None,
            },
            limits,
            diagnostics: Vec::new(),
            rows: 0,
            values: 0,
            continuation_lines: 0,
            high_control_bytes: 0,
            unknown_escapes: 0,
        }
    }

    fn document(mut self) -> Result<ExImDocument, LegacyError> {
        if self.lines.expect("the EX-IM line")? != b"EX-IM" {
            return Err(syntax(1, "missing EX-IM first line"));
        }
        let (header, mut more_tables) = self.header()?;
        let mut tables: Vec<ExImTable> = Vec::new();
        while more_tables {
            if tables.len() == self.limits.max_tables {
                return Err(limit("table count", self.limits.max_tables));
            }
            let (table, next) = self.table()?;
            if tables
                .iter()
                .any(|t| t.id == table.id || t.name == table.name)
            {
                return Err(syntax(
                    self.lines.line,
                    &format!("duplicate table {} {}", table.id, table.name),
                ));
            }
            tables.push(table);
            more_tables = next;
        }
        if let Some(_extra) = self.lines.next()? {
            return Err(syntax(self.lines.line, "text after the XXX end line"));
        }
        if self.high_control_bytes > 0 {
            self.diagnostics.push(ExImDiagnostic::Windows1252OnlyBytes {
                count: self.high_control_bytes,
            });
        }
        if self.unknown_escapes > 0 {
            self.diagnostics.push(ExImDiagnostic::UnknownEscapes {
                count: self.unknown_escapes,
            });
        }
        Ok(ExImDocument {
            header,
            tables,
            diagnostics: self.diagnostics,
            continuation_lines: self.continuation_lines,
        })
    }

    /// Header lines up to the first separator. Returns whether a table
    /// follows (`false` when `XXX` ends a table-less file).
    fn header(&mut self) -> Result<(Vec<(String, String)>, bool), LegacyError> {
        let mut header = Vec::new();
        loop {
            let line = self.lines.expect("a header line or a separator")?;
            if is_separator(line) {
                return Ok((header, true));
            }
            if line == b"XXX" {
                return Ok((header, false));
            }
            let valid = line.first().is_some_and(u8::is_ascii_uppercase)
                && (line.len() == 1 || line[1] == b' ');
            if !valid {
                return Err(syntax(self.lines.line, "not a header line"));
            }
            if header.len() == self.limits.max_header_lines {
                return Err(limit("header line count", self.limits.max_header_lines));
            }
            let key = char::from(line[0]).to_string();
            if !KNOWN_HEADER_KEYS.contains(&key.as_str()) {
                self.diagnostics.push(ExImDiagnostic::UnknownHeaderKey {
                    line: self.lines.line,
                    key: key.clone(),
                });
            }
            let value = line.get(2..).unwrap_or_default();
            self.count_high_control(value);
            header.push((key, decode_windows_1252(value).into_owned()));
        }
    }

    /// One table after its separator. Returns whether another table follows.
    fn table(&mut self) -> Result<(ExImTable, bool), LegacyError> {
        let line = self.lines.expect("a T table line")?;
        let fields: Vec<&[u8]> = line.split(|&b| b == b' ').collect();
        let (id, table_name) = match fields.as_slice() {
            [b"T", id, table_name] => (number(id), name(table_name)),
            _ => (None, None),
        };
        let (Some(id), Some(table_name)) = (id, table_name) else {
            return Err(syntax(self.lines.line, "expected `T <id> <name>`"));
        };
        let mut table = ExImTable {
            id,
            name: table_name.to_string(),
            columns: Vec::new(),
            rows: 0,
            spans: Vec::new(),
            source_spans: Vec::new(),
            arena: Vec::new(),
        };
        self.columns(&mut table)?;
        let mut empty_required = vec![0usize; table.columns.len()];
        loop {
            let line = self.lines.expect("a row, a separator or XXX")?;
            if is_separator(line) {
                self.report_empty(&table, &empty_required);
                return Ok((table, true));
            }
            if line == b"XXX" {
                self.report_empty(&table, &empty_required);
                return Ok((table, false));
            }
            self.row(line, &mut table, &mut empty_required)?;
        }
    }

    fn columns(&mut self, table: &mut ExImTable) -> Result<(), LegacyError> {
        let tag = format!("T{}", table.id);
        while let Some(line) = self.lines.peek()? {
            if line.first() != Some(&b'C') {
                break;
            }
            self.lines.next()?;
            let fields: Vec<&[u8]> = line.split(|&b| b == b' ').collect();
            let parsed = match fields.as_slice() {
                [index, table_tag, type_code, size, nullable, column]
                    if *table_tag == tag.as_bytes() =>
                {
                    let nullable = match *nullable {
                        b"Y" => Some(true),
                        b"N" => Some(false),
                        _ => None,
                    };
                    (
                        index.strip_prefix(b"C").and_then(number),
                        number(type_code),
                        number(size),
                        nullable,
                        name(column),
                    )
                }
                _ => (None, None, None, None, None),
            };
            let (Some(index), Some(type_code), Some(size), Some(nullable), Some(column)) = parsed
            else {
                return Err(syntax(
                    self.lines.line,
                    &format!("expected `C<n> {tag} <type> <size> <Y|N> <name>`"),
                ));
            };
            if usize::try_from(index).ok() != Some(table.columns.len() + 1) {
                return Err(syntax(self.lines.line, "column index out of sequence"));
            }
            if table.columns.len() == self.limits.max_columns {
                return Err(limit("column count", self.limits.max_columns));
            }
            if !KNOWN_TYPE_CODES.contains(&type_code) {
                self.diagnostics.push(ExImDiagnostic::UnknownTypeCode {
                    table: table.name.clone(),
                    column: column.to_string(),
                    type_code,
                });
            }
            table.columns.push(ExImColumn {
                index,
                type_code,
                size,
                nullable,
                name: column.to_string(),
            });
        }
        if table.columns.is_empty() {
            return Err(syntax(
                self.lines.line + 1,
                "expected a column declaration; a table needs at least one",
            ));
        }
        Ok(())
    }

    fn row(
        &mut self,
        line: &[u8],
        table: &mut ExImTable,
        empty_required: &mut [usize],
    ) -> Result<(), LegacyError> {
        let fields: Vec<&[u8]> = line.split(|&b| b == b' ').collect();
        let matches = match fields.as_slice() {
            [b"R", row, b"T", id, row_table] => {
                number(row).and_then(|n| usize::try_from(n).ok()) == Some(table.rows + 1)
                    && number(id) == Some(table.id)
                    && *row_table == table.name.as_bytes()
            }
            _ => false,
        };
        if !matches {
            return Err(syntax(
                self.lines.line,
                &format!(
                    "expected `R {} T {} {}`, a separator or XXX",
                    table.rows + 1,
                    table.id,
                    table.name
                ),
            ));
        }
        if self.rows == self.limits.max_rows {
            return Err(limit("row count", self.limits.max_rows));
        }
        self.rows += 1;
        table.rows += 1;
        for (column, empty) in empty_required.iter_mut().enumerate() {
            let first = self.lines.expect("a value")?;
            if first.starts_with(b"\\\\") {
                return Err(syntax(
                    self.lines.line,
                    "a value begins with the `\\\\` continuation prefix",
                ));
            }
            if self.values == self.limits.max_values {
                return Err(limit("value count", self.limits.max_values));
            }
            self.values += 1;
            let start = table.arena.len();
            let source_start = self.lines.offset_of(first);
            let mut source_end = source_start + first.len();
            table.arena.extend_from_slice(first);
            let mut continuations = 0usize;
            while let Some(next) = self.lines.peek()? {
                let Some(more) = next.strip_prefix(b"\\\\") else {
                    break;
                };
                self.lines.next()?;
                source_end = self.lines.offset_of(next) + next.len();
                continuations += 1;
                if continuations > self.limits.max_continuations_per_value {
                    return Err(limit(
                        "continuation lines per value",
                        self.limits.max_continuations_per_value,
                    ));
                }
                table.arena.extend_from_slice(more);
                if table.arena.len() - start > self.limits.max_value_len {
                    break;
                }
            }
            self.continuation_lines += continuations;
            let end = table.arena.len();
            if end - start > self.limits.max_value_len {
                return Err(limit("value length", self.limits.max_value_len));
            }
            let value = &table.arena[start..end];
            self.high_control_bytes += windows_1252_only_bytes(value);
            self.unknown_escapes += unknown_escapes(value);
            if value.is_empty() && !table.columns[column].nullable {
                *empty += 1;
            }
            let span = (
                u32::try_from(start).map_err(|_| limit("payload size", u32::MAX as usize))?,
                u32::try_from(end).map_err(|_| limit("payload size", u32::MAX as usize))?,
            );
            table.spans.push(span);
            let source = (
                u32::try_from(source_start)
                    .map_err(|_| limit("payload size", u32::MAX as usize))?,
                u32::try_from(source_end).map_err(|_| limit("payload size", u32::MAX as usize))?,
            );
            table.source_spans.push(source);
        }
        Ok(())
    }

    fn count_high_control(&mut self, bytes: &[u8]) {
        self.high_control_bytes += windows_1252_only_bytes(bytes);
    }

    fn report_empty(&mut self, table: &ExImTable, empty_required: &[usize]) {
        for (column, &count) in table.columns.iter().zip(empty_required) {
            if count > 0 {
                self.diagnostics.push(ExImDiagnostic::EmptyRequiredValues {
                    table: table.name.clone(),
                    column: column.name.clone(),
                    count,
                });
            }
        }
    }
}
