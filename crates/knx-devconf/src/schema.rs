//! Declared document types: loading them from a module and checking a document's structure.

use crate::json::{fields, str_of, Json};
use crate::Finding;
use std::collections::HashMap;
use std::sync::Arc;

/// Most rows a table may declare.
pub const MAX_ROWS: usize = 4096;
/// Most members an object or row may declare.
pub const MAX_MEMBERS: usize = 64;
/// Longest text or array a type may declare.
pub const MAX_LEN: usize = 4096;

/// One named member of the document root or a table row.
#[derive(Debug)]
pub struct Member {
    pub name: String,
    pub ty: Ty,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// A declared value type.
#[derive(Debug)]
pub enum Ty {
    Int {
        min: i64,
        max: i64,
    },
    Enum {
        values: Arc<[String]>,
    },
    Text {
        ascii: bool,
        min: usize,
        max: usize,
    },
    Const(Json),
    Array {
        item: Box<Ty>,
        min: usize,
        max: usize,
    },
    Ref {
        table: usize,
        none: Option<i64>,
    },
    Table(usize),
}

/// A declared table: rows of fixed members, optionally keyed by one integer member.
#[derive(Debug)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Member>,
    pub min_rows: usize,
    pub max_rows: usize,
    pub key: Option<usize>,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// The document type of a module.
#[derive(Debug)]
pub struct Schema {
    pub root: Vec<Member>,
    pub tables: Vec<Table>,
}

impl Schema {
    pub fn table(&self, name: &str) -> Option<usize> {
        self.tables.iter().position(|t| t.name == name)
    }

    pub fn root_member(&self, name: &str) -> Option<usize> {
        self.root.iter().position(|m| m.name == name)
    }
}

impl Table {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|m| m.name == name)
    }
}

/// Where a type is declared; decides which kinds are allowed.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Root,
    Row,
    Item,
}

pub(crate) fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    name.len() <= 64
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub(crate) fn printable(text: &str, max: usize) -> bool {
    let n = text.chars().count();
    (1..=max).contains(&n) && !text.chars().any(char::is_control)
}

fn annotations(f: &crate::json::Fields) -> Result<(Option<String>, Option<String>), String> {
    let title = f.opt_str("title")?;
    let description = f.opt_str("description")?;
    if title.is_some_and(|t| !printable(t, 80)) {
        return Err(format!(
            "{}: title must be 1..80 printable characters",
            f.at("title")
        ));
    }
    if description.is_some_and(|d| !printable(d, 400)) {
        return Err(format!(
            "{}: description must be 1..400 printable characters",
            f.at("description")
        ));
    }
    Ok((title.map(str::to_owned), description.map(str::to_owned)))
}

fn members_of<'a>(json: &'a Json, path: &str) -> Result<&'a [(String, Json)], String> {
    let Json::Object(members) = json else {
        return Err(format!(
            "{path}: expected an object of members, found {}",
            json.kind()
        ));
    };
    if members.is_empty() || members.len() > MAX_MEMBERS {
        return Err(format!("{path}: 1..{MAX_MEMBERS} members are required"));
    }
    for (name, _) in members {
        if !valid_name(name) {
            return Err(format!("{path}: invalid member name \"{name}\""));
        }
    }
    Ok(members)
}

/// Loads the module's `document` type.
pub fn load(json: &Json, path: &str) -> Result<Schema, String> {
    let f = fields(json, path, &["type", "members"], &["title", "description"])?;
    if f.str("type")? != "object" {
        return Err(format!(
            "{}: the document root must be of type \"object\"",
            f.at("type")
        ));
    }
    let members = members_of(f.req("members"), &f.at("members"))?;
    let table_names: Vec<&str> = members
        .iter()
        .filter(|(_, ty)| matches!(ty, Json::Object(m) if m.iter().any(|(k, v)| k == "type" && *v == Json::Str("table".into()))))
        .map(|(name, _)| name.as_str())
        .collect();
    let mut schema = Schema {
        root: Vec::new(),
        tables: Vec::new(),
    };
    let mut pending = Vec::new();
    for (name, ty) in members {
        let at = format!("{}/{name}", f.at("members"));
        if table_names.contains(&name.as_str()) {
            let index = schema.tables.len();
            schema.tables.push(load_table(ty, &at, name, &table_names)?);
            let (title, description) = (
                schema.tables[index].title.clone(),
                schema.tables[index].description.clone(),
            );
            pending.push((name.clone(), Ty::Table(index), title, description));
        } else {
            let (ty, title, description) = load_ty(ty, &at, Place::Root, &table_names)?;
            pending.push((name.clone(), ty, title, description));
        }
    }
    schema.root = pending
        .into_iter()
        .map(|(name, ty, title, description)| Member {
            name,
            ty,
            title,
            description,
        })
        .collect();
    for table in &schema.tables {
        if let Some(key) = table.key {
            if !matches!(table.columns[key].ty, Ty::Int { .. }) {
                return Err(format!(
                    "{path}: key of table \"{}\" must be an int member",
                    table.name
                ));
            }
        }
    }
    let tables = &schema.tables;
    let has_key = |t: usize| tables[t].key.is_some();
    let refs = schema
        .root
        .iter()
        .chain(schema.tables.iter().flat_map(|t| t.columns.iter()))
        .filter_map(|m| match m.ty {
            Ty::Ref { table, .. } => Some((m, table)),
            _ => None,
        });
    for (member, table) in refs {
        if !has_key(table) {
            return Err(format!(
                "{path}: member \"{}\" refers to table \"{}\", which has no key",
                member.name, tables[table].name
            ));
        }
    }
    Ok(schema)
}

