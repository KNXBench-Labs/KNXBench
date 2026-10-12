//! Opt-in literal group-address renames over the offline edit service.
//!
//! ADR-0116: only aliases enabled with `--allow-name-recipe` are served. The
//! plan, token and native compare-and-save come from `knx_edit::name_recipe`,
//! exactly as the `knx name-recipe` CLI uses them; the result lands in the
//! project's native working history, never in the saved root. The human
//! confirmation itself is asked for by the server layer, because the SDK
//! binds it to the originating tool call.

use std::path::PathBuf;

use knx_core::{CommandStack, GroupAddressId};
use knx_edit::name_recipe::{
    self as recipe, NameRecipeIntent, NameRecipeNativeTarget, NameRecipePlan, NameRecipeRevision,
};
use knx_store::project_history::{self as history, NativeSnapshot};
use serde_json::{json, Value};

use super::{envelope, ToolResult};
use crate::workspace::Workspace;

/// Rows quoted in the confirmation form; the full list is in the preview.
pub const CONFIRMATION_EXCERPT_ROWS: usize = 5;

/// Where an apply writes, repeated in every response.
pub const PERSISTENCE: &str = "native-working-history";

/// A recomputed plan for one alias, bound to the file as it is now.
pub struct Prepared {
    alias: String,
    file_name: String,
    canonical: PathBuf,
    snapshot: NativeSnapshot,
    stack: CommandStack,
    generation: i64,
    revision: NameRecipeRevision,
    plan: NameRecipePlan,
    token: String,
}

impl Prepared {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn changed(&self) -> usize {
        self.plan.preview().changed
    }

    fn source(&self) -> Value {
        json!({
            "project": self.alias,
            "file": self.file_name,
            "state": "working",
            "historyGeneration": self.generation,
        })
    }

    /// The text of the confirmation form: alias, count and a short excerpt.
    pub fn confirmation_message(&self) -> String {
        let preview = self.plan.preview();
        let mut message = format!(
            "Rename {} group address(es) in project \"{}\" ({}). The change goes to the \
             project's undo history; save it in KNXBench to keep it.",
            preview.changed, self.alias, self.file_name
        );
        for row in preview
            .rows
            .iter()
            .filter(|row| row.changed)
            .take(CONFIRMATION_EXCERPT_ROWS)
        {
            message.push_str(&format!(
                "\n#{}: {:?} -> {:?}",
                row.id.0, row.before, row.after
            ));
        }
        if preview.changed > CONFIRMATION_EXCERPT_ROWS {
            message.push_str(&format!(
                "\n... and {} more",
                preview.changed - CONFIRMATION_EXCERPT_ROWS
            ));
        }
        message
    }
}

/// Recomputes the plan for `alias` from the file. Refuses aliases that were
/// not enabled at launch and stores that would need a migration.
pub fn prepare(
    ws: &Workspace,
    alias: &str,
    ids: &[u32],
    prefix: &str,
    suffix: &str,
) -> Result<Prepared, String> {
    let path = ws.name_recipe_path(alias)?;
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| format!("project {alias:?} can no longer be read: {e}"))?;
    let readonly = knx_store::open_existing_read_only(&canonical)
        .map_err(|e| format!("project {alias:?} cannot be opened read-only: {e}"))?;
    if readonly.migrated_from.is_some() {
        return Err(format!(
            "project {alias:?} was saved by an older KNXBench; open and save it there first. \
             Naming recipes never migrate a file"
        ));
    }
    let editor = history::load_editor(&readonly.conn).map_err(|e| e.to_string())?;
    let (snapshot, stack, generation) = match editor {
        Some(editor) => (
            editor.working,
            CommandStack::from_snapshot_states(editor.undo, editor.redo),
            editor.generation,
        ),
        None => (
            NativeSnapshot::read(&readonly.conn).map_err(|e| e.to_string())?,
            CommandStack::new(),
            0,
        ),
    };
    drop(readonly);
    let path_hash = history::image_hash(canonical.as_os_str().as_encoded_bytes());
    let authority = format!("mcp-name-v1:{alias}:{path_hash}:{generation}");
    let revision = NameRecipeRevision {
        project_incarnation: format!("mcp-name-v1:{path_hash}"),
        generation: generation
            .try_into()
            .map_err(|_| "invalid native history generation".to_string())?,
    };
    let intent = NameRecipeIntent {
        ids: ids.iter().copied().map(GroupAddressId).collect(),
        prefix: prefix.to_string(),
        suffix: suffix.to_string(),
    };
    let plan =
        recipe::preview_name_recipe(&snapshot, &revision, intent).map_err(|e| e.to_string())?;
    let token = plan
        .confirmation_token(&authority)
        .map_err(|e| e.to_string())?;
    Ok(Prepared {
        alias: alias.to_string(),
        file_name: canonical
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        canonical,
        snapshot,
        stack,
        generation,
        revision,
        plan,
        token,
    })
}

/// `preview_name_recipe`: the exact rows and the token an apply must quote.
pub fn preview(prepared: &Prepared) -> ToolResult {
    Ok(envelope(
        prepared.source(),
        json!({
            "preview": prepared.plan.preview(),
            "confirmationToken": prepared.token,
            "persistence": PERSISTENCE,
            "applyRequires": "apply_name_recipe with this token, then the user's approval in a confirmation form",
        }),
    ))
}

/// A token must be the exact lower-case hex digest a preview returned.
pub fn check_token_shape(token: &str) -> Result<(), String> {
    let well_formed = token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if well_formed {
        Ok(())
    } else {
        Err("confirmationToken must be the exact token a preview returned".into())
    }
}

/// Refuses a token that no longer matches the file and intent.
pub fn check_token(prepared: &Prepared, token: &str) -> Result<(), String> {
    if prepared.token == token {
        Ok(())
    } else {
        Err(
            "the confirmation token is stale or belongs to another intent; preview again. \
             Nothing was changed"
                .into(),
        )
    }
}

/// Applies an approved plan through native compare-and-save.
pub fn apply(mut prepared: Prepared) -> ToolResult {
    let target = NameRecipeNativeTarget {
        path: &prepared.canonical,
        history_generation: prepared.generation,
    };
    let outcome = recipe::apply_name_recipe(
        &mut prepared.snapshot,
        &mut prepared.stack,
        &prepared.revision,
        &prepared.plan,
        Some(target),
    )
    .map_err(|e| format!("{e}. Nothing was changed"))?;
    Ok(envelope(
        prepared.source(),
        json!({
            "applied": outcome.changed,
            "changed": prepared.plan.preview().changed,
            "nativeGeneration": outcome.native_generation,
            "persistence": PERSISTENCE,
            "savedRootUnchanged": true,
            "note": "Read tools keep showing the saved file until the project is saved in KNXBench, where Undo is available.",
            "preview": prepared.plan.preview(),
        }),
    ))
}
