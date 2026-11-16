//! Binary exports: declared byte layouts filled from a valid document, with length and CRC.

use crate::expr::{compile, Env, EvalError, Expr, STy, Scope, V};
use crate::json::{fields, Json};
use crate::rules::valid_id;
use crate::schema::{printable, Doc, Schema};

/// Most exports a module may declare.
pub const MAX_EXPORTS: usize = 64;
/// Largest export output a module may declare.
pub const MAX_OUTPUT: usize = 1 << 20;
/// Largest fixed text, zero run or literal a layout item may declare.
pub const MAX_ITEM: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Enc {
    U8,
    U16Le,
    I16Le,
    U32Le,
}

impl Enc {
    fn parse(name: &str) -> Option<Enc> {
        Some(match name {
            "u8" => Enc::U8,
            "u16le" => Enc::U16Le,
            "i16le" => Enc::I16Le,
            "u32le" => Enc::U32Le,
            _ => return None,
        })
    }

    fn range(self) -> (i64, i64) {
        match self {
            Enc::U8 => (0, 0xff),
            Enc::U16Le => (0, 0xffff),
            Enc::I16Le => (-0x8000, 0x7fff),
            Enc::U32Le => (0, 0xffff_ffff),
        }
    }

    fn write(self, out: &mut [u8], value: i64) {
        let bytes = value.to_le_bytes();
        let n = match self {
            Enc::U8 => 1,
            Enc::U16Le | Enc::I16Le => 2,
            Enc::U32Le => 4,
        };
        out[..n].copy_from_slice(&bytes[..n]);
    }

    fn size(self) -> usize {
        match self {
            Enc::U8 => 1,
            Enc::U16Le | Enc::I16Le => 2,
            Enc::U32Le => 4,
        }
    }
}

#[derive(Debug)]
pub enum Item {
    Ascii(Vec<u8>),
    Int { enc: Enc, value: Expr },
    Text { value: Expr, size: usize, nul: bool },
    Zero(usize),
    Rows { table: usize, items: Vec<Item> },
    TotalLength(Enc),
    Crc,
}

/// A compiled export.
#[derive(Debug)]
pub struct Export {
    pub id: String,
    pub title: String,
    pub extension: String,
    pub max_bytes: usize,
    pub items: Vec<Item>,
}

struct Placeholders {
    length: usize,
    crc: usize,
}

