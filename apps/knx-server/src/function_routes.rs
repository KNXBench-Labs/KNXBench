//! Project Functions over HTTP: a projection read and revision-bound edits (ADR-0124).
//!
//! Project Functions over HTTP (ADR-0124): a read of the projection and
//! revision-bound edits that run the `knx_core` Function commands. No bus
//! access. The web UI that uses this is Goal B work.

use crate::{
    domain::{self, BoundCommandError, SnapshotBinding},
    errors::ApiError,
    SharedState,
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use knx_core::{
    function::{Function, FunctionDetails, FunctionMember},
    BuildingPartId, Command, CompletionStatus, FunctionId, FunctionMemberId, GroupAddressId,
    Project, SourceRef,
};
use knx_projection::{build_functions_view, FunctionNode};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

/// The ETS `CompletionStatus` spellings.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
enum Completion {
    #[default]
    Undefined,
    Editing,
    FinishedDesign,
    FinishedCommissioning,
    Tested,
    Locked,
    Accepted,
}
impl From<Completion> for CompletionStatus {
    fn from(value: Completion) -> Self {
        match value {
            Completion::Undefined => Self::Undefined,
            Completion::Editing => Self::Editing,
            Completion::FinishedDesign => Self::FinishedDesign,
            Completion::FinishedCommissioning => Self::FinishedCommissioning,
            Completion::Tested => Self::Tested,
            Completion::Locked => Self::Locked,
            Completion::Accepted => Self::Accepted,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Details {
    name: String,
    #[serde(default)]
    function_type: Option<String>,
    #[serde(default)]
    number: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    completion: Completion,
}
impl Details {
    fn normalized(self) -> FunctionDetails {
        FunctionDetails {
            name: self.name,
            function_type: self.function_type,
            number: self.number,
            comment: self.comment,
            description: self.description,
            completion: self.completion.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Action {
    Create {
        location: u32,
        details: Details,
    },
    Update {
        id: u32,
        details: Details,
    },
    Delete {
        id: u32,
    },
    Move {
        id: u32,
        location: u32,
    },
    /// Appends unless `position` is given.
    AddMember {
        function_id: u32,
        group_address_id: u32,
        #[serde(default)]
        name: String,
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        position: Option<usize>,
    },
    RemoveMember {
        function_id: u32,
        member_id: u32,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EditRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    action: Action,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    functions: Vec<FunctionNode>,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditView {
    /// The Function or member the edit created, if any.
    created_id: Option<u32>,
    #[serde(flatten)]
    view: View,
}

/// Source reference for an entity created here; it has no ETS origin.
fn native_source(kind: &str, id: u32) -> SourceRef {
    let label = format!("KB-{kind}-{id}");
    SourceRef {
        path: label.clone(),
        ets_id: label,
    }
}

fn view(state: &SharedState) -> Result<View, ApiError> {
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    Ok(View {
        functions: build_functions_view(project).functions,
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: domain::current_project_revision(state),
    })
}

async fn read(State(state): State<SharedState>) -> Result<Json<View>, ApiError> {
    view(&state).map(Json)
}

fn command(
    project: &mut Project,
    action: Action,
    created: &mut Option<u32>,
) -> Result<Command, String> {
    Ok(match action {
        Action::Create { location, details } => {
            let id = project.ids.next_function_id().map_err(|e| e.to_string())?;
            *created = Some(id.0);
            Command::CreateFunction {
                function: Function {
                    id,
                    source: native_source("Function", id.0),
                    location: BuildingPartId(location),
                    details: details.normalized(),
                    members: vec![],
                },
            }
        }
        Action::Update { id, details } => Command::UpdateFunction {
            id: FunctionId(id),
            details: details.normalized(),
        },
        Action::Delete { id } => Command::DeleteFunction { id: FunctionId(id) },
        Action::Move { id, location } => Command::MoveFunction {
            id: FunctionId(id),
            location: BuildingPartId(location),
        },
        Action::AddMember {
            function_id,
            group_address_id,
            name,
            role,
            position,
        } => {
            let length = project
                .functions
                .iter()
                .find(|f| f.id.0 == function_id)
                .map(|f| f.members.len())
                .ok_or_else(|| format!("Function {function_id} not found"))?;
            let id = project
                .ids
                .next_function_member_id()
                .map_err(|e| e.to_string())?;
            *created = Some(id.0);
            Command::AddFunctionMember {
                function: FunctionId(function_id),
                member: FunctionMember {
                    id,
                    source: native_source("FunctionMember", id.0),
                    group_address: GroupAddressId(group_address_id),
                    name,
                    role,
                },
                position: position.unwrap_or(length),
            }
        }
        Action::RemoveMember {
            function_id,
            member_id,
        } => Command::RemoveFunctionMember {
            function: FunctionId(function_id),
            member: FunctionMemberId(member_id),
        },
    })
}

async fn edit(
    State(state): State<SharedState>,
    Json(request): Json<EditRequest>,
) -> Result<Json<EditView>, ApiError> {
    let binding = SnapshotBinding {
        server_incarnation: request.server_incarnation,
        project_incarnation: request.project_incarnation,
        snapshot_revision: request.snapshot_revision,
    };
    let detail = format!("{:?}", request.action);
    let mut created = None;
    domain::apply_bound_command(&state, &binding, "FunctionEdit", detail, |project| {
        command(project, request.action, &mut created)
    })
    .map_err(|error| match error {
        BoundCommandError::NoProject => ApiError::bad_request("No project open"),
        BoundCommandError::Stale => ApiError::conflict(
            "functionStale",
            "The project changed since these Functions were read; refresh and edit again. Nothing was applied.",
        ),
        BoundCommandError::Refused(message) => ApiError::refused("functionRefused", message),
        BoundCommandError::Failure(message) => ApiError::internal(format!(
            "Function edit was not applied because it could not be persisted: {message}"
        )),
    })?;
    Ok(Json(EditView {
        created_id: created,
        view: view(&state)?,
    }))
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/project/functions", get(read))
        .route("/api/project/functions/edit", post(edit))
}
