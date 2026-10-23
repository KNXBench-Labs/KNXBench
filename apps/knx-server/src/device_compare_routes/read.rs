//! Full comparisons use App-created evidence; legacy partial reads have no adoption handle.
use super::{adoption, binding_at, ComparisonBinding};
use crate::{errors::ApiError, SharedState};
use knx_app::{
    comparison_adoption::{self as service, ObservedComparison, PreparedComparison},
    device_download::PreparedDownload,
};
use knx_core::{
    commissioning::partial_memory_download::PartialDownloadParts, ContactableAddress,
    IndividualAddress,
};
use knx_net::{
    commissioning::{memory_download::PlanComparison, SessionTiming},
    ManagementTransport,
};
use knx_productdb::{image::DownloadImage, image_comparison::ComparisonRow};

pub(super) enum Prepared {
    Full(Box<PreparedComparison>),
    Partial(Box<PreparedDownload>),
}
pub(super) struct Read {
    pub(super) prepared: Prepared,
    pub(super) name: String,
    pub(super) binding: ComparisonBinding,
}
pub(super) enum Observation {
    Full(Box<ObservedComparison>),
    Partial {
        prepared: Box<PreparedDownload>,
        comparison: Box<PlanComparison>,
        rows: Vec<ComparisonRow>,
    },
}
impl Read {
    pub(super) fn prepare(
        state: &SharedState,
        target: IndividualAddress,
        partial: Option<PartialDownloadParts>,
        expected: Option<&ComparisonBinding>,
    ) -> Result<Self, ApiError> {
        if let Some(parts) = partial {
            let before = binding_at(state, target);
            let prepared = crate::device_download_routes::prepare(state, target, Some(parts))?;
            let binding = binding_at(state, target)
                .filter(|b| {
                    before.as_ref() == Some(b)
                        && b.device_id == prepared.device.0
                        && expected.is_none_or(|e| e == b)
                })
                .ok_or_else(stale)?;
            return Ok(Self {
                name: prepared.device_name.clone(),
                prepared: Prepared::Partial(Box::new(prepared)),
                binding,
            });
        }
        let owner = state.product_db.as_ref().ok_or_else(|| {
            ApiError::conflict(
                "downloadProductDataMissing",
                crate::device_download_routes::no_application_program(state),
            )
        })?;
        let guard = state.project.lock().expect("state mutex poisoned");
        let project = guard
            .as_ref()
            .ok_or_else(|| ApiError::conflict("downloadProjectMissing", "no project is open"))?;
        let device =
            knx_app::programming_state::sole_device_at(project, target).ok_or_else(|| {
                ApiError::conflict(
                    "deviceCompareNotComparable",
                    "No unique project device at this address",
                )
            })?;
        let stack = state.command_stack.lock().expect("state mutex poisoned");
        let scope = adoption::scope(state);
        let binding = ComparisonBinding {
            server_incarnation: scope.server_incarnation.clone(),
            project_incarnation: scope.project_incarnation,
            device_id: device.0,
            comparison_revision: stack.comparison_revision(device),
        };
        if expected.is_some_and(|e| e != &binding) {
            return Err(stale());
        }
        let prepared = owner
            .with_read(|conn| service::prepare_comparison(conn, project, &stack, &scope, device))
            .map_err(crate::catalogue_owner::owner_error)?
            .map_err(adoption::error)?;
        let name = project
            .devices
            .get(device)
            .expect("resolved project device")
            .name
            .clone();
        Ok(Self {
            prepared: Prepared::Full(Box::new(prepared)),
            name,
            binding,
        })
    }
    pub(super) fn is_current(&self, state: &SharedState) -> bool {
        match &self.prepared {
            Prepared::Full(p) => current(state, |project, stack| {
                p.is_current(project, stack, &adoption::scope(state))
            }),
            Prepared::Partial(p) => binding_at(state, p.target).as_ref() == Some(&self.binding),
        }
    }
    pub(super) async fn observe(
        self,
        transport: &impl ManagementTransport,
        timing: SessionTiming,
    ) -> Result<Observation, ApiError> {
        match self.prepared {
            Prepared::Full(p) => (*p)
                .observe(transport, timing)
                .await
                .map(|observation| Observation::Full(Box::new(observation)))
                .map_err(adoption::error),
            Prepared::Partial(p) => {
                let target = ContactableAddress::new(p.target)
                    .map_err(|e| ApiError::bad_request(e.to_string()))?;
                let comparison = knx_net::commissioning::memory_download::compare_with_plan(
                    transport, target, timing, &p.plan,
                )
                .await
                .map_err(super::compare_error)?;
                let rows =
                    knx_productdb::image_comparison::decode(&p.image, &comparison.held, &p.plan);
                Ok(Observation::Partial {
                    prepared: p,
                    comparison: Box::new(comparison),
                    rows,
                })
            }
        }
    }
}
impl Observation {
    pub(super) fn comparison(&self) -> &PlanComparison {
        match self {
            Self::Full(o) => o.comparison(),
            Self::Partial { comparison, .. } => comparison,
        }
    }
    pub(super) fn rows(&self) -> &[ComparisonRow] {
        match self {
            Self::Full(o) => o.rows(),
            Self::Partial { rows, .. } => rows,
        }
    }
    pub(super) fn image(&self) -> &DownloadImage {
        match self {
            Self::Full(o) => o.preparation().image(),
            Self::Partial { prepared, .. } => &prepared.image,
        }
    }
    pub(super) fn program_id(&self) -> &str {
        match self {
            Self::Full(o) => o.preparation().program_id(),
            Self::Partial { prepared, .. } => &prepared.request.program_id,
        }
    }
    pub(super) fn is_current(
        &self,
        state: &SharedState,
        target: IndividualAddress,
        binding: &ComparisonBinding,
    ) -> bool {
        match self {
            Self::Full(o) => current(state, |project, stack| {
                o.is_current(project, stack, &adoption::scope(state))
            }),
            Self::Partial { .. } => binding_at(state, target).as_ref() == Some(binding),
        }
    }
}
fn current(
    state: &SharedState,
    check: impl FnOnce(&knx_core::Project, &knx_core::CommandStack) -> bool,
) -> bool {
    let project = state.project.lock().expect("state mutex poisoned");
    let stack = state.command_stack.lock().expect("state mutex poisoned");
    project.as_ref().is_some_and(|p| check(p, &stack))
}
pub(super) fn stale() -> ApiError {
    ApiError::conflict(
        "deviceCompareStale",
        "Project/device changed; refresh and compare again. No tunnel was opened.",
    )
}