fn load_items(
    json: &Json,
    schema: &Schema,
    table: Option<usize>,
    path: &str,
    marks: &mut Placeholders,
) -> Result<Vec<Item>, String> {
    let Json::Array(raw) = json else {
        return Err(format!("{path}: expected an array of layout items"));
    };
    let mut items = Vec::new();
    for (i, item) in raw.iter().enumerate() {
        let at = format!("{path}/{i}");
        let Json::Object(members) = item else {
            return Err(format!("{at}: expected a layout item object"));
        };
        let form = [
            "ascii",
            "u8",
            "u16le",
            "i16le",
            "u32le",
            "text",
            "zero",
            "rows",
            "total_length",
            "crc",
        ]
        .into_iter()
        .find(|k| members.iter().any(|(n, _)| n == k))
        .ok_or_else(|| format!("{at}: unknown layout item"))?;
        let mut scope = Scope::new(schema, table);
        items.push(match form {
            "ascii" => {
                let f = fields(item, &at, &["ascii"], &[])?;
                let text = f.str("ascii")?;
                if text.is_empty()
                    || text.len() > MAX_ITEM
                    || !text.bytes().all(|b| (0x20..=0x7e).contains(&b))
                {
                    return Err(format!(
                        "{at}: ascii must be 1..{MAX_ITEM} printable ASCII characters"
                    ));
                }
                Item::Ascii(text.as_bytes().to_vec())
            }
            "u8" | "u16le" | "i16le" | "u32le" => {
                let f = fields(item, &at, &[form], &[])?;
                let (value, sty) = compile(f.req(form), &mut scope, 0, &f.at(form))?;
                if !matches!(sty, STy::Int) {
                    return Err(format!("{}: an integer expression is required", f.at(form)));
                }
                Item::Int {
                    enc: Enc::parse(form).expect("listed encoding"),
                    value,
                }
            }
            "text" => {
                let f = fields(item, &at, &["text", "size", "pad"], &[])?;
                let (value, sty) = compile(f.req("text"), &mut scope, 0, &f.at("text"))?;
                if !matches!(sty, STy::Str(_)) {
                    return Err(format!("{}: a text expression is required", f.at("text")));
                }
                let size = usize::try_from(f.int("size", 1, MAX_ITEM as i64)?).expect("bounded");
                let nul = match f.str("pad")? {
                    "nul-terminated" => true,
                    "zero" => false,
                    other => {
                        return Err(format!("{}: unsupported padding \"{other}\"", f.at("pad")))
                    }
                };
                Item::Text { value, size, nul }
            }
            "zero" => {
                let f = fields(item, &at, &["zero"], &[])?;
                Item::Zero(usize::try_from(f.int("zero", 1, MAX_ITEM as i64)?).expect("bounded"))
            }
            "rows" => {
                let f = fields(item, &at, &["rows", "layout"], &[])?;
                if table.is_some() {
                    return Err(format!("{at}: \"rows\" items cannot be nested"));
                }
                let name = f.str("rows")?;
                let t = schema
                    .table(name)
                    .ok_or_else(|| format!("{}: no table \"{name}\"", f.at("rows")))?;
                Item::Rows {
                    table: t,
                    items: load_items(f.req("layout"), schema, Some(t), &f.at("layout"), marks)?,
                }
            }
            "total_length" => {
                let f = fields(item, &at, &["total_length"], &[])?;
                let enc = match f.str("total_length")? {
                    "u16le" => Enc::U16Le,
                    "u32le" => Enc::U32Le,
                    other => {
                        return Err(format!(
                            "{}: unsupported length encoding \"{other}\"",
                            f.at("total_length")
                        ))
                    }
                };
                marks.length += 1;
                if table.is_some() || marks.length > 1 {
                    return Err(format!(
                        "{at}: one total_length outside \"rows\" is allowed"
                    ));
                }
                Item::TotalLength(enc)
            }
            _ => {
                let f = fields(item, &at, &["crc", "encoding"], &[])?;
                if f.str("crc")? != "crc16-ccitt-false" || f.str("encoding")? != "u16le" {
                    return Err(format!(
                        "{at}: only crc16-ccitt-false as u16le is supported"
                    ));
                }
                marks.crc += 1;
                if table.is_some() || marks.crc > 1 {
                    return Err(format!("{at}: one crc outside \"rows\" is allowed"));
                }
                Item::Crc
            }
        });
    }
    Ok(items)
}

/// Loads the module's `exports` array.
pub fn load(json: &Json, schema: &Schema, path: &str) -> Result<Vec<Export>, String> {
    let Json::Array(raw) = json else {
        return Err(format!("{path}: expected an array"));
    };
    if raw.len() > MAX_EXPORTS {
        return Err(format!("{path}: at most {MAX_EXPORTS} exports"));
    }
    let mut exports: Vec<Export> = Vec::new();
    for (i, item) in raw.iter().enumerate() {
        let at = format!("{path}/{i}");
        let f = fields(
            item,
            &at,
            &["id", "title", "extension", "max_bytes", "layout"],
            &[],
        )?;
        let id = f.str("id")?;
        if !valid_id(id) || exports.iter().any(|e| e.id == id) {
            return Err(format!(
                "{}: invalid or repeated export id \"{id}\"",
                f.at("id")
            ));
        }
        let title = f.str("title")?;
        if !printable(title, 80) {
            return Err(format!(
                "{}: title must be 1..80 printable characters",
                f.at("title")
            ));
        }
        let extension = f.str("extension")?;
        if extension.is_empty()
            || extension.len() > 16
            || !extension
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Err(format!(
                "{}: extension must be 1..16 of [a-z0-9]",
                f.at("extension")
            ));
        }
        let max_bytes =
            usize::try_from(f.int("max_bytes", 1, MAX_OUTPUT as i64)?).expect("bounded");
        let mut marks = Placeholders { length: 0, crc: 0 };
        let items = load_items(f.req("layout"), schema, None, &f.at("layout"), &mut marks)?;
        exports.push(Export {
            id: id.to_owned(),
            title: title.to_owned(),
            extension: extension.to_owned(),
            max_bytes,
            items,
        });
    }
    Ok(exports)
}

/// Why an export produced no bytes although the document was valid.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// A value does not fit its declared encoding or space.
    Value { path: String, message: String },
    /// Evaluation failed.
    Eval(EvalError),
}

struct Writer<'a> {
    env: Env<'a>,
    out: Vec<u8>,
    max: usize,
    length_at: Option<(usize, Enc)>,
    crc_at: Option<usize>,
}

