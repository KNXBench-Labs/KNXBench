//! Rule expressions: compiled from JSON trees, type-checked at load and evaluated with limits.

use crate::json::{fields, str_of, Json};
use crate::schema::{valid_name, Doc, Schema, Ty, Val};
use std::sync::Arc;

/// Deepest expression nesting a module may use.
pub const MAX_DEPTH: usize = 32;
/// Deepest quantifier nesting a module may use.
pub const MAX_QUANTIFIERS: usize = 3;

/// Static type of an expression; every value may additionally be `null` at run time.
#[derive(Debug, Clone)]
pub enum STy {
    Null,
    Int,
    Bool,
    Str(Option<Arc<[String]>>),
    Row(usize),
    Array(Box<STy>),
}

impl STy {
    fn name(&self) -> &'static str {
        match self {
            STy::Null => "null",
            STy::Int => "integer",
            STy::Bool => "boolean",
            STy::Str(_) => "string",
            STy::Row(_) => "row",
            STy::Array(_) => "array",
        }
    }
}

/// Joins two static types or reports that they are of different kinds.
fn unify(a: &STy, b: &STy) -> Option<STy> {
    Some(match (a, b) {
        (STy::Null, x) | (x, STy::Null) => x.clone(),
        (STy::Int, STy::Int) => STy::Int,
        (STy::Bool, STy::Bool) => STy::Bool,
        (STy::Str(x), STy::Str(y)) => STy::Str(match (x, y) {
            (Some(x), Some(y)) if x == y => Some(x.clone()),
            _ => None,
        }),
        (STy::Row(x), STy::Row(y)) if x == y => STy::Row(*x),
        (STy::Array(x), STy::Array(y)) => STy::Array(Box::new(unify(x, y)?)),
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Mod,
    Xor,
    Neg,
    And,
    Or,
    Not,
    In,
    If,
    IsNull,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quant {
    Count,
    Any,
    All,
}

/// A compiled expression; member and table names are resolved to positions.
#[derive(Debug)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    RootField(usize),
    RowField {
        var: Option<usize>,
        member: usize,
    },
    Index(Option<usize>),
    Lookup {
        table: usize,
        key: Box<Expr>,
    },
    RowAt {
        table: usize,
        index: Box<Expr>,
    },
    Get {
        row: Box<Expr>,
        member: usize,
    },
    Item {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Ordinal {
        value: Box<Expr>,
        values: Arc<[String]>,
    },
    Length(Box<Expr>),
    Rows(usize),
    Quant {
        kind: Quant,
        table: usize,
        pred: Box<Expr>,
    },
    Depth {
        table: usize,
        parent: usize,
        root: i64,
    },
    Op {
        op: Op,
        args: Vec<Expr>,
    },
}

/// Names visible while compiling: the current table and the bound row variables.
pub struct Scope<'s> {
    pub schema: &'s Schema,
    pub current: Option<usize>,
    vars: Vec<(String, usize)>,
}

impl<'s> Scope<'s> {
    pub fn new(schema: &'s Schema, current: Option<usize>) -> Self {
        Scope {
            schema,
            current,
            vars: Vec::new(),
        }
    }
}

fn member_sty(ty: &Ty, path: &str) -> Result<(Option<Expr>, STy), String> {
    Ok(match ty {
        Ty::Int { .. } | Ty::Ref { .. } => (None, STy::Int),
        Ty::Enum { values } => (None, STy::Str(Some(values.clone()))),
        Ty::Text { .. } => (None, STy::Str(None)),
        Ty::Array { item, .. } => (None, STy::Array(Box::new(member_sty(item, path)?.1))),
        Ty::Const(Json::Null) => (Some(Expr::Null), STy::Null),
        Ty::Const(Json::Bool(b)) => (Some(Expr::Bool(*b)), STy::Bool),
        Ty::Const(Json::Int(i)) => (Some(Expr::Int(*i)), STy::Int),
        Ty::Const(Json::Str(s)) => (Some(Expr::Str(s.clone())), STy::Str(None)),
        Ty::Const(_) => {
            return Err(format!(
                "{path}: a fixed array or object cannot be used in an expression"
            ))
        }
        Ty::Table(_) => {
            return Err(format!(
                "{path}: a table is not a value; use \"rows\" or a quantifier"
            ))
        }
    })
}

fn table_of(scope: &Scope, json: &Json, path: &str) -> Result<usize, String> {
    let name = str_of(json, path)?;
    scope
        .schema
        .table(name)
        .ok_or_else(|| format!("{path}: no table \"{name}\""))
}

fn want(sty: &STy, expected: &str, path: &str) -> Result<(), String> {
    let ok = matches!(
        (sty, expected),
        (STy::Null, _) | (STy::Int, "integer") | (STy::Bool, "boolean") | (STy::Str(_), "string")
    );
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: expected {expected}, found {}", sty.name()))
    }
}

fn check_enum_literal(a: &(Expr, STy), b: &(Expr, STy), path: &str) -> Result<(), String> {
    for (x, y) in [(a, b), (b, a)] {
        if let ((_, STy::Str(Some(values))), (Expr::Str(s), _)) = (x, y) {
            if !values.iter().any(|v| v == s) {
                return Err(format!(
                    "{path}: \"{s}\" is not a value of the compared enum"
                ));
            }
        }
    }
    Ok(())
}

/// Compiles one expression and returns it with its static type.
pub fn compile(
    json: &Json,
    scope: &mut Scope,
    depth: usize,
    path: &str,
) -> Result<(Expr, STy), String> {
    if depth > MAX_DEPTH {
        return Err(format!("{path}: expression nesting exceeds {MAX_DEPTH}"));
    }
    let members = match json {
        Json::Null => return Ok((Expr::Null, STy::Null)),
        Json::Bool(b) => return Ok((Expr::Bool(*b), STy::Bool)),
        Json::Int(i) => return Ok((Expr::Int(*i), STy::Int)),
        Json::Str(s) => return Ok((Expr::Str(s.clone()), STy::Str(None))),
        Json::Array(_) => return Err(format!("{path}: an array is not an expression")),
        Json::Object(m) => m,
    };
    let form = [
        "field", "index", "lookup", "row", "get", "item", "ordinal", "length", "rows", "count",
        "any", "all", "depth", "op",
    ]
    .into_iter()
    .find(|k| members.iter().any(|(n, _)| n == k))
    .ok_or_else(|| format!("{path}: unknown expression form"))?;
    let sub = |name: &str| {
        format!(
            "{path}/{form}{}",
            if name.is_empty() {
                String::new()
            } else {
                format!("/{name}")
            }
        )
    };
    let next = depth + 1;
    match form {
        "field" => {
            let f = fields(json, path, &["field"], &["of"])?;
            let name = f.str("field")?;
            let table = match f.opt_str("of")? {
                Some(var) => {
                    let slot = scope
                        .vars
                        .iter()
                        .rposition(|(n, _)| n == var)
                        .ok_or_else(|| format!("{path}: no row variable \"{var}\""))?;
                    Some((Some(slot), scope.vars[slot].1))
                }
                None => scope.current.map(|t| (None, t)),
            };
            match table {
                Some((var, t)) => {
                    let table = &scope.schema.tables[t];
                    let member = table.column(name).ok_or_else(|| {
                        format!("{path}: table \"{}\" has no member \"{name}\"", table.name)
                    })?;
                    let (konst, sty) = member_sty(&table.columns[member].ty, path)?;
                    Ok((konst.unwrap_or(Expr::RowField { var, member }), sty))
                }
                None => {
                    let member = scope
                        .schema
                        .root_member(name)
                        .ok_or_else(|| format!("{path}: the document has no member \"{name}\""))?;
                    let (konst, sty) = member_sty(&scope.schema.root[member].ty, path)?;
                    Ok((konst.unwrap_or(Expr::RootField(member)), sty))
                }
            }
        }
        "index" => {
            let f = fields(json, path, &["index"], &[])?;
            match f.req("index") {
                Json::Null if scope.current.is_some() => Ok((Expr::Index(None), STy::Int)),
                Json::Null => Err(format!("{path}: \"index\" needs a current row")),
                other => {
                    let var = str_of(other, &sub(""))?;
                    let slot = scope
                        .vars
                        .iter()
                        .rposition(|(n, _)| n == var)
                        .ok_or_else(|| format!("{path}: no row variable \"{var}\""))?;
                    Ok((Expr::Index(Some(slot)), STy::Int))
                }
            }
        }
        "lookup" | "row" => {
            fields(json, path, &[form], &[])?;
            let arg_name = if form == "lookup" { "key" } else { "index" };
            let f = fields(&members[0].1, &sub(""), &["table", arg_name], &[])?;
            let table = table_of(scope, f.req("table"), &f.at("table"))?;
            if form == "lookup" && scope.schema.tables[table].key.is_none() {
                return Err(format!(
                    "{path}: table \"{}\" has no key",
                    scope.schema.tables[table].name
                ));
            }
            let (arg, sty) = compile(f.req(arg_name), scope, next, &f.at(arg_name))?;
            want(&sty, "integer", &f.at(arg_name))?;
            let arg = Box::new(arg);
            let expr = if form == "lookup" {
                Expr::Lookup { table, key: arg }
            } else {
                Expr::RowAt { table, index: arg }
            };
            Ok((expr, STy::Row(table)))
        }
        "get" | "item" => {
            fields(json, path, &[form], &[])?;
            let Json::Array(args) = &members[0].1 else {
                return Err(format!("{}: expected a two-element array", sub("")));
            };
            if args.len() != 2 {
                return Err(format!("{}: expected a two-element array", sub("")));
            }
            let (first, sty) = compile(&args[0], scope, next, &sub("0"))?;
            if form == "get" {
                let STy::Row(t) = sty else {
                    return Err(format!(
                        "{}: expected a row, found {}",
                        sub("0"),
                        sty.name()
                    ));
                };
                let name = str_of(&args[1], &sub("1"))?;
                let table = &scope.schema.tables[t];
                let member = table.column(name).ok_or_else(|| {
                    format!(
                        "{}: table \"{}\" has no member \"{name}\"",
                        sub("1"),
                        table.name
                    )
                })?;
                let (konst, sty) = member_sty(&table.columns[member].ty, path)?;
                if konst.is_some() {
                    return Err(format!(
                        "{path}: \"get\" of a fixed member; use the value directly"
                    ));
                }
                Ok((
                    Expr::Get {
                        row: Box::new(first),
                        member,
                    },
                    sty,
                ))
            } else {
                let STy::Array(item) = sty else {
                    return Err(format!(
                        "{}: expected an array, found {}",
                        sub("0"),
                        sty.name()
                    ));
                };
                let (index, isty) = compile(&args[1], scope, next, &sub("1"))?;
                want(&isty, "integer", &sub("1"))?;
                Ok((
                    Expr::Item {
                        array: Box::new(first),
                        index: Box::new(index),
                    },
                    *item,
                ))
            }
        }
        "ordinal" | "length" => {
            fields(json, path, &[form], &[])?;
            let (value, sty) = compile(&members[0].1, scope, next, &sub(""))?;
            match (form, &sty) {
                ("ordinal", STy::Str(Some(values))) => {
                    let values = values.clone();
                    Ok((
                        Expr::Ordinal {
                            value: Box::new(value),
                            values,
                        },
                        STy::Int,
                    ))
                }
                ("length", STy::Str(_) | STy::Array(_)) => {
                    Ok((Expr::Length(Box::new(value)), STy::Int))
                }
                _ => Err(format!(
                    "{}: \"{form}\" does not apply to {}",
                    sub(""),
                    sty.name()
                )),
            }
        }
        "rows" => {
            fields(json, path, &["rows"], &[])?;
            Ok((
                Expr::Rows(table_of(scope, &members[0].1, &sub(""))?),
                STy::Int,
            ))
        }
        "count" | "any" | "all" => {
            fields(json, path, &[form], &[])?;
            let f = fields(&members[0].1, &sub(""), &["table", "as", "where"], &[])?;
            let table = table_of(scope, f.req("table"), &f.at("table"))?;
            let var = f.str("as")?;
            if !valid_name(var) || scope.vars.iter().any(|(n, _)| n == var) {
                return Err(format!(
                    "{}: invalid or already bound row variable \"{var}\"",
                    f.at("as")
                ));
            }
            if scope.vars.len() >= MAX_QUANTIFIERS {
                return Err(format!(
                    "{path}: quantifier nesting exceeds {MAX_QUANTIFIERS}"
                ));
            }
            scope.vars.push((var.to_owned(), table));
            let compiled = compile(f.req("where"), scope, next, &f.at("where"));
            scope.vars.pop();
            let (pred, sty) = compiled?;
            want(&sty, "boolean", &f.at("where"))?;
            let kind = match form {
                "count" => Quant::Count,
                "any" => Quant::Any,
                _ => Quant::All,
            };
            let result = if kind == Quant::Count {
                STy::Int
            } else {
                STy::Bool
            };
            Ok((
                Expr::Quant {
                    kind,
                    table,
                    pred: Box::new(pred),
                },
                result,
            ))
        }
        "depth" => {
            fields(json, path, &["depth"], &[])?;
            let f = fields(&members[0].1, &sub(""), &["table", "parent", "root"], &[])?;
            let table = table_of(scope, f.req("table"), &f.at("table"))?;
            if scope.current != Some(table) {
                return Err(format!(
                    "{path}: \"depth\" applies to rows of the current table only"
                ));
            }
            let def = &scope.schema.tables[table];
            if def.key.is_none() {
                return Err(format!("{path}: table \"{}\" has no key", def.name));
            }
            let name = f.str("parent")?;
            let parent = def
                .column(name)
                .ok_or_else(|| format!("{}: no member \"{name}\"", f.at("parent")))?;
            let parent_ok = match def.columns[parent].ty {
                Ty::Int { .. } => true,
                Ty::Ref { table: t, .. } => t == table,
                _ => false,
            };
            if !parent_ok {
                return Err(format!(
                    "{}: the parent member must be an integer or a reference into the same table",
                    f.at("parent")
                ));
            }
            let root = f.int("root", i64::MIN, i64::MAX)?;
            Ok((
                Expr::Depth {
                    table,
                    parent,
                    root,
                },
                STy::Int,
            ))
        }
        _ => compile_op(json, scope, next, path),
    }
}

fn compile_op(
    json: &Json,
    scope: &mut Scope,
    depth: usize,
    path: &str,
) -> Result<(Expr, STy), String> {
    let f = fields(json, path, &["op", "args"], &[])?;
    let name = f.str("op")?;
    let op = match name {
        "eq" => Op::Eq,
        "ne" => Op::Ne,
        "lt" => Op::Lt,
        "le" => Op::Le,
        "gt" => Op::Gt,
        "ge" => Op::Ge,
        "add" => Op::Add,
        "sub" => Op::Sub,
        "mul" => Op::Mul,
        "mod" => Op::Mod,
        "xor" => Op::Xor,
        "neg" => Op::Neg,
        "and" => Op::And,
        "or" => Op::Or,
        "not" => Op::Not,
        "in" => Op::In,
        "if" => Op::If,
        "is_null" => Op::IsNull,
        other => return Err(format!("{}: unknown operator \"{other}\"", f.at("op"))),
    };
    let Json::Array(raw) = f.req("args") else {
        return Err(format!("{}: expected an array", f.at("args")));
    };
    let (min, max) = match op {
        Op::Neg | Op::Not | Op::IsNull => (1, 1),
        Op::And | Op::Or => (1, 64),
        Op::In => (2, 256),
        Op::If => (3, 3),
        _ => (2, 2),
    };
    if !(min..=max).contains(&raw.len()) {
        return Err(format!(
            "{}: \"{name}\" takes {min}..{max} arguments, found {}",
            f.at("args"),
            raw.len()
        ));
    }
    let mut args = Vec::with_capacity(raw.len());
    for (i, arg) in raw.iter().enumerate() {
        args.push(compile(
            arg,
            scope,
            depth,
            &format!("{}/{i}", f.at("args")),
        )?);
    }
    let at = |i: usize| format!("{}/{i}", f.at("args"));
    let result = match op {
        Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            let joined = unify(&args[0].1, &args[1].1).ok_or_else(|| {
                format!(
                    "{path}: cannot compare {} with {}",
                    args[0].1.name(),
                    args[1].1.name()
                )
            })?;
            let ordered = matches!(joined, STy::Null | STy::Int | STy::Str(_));
            let equatable = ordered || matches!(joined, STy::Bool);
            if !(equatable && (ordered || matches!(op, Op::Eq | Op::Ne))) {
                return Err(format!(
                    "{path}: \"{name}\" does not apply to {}",
                    joined.name()
                ));
            }
            check_enum_literal(&args[0], &args[1], path)?;
            STy::Bool
        }
        Op::Add | Op::Sub | Op::Mul | Op::Mod | Op::Xor | Op::Neg => {
            for (i, (_, sty)) in args.iter().enumerate() {
                want(sty, "integer", &at(i))?;
            }
            STy::Int
        }
        Op::And | Op::Or | Op::Not => {
            for (i, (_, sty)) in args.iter().enumerate() {
                want(sty, "boolean", &at(i))?;
            }
            STy::Bool
        }
        Op::In => {
            for i in 1..args.len() {
                let joined = unify(&args[0].1, &args[i].1).ok_or_else(|| {
                    format!(
                        "{}: cannot compare {} with {}",
                        at(i),
                        args[0].1.name(),
                        args[i].1.name()
                    )
                })?;
                if !matches!(joined, STy::Null | STy::Int | STy::Str(_) | STy::Bool) {
                    return Err(format!(
                        "{}: \"in\" does not apply to {}",
                        at(i),
                        joined.name()
                    ));
                }
                check_enum_literal(&args[0], &args[i], &at(i))?;
            }
            STy::Bool
        }
        Op::If => {
            want(&args[0].1, "boolean", &at(0))?;
            unify(&args[1].1, &args[2].1).ok_or_else(|| {
                format!(
                    "{path}: branches are {} and {}",
                    args[1].1.name(),
                    args[2].1.name()
                )
            })?
        }
        Op::IsNull => STy::Bool,
    };
    Ok((
        Expr::Op {
            op,
            args: args.into_iter().map(|(e, _)| e).collect(),
        },
        result,
    ))
}

