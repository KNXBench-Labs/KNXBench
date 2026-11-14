//! Reads translated parameter captions and source-bound literal grid-axis text.

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
/// The rename leaves are hosts too: an active replacement is translated by its own identity.
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
         WHERE n.program_id=?1 AND n.kind IN ('ParameterBlock','ParameterSeparator','ParameterBlockRename','Rename')
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

/// Request-local literal axis overlays; refusals retain their own original caption.
#[derive(Debug, Default)]
pub struct GridAxisCaptions {
    pub texts: HashMap<String, String>,
    /// A selected translated template is unsupported, never evaluated as a binding.
    pub unsupported: std::collections::HashSet<String>,
}

/// Select each source-bound program axis's own `Text`, never `Value` or another owner.
/// Seek distinct language keys through the existing (scope, scope_id, language,
/// ref_id, attribute_name) index, charging each key before retaining it. Each
/// candidate-axis probe and selected payload read is an exact primary-key lookup;
/// unrelated rows/payloads are never scanned or sorted. Work is bounded by the
/// source-admitted axis count and the same sticky metadata/text budget retained
/// through DTO copies, including when the requested language matches nothing.
pub fn parameter_grid_axis_captions(
    conn: &Connection,
    program: &str,
    layout: &crate::parameter_layout::ParameterLayout,
    language: Option<&str>,
    budget: &mut TextProjectionBudget,
) -> Result<GridAxisCaptions, CaptionTemplateError> {
    let mut result = GridAxisCaptions::default();
    let (Some(language), Some(axes)) = (language, layout.literal_grid_axes(program)) else {
        return Ok(result);
    };
    if axes.is_empty() {
        return Ok(result);
    }
    // LIMIT alone would not bound visited rows with an attribute/text filter.
    // These covering prefix seeks inspect the first key, then jump past the
    // entire preceding language (including unrelated references/attributes).
    let mut first_language = conn.prepare(
        "SELECT language FROM translation WHERE scope='Program' AND scope_id=?1
         ORDER BY language LIMIT 1",
    )?;
    let mut next_language = conn.prepare(
        "SELECT language FROM translation WHERE scope='Program' AND scope_id=?1
         AND language>?2 ORDER BY language LIMIT 1",
    )?;
    let mut candidate_axis = conn.prepare(
        "SELECT 1 FROM translation WHERE scope='Program' AND scope_id=?1
         AND language=?2 AND ref_id=?3 AND attribute_name='Text' AND text IS NOT NULL",
    )?;
    let mut cursor: Option<String> = None;
    let mut languages: HashMap<String, Vec<String>> = HashMap::new();
    loop {
        let mut rows = if let Some(previous) = &cursor {
            next_language.query(rusqlite::params![program, previous])?
        } else {
            first_language.query([program])?
        };
        let Some(row) = rows.next()? else {
            break;
        };
        let candidate = row
            .get_ref(0)?
            .as_str()
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "language".into(), Type::Text))?;
        // Even unmatched language metadata consumes the shared allowance, so
        // normalized-table growth cannot turn key enumeration into unbounded work.
        let candidate = budget.copy_text(candidate)?;
        if super::best_matching_language(language, &[candidate.as_str()]).is_some() {
            for id in axes {
                let mut rows = candidate_axis.query(rusqlite::params![program, candidate, id])?;
                if rows.next()?.is_some() {
                    budget.admit_copy(id)?;
                    budget.admit_copy(&candidate)?;
                    languages
                        .entry(id.clone())
                        .or_default()
                        .push(candidate.clone());
                }
            }
        }
        cursor = Some(candidate);
    }
    let mut selected = HashMap::new();
    for (id, candidates) in languages {
        let refs: Vec<_> = candidates.iter().map(String::as_str).collect();
        if let Some(matched) = super::best_matching_language(language, &refs) {
            selected.insert(id, budget.copy_text(matched)?);
        }
    }
    if selected.is_empty() {
        return Ok(result);
    }
    let mut stmt = conn.prepare(
        "SELECT text FROM translation WHERE scope='Program' AND scope_id=?1
         AND language=?2 AND ref_id=?3 AND attribute_name='Text' AND text IS NOT NULL",
    )?;
    for (id, language) in selected {
        let mut rows = stmt.query(rusqlite::params![program, language, id])?;
        let Some(row) = rows.next()? else {
            continue;
        };
        let text = row
            .get_ref(0)?
            .as_str()
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "text".into(), Type::Text))?;
        budget.admit_copy(&id)?;
        // Inspect selected content under the same byte allowance, even on refusal.
        budget.admit_copy(text)?;
        if text.contains("{{") || text.contains("}}") {
            result.unsupported.insert(id);
        } else {
            result.texts.insert(id, text.to_owned());
        }
    }
    Ok(result)
}