impl<'a> Writer<'a> {
    fn here(&self) -> String {
        match self.env.current {
            Some((t, r)) => format!("/{}/{r}", self.env.schema.tables[t].name),
            None => String::new(),
        }
    }

    fn refuse(&self, message: String) -> Refusal {
        Refusal::Value {
            path: self.here(),
            message,
        }
    }

    fn push(&mut self, bytes: &[u8]) -> Result<(), Refusal> {
        if self.out.len() + bytes.len() > self.max {
            return Err(self.refuse(format!("output exceeds {} bytes", self.max)));
        }
        self.out.extend_from_slice(bytes);
        Ok(())
    }

    fn items(&mut self, items: &'a [Item]) -> Result<(), Refusal> {
        for item in items {
            match item {
                Item::Ascii(bytes) => self.push(bytes)?,
                Item::Zero(n) => self.push(&vec![0; *n])?,
                Item::Int { enc, value } => {
                    let v = match self.env.eval(value).map_err(Refusal::Eval)? {
                        V::Int(v) => v,
                        _ => return Err(self.refuse("an integer item has no value".into())),
                    };
                    let (lo, hi) = enc.range();
                    if !(lo..=hi).contains(&v) {
                        return Err(self.refuse(format!("{v} does not fit {enc:?} ({lo}..{hi})")));
                    }
                    let mut buf = [0u8; 4];
                    enc.write(&mut buf, v);
                    self.push(&buf[..enc.size()])?;
                }
                Item::Text { value, size, nul } => {
                    let text = match self.env.eval(value).map_err(Refusal::Eval)? {
                        V::Str(s) => s,
                        _ => return Err(self.refuse("a text item has no value".into())),
                    };
                    let room = if *nul { size - 1 } else { *size };
                    if text.len() > room {
                        return Err(
                            self.refuse(format!("text is {} bytes, {room} fit", text.len()))
                        );
                    }
                    let mut buf = vec![0u8; *size];
                    buf[..text.len()].copy_from_slice(text.as_bytes());
                    self.push(&buf)?;
                }
                Item::Rows { table, items } => {
                    for r in 0..self.env.doc.tables[*table].rows.len() {
                        self.env.current = Some((*table, r));
                        self.items(items)?;
                    }
                    self.env.current = None;
                }
                Item::TotalLength(enc) => {
                    self.length_at = Some((self.out.len(), *enc));
                    self.push(&vec![0; enc.size()])?;
                }
                Item::Crc => {
                    self.crc_at = Some(self.out.len());
                    self.push(&[0, 0])?;
                }
            }
        }
        Ok(())
    }
}

/// CRC-16/CCITT-FALSE: polynomial 0x1021, initial 0xFFFF, no reflection, no final xor.
pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xffff;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Produces the bytes of `export` for a valid document.
pub fn run(export: &Export, schema: &Schema, doc: &Doc, limit: u64) -> Result<Vec<u8>, Refusal> {
    let env = Env {
        schema,
        doc,
        current: None,
        vars: Vec::new(),
        steps: 0,
        limit,
    };
    let mut w = Writer {
        env,
        out: Vec::new(),
        max: export.max_bytes,
        length_at: None,
        crc_at: None,
    };
    w.items(&export.items)?;
    if let Some((at, enc)) = w.length_at {
        let len = i64::try_from(w.out.len()).unwrap_or(i64::MAX);
        if len > enc.range().1 {
            return Err(Refusal::Value {
                path: String::new(),
                message: format!("length {len} does not fit {enc:?}"),
            });
        }
        enc.write(&mut w.out[at..], len);
    }
    if let Some(at) = w.crc_at {
        let crc = crc16_ccitt_false(&w.out);
        w.out[at..at + 2].copy_from_slice(&crc.to_le_bytes());
    }
    Ok(w.out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_published_check_value() {
        assert_eq!(crc16_ccitt_false(b"123456789"), 0x29b1);
        assert_eq!(crc16_ccitt_false(b""), 0xffff);
    }

    #[test]
    fn encodings_are_little_endian_twos_complement() {
        let mut buf = [0u8; 4];
        Enc::I16Le.write(&mut buf, -2);
        assert_eq!(&buf[..2], &[0xfe, 0xff]);
        Enc::U32Le.write(&mut buf, 0x0102_0304);
        assert_eq!(buf, [4, 3, 2, 1]);
        assert_eq!(Enc::I16Le.range(), (-32768, 32767));
    }
}