/// A run-time value borrowed from the document or the module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum V<'a> {
    Null,
    Int(i64),
    Bool(bool),
    Str(&'a str),
    Row(usize, usize),
    Array(&'a [Val]),
}

/// Why an evaluation stopped without a value.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// The evaluation step budget was exhausted.
    Budget,
    /// The module asked for something its declared types do not allow.
    Module(String),
}

/// Evaluation state: the document, the current row, bound variables and the step budget.
pub struct Env<'a> {
    pub schema: &'a Schema,
    pub doc: &'a Doc,
    pub current: Option<(usize, usize)>,
    pub vars: Vec<(usize, usize)>,
    pub steps: u64,
    pub limit: u64,
}

fn from_val(val: &Val) -> Result<V<'_>, EvalError> {
    Ok(match val {
        Val::Int(i) => V::Int(*i),
        Val::Str(s) => V::Str(s),
        Val::Array(items) => V::Array(items),
        Val::Fixed => {
            return Err(EvalError::Module(
                "a fixed member has no stored value".into(),
            ))
        }
    })
}

fn mismatch(what: &str) -> EvalError {
    EvalError::Module(format!("operand of the wrong kind for {what}"))
}

fn int(v: V, what: &str) -> Result<Option<i64>, EvalError> {
    match v {
        V::Int(i) => Ok(Some(i)),
        V::Null => Ok(None),
        _ => Err(mismatch(what)),
    }
}

