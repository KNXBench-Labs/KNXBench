//! Module rules: a required condition per document or per table row, with a user message.

use crate::expr::{compile, Env, EvalError, Expr, STy, Scope, V};
use crate::json::{fields, Json};
use crate::schema::{printable, Doc, Schema};
use crate::{Finding, Validation};

/// Most rules a module may declare.
pub const MAX_RULES: usize = 4096;

/// A compiled rule.
#[derive(Debug)]
pub struct Rule {
    pub id: String,
    pub table: Option<usize>,
    pub when: Option<Expr>,
    pub require: Expr,
    pub field: Option<String>,
    pub message: String,
}

pub(crate) fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    id.len() <= 64
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn boolean(json: &Json, scope: &mut Scope, path: &str) -> Result<Expr, String> {
    let (expr, sty) = compile(json, scope, 0, path)?;
    match sty {
        STy::Bool => Ok(expr),
        other => Err(format!(
            "{path}: a condition must be boolean, found {}",
            other.name()
        )),
    }
}

/// Loads the module's `rules` array.
pub fn load(json: &Json, schema: &Schema, path: &str) -> Result<Vec<Rule>, String> {
    let Json::Array(items) = json else {
        return Err(format!("{path}: expected an array"));
    };
    if items.len() > MAX_RULES {
        return Err(format!("{path}: at most {MAX_RULES} rules"));
    }
    let mut rules: Vec<Rule> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("{path}/{i}");
        let f = fields(
            item,
            &at,
            &["id", "require", "message"],
            &["table", "when", "field"],
        )?;
        let id = f.str("id")?;
        if !valid_id(id) || rules.iter().any(|r| r.id == id) {
            return Err(format!(
                "{}: invalid or repeated rule id \"{id}\"",
                f.at("id")
            ));
        }
        let table = match f.opt_str("table")? {
            Some(name) => Some(
                schema
                    .table(name)
                    .ok_or_else(|| format!("{}: no table \"{name}\"", f.at("table")))?,
            ),
            None => None,
        };
        let field = f.opt_str("field")?;
        if let Some(name) = field {
            let known = match table {
                Some(t) => schema.tables[t].column(name).is_some(),
                None => schema.root_member(name).is_some(),
            };
            if !known {
                return Err(format!(
                    "{}: no member \"{name}\" to report at",
                    f.at("field")
                ));
            }
        }
        let message = f.str("message")?;
        if !printable(message, 400) {
            return Err(format!(
                "{}: message must be 1..400 printable characters",
                f.at("message")
            ));
        }
        let mut scope = Scope::new(schema, table);
        let when = f
            .opt("when")
            .map(|w| boolean(w, &mut scope, &f.at("when")))
            .transpose()?;
        let require = boolean(f.req("require"), &mut scope, &f.at("require"))?;
        rules.push(Rule {
            id: id.to_owned(),
            table,
            when,
            require,
            field: field.map(str::to_owned),
            message: message.to_owned(),
        });
    }
    Ok(rules)
}

/// Evaluates every rule; keeps the first violations in rule order, then row order, and counts the rest.
pub fn run(
    rules: &[Rule],
    schema: &Schema,
    doc: &Doc,
    limit: u64,
) -> Result<Validation, EvalError> {
    let mut env = Env {
        schema,
        doc,
        current: None,
        vars: Vec::new(),
        steps: 0,
        limit,
    };
    let mut findings = Validation::empty();
    for rule in rules {
        let rows: Vec<Option<(usize, usize)>> = match rule.table {
            Some(t) => (0..doc.tables[t].rows.len())
                .map(|r| Some((t, r)))
                .collect(),
            None => vec![None],
        };
        for row in rows {
            env.current = row;
            let applies = match &rule.when {
                Some(when) => env.eval(when)? == V::Bool(true),
                None => true,
            };
            if applies && env.eval(&rule.require)? != V::Bool(true) {
                findings.push(|| {
                    let mut path = match row {
                        Some((t, r)) => format!("/{}/{r}", schema.tables[t].name),
                        None => String::new(),
                    };
                    if let Some(field) = &rule.field {
                        path.push('/');
                        path.push_str(field);
                    }
                    Finding::rule(&path, &rule.id, &rule.message)
                });
            }
        }
    }
    Ok(findings)
}
