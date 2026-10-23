//! Reads translated parameter-caption templates by exact dynamic-node ownership.

use std::collections::HashMap;
use std::fmt;

use rusqlite::{types::Type, Connection};

use crate::dynamic::{TextProjectionBudget, TextProjectionError};

/// Translation lookup and content-budget failures refuse the complete projection.
#[derive(Debug)]
pub enum CaptionTemplateError {
    Sqlite(rusqlite::Error),
    Projection(TextProjectionError),
}

impl fmt::Display for CaptionTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => error.fmt(f),
            Self::Projection(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for CaptionTemplateError {}
impl From<rusqlite::Error> for CaptionTemplateError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}
impl From<TextProjectionError> for CaptionTemplateError {
    fn from(error: TextProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// Select only `Text` translations for block/separator hosts, never parameter values.
///
/// Keys retain the defining tree and node, including repeated module expansions.
/// The existing exact/prefix language rule is shared with all other product text
/// readers. The caller retains this same budget through filling and DTO copies.
/// SQLite's own query allocations are not an aggregate memory guarantee.
pub fn parameter_caption_templates(
    conn: &Connection,
    program: &str,
    language: &str,
    budget: &mut TextProjectionBudget,
) -> Result<HashMap<(String, i64), String>, CaptionTemplateError> {
    let mut stmt = conn.prepare(
        "SELECT n.module_def_id, n.node_id, t.language
         FROM dynamic_node AS n JOIN translation AS t
           ON t.scope='Program' AND t.scope_id=n.program_id
          AND t.ref_id=n.element_id AND t.attribute_name='Text'
         WHERE n.program_id=?1 AND n.kind IN ('ParameterBlock','ParameterSeparator')
           AND t.text IS NOT NULL
         ORDER BY n.module_def_id, n.node_id, t.language",
    )?;
    let mut rows = stmt.query([program])?;
    let mut languages: HashMap<(String, i64), Vec<String>> = HashMap::new();
    while let Some(row) = rows.next()? {
        let tree = row.get_ref(0)?.as_str().map_err(|_| {
            rusqlite::Error::InvalidColumnType(0, "module_def_id".into(), Type::Text)
        })?;
        let language = row
            .get_ref(2)?
            .as_str()
            .map_err(|_| rusqlite::Error::InvalidColumnType(2, "language".into(), Type::Text))?;
        budget.admit_copy(tree)?;
        budget.admit_copy(language)?;
        languages
            .entry((tree.to_owned(), row.get(1)?))
            .or_default()
            .push(language.to_owned());
    }
    let mut text_query = conn.prepare(
        "SELECT t.text FROM dynamic_node AS n JOIN translation AS t
           ON t.scope='Program' AND t.scope_id=n.program_id
          AND t.ref_id=n.element_id AND t.attribute_name='Text'
         WHERE n.program_id=?1 AND n.module_def_id=?2 AND n.node_id=?3
           AND t.language=?4 AND t.text IS NOT NULL",
    )?;
    let mut result = HashMap::new();
    for ((tree, node), candidates) in languages {
        let refs: Vec<_> = candidates.iter().map(String::as_str).collect();
        let Some(selected) = super::best_matching_language(language, &refs) else {
            continue;
        };
        let mut rows = text_query.query(rusqlite::params![program, tree, node, selected])?;
        if let Some(row) = rows.next()? {
            let text = row
                .get_ref(0)?
                .as_str()
                .map_err(|_| rusqlite::Error::InvalidColumnType(0, "text".into(), Type::Text))?;
            result.insert((tree, node), budget.copy_text(text)?);
        }
    }
    Ok(result)
}