impl<'a> Env<'a> {
    fn row(&self, var: Option<usize>) -> Result<(usize, usize), EvalError> {
        match var {
            Some(slot) => self.vars.get(slot).copied(),
            None => self.current,
        }
        .ok_or_else(|| EvalError::Module("no row in scope".into()))
    }

    fn cell(&self, table: usize, row: usize, member: usize) -> Result<V<'a>, EvalError> {
        let doc: &'a Doc = self.doc;
        from_val(&doc.tables[table].rows[row][member])
    }

    /// Evaluates `expr`, counting one step per node.
    pub fn eval(&mut self, expr: &'a Expr) -> Result<V<'a>, EvalError> {
        self.steps += 1;
        if self.steps > self.limit {
            return Err(EvalError::Budget);
        }
        Ok(match expr {
            Expr::Null => V::Null,
            Expr::Bool(b) => V::Bool(*b),
            Expr::Int(i) => V::Int(*i),
            Expr::Str(s) => V::Str(s),
            Expr::RootField(member) => {
                let doc: &'a Doc = self.doc;
                from_val(&doc.root[*member])?
            }
            Expr::RowField { var, member } => {
                let (t, r) = self.row(*var)?;
                self.cell(t, r, *member)?
            }
            Expr::Index(var) => V::Int(self.row(*var)?.1 as i64),
            Expr::Lookup { table, key } => match int(self.eval(key)?, "lookup")? {
                Some(k) => self.doc.tables[*table]
                    .keys
                    .get(&k)
                    .map_or(V::Null, |r| V::Row(*table, *r)),
                None => V::Null,
            },
            Expr::RowAt { table, index } => match int(self.eval(index)?, "row")? {
                Some(i) if i >= 0 && (i as u64) < self.doc.tables[*table].rows.len() as u64 => {
                    V::Row(*table, i as usize)
                }
                _ => V::Null,
            },
            Expr::Get { row, member } => match self.eval(row)? {
                V::Row(t, r) => self.cell(t, r, *member)?,
                V::Null => V::Null,
                _ => return Err(mismatch("get")),
            },
            Expr::Item { array, index } => {
                let array = self.eval(array)?;
                let index = int(self.eval(index)?, "item")?;
                match (array, index) {
                    (V::Array(items), Some(i)) if i >= 0 && (i as u64) < items.len() as u64 => {
                        from_val(&items[i as usize])?
                    }
                    (V::Array(_) | V::Null, _) => V::Null,
                    _ => return Err(mismatch("item")),
                }
            }
            Expr::Ordinal { value, values } => {
                match self.eval(value)? {
                    V::Str(s) => {
                        V::Int(values.iter().position(|v| v == s).ok_or_else(|| {
                            EvalError::Module(format!("\"{s}\" is not an enum value"))
                        })? as i64)
                    }
                    V::Null => V::Null,
                    _ => return Err(mismatch("ordinal")),
                }
            }
            Expr::Length(value) => match self.eval(value)? {
                V::Str(s) => V::Int(s.len() as i64),
                V::Array(items) => V::Int(items.len() as i64),
                V::Null => V::Null,
                _ => return Err(mismatch("length")),
            },
            Expr::Rows(table) => V::Int(self.doc.tables[*table].rows.len() as i64),
            Expr::Quant { kind, table, pred } => self.quant(*kind, *table, pred)?,
            Expr::Depth {
                table,
                parent,
                root,
            } => self.depth(*table, *parent, *root)?,
            Expr::Op { op, args } => self.op(*op, args)?,
        })
    }

    fn quant(&mut self, kind: Quant, table: usize, pred: &'a Expr) -> Result<V<'a>, EvalError> {
        let mut count = 0i64;
        for r in 0..self.doc.tables[table].rows.len() {
            self.vars.push((table, r));
            let v = self.eval(pred);
            self.vars.pop();
            let hit = match v? {
                V::Bool(b) => b,
                V::Null => false,
                _ => return Err(mismatch("a quantifier")),
            };
            match (kind, hit) {
                (Quant::Any, true) => return Ok(V::Bool(true)),
                (Quant::All, false) => return Ok(V::Bool(false)),
                (Quant::Count, true) => count += 1,
                _ => {}
            }
        }
        Ok(match kind {
            Quant::Count => V::Int(count),
            Quant::Any => V::Bool(false),
            Quant::All => V::Bool(true),
        })
    }

    fn depth(&mut self, table: usize, parent: usize, root: i64) -> Result<V<'a>, EvalError> {
        let (t, start) = self.row(None)?;
        if t != table {
            return Err(EvalError::Module("depth outside its table".into()));
        }
        let doc: &'a Doc = self.doc;
        let data = &doc.tables[table];
        let parent_of = |r: usize| match data.rows[r][parent] {
            Val::Int(p) => Ok(p),
            _ => Err(mismatch("depth")),
        };
        let mut cursor = parent_of(start)?;
        let mut count = 0i64;
        while cursor != root {
            self.steps += 1;
            if self.steps > self.limit {
                return Err(EvalError::Budget);
            }
            count += 1;
            let Some(&r) = data.keys.get(&cursor) else {
                return Ok(V::Null);
            };
            if r == start || count as u64 > data.rows.len() as u64 {
                return Ok(V::Null);
            }
            cursor = parent_of(r)?;
        }
        Ok(V::Int(count))
    }

    fn op(&mut self, op: Op, args: &'a [Expr]) -> Result<V<'a>, EvalError> {
        match op {
            Op::And | Op::Or => {
                let decisive = op == Op::Or;
                let mut unknown = false;
                for arg in args {
                    match self.eval(arg)? {
                        V::Bool(b) if b == decisive => return Ok(V::Bool(decisive)),
                        V::Bool(_) => {}
                        V::Null => unknown = true,
                        _ => return Err(mismatch("and/or")),
                    }
                }
                return Ok(if unknown { V::Null } else { V::Bool(!decisive) });
            }
            Op::If => {
                return match self.eval(&args[0])? {
                    V::Bool(true) => self.eval(&args[1]),
                    V::Bool(false) | V::Null => self.eval(&args[2]),
                    _ => Err(mismatch("if")),
                };
            }
            Op::IsNull => return Ok(V::Bool(self.eval(&args[0])? == V::Null)),
            Op::In => {
                let needle = self.eval(&args[0])?;
                if needle == V::Null {
                    return Ok(V::Null);
                }
                let mut unknown = false;
                for candidate in &args[1..] {
                    match self.eval(candidate)? {
                        V::Null => unknown = true,
                        c if c == needle => return Ok(V::Bool(true)),
                        _ => {}
                    }
                }
                return Ok(if unknown { V::Null } else { V::Bool(false) });
            }
            _ => {}
        }
        let a = self.eval(&args[0])?;
        if op == Op::Not {
            return match a {
                V::Bool(b) => Ok(V::Bool(!b)),
                V::Null => Ok(V::Null),
                _ => Err(mismatch("not")),
            };
        }
        if op == Op::Neg {
            return Ok(int(a, "neg")?
                .and_then(i64::checked_neg)
                .map_or(V::Null, V::Int));
        }
        let b = self.eval(&args[1])?;
        if a == V::Null || b == V::Null {
            return Ok(V::Null);
        }
        let ordering = match (a, b) {
            (V::Int(x), V::Int(y)) => Some(x.cmp(&y)),
            (V::Str(x), V::Str(y)) => Some(x.as_bytes().cmp(y.as_bytes())),
            (V::Bool(x), V::Bool(y)) => Some(x.cmp(&y)),
            _ => None,
        };
        use std::cmp::Ordering::{Greater, Less};
        let compared = |f: fn(std::cmp::Ordering) -> bool| -> Result<V<'a>, EvalError> {
            ordering
                .map(|o| V::Bool(f(o)))
                .ok_or_else(|| mismatch("a comparison"))
        };
        match op {
            Op::Eq => compared(|o| o.is_eq()),
            Op::Ne => compared(|o| o.is_ne()),
            Op::Lt => compared(|o| o == Less),
            Op::Le => compared(|o| o != Greater),
            Op::Gt => compared(|o| o == Greater),
            Op::Ge => compared(|o| o != Less),
            _ => {
                let (Some(x), Some(y)) = (int(a, "arithmetic")?, int(b, "arithmetic")?) else {
                    return Err(mismatch("arithmetic"));
                };
                let r = match op {
                    Op::Add => x.checked_add(y),
                    Op::Sub => x.checked_sub(y),
                    Op::Mul => x.checked_mul(y),
                    Op::Mod => x.checked_rem(y),
                    Op::Xor => Some(x ^ y),
                    _ => unreachable!("handled above"),
                };
                Ok(r.map_or(V::Null, V::Int))
            }
        }
    }
}