fn load_table(json: &Json, path: &str, name: &str, tables: &[&str]) -> Result<Table, String> {
    let f = fields(
        json,
        path,
        &["type", "row", "min_rows", "max_rows"],
        &["key", "title", "description"],
    )?;
    let (title, description) = annotations(&f)?;
    let row = fields(f.req("row"), &f.at("row"), &["type", "members"], &[])?;
    if row.str("type")? != "object" {
        return Err(format!(
            "{}: a table row must be of type \"object\"",
            row.at("type")
        ));
    }
    let mut columns = Vec::new();
    for (column, ty) in members_of(row.req("members"), &row.at("members"))? {
        let at = format!("{}/{column}", row.at("members"));
        let (ty, title, description) = load_ty(ty, &at, Place::Row, tables)?;
        columns.push(Member {
            name: column.clone(),
            ty,
            title,
            description,
        });
    }
    let min_rows = f.count("min_rows", MAX_ROWS)?;
    let max_rows = f.count("max_rows", MAX_ROWS)?;
    if min_rows > max_rows {
        return Err(format!("{path}: min_rows exceeds max_rows"));
    }
    let key = match f.opt_str("key")? {
        None => None,
        Some(k) => Some(
            columns
                .iter()
                .position(|m| m.name == k)
                .ok_or_else(|| format!("{}: no row member \"{k}\"", f.at("key")))?,
        ),
    };
    Ok(Table {
        name: name.to_owned(),
        columns,
        min_rows,
        max_rows,
        key,
        title,
        description,
    })
}

type Loaded = (Ty, Option<String>, Option<String>);

