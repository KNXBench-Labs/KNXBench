//! Plans bounded literal naming batches and publishes only after durable admission.

use knx_core::{Command, CommandStack, GroupAddressId};
use knx_store::project_history::{self as history, NativeSnapshot};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

fn serialize_group_address_id<S: serde::Serializer>(
    id: &GroupAddressId,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_u32(id.0)
}

pub const MAX_RECIPE_SELECTION: usize = 256;
pub const MAX_RECIPE_AFFIX_BYTES: usize = 4 * knx_core::names::MAX_EDITED_NAME_CODEPOINTS;
pub const MAX_RECIPE_PREVIEW_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameRecipeRevision {
    pub project_incarnation: String,
    pub generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRecipeIntent {
    pub ids: Vec<GroupAddressId>,
    pub prefix: String,
    pub suffix: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameRecipeRow {
    #[serde(serialize_with = "serialize_group_address_id")]
    pub id: GroupAddressId,
    pub before: String,
    pub after: String,
    pub changed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameRecipePreview {
    pub version: u8,
    pub rows: Vec<NameRecipeRow>,
    pub changed: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRecipeError {
    InvalidRevision,
    InvalidSelection,
    Budget,
    DuplicateSelection(GroupAddressId),
    MissingTarget(GroupAddressId),
    AmbiguousTarget(GroupAddressId),
    InvalidName(GroupAddressId, knx_core::names::NameError),
    Conflict,
    Command(knx_core::CommandError),
    Persistence(String),
}
impl std::fmt::Display for NameRecipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "offline name recipe refused: {self:?}")
    }
}
impl std::error::Error for NameRecipeError {}

pub struct NameRecipePlan {
    before: NativeSnapshot,
    revision: NameRecipeRevision,
    intent: NameRecipeIntent,
    command: Command,
    preview: NameRecipePreview,
}
impl NameRecipePlan {
    pub fn preview(&self) -> &NameRecipePreview {
        &self.preview
    }
    /// Bind a stateless consumer's exact target authority to the displayed plan.
    pub fn confirmation_token(&self, authority: &str) -> Result<String, NameRecipeError> {
        if authority.is_empty() || authority.len() > 256 {
            return Err(NameRecipeError::InvalidRevision);
        }
        let hash = self
            .before
            .semantic_hash()
            .map_err(|e| NameRecipeError::Persistence(e.to_string()))?;
        let bytes = serde_json::to_vec(&(
            "literal-ga-name-v1",
            authority,
            &self.revision,
            hash,
            self.intent.ids.iter().map(|id| id.0).collect::<Vec<_>>(),
            &self.intent.prefix,
            &self.intent.suffix,
            &self.preview,
        ))
        .map_err(|e| NameRecipeError::Persistence(e.to_string()))?;
        Ok(history::image_hash(&bytes))
    }
}
#[derive(Debug, Clone, Copy)]
pub struct NameRecipeNativeTarget<'a> {
    pub path: &'a Path,
    pub history_generation: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameRecipeOutcome {
    pub changed: bool,
    pub native_generation: Option<i64>,
}

pub fn preview_name_recipe(
    snapshot: &NativeSnapshot,
    revision: &NameRecipeRevision,
    intent: NameRecipeIntent,
) -> Result<NameRecipePlan, NameRecipeError> {
    if revision.project_incarnation.is_empty()
        || revision.project_incarnation.len() > 128
        || revision.generation > 9_007_199_254_740_991
    {
        return Err(NameRecipeError::InvalidRevision);
    }
    if intent.ids.is_empty() || intent.ids.len() > MAX_RECIPE_SELECTION {
        return Err(NameRecipeError::InvalidSelection);
    }
    if intent.prefix.len() > MAX_RECIPE_AFFIX_BYTES || intent.suffix.len() > MAX_RECIPE_AFFIX_BYTES
    {
        return Err(NameRecipeError::Budget);
    }
    let mut selected = BTreeSet::new();
    let mut rows = Vec::new();
    let mut commands = Vec::new();
    let mut text_bytes = 0usize;
    for &id in &intent.ids {
        if !selected.insert(id) {
            return Err(NameRecipeError::DuplicateSelection(id));
        }
        let mut entries = snapshot
            .project
            .installations
            .iter()
            .flat_map(|i| &i.group_addresses)
            .filter(|entry| entry.id == id);
        let entry = entries.next().ok_or(NameRecipeError::MissingTarget(id))?;
        if entries.next().is_some() {
            return Err(NameRecipeError::AmbiguousTarget(id));
        }
        text_bytes = text_bytes
            .checked_add(entry.name.len())
            .ok_or(NameRecipeError::Budget)?;
        if text_bytes > MAX_RECIPE_PREVIEW_BYTES {
            return Err(NameRecipeError::Budget);
        }
        let after = format!("{}{}{}", intent.prefix, entry.name, intent.suffix);
        text_bytes = text_bytes
            .checked_add(after.len())
            .ok_or(NameRecipeError::Budget)?;
        if text_bytes > MAX_RECIPE_PREVIEW_BYTES {
            return Err(NameRecipeError::Budget);
        }
        let changed = after != entry.name;
        if changed {
            knx_core::names::validate_edited_name(&after)
                .map_err(|e| NameRecipeError::InvalidName(id, e))?;
            commands.push(Command::RenameGroupAddress {
                id,
                name: after.clone(),
            });
        }
        rows.push(NameRecipeRow {
            id,
            before: entry.name.clone(),
            after,
            changed,
        });
    }
    let changed = commands.len();
    let command = Command::Batch(commands);
    if changed > 0 {
        command
            .apply(&mut snapshot.project.clone())
            .map_err(NameRecipeError::Command)?;
    }
    Ok(NameRecipePlan {
        before: snapshot.clone(),
        revision: revision.clone(),
        intent,
        command,
        preview: NameRecipePreview {
            version: 1,
            rows,
            changed,
        },
    })
}

pub fn apply_name_recipe(
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    revision: &NameRecipeRevision,
    plan: &NameRecipePlan,
    native: Option<NameRecipeNativeTarget<'_>>,
) -> Result<NameRecipeOutcome, NameRecipeError> {
    if snapshot != &plan.before || revision != &plan.revision {
        return Err(NameRecipeError::Conflict);
    }
    if plan.preview.changed == 0 {
        return Ok(NameRecipeOutcome {
            changed: false,
            native_generation: None,
        });
    }
    let mut candidate = snapshot.clone();
    let mut candidate_stack = stack.clone();
    candidate_stack
        .do_command(&mut candidate.project, plan.command.clone())
        .map_err(NameRecipeError::Command)?;
    let native_generation = if let Some(target) = native {
        let expected_hash = plan
            .before
            .semantic_hash()
            .map_err(|e| NameRecipeError::Persistence(e.to_string()))?;
        let conn = history::open_editor_store(target.path, false)
            .map_err(|e| NameRecipeError::Persistence(e.to_string()))?;
        Some(
            history::save_editor_if_unchanged(
                &conn,
                &candidate,
                &candidate_stack,
                target.history_generation,
                &expected_hash,
                false,
            )
            .map_err(|e| NameRecipeError::Persistence(e.to_string()))?,
        )
    } else {
        None
    };
    *snapshot = candidate;
    *stack = candidate_stack;
    Ok(NameRecipeOutcome {
        changed: true,
        native_generation,
    })
}