fn load_ty(json: &Json, path: &str, place: Place, tables: &[&str]) -> Result<Loaded, String> {
    let Json::Object(members) = json else {
        return Err(format!(
            "{path}: expected a type object, found {}",
            json.kind()
        ));
    };
    let kind = members
        .iter()
        .find(|(k, _)| k == "type")
        .map(|(_, v)| str_of(v, &format!("{path}/type")))
        .transpose()?
        .ok_or_else(|| format!("{path}: missing member \"type\""))?;
    let common = ["title", "description"];
    let ty = match kind {
        "int" => {
            let f = fields(json, path, &["type", "min", "max"], &common)?;
            let min = f.int("min", i64::MIN, i64::MAX)?;
            let max = f.int("max", i64::MIN, i64::MAX)?;
            if min > max {
                return Err(format!("{path}: min exceeds max"));
            }
            (Ty::Int { min, max }, annotations(&f)?)
        }
        "enum" => {
            let f = fields(json, path, &["type", "values"], &common)?;
            let Json::Array(items) = f.req("values") else {
                return Err(format!("{}: expected an array", f.at("values")));
            };
            let mut values: Vec<String> = Vec::new();
            for item in items {
                let v = str_of(item, &f.at("values"))?;
                let ok =
                    (1..=64).contains(&v.len()) && v.bytes().all(|b| (0x20..=0x7e).contains(&b));
                if !ok || values.iter().any(|x| x == v) {
                    return Err(format!(
                        "{}: \"{v}\" is empty, too long, not printable ASCII or repeated",
                        f.at("values")
                    ));
                }
                values.push(v.to_owned());
            }
            if values.is_empty() || values.len() > 256 {
                return Err(format!("{}: 1..256 values are required", f.at("values")));
            }
            (
                Ty::Enum {
                    values: values.into(),
                },
                annotations(&f)?,
            )
        }
        "text" => {
            let f = fields(
                json,
                path,
                &["type", "charset", "min_bytes", "max_bytes"],
                &common,
            )?;
            let ascii = match f.str("charset")? {
                "printable-ascii" => true,
                "utf8" => false,
                other => {
                    return Err(format!(
                        "{}: unsupported charset \"{other}\"",
                        f.at("charset")
                    ))
                }
            };
            let min = f.count("min_bytes", MAX_LEN)?;
            let max = f.count("max_bytes", MAX_LEN)?;
            if min > max {
                return Err(format!("{path}: min_bytes exceeds max_bytes"));
            }
            (Ty::Text { ascii, min, max }, annotations(&f)?)
        }
        "const" => {
            let f = fields(json, path, &["type", "value"], &common)?;
            (Ty::Const(f.req("value").clone()), annotations(&f)?)
        }
        "array" if place != Place::Item => {
            let f = fields(
                json,
                path,
                &["type", "items", "min_len", "max_len"],
                &common,
            )?;
            let (item, _, _) = load_ty(f.req("items"), &f.at("items"), Place::Item, tables)?;
            let min = f.count("min_len", MAX_LEN)?;
            let max = f.count("max_len", MAX_LEN)?;
            if min > max {
                return Err(format!("{path}: min_len exceeds max_len"));
            }
            (
                Ty::Array {
                    item: Box::new(item),
                    min,
                    max,
                },
                annotations(&f)?,
            )
        }
        "ref" if place != Place::Item => {
            let f = fields(
                json,
                path,
                &["type", "table"],
                &["none", "title", "description"],
            )?;
            let name = f.str("table")?;
            let table = tables.iter().position(|t| *t == name).ok_or_else(|| {
                format!(
                    "{}: no table \"{name}\" in the document root",
                    f.at("table")
                )
            })?;
            let none = f
                .opt("none")
                .map(|_| f.int("none", i64::MIN, i64::MAX))
                .transpose()?;
            (Ty::Ref { table, none }, annotations(&f)?)
        }
        other => {
            return Err(format!("{path}: type \"{other}\" is not allowed here"));
        }
    };
    let (ty, (title, description)) = ty;
    Ok((ty, title, description))
}

/// A checked document value; constants carry no data because their value is declared.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Int(i64),
    Str(String),
    Array(Vec<Val>),
    Fixed,
}

/// A structurally valid document.
#[derive(Debug)]
pub struct Doc {
    pub root: Vec<Val>,
    pub tables: Vec<TableData>,
}

/// The rows of one table and the position of each key.
#[derive(Debug, Default)]
pub struct TableData {
    pub rows: Vec<Vec<Val>>,
    pub keys: HashMap<i64, usize>,
}

struct Checker<'s> {
    schema: &'s Schema,
    findings: Vec<Finding>,
    refs: Vec<(usize, Option<i64>, i64, String)>,
}

impl<'s> Checker<'s> {
    fn fail(&mut self, path: &str, message: String) {
        self.findings.push(Finding::structure(path, message));
    }

    fn value(&mut self, ty: &Ty, json: &Json, path: &str) -> Val {
        match (ty, json) {
            (Ty::Int { min, max }, Json::Int(v)) => {
                if !(min..=max).contains(&v) {
                    self.fail(path, format!("{v} is outside {min}..{max}"));
                }
                Val::Int(*v)
            }
            (Ty::Enum { values }, Json::Str(s)) => {
                if !values.iter().any(|v| v == s) {
                    self.fail(
                        path,
                        format!("\"{s}\" is not one of: {}", values.join(", ")),
                    );
                }
                Val::Str(s.clone())
            }
            (Ty::Text { ascii, min, max }, Json::Str(s)) => {
                if !(min..=max).contains(&&s.len()) {
                    self.fail(
                        path,
                        format!("text is {} bytes, allowed are {min}..{max}", s.len()),
                    );
                }
                if *ascii && !s.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
                    self.fail(path, "only printable ASCII characters are allowed".into());
                } else if !*ascii && s.chars().any(char::is_control) {
                    self.fail(path, "control characters are not allowed".into());
                }
                Val::Str(s.clone())
            }
            (Ty::Const(expected), actual) => {
                if !expected.same(actual) {
                    self.fail(
                        path,
                        "value differs from the fixed value the module declares".into(),
                    );
                }
                Val::Fixed
            }
            (Ty::Array { item, min, max }, Json::Array(items)) => {
                if !(min..=max).contains(&&items.len()) {
                    self.fail(
                        path,
                        format!("{} items, allowed are {min}..{max}", items.len()),
                    );
                }
                let vals = items
                    .iter()
                    .enumerate()
                    .map(|(i, j)| self.value(item, j, &format!("{path}/{i}")))
                    .collect();
                Val::Array(vals)
            }
            (Ty::Ref { table, none }, Json::Int(v)) => {
                self.refs.push((*table, *none, *v, path.to_owned()));
                Val::Int(*v)
            }
            (ty, actual) => {
                let expected = match ty {
                    Ty::Int { .. } | Ty::Ref { .. } => "an integer",
                    Ty::Enum { .. } | Ty::Text { .. } => "a string",
                    Ty::Array { .. } | Ty::Table(_) => "an array",
                    Ty::Const(_) => unreachable!("const accepts any kind"),
                };
                self.fail(
                    path,
                    format!("expected {expected}, found {}", actual.kind()),
                );
                Val::Fixed
            }
        }
    }

    fn object<'j>(
        &mut self,
        members: &[Member],
        json: &'j Json,
        path: &str,
    ) -> Option<Vec<&'j Json>> {
        let Json::Object(actual) = json else {
            self.fail(path, format!("expected an object, found {}", json.kind()));
            return None;
        };
        for (name, _) in actual {
            if !members.iter().any(|m| &m.name == name) {
                self.fail(path, format!("unknown member \"{name}\""));
            }
        }
        let mut found = Vec::new();
        let mut complete = true;
        for member in members {
            match actual.iter().find(|(n, _)| *n == member.name) {
                Some((_, v)) => found.push(v),
                None => {
                    self.fail(path, format!("missing member \"{}\"", member.name));
                    complete = false;
                }
            }
        }
        complete.then_some(found)
    }

    fn table(&mut self, index: usize, json: &Json, path: &str) -> TableData {
        let schema: &'s Schema = self.schema;
        let table = &schema.tables[index];
        let Json::Array(rows) = json else {
            self.fail(
                path,
                format!("expected an array of rows, found {}", json.kind()),
            );
            return TableData::default();
        };
        if !(table.min_rows..=table.max_rows).contains(&rows.len()) {
            self.fail(
                path,
                format!(
                    "{} rows, allowed are {}..{}",
                    rows.len(),
                    table.min_rows,
                    table.max_rows
                ),
            );
        }
        let mut data = TableData::default();
        for (i, row) in rows.iter().enumerate() {
            let row_path = format!("{path}/{i}");
            let Some(values) = self.object(&table.columns, row, &row_path) else {
                continue;
            };
            let vals: Vec<Val> = table
                .columns
                .iter()
                .zip(values)
                .map(|(m, j)| self.value(&m.ty, j, &format!("{row_path}/{}", m.name)))
                .collect();
            if let Some(key) = table.key {
                if let Val::Int(k) = vals[key] {
                    if let Some(first) = data.keys.insert(k, data.rows.len()) {
                        data.keys.insert(k, first);
                        let name = &table.columns[key].name;
                        self.fail(
                            &format!("{row_path}/{name}"),
                            format!("key {k} is already used by row {first}"),
                        );
                    }
                }
            }
            data.rows.push(vals);
        }
        data
    }
}

/// Checks `json` against the schema; returns the typed document when no finding was made.
pub fn check(schema: &Schema, json: &Json) -> Result<Doc, Vec<Finding>> {
    let mut c = Checker {
        schema,
        findings: Vec::new(),
        refs: Vec::new(),
    };
    let mut doc = Doc {
        root: Vec::new(),
        tables: (0..schema.tables.len())
            .map(|_| TableData::default())
            .collect(),
    };
    if let Some(values) = c.object(&schema.root, json, "") {
        for (member, j) in schema.root.iter().zip(values) {
            let path = format!("/{}", member.name);
            let val = match member.ty {
                Ty::Table(t) => {
                    doc.tables[t] = c.table(t, j, &path);
                    Val::Fixed
                }
                ref ty => c.value(ty, j, &path),
            };
            doc.root.push(val);
        }
    }
    for (table, none, value, path) in std::mem::take(&mut c.refs) {
        if Some(value) != none && !doc.tables[table].keys.contains_key(&value) {
            let name = &schema.tables[table].name;
            c.fail(&path, format!("no row of \"{name}\" has key {value}"));
        }
    }
    if c.findings.is_empty() {
        Ok(doc)
    } else {
        Err(c.findings)
    }
}
